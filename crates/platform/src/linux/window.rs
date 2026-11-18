//! X11 / Wayland window management for Nook on Linux.

use crate::{PlatformError, Result};
use x11rb::connection::Connection;
use x11rb::protocol::shape::{self, SK};
use x11rb::protocol::xproto::{
    self, AtomEnum, ClientMessageEvent, ConnectionExt as _, PropMode, Rectangle,
    Window as XWindow,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct Hwnd(isize);

impl Hwnd {
    pub fn from_raw(raw: isize) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> isize {
        self.0
    }
}

pub fn set_gdi_clearing(_on: bool) {}

pub fn ensure_client_fills(hwnd: Hwnd) -> Option<((i32, i32), (i32, i32))> {
    let (x, y, w, h) = window_rect(hwnd).ok()?;
    Some(((x, y), (w - x, h - y)))
}

pub fn fill_client(_hwnd: Hwnd) -> Result<()> {
    Ok(())
}

pub fn ensure_popup(_hwnd: Hwnd) -> Result<bool> {
    Ok(false)
}

pub fn enable_alpha(_hwnd: Hwnd) -> Result<()> {
    Ok(())
}

pub fn set_owner(_hwnd: Hwnd, _owner: Hwnd) {}

pub fn round_corners(_hwnd: Hwnd) {}

pub fn find_own(title: &str) -> Option<Hwnd> {
    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;
    let pid = std::process::id();
    let net_wm_pid = intern(&conn, "_NET_WM_PID");
    let net_wm_name = intern(&conn, "_NET_WM_NAME");
    find_window_recursive(&conn, root, pid, title, net_wm_pid, net_wm_name).map(|w| Hwnd(w as isize))
}

fn find_window_recursive<C: Connection>(
    conn: &C,
    win: XWindow,
    pid: u32,
    title: &str,
    net_wm_pid: u32,
    net_wm_name: u32,
) -> Option<XWindow> {
    if let Ok(reply) = conn.get_property(false, win, net_wm_pid, AtomEnum::CARDINAL, 0, 1)
        && let Ok(prop) = reply.reply()
        && let Some(mut vals) = prop.value32()
        && vals.next() == Some(pid)
    {
        if title.is_empty() {
            return Some(win);
        }
        let name = get_window_title(conn, win, net_wm_name);
        if name.as_deref() == Some(title) {
            return Some(win);
        }
    }
    if let Ok(tree) = conn.query_tree(win)
        && let Ok(tree) = tree.reply()
    {
        for child in tree.children {
            if let Some(found) =
                find_window_recursive(conn, child, pid, title, net_wm_pid, net_wm_name)
            {
                return Some(found);
            }
        }
    }
    None
}

fn get_window_title<C: Connection>(conn: &C, win: XWindow, net_wm_name: u32) -> Option<String> {
    if let Ok(r) = conn.get_property(false, win, net_wm_name, AtomEnum::ANY, 0, 256)
        && let Ok(p) = r.reply()
        && !p.value.is_empty()
    {
        return String::from_utf8(p.value).ok();
    }
    if let Ok(r) = conn.get_property(false, win, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 256)
        && let Ok(p) = r.reply()
        && !p.value.is_empty()
    {
        return String::from_utf8(p.value).ok();
    }
    None
}

fn intern<C: Connection>(conn: &C, name: &str) -> u32 {
    conn.intern_atom(false, name.as_bytes())
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.atom)
        .unwrap_or(0)
}

pub fn ensure_overlay(hwnd: Hwnd, _activatable: bool) -> Result<bool> {
    let win = hwnd.raw() as u32;
    if win == 0 {
        return Ok(false);
    }
    let Ok((conn, screen_num)) = x11rb::connect(None) else {
        return Ok(false);
    };
    let root = conn.setup().roots[screen_num].root;

    // Set _NET_WM_WINDOW_TYPE to _NET_WM_WINDOW_TYPE_DOCK (or UTILITY) so WMs
    // keep it borderless and out of Alt-Tab / taskbar.
    let wm_type = intern(&conn, "_NET_WM_WINDOW_TYPE");
    let dock = intern(&conn, "_NET_WM_WINDOW_TYPE_DOCK");
    if wm_type != 0 && dock != 0 {
        let _ = conn.change_property(
            PropMode::REPLACE,
            win,
            wm_type,
            AtomEnum::ATOM,
            32,
            1,
            &dock.to_ne_bytes(),
        );
    }

    // Request ABOVE + STICKY + SKIP_TASKBAR + SKIP_PAGER.
    let wm_state = intern(&conn, "_NET_WM_STATE");
    let above = intern(&conn, "_NET_WM_STATE_ABOVE");
    let sticky = intern(&conn, "_NET_WM_STATE_STICKY");
    let skip_tb = intern(&conn, "_NET_WM_STATE_SKIP_TASKBAR");
    let skip_pg = intern(&conn, "_NET_WM_STATE_SKIP_PAGER");
    for atom in [above, sticky, skip_tb, skip_pg] {
        if wm_state != 0 && atom != 0 {
            send_wm_state(&conn, root, win, wm_state, 1, atom);
        }
    }
    let _ = conn.flush();
    Ok(false)
}

fn send_wm_state<C: Connection>(
    conn: &C,
    root: XWindow,
    win: XWindow,
    wm_state: u32,
    action: u32,
    atom1: u32,
) {
    let event = ClientMessageEvent::new(32, win, wm_state, [action, atom1, 0, 1, 0]);
    let _ = conn.send_event(
        false,
        root,
        xproto::EventMask::SUBSTRUCTURE_REDIRECT | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
        event,
    );
}

pub fn post_left_button_up(_hwnd: Hwnd) {}

