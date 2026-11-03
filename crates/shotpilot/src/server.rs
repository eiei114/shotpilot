//! The app's local backend: a small HTTP + Server-Sent Events layer over Workspace.
//!
//! The desktop app (crates/shotpilot-app) opens one window on this server. It binds to
//! loopback, answers only loopback Host/Origin headers, and serves files only from the
//! project's render/, cut/ and audio/ folders.
//!
//! shots.yaml and .shotpilot/state.json stay the source of truth: every request opens a
//! fresh Workspace, so edits made from the CLI or a text editor show up immediately.
//! Mutations are serialized with one lock; a second mutation while one runs gets 409.
//! Status answers carry an ETag built from the two files' mtimes, so the window's
//! polling costs a stat() and a 304 while nothing changes.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::UNIX_EPOCH;

use serde_json::{json, Map, Value};

use crate::ai::{self, Claude};
use crate::connect::{self, Provider, Settings};
use crate::bail;
use crate::error::{Error, Kind, Result};
use crate::fingerprint as fp;
use crate::sound;
pub use crate::workspace::field_changes;
use crate::workspace::Workspace;

pub const INDEX_HTML: &str = include_str!("../web/index.html");
const LOOPBACK: &[&str] = &["127.0.0.1", "localhost", "::1"];
const SERVED_DIRS: &[&str] = &["render", "cut", "audio", "moodboard", "scout"];
/// Loopback only; large enough for a mood-board image sent as base64.
const MAX_BODY: usize = 20 << 20;

/// Makes the Claude client for one request from the current settings. Tests pass a
/// fake; the app uses [`connect::make_claude`] (Claude Code or an API key).
pub type ClaudeFactory = Box<dyn Fn(&Settings) -> Result<Box<dyn Claude>> + Send + Sync>;

pub fn default_claude() -> ClaudeFactory {
    Box::new(connect::make_claude)
}

/// Makes the image model for mood-board frames. Tests pass a fake.
pub type ImageFactory = Box<dyn Fn(&Settings) -> Result<Box<dyn crate::moodboard::ImageGen>> + Send + Sync>;

pub struct App {
    pub path: PathBuf,
    lock: Mutex<()>,
    claude: ClaudeFactory,
    images: ImageFactory,
    /// Where the connection settings live; None = read-only defaults.
    settings_path: Option<PathBuf>,
}

impl App {
    pub fn new(shots_path: &Path, claude: ClaudeFactory, settings_path: Option<PathBuf>) -> Result<App> {
        let ws = Workspace::open(shots_path)?; // fail fast on a broken definition
        Ok(App { path: ws.path, lock: Mutex::new(()), claude, images: Box::new(connect::make_image_gen), settings_path })
    }

    fn settings(&self) -> Settings {
        Settings::load(self.settings_path.as_deref())
    }

    /// The connection as the UI shows it: provider, what was found, and whether it works.
    fn ai_status(&self) -> Value {
        let s = self.settings();
        let mut v = connect::describe(&s);
        match (self.claude)(&s) {
            Ok(_) => v["ready"] = json!(true),
            Err(e) => {
                v["ready"] = json!(false);
                v["error"] = json!(e.msg);
            }
        }
        v["model"] = json!(ai::model_name());
        v
    }

    fn ws(&self) -> Result<Workspace> {
        Workspace::open(&self.path)
    }

    fn mutate(&self) -> Result<MutexGuard<'_, ()>> {
        match self.lock.try_lock() {
            Ok(g) => Ok(g),
            Err(std::sync::TryLockError::Poisoned(p)) => Ok(p.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => {
                Err(Error::busy("another operation is running; try again when it finishes"))
            }
        }
    }

    fn root(&self) -> PathBuf {
        self.path.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    /// A served file under render/, cut/ or audio/, from a `/files/...` remainder.
    fn served_file(&self, rel: &str) -> Option<PathBuf> {
        let rel = percent_decode(rel);
        let segs: Vec<&str> = rel.split('/').collect();
        if segs.len() < 2 || !SERVED_DIRS.contains(&segs[0]) {
            return None;
        }
        if segs.iter().any(|s| s.is_empty() || *s == "." || *s == ".." || s.contains(['\\', ':'])) {
            return None;
        }
        let p = segs.iter().fold(self.root(), |acc, s| acc.join(s));
        p.is_file().then_some(p)
    }
}

pub struct Server {
    listener: TcpListener,
    app: Arc<App>,
    bind_host: String,
}

impl Server {
    pub fn bind(shots_path: &Path, host: &str, port: u16) -> Result<Server> {
        Server::bind_with(shots_path, host, port, default_claude(), Settings::default_path())
    }

