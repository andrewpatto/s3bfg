use http::Request;
use hyper::Body;
use native_tls::TlsConnector;
use rayon::prelude::*;
use std::error::Error;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;
use std::io::{BufReader, Cursor};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::os::unix::prelude::FileExt;
use std::str;
use std::str::from_utf8;
use std::sync::Arc;
use std::time::Instant;

use crate::config::Config;
use crate::s3_ip_pool::S3IpPool;
use crate::datatype::BlockToStream;
use rusoto_core::signature::SignedRequest;
use rusoto_core::Region;
use rusoto_credential::{AwsCredentials, DefaultCredentialsProvider, ProvideAwsCredentials};
use std::convert::TryInto;

// use nix::fcntl::OFlag;
// const O_DIRECT: i32 = 0x4000;
/// Synchronously copy a file from S3 to a local drive.
///
/// Uses whatever tricks it can to speed up operations including
/// - multithreading
/// - pool of destination S3 IP addresses
pub fn sync_execute(
    connection_tracker: &Arc<S3IpPool>,
    blocks: &Vec<BlockToStream>,
    config: &Config,
    credentials: &AwsCredentials,
    client_region: &Region,
    bucket_region: &Region,
) {
    println!("Starting a threaded synchronous copy");

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.synchronous_threads as usize)
        .build()
        .unwrap();

    println!("Rayon thread pool is set up to operate with {} executions in parallel, across {} potential S3 endpoints", pool.current_num_threads(), connection_tracker.ip_count());

    pool.install(|| {
        blocks.into_par_iter()
            .for_each(|p: &BlockToStream| {

                // we can have occasional (rare) network fails on reads so we just have to retry
                // and hope it gets better.. so each block will retry 3 times and then fail
                for _read_attempt in 1..3 {

                    // we have no guarantees that we will have enough S3 IP addresses for our
                    // threads - so we have a pool and continually pick the least used
                    let (tcp_addr, tcp_count) = connection_tracker.use_least_used_ip();

                    let current_thread_index = pool.current_thread_index().unwrap_or(999);

                    // for debug purposes we have an id for this streaming attempt
                    let stream_id = format!("{}-{}-{}", tcp_addr, tcp_count, current_thread_index);

                    match sync_stream_range_from_s3(stream_id.as_str(),
                                                    tcp_addr,
                                                    p.start,
                                                    p.length,
                                                    p.start,
                                                    config, credentials, bucket_region) {
                        Ok(_) => return,
                        Err(e) => {
                            eprintln!("An S3 read attempt to {} failed with message {:?} but we will try again (up to 3 times)", tcp_addr, e);

                            // should we take the IP address out of our pool as potentially it has gone bad?
                        }
                    }
                }

                eprintln!("We attempted to read S3 block at {} ({} bytes) the maximum number of times and all failed - aborting the entire copy", p.start, p.length);
                std::process::exit(1);
            });
    });

    let ips = connection_tracker.ips.lock().unwrap();

    for ip in ips.iter() {
        println!(
            "Ending synchronous transfer with S3 {} used {} times",
            ip.0, ip.1
        );
    }
}

