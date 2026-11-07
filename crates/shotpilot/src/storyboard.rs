//! Storyboard frames: one picture per shot, painted by an image model over the shot's previz
//! layout, so framing can be judged with a near-real look before the engine runs.
//!
//! They are a preview aid only. They live in `storyboard/` (not `render/`), are not takes,
//! never reach the cut, and are keyed by the shot's look fingerprint: an edit that changes
//! the picture makes them stale, editorial edits leave them current. The teaser itself is
//! always rendered from the game.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::fingerprint as fp;
use crate::geometry as g;
use crate::model::{num, Definition, ResolvedShot};
use crate::moodboard::{self, ImageJob};

pub const DIR: &str = "storyboard";
/// The layout an image model starts from (16:9, near the model's own size).
pub const LAYOUT_W: u32 = 1536;
pub const LAYOUT_H: u32 = 864;
/// Game frames passed along to match materials and light.
const MAX_REFS: usize = 3;

/// A shot's storyboard frame, and whether it still matches the shot's picture.
#[derive(Debug, Clone)]
pub struct Board {
    pub file: PathBuf,
    pub fresh: bool,
}

/// The name a storyboard frame is stored under: the shot's look fingerprint.
pub fn key(defn: &Definition, shot: &ResolvedShot) -> String {
    fp::look_fingerprint(defn, shot)[..12].to_string()
}

pub fn shot_dir(root: &Path, id: &str) -> PathBuf {
    root.join(DIR).join(id)
}

/// The frame the storyboard depicts: the middle of the shot.
pub fn mid_frame(shot: &ResolvedShot) -> i64 {
    ((shot.frames - 1) / 2).max(0)
}

/// The shot's storyboard frame: the one for its current picture, else the newest older one.
pub fn current(defn: &Definition, shot: &ResolvedShot) -> Option<Board> {
    let dir = shot_dir(&defn.root(), &shot.id);
    let k = key(defn, shot);
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_board(p))
        .map(|p| (std::fs::metadata(&p).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH), p))
        .collect();
    if let Some((_, p)) = files.iter().find(|(_, p)| p.file_stem().is_some_and(|s| s == k.as_str())) {
        return Some(Board { file: p.clone(), fresh: true });
    }
    files.sort();
    files.pop().map(|(_, file)| Board { file, fresh: false })
}

fn is_board(p: &Path) -> bool {
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    !name.contains(".layout.") && matches!(p.extension().and_then(|e| e.to_str()), Some("png" | "jpg" | "webp"))
}

/// An anchor seen by the camera at a frame: its name, where it sits across the frame (0..1)
/// and how far it is (cm).
#[derive(Debug, Clone)]
pub struct Seen {
    pub name: String,
    pub x: f64,
    pub depth: f64,
}

/// Anchors whose centre is inside the frame, nearest first.
pub fn visible(defn: &Definition, shot: &ResolvedShot, frame: i64) -> Vec<Seen> {
    let cam = shot.camera_at(frame);
    let (fl, sensor) = (shot.focal_length(), shot.sensor_width());
    let mut out: Vec<Seen> = defn
        .anchor_boxes()
        .into_iter()
        .filter_map(|(name, b)| {
            let (x, y, depth) = g::project(b.center, &cam, fl, LAYOUT_W, LAYOUT_H, sensor)?;
            let inside = (0.0..=LAYOUT_W as f64).contains(&x) && (0.0..=LAYOUT_H as f64).contains(&y);
            inside.then(|| Seen { name, x: x / LAYOUT_W as f64, depth })
        })
        .collect();
    out.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal).then(a.name.cmp(&b.name)));
    out
}

fn light(shot: &ResolvedShot) -> Option<&'static str> {
    let e = shot.environment.get("sun_elevation").and_then(num)?;
    Some(if e < -5.0 {
        "night, no sun; light comes only from sources in the scene"
    } else if e < 12.0 {
        "the sun is low on the horizon (dawn or dusk), long shadows, warm low light"
    } else if e < 40.0 {
        "daylight, the sun about a third of the way up the sky"
    } else {
        "midday, the sun high overhead, short shadows"
    })
}

