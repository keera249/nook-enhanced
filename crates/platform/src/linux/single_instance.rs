//! Single-instance lock on Linux using `flock(2)`.

use crate::{PlatformError, Result};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;

pub struct InstanceGuard(#[allow(dead_code)] File);

pub fn acquire(name: &str) -> Result<Option<InstanceGuard>> {
    let uid = unsafe { libc::getuid() };
    let safe: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let path = std::env::temp_dir().join(format!("{safe}-{uid}.lock"));
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| PlatformError::Other(format!("lock file {}: {e}", path.display())))?;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        Ok(Some(InstanceGuard(file)))
    } else {
        Ok(None)
    }
}
