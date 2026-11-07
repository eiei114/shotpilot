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

/// Where a scout camera really stood, when the engine moved it: a camera whose line to the anchor
/// crosses a wall is pulled in front of the wall with a wider lens (`clear_view` in the build
/// result). None when it stayed put (or the engine has no geometry to check, like mock).
pub fn moved_camera(built: &Value) -> Option<(Vec3, f64)> {
    let cv = &built["clear_view"];
    Some((vec3(&cv["location"])?, cv["focal_length"].as_f64()?))
}

/// `scout/index.json`: anchor -> [{side, file, location, focal_length}], and when it was taken.
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
    scout_blocks(defn, None, limit)
}

/// [`planner_blocks`] for the named anchors only (an edit gets the places its shots use).
pub fn planner_blocks_for(defn: &Definition, names: &[String], limit: usize) -> Vec<Value> {
    scout_blocks(defn, Some(names), limit)
}

fn scout_blocks(defn: &Definition, only: Option<&[String]>, limit: usize) -> Vec<Value> {
    let root = defn.root();
    let idx = load_index(&root);
    let mut out = Vec::new();
    let anchors = idx["anchors"].as_object().cloned().unwrap_or_default();
    let wanted = anchors.iter().filter(|(name, _)| only.map_or(true, |o| o.iter().any(|n| n == *name)));
    for (name, views) in wanted.take(limit) {
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

/// A landmark the engine offers when nothing in the level is tagged (D-44, D-45).
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// An anchor id made from the label (SM_Old_Well3 -> old_well).
    pub name: String,
    /// The actor label in the level.
    pub label: String,
    /// The actor class.
    pub class: String,
    /// What it is to a scout (room, corridor, light, door, character, effect, item, prop, start),
    /// with what else stands there; empty from an older engine script.
    pub kind: String,
    /// How much it is there (cube root of its volume, capped).
    pub score: f64,
    pub at: Vec3,
    pub b: AnchorBox,
}

impl Candidate {
    pub fn from_json(v: &Value) -> Option<Candidate> {
        let name = v.get("name")?.as_str()?.to_string();
        let (at, b) = crate::model::parse_anchor(v).ok()?;
        Some(Candidate {
            label: v.get("label").map(str_of).unwrap_or_else(|| name.clone()),
            class: v.get("cls").or_else(|| v.get("class")).map(str_of).unwrap_or_default(),
            kind: {
                let kinds: Vec<String> = v.get("kinds").and_then(Value::as_array).into_iter().flatten().map(str_of).collect();
                if kinds.len() > 1 { kinds.join(" + ") } else { v.get("kind").map(str_of).unwrap_or_default() }
            },
            score: v.get("score").and_then(num).unwrap_or(0.0),
            name,
            at,
            b: b?,
        })
    }

    /// The anchors: entry for shots.yaml.
    pub fn entry(&self) -> Value {
        let r = |v: Vec3| v.map(|x| g::round_to(x, 1));
        if self.b.center == self.at {
            json!({"at": r(self.at), "size": r(self.b.size)})
        } else {
            json!({"at": r(self.at), "size": r(self.b.size), "center": r(self.b.center)})
        }
    }

    /// One line for the planner: name, what it is, size, where.
    pub fn line(&self) -> String {
        let f = |v: Vec3| v.iter().map(|x| fmt_g(g::round_to(*x, 0))).collect::<Vec<_>>().join(", ");
        let what = if self.kind.is_empty() { String::new() } else { format!("{}: ", self.kind) };
        format!("- `{}` ({what}{}, {}): size [{}] cm at [{}]", self.name, self.label, self.class, f(self.b.size), f(self.b.center))
    }
}

/// The fallback pick without Claude: the largest first, then each next one far from those
/// picked (weighted by its size), nothing closer than 3 m, so the photos cover the level.
pub fn spread(cands: &[Candidate], limit: usize) -> Vec<Candidate> {
    let mut pool: Vec<&Candidate> = cands.iter().collect();
    pool.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let mut picked: Vec<&Candidate> = Vec::new();
    while !pool.is_empty() && picked.len() < limit {
        let next = if picked.is_empty() {
            Some(0)
        } else {
            let near = |c: &Candidate| picked.iter().map(|p| g::length(g::sub(c.b.center, p.b.center))).fold(f64::INFINITY, f64::min);
            pool.iter()
                .enumerate()
                .filter(|(_, c)| near(c) >= 300.0)
                .max_by(|(_, a), (_, b)| (a.score * near(a)).partial_cmp(&(b.score * near(b))).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
        };
        let Some(i) = next else { break };
        picked.push(pool.remove(i));
    }
    picked.into_iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_candidate_tells_the_picker_what_kind_of_place_it_is() {
        // D-84: places, not building blocks: what stands there reaches Claude's pick
        let v = serde_json::json!({"name": "doorframe", "label": "BP_DoorFrame + SpotLight7", "cls": "BP_DoorFrame_C",
            "kind": "door", "kinds": ["door", "light"], "score": 540.0,
            "at": [-1700, -1250, 0], "size": [500, 900, 400], "center": [-1700, -1250, 200]});
        let c = Candidate::from_json(&v).unwrap();
        assert_eq!(c.kind, "door + light");
        assert!(c.line().starts_with("- `doorframe` (door + light: BP_DoorFrame + SpotLight7, BP_DoorFrame_C)"), "{}", c.line());
        // an older engine script sends no kind: the line reads as before
        let old = serde_json::json!({"name": "well", "label": "SM_Well", "cls": "StaticMeshActor", "score": 1.0,
            "at": [0, 0, 0], "size": [100, 100, 100]});
        assert!(Candidate::from_json(&old).unwrap().line().starts_with("- `well` (SM_Well, StaticMeshActor)"));
    }
}
