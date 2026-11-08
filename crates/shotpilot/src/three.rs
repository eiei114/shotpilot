//! The Three.js engine (D-72): a trial game people can film without Unreal.
//!
//! The game is a folder (`project.game`, `game/` by default) of JSON: `scene.json` holds what
//! lights the whole level (materials, fog, sky, moon), and `objects/<name>.json` one thing in it
//! each (a wall run, a desk, a patient), so editing one thing re-renders only the shots it was
//! seen in (D-37). `props/<name>.json` are things a shot can make appear.
//!
//! Drawing happens in a headless browser (Edge or Chrome): a loopback server hands the page
//! `three/render.html` the scene and one camera per frame, and the page posts each frame back.
//! What a frame showed (D-56) and where a scout camera must stand to see past a wall (D-55)
//! are worked out here in Rust from the same JSON, so they are testable without a browser.
//! Coordinates follow UE (cm, Z up, X forward, Y right); the page converts them.

use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use crate::bail;
use crate::engines::{shot_from_payload, Engine, SeqStore};
use crate::error::{Error, Result};
use crate::geometry::{self as g, Rot, Vec3, Xform};
use crate::model::{num, resolve_all, vec3, Definition, ResolvedShot};

/// The page that draws the frames, and Three.js itself (MIT, three/LICENSE-three.txt).
pub const RENDER_PAGE: &str = include_str!("../three/render.html");
pub const THREE_JS: &[u8] = include_bytes!("../three/three.module.min.js");
/// Bump when the page draws differently for the same scene and cameras. It is part of the
/// fingerprint of three takes only, so UE and mock takes never move with it.
pub const RENDERER_VERSION: i64 = 2;
/// How many seconds a moving thing's `path` loops over when it does not say.
const LOOP_SECS: f64 = 12.0;

pub const SCENE_FILE: &str = "scene.json";
pub const OBJECTS_DIR: &str = "objects";
pub const PROPS_DIR: &str = "props";

// --------------------------------------------------------------------------- scene

/// A box turned about Z only, in world cm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb {
    pub center: Vec3,
    pub half: Vec3,
    pub yaw: f64,
}

impl Obb {
    /// The axis-aligned box around it: (min, max).
    pub fn aabb(&self) -> (Vec3, Vec3) {
        let (s, c) = self.yaw.to_radians().sin_cos();
        let [hx, hy, hz] = self.half;
        let ex = [c.abs() * hx + s.abs() * hy, s.abs() * hx + c.abs() * hy, hz];
        (g::sub(self.center, ex), g::add(self.center, ex))
    }

    fn local(&self, p: Vec3) -> Vec3 {
        let (s, c) = (-self.yaw).to_radians().sin_cos();
        let d = g::sub(p, self.center);
        [d[0] * c - d[1] * s, d[0] * s + d[1] * c, d[2]]
    }

    pub fn contains(&self, p: Vec3) -> bool {
        let l = self.local(p);
        (0..3).all(|i| l[i].abs() <= self.half[i])
    }

    /// Where the ray `from + t * dir` (dir of unit length) enters the box, for t in (0, max).
    pub fn hit(&self, from: Vec3, dir: Vec3, max: f64) -> Option<f64> {
        let o = self.local(from);
        let (s, c) = (-self.yaw).to_radians().sin_cos();
        let d = [dir[0] * c - dir[1] * s, dir[0] * s + dir[1] * c, dir[2]];
        let (mut t0, mut t1) = (0.0f64, max);
        for i in 0..3 {
            if d[i].abs() < 1e-9 {
                if o[i].abs() > self.half[i] {
                    return None;
                }
                continue;
            }
            let (a, b) = ((-self.half[i] - o[i]) / d[i], (self.half[i] - o[i]) / d[i]);
            t0 = t0.max(a.min(b));
            t1 = t1.min(a.max(b));
            if t0 > t1 {
                return None;
            }
        }
        (t0 > 1e-6 && t0 < max).then_some(t0)
    }
}

/// One part of an object, in the object's own frame (cm; `yaw` about Z).
#[derive(Debug, Clone)]
struct Part {
    center: Vec3,
    half: Vec3,
    yaw: f64,
    /// Blocks sight (walls, furniture). Figures, lights and dust do not.
    solid: bool,
    /// Has a body at all (lights do not, so they are never "seen").
    body: bool,
}

/// A thing in the level: one file of `objects/` (or `props/`).
#[derive(Debug, Clone)]
pub struct Object {
    pub name: String,
    pub file: PathBuf,
    pub label: String,
    /// The JSON as written, handed to the page.
    pub raw: Value,
    at: Vec3,
    yaw: f64,
    /// `[t, x, y, facing]` keys: where it walks over one loop.
    path: Vec<[f64; 4]>,
    period: f64,
    parts: Vec<Part>,
    /// Part of the building (walls, floor): never offered as a place to scout.
    pub structure: bool,
}

/// Where a figure's body lies relative to its feet, by how it moves: (center, size) before
/// it is turned to face its way. Kept in step with the moves in three/render.html.
fn figure_box(mv: &str, h: f64) -> (Vec3, Vec3) {
    match mv {
        "reach" | "lie_back" => ([-h / 2.0, 0.0, 20.0], [h, 60.0, 40.0]),
        "face_down" => ([h / 2.0, 0.0, 15.0], [h, 70.0, 30.0]),
        "crawl" => ([h * 0.45, 0.0, 30.0], [h * 0.9, 70.0, 60.0]),
        "cower" => ([0.0, 0.0, h * 0.28], [70.0, 70.0, h * 0.56]),
        _ => ([0.0, 0.0, h / 2.0], [45.0, 55.0, h]),
    }
}

