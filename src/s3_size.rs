use std::any::Any;
use std::convert::TryInto;
use std::error::Error;
use std::fmt::Display;
use std::io;
use std::str::FromStr;
use std::time::Duration;

use regex::Regex;
use rusoto_core::{HttpClient, Region};
use rusoto_core::credential::{AutoRefreshingProvider, ChainProvider, ProfileProvider, ProvideAwsCredentials};
use rusoto_s3::{GetObjectError, GetObjectRequest, HeadObjectError, HeadObjectRequest, S3, S3Client};
use rusoto_s3::util::{PreSignedRequest, PreSignedRequestOption};
use rusoto_sts::{StsAssumeRoleSessionCredentialsProvider, StsClient};
use tokio::runtime::Builder;

use crate::config::Config;

pub async fn find_file_size_and_correct_region(cfg: &Config) -> Result<(i64, Region), io::Error> {

    // we start with a guess at the region of the S3 bucket and refine as we discover
    // more
    let mut region_attempt = Region::default();

    println!("Starting on the assumption the S3 bucket is in region {}", region_attempt.name());

    loop {
        let s3_client: S3Client;

        if cfg.aws_profile.is_some() {
            println!("Using profile to obtain credentials");

            // let sts = StsClient::new(Region::ApSoutheast2);

            let mut profile_provider = ProfileProvider::new().unwrap();

            profile_provider.set_profile(cfg.aws_profile.as_ref().unwrap());

            s3_client = S3Client::new_with(
                HttpClient::new().expect("failed to create request dispatcher"),
                profile_provider,
            region_attempt.clone()
            );

            /*let assume_role_provider = StsAssumeRoleSessionCredentialsProvider::new(
                sts,
                "arn:aws:iam::something:role/something".to_owned(),
                cfg.aws_profile.clone().unwrap(),
                None, None, None, None
            );

            let auto_refreshing_provider = AutoRefreshingProvider::new(assume_role_provider); */
        }
        else {
            println!("Using default to obtain credentials");

            let mut chain_provider = ChainProvider::new();

            // out expectation is to be running in AWS so this is plenty of time for it to
            // get any EC2 role credentials
            chain_provider.set_timeout(Duration::from_millis(500));

            s3_client = S3Client::new_with(
                HttpClient::new().expect("failed to create request dispatcher"),
                chain_provider,
                region_attempt.clone(),
            );
        }

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

            let re = Regex::new(r##""x-amz-bucket-region": "(?P<region>[a-z0-9-]+)""##).unwrap();

            let caps = re.captures(raw_head_error_result.as_str());

            region_attempt = Region::from_str(caps.unwrap().name("region").unwrap().as_str()).unwrap();

            println!("Based on AWS HEAD request we now believe the S3 bucket is in {}", region_attempt.name());

            continue;
        }

        return Ok((head_result.unwrap().content_length.unwrap(), region_attempt.clone()));
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

