//! The app's local backend: a small HTTP + Server-Sent Events layer over Workspace.
//!
//! The desktop app (crates/shotpilot-app) opens one window on this server. It binds to
//! loopback, answers only loopback Host/Origin headers, and serves files only from the
//! project's render/ and cut/ folders.
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

use crate::bail;
use crate::error::{Error, Kind, Result};
use crate::fingerprint as fp;
use crate::workspace::Workspace;

pub const INDEX_HTML: &str = include_str!("../web/index.html");
const LOOPBACK: &[&str] = &["127.0.0.1", "localhost", "::1"];
const SERVED_DIRS: &[&str] = &["render", "cut"];
const MAX_BODY: usize = 1 << 20;

pub struct App {
    pub path: PathBuf,
    lock: Mutex<()>,
}

impl App {
    pub fn new(shots_path: &Path) -> Result<App> {
        let ws = Workspace::open(shots_path)?; // fail fast on a broken definition
        Ok(App { path: ws.path, lock: Mutex::new(()) })
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

    /// A served file under render/ or cut/, from a `/files/...` remainder.
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
        let app = Arc::new(App::new(shots_path)?);
        let listener = TcpListener::bind((host, port))
            .map_err(|e| Error::user(format!("cannot listen on {host}:{port}: {e}")))?;
        Ok(Server { listener, app, bind_host: host.to_ascii_lowercase() })
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
            thumbs.insert(q.clone(), json!({"url": url, "fresh": is_fresh, "fingerprint": &rfp[..12]}));
        }
        let mut v = row.to_json();
        v["lens"] = s.lens.get("focal_length").cloned().unwrap_or(Value::Null);
        v["thumbs"] = Value::Object(thumbs);
        v["node"] = ws.defn.shots[row.index].clone();
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
        "shots": shots,
    }))
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
    format!("\"{}.{}\"", stamp(app.path.clone()), stamp(root.join(".shotpilot").join("state.json")))
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
            let body = status_payload(&app.ws()?)?;
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
            });
            send_json(stream, false, 200, &out).map_err(io)
        }
        ("POST", "/api/open") => {
            let body = req.json()?;
            let url = body.get("url").and_then(Value::as_str).unwrap_or("");
            let Some(file) = url.strip_prefix("/files/").and_then(|r| app.served_file(r)) else {
                bail!("can only open files under render/ or cut/");
            };
            open_with_os(&file)?;
            send_json(stream, false, 200, &json!({"opened": file.display().to_string()})).map_err(io)
        }
        ("POST", "/api/render") => render_stream(stream, app, req),
        _ => {
            send_json(stream, head, 404, &json!({"error": format!("no route for {} {}", req.method, req.path)})).map_err(io)
        }
    }
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
