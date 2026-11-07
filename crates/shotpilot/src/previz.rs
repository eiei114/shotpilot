//! Local previz: a graybox preview rendered without launching the engine.
//!
//! It draws the level as plain gray blocks (the shapes the engine read into
//! `scout/level.json`, D-86, or the three game's objects, D-72) with the anchors, the props that
//! pop in and the look target, from the exact camera the engine will use. A fixed key light,
//! haze with distance and ink outlines keep the shapes readable. A project whose level has not
//! been read draws its anchors as solid blocks on the ground. That is enough to judge framing,
//! lens and movement in milliseconds, before paying for an engine render.
//!
//! Only whether a level is drawn goes into the previz fingerprint ([`set_kind`]). What the level
//! holds is followed like an engine take's game content (D-37): a previz take keeps the level
//! file, or the three objects it showed, in its deps, so a change re-draws only those shots.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::geometry::{self as g, Vec3, Xform};
use crate::model::{num, vec3, AnchorBox, Definition, ResolvedShot};

pub type Rgb = [u8; 3];
type Col = [f32; 3];

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

    /// Lays `c` over a rectangle at opacity `a` (a label's backing, the header band).
    fn shade_rect(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, c: Rgb, a: f64) {
        let (x0, x1) = (x0.max(0), x1.min(self.w as i64));
        let (y0, y1) = (y0.max(0), y1.min(self.h as i64));
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y as u32 * self.w + x as u32) * 3) as usize;
                for k in 0..3 {
                    self.px[i + k] = (self.px[i + k] as f64 * (1.0 - a) + c[k] as f64 * a) as u8;
                }
            }
        }
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Rgb) {
        let Some((sx, sy, ex, ey)) = clip_2d(x0, y0, x1, y1, self.w as f64, self.h as f64) else { return };
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

    /// A name on a dark backing, so it reads over any part of the frame.
    fn label(&mut self, x: i64, y: i64, s: &str, c: Rgb, size: i64) {
        let w = 6 * size * s.chars().count() as i64;
        self.shade_rect(x - 2 * size, y - 2 * size, x + w + size, y + 9 * size, [8, 10, 14], 0.62);
        self.text(x, y, s, c, size);
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

/// Liang-Barsky: the part of a segment inside a w x h rectangle, or None.
fn clip_2d(x0: f64, y0: f64, x1: f64, y1: f64, w: f64, h: f64) -> Option<(f64, f64, f64, f64)> {
    let (t0, t1) = clip_t(x0, y0, x1, y1, w, h)?;
    let (dx, dy) = (x1 - x0, y1 - y0);
    Some((x0 + dx * t0, y0 + dy * t0, x0 + dx * t1, y0 + dy * t1))
}

fn clip_t(x0: f64, y0: f64, x1: f64, y1: f64, w: f64, h: f64) -> Option<(f64, f64)> {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let (dx, dy) = (x1 - x0, y1 - y0);
    for (p, q) in [(-dx, x0), (dx, w - 1.0 - x0), (-dy, y0), (dy, h - 1.0 - y0)] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
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
    (t0 <= t1).then_some((t0, t1))
}

fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t) as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])]
}

fn col(c: Rgb) -> Col {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0]
}

fn lerp_col(a: Col, b: Col, t: f32) -> Col {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: Vec3) -> Vec3 {
    let l = g::length(a);
    if l > 1e-12 { g::scale(a, 1.0 / l) } else { [0.0, 0.0, 1.0] }
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

/// How the time of day colours the blocks. The light stays bright enough to read shapes
/// at night: previz judges framing, not exposure.
fn tone(elev: Option<f64>) -> Col {
    match elev {
        Some(e) if e < -5.0 => [0.60, 0.66, 0.80],
        Some(e) if e < 12.0 => [0.98, 0.86, 0.76],
        _ => [1.0, 1.0, 1.0],
    }
}

fn anchor_color(name: &str) -> Rgb {
    let h = Sha256::digest(name.as_bytes());
    [90 + h[0] % 150, 90 + h[1] % 150, 90 + h[2] % 150]
}

// --------------------------------------------------------------------------- the world

/// What a block is, which sets its gray (doors orange and lamps yellow as on the blocking map).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Structure,
    Prop,
    Door,
    Fixture,
    Figure,
}

