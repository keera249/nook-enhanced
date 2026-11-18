//! Backdrop stub for Linux (egui renders the glass fill directly).

use crate::{Hwnd, Result};

#[derive(Copy, Clone, PartialEq)]
pub struct Tint {
    pub top: [u8; 4],
    pub upper_mid: [u8; 4],
    pub lower_mid: [u8; 4],
    pub bottom: [u8; 4],
    pub y0: f32,
    pub y1: f32,
    pub blur: bool,
    pub rim_top: [u8; 4],
    pub rim_mid: [u8; 4],
    pub rim_bottom: [u8; 4],
}

pub struct Backdrop {
    host: Hwnd,
}

impl Backdrop {
    pub fn new(main: Hwnd) -> Result<Self> {
        Ok(Self { host: main })
    }

    pub fn host(&self) -> Hwnd {
        self.host
    }

    pub fn follow(&mut self, _main: Hwnd) -> Result<()> {
        Ok(())
    }

    pub fn set_shape(&mut self, _polygon: &[(f32, f32)]) -> Result<()> {
        Ok(())
    }

    pub fn set_tint(&mut self, _tint: Tint) -> Result<()> {
        Ok(())
    }

    pub fn set_visible(&mut self, _visible: bool) -> Result<()> {
        Ok(())
    }
}
