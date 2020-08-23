# s3bfg

> The 'big gun' of S3 file transferring

## Mission statement

A utility to transfer a single multi gigabyte file from S3 as fast as possible

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

The invocation of `s3bfg` mirrors as much as possible the equivalent `aws s3 cp` command.
So basically

`s3bfg <source> <destination>`

For the moment `source` must be a URI representing an object in S3. `destination` must
be a local file path (uploading is feature on the roadmap).

Where it makes sense we will try to use equivalent command line switches to `aws s3`.

The most important new command line switch is

`--connections <numconnections>`

which controls the number of independent TCP streams which will be created for the download.
The default value for this setting is `16` which should give more than enough connections
to hit 1GiB+ bandwidth (should your network and disk allow that).

Another new command line switch which may give better performance is

`--block-size <blocksizemibs>`

which controls the basic block size of the object gets. This value is measured in MiB and
should probably be a multiple of 8. The default for `s3bfg` is `64` which we have found
gives reasonable results out of the box.

### Download files from S3

```shell script
s3bfg s3://3kricegenome/MANIFEST ./RICEMANIFEST
```

```shell script
s3bfg https://3kricegenome.s3.amazonaws.com/MANIFEST ./RICEMANIFEST2
```

### Download files for network benchmarking

If the local file destination is `/dev/null` then `s3bfg` will operate in a mode that purely
performs a network download of the S3 file, but does not attempt to write the resulting
data out to disk. This can be useful for benchmarking the network performance of AWS instances.




### Upload


## Testing

There a some basic unit tests - with the intention to definitely add some more!

Integration testing with AWS is quite hard because there needs to be a real AWS
account making the API calls, however inexpensive the calls are. For this reason
integration tests needing a real AWS account are split out into a feature
called `test_aws_with_credentials`.

These tests can be invoked using something like the following

```shell script
AWS_PROFILE=myprofilename cargo test --features test_aws_with_credentials
```

## Acknowledgements

* Rusoto team
* Melbourne Genomics team
* Daniel Vassallo who got us thinking about this via his s3 benchmark project (https://github.com/dvassallo/s3-benchmark)