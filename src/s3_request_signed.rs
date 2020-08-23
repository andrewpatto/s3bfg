use rusoto_core::signature::{Params, SignedRequest};
use rusoto_core::Region;
use rusoto_credential::AwsCredentials;

use std::error::Error;

use std::io::prelude::*;

use std::str;
use std::str::from_utf8;

/// Create a signed HTTP request for the given range of the given S3 object
/// and store the request raw data into 'request_packet'.
///
pub fn make_signed_get_range_request(
    credentials: &AwsCredentials,
    bucket_region: &Region,
    bucket_name: &str,
    bucket_key: &str,
    read_start: u64,
    read_length: u64,
    request_packet: &mut Vec<u8>,
) -> Result<String, Box<dyn Error>> {
    // sets up the standard rusoto signed request for S3 GET
    // (though we are about to use it in a non-standard way)
    let mut aws_request = SignedRequest::new(
        "GET",
        "s3",
        bucket_region,
        format!("/{}/{}", bucket_name, bucket_key).as_str(),
    );

    aws_request.set_hostname(Option::from(format!(
        "s3.{}.amazonaws.com",
        bucket_region.name()
    )));
    aws_request.add_header("Accept", "*/*");
    aws_request.add_header(
        "Range",
        format!("bytes={}-{}", read_start, read_start + read_length - 1).as_str(),
    );

    aws_request.sign(credentials);

    // write into the given buffer so we can send it one operation later
    write!(request_packet, "GET {} HTTP/1.1\n", aws_request.path())?;
    for (k, v) in aws_request.headers() {
        write!(
            request_packet,
            "{}: {}\n",
            k,
            from_utf8(v[0].as_ref()).unwrap()
        )?;
    }
    // whilst this may be too pessimistic - for the moment it guarantees our read() will
    // only get data for our request
    write!(request_packet, "{}: {}\n", "connection", "close")?;
    // write!(request_packet, "{}: {}\n", "user-agent", built_info::PKG_NAME)?;
    write!(request_packet, "\n")?;

    // we need to know what actual hostname we used in order to set up SSL correctly
    Ok(aws_request.hostname())
}

/// Create a signed HTTP request for the given part of the given S3 object
/// and store the request raw data into 'request_packet'.
///
pub fn make_signed_get_part_request(
    credentials: &AwsCredentials,
    bucket_region: &Region,
    bucket_name: &str,
    bucket_key: &str,
    read_part_number: u32,
    request_packet: &mut Vec<u8>,
) -> Result<String, Box<dyn Error>> {
    // sets up the standard rusoto signed request for S3 GET of a part
    // (though we are about to use it in a non-standard way)
    let mut aws_request = SignedRequest::new(
        "GET",
        "s3",
        bucket_region,
        format!("/{}/{}", bucket_name, bucket_key).as_str(),
    );

    aws_request.add_param("partNumber".to_string(), format!("{}", read_part_number));

    aws_request.set_hostname(Option::from(format!(
        "s3.{}.amazonaws.com",
        bucket_region.name()
    )));
    aws_request.add_header("Accept", "*/*");

    aws_request.sign(credentials);

    // write into the given buffer so we can send it one operation later
    write!(
        request_packet,
        "GET {}?{} HTTP/1.1\n",
        aws_request.path(),
        aws_request.canonical_query_string()
    )?;
    for (k, v) in aws_request.headers() {
        write!(
            request_packet,
            "{}: {}\n",
            k,
            from_utf8(v[0].as_ref()).unwrap()
        )?;
    }
    // whilst this may be too pessimistic - for the moment it guarantees our read() will
    // only get data for our request
    write!(request_packet, "{}: {}\n", "connection", "close")?;
    // write!(request_packet, "{}: {}\n", "user-agent", built_info::PKG_NAME)?;
    write!(request_packet, "\n")?;

    // we need to know what actual hostname we used in order to set up SSL correctly
    Ok(aws_request.hostname())
}

#[cfg(test)]
mod tests {
    use crate::s3_request_signed::make_signed_get_part_request;
    use chrono::{DateTime, Datelike, Timelike, Utc};
    use rusoto_credential::AwsCredentials;
    use std::str;

    #[test]
    fn part_number_request_constructed() {
        let mut http_request: Vec<u8> = Vec::with_capacity(1024);
        let key = "AKIAVAMWBWU2Z7GTJIZX";

        // note: credentials are realistic but not actually real!
        let _r = make_signed_get_part_request(
            &AwsCredentials::new(key, "aisXA534Tdfrwm12pppwWWWQ7v6D", None, None),
            &rusoto_core::Region::ApSoutheast2,
            "mybucket",
            "myfolder/myfile.txt",
            22,
            &mut http_request,
        )
        .unwrap();

        let now: DateTime<Utc> = Utc::now();

        let http_printable = str::from_utf8(http_request.as_slice()).unwrap();

        // println!("{}", http_printable);

        let http_lines: Vec<&str> = http_printable.split('\n').collect();

        assert_eq!(http_lines.len(), 11);

        assert_eq!(
            "GET /mybucket/myfolder/myfile.txt?partNumber=22 HTTP/1.1",
            http_lines[0]
        );
        assert_eq!("accept: */*", http_lines[1]);

        // the auth line is consistent via the static creds, but the time of issue does change..
        // note: we are not comparing the full auth string - only up to the signature..
        let auth = format!("authorization: AWS4-HMAC-SHA256 Credential={}/{}{:02}{:02}/ap-southeast-2/s3/aws4_request, SignedHeaders=accept;content-type;host;x-amz-content-sha256;x-amz-date, Signature=", key, now.year(), now.month(), now.day());

        assert!(http_lines[2].starts_with(&auth));
        assert_eq!("content-length: 0", http_lines[3]);
        assert_eq!("content-type: application/octet-stream", http_lines[4]);
        assert_eq!("host: s3.ap-southeast-2.amazonaws.com", http_lines[5]);
        // this is the standard sha256 hash of zero content so this is stable
        assert_eq!("x-amz-content-sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", http_lines[6]);
        assert!(http_lines[7].starts_with("x-amz-date: "));
        assert_eq!("connection: close", http_lines[8]);
        assert_eq!("", http_lines[9]);
        assert_eq!("", http_lines[10]);
    }
}
