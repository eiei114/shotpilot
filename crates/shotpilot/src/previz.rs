//! Local previz: a wireframe preview rendered without launching the engine.
//!
//! It draws the ground grid, the anchors (as human-scale boxes) and the look target
//! from the exact camera the engine will use. That is enough to judge framing, lens and
//! movement in milliseconds, before paying for an engine render.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::geometry::{self as g, Vec3};
use crate::model::{num, ResolvedShot};

pub type Rgb = [u8; 3];

/// 5x7 bitmap font, rows top->bottom, 5 bits each (MSB = left).
fn glyph(ch: char) -> [u8; 7] {
    match ch {
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        ' ' => [0; 7],
        '.' => [0, 0, 0, 0, 0, 0x0C, 0x0C],
        ':' => [0, 0x0C, 0x0C, 0, 0x0C, 0x0C, 0],
        '-' => [0, 0, 0, 0x1F, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 0x1F],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        '#' => [0x0A, 0x0A, 0x1F, 0x0A, 0x1F, 0x0A, 0x0A],
        '+' => [0, 0x04, 0x04, 0x1F, 0x04, 0x04, 0],
        _ => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04], // '?'
    }
}

pub struct Canvas {
    pub w: u32,
    pub h: u32,
    px: Vec<u8>,
}

impl Canvas {
    pub fn new(w: u32, h: u32) -> Canvas {
        Canvas { w, h, px: vec![0; (w * h * 3) as usize] }
    }

    fn row_fill(&mut self, y: u32, c: Rgb) {
        if y < self.h {
            let i = (y * self.w * 3) as usize;
            for px in self.px[i..i + (self.w * 3) as usize].chunks_exact_mut(3) {
                px.copy_from_slice(&c);
            }
        }
    }

    fn set(&mut self, x: i64, y: i64, c: Rgb) {
        if x >= 0 && y >= 0 && (x as u32) < self.w && (y as u32) < self.h {
            let i = ((y as u32 * self.w + x as u32) * 3) as usize;
            self.px[i..i + 3].copy_from_slice(&c);
        }
    }

    fn rect(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, c: Rgb) {
        let (x0, x1) = (x0.max(0), x1.min(self.w as i64));
        let (y0, y1) = (y0.max(0), y1.min(self.h as i64));
        for y in y0..y1 {
            for x in x0..x1 {
                self.set(x, y, c);
            }
        }
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Rgb) {
        // Liang-Barsky clip to the canvas so off-screen spans cost nothing
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let (ddx, ddy) = (x1 - x0, y1 - y0);
        let (w, h) = (self.w as f64, self.h as f64);
        for (p, q) in [(-ddx, x0), (ddx, w - 1.0 - x0), (-ddy, y0), (ddy, h - 1.0 - y0)] {
            if p == 0.0 {
                if q < 0.0 {
                    return;
                }
            } else {
                let r = q / p;
                if p < 0.0 {
                    t0 = t0.max(r);
                } else {
                    t1 = t1.min(r);
                }
            }
        }
        if t0 > t1 {
            return;
        }
        let (sx, sy, ex, ey) = (x0 + ddx * t0, y0 + ddy * t0, x0 + ddx * t1, y0 + ddy * t1);
        let n = (ex - sx).abs().max((ey - sy).abs()) as i64 + 1;
        let (dx, dy) = ((ex - sx) / n as f64, (ey - sy) / n as f64);
        let (mut x, mut y) = (sx, sy);
        for _ in 0..=n {
            self.set(x as i64, y as i64, c);
            x += dx;
            y += dy;
        }
    }

    fn text(&mut self, mut x: i64, y: i64, s: &str, c: Rgb, size: i64) {
        for ch in s.to_uppercase().chars() {
            for (ry, bits) in glyph(ch).iter().enumerate() {
                for rx in 0..5i64 {
                    if bits & (0x10 >> rx) != 0 {
                        let ry = ry as i64;
                        self.rect(x + rx * size, y + ry * size, x + (rx + 1) * size, y + (ry + 1) * size, c);
                    }
                }
            }
            x += 6 * size;
        }
    }

