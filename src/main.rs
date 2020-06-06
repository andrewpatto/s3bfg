extern crate nix;
extern crate num_cpus;
extern crate ureq;
#[macro_use]
extern crate clap;

use std::convert::TryInto;
use std::str;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use futures::future::join_all;
use humansize::{file_size_opts as options, FileSize};
use resolve::resolve_host;
use tokio::runtime::{Builder, Runtime};

use crate::asynchronous::async_execute;
use crate::config::Config;
use crate::datatype::{BlockToStream, ConnectionTracker};
use crate::empty_file::create_empty_target_file;
use crate::ips::populate_a_dns;
use crate::synchronous::sync_execute;

mod datatype;
mod asynchronous;
mod synchronous;
mod ips;
mod copy_exact;
mod config;
mod empty_file;

//  ~/s3zoom gos-test-cases-public /GIB1_8398.bam bf2 -s 83886080
// m5d.8xlarge Overall: rate MiB/sec = 1131.3945 (copied 29400082342 bytes in 24.781897s)

// m5d.24xlarge

// Overall: rate MiB/sec = 1296.0306 (copied 29400082342 bytes in 21.633827s)
// (base) [ec2-user@ip-10-1-1-63 data]$  ~/s3zoom gos-test-cases-public /GIB1_8398.bam bf2 -s 83886080 -t 32

// Overall: rate MiB/sec = 895.3182 (copied 29400082342 bytes in 31.316357s)
// (base) [ec2-user@ip-10-1-1-63 data]$  ~/s3zoom gos-test-cases-public /GIB1_8398.bam bf2 -s 83886080 -t 16

