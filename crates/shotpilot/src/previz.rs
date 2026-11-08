//! Local previz: a graybox preview rendered without launching the engine.
//!
//! It draws the level (the shapes the engine read into `scout/level.json`, D-86, or the three
//! game's objects, D-72) with the anchors, the props that pop in and the look target, from the
//! exact camera the engine will use. When previz knows how the level is lit (the three game, or
//! a plan read with its lamps) the frame is lit as the game is: its lamps with their shadows,
//! its fill, haze and exposure, the materials' colours, and the three game's figures in their
//! poses. Otherwise a fixed key light shades plain gray blocks. Ink outlines keep the shapes
//! readable either way. A project whose level has not been read draws its anchors as solid
//! blocks on the ground. That is enough to judge framing, lens, light and movement in seconds,
//! before paying for an engine render. The picture carries no marks: the places' names, the aim
//! point and the markers go to [`GUIDES`], and the app lays them over it with the thirds (D-96).
//!
//! Only whether a level is drawn goes into the previz fingerprint ([`set_kind`]). What the level
//! holds is followed like an engine take's game content (D-37): a previz take keeps the level
//! file, or the three objects it showed, in its deps, so a change re-draws only those shots.

mod figure;
mod game;
mod level;
mod mesh;
mod raster;
mod scene;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::geometry::{self as g, Vec3};
use crate::model::{num, AnchorBox, Definition, ResolvedShot};

use mesh::{srgb_to_linear, Builder, Geo, Surf, Xf};
use raster::{Frame, Graybox, ShadowMap, View};
use scene::{Env, Light};

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
                for (p, &k) in self.px[i..i + 3].iter_mut().zip(&c) {
                    *p = (*p as f64 * (1.0 - a) + k as f64 * a) as u8;
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
        Some(e) if e < -5.0 => [0.68, 0.73, 0.88],
        Some(e) if e < 12.0 => [0.98, 0.86, 0.76],
        _ => [1.0, 1.0, 1.0],
    }
}

fn anchor_color(name: &str) -> Rgb {
    let h = Sha256::digest(name.as_bytes());
    [90 + h[0] % 150, 90 + h[1] % 150, 90 + h[2] % 150]
}


// --------------------------------------------------------------------------- the world

enum Set {
    None,
    /// The plan read from the engine (D-86): pieces and lamps that never move.
    Level(level::Level),
    /// The three game (D-72), whose figures walk: the still objects built once, the rest at
    /// each frame's time.
    Game { scene: crate::three::Scene, still: OnceLock<Builder>, env: Env },
}

/// Everything previz draws around the shots: the anchors, the ground, and the level once read.
pub struct World {
    pub anchors: Vec<(String, AnchorBox)>,
    pub ground_z: f64,
    set: Set,
    /// (package, file) behind each piece's source: what a take that showed it depends on.
    sources: Vec<(String, PathBuf)>,
    /// The lamps' shadows of what stands still, drawn once (each frame adds what moves), and
    /// what is overhead (a roof keeps the sky's fill out).
    maps: OnceLock<(Vec<Option<ShadowMap>>, Option<ShadowMap>)>,
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

impl World {
    /// Anchors only, on the ground at `ground_z`.
    pub fn bare(anchors: Vec<(String, AnchorBox)>, ground_z: f64) -> World {
        World { anchors, ground_z, set: Set::None, sources: Vec::new(), maps: OnceLock::new() }
    }

