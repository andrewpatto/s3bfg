use clap::{self, Arg, App, ArgMatches, AppSettings};
use std::path::Path;
use std::time::Duration;
use regex::Regex;
use std::fs::metadata;

/// Stores information entered by the user and derived from the environment
/// for this particular run of the tool.
///
pub struct Config {
    // mandatory
    pub input_bucket_name: String,
    pub input_bucket_key: String,

    pub output_write_filename: Option<String>,
    pub memory_only: bool,

    pub aws_profile: Option<String>,

    pub dns_server: String,
    pub dns_concurrent: usize,
    pub dns_rounds: usize,
    pub dns_round_delay: Duration,


    pub s3_connections: usize,

    // settings for the synchronous Rayon thread pool
    pub synchronous_threads: usize,

    // settings for the asynchronous tokio runtime
    pub asynchronous_core_threads: usize,
    pub asynchronous_max_threads: usize,
    pub asynchronous_basic: bool,

    pub segment_size_mibs: u64,
    pub segment_size_bytes: u64,

    pub fallocate: bool,
    pub asynchronous: bool,

    pub instance_type: String,
}

const S3_REGION_ARG: &str = "s3-region";
const SYNC_THREADS_ARG: &str = "sync-threads";
const ASYNC_CORE_THREADS_ARG: &str = "async-core-threads";
const ASYNC_MAX_THREADS_ARG: &str = "async-max-threads";
const ASYNC_USE_BASIC_ARG: &str = "async-use-basic";

