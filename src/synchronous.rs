extern crate ureq;

use std::io;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Cursor, SeekFrom};
use std::io::prelude::*;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::net::Ipv4Addr;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::prelude::FileExt;
use std::str;
use std::sync::Arc;
use std::time::Instant;

use httparse::parse_headers;
use native_tls::TlsConnector;
use rayon::prelude::*;

use crate::config::Config;
use crate::datatype::{BlockToStream, ConnectionTracker};

// use nix::fcntl::OFlag;
// const O_DIRECT: i32 = 0x4000;


pub fn sync_execute(connection_tracker: &Arc<ConnectionTracker>, blocks: &Vec<BlockToStream>, config: &Config) {
    println!("Starting a threaded synchronous copy");

    let pool = rayon::ThreadPoolBuilder::new().num_threads(config.synchronous_threads).build().unwrap();

    println!("Rayon thread pool is set up to operate with {} executions in parallel, across {} potential S3 endpoints", pool.current_num_threads(), connection_tracker.ips.lock().unwrap().len());

    pool.install(|| {
        blocks.into_par_iter()
            .for_each(|p| {

                // we can have occasional (rare) network fails on reads so we just have to retry
                // and hope it gets better.. so each block will retry 3 times and then fail
                for _read_attempt in 1..3 {

                    // we have no guarantees that we will have enough S3 IP addresses for our
                    // threads - so we use a mutex protected set of ips and chose the least used
                    // one
                    let tcp_addr: IpAddr;
                    let stream_id: String;
                    {
                        let mut ips = connection_tracker.ips.lock().unwrap();

                        // our S3 endpoint with the lowest usage so far
                        let lowest_usage = ips.iter_mut().min_by_key(|x| *x.1);

                        let (ip, count) = lowest_usage.unwrap();

                        tcp_addr = ip.parse::<IpAddr>().unwrap();

                        let current_thread_index = pool.current_thread_index().unwrap_or(999);
                        let current_count = *count;

                        stream_id = format!("{}-{}-{}", ip, current_count, current_thread_index);

                        *count = *count + 1;
                    }

                    match sync_stream_range_from_s3(stream_id.as_str(),
                                                    tcp_addr,
                                                    p.start,
                                                    p.length,
                                                    p.start,
                                                    config) {
                        Ok(_) => return,
                        Err(e) => {
                            println!("An S3 read attempt to {} failed with message {:?} but we will try again (up to 3 times)", tcp_addr, e);
                        }
                    }
                }
            });
    });


    let ips = connection_tracker.ips.lock().unwrap();

    for ip in ips.iter() {
        println!("Ending synchronous transfer with S3 {} used {} times", ip.0, ip.1);
    }
}

fn sync_stream_range_from_s3(stream_id: &str, tcp_host_addr: IpAddr, read_start: u64, read_length: u64, write_location: u64, cfg: &Config) -> Result<(), Box<dyn Error>> {
    // these are all our benchmark points - set initially to be the starting time
    let now_started = Instant::now();
    // let now_connected: Instant;
    let now_response_body_received : Instant;

    let connector = TlsConnector::new().unwrap();

    let tcp_stream = TcpStream::connect(SocketAddr::new(tcp_host_addr, 443))?;

    // now_connected = Instant::now();

    // disable Nagle as we know we are just sending the whole request in one packet
    // tcp_stream.set_nodelay(true).unwrap();

    let ssl_host = format!("{}.s3-{}.amazonaws.com", cfg.input_bucket_name, cfg.input_bucket_region);

    let mut ssl_stream = connector.connect(ssl_host.as_str(), tcp_stream)?;

    // build into a buffer and send in one go.
    let mut prelude: Vec<u8> = vec![];

    // request line
    write!(
        prelude,
        "GET {} HTTP/1.1\r\n",
        cfg.input_bucket_key
    )?;

    write!(prelude, "Host: {}.s3-{}.amazonaws.com\r\n", cfg.input_bucket_name, cfg.input_bucket_region)?;
    write!(prelude, "User-Agent: s3bigfile\r\n")?;
    write!(prelude, "Accept: */*\r\n")?;
    write!(prelude, "Range: bytes={}-{}\r\n", read_start, read_start + read_length - 1)?;
    write!(prelude, "Connection: close\r\n")?;

    // finish
    write!(prelude, "\r\n")?;

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

        let disk_buffer = oo.open(&cfg.output_write_filename)?;

        disk_buffer.write_all_at(&mut memory_buffer.get_ref(), write_location)?;
    }

    now_response_body_received = Instant::now();

    let copied_duration = now_response_body_received.duration_since(now_started);

    assert_eq!(copied_bytes, read_length);

    println!("{}: rate MiB/sec = {} (copied {} bytes in {}s)",
             stream_id,
             (copied_bytes as f32 / (1024.0 * 1024.0)) / copied_duration.as_secs_f32(),
             copied_bytes,
             copied_duration.as_secs_f32());

    Ok(())
}
