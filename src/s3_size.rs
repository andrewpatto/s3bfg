use std::io;
use std::str::FromStr;

use regex::Regex;
use rusoto_core::{HttpClient, Region};

use rusoto_credential::{AwsCredentials, StaticProvider};
use rusoto_s3::{HeadObjectRequest, S3Client, S3};

use crate::config::Config;

pub struct S3ObjectDetails {
    pub size_in_bytes: u64,

    pub region: Region,

    pub number_of_parts: u32,

    pub part_size_in_bytes: u64,

    pub head_etag: String,
}

/// Returns the size in bytes and real region of the S3 file that has been specified in `cfg`.
///
/// Uses the standard S3 HEAD or GET object operation (at this point we are not yet
/// optimising for speed)
pub async fn find_file_size_and_correct_region(
    cfg: &Config,
    creds: &AwsCredentials,
) -> Result<(u64, Region), io::Error> {
    // we start with a guess at the region of the S3 bucket and refine as we discover more
    let mut region_attempt = Region::default();

    println!(
        "Starting on the assumption the S3 bucket is in region {}",
        region_attempt.name()
    );

    loop {
        let s3_client: S3Client = S3Client::new_with(
            HttpClient::new().expect("failed to create request dispatcher"),
            StaticProvider::new(
                creds.aws_access_key_id().to_string(),
                creds.aws_secret_access_key().to_string(),
                creds.token().clone(),
                None,
            ),
            region_attempt.clone(),
        );

        let head_request = HeadObjectRequest {
            bucket: cfg.input_bucket_name.clone(),
            key: cfg.input_bucket_key.clone(),
            ..Default::default()
        };

        let head_result = s3_client.head_object(head_request).await;

        // rustoto does not correctly follow/deal with 301 errors when a bucket is in a different region
        // so we have to parse out of the error message the new region
        // I presume this will be fixed at some point and we can then rewrite
        if head_result.is_err() {
            let raw_head_error_result = format!("{:#?}", head_result.unwrap_err());
            // print!("{}", raw_head_error_result);

            let re_region =
                Regex::new(r##""x-amz-bucket-region": "(?P<region>[a-z0-9-]+)""##).unwrap();
            let re_status = Regex::new(r##"status: (?P<status>[0-9]+)"##).unwrap();

            let caps_region = re_region
                .captures(raw_head_error_result.as_str())
                .unwrap_or_else(|| {
                    let caps_status = re_status.captures(raw_head_error_result.as_str());

                    if caps_status.is_some() {
                        println!(
                            "Couldn't access S3 source due to status {}",
                            caps_status.unwrap().name("status").unwrap().as_str()
                        );
                    } else {
                        println!(
                            "Couldn't find S3 source with unknown error {}",
                            raw_head_error_result
                        );
                    }

                    std::process::exit(1);
                });

            let region_from_re = caps_region.name("region").unwrap();

            region_attempt = Region::from_str(region_from_re.as_str()).unwrap();

            println!(
                "Based on AWS HEAD request we now believe the S3 bucket is in {}",
                region_attempt.name()
            );

            continue;
        }

        let head_result_real = head_result.unwrap();

        // print!("{:#?}", head_result_real);

        let s = head_result_real.content_length.unwrap() as u64;

        return Ok((s, region_attempt.clone()));
    }

    /*let cred_provider =  DefaultCredentialsProvider::new().unwrap();

        {
            let options = PreSignedRequestOption {
                expires_in: Duration::from_secs(60 * 30),
            };
            let presigned_multipart_put = part_req2.get_presigned_url(region, credentials, &options);
            println!("presigned multipart put: {:#?}", presigned_multipart_put);
            let client = reqwest::Client::new();
            let res = client
                .put(&presigned_multipart_put)
                .body(String::from("foo"))
                .send()
                .await
                .expect("Multipart put with presigned url failed");
            assert_eq!(res.status(), http::StatusCode::OK);
            let e_tag = res.headers().get("ETAG").unwrap().to_str().unwrap();
            completed_parts.push(CompletedPart {
                e_tag: Some(e_tag.to_string()),
                part_number: Some(part_req2.part_number),
            });
        }

        pub trait PreSignedRequest {
        /// http://docs.aws.amazon.com/AmazonS3/latest/API/sigv4-query-string-auth.html
        fn get_presigned_url(
            &self,
            region: &Region,
            credentials: &AwsCredentials,
            option: &PreSignedRequestOption,
        ) -> String;
    }

    impl PreSignedRequest for GetObjectRequest {
        /// https://docs.aws.amazon.com/AmazonS3/latest/API/RESTObjectGET.html
        fn get_presigned_url(
            &self,
            region: &Region,
            credentials: &AwsCredentials,
            option: &PreSignedRequestOption,
        ) -> String {
            let request_uri = format!("/{bucket}/{key}", bucket = self.bucket, key = self.key);
            let mut request = SignedRequest::new("GET", "s3", &region, &request_uri);
            let mut params = Params::new();

            add_headers!(
                self, request;
                range, "Range";
                if_modified_since, "If-Modified-Since";
                if_unmodified_since, "If-Unmodified-Since";
                if_match, "If-Match";
                if_none_match, "If-None-Match";
                sse_customer_algorithm, "x-amz-server-side-encryption-customer-algorithm";
                sse_customer_key, "x-amz-server-side-encryption-customer-key";
                sse_customer_key_md5, "x-amz-server-side-encryption-customer-key-MD5";
            );

            add_params!(
                self, params;
                part_number, "partNumber";
                response_content_type, "response-content-type";
                response_content_language, "response-content-language";
                response_expires, "response-expires";
                response_cache_control, "response-cache-control";
                response_content_disposition, "response-content-disposition";
                response_content_encoding, "response-content-encoding";
                version_id, "versionId";
            );

            request.set_params(params);
            request.generate_presigned_url(credentials, &option.expires_in, false)
        }
    }
     impl PreSignedRequest for UploadPartRequest {
        /// https://docs.aws.amazon.com/AmazonS3/latest/API/mpUploadUploadPart.html
        fn get_presigned_url(
            &self,
            region: &Region,
            credentials: &AwsCredentials,
            option: &PreSignedRequestOption,
        ) -> String {
            let request_uri = format!("/{bucket}/{key}", bucket = self.bucket, key = self.key);
            let mut request = SignedRequest::new("PUT", "s3", &region, &request_uri);

            request.add_param("partNumber", &self.part_number.to_string());
            request.add_param("uploadId", &self.upload_id);

            add_headers!(
                self, request;
                content_length, "Content-Length";
                content_md5, "Content-MD5";
                sse_customer_algorithm, "x-amz-server-side-encryption-customer-algorithm";
                sse_customer_key, "x-amz-server-side-encryption-customer-key";
                sse_customer_key_md5, "x-amz-server-side-encryption-customer-key-MD5";
                request_payer, "x-amz-request-payer";
            );

            request.generate_presigned_url(credentials, &option.expires_in, false)
        }
    }
        */
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