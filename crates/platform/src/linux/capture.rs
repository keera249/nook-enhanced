//! Screen region capture on Linux (X11 GetImage).

use crate::monitor::Rect;
use crate::{PlatformError, Result};
use image::{ImageEncoder, RgbImage};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat};

#[derive(Clone, Debug)]
pub struct Frame {
    pub origin: (i32, i32),
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl Frame {
    pub fn rect(&self) -> Rect {
        Rect {
            left: self.origin.0,
            top: self.origin.1,
            right: self.origin.0 + self.width as i32,
            bottom: self.origin.1 + self.height as i32,
        }
    }

    pub fn rgb(&self, x: i32, y: i32) -> [u8; 3] {
        let lx = (x - self.origin.0).clamp(0, self.width as i32 - 1) as u32;
        let ly = (y - self.origin.1).clamp(0, self.height as i32 - 1) as u32;
        let p = self.pixels[(ly * self.width + lx) as usize];
        [((p >> 16) & 0xff) as u8, ((p >> 8) & 0xff) as u8, (p & 0xff) as u8]
    }

    pub fn crop(&self, r: Rect) -> Option<Frame> {
        crop_pixels(&self.pixels, self.width, self.origin, r)
    }

    pub fn bgra_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for &p in &self.pixels {
            out.push((p & 0xff) as u8);
            out.push(((p >> 8) & 0xff) as u8);
            out.push(((p >> 16) & 0xff) as u8);
            out.push(((p >> 24) & 0xff) as u8);
        }
        out
    }

    pub fn to_rgb_image(&self) -> RgbImage {
        let mut raw = Vec::with_capacity(self.pixels.len() * 3);
        for &p in &self.pixels {
            raw.push(((p >> 16) & 0xff) as u8);
            raw.push(((p >> 8) & 0xff) as u8);
            raw.push((p & 0xff) as u8);
        }
        RgbImage::from_raw(self.width, self.height, raw).unwrap_or_default()
    }

    pub fn encode_png(&self) -> Result<Vec<u8>> {
        let img = self.to_rgb_image();
        let mut out = Vec::new();
        image::codecs::png::PngEncoder::new(&mut out)
            .write_image(
                img.as_raw(),
                self.width,
                self.height,
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| PlatformError::Other(e.to_string()))?;
        Ok(out)
    }

    pub fn encode_jpeg(&self, max_edge: u32, quality: u8) -> Result<Vec<u8>> {
        let mut img = self.to_rgb_image();
        let (w, h) = (img.width(), img.height());
        if w.max(h) > max_edge && max_edge > 0 {
            let scale = max_edge as f32 / w.max(h) as f32;
            let nw = ((w as f32 * scale).round() as u32).max(1);
            let nh = ((h as f32 * scale).round() as u32).max(1);
            img = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);
        }
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
            .write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| PlatformError::Other(e.to_string()))?;
        Ok(out)
    }
}

pub fn crop_pixels(
    pixels: &[u32],
    width: u32,
    origin: (i32, i32),
    r: Rect,
) -> Option<Frame> {
    if width == 0 {
        return None;
    }
    let height = (pixels.len() / width as usize) as i32;
    let x0 = (r.left - origin.0).max(0);
    let y0 = (r.top - origin.1).max(0);
    let x1 = (r.right - origin.0).min(width as i32);
    let y1 = (r.bottom - origin.1).min(height);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let cw = (x1 - x0) as u32;
    let ch = (y1 - y0) as u32;
    let mut out = Vec::with_capacity((cw * ch) as usize);
    for y in y0..y1 {
        let row = y as usize * width as usize;
        out.extend_from_slice(&pixels[row + x0 as usize..row + x1 as usize]);
    }
    Some(Frame {
        origin: (origin.0 + x0, origin.1 + y0),
        width: cw,
        height: ch,
        pixels: out,
    })
}

pub fn grab(r: Rect) -> Result<Frame> {
    let w = r.width().max(1) as u16;
    let h = r.height().max(1) as u16;
    let (conn, screen_num) =
        x11rb::connect(None).map_err(|e| PlatformError::Other(e.to_string()))?;
    let root = conn.setup().roots[screen_num].root;
    let reply = conn
        .get_image(ImageFormat::Z_PIXMAP, root, r.left as i16, r.top as i16, w, h, !0)
        .map_err(|e| PlatformError::Other(e.to_string()))?
        .reply()
        .map_err(|e| PlatformError::Other(e.to_string()))?;
    let mut pixels = Vec::with_capacity(w as usize * h as usize);
    for chunk in reply.data.chunks_exact(4) {
        let b = chunk[0] as u32;
        let g = chunk[1] as u32;
        let red = chunk[2] as u32;
        pixels.push(0xff00_0000 | (red << 16) | (g << 8) | b);
    }
    Ok(Frame {
        origin: (r.left, r.top),
        width: w as u32,
        height: h as u32,
        pixels,
    })
}
