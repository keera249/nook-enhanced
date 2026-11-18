//! System preferences: theme, animations, transparency, build number,
//! accent color and wallpaper path.

use crate::registry::{self, Hive};
use crate::{Result, Win32Context};
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetSystemMetricsForDpi};
use windows::Win32::UI::WindowsAndMessaging::{
    MB_ICONASTERISK, SM_CXSMICON, SPI_GETCLIENTAREAANIMATION, SPI_GETDESKWALLPAPER,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};
use windows::core::BOOL;

const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

/// Windows "app mode" is light.
pub fn apps_use_light_theme() -> bool {
    registry::read_dword(Hive::CurrentUser, PERSONALIZE, "AppsUseLightTheme").unwrap_or(0) != 0
}

/// "Transparency effects" toggle in Settings > Personalization > Colors.
pub fn transparency_enabled() -> bool {
    registry::read_dword(Hive::CurrentUser, PERSONALIZE, "EnableTransparency").unwrap_or(1) != 0
}

/// Pixel size of a small icon (the notification area's) at the system DPI.
pub fn small_icon_size() -> u32 {
    // SAFETY: plain metric queries with no pointers.
    let px = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForSystem()) };
    if px > 0 { px as u32 } else { 16 }
}

/// "Animation effects" (Settings > Accessibility > Visual effects).
pub fn animations_enabled() -> bool {
    let mut on = BOOL(1);
    // SAFETY: SPI_GETCLIENTAREAANIMATION writes a BOOL.
    let res = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(&mut on as *mut BOOL as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    res.is_err() || on.as_bool()
}

/// Windows build number, e.g. 22631. 0 if unknown.
pub fn windows_build() -> u32 {
    registry::read_string(
        Hive::LocalMachine,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentBuildNumber",
    )
    .and_then(|s| s.trim().parse().ok())
    .unwrap_or(0)
}

/// "Windows 11 Pro 24H2 (26100.4061)" for the diagnostics log.
pub fn windows_version() -> String {
    const KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let read = |name| registry::read_string(Hive::LocalMachine, KEY, name).unwrap_or_default();
    let build = windows_build();
    let ubr = registry::read_dword(Hive::LocalMachine, KEY, "UBR").unwrap_or(0);
    // ProductName still says "Windows 10" on Windows 11.
    let product = read("ProductName");
    let product = if build >= 22000 {
        product.replacen("Windows 10", "Windows 11", 1)
    } else {
        product
    };
    format!("{product} {} ({build}.{ubr})", read("DisplayVersion"))
}

/// Battery saver / Energy saver is on (it also turns transparency off).
pub fn battery_saver() -> bool {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut st = SYSTEM_POWER_STATUS::default();
    // SAFETY: valid out pointer.
    unsafe { GetSystemPowerStatus(&mut st) }.is_ok() && st.SystemStatusFlag == 1
}

/// The DWM colorization (accent) color as RGB.
pub fn system_accent() -> Result<[u8; 3]> {
    let mut argb = 0u32;
    let mut opaque = BOOL(0);
    // SAFETY: valid out pointers.
    unsafe { DwmGetColorizationColor(&mut argb, &mut opaque) }.api("DwmGetColorizationColor")?;
    Ok([(argb >> 16) as u8, (argb >> 8) as u8, argb as u8])
}

/// Path of the current desktop wallpaper image, if any.
pub fn wallpaper_path() -> Option<std::path::PathBuf> {
    let mut buf = [0u16; 520];
    // SAFETY: buffer length is passed in characters.
    let res = unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buf.len() as u32,
            Some(buf.as_mut_ptr() as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    res.ok()?;
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    if len == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..len]).into())
}

/// Local time and date, formatted with the user's Windows locale settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clock {
    /// e.g. "14:32" or "2:32 PM".
    pub time: String,
    /// Short weekday and day of month, e.g. "Mon 19".
    pub date: String,
    /// Seconds until the minute changes (for scheduling the next repaint).
    pub secs_to_next_minute: u32,
}

/// Local date and time for file names: `2026-10-20 04-31-12`.
pub fn file_timestamp() -> String {
    use windows::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: plain query returning a value.
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}-{:02}-{:02}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
    )
}