    pub fn bind_with(shots_path: &Path, host: &str, port: u16, claude: ClaudeFactory, settings_path: Option<PathBuf>)
        -> Result<Server> {
        let app = Arc::new(App::new(shots_path, claude, settings_path)?);
        let listener = TcpListener::bind((host, port))
            .map_err(|e| Error::user(format!("cannot listen on {host}:{port}: {e}")))?;
        Ok(Server { listener, app, bind_host: host.to_ascii_lowercase() })
    }

    /// Use another image model (tests).
    pub fn with_images(mut self, images: ImageFactory) -> Server {
        if let Some(app) = Arc::get_mut(&mut self.app) {
            app.images = images;
        }
        self
    }

    pub fn port(&self) -> u16 {
        self.listener.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    pub fn url(&self) -> String {
        let host = if self.bind_host == "0.0.0.0" || self.bind_host.is_empty() { "127.0.0.1" } else { &self.bind_host };
        format!("http://{host}:{}/", self.port())
    }

    pub fn serve_forever(self) {
        let bind_host = Arc::new(self.bind_host);
        for conn in self.listener.incoming() {
            let Ok(stream) = conn else { continue };
            let app = Arc::clone(&self.app);
            let bind_host = Arc::clone(&bind_host);
            std::thread::spawn(move || {
                let _ = handle(stream, &app, &bind_host);
            });
        }
    }

    /// Serve on a background thread.
    pub fn spawn(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || self.serve_forever())
    }
}

// --------------------------------------------------------------------------- HTTP plumbing

struct Request {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    fn json(&self) -> Result<Map<String, Value>> {
        if self.body.is_empty() {
            return Ok(Map::new());
        }
        let v: Value = serde_json::from_slice(&self.body)
            .map_err(|e| Error::user(format!("request body is not JSON: {e}")))?;
        match v {
            Value::Object(m) => Ok(m),
            _ => bail!("request body must be a JSON object"),
        }
    }
}

fn read_request(stream: &TcpStream) -> std::io::Result<Option<Request>> {
    let mut r = BufReader::new(stream);
    let mut line = String::new();
    if r.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target, String::new()),
    };
    let mut headers = Vec::new();
    loop {
        let mut h = String::new();
        if r.read_line(&mut h)? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    let len: usize = headers.iter().find(|(k, _)| k == "content-length").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
    let mut body = vec![0; len.min(MAX_BODY + 1)];
    r.read_exact(&mut body)?;
    Ok(Some(Request { method, path, query, headers, body }))
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        304 => "Not Modified",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Payload Too Large",
        _ => "Internal Server Error",
    }
}

