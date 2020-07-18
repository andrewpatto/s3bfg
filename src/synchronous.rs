use crate::config::Config;
use crate::datatype::BlockToStream;
use crate::metric_observer_ui::UiBuilder;
use crate::s3_ip_pool::S3IpPool;
use crate::s3_request_signed::make_signed_get_range_request;
use log::Level;
use metrics_core::{Builder, Drain, Label, Observe, Observer};
use metrics_runtime::Controller;
use metrics_runtime::{exporters::LogExporter, observers::YamlBuilder, Receiver};
use rayon::prelude::*;
use regex::Regex;
use rusoto_core::signature::SignedRequest;
use rusoto_core::Region;
use rusoto_credential::{AwsCredentials, DefaultCredentialsProvider, ProvideAwsCredentials};
use rustls;
use rustls::Session;
use socket2::{Domain, SockAddr, Socket, Type};
use std::convert::TryInto;
use std::error::Error;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;
use std::io::{BufReader, Cursor};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::os::unix::prelude::FileExt;
use std::str;
use std::str::from_utf8;
use std::sync::Arc;
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

// use nix::fcntl::OFlag;
// const O_DIRECT: i32 = 0x4000;
/// Synchronously copy a file from S3 to a local drive.
///
/// Uses whatever tricks it can to speed up operations including
/// - multithreading
/// - pool of destination S3 IP addresses
pub fn sync_execute(
    s3_ip_pool: &Arc<S3IpPool>,
    blocks: &Vec<BlockToStream>,
    config: &Config,
    credentials: &AwsCredentials,
    bucket_region: &Region,
) {
    println!("Starting a threaded synchronous copy");

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.synchronous_threads as usize)
        .build()
        .unwrap();

    println!("Rayon thread pool is set up to operate with {} executions in parallel, across {} potential S3 endpoints", pool.current_num_threads(), s3_ip_pool.ip_count());

    pool.install( || {
       // spawn(|| {
       //     loop {
       //         let snap = receiver.controller().snapshot();
//
  //              println!("{:?}", snap.into_measurements());

    //            sleep(Duration::new(10, 0));
      //      }
            //LogExporter::new(
            //    &receiver.controller(),
            //    YamlBuilder::new(),
            //    Level::Info,
            //    Duration::from_secs(5),
            //).run();
       // });

        let receiver = Receiver::builder().
                    histogram(Duration::from_secs(1000), Duration::from_secs(60)).build().expect("failed to create receiver");

        blocks.into_par_iter()
            .for_each(|p: &BlockToStream| {

                // we can have occasional (rare) network fails on reads so we just have to retry
                // and hope it gets better.. so each block will retry 3 times and then fail
                for _read_attempt in 0..3 {

                    // we have no guarantees that we will have enough S3 IP addresses for our
                    // threads - so we have a pool and continually pick the least used
                    let (tcp_addr, tcp_count) = s3_ip_pool.use_least_used_ip();

                    let current_thread_index = pool.current_thread_index().unwrap_or(999);

                    let current_thread_label = format!("{}", current_thread_index);

                    // for debug purposes we have an id for this streaming attempt
                    // let stream_id = format!("{}-{}-{}", tcp_addr, tcp_count, current_thread_index);

                    match sync_stream_range_from_s3(&receiver,
                                                    current_thread_index,
                                                    tcp_addr,
                                                    p.start,
                                                    p.length,
                                                    p.start,
                                                    config, credentials, bucket_region) {
                        Ok(_) =>  {
                            return;
                        } ,
                        Err(e) => {
                            eprintln!("An S3 read attempt to {} failed with message {:?} but we will try again (up to 3 times)", tcp_addr, e);

                            // should we take the IP address out of our pool as potentially it has gone bad?
                        }
                    }
                }

                eprintln!("We attempted to read S3 block at {} ({} bytes) the maximum number of times and all failed - aborting the entire copy", p.start, p.length);
                std::process::exit(1);
            });

        let mut observer = UiBuilder::new().build();

        receiver.controller().observe(&mut observer);

        observer.drain();
    });

    let ips = s3_ip_pool.ips.lock().unwrap();

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
    receiver: &Receiver,
    thread_index: usize,
    tcp_host_addr: Ipv4Addr,
    read_start: u64,
    read_length: u64,
    write_location: u64,
    cfg: &Config,
    credentials: &AwsCredentials,
    bucket_region: &Region,
) -> Result<(), Box<dyn Error>> {
    // our sink allows us to record performance metrics
    let mut root_sink = receiver.sink();

    let mut sink = root_sink.scoped("a");

    // sink.add_default_labels(&[("thread", THREAD_LABELS[thread_index])]);

    // these are all our benchmark points - set initially to be the starting time
    let now_started = sink.now();

    // println!("{:?}", aws_request);
    let socket = Socket::new(Domain::ipv4(), Type::stream(), None).unwrap();
    let socket_dest: SockAddr = SocketAddrV4::new(tcp_host_addr, 443).into();

    // disable Nagle as we know we are just sending the whole request in one packet
    socket.set_nodelay(true)?;

    // println!("Linger {:?}", socket.linger()?);

    socket.connect(&socket_dest)?;

    let tcp_stream = socket.into_tcp_stream();
    //TcpStream::connect(SocketAddr::new(tcp_host_addr, 443))?;

    //let connector = TlsConnector::new().unwrap();

    let mut prelude: Vec<u8> = vec![];
    let real_hostname = make_signed_get_range_request(
        credentials,
        bucket_region,
        cfg.input_bucket_name.as_str(),
        cfg.input_bucket_key.as_str(),
        read_start,
        read_length,
        &mut prelude,
    )
    .unwrap();

    // we need to use the 'real' name of the host in the SSL setup - even though the IP address
    // we are using is from a pool
    let mut ssl_stream = tcp_stream; // connector.connect(real_hostname.as_str(), tcp_stream)?;

    let now_ssl_connected = sink.now();

    // if debugging
    // println!("{}", from_utf8(&prelude).unwrap());

    // return Ok(());

    // write all to the wire
    ssl_stream.write_all(&prelude[..])?;

    let mut reader = BufReader::with_capacity(1024 * 1024, ssl_stream);

    // parse the first line of the HTTP response - the status line - which in our S3 case is about all we care
    // about for now
    {
        let mut status_line = String::new();

        reader.read_line(&mut status_line)?;

        let status_regex = Regex::new(r##"HTTP/1.1 (?P<code>[0-9][0-9][0-9]) "##).unwrap();

        let status_parse_result = status_regex.captures(status_line.as_str());

        if status_parse_result.is_some() {
            let status = status_parse_result.unwrap();

            let status_code = status.name("code").unwrap().as_str();

            if status_code != "200" && status_code != "206" {
                bail!(status_code);
            }
        } else {
            bail!("500")
        }
    }

    let now_http_status_response = sink.now();

    // now read the rest of the HTTP headers until they are done
    loop {
        let mut line = String::new();

        let _read_status = reader.read_line(&mut line)?;

        // print!("{}", line);

        if line.eq("\r\n") {
            break;
        }
    }

    // we will first stream down into memory from the network stream
    let mut memory_buffer = Cursor::new(vec![0; read_length as usize]);

    let copied_bytes = io::copy(&mut reader, &mut memory_buffer)?;

    let now_http_data_read = sink.now();

    sink.record_timing(
        "http_block_data_transfer",
        now_http_status_response,
        now_http_data_read,
    );

    // reader.into_inner().shutdown()?;

    // we can either just write into a memory buffer we then throw away
    // or onto a disk.. memory only allows benchmarking of networking without
    // disk io complicating things
    if !cfg.memory_only {
        let oo = OpenOptions::new()
            .write(true)
            .create(false)
            // .custom_flags(OFlag::O_DIRECT.bits())
            .clone();

        //            println!("{:?}", oo);

        let disk_buffer = oo.open(&cfg.output_write_filename.as_ref().unwrap())?;

        disk_buffer.write_all_at(&mut memory_buffer.get_ref(), write_location)?;
    }

    let now_data_written = sink.now();

    sink.record_timing(
        format!("stream_block_{}", read_length),
        now_started,
        now_data_written,
    );

    let nano_taken = now_data_written - now_started;
    let bytes_rate = copied_bytes * 1000 * 1000 / nano_taken;

    root_sink.increment_counter("bytes_transferred", read_length);
    root_sink.record_value("bytes_per_second", bytes_rate);

    //now_response_body_received = Instant::now();

    //let copied_duration = now_response_body_received.duration_since(now_started);

    assert_eq!(copied_bytes, read_length);

    // println!(
    //     "{}: rate MiB/sec = {} (copied {} bytes in {}s)",
    //     stream_id,
    //     (copied_bytes as f32 / (1024.0 * 1024.0)) / copied_duration.as_secs_f32(),
    //     copied_bytes,
    //     copied_duration.as_secs_f32()
    // );

    Ok(())
}

// let mut config = rustls::ClientConfig::new();
//     config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
//
//     let dns_name = webpki::DNSNameRef::try_from_ascii_str("google.com").unwrap();
//     let mut sess = rustls::ClientSession::new(&Arc::new(config), dns_name);
//     let mut sock = TcpStream::connect("google.com:443").unwrap();
//     let mut tls = rustls::Stream::new(&mut sess, &mut sock);
//     tls.write(concat!("GET / HTTP/1.1\r\n",
//                       "Host: google.com\r\n",
//                       "Connection: close\r\n",
//                       "Accept-Encoding: identity\r\n",
//                       "\r\n")
//               .as_bytes())
//         .unwrap();
//     let ciphersuite = tls.sess.get_negotiated_ciphersuite().unwrap();
//     writeln!(&mut std::io::stderr(), "Current ciphersuite: {:?}", ciphersuite.suite).unwrap();
//     let mut plaintext = Vec::new();
//     tls.read_to_end(&mut plaintext).unwrap();
//     stdout().write_all(&plaintext).unwrap();

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
