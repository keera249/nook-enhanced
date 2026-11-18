//! Region snip / color pick overlay on Linux (`slurp` + X11 grab fallback).

use crate::capture::Frame;
use crate::monitor::Rect;
use crate::{Hwnd, Result};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SnipMode {
    Region,
    Screenshot,
    Pick,
}

#[derive(Clone, Debug)]
pub enum SnipOutcome {
    Region { frame: Frame, single_line: bool },
    Color { rgb: [u8; 3], at: (i32, i32) },
    Cancelled,
}

pub struct Snip {
    open: Arc<AtomicBool>,
}

impl Snip {
    pub fn open(
        mode: SnipMode,
        _own: Option<Hwnd>,
        on_done: impl FnOnce(SnipOutcome) + Send + 'static,
    ) -> Result<Self> {
        let open = Arc::new(AtomicBool::new(true));
        let o = Arc::clone(&open);
        std::thread::spawn(move || {
            let outcome = run_snip(mode);
            o.store(false, Ordering::Relaxed);
            on_done(outcome);
        });
        Ok(Self { open })
    }

    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Relaxed)
    }
}

fn run_snip(mode: SnipMode) -> SnipOutcome {
    match mode {
        SnipMode::Pick => {
            if let Ok((x, y)) = crate::monitor::cursor_pos()
                && let Ok(frame) = crate::capture::grab(Rect {
                    left: x,
                    top: y,
                    right: x + 1,
                    bottom: y + 1,
                })
            {
                return SnipOutcome::Color {
                    rgb: frame.rgb(x, y),
                    at: (x, y),
                };
            }
            SnipOutcome::Cancelled
        }
        SnipMode::Region | SnipMode::Screenshot => {
            if let Ok(out) = Command::new("slurp").args(["-f", "%x %y %w %h"]).output()
                && out.status.success()
            {
                let s = String::from_utf8_lossy(&out.stdout);
                let nums: Vec<i32> = s
                    .split_whitespace()
                    .filter_map(|n| n.parse().ok())
                    .collect();
                if nums.len() == 4
                    && let Ok(frame) = crate::capture::grab(Rect {
                        left: nums[0],
                        top: nums[1],
                        right: nums[0] + nums[2],
                        bottom: nums[1] + nums[3],
                    })
                {
                    return SnipOutcome::Region {
                        frame,
                        single_line: false,
                    };
                }
            }
            SnipOutcome::Cancelled
        }
    }
}