impl Kind {
    fn base(self) -> Col {
        match self {
            Kind::Structure => [0.74, 0.74, 0.72],
            Kind::Prop => [0.60, 0.62, 0.64],
            Kind::Door => [0.80, 0.57, 0.36],
            Kind::Fixture => [0.88, 0.82, 0.56],
            Kind::Figure => [0.48, 0.62, 0.82],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Block,
    /// Along its local axis 0 (x), 1 (y) or 2 (z).
    Cylinder(usize),
    Sphere,
    Figure,
}

/// One block of the level: a box turned about Z, drawn as its form.
#[derive(Debug, Clone, Copy)]
struct Solid {
    center: Vec3,
    half: Vec3,
    yaw: f64,
    kind: Kind,
    form: Form,
    /// Index into [`World::sources`].
    source: usize,
}

enum Set {
    None,
    /// The plan read from the engine (D-86): blocks and lamps that never move.
    Level { solids: Vec<Solid>, lamps: Vec<Vec3> },
    /// The three game (D-72), whose figures walk: its objects at the frame's time.
    Game(crate::three::Scene),
}

/// Everything previz draws around the shots: the anchors, the ground, and the level once read.
pub struct World {
    pub anchors: Vec<(String, AnchorBox)>,
    pub ground_z: f64,
    set: Set,
    /// (package, file) behind each block's `source`: what a take that showed it depends on.
    sources: Vec<(String, PathBuf)>,
}

/// Where the level plan read from the engine lives (D-86, written by scout::save_level).
pub fn level_file(root: &Path) -> PathBuf {
    root.join(crate::scout::SCOUT_DIR).join("level.json")
}

/// Which level previz draws for this project: "three" (its game) or "level" (the plan the
/// engine read). Only this goes into the previz fingerprint; what the level holds is followed
/// through the deps of the takes that showed it.
pub fn set_kind(defn: &Definition) -> Option<&'static str> {
    if defn.project.engine == "three" {
        crate::three::game_dir(defn).join(crate::three::SCENE_FILE).is_file().then_some("three")
    } else {
        level_file(&defn.root()).is_file().then_some("level")
    }
}

fn level_kind(kind: &str) -> Kind {
    match kind {
        "structure" => Kind::Structure,
        "door" => Kind::Door,
        "light" => Kind::Fixture,
        "character" => Kind::Figure,
        _ => Kind::Prop,
    }
}

fn parse_level(v: &Value) -> Set {
    let mut solids = Vec::new();
    for s in v["shapes"].as_array().into_iter().flatten() {
        let (Some(center), Some(half)) = (s.get("center").and_then(vec3), s.get("half").and_then(vec3)) else { continue };
        let kind = level_kind(s["kind"].as_str().unwrap_or("prop"));
        let form = if kind == Kind::Figure { Form::Figure } else { Form::Block };
        solids.push(Solid { center, half: half.map(f64::abs), yaw: s["yaw"].as_f64().unwrap_or(0.0), kind, form, source: 0 });
    }
    let lamps = v["points"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["kind"] == "light")
        .filter_map(|p| p.get("at").and_then(vec3))
        .collect();
    Set::Level { solids, lamps }
}

impl World {
    /// Anchors only, on the ground at `ground_z`.
    pub fn bare(anchors: Vec<(String, AnchorBox)>, ground_z: f64) -> World {
        World { anchors, ground_z, set: Set::None, sources: Vec::new() }
    }

    /// What previz draws for this project: its anchors, and the level when one was read (or
    /// the three game). A level that does not load is left out (the anchors still draw).
    pub fn load(defn: &Definition) -> World {
        let ground_z = defn.defaults.get("previz").and_then(|p| p.get("ground_z")).and_then(num).unwrap_or(0.0);
        let mut w = World::bare(defn.anchor_boxes(), ground_z);
        match set_kind(defn) {
            Some("three") => {
                if let Ok(scene) = crate::three::Scene::load(&crate::three::game_dir(defn)) {
                    w.sources = scene
                        .objects
                        .iter()
                        .map(|o| (format!("game/{}/{}", crate::three::OBJECTS_DIR, o.name), o.file.clone()))
                        .collect();
                    w.set = Set::Game(scene);
                }
            }
            Some("level") => {
                let root = defn.root();
                if let Some(v) = crate::scout::load_level(&root) {
                    w.set = parse_level(&v);
                    w.sources = vec![("scout/level.json".into(), level_file(&root))];
                }
            }
            _ => {}
        }
        w
    }

    pub fn has_level(&self) -> bool {
        !matches!(self.set, Set::None)
    }

