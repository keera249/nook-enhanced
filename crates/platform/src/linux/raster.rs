//! Software pixel-buffer drawing for screenshots on Linux.

use crate::Result;

pub struct Canvas<'a> {
    pixels: &'a mut [u32],
    width: u32,
    height: u32,
}

pub fn text_size(text: &str, size: f32) -> (f32, f32) {
    (text.chars().count() as f32 * size * 0.6, size * 1.2)
}

pub fn average(
    pixels: &[u32],
    width: u32,
    height: u32,
    (x0, y0, x1, y1): (i32, i32, i32, i32),
) -> [u8; 3] {
    let x0 = x0.clamp(0, width as i32);
    let x1 = x1.clamp(0, width as i32);
    let y0 = y0.clamp(0, height as i32);
    let y1 = y1.clamp(0, height as i32);
    let mut r = 0u64;
    let mut g = 0u64;
    let mut b = 0u64;
    let mut n = 0u64;
    for y in y0..y1 {
        let row = y as usize * width as usize;
        for x in x0..x1 {
            let p = pixels[row + x as usize];
            r += ((p >> 16) & 0xff) as u64;
            g += ((p >> 8) & 0xff) as u64;
            b += (p & 0xff) as u64;
            n += 1;
        }
    }
    if n == 0 {
        return [0, 0, 0];
    }
    [(r / n) as u8, (g / n) as u8, (b / n) as u8]
}

impl<'a> Canvas<'a> {
    pub fn new(pixels: &'a mut [u32], width: u32, height: u32) -> Self {
        Self {
            pixels,
            width,
            height,
        }
    }

    pub fn stroke(&mut self, points: &[(f32, f32)], width: f32, rgb: [u8; 3], opacity: f32) {
        let r = (width * 0.5).max(1.0);
        let alpha = opacity.clamp(0.0, 1.0);
        for w in points.windows(2) {
            let (x0, y0) = w[0];
            let (x1, y1) = w[1];
            let steps = ((x1 - x0).hypot(y1 - y0).ceil() as i32).max(1);
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let cx = x0 + (x1 - x0) * t;
                let cy = y0 + (y1 - y0) * t;
                self.disc(cx, cy, r, rgb, alpha);
            }
        }
    }

    fn disc(&mut self, cx: f32, cy: f32, r: f32, rgb: [u8; 3], alpha: f32) {
        let min_x = ((cx - r).floor() as i32).max(0);
        let max_x = ((cx + r).ceil() as i32).min(self.width as i32 - 1);
        let min_y = ((cy - r).floor() as i32).max(0);
        let max_y = ((cy + r).ceil() as i32).min(self.height as i32 - 1);
        let r2 = r * r;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                if dx * dx + dy * dy <= r2 {
                    self.blend(x as u32, y as u32, rgb, alpha);
                }
            }
        }
    }

    fn blend(&mut self, x: u32, y: u32, rgb: [u8; 3], alpha: f32) {
        let idx = (y * self.width + x) as usize;
        let dst = self.pixels[idx];
        let dr = ((dst >> 16) & 0xff) as f32;
        let dg = ((dst >> 8) & 0xff) as f32;
        let db = (dst & 0xff) as f32;
        let nr = (dr * (1.0 - alpha) + rgb[0] as f32 * alpha).round() as u32;
        let ng = (dg * (1.0 - alpha) + rgb[1] as f32 * alpha).round() as u32;
        let nb = (db * (1.0 - alpha) + rgb[2] as f32 * alpha).round() as u32;
        self.pixels[idx] = 0xff00_0000 | (nr << 16) | (ng << 8) | nb;
    }

    pub fn fill(&mut self, (x0, y0, x1, y1): (i32, i32, i32, i32), rgb: [u8; 3]) {
        let px = 0xff00_0000 | ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32;
        for y in y0.max(0)..y1.min(self.height as i32) {
            let row = y as usize * self.width as usize;
            for x in x0.max(0)..x1.min(self.width as i32) {
                self.pixels[row + x as usize] = px;
            }
        }
    }

    pub fn average(&self, r: (i32, i32, i32, i32)) -> [u8; 3] {
        average(self.pixels, self.width, self.height, r)
    }

    pub fn text(&mut self, _at: (f32, f32), _text: &str, _size: f32, _rgb: [u8; 3]) -> Result<()> {
        Ok(())
    }
}