    /// Minimal RGB8 PNG: one zlib-compressed IDAT, filter 0 on every row.
    pub fn png(&self) -> Vec<u8> {
        let stride = (self.w * 3) as usize;
        let mut raw = Vec::with_capacity((stride + 1) * self.h as usize);
        for row in self.px.chunks_exact(stride) {
            raw.push(0);
            raw.extend_from_slice(row);
        }
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&self.w.to_be_bytes());
        ihdr.extend_from_slice(&self.h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &ihdr);
        chunk(&mut out, b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6));
        chunk(&mut out, b"IEND", &[]);
        out
    }
}

fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t) as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])]
}

/// (sky_top, sky_horizon, ground) for a sun elevation in degrees.
fn palette(elev: Option<f64>) -> (Rgb, Rgb, Rgb) {
    match elev {
        None => ([70, 110, 160], [170, 190, 210], [60, 66, 58]),
        Some(e) if e < -5.0 => ([6, 8, 20], [24, 30, 58], [14, 16, 20]),
        Some(e) if e < 12.0 => ([60, 50, 110], [240, 140, 80], [55, 42, 40]),
        Some(e) if e < 40.0 => ([60, 120, 200], [200, 210, 220], [70, 80, 60]),
        Some(_) => ([40, 110, 220], [170, 205, 240], [90, 100, 75]),
    }
}

fn anchor_color(name: &str) -> Rgb {
    let h = Sha256::digest(name.as_bytes());
    [90 + h[0] % 150, 90 + h[1] % 150, 90 + h[2] % 150]
}