    /// What previz draws for this project: its anchors, and the level when one was read (or
    /// the three game). A level that does not load is left out (the anchors still draw).
    pub fn load(defn: &Definition) -> World {
        let ground_z = defn.defaults.get("previz").and_then(|p| p.get("ground_z")).and_then(num).unwrap_or(0.0);
        let mut w = World::bare(defn.anchor_boxes(), ground_z);
        match set_kind(defn) {
            Some("three") => {
                if let Ok(scene) = crate::three::Scene::load(&crate::three::game_dir(defn)) {
                    // as a three take's deps: each object, the scene file (it lights everything),
                    // the props that pop in
                    use crate::three::{OBJECTS_DIR, PROPS_DIR, SCENE_FILE};
                    w.sources = scene.objects.iter().map(|o| (format!("game/{OBJECTS_DIR}/{}", o.name), o.file.clone())).collect();
                    w.sources.push((format!("game/{SCENE_FILE}"), scene.dir.join(SCENE_FILE)));
                    w.sources.extend(scene.props.iter().map(|p| (format!("game/{PROPS_DIR}/{}", p.name), p.file.clone())));
                    let env = game::env(&scene.globals);
                    w.set = Set::Game { scene, still: OnceLock::new(), env };
                }
            }
            Some("level") => {
                let root = defn.root();
                if let Some(v) = crate::scout::load_level(&root) {
                    w.set = Set::Level(level::parse(&v));
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

    /// Whether frames are lit as the game is (previz knows the level's light).
    pub fn is_lit(&self) -> bool {
        match &self.set {
            Set::Game { .. } => true,
            Set::Level(l) => l.env.is_some(),
            Set::None => false,
        }
    }

    /// The level's triangles at `t` seconds into a shot (every shot starts its level at 0 s),
    /// and how many of the first ones stand still.
    fn level_at(&self, t: f64) -> (Builder, usize) {
        match &self.set {
            Set::None => (Builder::default(), 0),
            Set::Level(l) => (l.built.clone(), l.built.tris.len()),
            Set::Game { .. } => {
                let still = self.still();
                let mut b = still.clone();
                if let Set::Game { scene, .. } = &self.set {
                    b.append(&game::build(scene, t, true));
                }
                (b, still.tris.len())
            }
        }
    }

    /// What stands still in the level.
    fn still(&self) -> &Builder {
        static EMPTY: OnceLock<Builder> = OnceLock::new();
        match &self.set {
            Set::Level(l) => &l.built,
            Set::Game { scene, still, .. } => still.get_or_init(|| game::build(scene, 0.0, false)),
            Set::None => EMPTY.get_or_init(Builder::default),
        }
    }

    /// The shadow map of each lamp (as `light_at(0)` lists them) over what stands still, and
    /// the map of what covers each point from the sky when the engine shades the sky's fill.
    fn still_maps(&self) -> (&[Option<ShadowMap>], Option<&ShadowMap>) {
        let (maps, sky) = self.maps.get_or_init(|| {
            let Some((lights, env)) = self.light_at(0.0) else { return (Vec::new(), None) };
            let tris = &self.still().tris;
            // a sun that names no area covers the whole level
            let (lo, hi) = tris.iter().map(|t| t.bounds()).fold(([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]), |(lo, hi), (a, b)| {
                (std::array::from_fn(|k| lo[k].min(a[k])), std::array::from_fn(|k| hi[k].max(b[k])))
            });
            let around = if lo[0].is_finite() {
                (g::scale(g::add(lo, hi), 0.5), ((hi[0] - lo[0]).max(hi[1] - lo[1]) / 2.0 + 100.0).min(20000.0))
            } else {
                ([0.0; 3], 3000.0)
            };
            let maps = lights.iter().map(|l| ShadowMap::build(l, tris, around)).collect();
            let overhead = Light {
                at: around.0,
                beam: scene::Beam::Sun { toward: [0.0, 0.0, 1.0] },
                color: [0.0; 3],
                range: 0.0,
                shadow: Some(scene::Shadow { size: 1024, area: Some(around) }),
                flicker: false,
                halo: None,
                haze_beam: 0.0,
                src: None,
                fitting: 0.0,
            };
            let sky = env.sky_cover.and_then(|_| ShadowMap::build(&overhead, tris, around));
            (maps, sky)
        });
        (maps, sky.as_ref())
    }

    /// The level's lamps at `t`, and how the frame is finished, when it is lit.
    fn light_at(&self, t: f64) -> Option<(Vec<Light>, &Env)> {
        match &self.set {
            Set::Game { scene, env, .. } => Some((game::lights(scene, t), env)),
            Set::Level(l) => l.env.as_ref().map(|e| (l.lights.clone(), e)),
            Set::None => None,
        }
    }

    /// Where the graybox draws a lamp's glow.
    fn lamps_at(&self, t: f64) -> Vec<Vec3> {
        match &self.set {
            Set::Level(l) => l.lamps.clone(),
            // moonlight and the like light the building from outside it
            Set::Game { scene, .. } => game::lights(scene, t)
                .iter()
                .filter(|l| !matches!(l.beam, scene::Beam::Sun { .. }) && l.halo.is_some())
                .map(|l| l.at)
                .collect(),
            Set::None => Vec::new(),
        }
    }
}

// --------------------------------------------------------------------------- drawing

fn linear(c: Rgb) -> Col {
    [srgb_to_linear(c[0] as f32 / 255.0), srgb_to_linear(c[1] as f32 / 255.0), srgb_to_linear(c[2] as f32 / 255.0)]
}

fn add_block(b: &mut Builder, center: Vec3, half: Vec3, yaw: f64, shown: Rgb) {
    let (s, piece) = (b.surf(Surf::plain(linear(shown))), b.piece(None));
    b.add(&Geo::cube(g::scale(half, 2.0)), &Xf::ue(center, g::Rot::new(0.0, yaw, 0.0), [1.0; 3]), s, piece);
}

/// A take's frame: the picture alone. The guides (thirds, the places' names, the aim point, the
/// marker) are laid over it by the app from [`GUIDES`], so they stay thin at any size and can be
/// turned off (D-96).
/// The props that have appeared by this frame: the game's own thing when it has one.
fn add_appeared(built: &mut Builder, shot: &ResolvedShot, frame: i64, world: &World, t: f64) {
    for ap in shot.appear.iter().filter(|a| frame >= a.frame) {
        let own = match &world.set {
            Set::Game { scene, .. } => scene.prop(&ap.asset).map(|o| (scene, o)),
            _ => None,
        };
        match own {
            Some((scene, o)) => {
                let src = scene.props.iter().position(|p| p.name == o.name).map(|j| scene.objects.len() + 1 + j);
                game::object(built, &scene.globals, o, ap.location, ap.yaw, t, src, &ap.prop);
            }
            None => {
                let b = ap.bbox();
                add_block(built, b.center, g::scale(b.size, 0.5), ap.yaw, [224, 87, 77]);
            }
        }
    }
}

pub fn render_frame(shot: &ResolvedShot, frame: i64, world: &World, w: u32, h: u32, tint: Option<Rgb>) -> Canvas {
    draw_frame(shot, frame, world, w, h, tint, false).0
}

/// The frame as a graybox with the anchors' wires and names and the aim point drawn in: the
/// layout an image model starts from when it paints a storyboard frame (the names tell it what
/// the boxes are).
pub fn render_layout(shot: &ResolvedShot, frame: i64, world: &World, w: u32, h: u32) -> Canvas {
    draw_frame(shot, frame, world, w, h, None, true).0
}

/// The frame, the level sources it showed, and its guides for the app. `layout` picks the look:
/// a take is lit as the game is when previz knows how and carries no marks; a layout is always a
/// graybox with the anchors marked on it.
fn draw_frame(shot: &ResolvedShot, frame: i64, world: &World, w: u32, h: u32, tint: Option<Rgb>, layout: bool)
    -> (Canvas, BTreeSet<usize>, Value) {
    // small frames are drawn at twice the size and averaged down; the engine stand-in (tinted)
    // and large frames are drawn once
    let ss = if tint.is_none() && (w as u64 * h as u64) <= 500_000 { 2usize } else { 1 };
    let (bw, bh) = (w as usize * ss, h as usize * ss);
    let cam = shot.camera_at(frame);
    let fl = shot.focal_length();
    let sensor = shot.sensor_width();
    let t = frame as f64 / shot.fps.max(1) as f64;

    let (mut built, still) = world.level_at(t);
    // with no level, the anchors are the set: solid blocks, unless the camera stands inside one
    let solid_anchors = !world.has_level();
    let mut wired: Vec<(&str, AnchorBox)> = Vec::new();
    for (name, b) in &world.anchors {
        if solid_anchors && !b.contains(cam.location, 0.0) {
            add_block(&mut built, b.center, g::scale(b.size, 0.5), 0.0, mix(anchor_color(name), [158, 158, 158], 0.45));
        } else {
            wired.push((name.as_str(), *b));
        }
    }
    add_appeared(&mut built, shot, frame, world, t);

    let mut fr = Frame::new(View::new(&cam, fl, sensor, bw, bh), bw, bh);
    if !matches!(world.set, Set::Game { .. }) {
        fr.ground(world.ground_z);
    }
    fr.draw(&built.tris);
    let mut seen = fr.seen(&built.pieces);

    let lit = if layout { None } else { world.light_at(t) };
    if let Some((lights, env)) = &lit {
        // the still lamps' maps drawn once, with what moves added; a lamp that cannot light
        // the frame needs none
        let (base, sky) = world.still_maps();
        let moving = &built.tris[still..];
        let maps: Vec<Option<ShadowMap>> = lights
            .iter()
            .enumerate()
            .map(|(i, l)| {
                if l.shadow.is_none() || !fr.reaches(l) {
                    return None;
                }
                match base.get(i).and_then(Option::as_ref).filter(|m| m.at == l.at) {
                    Some(m) => {
                        let mut m = m.clone();
                        m.add(moving);
                        Some(m)
                    }
                    None => ShadowMap::build(l, &built.tris, (cam.location, 3000.0)),
                }
            })
            .collect();
        let lit_by = fr.shade_lit(&built, lights, &maps, sky, env, t);
        // a lamp that lights the frame changes it, seen or not
        seen.extend(lights.iter().zip(lit_by).filter(|(_, on)| *on).filter_map(|(l, _)| l.src));
        fr.outlines();
        if let Some(c) = tint {
            fr.tint(col(c), 0.15);
        }
    } else {
        let elev = shot.environment.get("sun_elevation").and_then(num);
        let (mut top, mut horizon, mut ground) = palette(elev);
        if let Some(t) = tint {
            (top, horizon, ground) = (mix(top, t, 0.35), mix(horizon, t, 0.35), mix(ground, t, 0.35));
        }
        let key = {
            let e = elev.unwrap_or(50.0).clamp(28.0, 65.0).to_radians();
            let y = shot.environment.get("sun_yaw").and_then(num).unwrap_or(45.0).to_radians();
            mesh::unit([-e.cos() * y.cos(), -e.cos() * y.sin(), e.sin()])
        };
        let gb = Graybox { key, tone: tone(elev), top: col(top), horizon: col(horizon), ground: col(ground), ground_z: world.ground_z };
        fr.shade_graybox(&built, &gb);
        fr.outlines();
        for l in world.lamps_at(t) {
            fr.glow(l, ss as f64);
        }
    }
    if layout {
        for (name, b) in &wired {
            let c = col(anchor_color(name));
            let cs = b.corners();
            for (i, j) in EDGES {
                fr.wire(cs[i], cs[j], c, ss);
            }
        }
    }
    let mut cv = fr.resolve(ss);

    // the places in frame and the props that have popped in, far to near so the nearest is on top
    let view = View::new(&cam, fl, sensor, w as usize, h as usize);
    let (fw, fh) = (w as f64, h as f64);
    let on_screen = |p: &[f64; 3]| p[0] >= 0.0 && p[1] >= 0.0 && p[0] < fw && p[1] < fh;
    type Place = (f64, String, Rgb, Vec<[f64; 3]>, Option<[f64; 3]>);
    let mut places: Vec<Place> = Vec::new();
    for (name, b) in &world.anchors {
        let pts: Vec<[f64; 3]> = b.corners().iter().filter_map(|p| view.project(*p)).collect();
        if pts.iter().any(on_screen) {
            places.push((view.cam(b.center)[2], name.clone(), anchor_color(name), pts, view.project(b.center)));
        }
    }
    for ap in shot.appear.iter().filter(|a| frame >= a.frame) {
        let center = ap.bbox().center;
        let pts: Vec<[f64; 3]> = corners(center, g::scale(ap.size, 0.5), ap.yaw).iter().filter_map(|p| view.project(*p)).collect();
        if pts.iter().any(on_screen) {
            places.push((view.cam(ap.location)[2], ap.prop.clone(), [255, 120, 110], pts, view.project(center)));
        }
    }
    places.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(b.1.cmp(&a.1)));
    let aim = shot.target.and_then(|t| view.project(t)).filter(on_screen);

    if !layout {
        // a take carries no marks: the app lays these over it, thin at any size (D-96)
        let frac = |x: f64, y: f64| json!([(x.clamp(0.0, fw) / fw * 1000.0).round() / 1000.0, (y.clamp(0.0, fh) / fh * 1000.0).round() / 1000.0]);
        let names: Vec<Value> = places
            .iter()
            .map(|(_, name, c, pts, center)| {
                // on the place's centre when that is in frame, else in the middle of the part that is
                let at = match center.filter(on_screen) {
                    Some(p) => frac(p[0], p[1]),
                    None => {
                        let span = |k: usize, max: f64| {
                            let lo = pts.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min).clamp(0.0, max);
                            let hi = pts.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max).clamp(0.0, max);
                            (lo + hi) / 2.0
                        };
                        frac(span(0, fw), span(1, fh))
                    }
                };
                json!({"name": name, "color": format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]), "at": at})
            })
            .collect();
        let mut gd = json!({"places": names});
        if let Some(a) = aim {
            gd["aim"] = frac(a[0], a[1]);
        }
        let near = |m: &&Value| (m["frame"].as_i64().unwrap_or(0) - frame).abs() <= (shot.fps / 4).max(2);
        if let Some(label) = shot.markers.iter().find(near).and_then(|m| m["label"].as_str()) {
            gd["marker"] = json!(label);
        }
        return (cv, seen, gd);
    }

