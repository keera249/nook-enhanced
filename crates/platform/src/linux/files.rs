//! File picking, thumbnails, drag-out, and config directory on Linux.

use crate::{PlatformError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns Nook's per-user configuration/data directory (`$XDG_CONFIG_HOME/nook` or `~/.config/nook`).
pub fn config_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg).join("nook"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("nook"))
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DropResult {
    Copied,
    Moved,
    Linked,
    Cancelled,
}

pub fn take_own_drop(_paths: &[PathBuf]) -> bool {
    false
}

pub fn drag_out(_paths: &[PathBuf]) -> Result<DropResult> {
    Ok(DropResult::Cancelled)
}

pub fn init_thumbnail_thread() {}

pub fn thumbnail(path: &Path, size: i32) -> Result<(u32, u32, Vec<u8>)> {
    let img = image::open(path).map_err(|e| PlatformError::Other(e.to_string()))?;
    let thumb = img.thumbnail(size.max(16) as u32, size.max(16) as u32).to_rgba8();
    let (w, h) = thumb.dimensions();
    Ok((w, h, thumb.into_raw()))
}

pub fn pick_files(title: &str) -> Result<Vec<PathBuf>> {
    if let Ok(out) = Command::new("zenity")
        .args(["--file-selection", "--multiple", "--separator=\n", "--title", title])
        .output()
    {
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(PathBuf::from)
                .collect());
        }
        return Ok(Vec::new());
    }
    if let Ok(out) = Command::new("kdialog")
        .args(["--getopenfilename", ".", "--multiple", "--separate-output", "--title", title])
        .output()
    {
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(PathBuf::from)
                .collect());
        }
        return Ok(Vec::new());
    }
    Err(PlatformError::Other(
        "install zenity or kdialog to pick files".into(),
    ))
}