pub fn render_frame(shot: &ResolvedShot, frame: i64, anchors: &[(String, Vec3)], w: u32, h: u32, ground_z: f64,
                    tint: Option<Rgb>) -> Canvas {
    let cam = shot.camera_at(frame);
    let fl = shot.focal_length();
    let sensor = shot.sensor_width();
    let mut cv = Canvas::new(w, h);
    let (mut top, mut horizon, mut ground) = palette(shot.environment.get("sun_elevation").and_then(num));
    if let Some(t) = tint {
        (top, horizon, ground) = (mix(top, t, 0.35), mix(horizon, t, 0.35), mix(ground, t, 0.35));
    }
    let (wf, hf) = (w as f64, h as f64);

    let fx = (wf / 2.0) / (g::hfov_from_focal(fl, sensor).to_radians() / 2.0).tan();
    let hy = hf / 2.0 + fx * cam.rotation.pitch.clamp(-89.0, 89.0).to_radians().tan();
    for y in 0..h {
        let yf = y as f64;
        let c = if yf < hy {
            mix(top, horizon, if hy > 0.0 { (yf / hy.max(1.0)).powf(1.5) } else { 1.0 })
        } else {
            mix(mix(horizon, ground, 0.6), ground, (yf - hy) / (hf - hy).max(1.0) * 3.0)
        };
        cv.row_fill(y, c);
    }

    // ground grid around the camera
    let grid_c = mix(ground, [230, 230, 230], 0.25);
    let (step, span) = (250.0, 4000.0);
    let cx = (cam.location[0] / step).round() * step;
    let cy = (cam.location[1] / step).round() * step;
    let n = (span / step) as i64;
    let ticks: Vec<f64> = (-n..=n).map(|i| i as f64 * step).collect();
    for axis in 0..2 {
        for &off in &ticks {
            let mut prev: Option<(f64, f64, f64)> = None;
            for &along in &ticks {
                let p = if axis == 0 { [cx + off, cy + along, ground_z] } else { [cx + along, cy + off, ground_z] };
                let q = g::project(p, &cam, fl, w, h, sensor);
                if let (Some(a), Some(b)) = (prev, q) {
                    cv.line(a.0, a.1, b.0, b.1, grid_c);
                }
                prev = q;
            }
        }
    }

    // anchors as human-scale wireframe boxes (120 x 120 x 180 cm), far to near
    let size: i64 = if w >= 640 { 2 } else { 1 };
    let mut drawn: Vec<(f64, &str, Vec3)> = anchors
        .iter()
        .filter_map(|(name, pos)| g::project(*pos, &cam, fl, w, h, sensor).map(|c| (c.2, name.as_str(), *pos)))
        .collect();
    drawn.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(b.1.cmp(a.1)));
    const EDGES: [(usize, usize); 12] =
        [(0, 1), (1, 3), (3, 2), (2, 0), (4, 5), (5, 7), (7, 6), (6, 4), (0, 4), (1, 5), (2, 6), (3, 7)];
    for (_, name, pos) in drawn {
        let col = anchor_color(name);
        let mut corners = Vec::with_capacity(8);
        for sz in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sx in [-1.0, 1.0] {
                    corners.push(g::project([pos[0] + sx * 60.0, pos[1] + sy * 60.0, pos[2] + sz * 90.0], &cam, fl, w, h,
                                            sensor));
                }
            }
        }
        for (a, b) in EDGES {
            if let (Some(p), Some(q)) = (corners[a], corners[b]) {
                cv.line(p.0, p.1, q.0, q.1, col);
            }
        }
        let tops: Vec<(f64, f64, f64)> = corners[4..].iter().flatten().copied().collect();
        if !tops.is_empty() {
            let lx = tops.iter().map(|p| p.0).fold(f64::INFINITY, f64::min) as i64;
            let ly = tops.iter().map(|p| p.1).fold(f64::INFINITY, f64::min) as i64 - 9 * size;
            let label: String = name.chars().take(16).collect();
            let max_x = w as i64 - 6 * size * label.chars().count() as i64;
            cv.text(lx.min(max_x).max(0), ly.max(10 * size), &label, col, size);
        }
    }

    if let Some(target) = shot.target {
        if let Some(t) = g::project(target, &cam, fl, w, h, sensor) {
            let r = 4.0 * size as f64;
            cv.line(t.0 - r, t.1, t.0 + r, t.1, [255, 60, 60]);
            cv.line(t.0, t.1 - r, t.0, t.1 + r, [255, 60, 60]);
        }
    }

    // rule-of-thirds guides
    for i in [1.0, 2.0] {
        let gx = (wf * i / 3.0) as i64;
        let gy = (hf * i / 3.0) as i64;
        for y in (0..h as i64).step_by(4) {
            cv.set(gx, y, [255, 255, 255]);
        }
        for x in (0..w as i64).step_by(4) {
            cv.set(x, gy, [255, 255, 255]);
        }
    }

    // overlay
    cv.rect(0, 0, w as i64, 10 * size, [0, 0, 0]);
    let head = format!("{}  F{}/{}  {}MM", shot.id, frame + 1, shot.frames, fl.round() as i64);
    cv.text(2 * size, 2 * size, &head, [255, 255, 255], size);
    for m in &shot.markers {
        let mf = m["frame"].as_i64().unwrap_or(0);
        if (mf - frame).abs() <= (shot.fps / 4).max(2) {
            cv.rect(0, h as i64 - 10 * size, w as i64, h as i64, [200, 30, 30]);
            let label: String = m["label"].as_str().unwrap_or("").chars().take(40).collect();
            cv.text(2 * size, h as i64 - 8 * size, &label, [255, 255, 255], size);
        }
    }
    cv
}

#[allow(clippy::too_many_arguments)]
pub fn render_shot(shot: &ResolvedShot, frames: &[i64], anchors: &[(String, Vec3)], out_dir: &Path, w: u32, h: u32,
                   ground_z: f64, tint: Option<Rgb>) -> Result<Vec<String>> {
    std::fs::create_dir_all(out_dir)?;
    let mut written = Vec::with_capacity(frames.len());
    for &f in frames {
        let name = format!("frame.{f:04}.png");
        std::fs::write(out_dir.join(&name), render_frame(shot, f, anchors, w, h, ground_z, tint).png())?;
        written.push(name);
    }
    Ok(written)
}