fn send(stream: &mut TcpStream, head_only: bool, code: u16, ctype: &str, body: &[u8], extra: &[(&str, String)])
    -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {code} {}\r\nServer: shotpilot\r\nConnection: close\r\n", status_text(code));
    if code != 304 {
        head.push_str(&format!("Content-Type: {ctype}\r\nContent-Length: {}\r\n", body.len()));
    }
    for (k, v) in extra {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if !head_only && code != 304 {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn send_json(stream: &mut TcpStream, head_only: bool, code: u16, v: &Value) -> std::io::Result<()> {
    send(stream, head_only, code, "application/json; charset=utf-8", v.to_string().as_bytes(),
         &[("Cache-Control", "no-store".into())])
}

fn hostname(netloc: &str) -> String {
    let n = netloc.trim().to_ascii_lowercase();
    if let Some(rest) = n.strip_prefix('[') {
        return rest.split(']').next().unwrap_or("").to_string();
    }
    n.rsplit_once(':').map(|(h, _)| h.to_string()).unwrap_or(n)
}

fn host_ok(req: &Request, bind_host: &str) -> bool {
    let allowed = |h: &str| LOOPBACK.contains(&h) || h == bind_host;
    if !allowed(&hostname(req.header("host").unwrap_or(""))) {
        return false;
    }
    match req.header("origin") {
        None | Some("null") => true,
        Some(o) => {
            let netloc = o.split("://").nth(1).unwrap_or(o).split('/').next().unwrap_or("");
            allowed(&hostname(netloc))
        }
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn mime(path: &Path) -> &'static str {
    match path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("bmp") => "image/bmp",
        Some("mp4") => "video/mp4",
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("ogg" | "opus") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("m4a" | "aac") => "audio/mp4",
        Some("html") => "text/html; charset=utf-8",
        Some("md" | "txt") => "text/plain; charset=utf-8",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

// --------------------------------------------------------------------------- payloads

fn file_url(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    Some(format!("/files/{}", parts.join("/")))
}

pub fn status_payload(ws: &Workspace) -> Result<Value> {
    let qualities = fp::quality_names(&ws.defn);
    let resolved = ws.resolved()?;
    let mut shots = Vec::new();
    for row in ws.status()? {
        let s = resolved.iter().find(|s| s.id == row.id).expect("same definition");
        let mut thumbs = Map::new();
        for q in &qualities {
            let rfp = fp::render_fingerprint(&ws.defn, s, q)?;
            let fresh = ws.state.find_take(&s.id, q, &rfp);
            let is_fresh = fresh.is_some();
            let take = fresh.or_else(|| ws.state.list_takes(&s.id, q).pop());
            let url = take.as_ref().and_then(|t| t.thumb()).and_then(|p| file_url(&ws.root, &p));
            let frames: Vec<String> = take
                .as_ref()
                .map(|t| t.files().iter().filter_map(|f| file_url(&ws.root, &t.dir.join(f))).collect())
                .unwrap_or_default();
            let numbers = take.as_ref().map(|t| t.frames()).unwrap_or_default();
            thumbs.insert(q.clone(), json!({
                "url": url, "fresh": is_fresh, "fingerprint": &rfp[..12], "frames": frames, "frame_numbers": numbers,
            }));
        }
        let mut v = row.to_json();
        v["lens"] = s.lens.get("focal_length").cloned().unwrap_or(Value::Null);
        v["thumbs"] = Value::Object(thumbs);
        v["node"] = ws.defn.shots[row.index].clone();
        v["framing"] = crate::shotcheck::framing(&ws.defn, s).map(|f| f.to_json()).unwrap_or(Value::Null);
        shots.push(v);
    }
    let total: f64 = resolved.iter().map(|s| s.duration).sum();
    Ok(json!({
        "project": ws.defn.project.name,
        "engine": ws.defn.project.engine,
        "title": ws.defn.teaser.title,
        "path": ws.path.display().to_string(),
        "target_duration": ws.defn.teaser.target_duration,
        "total_duration": crate::geometry::round_to(total, 3),
        "qualities": qualities,
        "warnings": ws.structure_warnings()?,
        "checks": ws.check()?.iter().map(|f| f.to_json()).collect::<Vec<_>>(),
        "brief": brief_json(&ws.defn.brief),
        "scout": crate::scout::summary(&ws.root),
        "scout_taken": crate::scout::load_index(&ws.root)["taken"],
        "stage": {
            "summary": ws.defn.stage.summary,
            "anchors": ws.defn.stage.anchors.iter().map(|(n, d)| json!({"name": n, "description": d})).collect::<Vec<_>>(),
            "mood": ws.defn.stage.mood.iter().map(|(f, a)| (f.clone(), json!(a))).collect::<Map<String, Value>>(),
        },
        "props": ws.defn.props.iter().map(|(n, p)| json!({"name": n, "asset": p.asset, "kind": p.kind, "size": p.size})).collect::<Vec<_>>(),
        "assets_count": crate::scout::load_assets(&ws.root).len(),
        "pitch": ws.defn.teaser.pitch,
        "fps": ws.defn.teaser.fps,
        "shots": shots,
        "audio": ws.defn.audio,
        "cut": latest_cut(&ws.root),
    }))
}

fn brief_json(b: &crate::model::BriefCfg) -> Value {
    json!({
        "kind": b.kind, "genre": b.genre, "audience": b.audience, "core": b.core,
        "must_show": b.must_show, "references": b.references, "notes": b.notes,
        "missing": b.missing(), "written": !b.raw.is_empty(), "images": b.images,
    })
}

/// What the app's sound lanes and sound tab draw: every placement on the teaser's
/// timeline, the files that can be placed, and the cached mix with its sound check.
pub fn sound_payload(ws: &Workspace) -> Result<Value> {
    let shots = ws.resolved()?;
    let (starts, total) = sound::timing(&ws.defn, &shots);
    let fps = ws.defn.teaser.fps.max(1) as f64;
    let placements: Vec<Value> = sound::plan(&ws.defn, &shots)
        .iter()
        .map(|p| {
            let mut v = p.to_json();
            v["missing"] = json!(!ws.root.join(&p.file).is_file());
            v["url"] = json!(file_url(&ws.root, &ws.root.join(&p.file)));
            v
        })
        .collect();
    let mut out = json!({
        "files": sound::list_files(&ws.root)
            .iter()
            .map(|f| json!({"file": f, "url": file_url(&ws.root, &ws.root.join(f))}))
            .collect::<Vec<_>>(),
        "audio": ws.defn.audio,
        "total": crate::geometry::round_to(total, 3),
        "shots": shots.iter().zip(&starts).map(|(s, t)| json!({
            "id": s.id, "start": crate::geometry::round_to(*t, 3),
            "duration": crate::geometry::round_to(s.frames as f64 / fps, 3), "beat": s.beat,
        })).collect::<Vec<_>>(),
        "cuts": sound::cuts(&shots, &starts).iter().map(|c| json!({"t": crate::geometry::round_to(c.t, 3), "from": c.from, "to": c.to}))
            .collect::<Vec<_>>(),
        "placements": placements,
        "lead_max": sound::LEAD_MAX,
        "ffmpeg": crate::cut::ffmpeg().is_some(),
        "mix": Value::Null,
        "error": Value::Null,
    });
    match ws.mix() {
        Ok(Some(m)) => {
            let mut v = m.to_json();
            v["url"] = json!(file_url(&ws.root, &m.wav).map(|u| format!("{u}?v={}", &m.key[..12])));
            out["mix"] = v;
        }
        Ok(None) => {}
        Err(e) => out["error"] = json!(e.msg),
    }
    Ok(out)
}

/// The most recently written cut/<quality>/ folder that has a video or a storyboard.
fn latest_cut(root: &Path) -> Value {
    let Ok(dirs) = std::fs::read_dir(root.join("cut")) else { return Value::Null };
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for d in dirs.flatten() {
        let dir = d.path();
        let marker = if dir.join("cut.mp4").is_file() { dir.join("cut.mp4") } else { dir.join("index.html") };
        if let Ok(t) = std::fs::metadata(&marker).and_then(|m| m.modified()) {
            if !best.as_ref().is_some_and(|(bt, _)| t <= *bt) {
                best = Some((t, dir));
            }
        }
    }
    let Some((when, dir)) = best else { return Value::Null };
    let stamp = when.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let video = dir.join("cut.mp4");
    json!({
        "quality": dir.file_name().map(|n| n.to_string_lossy().to_string()),
        "video_url": if video.is_file() { file_url(root, &video) } else { None },
        "storyboard": file_url(root, &dir.join("index.html")),
        "stamp": stamp.to_string(),
    })
}

fn etag(app: &App) -> String {
    let stamp = |p: PathBuf| {
        std::fs::metadata(p)
            .ok()
            .map(|m| {
                let t = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0);
                format!("{t:x}-{:x}", m.len())
            })
            .unwrap_or_else(|| "0".into())
    };
    let root = app.root();
    let settings = app.settings_path.clone().map(stamp).unwrap_or_default();
    format!("\"{}.{}.{settings}\"", stamp(app.path.clone()), stamp(root.join(".shotpilot").join("state.json")))
}

// --------------------------------------------------------------------------- routes

fn handle(mut stream: TcpStream, app: &App, bind_host: &str) -> std::io::Result<()> {
    let Some(req) = read_request(&stream)? else { return Ok(()) };
    let head = req.method == "HEAD";
    if !host_ok(&req, bind_host) {
        return send_json(&mut stream, head, 403, &json!({"error": "forbidden host or origin"}));
    }
    if req.body.len() > MAX_BODY {
        return send_json(&mut stream, head, 413, &json!({"error": "request body too large"}));
    }
    let result = route(&mut stream, app, &req);
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            let code = if e.kind == Kind::Busy { 409 } else { 400 };
            send_json(&mut stream, head, code, &json!({"error": e.msg}))
        }
    }
}

fn route(stream: &mut TcpStream, app: &App, req: &Request) -> Result<()> {
    let head = req.method == "HEAD";
    let io = |e: std::io::Error| Error::user(format!("io: {e}"));
    match (req.method.as_str(), req.path.as_str()) {
        ("GET" | "HEAD", "/") => send(stream, head, 200, "text/html; charset=utf-8", INDEX_HTML.as_bytes(),
                                       &[("Cache-Control", "no-store".into())]).map_err(io),
        ("GET" | "HEAD", "/api/status") => {
            let tag = etag(app);
            if req.header("if-none-match") == Some(tag.as_str()) {
                return send(stream, head, 304, "", b"", &[("ETag", tag)]).map_err(io);
            }
            let mut body = status_payload(&app.ws()?)?;
            body["ai"] = app.ai_status();
            send(stream, head, 200, "application/json; charset=utf-8", body.to_string().as_bytes(),
                 &[("Cache-Control", "no-cache".into()), ("ETag", tag)]).map_err(io)
        }
        ("GET" | "HEAD", "/api/log") => {
            let n: usize = req.query.split('&').find_map(|kv| kv.strip_prefix("n=")).and_then(|v| v.parse().ok()).unwrap_or(30);
            let h = app.ws()?.state.history();
            let tail = h[h.len().saturating_sub(n)..].to_vec();
            send_json(stream, head, 200, &Value::Array(tail)).map_err(io)
        }
        ("GET" | "HEAD", p) if p.starts_with("/files/") => match app.served_file(&p["/files/".len()..]) {
            None => send_json(stream, head, 404, &json!({"error": "not found"})).map_err(io),
            Some(file) => {
                let body = std::fs::read(&file)?;
                // takes live under their fingerprint and are never overwritten, so they cache forever
                let cache = if p.starts_with("/files/render/") { "public, max-age=31536000, immutable" } else { "no-cache" };
                send(stream, head, 200, mime(&file), &body, &[("Cache-Control", cache.into())]).map_err(io)
            }
        },
        ("POST", p) if p.starts_with("/api/shots/") => {
            let body = req.json()?;
            let assignments: Vec<(String, Value)> = match body.get("set") {
                Some(Value::Object(m)) if !m.is_empty() => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                _ => bail!("expected {{\"set\": {{\"field.path\": value, ...}}}}"),
            };
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let sid = ws.select_one(&percent_decode(&p["/api/shots/".len()..]))?.id;
            let rep = ws.set_fields(&sid, &assignments)?;
            send_json(stream, false, 200, &rep.to_json()).map_err(io)
        }
        ("POST", "/api/approve") => {
            let body = req.json()?;
            let shots: Vec<String> = match body.get("shots") {
                Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
                _ => bail!("expected {{\"shots\": [\"s01\", ...], \"approved\": true}}"),
            };
            let approved = body.get("approved").and_then(Value::as_bool).unwrap_or(true);
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let mut done = Vec::new();
            for s in ws.select(&shots)? {
                ws.approve(&s.id, approved)?;
                done.push(s.id);
            }
            send_json(stream, false, 200, &json!({"shots": done, "approved": approved})).map_err(io)
        }
        ("POST", "/api/brief") => {
            let body = req.json()?;
            let entries: Vec<(String, Value)> = match body.get("set") {
                Some(Value::Object(m)) if !m.is_empty() => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                _ => bail!("expected {{\"set\": {{\"genre\": \"...\", ...}}}}"),
            };
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            ws.set_brief(&entries)?;
            send_json(stream, false, 200, &brief_json(&ws.defn.brief)).map_err(io)
        }
        ("POST", "/api/order") => {
            let body = req.json()?;
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let ids = ws.defn.shot_ids();
            let order: Vec<String> = body
                .get("order")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let (mut a, mut b) = (order.clone(), ids.clone());
            a.sort();
            b.sort();
            if a != b {
                bail!("order must list every shot exactly once: {}", ids.join(", "));
            }
            let rep = ws.edit_definition(&[], Some(&order), "order", "")?;
            send_json(stream, false, 200, &rep.to_json()).map_err(io)
        }
        ("POST", "/api/cut") => {
            let body = req.json()?;
            let quality = body.get("quality").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("best").to_string();
            let video = body.get("video").and_then(Value::as_bool).unwrap_or(true);
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let info = ws.cut(&quality, video)?;
            let out = json!({
                "dir": info.dir.display().to_string(), "seconds": info.seconds, "shots": info.shots,
                "video": info.video.as_ref().map(|p| p.display().to_string()),
                "storyboard": file_url(&ws.root, &info.dir.join("index.html")),
                "video_url": info.video.as_ref().and_then(|p| file_url(&ws.root, p)),
                "audio_url": info.audio.as_ref().and_then(|p| file_url(&ws.root, p)),
                "sound": info.check.as_ref().map(|c| json!({"verdict": c["verdict"], "verdict_ja": c["verdict_ja"]})),
                "notes": info.notes,
            });
            send_json(stream, false, 200, &out).map_err(io)
        }
        ("POST", "/api/open") => {
            let body = req.json()?;
            let url = body.get("url").and_then(Value::as_str).unwrap_or("");
            let Some(file) = url.strip_prefix("/files/").and_then(|r| app.served_file(r)) else {
                bail!("can only open files under render/, cut/ or audio/");
            };
            open_with_os(&file)?;
            send_json(stream, false, 200, &json!({"opened": file.display().to_string()})).map_err(io)
        }
        ("POST", "/api/render") => render_stream(stream, app, req),
        ("GET" | "HEAD", "/api/sound") => send_json(stream, head, 200, &sound_payload(&app.ws()?)?).map_err(io),
        ("POST", "/api/sound") => {
            let body = req.json()?;
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let mut sfx = Vec::new();
            if let Some(m) = body.get("sfx") {
                let Some(m) = m.as_object() else { bail!("sfx: {{\"<shot id>\": [ ... ] | null}}") };
                for (id, list) in m {
                    let sid = ws.select_one(id)?.id;
                    sfx.push((sid, Some(list.clone())));
                }
            }
            let audio = match body.get("audio") {
                None => None,
                Some(Value::Object(a)) => Some(a.clone()),
                Some(Value::Null) => Some(Map::new()),
                Some(_) => bail!("audio: a mapping like {{\"bgm\": {{\"file\": \"audio/bgm.wav\"}}}}"),
            };
            let note = body.get("note").and_then(Value::as_str).unwrap_or("");
            let rep = ws.edit_sound(&sfx, audio.as_ref(), note)?;
            let mut out = rep.to_json();
            out["sound"] = sound_payload(&ws)?;
            send_json(stream, false, 200, &out).map_err(io)
        }
        ("POST", "/api/audio-folder") => {
            let _guard = app.mutate()?;
            let dir = app.root().join("audio");
            std::fs::create_dir_all(&dir)?;
            open_with_os(&dir)?;
            send_json(stream, false, 200, &json!({"opened": dir.display().to_string()})).map_err(io)
        }
        ("GET" | "HEAD", "/api/settings") => send_json(stream, head, 200, &app.ai_status()).map_err(io),
        ("POST", "/api/settings") => {
            let body = req.json()?;
            let Some(path) = app.settings_path.clone() else {
                bail!("settings cannot be saved in this session");
            };
            let mut s = app.settings();
            if let Some(p) = body.get("provider").and_then(Value::as_str) {
                s.provider = Provider::parse(p)?;
            }
            if let Some(e) = body.get("effort").and_then(Value::as_str) {
                let e = e.trim();
                if !e.is_empty() && !ai::valid_effort(e) {
                    bail!("effort must be one of {} (got '{e}')", ai::EFFORTS.join(", "));
                }
                s.effort = (!e.is_empty()).then(|| e.to_string()); // empty = back to the default
            }
            if let Some(p) = body.get("image_provider").and_then(Value::as_str) {
                if !connect::IMAGE_PROVIDERS.contains(&p) {
                    bail!("image_provider must be one of {}", connect::IMAGE_PROVIDERS.join(", "));
                }
                s.image_provider = p.to_string();
            }
            if let Some(k) = body.get("openai_api_key").and_then(Value::as_str) {
                let k = k.trim();
                if !k.is_empty() && !k.starts_with("sk-") {
                    bail!("that does not look like an OpenAI API key (they start with sk-)");
                }
                s.openai_api_key = (!k.is_empty()).then(|| k.to_string());
            }
            if let Some(k) = body.get("api_key").and_then(Value::as_str) {
                let k = k.trim();
                if !k.is_empty() && !k.starts_with("sk-") {
                    bail!("that does not look like an Anthropic API key (they start with sk-)");
                }
                s.api_key = (!k.is_empty()).then(|| k.to_string());
            }
            s.save(&path)?;
            send_json(stream, false, 200, &app.ai_status()).map_err(io)
        }
        ("GET" | "HEAD", "/api/assets") => {
            let ws = app.ws()?;
            let list: Vec<Value> = crate::scout::load_assets(&ws.root)
                .iter()
                .map(|a| {
                    let mut v = a.to_json();
                    v["suggested"] = json!(a.suggested_name());
                    v
                })
                .collect();
            send_json(stream, head, 200, &json!({"assets": list})).map_err(io)
        }
        ("POST", "/api/assets/scan") => {
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let list = ws.scan_assets()?;
            send_json(stream, false, 200, &json!({"count": list.len()})).map_err(io)
        }
        ("POST", "/api/props") => {
            let body = req.json()?;
            let name = body.get("name").and_then(Value::as_str).unwrap_or("").trim().to_string();
            let asset = body.get("asset").and_then(Value::as_str).unwrap_or("").trim().to_string();
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            ws.add_prop(&name, &asset, None, None)?;
            send_json(stream, false, 200, &json!({"added": name})).map_err(io)
        }
        ("POST", "/api/props/remove") => {
            let body = req.json()?;
            let name = body.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            ws.remove_prop(&name)?;
            send_json(stream, false, 200, &json!({"removed": name})).map_err(io)
        }
        ("POST", "/api/stage") => {
            let claude = (app.claude)(&app.settings())?;
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let st = ai::understand_stage(&ws.defn, claude.as_ref())?;
            ws.set_stage(&st)?;
            send_json(stream, false, 200, &json!({"summary": st.summary, "anchors": st.anchors.len()})).map_err(io)
        }
        ("POST", "/api/scout") => {
            let body = req.json()?;
            let only: Vec<String> = body
                .get("anchors")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let idx = ws.take_scout(&only)?;
            send_json(stream, false, 200, &idx).map_err(io)
        }
        ("POST", "/api/moodboard/generate") => {
            let body = req.json()?;
            let direction = body.get("direction").and_then(Value::as_str).unwrap_or("").to_string();
            let n = body.get("n").and_then(Value::as_u64).unwrap_or(4).clamp(1, 4) as usize;
            let gen = (app.images)(&app.settings())?; // say what is missing before taking the lock
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let made = ws.generate_moodboard(gen.as_ref(), &direction, n)?;
            send_json(stream, false, 200, &json!({"added": made, "images": ws.defn.brief.images})).map_err(io)
        }
        ("POST", "/api/moodboard/upload") => {
            let body = req.json()?;
            let name = body.get("name").and_then(Value::as_str).unwrap_or("image");
            let data = body.get("data").and_then(Value::as_str).unwrap_or("");
            let data = data.split_once("base64,").map(|(_, d)| d).unwrap_or(data); // a data: URL is fine too
            let bytes = crate::moodboard::base64_decode(data).ok_or_else(|| Error::user("the image data is not base64"))?;
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let rel = ws.add_moodboard_image(name, &bytes)?;
            send_json(stream, false, 200, &json!({"added": [rel], "images": ws.defn.brief.images})).map_err(io)
        }
        ("POST", "/api/moodboard/remove") => {
            let body = req.json()?;
            let rel = body.get("image").and_then(Value::as_str).unwrap_or("");
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            ws.remove_moodboard_image(rel)?;
            send_json(stream, false, 200, &json!({"images": ws.defn.brief.images})).map_err(io)
        }
        ("POST", "/api/plan") => {
            let out = plan(app, &req.json()?)?;
            send_json(stream, false, 200, &out).map_err(io)
        }
        ("POST", "/api/revise") => {
            let out = revise(app, &req.json()?)?;
            send_json(stream, false, 200, &out).map_err(io)
        }
        _ => {
            send_json(stream, head, 404, &json!({"error": format!("no route for {} {}", req.method, req.path)})).map_err(io)
        }
    }
}

/// The structure stage: Claude writes the whole shot list from the brief, the skeleton and
/// the craft rules (one pass, no self-review; D-22). Code normalizes it, writes it, and the
/// shot check result goes to the human. Replacing existing shots needs `replace: true`.
fn plan(app: &App, body: &Map<String, Value>) -> Result<Value> {
    let note = body.get("note").and_then(Value::as_str).map(str::trim).unwrap_or("");
    let replace = body.get("replace").and_then(Value::as_bool).unwrap_or(false);
    let duration = body.get("duration").and_then(Value::as_f64);
    let n_shots = body.get("shots").and_then(Value::as_u64).map(|n| n as usize);
    let _guard = app.mutate()?;
    let mut ws = app.ws()?;
    if !ws.defn.shots.is_empty() && !replace {
        bail!("shots.yaml already has {} shots; replacing them needs confirmation", ws.defn.shots.len());
    }
    let claude = (app.claude)(&app.settings())?;
    let p = ai::plan(&ws.defn, None, duration, n_shots, Some(note).filter(|n| !n.is_empty()), claude.as_ref())?;
    ws.replace_all_shots(&p.shots)?;
    let found = ws.check()?;
    let count = |sev: &str| found.iter().filter(|f| f.severity.as_str() == sev).count();
    let checks = json!({"P0": count("P0"), "P1": count("P1"), "P2": count("P2")});
    let mut extra = Map::new();
    extra.insert("note".into(), json!(note));
    extra.insert("rationale".into(), json!(p.rationale));
    extra.insert("adjustments".into(), json!(p.adjustments));
    extra.insert("checks".into(), checks.clone());
    ws.state.annotate_last(extra);
    ws.state.save()?;
    Ok(json!({
        "shots": p.shots.iter().map(|s| &s["id"]).collect::<Vec<_>>(), "rationale": p.rationale,
        "adjustments": p.adjustments, "checks": checks,
    }))
}

/// A note from the app's chat: Claude edits the pointed-at shot (or, with no shot, the
/// whole list) and the result goes through the same validated edit path as the CLI.
fn revise(app: &App, body: &Map<String, Value>) -> Result<Value> {
    let note = body.get("note").and_then(Value::as_str).map(str::trim).unwrap_or("");
    if note.is_empty() {
        bail!("write what to change");
    }
    let _guard = app.mutate()?;
    let mut ws = app.ws()?;
    let scope = match body.get("shot").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        Some(s) => Some(vec![ws.select_one(s)?.id]),
        None => None,
    };
    let mut context = Vec::new();
    let mut pointer = ai::Pointer { sound: body.get("sound").and_then(Value::as_bool).unwrap_or(false), ..Default::default() };
    if let Some(s) = &scope {
        context.push(format!("shot {}", s[0]));
    }
    if pointer.sound {
        context.push("working on the sound (background music and sound effects)".into());
        if let Some(i) = body.get("sfx_index").and_then(Value::as_u64) {
            context.push(format!("sound effect #{} of that shot's sfx list", i + 1));
        }
        if let Some(t) = body.get("time").and_then(Value::as_f64) {
            context.push(format!("{t:.2} s into the teaser"));
        }
        // what the sound check says, so "fix what the check found" works
        if let Ok(Some(m)) = ws.mix() {
            let c = &m.check;
            let acts: Vec<String> = c["actions"]
                .as_array()
                .into_iter()
                .flatten()
                .take(5)
                .map(|a| format!("[{}] {}", a["weight"].as_str().unwrap_or(""), a["action"].as_str().unwrap_or("")))
                .collect();
            context.push(format!("sound check against 7 launch films: {}; to fix: {}",
                c["summary"].as_str().unwrap_or(""), if acts.is_empty() { "nothing".into() } else { acts.join(" / ") }));
        }
    }
    if let Some(f) = body.get("frame").and_then(Value::as_object) {
        if let (Some(i), Some(n)) = (f.get("index").and_then(Value::as_u64), f.get("count").and_then(Value::as_u64)) {
            let t = f.get("time").and_then(Value::as_f64).map(|t| format!(", {t:.2} s into the shot")).unwrap_or_default();
            context.push(format!("frame {} of {n}{t}", i + 1));
        }
        let url = f.get("url").and_then(Value::as_str).unwrap_or("");
        if let Some(file) = url.strip_prefix("/files/").filter(|r| r.starts_with("render/")).and_then(|r| app.served_file(r)) {
            let bytes = std::fs::read(&file).map_err(|e| Error::user(format!("cannot read {}: {e}", file.display())))?;
            if bytes.len() <= ai::MAX_IMAGE_BYTES {
                pointer.image = Some((mime(&file).to_string(), bytes));
            }
        }
    }
    if let Some(r) = body.get("region").and_then(Value::as_array) {
        let v: Vec<f64> = r.iter().filter_map(Value::as_f64).map(|x| (x.clamp(0.0, 1.0) * 100.0).round()).collect();
        if v.len() == 4 {
            context.push(format!("the marked region spans x {}-{}% and y {}-{}% of the frame", v[0], v[2], v[1], v[3]));
        }
    }
    if !context.is_empty() {
        pointer.context = Some(context.join("; "));
    }
    let claude = (app.claude)(&app.settings())?;
    let rev = ai::revise_at(&ws.defn, note, scope.as_deref(), &pointer, claude.as_ref())?;
    let mut changes = Map::new();
    for (id, new) in &rev.edits {
        let old = ws.defn.find(id).cloned().unwrap_or(Value::Null);
        changes.insert(id.clone(), Value::Array(field_changes(&old, new)));
    }
    if let Some(a) = &rev.audio {
        changes.insert("audio".into(), Value::Array(field_changes(&Value::Object(ws.defn.audio.clone()), &Value::Object(a.clone()))));
    }
    let rep = ws.edit_definition_with(&rev.edits, rev.order.as_deref(), rev.audio.as_ref(), "revise", note)?;
    let mut extra = Map::new();
    extra.insert("summary".into(), json!(rev.summary));
    extra.insert("changes".into(), Value::Object(changes.clone()));
    extra.insert("pointer".into(), json!(pointer.context));
    ws.state.annotate_last(extra);
    ws.state.save()?;
    let mut out = rep.to_json();
    out["summary"] = json!(rev.summary);
    out["changes"] = Value::Object(changes);
    Ok(out)
}

