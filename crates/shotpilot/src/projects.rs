//! The home screen: recent projects, a card per project, and making a new one.
//!
//! The app is no longer tied to one shots.yaml. The home lists the projects opened
//! before (kept in `<config>/shotpilot/projects.json`, next to the settings) and makes
//! new ones from a prompt. Every project stays a plain folder with its own shots.yaml;
//! the list only remembers where they are.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::bail;
use crate::error::Result;
use crate::fingerprint as fp;
use crate::templates;
use crate::workspace::Workspace;

/// How many projects the list remembers.
const MAX_RECENT: usize = 30;
/// Shots shown on a project card's cover.
const COVER_SHOTS: usize = 4;

/// `<config>/shotpilot/projects.json`.
pub fn default_path() -> Option<PathBuf> {
    crate::connect::Settings::default_path().map(|p| p.with_file_name("projects.json"))
}

/// Where new projects go unless the user picks another folder.
pub fn default_location() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join("Documents").join("Shotpilot"))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recent {
    /// Absolute path of the shots.yaml.
    pub path: PathBuf,
    /// When it was last opened (RFC 3339).
    pub opened: String,
}

/// The recent-projects list. `None` keeps nothing (a session without a config folder).
pub struct Recents {
    file: Option<PathBuf>,
}

impl Recents {
    pub fn new(file: Option<PathBuf>) -> Recents {
        Recents { file }
    }

    /// Most recent first. A missing or broken file is an empty list.
    pub fn list(&self) -> Vec<Recent> {
        let Some(file) = &self.file else { return Vec::new() };
        let Ok(text) = std::fs::read_to_string(file) else { return self.migrated() };
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        v["projects"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|p| {
                        let path = PathBuf::from(p["path"].as_str()?);
                        Some(Recent { path, opened: p["opened"].as_str().unwrap_or("").to_string() })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The app used to remember one project in `last_project.txt`; carry it over.
    fn migrated(&self) -> Vec<Recent> {
        let Some(old) = self.file.as_ref().map(|f| f.with_file_name("last_project.txt")) else { return Vec::new() };
        match std::fs::read_to_string(old) {
            Ok(s) if !s.trim().is_empty() => vec![Recent { path: PathBuf::from(s.trim()), opened: String::new() }],
            _ => Vec::new(),
        }
    }

    /// Put a project at the top of the list.
    pub fn touch(&self, shots: &Path) -> Result<()> {
        let abs = std::path::absolute(shots)?;
        let mut list = self.list();
        list.retain(|r| !same_path(&r.path, &abs));
        list.insert(0, Recent { path: abs, opened: chrono::Local::now().to_rfc3339() });
        list.truncate(MAX_RECENT);
        self.save(&list)
    }

    /// Drop a project from the list (its folder is not touched).
    pub fn forget(&self, shots: &Path) -> Result<()> {
        let mut list = self.list();
        list.retain(|r| !same_path(&r.path, shots));
        self.save(&list)
    }

    pub fn contains(&self, shots: &Path) -> bool {
        self.list().iter().any(|r| same_path(&r.path, shots))
    }

    fn save(&self, list: &[Recent]) -> Result<()> {
        let Some(file) = &self.file else { return Ok(()) };
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let v = json!({"projects": list.iter().map(|r| json!({"path": r.path.display().to_string(), "opened": r.opened}))
            .collect::<Vec<_>>()});
        std::fs::write(file, serde_json::to_string_pretty(&v)?)?;
        Ok(())
    }
}

/// Paths compare without case on Windows, where the file system ignores it.
pub fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let s = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf()).display().to_string().replace('\\', "/");
        if cfg!(windows) { s.to_lowercase() } else { s }
    };
    norm(a) == norm(b)
}