fn parse_part(v: &Value, where_: &str) -> Result<Part> {
    let at = v.get("at").and_then(vec3).unwrap_or([0.0; 3]);
    let yaw = v.get("yaw").and_then(num).unwrap_or(0.0);
    let solid = v.get("solid").and_then(Value::as_bool);
    let part = |center: Vec3, size: Vec3, yaw: f64, solid_default: bool, body: bool| Part {
        center,
        half: size.map(|s| s.abs() / 2.0),
        yaw,
        solid: solid.unwrap_or(solid_default),
        body,
    };
    if let Some(size) = v.get("box") {
        let Some(size) = vec3(size) else { bail!("{where_}: `box` must be [w, d, h] in cm") };
        return Ok(part(at, size, yaw, true, true));
    }
    if let Some(c) = v.get("cylinder") {
        let rh: Vec<f64> = c.as_array().map(|a| a.iter().filter_map(num).collect()).unwrap_or_default();
        let [r, h] = rh[..] else { bail!("{where_}: `cylinder` must be [radius, height] in cm") };
        let size = match v.get("axis").and_then(Value::as_str).unwrap_or("z") {
            "x" => [h, 2.0 * r, 2.0 * r],
            "y" => [2.0 * r, h, 2.0 * r],
            _ => [2.0 * r, 2.0 * r, h],
        };
        // thin poles and pipes never hide what is behind them
        return Ok(part(at, size, yaw, r >= 15.0, true));
    }
    if let Some(r) = v.get("sphere") {
        let Some(r) = num(r) else { bail!("{where_}: `sphere` must be a radius in cm") };
        return Ok(part(at, [2.0 * r; 3], yaw, r >= 15.0, true));
    }
    if let Some(mv) = v.get("figure") {
        let mv = mv.as_str().unwrap_or("idle");
        let h = v.get("height").and_then(num).unwrap_or(175.0);
        let facing = v.get("facing").and_then(num).unwrap_or(0.0);
        let (c, size) = figure_box(mv, h);
        let (s, co) = facing.to_radians().sin_cos();
        let center = [at[0] + c[0] * co - c[1] * s, at[1] + c[0] * s + c[1] * co, at[2] + c[2]];
        return Ok(part(center, size, facing, false, true));
    }
    if v.get("dust").is_some() {
        let size = v.get("dust").and_then(vec3).unwrap_or([200.0, 200.0, 200.0]);
        return Ok(part(at, size, yaw, false, true));
    }
    if v.get("light").is_some() {
        return Ok(part(at, [0.0; 3], 0.0, false, false));
    }
    bail!("{where_}: a part is one of box, cylinder, sphere, figure, light or dust")
}

impl Object {
    fn load(path: &Path) -> Result<Object> {
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let shown = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|e| Error::user(format!("{shown}: {e}")))?;
        let raw: Value = serde_json::from_str(&text).map_err(|e| Error::user(format!("{shown}: {e}")))?;
        let mut parts = Vec::new();
        for (i, p) in raw.get("parts").and_then(Value::as_array).into_iter().flatten().enumerate() {
            parts.push(parse_part(p, &format!("{shown}: parts[{i}]"))?);
        }
        let path_keys: Vec<[f64; 4]> = raw
            .get("path")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|k| {
                let a: Vec<f64> = k.as_array()?.iter().filter_map(num).collect();
                (a.len() == 4).then(|| [a[0], a[1], a[2], a[3]])
            })
            .collect();
        let period = raw.get("loop").and_then(num).unwrap_or(LOOP_SECS).max(0.1);
        Ok(Object {
            label: raw.get("label").and_then(Value::as_str).unwrap_or(&name).to_string(),
            at: raw.get("at").and_then(vec3).unwrap_or([0.0; 3]),
            yaw: raw.get("yaw").and_then(num).unwrap_or(0.0),
            structure: raw.get("structure").and_then(Value::as_bool).unwrap_or(false),
            path: path_keys,
            period,
            parts,
            raw,
            file: path.to_path_buf(),
            name,
        })
    }

    /// Where it stands and which way it faces at `t` seconds into a shot (every shot starts
    /// its level at 0 s, as PIE does in UE).
    pub fn pose_at(&self, t: f64) -> (Vec3, f64) {
        if self.path.is_empty() {
            return (self.at, self.yaw);
        }
        let t = g::pymod(t, self.period);
        let k = &self.path;
        let i = k.iter().rposition(|p| p[0] <= t).unwrap_or(0);
        let (a, b) = (k[i], *k.get(i + 1).unwrap_or(&k[i]));
        let u = if b[0] > a[0] { ((t - a[0]) / (b[0] - a[0])).clamp(0.0, 1.0) } else { 0.0 };
        ([g::lerp(a[1], b[1], u), g::lerp(a[2], b[2], u), self.at[2]], g::lerp_angle(a[3], b[3], u))
    }

    fn placed(&self, part: &Part, at: Vec3, yaw: f64) -> Obb {
        let (s, c) = yaw.to_radians().sin_cos();
        let p = part.center;
        Obb { center: [at[0] + p[0] * c - p[1] * s, at[1] + p[0] * s + p[1] * c, at[2] + p[2]], half: part.half, yaw: yaw + part.yaw }
    }

    /// Its parts with a body, in the world at `t`: (box, blocks sight).
    pub fn boxes_at(&self, t: f64) -> Vec<(Obb, bool)> {
        let (at, yaw) = self.pose_at(t);
        self.parts.iter().filter(|p| p.body).map(|p| (self.placed(p, at, yaw), p.solid)).collect()
    }

    /// The box around all of it at `t`: (min, max), None when it has no body (lights only).
    pub fn bounds_at(&self, t: f64) -> Option<(Vec3, Vec3)> {
        let boxes = self.boxes_at(t);
        let mut it = boxes.iter().map(|(b, _)| b.aabb());
        let first = it.next()?;
        Some(it.fold(first, |(lo, hi), (a, b)| {
            ([lo[0].min(a[0]), lo[1].min(a[1]), lo[2].min(a[2])], [hi[0].max(b[0]), hi[1].max(b[1]), hi[2].max(b[2])])
        }))
    }

    /// The anchors: entry it offers when tagged (`anchor: true`, or the entry itself).
    pub fn anchor_entry(&self) -> Option<Value> {
        match self.raw.get("anchor")? {
            Value::Bool(true) => {
                let (lo, hi) = self.bounds_at(0.0)?;
                let r = |v: Vec3| v.map(|x| g::round_to(x, 1));
                Some(json!({"at": r(self.at), "size": r(g::sub(hi, lo)), "center": r(g::scale(g::add(lo, hi), 0.5))}))
            }
            v @ Value::Object(_) => Some(v.clone()),
            _ => None,
        }
    }
}