/// Local date and time, split into fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalTime {
    pub year: i32,
    /// 1..=12.
    pub month: u32,
    /// 1..=31.
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

pub fn local_time() -> LocalTime {
    use windows::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: plain query returning a value.
    let t = unsafe { GetLocalTime() };
    LocalTime {
        year: i32::from(t.wYear),
        month: u32::from(t.wMonth),
        day: u32::from(t.wDay),
        hour: u32::from(t.wHour),
        minute: u32::from(t.wMinute),
        second: u32::from(t.wSecond),
    }
}

/// First day of the week in the user's locale: 0 = Monday ... 6 = Sunday.
pub fn first_day_of_week() -> u32 {
    use windows::Win32::Globalization::{GetLocaleInfoEx, LOCALE_IFIRSTDAYOFWEEK};
    use windows::core::PCWSTR;
    let mut buf = [0u16; 4];
    // SAFETY: a sized buffer; a null locale name means the user default.
    let n = unsafe { GetLocaleInfoEx(PCWSTR::null(), LOCALE_IFIRSTDAYOFWEEK, Some(&mut buf)) };
    if n < 2 {
        return 0;
    }
    char::from_u32(u32::from(buf[0]))
        .and_then(|c| c.to_digit(10))
        .filter(|d| *d <= 6)
        .unwrap_or(0)
}

pub fn clock() -> Clock {
    use windows::Win32::Globalization::{
        ENUM_DATE_FORMATS_FLAGS, GetDateFormatEx, GetTimeFormatEx, TIME_NOSECONDS,
    };
    use windows::Win32::System::SystemInformation::GetLocalTime;
    use windows::core::{PCWSTR, w};

    let read = |buf: &[u16], n: i32| {
        let n = (n.max(1) as usize - 1).min(buf.len());
        String::from_utf16_lossy(&buf[..n])
            .chars()
            .filter(|c| {
                !matches!(
                    *c,
                    '\u{200E}'
                        | '\u{200F}'
                        | '\u{061C}'
                        | '\u{202A}'..='\u{202E}'
                        | '\u{2066}'..='\u{2069}'
                )
            })
            .collect::<String>()
    };
    // SAFETY: buffers are sized and the SYSTEMTIME is a local value; a null
    // locale name means the user default.
    unsafe {
        let st = GetLocalTime();
        let mut tbuf = [0u16; 32];
        let n = GetTimeFormatEx(
            PCWSTR::null(),
            TIME_NOSECONDS,
            Some(&st),
            PCWSTR::null(),
            Some(&mut tbuf),
        );
        let time = if n > 0 {
            read(&tbuf, n)
        } else {
            format!("{:02}:{:02}", st.wHour, st.wMinute)
        };
        let mut dbuf = [0u16; 32];
        let n = GetDateFormatEx(
            PCWSTR::null(),
            ENUM_DATE_FORMATS_FLAGS(0),
            Some(&st),
            w!("ddd d"),
            Some(&mut dbuf),
            PCWSTR::null(),
        );
        let date = if n > 0 {
            read(&dbuf, n)
        } else {
            st.wDay.to_string()
        };
        Clock {
            time,
            date,
            secs_to_next_minute: 60 - u32::from(st.wSecond).min(59),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

/// `None` on machines without a battery.
pub fn battery() -> Option<Battery> {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut st = SYSTEM_POWER_STATUS::default();
    // SAFETY: valid out pointer.
    unsafe { GetSystemPowerStatus(&mut st) }.ok()?;
    // 128 = no system battery, 255 = unknown status.
    if st.BatteryFlag & 128 != 0 || st.BatteryFlag == 255 || st.BatteryLifePercent > 100 {
        return None;
    }
    Some(Battery {
        percent: st.BatteryLifePercent,
        charging: st.ACLineStatus == 1,
    })
}

/// Plays the Windows "notification" sound (respects the user's sound scheme
/// and mute). Cosmetic, so a failure is only logged.
pub fn alert_sound() {
    // SAFETY: plain call with a sound-type constant.
    if let Err(e) = unsafe { MessageBeep(MB_ICONASTERISK) } {
        tracing::debug!("MessageBeep: {e}");
    }
}

/// Toggle states of `(Caps Lock, Num Lock, Scroll Lock)`.
pub fn lock_keys() -> (bool, bool, bool) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyState, VK_CAPITAL, VK_NUMLOCK, VK_SCROLL,
    };
    // SAFETY: GetKeyState is a read-only query of the key toggle bit (bit 0).
    unsafe {
        let caps = (GetKeyState(i32::from(VK_CAPITAL.0)) & 1) != 0;
        let num = (GetKeyState(i32::from(VK_NUMLOCK.0)) & 1) != 0;
        let scroll = (GetKeyState(i32::from(VK_SCROLL.0)) & 1) != 0;
        (caps, num, scroll)
    }
}

/// Sends native Windows media/volume virtual keys (`VK_VOLUME_UP` / `VK_VOLUME_DOWN`)
/// with zero process-spawn overhead.
pub fn volume_step(steps: i32) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VK_VOLUME_DOWN, VK_VOLUME_UP, keybd_event,
    };
    let vk = if steps >= 0 {
        VK_VOLUME_UP.0 as u8
    } else {
        VK_VOLUME_DOWN.0 as u8
    };
    let count = steps.unsigned_abs().clamp(1, 10);
    unsafe {
        for _ in 0..count {
            keybd_event(vk, 0, KEYEVENTF_EXTENDEDKEY, 0);
            keybd_event(vk, 0, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP, 0);
        }
    }
}

