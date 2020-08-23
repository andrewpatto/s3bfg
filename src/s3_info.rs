use anyhow::Result;
use rusoto_core::{HttpClient, Region};
use std::io;
use std::str::FromStr;

use rusoto_credential::{AwsCredentials, StaticProvider};
use rusoto_s3::{GetBucketLocationRequest, HeadObjectRequest, S3Client, S3};

use crate::config::Config;

#[derive(Debug, Copy, Clone)]
pub struct S3ObjectBlock {
    pub start: u64,

    pub length: u64,

    pub part_number: u32,
}

#[derive(Debug)]
pub struct S3ObjectDetails {
    // the region of the bucket the object is in
    pub region: Region,

    // the bucket name
    pub bucket: String,

    // the object key
    pub key: String,

    // the total size of the object in bytes
    pub size_in_bytes: u64,

    // the etag from a HEAD of the entire object
    pub etag: String,

    // the number of the last part of this object, or zero if this object has no parts
    // note that S3 counts part from 1 -> last_part_number(inclusive) so this is not traditional
    // zero indexing in some of our loops
    last_part_number: u32,

    // if the object has parts, this is the size of each part
    part_size_in_bytes: u64,

    // if the object has parts, this is the size of the last part
    // NOTE: this could be the same as the part size
    last_part_size_in_bytes: u64,
}

impl S3ObjectDetails {
    pub fn has_parts(&self) -> bool {
        self.last_part_number > 0
    }

    /// From all the details of the S3 object break the object up into units of work
    /// depending on whether we want to split along S3 part number boundaries or some given
    /// block size.
    ///
    pub fn break_into_blocks(&self, forced_block_size: Option<u64>) -> Vec<S3ObjectBlock> {
        let mut blocks = vec![];

        let mut starter: u64 = 0;

        // the file has parts and we have not been asked to override that
        if self.has_parts() && forced_block_size.is_none() {
            // note the inclusive range because S3 parts are not zero indexed
            for part_number in 1..=self.last_part_number {
                blocks.push(S3ObjectBlock {
                    start: starter,
                    length: if part_number == self.last_part_number {
                        self.last_part_size_in_bytes as u64
                    } else {
                        self.part_size_in_bytes
                    },
                    part_number,
                });
                starter += self.part_size_in_bytes;
            }

            return blocks;
        }

        // either the file is not made up of parts, or the user has asked us to ignore
        // the part.. either way we are going to need to chose a block size
        let block_size = forced_block_size.unwrap_or(8 * 1024 * 1024);

        let full_blocks_count = self.size_in_bytes / block_size;
        let leftover_bytes = self.size_in_bytes % block_size;

        for _x in 0..full_blocks_count {
            blocks.push(S3ObjectBlock {
                start: starter,
                length: block_size,
                part_number: 0,
            });
            starter += block_size;
        }

        if leftover_bytes > 0 {
            blocks.push(S3ObjectBlock {
                start: starter,
                length: leftover_bytes,
                part_number: 0,
            });
        }

        blocks
    }
}

/// Returns the concrete details of an actual S3 object.
///
/// Uses a couple of API calls, sometimes 3 - given we are going to be downloading
/// large files, there has not been too much attention paid to optimising this early stage
/// as it will all be dwarfed by the later transfer..
pub async fn find_s3_object(
    provider: &StaticProvider,
    bucket: &str,
    key: &str,
) -> Result<S3ObjectDetails, anyhow::Error> {
    // start by locating the region of the bucket
    // (there is a possibility of some optimisation here by guessing the bucket and doing
    //  the first HEAD on the assumption it is in the _current_region.. however
    //  Rusoto currently has an issue with the 301 redirect we then get - so this is
    //  probably safer for the moment albeit has an extra round trip)
    let location_of_bucket =
        find_s3_bucket_region_using_get_bucket_location(provider, bucket).await?;

    // we now make a client in the same region as the bucket
    let s3_client: S3Client = S3Client::new_with(
        HttpClient::new().expect("failed to create request dispatcher"),
        provider.clone(),
        location_of_bucket.clone(),
    );

    // start with a head of the full object
    let head_full_request = HeadObjectRequest {
        bucket: bucket.to_string(),
        key: key.to_string(),
        ..Default::default()
    };

    let head_full_result = s3_client.head_object(head_full_request).await?;

    let full_size = head_full_result.content_length.unwrap() as u64;

    // println!("{:?}", head_full_result);

    // then a head asking for the first part
    let head_part_request = HeadObjectRequest {
        bucket: bucket.to_string(),
        key: key.to_string(),
        part_number: Option::from(1),
        ..Default::default()
    };

    let head_part_result = s3_client.head_object(head_part_request).await?;

    if head_part_result.parts_count.is_some() {
        let from_parts =
            head_part_result.content_length.unwrap() * head_part_result.parts_count.unwrap();
        let leftover = from_parts - full_size as i64;

        Ok(S3ObjectDetails {
            region: location_of_bucket,
            bucket: bucket.to_string(),
            key: key.to_string(),
            etag: head_full_result.e_tag.unwrap(),
            size_in_bytes: full_size,
            last_part_number: head_part_result.parts_count.unwrap() as u32,
            part_size_in_bytes: head_part_result.content_length.unwrap() as u64,
            last_part_size_in_bytes: leftover as u64,
        })
    } else {
        Ok(S3ObjectDetails {
            region: location_of_bucket,
            bucket: bucket.to_string(),
            key: key.to_string(),
            etag: head_full_result.e_tag.unwrap(),
            size_in_bytes: full_size,
            // this file does not have parts so fetching by parts is not available to us
            last_part_number: 0,
            part_size_in_bytes: 0,
            last_part_size_in_bytes: 0,
        })
    }
}

