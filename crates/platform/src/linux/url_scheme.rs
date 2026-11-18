//! Custom URL scheme (`nook://`) registration on Linux via `.desktop` + `xdg-mime`.

use crate::{PlatformError, Result};
use std::path::PathBuf;
use std::process::Command;

pub fn register(scheme: &str, description: &str) -> Result<()> {
    let Some(data_home) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
        })
    else {
        return Ok(());
    };
    let apps = data_home.join("applications");
    std::fs::create_dir_all(&apps).map_err(|e| PlatformError::Other(e.to_string()))?;
    let exe = std::env::current_exe().map_err(|e| PlatformError::Other(e.to_string()))?;
    let desktop_name = format!("{scheme}-url.desktop");
    let desktop = apps.join(&desktop_name);
    let content = format!(
        "[Desktop Entry]\nType=Application\nName={description}\nExec=\"{}\" %u\nTerminal=false\nNoDisplay=true\nMimeType=x-scheme-handler/{scheme};\n",
        exe.display()
    );
    std::fs::write(&desktop, content).map_err(|e| PlatformError::Other(e.to_string()))?;
    let _ = Command::new("xdg-mime")
        .args(["default", &desktop_name, &format!("x-scheme-handler/{scheme}")])
        .status();
    Ok(())
}