/// Streams a portion of an S3 object from network to disk.
///
fn sync_stream_range_from_s3(
    stream_id: &str,
    tcp_host_addr: IpAddr,
    read_start: u64,
    read_length: u64,
    write_location: u64,
    cfg: &Config,
    credentials: &AwsCredentials,
    bucket_region: &Region,
) -> Result<(), Box<dyn Error>> {
    // these are all our benchmark points - set initially to be the starting time
    let now_started = Instant::now();
    let now_response_body_received: Instant;

    // sets up the standard rusoto signed request for S3 GET
    // (though we are about to use it in a non-standard way)
    let mut aws_request = SignedRequest::new(
        "GET",
        "s3",
        bucket_region,
        format!("/{}/{}", cfg.input_bucket_name, cfg.input_bucket_key).as_str(),
    );

    // S3 is very finnicky here when doing v4 signing.. so some of the DNS names that resolve correctly
    // are not correct for signing.. it *MUST* be s3-<region>.amazonaws.com
    aws_request.set_hostname(Option::from(format!(
        "s3-{}.amazonaws.com",
        bucket_region.name()
    )));
    aws_request.add_header("Accept", "*/*");
    aws_request.add_header(
        "Range",
        format!("bytes={}-{}", read_start, read_start + read_length - 1).as_str(),
    );

    aws_request.sign(credentials);

    // println!("{:?}", aws_request);

    let connector = TlsConnector::new().unwrap();

    let tcp_stream = TcpStream::connect(SocketAddr::new(tcp_host_addr, 443))?;

    // disable Nagle as we know we are just sending the whole request in one packet
    // tcp_stream.set_nodelay(true).unwrap();

    // we need to use the 'real' name of the host in the SSL setup - even though the IP address
    // we are using is from a pool
    //let ssl_host = format!(
    //    "{}.s3-{}.amazonaws.com",
    //    cfg.input_bucket_name, bucket_region
    //);

    let mut ssl_stream = connector.connect(aws_request.hostname().as_str(), tcp_stream)?;

    // build into a buffer and send in one go.
    let mut prelude: Vec<u8> = vec![];
    write!(prelude, "GET {} HTTP/1.1\n", aws_request.path())?;
    for (k, v) in aws_request.headers() {
        write!(prelude, "{}: {}\n", k, from_utf8(v[0].as_ref()).unwrap());
    }
    // whilst this may be too pessimistic - for the moment it guarantees our read() will
    // only get data for our request
    write!(prelude, "{}: {}\n", "connection", "close")?;
    write!(prelude, "{}: {}\n", "user-agent", "s3bfg")?;
    write!(prelude, "\n")?;

    // if debugging
    println!("{}", from_utf8(&prelude).unwrap());

    // return Ok(());

    // write all to the wire
    ssl_stream.write_all(&prelude[..])?;

    //let mut progress_reader = ProgressReader::new(&mut tcp_stream, |progress: usize| {
    //    total.fetch_add(progress, Ordering::SeqCst);
    //});

    let mut reader = BufReader::with_capacity(1024 * 1024, ssl_stream);

    // read HTTP headers until they are done
    loop {
        let mut line = String::new();

        let _read_status = reader.read_line(&mut line)?;

        print!("{}", line);

        if line.eq("\r\n") {
            break;
        }
    }

    // we will first stream down into memory from the network stream
    let mut memory_buffer = Cursor::new(vec![0; read_length as usize]);

    let copied_bytes = io::copy(&mut reader, &mut memory_buffer)?;

    // we can either just write into a memory buffer we then throw away
    // or onto a disk.. memory only allows benchmarking of networking without
    // disk io complicating things
    if !cfg.memory_only {
        let oo = OpenOptions::new()
            .write(true)
            .create(false)
            //                 .custom_flags(OFlag::O_DIRECT.bits())
            .clone();

        //            println!("{:?}", oo);

        let disk_buffer = oo.open(&cfg.output_write_filename.as_ref().unwrap())?;

        disk_buffer.write_all_at(&mut memory_buffer.get_ref(), write_location)?;
    }

    now_response_body_received = Instant::now();

    let copied_duration = now_response_body_received.duration_since(now_started);

    assert_eq!(copied_bytes, read_length);

    println!(
        "{}: rate MiB/sec = {} (copied {} bytes in {}s)",
        stream_id,
        (copied_bytes as f32 / (1024.0 * 1024.0)) / copied_duration.as_secs_f32(),
        copied_bytes,
        copied_duration.as_secs_f32()
    );

    Ok(())
}

// /// A data structure for all the elements of an HTTP request that are involved in
// /// the Amazon Signature Version 4 signing process
// #[derive(Debug)]
// pub struct SignedRequest {
//     /// The HTTP Method
//     pub method: String,
//     /// The AWS Service
//     pub service: String,
//     /// The AWS Region
//     pub region: Region,
//     /// The HTTP request path
//     pub path: String,
//     /// The HTTP Request Headers
//     pub headers: BTreeMap<String, Vec<Vec<u8>>>,
//     /// The HTTP request paramaters
//     pub params: Params,
//     /// The HTTP/HTTPS protocol
//     pub scheme: Option<String>,
//     /// The AWS hostname
//     pub hostname: Option<String>,
//     /// The HTTP Content
//     pub payload: Option<SignedRequestPayload>,
//     /// The Standardised query string
//     pub canonical_query_string: String,
//     /// The Standardised URI
//     pub canonical_uri: String,
// }