/// The whole game: `scene.json` and every object.
#[derive(Debug, Clone)]
pub struct Scene {
    pub dir: PathBuf,
    pub globals: Value,
    pub objects: Vec<Object>,
    pub props: Vec<Object>,
}

fn json_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect())
        .unwrap_or_default();
    v.sort();
    v
}

impl Scene {
    pub fn load(dir: &Path) -> Result<Scene> {
        let scene_file = dir.join(SCENE_FILE);
        if !scene_file.is_file() {
            bail!("the game for the three engine is not there: {} (project.game; `shotpilot init --engine three` writes a sample)",
                  scene_file.display());
        }
        let text = std::fs::read_to_string(&scene_file)?;
        let globals: Value = serde_json::from_str(&text).map_err(|e| Error::user(format!("{}: {e}", scene_file.display())))?;
        let objects = json_files(&dir.join(OBJECTS_DIR)).iter().map(|p| Object::load(p)).collect::<Result<Vec<_>>>()?;
        let props = json_files(&dir.join(PROPS_DIR)).iter().map(|p| Object::load(p)).collect::<Result<Vec<_>>>()?;
        Ok(Scene { dir: dir.to_path_buf(), globals, objects, props })
    }

    pub fn prop(&self, asset: &str) -> Option<&Object> {
        let leaf = asset.trim_end_matches(".json").rsplit('/').next().unwrap_or(asset);
        self.props.iter().find(|p| p.name == leaf)
    }

    /// The tagged things, as `anchors:` entries.
    pub fn anchors(&self) -> Map<String, Value> {
        self.objects.iter().filter_map(|o| o.anchor_entry().map(|e| (o.name.clone(), e))).collect()
    }

    /// Places to scout in a level with nothing tagged (scout::Candidate), scored by size as UE does.
    pub fn candidates(&self) -> Vec<Value> {
        self.objects
            .iter()
            .filter(|o| !o.structure && o.anchor_entry().is_none())
            .filter_map(|o| {
                let (lo, hi) = o.bounds_at(0.0)?;
                let size = g::sub(hi, lo);
                let vol = size.iter().product::<f64>().max(0.0);
                let figure = o.parts.iter().any(|p| !p.solid && p.body) && o.raw["parts"].as_array()?.iter().any(|p| p.get("figure").is_some());
                let r = |v: Vec3| v.map(|x| g::round_to(x, 1));
                Some(json!({"name": o.name, "label": o.label, "cls": if figure { "Figure" } else { "Object" },
                            "score": vol.cbrt().min(400.0), "at": r(o.at), "size": r(size),
                            "center": r(g::scale(g::add(lo, hi), 0.5))}))
            })
            .collect()
    }

    /// What a shot can make appear (`props:`), with sizes: the asset inventory.
    pub fn inventory(&self) -> Vec<Value> {
        self.props
            .iter()
            .map(|p| {
                let figure = p.raw["parts"].as_array().is_some_and(|a| a.iter().any(|x| x.get("figure").is_some()));
                let size = p.bounds_at(0.0).map(|(lo, hi)| g::sub(hi, lo).map(|x| g::round_to(x, 1)));
                json!({"asset": format!("{PROPS_DIR}/{}", p.name), "kind": if figure { "skeletal" } else { "static" }, "size": size})
            })
            .collect()
    }

    /// Every box that blocks sight at `t`, with the object it belongs to.
    fn blockers(&self, t: f64) -> Vec<(usize, Obb)> {
        let mut out = Vec::new();
        for (i, o) in self.objects.iter().enumerate() {
            for (b, solid) in o.boxes_at(t) {
                if solid {
                    out.push((i, b));
                }
            }
        }
        out
    }

    /// Where a scout camera that looks at `target` from `camera` must stand to see it past a wall
    /// (D-55): pulled along the line to just before the nearest wall, with the lens widened by
    /// the same ratio (no wider than 12 mm). None when nothing is in the way.
    pub fn clear_view(&self, target: Vec3, camera: Vec3, focal: f64) -> Option<(Vec3, f64)> {
        let d = g::sub(camera, target);
        let dist = g::length(d);
        if dist < 1.0 {
            return None;
        }
        let dir = g::scale(d, 1.0 / dist);
        let hit = self
            .blockers(0.0)
            .iter()
            .filter(|(_, b)| !b.contains(target))
            .filter_map(|(_, b)| b.hit(target, dir, dist))
            .fold(f64::INFINITY, f64::min);
        if !hit.is_finite() {
            return None;
        }
        // the same rule as clear_view in ue/sp_entry.py: 30 cm short of the wall, lens no wider than 12 mm
        let near = (hit - 30.0).max((hit * 0.5).min(40.0));
        let at = g::add(target, g::scale(dir, near)).map(|v| g::round_to(v, 1));
        Some((at, g::round_to((focal * near / dist).max(12.0), 1)))
    }

