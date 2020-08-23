use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use metrics_core::{Builder as MetricsBuilder, Drain, Observe};
use rusoto_credential::StaticProvider;

use s3bfg::asynchronous_download::download_s3_file;
use s3bfg::config::Config;
use s3bfg::empty_file::create_empty_target_file;
use s3bfg::metric_observer_ui::UiBuilder;
use s3bfg::s3_info::{find_s3_object, S3ObjectDetails};
use s3bfg::s3_ip_pool::S3IpPool;
use s3bfg::setup_aws_credentials::fetch_credentials;
use s3bfg::setup_metrics::create_metrics;
use s3bfg::setup_tokio::create_runtime;
use s3bfg::ui_console::progress_worker;

/// The big gun of S3 file copying.
///
fn main() -> std::io::Result<()> {
    // parse cmd line
    let config = Config::new();

    // we use a metrics engine to help drive optimisations and progress meters etc
    // the intention of this particular receiver is to record metrics across
    // the entire run of the transfer (and not merely of a small time window)
    let (receiver, _metrics_level) = create_metrics(&config);

    // we use tokio runtime for various async activity
    let (mut rt, rt_msg) = create_runtime(&config);

    // a single set of credentials which we are assuming will last throughout the whole copy
    let (creds, creds_msg) = rt.block_on(fetch_credentials(&config));

    let cred_provider = StaticProvider::new(
        creds.aws_access_key_id().to_string(),
        creds.aws_secret_access_key().to_string(),
        creds.token().clone(),
        None,
    );

    println!("{}", creds_msg);

    // try to find details of the s3 bucket and file
    let s3_object_details = rt
        .block_on(find_s3_object(
            &cred_provider,
            &config.input_bucket_name,
            &config.input_bucket_key,
        ))
        .unwrap();

    if config.memory_only {
        println!(
            "Copying s3://{}/{} ({}) to /dev/null (network benchmark only)",
            config.input_bucket_name,
            config.input_bucket_key,
            s3_object_details.region.name()
        );
    } else {
        println!(
            "Copying s3://{}/{} ({}) to {} (local)",
            config.input_bucket_name,
            config.input_bucket_key,
            s3_object_details.region.name(),
            config.output_write_filename.as_ref().unwrap().display()
        );
    }

    println!("{:?}", s3_object_details);

    println!("Running on: {}", config.instance_type);
    println!("DNS server chosen: {}", config.dns_server);
    println!(
        "Aiming for {} distinct concurrent connections to S3",
        config.s3_connections
    );

    if !config.memory_only {
        create_empty_target_file(
            &config.output_write_filename.clone().unwrap().as_ref(),
            s3_object_details.size_in_bytes,
        )?;
    }

    let blocks = s3_object_details.break_into_blocks(None);

    let total_started = Instant::now();

    let s3_ip_pool = Arc::new(S3IpPool::new());

    {
        let dns_started = Instant::now();

        rt.block_on(s3_ip_pool.populate_ips(
            &s3_object_details.region,
            config.dns_server.as_str(),
            config.dns_desired_ips,
            config.dns_rounds,
            config.dns_concurrent,
            config.dns_round_delay,
        ));

        let ips_db = s3_ip_pool.ips.lock().unwrap();

        println!(
            "Discovered {} distinct S3 endpoints in {}s",
            ips_db.len(),
            Instant::now().duration_since(dns_started).as_secs_f32()
        );
    }

    // start a regular (non tokio runtime) thread which displays a progress meter off our metrics
    {
        let controller = receiver.controller();
        let file_size_bytes = s3_object_details.size_in_bytes;

        std::thread::spawn(move || {
            progress_worker(controller, file_size_bytes);
        });
    }

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
        &s3_object_details.region,
    ));

    println!();

    rt.shutdown_timeout(Duration::from_millis(100));

    let mut observer = UiBuilder::new().build();

    receiver.controller().observe(&mut observer);

    println!("{}", observer.drain());

    let total_duration = Instant::now().duration_since(total_started);

    println!(
        "{}: rate MiB/sec = {} (copied {} bytes in {}s)",
        "Overall",
        (s3_object_details.size_in_bytes as f32 / (1024.0 * 1024.0)) / total_duration.as_secs_f32(),
        s3_object_details.size_in_bytes,
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