/// A card for the home: name, size, how much is shot, and up to four cover frames
/// (absolute paths; the server decides which it serves). Never fails: a missing or
/// broken project comes back with `missing` or `error` set.
pub fn summary(shots: &Path) -> Value {
    let root = shots.parent().map(Path::to_path_buf).unwrap_or_default();
    let folder = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut v = json!({"path": shots.display().to_string(), "root": root.display().to_string(), "folder": folder});
    if !shots.is_file() {
        v["missing"] = json!(true);
        return v;
    }
    v["modified"] = json!(modified(shots));
    let ws = match Workspace::open(shots) {
        Ok(ws) => ws,
        Err(e) => {
            v["error"] = json!(e.msg);
            return v;
        }
    };
    let d = &ws.defn;
    v["name"] = json!(d.project.name);
    v["engine"] = json!(d.project.engine);
    v["title"] = json!(d.teaser.title);
    v["pitch"] = json!(d.teaser.pitch);
    v["genre"] = json!(d.brief.genre);
    v["kind"] = json!(if d.brief.kind.is_empty() { "teaser" } else { d.brief.kind.as_str() });
    v["target"] = json!(d.teaser.target_duration);
    v["shots"] = json!(d.shots.len());
    let Ok(resolved) = ws.resolved() else {
        v["error"] = json!("shots.yaml has errors");
        return v;
    };
    v["duration"] = json!(crate::geometry::round_to(resolved.iter().map(|s| s.duration).sum(), 1));
    v["beats"] = json!(resolved.iter().map(|s| s.beat.clone()).collect::<Vec<_>>());
    // best quality first, so a card shows the final frame when there is one
    let mut qualities = fp::quality_names(d);
    qualities.reverse();
    let (mut shot, mut fresh, mut covers) = (0, 0, Vec::new());
    for s in &resolved {
        let mut best: Option<(bool, PathBuf)> = None;
        for q in &qualities {
            let Ok(rfp) = fp::render_fingerprint(d, s, q) else { continue };
            if let Some(t) = ws.state.find_take(&s.id, q, &rfp) {
                if let Some(p) = t.thumb() {
                    best = Some((true, p));
                    break;
                }
            }
            if best.is_none() {
                if let Some(p) = ws.state.latest_take(&s.id, q).and_then(|t| t.thumb()) {
                    best = Some((false, p));
                }
            }
        }
        if let Some((is_fresh, p)) = best {
            shot += 1;
            fresh += usize::from(is_fresh);
            if covers.len() < COVER_SHOTS {
                covers.push(json!({"file": p.display().to_string(), "beat": s.beat}));
            }
        }
    }
    v["taken"] = json!(shot);
    v["fresh"] = json!(fresh);
    v["covers"] = json!(covers);
    v["cut"] = json!(has_cut(&ws.root));
    v
}

fn modified(p: &Path) -> Option<String> {
    let t = std::fs::metadata(p).and_then(|m| m.modified()).ok()?;
    Some(chrono::DateTime::<chrono::Local>::from(t).to_rfc3339())
}

/// A cut is written per quality: cut/<quality>/cut.mp4.
fn has_cut(root: &Path) -> bool {
    std::fs::read_dir(root.join("cut"))
        .map(|rd| rd.flatten().any(|e| e.path().join("cut.mp4").is_file()))
        .unwrap_or(false)
}

/// shots.yaml files in a folder or one level below (a UE project keeps its teasers in
/// subfolders next to the .uproject).
pub fn find_projects(dir: &Path) -> Vec<PathBuf> {
    let here = dir.join("shots.yaml");
    if here.is_file() {
        return vec![here];
    }
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path().join("shots.yaml")).filter(|p| p.is_file()).collect())
        .unwrap_or_default();
    out.sort();
    out
}

/// The .uproject a folder (or the file itself) points at.
pub fn find_uproject(p: &Path) -> Option<PathBuf> {
    let is_uproject = |p: &Path| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("uproject"));
    if p.is_file() {
        return is_uproject(p).then(|| p.to_path_buf());
    }
    let mut found: Vec<PathBuf> =
        std::fs::read_dir(p).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_file() && is_uproject(p)).collect();
    found.sort();
    found.into_iter().next()
}

/// Levels under the project's Content folder, as /Game/... paths. Found on disk, so it
/// needs no engine start; World Partition's external actor folders are skipped.
pub fn list_maps(uproject: &Path) -> Vec<String> {
    let Some(content) = uproject.parent().map(|d| d.join("Content")) else { return Vec::new() };
    let mut out = Vec::new();
    walk_maps(&content, &content, 0, &mut out);
    out.sort();
    out
}