    /// The objects a shot showed (D-56): at its first, middle and last frame, in view (with a
    /// 10% margin) and reached by a line from the camera that no other solid thing blocks.
    /// When no point of a thing in view lands on screen to test, it counts as shown.
    pub fn seen(&self, shot: &ResolvedShot, w: u32, h: u32) -> Vec<usize> {
        let last = (shot.frames - 1).max(0);
        let mut seen = std::collections::BTreeSet::new();
        let (fl, sensor) = (shot.focal_length(), shot.sensor_width());
        let (mw, mh) = (w as f64 * 0.1, h as f64 * 0.1);
        let on_screen = |x: f64, y: f64| x >= -mw && x <= w as f64 + mw && y >= -mh && y <= h as f64 + mh;
        for f in [0, last / 2, last] {
            let t = f as f64 / shot.fps.max(1) as f64;
            let cam = shot.camera_at(f);
            let eye = cam.location;
            let blockers: Vec<(usize, Obb)> = self.blockers(t).into_iter().filter(|(_, b)| !b.contains(eye)).collect();
            // rays across the frame: what each one meets first is on screen (large surfaces, such
            // as a floor whose corners and middle are all off screen, are found this way)
            let bodies: Vec<(usize, Obb, bool)> = self
                .objects
                .iter()
                .enumerate()
                .flat_map(|(i, o)| o.boxes_at(t).into_iter().map(move |(b, solid)| (i, b, solid)))
                .filter(|(_, b, _)| !b.contains(eye))
                .collect();
            for dir in screen_rays(&cam, fl, sensor, w, h) {
                let wall = bodies.iter().filter(|(_, _, s)| *s).filter_map(|(_, b, _)| b.hit(eye, dir, 1e7)).fold(f64::INFINITY, f64::min);
                for (i, b, _) in &bodies {
                    if b.hit(eye, dir, wall + 1.0).is_some() {
                        seen.insert(*i);
                    }
                }
            }
            for (i, o) in self.objects.iter().enumerate() {
                if seen.contains(&i) {
                    continue;
                }
                let Some((lo, hi)) = o.bounds_at(t) else { continue };
                let corners: Vec<Vec3> = (0..8)
                    .map(|k| [if k & 1 == 0 { lo[0] } else { hi[0] }, if k & 2 == 0 { lo[1] } else { hi[1] }, if k & 4 == 0 { lo[2] } else { hi[2] }])
                    .collect();
                if !in_view(&corners, &cam, fl, sensor, w, h, mw, mh) {
                    continue;
                }
                let mid = g::scale(g::add(lo, hi), 0.5);
                let mut points = vec![mid];
                points.extend(corners.iter().map(|c| g::add(mid, g::scale(g::sub(*c, mid), 0.8))));
                let mut boxes = o.boxes_at(t);
                boxes.sort_by(|a, b| {
                    let v = |x: &Obb| x.half.iter().product::<f64>();
                    v(&b.0).partial_cmp(&v(&a.0)).unwrap_or(std::cmp::Ordering::Equal)
                });
                points.extend(boxes.iter().take(12).map(|(b, _)| b.center));
                let mut tested = false;
                let mut clear = false;
                for p in points {
                    let Some((x, y, _)) = g::project(p, &cam, fl, w, h, sensor) else { continue };
                    if !on_screen(x, y) {
                        continue;
                    }
                    tested = true;
                    let d = g::sub(p, eye);
                    let dist = g::length(d);
                    if dist < 1.0 {
                        clear = true;
                        break;
                    }
                    let dir = g::scale(d, 1.0 / dist);
                    let blocked = blockers.iter().any(|(j, b)| *j != i && !b.contains(p) && b.hit(eye, dir, dist - 1.0).is_some());
                    if !blocked {
                        clear = true;
                        break;
                    }
                }
                if clear || !tested {
                    seen.insert(i);
                }
            }
        }
        seen.into_iter().collect()
    }
}

/// Unit rays from the camera through a 16 x 9 grid over the frame (the inverse of
/// geometry::project, roll included).
fn screen_rays(cam: &Xform, fl: f64, sensor: f64, w: u32, h: u32) -> Vec<Vec3> {
    let f = g::forward(cam.rotation);
    let r = g::right(cam.rotation);
    let u = g::up(cam.rotation);
    let (s, c) = (-cam.rotation.roll).to_radians().sin_cos();
    let fx = (w as f64 / 2.0) / (g::hfov_from_focal(fl, sensor).to_radians() / 2.0).tan();
    let mut out = Vec::new();
    for j in 0..9 {
        for i in 0..16 {
            let (px, py) = ((i as f64 + 0.5) / 16.0 * w as f64, (j as f64 + 0.5) / 9.0 * h as f64);
            let (sx, sy) = ((px - w as f64 / 2.0) / fx, (h as f64 / 2.0 - py) / fx);
            let (x, y) = (sx * c + sy * s, -sx * s + sy * c);
            let d = [f[0] + x * r[0] + y * u[0], f[1] + x * r[1] + y * u[1], f[2] + x * r[2] + y * u[2]];
            out.push(g::scale(d, 1.0 / g::length(d)));
        }
    }
    out
}

/// Whether a box (its 8 corners) may overlap the view, with a margin: it is out only when all
/// corners lie outside the same side of the view pyramid (the near plane, left, right, top or
/// bottom). Conservative: a box that is out in a way this misses counts as in (when unsure,
/// render again rather than miss a change).
#[allow(clippy::too_many_arguments)]
fn in_view(corners: &[Vec3], cam: &Xform, fl: f64, sensor: f64, w: u32, h: u32, mw: f64, mh: f64) -> bool {
    let f = g::forward(cam.rotation);
    let r = g::right(cam.rotation);
    let u = g::up(cam.rotation);
    let (s, c) = (-cam.rotation.roll).to_radians().sin_cos();
    let fx = (w as f64 / 2.0) / (g::hfov_from_focal(fl, sensor).to_radians() / 2.0).tan();
    let (tx, ty) = ((w as f64 / 2.0 + mw) / fx, (h as f64 / 2.0 + mh) / fx);
    let dot = |a: Vec3, b: Vec3| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let mut out = [true; 5];
    for p in corners {
        let rel = g::sub(*p, cam.location);
        let d = dot(rel, f);
        let (x0, y0) = (dot(rel, r), dot(rel, u));
        let (x, y) = (x0 * c - y0 * s, x0 * s + y0 * c);
        let sides = [d < 1.0, x < -tx * d, x > tx * d, y < -ty * d, y > ty * d];
        for (o, side) in out.iter_mut().zip(sides) {
            *o &= side;
        }
    }
    !out.iter().any(|o| *o)
}