fn side(x: f64) -> &'static str {
    if x < 0.34 {
        "left"
    } else if x > 0.66 {
        "right"
    } else {
        "centre"
    }
}

/// The prompt for one shot's storyboard frame.
pub fn prompt(defn: &Definition, shot: &ResolvedShot, direction: &str) -> String {
    let frame = mid_frame(shot);
    let mut lines = vec![format!(
        "A photorealistic still frame from a video game teaser, 16:9, as rendered in real time by a modern game \
         engine (Unreal Engine 5): physically based materials, global illumination, believable scale. \
         Shot with a {:.0} mm lens. No text, no logos, no user interface, no watermark.",
        shot.focal_length()
    )];
    if !shot.purpose.is_empty() {
        lines.push(format!("What the shot is for: {}", shot.purpose));
    }
    let seen = visible(defn, shot, frame);
    if !seen.is_empty() {
        lines.push("In frame:".into());
        for s in &seen {
            let what = defn.stage.anchor(&s.name).filter(|d| !d.is_empty()).map(|d| format!(": {d}")).unwrap_or_default();
            lines.push(format!("- `{}` ({}, about {:.0} m away){what}", s.name, side(s.x), s.depth / 100.0));
        }
    }
    let props: Vec<&str> = shot.appear.iter().filter(|a| frame >= a.frame).map(|a| a.prop.as_str()).collect();
    if !props.is_empty() {
        lines.push(format!("Also in frame (the red boxes): {}", props.join(", ")));
    }
    if let Some(l) = light(shot) {
        lines.push(format!("Light: {l}"));
    }
    lines.extend(moodboard::tone(defn, direction));
    lines.join("\n")
}

/// The front scout photo of an anchor, if one was taken.
fn scout_photo(defn: &Definition, anchor: &str) -> Option<PathBuf> {
    let root = defn.root();
    let idx = crate::scout::load_index(&root);
    let file = idx["anchors"][anchor].as_array()?.iter().find(|v| v["side"] == "front")?["file"].as_str()?.to_string();
    let p = root.join(file);
    p.is_file().then_some(p)
}

/// The image job for a shot: its previz layout (written next to the frames as
/// `<key>.layout.png`) as the base, the scout photos of what is in frame and the first
/// mood-board image as references.
pub fn job(defn: &Definition, shot: &ResolvedShot, direction: &str) -> Result<ImageJob> {
    let root = defn.root();
    let dir = shot_dir(&root, &shot.id);
    std::fs::create_dir_all(&dir)?;
    let frame = mid_frame(shot);
    let layout = dir.join(format!("{}.layout.png", key(defn, shot)));
    let cv = crate::previz::render_layout(shot, frame, &crate::previz::World::load(defn), LAYOUT_W, LAYOUT_H);
    std::fs::write(&layout, cv.png())?;
    let mut refs: Vec<PathBuf> = visible(defn, shot, frame).iter().filter_map(|s| scout_photo(defn, &s.name)).collect();
    refs.dedup();
    refs.truncate(MAX_REFS - 1);
    if let Some(m) = defn.brief.images.iter().find_map(|rel| moodboard::resolve(&root, rel).filter(|p| p.is_file())) {
        refs.push(m);
    }
    Ok(ImageJob { prompt: prompt(defn, shot, direction), base: Some(layout), layout: true, refs })
}

/// Store a generated frame for a shot; returns its path.
pub fn store(defn: &Definition, shot: &ResolvedShot, bytes: &[u8]) -> Result<PathBuf> {
    if bytes.len() > moodboard::MAX_BYTES {
        crate::bail!("image is larger than {} MB", moodboard::MAX_BYTES / 1_000_000);
    }
    let Some((ext, _)) = moodboard::image_type(bytes) else { crate::bail!("not a PNG, JPEG or WebP image") };
    let dir = shot_dir(&defn.root(), &shot.id);
    std::fs::create_dir_all(&dir)?;
    let k = key(defn, shot);
    for old in ["png", "jpg", "webp"] {
        let _ = std::fs::remove_file(dir.join(format!("{k}.{old}"))); // a regenerated frame replaces the last one
    }
    let p = dir.join(format!("{k}.{ext}"));
    std::fs::write(&p, bytes)?;
    Ok(p)
}
