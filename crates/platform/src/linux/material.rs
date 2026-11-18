//! Material capability detection for Linux.

use crate::Hwnd;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Applied {
    Acrylic,
    LiquidGlass,
    Opaque,
}

impl Applied {
    pub fn translucent(self) -> bool {
        !matches!(self, Self::Opaque)
    }

    pub fn is_liquid(self) -> bool {
        matches!(self, Self::LiquidGlass)
    }
}

pub fn prepare(_hwnd: Hwnd, _dark: bool) {}