fn walk_maps(base: &Path, dir: &Path, depth: usize, out: &mut Vec<String>) {
    const SKIP: &[&str] = &["__ExternalActors__", "__ExternalObjects__", "Developers", "Collections"];
    if depth > 8 || out.len() >= 300 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if !name.starts_with('.') && !SKIP.contains(&name.as_str()) {
                walk_maps(base, &p, depth + 1, out);
            }
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("umap")) {
            if let Ok(rel) = p.with_extension("").strip_prefix(base) {
                let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
                out.push(format!("/Game/{}", parts.join("/")));
            }
        }
    }
}

/// A new project from the home.
#[derive(Debug, Clone, Default)]
pub struct NewProject {
    /// The folder the project folder goes in (ignored for ue5: next to the .uproject).
    pub location: PathBuf,
    /// Project name; also the folder name, made safe for the file system.
    pub name: String,
    /// mock | ue5
    pub engine: String,
    pub uproject: Option<PathBuf>,
    /// e.g. /Game/Maps/Courtyard (ue5 only)
    pub map: Option<String>,
    /// The prompt typed on the home, kept as the teaser's pitch.
    pub pitch: String,
    /// teaser | gameplay
    pub kind: String,
    /// The genre picked on the home (written to brief.genre; empty = none yet).
    pub genre: String,
    /// The six-shot sample instead of an empty project.
    pub sample: bool,
}

/// Write the new project's shots.yaml (and .gitignore, and the sample's placeholder
/// audio). Never writes into a folder that already has a shots.yaml.
pub fn create(p: &NewProject) -> Result<PathBuf> {
    let name = p.name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        bail!("プロジェクト名を入れてください");
    }
    let (parent, uproject_yaml, map_yaml) = match p.engine.as_str() {
        "mock" => (p.location.clone(), "null".to_string(), "null".to_string()),
        "ue5" => {
            let Some(up) = p.uproject.as_deref().and_then(find_uproject) else {
                bail!("UE プロジェクト（.uproject）を選んでください");
            };
            let map = p.map.as_deref().map(str::trim).unwrap_or("");
            if !map.starts_with('/') {
                bail!("撮るマップを選んでください（例: /Game/Maps/Courtyard）");
            }
            let file = up.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let parent = up.parent().map(Path::to_path_buf).unwrap_or_default();
            // the project folder sits next to the .uproject, so the link stays relative
            (parent, serde_json::to_string(&format!("../{file}"))?, crate::yamlio::emit_str(map))
        }
        other => bail!("engine must be mock or ue5 (got '{other}')"),
    };
    if parent.as_os_str().is_empty() {
        bail!("保存先のフォルダを選んでください");
    }
    let dir = free_folder(&parent, &folder_name(&name));
    std::fs::create_dir_all(&dir)?;
    let shots = dir.join("shots.yaml");
    let text = if p.sample {
        templates::render(templates::DEMO, &name, &p.engine, &uproject_yaml, &map_yaml)
    } else {
        let extra = templates::Extra { title: "", pitch: &p.pitch, kind: &p.kind, genre: &p.genre };
        templates::render_with(templates::EMPTY, &name, &p.engine, &uproject_yaml, &map_yaml, &extra)
    };
    std::fs::write(&shots, text)?;
    if p.sample {
        crate::sound::write_demo_audio(&dir)?;
    }
    let gi = dir.join(".gitignore");
    if !gi.exists() {
        std::fs::write(gi, templates::GITIGNORE)?;
    }
    if let Err(e) = crate::model::load(&shots) {
        let _ = std::fs::remove_dir_all(&dir); // it is ours and new; leave nothing half-made
        return Err(e);
    }
    Ok(std::path::absolute(shots)?)
}

/// A folder name the file system accepts on every platform.
pub fn folder_name(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '-' } else { c }).collect();
    let s: String = s.trim().trim_matches('.').trim().chars().take(60).collect();
    if s.is_empty() { "teaser".into() } else { s }
}

