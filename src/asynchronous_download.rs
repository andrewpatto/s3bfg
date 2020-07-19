use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, ToSocketAddrs};
use std::str;
use std::sync::Arc;

use futures::stream::FuturesUnordered;

use metrics_runtime::{Receiver, Sink};
use rusoto_core::Region;
use rusoto_credential::AwsCredentials;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

use tokio_rustls::*;

use crate::config::Config;

use crate::datatype::BlockToStream;
use crate::metric_names::{
    METRIC_OVERALL_TRANSFERRED_BYTES, METRIC_SLOT_DISK_RATE_BYTES_PER_SEC,
    METRIC_SLOT_NETWORK_RATE_BYTES_PER_SEC, METRIC_SLOT_REQUEST, METRIC_SLOT_RESPONSE,
    METRIC_SLOT_SSL_SETUP, METRIC_SLOT_STATE_SETUP, METRIC_SLOT_TCP_SETUP,
    METRIC_SLOT_TRANSFER_RATE_BYTES_PER_SEC, TIMING_NANOSEC_SUFFIX,
};

use crate::s3_ip_pool::S3IpPool;
use crate::s3_request_signed::make_signed_get_range_request;

use regex::Regex;

pub type BoxError = std::boxed::Box<
    dyn std::error::Error + std::marker::Send + std::marker::Sync, // needed for threads
>;

