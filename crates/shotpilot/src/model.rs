//! shots.yaml: the definition side of Shotpilot.
//!
//! The definition is the single source of truth. Everything under render/ and every
//! engine asset is derived from it, so any shot can be rebuilt from this file alone.

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::bail;
use crate::error::{Error, Result};
use crate::geometry::{self as g, Rot, Vec3, Xform};
use crate::yamlio;

pub const SCHEMA_VERSION: i64 = 1;

/// Sun elevation in degrees above the horizon. UE DirectionalLight pitch = -elevation.
pub const TIME_OF_DAY: &[(&str, f64)] = &[
    ("dawn", 6.0),
    ("morning", 25.0),
    ("noon", 65.0),
    ("afternoon", 35.0),
    ("golden_hour", 10.0),
    ("dusk", 3.0),
    ("night", -18.0),
];

/// The teaser shape Shotpilot ships with. Beats are editorial: they never affect pixels.
pub const BEATS: &[&str] = &["hook", "establish", "escalate", "title"];

pub fn beat_guide(beat: &str) -> &'static str {
    match beat {
        "hook" => "first 2-5 s: one striking image or question that stops the scroll",
        "establish" => "where we are and what kind of game this is",
        "escalate" => "tension, stakes or spectacle rising shot by shot",
        _ => "a calm final image with room for the title card",
    }
}

pub const MOVES: &[&str] = &[
    "crane_down", "crane_up", "dolly_in", "dolly_out", "orbit_left", "orbit_right", "static", "truck_left",
    "truck_right",
];
pub const CURVES: &[&str] = &["ease_in", "ease_in_out", "ease_out", "linear"];

pub fn tod_elevation(name: &str) -> Option<f64> {
    TIME_OF_DAY.iter().find(|(n, _)| *n == name).map(|(_, e)| *e)
}

pub fn tod_names_sorted() -> Vec<&'static str> {
    let mut v: Vec<&str> = TIME_OF_DAY.iter().map(|(n, _)| *n).collect();
    v.sort_unstable();
    v
}

pub fn valid_id(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 48
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_' || *c == b'-')
}

/// `/Game/Maps/Courtyard`-style UE package path.
pub fn valid_package(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('/') else { return false };
    let segs: Vec<&str> = rest.split('/').collect();
    segs.len() >= 2
        && !segs[0].is_empty()
        && segs[0].chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && segs[1..]
            .iter()
            .all(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "_-. ".contains(c)))
}

#[derive(Debug, Clone)]
pub struct ProjectCfg {
    pub name: String,
    pub engine: String, // mock | ue5
    pub uproject: Option<String>,
    pub map: Option<String>,
    pub ue_cmd: Option<String>,
    pub content_root: String,
    pub sun_actor: Option<String>,
    pub isolate_userdir: bool,
    /// Extra UnrealEditor arguments for this project (e.g. a game's own flags).
    pub ue_args: Vec<String>,
    /// GameMode class MRQ should run (e.g. /Script/MyGame.MyMode); None = MRQ's own empty mode.
    pub game_mode: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TeaserCfg {
    pub title: String,
    pub pitch: String,
    pub target_duration: f64,
    pub fps: i64,
    pub resolution: (i64, i64),
}

/// `stage:`: what the level is, read from the scout photos (written by Claude, editable by
/// hand). Editorial: it feeds the planner and the mood board, never pixels.
#[derive(Debug, Clone, Default)]
pub struct StageCfg {
    pub summary: String,
    /// anchor -> what is there, light, materials, where a camera has room.
    pub anchors: Vec<(String, String)>,
    /// mood-board frame (place, detail, threat, title) -> the anchor whose scout photo it is based on.
    pub mood: Vec<(String, String)>,
}

impl StageCfg {
    pub fn is_empty(&self) -> bool {
        self.summary.is_empty() && self.anchors.is_empty()
    }

    pub fn anchor(&self, name: &str) -> Option<&str> {
        self.anchors.iter().find(|(n, _)| n == name).map(|(_, d)| d.as_str())
    }

    pub fn mood_base(&self, frame: &str) -> Option<&str> {
        self.mood.iter().find(|(f, _)| f == frame).map(|(_, a)| a.as_str())
    }
}

/// What a trailer is: a teaser (title/IP first) or a gameplay trailer (gameplay first).
/// Steam treats them as different kinds, and some checks weigh differently for each.
pub const TRAILER_KINDS: &[&str] = &["teaser", "gameplay"];

/// Fields of `brief:` a planner reads. Order is the order the app asks them in.
pub const BRIEF_FIELDS: &[&str] = &[
    "kind", "genre", "style", "audience", "core", "must_show", "references", "notes", "images", "design", "skills", "plan",
    "direction", "reference_teaser",
];

/// `brief:`: what the trailer is for, written before any shot. Editorial only: it never
/// affects pixels, so editing it never re-renders anything.
#[derive(Debug, Clone, Default)]
pub struct BriefCfg {
    pub kind: String, // teaser | gameplay
    pub genre: String,
    /// A recently popular teaser whose structure to follow (an id of refs::styles, D-39).
    pub style: String,
    pub audience: String,
    /// The core experience the viewer must get (one sentence).
    pub core: String,
    pub must_show: Vec<String>,
    /// Trailers to learn from ("Inspired by ...").
    pub references: Vec<String>,
    /// Mood-board images (paths under moodboard/): palette, light and tone, not shot designs.
    pub images: Vec<String>,
    pub notes: String,
    /// The look: a bundled design system id (designsys, D-45). A project `DESIGN.md` wins for the prose.
    pub design: String,
    /// Skill ids the planning turn picked (skills, D-45).
    pub skills: Vec<String>,
    /// The planning turn's plan, in prose (goal, deliverable, direction, steps, assumptions).
    pub plan: String,
    /// The direction in one sentence, so the human can redirect cheaply.
    pub direction: String,
    /// A measured reference teaser (reference/<id>/, D-71) whose rhythm and hand-overs the cut
    /// borrows; never its pictures, words or sound.
    pub reference_teaser: String,
    /// The raw mapping, for the app's form.
    pub raw: Map<String, Value>,
}

impl BriefCfg {
    pub fn is_gameplay(&self) -> bool {
        self.kind == "gameplay"
    }

