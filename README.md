# s3bfg

> The 'big gun' of S3 file transferring

## Mission statement

A utility to transfer a single large (1 GB+) file from S3 as fast as possible

### Primary use cases (definitely in scope)

- Large files
- Single files
- Download to local disk from S3
- Optimising for actual AWS environment i.e machines in a VPC
- Command line options for benchmarking

### To be done (possibly in scope)

- Upload (not yet) from local disk to S3
- Cross region detection/warnings
- Exotic AWS S3 modes (KMS, user pays etc)

### Out of scope

- Multiple files
- Sources or destinations other than S3
- S3 compatible services that aren't actually in AWS