// --------------------------------------------------------------------------- cameras

/// UE cm -> what the page needs for one frame: eye, a point ahead, up (with roll), and the
/// vertical field of view that matches geometry::project's horizontal one.
pub fn frame_camera(cam: &Xform, focal: f64, sensor: f64, w: u32, h: u32) -> Value {
    let f = g::forward(cam.rotation);
    let r = g::right(cam.rotation);
    let u = g::up(cam.rotation);
    // geometry::project turns the image by -roll: the same as turning the up vector
    let a = (-cam.rotation.roll).to_radians();
    let (s, c) = a.sin_cos();
    let up = [r[0] * s + u[0] * c, r[1] * s + u[1] * c, r[2] * s + u[2] * c];
    let fx = (w as f64 / 2.0) / (g::hfov_from_focal(focal, sensor).to_radians() / 2.0).tan();
    let vfov = (2.0 * ((h as f64 / 2.0) / fx).atan()).to_degrees();
    json!({"eye": cam.location, "ahead": g::add(cam.location, g::scale(f, 100.0)), "up": up, "fov": vfov})
}

/// What the page needs for depth of field: focal length and sensor (mm), f-stop, and the focus
/// distance in cm (null when the shot names none: the page focuses on what is in the middle).
fn lens_of(shot: &ResolvedShot) -> Value {
    json!({
        "focal": shot.focal_length(),
        "sensor": shot.sensor_width(),
        "fstop": shot.lens.get("aperture").and_then(num).unwrap_or(2.8),
        "focus": shot.lens.get("focus_distance").and_then(num),
    })
}

/// A one-frame shot made from a stored payload alone (scout cameras are not shots).
fn bare_shot(id: &str, payload: &Value) -> ResolvedShot {
    let xf = |d: &Value| Xform::new(vec3(&d["location"]).unwrap_or([0.0; 3]), Rot::from_array(vec3(&d["rotation"]).unwrap_or([0.0; 3])));
    let base = ResolvedShot {
        id: id.to_string(),
        index: 0,
        purpose: String::new(),
        duration: 0.0,
        fps: 30,
        frames: 1,
        start: xf(&payload["start"]),
        end: xf(&payload["end"]),
        curve: "linear".into(),
        lens: Map::new(),
        environment: Map::new(),
        markers: Vec::new(),
        target: None,
        notes: String::new(),
        beat: None,
        appear: Vec::new(),
    };
    shot_from_payload(&base, payload)
}

// --------------------------------------------------------------------------- browser

static STAND_IN: AtomicBool = AtomicBool::new(false);

/// Tests only: answer the page's requests from a thread that posts tiny blank frames instead
/// of starting a browser (the tests never start Edge or Chrome).
#[doc(hidden)]
pub fn use_stand_in_browser(on: bool) {
    STAND_IN.store(on, Ordering::SeqCst);
}

/// Edge or Chrome: `$SHOTPILOT_BROWSER`, then the usual places, then PATH.
pub fn find_browser() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("SHOTPILOT_BROWSER").map(PathBuf::from) {
        return p.is_file().then_some(p);
    }
    let mut known: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        for base in ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"].iter().filter_map(std::env::var_os) {
            let b = PathBuf::from(base);
            known.push(b.join(r"Microsoft\Edge\Application\msedge.exe"));
            known.push(b.join(r"Google\Chrome\Application\chrome.exe"));
        }
    } else {
        known.push("/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".into());
        known.push("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into());
        known.push("/Applications/Chromium.app/Contents/MacOS/Chromium".into());
    }
    known
        .into_iter()
        .find(|p| p.is_file())
        .or_else(|| ["msedge", "microsoft-edge", "google-chrome", "chromium", "chromium-browser", "chrome"].iter().find_map(|n| crate::engines::find_on_path(n)))
}

struct Board {
    outputs: Vec<PathBuf>,
    written: Vec<bool>,
    done: Option<std::result::Result<(), String>>,
    last: Instant,
    log: Vec<String>,
}

struct Request {
    method: String,
    path: String,
    body: Vec<u8>,
}