    /// The blocks and lamps at `t` seconds into a shot.
    fn level_at(&self, t: f64) -> (Vec<Solid>, Vec<Vec3>) {
        use crate::three::Form as F;
        match &self.set {
            Set::None => (Vec::new(), Vec::new()),
            Set::Level { solids, lamps } => (solids.clone(), lamps.clone()),
            Set::Game(scene) => {
                let (mut solids, mut lamps) = (Vec::new(), Vec::new());
                for (i, o) in scene.objects.iter().enumerate() {
                    let door = o.name.contains("door");
                    for (b, form) in o.forms_at(t) {
                        let form = match form {
                            F::Dust => continue,
                            F::Light => {
                                // moonlight and the like light the building from outside it
                                if !o.structure {
                                    lamps.push(b.center);
                                }
                                continue;
                            }
                            F::Box => Form::Block,
                            F::Cylinder(a) => Form::Cylinder(a),
                            F::Sphere => Form::Sphere,
                            F::Figure => Form::Figure,
                        };
                        let kind = match () {
                            _ if form == Form::Figure => Kind::Figure,
                            _ if o.structure => Kind::Structure,
                            _ if door => Kind::Door,
                            _ => Kind::Prop,
                        };
                        solids.push(Solid { center: b.center, half: b.half, yaw: b.yaw, kind, form, source: i });
                    }
                }
                (solids, lamps)
            }
        }
    }
}

// --------------------------------------------------------------------------- drawing

/// Nothing nearer than this (cm) is drawn: what the camera stands inside is cut away.
const NEAR: f64 = 2.0;
/// Distance (cm) over which the haze closes in.
const HAZE: f64 = 6000.0;
const SKY: u32 = 0;
const GROUND: u32 = 1;

/// The shot's camera as a pinhole, in the units of the buffer it draws into. Same projection
/// as `geometry::project`.
struct View {
    eye: Vec3,
    f: Vec3,
    r: Vec3,
    u: Vec3,
    /// sin and cos of -roll.
    rs: f64,
    rc: f64,
    fx: f64,
    cx: f64,
    cy: f64,
}

impl View {
    fn new(cam: &Xform, focal: f64, sensor: f64, w: usize, h: usize) -> View {
        let (rs, rc) = (-cam.rotation.roll).to_radians().sin_cos();
        let fx = (w as f64 / 2.0) / (g::hfov_from_focal(focal, sensor).to_radians() / 2.0).tan();
        View {
            eye: cam.location,
            f: g::forward(cam.rotation),
            r: g::right(cam.rotation),
            u: g::up(cam.rotation),
            rs,
            rc,
            fx,
            cx: w as f64 / 2.0,
            cy: h as f64 / 2.0,
        }
    }

    /// Camera space: (right, up, depth).
    fn cam(&self, p: Vec3) -> Vec3 {
        let d = g::sub(p, self.eye);
        let (x, y) = (dot(d, self.r), dot(d, self.u));
        [x * self.rc - y * self.rs, x * self.rs + y * self.rc, dot(d, self.f)]
    }

    /// Pixel position and 1/depth of a camera-space point in front of the camera.
    fn screen(&self, c: Vec3) -> [f64; 3] {
        [self.cx + self.fx * c[0] / c[2], self.cy - self.fx * c[1] / c[2], 1.0 / c[2]]
    }

    fn project(&self, p: Vec3) -> Option<[f64; 3]> {
        let c = self.cam(p);
        (c[2] > NEAR).then(|| self.screen(c))
    }

    /// The world direction through a pixel; its part along the view is 1, so a hit at
    /// `eye + t * ray` lies at depth t.
    fn ray(&self, px: f64, py: f64) -> Vec3 {
        let (xr, yr) = ((px - self.cx) / self.fx, (self.cy - py) / self.fx);
        let (x, y) = (xr * self.rc + yr * self.rs, -xr * self.rs + yr * self.rc);
        [self.f[0] + self.r[0] * x + self.u[0] * y, self.f[1] + self.r[1] * x + self.u[1] * y, self.f[2] + self.r[2] * x + self.u[2] * y]
    }
}

/// Sutherland-Hodgman against the near plane, in camera space.
fn clip_near(p: &[Vec3]) -> Vec<Vec3> {
    let mut out = Vec::with_capacity(p.len() + 2);
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        let (ina, inb) = (a[2] >= NEAR, b[2] >= NEAR);
        if ina {
            out.push(a);
        }
        if ina != inb {
            let t = (NEAR - a[2]) / (b[2] - a[2]);
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, NEAR]);
        }
    }
    out
}

/// One frame being drawn: colour, 1/depth (0 = nothing) and which face covers each pixel.
struct Frame {
    w: usize,
    h: usize,
    view: View,
    col: Vec<Col>,
    inv: Vec<f32>,
    id: Vec<u32>,
    /// The level source behind each face id (None for anchors, props and the backdrop).
    face_src: Vec<Option<usize>>,
    /// Direction toward the key light.
    key: Vec3,
    tone: Col,
}

impl Frame {
    fn new_face(&mut self, src: Option<usize>) -> u32 {
        self.face_src.push(src);
        (self.face_src.len() - 1) as u32
    }

    /// Flat light for a face: a key light, a sky-over-ground fill and a little light from the
    /// camera, so no face goes black.
    fn lit(&self, base: Col, n: Vec3, at: Vec3) -> Col {
        let key = dot(n, self.key).max(0.0);
        let hemi = 0.5 + 0.5 * n[2];
        let head = dot(n, unit(g::sub(self.view.eye, at))).max(0.0);
        let s = (0.26 + 0.24 * hemi + 0.36 * key + 0.20 * head) as f32;
        [base[0] * s * self.tone[0], base[1] * s * self.tone[1], base[2] * s * self.tone[2]]
    }

    /// A flat convex polygon (world points), culled when it faces away.
    fn poly(&mut self, pts: &[Vec3], n: Vec3, base: Col, id: u32) {
        if dot(n, g::sub(self.view.eye, pts[0])) <= 0.0 {
            return;
        }
        let mut mid = [0.0; 3];
        for p in pts {
            mid = g::add(mid, *p);
        }
        let c = self.lit(base, n, g::scale(mid, 1.0 / pts.len() as f64));
        let cam: Vec<Vec3> = pts.iter().map(|p| self.view.cam(*p)).collect();
        let clipped = clip_near(&cam);
        if clipped.len() < 3 {
            return;
        }
        let s: Vec<[f64; 3]> = clipped.iter().map(|p| self.view.screen(*p)).collect();
        for i in 1..s.len() - 1 {
            self.tri(s[0], s[i], s[i + 1], c, id);
        }
    }

