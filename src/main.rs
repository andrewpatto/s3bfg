extern crate ureq;
extern crate num_cpus;

use std::net::{TcpStream, ToSocketAddrs, IpAddr, SocketAddr};

use std::{io, thread, env};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{SeekFrom, BufReader};
use std::time::{Duration, Instant};
use std::str;

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
use std::collections::HashSet;
use rand::Rng;

//  ~/s3zoom gos-test-cases-public /GIB1_8398.bam bf2 -s 83886080
// m5d.8xlarge Overall: rate MiB/sec = 1131.3945 (copied 29400082342 bytes in 24.781897s)

// m5d.24xlarge

// Overall: rate MiB/sec = 1296.0306 (copied 29400082342 bytes in 21.633827s)
// (base) [ec2-user@ip-10-1-1-63 data]$  ~/s3zoom gos-test-cases-public /GIB1_8398.bam bf2 -s 83886080 -t 32

// Overall: rate MiB/sec = 895.3182 (copied 29400082342 bytes in 31.316357s)
// (base) [ec2-user@ip-10-1-1-63 data]$  ~/s3zoom gos-test-cases-public /GIB1_8398.bam bf2 -s 83886080 -t 16

const S3_DOMAIN_SUFFIX: &str = ".s3.ap-southeast-2.amazonaws.com";

struct BlockToStream {
    start: u64,
    length: u64
}

fn main() -> std::io::Result<()> {
    let num_cpus = num_cpus::get();

    let matches = App::new("s3zoom")
        .version("1.0")
        .author("AP")
        .about("Copies S3 files real quick")
        .arg(Arg::with_name("INPUTBUCKET")
            .about("Sets the S3 bucket name of the input file")
            .required(true)
            .index(1))
        .arg(Arg::with_name("INPUTKEY")
            .about("Sets the S3 key of the input file")
            .required(true)
            .index(2))
        .arg(Arg::with_name("OUTPUTFILE")
            .about("Sets the output file to write to")
            .required(true)
            .index(3))
        .arg(Arg::with_name("segment-size")
            .short('s')
            .long("size")
            .about("Sets the size in bytes of each independently streamed part of the file")
            .takes_value(true))
        .arg(Arg::with_name("threads")
            .short('t')
            .long("threads")
            .about("Sets the number of threads to use to execute the streaming gets, default is detected core count")
            .default_value(num_cpus.to_string().as_str())
            .takes_value(true))
        .arg(Arg::with_name("start-jitter")
            .short('j')
            .long("jitter")
            .about("Sets the range of jitter in sleep before starting each thread, allows S3 names to resolve differently")
            .default_value("1.0")
            .takes_value(true))
        .arg(Arg::with_name("prevent-s3-duplicate")
            .short('p')
            .long("prevent")
            .about("If specified tells us to never allow duplicate S3 servers to be used for independent streams"))
        .get_matches();


    let bucket_name= String::from(matches.value_of("INPUTBUCKET").unwrap());
    let bucket_key = String::from(matches.value_of("INPUTKEY").unwrap());
    let write_filename = String::from(matches.value_of("OUTPUTFILE").unwrap());

    let segment_size = matches.value_of_t::<u64>("segment-size").unwrap_or(8388608);
    let threads = matches.value_of_t::<usize>("threads").unwrap();
    let start_jitter = matches.value_of_t::<f64>("start-jitter").unwrap();
    let prevent_duplicates = matches.is_present("prevent-s3-duplicate");

    println!("Copying file s3://{}{} to {}", bucket_name, bucket_key, write_filename);

    let total_size: u64 = head_size_from_s3(bucket_name.as_str(), bucket_key.as_str()).unwrap_or_default();
    let full_chunks = total_size / segment_size;
    let leftover_chunk_size = total_size % segment_size;

    println!("File size is {} which means {} segments of chosen size {} + leftover {}",
             total_size.file_size(options::BINARY).unwrap(),
             full_chunks,
             segment_size.file_size(options::BINARY).unwrap(),
             leftover_chunk_size.file_size(options::BINARY).unwrap());

    rayon::ThreadPoolBuilder::new().num_threads(threads).build_global().unwrap();

    println!("Thread pool is set up to operate with {} executions in parallel",threads);

    let mut blocks = vec![];
    let mut starter: u64 = 0;

    for x in 0..full_chunks {
        blocks.push(BlockToStream {start: starter, length: segment_size});
        starter += segment_size;
    }

    if leftover_chunk_size > 0 {
        blocks.push(BlockToStream {start: starter, length: leftover_chunk_size});
    }

    // start with the tcp destination having a host name
    let bucket_host: String = format!("{}.s3.ap-southeast-2.amazonaws.com", bucket_name);

    let total = Arc::new(AtomicUsize::new(0));
    let total_clone = total.clone();
    let total_started = Instant::now();

    //let progress_thread = thread::spawn( move || {
    //    let x = total.load(Ordering::Relaxed);
    //    println!("{}", x);
    //    sleep(Duration::from_secs(5));
    //});
    let mut seen_addr_lock = Arc::new(Mutex::new(HashSet::new()));

    blocks.into_par_iter()
        .for_each(|p:BlockToStream| {
            let now_started = Instant::now();
            let mut rng = rand::thread_rng();

            // start jitter
            sleep(Duration::from_secs_f64(rng.gen::<f64>() * start_jitter));

            // force the resolver to dip into the DNS round robin for S3 and get an IP
            // until it finds one we haven't seen before
            let mut tcp_addr: IpAddr;

            loop {
                tcp_addr = resolve_host(bucket_host.as_str()).unwrap().next().unwrap();

                if prevent_duplicates {
                    // obtain a lock on our shared set and then either break out of the loop
                    // (if we have never seen the ip addr) or retry (if we have)
                    let mut seen_addr = seen_addr_lock.lock().unwrap();

                    if !seen_addr.contains(&tcp_addr) {
                        seen_addr.insert(tcp_addr.clone());
                        break;
                    }
                }
                else {
                    break;
                }

                sleep(Duration::from_secs_f64(rng.gen::<f64>()))
            }

            let stream_id = format!("{}", tcp_addr.clone().to_string());

            println!("{}: Spent {:#?} finding S3 destination server", stream_id, Instant::now().duration_since(now_started));
            println!("{}: Streaming range at {} for {} bytes", stream_id, p.start, p.length);

            sync_stream_range_from_s3(stream_id.as_str(),
                                      tcp_addr,
                                      bucket_name.as_str(),
                                      bucket_key.as_str(),
                                      p.start,
                                      p.length,
                                      write_filename.as_str(),
                                      p.start);
        });

    //if cfg!(unix) {
    //    options.custom_flags(libc::O_EXCL);
    //}

    //for part in 1..block_count + 1 {
    //    pool.execute(|| {
    //        fetchBlock(part, (part - 1) * block, bucket_host.as_str(), bucket_key.as_str());
    //    });
    //}
    let total_duration = Instant::now().duration_since(total_started);

    println!("{}: rate MiB/sec = {} (copied {} bytes in {}s)",
             "Overall",
             (total_size as f32 / (1024.0*1024.0)) / total_duration.as_secs_f32(),
             total_size,
             total_duration.as_secs_f32());


    Ok(())
}