    // the layout's names and aim point at the final size, so they stay crisp
    let size: i64 = if w >= 640 { 2 } else { 1 };
    for (_, name, c, pts, _) in &places {
        let label: String = name.chars().take(16).collect();
        let lx = pts.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min).max(0.0) as i64;
        let ly = pts.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min) as i64 - 10 * size;
        let max_x = w as i64 - 6 * size * label.chars().count() as i64 - 2 * size;
        cv.label(lx.min(max_x).max(2 * size), ly.max(12 * size), &label, *c, size);
    }
    if let Some(t) = aim {
        let r = 5.0 * size as f64;
        for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)] {
            cv.line(t[0] - r + dx, t[1] + dy, t[0] + r + dx, t[1] + dy, [255, 60, 60]);
            cv.line(t[0] + dx, t[1] - r + dy, t[0] + dx, t[1] + r + dy, [255, 60, 60]);
        }
    }
    (cv, seen, Value::Null)
}

/// The shot's scene for the app's 3D view (D-97): the shapes previz draws, as triangles in UE
/// millimetres with the index of their surface, split into what stands still and what is there
/// at `frame` (the figures in their pose, the props that have popped in). What stands still is
/// sent once: its `key` names it, and `have` (the key the page already holds) leaves it out.
/// With them go the surfaces' colours, the anchors, the ground, the aim point and the shot's
/// camera at every frame. It only reads; the take stays the picture previz drew.
pub fn view_scene(shot: &ResolvedShot, frame: i64, world: &World, w: u32, h: u32, have: Option<&str>) -> Value {
    let frame = frame.clamp(0, (shot.frames - 1).max(0));
    let t = frame as f64 / shot.fps.max(1) as f64;
    let (mut built, still) = world.level_at(t);
    add_appeared(&mut built, shot, frame, world, t);
    let mm = |v: f64| (v * 10.0).round() as i64;
    let flat = |tris: &[mesh::Tri]| {
        let p: Vec<i64> = tris.iter().flat_map(|tr| tr.p.iter().flat_map(|q| q.iter().map(|v| mm(*v)))).collect();
        let s: Vec<u32> = tris.iter().map(|tr| tr.surf).collect();
        (p, s)
    };
    let (sp, ss) = flat(&built.tris[..still]);
    let key = {
        let mut h = Sha256::new();
        for v in &sp {
            h.update(v.to_le_bytes());
        }
        for v in &ss {
            h.update(v.to_le_bytes());
        }
        h.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect::<String>()
    };
    let still_json = if have == Some(key.as_str()) { json!({"key": key}) } else { json!({"key": key, "p": sp, "s": ss}) };
    let (mp, ms) = flat(&built.tris[still..]);
    // whether another frame looks different from this one (the camera moves either way)
    let dynamic = !mp.is_empty() || !shot.appear.is_empty();
    // a glowing surface shows its light, the rest their colour
    let hex = |c: Col| {
        let b = |x: f32| (mesh::linear_to_srgb(x.clamp(0.0, 1.0)) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2]))
    };
    let surfs: Vec<Value> = built
        .surfs
        .iter()
        .map(|s| {
            let peak = s.emit.iter().cloned().fold(0.0f32, f32::max);
            if peak > 0.0 {
                json!({"color": hex([s.emit[0] / peak, s.emit[1] / peak, s.emit[2] / peak]), "glow": true})
            } else {
                json!({"color": hex(s.albedo)})
            }
        })
        .collect();
    let rgb = |c: Rgb| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
    let anchors: Vec<Value> = world
        .anchors
        .iter()
        .map(|(name, b)| json!({"name": name, "color": rgb(anchor_color(name)), "center": b.center, "size": b.size}))
        .collect();
    let props: Vec<Value> = shot
        .appear
        .iter()
        .filter(|a| frame >= a.frame)
        .map(|a| json!({"name": a.prop, "center": a.bbox().center}))
        .collect();
    let (fl, sensor) = (shot.focal_length(), shot.sensor_width());
    let cameras: Vec<Value> = (0..shot.frames.max(1)).map(|f| crate::three::frame_camera(&shot.camera_at(f), fl, sensor, w, h)).collect();
    json!({
        "shot": shot.id, "frame": frame, "frames": shot.frames, "aspect": w as f64 / h.max(1) as f64,
        "still": still_json, "moving": {"p": mp, "s": ms}, "dynamic": dynamic, "surfs": surfs,
        // with no level read the anchors are the set (previz draws them as blocks)
        "level": world.has_level(), "anchors": anchors, "props": props,
        "ground": (!matches!(world.set, Set::Game { .. })).then_some(world.ground_z),
        "target": shot.target, "cameras": cameras,
    })
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

