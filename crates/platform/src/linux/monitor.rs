//! Display monitors and cursor position on Linux (X11 RandR + fallback).

use crate::{Hwnd, PlatformError, Result};
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::ConnectionExt as _;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn covers(&self, other: &Rect) -> bool {
        self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Monitor {
    pub id: isize,
    pub bounds: Rect,
    pub work: Rect,
    pub dpi: u32,
}

pub fn all() -> Vec<Monitor> {
    if let Some(list) = x11_monitors()
        && !list.is_empty()
    {
        return list;
    }
    vec![Monitor {
        id: 1,
        bounds: Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        },
        work: Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        },
        dpi: 96,
    }]
}

fn x11_monitors() -> Option<Vec<Monitor>> {
    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let res = conn.randr_get_monitors(root, true).ok()?.reply().ok()?;
    let mut out = Vec::new();
    for (idx, m) in res.monitors.iter().enumerate() {
        let bounds = Rect {
            left: m.x as i32,
            top: m.y as i32,
            right: m.x as i32 + m.width as i32,
            bottom: m.y as i32 + m.height as i32,
        };
        let dpi = if m.width_in_millimeters > 0 {
            ((m.width as f32 * 25.4 / m.width_in_millimeters as f32).round() as u32).clamp(72, 384)
        } else {
            96
        };
        out.push(Monitor {
            id: (idx as isize) + 1,
            bounds,
            work: bounds,
            dpi,
        });
    }
    if out.is_empty() {
        out.push(Monitor {
            id: 1,
            bounds: Rect {
                left: 0,
                top: 0,
                right: screen.width_in_pixels as i32,
                bottom: screen.height_in_pixels as i32,
            },
            work: Rect {
                left: 0,
                top: 0,
                right: screen.width_in_pixels as i32,
                bottom: screen.height_in_pixels as i32,
            },
            dpi: 96,
        });
    }
    Some(out)
}

pub fn virtual_bounds() -> Result<Rect> {
    let mons = all();
    let mut b = mons[0].bounds;
    for m in &mons[1..] {
        b.left = b.left.min(m.bounds.left);
        b.top = b.top.min(m.bounds.top);
        b.right = b.right.max(m.bounds.right);
        b.bottom = b.bottom.max(m.bounds.bottom);
    }
    Ok(b)
}

pub fn cursor_pos() -> Result<(i32, i32)> {
    let (conn, screen_num) =
        x11rb::connect(None).map_err(|e| PlatformError::Other(e.to_string()))?;
    let root = conn.setup().roots[screen_num].root;
    let reply = conn
        .query_pointer(root)
        .map_err(|e| PlatformError::Other(e.to_string()))?
        .reply()
        .map_err(|e| PlatformError::Other(e.to_string()))?;
    Ok((reply.root_x as i32, reply.root_y as i32))
}

pub fn primary() -> Result<Monitor> {
    all()
        .into_iter()
        .next()
        .ok_or_else(|| PlatformError::Other("no monitors found".into()))
}

pub fn under_cursor() -> Result<Monitor> {
    let (x, y) = cursor_pos().unwrap_or((0, 0));
    let mons = all();
    Ok(mons
        .iter()
        .find(|m| m.bounds.contains(x, y))
        .cloned()
        .unwrap_or_else(|| mons[0].clone()))
}

pub fn of_window(hwnd: Hwnd) -> Result<Monitor> {
    if let Ok((x, y, _, _)) = crate::window::window_rect(hwnd) {
        let mons = all();
        if let Some(m) = mons.iter().find(|m| m.bounds.contains(x, y)) {
            return Ok(m.clone());
        }
    }
    primary()
}