    /// Fields a planner needs that are still empty.
    pub fn missing(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.genre.is_empty() {
            out.push("genre");
        }
        if self.core.is_empty() {
            out.push("core");
        }
        out
    }
}

/// Size of what sits at an anchor, when known (from `scan`, or written by hand).
/// Without one, previz draws a person-sized box (120 x 120 x 180 cm) centred on the anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnchorBox {
    /// World-space centre of the box (cm).
    pub center: Vec3,
    /// Extent along X, Y, Z (cm).
    pub size: Vec3,
}

pub const DEFAULT_BOX: Vec3 = [120.0, 120.0, 180.0];

/// Where on an anchor the camera looks (`camera.aim`): `base` is the anchor's own point (a
/// mannequin's feet, a building's footing; what a camera always aimed at), `center` the middle of
/// its box, `upper` its upper part (a person's head and shoulders, the top of a tower). Shots
/// that leave it out resolve as before, so their takes stay valid.
pub const AIMS: &[&str] = &["base", "center", "upper"];

/// What a prop is in the engine: a static mesh, or a skeletal mesh (a character, shown in
/// its reference pose: no animation).
pub const PROP_KINDS: &[&str] = &["static", "skeletal"];

/// `props.<name>`: something a shot can make appear (an enemy, an object).
#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    /// UE asset path of the mesh, e.g. /Game/Characters/Mannequins/Meshes/SKM_Manny.
    pub asset: String,
    pub kind: String,
    /// Extent along X, Y, Z (cm), for previz and framing. The prop stands on its location.
    pub size: Vec3,
}

/// A prop popping into a shot at a frame (no animation: it is simply there from then on).
#[derive(Debug, Clone, PartialEq)]
pub struct Appear {
    pub prop: String,
    pub asset: String,
    pub kind: String,
    /// First frame it is visible (0 = from the start of the shot).
    pub frame: i64,
    /// Where it stands (its feet / base), world cm.
    pub location: Vec3,
    /// Facing, degrees (UE yaw).
    pub yaw: f64,
    pub size: Vec3,
}

impl Appear {
    pub fn to_json(&self) -> Value {
        json!({"prop": self.prop, "asset": self.asset, "kind": self.kind, "frame": self.frame,
               "location": self.location, "rotation": [0.0, self.yaw, 0.0], "size": self.size})
    }

    pub fn from_json(v: &Value) -> Option<Appear> {
        Some(Appear {
            prop: v.get("prop")?.as_str()?.to_string(),
            asset: v.get("asset").map(str_of).unwrap_or_default(),
            kind: v.get("kind").map(str_of).unwrap_or_else(|| "static".into()),
            frame: v.get("frame")?.as_i64()?,
            location: vec3(v.get("location")?)?,
            yaw: v.get("rotation").and_then(vec3).map(|r| r[1]).unwrap_or(0.0),
            size: v.get("size").and_then(vec3).unwrap_or(DEFAULT_BOX),
        })
    }

    /// The box previz draws (standing on its location).
    pub fn bbox(&self) -> AnchorBox {
        AnchorBox { center: [self.location[0], self.location[1], self.location[2] + self.size[2] / 2.0], size: self.size }
    }
}

impl AnchorBox {
    pub fn corners(&self) -> [Vec3; 8] {
        let mut out = [[0.0; 3]; 8];
        let mut i = 0;
        for sz in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sx in [-1.0, 1.0] {
                    out[i] = [
                        self.center[0] + sx * self.size[0] / 2.0,
                        self.center[1] + sy * self.size[1] / 2.0,
                        self.center[2] + sz * self.size[2] / 2.0,
                    ];
                    i += 1;
                }
            }
        }
        out
    }

    pub fn contains(&self, p: Vec3, margin: f64) -> bool {
        (0..3).all(|k| (p[k] - self.center[k]).abs() <= self.size[k] / 2.0 + margin)
    }

    pub fn to_json(&self) -> Value {
        json!({"center": self.center, "size": self.size})
    }
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub path: PathBuf,
    pub project: ProjectCfg,
    pub teaser: TeaserCfg,
    pub brief: BriefCfg,
    pub stage: StageCfg,
    /// `finish:`: letterbox, title, end card (burned in at cut; see finish.rs).
    pub finish: crate::finish::FinishCfg,
    /// `script:`: the teaser's text, cards between shots and captions on them (script.rs).
    pub script: Vec<crate::script::Line>,
    pub anchors: Vec<(String, Vec3)>,
    /// Anchors whose size is known (`anchors.<name>: {at, size, center}`).
    pub boxes: Vec<(String, AnchorBox)>,
    /// `props:`: what shots can make appear.
    pub props: Vec<(String, Prop)>,
    pub defaults: Map<String, Value>,
    pub qualities: Map<String, Value>,
    /// `audio:` (BGM, master gain). Sound effects live on their shots (`sfx:`). See sound.rs.
    pub audio: Map<String, Value>,
    pub shots: Vec<Value>,
}