impl Config {
    pub fn new() -> Config {

        let matches = App::new("s3bfg")
            .version("1.0")
            .author("AP")
            .about("The big gun of S3 file copying")
            .setting(AppSettings::SubcommandRequiredElseHelp)

            .arg(Arg::with_name("profile")
                .long("profile")
                .about("An AWS profile to assume")
                .takes_value(true))

            .arg(Arg::with_name("connections")
                .long("connections")
                .about("Sets the number of connections to S3 to stream simultaneously")
                .default_value("10")
                .takes_value(true))


            .subcommand(App::new("down")
                .about("brings file down from S3")
                .arg(Arg::with_name("s3")
                    .about("The S3 location (s3 uri, s3 https)")
                    .required(true)
                    .index(1))
                .arg(Arg::with_name("local")
                    .about("The local path to write to or /dev/null to mean memory benchmark")
                    .required(true)
                    .index(2))
                .arg(Arg::with_name("fallocate")
                    .long("fallocate")
                    .about("If specified tells us to create the blank destination file using fallocate()"))
            )

            .subcommand(App::new("up")
                .about("sends file to S3")
                .arg(Arg::with_name("local")
                    .about("The local path to read")
                    .required(true)
                    .index(1))
                .arg(Arg::with_name("s3")
                    .about("The S3 destination (s3 uri, s3 https)")
                    .required(true)
                    .index(2))
            )




            .arg(Arg::with_name("segment-size")
                .long("segment-size")
                .about("Sets the size in mebibytes of each independently streamed part of the file - multiples of 8 will generally match S3 part sizing")
                .takes_value(true))

            .arg(Arg::with_name("expected-mibs")
                .long("expected-mibs")
                .about("Sets the expected MiB/s network bandwidth available to this process, which will then auto compute other settings to maximise performance")
                .default_value("1024")
                .takes_value(true))


            .arg(Arg::with_name(SYNC_THREADS_ARG)
                .long(SYNC_THREADS_ARG)
                .about("Sets the number of threads in the Rayon thread pool for synchronous gets, default is 0 to tell Rayon to detect core count")
                .takes_value(true))

            .arg(Arg::with_name(ASYNC_CORE_THREADS_ARG)
                .long(ASYNC_CORE_THREADS_ARG)
                .about("Sets the number of core threads in the Tokio runtime, default is for Tokio to detect core count")
                .takes_value(true))
            .arg(Arg::with_name(ASYNC_MAX_THREADS_ARG)
                .long(ASYNC_MAX_THREADS_ARG)
                .about("Sets the number of max threads in the Tokio runtime, default is 512")
                .takes_value(true))


            .arg(Arg::with_name("dns-server")
                .long("dns-server")
                .about("Sets the DNS resolver to directly query to find S3 bucket IP addresses, defaults to Google or AWS depending on detected location")
                .takes_value(true))
            .arg(Arg::with_name("dns-concurrent")
                .long("dns-concurrent")
                .about("Sets the number of concurrent attempts that will be made to obtain S3 bucket IP addresses in each DNS round")
                .default_value("32")
                .takes_value(true))
            .arg(Arg::with_name("dns-rounds")
                .long("dns-rounds")
                .default_value("16")
                .about("Sets the number of rounds of DNS lookups that will be performed looking for distinct S3 bucket IP addresses")
                .takes_value(true))
            .arg(Arg::with_name("dns-round-delay")
                .long("dns-round-delay")
                .default_value("250")
                .about("Sets the number of milliseconds between DNS rounds")
                .takes_value(true))

            .arg(Arg::with_name("basic")
                .long("basic")
                .about("If specified tells us to use basic tokio runtime rather than threaded"))
            .arg(Arg::with_name("async")
                .long("async")
                .about("If specified tells us to use async code rather than sync"))
            .arg(Arg::with_name("not-ec2")
                .long("not-ec2")
                .about("If specified tells us that we are definitely not on an EC2 instance and we should not attempt to use EC2 tricks"))

            .get_matches();

        let mut dns_server: String = String::from("8.8.8.8:53");

        if matches.is_present("dns-server") {
            dns_server = String::from(matches.value_of("dns-server").unwrap());
        }

        let mut region: String = String::new();

        let mut aws_instance_type = String::from("not an AWS EC2 instance");

        // try to work out if we are running on an EC2 instance or not, and if so change the
        // defaults - we have a command line switch to disable this detection though
        let not_ec2 = matches.is_present("not-ec2");

        if !not_ec2 {
            // we *may* be running on an EC2 instance in which case we have a few tricks up our sleeve
            let resp =
                ureq::get("http://169.254.169.254/latest/dynamic/instance-identity/document")
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
                    dns_server = String::from("169.254.169.253:53");
                }
            }
        }

        if let Some(sub_down) = matches.subcommand_matches("down") {
            let (in_bucket_name, in_key, out_filename, memory_only) = parse_in_out(&sub_down);

            return Config {
                input_bucket_name: in_bucket_name.to_string(),
                input_bucket_key: in_key.to_string(),
                output_write_filename: out_filename,

                //input_bucket_region: region,
                aws_profile: if matches.is_present("profile") { Some(String::from(matches.value_of("profile").unwrap())) } else { None } ,

                // DNS settings
                dns_server,
                dns_concurrent: matches.value_of_t::<usize>("dns-concurrent").unwrap_or(24),
                dns_rounds: matches.value_of_t::<usize>("dns-rounds").unwrap(),
                dns_round_delay: Duration::from_millis(matches.value_of_t::<u64>("dns-round-delay").unwrap()),

                memory_only: memory_only,

                s3_connections: matches.value_of_t::<usize>("connections").unwrap(),

                synchronous_threads: matches.value_of_t::<usize>(SYNC_THREADS_ARG).unwrap_or(0),

                asynchronous_basic: matches.is_present(ASYNC_USE_BASIC_ARG),
                asynchronous_core_threads: matches.value_of_t::<usize>(ASYNC_CORE_THREADS_ARG).unwrap_or(0),
                asynchronous_max_threads: matches.value_of_t::<usize>(ASYNC_MAX_THREADS_ARG).unwrap_or(0),

                segment_size_mibs: matches.value_of_t::<u64>("segment-size").unwrap_or(8),
                segment_size_bytes: matches.value_of_t::<u64>("segment-size").unwrap_or(8) * 1024 * 1024,

                fallocate: matches.is_present("fallocate"),
                asynchronous: matches.is_present("async"),

                instance_type: aws_instance_type,
            }
        }

        if let Some(sub_up) = matches.subcommand_matches("up") {
            println!("Up not implemented yet");
            std::process::exit(1);
        }

        println!("One of down or up must be chosen");
        std::process::exit(1);
    }
}

