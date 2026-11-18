//! Desktop shell helpers on Linux (XDG directories, xdg-open, trash).

use crate::{PlatformError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn xdg_dir(key: &str, fallback: &str) -> Option<PathBuf> {
    if let Ok(out) = Command::new("xdg-user-dir").arg(key).output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        if !s.is_empty() {
            return Some(PathBuf::from(s));
        }
    }
    home().map(|h| h.join(fallback))
}

pub fn screenshots_dir() -> Option<PathBuf> {
    pictures_dir().map(|p| {
        let sc = p.join("Screenshots");
        if sc.is_dir() { sc } else { p }
    })
}

pub fn downloads_dir() -> Option<PathBuf> {
    xdg_dir("DOWNLOAD", "Downloads")
}

pub fn pictures_dir() -> Option<PathBuf> {
    xdg_dir("PICTURES", "Pictures")
}

pub fn music_dir() -> Option<PathBuf> {
    xdg_dir("MUSIC", "Music")
}

pub fn recycle(path: &Path) -> Result<()> {
    let status = Command::new("gio")
        .arg("trash")
        .arg(path)
        .status()
        .or_else(|_| Command::new("trash-put").arg(path).status());
    match status {
        Ok(s) if s.success() => Ok(()),
        _ => std::fs::remove_file(path)
            .or_else(|_| std::fs::remove_dir_all(path))
            .map_err(|e| PlatformError::Other(e.to_string())),
    }
}

pub fn reveal(path: &Path) -> Result<()> {
    let target = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map_err(|e| PlatformError::Other(format!("xdg-open: {e}")))?;
    Ok(())
}

pub fn open(target: &str) -> Result<()> {
    Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map_err(|e| PlatformError::Other(format!("xdg-open: {e}")))?;
    Ok(())
}
