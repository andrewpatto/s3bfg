use std::fs::{OpenOptions, File};
use std::io;

#[cfg(target_os = "linux")]
use nix::fcntl::fallocate;
#[cfg(target_os = "linux")]
use nix::fcntl::FallocateFlags;

#[cfg(target_os = "linux")]
pub fn create_empty_target_file(write_filename: &str, size: i64) -> Result<File, io::Error> {

    // because we want to let fallocate do its best we want to always work on a new file
    let file = OpenOptions::new().write(true)
        .create_new(true)
        .open(write_filename)?;

    let fd = file.as_raw_fd();

    // for linux we have the added ability to allocate the full size of the file
    // without any actual zero initialising
    fallocate(fd, FallocateFlags::empty(), 0, size);

    Ok(file)
}

#[cfg(not(target_os = "linux"))]
pub fn create_empty_target_file(write_filename: &str, size: i64) -> Result<File, io::Error> {
    let file = OpenOptions::new().write(true)
        .create(true)
        .open(write_filename)?;

    Ok(file)
}

