extern crate nix;
#[macro_use]
extern crate simple_error;
#[macro_use]
extern crate lazy_static;



use std::convert::TryInto;

use std::sync::Arc;

use std::time::Duration;
use std::time::Instant;

use humansize::{file_size_opts as options, FileSize};
use metrics_core::{Builder as MetricsBuilder, Drain, Observe};
use metrics_runtime::Receiver;

use crate::asynchronous_download::download_s3_file;
use crate::config::Config;
use crate::datatype::BlockToStream;
use crate::empty_file::create_empty_target_file;
use crate::metric_observer_ui::UiBuilder;
use crate::s3_ip_pool::S3IpPool;
use crate::s3_size::find_file_size_and_correct_region;
use crate::setup_aws_credentials::fetch_credentials;
use crate::setup_tokio::create_runtime;
use crate::synchronous::sync_execute;
use crate::ui_console::progress_worker;

mod asynchronous_download;
mod config;
mod copy_exact;
mod datatype;
mod empty_file;
mod metric_names;
mod metric_observer_progress;
mod metric_observer_ui;
mod s3_ip_pool;
mod s3_request_signed;
mod s3_size;
mod setup_aws_credentials;
mod setup_tokio;
mod synchronous;
mod ui_console;

/// The big gun of S3 file copying.
///
fn main() -> std::io::Result<()> {
    // parse cmd line
    let mut config = Config::new();

    // we use a metrics engine to help drive optimisations and progress meters etc
    // the intention is that this particular receiver is to record metrics across
    // the entire run of the transfer (and not merely of a small time window)
    let receiver = Receiver::builder()
        // 2 hrs worth of room for stats!
        .histogram(Duration::from_secs(2 * 60 * 60), Duration::from_secs(60))
        .build()
        .expect("failed to create receiver");

    // we use tokio runtime for various async activity
    let (mut rt, rt_msg) = create_runtime(&config);

    // a single set of credentials which we are assuming will last throughout the whole copy
    let (creds, _creds_msg) = rt.block_on(fetch_credentials(&config));

    // try to find details of the s3 bucket and file
    let (total_size_bytes, bucket_region) = rt
        .block_on(find_file_size_and_correct_region(&config, &creds))
        .unwrap();

    config.file_size_bytes = total_size_bytes;

    if config.memory_only {
        println!(
            "Copying s3://{}/{} ({}) to /dev/null (network benchmark only)",
            config.input_bucket_name,
            config.input_bucket_key,
            bucket_region.name()
        );
    } else {
        println!(
            "Copying s3://{}/{} ({}) to {} (local)",
            config.input_bucket_name,
            config.input_bucket_key,
            bucket_region.name(),
            config.output_write_filename.as_ref().unwrap()
        );
    }

    println!("Running on: {}", config.instance_type);
    println!("DNS server chosen: {}", config.dns_server);
    println!(
        "Aiming for {} distinct concurrent connections to S3",
        config.s3_connections
    );

    if !config.memory_only {
        create_empty_target_file(
            &config.output_write_filename.clone().unwrap().as_ref(),
            total_size_bytes.try_into().unwrap(),
        )?;
    }

    let mut blocks = vec![];

    // construct our units of 'copy' activity that we want to do.. whilst this is a pretty simple
    // splitting up of a file we could potentially do something more sophisticated
    {
        let mut starter: u64 = 0;
        let full_chunks = total_size_bytes / config.segment_size_bytes;
        let leftover_chunk_size_bytes = total_size_bytes % config.segment_size_bytes;

        for _x in 0..full_chunks {
            blocks.push(BlockToStream {
                start: starter,
                length: config.segment_size_bytes,
            });
            starter += config.segment_size_bytes;
        }

        if leftover_chunk_size_bytes > 0 {
            blocks.push(BlockToStream {
                start: starter,
                length: leftover_chunk_size_bytes,
            });
        }

        println!(
            "File size is {} which means {} segments of chosen size {} MiB + leftover {}",
            total_size_bytes.file_size(options::BINARY).unwrap(),
            full_chunks,
            config.segment_size_mibs,
            leftover_chunk_size_bytes
                .file_size(options::BINARY)
                .unwrap()
        );
    }

    let total_started = Instant::now();

    let s3_ip_pool = Arc::new(S3IpPool::new());

    {
        let dns_started = Instant::now();

        rt.block_on(s3_ip_pool.populate_ips(
            &bucket_region,
            config.dns_server.as_str(),
            config.dns_desired_ips,
            config.dns_rounds,
            config.dns_concurrent,
            config.dns_round_delay,
        ));

        let dns_duration = Instant::now().duration_since(dns_started);

        let ips_db = s3_ip_pool.ips.lock().unwrap();

        println!(
            "Discovered {} distinct S3 endpoints in {}s",
            ips_db.len(),
            dns_duration.as_secs_f32()
        );
    }

    //AwsCredentials::default();

    //{

    //}

    let controller = receiver.controller();
    let file_size_bytes = config.file_size_bytes;

    // start a regular (non tokio runtime) thread which displays a progress meter off our metrics
    std::thread::spawn(move || {
        progress_worker(controller, file_size_bytes);
    });

    if config.synchronous {
        sync_execute(&s3_ip_pool, &blocks, &config, &creds, &bucket_region);
    } else {
        println!(
            "Tokio runtime is set up to operate with {} config, utilising {} S3 connections",
            rt_msg, config.s3_connections
        );

        rt.block_on(download_s3_file(
            &receiver,
            &s3_ip_pool,
            blocks,
            &config,
            &creds,
            &bucket_region,
        ));

        println!();

        rt.shutdown_timeout(Duration::from_millis(100));

        let mut observer = UiBuilder::new().build();

        receiver.controller().observe(&mut observer);

        println!("{}", observer.drain());
    }

    let total_duration = Instant::now().duration_since(total_started);

    println!(
        "{}: rate MiB/sec = {} (copied {} bytes in {}s)",
        "Overall",
        (total_size_bytes as f32 / (1024.0 * 1024.0)) / total_duration.as_secs_f32(),
        total_size_bytes,
        total_duration.as_secs_f32()
    );

    Ok(())
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
