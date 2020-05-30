extern crate ureq;
extern crate num_cpus;
extern crate nix;
extern crate hyper;

mod datatype;
mod asynchronous;
mod synchronous;
mod ips;
mod copy_exact;

use std::net::{TcpStream, ToSocketAddrs, IpAddr, SocketAddr};

use std::{io, thread, env};
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{SeekFrom, BufReader};
use std::time::{Duration, Instant};
use std::str;
use std::os::unix::io::AsRawFd;
use progress_streams::ProgressReader;
use resolve::resolve_host;
use rayon::prelude::*;
use std::thread::sleep;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicUsize;
use std::borrow::Borrow;
use clap::{self, Arg, App};
use httparse::parse_headers;
use humansize::{FileSize, file_size_opts as options};
use std::collections::{HashSet, BTreeMap};
use rand::Rng;

#[cfg(target_os = "linux")]
use nix::fcntl::fallocate;
#[cfg(target_os = "linux")]
use nix::fcntl::FallocateFlags;

use crate::datatype::{BlockToStream, ConnectionTracker};
use crate::synchronous::sync_execute;
use crate::asynchronous::async_execute;
use crate::ips::populate_by_dns;
use std::convert::TryInto;
use tokio::runtime::{Runtime, Builder};
use futures::TryFutureExt;

//  ~/s3zoom gos-test-cases-public /GIB1_8398.bam bf2 -s 83886080
// m5d.8xlarge Overall: rate MiB/sec = 1131.3945 (copied 29400082342 bytes in 24.781897s)

// m5d.24xlarge

// Overall: rate MiB/sec = 1296.0306 (copied 29400082342 bytes in 21.633827s)
// (base) [ec2-user@ip-10-1-1-63 data]$  ~/s3zoom gos-test-cases-public /GIB1_8398.bam bf2 -s 83886080 -t 32

// Overall: rate MiB/sec = 895.3182 (copied 29400082342 bytes in 31.316357s)
// (base) [ec2-user@ip-10-1-1-63 data]$  ~/s3zoom gos-test-cases-public /GIB1_8398.bam bf2 -s 83886080 -t 16