    fn tri(&mut self, a: [f64; 3], b: [f64; 3], c: [f64; 3], color: Col, id: u32) {
        let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        if area.abs() < 1e-12 || !area.is_finite() {
            return;
        }
        let (wf, hf) = (self.w as f64, self.h as f64);
        let x0 = a[0].min(b[0]).min(c[0]).floor().max(0.0);
        let x1 = a[0].max(b[0]).max(c[0]).ceil().min(wf - 1.0);
        let y0 = a[1].min(b[1]).min(c[1]).floor().max(0.0);
        let y1 = a[1].max(b[1]).max(c[1]).ceil().min(hf - 1.0);
        if x0 > x1 || y0 > y1 {
            return;
        }
        let inv_area = 1.0 / area;
        // edge functions: e(p, q) at a pixel is the weight of the vertex opposite p-q, times area
        let e = |p: [f64; 3], q: [f64; 3], x: f64, y: f64| (q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0]);
        let (d0, d1, d2) = (-(c[1] - b[1]) * inv_area, -(a[1] - c[1]) * inv_area, -(b[1] - a[1]) * inv_area);
        let eps = -1e-9;
        for y in y0 as usize..=y1 as usize {
            let (py, px) = (y as f64 + 0.5, x0 + 0.5);
            let mut l0 = e(b, c, px, py) * inv_area;
            let mut l1 = e(c, a, px, py) * inv_area;
            let mut l2 = e(a, b, px, py) * inv_area;
            let row = y * self.w;
            for x in x0 as usize..=x1 as usize {
                if l0 >= eps && l1 >= eps && l2 >= eps {
                    let z = (l0 * a[2] + l1 * b[2] + l2 * c[2]) as f32;
                    let i = row + x;
                    if z > self.inv[i] {
                        self.inv[i] = z;
                        self.col[i] = color;
                        self.id[i] = id;
                    }
                }
                l0 += d0;
                l1 += d1;
                l2 += d2;
            }
        }
    }

    /// Sky by the height of each ray, and the ground plane with a 1 m and 5 m grid that
    /// fades where its lines would shimmer.
    fn backdrop(&mut self, ground_z: f64, top: Rgb, horizon: Rgb, ground: Rgb) {
        let (top, horizon, ground) = (col(top), col(horizon), col(ground));
        let above = self.view.eye[2] > ground_z + 0.5;
        let lit_ground = self.lit([1.0, 1.0, 1.0], [0.0, 0.0, 1.0], [self.view.eye[0], self.view.eye[1], ground_z]);
        let ground = [ground[0] * lit_ground[0] * 1.6, ground[1] * lit_ground[1] * 1.6, ground[2] * lit_ground[2] * 1.6];
        let line = lerp_col(ground, [0.92, 0.92, 0.90], 0.22);
        let major = lerp_col(ground, [0.95, 0.95, 0.93], 0.42);
        for y in 0..self.h {
            for x in 0..self.w {
                let i = y * self.w + x;
                let dir = self.view.ray(x as f64 + 0.5, y as f64 + 0.5);
                let len = g::length(dir);
                let ez = dir[2] / len;
                if above && ez < -1e-4 {
                    let t = (ground_z - self.view.eye[2]) / dir[2];
                    let (px, py) = (self.view.eye[0] + dir[0] * t, self.view.eye[1] + dir[1] * t);
                    // cm of ground per pixel, stretched where the ray grazes it
                    let foot = t * len / self.view.fx / (-ez).max(0.03);
                    let grid = |step: f64| {
                        let fade = (1.0 - foot / (step * 0.3)).clamp(0.0, 1.0);
                        if fade <= 0.0 {
                            return 0.0;
                        }
                        let near = |v: f64| (v / step - (v / step).round()).abs() * step;
                        let d = near(px).min(near(py));
                        (1.0 - d / (0.8 * foot)).clamp(0.0, 1.0) * fade
                    };
                    let (m, n) = (grid(500.0), grid(100.0));
                    let c = lerp_col(lerp_col(ground, line, n as f32), major, m as f32);
                    self.col[i] = c;
                    // a hair farther than the plane, so floors resting on it win
                    self.inv[i] = (1.0 / (t * 1.0005)) as f32;
                    self.id[i] = GROUND;
                } else {
                    let s = (ez / 0.55).clamp(0.0, 1.0).powf(0.7) as f32;
                    self.col[i] = if ez >= 0.0 { lerp_col(horizon, top, s) } else { horizon };
                    self.inv[i] = 0.0;
                    self.id[i] = SKY;
                }
            }
        }
    }

    /// Ink where one face meets another (or the sky), two pixels wide.
    fn outlines(&mut self) {
        let (w, h) = (self.w, self.h);
        let mut ink = vec![false; w * h];
        let pair = |a: u32, b: u32| a != b && !(a <= GROUND && b <= GROUND);
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if x + 1 < w && pair(self.id[i], self.id[i + 1]) {
                    ink[i] = true;
                    ink[i + 1] = true;
                }
                if y + 1 < h && pair(self.id[i], self.id[i + w]) {
                    ink[i] = true;
                    ink[i + w] = true;
                }
            }
        }
        for (c, on) in self.col.iter_mut().zip(ink) {
            if on {
                *c = lerp_col(*c, [0.05, 0.05, 0.07], 0.72);
            }
        }
    }

    /// Haze toward the horizon colour with distance.
    fn haze(&mut self, horizon: Rgb) {
        let fog = col(horizon);
        for (c, &inv) in self.col.iter_mut().zip(&self.inv) {
            if inv > 0.0 {
                let f = ((1.0 - (-(1.0 / inv as f64) / HAZE).exp()) * 0.8) as f32;
                *c = lerp_col(*c, fog, f);
            }
        }
    }

    /// A soft glow where a lamp is in sight.
    fn glow(&mut self, at: Vec3, ss: f64) {
        let Some([sx, sy, inv]) = self.view.project(at) else { return };
        let (xi, yi) = (sx.floor(), sy.floor());
        if xi < 0.0 || yi < 0.0 || xi >= self.w as f64 || yi >= self.h as f64 {
            return;
        }
        // in front of what covers its pixel, or just behind it (the lamp inside its fitting)
        let cover = self.inv[yi as usize * self.w + xi as usize] as f64;
        if inv < cover * 0.9 {
            return;
        }
        let r = (self.view.fx * 35.0 * inv).clamp(2.5 * ss, 28.0 * ss);
        let warm = [1.0f32, 0.86, 0.55];
        let (x0, x1) = ((sx - r).floor().max(0.0) as usize, ((sx + r).ceil() as usize).min(self.w - 1));
        let (y0, y1) = ((sy - r).floor().max(0.0) as usize, ((sy + r).ceil() as usize).min(self.h - 1));
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f64 + 0.5 - sx).powi(2) + (y as f64 + 0.5 - sy).powi(2)).sqrt() / r;
                if d < 1.0 {
                    let a = ((1.0 - d) * (1.0 - d) * 0.9) as f32;
                    let c = &mut self.col[y * self.w + x];
                    for k in 0..3 {
                        c[k] = (c[k] + warm[k] * a).min(1.0);
                    }
                }
            }
        }
    }

    /// A world-space segment, solid where it is in sight and dashed and faint where something
    /// stands in front of it.
    fn wire(&mut self, a: Vec3, b: Vec3, color: Col, ss: usize) {
        let (mut ca, mut cb) = (self.view.cam(a), self.view.cam(b));
        if ca[2] < NEAR && cb[2] < NEAR {
            return;
        }
        if ca[2] < NEAR {
            let t = (NEAR - ca[2]) / (cb[2] - ca[2]);
            ca = [ca[0] + (cb[0] - ca[0]) * t, ca[1] + (cb[1] - ca[1]) * t, NEAR];
        } else if cb[2] < NEAR {
            let t = (NEAR - cb[2]) / (ca[2] - cb[2]);
            cb = [cb[0] + (ca[0] - cb[0]) * t, cb[1] + (ca[1] - cb[1]) * t, NEAR];
        }
        let (sa, sb) = (self.view.screen(ca), self.view.screen(cb));
        let Some((t0, t1)) = clip_t(sa[0], sa[1], sb[0], sb[1], self.w as f64, self.h as f64) else { return };
        let len = ((sb[0] - sa[0]).powi(2) + (sb[1] - sa[1]).powi(2)).sqrt() * (t1 - t0);
        let n = len.ceil().max(1.0) as usize;
        let dash = 4 * ss;
        for k in 0..=n {
            let t = t0 + (t1 - t0) * k as f64 / n as f64;
            let (x, y) = (sa[0] + (sb[0] - sa[0]) * t, sa[1] + (sb[1] - sa[1]) * t);
            let inv = sa[2] + (sb[2] - sa[2]) * t;
            let (xi, yi) = (x.floor().clamp(0.0, self.w as f64 - 1.0) as usize, y.floor().clamp(0.0, self.h as f64 - 1.0) as usize);
            let shown = inv as f32 >= self.inv[yi * self.w + xi] * 0.985;
            if !shown && (k / dash) % 2 == 1 {
                continue;
            }
            let a = if shown { 1.0 } else { 0.45 };
            for dy in 0..ss {
                for dx in 0..ss {
                    let (px, py) = (xi + dx, yi + dy);
                    if px < self.w && py < self.h {
                        let c = &mut self.col[py * self.w + px];
                        *c = lerp_col(*c, color, a);
                    }
                }
            }
        }
    }

    /// Averages each ss x ss block into the canvas.
    fn resolve(&self, ss: usize) -> Canvas {
        let (w, h) = (self.w / ss, self.h / ss);
        let mut cv = Canvas::new(w as u32, h as u32);
        let k = 1.0 / (ss * ss) as f32;
        for y in 0..h {
            for x in 0..w {
                let mut s = [0.0f32; 3];
                for dy in 0..ss {
                    let row = (y * ss + dy) * self.w + x * ss;
                    for c in &self.col[row..row + ss] {
                        s[0] += c[0];
                        s[1] += c[1];
                        s[2] += c[2];
                    }
                }
                let i = (y * w + x) * 3;
                for c in 0..3 {
                    cv.px[i + c] = ((s[c] * k).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                }
            }
        }
        cv
    }
}

