//! Claude Code integration (D-66): keep a game's teaser in step with the game while the
//! developer builds the game with Claude Code.
//!
//! Three pieces, all opt-in:
//! - `shotpilot hook session-start`: at the start of a Claude Code session, tells Claude which
//!   teaser cuts went out of date (the game content they showed changed, or their definition
//!   did). Plain text on stdout, which Claude Code adds to the session's context.
//! - `shotpilot hook post-tool`: after each tool call, says only what changed since the last
//!   call: newly outdated cuts when a file a take showed changed (D-37), or what an edit to
//!   shots.yaml means for re-rendering (D-13). Quiet otherwise; it only stats files until
//!   something moved.
//! - `shotpilot claude install`: writes a Skill (`.claude/skills/shotpilot-teaser/SKILL.md`)
//!   and the two hooks (`.claude/settings.json`) into the game's folder.
//!
//! A hook never blocks the session and never renders: it reports facts, and the Skill holds
//! the steps (re-render only the outdated cuts, ask before launching the engine).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::content;
use crate::error::Result;
use crate::fingerprint as fp;
use crate::{bail, Workspace};

/// The Skill as shipped; `{{SHOTS}}` becomes the path of shots.yaml from the game's folder.
pub const SKILL_TEMPLATE: &str = include_str!("../claude/skills/shotpilot-teaser/SKILL.md");
pub const SKILL_NAME: &str = "shotpilot-teaser";

/// Where the hook keeps what it last saw (inside the project's `.shotpilot/`).
const HOOK_STATE: &str = "claude_hook.json";

/// One outdated cut at one quality, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Outdated {
    pub shot: String,
    pub quality: String,
    /// The game content that changed under it; empty when its definition changed instead.
    pub packages: Vec<String>,
}

/// Every cut whose take at an engine quality (or previz) is out of date. Qualities the cut
/// was never rendered at are not listed (nothing to re-shoot there yet).
pub fn outdated(ws: &Workspace) -> Result<Vec<Outdated>> {
    let mut out = Vec::new();
    let qs = fp::quality_names(&ws.defn);
    for row in ws.status()? {
        for q in &qs {
            if row.quality(q) != "stale" {
                continue;
            }
            let packages = row.changed.iter().find(|(k, _)| k == q).map(|(_, v)| v.clone()).unwrap_or_default();
            out.push(Outdated { shot: row.id.clone(), quality: q.clone(), packages });
        }
    }
    Ok(out)
}

/// "s01_chair_hook, s05_corridor" for each quality, engine qualities first, previz last.
fn by_quality(items: &[&Outdated]) -> Vec<(String, Vec<String>)> {
    let mut m: BTreeMap<usize, (String, Vec<String>)> = BTreeMap::new();
    for o in items {
        let rank = match o.quality.as_str() {
            "final" => 0,
            "preview" => 1,
            "still" => 2,
            "previz" => 4,
            _ => 3,
        };
        let e = m.entry(rank).or_insert_with(|| (o.quality.clone(), Vec::new()));
        if !e.1.contains(&o.shot) {
            e.1.push(o.shot.clone());
        }
    }
    m.into_values().collect()
}

fn packages_of(items: &[&Outdated]) -> Vec<String> {
    let mut seen = Vec::new();
    for o in items {
        for p in &o.packages {
            let s = content::short(p).to_string();
            if !seen.contains(&s) {
                seen.push(s);
            }
        }
    }
    seen
}

/// How the developer runs shotpilot on this project from the game's folder.
fn cmd(shots: &str, rest: &str) -> String {
    format!("shotpilot --file {shots} {rest}")
}