fn matches_s3_uri(arg: &str) -> Option<(String, String)> {
    // Rules for Bucket Naming
    // The following rules apply for naming S3 buckets:
    // Bucket names must be between 3 and 63 characters long.
    // Bucket names can consist only of lowercase letters, numbers, dots (.), and hyphens (-).
    // Bucket names must begin and end with a letter or number.
    // Bucket names must not be formatted as an IP address (for example, 192.168.5.4).
    // Bucket names can't begin with xn-- (for buckets created after February 2020).
    // Bucket names must be unique within a partition. A partition is a grouping of Regions. AWS currently has three partitions: aws (Standard Regions), aws-cn (China Regions), and aws-us-gov (AWS GovCloud [US] Regions).
    // Buckets used with Amazon S3 Transfer Acceleration can't have dots (.) in their names. For more information about transfer acceleration, see Amazon S3 Transfer Acceleration.
    // For best compatibility, we recommend that you avoid using dots (.) in bucket names, except for buckets that are used only for static website hosting. If you include dots in a bucket's name, you can't use virtual-host-style addressing over HTTPS, unless you perform your own certificate validation. This is because the security certificates used for virtual hosting of buckets don't work for buckets with dots in their names.
    // This limitation doesn't affect buckets used for static website hosting, because static website hosting is only available over HTTP. For more information about virtual-host-style addressing, see Virtual Hosting of Buckets. For more information about static website hosting, see Hosting a static website on Amazon S3.

    let re = Regex::new(r##"s3://(?P<bucket>[A-za-z0-9-]{3,63})/(?P<key>[A-Za-z0-9-/\\.]+)"##).unwrap();

    let caps_result = re.captures(arg);

    return if caps_result.is_some() {
        let caps = caps_result.unwrap();

        Some((String::from(caps.name("bucket").unwrap().as_str()),
              String::from(caps.name("key").unwrap().as_str())))
    } else {
        None
    }
}

fn matches_dev_null(arg: &str) -> bool {
    let re = Regex::new(r##"/dev/null"##).unwrap();

    return re.is_match(arg);
}

fn parse_in_out(matches: &ArgMatches) -> (String, String, Option<String>, bool) {

    // if we notice we are asked to send to /dev/null we use that to put us in 'special'
    // memory only mode which skips the entire output IO (useful for network benchmarking)
    let mut memory_only = false;

    let i = String::from(matches.value_of("s3").unwrap());
    let o = Path::new(matches.value_of("local").unwrap());

    let s3 = matches_s3_uri(i.as_str()).unwrap_or_else(|| {
        println!("Input must be an S3 path in the form s3://<bucket>/<key>");
        std::process::exit(1);
    });

    if o.is_absolute() && o.ends_with("null") && o.starts_with("/dev") {
        memory_only = true;
    }

    // the 'base' of the key is possibly going to be useful for us as a filename
    let key_as_filename = String::from(Path::new(s3.1.as_str()).file_name().unwrap().to_str().unwrap());

    // determine a suitable local path from the info we have
    let local: String;

    // if the local file specified exists then we work out if it is a directory first
    let md_result = metadata(o);

    if md_result.is_ok() {
        if md_result.unwrap().is_dir() {
            local = o.join(key_as_filename).to_str().unwrap().parse().unwrap();
        } else {
            local = String::from(o.to_str().unwrap());
        }
    } else {
        local = String::from(o.to_str().unwrap());
    }

    return (String::from(s3.0),
            String::from(s3.1.as_str()),
            Option::from(String::from(local)),
            memory_only);
}

/*
REGEXP = r'https://.+&bucket=(?P<bucket>.*)&prefix=(?P<prefix>.*)'
S3URI_FORMAT = 's3://{bucket}/{prefix}'

 */