impl Definition {
    pub fn root(&self) -> PathBuf {
        self.path.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    pub fn shot_ids(&self) -> Vec<String> {
        self.shots.iter().map(|s| str_of(&s["id"])).collect()
    }

    pub fn index_of(&self, shot_id: &str) -> Result<usize> {
        self.shots
            .iter()
            .position(|s| s["id"].as_str() == Some(shot_id))
            .ok_or_else(|| Error::user(format!("unknown shot '{shot_id}'. known: {}", self.shot_ids().join(", "))))
    }

    pub fn find(&self, shot_id: &str) -> Result<&Value> {
        Ok(&self.shots[self.index_of(shot_id)?])
    }

    pub fn anchor(&self, name: &str) -> Option<Vec3> {
        self.anchors.iter().find(|(n, _)| n == name).map(|(_, v)| *v)
    }

    pub fn prop(&self, name: &str) -> Option<&Prop> {
        self.props.iter().find(|(n, _)| n == name).map(|(_, p)| p)
    }

    /// The known box of an anchor, if its size was given.
    pub fn known_box(&self, name: &str) -> Option<AnchorBox> {
        self.boxes.iter().find(|(n, _)| n == name).map(|(_, b)| *b)
    }

    /// The box to draw for an anchor: its known size, or a person-sized default.
    pub fn anchor_box(&self, name: &str) -> Option<AnchorBox> {
        self.known_box(name).or_else(|| self.anchor(name).map(|at| AnchorBox { center: at, size: DEFAULT_BOX }))
    }

    /// Every anchor with the box previz draws for it.
    pub fn anchor_boxes(&self) -> Vec<(String, AnchorBox)> {
        self.anchors.iter().filter_map(|(n, _)| self.anchor_box(n).map(|b| (n.clone(), b))).collect()
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedShot {
    pub id: String,
    pub index: usize,
    pub purpose: String,
    pub duration: f64,
    pub fps: i64,
    pub frames: i64,
    pub start: Xform,
    pub end: Xform,
    pub curve: String,
    pub lens: Map<String, Value>,
    pub environment: Map<String, Value>,
    pub markers: Vec<Value>,
    pub target: Option<Vec3>,
    pub notes: String,
    pub beat: Option<String>,
    /// Props that pop in during the shot.
    pub appear: Vec<Appear>,
}

impl ResolvedShot {
    pub fn camera_at(&self, frame: i64) -> Xform {
        if self.frames <= 1 {
            return self.start;
        }
        let t = g::ease(frame as f64 / (self.frames - 1) as f64, &self.curve);
        g::interpolate(&self.start, &self.end, t)
    }

    pub fn focal_length(&self) -> f64 {
        num(&self.lens["focal_length"]).unwrap_or(35.0)
    }

    pub fn sensor_width(&self) -> f64 {
        self.lens.get("sensor_width").and_then(num).unwrap_or(36.0)
    }

    /// Everything that gets written into the engine's shot sequence.
    pub fn engine_payload(&self) -> Value {
        let mut p = json!({
            "frames": self.frames,
            "fps": self.fps,
            "curve": self.curve,
            "start": self.start.to_json(),
            "end": self.end.to_json(),
            "lens": self.lens,
            "environment": self.environment,
            "markers": self.markers,
        });
        // only when used, so fingerprints of shots without props stay as they were
        if !self.appear.is_empty() {
            p["appear"] = Value::Array(self.appear.iter().map(Appear::to_json).collect());
        }
        p
    }
}

// --------------------------------------------------------------------------- value helpers

/// A number (int or float); bools are not numbers.
pub fn num(v: &Value) -> Option<f64> {
    v.as_f64()
}

/// Python-ish `str(v)`: strings as-is, null as "", numbers formatted.
pub fn str_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Number(_) | Value::Bool(_) => v.to_string(),
        other => other.to_string(),
    }
}

fn opt_str(v: Option<&Value>) -> Option<String> {
    match v {
        None | Some(Value::Null) => None,
        Some(x) => Some(str_of(x)),
    }
}

pub fn vec3(v: &Value) -> Option<Vec3> {
    let a = v.as_array()?;
    if a.len() != 3 {
        return None;
    }
    Some([num(&a[0])?, num(&a[1])?, num(&a[2])?])
}

fn obj(v: Option<&Value>) -> Map<String, Value> {
    match v {
        Some(Value::Object(m)) => m.clone(),
        _ => Map::new(),
    }
}

/// `{:g}`-style compact number formatting for messages.
pub fn fmt_g(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

// --------------------------------------------------------------------------- loading

pub fn load(path: &Path) -> Result<Definition> {
    if !path.exists() {
        bail!("{} not found. run `shotpilot init` first.", path.display());
    }
    let data = yamlio::load(path)?;
    parse(&data, path)
}

pub fn parse(data: &Value, path: &Path) -> Result<Definition> {
    let Some(top) = data.as_object() else {
        bail!("{}: top level must be a mapping", path.display());
    };
    let mut errs: Vec<String> = Vec::new();
    match top.get("version") {
        None => {}
        Some(v) if v.as_i64() == Some(SCHEMA_VERSION) => {}
        Some(v) => errs.push(format!("version: expected {SCHEMA_VERSION}, got {v}")),
    }

    let p = obj(top.get("project"));
    let dir_name = path
        .canonicalize()
        .ok()
        .and_then(|p| p.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "shotpilot".into());
    let project = ProjectCfg {
        name: opt_str(p.get("name")).filter(|s| !s.is_empty()).unwrap_or(dir_name),
        engine: opt_str(p.get("engine")).unwrap_or_else(|| "mock".into()),
        uproject: opt_str(p.get("uproject")),
        map: opt_str(p.get("map")),
        ue_cmd: opt_str(p.get("ue_cmd")),
        content_root: opt_str(p.get("content_root"))
            .unwrap_or_else(|| "/Game/Shotpilot".into())
            .trim_end_matches('/')
            .to_string(),
        sun_actor: opt_str(p.get("sun_actor")),
        isolate_userdir: p.get("isolate_userdir").and_then(Value::as_bool).unwrap_or(true),
        game_mode: opt_str(p.get("game_mode")).filter(|s| !s.is_empty()),
        ue_args: match p.get("ue_args") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::String(s)) => s.split_whitespace().map(str::to_string).collect(),
            Some(Value::Array(a)) if a.iter().all(Value::is_string) => {
                a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()
            }
            Some(_) => {
                errs.push("project.ue_args: a list of strings, e.g. [-MyGameFlag]".into());
                Vec::new()
            }
        },
    };
    if project.engine != "mock" && project.engine != "ue5" {
        errs.push(format!("project.engine: must be 'mock' or 'ue5', got '{}'", project.engine));
    }
    if project.engine == "ue5" {
        if project.uproject.is_none() {
            errs.push("project.uproject: required when engine is ue5".into());
        }
        if project.map.is_none() {
            errs.push("project.map: required when engine is ue5 (e.g. /Game/Maps/Courtyard)".into());
        }
    }
    if let Some(g) = &project.game_mode {
        if !g.starts_with('/') || !g.contains('.') {
            errs.push(format!("project.game_mode: a class path like /Script/MyGame.MyGameMode (got '{g}')"));
        }
    }
    if let Some(m) = &project.map {
        if !valid_package(m) {
            errs.push(format!(
                "project.map: must be a UE package path like /Game/Maps/Courtyard (got '{m}'). \
                 Git Bash rewrites /Game/... arguments into Windows paths; set MSYS_NO_PATHCONV=1 or edit shots.yaml"
            ));
        }
    }

