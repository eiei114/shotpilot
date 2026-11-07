//! The release plan (D-47): what a game puts out over its life, as a checklist.
//!
//! A teaser is not the last video. After it come a gameplay trailer, a demo or festival
//! cut, the launch trailer, one trailer per content update, sale and event reminders, and
//! posts on a regular schedule in between. The plan lists them with a due date and a
//! checklist each, and it is where the next one starts from:
//!
//! - **Milestone videos** are projects of their own. `derive` makes one next to the current
//!   project, with its brief, props, anchors, stage, scout photos and mood board, and, for
//!   an update trailer, its shots and takes too: the takes are content-addressed, so only
//!   the cuts whose game content changed (content.rs) render again.
//! - **Update-driven** work comes from the same detection: the cuts the game changed under.
//! - **Regular posts** are one take as it is (a still or a one-cut clip, landscape) written
//!   to `promo/posts/`. Retakes since the last post come first: what changed in the game
//!   this week is the obvious thing to post.
//!
//! The plan lives in `promo.json` next to shots.yaml (editorial; nothing renders from it).
//! A derived project's `promo.json` only holds `{"link": "<path to the plan>"}`, so every
//! video of a game shares one plan. Nothing here is in a fingerprint, and nothing here
//! writes shots.yaml except `derive`, which writes a new project's.

use std::path::{Component, Path, PathBuf};

use chrono::{Duration, NaiveDate};
use serde_json::{json, Map, Value};

use crate::bail;
use crate::error::Result;
use crate::shotcheck::Severity;
use crate::workspace::Workspace;

pub const FILE: &str = "promo.json";
pub const VERSION: i64 = 1;
/// Days between regular posts unless the plan says otherwise.
pub const EVERY_DAYS: i64 = 7;
/// Where exported posts go, under the project folder.
pub const POSTS_DIR: &str = "promo/posts";

/// A kind of video in the plan.
pub struct Kind {
    pub id: &'static str,
    pub ja: &'static str,
    /// `brief.kind` of a project made for it.
    pub brief: &'static str,
    /// When it is usually made, for the app's add menu.
    pub when: &'static str,
    /// Checks of this kind only, before release (id, text, hint).
    pub checks: &'static [(&'static str, &'static str, &'static str)],
}

pub const KINDS: &[Kind] = &[
    Kind { id: "announce", ja: "発表ティザー", brief: "teaser", when: "発売の半年〜1年前。ストアページの公開と同時",
        checks: &[("wishlist", "最後にウィッシュリストへの誘導を入れた", "発表直後に一番集めたいのはウィッシュリスト")] },
    Kind { id: "gameplay", ja: "ゲームプレイトレーラー", brief: "gameplay", when: "ストアの最初の動画として",
        checks: &[("play_first", "最初の数秒からゲームプレイを見せている", "Steam はストアの最初の動画にゲームプレイ中心のものを勧めている")] },
    Kind { id: "demo", ja: "デモ・フェス向け", brief: "gameplay", when: "Next Fest などのイベント前",
        checks: &[("demo_dates", "デモを遊べる期間とイベント名を書いた", "フェスの提出仕様は主催ごとに違うので、要項を確かめる")] },
    Kind { id: "launch", ja: "ローンチトレーラー", brief: "gameplay", when: "発売の1〜2週間前から発売日まで",
        checks: &[("launch_facts", "発売日・価格・対応プラットフォームを入れた", "エンドカードには今決まっている事実だけを書く")] },
    Kind { id: "update", ja: "アップデートトレーラー", brief: "gameplay", when: "大きなアップデートの公開日",
        checks: &[
            ("whats_new", "何が新しくなったかを最初の数秒で見せている", "前から遊んでいる人が見る。新しい中身を先に"),
            ("patch_notes", "パッチノートや告知文と一緒に出す", "動画だけでは何が変わったか細部までは伝わらない"),
        ] },
    Kind { id: "event", ja: "セール・イベント告知", brief: "teaser", when: "セールや記念日の前日〜当日",
        checks: &[("period", "期間（セール・イベント）と割引を入れた", "期間が過ぎた動画は外す")] },
];

pub fn kind(id: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.id == id)
}

/// One line of a video's checklist.
struct Check {
    id: &'static str,
    section: &'static str,
    text: &'static str,
    hint: &'static str,
    /// Worked out from the project, not ticked by hand.
    auto: bool,
}

