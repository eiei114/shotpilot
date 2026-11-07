//! The app's local backend: a small HTTP + Server-Sent Events layer over Workspace.
//!
//! The desktop app (crates/shotpilot-app) opens one window on this server. It binds to
//! loopback, answers only loopback Host/Origin headers, and serves files only from the
//! project's render/, cut/ and audio/ folders (and, for the home's cards, the render/
//! folders of the projects in the recent list).
//!
//! The server starts on the home (`/home`) or on a project. One project is open at a
//! time; the home switches it, and `/` is the studio for whichever is open.
//!
//! shots.yaml and .shotpilot/state.json stay the source of truth: every request opens a
//! fresh Workspace, so edits made from the CLI or a text editor show up immediately.
//! Mutations are serialized with one lock; a second mutation while one runs gets 409.
//! Status answers carry an ETag built from the two files' mtimes, so the window's
//! polling costs a stat() and a 304 while nothing changes.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::UNIX_EPOCH;

use serde_json::{json, Map, Value};

use crate::ai::{self, Claude};
use crate::connect::{self, Provider, Settings};
use crate::bail;
use crate::error::{Error, Kind, Result};
use crate::fingerprint as fp;
use crate::library::{self, Library};
use crate::projects::{self, NewProject, Recents};
use crate::sound;
pub use crate::workspace::field_changes;
use crate::workspace::Workspace;

pub const INDEX_HTML: &str = include_str!("../web/index.html");
pub const HOME_HTML: &str = include_str!("../web/home.html");
/// The English mode (web/i18n.js) and its dictionary: the pages switch to English on a PC whose
/// language is not Japanese.
pub const I18N_JS: &str = include_str!("../web/i18n.js");
pub const I18N_EN: &str = include_str!("../web/i18n-en.json");
/// For each dictionary key, the holes that hold the user's or Claude's own words.
pub const I18N_FREE: &str = include_str!("../web/i18n-free.json");

/// The English mode with its dictionary in front.
pub fn i18n_js() -> String {
    format!("const SP_EN = {};\nconst SP_FREE = {};\n{I18N_JS}", I18N_EN.trim(), I18N_FREE.trim())
}
const LOOPBACK: &[&str] = &["127.0.0.1", "localhost", "::1"];
const SERVED_DIRS: &[&str] = &["render", "cut", "audio", "moodboard", "scout", "storyboard", "concept", "promo", "reference"];
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

/// What the home asks the native side to pick.
#[derive(Debug, Clone)]
pub enum Pick {
    /// A folder (to open a project, or to put a new one in).
    Folder { title: String, start: Option<PathBuf> },
    /// A .uproject file.
    Uproject { start: Option<PathBuf> },
    /// An image (the game's logo for the title).
    Image { start: Option<PathBuf> },
    /// A video (a reference teaser to measure, D-71).
    Video { start: Option<PathBuf> },
}

/// Shows a native file dialog. Only the desktop app has one; without it the home asks
/// for paths as text.
pub type Picker = Box<dyn Fn(Pick) -> Option<PathBuf> + Send + Sync>;

pub struct App {
    /// The open project's shots.yaml; None while the home has nothing open.
    current: RwLock<Option<PathBuf>>,
    lock: Mutex<()>,
    claude: ClaudeFactory,
    images: ImageFactory,
    /// Where the connection settings live; None = read-only defaults.
    settings_path: Option<PathBuf>,
    /// The global sound library (D-26); its `sounds/` folder sits under this.
    library_root: Option<PathBuf>,
    /// Projects opened before, for the home.
    recents: Recents,
    picker: Option<Picker>,
    /// The content files the takes recorded (content.rs), keyed by the state.json stamp
    /// they were read at: the status ETag watches them so a change to the game shows up.
    watched: Mutex<(String, Vec<PathBuf>)>,
}

impl App {
    pub fn new(shots_path: Option<&Path>, claude: ClaudeFactory, settings_path: Option<PathBuf>) -> Result<App> {
        let current = match shots_path {
            Some(p) => Some(Workspace::open(p)?.path), // fail fast on a broken definition
            None => None,
        };
        Ok(App { current: RwLock::new(current), lock: Mutex::new(()), claude, images: Box::new(connect::make_image_gen),
            settings_path, library_root: library::default_root(), recents: Recents::new(None), picker: None,
            watched: Mutex::new((String::new(), Vec::new())) })
    }

    /// The open project's shots.yaml.
    pub fn current(&self) -> Option<PathBuf> {
        self.current.read().map(|g| g.clone()).unwrap_or_else(|p| p.into_inner().clone())
    }