    let t = obj(top.get("teaser"));
    let res = t.get("resolution").and_then(Value::as_array).cloned().unwrap_or_default();
    let teaser = TeaserCfg {
        title: opt_str(t.get("title")).unwrap_or_default(),
        pitch: opt_str(t.get("pitch")).unwrap_or_default(),
        target_duration: t.get("target_duration").and_then(num).unwrap_or(45.0),
        fps: t.get("fps").and_then(num).map(|f| f as i64).unwrap_or(30),
        resolution: (
            res.first().and_then(num).map(|f| f as i64).unwrap_or(1920),
            res.get(1).and_then(num).map(|f| f as i64).unwrap_or(1080),
        ),
    };
    if teaser.fps <= 0 || teaser.fps > 120 {
        errs.push("teaser.fps: must be 1..120".into());
    }

    let brief = parse_brief(top.get("brief"), &mut errs);
    let finish = crate::finish::parse(top.get("finish"), &mut errs);
    let stage = {
        let st = obj(top.get("stage"));
        let pairs = |k: &str| -> Vec<(String, String)> {
            obj(st.get(k)).into_iter().filter(|(_, v)| !v.is_null()).map(|(n, v)| (n, str_of(&v))).collect()
        };
        StageCfg { summary: opt_str(st.get("summary")).unwrap_or_default(), anchors: pairs("anchors"), mood: pairs("mood") }
    };

    let mut anchors = Vec::new();
    let mut boxes = Vec::new();
    for (name, v) in obj(top.get("anchors")) {
        match parse_anchor(&v) {
            Ok((at, b)) => {
                anchors.push((name.clone(), at));
                if let Some(b) = b {
                    boxes.push((name, b));
                }
            }
            Err(e) => errs.push(format!("anchors.{name}: {e}")),
        }
    }

    let mut props = Vec::new();
    for (name, v) in obj(top.get("props")) {
        if v.is_null() {
            continue; // removed (`shotpilot prop rm` leaves `name: null`)
        }
        match parse_prop(&v) {
            Ok(p) => props.push((name, p)),
            Err(e) => errs.push(format!("props.{name}: {e}")),
        }
    }

