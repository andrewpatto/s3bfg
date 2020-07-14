use std::fs::{File, OpenOptions};
use std::io;

#[cfg(target_os = "linux")]
use nix::fcntl::fallocate;
#[cfg(target_os = "linux")]
use nix::fcntl::FallocateFlags;
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

#[cfg(target_os = "linux")]
pub fn create_empty_target_file(write_filename: &str, size: u64) -> Result<File, io::Error> {
    // because we want to let fallocate do its best we want to always work on a new file (disabled)
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(write_filename)?;

    let fd = file.as_raw_fd();

    // for linux we have the added ability to allocate the full size of the file
    // without any actual zero initialising
    fallocate(fd, FallocateFlags::empty(), 0, size as i64);

    Ok(file)
}

#[cfg(not(target_os = "linux"))]
pub fn create_empty_target_file(write_filename: &str, _size: u64) -> Result<File, io::Error> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(write_filename)?;

    Ok(file)
}