fn main() -> std::io::Result<()> {
    let config = Config::new();

    if config.memory_only {
        println!("Copying file s3://{}{} (region {}) to memory", config.input_bucket_name, config.input_bucket_key, config.input_bucket_region);
    } else {
        println!("Copying file s3://{}{} (region {}) to {}", config.input_bucket_name, config.input_bucket_key, config.input_bucket_region, config.output_write_filename);
    }

    println!("Running on: {}", config.instance_type);
    println!("DNS server chosen: {}", config.dns_server);
    println!("Aiming for {} distinct concurrent connections to S3", config.connections);

    let total_size_bytes: u64 = head_size_from_s3(config.input_bucket_name.as_str(), config.input_bucket_key.as_str(), config.input_bucket_region.as_str()).unwrap_or_default();

    if !config.memory_only {
        create_empty_target_file(config.output_write_filename.as_str(), total_size_bytes.try_into().unwrap())?;
    }

    let mut blocks = vec![];

    // construct our units of 'copy' activity that we want to do.. whilst this is pretty simple we
    // splitting up of a file we could potentially do something more sophisticated
    {
        let mut starter: u64 = 0;
        let full_chunks = total_size_bytes / config.segment_size_bytes;
        let leftover_chunk_size_bytes = total_size_bytes % config.segment_size_bytes;

        for _x in 0..full_chunks {
            blocks.push(BlockToStream { start: starter, length: config.segment_size_bytes });
            starter += config.segment_size_bytes;
        }

        if leftover_chunk_size_bytes > 0 {
            blocks.push(BlockToStream { start: starter, length: leftover_chunk_size_bytes });
        }

        println!("File size is {} which means {} segments of chosen size {} MiB + leftover {}",
                 total_size_bytes.file_size(options::BINARY).unwrap(),
                 full_chunks,
                 config.segment_size_mibs,
                 leftover_chunk_size_bytes.file_size(options::BINARY).unwrap());
    }


    //println!("Thread pool is set up to operate with {} executions in parallel",threads);


    let total_started = Instant::now();

    let connection_tracker = Arc::new(ConnectionTracker::new());

    {
        let dns_started = Instant::now();

        let mut dns_rt = Builder::new()
            .enable_all()
            .threaded_scheduler()
            .build()
            .unwrap();

        for round in 0..config.dns_rounds {
            let mut dns_futures1 = Vec::new();

            for _c in 0..config.dns_concurrent {
                dns_futures1.push(populate_a_dns(&connection_tracker, &config));
            }

            dns_rt.block_on(join_all(dns_futures1));

            if connection_tracker.ips.lock().unwrap().len() < config.connections {
                println!("Didn't find enough distinct S3 endpoints in round {} so trying again", round+1);
                sleep(config.dns_round_delay);
            } else {
                break;
            }
        }

        let dns_duration = Instant::now().duration_since(dns_started);

        let ips_db = connection_tracker.ips.lock().unwrap();

        println!("Discovered {} distinct S3 endpoints in {}s",
                 ips_db.len(), dns_duration.as_secs_f32());
    }

    let transfer_started = Instant::now();

    if !config.asynchronous {
        println!("Starting a threaded synchronous copy");

        sync_execute(&connection_tracker, &blocks, &config);

        let ips = connection_tracker.ips.lock().unwrap();

        for ip in ips.iter() {
            println!("Ending with {} used {} times", ip.0, ip.1);
        }

    } else {
        println!("Starting a tokio asynchronous copy");

        // a tokio runtime we will use for our async io
        let mut rt = if config.basic {
            Builder::new()
                .enable_all()
                .basic_scheduler()
                .build()
                .unwrap()
        } else {
            Builder::new()
                .enable_all()
                .threaded_scheduler()
                .core_threads(1)
                .max_threads(2)
                .build()
                .unwrap()
        };

        let ips_db = connection_tracker.ips.lock().unwrap();

        let mut futures = Vec::new();

        {
            let mut starter: usize = 0;

            let connectors = if config.connections < ips_db.len() { config.connections } else { ips_db.len() };


            let connection_chunk = blocks.len() / connectors;

            for (count, (ip, _)) in ips_db.iter().enumerate() {
                if count >= connectors {
                    break;
                }

                futures.push(async_execute(&ip.as_str(), &blocks[starter..starter + connection_chunk], &config));

                starter += connection_chunk;
            }
        }

        println!("Using {} connections to S3", futures.len());

        // spawn a task waiting on all the async streams to finish
        rt.block_on(join_all(futures));

        rt.shutdown_timeout(Duration::from_millis(100));
    }

    let transfer_duration = Instant::now().duration_since(transfer_started);

    println!("{}: rate MiB/sec = {} (copied {} bytes in {}s)",
             "Overall",
             (total_size_bytes as f32 / (1024.0 * 1024.0)) / transfer_duration.as_secs_f32(),
             total_size_bytes,
             transfer_duration.as_secs_f32());

    let total_duration = Instant::now().duration_since(total_started);

    println!("Total exec time was {}s", total_duration.as_secs_f32());

    Ok(())
}

// we need to start by working out how large the actual file is before segmenting
fn head_size_from_s3(s3_bucket: &str, s3_key: &str, s3_region: &str) -> Result<u64, &'static str> {
    let src = format!("https://{}.s3-{}.amazonaws.com{}", s3_bucket, s3_region, s3_key);
    let headresp = ureq::head(src.as_str())
        //.set("X-My-Header", "Secret")
        .call();

    let size = headresp.header("content-length").unwrap();

    Ok(size.parse::<u64>().unwrap())
}


/*
ssm-user@ip-172-31-8-71:~$ curl -s http://169.254.169.254/latest/dynamic/instance-identity/document
{
  "accountId" : "667213777749",
  "architecture" : "x86_64",
  "availabilityZone" : "us-west-2c",
  "billingProducts" : null,
  "devpayProductCodes" : null,
  "marketplaceProductCodes" : null,
  "imageId" : "ami-003634241a8fcdec0",
  "instanceId" : "i-08f9a00855fb6a2c0",
  "instanceType" : "m5dn.8xlarge",
  "kernelId" : null,
  "pendingTime" : "2020-05-31T03:37:57Z",
  "privateIp" : "172.31.8.71",
  "ramdiskId" : null,
  "region" : "us-west-2",
  "version" : "2017-09-30"
}
 */