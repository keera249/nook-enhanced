//! System performance metrics on Linux (`/proc` and `statvfs`).

use crate::{PlatformError, Result};
use std::ffi::CString;
use std::fs;
use std::time::Duration;

#[derive(Copy, Clone, Debug, Default)]
pub struct CpuTimes {
    pub idle: u64,
    pub total: u64,
}

impl CpuTimes {
    pub fn usage_since(&self, prev: &CpuTimes) -> f32 {
        let d_total = self.total.saturating_sub(prev.total);
        let d_idle = self.idle.saturating_sub(prev.idle);
        if d_total == 0 {
            return 0.0;
        }
        ((d_total.saturating_sub(d_idle)) as f32 / d_total as f32).clamp(0.0, 1.0)
    }
}

pub fn cpu_times() -> Result<CpuTimes> {
    let stat = fs::read_to_string("/proc/stat")
        .map_err(|e| PlatformError::Other(format!("/proc/stat: {e}")))?;
    let line = stat
        .lines()
        .find(|l| l.starts_with("cpu "))
        .ok_or_else(|| PlatformError::Other("missing cpu line in /proc/stat".into()))?;
    let nums: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    if nums.len() < 4 {
        return Err(PlatformError::Other("malformed /proc/stat".into()));
    }
    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    let total: u64 = nums.iter().sum();
    Ok(CpuTimes { idle, total })
}

#[derive(Copy, Clone, Debug)]
pub struct Memory {
    pub used: u64,
    pub total: u64,
}

pub fn memory() -> Result<Memory> {
    let info = fs::read_to_string("/proc/meminfo")
        .map_err(|e| PlatformError::Other(format!("/proc/meminfo: {e}")))?;
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    for line in info.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total_kb = parse_kb(rest);
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            avail_kb = parse_kb(rest);
        }
    }
    let total = total_kb * 1024;
    let used = total.saturating_sub(avail_kb * 1024);
    Ok(Memory { used, total })
}

fn parse_kb(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

pub fn net_bytes() -> Result<(u64, u64)> {
    let text = fs::read_to_string("/proc/net/dev")
        .map_err(|e| PlatformError::Other(format!("/proc/net/dev: {e}")))?;
    let mut rx = 0u64;
    let mut tx = 0u64;
    for line in text.lines().skip(2) {
        let Some((iface, rest)) = line.split_once(':') else {
            continue;
        };
        if iface.trim() == "lo" {
            continue;
        }
        let cols: Vec<u64> = rest
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if cols.len() >= 9 {
            rx = rx.saturating_add(cols[0]);
            tx = tx.saturating_add(cols[8]);
        }
    }
    Ok((rx, tx))
}

#[derive(Clone, Debug)]
pub struct Disk {
    pub name: String,
    pub free: u64,
    pub total: u64,
}

pub fn disks() -> Vec<Disk> {
    let Ok(mounts) = fs::read_to_string("/proc/mounts") else {
        return Vec::new();
    };
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let dev = parts[0];
        let mount = parts[1];
        if !dev.starts_with("/dev/") || seen.contains(&mount.to_owned()) {
            continue;
        }
        seen.push(mount.to_owned());
        if let Ok(c_path) = CString::new(mount) {
            unsafe {
                let mut st: libc::statvfs = std::mem::zeroed();
                if libc::statvfs(c_path.as_ptr(), &mut st) == 0 && st.f_blocks > 0 {
                    let bsize = st.f_frsize as u64;
                    out.push(Disk {
                        name: mount.to_owned(),
                        free: (st.f_bavail as u64).saturating_mul(bsize),
                        total: (st.f_blocks as u64).saturating_mul(bsize),
                    });
                }
            }
        }
    }
    out
}

pub fn uptime() -> Duration {
    if let Ok(text) = fs::read_to_string("/proc/uptime")
        && let Some(first) = text.split_whitespace().next()
        && let Ok(secs) = first.parse::<f64>()
    {
        return Duration::from_secs_f64(secs.max(0.0));
    }
    Duration::ZERO
}

#[derive(Clone, Debug)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub cpu: u64,
    pub memory: u64,
}

pub fn processes() -> Result<Vec<Process>> {
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(4096) as u64;
    let mut out = Vec::new();
    let entries = fs::read_dir("/proc")
        .map_err(|e| PlatformError::Other(format!("/proc: {e}")))?;
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let p = entry.path();
        let Ok(stat) = fs::read_to_string(p.join("stat")) else {
            continue;
        };
        let Some(open) = stat.find('(') else { continue; };
        let Some(close) = stat.rfind(')') else { continue; };
        let name = stat[open + 1..close].to_owned();
        let rest: Vec<&str> = stat[close + 2..].split_whitespace().collect();
        let utime = rest.get(11).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let stime = rest.get(12).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let rss_pages = rest.get(21).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        out.push(Process {
            pid,
            name,
            cpu: (utime + stime) * 100_000,
            memory: rss_pages.saturating_mul(page_size),
        });
    }
    Ok(out)
}