const AUTO_CHECKS: &[(&str, &str, &str)] = &[
    ("approved", "全カットの構図を承認した", "構成の段のカット表で1カットずつ承認する"),
    ("p0", "カット検査の P0 がない", "出典のある規則に反しているカットがない"),
    ("game", "ゲームの変化で古くなったカットがない", "中身が変わったカットは「撮り直す」で、そのカットだけ撮り直る"),
    ("cut", "Cut を書き出した", "仕上げの段の「Cut」で、文字と音を焼き込んだ1本にする"),
];

const BEFORE_CHECKS: &[(&str, &str, &str)] = &[
    ("mute", "音を消して見ても伝わる", "ストアや SNS では音なしで再生されることが多い"),
    ("opening", "最初の数秒にロゴやメニューを置いていない", "ストアを見る人は数秒で印象を決める"),
    ("endcard", "エンドカードの日付・プラットフォーム・ストア名が今の事実と合っている", "古い日付のまま出しやすい"),
];

const PUBLISH_CHECKS: &[(&str, &str, &str)] = &[
    ("store", "ストアページに載せた", "古い動画は差し替えるか、順番を後ろに回す"),
    ("sns", "SNS に投稿した", "定期発信の予定にも入れておくと忘れない"),
    ("presskit", "プレスキットに入れた", "メディアや配信者が使えるように"),
];

