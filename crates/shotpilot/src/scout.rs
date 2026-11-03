//! Stage 0, location scouting: what is in the project before anything is planned.
//!
//! - Asset inventory: the meshes the engine has (with their size), so a human can pick the
//!   ones shots may make appear (`props:`). Kept in `.shotpilot/assets.json` (derived).
//! - Scout photos: each anchor seen from four sides at a distance that frames it, so the
//!   human and the planner see the level instead of guessing from coordinates. Kept in
//!   `scout/` with an `index.json`. Scout cameras are not shots: they never enter
//!   shots.yaml or the take state.

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::craft::distance_for;
use crate::geometry::{self as g, Vec3};
use crate::model::{fmt_g, num, str_of, vec3, AnchorBox, Definition};

pub const SCOUT_DIR: &str = "scout";
pub const ASSETS_FILE: &str = "assets.json";
/// Directions per anchor, as yaw offsets from "seen from the middle of the set".
pub const SIDES: &[(&str, f64)] = &[("front", 0.0), ("right", 90.0), ("back", 180.0), ("left", 270.0)];

/// One mesh the engine has.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetInfo {
    pub asset: String,
    pub kind: String,
    pub size: Option<Vec3>,
}

impl AssetInfo {
    pub fn to_json(&self) -> Value {
        json!({"asset": self.asset, "kind": self.kind, "size": self.size})
    }

    pub fn from_json(v: &Value) -> Option<AssetInfo> {
        Some(AssetInfo {
            asset: v.get("asset")?.as_str()?.to_string(),
            kind: v.get("kind").map(str_of).unwrap_or_else(|| "static".into()),
            size: v.get("size").and_then(vec3).filter(|s| s.iter().all(|x| *x > 0.0)),
        })
    }

    /// A prop name from the asset's file name: SM_CY_Observer -> observer.
    pub fn suggested_name(&self) -> String {
        let leaf = self.asset.rsplit('/').next().unwrap_or("prop").to_lowercase();
        let parts: Vec<&str> = leaf.split('_').filter(|p| !p.is_empty()).collect();
        let trimmed: Vec<&str> = match parts.first() {
            Some(p) if ["sm", "sk", "skm", "s"].contains(p) && parts.len() > 1 => parts[1..].to_vec(),
            _ => parts,
        };
        let name = trimmed.last().map(|s| s.to_string()).unwrap_or_else(|| "prop".into());
        let clean: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if clean.is_empty() || !clean.chars().next().is_some_and(|c| c.is_ascii_lowercase()) { format!("p_{clean}") } else { clean }
    }
}

pub fn assets_path(root: &Path) -> PathBuf {
    root.join(".shotpilot").join(ASSETS_FILE)
}

pub fn load_assets(root: &Path) -> Vec<AssetInfo> {
    std::fs::read_to_string(assets_path(root))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["assets"].as_array().cloned())
        .map(|a| a.iter().filter_map(AssetInfo::from_json).collect())
        .unwrap_or_default()
}

pub fn save_assets(root: &Path, assets: &[AssetInfo]) -> crate::Result<()> {
    let p = assets_path(root);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d)?;
    }
    let v = json!({"scanned": chrono::Local::now().to_rfc3339(), "assets": assets.iter().map(AssetInfo::to_json).collect::<Vec<_>>()});
    std::fs::write(p, serde_json::to_string_pretty(&v)?)?;
    Ok(())
}

/// A scout camera: where it stands, where it looks, which lens.
#[derive(Debug, Clone)]
pub struct ScoutCam {
    pub anchor: String,
    pub side: String,
    pub location: Vec3,
    pub look_at: Vec3,
    pub focal_length: f64,
}

impl ScoutCam {
    /// A one-frame shot definition for this camera (it goes through resolve like any shot).
    pub fn shot(&self, fps: i64) -> Value {
        json!({
            "id": format!("scout_{}_{}", self.anchor, self.side),
            "duration": 1.0 / fps.max(1) as f64,
            "camera": {"location": self.location, "look_at": self.look_at, "lens": {"focal_length": self.focal_length}},
        })
    }
}

