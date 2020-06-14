extern crate nix;

use std::convert::TryInto;
use std::str;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Instant;
use futures::future::join_all;
use humansize::{file_size_opts as options, FileSize};
use tokio::runtime::{Builder};

use crate::asynchronous::async_execute;
use crate::config::Config;
use crate::datatype::{BlockToStream, ConnectionTracker};
use crate::empty_file::create_empty_target_file;
use crate::ips::populate_a_dns;
use crate::synchronous::sync_execute;
use crate::s3_size::find_file_size_and_correct_region;

mod datatype;
mod asynchronous;
mod synchronous;
mod ips;
mod copy_exact;
mod config;
mod empty_file;
mod s3_size;

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
        println!("Copying file s3://{}/{} to /dev/null (pure network->memory benchmark)", config.input_bucket_name, config.input_bucket_key);
    } else {
        println!("Copying file s3://{}/{} to {}", config.input_bucket_name, config.input_bucket_key, config.output_write_filename.as_ref().unwrap());
    }

    println!("Running on: {}", config.instance_type);
    println!("DNS server chosen: {}", config.dns_server);
    println!("Aiming for {} distinct concurrent connections to S3", config.s3_connections);

    let (total_size_bytes, bucket_region) = find_source_size_and_region(&config);

    if !config.memory_only {
        create_empty_target_file(&config.output_write_filename.clone().unwrap().as_ref(), total_size_bytes.try_into().unwrap())?;
    }

    return Ok(());

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
            let mut dns_futures = Vec::new();


            for _c in 0..config.dns_concurrent {
                dns_futures.push(populate_a_dns(&connection_tracker, &config, bucket_region.as_ref()));
            }

            let res = dns_rt.block_on(join_all(dns_futures));

            if connection_tracker.ips.lock().unwrap().len() < config.s3_connections {
                println!("Didn't find enough distinct S3 endpoints (currently {}) in round {} so trying again (btw futures result was {:?})", connection_tracker.ips.lock().unwrap().len(), round+1, res);
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
        sync_execute(&connection_tracker, &blocks, &config, bucket_region.as_ref());

    } else {
        async_execute(&connection_tracker, &blocks, &config, bucket_region.as_ref());

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

///
///
fn find_source_size_and_region(config: &Config) -> (u64, String) {
    let mut head_rt = Builder::new()
        .enable_all()
        .threaded_scheduler()
        .build()
        .unwrap();

    let head_result = head_rt.block_on(find_file_size_and_correct_region(&config)).unwrap();

    return (head_result.0, head_result.1.name().to_string())
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