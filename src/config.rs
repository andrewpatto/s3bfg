use clap::{self, App, AppSettings, Arg, ArgMatches};

use crate::built_info;
use crate::s3_uris::is_s3_uri;
use regex::Regex;
use std::fs::metadata;
use std::path::{Path, PathBuf};
use std::time::Duration;

// constants that are used both as the 'long' command line args AND as the argument key in CLAP

const SOURCE_ARG: &str = "source";
const DESTINATION_ARG: &str = "destination";

const BLOCK_SIZE_ARG: &str = "block-size";

const PROFILE_ARG: &str = "profile";
const CONNECTIONS_ARG: &str = "connections";
const S3_REGION_ARG: &str = "s3-region";
const FALLOCATE_ARG: &str = "fallocate";
const ASYNC_CORE_THREADS_ARG: &str = "tokio-core-threads";
const ASYNC_MAX_THREADS_ARG: &str = "tokio-max-threads";
const ASYNC_USE_BASIC_ARG: &str = "tokio-use-basic";
const DNS_DESIRED_IPS_ARG: &str = "dns-desired-ips";
const DNS_SERVER_ARG: &str = "dns-server";
const NOT_EC2_ARG: &str = "not-ec2";

// https://aws.amazon.com/s3/faqs/
// Q: How much data can I store in Amazon S3?
//
// The total volume of data and number of objects you can store are unlimited. Individual
// Amazon S3 objects can range in size from a minimum of 0 bytes to a maximum of 5 terabytes.
// The largest object that can be uploaded in a single PUT is 5 gigabytes. For objects
// larger than 100 megabytes, customers should consider using the Multipart Upload capability.
const AWS_LIMIT_MAXIMUM_FILE_SIZE_BYTES: u64 = 5 * 1024 * 1024 * 1024 * 1024;
const AWS_LIMIT_MAXIMUM_BLOCK_SIZE_BYTES: u64 = 5 * 1024 * 1024 * 1024;

const AWS_INSTANCE_IDENTITY_URL: &str =
    "http://169.254.169.254/latest/dynamic/instance-identity/document";
const AWS_INSTANCE_DNS: &str = "169.254.169.253:53";

/// Stores information entered by the user and derived from the environment
/// for this particular run of the tool.
///
pub struct Config {
    // mandatory values of the S3 file that is the input or output source
    pub input_bucket_name: String,
    pub input_bucket_key: String,

    pub output_write_filename: Option<PathBuf>,
    pub memory_only: bool,

    pub aws_profile: Option<String>,

    pub dns_server: String,
    pub dns_desired_ips: Option<u16>,
    pub dns_concurrent: u16,
    pub dns_rounds: u16,
    pub dns_round_delay: Duration,

    pub s3_connections: u16,

    // settings for the asynchronous tokio runtime
    pub tokio_core_threads: u16,
    pub tokio_max_threads: u16,
    pub tokio_basic: bool,

    pub block_size_mibs: u64,

    pub network_buffer_size_kibs: u64,
    pub disk_buffer_size_kibs: u64,

    pub fallocate: bool,

    pub instance_type: String,
}

