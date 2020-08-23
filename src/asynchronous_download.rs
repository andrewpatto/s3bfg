use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::str;
use std::sync::Arc;

use futures::stream::FuturesUnordered;
use metrics_runtime::{Receiver, Sink};
use regex::Regex;
use rusoto_core::Region;
use rusoto_credential::AwsCredentials;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio_rustls::*;

use crate::config::Config;
use crate::copy_exact::copy_exact;
use crate::download_block::download_block_work;
use crate::s3_info::S3ObjectBlock;
use crate::s3_ip_pool::S3IpPool;

/// Asynchronously transfer a file from S3 using multiple connections each
/// independently fetching blocks or parts of the file.
///
pub async fn download_s3_file(
    receiver: &Receiver,
    s3_ip_pool: &Arc<S3IpPool>,
    blocks: Vec<S3ObjectBlock>,
    config: &Config,
    credentials: &AwsCredentials,
    bucket_region: &Region,
) {
    let mut slot_sockets = vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 443);
        config.s3_connections as usize
    ];

    // We will do some error tracking against connections this way
    // let mut slot_errors = vec![0u16; config.s3_connections as usize];

    // from our pool of S3 ip addresses we create slots that will target each of them
    // up to the number of concurrent connections that have been asked for
    // (note: possibly using the same S3 IP address more than once - as it turns out this doesn't matter)
    for slot in 0..config.s3_connections as usize {
        let (tcp_addr, _tcp_count) = s3_ip_pool.use_least_used_ip();

        slot_sockets[slot] = SocketAddr::new(IpAddr::from(tcp_addr), 443);
    }

    let mut futs = FuturesUnordered::new();

    // the current slot indicates which S3 connection slot we are making units of work for
    let mut current_slot: usize = 0;

    for b in blocks {
        // before creating our async closure we create local variables that are copies
        // of any params - such that our spawned tokio task can own them forever
        // TODO: what is the idiomatic rust way of doing this??
        let local_credentials = credentials.clone();
        let local_s3_addr = slot_sockets[current_slot];
        let local_s3_bucket_region = bucket_region.clone();
        let local_s3_bucket_name = config.input_bucket_name.clone();
        let local_s3_bucket_key = config.input_bucket_key.clone();
        let local_memory_only = config.memory_only;
        let local_output_filename = config.output_write_filename.clone();

        // construct a sink for any metrics
        let mut block_sink = receiver.sink();

        // create the worker to work in this slot and spawn it on any tokio runtime thread
        futs.push(tokio::spawn(async move {
            let slot = current_slot;

            let actual_work_future = download_block_work(
                slot,
                &mut block_sink,
                &local_credentials,
                local_s3_addr,
                &local_s3_bucket_region,
                local_s3_bucket_name.as_str(),
                local_s3_bucket_key.as_str(),
                b.start,
                b.length,
                b.part_number,
                local_memory_only,
                local_output_filename,
                b.start,
            );

            // TODO: handle errors here - and start to disable slots if too many errors
            // rather than have out futures loop deal with errors we want to put
            // some error handling here - currently none
            actual_work_future.await.unwrap();

            // we need to return the slot *we* were in order that the next
            // worker that is created takes over our slot
            slot
        }));

        // this is only of relevance in the opening N iterations of the loop -
        // after the future we 'wait' on will define the replacement current_slot
        current_slot += 1;

        if futs.len() == config.s3_connections as usize {
            // we have hit the limit of concurrency we are aiming for
            // so we now await the finish of (any!) worker
            // the slot it returns is then open for us to use as the next worker slot
            current_slot = futures::stream::StreamExt::next(&mut futs)
                .await
                .unwrap()
                .unwrap();
        }
    }

    // drain for remaining work from the queue
    while let Some(_) = futures::stream::StreamExt::next(&mut futs).await {}
}
