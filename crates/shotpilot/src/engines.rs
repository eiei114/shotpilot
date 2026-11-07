//! Engine backends.
//!
//! Both backends speak the same job/result JSON protocol. `ue5` hands the job to
//! ue/sp_entry.py inside Unreal (the only Python left: it must run in Unreal's own
//! interpreter); `mock` executes it in-process so the whole loop
//! (definition -> build -> render -> pull -> cut) runs and is testable without UE.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use crate::bail;
use crate::error::{Error, Result};
use crate::geometry::{Rot, Xform};
use crate::model::{resolve_all, vec3, Definition, ResolvedShot};
use crate::previz;
use crate::sync::same_payload;

/// The script Unreal runs. Embedded so the binary is self-contained; written next to
/// the job files before each launch.
pub const ENTRY_SCRIPT: &str = include_str!("../ue/sp_entry.py");

pub fn shot_asset(defn: &Definition, shot_id: &str) -> String {
    format!("{}/Shots/SEQ_{shot_id}", defn.project.content_root)
}

pub fn master_asset(defn: &Definition) -> String {
    format!("{}/SEQ_Teaser", defn.project.content_root)
}

/// The part of a shot payload that can be read back from a sequence asset.
pub fn readback_subset(payload: &Value) -> Value {
    let mut lens = Map::new();
    if let Some(l) = payload.get("lens").and_then(Value::as_object) {
        for k in ["focal_length", "aperture", "focus_distance"] {
            if let Some(v) = l.get(k) {
                lens.insert(k.into(), v.clone());
            }
        }
    }
    json!({
        "frames": payload["frames"], "fps": payload["fps"],
        "start": payload["start"], "end": payload["end"],
        "lens": lens,
    })
}

pub trait Engine {
    fn name(&self) -> &'static str;
    fn build(&self, items: &[Value], master: Option<&Value>, force: bool) -> Result<Value>;
    fn render(&self, build_items: &[Value], master: Option<&Value>, render_items: &[Value], force: bool)
        -> Result<Value>;
    fn pull(&self, items: &[Value]) -> Result<Value>;
    fn scan(&self, tag: &str, prefix: &str) -> Result<Value>;
    /// Stage 0 in one launch: {"anchors": ..., "candidates"?: [...], "assets"?: [...]}. The anchors
    /// are the tagged ones as in [`Engine::scan`]; with none tagged, `candidates` lists landmarks
    /// in the level to pick from (scout::Candidate); with `candidates`, always (to find other
    /// places for concept frames, D-46). The mesh inventory comes along when `assets`.
    fn survey(&self, tag: &str, prefix: &str, assets: bool, candidates: bool) -> Result<Value>;
    /// Meshes the project has: {"assets": [{asset, kind: static|skeletal, size: [w, d, h] | null}]}.
    fn assets(&self) -> Result<Value>;
}

pub fn make_engine(defn: &Definition) -> Result<Box<dyn Engine>> {
    if defn.project.engine == "ue5" {
        Ok(Box::new(Ue5Engine::new(defn)?))
    } else if defn.project.engine == "three" {
        Ok(Box::new(crate::three::ThreeEngine::new(defn)?))
    } else {
        Ok(Box::new(MockEngine::new(defn)?))
    }
}

