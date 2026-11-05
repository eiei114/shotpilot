//! The app backend: the same partial-fix loop, driven over HTTP + SSE.

mod common;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;

use common::{launches, set, ws, Fixture};
use serde_json::{json, Value};
use shotpilot::server::Server;

struct Srv {
    port: u16,
    f: Fixture,
}

fn srv() -> Srv {
    let f = ws();
    let s = Server::bind(&f.path(), "127.0.0.1", 0).unwrap();
    let port = s.port();
    s.spawn();
    Srv { port, f }
}

struct Resp {
    code: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Resp {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
    fn header(&self, k: &str) -> &str {
        self.headers.get(k).map(String::as_str).unwrap_or("")
    }
}

fn call(s: &Srv, method: &str, path: &str, body: Option<Value>, headers: &[(&str, &str)]) -> Resp {
    let mut c = TcpStream::connect(("127.0.0.1", s.port)).unwrap();
    let data = body.map(|b| b.to_string()).unwrap_or_default();
    let mut req = format!("{method} {path} HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n", data.len());
    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
        req.push_str(&format!("Host: 127.0.0.1:{}\r\n", s.port));
    }
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(&data);
    c.write_all(req.as_bytes()).unwrap();
    let mut raw = Vec::new();
    c.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let mut lines = head.lines();
    let code: u16 = lines.next().unwrap().split_whitespace().nth(1).unwrap().parse().unwrap();
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    Resp { code, headers, body: raw[split + 4..].to_vec() }
}

fn jcall(s: &Srv, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
    let r = call(s, method, path, body, &[]);
    (r.code, r.json())
}

fn sse(raw: &[u8]) -> Vec<(String, Value)> {
    String::from_utf8_lossy(raw)
        .split("\n\n")
        .filter(|b| !b.trim().is_empty())
        .map(|block| {
            let mut ev = String::new();
            let mut data = Value::Null;
            for line in block.lines() {
                if let Some(e) = line.strip_prefix("event: ") {
                    ev = e.to_string();
                } else if let Some(d) = line.strip_prefix("data: ") {
                    data = serde_json::from_str(d).unwrap();
                }
            }
            (ev, data)
        })
        .collect()
}

fn render(s: &Srv, quality: &str) -> (HashMap<String, String>, Vec<(String, Value)>) {
    let r = call(s, "POST", "/api/render", Some(json!({"quality": quality})), &[]);
    assert_eq!(r.code, 200);
    assert!(r.header("content-type").starts_with("text/event-stream"));
    let events = sse(&r.body);
    assert_eq!(events.last().unwrap().0, "done", "{:?}", events.last());
    let acts = events
        .iter()
        .filter(|(e, _)| e == "shot")
        .map(|(_, d)| (d["shot_id"].as_str().unwrap().to_string(), d["action"].as_str().unwrap().to_string()))
        .collect();
    (acts, events)
}

