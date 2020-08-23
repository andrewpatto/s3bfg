use metrics_runtime::{Receiver, Sink};
use rusoto_core::region::Region::{ApSoutheast2, UsEast1};
use rusoto_core::Region;
use rusoto_credential::{AwsCredentials, ChainProvider, ProvideAwsCredentials};
use sha1::{Digest, Sha1};
use std::net::{SocketAddr, ToSocketAddrs};
use tempfile::NamedTempFile;
use tokio::fs;
use tokio::time::Duration;

// aws s3api get-object --bucket broad-references --key hg19/v0/Homo_sapiens_assembly19.fasta --part-number 374 homosapien374
// {
//     "AcceptRanges": "bytes",
//     "LastModified": "2018-11-06T23:37:53+00:00",
//     "ContentLength": 8388608,
//     "ETag": "\"be7cb527b5c2914c35cb1bf513d63a61-375\"",
//     "ContentRange": "bytes 3128950784-3137339391/3140756381",
//     "ContentType": "binary/octet-stream",
//     "Metadata": {},
//     "PartsCount": 375
// }
//

#[tokio::test]
#[cfg_attr(not(feature = "test_aws_with_credentials"), ignore)]
async fn test_byte_range_access_in_us_east_1() {
    let (mut sink, creds, addr) = setup(&UsEast1).await;

    let file = NamedTempFile::new().unwrap();
    let path = file.into_temp_path();

    let r = s3bfg::download_block::download_block_work(
        5,
        &mut sink,
        &creds,
        addr,
        &rusoto_core::Region::UsEast1,
        "broad-references",
        "hg19/v0/Homo_sapiens_assembly19.fasta",
        0,
        16384,
        0,
        false,
        Some(path.to_owned()),
        0,
    )
    .await
    .unwrap();

    // slot passed in should be the slot returned
    assert_eq!(5, r);

    // read back the file
    let contents = fs::read(path).await.unwrap();

    // sha it
    let hash = Sha1::digest(contents.as_slice());

    // independently verified by downloading file and doing
    // head -c 16384 Homo_sapiens_assembly19.fasta | sha1sum
    assert_eq!(
        format!("{:x}", hash),
        "4965d586706a2f242b9875c876df7cd3c6e29cd7"
    );
}

#[tokio::test]
#[cfg_attr(not(feature = "test_aws_with_credentials"), ignore)]
async fn test_part_number_access_in_us_east_1() {
    let (mut sink, creds, addr) = setup(&UsEast1).await;

    let file = NamedTempFile::new().unwrap();
    let path = file.into_temp_path();

    // aws s3api get-object --bucket broad-references --key hg19/v0/Homo_sapiens_assembly19.fasta --part-number 375 homosapien375
    // {
    //     "AcceptRanges": "bytes",
    //     "LastModified": "2018-11-06T23:37:53+00:00",
    //     "ContentLength": 3416989,
    //     "ETag": "\"be7cb527b5c2914c35cb1bf513d63a61-375\"",
    //     "ContentRange": "bytes 3137339392-3140756380/3140756381",
    //     "ContentType": "binary/octet-stream",
    //     "Metadata": {},
    //     "PartsCount": 375
    // }

    let r = s3bfg::download_block::download_block_work(
        1,
        &mut sink,
        &creds,
        addr,
        &rusoto_core::Region::UsEast1,
        "broad-references",
        "hg19/v0/Homo_sapiens_assembly19.fasta",
        0,
        3416989,
        375,
        false,
        Some(path.to_owned()),
        0,
    )
    .await
    .unwrap();

    assert_eq!(1, r);

    let contents = fs::read(path).await.unwrap();

    let hash = Sha1::digest(contents.as_slice());

    // sha1sum homosapien375 = 5f65dbe0bc0e46f11393e773c73c34fa5f73e57d
    // verified independently
    assert_eq!(
        format!("{:x}", hash),
        "5f65dbe0bc0e46f11393e773c73c34fa5f73e57d"
    );
}

/// Do the basic setup to perform S3 testing in a particular region.
///
async fn setup(region: &Region) -> (Sink, AwsCredentials, SocketAddr) {
    // we need a receiver so we can make sinks - but we don't particulary care about how it
    // is set up
    let receiver = Receiver::builder()
        .histogram(Duration::from_secs(2), Duration::from_secs(1))
        .build()
        .expect("failed to create receiver");

    let creds = ChainProvider::new().credentials().await.unwrap();

    // we need the socket address for an S3 server in the given region
    let server_details = format!("test.s3.{}.amazonaws.com:443", region.name());
    let server: Vec<_> = server_details
        .to_socket_addrs()
        .expect("Unable to resolve domain")
        .collect();

    (receiver.sink(), creds, server[0])
}