/// The corners of a box turned by `yaw` about Z: x varies fastest, then y, then z.
fn corners(center: Vec3, half: Vec3, yaw: f64) -> [Vec3; 8] {
    let (s, c) = yaw.to_radians().sin_cos();
    let mut out = [[0.0; 3]; 8];
    let mut i = 0;
    for sz in [-1.0, 1.0] {
        for sy in [-1.0, 1.0] {
            for sx in [-1.0, 1.0] {
                let (lx, ly) = (sx * half[0], sy * half[1]);
                out[i] = [center[0] + lx * c - ly * s, center[1] + lx * s + ly * c, center[2] + sz * half[2]];
                i += 1;
            }
        }
    }
    out
}

const EDGES: [(usize, usize); 12] = [(0, 1), (1, 3), (3, 2), (2, 0), (4, 5), (5, 7), (7, 6), (6, 4), (0, 4), (1, 5), (2, 6), (3, 7)];

/// The six faces of a box as corner indices (outward), with the local axis each faces.
const FACES: [([usize; 4], usize, f64); 6] = [
    ([1, 3, 7, 5], 0, 1.0),
    ([0, 4, 6, 2], 0, -1.0),
    ([2, 6, 7, 3], 1, 1.0),
    ([0, 1, 5, 4], 1, -1.0),
    ([4, 5, 7, 6], 2, 1.0),
    ([0, 2, 3, 1], 2, -1.0),
];