/// The facts a hook hands to Claude: which cuts are out of date, why, and the command that
/// re-shoots only those. None when nothing is out of date.
pub fn describe(ws: &Workspace, shots: &str, items: &[Outdated]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let total = ws.defn.shot_ids().len();
    let game: Vec<&Outdated> = items.iter().filter(|o| !o.packages.is_empty()).collect();
    let defn: Vec<&Outdated> = items.iter().filter(|o| o.packages.is_empty() && o.quality != "previz").collect();
    let mut lines = vec![format!("Shotpilot（ティザー {shots}、{total} カット）: 撮り直しが要るカットがある。")];
    if !game.is_empty() {
        let pk = packages_of(&game);
        let shown = if pk.len() > 4 { format!("{} ほか {} 件", pk[..4].join(", "), pk.len() - 4) } else { pk.join(", ") };
        lines.push(format!("- ゲームの素材が変わった（{shown}）。それが画面に写っていたカットだけが古い:"));
        for (q, ids) in by_quality(&game) {
            lines.push(format!("  - {q}: {}", ids.join(", ")));
        }
    }
    if !defn.is_empty() {
        lines.push("- shots.yaml の定義が変わったカット:".to_string());
        for (q, ids) in by_quality(&defn) {
            lines.push(format!("  - {q}: {}", ids.join(", ")));
        }
    }
    let engine: Vec<String> = by_quality(&items.iter().filter(|o| o.quality != "previz").collect::<Vec<_>>())
        .into_iter()
        .map(|(q, _)| q)
        .collect();
    if let Some(q) = engine.first() {
        let slow = if ws.defn.project.engine == "ue5" { "（UE を起動する。数分かかる）" } else { "" };
        lines.push(format!("撮り直すのはこのカットだけで、ほかのカットは前の take を使い回す: `{}`{slow}。", cmd(shots, &format!("render {q}"))));
    }
    if items.iter().any(|o| o.quality == "previz") {
        lines.push(format!("エンジンなしの確認は `{}`（数秒）。", cmd(shots, "render previz")));
    }
    lines.push(format!("手順は Skill「{SKILL_NAME}」にある。ライト・配置・コードの変更は検知の対象外。"));
    Some(lines.join("\n"))
}

// ---------------------------------------------------------------------------- hooks

#[derive(Default)]
struct HookState {
    game_stamp: String,
    game_reported: Vec<String>,
    shots_hash: String,
}

