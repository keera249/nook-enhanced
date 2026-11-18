//! Cursor hit-testing against the notch polygons on Linux.

use crate::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Default)]
pub struct HitShape {
    pub polygons: Vec<Vec<(f32, f32)>>,
}

impl HitShape {
    pub fn contains(&self, (x, y): (f32, f32)) -> bool {
        self.polygons
            .iter()
            .any(|poly| polygon_contains(poly, (x, y)))
    }
}

pub fn polygon_contains(poly: &[(f32, f32)], (x, y): (f32, f32)) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        let intersect = ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

struct Shared {
    shape: Mutex<HitShape>,
    inside: AtomicBool,
    stop: AtomicBool,
}

pub struct HitTester {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl HitTester {
    pub fn spawn(on_change: impl Fn(bool) + Send + 'static) -> Result<Self> {
        let shared = Arc::new(Shared {
            shape: Mutex::new(HitShape::default()),
            inside: AtomicBool::new(false),
            stop: AtomicBool::new(false),
        });
        let s = Arc::clone(&shared);
        let thread = thread::spawn(move || {
            while !s.stop.load(Ordering::Relaxed) {
                if let Ok((cx, cy)) = crate::monitor::cursor_pos() {
                    let hit = s
                        .shape
                        .lock()
                        .map(|sh| sh.contains((cx as f32, cy as f32)))
                        .unwrap_or(false);
                    let prev = s.inside.swap(hit, Ordering::Relaxed);
                    if prev != hit {
                        on_change(hit);
                    }
                }
                thread::sleep(Duration::from_millis(16));
            }
        });
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    pub fn set_shape(&self, shape: HitShape) {
        if let Ok(mut g) = self.shared.shape.lock() {
            *g = shape;
        }
    }

    pub fn inside(&self) -> bool {
        self.shared.inside.load(Ordering::Relaxed)
    }
}

impl Drop for HitTester {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