fn previz_states(st: &Value) -> HashMap<String, String> {
    st["shots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["id"].as_str().unwrap().to_string(), s["qualities"]["previz"].as_str().unwrap().to_string()))
        .collect()
}

#[test]
fn index_and_status() {
    let s = srv();
    let r = call(&s, "GET", "/", None, &[]);
    assert!(r.code == 200 && r.header("content-type").starts_with("text/html"));
    assert!(String::from_utf8_lossy(&r.body).contains("shotpilot"));
    let (code, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(code, 200);
    assert_eq!(st["shots"].as_array().unwrap().len(), 6);
    assert_eq!(st["shots"][2]["id"], "s03_window");
    assert_eq!(st["shots"][0]["qualities"]["previz"], "none");
    assert_eq!(st["shots"][0]["node"]["id"], "s01_gate");
}

#[test]
fn status_polling_is_a_304_while_nothing_changes() {
    let s = srv();
    let first = call(&s, "GET", "/api/status", None, &[]);
    let tag = first.header("etag").to_string();
    assert!(!tag.is_empty());
    let again = call(&s, "GET", "/api/status", None, &[("If-None-Match", &tag)]);
    assert_eq!(again.code, 304);
    assert!(again.body.is_empty());
}

#[test]
fn http_only_changed_shot_rerenders() {
    let s = srv();
    let (first, events) = render(&s, "previz");
    assert!(first.values().all(|a| a == "rendered"));
    assert_eq!(events[0].0, "plan");
    assert_eq!(events[0].1["render"].as_array().unwrap().len(), first.len());
    assert!(events.iter().filter(|(e, _)| e == "shot").all(|(_, d)| d["thumb"].as_str().unwrap().starts_with("/files/render/")));

    let (code, rep) = jcall(&s, "POST", "/api/shots/s03_window", Some(json!({"set": {"camera.lens.focal_length": 85}})));
    assert_eq!(code, 200);
    assert_eq!(rep["stale"], json!(["s03_window"]));
    let st = jcall(&s, "GET", "/api/status", None).1;
    assert_eq!(previz_states(&st)["s03_window"], "stale");

    let (mut second, events) = render(&s, "previz");
    assert_eq!(second.remove("s03_window").unwrap(), "rendered");
    assert!(second.values().all(|a| a == "reused"));
    assert_eq!(events.last().unwrap().1, json!({"quality": "previz", "rendered": 1, "reused": second.len(), "failed": 0}));
}

#[test]
fn http_engine_render_launches_only_changed_shot() {
    let s = srv();
    render(&s, "preview");
    let (code, rep) = jcall(&s, "POST", "/api/shots/3", Some(json!({"set": {"duration": 3}}))); // "3" = third shot
    assert_eq!(code, 200);
    assert_eq!(rep["stale"], json!(["s03_window"]));
    let (mut second, _) = render(&s, "preview");
    assert_eq!(second.remove("s03_window").unwrap(), "rendered");
    assert!(second.values().all(|a| a == "reused"));
    let last = launches(&s.f.ws).into_iter().rev().find(|x| x["kind"] == "render").unwrap();
    assert_eq!(last["shots"], json!(["s03_window:preview"]));
}

#[test]
fn reorder_over_http_never_invalidates() {
    let s = srv();
    render(&s, "previz");
    let mut order = s.f.ws.defn.shot_ids();
    order.reverse();
    let (code, rep) = jcall(&s, "POST", "/api/order", Some(json!({"order": order})));
    assert_eq!(code, 200);
    assert_eq!(rep["stale"], json!([]));
    let (again, _) = render(&s, "previz");
    assert!(again.values().all(|a| a == "reused"));
}

#[test]
fn thumbnails_are_served_and_immutable() {
    let s = srv();
    let (_, events) = render(&s, "previz");
    let url = events.iter().find(|(e, _)| e == "shot").unwrap().1["thumb"].as_str().unwrap().to_string();
    let r = call(&s, "GET", &url, None, &[]);
    assert_eq!(r.code, 200);
    assert_eq!(r.header("content-type"), "image/png");
    assert!(r.header("cache-control").contains("immutable"));
    assert_eq!(&r.body[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn files_stay_inside_project_outputs() {
    let s = srv();
    render(&s, "previz");
    for path in ["/files/../shots.yaml", "/files/render/../shots.yaml", "/files/.shotpilot/state.json",
                 "/files/render/%2e%2e/shots.yaml", "/files/render/..%5Cshots.yaml"] {
        assert_eq!(call(&s, "GET", path, None, &[]).code, 404, "{path}");
    }
}

#[test]
fn foreign_host_and_origin_are_rejected() {
    let s = srv();
    assert_eq!(call(&s, "GET", "/api/status", None, &[("Host", "evil.example")]).code, 403);
    let r = call(&s, "POST", "/api/shots/s01_gate", Some(json!({"set": {"duration": 9}})), &[("Origin", "http://evil.example")]);
    assert_eq!(r.code, 403);
}

#[test]
fn user_errors_are_400_with_message() {
    let s = srv();
    let (code, body) = jcall(&s, "POST", "/api/shots/nope", Some(json!({"set": {"duration": 2}})));
    assert_eq!(code, 400);
    assert!(body["error"].as_str().unwrap().contains("unknown shot"));
    let (code, body) = jcall(&s, "POST", "/api/render", Some(json!({"quality": "ultra"})));
    assert_eq!(code, 400);
    assert!(body["error"].as_str().unwrap().contains("unknown quality"));
    let (code, _) = jcall(&s, "POST", "/api/order", Some(json!({"order": ["s01_gate"]})));
    assert_eq!(code, 400);
}

#[test]
fn cli_edits_show_up_without_restart() {
    let mut s = srv();
    render(&s, "previz");
    s.f.ws.set_fields("s02_well", &set(&[("duration", json!(2))])).unwrap(); // as if from another terminal
    let st = jcall(&s, "GET", "/api/status", None).1;
    assert_eq!(previz_states(&st)["s02_well"], "stale");
}

#[test]
fn cut_over_http_links_outputs() {
    let s = srv();
    render(&s, "previz");
    let (code, info) = jcall(&s, "POST", "/api/cut", Some(json!({"quality": "best", "video": false})));
    assert_eq!(code, 200, "{info}");
    assert_eq!(info["shots"], 6);
    assert_eq!(info["storyboard"], "/files/cut/best/index.html");
    assert_eq!(call(&s, "GET", "/files/cut/best/index.html", None, &[]).code, 200);
}

#[test]
fn files_serve_byte_ranges_and_revalidate() {
    let s = srv();
    render(&s, "previz");
    jcall(&s, "POST", "/api/cut", Some(json!({"quality": "best", "video": false})));
    let whole = call(&s, "GET", "/files/cut/best/index.html", None, &[]);
    assert_eq!(whole.code, 200);
    assert_eq!(whole.header("accept-ranges"), "bytes");
    let n = whole.body.len();
    let part = call(&s, "GET", "/files/cut/best/index.html", None, &[("Range", "bytes=10-19")]);
    assert_eq!(part.code, 206);
    assert_eq!(part.header("content-range"), format!("bytes 10-19/{n}"));
    assert_eq!(part.body, whole.body[10..20]);
    let tail = call(&s, "GET", "/files/cut/best/index.html", None, &[("Range", "bytes=-5")]);
    assert_eq!(tail.body, whole.body[n - 5..]);
    assert_eq!(call(&s, "GET", "/files/cut/best/index.html", None, &[("Range", &format!("bytes={n}-"))]).code, 416);
    let tag = whole.header("etag").to_string();
    assert_eq!(call(&s, "GET", "/files/cut/best/index.html", None, &[("If-None-Match", &tag)]).code, 304);
}

#[test]
fn log_revalidates_like_status() {
    let s = srv();
    let first = call(&s, "GET", "/api/log?n=5", None, &[]);
    assert_eq!(first.code, 200);
    let tag = first.header("etag").to_string();
    assert_eq!(call(&s, "GET", "/api/log?n=5", None, &[("If-None-Match", &tag)]).code, 304);
    render(&s, "previz"); // writes state.json: the history changed
    assert_eq!(call(&s, "GET", "/api/log?n=5", None, &[("If-None-Match", &tag)]).code, 200);
}

// --------------------------------------------------------------------------- chat (revise)

struct Recorder {
    payload: Value,
    calls: std::sync::Arc<std::sync::Mutex<Vec<Value>>>,
}

impl shotpilot::ai::Claude for Recorder {
    fn ask(&self, request: &Value) -> shotpilot::error::Result<String> {
        self.calls.lock().unwrap().push(request.clone());
        Ok(self.payload.to_string())
    }
}

fn srv_with_claude(payload: Value) -> (Srv, std::sync::Arc<std::sync::Mutex<Vec<Value>>>) {
    let f = ws();
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let c = calls.clone();
    let factory: shotpilot::server::ClaudeFactory = Box::new(move |_| {
        Ok(Box::new(Recorder { payload: payload.clone(), calls: c.clone() }) as Box<dyn shotpilot::ai::Claude>)
    });
    let s = Server::bind_with(&f.path(), "127.0.0.1", 0, factory, None).unwrap();
    let port = s.port();
    s.spawn();
    (Srv { port, f }, calls)
}

const S03_CLOSER: &str = "id: s03_window
purpose: 2階の窓を見上げる。人影が一瞬だけ映る
duration: 4
camera:
  anchor: window
  offset: [-400, -300, -280]
  move: {type: crane_up, distance: 80}
  lens: {focal_length: 85}
environment: {time_of_day: night}
";

#[test]
fn chat_note_on_a_frame_edits_only_that_shot() {
    let (s, calls) = srv_with_claude(json!({
        "summary": "寄りました", "edits": [{"id": "s03_window", "yaml": S03_CLOSER, "reason": "r"}], "order": []}));
    render(&s, "previz");
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["ai"]["ready"], true);
    let frames = st["shots"][2]["thumbs"]["previz"]["frames"].as_array().unwrap().clone();
    assert!(!frames.is_empty());
    let before = s.f.text();

    let (code, out) = jcall(&s, "POST", "/api/revise", Some(json!({
        "note": "もっと寄って", "shot": "s03_window",
        "frame": {"url": frames[0], "index": 0, "count": frames.len(), "time": 0.0},
        "region": [0.1, 0.2, 0.4, 0.6],
    })));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["stale"], json!(["s03_window"]));
    assert_eq!(out["summary"], "寄りました");
    assert!(out["changes"]["s03_window"].as_array().unwrap().iter().any(|c| c["path"] == "camera.lens.focal_length"));
    // only s03 went stale; the other five keep their previz takes
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    let stale: Vec<String> = previz_states(&st).into_iter().filter(|(_, v)| v == "stale").map(|(k, _)| k).collect();
    assert_eq!(stale, vec!["s03_window".to_string()]);
    assert_ne!(before, s.f.text());

    // Claude saw the frame and where the director pointed, and was scoped to that shot
    let req = calls.lock().unwrap()[0].clone();
    let content = req["messages"][0]["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "image");
    assert_eq!(content[0]["source"]["media_type"], "image/png");
    let text = content[1]["text"].as_str().unwrap();
    assert!(text.contains("You may only edit these shots: s03_window"));
    assert!(text.contains("x 10-40% and y 20-60%"));

    // the note and Claude's summary are kept in the history the chat shows
    let (_, log) = jcall(&s, "GET", "/api/log", None);
    let last = log.as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["action"], "revise");
    assert_eq!(last["note"], "もっと寄って");
    assert_eq!(last["summary"], "寄りました");
}

#[test]
fn chat_scope_is_enforced_over_http() {
    let (s, _) = srv_with_claude(json!({
        "summary": "x", "edits": [{"id": "s03_window", "yaml": S03_CLOSER, "reason": "r"}], "order": []}));
    let before = s.f.text();
    let (code, out) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "直して", "shot": "s01_gate"})));
    assert_eq!(code, 400);
    assert!(out["error"].as_str().unwrap().contains("outside the allowed scope"));
    assert_eq!(before, s.f.text());
    let (code, _) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "  "})));
    assert_eq!(code, 400);
}