/// Where the cameras stand for each anchor: far enough that its box fills ~40% of the frame
/// height (and half the width), so its surroundings show; a little above its centre, the first one on the side facing
/// the middle of all anchors.
pub fn cameras(defn: &Definition, only: &[String]) -> Vec<ScoutCam> {
    let boxes: Vec<(String, AnchorBox)> = defn.anchor_boxes();
    if boxes.is_empty() {
        return Vec::new();
    }
    let n = boxes.len() as f64;
    let mid = boxes.iter().fold([0.0; 3], |a, (_, b)| g::add(a, g::scale(b.center, 1.0 / n)));
    let sensor = defn.defaults.get("lens").and_then(|l| l.get("sensor_width")).and_then(num).unwrap_or(36.0);
    let (w, h) = defn.teaser.resolution;
    let aspect = if h > 0 { w as f64 / h as f64 } else { 16.0 / 9.0 };
    let focal = 35.0;
    let mut out = Vec::new();
    for (name, b) in boxes.iter().filter(|(n, _)| only.is_empty() || only.contains(n)) {
        let by_height = distance_for(b.size[2] / 0.4, focal, sensor, aspect);
        let wide = b.size[0].max(b.size[1]);
        let by_width = focal * (wide / 0.5) / sensor;
        let d = by_height.max(by_width).max(150.0);
        let to = [b.center[0] - mid[0], b.center[1] - mid[1]];
        let base = if (to[0] * to[0] + to[1] * to[1]).sqrt() < 1.0 { 180.0 } else { to[1].atan2(to[0]).to_degrees() + 180.0 };
        let z = b.center[2] + b.size[2] * 0.15;
        for (side, off) in SIDES {
            let yaw = (base + off).to_radians();
            out.push(ScoutCam {
                anchor: name.clone(),
                side: side.to_string(),
                location: [b.center[0] + d * yaw.cos(), b.center[1] + d * yaw.sin(), z].map(|v| g::round_to(v, 1)),
                look_at: b.center,
                focal_length: focal,
            });
        }
    }
    out
}

/// `scout/index.json`: anchor -> [{side, file, location}], and when it was taken.
pub fn load_index(root: &Path) -> Value {
    std::fs::read_to_string(root.join(SCOUT_DIR).join("index.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({"anchors": {}}))
}

pub fn save_index(root: &Path, idx: &Value) -> crate::Result<()> {
    let d = root.join(SCOUT_DIR);
    std::fs::create_dir_all(&d)?;
    std::fs::write(d.join("index.json"), serde_json::to_string_pretty(idx)?)?;
    Ok(())
}

/// The scout photos the planner gets: per anchor, the view from the middle of the set,
/// each preceded by a line naming it.
pub fn planner_blocks(defn: &Definition, limit: usize) -> Vec<Value> {
    let root = defn.root();
    let idx = load_index(&root);
    let mut out = Vec::new();
    let anchors = idx["anchors"].as_object().cloned().unwrap_or_default();
    for (name, views) in anchors.iter().take(limit) {
        if defn.anchor(name).is_none() {
            continue;
        }
        let Some(front) = views.as_array().and_then(|a| a.iter().find(|v| v["side"] == "front")) else { continue };
        let Some(file) = front["file"].as_str() else { continue };
        let Ok(bytes) = std::fs::read(root.join(file)) else { continue };
        let Some((_, media)) = crate::moodboard::image_type(&bytes) else { continue };
        if bytes.len() > crate::ai::MAX_IMAGE_BYTES {
            continue;
        }
        let loc = front["location"].as_array().map(|a| a.iter().filter_map(num).map(fmt_g).collect::<Vec<_>>().join(", ")).unwrap_or_default();
        out.push(json!({"type": "text", "text": format!("Location scout: `{name}` seen from [{loc}] (35mm, looking at it)")}));
        out.push(json!({"type": "image", "source": {"type": "base64", "media_type": media, "data": crate::ai::base64(&bytes)}}));
    }
    out
}

/// Status for the app: anchors with their scout photos.
pub fn summary(root: &Path) -> Map<String, Value> {
    load_index(root)["anchors"].as_object().cloned().unwrap_or_default()
}