/// What a shot's previz wrote: its frame files, and the level it showed as `[{package, file}]`
/// for the take's deps (content::record).
pub struct Drawn {
    pub files: Vec<String>,
    pub deps: Vec<Value>,
}

/// Next to a previz take's frames: what the app lays over each of them when its guides are on
/// (`{"frames": [{"places": [{name, color, at}], "aim"?, "marker"?}]}`, in the order of the
/// frame files, positions as fractions of the frame). Not a take file: the picture never has it.
pub const GUIDES: &str = "guides.json";

pub fn render_shot(shot: &ResolvedShot, frames: &[i64], world: &World, out_dir: &Path, w: u32, h: u32, tint: Option<Rgb>)
    -> Result<Drawn> {
    std::fs::create_dir_all(out_dir)?;
    // frames are independent: draw them on every core, then write them in order
    let next = std::sync::atomic::AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(frames.len()).max(1);
    let mut drawn: Vec<(usize, Vec<u8>, BTreeSet<usize>, Value)> = std::thread::scope(|sc| {
        let jobs: Vec<_> = (0..workers)
            .map(|_| {
                sc.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(&f) = frames.get(k) else { break };
                        let (cv, s, gd) = draw_frame(shot, f, world, w, h, tint, false);
                        out.push((k, cv.png(), s, gd));
                    }
                    out
                })
            })
            .collect();
        jobs.into_iter().flat_map(|j| j.join().unwrap_or_default()).collect()
    });
    drawn.sort_by_key(|d| d.0);
    let mut files = Vec::with_capacity(frames.len());
    let mut guides = Vec::with_capacity(frames.len());
    let mut seen = BTreeSet::new();
    for (k, png, s, gd) in drawn {
        let name = format!("frame.{:04}.png", frames[k]);
        std::fs::write(out_dir.join(&name), png)?;
        files.push(name);
        guides.push(gd);
        seen.extend(s);
    }
    if files.len() != frames.len() {
        crate::bail!("previz of {}: {} of {} frames were drawn", shot.id, files.len(), frames.len());
    }
    std::fs::write(out_dir.join(GUIDES), json!({"frames": guides}).to_string())?;
    // the level plan is one file: a take that drew it depends on it whether or not a block was
    // in frame (it may hold one that should have been)
    match &world.set {
        Set::Level(_) => {
            seen.insert(0);
        }
        // the scene file sets the light, haze and colours of every frame
        Set::Game { scene, .. } => {
            seen.insert(scene.objects.len());
        }
        Set::None => {}
    }
    let deps = seen
        .iter()
        .filter_map(|&i| world.sources.get(i))
        .map(|(package, file)| json!({"package": package, "file": file.display().to_string()}))
        .collect();
    Ok(Drawn { files, deps })
}
