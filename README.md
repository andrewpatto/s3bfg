# s3bfg

> The 'big gun' of S3 file transferring

## Mission statement

A utility to transfer a single large (think many GB) file from S3 as fast as possible

### Primary use cases (definitely in scope)

- Large file
- Single file
- To local disk from S3
- Optimising for actual AWS environment i.e machines in a VPC
- Command line options for benchmarking

### To be done (possibly in scope)

- Upload from local disk to S3
- Cross region detection/warnings
- Exotic AWS S3 modes (KMS, user pays etc)

### Out of scope

- Multiple files
- Sources or destinations other than S3
- S3 compatible services that aren't actually in AWS

## Usage

### Download

```shell script
USAGE:
    s3bfg down [FLAGS] [OPTIONS] <s3> <local>

ARGS:
    <s3>       The S3 location (s3 uri)
    <local>    The local path to write to (or /dev/null to mean perform a network only benchmark)

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
        --not-ec2    If specified tells us that we are definitely not on an EC2 instance and we should not attempt to
                     use EC2 tricks

OPTIONS:
        --profile <profile>                          An AWS profile to assume

        --connections <connections>
            Sets the number of connections to S3 to stream simultaneously [default: 16]

        --async-core-threads <async-core-threads>
            Sets the number of core threads in the Tokio runtime, default is for Tokio to detect core count

        --async-max-threads <async-max-threads>      Sets the number of max threads in the Tokio runtime, default is 512

        --dns-concurrent <dns-concurrent>
            Sets the number of concurrent attempts that will be made to obtain S3 bucket IP addresses in each DNS round
            [default: 16]
        --dns-round-delay <dns-round-delay>          Sets the number of milliseconds between DNS rounds [default: 500]
        --dns-rounds <dns-rounds>
            Sets the maxmimum number of rounds of DNS lookups that will be performed looking for distinct S3 bucket IP
            addresses [default: 8]
        --dns-server <dns-server>
            Sets the DNS resolver to directly query to find S3 bucket IP addresses, defaults to Google or AWS depending
            on detected location
        --segment-size <segment-size>
            Sets the size in mebibytes of each independently streamed part of the file - multiples of 8 will generally
            match S3 part sizing
        --sync-threads <sync-threads>
            Sets the number of threads in the Rayon thread pool for synchronous gets, default is 0 to tell Rayon to
            detect core count [default: 0]
```

### Upload

TBD
