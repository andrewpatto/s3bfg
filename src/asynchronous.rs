use std::io::stdout;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, ToSocketAddrs};
use std::str;
use std::sync::{Arc};
use tokio::sync::Mutex;
use std::time::{Duration, Instant};

use futures::stream::{FuturesUnordered, StreamExt as FuturesStreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use metrics_core::{Builder, Drain, Observe};
use metrics_runtime::{Receiver, Sink, Controller};
use rusoto_core::Region;
use rusoto_credential::AwsCredentials;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Runtime;
use tokio::stream::StreamExt as TokioStreamExt;
use tokio_rustls::*;

use crate::config::Config;
use crate::copy_exact::copy_exact;
use crate::datatype::BlockToStream;
use crate::metric_names::{
    METRIC_OVERALL_TRANSFER_BYTES, METRIC_OVERALL_TRANSFER_STARTED, METRIC_SLOT_RATE_BYTES_PER_SEC,
    METRIC_SLOT_REQUEST, METRIC_SLOT_RESPONSE, METRIC_SLOT_SSL_SETUP, METRIC_SLOT_STATE_SETUP,
    METRIC_SLOT_TCP_SETUP,
};
use crate::metric_observer_progress::ProgressObserver;
use crate::metric_observer_ui::UiBuilder;
use crate::s3_ip_pool::S3IpPool;
use crate::s3_request_signed::make_signed_get_range_request;
use std::borrow::{BorrowMut, Borrow};

pub type BoxError = std::boxed::Box<
    dyn std::error::Error + std::marker::Send + std::marker::Sync, // needed for threads
>;

pub fn async_execute(
    s3_ip_pool: &Arc<S3IpPool>,
    blocks: &Vec<BlockToStream>,
    config: &Config,
    credentials: &AwsCredentials,
    bucket_region: &Region,
) {
    println!("Starting a tokio asynchronous copy");

    // a tokio runtime we will use for our async io
    // if we are being asked to work in async mode then construct a Tokio runtime
    // builder using any command line args set
    let mut rt: Runtime;
    let mut rt_description: String = String::new();

    if config.asynchronous_basic {
        rt = tokio::runtime::Builder::new()
            .enable_all()
            .basic_scheduler()
            .build()
            .unwrap();
        rt_description.push_str("basic");
    } else {
        let mut temp = tokio::runtime::Builder::new();

        temp.enable_all();
        temp.threaded_scheduler();

        rt_description.push_str("threaded ");

        if config.asynchronous_core_threads > 0 {
            temp.core_threads(config.asynchronous_core_threads as usize);
            rt_description.push_str(config.asynchronous_core_threads.to_string().as_str());
            rt_description.push_str("(core)/");
        } else {
            rt_description.push_str("default(core)/");
        }
        if config.asynchronous_max_threads > 0 {
            temp.max_threads(config.asynchronous_max_threads as usize);
            rt_description.push_str(config.asynchronous_max_threads.to_string().as_str());
            rt_description.push_str("(max)");
        } else {
            rt_description.push_str("512(max)");
        }

        rt = temp.build().unwrap();
    }

    println!(
        "Tokio runtime is set up to operate with {} config, utilising {} S3 connections",
        rt_description, config.s3_connections
    );

    //let mut overall_sink = config.receiver.sink();

    //overall_sink.update_gauge(METRIC_OVERALL_TRANSFER_STARTED, overall_sink.now() as i64);


    rt.block_on(async_execute_transfer(s3_ip_pool, blocks, config, credentials, bucket_region));

    println!();

    rt.shutdown_timeout(Duration::from_millis(100));

    let mut builder = UiBuilder::new();
    let mut observer = builder.build();

    config.receiver.controller().observe(&mut observer);

    println!("{}", observer.drain());
}

async fn async_execute_transfer(s3_ip_pool: &Arc<S3IpPool>,
                                blocks: &Vec<BlockToStream>,
                                config: &Config,
                                credentials: &AwsCredentials,
                                bucket_region: &Region,) {

    let controller = config.receiver.controller();
    let file_size_bytes = config.file_size_bytes;

    // start a blocking thread which displays a progress meter off our metrics
    tokio::task::spawn_blocking(move || {
        progress_worker(controller, file_size_bytes);
    });

    // create a vector of the actual block structs
    let blocks_real: Vec<BlockToStream> = blocks.iter().cloned().collect();


    let mut slot_sockets =
        vec![SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 443); config.s3_connections as usize];

    let mut slot_buffers =
        vec![Arc::new(Mutex::new(Vec::<u8>::with_capacity(config.segment_size_bytes as usize))); config.s3_connections as usize];

    for slot in 0..config.s3_connections as usize {
        let (tcp_addr, tcp_count) = s3_ip_pool.use_least_used_ip();
        let socket_addr = SocketAddrV4::new(tcp_addr, 443);
        slot_sockets[slot] = socket_addr;
        // slot_buffers[slot].lock().unwrap().resize(config.segment_size_bytes as usize, 0u8);
    }


    let mut futs = FuturesUnordered::new();

    // the current slot indicates which S3 connection slot we are making units of work for
    let mut current_slot: usize = 0;

    for b in blocks_real {
        // get the S3 addr that this slot targets
        let s3_addr = slot_sockets[current_slot];
        let mut buffer = slot_buffers[current_slot].clone();

        // create the worker to work in this slot
        let fut = async move {
            let actual_work_future = async_execute_work(
                current_slot,
                s3_addr,
                buffer,
                b,
                config,
                credentials,
                bucket_region,
            );

            let finished_slot = actual_work_future.await.unwrap();

            finished_slot
        };

        // this is only of relevance in the opening N iterations of the loop -
        // after the future we 'wait' on will define the replacement current_slot
        current_slot += 1;

        futs.push(fut);

        if futs.len() == config.s3_connections as usize {
            // we have hit the limit of concurrency we are aiming for
            // so we now await the finish of a worker
            // the slot it returns is then open up for use
            current_slot = futures::stream::StreamExt::next(&mut futs).await.unwrap();
        }
    }

    // drain for remaining work from the queue
    while let Some(item) = futures::stream::StreamExt::next(&mut futs).await {}
}