impl HookState {
    fn load(ws: &Workspace) -> HookState {
        let p = ws.root.join(".shotpilot").join(HOOK_STATE);
        let v: Value = std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
        let strs = |k: &str| v[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        HookState {
            game_stamp: v["game_stamp"].as_str().unwrap_or("").to_string(),
            game_reported: strs("game_reported"),
            shots_hash: v["shots_hash"].as_str().unwrap_or("").to_string(),
        }
    }

    fn save(&self, ws: &Workspace) {
        let dir = ws.root.join(".shotpilot");
        let _ = std::fs::create_dir_all(&dir);
        let v = json!({"game_stamp": self.game_stamp, "game_reported": self.game_reported, "shots_hash": self.shots_hash});
        let _ = std::fs::write(dir.join(HOOK_STATE), serde_json::to_string_pretty(&v).unwrap_or_default());
    }
}

fn key(o: &Outdated) -> String {
    format!("{}:{}", o.shot, o.quality)
}

/// SessionStart: everything out of date now (or nothing). Remembers what it said, so the
/// post-tool hook only speaks about later changes.
pub fn session_start(ws: &Workspace, shots: &str) -> Result<Option<String>> {
    let items = outdated(ws)?;
    let mut st = HookState::load(ws);
    st.game_stamp = content::stamp_files(&content::recorded_files(&ws.root));
    st.game_reported = items.iter().filter(|o| !o.packages.is_empty()).map(key).collect();
    st.shots_hash = content::hash_file(&ws.path).unwrap_or_default();
    st.save(ws);
    Ok(describe(ws, shots, &items))
}

/// PostToolUse: what the last tool call changed, if anything. `edited` is the file the tool
/// wrote (Edit/Write), when it says so.
pub fn post_tool(ws: &Workspace, shots: &str, edited: Option<&Path>) -> Result<Option<String>> {
    let mut st = HookState::load(ws);
    let mut said = Vec::new();

    // an edit to shots.yaml: what it means for re-rendering (D-13: only the cuts whose picture changed)
    let hash = content::hash_file(&ws.path).unwrap_or_default();
    let touched_shots = edited.is_some_and(|p| same_file(p, &ws.path));
    if hash != st.shots_hash && (touched_shots || !st.shots_hash.is_empty()) {
        let items: Vec<Outdated> = outdated(ws)?.into_iter().filter(|o| o.packages.is_empty()).collect();
        let total = ws.defn.shot_ids().len();
        if items.is_empty() {
            said.push(format!("Shotpilot: {shots} が変わったが、画を変える項目は変わっていない（並び・目的・音・文字だけの変更）。撮り直しは無い（{total} カットとも前の take のまま）。"));
        } else if let Some(text) = describe(ws, shots, &items) {
            said.push(text);
        }
        st.shots_hash = hash;
    }

    // a change to the game: only stat the files the takes showed; look closer when they moved
    let stamp = content::stamp_files(&content::recorded_files(&ws.root));
    if stamp != st.game_stamp {
        let game: Vec<Outdated> = outdated(ws)?.into_iter().filter(|o| !o.packages.is_empty()).collect();
        let fresh: Vec<Outdated> = game.iter().filter(|o| !st.game_reported.contains(&key(o))).cloned().collect();
        if let Some(text) = describe(ws, shots, &fresh) {
            said.push(text);
        }
        st.game_reported = game.iter().map(key).collect();
        st.game_stamp = stamp;
    }
    st.save(ws);
    Ok(if said.is_empty() { None } else { Some(said.join("\n")) })
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// The file an Edit/Write/MultiEdit call wrote, from the hook's stdin.
pub fn edited_file(input: &Value) -> Option<PathBuf> {
    let p = input["tool_input"]["file_path"].as_str()?;
    let path = PathBuf::from(p);
    if path.is_absolute() {
        return Some(path);
    }
    let cwd = input["cwd"].as_str().map(PathBuf::from).or_else(|| std::env::current_dir().ok())?;
    Some(cwd.join(path))
}

/// The PostToolUse output: context for Claude (and a line for the developer).
pub fn post_tool_json(text: &str) -> Value {
    json!({
        "hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": text},
        "systemMessage": text.lines().next().unwrap_or(""),
    })
}

// ---------------------------------------------------------------------------- install

/// What `claude install` wrote.
#[derive(Debug, Default)]
pub struct InstallReport {
    pub game_root: PathBuf,
    pub shots: String,
    pub skill: PathBuf,
    pub skill_written: bool,
    pub settings: PathBuf,
    pub hooks_added: Vec<String>,
}

/// The game's folder for a project: the folder of its .uproject when it has one, else the
/// project's own folder.
pub fn game_root(ws: &Workspace) -> PathBuf {
    if let Some(up) = ws.defn.project.uproject.as_deref().filter(|u| !u.is_empty()) {
        let p = ws.root.join(up);
        if let Some(dir) = p.parent().filter(|_| p.is_file()) {
            return abs(dir);
        }
    }
    abs(&ws.root)
}

/// An absolute, normalized path (canonicalize would add a verbatim prefix on Windows).
fn abs(p: &Path) -> PathBuf {
    let a = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    a.components().collect()
}

/// shots.yaml as the hooks name it: relative to the game's folder (through Claude Code's
/// project folder variable), or absolute when it lives elsewhere.
pub fn shots_arg(ws: &Workspace, root: &Path) -> String {
    let file = abs(&ws.path);
    match file.strip_prefix(abs(root)) {
        Ok(rel) => format!("\"${{CLAUDE_PROJECT_DIR}}/{}\"", rel.to_string_lossy().replace('\\', "/")),
        Err(_) => format!("\"{}\"", file.to_string_lossy().replace('\\', "/")),
    }
}

/// shots.yaml for people (the Skill's text): relative to the game's folder when it can be.
fn shots_display(ws: &Workspace, root: &Path) -> String {
    let file = abs(&ws.path);
    match file.strip_prefix(abs(root)) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => file.to_string_lossy().replace('\\', "/"),
    }
}