lazy_static! {
    static ref STATUS_REGEX: Regex =
        Regex::new(r##"HTTP/1.1 (?P<code>[0-9][0-9][0-9]) "##).unwrap();
}

/// Asynchronously transfer a file from S3.
///
pub async fn download_s3_file(
    receiver: &Receiver,
    s3_ip_pool: &Arc<S3IpPool>,
    blocks: Vec<BlockToStream>,
    config: &Config,
    credentials: &AwsCredentials,
    bucket_region: &Region,
) {
    let mut slot_sockets =
        vec![SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 443); config.s3_connections as usize];

    // from our pool of S3 ip addresses we create slots that will target each of them
    // up to the number of concurrent connections that have been asked for
    // (note: possibly using the same S3 IP address more than once)
    for slot in 0..config.s3_connections as usize {
        let (tcp_addr, _tcp_count) = s3_ip_pool.use_least_used_ip();

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
            let actual_work_future = download_block_work(
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

///
async fn download_block_work(
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
    // our slot sink is used for per slot timings
    let mut slot_sink = overall_sink.scoped(format!("slot-{}", slot).as_str());

    // our thread sink is used for per thread metrics
    let thread_unique = thread_id::get();

    let mut thread_sink = overall_sink.scoped(format!("thread-{}", thread_unique).as_str());

    thread_sink.increment_counter("blocks_processed", 1);

    let now_start = slot_sink.now();

    metric_it!("overall-construct_signed_request", overall_sink, true,
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

    metric_it!("overall-tls_config_setup", overall_sink, true,
        let mut tls_config = rustls::ClientConfig::new();
        tls_config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS)
    );

    metric_it!("overall-tls_connector_setup", overall_sink, false,
        let tls_connector = tokio_rustls::TlsConnector::from(Arc::new(tls_config))
    );

    metric_it!("overall-dnsname_ref_setup", overall_sink, false,
        let domain =
            tokio_rustls::webpki::DNSNameRef::try_from_ascii_str(real_hostname.as_str()).unwrap()
    );

    let _now_setup = slot_sink.now();

    //
    // -- initial tcp stream connection
    //

    metric_it!(METRIC_SLOT_TCP_SETUP, overall_sink, true,
        let tcp_stream = tokio::net::TcpStream::connect(s3_socket_addr).await?
    );

    //
    // -- do SSL handshake and setup SSL stream
    //

    metric_it!(METRIC_SLOT_SSL_SETUP, overall_sink, true,
        let stream = tls_connector.connect(domain, tcp_stream).await?
    );

    let (reader, mut writer) = tokio::io::split(stream);

    // most of our network reads on linux seem to be in the ~20k range so a 256k buffer for the reader seems plenty
    let mut buf_reader = tokio::io::BufReader::with_capacity(256 * 1024, reader);

    metric_it!(
        METRIC_SLOT_REQUEST,
        slot_sink,
        true,
        writer.write_all(http_request.as_slice()).await?
    );

    // parse the first line of the HTTP response - the status line - which in our S3 case is about all we care
    // about for now
    /* {
        let mut status_line = String::new();

        let _status_line_length = buf_reader.read_line(&mut status_line).await?;

        let status_parse_result = STATUS_REGEX.captures(status_line.as_str());

        if status_parse_result.is_some() {
            let status = status_parse_result.unwrap();

            let status_code = status.name("code").unwrap().as_str();

            if status_code != "200" && status_code != "206" {
                bail!(status_code);
            }
        } else {
            bail!("500")
        }
    } */

    let _now_request_sent = slot_sink.now();

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

    let _now_response_headers_received = slot_sink.now();

    let copied_bytes: u64;

    {
        let a = slot_sink.now();

        let mut writeable_buf = vec![0u8; block.length as usize];

        let b = slot_sink.now();
        slot_sink.record_timing("networkmalloc", a, b);

        copied_bytes = buf_reader.read_exact(&mut writeable_buf.as_mut()).await? as u64;

        let e = slot_sink.now();
        slot_sink.record_timing("networkread", b, e);

        let network_elapsed_seconds = (e - now_start) as f64 / (1000.0 * 1000.0 * 1000.0);

        if network_elapsed_seconds > 0.0 {
            let bytes_per_sec = copied_bytes as f64 / network_elapsed_seconds;
            slot_sink.record_value(METRIC_SLOT_NETWORK_RATE_BYTES_PER_SEC, bytes_per_sec as u64);
        }

        if !memory_only {
            let before_fileopen = slot_sink.now();

            let mut oo = std::fs::OpenOptions::new();
            oo.write(true);
            oo.create(false);

            let mut file_writer = tokio::fs::OpenOptions::from(oo)
                .open(output_filename.unwrap())
                .await?;

            let after_fileopen = slot_sink.now();
            slot_sink.record_timing("diskopen", before_fileopen, after_fileopen);

            file_writer
                .seek(std::io::SeekFrom::Start(block.start))
                .await?;

            let after_fileseek = slot_sink.now();
            slot_sink.record_timing("diskseek", after_fileopen, after_fileseek);

            file_writer.write_all(&writeable_buf).await?;

            let after_filewrite = slot_sink.now();
            slot_sink.record_timing(
                format!("diskwrite_{}", TIMING_NANOSEC_SUFFIX),
                after_fileseek,
                after_filewrite,
            );

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

            let after_fileflush = slot_sink.now();
            slot_sink.record_timing("diskflush", after_filewrite, after_fileflush);

            let disk_elapsed_seconds =
                (after_fileflush - before_fileopen) as f64 / (1000.0 * 1000.0 * 1000.0);

            if disk_elapsed_seconds > 0.0 {
                let bytes_per_sec = copied_bytes as f64 / disk_elapsed_seconds;
                slot_sink.record_value(METRIC_SLOT_DISK_RATE_BYTES_PER_SEC, bytes_per_sec as u64);
            }
        }

        // record that we have transferred bytes
        overall_sink.increment_counter(METRIC_OVERALL_TRANSFERRED_BYTES, copied_bytes);
    }

    let now_response_received = slot_sink.now();

    assert_eq!(copied_bytes, block.length);

    let elapsed_seconds = (now_response_received - now_start) as f64 / (1000.0 * 1000.0 * 1000.0);

    if elapsed_seconds > 0.0 {
        let bytes_per_sec = copied_bytes as f64 / elapsed_seconds;

        slot_sink.record_value(
            METRIC_SLOT_TRANSFER_RATE_BYTES_PER_SEC,
            bytes_per_sec as u64,
        );
    }

    Ok(slot)
}

// async-std
//let connector = TlsConnector::default();
//let tcp_stream = async_std::net::TcpStream::connect(&sa).await?;
//let mut tls_stream = connector.connect(&real_hostname, tcp_stream).await?;
//let (mut reader, mut writer) = &mut (&tls_stream, &tls_stream);
//let mut buf_reader = async_std::io::BufReader::new(reader);
