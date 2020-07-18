use std::io::stdout;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, ToSocketAddrs};
use std::str;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use futures::stream::{FuturesUnordered, StreamExt as FuturesStreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use metrics_core::{Builder, Drain, Observe};
use metrics_runtime::{Controller, Receiver, Sink};
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
use std::borrow::{Borrow, BorrowMut};

pub type BoxError = std::boxed::Box<
    dyn std::error::Error + std::marker::Send + std::marker::Sync, // needed for threads
>;

pub async fn async_execute_transfer(
    receiver: &Receiver,
    s3_ip_pool: &Arc<S3IpPool>,
    blocks: Vec<BlockToStream>,
    config: &Config,
    credentials: &AwsCredentials,
    bucket_region: &Region,
) {
    let controller = receiver.controller();
    let file_size_bytes = config.file_size_bytes;

    // start a blocking thread which displays a progress meter off our metrics
    tokio::task::spawn_blocking(move || {
        progress_worker(controller, file_size_bytes);
    });

    let mut slot_sockets =
        vec![SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 443); config.s3_connections as usize];

    let mut slot_buffers = vec![
        Arc::new(Mutex::new(Vec::<u8>::with_capacity(
            config.segment_size_bytes as usize
        )));
        config.s3_connections as usize
    ];

    // from our pool of S3 ip addresses we create slots that will target them
    // (but only as many slots as the number of s3 connections we are targetting)
    for slot in 0..config.s3_connections as usize {
        let (tcp_addr, tcp_count) = s3_ip_pool.use_least_used_ip();

        slot_sockets[slot] = SocketAddrV4::new(tcp_addr, 443);
    }

    let mut futs = FuturesUnordered::new();

    // the current slot indicates which S3 connection slot we are making units of work for
    let mut current_slot: usize = 0;

    for b in blocks {
        // before creating our async closure we create local variables that are copies
        // of any params - such that our spawned tokio task can own them forever
        let local_credentials = credentials.clone();
        let local_s3_addr = slot_sockets[current_slot];
        let local_s3_bucket_region = bucket_region.clone();
        let local_s3_bucket_name = config.input_bucket_name.clone();
        let local_s3_bucket_key = config.input_bucket_key.clone();
        let local_memory_only = config.memory_only;
        let local_output_filename = config.output_write_filename.clone();

        // construct a sink for any metrics
        let mut block_sink = receiver.sink();

        // create the worker to work in this slot and spawn it on any tokio runtime thread
        futs.push(tokio::spawn(async move {
            let actual_work_future = async_execute_work(
                current_slot,
                &mut block_sink,
                &local_credentials,
                local_s3_addr,
                &local_s3_bucket_region,
                local_s3_bucket_name.as_str(),
                local_s3_bucket_key.as_str(),
                b,
                local_memory_only,
                local_output_filename,
            );

            // rather than have out futures loop deal with errors we want to put
            // some error handling here - currently none
            let finished_slot = actual_work_future.await.unwrap();

            // we need to return the slot *we* were in order that the next
            // worker that is created takes over our slot
            finished_slot
        }));

        // this is only of relevance in the opening N iterations of the loop -
        // after the future we 'wait' on will define the replacement current_slot
        current_slot += 1;

        if futs.len() == config.s3_connections as usize {
            // we have hit the limit of concurrency we are aiming for
            // so we now await the finish of (any!) worker
            // the slot it returns is then open for us to use as the next worker slot
            current_slot = futures::stream::StreamExt::next(&mut futs)
                .await
                .unwrap()
                .unwrap();
        }
    }

    // drain for remaining work from the queue
    while let Some(_) = futures::stream::StreamExt::next(&mut futs).await {}
}

fn progress_worker(controller: Controller, size: u64) {
    //let m = MultiProgress::new();
    let sty = ProgressStyle::default_bar()
        .template("\r{spinner:.green} [{elapsed_precise}] {bar:20.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}")
        .progress_chars("#>-");

    let pb = ProgressBar::new(size);
    pb.set_style(sty.clone());
    pb.println(format!("[+] finished #"));

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
    overall_sink: &mut Sink,
    credentials: &AwsCredentials,
    s3_socket_addr: SocketAddrV4,
    s3_bucket_region: &Region,
    s3_bucket_name: &str,
    s3_bucket_key: &str,
    block: BlockToStream,
    memory_only: bool,
    output_filename: Option<String>,
) -> std::result::Result<usize, BoxError> {
    // the master sink is where we collate 'overall' stats
    //    let mut overall_sink = receiver.sink();

    // our slot sink is used for per slot timings
    let mut slot_sink = overall_sink.scoped(format!("{}", slot).as_str());

    // our thread sink is used for per thread metrics
    let mut thread_sink = overall_sink.scoped(format!("{}", std::process::id()).as_str());

    thread_sink.increment_counter("blocks_processed", 1);

    let now_start = slot_sink.now();

    metric_it!("construct_signed_request", overall_sink, true,
        let mut http_request: Vec<u8> = Vec::with_capacity(1024);
        let real_hostname = make_signed_get_range_request(
            credentials,
            s3_bucket_region,
            s3_bucket_name,
            s3_bucket_key,
            block.start,
            block.length,
            &mut http_request,
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

    metric_it!(
        METRIC_SLOT_REQUEST,
        slot_sink,
        true,
        writer.write_all(http_request.as_slice()).await?
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
        slot_sink.record_timing("allocatebuffer", a, b);

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
        slot_sink.record_timing("readexact", b, e);

        // record that we have transferred bytes (even though we haven't written them to disk)
        overall_sink.increment_counter(METRIC_OVERALL_TRANSFER_BYTES, copied_bytes);

        // println!("read {}", copied_bytes);

        //if cfg.memory_only {
        //
        //      } else {
        if memory_only {
            let mut oo = std::fs::OpenOptions::new();
            oo.write(true);
            oo.create(false);

            let mut file_writer = tokio::fs::OpenOptions::from(oo)
                .open(output_filename.unwrap())
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