impl Config {
    pub fn new() -> Config {
        let matches = App::new(built_info::PKG_NAME)
            .version(built_info::PKG_VERSION)
            .author(built_info::PKG_AUTHORS)
            .about("The big gun of S3 file copying")

            .arg(Arg::with_name(SOURCE_ARG)
                .about("The S3 location (eg: s3://my-bucket/my-folder/my-file")
                .required(true)
                .index(1))

            .arg(Arg::with_name(DESTINATION_ARG)
                .about("The local path to write to (or /dev/null to run network only benchmark)")
                .required(true)
                .index(2))

            .arg(Arg::with_name(PROFILE_ARG)
                .long(PROFILE_ARG)
                .about("An AWS profile to use for credentials (note assume-role not supported yet)")
                .takes_value(true))

            .arg(Arg::with_name(CONNECTIONS_ARG)
                .long(CONNECTIONS_ARG)
                .about("Sets the number of connections to S3 to stream simultaneously")
                .default_value("16")
                .takes_value(true))

            .arg(Arg::with_name(BLOCK_SIZE_ARG)
                .long(BLOCK_SIZE_ARG)
                .about("Sets the size in mebibytes of each independently streamed block of the file, overriding the use of the files part size - multiples of 8 generally preferred")
                .default_value("64")
                .takes_value(true))


            .arg(Arg::with_name(FALLOCATE_ARG)
                .long(FALLOCATE_ARG)
                .about("If specified tells us to create the blank destination file using fallocate() on supported unix systems"))


            .arg(Arg::with_name(ASYNC_CORE_THREADS_ARG)
                .long(ASYNC_CORE_THREADS_ARG)
                .about("Sets the number of core threads in the Tokio runtime, default is for Tokio to detect core count")
                .takes_value(true))
            .arg(Arg::with_name(ASYNC_MAX_THREADS_ARG)
                .long(ASYNC_MAX_THREADS_ARG)
                .about("Sets the number of max threads in the Tokio runtime, default is 512")
                .takes_value(true))


            .arg(Arg::with_name(DNS_DESIRED_IPS_ARG)
                .long(DNS_DESIRED_IPS_ARG)
                .about("Sets the number of different S3 IP addresses we will make the DNS try to obtain")
                .takes_value(true))
            .arg(Arg::with_name("dns-server")
                .long("dns-server")
                .about("Sets the DNS resolver to directly query to find S3 IP addresses, defaults to Google [8.8.8.8:53] or AWS [169.254.169.253:53] depending on detected location")
                .takes_value(true))


            .arg(Arg::with_name(ASYNC_USE_BASIC_ARG)
                .long(ASYNC_USE_BASIC_ARG)
                .about("If specified tells us to use basic tokio runtime rather than threaded"))
            .arg(Arg::with_name(NOT_EC2_ARG)
                .long(NOT_EC2_ARG)
                .about("If specified tells us that we are definitely not on an EC2 instance and we should not attempt to use EC2 tricks"))

            .get_matches();

        println!("Here");

        let mut dns_server: String = String::from("8.8.8.8:53");

        if matches.is_present("dns-server") {
            dns_server = String::from(matches.value_of("dns-server").unwrap());
        }

        let mut region: String = String::new();

        let mut aws_instance_type = String::from("not an AWS EC2 instance");

        // try to work out if we are running on an EC2 instance or not, and if so change the
        // defaults - we have a command line switch to disable this detection though
        let not_ec2 = matches.is_present(NOT_EC2_ARG);

        if !not_ec2 {
            // we *may* be running on an EC2 instance in which case we have a few tricks up our sleeve
            let resp = ureq::get(AWS_INSTANCE_IDENTITY_URL)
                .timeout_connect(500)
                .timeout_read(500)
                .call();

            if resp.status() == 200 {
                let json = resp.into_json().unwrap();

                aws_instance_type = String::from(json["instanceType"].as_str().unwrap());
                region = String::from(json["region"].as_str().unwrap());

                // running in AWS means we have a more sensible default DNS server - but we
                // only want to use if one wasn't explicitly given on the command line
                if !matches.is_present("dns-server") {
                    dns_server = String::from(AWS_INSTANCE_DNS);
                }
            }
        }

        let (in_bucket_name, in_key, out_filename, memory_only) = parse_in_out(&matches);

        return Config {
            input_bucket_name: in_bucket_name.to_string(),
            input_bucket_key: in_key.to_string(),

            output_write_filename: out_filename,

            //input_bucket_region: region,
            aws_profile: if matches.is_present(PROFILE_ARG) {
                Some(String::from(matches.value_of(PROFILE_ARG).unwrap()))
            } else {
                None
            },

            // DNS settings
            dns_server,
            // allow the user to specify how many desired S3 ips but default to
            // just use whatever is returned
            dns_desired_ips: if matches.occurrences_of(DNS_DESIRED_IPS_ARG) > 0 {
                Some(matches.value_of_t::<u16>(DNS_DESIRED_IPS_ARG).unwrap())
            } else {
                None
            },
            dns_concurrent: 16,
            dns_rounds: 4,
            dns_round_delay: Duration::from_millis(500),

            memory_only,

            s3_connections: matches.value_of_t::<u16>(CONNECTIONS_ARG).unwrap(),

            tokio_basic: matches.is_present(ASYNC_USE_BASIC_ARG),
            tokio_core_threads: matches
                .value_of_t::<u16>(ASYNC_CORE_THREADS_ARG)
                .unwrap_or(0),
            tokio_max_threads: matches
                .value_of_t::<u16>(ASYNC_MAX_THREADS_ARG)
                .unwrap_or(0),

            block_size_mibs: matches.value_of_t::<u64>(BLOCK_SIZE_ARG).unwrap(),

            network_buffer_size_kibs: 128,
            disk_buffer_size_kibs: 512,

            fallocate: matches.is_present(FALLOCATE_ARG),

            instance_type: aws_instance_type,
        };
    }
}