/// Find the location of a bucket using the AWS API for this purpose.
/// Unfortunately, the AWS API call requires extra permissions over that
/// which is necessary for a plain GetObject - so this is not ideal.
/// Even worse, the GetBucketLocation permission can only be granted
/// to the owner of a bucket, so cross account use of this is not possible,
/// even if the object we are trying to get *does* allow cross account
/// access.
///
async fn find_s3_bucket_region_using_get_bucket_location(
    provider: &StaticProvider,
    bucket: &str,
) -> anyhow::Result<Region, anyhow::Error> {
    // a client in any region can determine a bucket location
    let s3_client: S3Client = S3Client::new_with(
        HttpClient::new().expect("failed to create request dispatcher"),
        provider.clone(),
        Region::default(),
    );

    let location_request = s3_client
        .get_bucket_location(GetBucketLocationRequest {
            bucket: bucket.to_string(),
            ..Default::default()
        })
        .await?;

    // a bucket in us-east-1 does not come back as a string but instead as a None location_constraint
    // so we need to cope with converting that into a real region
    Ok(Region::from_str(
        location_request
            .location_constraint
            .unwrap_or(String::from("us-east-1"))
            .as_str(),
    )
    .unwrap())
}

/*
Unknown(
    BufferedHttpResponse {status: 301, body: "", headers: {"x-amz-bucket-region": "ap-southeast-2", "x-amz-request-id": "E717CA57AB42529D", "x-amz-id-2": "60+j/RZWZFsSIOSDIw0n+osotfzUjTxS98AqAfIbiq/hfPwtS84iwfplmr/Wn+gbUTVx9w1Ozd8=", "content-type": "application/xml", "transfer-encoding": "chunked", "date": "Sun, 19 Jul 2020 04:38:28 GMT", "server": "AmazonS3"} },
)Based on AWS HEAD request we now believe the S3 bucket is in ap-southeast-2
HeadObjectOutput {
    accept_ranges: Some(
        "bytes",
    ),
    cache_control: None,
    content_disposition: None,
    content_encoding: None,
    content_language: None,
    content_length: Some(
        1073741824,
    ),
    content_type: Some(
        "binary/octet-stream",
    ),
    delete_marker: None,
    e_tag: Some(
        "\"06ea442348b0ad54fd23c0995839db52-128\"",
    ),
    expiration: None,
    expires: None,
    last_modified: Some(
        "Sun, 12 Jul 2020 05:27:49 GMT",
    ),
    metadata: Some(
        {},
    ),
    missing_meta: None,
    object_lock_legal_hold_status: None,
    object_lock_mode: None,
    object_lock_retain_until_date: None,
    parts_count: None,
    replication_status: None,
    request_charged: None,
    restore: None,
    sse_customer_algorithm: None,
    sse_customer_key_md5: None,
    ssekms_key_id: None,
    server_side_encryption: None,
    storage_class: None,
    version_id: Some(
        "Z1TLqMTIddg6dNIoBvY7efXiRCJEc4Cu",
    ),
    website_redirect_location: None,
}
 */
