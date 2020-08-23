use std::convert::TryFrom;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::path::PathBuf;
use std::str;

use anyhow::{anyhow, Context, Result};
use md5::{Digest, Md5};
use metrics_runtime::{Receiver, Sink};
use regex::Regex;
use rusoto_core::Region;
use rusoto_credential::AwsCredentials;
use simple_error::SimpleError;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio_rustls::*;

use crate::copy_exact::copy_exact;
use crate::metric_names::METRIC_OVERALL_TRANSFERRED_BYTES;
use crate::metric_names::METRIC_SLOT_REQUEST;
use crate::metric_names::METRIC_SLOT_SSL_SETUP;
use crate::metric_names::METRIC_SLOT_TCP_SETUP;
use crate::metric_names::METRIC_SLOT_TRANSFER_RATE_BYTES_PER_SEC;
use crate::s3_request_signed::{make_signed_get_part_request, make_signed_get_range_request};
use std::sync::Arc;

lazy_static! {
    static ref STATUS_REGEX: Regex =
        Regex::new(r##"HTTP/1.1 (?P<code>[0-9][0-9][0-9]) "##).unwrap();
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

/// Asynchronously do the actual work of transferring a single block of data from S3.
/// Block can either be specified as a byte range of an object, or as a part number.
///
pub async fn download_block_work(
    slot: usize,
    overall_sink: &mut Sink,
    credentials: &AwsCredentials,
    s3_socket_addr: SocketAddr,
    s3_bucket_region: &Region,
    s3_bucket_name: &str,
    s3_bucket_key: &str,
    start: u64,
    length: u64,
    part_number: u32,
    memory_only: bool,
    output_filename: Option<PathBuf>,
    output_start: u64,
) -> anyhow::Result<usize, anyhow::Error> {
    if !memory_only && output_filename.is_none() {
        return Err(anyhow::Error::new(SimpleError::new(
            "If memory_only is False then an output_filename must be specified",
        )));
    }

    // our slot sink is used for per slot timings
    let mut slot_sink = overall_sink.scoped(format!("slot-{}", slot).as_str());

    let now_start = slot_sink.now();

    // our thread sink is used for per thread metrics
    let thread_unique = thread_id::get();

    let mut thread_sink = overall_sink.scoped(format!("thread-{}", thread_unique).as_str());

    thread_sink.increment_counter("blocks_processed", 1);

    metric_it!("overall-construct_signed_request", overall_sink, true,
        let mut http_request: Vec<u8> = Vec::with_capacity(1024);
        let real_hostname =
            if part_number > 0 {
                make_signed_get_part_request(
                    credentials,
                    s3_bucket_region,
                    s3_bucket_name,
                    s3_bucket_key,
                    part_number,
                    &mut http_request,
                )
                .unwrap()
            } else {
                make_signed_get_range_request(
                    credentials,
                    s3_bucket_region,
                    s3_bucket_name,
                    s3_bucket_key,
                    start,
                    length,
                    &mut http_request,
                )
                .unwrap()
            }
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

    //
    // -- split our io into reading and writing streams
    //

    let (reader, mut writer) = tokio::io::split(stream);

    //
    // -- blast out our request for data
    //
    metric_it!(
        METRIC_SLOT_REQUEST,
        slot_sink,
        true,
        writer.write_all(http_request.as_slice()).await?
    );

    //
    // -- process the response from S3
    //

    // most of our network reads on linux seem to be in the ~20k range so a 256k buffer for the reader seems plenty
    let mut buf_reader = tokio::io::BufReader::with_capacity(256 * 1024, reader);

    // parse the first line of the HTTP response - the status line - which in our S3 case is about all we care
    // about for now
    {
        let mut status_line = String::new();

        let _status_line_length = buf_reader.read_line(&mut status_line).await?;

        let status_parse_result = STATUS_REGEX.captures(status_line.as_str());

        if status_parse_result.is_none() {
            return Err(anyhow!(
                "Unparseable HTTP status line from AWS S3 - `{}`",
                status_line
            ));
        }

        let status = status_parse_result.unwrap();

        let status_code = status.name("code").unwrap().as_str();

        match status_code {
            "200" | "206" => {}
            "403" => return Err(anyhow!("Forbidden - `{}`", status_line)),
            _ => {
                return Err(anyhow!(
                    "Unexpected HTTP status code response from AWS S3 - `{}`",
                    status_line
                ))
            }
        }
    }

    let mut header_count = 0;

    loop {
        let mut headers = String::new();

        let line_length = buf_reader.read_line(&mut headers).await?;

        if line_length == 0 {
            return Err(anyhow!(
                "Connection to S3 was closed before we finished reading response headers"
            ));
        }

        // print!("{}", headers);
        // our headers will be terminated by a single line "\r\n"
        if line_length == 2 {
            break;
        }

        header_count += 1;

        if header_count > 100 {
            return Err(anyhow!(
                        "More than 100 HTTP headers were returned from AWS S3 which is wrong so we are aborting"
                    ));
        }
    }

    //
    // -- copy the actual data from S3 and write to disk
    //

    let copied_bytes;

    if memory_only {
        // we use a tokio sink to send the data to nowhere..
        copied_bytes = copy_exact(
            &mut slot_sink,
            &mut buf_reader,
            &mut tokio::io::sink(),
            length,
        )
        .await?;
    } else {
        let mut oo = std::fs::OpenOptions::new();
        oo.write(true);
        oo.create(false);

        let mut file_writer = tokio::fs::OpenOptions::from(oo)
            .open(output_filename.unwrap())
            .await?;

        file_writer
            .seek(std::io::SeekFrom::Start(output_start))
            .await?;

        let mut buf_writer = tokio::io::BufWriter::with_capacity(512 * 1024, file_writer);

        // note that copy_exact is responsible for generating some metrics via the passed
        // in sink (including the overall bytes transferred counter)
        copied_bytes = copy_exact(&mut slot_sink, &mut buf_reader, &mut buf_writer, length).await?;

        buf_writer.flush();

        // TODO: assert block checksums if possible
        // whilst we could possibly compute this hash during the copy_exact routine, we really
        // want to be making an assertion about the content as it has ended up on the disk
        // so we do a reload of the data we just wrote (presumably the OS might help us out
        // by caching this)
        //if false {
        //    let mut hasher = Md5::new();
        //    hasher.update(from block on disk);
        //    let result = hasher.finalize();
        //    assert_eq!(result[..], hex!("5eb63bbbe01eeed093cb22bb8f5acdc3"));
        //}
    }

    assert_eq!(
        copied_bytes, length,
        "Amount recorded as having being copied did not match the length of the block"
    );

    // compute a per slot metric of how fast we are copying things
    {
        let elapsed_seconds = (slot_sink.now() - now_start) as f64 / (1000.0 * 1000.0 * 1000.0);

        if elapsed_seconds > 0.0 {
            let bytes_per_sec = copied_bytes as f64 / elapsed_seconds;

            slot_sink.record_value(
                METRIC_SLOT_TRANSFER_RATE_BYTES_PER_SEC,
                bytes_per_sec as u64,
            );
        }
    }

    // keep an overall record that tracks our total bytes copied
    overall_sink.increment_counter(METRIC_OVERALL_TRANSFERRED_BYTES, copied_bytes);

    Ok(slot)
}
