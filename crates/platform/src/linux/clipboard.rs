//! Clipboard watcher and read/write on Linux (`wl-paste`/`wl-copy` or `xclip`).

use crate::{PlatformError, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const THUMB_SIZE: u32 = 240;

#[derive(Clone, Debug)]
pub struct ClipImage {
    pub width: u32,
    pub height: u32,
    pub thumb: Vec<u8>,
    pub thumb_size: [usize; 2],
    raw_png: Vec<u8>,
}

#[derive(Clone, Debug)]
pub enum ClipContent {
    Text(String),
    Image(Arc<ClipImage>),
}

pub struct ClipboardWatcher {
    rx: Receiver<ClipContent>,
    writes: Sender<ClipContent>,
    stop: Arc<AtomicBool>,
}

impl ClipboardWatcher {
    pub fn spawn(on_change: impl Fn() + Send + 'static) -> Result<Self> {
        let (tx, rx) = mpsc::channel();
        let (w_tx, w_rx) = mpsc::channel::<ClipContent>();
        let stop = Arc::new(AtomicBool::new(false));
        let s = Arc::clone(&stop);

        thread::spawn(move || {
            let mut last_text = read_text().unwrap_or_default();
            while !s.load(Ordering::Relaxed) {
                while let Ok(item) = w_rx.try_recv() {
                    match &item {
                        ClipContent::Text(t) => {
                            last_text = t.clone();
                            let _ = copy_text(t);
                        }
                        ClipContent::Image(img) => {
                            let _ = write_png(&img.raw_png);
                        }
                    }
                }
                if let Some(now) = read_text()
                    && !now.is_empty()
                    && now != last_text
                {
                    last_text = now.clone();
                    if tx.send(ClipContent::Text(now)).is_err() {
                        return;
                    }
                    on_change();
                }
                thread::sleep(Duration::from_millis(600));
            }
        });

        Ok(Self {
            rx,
            writes: w_tx,
            stop,
        })
    }

    pub fn poll(&self) -> Vec<ClipContent> {
        let mut out = Vec::new();
        while let Ok(v) = self.rx.try_recv() {
            out.push(v);
        }
        out
    }

    pub fn set(&self, content: ClipContent) {
        let _ = self.writes.send(content);
    }
}

impl Drop for ClipboardWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub fn copy_text(text: &str) -> Result<()> {
    let (cmd, args): (&str, &[&str]) = if is_wayland() {
        ("wl-copy", &[])
    } else {
        ("xclip", &["-selection", "clipboard"])
    };
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| PlatformError::Other(format!("{cmd}: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }
    let _ = child.wait();
    Ok(())
}

pub fn read_text() -> Option<String> {
    let (cmd, args): (&str, &[&str]) = if is_wayland() {
        ("wl-paste", &["--no-newline"])
    } else {
        ("xclip", &["-selection", "clipboard", "-o"])
    };
    let out = Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

pub fn copy_image(frame: &crate::capture::Frame) -> Result<()> {
    let png = frame.encode_png()?;
    write_png(&png)
}

fn write_png(png: &[u8]) -> Result<()> {
    let (cmd, args): (&str, &[&str]) = if is_wayland() {
        ("wl-copy", &["--type", "image/png"])
    } else {
        ("xclip", &["-selection", "clipboard", "-t", "image/png"])
    };
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| PlatformError::Other(format!("{cmd}: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(png);
    }
    let _ = child.wait();
    Ok(())
}
