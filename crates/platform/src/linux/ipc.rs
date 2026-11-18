//! Single-instance IPC over Unix domain sockets on Linux.

use crate::{PlatformError, Result};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::thread;

fn socket_path(name: &str) -> PathBuf {
    let uid = unsafe { libc::getuid() };
    let safe: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    std::env::temp_dir().join(format!("nook-ipc-{safe}-{uid}.sock"))
}

pub fn serve(name: &str, on_message: impl Fn(String) + Send + 'static) -> Result<()> {
    let path = socket_path(name);
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)
        .map_err(|e| PlatformError::Other(format!("ipc bind {}: {e}", path.display())))?;
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut s = stream;
            let mut msg = String::new();
            if s.read_to_string(&mut msg).is_ok() && !msg.is_empty() {
                on_message(msg);
            }
        }
    });
    Ok(())
}

pub fn send(name: &str, message: &str) -> Result<()> {
    let path = socket_path(name);
    let mut stream = UnixStream::connect(&path)
        .map_err(|e| PlatformError::Other(format!("ipc connect {}: {e}", path.display())))?;
    stream
        .write_all(message.as_bytes())
        .map_err(|e| PlatformError::Other(e.to_string()))?;
    Ok(())
}