#[test]
fn cut_sheet_edits_and_undo_over_http() {
    let (s, _) = srv_with_claude(json!({
        "summary": "寄った", "edits": [{"id": "s03_window", "yaml": S03_CLOSER, "reason": "r"}], "order": []}));
    let original = s.f.text();
    // one tap: only that shot goes stale, and the status offers to take it back
    let (code, out) = jcall(&s, "POST", "/api/nudge", Some(json!({"shot": "s01_gate", "kind": "tighter"})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["stale"], json!(["s01_gate"]));
    assert_eq!(out["changes"]["s01_gate"][0]["path"], "camera.lens.focal_length");
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["undo"]["action"], "nudge");
    assert_eq!(st["undo"]["shots"], json!(["s01_gate"]));
    let (code, out) = jcall(&s, "POST", "/api/undo", Some(json!({})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["undid"]["action"], "nudge");
    assert_eq!(s.f.text(), original);
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert!(st["undo"].is_null());
    let (code, _) = jcall(&s, "POST", "/api/undo", Some(json!({})));
    assert_eq!(code, 400);

    // a note pointed at the camera keeps the rest of the shot (S03_CLOSER drops its sound)
    let (code, out) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "寄って", "shot": "s03_window", "aspect": "camera"})));
    assert_eq!(code, 200, "{out}");
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    let s03 = &st["shots"][2]["node"];
    assert_eq!(s03["camera"]["lens"]["focal_length"], 85);
    assert!(s03["sfx"].as_array().is_some_and(|a| !a.is_empty()), "{s03}");
    let (code, out) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "寄って", "aspect": "camera"})));
    assert_eq!(code, 400, "{out}");
    let (code, _) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "寄って", "shot": "s03_window", "aspect": "nope"})));
    assert_eq!(code, 400);

    let (code, out) = jcall(&s, "POST", "/api/script", Some(json!({"id": "nope", "set": {"ja": "x"}})));
    assert_eq!(code, 400);
    assert!(out["error"].as_str().unwrap().contains("unknown script line"));
    let (code, _) = jcall(&s, "POST", "/api/nudge", Some(json!({"shot": "s01_gate", "kind": "sideways"})));
    assert_eq!(code, 400);
}

#[test]
fn a_game_change_shows_in_status_and_renders_only_its_shots() {
    let s = srv();
    let content = s.f.ws.root.join("content");
    std::fs::create_dir_all(&content).unwrap();
    std::fs::write(content.join("well.png"), "v1").unwrap();
    let (acts, _) = render(&s, "still");
    assert!(acts.values().all(|a| a == "rendered"));
    // the app polls with the ETag: unchanged game, 304; a new texture, a fresh status
    let tag = call(&s, "GET", "/api/status", None, &[]).header("etag").to_string();
    assert_eq!(call(&s, "GET", "/api/status", None, &[("If-None-Match", &tag)]).code, 304);
    std::fs::write(content.join("well.png"), "v2, repainted").unwrap();
    assert_eq!(call(&s, "GET", "/api/status", None, &[("If-None-Match", &tag)]).code, 200);
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    let changed: Vec<String> = st["shots"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["changed"]["still"].is_array())
        .map(|x| x["id"].as_str().unwrap().to_string())
        .collect();
    assert!(changed.contains(&"s02_well".to_string()), "{changed:?}");
    assert_eq!(st["shots"][1]["changed"]["still"], json!(["/Game/Mock/well"]));
    let (acts, events) = render(&s, "still");
    let plan = &events.iter().find(|(e, _)| e == "plan").unwrap().1;
    let mut why: Vec<String> = plan["game_changed"].as_object().unwrap().keys().cloned().collect();
    why.sort();
    let mut rendered: Vec<String> = acts.iter().filter(|(_, a)| *a == "rendered").map(|(k, _)| k.clone()).collect();
    rendered.sort();
    assert_eq!(rendered, why);
    assert_eq!(rendered, changed);
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert!(st["shots"].as_array().unwrap().iter().all(|x| x["changed"].as_object().unwrap().is_empty()));
}