/// A local direction turned by `yaw` about Z.
fn turn(v: Vec3, yaw: f64) -> Vec3 {
    let (s, c) = yaw.to_radians().sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]]
}

fn draw_box(fr: &mut Frame, center: Vec3, half: Vec3, yaw: f64, base: Col, src: Option<usize>) {
    let cs = corners(center, half, yaw);
    for (idx, axis, sign) in FACES {
        let mut n = [0.0; 3];
        n[axis] = sign;
        let id = fr.new_face(src);
        fr.poly(&[cs[idx[0]], cs[idx[1]], cs[idx[2]], cs[idx[3]]], turn(n, yaw), base, id);
    }
}

fn draw_cylinder(fr: &mut Frame, s: &Solid, axis: usize, base: Col) {
    const SEG: usize = 14;
    let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
    let (ra, rb, hl) = (s.half[a], s.half[b], s.half[axis]);
    let local = |ang: f64, h: f64| {
        let mut v = [0.0; 3];
        v[a] = ra * ang.cos();
        v[b] = rb * ang.sin();
        v[axis] = h;
        v
    };
    let world = |v: Vec3| g::add(s.center, turn(v, s.yaw));
    let side = fr.new_face(Some(s.source));
    for k in 0..SEG {
        let (t0, t1) = (k as f64 / SEG as f64 * std::f64::consts::TAU, (k + 1) as f64 / SEG as f64 * std::f64::consts::TAU);
        let mut n = [0.0; 3];
        let tm = (t0 + t1) / 2.0;
        n[a] = tm.cos() / ra.max(1e-6);
        n[b] = tm.sin() / rb.max(1e-6);
        let quad = [world(local(t0, -hl)), world(local(t1, -hl)), world(local(t1, hl)), world(local(t0, hl))];
        fr.poly(&quad, unit(turn(n, s.yaw)), base, side);
    }
    for sign in [1.0, -1.0] {
        let mut ring: Vec<Vec3> = (0..SEG).map(|k| world(local(k as f64 / SEG as f64 * std::f64::consts::TAU, sign * hl))).collect();
        if sign < 0.0 {
            ring.reverse();
        }
        let mut n = [0.0; 3];
        n[axis] = sign;
        let id = fr.new_face(Some(s.source));
        fr.poly(&ring, turn(n, s.yaw), base, id);
    }
}

fn draw_sphere(fr: &mut Frame, s: &Solid, base: Col) {
    const LAT: usize = 6;
    const LON: usize = 10;
    let id = fr.new_face(Some(s.source));
    let at = |i: usize, j: usize| {
        let (th, ph) = (std::f64::consts::PI * i as f64 / LAT as f64, std::f64::consts::TAU * j as f64 / LON as f64);
        [th.sin() * ph.cos(), th.sin() * ph.sin(), th.cos()]
    };
    let place = |v: Vec3| g::add(s.center, turn([v[0] * s.half[0], v[1] * s.half[1], v[2] * s.half[2]], s.yaw));
    for i in 0..LAT {
        for j in 0..LON {
            let (p0, p1, p2, p3) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
            let n = unit(turn(g::scale(g::add(g::add(p0, p1), g::add(p2, p3)), 0.25), s.yaw));
            fr.poly(&[place(p0), place(p1), place(p2), place(p3)], n, base, id);
        }
    }
}

