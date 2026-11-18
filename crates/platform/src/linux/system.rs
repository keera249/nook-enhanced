//! OS settings, time, battery, audio keys and privacy indicators on Linux.

use crate::{Hwnd, PlatformError, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn apps_use_light_theme() -> bool {
    if let Ok(out) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        if s.contains("prefer-dark") {
            return false;
        }
        if s.contains("prefer-light") {
            return true;
        }
    }
    false
}

pub fn transparency_enabled() -> bool {
    true
}

pub fn small_icon_size() -> u32 {
    24
}

pub fn animations_enabled() -> bool {
    true
}

pub fn windows_build() -> u32 {
    0
}

pub fn windows_version() -> String {
    if let Ok(os_release) = fs::read_to_string("/etc/os-release") {
        for line in os_release.lines() {
            if let Some(pretty) = line.strip_prefix("PRETTY_NAME=") {
                return pretty.trim_matches('"').to_owned();
            }
        }
    }
    "Linux".to_owned()
}

pub fn battery_saver() -> bool {
    if let Ok(out) = Command::new("powerprofilesctl").arg("get").output() {
        return String::from_utf8_lossy(&out.stdout).trim() == "power-saver";
    }
    false
}

pub fn system_accent() -> Result<[u8; 3]> {
    Ok([100, 160, 255])
}

pub fn wallpaper_path() -> Option<PathBuf> {
    if let Ok(out) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.background", "picture-uri-dark"])
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout)
            .trim()
            .trim_matches('\'')
            .to_owned();
        if let Some(p) = s.strip_prefix("file://") {
            let pb = PathBuf::from(p);
            if pb.is_file() {
                return Some(pb);
            }
        }
    }
    None
}

pub struct Clock {
    pub time: String,
    pub date: String,
    pub secs_to_next_minute: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct LocalTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

pub fn file_timestamp() -> String {
    let t = local_time();
    format!(
        "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
        t.year, t.month, t.day, t.hour, t.minute, t.second
    )
}

pub fn local_time() -> LocalTime {
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        LocalTime {
            year: tm.tm_year + 1900,
            month: (tm.tm_mon + 1) as u32,
            day: tm.tm_mday as u32,
            hour: tm.tm_hour as u32,
            minute: tm.tm_min as u32,
            second: tm.tm_sec as u32,
        }
    }
}

pub fn first_day_of_week() -> u32 {
    1 // Monday
}

pub fn clock() -> Clock {
    let t = local_time();
    let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let wday = unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        days.get(tm.tm_wday as usize).copied().unwrap_or("Mon")
    };
    Clock {
        time: format!("{:02}:{:02}", t.hour, t.minute),
        date: format!("{wday} {}", t.day),
        secs_to_next_minute: 60u32.saturating_sub(t.second).max(1),
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

pub fn battery() -> Option<Battery> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let p = entry.path();
        let kind = fs::read_to_string(p.join("type")).unwrap_or_default();
        if kind.trim() != "Battery" {
            continue;
        }
        let cap: u8 = fs::read_to_string(p.join("capacity"))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let status = fs::read_to_string(p.join("status")).unwrap_or_default();
        let charging = matches!(status.trim(), "Charging" | "Full");
        return Some(Battery {
            percent: cap.min(100),
            charging,
        });
    }
    None
}

pub fn alert_sound() {
    let _ = Command::new("paplay")
        .arg("/usr/share/sounds/freedesktop/stereo/bell.oga")
        .spawn();
}

pub fn lock_keys() -> (bool, bool, bool) {
    (false, false, false)
}

pub fn volume_step(steps: i32) {
    let pct = steps * 2;
    let arg = if pct >= 0 {
        format!("+{pct}%")
    } else {
        format!("{pct}%")
    };
    let _ = Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", &arg])
        .spawn();
}

pub fn volume_mute_toggle() {
    let _ = Command::new("pactl")
        .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
        .spawn();
}

pub fn media_play_pause_key() {
    let _ = Command::new("playerctl").arg("play-pause").spawn();
}

pub fn privacy_in_use() -> (bool, bool) {
    let mic = Command::new("pactl")
        .args(["list", "short", "source-outputs"])
        .output()
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);
    (false, mic)
}

pub fn foreground_app_name(_own: Hwnd) -> Option<String> {
    None
}

#[allow(dead_code)]
fn _unused(_e: PlatformError) {}
