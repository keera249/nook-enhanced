//! "Is a fullscreen app or game in front?"

use crate::monitor::{self, Rect};
use crate::window::Hwnd;
use windows::Win32::Foundation::RECT;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Shell::{
    QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetShellWindow, GetWindowRect, GetWindowThreadProcessId,
    IsWindowVisible, IsZoomed,
};

/// True when the foreground window covers the whole monitor the notch is on
/// (borderless fullscreen) or the shell reports exclusive D3D fullscreen /
/// presentation mode. Maximized windows, windows on other monitors (a
/// maximized window on a monitor without a taskbar covers it too), the
/// desktop and our own windows (e.g. the draw-on-screen overlay) never count.
pub fn foreground_is_fullscreen(own: Hwnd) -> bool {
    // SAFETY: read-only shell query.
    if let Ok(state) = unsafe { SHQueryUserNotificationState() }
        && (state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE)
    {
        return true;
    }
    // SAFETY: read-only window queries; handles may be stale, which only
    // makes the calls fail.
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid() || fg == own.get() || fg == GetShellWindow() {
            return false;
        }
        if !IsWindowVisible(fg).as_bool() || IsZoomed(fg).as_bool() {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return false;
        }
        let mut class = [0u16; 64];
        let n = GetClassNameW(fg, &mut class).max(0) as usize;
        let class = String::from_utf16_lossy(&class[..n]);
        if matches!(class.as_str(), "WorkerW" | "Progman" | "Shell_TrayWnd") {
            return false;
        }
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        let (Ok(mon), Ok(ours)) = (
            monitor::of_window(Hwnd::from_raw(fg.0 as isize)),
            monitor::of_window(own),
        ) else {
            return false;
        };
        mon.bounds == ours.bounds && Rect::from(r).covers(&mon.bounds)
    }
}

/// True when the foreground window on the notch's monitor is either fullscreen
/// or maximized (or covers the monitor's work area), and false when the user
/// is on the desktop / home screen or using a non-maximized (partial) window.
pub fn foreground_is_full_window(own: Hwnd) -> bool {
    if foreground_is_fullscreen(own) {
        return true;
    }
    // SAFETY: read-only window queries.
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid() || fg == own.get() || fg == GetShellWindow() {
            return false;
        }
        if !IsWindowVisible(fg).as_bool() {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return false;
        }
        let mut class = [0u16; 64];
        let n = GetClassNameW(fg, &mut class).max(0) as usize;
        let class = String::from_utf16_lossy(&class[..n]);
        if matches!(
            class.as_str(),
            "WorkerW"
                | "Progman"
                | "Shell_TrayWnd"
                | "Shell_SecondaryTrayWnd"
                | "MultitaskingViewFrame"
        ) {
            return false;
        }
        let mon = monitor::of_window(Hwnd::from_raw(fg.0 as isize))
            .or_else(|_| monitor::of_window(own))
            .or_else(|_| monitor::primary())
            .ok();
        let Some(mon) = mon else {
            return false;
        };
        if IsZoomed(fg).as_bool() {
            return true;
        }
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        let w_width = (r.right - r.left).max(0);
        let w_height = (r.bottom - r.top).max(0);
        let mon_w = (mon.work.right - mon.work.left).max(1);
        let mon_h = (mon.work.bottom - mon.work.top).max(1);
        let covers_most = w_width >= (mon_w - 30) && w_height >= (mon_h - 30) && r.top <= mon.work.top + 30;

        Rect::from(r).covers(&mon.work) || covers_most
    }
}