/// A standing figure as a body and a head; one lying down stays a single block.
fn draw_figure(fr: &mut Frame, s: &Solid, base: Col) {
    let h = 2.0 * s.half[2];
    if s.half[2] < 1.2 * s.half[0].max(s.half[1]) {
        draw_box(fr, s.center, s.half, s.yaw, base, Some(s.source));
        return;
    }
    let foot = s.center[2] - s.half[2];
    let body = [s.half[0] * 0.8, s.half[1] * 0.8, h * 0.41];
    draw_box(fr, [s.center[0], s.center[1], foot + body[2]], body, s.yaw, base, Some(s.source));
    let r = s.half[0].min(s.half[1]) * 0.62;
    let head = [r, r, h * 0.075];
    draw_box(fr, [s.center[0], s.center[1], foot + h - head[2]], head, s.yaw, base, Some(s.source));
}

/// Cheap reject: every corner behind the near plane, or all beyond one side of the picture.
fn off_screen(fr: &Frame, cs: &[Vec3; 8]) -> bool {
    let cam: Vec<Vec3> = cs.iter().map(|p| fr.view.cam(*p)).collect();
    let (tx, ty) = (fr.view.cx / fr.view.fx, fr.view.cy / fr.view.fx);
    let all = |f: &dyn Fn(&Vec3) -> bool| cam.iter().all(f);
    all(&|c| c[2] < NEAR)
        || all(&|c| c[0] > tx * c[2])
        || all(&|c| c[0] < -tx * c[2])
        || all(&|c| c[1] > ty * c[2])
        || all(&|c| c[1] < -ty * c[2])
}

pub fn render_frame(shot: &ResolvedShot, frame: i64, world: &World, w: u32, h: u32, tint: Option<Rgb>) -> Canvas {
    draw_frame(shot, frame, world, w, h, tint, true).0
}

/// The frame without the framing guides and the shot header: the layout an image model
/// starts from when it paints a storyboard frame (anchor labels stay, they name the boxes).
pub fn render_layout(shot: &ResolvedShot, frame: i64, world: &World, w: u32, h: u32) -> Canvas {
    draw_frame(shot, frame, world, w, h, None, false).0
}