    let defaults = obj(top.get("defaults"));
    let qualities = obj(top.get("qualities"));
    let audio = obj(top.get("audio"));
    let shots: Vec<Value> = match top.get("shots") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(a)) => a.clone(),
        Some(_) => {
            errs.push("shots: must be a list".into());
            Vec::new()
        }
    };

    let mut seen = std::collections::HashSet::new();
    for (i, s) in shots.iter().enumerate() {
        let at = format!("shots[{i}]");
        if !s.is_object() {
            errs.push(format!("{at}: must be a mapping"));
            continue;
        }
        match s.get("id").and_then(Value::as_str) {
            Some(sid) if valid_id(sid) => {
                if !seen.insert(sid.to_string()) {
                    errs.push(format!("{at}.id: duplicate '{sid}'"));
                }
            }
            other => errs.push(format!(
                "{at}.id: lowercase letters/digits/_/- (got {})",
                other.map(|x| format!("'{x}'")).unwrap_or_else(|| s.get("id").map(Value::to_string).unwrap_or("None".into()))
            )),
        }
        if !s.get("duration").and_then(num).is_some_and(|d| d > 0.0) {
            errs.push(format!("{at}.duration: positive seconds required"));
        }
        crate::finish::validate_shot(&at, s, &mut errs);
        crate::fx::validate(&at, s, &mut errs);
        match s.get("beat") {
            None | Some(Value::Null) => {}
            Some(Value::String(b)) if BEATS.contains(&b.as_str()) => {}
            Some(b) => errs.push(format!("{at}.beat: one of {} (got {b})", BEATS.join(", "))),
        }
    }

    crate::sound::validate(top, &shots, &mut errs);
    let shot_ids: Vec<String> = shots.iter().map(|s| str_of(&s["id"])).collect();
    let script = crate::script::parse(top.get("script"), &shot_ids, &mut errs);

    let defn = Definition { path: path.to_path_buf(), project, teaser, brief, stage, finish, script, anchors, boxes, props, defaults, qualities, audio, shots };
    if errs.is_empty() {
        for i in 0..defn.shots.len() {
            if let Err(e) = resolve_shot(&defn, &defn.shots[i], i) {
                errs.push(e.msg);
            }
        }
    }
    if !errs.is_empty() {
        bail!("{} has {} problem(s):\n  - {}", path.display(), errs.len(), errs.join("\n  - "));
    }
    Ok(defn)
}

/// `[x, y, z]`, or `{at: [x, y, z], size: [w, d, h], center: [x, y, z]}` (centre defaults to `at`).
pub fn parse_anchor(v: &Value) -> std::result::Result<(Vec3, Option<AnchorBox>), String> {
    if let Some(at) = vec3(v) {
        return Ok((at, None));
    }
    let Some(m) = v.as_object() else { return Err("must be [x, y, z] or {at: [x, y, z], size: [w, d, h]}".into()) };
    let at = m.get("at").and_then(vec3).ok_or("`at` must be [x, y, z]")?;
    let b = match m.get("size") {
        None | Some(Value::Null) => None,
        Some(s) => {
            let size = vec3(s).filter(|s| s.iter().all(|v| *v > 0.0)).ok_or("`size` must be [w, d, h] in cm, all > 0")?;
            let center = match m.get("center") {
                None | Some(Value::Null) => at,
                Some(c) => vec3(c).ok_or("`center` must be [x, y, z]")?,
            };
            Some(AnchorBox { center, size })
        }
    };
    Ok((at, b))
}

/// `{asset: /Game/..., kind: static|skeletal, size: [w, d, h]}`.
fn parse_prop(v: &Value) -> std::result::Result<Prop, String> {
    let m = v.as_object().ok_or("must be {asset: /Game/..., kind: static|skeletal, size: [w, d, h]}")?;
    let asset = m.get("asset").map(str_of).unwrap_or_default();
    if !valid_package(&asset) {
        return Err(format!(
            "asset: a UE asset path like /Game/Characters/Mannequins/Meshes/SKM_Manny (got '{asset}').              Git Bash rewrites /Game/... arguments into Windows paths; set MSYS_NO_PATHCONV=1"
        ));
    }
    let kind = m.get("kind").map(str_of).unwrap_or_else(|| "static".into());
    if !PROP_KINDS.contains(&kind.as_str()) {
        return Err(format!("kind: one of {} (got '{kind}')", PROP_KINDS.join(", ")));
    }
    let size = match m.get("size") {
        None | Some(Value::Null) => if kind == "skeletal" { [60.0, 40.0, 180.0] } else { [100.0, 100.0, 100.0] },
        Some(s) => vec3(s).filter(|s| s.iter().all(|v| *v > 0.0)).ok_or("size: [w, d, h] in cm, all > 0")?,
    };
    Ok(Prop { asset, kind, size })
}

