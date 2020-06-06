extern crate ureq;
extern crate num_cpus;

use std::net::{TcpStream, ToSocketAddrs, IpAddr, SocketAddr};

use std::{io, thread, env};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{SeekFrom, BufReader, Cursor, BufWriter};
use std::time::{Duration, Instant};
use std::str;
use std::os::unix::prelude::FileExt;
use std::os::unix::fs::OpenOptionsExt;
use nix::fcntl::OFlag;
use progress_streams::ProgressReader;
use resolve::resolve_host;
use rayon::prelude::*;
use std::thread::sleep;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicUsize;
use std::borrow::Borrow;
use clap::{Arg, App};
use httparse::parse_headers;
use humansize::{FileSize, file_size_opts as options};
use std::collections::{HashSet, BTreeMap};
use rand::Rng;
use crate::datatype::{BlockToStream, ConnectionTracker};
use std::str::FromStr;
use std::net::Ipv4Addr;
use crate::config::Config;

const O_DIRECT: i32 = 0x4000;


pub fn sync_execute(connection_tracker: &Arc<ConnectionTracker>, blocks: &Vec<BlockToStream>, config: &Config) {

    blocks.into_par_iter()
        .for_each(|p| {

            // we have no guarantees that we will have enough S3 IP addresses for our
            // threads - so we just chose the least used so far
            let tcp_addr: IpAddr;
            let mut stream_id: String;
            {
                let mut ips = connection_tracker.ips.lock().unwrap();

                let lowest_usage = ips.iter_mut().min_by_key(|x| *x.1);

                let (ip,count) = lowest_usage.unwrap();

                tcp_addr = ip.parse::<IpAddr>().unwrap();
                stream_id = format!("{}-{}", ip, count);

                *count = *count + 1;
            }

            // println!("{}: Streaming range at {} for {} bytes", stream_id, p.start, p.length);

            sync_stream_range_from_s3(stream_id.as_str(),
                                      tcp_addr,
                                      &config.input_bucket_name,
                                      &config.input_bucket_key,
                                      p.start,
                                      p.length,
                                      &config.output_write_filename,
                                      p.start,
                                        config.memory_only);
        });
}

fn sync_stream_range_from_s3(stream_id: &str, tcp_host_addr: IpAddr, s3_bucket: &str, s3_key: &str, read_start: u64, read_length: u64, write_filename: &str, write_location: u64, memory_only: bool) {
    // these are all our benchmark points - set initially to be the starting time
    let now_started = Instant::now();
    let mut now_connected = now_started.clone();
    let mut now_response_body_received = now_started.clone();

    if let Ok(mut tcp_stream) = TcpStream::connect(SocketAddr::new(tcp_host_addr, 80)) {
        now_connected = Instant::now();

        // disable Nagle as we know we are just sending the whole request in one packet
        tcp_stream.set_nodelay(true).unwrap();

        // build into a buffer and send in one go.
        let mut prelude: Vec<u8> = vec![];

        // request line
        write!(
            prelude,
            "GET {} HTTP/1.1\r\n",
            s3_key
        ).unwrap();

        write!(prelude, "Host: {}.s3.ap-southeast-2.amazonaws.com\r\n", s3_bucket).unwrap();
        write!(prelude, "User-Agent: s3bigfile\r\n").unwrap();
        write!(prelude, "Accept: */*\r\n").unwrap();
        write!(prelude, "Range: bytes={}-{}\r\n", read_start, read_start + read_length - 1).unwrap();
        write!(prelude, "Connection: close\r\n").unwrap();

        // finish
        write!(prelude, "\r\n").unwrap();

        // write all to the wire
        tcp_stream.write_all(&prelude[..]).unwrap();

        //let mut progress_reader = ProgressReader::new(&mut tcp_stream, |progress: usize| {
        //    total.fetch_add(progress, Ordering::SeqCst);
        //});

        let mut reader = BufReader::with_capacity(1024 * 1024, tcp_stream);

        // read HTTP headers until they are done
        loop {
            let mut line = String::new();

            let read_status = reader.read_line(&mut line).unwrap();

            if line.eq("\r\n") {
                break;
            }
        }

        // stream down into memory
        let copied_bytes: u64;
        let mut memory_buffer = Cursor::new(vec![0; read_length as usize]);

        copied_bytes = io::copy(&mut reader, &mut memory_buffer).unwrap();

        // we can either just write into a memory buffer we then throw away
        // or onto a disk.. memory only allows benchmarking of networking without
        // disk io complicating things
        if !memory_only {
             let oo = OpenOptions::new()
                .write(true)
                .create(false)
//                 .custom_flags(OFlag::O_DIRECT.bits())
                 .clone();

//            println!("{:?}", oo);

            let disk_buffer = oo.open(write_filename)
                .unwrap();

            disk_buffer.write_all_at(&mut memory_buffer.get_ref(), write_location).unwrap();
        }

        now_response_body_received = Instant::now();

        let copied_duration = now_response_body_received.duration_since(now_started);

        assert_eq!(copied_bytes, read_length);

        println!("{}: rate MiB/sec = {} (copied {} bytes in {}s)",
                 stream_id,
                 (copied_bytes as f32 / (1024.0 * 1024.0)) / copied_duration.as_secs_f32(),
                 copied_bytes,
                 copied_duration.as_secs_f32())
    }
}