const S3_DOMAIN_SUFFIX: &str = ".s3.ap-southeast-2.amazonaws.com";


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
            .about("Sets the size in mebibytes of each independently streamed part of the file - multiples of 8 will generally match S3 part sizing")
            .takes_value(true))
        .arg(Arg::with_name("threads")
            .short('t')
            .long("threads")
            .about("Sets the number of threads to use to execute the streaming gets, default is detected core count")
            .default_value(num_cpus.to_string().as_str())
            .takes_value(true))
        .arg(Arg::with_name("dns-server")
            .long("dns-server")
            .about("Sets the DNS resolver to directly query to find S3 bucket IP addresses")
            .default_value("169.254.169.253:53")
            .takes_value(true))
        .arg(Arg::with_name("dns-count")
            .long("dns-count")
            .about("Sets the number of attempts that will be made to obtain distinct S3 bucket IP addresses")
            .takes_value(true))
        .arg(Arg::with_name("memory")
            .long("memory")
            .about("If specified tells us to just transfer the data to memory and not then write it out to disk"))
        .arg(Arg::with_name("fallocate")
            .long("fallocate")
            .about("If specified tells us to create the blank destination file using fallocate()"))
        .arg(Arg::with_name("basic")
            .long("basic")
            .about("If specified tells us to use basic tokio runtime rather than threaded"))
        .get_matches();


    // the file to copy and where to go
    let bucket_name = String::from(matches.value_of("INPUTBUCKET").unwrap());
    let bucket_key = String::from(matches.value_of("INPUTKEY").unwrap());
    let write_filename = String::from(matches.value_of("OUTPUTFILE").unwrap());

    // how to split up the file
    let segment_size = matches.value_of_t::<u64>("segment-size").unwrap_or(8);
    let segment_size_bytes = segment_size * 1024 * 1024;
    let threads = matches.value_of_t::<usize>("threads").unwrap();

    // DNS settings
    let dns_server = String::from(matches.value_of("dns-server").unwrap());
    let dns_count = matches.value_of_t::<usize>("dns-count").unwrap_or(threads * 2);

    //
    let memory_only = matches.is_present("memory");
    let fallocate = matches.is_present("fallocate");
    let basic = matches.is_present("basic");

    if memory_only {
        println!("Copying file s3://{}{} to memory", bucket_name, bucket_key);
    } else {
        println!("Copying file s3://{}{} to {}", bucket_name, bucket_key, write_filename);
    }

    let total_size_bytes: u64 = head_size_from_s3(bucket_name.as_str(), bucket_key.as_str()).unwrap_or_default();

    if !memory_only {
        create_empty_target_file(write_filename.as_str(), total_size_bytes.try_into().unwrap()).unwrap();
    }

    let mut blocks = vec![];

    // construct our units of 'copy' activity that we want to do.. whilst this is pretty simple we
    // splitting up of a file we could potentially do something more sophisticated
    {
        let mut starter: u64 = 0;
        let full_chunks = total_size_bytes / segment_size_bytes;
        let leftover_chunk_size_bytes = total_size_bytes % segment_size_bytes;

        for x in 0..full_chunks {
            blocks.push(BlockToStream { start: starter, length: segment_size_bytes });
            starter += segment_size_bytes;
        }

        if leftover_chunk_size_bytes > 0 {
            blocks.push(BlockToStream { start: starter, length: leftover_chunk_size_bytes });
        }

        println!("File size is {} which means {} segments of chosen size {} MiB + leftover {}",
                 total_size_bytes.file_size(options::BINARY).unwrap(),
                 full_chunks,
                 segment_size,
                 leftover_chunk_size_bytes.file_size(options::BINARY).unwrap());
    }

    //rayon::ThreadPoolBuilder::new().num_threads(threads).build_global().unwrap();

    //println!("Thread pool is set up to operate with {} executions in parallel",threads);

    // start with the tcp destination having a host name
    let bucket_host: String = format!("{}.s3.ap-southeast-2.amazonaws.com", bucket_name);
    let bucket_host_with_port: String = format!("{}.s3.ap-southeast-2.amazonaws.com:80", bucket_name);

    let total_started = Instant::now();

    // let address = dns_server.parse().unwrap();

    let connection_tracker = Arc::new(ConnectionTracker::new());

    /*  {
          let dns_started = Instant::now();

          populate_by_dns(&connection_tracker, &address, dns_count).unwrap();

          let dns_duration = Instant::now().duration_since(dns_started);
          let ips_db = connection_tracker.ips.lock().unwrap();

          println!("Discovered {} distinct S3 endpoints in {}s",
                   ips_db.len(), dns_duration.as_secs_f32());
      } */

    let s3_ip = ("52.95.132.218", 443)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;


    let mut rt = if basic {
        Builder::new()
            .enable_all()
            .basic_scheduler()
            .build()
            .unwrap()
    } else {
        Builder::new()
            .enable_all()
            .threaded_scheduler()
            .build()
            .unwrap()
    };

    // spawn the server task
    rt.block_on(async move {
        let r = async_execute(&s3_ip, &blocks, &bucket_name, &bucket_key, &write_filename, memory_only).await;

        println!("{:?}", r.unwrap());
    });

    rt.shutdown_timeout(Duration::from_millis(100));

    /*    sync_execute(&connection_tracker, &blocks, &bucket_host, &bucket_name, &bucket_key, &write_filename, memory_only);

        //if cfg!(unix) {
        //    options.custom_flags(libc::O_EXCL);
        //}
        let ips = connection_tracker.ips.lock().unwrap();

        for ip in ips.iter() {
            println!("Ending with {} used {} times", ip.0, ip.1);
        } */

    let total_duration = Instant::now().duration_since(total_started);

    println!("{}: rate MiB/sec = {} (copied {} bytes in {}s)",
             "Overall",
             (total_size_bytes as f32 / (1024.0 * 1024.0)) / total_duration.as_secs_f32(),
             total_size_bytes,
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

#[cfg(target_os = "linux")]
fn create_empty_target_file(write_filename: &str, size: i64) -> Result<File, io::Error> {

    // because we want to let fallocate do its best we want to always work on a new file
    let file = OpenOptions::new().write(true)
        .create_new(true)
        .open(write_filename)?;

    let fd = file.as_raw_fd();

    // for linux we have the added ability to allocate the full size of the file
    // without any actual zero initialising
    fallocate(fd, FallocateFlags::empty(), 0, size);

    Ok(file)
}

#[cfg(not(target_os = "linux"))]
fn create_empty_target_file(write_filename: &str, size: i64) -> Result<File, io::Error> {
    let file = OpenOptions::new().write(true)
        .create(true)
        .open(write_filename)?;

    Ok(file)
}
