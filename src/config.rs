use clap::{self, Arg, App};
use std::path::Path;
use std::time::Duration;

/// Stores information entered by the user and derived from the environment
/// for this particular run of the tool.
///
pub struct Config {
    pub input_bucket_name: String,
    pub input_bucket_key: String,
    pub input_bucket_region: String,

    pub dns_server: String,
    pub dns_concurrent: usize,
    pub dns_rounds: usize,
    pub dns_round_delay: Duration,

    pub memory_only: bool,
    pub output_write_filename: String,

    pub connections: usize,

    pub threads: usize,
    pub segment_size_mibs: u64,
    pub segment_size_bytes: u64,

    pub fallocate: bool,
    pub basic: bool,
    pub asynchronous: bool,

    pub instance_type: String
}

const S3_REGION_ARG: &str = "s3-region";

impl Config {
    pub fn new() -> Config {
        let num_cpus = num_cpus::get();

        let matches = App::new("s3bigfile")
            .version("1.0")
            .author("AP")
            .about("Copies S3 files real quick")

            // the only mandatory elements are the bucket and file path within
            .arg(Arg::with_name("INPUTBUCKET")
                .about("The S3 bucket name of the input file")
                .required(true)
                .index(1))
            .arg(Arg::with_name("INPUTKEY")
                .about("The S3 key of the input file")
                .required(true)
                .index(2))

            .arg(Arg::with_name(S3_REGION_ARG)
                .long(S3_REGION_ARG)
                .about("The S3 region of the input bucket, defaults to the current region when running in AWS")
                .takes_value(true))

            // control what we do with the file when we get in from S3
            .arg(Arg::with_name("memory")
                .long("memory")
                .about("If specified tells us to just transfer the data to memory and not then write it out to disk"))
            .arg(Arg::with_name("output-file")
                .long("output-file")
                .about("Sets the output file to write to, defaults to a file with the same basename as S3 in the current directory")
                .takes_value(true))
            .arg(Arg::with_name("fallocate")
                .long("fallocate")
                .about("If specified tells us to create the blank destination file using fallocate()"))

            .arg(Arg::with_name("segment-size")
                .long("segment-size")
                .about("Sets the size in mebibytes of each independently streamed part of the file - multiples of 8 will generally match S3 part sizing")
                .takes_value(true))

            .arg(Arg::with_name("expected-mibs")
                .long("expected-mibs")
                .about("Sets the expected MiB/s network bandwidth available to this process, which will then auto compute other settings to maximise performance")
                .default_value("1024")
                .takes_value(true))

            .arg(Arg::with_name("connections")
                .long("connections")
                .about("Sets the number of connections to S3 to use to execute the streaming gets")
                .default_value("10")
                .takes_value(true))

            .arg(Arg::with_name("threads")
                .long("threads")
                .about("Sets the number of threads to use to execute the streaming gets, default is detected core count")
                .default_value(num_cpus.to_string().as_str())
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

                aws_instance_type = json["instanceType"].to_string();
                region = json["region"].to_string();

                // running in AWS means we have a more sensible default DNS server - but we
                // only want to use if one wasn't explicitly given on the command line
                if !matches.is_present("dns-server") {
                    dns_server = String::from("169.254.169.253:53");
                }
            }
        }

        if region.is_empty() {
            region = String::from(matches.value_of("s3-region").unwrap_or_else(|| {
                println!("If not running in AWS you need to specify the region of the bucket");
                std::process::exit(1);
            }));
        }

        let in_key = String::from(matches.value_of("INPUTKEY").unwrap());
        let out_current_dir = Path::new(&in_key).file_name().unwrap().to_str().unwrap();

        Config {
            input_bucket_name: String::from(matches.value_of("INPUTBUCKET").unwrap()),
            input_bucket_key: in_key.clone(),
            input_bucket_region: region,

            // DNS settings
            dns_server,
            dns_concurrent: matches.value_of_t::<usize>("dns-concurrent").unwrap_or(24),
            dns_rounds: matches.value_of_t::<usize>("dns-rounds").unwrap(),
            dns_round_delay: Duration::from_millis(matches.value_of_t::<u64>("dns-round-delay").unwrap()),

            output_write_filename: String::from(matches.value_of("output-file").unwrap_or(out_current_dir)),
            memory_only: matches.is_present("memory"),

            connections: matches.value_of_t::<usize>("connections").unwrap(),

            threads:  matches.value_of_t::<usize>("threads").unwrap_or(num_cpus),
            segment_size_mibs: matches.value_of_t::<u64>("segment-size").unwrap_or(8),
            segment_size_bytes: matches.value_of_t::<u64>("segment-size").unwrap_or(8) * 1024 * 1024,

            fallocate: matches.is_present("fallocate"),
            basic: matches.is_present("basic"),
            asynchronous: matches.is_present("async"),

            instance_type: aws_instance_type
        }
    }
}