fn checks_for(kind: &Kind) -> Vec<Check> {
    let mk = |section, auto| move |&(id, text, hint): &(&'static str, &'static str, &'static str)| Check { id, section, text, hint, auto };
    let mut out: Vec<Check> = AUTO_CHECKS.iter().map(mk("制作", true)).collect();
    out.extend(BEFORE_CHECKS.iter().map(mk("公開前の確認", false)));
    out.extend(kind.checks.iter().map(mk("公開前の確認", false)));
    out.extend(PUBLISH_CHECKS.iter().map(mk("公開", false)));
    out
}

// ---------------------------------------------------------------------- the plan file

/// The release plan and the file it lives in.
pub struct Plan {
    /// The promo.json that holds the plan (another project's, when this one links to it).
    pub path: PathBuf,
    pub data: Value,
    /// This project's promo.json points at another project's plan.
    pub linked: bool,
}

/// `a/b/../c` -> `a/c`, without touching the file system (the target may not exist yet).
pub fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn read_json(path: &Path) -> Result<Option<Value>> {
    match std::fs::read_to_string(path) {
        Ok(t) => Ok(Some(serde_json::from_str(&t).map_err(|e| crate::Error::user(format!("{}: {e}", path.display())))?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// The plan file the project at `root` belongs to: its own promo.json, or the one its link
/// names. Projects with the same plan are videos of the same game (D-47).
pub fn plan_of(root: &Path) -> PathBuf {
    let own = root.join(FILE);
    if let Ok(Some(v)) = read_json(&own) {
        if let Some(link) = v.get("link").and_then(Value::as_str).filter(|s| !s.is_empty()) {
            return normalize(&root.join(link));
        }
    }
    normalize(&own)
}

impl Plan {
    /// The plan this project belongs to. Without a promo.json, a starting plan: this video,
    /// and the usual next ones (not written until something changes).
    pub fn open(ws: &Workspace) -> Result<Plan> {
        let own = ws.root.join(FILE);
        if let Some(v) = read_json(&own)? {
            if let Some(link) = v.get("link").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                let target = normalize(&ws.root.join(link));
                if let Some(data) = read_json(&target)? {
                    return Ok(Plan { path: target, data: upgrade(data), linked: true });
                }
                // the plan it pointed at is gone: this project starts its own
            } else {
                return Ok(Plan { path: own, data: upgrade(v), linked: false });
            }
        }
        Ok(Plan { path: own, data: starting_plan(ws), linked: false })
    }

    pub fn dir(&self) -> PathBuf {
        self.path.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.data)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    fn list(&mut self, key: &str) -> &mut Vec<Value> {
        if !self.data[key].is_array() {
            self.data[key] = json!([]);
        }
        self.data[key].as_array_mut().expect("array")
    }

    pub fn videos(&self) -> Vec<Value> {
        self.data["videos"].as_array().cloned().unwrap_or_default()
    }

    pub fn posts(&self) -> Vec<Value> {
        self.data["posts"].as_array().cloned().unwrap_or_default()
    }

    fn video_mut(&mut self, id: &str) -> Result<&mut Map<String, Value>> {
        match self.list("videos").iter_mut().find(|v| v["id"] == id).and_then(Value::as_object_mut) {
            Some(v) => Ok(v),
            None => bail!("発信計画に動画 '{id}' がありません"),
        }
    }

    fn post_mut(&mut self, id: &str) -> Result<&mut Map<String, Value>> {
        match self.list("posts").iter_mut().find(|v| v["id"] == id).and_then(Value::as_object_mut) {
            Some(v) => Ok(v),
            None => bail!("投稿 '{id}' がありません"),
        }
    }

    fn next_id(&self, key: &str, prefix: &str) -> String {
        let used: Vec<String> = self.data[key].as_array().into_iter().flatten().filter_map(|v| v["id"].as_str().map(str::to_string)).collect();
        (1..).map(|n| format!("{prefix}{n}")).find(|id| !used.contains(id)).expect("unbounded")
    }

    /// The project a video row is made in (an absolute shots.yaml), if it has one.
    pub fn project_of(&self, video: &Value) -> Option<PathBuf> {
        video["project"].as_str().filter(|s| !s.is_empty()).map(|rel| normalize(&self.dir().join(rel)))
    }

    /// Add a video row; returns its id.
    pub fn add_video(&mut self, kind_id: &str, title: &str, due: &str, update: &str) -> Result<String> {
        let Some(k) = kind(kind_id) else { bail!("動画の種類が分かりません: {kind_id}") };
        let due = clean_date(due)?;
        let id = self.next_id("videos", "v");
        let title = if title.trim().is_empty() { k.ja.to_string() } else { title.trim().to_string() };
        self.list("videos").push(json!({
            "id": id, "kind": k.id, "title": title, "due": due, "update": update.trim(), "project": null, "checks": {},
        }));
        Ok(id)
    }

    /// Change a video row's title, kind, due date or update name.
    pub fn set_video(&mut self, id: &str, fields: &Map<String, Value>) -> Result<()> {
        let mut clean = Map::new();
        for (k, v) in fields {
            let s = v.as_str().unwrap_or("").trim().to_string();
            match k.as_str() {
                "title" if s.is_empty() => bail!("動画の名前を入れてください"),
                "title" | "update" => {
                    clean.insert(k.clone(), json!(s));
                }
                "due" => {
                    clean.insert(k.clone(), json!(clean_date(&s)?));
                }
                "kind" if kind(&s).is_some() => {
                    clean.insert(k.clone(), json!(s));
                }
                "kind" => bail!("動画の種類が分かりません: {s}"),
                other => bail!("発信計画の動画に '{other}' は書けません（title / kind / due / update）"),
            }
        }
        self.video_mut(id)?.extend(clean);
        Ok(())
    }

    /// Remove a video row. Its project, if any, stays on disk.
    pub fn remove_video(&mut self, id: &str) -> Result<()> {
        let list = self.list("videos");
        let before = list.len();
        list.retain(|v| v["id"] != id);
        if list.len() == before {
            bail!("発信計画に動画 '{id}' がありません");
        }
        Ok(())
    }

    /// Tick or untick a hand check. Checks worked out from the project cannot be ticked.
    pub fn set_check(&mut self, video: &str, check: &str, done: bool) -> Result<()> {
        let kind_id = self.video_mut(video)?.get("kind").and_then(Value::as_str).unwrap_or("").to_string();
        let k = kind(&kind_id).unwrap_or(&KINDS[0]);
        match checks_for(k).iter().find(|c| c.id == check) {
            None => bail!("チェック項目 '{check}' がありません"),
            Some(c) if c.auto => bail!("「{}」はプロジェクトの状態から自動で判定します", c.text),
            Some(_) => {}
        }
        let v = self.video_mut(video)?;
        if !v.get("checks").is_some_and(Value::is_object) {
            v.insert("checks".into(), json!({}));
        }
        let checks = v.get_mut("checks").and_then(Value::as_object_mut).expect("object");
        if done {
            checks.insert(check.into(), json!(true));
        } else {
            checks.remove(check);
        }
        Ok(())
    }

    /// The regular posting schedule and the release date.
    pub fn set_schedule(&mut self, fields: &Map<String, Value>) -> Result<()> {
        for (k, v) in fields {
            match k.as_str() {
                "every_days" => {
                    let n = v.as_i64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())).unwrap_or(0);
                    if !(1..=90).contains(&n) {
                        bail!("投稿の間隔は 1〜90 日で入れてください");
                    }
                    self.data["cadence"]["every_days"] = json!(n);
                }
                "channel" => self.data["cadence"]["channel"] = json!(v.as_str().unwrap_or("").trim()),
                "release" => self.data["release"] = json!(clean_date(v.as_str().unwrap_or(""))?),
                other => bail!("発信計画に '{other}' は書けません（every_days / channel / release）"),
            }
        }
        Ok(())
    }

    /// Mark a post as posted (or not), or change its text.
    pub fn set_post(&mut self, id: &str, fields: &Map<String, Value>) -> Result<()> {
        let mut clean = Map::new();
        for (k, v) in fields {
            match k.as_str() {
                "done" => {
                    clean.insert("done".into(), json!(v.as_bool().unwrap_or(false)));
                }
                "text" | "channel" => {
                    clean.insert(k.clone(), json!(v.as_str().unwrap_or("").trim()));
                }
                other => bail!("投稿に '{other}' は書けません（done / text / channel）"),
            }
        }
        self.post_mut(id)?.extend(clean);
        Ok(())
    }

    /// Forget a post (its exported file stays).
    pub fn remove_post(&mut self, id: &str) -> Result<()> {
        let list = self.list("posts");
        let before = list.len();
        list.retain(|v| v["id"] != id);
        if list.len() == before {
            bail!("投稿 '{id}' がありません");
        }
        Ok(())
    }

    pub fn every_days(&self) -> i64 {
        self.data["cadence"]["every_days"].as_i64().filter(|n| *n > 0).unwrap_or(EVERY_DAYS)
    }
}

fn upgrade(mut v: Value) -> Value {
    if !v.is_object() {
        v = json!({});
    }
    for key in ["videos", "posts"] {
        if !v[key].is_array() {
            v[key] = json!([]);
        }
    }
    if !v["cadence"].is_object() {
        v["cadence"] = json!({"every_days": EVERY_DAYS, "channel": ""});
    }
    if v["version"].is_null() {
        v["version"] = json!(VERSION);
    }
    v
}

fn starting_plan(ws: &Workspace) -> Value {
    let here = if ws.defn.brief.is_gameplay() { "gameplay" } else { "announce" };
    let title = Some(ws.defn.teaser.title.trim()).filter(|t| !t.is_empty()).unwrap_or(ws.defn.project.name.as_str());
    let shots = ws.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "shots.yaml".into());
    let mut videos = vec![json!({
        "id": "v1", "kind": here, "title": title, "due": "", "update": "", "project": shots, "checks": {},
    })];
    for (i, k) in ["demo", "launch"].iter().enumerate() {
        let k = kind(k).expect("known kind");
        videos.push(json!({
            "id": format!("v{}", i + 2), "kind": k.id, "title": k.ja, "due": "", "update": "", "project": null, "checks": {},
        }));
    }
    upgrade(json!({"version": VERSION, "release": "", "videos": videos}))
}

/// "" or a valid YYYY-MM-DD.
fn clean_date(s: &str) -> Result<String> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(String::new());
    }
    match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        Ok(d) => Ok(d.format("%Y-%m-%d").to_string()),
        Err(_) => bail!("日付は 2027-03-01 の形で入れてください（{s}）"),
    }
}