fn jobs_dir(defn: &Definition) -> Result<PathBuf> {
    let d = defn.root().join(".shotpilot").join("jobs");
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

fn project_dict(defn: &Definition) -> Value {
    let p = &defn.project;
    json!({"name": p.name, "map": p.map, "content_root": p.content_root, "sun_actor": p.sun_actor,
           "game_mode": p.game_mode})
}

fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

// --------------------------------------------------------------------------- sequence store

/// The shot sequences an engine without an editor keeps (mock, three): each pushed payload as a
/// JSON file, read back by `pull`, and a log of every launch (what the tests count).
pub struct SeqStore {
    pub dir: PathBuf,
}

impl SeqStore {
    pub fn new(dir: PathBuf) -> Result<SeqStore> {
        std::fs::create_dir_all(&dir)?;
        Ok(SeqStore { dir })
    }

    pub fn asset_file(&self, asset: &str) -> PathBuf {
        self.dir.join(format!("{}.json", asset.trim_matches('/').replace('/', "__")))
    }

    pub fn launch(&self, kind: &str, detail: Value) -> Result<()> {
        let mut rec = Map::new();
        rec.insert("kind".into(), json!(kind));
        if let Value::Object(d) = detail {
            rec.extend(d);
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(self.dir.join("launches.jsonl"))?;
        writeln!(f, "{}", Value::Object(rec))?;
        Ok(())
    }

    pub fn build(&self, items: &[Value], master: Option<&Value>, force: bool) -> Result<Value> {
        let mut shots = Map::new();
        for it in items {
            let id = it["id"].as_str().unwrap_or_default().to_string();
            let p = self.asset_file(it["asset"].as_str().unwrap_or_default());
            if p.exists() && !it["base"].is_null() && !force {
                let current = read_json(&p)?;
                if !same_payload(Some(&readback_subset(&current)), Some(&it["base"])) {
                    shots.insert(id, json!({"status": "diverged", "engine": readback_subset(&current)}));
                    continue;
                }
            }
            std::fs::write(&p, serde_json::to_string_pretty(&it["payload"])?)?;
            shots.insert(id, json!({"status": "built", "engine": readback_subset(&it["payload"])}));
        }
        let mut out = json!({"shots": shots});
        if let Some(m) = master {
            std::fs::write(self.asset_file(m["asset"].as_str().unwrap_or_default()), serde_json::to_string_pretty(m)?)?;
            let frames: i64 = m["shots"].as_array().map(|a| a.iter().filter_map(|s| s["frames"].as_i64()).sum()).unwrap_or(0);
            out["master"] = json!({"asset": m["asset"], "frames": frames});
        }
        Ok(out)
    }

    pub fn pull(&self, items: &[Value]) -> Result<Value> {
        self.launch("pull", json!({"shots": items.iter().map(|i| &i["id"]).collect::<Vec<_>>()}))?;
        let mut shots = Map::new();
        for it in items {
            let p = self.asset_file(it["asset"].as_str().unwrap_or_default());
            let v = if p.exists() { readback_subset(&read_json(&p)?) } else { Value::Null };
            shots.insert(it["id"].as_str().unwrap_or_default().to_string(), v);
        }
        Ok(json!({"shots": shots}))
    }

    /// The shots whose build did not land (diverged or failed): their renders are refused.
    pub fn blocked(build: &Value) -> Vec<String> {
        build["shots"]
            .as_object()
            .map(|m| m.iter().filter(|(_, v)| v["status"] != "built").map(|(k, _)| k.clone()).collect())
            .unwrap_or_default()
    }
}

// --------------------------------------------------------------------------- mock

/// Stands in for UE: "assets" are JSON files, "renders" are tinted previz frames.
pub struct MockEngine {
    defn: Definition,
    store: SeqStore,
}

impl MockEngine {
    pub fn new(defn: &Definition) -> Result<MockEngine> {
        let store = SeqStore::new(defn.root().join(".shotpilot").join("mock_engine"))?;
        jobs_dir(defn)?;
        Ok(MockEngine { defn: defn.clone(), store })
    }

    fn asset_file(&self, asset: &str) -> PathBuf {
        self.store.asset_file(asset)
    }

    fn launch(&self, kind: &str, detail: Value) -> Result<()> {
        self.store.launch(kind, detail)
    }

    /// The mock game's content behind a shot, as UE reports it from the PIE world: the files
    /// `content/<name>.*` next to shots.yaml for each anchor in view at the first, middle or
    /// last frame, and for each prop that pops in. None without a `content/` folder (a game
    /// with no content to track, like the tests that predate it).
    fn visible_content(&self, shot: &ResolvedShot, w: u32, h: u32) -> Option<Vec<Value>> {
        let dir = self.defn.root().join("content");
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
        files.sort();
        let mut seen = std::collections::BTreeSet::new();
        let last = (shot.frames - 1).max(0);
        let (fl, sensor) = (shot.focal_length(), shot.sensor_width());
        // in view when the box's projected corners overlap the frame; a box the camera is
        // partly inside of counts too (when unsure, render again rather than miss a change)
        let in_view = |b: &crate::model::AnchorBox, cam: &Xform| {
            let [cx, cy, cz] = b.center;
            let [sx, sy, sz] = b.size.map(|v| v / 2.0);
            let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            let mut behind = 0;
            for dx in [-sx, sx] {
                for dy in [-sy, sy] {
                    for dz in [-sz, sz] {
                        match crate::geometry::project([cx + dx, cy + dy, cz + dz], cam, fl, w, h, sensor) {
                            Some((x, y, _)) => {
                                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                            }
                            None => behind += 1,
                        }
                    }
                }
            }
            match behind {
                8 => false,
                0 => x1 >= 0.0 && x0 <= w as f64 && y1 >= 0.0 && y0 <= h as f64,
                _ => true,
            }
        };
        for f in [0, last / 2, last] {
            let cam = shot.camera_at(f);
            for (name, b) in self.defn.anchor_boxes() {
                if in_view(&b, &cam) {
                    seen.insert(name);
                }
            }
        }
        seen.extend(shot.appear.iter().map(|a| a.prop.clone()));
        Some(
            files
                .iter()
                .filter_map(|p| {
                    let stem = p.file_stem()?.to_string_lossy().to_string();
                    seen.contains(&stem).then(|| json!({"package": format!("/Game/Mock/{stem}"), "file": p.display().to_string()}))
                })
                .collect(),
        )
    }

    /// Simulate a human editing the shot sequence in Sequencer (tests / demo).
    pub fn edit_asset(&self, shot_id: &str, mutate: impl FnOnce(&mut Value)) -> Result<()> {
        let p = self.asset_file(&shot_asset(&self.defn, shot_id));
        let mut data = read_json(&p)?;
        mutate(&mut data);
        std::fs::write(&p, serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }

    fn do_build(&self, items: &[Value], master: Option<&Value>, force: bool) -> Result<Value> {
        self.store.build(items, master, force)
    }
}

impl Engine for MockEngine {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn build(&self, items: &[Value], master: Option<&Value>, force: bool) -> Result<Value> {
        self.launch("build", json!({"shots": items.iter().map(|i| &i["id"]).collect::<Vec<_>>()}))?;
        self.do_build(items, master, force)
    }

    fn render(&self, build_items: &[Value], master: Option<&Value>, render_items: &[Value], force: bool)
        -> Result<Value> {
        self.launch("render", json!({
            "shots": render_items.iter().map(|i| &i["key"]).collect::<Vec<_>>(),
            "built": build_items.iter().map(|i| &i["id"]).collect::<Vec<_>>(),
        }))?;
        let build = if !build_items.is_empty() || master.is_some() {
            self.do_build(build_items, master, force)?
        } else {
            Value::Null
        };
        let blocked = SeqStore::blocked(&build);
        let resolved = resolve_all(&self.defn)?;
        let t0 = Instant::now();
        let mut renders = Map::new();
        for it in render_items {
            let key = it["key"].as_str().unwrap_or_default().to_string();
            let id = it["id"].as_str().unwrap_or_default();
            if blocked.iter().any(|b| b == id) {
                renders.insert(key, json!({"status": "error", "error": "build failed or diverged"}));
                continue;
            }
            let p = self.asset_file(it["asset"].as_str().unwrap_or_default());
            if !p.exists() {
                renders.insert(key, json!({"status": "error", "error": "sequence asset missing; build first"}));
                continue;
            }
            let Some(base) = resolved.iter().find(|s| s.id == id) else {
                renders.insert(key, json!({"status": "error", "error": "unknown shot"}));
                continue;
            };
            let shot = shot_from_payload(base, &read_json(&p)?);
            let w = it["resolution"][0].as_f64().unwrap_or(960.0);
            let h = it["resolution"][1].as_f64().unwrap_or(540.0);
            // cap mock resolution: it's a stand-in, not a renderer
            let k = (480.0 / w).min(1.0);
            let frames: Vec<i64> = it["frames"].as_array().map(|a| a.iter().filter_map(Value::as_i64).collect()).unwrap_or_default();
            previz::render_shot(&shot, &frames, &self.defn.anchor_boxes(), Path::new(it["out_dir"].as_str().unwrap_or(".")),
                                ((w * k) as u32).max(2), ((h * k) as u32).max(2), 0.0, Some([255, 170, 60]))?;
            let mut r = json!({"status": "ok"});
            if let Some(deps) = self.visible_content(&shot, w as u32, h as u32) {
                r["deps"] = json!(deps);
            }
            renders.insert(key, r);
        }
        Ok(json!({"build": build, "renders": renders, "seconds": (t0.elapsed().as_secs_f64() * 100.0).round() / 100.0}))
    }

    fn pull(&self, items: &[Value]) -> Result<Value> {
        self.store.pull(items)
    }

    fn assets(&self) -> Result<Value> {
        self.launch("assets", json!({}))?;
        self.inventory()
    }

    fn scan(&self, _tag: &str, _prefix: &str) -> Result<Value> {
        self.launch("scan", json!({}))?;
        self.level()
    }

    fn survey(&self, _tag: &str, _prefix: &str, assets: bool, candidates: bool) -> Result<Value> {
        self.launch("survey", json!({"assets": assets, "candidates": candidates}))?;
        let mut out = self.level()?;
        // the mock level: `level.json` holds what is tagged, `landmarks.json` ({name: entry})
        // what a level without tags offers, scored by size as the real engine does
        let tagged = out["anchors"].as_object().is_some_and(|m| !m.is_empty());
        let marks = self.store.dir.join("landmarks.json");
        // a new project with no level of its own films the sample courtyard, so trying
        // Shotpilot without UE still gets places to scout
        let courtyard = !marks.exists() && !self.store.dir.join("level.json").exists() && self.defn.anchors.is_empty();
        if (!tagged || candidates) && (marks.exists() || courtyard) {
            let level = if courtyard { mock_courtyard() } else { read_json(&marks)? };
            let cands: Vec<Value> = level
                .as_object()
                .into_iter()
                .flatten()
                .map(|(name, e)| {
                    let size = e["size"].as_array().map(|a| a.iter().filter_map(Value::as_f64).product::<f64>()).unwrap_or(0.0);
                    let mut c = e.clone();
                    c["name"] = json!(name);
                    c["label"] = json!(format!("SM_{name}"));
                    c["cls"] = json!("StaticMeshActor");
                    c["score"] = json!(size.cbrt().min(400.0));
                    c
                })
                .collect();
            out["candidates"] = json!(cands);
        }
        if assets {
            out["assets"] = self.inventory()?["assets"].take();
        }
        Ok(out)
    }
}

/// The mock's own level for a project that brings none: the sample's courtyard (templates::DEMO).
fn mock_courtyard() -> Value {
    json!({
        "gate": {"at": [0, -1500, 100], "size": [300, 40, 260], "center": [0, -1500, 130]},
        "well": {"at": [0, 0, 80], "size": [140, 140, 90], "center": [0, 0, 45]},
        "window": {"at": [800, 600, 450], "size": [20, 140, 160], "center": [800, 600, 450]},
        "door": {"at": [-700, 500, 120], "size": [20, 110, 220], "center": [-700, 500, 110]},
        "lantern": {"at": [300, -300, 180], "size": [30, 30, 45], "center": [300, -300, 180]},
    })
}

impl MockEngine {
    fn inventory(&self) -> Result<Value> {
        let f = self.store.dir.join("assets.json");
        if f.exists() {
            return Ok(json!({"assets": read_json(&f)?}));
        }
        let mut list: Vec<Value> = self
            .defn
            .props
            .iter()
            .map(|(_, p)| json!({"asset": p.asset, "kind": p.kind, "size": p.size}))
            .collect();
        list.push(json!({"asset": "/Game/Mock/Props/SM_Crate", "kind": "static", "size": [100, 100, 80]}));
        list.push(json!({"asset": "/Game/Mock/Characters/SK_Figure", "kind": "skeletal", "size": [60, 40, 180]}));
        Ok(json!({"assets": list}))
    }

    fn level(&self) -> Result<Value> {
        let level = self.store.dir.join("level.json");
        if level.exists() {
            return Ok(json!({"anchors": read_json(&level)?}));
        }
        let anchors: Map<String, Value> = self
            .defn
            .anchors
            .iter()
            .map(|(k, v)| {
                let entry = match self.defn.known_box(k) {
                    Some(b) => json!({"at": v, "size": b.size, "center": b.center}),
                    None => json!(v),
                };
                (k.clone(), entry)
            })
            .collect();
        Ok(json!({"anchors": anchors}))
    }
}

/// Rebuild a ResolvedShot from what is actually stored in the engine asset.
pub(crate) fn shot_from_payload(base: &ResolvedShot, payload: &Value) -> ResolvedShot {
    let xf = |d: &Value| -> Xform {
        Xform::new(vec3(&d["location"]).unwrap_or([0.0; 3]), Rot::from_array(vec3(&d["rotation"]).unwrap_or([0.0; 3])))
    };
    let frames = payload["frames"].as_i64().unwrap_or(base.frames);
    let fps = payload["fps"].as_i64().unwrap_or(base.fps);
    let mut lens = base.lens.clone();
    if let Some(l) = payload["lens"].as_object() {
        lens.extend(l.clone());
    }
    ResolvedShot {
        id: base.id.clone(),
        index: base.index,
        purpose: base.purpose.clone(),
        duration: frames as f64 / fps as f64,
        fps,
        frames,
        start: xf(&payload["start"]),
        end: xf(&payload["end"]),
        curve: payload["curve"].as_str().map(str::to_string).unwrap_or_else(|| base.curve.clone()),
        lens,
        environment: payload["environment"].as_object().cloned().unwrap_or_else(|| base.environment.clone()),
        markers: payload["markers"].as_array().cloned().unwrap_or_default(),
        target: base.target,
        notes: String::new(),
        beat: None,
        appear: payload["appear"].as_array().map(|a| a.iter().filter_map(crate::model::Appear::from_json).collect()).unwrap_or_default(),
    }
}

// --------------------------------------------------------------------------- UE5

pub struct Ue5Engine {
    defn: Definition,
    uproject: PathBuf,
    cmd: PathBuf,
    /// Plugins the script needs that the .uproject does not enable (turned on per launch).
    enable: Vec<&'static str>,
    logs: PathBuf,
    timeout: Duration,
}

impl Ue5Engine {
    pub fn new(defn: &Definition) -> Result<Ue5Engine> {
        let mut uproject = PathBuf::from(defn.project.uproject.clone().unwrap_or_default());
        if !uproject.is_absolute() {
            uproject = defn.root().join(uproject);
        }
        if !uproject.exists() {
            bail!("project.uproject not found: {}", uproject.display());
        }
        let cmd = find_ue_cmd(defn, &uproject)?;
        let enable = plugins_to_enable(&std::fs::read_to_string(&uproject).unwrap_or_default());
        let logs = defn.root().join(".shotpilot").join("logs");
        std::fs::create_dir_all(&logs)?;
        jobs_dir(defn)?;
        let timeout = std::env::var("SHOTPILOT_UE_TIMEOUT").ok().and_then(|s| s.parse().ok()).unwrap_or(3600);
        Ok(Ue5Engine { defn: defn.clone(), uproject, cmd, enable, logs, timeout: Duration::from_secs(timeout) })
    }

    fn common(&self) -> Vec<String> {
        let mut args: Vec<String> = ["-unattended", "-nosplash", "-nop4", "-stdout", "-FullStdOutLogOutput", "-NoSound"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if !self.enable.is_empty() {
            args.push(format!("-EnablePlugins={}", self.enable.join(",")));
        }
        args.extend(self.defn.project.ue_args.iter().cloned());
        if self.defn.project.isolate_userdir {
            args.push(format!("-UserDir={}", self.defn.root().join(".shotpilot").join("ue_userdir").display()));
        }
        args
    }

    fn run(&self, kind: &str, job: Value) -> Result<Value> {
        let dir = jobs_dir(&self.defn)?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f").to_string();
        let job_file = dir.join(format!("{stamp}-{kind}.json"));
        let mut body = Map::new();
        body.insert("kind".into(), json!(kind));
        body.insert("project".into(), project_dict(&self.defn));
        if let Value::Object(j) = job {
            body.extend(j);
        }
        std::fs::write(&job_file, serde_json::to_string_pretty(&Value::Object(body))?)?;
        let result_file = dir.join(format!("{stamp}-{kind}.result.json"));

        let script = self.defn.root().join(".shotpilot").join("sp_entry.py");
        if std::fs::read_to_string(&script).ok().as_deref() != Some(ENTRY_SCRIPT) {
            std::fs::write(&script, ENTRY_SCRIPT)?;
        }
        let mut argv: Vec<String> = vec![self.uproject.display().to_string()];
        if kind == "render" {
            argv.push(self.defn.project.map.clone().unwrap_or_default());
            argv.push(format!("-ExecutePythonScript={}", script.display()));
            argv.extend(["-windowed", "-ResX=1280", "-ResY=720", "-NoLoadingScreen"].map(String::from));
        } else {
            argv.extend(["-run=pythonscript".to_string(), format!("-script={}", script.display()), "-NullRHI".into()]);
        }
        argv.extend(self.common());

        let log_file = self.logs.join(format!("{stamp}-{kind}.log"));
        let mut log = std::fs::File::create(&log_file)?;
        let shown: Vec<String> = std::iter::once(self.cmd.display().to_string())
            .chain(argv.iter().cloned())
            .map(|a| if a.contains(' ') { format!("\"{a}\"") } else { a })
            .collect();
        writeln!(log, "{}\n", shown.join(" "))?;
        let err_log = log.try_clone()?;
        let t0 = Instant::now();
        let mut child = crate::cut::no_window(&mut Command::new(&self.cmd))
            .args(&argv)
            .env("SHOTPILOT_JOB", &job_file)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(err_log))
            .spawn()
            .map_err(|e| Error::user(format!("could not start {}: {e}", self.cmd.display())))?;
        let status = loop {
            if let Some(s) = child.try_wait()? {
                break s;
            }
            if t0.elapsed() > self.timeout {
                let _ = child.kill();
                bail!("UE {kind} timed out after {}s. log: {}", self.timeout.as_secs(), log_file.display());
            }
            std::thread::sleep(Duration::from_millis(250));
        };
        if !result_file.exists() {
            bail!("UE {kind} exited ({}) without a result after {}s. log: {}",
                  status.code().map(|c| c.to_string()).unwrap_or("?".into()), t0.elapsed().as_secs(), log_file.display());
        }
        let mut res = read_json(&result_file)?;
        if let Some(f) = res.get("fatal").filter(|v| !v.is_null() && v.as_str() != Some("")) {
            bail!("UE {kind} failed:\n{}\nlog: {}", f.as_str().map(str::to_string).unwrap_or(f.to_string()), log_file.display());
        }
        res["log"] = json!(log_file.display().to_string());
        Ok(res)
    }
}

impl Engine for Ue5Engine {
    fn name(&self) -> &'static str {
        "ue5"
    }

    fn build(&self, items: &[Value], master: Option<&Value>, force: bool) -> Result<Value> {
        self.run("build", json!({"build": items, "master": master, "force": force}))
    }

    fn render(&self, build_items: &[Value], master: Option<&Value>, render_items: &[Value], force: bool)
        -> Result<Value> {
        self.run("render", json!({"build": build_items, "master": master, "render": render_items, "force": force}))
    }

    fn pull(&self, items: &[Value]) -> Result<Value> {
        self.run("pull", json!({"pull": items}))
    }

    fn assets(&self) -> Result<Value> {
        self.run("assets", json!({}))
    }

    fn scan(&self, tag: &str, prefix: &str) -> Result<Value> {
        self.run("scan", json!({"tag": tag, "prefix": prefix}))
    }

    fn survey(&self, tag: &str, prefix: &str, assets: bool, candidates: bool) -> Result<Value> {
        self.run("survey", json!({"tag": tag, "prefix": prefix, "assets": assets, "candidates": candidates}))
    }
}

/// Plugins sp_entry.py needs in the editor: Python, the editor scripting helpers, Sequencer's
/// scripting and Movie Render Queue.
pub const NEEDED_PLUGINS: &[&str] =
    &["PythonScriptPlugin", "EditorScriptingUtilities", "SequencerScripting", "LevelSequenceEditor", "MovieRenderPipeline"];

/// The needed plugins a .uproject (its text) does not enable. shotpilot turns them on for its
/// own launch with `-EnablePlugins`, so the game's project file is never rewritten. A file
/// that does not read as JSON asks for all of them (enabling an enabled plugin changes nothing).
pub fn plugins_to_enable(uproject: &str) -> Vec<&'static str> {
    let on: Vec<String> = serde_json::from_str::<Value>(uproject)
        .ok()
        .and_then(|v| v["Plugins"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter(|p| p["Enabled"].as_bool() == Some(true))
        .filter_map(|p| p["Name"].as_str().map(str::to_ascii_lowercase))
        .collect();
    NEEDED_PLUGINS.iter().copied().filter(|n| !on.contains(&n.to_ascii_lowercase())).collect()
}

fn find_ue_cmd(defn: &Definition, uproject: &Path) -> Result<PathBuf> {
    if let Some(c) = defn.project.ue_cmd.clone().or_else(|| std::env::var("SHOTPILOT_UE_CMD").ok()) {
        let p = PathBuf::from(&c);
        if !p.exists() {
            bail!("UnrealEditor-Cmd not found at {c}");
        }
        return Ok(p);
    }
    let assoc = std::fs::read_to_string(uproject)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["EngineAssociation"].as_str().map(str::to_string))
        .unwrap_or_default();
    if !assoc.is_empty() {
        for base in [r"C:\Program Files\Epic Games", r"D:\Epic Games"] {
            let p = Path::new(base).join(format!("UE_{assoc}")).join(r"Engine\Binaries\Win64\UnrealEditor-Cmd.exe");
            if p.exists() {
                return Ok(p);
            }
        }
    }
    if let Some(p) = find_on_path("UnrealEditor-Cmd") {
        return Ok(p);
    }
    bail!("could not locate UnrealEditor-Cmd for EngineAssociation '{assoc}'. \
           set project.ue_cmd in shots.yaml or SHOTPILOT_UE_CMD")
}

/// How long a PATH lookup is remembered. A long PATH makes one lookup cost thousands of
/// stat() calls, and the app asks for `claude`, `codex` and `ffmpeg` on every status.
const PATH_TTL: Duration = Duration::from_secs(10);

/// `which`: look an executable up on PATH (with PATHEXT on Windows). Answers are kept
/// for a few seconds per (name, PATH), so installing a tool shows up shortly after.
pub fn find_on_path(name: &str) -> Option<PathBuf> {
    type Cache = std::collections::HashMap<(String, std::ffi::OsString), (Instant, Option<PathBuf>)>;
    static CACHE: std::sync::OnceLock<std::sync::Mutex<Cache>> = std::sync::OnceLock::new();
    let key = (name.to_string(), std::env::var_os("PATH")?);
    let cache = CACHE.get_or_init(Default::default);
    if let Some((at, found)) = cache.lock().unwrap_or_else(|p| p.into_inner()).get(&key) {
        if at.elapsed() < PATH_TTL && found.as_ref().map_or(true, |p| p.is_file()) {
            return found.clone();
        }
    }
    let found = lookup_on_path(name);
    cache.lock().unwrap_or_else(|p| p.into_inner()).insert(key, (Instant::now(), found.clone()));
    found
}

fn lookup_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT".into())
            .split(';')
            .map(|e| e.to_ascii_lowercase())
            .chain(std::iter::once(String::new()))
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(&path) {
        for ext in &exts {
            let p = dir.join(format!("{name}{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_without_movie_render_queue_gets_it_for_the_launch_only() {
        // RoomSeven's own .uproject: Python and the scripting helpers, but no Sequencer
        // scripting and no Movie Render Queue
        let room_seven = r#"{"FileVersion": 3, "EngineAssociation": "5.8", "Plugins": [
            {"Name": "ModelingToolsEditorMode", "Enabled": true},
            {"Name": "PythonScriptPlugin", "Enabled": true, "TargetAllowList": ["Editor"]},
            {"Name": "EditorScriptingUtilities", "Enabled": true}]}"#;
        assert_eq!(plugins_to_enable(room_seven), ["SequencerScripting", "LevelSequenceEditor", "MovieRenderPipeline"]);
        // the samples enable all of them. A plugin the project switches off is still asked for,
        // but UE keeps the project's word (PluginManager skips configured names): then the
        // script's own message says how to turn it on

        let all = NEEDED_PLUGINS.iter().map(|n| format!(r#"{{"Name": "{n}", "Enabled": true}}"#)).collect::<Vec<_>>().join(",");
        assert!(plugins_to_enable(&format!(r#"{{"Plugins": [{all}]}}"#)).is_empty());
        let off = format!(r#"{{"Plugins": [{}]}}"#, all.replace(r#""MovieRenderPipeline", "Enabled": true"#, r#""MovieRenderPipeline", "Enabled": false"#));
        assert_eq!(plugins_to_enable(&off), ["MovieRenderPipeline"]);
        // no plugin list, or a file that does not read: ask for every one
        assert_eq!(plugins_to_enable(r#"{"FileVersion": 3}"#), NEEDED_PLUGINS);
        assert_eq!(plugins_to_enable("not json"), NEEDED_PLUGINS);
    }
}