pub fn activate(hwnd: Hwnd) {
    let win = hwnd.raw() as u32;
    if win == 0 {
        return;
    }
    if let Ok((conn, screen_num)) = x11rb::connect(None) {
        let root = conn.setup().roots[screen_num].root;
        let active = intern(&conn, "_NET_ACTIVE_WINDOW");
        if active != 0 {
            let event = ClientMessageEvent::new(32, win, active, [1, 0, 0, 0, 0]);
            let _ = conn.send_event(
                false,
                root,
                xproto::EventMask::SUBSTRUCTURE_REDIRECT | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                event,
            );
        }
        let _ = conn.set_input_focus(xproto::InputFocus::POINTER_ROOT, win, x11rb::CURRENT_TIME);
        let _ = conn.flush();
    }
}

pub fn set_opacity(hwnd: Hwnd, alpha: u8) -> Result<()> {
    let win = hwnd.raw() as u32;
    if win == 0 {
        return Ok(());
    }
    if let Ok((conn, _)) = x11rb::connect(None) {
        let atom = intern(&conn, "_NET_WM_WINDOW_OPACITY");
        if atom != 0 {
            let val = ((alpha as u32) * 0x0101_0101u32).to_ne_bytes();
            let _ = conn.change_property(PropMode::REPLACE, win, atom, AtomEnum::CARDINAL, 32, 1, &val);
            let _ = conn.flush();
        }
    }
    Ok(())
}

pub fn ensure_shown(hwnd: Hwnd) {
    let win = hwnd.raw() as u32;
    if win == 0 {
        return;
    }
    if let Ok((conn, _)) = x11rb::connect(None) {
        let _ = conn.map_window(win);
        let _ = conn.flush();
    }
}

pub fn set_click_through(hwnd: Hwnd, through: bool) -> Result<()> {
    let win = hwnd.raw() as u32;
    if win == 0 {
        return Ok(());
    }
    if let Ok((conn, _)) = x11rb::connect(None) {
        if through {
            // Empty input region -> all pointer events pass through.
            let _ = shape::rectangles(
                &conn,
                shape::SO::SET,
                SK::INPUT,
                xproto::ClipOrdering::UNSORTED,
                win,
                0,
                0,
                &[],
            );
        } else {
            // Reset input shape to full window.
            let geom = conn
                .get_geometry(win)
                .ok()
                .and_then(|c| c.reply().ok());
            if let Some(g) = geom {
                let rect = Rectangle {
                    x: 0,
                    y: 0,
                    width: g.width,
                    height: g.height,
                };
                let _ = shape::rectangles(
                    &conn,
                    shape::SO::SET,
                    SK::INPUT,
                    xproto::ClipOrdering::UNSORTED,
                    win,
                    0,
                    0,
                    &[rect],
                );
            }
        }
        let _ = conn.flush();
    }
    Ok(())
}

pub fn is_covered(_hwnd: Hwnd) -> bool {
    false
}

pub fn is_topmost(_hwnd: Hwnd) -> bool {
    true
}

pub fn set_layer(main: Hwnd, _owner: Option<Hwnd>, on_top: bool) -> Result<()> {
    let win = main.raw() as u32;
    if win == 0 {
        return Ok(());
    }
    if let Ok((conn, screen_num)) = x11rb::connect(None) {
        let root = conn.setup().roots[screen_num].root;
        let wm_state = intern(&conn, "_NET_WM_STATE");
        let above = intern(&conn, "_NET_WM_STATE_ABOVE");
        let below = intern(&conn, "_NET_WM_STATE_BELOW");
        if wm_state != 0 && above != 0 && below != 0 {
            if on_top {
                send_wm_state(&conn, root, win, wm_state, 0, below);
                send_wm_state(&conn, root, win, wm_state, 1, above);
            } else {
                send_wm_state(&conn, root, win, wm_state, 0, above);
                send_wm_state(&conn, root, win, wm_state, 1, below);
            }
        }
        let _ = conn.flush();
    }
    Ok(())
}

pub fn window_rect(hwnd: Hwnd) -> Result<(i32, i32, i32, i32)> {
    let win = hwnd.raw() as u32;
    if win == 0 {
        return Err(PlatformError::Other("null window handle".into()));
    }
    let (conn, screen_num) =
        x11rb::connect(None).map_err(|e| PlatformError::Other(format!("x11 connect: {e}")))?;
    let root = conn.setup().roots[screen_num].root;
    let geom = conn
        .get_geometry(win)
        .map_err(|e| PlatformError::Other(e.to_string()))?
        .reply()
        .map_err(|e| PlatformError::Other(e.to_string()))?;
    let coords = conn
        .translate_coordinates(win, root, 0, 0)
        .map_err(|e| PlatformError::Other(e.to_string()))?
        .reply()
        .map_err(|e| PlatformError::Other(e.to_string()))?;
    let x = coords.dst_x as i32;
    let y = coords.dst_y as i32;
    Ok((x, y, x + geom.width as i32, y + geom.height as i32))
}

pub fn set_rect(hwnd: Hwnd, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
    let win = hwnd.raw() as u32;
    if win == 0 {
        return Ok(());
    }
    if let Ok((conn, _)) = x11rb::connect(None) {
        let aux = xproto::ConfigureWindowAux::new()
            .x(x)
            .y(y)
            .width(w.max(1) as u32)
            .height(h.max(1) as u32);
        let _ = conn.configure_window(win, &aux);
        let _ = conn.flush();
    }
    Ok(())
}

pub fn clear_gdi_surface(_hwnd: Hwnd) {}

pub fn dpi(_hwnd: Hwnd) -> u32 {
    if let Ok(scale) = std::env::var("GDK_SCALE")
        && let Ok(s) = scale.parse::<f32>()
    {
        return (96.0 * s).round() as u32;
    }
    96
}
