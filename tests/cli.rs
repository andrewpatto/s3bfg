use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

#[test]
fn one_of_the_files_must_be_in_s3() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("s3bfg")?;

    cmd.arg("afile.txt").arg("destfile.txt");
    cmd.assert().failure().stdout(predicate::str::contains(
        "One of the input or output arguments must be something we can recognise as a S3 location",
    ));

    Ok(())
}
