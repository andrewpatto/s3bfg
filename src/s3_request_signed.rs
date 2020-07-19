use rusoto_core::signature::SignedRequest;
use rusoto_core::Region;
use rusoto_credential::AwsCredentials;

use std::error::Error;

use std::io::prelude::*;

use std::str;
use std::str::from_utf8;

/// Create a signed HTTP request for the given range of the given S3 object
/// and put the request raw data into 'request_to_be_written'.
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
    write!(request_packet, "{}: {}\n", "user-agent", "s3bfg")?;
    write!(request_packet, "\n")?;

    // we need to know what actual hostname we used in order to set up SSL correctly
    Ok(aws_request.hostname())
}