#[test]
fn settings_save_the_key_but_never_send_it_back() {
    let f = ws();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let factory: shotpilot::server::ClaudeFactory = Box::new(|s| match s.provider {
        shotpilot::connect::Provider::ApiKey if s.api_key.is_some() => {
            Ok(Box::new(Recorder { payload: json!({}), calls: Default::default() }) as Box<dyn shotpilot::ai::Claude>)
        }
        _ => Err(shotpilot::error::Error::user("not connected")),
    });
    let srv = Server::bind_with(&f.path(), "127.0.0.1", 0, factory, Some(path.clone())).unwrap();
    let port = srv.port();
    srv.spawn();
    let s = Srv { port, f };

    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["ai"]["ready"], false);
    assert_eq!(st["ai"]["error"], "not connected");

    let (code, out) = jcall(&s, "POST", "/api/settings", Some(json!({"provider": "api-key", "api_key": "not-a-key"})));
    assert_eq!(code, 400, "{out}");
    let (code, out) = jcall(&s, "POST", "/api/settings", Some(json!({"provider": "api-key", "api_key": "sk-ant-test-9876"})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["ready"], true);
    assert_eq!(out["provider"], "api-key");
    assert_eq!(out["api_key"], "…9876");
    assert!(!out.to_string().contains("sk-ant-test"));
    assert!(std::fs::read_to_string(&path).unwrap().contains("sk-ant-test-9876"));

    // status picks the change up (the settings file is part of the ETag)
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["ai"]["ready"], true);
    assert!(!st.to_string().contains("sk-ant-test"));

    // an empty key removes it
    let (_, out) = jcall(&s, "POST", "/api/settings", Some(json!({"api_key": ""})));
    assert_eq!(out["ready"], false);
    assert!(out["api_key"].is_null());
    // effort: one of the levels, saved; empty goes back to the default
    assert_eq!(out["effort"], "medium");
    let (code, out) = jcall(&s, "POST", "/api/settings", Some(json!({"effort": "turbo"})));
    assert_eq!(code, 400, "{out}");
    let (code, out) = jcall(&s, "POST", "/api/settings", Some(json!({"effort": "high"})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["effort"], "high");
    assert!(std::fs::read_to_string(&path).unwrap().contains("\"high\""));
    let (_, out) = jcall(&s, "POST", "/api/settings", Some(json!({"effort": ""})));
    assert_eq!(out["effort"], "medium");
}

// ------------------------------------------------------------------ sound

#[test]
fn sound_lanes_and_edits_over_http() {
    let s = srv();
    shotpilot::sound::write_demo_audio(&s.f.ws.root).unwrap();
    let (code, snd) = jcall(&s, "GET", "/api/sound", None);
    assert_eq!(code, 200, "{snd}");
    assert_eq!(snd["placements"].as_array().unwrap().len(), 4);
    assert_eq!(snd["mix"]["lanes"].as_array().unwrap().len(), 4);
    assert!(snd["mix"]["url"].as_str().unwrap().starts_with("/files/cut/sound/mix.wav"));
    assert!(snd["mix"]["check"]["verdict"].is_string());
    assert_eq!(snd["cuts"].as_array().unwrap().len(), 5);
    // source files are served for previewing
    let r = call(&s, "GET", "/files/audio/hit.wav", None, &[]);
    assert_eq!((r.code, r.header("content-type")), (200, "audio/wav"));

    // move the silhouette hit from s03 to s04, as the lanes do after a drag
    let before = launches(&s.f.ws).len();
    let (code, out) = jcall(&s, "POST", "/api/sound", Some(json!({
        "sfx": {"s03_window": [], "s04_lantern": [{"file": "audio/hit.wav", "at": 0.5, "label": "silhouette"}]},
        "note": "drag"})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["stale"], json!([]));
    assert_eq!(out["edited"], json!(["s03_window", "s04_lantern"]));
    let p = out["sound"]["placements"].as_array().unwrap().iter().find(|p| p["label"] == "silhouette").unwrap().clone();
    assert_eq!(p["shot"], "s04_lantern");
    assert_eq!(launches(&s.f.ws).len(), before);
    let text = s.f.text();
    assert!(text.contains("- {file: audio/hit.wav, at: 0.5, label: silhouette}"), "{text}");

    // BGM off and back on
    let (code, out) = jcall(&s, "POST", "/api/sound", Some(json!({"audio": {"gain": 0}})));
    assert_eq!(code, 200, "{out}");
    assert!(out["audio"].as_bool().unwrap());
    assert!(out["sound"]["placements"].as_array().unwrap().iter().all(|p| p["role"] == "sfx"));
    let (code, out) = jcall(&s, "POST", "/api/sound", Some(json!({"audio": {"gain": 0, "bgm": {"file": "../x.wav"}}})));
    assert_eq!(code, 400, "{out}");
}

#[test]
fn a_sound_note_over_http_changes_only_sound() {
    let (s, calls) = srv_with_claude(json!({
        "summary": "hit", "edits": [{"id": "s03_window", "yaml": format!("{S03_CLOSER}sfx:\n  - {{file: audio/hit.wav, at: 0}}\n"), "reason": "r"}],
        "order": [], "audio": ""}));
    shotpilot::sound::write_demo_audio(&s.f.ws.root).unwrap();
    let (code, out) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "頭にヒット", "shot": "s03_window", "sound": true, "sfx_index": 0})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["stale"], json!([]), "the camera change must not come through a sound note");
    let changes = out["changes"]["s03_window"].as_array().unwrap();
    assert!(changes.iter().all(|c| c["path"].as_str().unwrap().starts_with("sfx")), "{changes:?}");
    let req = calls.lock().unwrap()[0].to_string();
    assert!(req.contains("working on the sound") && req.contains("sound check against 7 launch films"), "{req}");
}

