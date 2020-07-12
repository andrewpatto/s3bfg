use std::error::Error;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;
use std::io::{BufReader, Cursor};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::str;
use std::str::from_utf8;
use crate::config::Config;
use rusoto_core::signature::SignedRequest;
use rusoto_core::Region;
use rusoto_credential::{AwsCredentials, DefaultCredentialsProvider, ProvideAwsCredentials};
use std::convert::TryInto;
use regex::Regex;

pub fn make_signed_get_range_request(
    read_start: u64,
    read_length: u64,
    cfg: &Config,
    credentials: &AwsCredentials,
    bucket_region: &Region,
    request_to_be_written: &mut Vec<u8>
) -> Result<String, Box<dyn Error>> {

    // sets up the standard rusoto signed request for S3 GET
    // (though we are about to use it in a non-standard way)
    let mut aws_request = SignedRequest::new(
        "GET",
        "s3",
        bucket_region,
        format!("/{}/{}", cfg.input_bucket_name, cfg.input_bucket_key).as_str(),
    );

    aws_request.set_hostname(Option::from(format!(
        "s3-{}.amazonaws.com",
        bucket_region.name()
    )));
    aws_request.add_header("Accept", "*/*");
    aws_request.add_header(
        "Range",
        format!("bytes={}-{}", read_start, read_start + read_length - 1).as_str(),
    );

    aws_request.sign(credentials);

    // write into the given buffer so we can send it one operation later
    write!(request_to_be_written, "GET {} HTTP/1.1\n", aws_request.path())?;
    for (k, v) in aws_request.headers() {
        write!(request_to_be_written, "{}: {}\n", k, from_utf8(v[0].as_ref()).unwrap())?;
    }
    // whilst this may be too pessimistic - for the moment it guarantees our read() will
    // only get data for our request
    write!(request_to_be_written, "{}: {}\n", "connection", "close")?;
    write!(request_to_be_written, "{}: {}\n", "user-agent", "s3bfg")?;
    write!(request_to_be_written, "\n")?;

    // we need to know what actual hostname we used in order to set up SSL correctly
    Ok(aws_request.hostname())
}