fn date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.get(..10)?, "%Y-%m-%d").ok()
}

pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

// ---------------------------------------------------------------------- what the app shows

/// What the project's state says for the checks worked out in code.
struct Auto {
    approved: (bool, String),
    p0: (bool, String),
    game: (bool, String),
    cut: (bool, String),
}

fn has_cut(root: &Path) -> bool {
    std::fs::read_dir(root.join("cut"))
        .map(|rd| rd.flatten().any(|d| d.path().join("cut.mp4").is_file() || d.path().join("index.html").is_file()))
        .unwrap_or(false)
}

/// The cuts whose game content changed under their takes (content.rs, D-37): id, purpose,
/// the packages that changed, and the qualities whose takes went out of date.
pub fn changed_cuts(ws: &Workspace) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for r in ws.status()? {
        let mut pkgs: Vec<String> = r.changed.iter().flat_map(|(_, p)| p.iter().map(|x| crate::content::short(x).to_string())).collect();
        pkgs.sort();
        pkgs.dedup();
        if !pkgs.is_empty() {
            let qualities: Vec<&str> = r.changed.iter().map(|(q, _)| q.as_str()).collect();
            out.push(json!({"id": r.id, "index": r.index, "purpose": r.purpose, "packages": pkgs, "qualities": qualities}));
        }
    }
    Ok(out)
}