// we need to start by working out how large the actual file is before segmenting
fn head_size_from_s3(s3_bucket: &str, s3_key: &str) -> Result<u64, &'static str> {
    let src = format!("http://{}{}{}", s3_bucket, S3_DOMAIN_SUFFIX, s3_key);
    let headresp = ureq::head(src.as_str())
        //.set("X-My-Header", "Secret")
        .call();

    let size = headresp.header("content-length").unwrap();

    Ok(size.parse::<u64>().unwrap())
}


fn sync_stream_range_from_s3(stream_id: &str, tcp_host_addr: IpAddr, s3_bucket: &str, s3_key: &str, read_start: u64, read_length: u64, write_filename: &str, write_location: u64) {
    // these are all our benchmark points - set initially to be the starting time
    let now_started = Instant::now();
    let mut now_connected = now_started.clone();
    let mut now_request_sent = now_started.clone();
    let mut now_response_header_received = now_started.clone();
    let mut now_response_body_received = now_started.clone();

   // let mut file = File::open("/dev/urandom").unwrap();


    if let Ok(mut tcp_stream) = TcpStream::connect(SocketAddr::new(tcp_host_addr, 80)) {
        now_connected = Instant::now();

        // disable Nagle as we know we are just sending the whole request in one packet
        tcp_stream.set_nodelay(true);

        // build into a buffer and send in one go.
        let mut prelude: Vec<u8> = vec![];

        // request line
        write!(
            prelude,
            "GET {} HTTP/1.1\r\n",
            s3_key
        );

        write!(prelude, "Host: {}.s3.ap-southeast-2.amazonaws.com\r\n", s3_bucket);
        write!(prelude, "User-Agent: patto\r\n");
        write!(prelude, "Accept: */*\r\n");
        write!(prelude, "Range: bytes={}-{}\r\n", read_start, read_start + read_length - 1);
        write!(prelude, "Connection: close\r\n");

        // finish
        write!(prelude, "\r\n");

        // write all to the wire
        tcp_stream.write_all(&prelude[..]);

        now_request_sent = Instant::now();

        //let mut progress_reader = ProgressReader::new(&mut tcp_stream, |progress: usize| {
        //    total.fetch_add(progress, Ordering::SeqCst);
        //});

        let mut reader = BufReader::with_capacity(128 * 1024,tcp_stream);

        loop {
            let mut line = String::new();

            let read_status = reader.read_line(&mut line).unwrap();

            if line.eq("\r\n") {
                break;
            }
        }

        now_response_header_received = Instant::now();

        let mut disk_buffer = OpenOptions::new()
            .write(true)
            .create(false)
            .open(write_filename)
            .unwrap();

        disk_buffer.seek(SeekFrom::Start(write_location.into()));

        let copied_bytes = io::copy(&mut reader, &mut disk_buffer).unwrap();

        now_response_body_received = Instant::now();

        let copied_duration = now_response_body_received.duration_since(now_started);

        assert_eq!(copied_bytes, read_length);

        println!("{}: rate MiB/sec = {} (copied {} bytes in {}s)",
                 stream_id,
                 (copied_bytes as f32 / (1024.0*1024.0)) / copied_duration.as_secs_f32(),
                    copied_bytes,
                 copied_duration.as_secs_f32())
    }
}