/// `parent/name`, or `name-2`, `name-3`... when that is already taken.
pub fn free_folder(parent: &Path, name: &str) -> PathBuf {
    let first = parent.join(name);
    if !first.exists() {
        return first;
    }
    (2..).map(|i| parent.join(format!("{name}-{i}"))).find(|p| !p.exists()).unwrap_or(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recents_move_to_top_and_forget() {
        let dir = tempfile::tempdir().unwrap();
        let r = Recents::new(Some(dir.path().join("projects.json")));
        let a = dir.path().join("a").join("shots.yaml");
        let b = dir.path().join("b").join("shots.yaml");
        r.touch(&a).unwrap();
        r.touch(&b).unwrap();
        r.touch(&a).unwrap();
        let list: Vec<PathBuf> = r.list().into_iter().map(|x| x.path).collect();
        assert_eq!(list.len(), 2);
        assert!(same_path(&list[0], &a));
        r.forget(&a).unwrap();
        assert_eq!(r.list().len(), 1);
        assert!(!r.contains(&a));
    }

    #[test]
    fn recents_carry_over_the_last_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("last_project.txt"), "C:\\x\\shots.yaml\n").unwrap();
        let r = Recents::new(Some(dir.path().join("projects.json")));
        assert_eq!(r.list()[0].path, PathBuf::from("C:\\x\\shots.yaml"));
    }

    #[test]
    fn creates_an_empty_project_with_the_pitch() {
        let dir = tempfile::tempdir().unwrap();
        let p = NewProject {
            location: dir.path().to_path_buf(),
            name: "七号室: ティザー".into(),
            engine: "mock".into(),
            pitch: "中庭で住人が\n一人ずつ消える".into(),
            kind: "gameplay".into(),
            genre: "コージー".into(),
            ..Default::default()
        };
        let shots = create(&p).unwrap();
        assert_eq!(shots.parent().unwrap().file_name().unwrap(), "七号室- ティザー");
        let d = crate::model::load(&shots).unwrap();
        assert_eq!(d.teaser.pitch, "中庭で住人が 一人ずつ消える");
        assert_eq!(d.brief.kind, "gameplay");
        assert_eq!(d.brief.genre, "コージー"); // picked on the home
        assert_eq!(crate::refs::genre_for(&d.brief.genre).map(|g| g.id), Some("cozy"));
        assert!(d.shots.is_empty());
        // the same name again gets its own folder
        let again = create(&p).unwrap();
        assert!(again.parent().unwrap().ends_with("七号室- ティザー-2"));
    }

    #[test]
    fn sample_project_has_shots_and_a_card() {
        let dir = tempfile::tempdir().unwrap();
        let p = NewProject {
            location: dir.path().to_path_buf(),
            name: "sample".into(),
            engine: "mock".into(),
            sample: true,
            ..Default::default()
        };
        let shots = create(&p).unwrap();
        let card = summary(&shots);
        assert_eq!(card["shots"], 6);
        assert_eq!(card["taken"], 0);
        assert_eq!(card["engine"], "mock");
        assert_eq!(card["cut"], false);
        // cuts live in a folder per quality
        let root = shots.parent().unwrap();
        std::fs::create_dir_all(root.join("cut").join("sound")).unwrap();
        assert_eq!(summary(&shots)["cut"], false);
        std::fs::create_dir_all(root.join("cut").join("best")).unwrap();
        std::fs::write(root.join("cut").join("best").join("cut.mp4"), b"").unwrap();
        assert_eq!(summary(&shots)["cut"], true);
        assert!(summary(&dir.path().join("nope").join("shots.yaml"))["missing"].as_bool().unwrap());
        assert_eq!(find_projects(dir.path()), vec![shots.clone()]);
    }

    #[test]
    fn ue_project_sits_next_to_the_uproject() {
        let dir = tempfile::tempdir().unwrap();
        let up = dir.path().join("Game.uproject");
        std::fs::write(&up, "{}").unwrap();
        let maps = dir.path().join("Content").join("Maps");
        std::fs::create_dir_all(maps.join("__ExternalActors__")).unwrap();
        std::fs::write(maps.join("Courtyard.umap"), "").unwrap();
        std::fs::write(maps.join("__ExternalActors__").join("X.umap"), "").unwrap();
        assert_eq!(list_maps(&up), vec!["/Game/Maps/Courtyard".to_string()]);
        let mut p = NewProject { name: "teaser".into(), engine: "ue5".into(), uproject: Some(dir.path().to_path_buf()), ..Default::default() };
        assert!(create(&p).is_err(), "a map is required");
        p.map = Some("/Game/Maps/Courtyard".into());
        let shots = create(&p).unwrap();
        let d = crate::model::load(&shots).unwrap();
        assert_eq!(d.project.uproject.as_deref(), Some("../Game.uproject"));
        assert_eq!(find_projects(dir.path()), vec![shots]);
    }
}
