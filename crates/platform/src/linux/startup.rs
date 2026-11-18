//! XDG Autostart (`~/.config/autostart/nook.desktop`) on Linux.

use crate::{PlatformError, Result};
use std::path::PathBuf;

fn desktop_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("autostart").join("nook.desktop"))
}

pub fn is_enabled() -> bool {
    desktop_path().is_some_and(|p| p.is_file())
}

pub fn set_enabled(enabled: bool) -> Result<()> {
    let Some(path) = desktop_path() else {
        return Ok(());
    };
    if !enabled {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| PlatformError::Other(e.to_string()))?;
        }
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| PlatformError::Other(e.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| PlatformError::Other(e.to_string()))?;
    }
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Nook\nExec=\"{}\"\nX-GNOME-Autostart-enabled=true\n",
        exe.display()
    );
    std::fs::write(&path, content).map_err(|e| PlatformError::Other(e.to_string()))?;
    Ok(())
}