/// The frame, and the level sources it showed.
fn draw_frame(shot: &ResolvedShot, frame: i64, world: &World, w: u32, h: u32, tint: Option<Rgb>, guides: bool)
    -> (Canvas, BTreeSet<usize>) {
    // small frames are drawn at twice the size and averaged down; the engine stand-in (tinted)
    // and large frames are drawn once
    let ss = if tint.is_none() && (w as u64 * h as u64) <= 500_000 { 2usize } else { 1 };
    let (bw, bh) = (w as usize * ss, h as usize * ss);
    let cam = shot.camera_at(frame);
    let fl = shot.focal_length();
    let sensor = shot.sensor_width();
    let elev = shot.environment.get("sun_elevation").and_then(num);
    let (mut top, mut horizon, mut ground) = palette(elev);
    if let Some(t) = tint {
        (top, horizon, ground) = (mix(top, t, 0.35), mix(horizon, t, 0.35), mix(ground, t, 0.35));
    }
    let key = {
        let e = elev.unwrap_or(50.0).clamp(28.0, 65.0).to_radians();
        let y = shot.environment.get("sun_yaw").and_then(num).unwrap_or(45.0).to_radians();
        unit([-e.cos() * y.cos(), -e.cos() * y.sin(), e.sin()])
    };
    let mut fr = Frame {
        w: bw,
        h: bh,
        view: View::new(&cam, fl, sensor, bw, bh),
        col: vec![[0.0; 3]; bw * bh],
        inv: vec![0.0; bw * bh],
        id: vec![SKY; bw * bh],
        face_src: vec![None, None],
        key,
        tone: tone(elev),
    };
    fr.backdrop(world.ground_z, top, horizon, ground);

    let t = frame as f64 / shot.fps.max(1) as f64;
    let (solids, lamps) = world.level_at(t);
    for s in &solids {
        if off_screen(&fr, &corners(s.center, s.half, s.yaw)) {
            continue;
        }
        let base = s.kind.base();
        match s.form {
            Form::Block => draw_box(&mut fr, s.center, s.half, s.yaw, base, Some(s.source)),
            Form::Cylinder(axis) => draw_cylinder(&mut fr, s, axis, base),
            Form::Sphere => draw_sphere(&mut fr, s, base),
            Form::Figure => draw_figure(&mut fr, s, base),
        }
    }
    // with no level, the anchors are the set: solid blocks, unless the camera stands inside one
    let solid_anchors = !world.has_level();
    let mut wired: Vec<(&str, AnchorBox)> = Vec::new();
    for (name, b) in &world.anchors {
        let half = g::scale(b.size, 0.5);
        if solid_anchors && !b.contains(cam.location, 0.0) {
            let base = lerp_col(col(anchor_color(name)), [0.62, 0.62, 0.62], 0.45);
            draw_box(&mut fr, b.center, half, 0.0, base, None);
        } else {
            wired.push((name.as_str(), *b));
        }
    }
    // props that have appeared by this frame
    for ap in shot.appear.iter().filter(|a| frame >= a.frame) {
        let b = ap.bbox();
        draw_box(&mut fr, b.center, g::scale(b.size, 0.5), ap.yaw, [0.88, 0.34, 0.30], None);
    }

    let mut shown = vec![false; fr.face_src.len()];
    for &i in &fr.id {
        shown[i as usize] = true;
    }
    let seen: BTreeSet<usize> = fr.face_src.iter().zip(shown).filter(|(_, on)| *on).filter_map(|(s, _)| *s).collect();
    fr.outlines();
    fr.haze(horizon);
    for l in &lamps {
        fr.glow(*l, ss as f64);
    }
    for (name, b) in &wired {
        let c = col(anchor_color(name));
        let cs = b.corners();
        for (i, j) in EDGES {
            fr.wire(cs[i], cs[j], c, ss);
        }
    }
    let mut cv = fr.resolve(ss);

    // labels and marks at the final size, so they stay crisp
    let size: i64 = if w >= 640 { 2 } else { 1 };
    let view = View::new(&cam, fl, sensor, w as usize, h as usize);
    let on_screen = |p: &[f64; 3]| p[0] >= 0.0 && p[1] >= 0.0 && p[0] < w as f64 && p[1] < h as f64;
    let mut labels: Vec<(f64, String, Rgb, Vec<[f64; 3]>)> = Vec::new();
    for (name, b) in &world.anchors {
        let cs = b.corners();
        let pts: Vec<[f64; 3]> = cs.iter().filter_map(|p| view.project(*p)).collect();
        if pts.iter().any(on_screen) {
            let depth = view.cam(b.center)[2];
            labels.push((depth, name.chars().take(16).collect(), anchor_color(name), pts));
        }
    }
    for ap in shot.appear.iter().filter(|a| frame >= a.frame) {
        let cs = corners(ap.bbox().center, g::scale(ap.size, 0.5), ap.yaw);
        let pts: Vec<[f64; 3]> = cs.iter().filter_map(|p| view.project(*p)).collect();
        if pts.iter().any(on_screen) {
            labels.push((view.cam(ap.location)[2], ap.prop.to_uppercase().chars().take(16).collect(), [255, 120, 110], pts));
        }
    }
    // far to near, so the nearest name is on top
    labels.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(b.1.cmp(&a.1)));
    for (_, label, c, pts) in &labels {
        let lx = pts.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min).max(0.0) as i64;
        let ly = pts.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min) as i64 - 10 * size;
        let max_x = w as i64 - 6 * size * label.chars().count() as i64 - 2 * size;
        cv.label(lx.min(max_x).max(2 * size), ly.max(12 * size), label, *c, size);
    }

    if let Some(target) = shot.target {
        if let Some(t) = view.project(target).filter(on_screen) {
            let r = 5.0 * size as f64;
            for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)] {
                cv.line(t[0] - r + dx, t[1] + dy, t[0] + r + dx, t[1] + dy, [255, 60, 60]);
                cv.line(t[0] + dx, t[1] - r + dy, t[0] + dx, t[1] + r + dy, [255, 60, 60]);
            }
        }
    }

    if guides {
        // rule-of-thirds guides
        for i in [1.0, 2.0] {
            let gx = (w as f64 * i / 3.0) as i64;
            let gy = (h as f64 * i / 3.0) as i64;
            for y in (0..h as i64).step_by(4) {
                cv.set(gx, y, [235, 235, 235]);
            }
            for x in (0..w as i64).step_by(4) {
                cv.set(x, gy, [235, 235, 235]);
            }
        }

        // overlay
        cv.shade_rect(0, 0, w as i64, 10 * size, [0, 0, 0], 0.7);
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
    }
    (cv, seen)
}

/// What a shot's previz wrote: its frame files, and the level it showed as `[{package, file}]`
/// for the take's deps (content::record).
pub struct Drawn {
    pub files: Vec<String>,
    pub deps: Vec<Value>,
}

pub fn render_shot(shot: &ResolvedShot, frames: &[i64], world: &World, out_dir: &Path, w: u32, h: u32, tint: Option<Rgb>)
    -> Result<Drawn> {
    std::fs::create_dir_all(out_dir)?;
    let mut files = Vec::with_capacity(frames.len());
    let mut seen = BTreeSet::new();
    for &f in frames {
        let name = format!("frame.{f:04}.png");
        let (cv, s) = draw_frame(shot, f, world, w, h, tint, true);
        std::fs::write(out_dir.join(&name), cv.png())?;
        files.push(name);
        seen.extend(s);
    }
    // the level plan is one file: a take that drew it depends on it whether or not a block was
    // in frame (it may hold one that should have been)
    if matches!(world.set, Set::Level { .. }) {
        seen.insert(0);
    }
    let deps = seen
        .iter()
        .filter_map(|&i| world.sources.get(i))
        .map(|(package, file)| json!({"package": package, "file": file.display().to_string()}))
        .collect();
    Ok(Drawn { files, deps })
}