/// The hooks block `claude install` adds (also printed by `claude install --print`).
pub fn hooks_json(shots_arg: &str) -> Value {
    json!({
        "SessionStart": [{
            "matcher": "startup|resume|clear|compact",
            "hooks": [{"type": "command", "command": format!("shotpilot --file {shots_arg} hook session-start"), "timeout": 30}]
        }],
        "PostToolUse": [{
            "matcher": "Edit|Write|MultiEdit|Bash|PowerShell",
            "hooks": [{"type": "command", "command": format!("shotpilot --file {shots_arg} hook post-tool"), "timeout": 30}]
        }]
    })
}

fn has_command(groups: &Value, needle: &str) -> bool {
    groups.as_array().into_iter().flatten().any(|g| {
        g["hooks"].as_array().into_iter().flatten().any(|h| h["command"].as_str().is_some_and(|c| c.contains(needle)))
    })
}

/// Write the Skill and the hooks into the game's folder. Existing settings are kept (keys,
/// order and other hooks); a hook already there is not added twice; a Skill edited by hand
/// is kept unless `force`.
pub fn install(ws: &Workspace, root: &Path, force: bool) -> Result<InstallReport> {
    let mut rep = InstallReport { game_root: root.to_path_buf(), ..Default::default() };
    let shots_disp = shots_display(ws, root);
    let arg = shots_arg(ws, root);
    rep.shots = shots_disp.clone();

    // the Skill
    let skill_dir = root.join(".claude").join("skills").join(SKILL_NAME);
    let skill = skill_dir.join("SKILL.md");
    let text = SKILL_TEMPLATE.replace("{{SHOTS}}", &shots_disp);
    let current = std::fs::read_to_string(&skill).ok();
    if current.as_deref() != Some(text.as_str()) && (current.is_none() || force) {
        std::fs::create_dir_all(&skill_dir)?;
        std::fs::write(&skill, &text)?;
        rep.skill_written = true;
    }
    rep.skill = skill;

    // the hooks, merged into .claude/settings.json
    let settings = root.join(".claude").join("settings.json");
    let mut doc: Value = match std::fs::read_to_string(&settings) {
        Ok(t) if !t.trim().is_empty() => match serde_json::from_str(&t) {
            Ok(v) => v,
            Err(e) => bail!("{} is not valid JSON ({e}); add the hooks by hand: `shotpilot claude install --print`", settings.display()),
        },
        _ => json!({}),
    };
    let Some(obj) = doc.as_object_mut() else { bail!("{} is not a JSON object", settings.display()) };
    let hooks = obj.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    let Some(hooks) = hooks.as_object_mut() else { bail!("`hooks` in {} is not an object", settings.display()) };
    let want = hooks_json(&arg);
    for (event, needle) in [("SessionStart", "hook session-start"), ("PostToolUse", "hook post-tool")] {
        let groups = hooks.entry(event).or_insert_with(|| json!([]));
        if has_command(groups, needle) {
            continue;
        }
        let Some(list) = groups.as_array_mut() else { bail!("`hooks.{event}` in {} is not a list", settings.display()) };
        list.extend(want[event].as_array().cloned().unwrap_or_default());
        rep.hooks_added.push(event.to_string());
    }
    if !rep.hooks_added.is_empty() {
        std::fs::create_dir_all(settings.parent().unwrap_or(root))?;
        std::fs::write(&settings, serde_json::to_string_pretty(&doc)? + "\n")?;
    }
    rep.settings = settings;
    Ok(rep)
}