/// Toggles master system audio mute via native `VK_VOLUME_MUTE`.
pub fn volume_mute_toggle() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VK_VOLUME_MUTE, keybd_event,
    };
    let vk = VK_VOLUME_MUTE.0 as u8;
    unsafe {
        keybd_event(vk, 0, KEYEVENTF_EXTENDEDKEY, 0);
        keybd_event(vk, 0, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP, 0);
    }
}

/// Toggles global media Play/Pause via native `VK_MEDIA_PLAY_PAUSE`.
pub fn media_play_pause_key() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VK_MEDIA_PLAY_PAUSE, keybd_event,
    };
    let vk = VK_MEDIA_PLAY_PAUSE.0 as u8;
    unsafe {
        keybd_event(vk, 0, KEYEVENTF_EXTENDEDKEY, 0);
        keybd_event(vk, 0, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP, 0);
    }
}

/// Returns `(microphone_in_use, webcam_in_use)` by inspecting Windows
/// `CapabilityAccessManager\ConsentStore` registry keys (`LastUsedTimeStop == 0`
/// while `LastUsedTimeStart > 0`).
pub fn privacy_in_use() -> (bool, bool) {
    (
        consent_capability_active("microphone"),
        consent_capability_active("webcam"),
    )
}

fn consent_capability_active(capability: &str) -> bool {
    let base = format!(
        r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\{capability}"
    );
    let non_packaged = format!(r"{base}\NonPackaged");
    consent_tree_has_active(&base) || consent_tree_has_active(&non_packaged)
}