fn auto(ws: &Workspace) -> Result<Auto> {
    let rows = ws.status()?;
    let n = rows.len();
    let approved = rows.iter().filter(|r| r.approval == "approved").count();
    let p0 = ws.check()?.iter().filter(|f| f.severity == Severity::P0).count();
    let changed: Vec<String> = changed_cuts(ws)?.iter().filter_map(|c| c["id"].as_str().map(str::to_string)).collect();
    Ok(Auto {
        approved: (n > 0 && approved == n, format!("{approved}/{n}")),
        p0: (p0 == 0, if p0 == 0 { String::new() } else { format!("P0 {p0} 件") }),
        game: (changed.is_empty(), if changed.is_empty() { String::new() } else { changed.join(", ") }),
        cut: (has_cut(&ws.root), String::new()),
    })
}

/// One video row with its checklist worked out.
fn video_view(plan: &Plan, v: &Value, ws: &Workspace, today: NaiveDate) -> Value {
    let k = kind(v["kind"].as_str().unwrap_or("")).unwrap_or(&KINDS[0]);
    let project = plan.project_of(v);
    let here = project.as_deref().is_some_and(|p| crate::projects::same_path(p, &ws.path));
    let exists = project.as_deref().is_some_and(Path::is_file);
    // the checks worked out in code read that video's own project
    let other;
    let target = if here {
        Some(ws)
    } else if exists {
        other = Workspace::open(project.as_deref().expect("exists")).ok();
        other.as_ref()
    } else {
        None
    };
    let auto = target.and_then(|w| auto(w).ok());
    let ticked = v["checks"].as_object().cloned().unwrap_or_default();
    let mut done_n = 0;
    let checks: Vec<Value> = checks_for(k)
        .iter()
        .map(|c| {
            let (done, detail) = match (c.auto, &auto) {
                (false, _) => (ticked.get(c.id).and_then(Value::as_bool).unwrap_or(false), String::new()),
                (true, None) => (false, "プロジェクトを作ると判定します".to_string()),
                (true, Some(a)) => match c.id {
                    "approved" => a.approved.clone(),
                    "p0" => a.p0.clone(),
                    "game" => a.game.clone(),
                    _ => a.cut.clone(),
                },
            };
            done_n += usize::from(done);
            json!({"id": c.id, "section": c.section, "text": c.text, "hint": c.hint, "auto": c.auto, "done": done, "detail": detail})
        })
        .collect();
    let total = checks.len();
    let due = v["due"].as_str().unwrap_or("");
    let days = date(due).map(|d| (d - today).num_days());
    let state = if done_n == total { "done" } else if exists { "doing" } else { "todo" };
    json!({
        "id": v["id"], "kind": k.id, "kind_ja": k.ja, "title": v["title"], "due": due, "days": days,
        "update": v["update"], "project": project.as_ref().map(|p| p.display().to_string()),
        "here": here, "exists": exists, "missing": project.is_some() && !exists,
        "state": state, "done": done_n, "total": total, "checks": checks,
        "changed": auto.as_ref().map(|a| if a.game.0 { 0 } else { a.game.1.split(", ").count() }).unwrap_or(0),
    })
}

/// A path under the project as the app's file URL.
fn url(root: &Path, p: &Path) -> Option<String> {
    let rel = p.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    Some(format!("/files/{}", parts.join("/")))
}