fn parse_brief(v: Option<&Value>, errs: &mut Vec<String>) -> BriefCfg {
    let raw = match v {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(m)) => m.clone(),
        Some(_) => {
            errs.push("brief: must be a mapping".into());
            Map::new()
        }
    };
    let list = |key: &str, errs: &mut Vec<String>| -> Vec<String> {
        match raw.get(key) {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::String(s)) if s.is_empty() => Vec::new(),
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Array(a)) => a.iter().map(str_of).filter(|s| !s.is_empty()).collect(),
            Some(_) => {
                errs.push(format!("brief.{key}: a list of strings"));
                Vec::new()
            }
        }
    };
    let kind = opt_str(raw.get("kind")).unwrap_or_else(|| "teaser".into());
    if !TRAILER_KINDS.contains(&kind.as_str()) {
        errs.push(format!("brief.kind: one of {} (got '{kind}')", TRAILER_KINDS.join(", ")));
    }
    let design = opt_str(raw.get("design")).unwrap_or_default();
    if !design.is_empty() && crate::designsys::get(&design).is_none() {
        let ids: Vec<&str> = crate::designsys::all().iter().map(|d| d.id).collect();
        errs.push(format!("brief.design: one of {} (got '{design}')", ids.join(", ")));
    }
    let reference_teaser = opt_str(raw.get("reference_teaser")).unwrap_or_default();
    if !reference_teaser.is_empty() && !crate::reference::valid_id(&reference_teaser) {
        errs.push(format!("brief.reference_teaser: a reference id under reference/ (got '{reference_teaser}')"));
    }
    let skills = list("skills", errs);
    for s in skills.iter().filter(|s| crate::skills::get(s).is_none()) {
        let ids: Vec<&str> = crate::skills::all().iter().map(|k| k.id).collect();
        errs.push(format!("brief.skills: '{s}' is not a skill ({})", ids.join(", ")));
    }
    BriefCfg {
        kind,
        genre: opt_str(raw.get("genre")).unwrap_or_default(),
        style: opt_str(raw.get("style")).unwrap_or_default(),
        audience: opt_str(raw.get("audience")).unwrap_or_default(),
        core: opt_str(raw.get("core")).unwrap_or_default(),
        must_show: list("must_show", errs),
        references: list("references", errs),
        images: list("images", errs),
        notes: opt_str(raw.get("notes")).unwrap_or_default(),
        design,
        skills,
        plan: opt_str(raw.get("plan")).unwrap_or_default(),
        direction: opt_str(raw.get("direction")).unwrap_or_default(),
        reference_teaser,
        raw,
    }
}

// --------------------------------------------------------------------------- resolution

pub fn resolve_all(defn: &Definition) -> Result<Vec<ResolvedShot>> {
    defn.shots.iter().enumerate().map(|(i, s)| resolve_shot(defn, s, i)).collect()
}

