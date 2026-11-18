//! Screen drawing overlay stub on Linux.

use crate::{Hwnd, Result};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum InkTool {
    Pen { color: [u8; 4], width: f32 },
    Eraser,
}

pub struct InkCanvas {
    hwnd: isize,
}

impl InkCanvas {
    pub fn open(
        _rect: crate::monitor::Rect,
        _tool: InkTool,
        _on_escape: impl Fn() + Send + 'static,
    ) -> Result<Self> {
        Ok(Self { hwnd: 0 })
    }

    pub fn hwnd(&self) -> Hwnd {
        Hwnd::from_raw(self.hwnd)
    }

    pub fn is_open(&self) -> bool {
        false
    }

    pub fn set_tool(&self, _tool: InkTool) {}
    pub fn undo(&self) {}
    pub fn clear(&self) {}
    pub fn focus(&self) {}
}
