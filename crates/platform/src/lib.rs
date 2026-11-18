//! Thin wrappers over the OS APIs Nook uses (Win32/DWM on Windows, X11/Wayland/POSIX on Linux).
//!
//! Every call into the OS goes through a function here that returns a
//! [`Result`] naming the API that failed, so the rest of the app never sees
//! raw OS handles or error codes.

#[cfg_attr(target_os = "linux", path = "linux/audio.rs")]
pub mod audio;
#[cfg_attr(target_os = "linux", path = "linux/backdrop.rs")]
pub mod backdrop;
#[cfg_attr(target_os = "linux", path = "linux/capture.rs")]
pub mod capture;
#[cfg_attr(target_os = "linux", path = "linux/crash.rs")]
pub mod crash;
#[cfg_attr(target_os = "linux", path = "linux/clipboard.rs")]
pub mod clipboard;
#[cfg_attr(target_os = "linux", path = "linux/embed.rs")]
pub mod embed;
#[cfg_attr(target_os = "linux", path = "linux/files.rs")]
pub mod files;
#[cfg_attr(target_os = "linux", path = "linux/fontmatch.rs")]
pub mod fontmatch;
pub mod fontnet;
#[cfg_attr(target_os = "linux", path = "linux/fullscreen.rs")]
pub mod fullscreen;
#[cfg_attr(target_os = "linux", path = "linux/hittest.rs")]
pub mod hittest;
#[cfg_attr(target_os = "linux", path = "linux/ink.rs")]
pub mod ink;
#[cfg_attr(target_os = "linux", path = "linux/ipc.rs")]
pub mod ipc;
#[cfg_attr(target_os = "linux", path = "linux/material.rs")]
pub mod material;
#[cfg_attr(target_os = "linux", path = "linux/media.rs")]
pub mod media;
#[cfg_attr(target_os = "linux", path = "linux/monitor.rs")]
pub mod monitor;
#[cfg_attr(target_os = "linux", path = "linux/ocr.rs")]
pub mod ocr;
#[cfg_attr(target_os = "linux", path = "linux/perf.rs")]
pub mod perf;
#[cfg_attr(target_os = "linux", path = "linux/power.rs")]
pub mod power;
#[cfg_attr(target_os = "linux", path = "linux/process.rs")]
pub mod process;
#[cfg_attr(target_os = "linux", path = "linux/raster.rs")]
pub mod raster;
#[cfg_attr(target_os = "linux", path = "linux/registry.rs")]
pub mod registry;
#[cfg_attr(target_os = "linux", path = "linux/shell.rs")]
pub mod shell;
#[cfg_attr(target_os = "linux", path = "linux/single_instance.rs")]
pub mod single_instance;
#[cfg_attr(target_os = "linux", path = "linux/snip.rs")]
pub mod snip;
#[cfg_attr(target_os = "linux", path = "linux/startup.rs")]
pub mod startup;
#[cfg_attr(target_os = "linux", path = "linux/system.rs")]
pub mod system;
#[cfg_attr(target_os = "linux", path = "linux/url_scheme.rs")]
pub mod url_scheme;
#[cfg_attr(target_os = "linux", path = "linux/wallpaper.rs")]
pub mod wallpaper;
#[cfg_attr(target_os = "linux", path = "linux/window.rs")]
pub mod window;

pub use window::Hwnd;

/// A failed platform API call.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[cfg(windows)]
    #[error("{call} failed: {source}")]
    Win32 {
        call: &'static str,
        #[source]
        source: windows::core::Error,
    },
    #[cfg(windows)]
    #[error("{call} failed with Win32 error {code}")]
    Code { call: &'static str, code: u32 },
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, PlatformError>;

/// Attaches the API name to a `windows::core::Result`.
#[cfg(windows)]
pub(crate) trait Win32Context<T> {
    fn api(self, call: &'static str) -> Result<T>;
}

#[cfg(windows)]
impl<T> Win32Context<T> for windows::core::Result<T> {
    fn api(self, call: &'static str) -> Result<T> {
        self.map_err(|source| PlatformError::Win32 { call, source })
    }
}

/// Converts a `WIN32_ERROR` return code into a `Result`.
#[cfg(windows)]
pub(crate) fn check(
    call: &'static str,
    err: windows::Win32::Foundation::WIN32_ERROR,
) -> Result<()> {
    if err.is_ok() {
        Ok(())
    } else {
        Err(PlatformError::Code { call, code: err.0 })
    }
}

/// Null-terminated UTF-16 for `PCWSTR` parameters.
#[cfg(windows)]
pub(crate) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