pub fn resolve_shot(defn: &Definition, shot: &Value, index: usize) -> Result<ResolvedShot> {
    let at = format!("shots[{index}]({})", str_of(&shot["id"]));
    let cam = obj(shot.get("camera"));
    let fps = shot.get("fps").and_then(num).map(|f| f as i64).unwrap_or(defn.teaser.fps);
    let Some(duration) = shot.get("duration").and_then(num) else {
        bail!("{at}.duration: positive seconds required");
    };
    let frames = g::round_half_even(duration * fps as f64).max(1);

    // where on the subject the camera looks (`aim`): its base point (the default, as always), the
    // centre of its box, or its upper part (a person's head and shoulders)
    let aim = cam.get("aim").map(str_of).filter(|a| !a.is_empty()).unwrap_or_else(|| "base".into());
    if !AIMS.contains(&aim.as_str()) {
        bail!("{at}.camera.aim: one of {} (got '{aim}')", AIMS.join(", "));
    }
    let aimed = |name: Option<&Value>, p: Option<Vec3>| -> Option<Vec3> {
        let p = p?;
        let Some(Value::String(n)) = name else { return Some(p) };
        match (aim.as_str(), defn.known_box(n)) {
            ("center", Some(b)) => Some(b.center),
            ("upper", Some(b)) => Some([b.center[0], b.center[1], b.center[2] + b.size[2] * 0.3]),
            _ => Some(p),
        }
    };
    let mut target = aimed(cam.get("look_at"), point(defn, cam.get("look_at"), &format!("{at}.camera.look_at"))?);
    let roll = cam.get("roll").and_then(num).unwrap_or(0.0);

    let eye: Vec3 = if cam.contains_key("anchor") {
        let Some(anchor) = point(defn, cam.get("anchor").filter(|v| !v.is_null()).or(Some(&Value::Bool(false))),
                                 &format!("{at}.camera.anchor"))?
        else {
            bail!("{at}.camera.anchor: anchor name or [x, y, z]");
        };
        let off = match cam.get("offset") {
            None => [-600.0, 0.0, 150.0],
            Some(v) => vec3(v).ok_or_else(|| Error::user(format!("{at}.camera.offset: must be [x, y, z]")))?,
        };
        if target.is_none() {
            target = aimed(cam.get("anchor"), Some(anchor));
        }
        g::add(anchor, off)
    } else if let Some(loc) = cam.get("location") {
        vec3(loc).ok_or_else(|| Error::user(format!("{at}.camera.location: must be [x, y, z]")))?
    } else {
        bail!("{at}.camera: needs either `location` or `anchor`");
    };

    let start_rot = if let Some(t) = target {
        g::look_at(eye, t, roll)
    } else if let Some(r) = cam.get("rotation") {
        Rot::from_array(vec3(r).ok_or_else(|| Error::user(format!("{at}.camera.rotation: must be [pitch, yaw, roll]")))?)
    } else {
        bail!("{at}.camera: needs `rotation`, `look_at` or `anchor`");
    };
    let start = Xform::new(eye, start_rot);

    let mut curve = "ease_in_out".to_string();
    let end = if let Some(end_v) = cam.get("end") {
        let end_cfg = obj(Some(end_v));
        let e_loc = match end_cfg.get("location") {
            None => eye,
            Some(v) => vec3(v).ok_or_else(|| Error::user(format!("{at}.camera.end.location: must be [x, y, z]")))?,
        };
        let e_target = aimed(end_cfg.get("look_at"), point(defn, end_cfg.get("look_at"), &format!("{at}.camera.end.look_at"))?)
            .or(target);
        let e_rot = match (e_target, end_cfg.get("rotation")) {
            (Some(t), None) => g::look_at(e_loc, t, end_cfg.get("roll").and_then(num).unwrap_or(roll)),
            (_, Some(r)) => Rot::from_array(
                vec3(r).ok_or_else(|| Error::user(format!("{at}.camera.end.rotation: must be [pitch, yaw, roll]")))?,
            ),
            (None, None) => start_rot,
        };
        if let Some(c) = end_cfg.get("curve") {
            curve = str_of(c);
        }
        Xform::new(e_loc, e_rot)
    } else {
        let mv = match cam.get("move") {
            Some(Value::Object(m)) => m.clone(),
            _ => obj(Some(&json!({"type": "static"}))),
        };
        if let Some(c) = mv.get("curve") {
            curve = str_of(c);
        }
        apply_move(&start, &mv, target, &format!("{at}.camera.move"))?
    };
    if !CURVES.contains(&curve.as_str()) {
        bail!("{at}: curve must be one of {CURVES:?}");
    }
    // Unwrap end angles relative to start so a plain numeric interpolation (Sequencer
    // keys) takes the short way round, exactly like the previz does.
    let s = start.rotation.as_array();
    let e = end.rotation.as_array();
    let end = Xform::new(end.location, Rot::new(
        s[0] + g::angle_delta(s[0], e[0]),
        s[1] + g::angle_delta(s[1], e[1]),
        s[2] + g::angle_delta(s[2], e[2]),
    ));

    let mut lens = Map::new();
    lens.insert("focal_length".into(), json!(35.0));
    lens.insert("aperture".into(), json!(2.8));
    lens.insert("sensor_width".into(), json!(36.0));
    for src in [obj(defn.defaults.get("lens")), obj(cam.get("lens"))] {
        for (k, v) in src {
            lens.insert(k, v);
        }
    }
    match lens.get("focal_length").and_then(num) {
        Some(fl) if (4.0..=1200.0).contains(&fl) => {}
        _ => bail!("{at}.camera.lens.focal_length: 4..1200 mm"),
    }
    if lens.get("focus_distance").map_or(true, Value::is_null) {
        if let Some(t) = target {
            lens.insert("focus_distance".into(), json!(g::round_to(g::length(g::sub(t, eye)), 1)));
        }
    }
    let lens: Map<String, Value> = lens
        .into_iter()
        .map(|(k, v)| match num(&v) {
            Some(f) => (k, json!(g::r3(f))),
            None => (k, v),
        })
        .collect();

    let environment = resolve_env(defn, shot, &at)?;

    let mut markers = Vec::new();
    if let Some(Value::Array(ms)) = shot.get("markers") {
        for m in ms {
            let at_s = m.get("at").and_then(num).unwrap_or(0.0);
            if !(0.0..=duration).contains(&at_s) {
                bail!("{at}.markers: at={} outside 0..{}", fmt_g(at_s), fmt_g(duration));
            }
            let label = m.get("label").map(str_of).unwrap_or_else(|| "marker".into());
            markers.push(json!({
                "frame": (frames - 1).min(g::round_half_even(at_s * fps as f64)),
                "label": label,
            }));
        }
    }

    let mut appear = Vec::new();
    match shot.get("appear") {
        None | Some(Value::Null) => {}
        Some(Value::Array(items)) => {
            for (k, it) in items.iter().enumerate() {
                appear.push(resolve_appear(defn, it, &format!("{at}.appear[{k}]"), duration, fps, frames, eye)?);
            }
        }
        Some(_) => bail!("{at}.appear: a list of {{prop, at, anchor, offset}}"),
    }

    Ok(ResolvedShot {
        appear,
        id: str_of(&shot["id"]),
        index,
        purpose: shot.get("purpose").map(str_of).unwrap_or_default(),
        duration,
        fps,
        frames,
        start: start.rounded(),
        end: end.rounded(),
        curve,
        lens,
        environment,
        markers,
        target,
        notes: shot.get("notes").map(str_of).unwrap_or_default(),
        beat: opt_str(shot.get("beat")),
    })
}

