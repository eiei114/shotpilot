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
