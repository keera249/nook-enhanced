//! Fullscreen and maximized foreground window detection on Linux (X11 EWMH).

use crate::Hwnd;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, Window as XWindow};

fn active_window() -> Option<(x11rb::rust_connection::RustConnection, XWindow, XWindow)> {
    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;
    let atom = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let prop = conn
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    let win = prop.value32()?.next()?;
    if win == 0 {
        return None;
    }
    Some((conn, root, win))
}

fn has_wm_state(conn: &impl Connection, win: XWindow, state_name: &[u8]) -> bool {
    let Ok(wm_state) = conn.intern_atom(false, b"_NET_WM_STATE").and_then(|c| Ok(c.reply())) else {
        return false;
    };
    let Ok(wm_state) = wm_state else { return false; };
    let Ok(target) = conn.intern_atom(false, state_name).and_then(|c| Ok(c.reply())) else {
        return false;
    };
    let Ok(target) = target else { return false; };
    let Ok(prop) = conn
        .get_property(false, win, wm_state.atom, AtomEnum::ATOM, 0, 32)
        .and_then(|c| Ok(c.reply()))
    else {
        return false;
    };
    let Ok(prop) = prop else { return false; };
    prop.value32().is_some_and(|mut it| it.any(|a| a == target.atom))
}

pub fn foreground_is_fullscreen(own: Hwnd) -> bool {
    let Some((conn, root, win)) = active_window() else {
        return false;
    };
    if win as isize == own.raw() {
        return false;
    }
    if has_wm_state(&conn, win, b"_NET_WM_STATE_FULLSCREEN") {
        return true;
    }
    if let Ok(geom) = conn.get_geometry(win).and_then(|c| Ok(c.reply()))
        && let Ok(geom) = geom
        && let Ok(coords) = conn.translate_coordinates(win, root, 0, 0).and_then(|c| Ok(c.reply()))
        && let Ok(coords) = coords
    {
        let r = crate::monitor::Rect {
            left: coords.dst_x as i32,
            top: coords.dst_y as i32,
            right: coords.dst_x as i32 + geom.width as i32,
            bottom: coords.dst_y as i32 + geom.height as i32,
        };
        return crate::monitor::all().iter().any(|m| r.covers(&m.bounds));
    }
    false
}

pub fn foreground_is_full_window(own: Hwnd) -> bool {
    if foreground_is_fullscreen(own) {
        return true;
    }
    let Some((conn, _root, win)) = active_window() else {
        return false;
    };
    if win as isize == own.raw() {
        return false;
    }
    has_wm_state(&conn, win, b"_NET_WM_STATE_MAXIMIZED_VERT")
        && has_wm_state(&conn, win, b"_NET_WM_STATE_MAXIMIZED_HORZ")
}
