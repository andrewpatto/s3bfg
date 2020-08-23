use regex::{Captures, Regex};
use rusoto_core::Region;
use std::str::FromStr;

/// Parses the given URI and determines if it is plausibly referring to an S3 object
/// and if so, returns the object bucket, key and potentially region.
/// Supports a variety of S3 formats
///   s3://bucket/key
///   https://bucket.s3.region.amazonaws.com/key
///   https://bucket.s3.amazonaws.com/key
///   https://s3.amazonaws.com/bucket/key
///   https://s3-region.amazonaws.com/bucket/key
///
pub fn is_s3_uri(uri: &str) -> Option<(String, String, Option<Region>)> {
    // this is our S3 uri format - which is not officially a URI format but which is supported
    // by most S3 tools including the AWS cli tools
    let s3_capture = S3_URI_REGEX.captures(uri);

    if s3_capture.is_some() {
        return Some(extract_bucket_key(s3_capture));
    }

    // this is the now AWS preferred format - which is virtual hosted style requests
    let virt_reg_capture = S3_VIRTUAL_STYLE_REGIONAL_URI_REGEX.captures(uri);

    if virt_reg_capture.is_some() {
        return Some(extract_bucket_key_region(virt_reg_capture));
    }

    let virt_glob_capture = S3_VIRTUAL_STYLE_GLOBAL_URI_REGEX.captures(uri);

    if virt_glob_capture.is_some() {
        return Some(extract_bucket_key(virt_glob_capture));
    }

    let path_reg_capture = S3_PATH_STYLE_REGIONAL_URI_REGEX.captures(uri);

    if path_reg_capture.is_some() {
        return Some(extract_bucket_key_region(path_reg_capture));
    }

    let path_glob_capture = S3_PATH_STYLE_GLOBAL_URI_REGEX.captures(uri);

    if path_glob_capture.is_some() {
        return Some(extract_bucket_key(path_glob_capture));
    }

    None
}

// Rules for Bucket Naming from AWS docs
// We have ticked the ones our regex enforces, others we should maybe get around to enforcing
// The following rules apply for naming S3 buckets:
// ✓ Bucket names must be between 3 and 63 characters long.
// ✓ Bucket names can consist only of lowercase letters, numbers, dots (.), and hyphens (-).
// ✓ Bucket names must begin and end with a letter or number.
// Bucket names must not be formatted as an IP address (for example, 192.168.5.4).
// Bucket names can't begin with xn-- (for buckets created after February 2020).
// Bucket names must be unique within a partition. A partition is a grouping of
// Regions. AWS currently has three partitions:
// aws (Standard Regions), aws-cn (China Regions), and aws-us-gov (AWS GovCloud [US] Regions).
static BUCKET_PART: &'static str = r##"(?P<bucket>[a-z0-9][a-z0-9-\.]{1,61}[a-z0-9])"##;

static KEY_PART: &'static str = r##"(?P<key>.+)"##;

// this regex is not too specific to match east/west etc but does at least capture the 'vibe' of AWS regions
// we will be using Rusoto parsing to turn this string into a Region enum so we don't need to do all
// the rules here
static REGION_PART: &'static str =
    r##"(?P<region>(us(-gov)?|af|ap|ca|cn|eu|me|sa)-[a-z]{1,16}-\d)"##;

lazy_static! {

    // uri example 's3://my_bucket/foobar/file.mp3'
    static ref S3_URI_REGEX: Regex =
        Regex::new(format!("^s3://{}/{}$", BUCKET_PART, KEY_PART).as_str()).unwrap();

    // Although you might see legacy endpoints in your logs, we recommend that you always use the standard endpoint syntax to access your buckets.
    // Amazon S3 virtual hosted style URLs follow the format shown below.
    //   https://bucket-name.s3.Region.amazonaws.com/key name
    static ref S3_VIRTUAL_STYLE_GLOBAL_URI_REGEX: Regex =
        Regex::new(format!("^https://{}\\.s3\\.amazonaws\\.com/{}$", BUCKET_PART, KEY_PART).as_str()).unwrap();
    static ref S3_VIRTUAL_STYLE_REGIONAL_URI_REGEX: Regex =
        Regex::new(format!("^https://{}\\.s3\\.{}\\.amazonaws\\.com/{}$", BUCKET_PART, REGION_PART, KEY_PART).as_str()).unwrap();

    // In Amazon S3, path-style URLs follow the format shown below.
    //   https://s3.Region.amazonaws.com/bucket-name/key name
    // old style paths that we will recognise even though they are being deprecated
    // (just because we recognise them in this format doesn't mean we are actually using this format to access them)
    static ref S3_PATH_STYLE_GLOBAL_URI_REGEX: Regex =
        Regex::new(format!("^https://s3\\.amazonaws\\.com/{}/{}$", BUCKET_PART, KEY_PART).as_str()).unwrap();
    static ref S3_PATH_STYLE_REGIONAL_URI_REGEX: Regex =
        Regex::new(format!("^https://s3-{}\\.amazonaws\\.com/{}/{}$", REGION_PART, BUCKET_PART, KEY_PART).as_str()).unwrap();

}