#[test]
fn brief_checks_and_approval_over_http() {
    let s = srv();
    let (code, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(code, 200);
    assert_eq!(st["brief"]["kind"], json!("teaser"));
    assert!(st["checks"].is_array());
    assert_eq!(st["shots"][2]["approval"], json!("none"));

    let (code, b) = jcall(&s, "POST", "/api/brief", Some(json!({"set": {"core": "中庭が少しずつ変わる"}})));
    assert_eq!(code, 200, "{b}");
    assert_eq!(b["core"], json!("中庭が少しずつ変わる"));

    let (code, _) = jcall(&s, "POST", "/api/approve", Some(json!({"shots": ["s03_window"], "approved": true})));
    assert_eq!(code, 200);
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["shots"][2]["approval"], json!("approved"));

    // a P0 (opening on the title) is a 400 with the reason, and approves nothing
    let order = json!({"order": ["s06_title", "s01_gate", "s02_well", "s03_window", "s04_lantern", "s05_door"]});
    assert_eq!(jcall(&s, "POST", "/api/order", Some(order)).0, 200);
    let (code, err) = jcall(&s, "POST", "/api/approve", Some(json!({"shots": ["s06_title"]})));
    assert_eq!(code, 400);
    assert!(err["error"].as_str().unwrap().contains("open.title"), "{err}");
}

#[test]
fn plan_over_http_needs_confirmation_and_logs_the_check() {
    let shot = |id: &str, beat: &str, anchor: &str| json!({"id": id, "beat": beat, "purpose": "p", "duration": 7,
        "anchor": anchor, "offset": [-300, 0, 120], "look_at": "", "focal_length": 35, "move": "static",
        "move_amount": 0, "time_of_day": "default", "notes": ""});
    let (s, calls) = srv_with_claude(json!({"rationale": "静かに始めて、最後に崩す", "shots": [
        shot("p_title", "title", "well"), shot("p_hook", "hook", "gate"), shot("p_est", "establish", "well"),
        shot("p_esc", "escalate", "door")]}));
    // existing shots are not replaced without confirmation, and Claude is not called
    let (code, err) = jcall(&s, "POST", "/api/plan", Some(json!({"note": "静かに"})));
    assert_eq!(code, 400, "{err}");
    assert!(calls.lock().unwrap().is_empty());

    let (code, out) = jcall(&s, "POST", "/api/plan", Some(json!({"note": "静かに", "replace": true})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["shots"], json!(["p_hook", "p_est", "p_esc", "p_title"])); // code put the beats in order
    assert!(out["checks"]["P0"].is_number());
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["shots"].as_array().unwrap().len(), 4);
    let (_, log) = jcall(&s, "GET", "/api/log?n=1", None);
    assert_eq!(log[0]["action"], json!("plan"));
    assert_eq!(log[0]["rationale"], json!("静かに始めて、最後に崩す"));
    assert_eq!(log[0]["note"], json!("静かに"));
}

struct OnePng;
impl shotpilot::moodboard::ImageGen for OnePng {
    fn generate(&self, _job: &shotpilot::moodboard::ImageJob) -> shotpilot::error::Result<Vec<u8>> {
        Ok(vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3])
    }
}

#[test]
fn mood_board_over_http() {
    let f = ws();
    let srv = Server::bind_with(&f.path(), "127.0.0.1", 0, shotpilot::server::default_claude(), None).unwrap()
        .with_images(Box::new(|_| Ok(Box::new(OnePng) as Box<dyn shotpilot::moodboard::ImageGen>)));
    let port = srv.port();
    srv.spawn();
    let s = Srv { port, f };
    let (code, out) = jcall(&s, "POST", "/api/moodboard/generate", Some(json!({"direction": "霧", "n": 2})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["added"].as_array().unwrap().len(), 2);
    let first = out["added"][0].as_str().unwrap().to_string();
    let r = call(&s, "GET", &format!("/files/{first}"), None, &[]);
    assert_eq!(r.code, 200);
    assert_eq!(r.header("content-type"), "image/png");

    let data = format!("data:image/png;base64,{}", shotpilot::ai::base64(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 9]));
    let (code, out) = jcall(&s, "POST", "/api/moodboard/upload", Some(json!({"name": "ref.png", "data": data})));
    assert_eq!(code, 200, "{out}");
    let (code, _) = jcall(&s, "POST", "/api/moodboard/upload", Some(json!({"name": "x.png", "data": "aGVsbG8="})));
    assert_eq!(code, 400);
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["brief"]["images"].as_array().unwrap().len(), 3);
    let (code, out) = jcall(&s, "POST", "/api/moodboard/remove", Some(json!({"image": first})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["images"].as_array().unwrap().len(), 2);
    // outside moodboard/ is never served
    assert_eq!(call(&s, "GET", "/files/moodboard/../shots.yaml", None, &[]).code, 404);
}