/// The regular posts: when the next one is due, the ones made, and what to post next.
fn posting_view(plan: &Plan, ws: &Workspace, today: NaiveDate) -> Result<Value> {
    let every = plan.every_days();
    let posts = plan.posts();
    let last_done = posts.iter().filter(|p| p["done"].as_bool().unwrap_or(false)).filter_map(|p| date(p["date"].as_str()?)).max();
    let last_any = posts.iter().filter_map(|p| date(p["date"].as_str()?)).max();
    let next = last_done.map(|d| d + Duration::days(every)).unwrap_or(today);
    let upcoming: Vec<Value> = (0..4)
        .map(|i| {
            let d = next + Duration::days(every * i);
            json!({"date": d.format("%Y-%m-%d").to_string(), "days": (d - today).num_days()})
        })
        .collect();
    let dir = plan.dir();
    let mut made: Vec<Value> = posts
        .iter()
        .map(|p| {
            let file = p["file"].as_str().map(|f| normalize(&dir.join(f)));
            let mut out = p.clone();
            out["url"] = json!(file.as_deref().and_then(|f| url(&ws.root, f)));
            out["exists"] = json!(file.as_deref().is_some_and(Path::is_file));
            out
        })
        .collect();
    made.sort_by(|a, b| b["date"].as_str().cmp(&a["date"].as_str()).then(b["id"].as_str().cmp(&a["id"].as_str())));

    // what to post next: one take per cut (the best fresh one), retakes and new ones first
    let mut cands = Vec::new();
    for (shot, take, q) in ws.pick_takes("best")? {
        let (Some(take), Some(q)) = (take, q) else { continue };
        let created = take.created().to_string();
        let new = match (last_any, date(&created)) {
            (Some(l), Some(c)) => c >= l,
            _ => true,
        };
        let retake = take.dir.file_name().is_some_and(|n| n.to_string_lossy().contains('~'));
        cands.push(json!({
            "shot": shot.id, "index": shot.index, "purpose": shot.purpose, "quality": q, "created": created,
            "new": new, "retake": retake, "thumb": take.thumb().as_deref().and_then(|t| url(&ws.root, t)),
            "posted": posts.iter().any(|p| p["shot"] == shot.id.as_str() && p["take"] == take.fingerprint()),
        }));
    }
    let rank = |c: &Value| (!c["retake"].as_bool().unwrap_or(false), !c["new"].as_bool().unwrap_or(false), c["posted"].as_bool().unwrap_or(false));
    cands.sort_by(|a, b| rank(a).cmp(&rank(b)).then(b["created"].as_str().cmp(&a["created"].as_str())));
    Ok(json!({
        "every_days": every, "channel": plan.data["cadence"]["channel"].as_str().unwrap_or(""),
        "next": next.format("%Y-%m-%d").to_string(), "days": (next - today).num_days(),
        "upcoming": upcoming, "posts": made, "candidates": cands,
        "ffmpeg": crate::cut::ffmpeg().is_some(),
    }))
}

/// Everything the app's publish stage shows.
pub fn view(ws: &Workspace, today: NaiveDate) -> Result<Value> {
    let plan = Plan::open(ws)?;
    let mut videos: Vec<Value> = plan.videos().iter().map(|v| video_view(&plan, v, ws, today)).collect();
    // dated rows in date order, undated ones after them in the order they were added
    videos.sort_by_key(|v| v["due"].as_str().filter(|d| !d.is_empty()).map(str::to_string).unwrap_or_else(|| "~".into()));
    let release = plan.data["release"].as_str().unwrap_or("");
    Ok(json!({
        "plan": plan.path.display().to_string(), "linked": plan.linked, "saved": plan.path.is_file(),
        "today": today.format("%Y-%m-%d").to_string(),
        "release": release, "release_days": date(release).map(|d| (d - today).num_days()),
        "videos": videos, "changed": changed_cuts(ws)?, "posting": posting_view(&plan, ws, today)?,
        "kinds": KINDS.iter().map(|k| json!({"id": k.id, "ja": k.ja, "when": k.when})).collect::<Vec<_>>(),
    }))
}

/// A line for the stage bar: how many videos are done, and when the next post is due.
pub fn summary(ws: &Workspace, today: NaiveDate) -> Value {
    let Ok(plan) = Plan::open(ws) else { return Value::Null };
    let videos = plan.videos();
    let every = plan.every_days();
    let last_done = plan.posts().iter().filter(|p| p["done"].as_bool().unwrap_or(false)).filter_map(|p| date(p["date"].as_str()?)).max();
    let next = last_done.map(|d| d + Duration::days(every));
    let all = |v: &Value| {
        let k = kind(v["kind"].as_str().unwrap_or("")).unwrap_or(&KINDS[0]);
        let ticked = v["checks"].as_object().map(|m| m.len()).unwrap_or(0);
        ticked >= checks_for(k).iter().filter(|c| !c.auto).count()
    };
    json!({
        "saved": plan.path.is_file(), "videos": videos.len(), "published": videos.iter().filter(|v| all(v)).count(),
        "post_days": next.map(|d| (d - today).num_days()),
    })
}

// ---------------------------------------------------------------------- doing things

/// What `derive` carries over from the project it starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carry {
    /// Brief, props, anchors, stage, finish, scout photos, mood board; no shots.
    Groundwork,
    /// All of that, and the shots, script and takes: an update trailer reuses every cut
    /// the game did not change.
    Shots,
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    if !from.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()))?;
        } else {
            std::fs::copy(&p, to.join(e.file_name()))?;
        }
    }
    Ok(())
}