fn matches_s3_uri(arg: &str) -> Option<(String, String)> {
    let re =
        Regex::new(r##"s3://(?P<bucket>[a-z0-9][a-z0-9-\\.]{1,61}[a-z0-9])/(?P<key>.+)"##).unwrap();

    let caps_result = re.captures(arg);

    return if caps_result.is_some() {
        let caps = caps_result.unwrap();

        Some((
            String::from(caps.name("bucket").unwrap().as_str()),
            String::from(caps.name("key").unwrap().as_str()),
        ))
    } else {
        None
    };
}

fn parse_in_out(matches: &ArgMatches) -> (String, String, Option<PathBuf>, bool) {
    // if we notice we are asked to send to /dev/null we use that to put us in 'special'
    // memory only mode which skips the entire output IO (useful for network benchmarking)
    let mut memory_only = false;

    println!("{}", matches.value_of(SOURCE_ARG).unwrap());

    let source_s3 = is_s3_uri(matches.value_of(SOURCE_ARG).unwrap());
    let destination_s3 = is_s3_uri(matches.value_of(DESTINATION_ARG).unwrap());

    if source_s3.is_none() && destination_s3.is_none() {
        println!("One of the input or output arguments must be something we can recognise as a S3 location");
        std::process::exit(1);
    }

    let i = String::from(matches.value_of(SOURCE_ARG).unwrap());
    let o = Path::new(matches.value_of(DESTINATION_ARG).unwrap());

    let s3 = matches_s3_uri(i.as_str()).unwrap_or_else(|| {
        println!("Input must be an S3 path in the form s3://<bucket>/<key>");
        std::process::exit(1);
    });

    if o.is_absolute() && o.ends_with("null") && o.starts_with("/dev") {
        memory_only = true;
    }

    // the 'base' of the key is possibly going to be useful for us as a filename
    let key_as_filename = String::from(
        Path::new(s3.1.as_str())
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
    );

    // determine a suitable local path to write out the content from the info we have
    let mut local = PathBuf::new();

    // if the local file specified exists then we work out if it is a directory first
    // because that will change our behaviour
    let md_result = metadata(o);

    if md_result.is_ok() {
        // path specified exists and is a directory
        if md_result.unwrap().is_dir() {
            local.push(o);
            local.set_file_name(key_as_filename);
        } else {
            local.push(o);
        }
    } else {
        // path specified doesn't exist so we can just use as the file name
        local.push(o);
    }

    return (
        String::from(s3.0),
        String::from(s3.1.as_str()),
        Option::from(local),
        memory_only,
    );
}

/*
REGEXP = r'https://.+&bucket=(?P<bucket>.*)&prefix=(?P<prefix>.*)'
S3URI_FORMAT = 's3://{bucket}/{prefix}'

 */