fn read_request(s: &mut TcpStream) -> std::result::Result<Request, String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 65536];
    let head_end = loop {
        let n = s.read(&mut chunk).map_err(|e| format!("reading the head: {e}"))?;
        if n == 0 {
            return Err(format!("closed before the head ended ({} bytes)", buf.len()));
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if buf.len() > 64 * 1024 {
            return Err("head over 64 KB".into());
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.lines();
    let mut first = lines.next().unwrap_or_default().split_whitespace();
    let (Some(method), Some(path)) = (first.next(), first.next()) else { return Err(format!("bad request line: {head}")) };
    let (method, path) = (method.to_string(), path.to_string());
    let len: usize = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse().ok())
        .unwrap_or(0);
    let mut body = buf[head_end..].to_vec();
    while body.len() < len {
        let n = s.read(&mut chunk).map_err(|e| format!("reading the body of {path} at {} of {len}: {e}", body.len()))?;
        if n == 0 {
            return Err(format!("closed in the body of {path} at {} of {len}", body.len()));
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(len);
    Ok(Request { method, path, body })
}

fn respond(s: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = s.write_all(head.as_bytes());
    let _ = s.write_all(body);
    let _ = s.flush();
}

fn serve(mut s: TcpStream, token: &str, job: &[u8], board: &Mutex<Board>) {
    let _ = s.set_nonblocking(false);
    let _ = s.set_read_timeout(Some(Duration::from_secs(30)));
    let lock = || board.lock().unwrap_or_else(|p| p.into_inner());
    let req = match read_request(&mut s) {
        Ok(r) => r,
        // a connection opened ahead and closed unused is not worth a line
        Err(why) if why.contains("(0 bytes)") => return,
        Err(why) => {
            lock().log.push(format!("server: {why}"));
            return;
        }
    };
    lock().last = Instant::now();
    let prefix = format!("/{token}/");
    let Some(route) = req.path.strip_prefix(&prefix) else {
        respond(&mut s, "404 Not Found", "text/plain", b"");
        return;
    };
    let (name, query) = route.split_once('?').unwrap_or((route, ""));
    match (req.method.as_str(), name) {
        ("GET", "") => respond(&mut s, "200 OK", "text/html; charset=utf-8", RENDER_PAGE.as_bytes()),
        ("GET", "three.module.min.js") => respond(&mut s, "200 OK", "text/javascript", THREE_JS),
        ("GET", "job") => respond(&mut s, "200 OK", "application/json", job),
        ("POST", "frame") => {
            let n: Option<usize> = query.split('&').find_map(|kv| kv.strip_prefix("n=")).and_then(|v| v.parse().ok());
            let mut b = lock();
            match n.filter(|n| *n < b.outputs.len()) {
                Some(n) => {
                    let ok = std::fs::write(&b.outputs[n], &req.body).is_ok();
                    b.written[n] = ok;
                    drop(b);
                    respond(&mut s, if ok { "200 OK" } else { "500 Internal Server Error" }, "text/plain", b"");
                }
                None => {
                    drop(b);
                    respond(&mut s, "400 Bad Request", "text/plain", b"");
                }
            }
        }
        ("POST", "log") => {
            lock().log.push(String::from_utf8_lossy(&req.body).to_string());
            respond(&mut s, "200 OK", "text/plain", b"");
        }
        ("POST", "done") => {
            lock().done.get_or_insert(Ok(()));
            respond(&mut s, "200 OK", "text/plain", b"");
        }
        ("POST", "fail") => {
            let why = String::from_utf8_lossy(&req.body).to_string();
            lock().done.get_or_insert(Err(why));
            respond(&mut s, "200 OK", "text/plain", b"");
        }
        _ => respond(&mut s, "404 Not Found", "text/plain", b""),
    }
}

/// Stop the browser and everything it started (Edge and Chrome fork helper processes).
fn stop(child: &mut Child) {
    if cfg!(windows) {
        let _ = crate::cut::no_window(&mut Command::new("taskkill"))
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// The tests' browser: fetch the job, post a blank frame for each, say done.
fn stand_in(port: u16, token: String) {
    std::thread::spawn(move || {
        let call = |method: &str, path: &str, body: &[u8]| -> Option<Vec<u8>> {
            let mut s = TcpStream::connect(("127.0.0.1", port)).ok()?;
            let head = format!("{method} /{token}/{path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            s.write_all(head.as_bytes()).ok()?;
            s.write_all(body).ok()?;
            let mut out = Vec::new();
            s.read_to_end(&mut out).ok()?;
            let i = out.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
            Some(out[i..].to_vec())
        };
        let Some(job) = call("GET", "job", b"").and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { return };
        let png = crate::previz::Canvas::new(4, 4).png();
        for f in job["frames"].as_array().into_iter().flatten() {
            let _ = call("POST", &format!("frame?n={}", f["n"]), &png);
        }
        let _ = call("POST", "done", b"");
    });
}

/// Draw every frame of the job in one browser launch. `outputs[n]` is where frame `n` goes.
/// Returns which frames came back.
pub fn draw(job: &Value, outputs: Vec<PathBuf>, log_file: &Path) -> Result<Vec<bool>> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    let token = format!("{:016x}", {
        use sha2::Digest;
        let seed = format!("{:?}{}{port}", Instant::now(), std::process::id());
        let h = sha2::Sha256::digest(seed.as_bytes());
        u64::from_le_bytes(h[..8].try_into().unwrap_or([0; 8]))
    });
    let n = outputs.len();
    let board = Arc::new(Mutex::new(Board { outputs, written: vec![false; n], done: None, last: Instant::now(), log: Vec::new() }));
    let job_bytes = Arc::new(serde_json::to_vec(job)?);
    let stop_flag = Arc::new(AtomicBool::new(false));
    let server = {
        let (board, job_bytes, stop_flag, token) = (board.clone(), job_bytes.clone(), stop_flag.clone(), token.clone());
        std::thread::spawn(move || {
            while !stop_flag.load(Ordering::SeqCst) {
                match listener.accept() {
                    // one thread per connection: a socket the browser opens ahead and never uses
                    // must not hold up the next frame
                    Ok((s, _)) => {
                        let (token, job_bytes, board) = (token.clone(), job_bytes.clone(), board.clone());
                        std::thread::spawn(move || serve(s, &token, &job_bytes, &board));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(5)),
                    Err(_) => std::thread::sleep(Duration::from_millis(20)),
                }
            }
        })
    };
    let url = format!("http://127.0.0.1:{port}/{token}/");
    let (w, h) = job["frames"]
        .as_array()
        .and_then(|a| a.first())
        .map(|f| (f["w"].as_u64().unwrap_or(1280), f["h"].as_u64().unwrap_or(720)))
        .unwrap_or((1280, 720));
    let mut child: Option<Child> = None;
    let mut profile: Option<PathBuf> = None;
    if STAND_IN.load(Ordering::SeqCst) {
        stand_in(port, token.clone());
    } else {
        let Some(browser) = find_browser() else {
            stop_flag.store(true, Ordering::SeqCst);
            let _ = server.join();
            bail!("the three engine draws in Edge or Chrome, and neither was found. install one, or set SHOTPILOT_BROWSER to its path");
        };
        // a fresh profile each launch: a browser left running with the same profile would take
        // the URL over and never draw
        let dir = std::env::temp_dir().join(format!("shotpilot-three-{}-{token}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let log = std::fs::File::create(log_file)?;
        let args = [
            "--headless=new".to_string(),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
            "--disable-extensions".into(),
            "--disable-sync".into(),
            "--disable-background-networking".into(),
            "--disable-component-update".into(),
            "--mute-audio".into(),
            "--hide-scrollbars".into(),
            "--ignore-gpu-blocklist".into(),
            format!("--user-data-dir={}", dir.display()),
            format!("--window-size={w},{h}"),
            url,
        ];
        let spawned = crate::cut::no_window(&mut Command::new(&browser))
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn();
        match spawned {
            Ok(c) => child = Some(c),
            Err(e) => {
                stop_flag.store(true, Ordering::SeqCst);
                let _ = server.join();
                bail!("could not start {}: {e}", browser.display());
            }
        }
        profile = Some(dir);
    }
    let total = Duration::from_secs(std::env::var("SHOTPILOT_THREE_TIMEOUT").ok().and_then(|s| s.parse().ok()).unwrap_or(1800));
    let idle = Duration::from_secs(120);
    let t0 = Instant::now();
    let outcome: std::result::Result<(), String> = loop {
        {
            let b = board.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(d) = &b.done {
                break d.clone();
            }
            if b.last.elapsed() > idle {
                break Err(format!("the browser went quiet for {}s", idle.as_secs()));
            }
        }
        if t0.elapsed() > total {
            break Err(format!("drawing took longer than {}s (SHOTPILOT_THREE_TIMEOUT)", total.as_secs()));
        }
        if let Some(c) = child.as_mut() {
            if let Ok(Some(st)) = c.try_wait() {
                break Err(format!("the browser exited ({st}) before it finished"));
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    if let Some(mut c) = child {
        stop(&mut c);
    }
    stop_flag.store(true, Ordering::SeqCst);
    let _ = server.join();
    if let Some(dir) = profile {
        for _ in 0..10 {
            if std::fs::remove_dir_all(&dir).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    let b = board.lock().unwrap_or_else(|p| p.into_inner());
    if !b.log.is_empty() {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_file) {
            for l in &b.log {
                let _ = writeln!(f, "[page] {l}");
            }
        }
    }
    if let Err(why) = outcome {
        bail!("the three engine could not draw: {why}. log: {}", log_file.display());
    }
    Ok(b.written.clone())
}

// --------------------------------------------------------------------------- engine

pub struct ThreeEngine {
    defn: Definition,
    store: SeqStore,
    game: PathBuf,
    logs: PathBuf,
}

/// The game folder of a three project.
pub fn game_dir(defn: &Definition) -> PathBuf {
    let g = PathBuf::from(defn.project.game.clone().unwrap_or_else(|| "game".into()));
    if g.is_absolute() { g } else { defn.root().join(g) }
}

impl ThreeEngine {
    pub fn new(defn: &Definition) -> Result<ThreeEngine> {
        let game = game_dir(defn);
        Scene::load(&game)?; // a missing or broken game says so before anything starts
        let store = SeqStore::new(defn.root().join(".shotpilot").join("three_engine"))?;
        let logs = defn.root().join(".shotpilot").join("logs");
        std::fs::create_dir_all(&logs)?;
        Ok(ThreeEngine { defn: defn.clone(), store, game, logs })
    }

    fn scene(&self) -> Result<Scene> {
        Scene::load(&self.game)
    }

    /// Scout cameras that would look through a wall are moved in front of it (D-55); the
    /// payload stored is the camera that really stood there.
    fn build_with_views(&self, items: &[Value], master: Option<&Value>, force: bool) -> Result<Value> {
        let scene = self.scene()?;
        let mut moved = Map::new();
        let items: Vec<Value> = items
            .iter()
            .map(|it| {
                let mut it = it.clone();
                let Some(target) = vec3(&it["clear_view"]["target"]) else { return it };
                let p = &mut it["payload"];
                let (Some(loc), Some(focal)) = (vec3(&p["start"]["location"]), p["lens"]["focal_length"].as_f64()) else { return it };
                if let Some((at, fl)) = scene.clear_view(target, loc, focal) {
                    for k in ["start", "end"] {
                        p[k]["location"] = json!(at);
                    }
                    p["lens"]["focal_length"] = json!(fl);
                    moved.insert(it["id"].as_str().unwrap_or_default().to_string(), json!({"location": at, "focal_length": fl}));
                }
                it
            })
            .collect();
        let mut out = self.store.build(&items, master, force)?;
        for (id, cv) in moved {
            if out["shots"][&id].is_object() {
                out["shots"][&id]["clear_view"] = cv;
            }
        }
        Ok(out)
    }

    fn job_scene(&self, scene: &Scene) -> Value {
        let objs: Vec<Value> = scene
            .objects
            .iter()
            .map(|o| {
                let mut v = o.raw.clone();
                v["name"] = json!(o.name);
                v
            })
            .collect();
        let props: Map<String, Value> = scene.props.iter().map(|p| (format!("{PROPS_DIR}/{}", p.name), p.raw.clone())).collect();
        json!({"globals": scene.globals, "objects": objs, "props": props})
    }
}

/// What the take records as having been on screen: the scene file (it lights everything) and
/// each object seen, plus the props that popped in (D-37).
fn deps_of(scene: &Scene, seen: &[usize], shot: &ResolvedShot) -> Vec<Value> {
    let mut out = vec![json!({"package": format!("game/{SCENE_FILE}"), "file": scene.dir.join(SCENE_FILE).display().to_string()})];
    for i in seen {
        let o = &scene.objects[*i];
        out.push(json!({"package": format!("game/{OBJECTS_DIR}/{}", o.name), "file": o.file.display().to_string()}));
    }
    for a in &shot.appear {
        if let Some(p) = scene.prop(&a.asset) {
            out.push(json!({"package": format!("game/{PROPS_DIR}/{}", p.name), "file": p.file.display().to_string()}));
        }
    }
    out
}

impl Engine for ThreeEngine {
    fn name(&self) -> &'static str {
        "three"
    }

    fn build(&self, items: &[Value], master: Option<&Value>, force: bool) -> Result<Value> {
        self.store.launch("build", json!({"shots": items.iter().map(|i| &i["id"]).collect::<Vec<_>>()}))?;
        self.build_with_views(items, master, force)
    }

    fn render(&self, build_items: &[Value], master: Option<&Value>, render_items: &[Value], force: bool) -> Result<Value> {
        self.store.launch("render", json!({
            "shots": render_items.iter().map(|i| &i["key"]).collect::<Vec<_>>(),
            "built": build_items.iter().map(|i| &i["id"]).collect::<Vec<_>>(),
        }))?;
        let build = if !build_items.is_empty() || master.is_some() {
            self.build_with_views(build_items, master, force)?
        } else {
            Value::Null
        };
        let blocked = SeqStore::blocked(&build);
        let scene = self.scene()?;
        let resolved = resolve_all(&self.defn)?;
        let t0 = Instant::now();
        let mut renders = Map::new();
        let mut frames = Vec::new();
        let mut outputs = Vec::new();
        // per item: (key, shot, frame indices into `outputs`)
        let mut plan: Vec<(String, ResolvedShot, Vec<usize>, u32, u32)> = Vec::new();
        for it in render_items {
            let key = it["key"].as_str().unwrap_or_default().to_string();
            let id = it["id"].as_str().unwrap_or_default();
            if blocked.iter().any(|b| b == id) {
                renders.insert(key, json!({"status": "error", "error": "build failed or diverged"}));
                continue;
            }
            let p = self.store.asset_file(it["asset"].as_str().unwrap_or_default());
            let Ok(payload) = std::fs::read_to_string(&p).map_err(Error::from).and_then(|t| Ok(serde_json::from_str::<Value>(&t)?)) else {
                renders.insert(key, json!({"status": "error", "error": "sequence missing; build first"}));
                continue;
            };
            let shot = match resolved.iter().find(|s| s.id == id) {
                Some(base) => shot_from_payload(base, &payload),
                None => bare_shot(id, &payload),
            };
            let w = it["resolution"][0].as_u64().unwrap_or(1280).max(2) as u32;
            let h = it["resolution"][1].as_u64().unwrap_or(720).max(2) as u32;
            let q = &it["quality"];
            let (ext, mime) = if q["format"].as_str() == Some("png") { ("png", "image/png") } else { ("jpg", "image/jpeg") };
            let out_dir = PathBuf::from(it["out_dir"].as_str().unwrap_or("."));
            std::fs::create_dir_all(&out_dir)?;
            let mut idx = Vec::new();
            for f in it["frames"].as_array().into_iter().flatten().filter_map(Value::as_i64) {
                let cam = shot.camera_at(f);
                let t = f as f64 / shot.fps.max(1) as f64;
                let appear: Vec<Value> = shot
                    .appear
                    .iter()
                    .filter(|a| f >= a.frame)
                    .map(|a| json!({"asset": a.asset, "at": a.location, "yaw": a.yaw}))
                    .collect();
                let n = outputs.len();
                outputs.push(out_dir.join(format!("frame.{f:04}.{ext}")));
                frames.push(json!({"n": n, "w": w, "h": h, "t": t, "mime": mime,
                                   "cam": frame_camera(&cam, shot.focal_length(), shot.sensor_width(), w, h),
                                   "lens": lens_of(&shot),
                                   "appear": appear}));
                idx.push(n);
            }
            plan.push((key, shot, idx, w, h));
        }
        if !frames.is_empty() {
            let job = json!({"scene": self.job_scene(&scene), "frames": frames});
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f").to_string();
            let written = draw(&job, outputs, &self.logs.join(format!("{stamp}-three.log")))?;
            for (key, shot, idx, w, h) in plan {
                let got = idx.iter().filter(|n| written[**n]).count();
                if got != idx.len() {
                    renders.insert(key, json!({"status": "error", "error": format!("{got} of {} frames came back from the browser", idx.len())}));
                    continue;
                }
                let seen = scene.seen(&shot, w, h);
                renders.insert(key, json!({"status": "ok", "deps": deps_of(&scene, &seen, &shot)}));
            }
        }
        Ok(json!({"build": build, "renders": renders, "seconds": (t0.elapsed().as_secs_f64() * 100.0).round() / 100.0}))
    }

    fn pull(&self, items: &[Value]) -> Result<Value> {
        self.store.pull(items)
    }

    fn scan(&self, _tag: &str, _prefix: &str) -> Result<Value> {
        self.store.launch("scan", json!({}))?;
        Ok(json!({"anchors": self.scene()?.anchors()}))
    }

    fn survey(&self, _tag: &str, _prefix: &str, assets: bool, candidates: bool) -> Result<Value> {
        self.store.launch("survey", json!({"assets": assets, "candidates": candidates}))?;
        let scene = self.scene()?;
        let anchors = scene.anchors();
        let mut out = json!({"anchors": anchors});
        if anchors.is_empty() || candidates {
            out["candidates"] = json!(scene.candidates());
        }
        if assets {
            out["assets"] = json!(scene.inventory());
        }
        Ok(out)
    }

    fn assets(&self) -> Result<Value> {
        self.store.launch("assets", json!({}))?;
        Ok(json!({"assets": self.scene()?.inventory()}))
    }
}