/// Copy the takes of the shots' current definitions (every generation of each), so the
/// new project reuses them: the same definition has the same fingerprint there.
fn copy_takes(ws: &Workspace, to_root: &Path) -> Result<usize> {
    let mut n = 0;
    for s in ws.resolved()? {
        for q in crate::fingerprint::quality_names(&ws.defn) {
            let rfp = crate::fingerprint::render_fingerprint(&ws.defn, &s, &q)?;
            for t in ws.state.takes_for(&s.id, &q, &rfp) {
                let Ok(rel) = t.dir.strip_prefix(&ws.root) else { continue };
                let dest = to_root.join(rel);
                copy_dir(&t.dir, &dest)?;
                repoint_deps(&ws.root, to_root, &dest)?;
                n += 1;
            }
        }
    }
    Ok(n)
}

/// A copied take still watches the same game: content files it recorded relative to the old
/// project folder are pointed at from the new one (absolute ones, such as a UE project's
/// Content/, are left as they are). Only the copy's take.json changes.
fn repoint_deps(from_root: &Path, to_root: &Path, take_dir: &Path) -> Result<()> {
    let path = take_dir.join("take.json");
    let Some(mut info) = read_json(&path)? else { return Ok(()) };
    let Some(deps) = info.get_mut("deps").and_then(Value::as_array_mut) else { return Ok(()) };
    let mut changed = false;
    for d in deps {
        let Some(file) = d["file"].as_str().filter(|f| !Path::new(f).is_absolute()) else { continue };
        let target = normalize(&std::path::absolute(from_root.join(file))?);
        d["file"] = json!(crate::cut::relpath(&target, &normalize(&std::path::absolute(to_root)?)));
        changed = true;
    }
    if changed {
        std::fs::write(&path, serde_json::to_string_pretty(&info)?)?;
    }
    Ok(())
}

/// Start the project for a video row next to this one (D-47). Returns the new shots.yaml.
pub fn derive(ws: &Workspace, video: &str, carry: Carry) -> Result<PathBuf> {
    let mut plan = Plan::open(ws)?;
    let row = plan.videos().into_iter().find(|v| v["id"] == video);
    let Some(row) = row else { bail!("発信計画に動画 '{video}' がありません") };
    if let Some(p) = plan.project_of(&row).filter(|p| p.is_file()) {
        bail!("この動画のプロジェクトはもうあります: {}", p.display());
    }
    let k = kind(row["kind"].as_str().unwrap_or("")).unwrap_or(&KINDS[0]);
    let title = row["title"].as_str().filter(|t| !t.trim().is_empty()).unwrap_or(k.ja).to_string();
    let Some(parent) = ws.root.parent() else { bail!("{} の隣にフォルダを作れません", ws.root.display()) };
    let dir = crate::projects::free_folder(parent, &crate::projects::folder_name(&title));

    let mut doc = crate::yamlio::Doc::load(&ws.path)?;
    let value = doc.value()?;
    doc.upsert_entries("project", &[("name".into(), json!(title))])?;
    if ws.defn.project.engine == "ue5" {
        // the engine assets of each video in their own folder of the UE project
        let base = ws.defn.project.content_root.trim_end_matches('/');
        doc.upsert_entries("project", &[("content_root".into(), json!(format!("{base}/{video}")))])?;
    }
    doc.upsert_entries("brief", &[("kind".into(), json!(k.brief))])?;
    if carry == Carry::Groundwork {
        doc.set_items("shots", &[])?;
        if value.get("script").is_some_and(Value::is_array) {
            doc.set_items("script", &[])?;
        }
    }

    std::fs::create_dir_all(&dir)?;
    let made = (|| -> Result<PathBuf> {
        let shots = dir.join("shots.yaml");
        doc.save(&shots)?;
        for sub in ["scout", "moodboard", "audio"] {
            copy_dir(&ws.root.join(sub), &dir.join(sub))?;
        }
        let gi = ws.root.join(".gitignore");
        std::fs::write(dir.join(".gitignore"), std::fs::read(&gi).unwrap_or_else(|_| crate::templates::GITIGNORE.as_bytes().to_vec()))?;
        if carry == Carry::Shots {
            copy_takes(ws, &dir)?;
        }
        let plan_rel = crate::cut::relpath(&plan.path, &dir);
        std::fs::write(dir.join(FILE), serde_json::to_string_pretty(&json!({"link": plan_rel}))?)?;
        crate::model::load(&shots)?; // the copy must open as a project
        Ok(std::path::absolute(shots)?)
    })();
    let shots = match made {
        Ok(s) => s,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dir); // ours and new; leave nothing half-made
            return Err(e);
        }
    };
    let rel = crate::cut::relpath(&shots, &plan.dir());
    plan.video_mut(video)?.insert("project".into(), json!(rel));
    plan.save()?;
    Ok(shots)
}