#[test]
fn storyboard_over_http() {
    let f = ws();
    let srv = Server::bind_with(&f.path(), "127.0.0.1", 0, shotpilot::server::default_claude(), None).unwrap()
        .with_images(Box::new(|_| Ok(Box::new(OnePng) as Box<dyn shotpilot::moodboard::ImageGen>)));
    let port = srv.port();
    srv.spawn();
    let s = Srv { port, f };
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert!(st["shots"].as_array().unwrap().iter().all(|x| x["storyboard"].is_null()));
    let (code, out) = jcall(&s, "POST", "/api/storyboard", Some(json!({"shots": ["s02_well"]})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["made"], json!(["s02_well"]));
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    let well = st["shots"].as_array().unwrap().iter().find(|x| x["id"] == "s02_well").unwrap().clone();
    assert_eq!(well["storyboard"]["fresh"], true);
    let url = well["storyboard"]["url"].as_str().unwrap();
    assert!(url.starts_with("/files/storyboard/s02_well/"), "{url}");
    assert_eq!(call(&s, "GET", url, None, &[]).code, 200);
    // painting a storyboard frame never renders a take
    assert!(st["shots"].as_array().unwrap().iter().all(|x| x["thumbs"]["previz"]["url"].is_null()));
}

#[test]
fn library_lists_plays_and_imports_without_rendering() {
    let f = ws();
    let lib = tempfile::tempdir().unwrap();
    let sounds = lib.path().join("sounds");
    std::fs::create_dir_all(sounds.join("splice/Kit")).unwrap();
    std::fs::write(sounds.join("splice/Kit/boom.wav"), b"RIFF-boom").unwrap();
    std::fs::write(sounds.join("splice/Kit/drone.wav"), b"RIFF-drone").unwrap();
    std::fs::write(sounds.join("library.json"), json!({"sounds": [
        {"id": "splice:b1", "path": "splice/Kit/boom.wav", "kind": "oneshot", "duration_s": 1.0, "tags": ["impact"], "status": "ok"},
        {"id": "splice:d1", "path": "splice/Kit/drone.wav", "kind": "loop", "duration_s": 40.0, "tags": ["drones", "horror"], "status": "ok"},
    ]}).to_string()).unwrap();
    let srv = Server::bind(&f.path(), "127.0.0.1", 0).unwrap().with_library(Some(lib.path().to_path_buf()));
    let s = Srv { port: srv.port(), f };
    srv.spawn();

    let (code, out) = jcall(&s, "GET", "/api/library?q=horror&kind=bgm", None);
    assert_eq!(code, 200, "{out}");
    assert_eq!((out["total"].as_u64(), out["bgm"].as_u64()), (Some(2), Some(1)));
    let items = out["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "splice:d1");
    assert_eq!(items[0]["imported"], false);
    // audition straight from the library
    let url = items[0]["url"].as_str().unwrap();
    let r = call(&s, "GET", url, None, &[]);
    assert_eq!((r.code, r.body.as_slice()), (200, b"RIFF-drone".as_slice()));
    assert_eq!(call(&s, "GET", "/library/..%2Fsounds%2Flibrary.json", None, &[]).code, 404);

    let before = launches(&s.f.ws).len();
    let (code, out) = jcall(&s, "POST", "/api/library/import", Some(json!({"id": "splice:b1"})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["file"], "audio/lib/boom.wav");
    assert!(out["sound"]["files"].as_array().unwrap().iter().any(|x| x["file"] == "audio/lib/boom.wav"));
    assert_eq!(launches(&s.f.ws).len(), before);
    let (_, out) = jcall(&s, "GET", "/api/library?q=impact", None);
    assert_eq!(out["items"][0]["imported"], true);
    let (code, _) = jcall(&s, "POST", "/api/library/import", Some(json!({"id": "splice:zz"})));
    assert_eq!(code, 400);
}

#[test]
fn home_creates_opens_and_switches_projects() {
    let tmp = tempfile::tempdir().unwrap();
    let srv = Server::bind_opt(None, "127.0.0.1", 0, shotpilot::server::default_claude(), None)
        .unwrap()
        .with_recents(Some(tmp.path().join("projects.json")));
    let s = Srv { port: srv.port(), f: ws() };
    srv.spawn();

    // nothing open: the studio sends you home, and project routes say so
    let r = call(&s, "GET", "/", None, &[]);
    assert_eq!((r.code, r.header("location")), (302, "/home"));
    assert_eq!(call(&s, "GET", "/home", None, &[]).code, 200);
    assert_eq!(jcall(&s, "GET", "/api/status", None).0, 400);
    let (_, home) = jcall(&s, "GET", "/api/home", None);
    assert!(home["current"].is_null());
    assert_eq!(home["projects"].as_array().unwrap().len(), 0);

    // a new sample project opens at once
    let loc = tmp.path().join("projects");
    let (code, out) = jcall(&s, "POST", "/api/projects/create",
        Some(json!({"location": loc.display().to_string(), "name": "demo", "sample": true})));
    assert_eq!(code, 200, "{out}");
    let demo = out["path"].as_str().unwrap().to_string();
    assert_eq!(call(&s, "GET", "/", None, &[]).code, 200);
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["project"], "demo");
    let etag_demo = call(&s, "GET", "/api/status", None, &[]).header("etag").to_string();

    // its card shows what is shot, with cover frames served from its render/ only
    render(&s, "previz");
    let (_, home) = jcall(&s, "GET", "/api/home", None);
    let card = &home["projects"][0];
    assert_eq!((card["shots"].as_u64(), card["taken"].as_u64(), card["current"].as_bool()), (Some(6), Some(6), Some(true)));
    let cover = card["covers"][0]["url"].as_str().unwrap();
    let r = call(&s, "GET", cover, None, &[]);
    assert_eq!((r.code, r.header("content-type")), (200, "image/png"));
    let yaml = format!("/home/cover?p={}", demo.replace(' ', "%20"));
    assert_eq!(call(&s, "GET", &yaml, None, &[]).code, 404);

    // opening another project switches the studio; both stay on the list
    let (code, _) = jcall(&s, "POST", "/api/projects/open", Some(json!({"path": s.f.path().display().to_string()})));
    assert_eq!(code, 200);
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["project"], "t");
    let r = call(&s, "GET", "/api/status", None, &[("If-None-Match", &etag_demo)]);
    assert_eq!(r.code, 200, "a switched project never answers 304");
    let (_, home) = jcall(&s, "GET", "/api/home", None);
    assert_eq!(home["projects"].as_array().unwrap().len(), 2);
    assert_eq!(home["projects"][0]["name"], "t");

    // forgetting drops it from the list, not from disk
    let (code, _) = jcall(&s, "POST", "/api/projects/forget", Some(json!({"path": demo})));
    assert_eq!(code, 200);
    let (_, home) = jcall(&s, "GET", "/api/home", None);
    assert_eq!(home["projects"].as_array().unwrap().len(), 1);
    assert!(std::path::Path::new(&demo).is_file());
    // without a native picker the home falls back to typed paths
    assert_eq!(jcall(&s, "POST", "/api/projects/pick", Some(json!({"kind": "open"}))).0, 400);
}

#[test]
fn the_planning_turn_runs_over_http() {
    // the catalogues for the app
    let s = srv();
    let (code, kits) = jcall(&s, "GET", "/api/kits", None);
    assert_eq!(code, 200);
    assert_eq!(kits["design_systems"].as_array().unwrap().len(), 12);
    assert_eq!(kits["skills"].as_array().unwrap().len(), 18);

    // a plan lands in the brief, with the form answers handed to Claude
    let (s, calls) = srv_with_claude(common::discovery_reply());
    let answers = json!([{"id": "genre", "label": "ジャンル", "value": "horror"}, {"id": "show", "label": "見せたいもの", "value": ["well", "door"]}]);
    let (code, out) = jcall(&s, "POST", "/api/discover", Some(json!({"request": "規則のホラー", "answers": answers})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["route"], "plan");
    assert_eq!(out["design"], "quiet-dread");
    let user = calls.lock().unwrap()[0]["messages"][0]["content"].as_str().unwrap().to_string();
    assert!(user.contains("[form answers — discovery]") && user.contains("見せたいもの [show]: well, door"), "{user}");
    // the chat shows the turn from the history
    let (_, log) = jcall(&s, "GET", "/api/log", None);
    let last = log.as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["action"], "discover");
    assert_eq!(last["note"], "規則のホラー");
    assert_eq!(last["route"], "plan");
    let text = std::fs::read_to_string(s.f.path()).unwrap();
    assert!(text.contains("design: quiet-dread") && text.contains("rules-teaser"), "{text}");
}

#[test]
fn concept_frames_find_their_places_over_http() {
    let (mut s, calls) = srv_with_claude(json!({"matches": [{"frame": "c1", "photo": "well/front", "fit": "good", "why": "井戸"}]}));
    s.f.ws.take_scout(&[]).unwrap();
    let png: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0];
    let root = s.f.ws.root.clone();
    let mut frames = shotpilot::concept::replace(&root, &[("hook".into(), "暗い井戸".into())]).unwrap();
    frames[0].file = Some(shotpilot::concept::store(&root, &frames[0], png).unwrap());
    shotpilot::concept::save(&root, &frames).unwrap();

    let (code, m) = jcall(&s, "POST", "/api/concept/match", Some(json!({})));
    assert_eq!(code, 200, "{m}");
    assert_eq!(m["found"], 1);
    assert_eq!(m["open"], json!([]));
    assert_eq!(calls.lock().unwrap().len(), 1); // one comparison, nothing else to try
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["concept"][0]["found"]["anchor"], "well");
    let file = st["concept"][0]["file"].as_str().unwrap().to_string();
    assert_eq!(call(&s, "GET", &format!("/files/{file}"), None, &[]).code, 200);
}