/*
fn fetchBlock(bs: u32, part: u32, write_location: u32, host: &str, key: &str) {
    print!("{}: Starting\n", part);




    now_resolved_dns = Instant::now();
    {


        println!("{}: connect = {}", part, stream.peer_addr().unwrap());




        // let mut reader = BufReader::new(stream);


        {
            let mut header_buf = vec![0u8; 1024*1024];

            let res = stream.read(&mut header_buf);
            let res_size = res.unwrap();

            print!("{}: read header num bytes {}\n", part, res_size);

            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut req = httparse::Response::new(&mut headers);

            let res2 = req.parse(&header_buf);

            if res2.unwrap().is_complete() {
                let content_slice = &header_buf[0..res_size];
                print!("{}: header version {}",part, req.version.unwrap());
                print!("{}: header code {}",part, req.code.unwrap());
                print!("{}: read header content {:#?}\n", part, str::from_utf8(content_slice));
            }

            now_response_header_received = Instant::now();
        }

        {
            let mut body_buf = vec![0u8; 8388608];

            let res = stream.read_exact(&mut body_buf);

            if res.is_ok() {
                print!("{}: read content bytes\n", part);
            } else {
                print!("{}: failed read content bytes\n", part);
            }


        }

        stream.write_all(&prelude[..]);

        {
            let mut header_buf = vec![0u8; 1024*1024];

            let res = stream.read(&mut header_buf);
            let res_size = res.unwrap();

            print!("{}: read 2nd header num bytes {}\n", part, res_size);

            let content_slice = &header_buf[0..res_size];

            print!("{}: read 2nd header content {:#?}\n", part, str::from_utf8(content_slice));

            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut req = httparse::Request::new(&mut headers);

            let res2 = req.parse(&header_buf);


        }

        //print!("{}: status is partial {}\n", part, res2.unwrap().is_partial());


    } else {
        println!("Couldn't connect to server...");
    }

    let now3 = Instant::now();

    sleep(Duration::from_secs(1));

    println!("{}: DNS Lookup {:?}\n", part, now_resolved_dns.duration_since(now_started));
    println!("{}: Tcp connect {:?}\n", part, now_connected.duration_since(now_resolved_dns));
    println!("{}: Request sent {:?}\n", part, now_request_sent.duration_since(now_connected));
    println!("{}: Header back {:?}\n", part, now_response_header_received.duration_since(now_request_sent));
    println!("{}: Body back {:?}\n", part, now_response_body_received.duration_since(now_response_header_received));
    // println!("{:?}", new_now.duration_since(now));

    return;

    let getname = format!("https://{}/{}?partNumber={}", host, key, part);

    let resp = ureq::get(getname.as_str())
        //.set("X-My-Header", "Secret")
        .call();

    let new_now = Instant::now();

    //println!("{:?}", new_now.duration_since(now));




    // .ok() tells if response is 200-299.
    if resp.ok() {
        assert!(resp.has("Content-Length"));
        let len = resp.header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok()).unwrap();

        let mut reader = resp.into_reader();
        let fname = format!("{:04}.bam", part);

        if true {
            let mut memBytes = vec![0u8; bs as usize];
            reader.read_to_end(&mut memBytes);
            print!("Wrote to memory {}\n", part);

        } else {
            let mut diskBuffer = OpenOptions::new().write(true).create(false).open("real.data").unwrap();
            diskBuffer.seek(SeekFrom::Start(write_location.into()));
            let r = io::copy(&mut reader, &mut diskBuffer);

            if r.is_ok() {
                print!("Wrote to disk {}\n", part);
            }
        }



        //assert_eq!(bytes.len(), len);
    }
}

 */