fn extract_bucket_key(cap: Option<Captures>) -> (String, String, Option<Region>) {
    let actual = cap.unwrap();

    (
        String::from(actual.name("bucket").unwrap().as_str()),
        String::from(actual.name("key").unwrap().as_str()),
        None,
    )
}

fn extract_bucket_key_region(cap: Option<Captures>) -> (String, String, Option<Region>) {
    let actual = cap.unwrap();

    (
        String::from(actual.name("bucket").unwrap().as_str()),
        String::from(actual.name("key").unwrap().as_str()),
        Some(Region::from_str(actual.name("region").unwrap().as_str()).unwrap()),
    )
}

#[cfg(test)]
mod tests {
    use crate::s3_uris::is_s3_uri;
    use regex::Regex;
    use rusoto_core::Region;

    fn assert_not_match(uri: &str) {
        let result = is_s3_uri(uri);

        assert!(
            result.is_none(),
            "Uri was not expected to be recognised as valid S3 but it was"
        );
    }

    fn assert_match(uri: &str, bucket: &str, key: &str, region: Option<Region>) {
        let result = is_s3_uri(uri);

        assert!(
            result.is_some(),
            "Uri was expected to be recognised as valid S3 but was not"
        );

        let result_actual = result.unwrap();

        assert_eq!(result_actual.0, bucket);
        assert_eq!(result_actual.1, key);

        if region.is_some() {
            assert_eq!(result_actual.2.unwrap(), region.unwrap());
        }
    }

    #[test]
    fn uri_style() {
        assert_match(
            "s3://jbarr-public/images/abc.jpeg",
            "jbarr-public",
            "images/abc.jpeg",
            None,
        );
    }

    #[test]
    fn path_style() {
        assert_match(
            "https://s3-us-east-2.amazonaws.com/jbarr-public/images/abc.jpeg",
            "jbarr-public",
            "images/abc.jpeg",
            Some(Region::UsEast2),
        );
        assert_match(
            "https://s3.amazonaws.com/jbarr-public/images/abc.jpeg",
            "jbarr-public",
            "images/abc.jpeg",
            None,
        );
    }

    #[test]
    fn virtual_style() {
        assert_match(
            "https://jbarr-public.s3.us-east-2.amazonaws.com/images/abc.jpeg",
            "jbarr-public",
            "images/abc.jpeg",
            Some(Region::UsEast2),
        );
        assert_match(
            "https://jbarr-public.s3.amazonaws.com/images/abc.jpeg",
            "jbarr-public",
            "images/abc.jpeg",
            None,
        );
    }

    #[test]
    fn virtual_style_bucket_like_region() {
        // no reason a bucket can't have a name that looks like a region
        assert_match(
            "https://ap-southeast-2.s3.us-east-2.amazonaws.com/images/abc.jpeg",
            "ap-southeast-2",
            "images/abc.jpeg",
            Some(Region::UsEast2),
        );
    }

    #[test]
    fn path_style_invalid() {
        // the character x here ..s3xamazon.. is to test out that our '.' in the paths are not being
        // matched as regex wildcards
        assert_not_match("https://s3xamazonaws.com/jbarr-public/images/abc.jpeg");
    }

    #[test]
    fn virtual_style_invalid() {
        // not long enough bucket name (only 2 characters)
        assert_not_match("https://to.s3.ap-southeast-2/images/abc.jpeg");
        // bucket names can't end with .
        assert_not_match("https://abucketname..s3.ap-southeast-2/images/abc.jpeg");
        // _ is not valid in a bucket name
        assert_not_match("https://bucket_name.s3.ap-southeast-2/images/abc.jpeg");
    }
}