/// s03 with a caption and a flash into the next shot, and a camera that must not land.
const S03_FINISHED: &str = "id: s03_window
purpose: 2階の窓を見上げる。人影が一瞬だけ映る
duration: 4
camera:
  anchor: window
  offset: [-400, -300, -280]
  lens: {focal_length: 85}
environment: {time_of_day: night}
caption: {text: 誰かいる, at: 1.0}
transition: flash
";

/// A server over the demo with a script (a card after s02, a count over s03).
fn srv_with_script(payload: Value) -> (Srv, std::sync::Arc<std::sync::Mutex<Vec<Value>>>) {
    let (s, calls) = srv_with_claude(payload);
    let mut text = s.f.text();
    text.push_str("\nscript:\n  - {id: seven, kind: card, ja: 七人。, en: SEVEN, dur: 2, after: s02_well}\n  - {id: cups, kind: count, ja: コップ、七つ。, on: s03_window, at: 0.5}\n");
    s.f.write(&text);
    (s, calls)
}

#[test]
fn finishing_notes_over_http() {
    // one cut: its caption and hand-over and the text over it, never its camera; nothing re-renders
    let (s, calls) = srv_with_script(json!({"summary": "白く飛ばした", "order": [],
        "edits": [{"id": "s03_window", "yaml": S03_FINISHED, "reason": "r"}], "texts": [{"id": "cups", "yaml": "ja: コップ、八つ。"}]}));
    let (code, out) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "白く飛ばして", "shot": "s03_window", "aspect": "finish"})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["stale"], json!([]));
    assert_eq!(out["changes"]["cups"][0]["path"], "ja");
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    let s03 = &st["shots"][2]["node"];
    assert_eq!((s03["transition"].as_str(), s03["camera"]["lens"]["focal_length"].as_i64()), (Some("flash"), Some(50)), "{s03}");
    assert_eq!(st["script"][1]["ja"], "コップ、八つ。");
    let user = calls.lock().unwrap()[0]["messages"][0]["content"].to_string();
    assert!(user.contains("script line(s) cups may change"), "{user}");
    // the card is not that cut's text
    let (code, out) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "x", "shot": "s01_gate", "aspect": "finish"})));
    assert_eq!(code, 400, "{out}");

    // one card: its words only; a shot Claude sends back is refused
    let (s, _) = srv_with_script(json!({"summary": "", "order": [], "edits": [], "texts": [{"id": "seven", "yaml": "ja: 自分を入れて、七人。\ndur: 3"}]}));
    let (code, out) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "言い切って", "line": "seven", "aspect": "finish"})));
    assert_eq!(code, 200, "{out}");
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!((st["script"][0]["ja"].as_str(), st["script"][0]["dur"].as_f64()), (Some("自分を入れて、七人。"), Some(3.0)));
    let (code, _) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "x", "line": "seven", "shot": "s03_window", "aspect": "finish"})));
    assert_eq!(code, 400);
    let (s, _) = srv_with_script(json!({"summary": "", "order": [], "edits": [{"id": "s03_window", "yaml": S03_FINISHED, "reason": "r"}]}));
    let (code, out) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "x", "line": "seven", "aspect": "finish"})));
    assert_eq!(code, 400, "{out}");

    // the whole teaser: every line and the grade, still no camera and no render
    let (s, _) = srv_with_script(json!({"summary": "", "order": [], "finish": "grade: cool",
        "edits": [{"id": "s03_window", "yaml": S03_FINISHED, "reason": "r"}], "texts": [{"id": "seven", "yaml": "en: ''"}]}));
    let (code, out) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "冷たく", "aspect": "finish", "all": true})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["stale"], json!([]));
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!(st["finish"]["grade"], "cool");
    assert_eq!(st["script"][0]["en"], "");
    assert_eq!(st["shots"][2]["node"]["camera"]["lens"]["focal_length"], 50);
    assert_eq!(st["undo"]["action"], "revise");
    let (code, _) = jcall(&s, "POST", "/api/undo", Some(json!({})));
    assert_eq!(code, 200);
    let (_, st) = jcall(&s, "GET", "/api/status", None);
    assert_eq!((st["finish"]["grade"].as_str(), st["script"][0]["en"].as_str()), (Some("none"), Some("SEVEN")));

    // a camera note keeps the script's words
    let (s, _) = srv_with_script(json!({"summary": "", "order": [], "edits": [], "texts": [{"id": "cups", "yaml": "ja: x"}]}));
    let (code, _) = jcall(&s, "POST", "/api/revise", Some(json!({"note": "x", "shot": "s03_window", "aspect": "camera"})));
    assert_eq!(code, 400);
}