fn render_stream(stream: &mut TcpStream, app: &App, req: &Request) -> Result<()> {
    let body = req.json()?;
    let quality = body.get("quality").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("previz").to_string();
    let ids: Vec<String> = body
        .get("shots")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let force = body.get("force").and_then(Value::as_bool).unwrap_or(false);
    let _guard = app.mutate()?;
    let mut ws = app.ws()?;
    fp::quality(&ws.defn, &quality)?; // bad input -> plain 400 before streaming starts
    ws.select(&ids)?;

    let head = "HTTP/1.1 200 OK\r\nServer: shotpilot\r\nConnection: close\r\n\
                Content-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-cache, no-transform\r\n\
                X-Accel-Buffering: no\r\n\r\n";
    // the render must finish even if the window goes away, so write errors only stop the stream
    let mut alive = stream.write_all(head.as_bytes()).is_ok();
    let root = ws.root.clone();
    let mut counts = [0usize; 3]; // rendered, reused, failed
    let mut send_event = |event: &str, data: &Value, alive: &mut bool| {
        if *alive {
            let msg = format!("event: {event}\ndata: {data}\n\n");
            *alive = stream.write_all(msg.as_bytes()).and_then(|_| stream.flush()).is_ok();
        }
    };
    let mut on_progress = |event: &str, data: &Value| {
        let mut data = data.clone();
        if event == "shot" {
            match data["action"].as_str() {
                Some("rendered") => counts[0] += 1,
                Some("reused") => counts[1] += 1,
                _ => counts[2] += 1,
            }
            let thumb = data["thumb_path"].as_str().and_then(|p| file_url(&root, Path::new(p)));
            if let Some(m) = data.as_object_mut() {
                m.remove("thumb_path");
            }
            data["thumb"] = json!(thumb);
        }
        send_event(event, &data, &mut alive);
    };
    let result = ws.render(&ids, &quality, force, Some(&mut on_progress));
    match result {
        Ok(_) => {
            let done = json!({"quality": quality, "rendered": counts[0], "reused": counts[1], "failed": counts[2]});
            send_event("done", &done, &mut alive);
        }
        Err(e) => send_event("error", &json!({"error": e.msg}), &mut alive),
    }
    Ok(())
}

/// Open a file with the OS default application (video player, browser for HTML).
pub fn open_with_os(path: &Path) -> Result<()> {
    let r = if cfg!(windows) {
        std::process::Command::new("explorer").arg(path).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(path).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(path).spawn()
    };
    r.map(|_| ()).map_err(|e| Error::user(format!("could not open {}: {e}", path.display())))
}
