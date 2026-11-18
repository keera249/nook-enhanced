//! Embedding external windows (used by Phone Link / scrcpy) on Linux.

use crate::{Hwnd, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, Window as XWindow};

pub fn find_process_window(pid: u32, title: &str) -> Option<Hwnd> {
    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;
    let net_wm_pid = conn.intern_atom(false, b"_NET_WM_PID").ok()?.reply().ok()?.atom;
    let net_wm_name = conn.intern_atom(false, b"_NET_WM_NAME").ok()?.reply().ok()?.atom;
    find_rec(&conn, root, pid, title, net_wm_pid, net_wm_name).map(|w| Hwnd::from_raw(w as isize))
}

fn find_rec<C: Connection>(
    conn: &C,
    win: XWindow,
    pid: u32,
    title: &str,
    net_wm_pid: u32,
    net_wm_name: u32,
) -> Option<XWindow> {
    if let Ok(r) = conn.get_property(false, win, net_wm_pid, AtomEnum::CARDINAL, 0, 1)
        && let Ok(p) = r.reply()
        && let Some(mut v) = p.value32()
        && v.next() == Some(pid)
    {
        if title.is_empty() {
            return Some(win);
        }
        if let Ok(nr) = conn.get_property(false, win, net_wm_name, AtomEnum::ANY, 0, 256)
            && let Ok(np) = nr.reply()
            && let Ok(s) = String::from_utf8(np.value)
            && s.contains(title)
        {
            return Some(win);
        }
    }
    if let Ok(tree) = conn.query_tree(win)
        && let Ok(tree) = tree.reply()
    {
        for child in tree.children {
            if let Some(found) = find_rec(conn, child, pid, title, net_wm_pid, net_wm_name) {
                return Some(found);
            }
        }
    }
    None
}

pub fn is_alive(hwnd: Hwnd) -> bool {
    let win = hwnd.raw() as u32;
    if win == 0 {
        return false;
    }
    x11rb::connect(None)
        .ok()
        .and_then(|(c, _)| c.get_geometry(win).ok()?.reply().ok())
        .is_some()
}

pub fn adopt(_guest: Hwnd, _owner: Hwnd) -> Result<()> {
    Ok(())
}

pub fn place(guest: Hwnd, x: i32, y: i32, w: i32, h: i32, _radius: i32) -> Result<()> {
    crate::window::set_rect(guest, x, y, w, h)
}

pub fn raise_above(guest: Hwnd, _owner: Hwnd) {
    crate::window::activate(guest);
}

pub fn set_visible(guest: Hwnd, visible: bool) {
    let win = guest.raw() as u32;
    if win == 0 {
        return;
    }
    if let Ok((conn, _)) = x11rb::connect(None) {
        if visible {
            let _ = conn.map_window(win);
        } else {
            let _ = conn.unmap_window(win);
        }
        let _ = conn.flush();
    }
}

pub fn set_pass_through(guest: Hwnd, through: bool) {
    let _ = crate::window::set_click_through(guest, through);
}

pub fn foreground() -> Option<Hwnd> {
    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;
    let atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").ok()?.reply().ok()?.atom;
    let prop = conn.get_property(false, root, atom, AtomEnum::WINDOW, 0, 1).ok()?.reply().ok()?;
    let win = prop.value32()?.next()?;
    (win != 0).then(|| Hwnd::from_raw(win as isize))
}

pub fn give_focus(hwnd: Hwnd) {
    crate::window::activate(hwnd);
}

pub struct EscHook {
    _stop: Arc<AtomicBool>,
}

impl EscHook {
    pub fn install(_target: Hwnd, _on_esc: impl Fn() + Send + 'static) -> Result<Self> {
        Ok(Self {
            _stop: Arc::new(AtomicBool::new(false)),
        })
    }
}