    /// Open a project (a shots.yaml, or a folder with one) and put it at the top of the
    /// recent list. A running operation keeps it from switching (409).
    pub fn open(&self, path: &Path) -> Result<PathBuf> {
        let shots = if path.is_dir() {
            match projects::find_projects(path).as_slice() {
                [one] => one.clone(),
                [] => bail!("{} に shots.yaml がありません", path.display()),
                _ => bail!("{} には shots.yaml が複数あります。どれかを選んでください", path.display()),
            }
        } else {
            path.to_path_buf()
        };
        let ws = Workspace::open(&shots)?;
        let _guard = self.mutate()?;
        *self.current.write().unwrap_or_else(|p| p.into_inner()) = Some(ws.path.clone());
        let _ = self.recents.touch(&ws.path); // a read-only config folder must not block opening
        Ok(ws.path)
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

    fn library(&self) -> Result<Library> {
        Library::open(self.library_root.as_deref())
    }

    fn ws(&self) -> Result<Workspace> {
        match self.current() {
            Some(p) => Workspace::open(p),
            None => bail!("プロジェクトが開かれていません。ホームから開いてください"),
        }
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
        self.current().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default()
    }

    /// A cover frame for a home card: an image under the render/ folder of the open
    /// project or of one in the recent list.
    fn cover_file(&self, path: &str) -> Option<PathBuf> {
        let p = std::fs::canonicalize(percent_decode(path)).ok()?;
        if !matches!(mime(&p), "image/png" | "image/jpeg" | "image/bmp") {
            return None;
        }
        let roots = self.recents.list().into_iter().map(|r| r.path).chain(self.current());
        for shots in roots {
            let render = shots.parent().map(|r| r.join("render")).and_then(|r| std::fs::canonicalize(r).ok());
            if render.is_some_and(|r| p.starts_with(r)) {
                return p.is_file().then_some(p);
            }
        }
        None
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
        Server::bind_opt(Some(shots_path), host, port, claude, settings_path)
    }

    /// Start on the home with no project open; the recent list is the user's.
    pub fn bind_home(host: &str, port: u16) -> Result<Server> {
        Ok(Server::bind_opt(None, host, port, default_claude(), Settings::default_path())?.with_recents(projects::default_path()))
    }

    pub fn bind_opt(shots_path: Option<&Path>, host: &str, port: u16, claude: ClaudeFactory, settings_path: Option<PathBuf>)
        -> Result<Server> {
        let app = Arc::new(App::new(shots_path, claude, settings_path)?);
        let listener = TcpListener::bind((host, port))
            .map_err(|e| Error::user(format!("cannot listen on {host}:{port}: {e}")))?;
        Ok(Server { listener, app, bind_host: host.to_ascii_lowercase() })
    }

    /// Keep the recent-projects list in this file (None: remember nothing).
    pub fn with_recents(mut self, file: Option<PathBuf>) -> Server {
        if let Some(app) = Arc::get_mut(&mut self.app) {
            app.recents = Recents::new(file);
        }
        self
    }

    /// Native file dialogs for the home (the desktop app).
    pub fn with_picker(mut self, picker: Picker) -> Server {
        if let Some(app) = Arc::get_mut(&mut self.app) {
            app.picker = Some(picker);
        }
        self
    }

    /// Open a project before the window shows (a path given on the command line).
    pub fn open(&self, path: &Path) -> Result<PathBuf> {
        self.app.open(path)
    }

    /// Use another image model (tests).
    pub fn with_images(mut self, images: ImageFactory) -> Server {
        if let Some(app) = Arc::get_mut(&mut self.app) {
            app.images = images;
        }
        self
    }

    /// Use another sound library folder (tests, or a library kept elsewhere).
    pub fn with_library(mut self, root: Option<PathBuf>) -> Server {
        if let Some(app) = Arc::get_mut(&mut self.app) {
            app.library_root = root;
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
        206 => "Partial Content",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Payload Too Large",
        416 => "Range Not Satisfiable",
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

/// A file from disk, streamed (videos and mixes are tens of MB) and with byte ranges, so
/// the window's video and audio players can seek without fetching the whole file again.
fn send_file(stream: &mut TcpStream, req: &Request, path: &Path, cache: &str) -> std::io::Result<()> {
    let head_only = req.method == "HEAD";
    let mut file = std::fs::File::open(path)?;
    let meta = file.metadata()?;
    let len = meta.len();
    let stamp = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0);
    let tag = format!("\"{stamp:x}-{len:x}\"");
    let mut extra: Vec<(&str, String)> =
        vec![("Cache-Control", cache.into()), ("Accept-Ranges", "bytes".into()), ("ETag", tag.clone())];
    if req.header("if-none-match") == Some(tag.as_str()) {
        return send(stream, true, 304, "", b"", &extra);
    }
    // one range only ("bytes=a-b", "bytes=a-", "bytes=-n"); anything else gets the whole file
    let range = req.header("range").and_then(|r| r.strip_prefix("bytes=")).filter(|r| !r.contains(',')).and_then(|r| {
        let (a, b) = r.split_once('-')?;
        let (a, b) = (a.trim(), b.trim());
        match (a.parse::<u64>().ok(), b.parse::<u64>().ok()) {
            (Some(a), Some(b)) => Some((a, b.min(len.saturating_sub(1)))),
            (Some(a), None) if b.is_empty() => Some((a, len.saturating_sub(1))),
            (None, Some(n)) if a.is_empty() => Some((len.saturating_sub(n), len.saturating_sub(1))),
            _ => None,
        }
    });
    let (code, start, count) = match range {
        Some((a, b)) if a < len && a <= b => {
            extra.push(("Content-Range", format!("bytes {a}-{b}/{len}")));
            (206, a, b - a + 1)
        }
        Some(_) => {
            extra.push(("Content-Range", format!("bytes */{len}")));
            return send(stream, head_only, 416, "text/plain", b"", &extra);
        }
        None => (200, 0, len),
    };
    let mut head = format!("HTTP/1.1 {code} {}\r\nServer: shotpilot\r\nConnection: close\r\n", status_text(code));
    head.push_str(&format!("Content-Type: {}\r\nContent-Length: {count}\r\n", mime(path)));
    for (k, v) in &extra {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if !head_only {
        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(start))?;
        std::io::copy(&mut file.take(count), stream)?;
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

/// Encode a value for one URL path segment (library ids hold ':').
fn percent_encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
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
        Some("aif" | "aiff") => "audio/aiff",
        Some("m4a" | "aac") => "audio/mp4",
        Some("html") => "text/html; charset=utf-8",
        Some("md" | "txt") => "text/plain; charset=utf-8",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

// --------------------------------------------------------------------------- payloads

/// The reference teaser the brief names (D-71): what was measured, and the latest comparison
/// with the cut, for the design panel.
fn reference_json(ws: &Workspace) -> Value {
    let Some(r) = crate::reference::active(&ws.defn) else { return Value::Null };
    let dir = crate::reference::dir(&ws.root, &r.id);
    let compare = std::fs::read_to_string(dir.join("compare").join("compare.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .map(|c| {
            let pairs: Vec<Value> = c["pairs"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|p| {
                    let url = file_url(&ws.root, &dir.join("compare").join(p["file"].as_str().unwrap_or("")));
                    json!({"label": p["label"], "at_ref": p["at_ref"], "at_cut": p["at_cut"], "url": url})
                })
                .collect();
            json!({"cut": crate::reference::Measure::from_json(&c["measure"]["cut"]).summary(), "pairs": pairs, "compared": c["compared"]})
        });
    json!({
        "id": r.id, "source": r.source, "summary": r.measure.summary(),
        "contact": file_url(&ws.root, &dir.join("contact.jpg")).filter(|_| dir.join("contact.jpg").is_file()),
        "borrowed": crate::reference::motion_entries(&r.measure).0,
        "compare": compare,
    })
}

fn file_url(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    Some(format!("/files/{}", parts.join("/")))
}

/// What the blocking map draws (D-77), seen from above: the anchors, the game's shapes (three
/// only: its objects' boxes at t = 0), and every shot's camera resolved to where it stands, where
/// it ends, where it looks and how wide it sees. Read-only; the page writes through /api/shots/.
pub fn blocking_payload(ws: &Workspace) -> Result<Value> {
    let defn = &ws.defn;
    let anchors: Vec<Value> = defn
        .anchors
        .iter()
        .map(|(name, at)| {
            let b = defn.known_box(name);
            json!({"name": name, "at": at, "center": b.map(|b| b.center), "size": b.map(|b| b.size)})
        })
        .collect();
    let mut shapes = Vec::new();
    if defn.project.engine == "three" {
        // a missing or broken game leaves the map with the anchors only (the render says why)
        if let Ok(scene) = crate::three::Scene::load(&crate::three::game_dir(defn)) {
            for o in &scene.objects {
                for (b, solid) in o.boxes_at(0.0) {
                    shapes.push(json!({"name": o.name, "label": o.label, "center": b.center, "half": b.half, "yaw": b.yaw,
                                       "solid": solid, "structure": o.structure}));
                }
            }
        }
    }
    // a shot that does not resolve is left off the map (the status shows its error)
    let shots: Vec<Value> = defn
        .shots
        .iter()
        .enumerate()
        .filter_map(|(i, node)| {
            let r = crate::model::resolve_shot(defn, node, i).ok()?;
            let cam = node.get("camera").cloned().unwrap_or_else(|| json!({}));
            Some(json!({
                "id": r.id, "index": i, "beat": r.beat, "duration": r.duration, "curve": r.curve,
                "form": if cam.get("anchor").is_some() { "anchor" } else { "location" },
                "anchor": cam.get("anchor"), "offset": cam.get("offset"), "look_at": cam.get("look_at"),
                "rotation": cam.get("rotation"), "end": cam.get("end"), "move": cam.get("move"),
                "start": {"location": r.start.location, "rotation": r.start.rotation.as_array()},
                "finish": {"location": r.end.location, "rotation": r.end.rotation.as_array()},
                "target": r.target, "lens": r.focal_length(),
                "hfov": crate::geometry::hfov_from_focal(r.focal_length(), r.sensor_width()),
            }))
        })
        .collect();
    Ok(json!({"engine": defn.project.engine, "anchors": anchors, "shapes": shapes, "shots": shots}))
}

pub fn status_payload(ws: &Workspace) -> Result<Value> {
    let qualities = fp::quality_names(&ws.defn);
    let resolved = ws.resolved()?;
    let mut shots = Vec::new();
    for ((row, takes), s) in ws.status_detail(&resolved, true)?.into_iter().zip(&resolved) {
        let mut thumbs = Map::new();
        for qt in takes {
            let (q, rfp, is_fresh, take) = (qt.quality, qt.fingerprint, qt.fresh, qt.take);
            let url = take.as_ref().and_then(|t| t.thumb()).and_then(|p| file_url(&ws.root, &p));
            let frames: Vec<String> = take
                .as_ref()
                .map(|t| t.files().iter().filter_map(|f| file_url(&ws.root, &t.dir.join(f))).collect())
                .unwrap_or_default();
            let numbers = take.as_ref().map(|t| t.frames()).unwrap_or_default();
            thumbs.insert(q, json!({
                "url": url, "fresh": is_fresh, "fingerprint": &rfp[..12], "frames": frames, "frame_numbers": numbers,
            }));
        }
        let mut v = row.to_json();
        v["lens"] = s.lens.get("focal_length").cloned().unwrap_or(Value::Null);
        v["thumbs"] = Value::Object(thumbs);
        v["storyboard"] = match crate::storyboard::current(&ws.defn, s) {
            Some(b) => json!({"url": file_url(&ws.root, &b.file), "fresh": b.fresh}),
            None => Value::Null,
        };
        v["node"] = ws.defn.shots[row.index].clone();
        v["framing"] = crate::shotcheck::framing(&ws.defn, s).map(|f| f.to_json()).unwrap_or(Value::Null);
        shots.push(v);
    }
    let timeline = timeline_json(&ws.defn, &resolved);
    let total: f64 = timeline.last().map(|it| it["start"].as_f64().unwrap_or(0.0) + it["duration"].as_f64().unwrap_or(0.0)).unwrap_or(0.0);
    // the shot check once: the warnings are its P0 and P1 lines (Workspace::structure_warnings)
    let checks = ws.check_resolved(&resolved)?;
    let warnings: Vec<String> =
        checks.iter().filter(|f| f.severity <= crate::shotcheck::Severity::P1).map(crate::shotcheck::Finding::line).collect();
    let scout = crate::scout::load_index(&ws.root);
    Ok(json!({
        "project": ws.defn.project.name,
        "engine": ws.defn.project.engine,
        "title": ws.defn.teaser.title,
        "path": ws.path.display().to_string(),
        "target_duration": ws.defn.teaser.target_duration,
        "total_duration": crate::geometry::round_to(total, 3),
        "qualities": qualities,
        "warnings": warnings,
        "checks": checks.iter().map(|f| f.to_json()).collect::<Vec<_>>(),
        "timeline": timeline,
        "script": ws.defn.script.iter().map(line_json).collect::<Vec<_>>(),
        "undo": ws.undo_target().map(|(_, e)| undo_json(&e)),
        "script_approval": ws.script_approval(),
        "review": review_json(ws),
        "brief": brief_json(&ws.defn.brief),
        "own_design": crate::designsys::project_doc(&ws.root).is_some(),
        "concept": crate::concept::load(&ws.root).iter().map(crate::concept::Frame::to_json).collect::<Vec<_>>(),
        "scout": scout["anchors"].as_object().cloned().unwrap_or_default(), // scout::summary
        "scout_taken": scout["taken"],
        "finish": {
            "letterbox": ws.defn.finish.letterbox,
            "grade": ws.defn.finish.grade,
            "typeface": ws.defn.finish.typeface,
            "title": ws.defn.finish.title.as_ref().map(|t| json!({"text": t.text, "at": t.at, "duration": t.duration})),
            "end_card": ws.defn.finish.end_card.as_ref().map(|(l, d)| json!({"lines": l, "duration": d})),
            "font": crate::finish::font(&ws.defn.finish).map(|p| p.display().to_string()),
            "fonts": crate::finish::describe(&ws.defn.finish, &[])["fonts"],
            "title_size": ws.defn.finish.title_size,
            "logo": ws.defn.finish.logo,
            "motion": motion_json(&ws.defn.finish.motion),
            "motion_cfg": ws.defn.finish.motion.to_value(),
        },
        "reference": reference_json(ws),
        "stage": {
            "summary": ws.defn.stage.summary,
            "anchors": ws.defn.stage.anchors.iter().map(|(n, d)| json!({"name": n, "description": d})).collect::<Vec<_>>(),
            "mood": ws.defn.stage.mood.iter().map(|(f, a)| (f.clone(), json!(a))).collect::<Map<String, Value>>(),
        },
        "props": ws.defn.props.iter().map(|(n, p)| json!({"name": n, "asset": p.asset, "kind": p.kind, "size": p.size})).collect::<Vec<_>>(),
        "assets_count": crate::scout::load_assets(&ws.root).len(),
        "pitch": ws.defn.teaser.pitch,
        "fps": ws.defn.teaser.fps,
        "vocab": vocab_json(),
        "shots": shots,
        "audio": ws.defn.audio,
        "cut": latest_cut(&ws.root),
        "promo": crate::promo::summary(ws, crate::promo::today()),
    }))
}

/// The closed vocabularies the page offers as choices (one copy, here): it only adds Japanese names.
fn vocab_json() -> Value {
    json!({
        "time_of_day": crate::model::TIME_OF_DAY.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
        "moves": crate::model::MOVES,
        "aims": crate::model::AIMS,
        "transitions": crate::finish::TRANSITIONS,
        "fx": crate::fx::FX.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
        "fx_max": crate::fx::MAX_PER_SHOT,
        "fx_burst": crate::fx::FX.iter().map(|(n, _)| (n.to_string(), json!(crate::fx::burst(n)))).collect::<serde_json::Map<_, _>>(),
        "lens_steps": crate::workspace::LENS_STEPS,
        "energies": crate::motion::ENERGIES,
        "title_size_default": crate::finish::TITLE_SIZE,
        "avoidable": crate::motion::AVOIDABLE,
        "faces": crate::fonts::catalog().families.iter().filter(|f| crate::fonts::overused(&f.family).is_none()).map(|f| {
            json!({"f": f.family, "s": if f.script == "latin" { "latin" } else { "ja" }, "k": f.kanji != "none",
                   "t": f.roles.iter().any(|r| r == "title"), "w": f.weights})
        }).collect::<Vec<_>>(),
    })
}

/// The cut in order (shots and script slots) with start and length in seconds.
fn timeline_json(defn: &crate::model::Definition, shots: &[crate::model::ResolvedShot]) -> Vec<Value> {
    let fps = defn.teaser.fps.max(1);
    let sec = |f: i64| crate::geometry::round_to(f as f64 / fps as f64, 3);
    crate::script::layout(&defn.script, shots, fps)
        .iter()
        .map(|it| match it.piece {
            crate::script::Piece::Shot(_) => json!({"id": it.id, "kind": "shot", "start": sec(it.start), "duration": sec(it.frames)}),
            crate::script::Piece::Slot(i) => {
                let l = &defn.script[i];
                json!({"id": it.id, "kind": l.kind, "text": l.ja, "start": sec(it.start), "duration": sec(it.frames)})
            }
        })
        .collect()
}

/// Where the first pass stands (D-38): stopped at the review and not continued since, the
/// quality it rendered at, and when.
fn review_json(ws: &Workspace) -> Value {
    let hist = ws.state.history();
    let last = hist.iter().rev().find(|e| e["action"] == "produce" || e["action"] == "plan");
    match last {
        Some(e) if e["action"] == "produce" && e["stopped"] == "review" => json!({"stopped": true, "at": e["at"], "quality": e["quality"]}),
        _ => Value::Null,
    }
}

/// What "undo" would take back, for the app's button: the action, its note and the pieces.
fn undo_json(e: &Value) -> Value {
    let keys = |k: &str| -> Vec<String> { e["undo"][k].as_object().map(|m| m.keys().cloned().collect()).unwrap_or_default() };
    json!({
        "action": e["action"], "note": e["note"], "at": e["at"],
        "shots": keys("shots"), "lines": keys("lines"),
        "order": e["undo"].get("order").is_some(), "audio": e["undo"].get("audio").is_some(),
    })
}

fn line_json(l: &crate::script::Line) -> Value {
    json!({"id": l.id, "kind": l.kind, "ja": l.ja, "en": l.en, "head": l.head, "from": l.from, "dur": l.dur,
           "after": l.after, "on": l.on, "at": l.at, "sfx": l.sfx})
}

/// The cut's motion as the app shows it (D-53): the profile and its name, the energy, what is
/// avoided. Null when `finish.motion` is not set (the base motion).
fn motion_json(cfg: &crate::motion::MotionCfg) -> Value {
    if !cfg.is_set() {
        return Value::Null;
    }
    let m = crate::motion::resolve(cfg);
    json!({
        "profile": m.profile,
        "name": crate::motion::profile(&m.profile).map(|p| p.name.clone()),
        "energy": m.energy,
        "avoid": m.avoid,
    })
}

fn brief_json(b: &crate::model::BriefCfg) -> Value {
    json!({
        "kind": b.kind, "genre": b.genre, "style": b.style, "genre_id": crate::refs::genre_for(&b.genre).map(|r| r.id),
        "audience": b.audience, "core": b.core,
        "must_show": b.must_show, "references": b.references, "notes": b.notes,
        "missing": b.missing(), "written": !b.raw.is_empty(), "images": b.images,
        "design": b.design, "skills": b.skills, "plan": b.plan, "direction": b.direction,
        "active_skills": crate::skills::active(b).iter().map(|s| s.id).collect::<Vec<_>>(),
    })
}

/// What the app's sound lanes and sound tab draw: every placement on the teaser's
/// timeline, the files that can be placed, and the cached mix with its sound check.
/// The places the cut hears, in the order it first hears them (D-76): each with the shots under
/// its air and the air written for it (`audio.ambience.<place>`, or null when it has none yet).
fn ambience_places(ws: &Workspace, shots: &[crate::model::ResolvedShot]) -> Vec<Value> {
    let air = ws.defn.audio.get("ambience").and_then(Value::as_object);
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for (s, place) in shots.iter().zip(sound::heard(&ws.defn, shots)) {
        let Some(place) = place else { continue };
        match out.iter_mut().find(|(p, _)| *p == place) {
            Some((_, ids)) => ids.push(s.id.clone()),
            None => out.push((place, vec![s.id.clone()])),
        }
    }
    out.into_iter()
        .map(|(place, ids)| {
            let v = air.and_then(|a| a.get(&place)).cloned().unwrap_or(Value::Null);
            let file = v.get("file").unwrap_or(&v).as_str().unwrap_or("").replace('\\', "/");
            json!({
                "place": place, "shots": ids, "air": v,
                "file": file, "missing": !file.is_empty() && !ws.root.join(&file).is_file(),
                "known": ws.defn.anchor(&place).is_some(),
            })
        })
        .collect()
}

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
        "timeline": timeline_json(&ws.defn, &shots),
        "shots": shots.iter().zip(&starts).map(|(s, t)| json!({
            "id": s.id, "start": crate::geometry::round_to(*t, 3),
            "duration": crate::geometry::round_to(s.frames as f64 / fps, 3), "beat": s.beat,
        })).collect::<Vec<_>>(),
        "cuts": sound::cuts(&shots, &starts).iter().map(|c| json!({"t": crate::geometry::round_to(c.t, 3), "from": c.from, "to": c.to}))
            .collect::<Vec<_>>(),
        "placements": placements,
        "places": ambience_places(ws, &shots),
        "orphans": sound::orphans(&ws.defn, &shots).iter().map(sound::Orphan::to_json).collect::<Vec<_>>(),
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
    let shots = app.current().unwrap_or_default();
    // the project's path too, so switching projects never answers 304
    let which = path_tag(&shots);
    let state = stamp(root.join(".shotpilot").join("state.json"));
    // the game content the takes showed: a new texture changes the status without touching
    // shots.yaml or state.json (the list is re-read only when state.json changes)
    let content = {
        let mut w = app.watched.lock().unwrap_or_else(|p| p.into_inner());
        let key = format!("{which}.{state}");
        if w.0 != key {
            *w = (key, crate::content::recorded_files(&root));
        }
        crate::content::stamp_files(&w.1)
    };
    // the release plan behind the stage badge (this project's promo.json, D-47)
    let promo = stamp(root.join(crate::promo::FILE));
    format!("\"{which}.{}.{state}.{content}.{settings}.{promo}\"", stamp(shots))
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
        ("GET" | "HEAD", "/") if app.current().is_none() => {
            send(stream, head, 302, "text/plain", b"", &[("Location", "/home".into())]).map_err(io)
        }
        ("GET" | "HEAD", "/") => send(stream, head, 200, "text/html; charset=utf-8", INDEX_HTML.as_bytes(),
                                       &[("Cache-Control", "no-store".into())]).map_err(io),
        ("GET" | "HEAD", "/home") => send(stream, head, 200, "text/html; charset=utf-8", HOME_HTML.as_bytes(),
                                           &[("Cache-Control", "no-store".into())]).map_err(io),
        ("GET" | "HEAD", "/i18n.js") => send(stream, head, 200, "text/javascript; charset=utf-8", i18n_js().as_bytes(),
                                              &[("Cache-Control", "no-store".into())]).map_err(io),
        ("GET" | "HEAD", "/api/home") => send_json(stream, head, 200, &home_payload(app)).map_err(io),
        ("GET" | "HEAD", "/home/cover") => {
            let p = req.query.split('&').find_map(|kv| kv.strip_prefix("p=")).unwrap_or("");
            match app.cover_file(p) {
                Some(file) => send_file(stream, req, &file, "public, max-age=31536000, immutable").map_err(io),
                None => send_json(stream, head, 404, &json!({"error": "not found"})).map_err(io),
            }
        }
        ("POST", p) if p.starts_with("/api/projects/") => projects_route(stream, app, &p["/api/projects/".len()..], req),
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
            // the history lives in state.json, which the status ETag covers
            let tag = format!("\"log{n}.{}", etag(app).trim_start_matches('"'));
            if req.header("if-none-match") == Some(tag.as_str()) {
                return send(stream, head, 304, "", b"", &[("ETag", tag)]).map_err(io);
            }
            let h = app.ws()?.state.history();
            let tail = h[h.len().saturating_sub(n)..].to_vec();
            send(stream, head, 200, "application/json; charset=utf-8", Value::Array(tail).to_string().as_bytes(),
                 &[("Cache-Control", "no-cache".into()), ("ETag", tag)]).map_err(io)
        }
        ("GET" | "HEAD", p) if p.starts_with("/files/") => match app.served_file(&p["/files/".len()..]) {
            None => send_json(stream, head, 404, &json!({"error": "not found"})).map_err(io),
            Some(file) => {
                // takes live under their fingerprint and are never overwritten, so they cache forever
                let cache = if p.starts_with("/files/render/") { "public, max-age=31536000, immutable" } else { "no-cache" };
                send_file(stream, req, &file, cache).map_err(io)
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
        ("POST", "/api/nudge") => {
            let body = req.json()?;
            let shot = body.get("shot").and_then(Value::as_str).unwrap_or("");
            let kind = body.get("kind").and_then(Value::as_str).unwrap_or("");
            // how big: s / m / l (m when left out); anything else is refused
            let amount = match body.get("amount") {
                None | Some(Value::Null) => "m",
                Some(v) => v.as_str().unwrap_or(""),
            };
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let sid = ws.select_one(shot)?.id;
            let rep = ws.nudge_by(&sid, kind, amount)?;
            let mut out = rep.to_json();
            out["changes"] = ws.state.history().last().map(|e| e["changes"].clone()).unwrap_or(Value::Null);
            send_json(stream, false, 200, &out).map_err(io)
        }
        ("POST", "/api/script") => {
            let body = req.json()?;
            let id = body.get("id").and_then(Value::as_str).unwrap_or("");
            let assignments: Vec<(String, Value)> = match body.get("set") {
                Some(Value::Object(m)) if !m.is_empty() => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                _ => bail!("expected {{\"id\": \"<line>\", \"set\": {{\"ja\": \"...\", ...}}}}"),
            };
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let rep = ws.set_line(id, &assignments)?;
            send_json(stream, false, 200, &rep.to_json()).map_err(io)
        }
        ("POST", "/api/undo") => {
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let (rep, what) = ws.undo()?;
            let mut out = rep.to_json();
            out["undid"] = what;
            send_json(stream, false, 200, &out).map_err(io)
        }
        ("POST", "/api/approve") => {
            let body = req.json()?;
            if body.get("script").and_then(Value::as_bool) == Some(true) {
                let approved = body.get("approved").and_then(Value::as_bool).unwrap_or(true);
                let _guard = app.mutate()?;
                let mut ws = app.ws()?;
                ws.approve_script(approved)?;
                return send_json(stream, false, 200, &json!({"script": ws.script_approval()})).map_err(io);
            }
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
            let look = ws.defn.brief.design.clone();
            ws.set_brief(&entries)?;
            // a look picked here brings its typefaces, title size and motion (D-52, D-53)
            if ws.defn.brief.design != look {
                ws.apply_look()?;
            }
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
                bail!("can only open files the app serves (render/, cut/, audio/, promo/ ...)");
            };
            open_with_os(&file)?;
            send_json(stream, false, 200, &json!({"opened": file.display().to_string()})).map_err(io)
        }
        ("POST", "/api/render") => render_stream(stream, app, req),
        ("POST", "/api/produce") => produce_stream(stream, app, req),
        ("GET" | "HEAD", "/api/sound") => send_json(stream, head, 200, &sound_payload(&app.ws()?)?).map_err(io),
        ("GET" | "HEAD", "/api/blocking") => send_json(stream, head, 200, &blocking_payload(&app.ws()?)?).map_err(io),
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
            let mut lines = Vec::new();
            if let Some(m) = body.get("lines") {
                let Some(m) = m.as_object() else { bail!("lines: {{\"<script line id>\": [ ... ] | null}}") };
                lines.extend(m.iter().map(|(id, list)| (id.clone(), list.clone())));
            }
            let audio = match body.get("audio") {
                None => None,
                Some(Value::Object(a)) => Some(a.clone()),
                Some(Value::Null) => Some(Map::new()),
                Some(_) => bail!("audio: a mapping like {{\"bgm\": {{\"file\": \"audio/bgm.wav\"}}}}"),
            };
            let note = body.get("note").and_then(Value::as_str).unwrap_or("");
            let rep = ws.edit_sound_full(&sfx, &lines, audio.as_ref(), note)?;
            let mut out = rep.to_json();
            out["sound"] = sound_payload(&ws)?;
            send_json(stream, false, 200, &out).map_err(io)
        }
        ("GET" | "HEAD", "/api/promo") => send_json(stream, head, 200, &crate::promo::view(&app.ws()?, crate::promo::today())?).map_err(io),
        ("POST", "/api/promo") => promo_route(stream, app, req),
        ("GET" | "HEAD", "/api/genres") => send_json(stream, head, 200, &crate::refs::catalogue()).map_err(io),
        ("GET" | "HEAD", "/api/library") => {
            let lib = app.library()?;
            let arg = |k: &str| {
                req.query.split('&').find_map(|kv| kv.strip_prefix(k).and_then(|v| v.strip_prefix('=')))
                    .map(percent_decode).unwrap_or_default()
            };
            let limit = arg("limit").parse().unwrap_or(200).min(1000);
            let root = app.root();
            let worn = lib.worn(&root);
            let items: Vec<Value> = lib.search(&arg("q"), &arg("kind"), limit).into_iter().map(|s| {
                let mut v = s.to_json();
                v["url"] = json!(format!("/library/{}", percent_encode(&s.id)));
                v["imported"] = json!(lib.imported(s, &root));
                // teasers of other games that use it now (D-64)
                v["others"] = json!(worn.get(&s.id).copied().unwrap_or(0));
                v
            }).collect();
            let out = json!({
                "available": lib.dir.is_some(),
                "dir": lib.dir.as_ref().map(|d| d.display().to_string()),
                "total": lib.sounds.len(),
                "bgm": lib.sounds.iter().filter(|s| s.is_bgm()).count(),
                "items": items,
            });
            send_json(stream, head, 200, &out).map_err(io)
        }
        ("GET" | "HEAD", p) if p.starts_with("/library/") => {
            let lib = app.library()?;
            let id = percent_decode(&p["/library/".len()..]);
            match lib.get(&id).and_then(|s| lib.file(s)) {
                None => send_json(stream, head, 404, &json!({"error": "not found"})).map_err(io),
                Some(file) => send_file(stream, req, &file, "max-age=3600").map_err(io),
            }
        }
        ("POST", "/api/library/import") => {
            let body = req.json()?;
            let Some(id) = body.get("id").and_then(Value::as_str) else { bail!("id: the library sound to use") };
            let _guard = app.mutate()?;
            let file = app.library()?.import(id, &app.root())?;
            let ws = app.ws()?;
            send_json(stream, false, 200, &json!({"file": file, "sound": sound_payload(&ws)?})).map_err(io)
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
        ("POST", "/api/finish") => {
            let body = req.json()?;
            let entries: Vec<(String, Value)> = match body.get("set") {
                Some(Value::Object(m)) if !m.is_empty() => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                _ => bail!("expected {{\"set\": {{\"letterbox\": 2.39, ...}}}}"),
            };
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            ws.set_finish(&entries)?;
            send_json(stream, false, 200, &json!({"ok": true})).map_err(io)
        }
        ("POST", "/api/reference") => {
            // a reference teaser (D-71): {pick: true} opens the app's file dialog for a video,
            // {path: "..."} names one, {id: "..."} borrows one measured before. Measuring runs
            // ffmpeg over the video (a few seconds); nothing re-renders
            let body = req.json()?;
            let video = if body.get("pick").and_then(Value::as_bool) == Some(true) {
                let Some(picker) = &app.picker else {
                    bail!("この画面では動画を選べません。動画のパスを入力してください");
                };
                let start = app.current().and_then(|p| p.parent().map(Path::to_path_buf));
                match picker(Pick::Video { start }) {
                    None => return send_json(stream, false, 200, &json!({"cancelled": true})).map_err(io),
                    Some(p) => Some(p),
                }
            } else {
                body.get("path").and_then(Value::as_str).map(str::trim).filter(|p| !p.is_empty()).map(PathBuf::from)
            };
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let (id, notes) = match (video, body.get("id").and_then(Value::as_str)) {
                (Some(v), _) => {
                    let (r, notes) = ws.add_reference(&v, true)?;
                    (r.id, notes)
                }
                (None, Some(id)) => (id.to_string(), ws.use_reference(id)?),
                (None, None) => bail!("expected {{\"pick\": true}}, {{\"path\": \"...\"}} or {{\"id\": \"...\"}}"),
            };
            send_json(stream, false, 200, &json!({"id": id, "notes": notes})).map_err(io)
        }
        ("POST", "/api/reference/compare") => {
            // measure the newest cut the same way and pair its frames with the reference's
            let ws = app.ws()?;
            let c = ws.compare_reference(None)?;
            send_json(stream, false, 200, &json!({"pairs": c.pairs.len(), "cut": c.cut.to_json()})).map_err(io)
        }
        ("POST", "/api/finish/logo") => {
            // {pick: true} opens the app's file dialog; {path: "art/logo.png"} names one inside
            // the project; {clear: true} takes the logo off
            let body = req.json()?;
            let flag = |k: &str| body.get(k).and_then(Value::as_bool) == Some(true);
            let picked = if flag("pick") {
                let Some(picker) = &app.picker else {
                    bail!("この画面では画像を選べません。プロジェクトの中の画像のパスを入力してください");
                };
                let start = app.current().and_then(|p| p.parent().map(Path::to_path_buf));
                match picker(Pick::Image { start }) {
                    None => return send_json(stream, false, 200, &json!({"cancelled": true})).map_err(io),
                    Some(p) => Some(p),
                }
            } else {
                None
            };
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let rel = match picked {
                Some(p) => Some(ws.import_logo(&p)?),
                None if flag("clear") => None,
                None => match body.get("path").and_then(Value::as_str).map(str::trim).filter(|p| !p.is_empty()) {
                    Some(p) if ws.root.join(p).is_file() => Some(p.replace('\\', "/")),
                    Some(p) => bail!("{p} がプロジェクトの中に見つかりません"),
                    None => bail!("expected {{\"pick\": true}}, {{\"path\": \"art/logo.png\"}} or {{\"clear\": true}}"),
                },
            };
            ws.set_finish(&[("logo".to_string(), json!(rel))])?;
            send_json(stream, false, 200, &json!({"logo": ws.defn.finish.logo})).map_err(io)
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
            // Claude picks the places by the plan when connected; without it the scout still runs
            let claude = (app.claude)(&app.settings()).ok();
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let rep = ws.scout(&only, claude.as_deref())?;
            let mut out = rep.index;
            out["scanned"] = json!(rep.scan.map(|s| json!({"added": s.added, "auto": s.auto, "picked_by": s.picked_by, "note": s.note})));
            out["assets"] = json!(rep.assets);
            out["matched"] = json!(rep.matched.map(|m| json!({"rounds": m.rounds, "added": m.added, "found": m.found, "open": m.open})));
            send_json(stream, false, 200, &out).map_err(io)
        }
        ("POST", "/api/discover") => {
            // the planning turn (D-45): a question form, or a plan written to brief:
            let body = req.json()?;
            let request = body.get("request").and_then(Value::as_str).unwrap_or("").to_string();
            let answers = answers_text(body.get("answers"));
            let claude = (app.claude)(&app.settings())?;
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let d = ai::discover(&ws.defn, &request, &answers, true, claude.as_ref())?;
            let entry = ws.apply_discovery(&d, &request, body.get("answers").unwrap_or(&Value::Null))?;
            send_json(stream, false, 200, &entry).map_err(io)
        }
        ("POST", "/api/concept/generate") => {
            // the concept frames' pictures (D-46), only when the human asks
            let gen = (app.images)(&app.settings())?; // say what is missing before taking the lock
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let made = ws.generate_concept(gen.as_ref())?;
            send_json(stream, false, 200, &json!({"made": made})).map_err(io)
        }
        ("POST", "/api/concept/match") => {
            // find the frames' places in the level; may photograph other candidates (UE)
            let claude = (app.claude)(&app.settings())?;
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let m = ws.match_concept(claude.as_ref(), None)?;
            send_json(stream, false, 200, &json!({"rounds": m.rounds, "added": m.added, "found": m.found, "open": m.open})).map_err(io)
        }
        ("GET" | "HEAD", "/api/kits") => send_json(stream, head, 200, &kits_json()).map_err(io),
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
        ("POST", "/api/storyboard") => {
            let body = req.json()?;
            let shots: Vec<String> = body
                .get("shots")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let direction = body.get("direction").and_then(Value::as_str).unwrap_or("").to_string();
            let force = body.get("force").and_then(Value::as_bool).unwrap_or(false);
            let gen = (app.images)(&app.settings())?; // say what is missing before taking the lock
            let _guard = app.mutate()?;
            let mut ws = app.ws()?;
            let out = ws.generate_storyboard(gen.as_ref(), &shots, &direction, force)?;
            let made: Vec<&str> = out.iter().filter(|o| o.action == "made").map(|o| o.shot_id.as_str()).collect();
            let kept: Vec<&str> = out.iter().filter(|o| o.action == "kept").map(|o| o.shot_id.as_str()).collect();
            send_json(stream, false, 200, &json!({"made": made, "kept": kept})).map_err(io)
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
        ("POST", "/api/suggest") => {
            // chips for the stack composer (D-62): one quick read, nothing is written and no lock is
            // taken. The home asks before a project exists (its first prompt), so no workspace then.
            let body = req.json()?;
            let ask = crate::suggest::Ask::from_json(&body);
            let claude = (app.claude)(&app.settings())?;
            let ws = if ask.target == "pitch" { None } else { Some(app.ws()?) };
            let chips = crate::suggest::suggest(ws.as_ref().map(|w| &w.defn), &ask, claude.as_ref())?;
            send_json(stream, false, 200, &json!({"chips": chips})).map_err(io)
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
    ws.set_finish(&p.finish)?;
    ws.replace_plan(&p.shots, Some(&p.script))?; // last, so the log entry annotated below is the plan
    let found = ws.check()?;
    let count = |sev: &str| found.iter().filter(|f| f.severity.as_str() == sev).count();
    let checks = json!({"P0": count("P0"), "P1": count("P1"), "P2": count("P2")});
    let mut extra = Map::new();
    extra.insert("note".into(), json!(note));
    extra.insert("rationale".into(), json!(p.rationale));
    extra.insert("adjustments".into(), json!(p.adjustments));
    extra.insert("references".into(), json!(p.references));
    extra.insert("checks".into(), checks.clone());
    ws.state.annotate_last(extra);
    ws.state.save()?;
    Ok(json!({
        "shots": p.shots.iter().map(|s| &s["id"]).collect::<Vec<_>>(), "rationale": p.rationale,
        "adjustments": p.adjustments, "references": p.references, "checks": checks,
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
    let aspect = body.get("aspect").and_then(Value::as_str).filter(|a| !a.is_empty() && *a != "shot");
    // the finishing: the text, the hand-overs and the look burned in at the cut (never re-renders)
    let finishing = aspect == Some("finish");
    let all = body.get("all").and_then(Value::as_bool) == Some(true);
    let line_id = body.get("line").and_then(Value::as_str).filter(|s| !s.is_empty());
    let shot_given = body.get("shot").and_then(Value::as_str).is_some_and(|s| !s.is_empty());
    // the cuts a mood on the canvas is wired to (D-62): a note for those cuts and no other
    let wired: Vec<&str> = body.get("shots").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).filter(|s| !s.is_empty()).collect();
    let scope = match body.get("shot").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        Some(s) => Some(vec![ws.select_one(s)?.id]),
        None if !wired.is_empty() => {
            let mut ids: Vec<String> = Vec::new();
            for s in &wired {
                let id = ws.select_one(s)?.id;
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            Some(ids)
        }
        // one part of every shot (e.g. the cameras, keeping the script) is still a scoped note
        None if aspect.is_some() && all => Some(ws.defn.shot_ids()),
        // the words of one card between shots: no shot changes
        None if finishing && line_id.is_some() => Some(Vec::new()),
        None => None,
    };
    let mut context = Vec::new();
    let mut pointer = ai::Pointer { sound: body.get("sound").and_then(Value::as_bool).unwrap_or(false), ..Default::default() };
    match scope.as_deref() {
        Some([]) | None => {}
        Some([one]) => context.push(format!("shot {one}")),
        Some(many) if many.len() == ws.defn.shots.len() => context.push(format!("all {} shots", many.len())),
        Some(many) => context.push(format!("shots {}", many.join(", "))),
    }
    if let Some(a) = aspect {
        let Some(fields) = ai::aspect_fields(a) else {
            bail!("aspect: one of shot, {}", ai::ASPECTS.iter().map(|(k, _, _)| *k).collect::<Vec<_>>().join(", "));
        };
        if scope.is_none() || pointer.sound {
            bail!("a note on one part of a shot needs that shot");
        }
        if scope.as_ref().is_some_and(|s| !s.is_empty()) {
            context.push(format!("the director points at this part of the shot: {a}"));
            pointer.fields = Some(fields);
        }
    }
    if let Some(l) = line_id {
        let Some(line) = ws.defn.script.iter().find(|x| x.id == l) else { bail!("unknown script line '{l}'") };
        if finishing {
            if shot_given {
                bail!("a note on a script line's text takes no shot");
            }
            context.push(format!("the words and timing of script line {} ({}: {})", line.id, line.kind, line.ja));
            pointer.texts = Some(vec![line.id.clone()]);
        } else {
            if scope.is_some() || !pointer.sound {
                bail!("a note on a script line's sound takes no shot");
            }
            context.push(format!("script line {} ({}: {})", line.id, line.kind, line.ja));
            pointer.line = Some(line.id.clone());
        }
    }
    // the script's words go with the shots a note is about (every line for the whole teaser), and
    // the teaser's look with notes on the whole teaser; a note on the camera, light or sound keeps them
    if pointer.texts.is_none() && !pointer.sound && (aspect.is_none() || finishing) {
        let whole = scope.is_none() || (finishing && all);
        let on_scope = |l: &crate::script::Line| l.on.as_ref().is_some_and(|o| scope.as_ref().is_some_and(|s| s.contains(o)));
        pointer.texts = Some(ws.defn.script.iter().filter(|l| whole || on_scope(l)).map(|l| l.id.clone()).collect());
        pointer.finish = whole;
    }
    if pointer.sound {
        context.push("working on the sound (background music and sound effects)".into());
        if let Some(i) = body.get("sfx_index").and_then(Value::as_u64) {
            let owner = if pointer.line.is_some() { "that script line's" } else { "that shot's" };
            context.push(format!("sound effect #{} of {owner} sfx list", i + 1));
        }
        if let Some(i) = body.get("music_index").and_then(Value::as_u64) {
            context.push(format!("music cue #{} of audio.music (keep the other cues as they are)", i + 1));
        } else if body.get("bgm").and_then(Value::as_bool) == Some(true) {
            context.push("the background music audio.bgm".into());
        }
        for o in sound::orphans(&ws.defn, &ws.resolved()?) {
            context.push(o.text());
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
    if pointer.sound {
        pointer.library = Some(app.library()?);
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
    for (id, list) in &rev.lines {
        let old = ws.defn.script.iter().find(|l| l.id == *id).map(|l| json!({"sfx": l.sfx})).unwrap_or(Value::Null);
        let new = json!({"sfx": if list.is_null() { json!([]) } else { list.clone() }});
        changes.insert(id.clone(), Value::Array(field_changes(&old, &new)));
    }
    for (id, set) in &rev.texts {
        let Some(l) = ws.defn.script.iter().find(|l| l.id == *id) else { continue };
        let old = json!({"ja": l.ja, "en": l.en, "head": l.head, "from": l.from, "kind": l.kind, "at": l.at, "dur": l.dur});
        let mut new = old.clone();
        for (k, v) in set {
            new[k] = if v.is_null() { json!("") } else { v.clone() };
        }
        let list = changes.entry(id.clone()).or_insert_with(|| json!([]));
        if let Some(a) = list.as_array_mut() {
            a.extend(field_changes(&old, &new));
        }
    }
    if !rev.finish.is_empty() {
        let f = &ws.defn.finish;
        let old = json!({"grade": f.grade, "letterbox": f.letterbox, "typeface": f.typeface});
        let mut new = old.clone();
        for (k, v) in &rev.finish {
            new[k] = v.clone();
        }
        changes.insert("finish".into(), Value::Array(field_changes(&old, &new)));
    }
    let rep = ws.apply_revision(&rev, "revise", note)?;
    let mut extra = Map::new();
    extra.insert("summary".into(), json!(rev.summary));
    extra.insert("changes".into(), Value::Object(changes.clone()));
    extra.insert("pointer".into(), json!(pointer.context));
    // what the human pointed at, in the app's words (the context above is written for Claude)
    if let Some(l) = body.get("label").and_then(Value::as_str).filter(|l| !l.is_empty()) {
        extra.insert("label".into(), json!(l));
    }
    ws.state.annotate_last(extra);
    ws.state.save()?;
    let mut out = rep.to_json();
    out["summary"] = json!(rev.summary);
    out["changes"] = Value::Object(changes);
    Ok(out)
}

/// The first pass in one go, streamed as server-sent events (one per step and per shot).
/// A question form's answers ([{id, label, value}]) as the text the planning turn reads.
fn answers_text(v: Option<&Value>) -> String {
    let rows: Vec<String> = v
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let value = match &a["value"] {
                Value::Array(xs) => xs.iter().map(crate::model::str_of).collect::<Vec<_>>().join(", "),
                Value::Null => return None,
                x => crate::model::str_of(x),
            };
            let label = a["label"].as_str().or(a["id"].as_str()).unwrap_or("");
            Some(format!("- {label} [{}]: {value}", a["id"].as_str().unwrap_or("")))
        })
        .collect();
    if rows.is_empty() {
        String::new()
    } else {
        format!("[form answers — discovery]\n{}", rows.join("\n"))
    }
}

/// The design systems and skills the app can show and pick from (D-45).
pub fn kits_json() -> Value {
    json!({
        "design_systems": crate::designsys::all().iter().map(|d| d.to_json()).collect::<Vec<_>>(),
        "skills": crate::skills::all().iter().map(|s| s.to_json()).collect::<Vec<_>>(),
    })
}

fn produce_stream(stream: &mut TcpStream, app: &App, req: &Request) -> Result<()> {
    let body = req.json()?;
    let opts = crate::produce::ProduceOpts {
        quality: body.get("quality").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("previz").to_string(),
        direction: body.get("note").and_then(Value::as_str).unwrap_or("").to_string(),
        mood: body.get("mood").and_then(Value::as_bool).unwrap_or(true),
        library: None,
        review: body.get("review").and_then(Value::as_bool).unwrap_or(false),
    }
    .with_user_library();
    let resume = body.get("resume").and_then(Value::as_bool).unwrap_or(false);
    let settings = app.settings();
    let claude = (app.claude)(&settings)?; // a plain 400 before streaming when Claude is not connected
    let images = if opts.mood && !resume { (app.images)(&settings).ok() } else { None };
    let _guard = app.mutate()?;
    let mut ws = app.ws()?;
    fp::quality(&ws.defn, &opts.quality)?;
    let head = "HTTP/1.1 200 OK\r\nServer: shotpilot\r\nConnection: close\r\n\
                Content-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-cache, no-transform\r\n\
                X-Accel-Buffering: no\r\n\r\n";
    // the run must finish even if the window goes away, so write errors only stop the stream
    let mut alive = stream.write_all(head.as_bytes()).is_ok();
    let mut send_event = |event: &str, data: &Value, alive: &mut bool| {
        if *alive {
            let msg = format!("event: {event}\ndata: {data}\n\n");
            *alive = stream.write_all(msg.as_bytes()).and_then(|_| stream.flush()).is_ok();
        }
    };
    let mut on = |event: &str, data: &Value| send_event(event, data, &mut alive);
    let result = if resume {
        crate::produce::resume(&mut ws, claude.as_ref(), &opts, &mut on)
    } else {
        crate::produce::produce(&mut ws, claude.as_ref(), images.as_deref(), &opts, &mut on)
    };
    match result {
        Ok(out) => send_event("done", &out, &mut alive),
        Err(e) => send_event("error", &json!({"error": e.msg}), &mut alive),
    }
    Ok(())
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

// --------------------------------------------------------------------------- home

/// A short tag for a path (part of the status ETag).
fn path_tag(p: &Path) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(p.display().to_string().as_bytes()).iter().take(4).map(|b| format!("{b:02x}")).collect()
}

/// The home: the open project, a card per recent project, and what a new one needs.
fn home_payload(app: &App) -> Value {
    let current = app.current();
    let mut list = app.recents.list();
    // the open project is on the list even when no recents file is kept
    if let Some(c) = &current {
        if !list.iter().any(|r| projects::same_path(&r.path, c)) {
            list.insert(0, projects::Recent { path: c.clone(), opened: String::new() });
        }
    }
    let cards: Vec<Value> = list
        .iter()
        .map(|r| {
            let mut v = projects::summary(&r.path);
            v["opened"] = json!(r.opened);
            v["current"] = json!(current.as_ref().is_some_and(|c| projects::same_path(c, &r.path)));
            if let Some(covers) = v["covers"].as_array_mut() {
                for c in covers {
                    let file = c["file"].as_str().unwrap_or("").to_string();
                    c["url"] = json!(format!("/home/cover?p={}", percent_encode(&file)));
                }
            }
            v
        })
        .collect();
    json!({
        "current": current.map(|p| p.display().to_string()),
        "projects": cards,
        "location": projects::default_location().map(|p| p.display().to_string()),
        "picker": app.picker.is_some(),
        "ai": app.ai_status(),
    })
}

/// `POST /api/projects/<action>`: open, create, forget, reveal, pick, maps.
/// The release plan (D-47): one POST with an `op`, answered with the whole view again.
fn promo_route(stream: &mut TcpStream, app: &App, req: &Request) -> Result<()> {
    use crate::promo::{self, Carry, Plan};
    let io = |e: std::io::Error| Error::user(format!("io: {e}"));
    let body = req.json()?;
    let text = |k: &str| body.get(k).and_then(Value::as_str).map(str::trim).unwrap_or("").to_string();
    let set = || match body.get("set") {
        Some(Value::Object(m)) => Ok(m.clone()),
        _ => Err(Error::user("expected {\"set\": {...}}")),
    };
    let today = promo::today();
    let mut extra = Map::new();
    {
        let _guard = app.mutate()?;
        let ws = app.ws()?;
        match text("op").as_str() {
            "derive" | "update" | "export" => {}
            _ => {
                let mut plan = Plan::open(&ws)?;
                match text("op").as_str() {
                    "add" => {
                        let id = plan.add_video(&text("kind"), &text("title"), &text("due"), &text("update"))?;
                        extra.insert("id".into(), json!(id));
                    }
                    "video" => plan.set_video(&text("id"), &set()?)?,
                    "remove" => plan.remove_video(&text("id"))?,
                    "check" => plan.set_check(&text("video"), &text("check"), body.get("done").and_then(Value::as_bool).unwrap_or(false))?,
                    "schedule" => plan.set_schedule(&set()?)?,
                    "post" => plan.set_post(&text("id"), &set()?)?,
                    "post.remove" => plan.remove_post(&text("id"))?,
                    other => bail!("unknown op '{other}' (add / video / remove / check / schedule / post / post.remove / export / derive / update)"),
                }
                plan.save()?;
            }
        }
        match text("op").as_str() {
            "derive" => {
                let carry = if text("carry") == "shots" { Carry::Shots } else { Carry::Groundwork };
                let p = promo::derive(&ws, &text("video"), carry)?;
                extra.insert("path".into(), json!(p.display().to_string()));
            }
            "update" => {
                let (id, p) = promo::start_update(&ws, &text("update"), &text("due"))?;
                extra.insert("id".into(), json!(id));
                extra.insert("path".into(), json!(p.display().to_string()));
            }
            "export" => {
                let clip = body.get("clip").and_then(Value::as_bool).unwrap_or(false);
                extra.insert("id".into(), json!(promo::export_post(&ws, &text("shot"), clip, today)?));
            }
            _ => {}
        }
    }
    // a derived project goes on the home's recent list (it is opened from there or from the plan)
    if let Some(p) = extra.get("path").and_then(Value::as_str) {
        let _ = app.recents.touch(Path::new(p));
    }
    let mut out = promo::view(&app.ws()?, today)?;
    out["result"] = Value::Object(extra);
    send_json(stream, false, 200, &out).map_err(io)
}

fn projects_route(stream: &mut TcpStream, app: &App, action: &str, req: &Request) -> Result<()> {
    let io = |e: std::io::Error| Error::user(format!("io: {e}"));
    let body = req.json()?;
    let text = |k: &str| body.get(k).and_then(Value::as_str).map(str::trim).unwrap_or("").to_string();
    let path_arg = || Some(text("path")).filter(|s| !s.is_empty()).map(PathBuf::from);
    let out = match action {
        "open" => {
            let Some(p) = path_arg() else { bail!("path is required") };
            json!({"path": app.open(&p)?.display().to_string()})
        }
        "create" => {
            let location = match text("location") {
                l if l.is_empty() => projects::default_location().unwrap_or_default(),
                l => PathBuf::from(l),
            };
            let engine = match text("engine") {
                e if e.is_empty() => "mock".to_string(),
                e => e,
            };
            let np = NewProject {
                location,
                name: text("name"),
                engine,
                uproject: Some(text("uproject")).filter(|s| !s.is_empty()).map(PathBuf::from),
                map: Some(text("map")).filter(|s| !s.is_empty()),
                pitch: text("pitch"),
                kind: text("kind"),
                genre: text("genre"),
                sample: body.get("sample").and_then(Value::as_bool).unwrap_or(false),
            };
            let shots = {
                let _guard = app.mutate()?;
                projects::create(&np)?
            };
            json!({"path": app.open(&shots)?.display().to_string()})
        }
        "forget" => {
            let Some(p) = path_arg() else { bail!("path is required") };
            app.recents.forget(&p)?;
            json!({"forgot": p.display().to_string()})
        }
        "reveal" => {
            let Some(p) = path_arg() else { bail!("path is required") };
            let known = app.recents.contains(&p) || app.current().is_some_and(|c| projects::same_path(&c, &p));
            let Some(dir) = p.parent().filter(|d| known && d.is_dir()) else {
                bail!("そのプロジェクトは一覧にありません");
            };
            open_with_os(dir)?;
            json!({"opened": dir.display().to_string()})
        }
        "pick" => {
            let Some(picker) = &app.picker else {
                bail!("この画面ではフォルダを選べません。パスを入力してください");
            };
            let start = path_arg().filter(|p| p.exists());
            match text("kind").as_str() {
                "open" => match picker(Pick::Folder { title: "Shotpilot: shots.yaml のあるフォルダを選ぶ".into(), start }) {
                    None => json!({"cancelled": true}),
                    Some(d) => json!({"path": d.display().to_string(),
                        "projects": projects::find_projects(&d).iter().map(|p| p.display().to_string()).collect::<Vec<_>>()}),
                },
                "location" => match picker(Pick::Folder { title: "Shotpilot: 新しいプロジェクトを置くフォルダ".into(), start }) {
                    None => json!({"cancelled": true}),
                    Some(d) => json!({"path": d.display().to_string()}),
                },
                "uproject" => match picker(Pick::Uproject { start }) {
                    None => json!({"cancelled": true}),
                    Some(up) => uproject_json(&up)?,
                },
                other => bail!("kind must be open, location or uproject (got '{other}')"),
            }
        }
        "maps" => {
            let Some(p) = path_arg() else { bail!("path is required") };
            uproject_json(&p)?
        }
        other => bail!("unknown action: {other}"),
    };
    send_json(stream, false, 200, &out).map_err(io)
}

/// A .uproject (or its folder) with the levels it has, for the new-project form.
fn uproject_json(p: &Path) -> Result<Value> {
    let Some(up) = projects::find_uproject(p) else {
        bail!("{} に .uproject がありません", p.display());
    };
    Ok(json!({
        "path": up.display().to_string(),
        "name": up.file_stem().map(|s| s.to_string_lossy().to_string()),
        "maps": projects::list_maps(&up),
        "projects": up.parent().map(projects::find_projects).unwrap_or_default().iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
    }))
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
