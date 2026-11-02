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
}

#[derive(Debug, Clone)]
pub struct TeaserCfg {
    pub title: String,
    pub pitch: String,
    pub target_duration: f64,
    pub fps: i64,
    pub resolution: (i64, i64),
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub path: PathBuf,
    pub project: ProjectCfg,
    pub teaser: TeaserCfg,
    pub anchors: Vec<(String, Vec3)>,
    pub defaults: Map<String, Value>,
    pub qualities: Map<String, Value>,
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
        json!({
            "frames": self.frames,
            "fps": self.fps,
            "curve": self.curve,
            "start": self.start.to_json(),
            "end": self.end.to_json(),
            "lens": self.lens,
            "environment": self.environment,
            "markers": self.markers,
        })
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

    let mut anchors = Vec::new();
    for (name, v) in obj(top.get("anchors")) {
        match vec3(&v) {
            Some(vec) => anchors.push((name, vec)),
            None => errs.push(format!("anchors.{name}: must be [x, y, z]")),
        }
    }

    let defaults = obj(top.get("defaults"));
    let qualities = obj(top.get("qualities"));
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
        match s.get("beat") {
            None | Some(Value::Null) => {}
            Some(Value::String(b)) if BEATS.contains(&b.as_str()) => {}
            Some(b) => errs.push(format!("{at}.beat: one of {} (got {b})", BEATS.join(", "))),
        }
    }

    let defn = Definition { path: path.to_path_buf(), project, teaser, anchors, defaults, qualities, shots };
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

    let mut target = point(defn, cam.get("look_at"), &format!("{at}.camera.look_at"))?;
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
            target = Some(anchor);
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
        let e_target = point(defn, end_cfg.get("look_at"), &format!("{at}.camera.end.look_at"))?.or(target);
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

    Ok(ResolvedShot {
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

/// Advisory checks against the hook -> establish -> escalate -> title shape.
pub fn structure_warnings(shots: &[ResolvedShot], target_duration: f64) -> Vec<String> {
    if shots.is_empty() {
        return Vec::new();
    }
    let beats: Vec<Option<&str>> = shots.iter().map(|s| s.beat.as_deref()).collect();
    if beats.iter().all(Option::is_none) {
        return vec![
            "no shot has a `beat`; tag shots with hook / establish / escalate / title to check the teaser shape".into(),
        ];
    }
    let mut out = Vec::new();
    let untagged: Vec<&str> = shots.iter().filter(|s| s.beat.is_none()).map(|s| s.id.as_str()).collect();
    if !untagged.is_empty() {
        out.push(format!("no beat on: {}", untagged.join(", ")));
    }
    for b in BEATS {
        if !beats.contains(&Some(*b)) {
            out.push(format!("no `{b}` shot ({})", beat_guide(b)));
        }
    }
    if let Some(Some(first)) = beats.first() {
        if *first != "hook" {
            out.push(format!("the cut opens on `{first}`; teasers usually open on the hook"));
        }
    }
    if let Some(Some(last)) = beats.last() {
        if *last != "title" {
            out.push(format!("the cut ends on `{last}`; end on the title beat"));
        }
    }
    let ranks: Vec<usize> = beats.iter().flatten().filter_map(|b| BEATS.iter().position(|x| x == b)).collect();
    if ranks.windows(2).any(|w| w[1] < w[0]) {
        out.push("beats go backwards (e.g. establish after escalate); intentional?".into());
    }
    let total: f64 = shots.iter().map(|s| s.duration).sum();
    let hook: f64 = shots.iter().filter(|s| s.beat.as_deref() == Some("hook")).map(|s| s.duration).sum();
    if hook > 6.0 {
        out.push(format!("hook runs {}s; keep it under ~5s", fmt_g(hook)));
    }
    if target_duration != 0.0 && (total - target_duration).abs() > 0.1 * target_duration {
        out.push(format!("runtime {}s is more than 10% off the {}s target", fmt_g(total), fmt_g(target_duration)));
    }
    out
}