/// Add an update trailer row for the cuts the game changed under, made from this project
/// with its shots (only those cuts render again). Returns (video id, new shots.yaml).
pub fn start_update(ws: &Workspace, update: &str, due: &str) -> Result<(String, PathBuf)> {
    let mut plan = Plan::open(ws)?;
    let name = update.trim();
    let title = if name.is_empty() { "アップデートトレーラー".to_string() } else { format!("アップデートトレーラー {name}") };
    let id = plan.add_video("update", &title, due, name)?;
    plan.save()?;
    match derive(ws, &id, Carry::Shots) {
        Ok(p) => Ok((id, p)),
        Err(e) => {
            // keep the plan as it was before the failed start
            let mut plan = Plan::open(ws)?;
            let _ = plan.remove_video(&id);
            plan.save()?;
            Err(e)
        }
    }
}

/// Export one cut's take as a post: a still (its middle frame) or a one-cut clip, as it
/// is (landscape; nothing is re-framed or edited). Returns the post's id.
pub fn export_post(ws: &Workspace, shot: &str, clip: bool, today: NaiveDate) -> Result<String> {
    let picks = ws.pick_takes("best")?;
    let Some((s, take, q)) = picks.into_iter().find(|(s, _, _)| s.id == shot) else {
        bail!("カット '{shot}' がありません");
    };
    let (Some(take), Some(q)) = (take, q) else { bail!("{shot} にはまだ使える take がありません。先に撮ってください") };
    let dir = ws.root.join(POSTS_DIR);
    std::fs::create_dir_all(&dir)?;
    let day = today.format("%Y-%m-%d").to_string();
    let file = if clip {
        if crate::cut::ffmpeg().is_none() {
            bail!("ffmpeg が見つからないので動画は書き出せません。静止画なら書き出せます");
        }
        let (w, h) = crate::fingerprint::resolution(&ws.defn, &crate::fingerprint::quality(&ws.defn, &q)?);
        let seg = crate::cut::segment(&take, w, h, ws.defn.teaser.fps)?;
        let out = dir.join(format!("{day}_{}.mp4", s.id));
        std::fs::copy(seg, &out)?;
        out
    } else {
        let Some(thumb) = take.thumb().filter(|t| t.is_file()) else { bail!("{shot} の take に画像がありません") };
        let ext = thumb.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_else(|| "png".into());
        let out = dir.join(format!("{day}_{}.{ext}", s.id));
        std::fs::copy(thumb, &out)?;
        out
    };
    let mut plan = Plan::open(ws)?;
    let id = plan.next_id("posts", "p");
    let channel = plan.data["cadence"]["channel"].as_str().unwrap_or("").to_string();
    let rel = crate::cut::relpath(&file, &plan.dir());
    plan.list("posts").push(json!({
        "id": id, "date": day, "shot": s.id, "take": take.fingerprint(), "quality": q, "kind": if clip { "clip" } else { "still" },
        "file": rel, "channel": channel, "text": s.purpose, "done": false,
    }));
    plan.save()?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_folds_parent_dirs() {
        assert_eq!(normalize(Path::new("a/b/../c/./d")), PathBuf::from("a/c/d"));
        assert_eq!(normalize(Path::new("../x")), PathBuf::from("../x"));
    }

    #[test]
    fn dates_are_checked() {
        assert_eq!(clean_date(" 2027-3-01 ").unwrap(), "2027-03-01");
        assert_eq!(clean_date("").unwrap(), "");
        assert!(clean_date("来週").is_err());
    }

    #[test]
    fn every_kind_has_its_own_checks_and_the_shared_ones() {
        for k in KINDS {
            let c = checks_for(k);
            assert!(c.iter().any(|c| c.id == "game" && c.auto), "{}", k.id);
            assert!(c.iter().any(|c| c.id == "store" && !c.auto), "{}", k.id);
            let mut ids: Vec<&str> = c.iter().map(|c| c.id).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), c.len(), "duplicate check ids in {}", k.id);
        }
    }
}