fn progress_worker(controller: Controller, size: u64) {
    //let m = MultiProgress::new();
    let sty = ProgressStyle::default_bar()
        .template("\r{spinner:.green} [{elapsed_precise}] {bar:20.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}")
        .progress_chars("#>-");

    let pb = ProgressBar::new(size);
    pb.set_style(sty.clone());

    loop {
        let mut observer = ProgressObserver::new();

        controller.observe(&mut observer);

        let msg = observer.render();

        pb.set_message(msg.as_str());
        pb.set_position(observer.transferred());

        //print!("{}", msg);
        //std::io::stdout().flush();
        //m.join().unwrap();

        std::thread::sleep(Duration::from_secs(1));
    }
    pb.finish_with_message("done");

    // m.join_and_clear().unwrap();
}

macro_rules! metric_it {
    ($context:expr, $sink:ident, $record:expr, $($s:stmt);+) => {
        let before = $sink.now();
        $(
            $s
        )*
        if $record {
            $sink.record_timing($context, before, $sink.now());
        }
    }
}

async fn async_execute_work(
    slot: usize,
    s3_socket_addr: SocketAddrV4,
    passed_buffer: Arc<Mutex<Vec<u8>>>,
    block: BlockToStream,
    cfg: &Config,
    credentials: &AwsCredentials,
    bucket_region: &Region,
) -> std::result::Result<usize, BoxError> {
    // the master sink is where we collate 'overall' stats
    let mut overall_sink = cfg.receiver.sink();

    // our slot sink is used for per slot timings
    let mut slot_sink = overall_sink.scoped(format!("{}", slot).as_str());

    let now_start = slot_sink.now();


    metric_it!("construct_signed_request", overall_sink, true,
        let mut http_reqest: Vec<u8> = Vec::with_capacity(1024);
        let real_hostname = make_signed_get_range_request(
            block.start,
            block.length,
            cfg,
            credentials,
            bucket_region,
            &mut http_reqest,
        )
        .unwrap()
    );

    metric_it!("tls_config_setup", overall_sink, true,
        let mut tls_config = rustls::ClientConfig::new();
        tls_config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS)
    );

    metric_it!("tls_connector_setup", overall_sink, false,
        let tls_connector = tokio_rustls::TlsConnector::from(Arc::new(tls_config))
    );

    metric_it!("dnsname_ref_setup", overall_sink, false,
        let domain =
            tokio_rustls::webpki::DNSNameRef::try_from_ascii_str(real_hostname.as_str()).unwrap()
    );

    let now_setup = slot_sink.now();


    //
    // -- initial tcp stream connection
    //

    metric_it!(METRIC_SLOT_TCP_SETUP, slot_sink, true,
        let tcp_stream = tokio::net::TcpStream::connect(s3_socket_addr).await?
    );

    //
    // -- do SSL handshake and setup SSL stream
    //

    metric_it!(METRIC_SLOT_SSL_SETUP, slot_sink, true,
        let mut stream = tls_connector.connect(domain, tcp_stream).await?
    );

    let (reader, mut writer) = tokio::io::split(stream);

    let mut buf_reader = tokio::io::BufReader::new(reader);

    metric_it!(METRIC_SLOT_REQUEST, slot_sink, true,
        writer.write_all(http_reqest.as_slice()).await?
    );

    let now_request_sent = slot_sink.now();

    loop {
        let mut headers = String::new();

        let line_length = buf_reader.read_line(&mut headers).await?;

        // TODO: should detect if the server returns Connection: closed in which case this is our last possible get

        // otherwise we get this - when we come around *after* the close
        if line_length == 0 {
            let err = std::io::Error::new(std::io::ErrorKind::InvalidInput, "connection closed");

            return std::result::Result::Err(Box::new(err));
        }

        //print!("{}", headers);

        // our headers will be terminated by a single line "\r\n"
        if line_length == 2 {
            break;
        }
    }

    let now_response_headers_received = slot_sink.now();

    let copied_bytes: u64;

    {
        let a = slot_sink.now();

        let mut writeable_buf = vec![0u8; block.length as usize];

        let b = slot_sink.now();
        slot_sink.record_timing(
            "allocatebuffer",
            a,
            b,
        );

        //let mut writeable_buf_2 = passed_buffer.lock().await;

        //let c = slot_sink.now();
        //slot_sink.record_timing(
         //   "awaitlock",
         //   b,
         //   c,
        //);

        //writeable_buf_2.resize(block.length as usize, 0u8);

       // let d = slot_sink.now();
       // slot_sink.record_timing(
       //     "resizebuf",
       //     c,
       //     d,
       // );

        // println!("len {}", writeable_buf.len());

        copied_bytes = buf_reader.read_exact(&mut writeable_buf.as_mut()).await? as u64;

        let e = slot_sink.now();
        slot_sink.record_timing(
            "readexact",
            b,
            e,
        );

        // record that we have transferred bytes (even though we haven't written them to disk)
        overall_sink.increment_counter(METRIC_OVERALL_TRANSFER_BYTES, copied_bytes);

       // println!("read {}", copied_bytes);

        //if cfg.memory_only {
//
  //      } else {
        if !cfg.memory_only {
            let mut oo = std::fs::OpenOptions::new();
            oo.write(true);
            oo.create(false);

            let mut file_writer = tokio::fs::OpenOptions::from(oo)
                .open(cfg.output_write_filename.as_ref().unwrap())
                .await?;

            file_writer
                .seek(std::io::SeekFrom::Start(block.start))
                .await?;

            file_writer.write_all(&writeable_buf).await?;

            // note that copy_exact is responsible for generating some metrics via the passed
            // in sink (including the overall bytes transferred counter)
            //copied_bytes = copy_exact(
            //    overall_sink,
             //   &mut buf_reader,
             //   &mut file_writer,
             //   block.length,
           // )
            //.await?;

            file_writer.flush();
        }
    }

    let now_response_received = slot_sink.now();

    slot_sink.record_timing(
        METRIC_SLOT_RESPONSE,
        now_request_sent,
        now_response_received,
    );

    assert_eq!(copied_bytes, block.length);

    let elapsed_seconds = (now_response_received - now_start) as f64 / (1000.0 * 1000.0 * 1000.0);

    if elapsed_seconds > 0.0 {
        let bytes_per_sec = copied_bytes as f64 / elapsed_seconds;

        slot_sink.record_value(METRIC_SLOT_RATE_BYTES_PER_SEC, bytes_per_sec as u64);
    }

    Ok(slot)
}

// async-std
//let connector = TlsConnector::default();
//let tcp_stream = async_std::net::TcpStream::connect(&sa).await?;
//let mut tls_stream = connector.connect(&real_hostname, tcp_stream).await?;
//let (mut reader, mut writer) = &mut (&tls_stream, &tls_stream);
//let mut buf_reader = async_std::io::BufReader::new(reader);