/// One `appear` item: {prop, at: seconds, anchor (or location), offset, yaw}. It faces the
/// camera unless `yaw` is given.
fn resolve_appear(defn: &Definition, it: &Value, at: &str, duration: f64, fps: i64, frames: i64, eye: Vec3)
    -> Result<Appear> {
    let name = it.get("prop").map(str_of).unwrap_or_default();
    let Some(prop) = defn.prop(&name) else {
        let known: Vec<&str> = defn.props.iter().map(|(n, _)| n.as_str()).collect();
        bail!("{at}.prop: unknown prop '{name}'. known: {}", if known.is_empty() { "(none; add `props:`)".to_string() } else { known.join(", ") });
    };
    let t = it.get("at").and_then(num).unwrap_or(0.0);
    if !(0.0..duration).contains(&t) {
        bail!("{at}.at: {}s is outside the shot (0..{}s)", fmt_g(t), fmt_g(duration));
    }
    let base = if let Some(l) = it.get("location") {
        vec3(l).ok_or_else(|| Error::user(format!("{at}.location: must be [x, y, z]")))?
    } else {
        let Some(a) = point(defn, it.get("anchor"), &format!("{at}.anchor"))? else {
            bail!("{at}: needs `anchor` (or `location`)");
        };
        a
    };
    let off = match it.get("offset") {
        None | Some(Value::Null) => [0.0; 3],
        Some(v) => vec3(v).ok_or_else(|| Error::user(format!("{at}.offset: must be [x, y, z]")))?,
    };
    let location = g::add(base, off).map(|v| g::round_to(v, 1));
    let yaw = match it.get("yaw").and_then(num) {
        Some(y) => y,
        None => g::round_to(g::look_at(location, [eye[0], eye[1], location[2]], 0.0).yaw, 1), // face the camera
    };
    Ok(Appear {
        prop: name,
        asset: prop.asset.clone(),
        kind: prop.kind.clone(),
        frame: (frames - 1).min(g::round_half_even(t * fps as f64)),
        location,
        yaw,
        size: prop.size,
    })
}

fn point(defn: &Definition, v: Option<&Value>, at: &str) -> Result<Option<Vec3>> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) => match defn.anchor(name) {
            Some(p) => Ok(Some(p)),
            None => {
                let known: Vec<&str> = defn.anchors.iter().map(|(n, _)| n.as_str()).collect();
                let known = if known.is_empty() {
                    "(none; run `shotpilot scan` or add `anchors:`)".to_string()
                } else {
                    known.join(", ")
                };
                bail!("{at}: unknown anchor '{name}'. known: {known}")
            }
        },
        Some(other) => vec3(other).map(Some).ok_or_else(|| Error::user(format!("{at}: anchor name or [x, y, z]"))),
    }
}

fn apply_move(start: &Xform, mv: &Map<String, Value>, target: Option<Vec3>, at: &str) -> Result<Xform> {
    let kind = mv.get("type").map(str_of).unwrap_or_else(|| "static".into());
    if !MOVES.contains(&kind.as_str()) {
        bail!("{at}.type: one of {MOVES:?}");
    }
    if kind == "static" {
        return Ok(*start);
    }
    let dist = mv.get("distance").and_then(num).unwrap_or(150.0);
    let mut eye = start.location;
    match kind.as_str() {
        "dolly_in" | "dolly_out" => {
            let d = g::forward(start.rotation);
            eye = g::add(eye, g::scale(d, if kind == "dolly_in" { dist } else { -dist }));
        }
        "truck_left" | "truck_right" => {
            let d = g::right(start.rotation);
            eye = g::add(eye, g::scale(d, if kind == "truck_right" { dist } else { -dist }));
        }
        "crane_up" | "crane_down" => {
            eye = g::add(eye, [0.0, 0.0, if kind == "crane_up" { dist } else { -dist }]);
        }
        _ => {
            let deg = mv.get("degrees").and_then(num).unwrap_or(30.0);
            let pivot = target.unwrap_or_else(|| g::add(eye, g::scale(g::forward(start.rotation), 500.0)));
            eye = g::orbit(eye, pivot, if kind == "orbit_left" { deg } else { -deg });
            return Ok(Xform::new(eye, g::look_at(eye, pivot, start.rotation.roll)));
        }
    }
    let keep_aim = mv.get("keep_aim").and_then(Value::as_bool).unwrap_or(target.is_some());
    if let (true, Some(t)) = (keep_aim, target) {
        if kind != "dolly_in" && kind != "dolly_out" {
            return Ok(Xform::new(eye, g::look_at(eye, t, start.rotation.roll)));
        }
    }
    Ok(Xform::new(eye, start.rotation))
}

fn resolve_env(defn: &Definition, shot: &Value, at: &str) -> Result<Map<String, Value>> {
    let mut env = Map::new();
    env.insert("time_of_day".into(), Value::Null);
    env.insert("sun_yaw".into(), json!(45.0));
    for src in [obj(defn.defaults.get("environment")), obj(shot.get("environment"))] {
        for (k, v) in src {
            env.insert(k, v);
        }
    }
    let tod = env.get("time_of_day").cloned().unwrap_or(Value::Null);
    if !tod.is_null() {
        let elev = tod.as_str().and_then(tod_elevation);
        let Some(elev) = elev else {
            bail!("{at}.environment.time_of_day: one of {:?}", tod_names_sorted());
        };
        if env.get("sun_elevation").map_or(true, Value::is_null) {
            env.insert("sun_elevation".into(), json!(elev));
        }
    }
    let mut out: Vec<(String, Value)> = env.into_iter().filter(|(_, v)| !v.is_null()).collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out.into_iter().collect())
}
