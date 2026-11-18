//! Desktop wallpaper and accent color extraction on Linux.

use crate::{PlatformError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn set_desktop(path: &Path) -> Result<()> {
    let uri = format!("file://{}", path.display());
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.background", "picture-uri", &uri])
        .status();
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.background", "picture-uri-dark", &uri])
        .status();
    let _ = Command::new("plasma-apply-wallpaperimage").arg(path).status();
    let _ = Command::new("feh").args(["--bg-fill", &path.to_string_lossy()]).status();
    Ok(())
}

pub fn pick_folder(title: &str, start: Option<&Path>) -> Result<Option<PathBuf>> {
    let mut cmd = Command::new("zenity");
    cmd.args(["--file-selection", "--directory", "--title", title]);
    if let Some(s) = start {
        cmd.arg("--filename").arg(s);
    }
    if let Ok(out) = cmd.output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            if !p.is_empty() {
                return Ok(Some(PathBuf::from(p)));
            }
        }
        return Ok(None);
    }
    Err(PlatformError::Other("zenity not found".into()))
}

pub fn accent_from_wallpaper() -> Result<Option<[u8; 3]>> {
    let Some(path) = crate::system::wallpaper_path() else {
        return Ok(None);
    };
    let Ok(img) = image::open(&path) else {
        return Ok(None);
    };
    let small = img.thumbnail(64, 64).to_rgb8();
    let pixels: Vec<[u8; 3]> = small.pixels().map(|p| p.0).collect();
    Ok(dominant_accent(&pixels))
}

pub fn dominant_accent(pixels: &[[u8; 3]]) -> Option<[u8; 3]> {
    if pixels.is_empty() {
        return None;
    }
    let mut r_sum = 0u64;
    let mut g_sum = 0u64;
    let mut b_sum = 0u64;
    let mut count = 0u64;
    for &[r, g, b] in pixels {
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        if max.saturating_sub(min) > 25 {
            r_sum += r as u64;
            g_sum += g as u64;
            b_sum += b as u64;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    Some([
        (r_sum / count) as u8,
        (g_sum / count) as u8,
        (b_sum / count) as u8,
    ])
}
