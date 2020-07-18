extern crate nix;
#[macro_use]
extern crate simple_error;

use std::convert::TryInto;
use std::str;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Instant;
use std::{thread, time::Duration};

use futures::future::join_all;
use humansize::{file_size_opts as options, FileSize};
use metrics_core::{Builder as MetricsBuilder, Drain, Observe};
use metrics_runtime::Receiver;
use rusoto_credential::{
    AwsCredentials, ChainProvider, DefaultCredentialsProvider, ProfileProvider,
    ProvideAwsCredentials,
};

use crate::asynchronous::async_execute_transfer;
use crate::config::Config;
use crate::datatype::BlockToStream;
use crate::empty_file::create_empty_target_file;
use crate::metric_observer_ui::UiBuilder;
use crate::s3_ip_pool::S3IpPool;
use crate::s3_size::find_file_size_and_correct_region;
use crate::setup_tokio::create_runtime;
use crate::synchronous::sync_execute;

mod asynchronous;
mod config;
mod copy_exact;
mod datatype;
mod empty_file;
mod ips;
mod metric_names;
mod metric_observer_progress;
mod metric_observer_ui;
mod s3_ip_pool;
mod s3_request_signed;
mod s3_size;
mod setup_tokio;
mod synchronous;

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

    // try to find details of the s3 bucket and file
    let (total_size_bytes, bucket_region) = rt
        .block_on(find_file_size_and_correct_region(&config))
        .unwrap();

    config.file_size_bytes = total_size_bytes;

    if config.memory_only {
        println!(
            "Copying file s3://{}/{} ({}) to /dev/null (network->memory benchmark only)",
            config.input_bucket_name, config.input_bucket_key, bucket_region.name()
        );
    } else {
        println!(
            "Copying file s3://{}/{} ({}) to {}",
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

    // return Ok(());

    let mut blocks = vec![];

    // construct our units of 'copy' activity that we want to do.. whilst this is pretty simple we
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

    let connection_tracker = Arc::new(S3IpPool::new());

    let mut creds: AwsCredentials = AwsCredentials::default();

    {
        let dns_started = Instant::now();

        connection_tracker.populate_ips(
            &bucket_region,
            config.dns_server.as_str(),
            config.dns_desired_ips,
            config.dns_rounds,
            config.dns_concurrent,
            config.dns_round_delay,
        );

        let dns_duration = Instant::now().duration_since(dns_started);

        let ips_db = connection_tracker.ips.lock().unwrap();

        println!(
            "Discovered {} distinct S3 endpoints in {}s",
            ips_db.len(),
            dns_duration.as_secs_f32()
        );

        if config.aws_profile.is_some() {
            let profile_name = config.aws_profile.as_ref().unwrap();

            rt.block_on(async {
                let mut pp = ProfileProvider::new().unwrap();
                pp.set_profile(profile_name);
                let cp = ChainProvider::with_profile_provider(pp);

                creds = cp.credentials().await.unwrap();
                println!(
                    "Got AWS credentials {:?} using profile {}",
                    creds, profile_name
                );
            });
        } else {
            rt.block_on(async {
                creds = DefaultCredentialsProvider::new()
                    .unwrap()
                    .credentials()
                    .await
                    .unwrap();
                println!("Got AWS credentials {:?} using default provider", creds);
            });
        }
    }

    let transfer_started = Instant::now();

    if config.synchronous {
        sync_execute(
            &connection_tracker,
            &blocks,
            &config,
            &creds,
            &bucket_region,
        );
    } else {
        println!(
            "Tokio runtime is set up to operate with {} config, utilising {} S3 connections",
            rt_msg, config.s3_connections
        );

        rt.block_on(async_execute_transfer(
            &receiver,
            &connection_tracker,
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

    let transfer_duration = Instant::now().duration_since(transfer_started);

    println!(
        "{}: rate MiB/sec = {} (copied {} bytes in {}s)",
        "Overall",
        (total_size_bytes as f32 / (1024.0 * 1024.0)) / transfer_duration.as_secs_f32(),
        total_size_bytes,
        transfer_duration.as_secs_f32()
    );

    let total_duration = Instant::now().duration_since(total_started);

    println!("Total exec time was {}s", total_duration.as_secs_f32());

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