fn consent_tree_has_active(subkey_path: &str) -> bool {
    use crate::wide;
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, RRF_RT_REG_QWORD, RegCloseKey, RegEnumKeyExW,
        RegGetValueW, RegOpenKeyExW,
    };
    use windows::core::{PCWSTR, PWSTR};

    let path_w = wide(subkey_path);
    let mut hkey = HKEY::default();
    // SAFETY: read-only registry open and enumeration.
    unsafe {
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(path_w.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        )
        .is_err()
        {
            return false;
        }
        let mut active = false;
        let mut idx = 0u32;
        let mut name_buf = [0u16; 260];
        let start_w = wide("LastUsedTimeStart");
        let stop_w = wide("LastUsedTimeStop");
        loop {
            let mut name_len = name_buf.len() as u32;
            let res = RegEnumKeyExW(
                hkey,
                idx,
                 Some(PWSTR(name_buf.as_mut_ptr())),
                &mut name_len,
                None,
                None,
                None,
                None,
            );
            if res.is_err() {
                break;
            }
            idx += 1;
            let child = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            if child == "NonPackaged" {
                continue;
            }
            let child_w = wide(&child);
            let mut stop_val = 1u64;
            let mut sz = std::mem::size_of::<u64>() as u32;
            let ok_stop = RegGetValueW(
                hkey,
                PCWSTR(child_w.as_ptr()),
                PCWSTR(stop_w.as_ptr()),
                RRF_RT_REG_QWORD,
                None,
                Some(&mut stop_val as *mut u64 as *mut _),
                Some(&mut sz),
            )
            .is_ok();
            if ok_stop && stop_val == 0 {
                let mut start_val = 0u64;
                let mut sz2 = std::mem::size_of::<u64>() as u32;
                let ok_start = RegGetValueW(
                    hkey,
                    PCWSTR(child_w.as_ptr()),
                    PCWSTR(start_w.as_ptr()),
                    RRF_RT_REG_QWORD,
                    None,
                    Some(&mut start_val as *mut u64 as *mut _),
                    Some(&mut sz2),
                )
                .is_ok();
                if ok_start && start_val > 0 {
                    active = true;
                    break;
                }
            }
        }
        let _ = RegCloseKey(hkey);
        active
    }
}

/// Friendly name of the current foreground application (excluding desktop
/// shell and Nook itself), for daily Screen Time tracking.
pub fn foreground_app_name(own: crate::Hwnd) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetShellWindow, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible,
    };
    use windows::core::PWSTR;

    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid() || fg == own.get() || fg == GetShellWindow() {
            return None;
        }
        if !IsWindowVisible(fg).as_bool() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if pid == 0 || pid == GetCurrentProcessId() {
            return None;
        }
        let mut raw_stem = String::new();
        if let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; 520];
            let mut len = buf.len() as u32;
            if QueryFullProcessImageNameW(
                h,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok()
                && len > 0
            {
                let full = String::from_utf16_lossy(&buf[..len as usize]);
                if let Some(file) = full.rsplit(['\\', '/']).next() {
                    raw_stem = file
                        .strip_suffix(".exe")
                        .or_else(|| file.strip_suffix(".EXE"))
                        .unwrap_or(file)
                        .to_owned();
                }
            }
            let _ = CloseHandle(h);
        }
        let lower = raw_stem.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "" | "idle"
                | "system"
                | "searchhost"
                | "searchapp"
                | "startmenuexperiencehost"
                | "shellexperiencehost"
                | "lockapp"
                | "textinputhost"
                | "nook"
        ) {
            return None;
        }
        if lower == "applicationframehost" {
            let mut tbuf = [0u16; 128];
            let n = GetWindowTextW(fg, &mut tbuf);
            if n > 0 {
                let title = String::from_utf16_lossy(&tbuf[..n as usize]).trim().to_owned();
                if !title.is_empty() {
                    return Some(title);
                }
            }
        }
        Some(friendly_process_name(&raw_stem))
    }
}

fn friendly_process_name(stem: &str) -> String {
    match stem.to_ascii_lowercase().as_str() {
        "chrome" => "Google Chrome".into(),
        "msedge" => "Microsoft Edge".into(),
        "firefox" => "Firefox".into(),
        "brave" => "Brave".into(),
        "opera" => "Opera".into(),
        "code" => "VS Code".into(),
        "devenv" => "Visual Studio".into(),
        "discord" => "Discord".into(),
        "spotify" => "Spotify".into(),
        "steam" | "steamwebhelper" => "Steam".into(),
        "explorer" => "File Explorer".into(),
        "windowsterminal" | "wt" => "Terminal".into(),
        "powershell" | "pwsh" => "PowerShell".into(),
        "cmd" => "Command Prompt".into(),
        "notepad" => "Notepad".into(),
        "obsidian" => "Obsidian".into(),
        "telegram" => "Telegram".into(),
        "whatsapp" => "WhatsApp".into(),
        "slack" => "Slack".into(),
        "vlc" => "VLC".into(),
        "obs64" | "obs" => "OBS Studio".into(),
        _ => {
            let mut chars = stem.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => stem.to_owned(),
            }
        }
    }
}