#[test]
fn the_pages_switch_to_english_with_a_dictionary() {
    // the pages load the English mode; it decides from the PC's language and keeps Japanese on a Japanese PC
    let s = srv();
    for page in ["/", "/home"] {
        let r = call(&s, "GET", page, None, &[]);
        assert!(String::from_utf8_lossy(&r.body).contains(r#"<script src="/i18n.js"></script>"#), "{page}");
    }
    let r = call(&s, "GET", "/i18n.js", None, &[]);
    assert_eq!(r.code, 200);
    assert!(r.header("content-type").starts_with("text/javascript"));
    let js = String::from_utf8_lossy(&r.body).to_string();
    assert!(js.starts_with("const SP_EN = {") && js.contains("window.SP_LANG"));

    // every entry is English, with the same run-time holes as its Japanese key
    let dict: serde_json::Map<String, Value> = serde_json::from_str(shotpilot::server::I18N_EN).unwrap();
    assert!(dict.len() > 500, "{}", dict.len());
    fn holes(s: &str) -> Vec<&str> {
        let mut v: Vec<&str> = s.match_indices('{').filter_map(|(i, _)| s[i..].find('}').map(|j| &s[i..=i + j])).filter(|h| h[1..h.len() - 1].chars().all(|c| c.is_ascii_digit())).collect();
        v.sort();
        v
    }
    let kana = |s: &str| s.chars().any(|c| ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c));
    for (k, v) in &dict {
        let v = v.as_str().unwrap_or("");
        assert!(!v.trim().is_empty(), "empty: {k}");
        assert_eq!(holes(k), holes(v), "holes differ: {k} => {v}");
        // a key is Japanese, or a pattern that joins translated parts ("{0} +{1}s")
        assert!(kana(k) || !holes(k).is_empty(), "not Japanese: {k}");
    }
    for key in ["一気に作る", "カット表", "撮った画に被写体が写っていない"] {
        assert!(dict.contains_key(key), "missing {key}");
    }
}

#[test]
fn the_release_plan_is_a_checklist_the_app_ticks() {
    let s = srv();
    let v = call(&s, "GET", "/api/promo", None, &[]).json();
    assert_eq!(v["videos"].as_array().unwrap().len(), 3);
    assert_eq!(v["saved"], false);
    let r = call(&s, "POST", "/api/promo", Some(json!({"op": "check", "video": "v1", "check": "mute", "done": true})), &[]);
    assert_eq!(r.code, 200);
    let here = r.json()["videos"].as_array().unwrap().iter().find(|x| x["id"] == "v1").unwrap().clone();
    assert_eq!(here["checks"].as_array().unwrap().iter().find(|c| c["id"] == "mute").unwrap()["done"], true);
    // made-in-code checks cannot be ticked by hand
    let r = call(&s, "POST", "/api/promo", Some(json!({"op": "check", "video": "v1", "check": "cut", "done": true})), &[]);
    assert_eq!(r.code, 400);
    let r = call(&s, "POST", "/api/promo", Some(json!({"op": "add", "kind": "event", "title": "", "due": "2027-06-20"})), &[]);
    let id = r.json()["result"]["id"].as_str().unwrap().to_string();
    let row = r.json()["videos"].as_array().unwrap().iter().find(|x| x["id"] == id.as_str()).unwrap().clone();
    assert_eq!((row["title"].as_str(), row["due"].as_str()), (Some("セール・イベント告知"), Some("2027-06-20")));
    // the stage badge comes with the status
    let st = call(&s, "GET", "/api/status", None, &[]).json();
    assert_eq!((st["promo"]["videos"].as_i64(), st["promo"]["saved"].as_bool()), (Some(4), Some(true)));
    // promo.json is editorial: nothing went stale and nothing launched
    assert!(launches(&s.f.ws).is_empty());
}

#[test]
fn a_look_picked_in_the_app_brings_its_faces_and_motion_and_a_logo_can_be_named() {
    // D-52 / D-53: picking a look writes its typefaces, title size and motion; the logo is a
    // file inside the project (the app's dialog copies one from outside into art/)
    let s = srv();
    let (code, _) = jcall(&s, "POST", "/api/brief", Some(json!({"set": {"design": "neon-action"}})));
    assert_eq!(code, 200);
    let st = call(&s, "GET", "/api/status", None, &[]).json();
    assert_eq!(st["finish"]["fonts"]["display"], json!("Dela Gothic One:400"));
    assert_eq!(st["finish"]["title_size"], json!(0.15));
    assert_eq!(st["finish"]["motion"]["profile"], json!("action"));
    assert_eq!(st["finish"]["motion"]["energy"], json!("driving"));

    std::fs::create_dir_all(s.f.ws.root.join("art")).unwrap();
    std::fs::write(s.f.ws.root.join("art/logo.png"), b"png").unwrap();
    let (code, out) = jcall(&s, "POST", "/api/finish/logo", Some(json!({"path": "art/logo.png"})));
    assert_eq!(code, 200, "{out}");
    assert_eq!(out["logo"], json!("art/logo.png"));
    let (code, _) = jcall(&s, "POST", "/api/finish/logo", Some(json!({"path": "art/missing.png"})));
    assert_eq!(code, 400);
    // no native dialog behind this server: picking is refused, typing a path is the way
    let (code, out) = jcall(&s, "POST", "/api/finish/logo", Some(json!({"pick": true})));
    assert_eq!(code, 400, "{out}");
    let (_, out) = jcall(&s, "POST", "/api/finish/logo", Some(json!({"clear": true})));
    assert_eq!(out["logo"], Value::Null);
    // editorial: nothing launched
    assert!(launches(&s.f.ws).is_empty());
}
