//! Claude integration, with a fake client: scope enforcement and validation are code, not prompt.

mod common;

use std::cell::RefCell;

use common::{ids, ws};
use serde_json::{json, Value};
use shotpilot::ai::{self, Claude};
use shotpilot::Result;

struct FakeClaude {
    payload: Value,
    calls: RefCell<Vec<Value>>,
}

impl FakeClaude {
    fn new(payload: Value) -> Self {
        FakeClaude { payload, calls: RefCell::new(Vec::new()) }
    }
}

impl Claude for FakeClaude {
    fn ask(&self, request: &Value) -> Result<String> {
        self.calls.borrow_mut().push(request.clone());
        Ok(self.payload.to_string())
    }
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
fn revise_edits_only_named_shot() {
    let mut f = ws();
    let fake = FakeClaude::new(json!({"summary": "closer", "edits": [{"id": "s03_window", "yaml": S03_CLOSER, "reason": "r"}],
                                      "order": []}));
    let rev = ai::revise(&f.ws.defn, "3番目はもっと寄って", None, &fake).unwrap();
    assert_eq!(rev.edits.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(), ids(&["s03_window"]));
    assert!(rev.order.is_none());
    let rep = f.ws.edit_definition(&rev.edits, rev.order.as_deref(), "revise", "note").unwrap();
    assert_eq!(rep.stale, ids(&["s03_window"]));
    let req = &fake.calls.borrow()[0];
    assert_eq!(req["model"], "claude-opus-5-5");
    assert_eq!(req["output_config"]["format"]["type"], "json_schema");
    assert_eq!(req["thinking"]["type"], "adaptive");
    assert_eq!(req["fallbacks"], "default");
}

#[test]
fn revise_scope_is_enforced() {
    let f = ws();
    let fake = FakeClaude::new(json!({"summary": "x", "edits": [{"id": "s03_window", "yaml": S03_CLOSER, "reason": "r"}],
                                      "order": []}));
    let err = ai::revise(&f.ws.defn, "note", Some(&ids(&["s01_gate"])), &fake).err().unwrap();
    assert!(err.msg.contains("outside the allowed scope"), "{}", err.msg);
}

#[test]
fn revise_rejects_invalid_shot() {
    let f = ws();
    let bad = S03_CLOSER.replace("anchor: window", "anchor: moon");
    let fake = FakeClaude::new(json!({"summary": "x", "edits": [{"id": "s03_window", "yaml": bad, "reason": "r"}], "order": []}));
    let err = ai::revise(&f.ws.defn, "note", None, &fake).err().unwrap();
    assert!(err.msg.contains("unknown anchor"), "{}", err.msg);
}

#[test]
fn revise_reorder_must_be_permutation() {
    let f = ws();
    let fake = FakeClaude::new(json!({"summary": "x", "edits": [], "order": ["s06_title", "s01_gate"]}));
    let err = ai::revise(&f.ws.defn, "タイトルを先に", None, &fake).err().unwrap();
    assert!(err.msg.contains("permutation"), "{}", err.msg);
}

#[test]
fn plan_builds_valid_shots() {
    let mut f = ws();
    let shot = json!({"id": "S01 Hook!", "beat": "hook", "purpose": "p", "duration": 4, "anchor": "well",
                      "offset": [-300, 0, 120], "look_at": "", "focal_length": 35, "move": "orbit_left",
                      "move_amount": 20, "time_of_day": "night", "notes": ""});
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": [shot]}));
    let shots = ai::plan(&f.ws.defn, Some("test"), None, None, None, &fake).unwrap().shots;
    assert_eq!(shots[0]["id"], "s01_hook");
    assert_eq!(shots[0]["beat"], "hook");
    assert_eq!(shots[0]["camera"]["move"], json!({"type": "orbit_left", "degrees": 20.0}));
    assert_eq!(shots[0]["environment"], json!({"time_of_day": "night"}));
    // writing a plan keeps the file's header comments
    f.ws.replace_all_shots(&shots).unwrap();
    assert_eq!(f.ws.defn.shot_ids(), ids(&["s01_hook"]));
    assert!(f.text().contains("# Order here is the order of the cut"));
}

fn plan_shot(id: &str, beat: &str, duration: f64, anchor: &str) -> Value {
    json!({"id": id, "beat": beat, "purpose": "p", "duration": duration, "anchor": anchor,
           "offset": [-300, 0, 120], "look_at": "", "focal_length": 35, "move": "static",
           "move_amount": 0, "time_of_day": "default", "notes": "", "appear": []})
}

#[test]
fn plan_gets_the_brief_skeleton_and_craft_before_writing() {
    let mut f = ws();
    f.ws.set_brief(&common::set(&[("genre", json!("一人称ホラー")), ("must_show", json!(["井戸"]))])).unwrap();
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": [plan_shot("a", "hook", 4.0, "well")]}));
    let p = ai::plan(&f.ws.defn, None, None, None, Some("ジャンプスケアなしで"), &fake).unwrap();
    assert!(p.references.iter().any(|r| r.starts_with("horror (")), "{:?}", p.references);
    let req = &fake.calls.borrow()[0];
    let user = req["messages"][0]["content"].as_str().unwrap();
    for part in ["well: [0.0, 0.0, 80.0], 140 x 140 x 90 cm", "fills the frame height from about", "## Brief", "Genre: 一人称ホラー", "Must show: 井戸", "## Skeleton", "## Craft rules",
                 "Do not open on the title", "## Measured teaser craft", "## Genre reference: horror", "### 2. Do", "## Framing", "ジャンプスケアなしで"] {
        assert!(user.contains(part), "missing {part}:
{user}");
    }
}

#[test]
fn prompts_never_ask_claude_to_inspect_its_own_output() {
    // D-22: quality comes from the spec given before writing, not from self-review
    let f = ws();
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": [plan_shot("a", "hook", 4.0, "well")]}));
    ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    let rev = FakeClaude::new(json!({"summary": "x", "edits": [], "order": [], "audio": ""}));
    ai::revise(&f.ws.defn, "3番目はもっと寄って", Some(&ids(&["s03_window"])), &rev).unwrap();
    for req in [&fake.calls.borrow()[0], &rev.calls.borrow()[0]] {
        let system = req["system"].as_str().unwrap();
        assert!(shotpilot::craft::self_review_words(system).is_empty(), "{:?}", shotpilot::craft::self_review_words(system));
    }
    let plan_user = fake.calls.borrow()[0]["messages"][0]["content"].as_str().unwrap().to_string();
    assert!(shotpilot::craft::self_review_words(&plan_user).is_empty(), "{:?}", shotpilot::craft::self_review_words(&plan_user));
}

/// Everything a request says: the system prompt and every text block of the user turn.
fn request_text(req: &Value) -> String {
    let user = match &req["messages"][0]["content"] {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks.iter().filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    };
    format!("{}\n{user}", req["system"].as_str().unwrap_or(""))
}

#[test]
fn every_path_that_writes_shots_gets_the_frozen_spec() {
    // D-48: open-design hands the same frozen spec to every path that writes the artifact.
    // Each path that asks Claude for shots, and which parts of the spec it carries.
    let mut f = ws();
    f.ws.set_brief(&[
        ("genre".into(), json!("一人称ホラー")),
        ("design".into(), json!("quiet-dread")),
        ("skills".into(), json!(["count-run"])),
        ("direction".into(), json!("記録映像のふりをする。")),
    ])
    .unwrap();
    let defn = &f.ws.defn;
    let empty = || FakeClaude::new(json!({"summary": "x", "edits": [], "order": [], "audio": ""}));
    let one = ids(&["s03_window"]);
    let all = defn.shot_ids();
    let fields = |a: &str| ai::Pointer { fields: ai::aspect_fields(a), ..Default::default() };
    let plan = FakeClaude::new(json!({"rationale": "r", "shots": [plan_shot("a", "hook", 4.0, "well")]}));
    ai::plan(defn, None, None, None, None, &plan).unwrap();
    let mut paths: Vec<(&str, String)> = vec![("plan", request_text(&plan.calls.borrow()[0]))];
    let cases: Vec<(&str, Option<&[String]>, ai::Pointer)> = vec![
        ("revise: the whole teaser", None, ai::Pointer::default()),
        ("revise: one shot", Some(&one), ai::Pointer::default()),
        ("revise: its camera", Some(&one), fields("camera")),
        ("revise: its light", Some(&one), fields("light")),
        ("revise: every camera", Some(&all), fields("camera")),
        ("revise: its finishing", Some(&one), fields("finish")),
        ("revise: the sound", Some(&one), ai::Pointer { sound: true, ..Default::default() }),
    ];
    for (name, scope, pointer) in cases {
        let c = empty();
        ai::revise_at(defn, "もっと怖く", scope, &pointer, &c).unwrap();
        paths.push((name, request_text(&c.calls.borrow()[0])));
    }
    let blocks = [
        ("brief", "Genre: 一人称ホラー"),
        ("direction", "Direction: 記録映像のふりをする。"),
        ("look", "## Active design system"),
        ("skill", "## Active skill — count-run"),
        ("craft rules", "## Craft rules"),
        ("framing", "## Framing"),
        ("baseline", "## Teaser baseline"),
        ("type", "## On-screen type"),
        ("motion", "## Text motion"),
        ("hand-overs", "## Hand-overs between shots"),
    ];
    // which blocks each path must carry: picture notes get all of it, text notes the look, the
    // baseline and how lines are written (D-52), sound notes the brief and the direction only
    let want = |name: &str| -> &[&str] {
        match name {
            "revise: its finishing" => &["brief", "direction", "look", "skill", "baseline", "type", "motion", "hand-overs"],
            "revise: the sound" => &["brief", "direction"],
            _ => &["brief", "direction", "look", "skill", "craft rules", "framing", "baseline", "type", "motion", "hand-overs"],
        }
    };
    for (name, text) in &paths {
        for (block, marker) in blocks {
            assert_eq!(text.contains(marker), want(name).contains(&block), "{name}: {block}");
        }
        assert!(shotpilot::craft::self_review_words(text).is_empty(), "{name}: {:?}", shotpilot::craft::self_review_words(text));
    }
}

#[test]
fn the_plan_moves_as_the_look_says_unless_the_brief_asks_otherwise() {
    // D-53: the look's motion tokens become finish.motion; the planner adds only what the
    // brief or the direction asked for (energy, the Japanese PV variant, moves to avoid), and a
    // line keeps the entrance it asked for
    let mut f = ws();
    f.ws.set_brief(&[("design".into(), json!("quiet-dread"))]).unwrap();
    let line = json!({"id": "hit", "kind": "card", "ja": "逃げろ", "en": "", "head": "", "from": "", "dur": 2.0,
                      "after": "start", "on": "", "at": 0, "motion": "slam"});
    let fake = FakeClaude::new(json!({"rationale": "r", "script": [line],
        "shots": [plan_shot("a", "hook", 4.0, "well"), plan_shot("b", "title", 4.0, "gate")],
        "motion": {"energy": "driving", "variant": "ja", "avoid": ["flash"]}}));
    let p = ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    let motion = &p.finish.iter().find(|(k, _)| k == "motion").unwrap().1;
    assert_eq!(motion["profile"], json!("horror-ja"));
    assert_eq!(motion["energy"], json!("driving"));
    assert_eq!(motion["avoid"], json!(["glitch", "flash"])); // the look's, then the brief's
    assert_eq!(motion["title"]["in"], json!("fade")); // the look's override stays
    assert_eq!(p.script[0]["motion"], json!("slam"));
    // with nothing asked, the look's motion as it is
    let quiet = FakeClaude::new(json!({"rationale": "r", "shots": [plan_shot("a", "hook", 4.0, "well")]}));
    let p = ai::plan(&f.ws.defn, None, None, None, None, &quiet).unwrap();
    let motion = &p.finish.iter().find(|(k, _)| k == "motion").unwrap().1;
    assert_eq!(motion["profile"], json!("horror"));
    assert!(motion.get("energy").is_none());
    // no look: the genre's profile
    f.ws.set_brief(&[("design".into(), json!("")), ("genre".into(), json!("コージーな農業シム"))]).unwrap();
    let p = ai::plan(&f.ws.defn, None, None, None, None, &quiet).unwrap();
    assert_eq!(p.finish.iter().find(|(k, _)| k == "motion").unwrap().1["profile"], json!("cozy"));
}

#[test]
fn text_stays_long_enough_to_read() {
    // D-49: code gives a planned line the time it takes to read; the check points at hand-written ones
    assert_eq!(shotpilot::script::reading_secs("card", "七号室", ""), 1.5);
    assert_eq!(shotpilot::script::reading_secs("card", "今日から、ここに住む。", ""), 3.25);
    assert_eq!(shotpilot::script::reading_secs("count", "コップ、七つ。", ""), 1.0);
    assert_eq!(shotpilot::script::reading_secs("black", "", ""), 0.0);

    let mut f = ws();
    let line = |id: &str, kind: &str, ja: &str, dur: f64, after: &str, on: &str| json!({"id": id, "kind": kind, "ja": ja,
        "en": "", "head": "", "from": "", "dur": dur, "after": after, "on": on, "at": 0.3});
    let fake = FakeClaude::new(json!({"rationale": "r",
        "script": [line("intro", "card", "今日から、ここに住む。", 1.0, "start", ""), line("ok", "card", "七号室", 2.0, "a", "")],
        "shots": [plan_shot("a", "hook", 4.0, "well"), plan_shot("b", "title", 4.0, "gate")]}));
    let p = ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    assert_eq!(p.script[0]["dur"], json!(3.3));
    assert_eq!(p.script[1]["dur"], json!(2.0)); // already long enough
    assert!(p.adjustments.iter().any(|a| a.contains("`intro` を 1秒 → 3.3秒")), "{:?}", p.adjustments);

    // a line written by hand is left as written, and the shot check names it
    f.ws.render(&[], "previz", false, None).unwrap();
    let mut text = f.text();
    text.push_str("\nscript:\n  - {id: intro, kind: card, ja: 今日から、ここに住む。, dur: 1, after: start}\n");
    f.write(&text);
    f.ws.reload().unwrap();
    let found = f.ws.check().unwrap();
    let hold: Vec<_> = found.iter().filter(|x| x.rule == "text.hold").collect();
    assert_eq!(hold.len(), 1, "{found:?}");
    assert!(hold[0].message.contains("3.3"), "{}", hold[0].message);
    // editorial: nothing re-renders
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));
}

#[test]
fn code_puts_beats_in_order_and_fits_the_runtime() {
    let f = ws(); // target 30s
    let shots = vec![
        plan_shot("t", "title", 4.0, "well"),
        plan_shot("h", "hook", 12.0, "gate"),
        plan_shot("e", "establish", 10.0, "well"),
        plan_shot("x", "escalate", 20.0, "door"),
    ];
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": shots}));
    let p = ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    let order: Vec<&str> = p.shots.iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert_eq!(order, ["h", "e", "x", "t"]);
    let total: f64 = p.shots.iter().map(|s| s["duration"].as_f64().unwrap()).sum();
    assert!((total - 30.0).abs() <= 3.0, "{total}");
    assert!(p.shots.iter().all(|s| (1.0..=8.0).contains(&s["duration"].as_f64().unwrap())));
    // 12/10/20 s clamp to 8 s; 28 s is within 10% of 30 s, so nothing is rescaled
    assert_eq!(p.adjustments.len(), 2, "{:?}", p.adjustments);

    // far off the target: durations are rescaled to fit
    let short = vec![plan_shot("h", "hook", 1.0, "gate"), plan_shot("e", "establish", 2.0, "well"),
                     plan_shot("x", "escalate", 2.0, "door"), plan_shot("t", "title", 2.0, "well")];
    let p = ai::plan(&f.ws.defn, None, None, None, None, &FakeClaude::new(json!({"rationale": "r", "shots": short}))).unwrap();
    let total: f64 = p.shots.iter().map(|s| s["duration"].as_f64().unwrap()).sum();
    assert!((total - 30.0).abs() <= 3.0, "{total}");
    assert!(p.adjustments.iter().any(|a| a.starts_with("カットの合計を")), "{:?}", p.adjustments);
}

#[test]
fn skeleton_follows_the_craft() {
    use shotpilot::craft::skeleton;
    use shotpilot::model::BriefCfg;
    for target in [30.0, 45.0, 60.0] {
        let slots = skeleton(&BriefCfg::default(), target, None);
        let total: f64 = slots.iter().map(|s| s.duration).sum();
        assert!((total - target).abs() < 0.6, "{target}: {total}");
        assert_eq!(slots.first().unwrap().beat, "hook");
        assert_eq!(slots.last().unwrap().beat, "title");
        let mean = |b: &str| {
            let d: Vec<f64> = slots.iter().filter(|s| s.beat == b).map(|s| s.duration).collect();
            d.iter().sum::<f64>() / d.len() as f64
        };
        assert!(mean("escalate") < mean("establish"), "{slots:?}");
    }
    let gameplay = BriefCfg { kind: "gameplay".into(), ..Default::default() };
    assert!(skeleton(&gameplay, 60.0, None)[0].duration <= 3.0);

    // hook and title lengths follow the genre's measured teasers: horror holds both longer
    let genre = |g: &str| BriefCfg { genre: g.into(), ..Default::default() };
    let (horror, action) = (skeleton(&genre("ホラー"), 45.0, None), skeleton(&genre("アクション"), 45.0, None));
    assert!(horror[0].duration > action[0].duration, "{horror:?}\n{action:?}");
    assert!(horror.last().unwrap().duration > action.last().unwrap().duration, "{horror:?}\n{action:?}");
}

#[test]
fn brief_genre_picks_a_measured_reference() {
    use shotpilot::refs::{genre_for, genres};
    let id = |t: &str| genre_for(t).map(|r| r.id);
    assert_eq!(id("一人称ホラー"), Some("horror"));
    assert_eq!(id("Psychological Horror"), Some("horror"));
    assert_eq!(id("ほのぼの農場シム"), Some("cozy"));
    assert_eq!(id("アクションRPG"), Some("rpg")); // rpg is matched before action
    assert_eq!(id("見下ろし型シューター"), Some("action"));
    assert_eq!(id("謎解きアドベンチャー"), Some("adventure"));
    assert_eq!(id("adventure"), Some("adventure"));
    assert_eq!(id("都市開発シミュレーション"), Some("strategy"));
    assert_eq!(id("タクティクスRPG"), Some("strategy")); // strategy is matched before rpg
    assert_eq!(id("ライフシミュレーション"), Some("cozy")); // a life sim stays cozy
    assert_eq!(id("4X strategy"), Some("strategy"));
    assert_eq!(id("レース"), None);
    assert_eq!(id(""), None);
    for g in genres() {
        assert!(g.n >= 20, "{} has only {} teasers", g.id, g.n);
        assert!(!g.prose().is_empty() && !g.prose().contains("auto:"), "{}", g.id);
    }
}

#[test]
fn references_never_ask_claude_to_inspect_its_own_output() {
    // D-22 holds for the measured references too: they are a spec, not a rubric
    use shotpilot::model::BriefCfg;
    for g in shotpilot::refs::genres() {
        let text = shotpilot::refs::planner_text(&BriefCfg { genre: g.id.into(), ..Default::default() });
        assert!(text.contains(&format!("## Genre reference: {}", g.id)));
        assert!(shotpilot::craft::self_review_words(&text).is_empty(), "{}: {:?}", g.id, shotpilot::craft::self_review_words(&text));
    }
}

#[test]
fn plan_can_make_a_prop_appear() {
    let f = ws();
    let mut hook = plan_shot("h", "hook", 4.0, "door");
    hook["appear"] = json!([{"prop": "stranger", "at": 1.2, "anchor": "door", "offset": [150, 0, -120]}]);
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": [hook]}));
    let p = ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    assert_eq!(p.shots[0]["appear"][0]["prop"], "stranger");
    let user = fake.calls.borrow()[0]["messages"][0]["content"].as_str().unwrap().to_string();
    assert!(user.contains("- stranger: /Game/Characters/Mannequins/Meshes/SKM_Manny (a character, no animation)"), "{user}");
}

#[test]
fn plan_sees_the_mood_board_as_images() {
    let mut f = ws();
    let png = [0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
    f.ws.add_moodboard_image("mood.png", &png).unwrap();
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": [plan_shot("h", "hook", 4.0, "well")]}));
    ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    let content = fake.calls.borrow()[0]["messages"][0]["content"].clone();
    let blocks = content.as_array().unwrap();
    assert_eq!(blocks[0]["type"], "image");
    let text = blocks.last().unwrap()["text"].as_str().unwrap();
    assert!(text.contains("## Mood board") && text.contains("not shot designs"), "{text}");
    assert!(shotpilot::craft::self_review_words(text).is_empty());
}

#[test]
fn overlays_on_one_shot_come_one_at_a_time() {
    let f = ws();
    let line = |id: &str, kind: &str, on: &str, at: f64, dur: f64| json!({"id": id, "kind": kind, "ja": id, "en": "", "head": "",
        "from": "", "dur": dur, "after": "", "on": on, "at": at});
    let mut shot = plan_shot("h", "hook", 4.0, "well");
    shot["transition"] = json!("cut");
    let fake = FakeClaude::new(json!({"rationale": "r", "grade": "none", "letterbox": 0, "shots": [shot],
        "script": [line("a", "caption", "h", 0.5, 2.0), line("b", "count", "h", 0.8, 1.0), line("c", "caption", "h", 3.0, 2.0)]}));
    let p = ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    let at = |id: &str| p.script.iter().find(|l| l["id"] == id).map(|l| (l["at"].as_f64().unwrap(), l["dur"].as_f64().unwrap()));
    assert_eq!(at("a"), Some((0.5, 2.0)));
    assert_eq!(at("b"), Some((2.6, 1.0))); // waits for `a` (+0.1 s)
    assert_eq!(at("c"), None); // no room left on the shot
    assert!(p.adjustments.iter().any(|a| a.contains("台本の行 `b` を `")) && p.adjustments.iter().any(|a| a.contains("台本の行 `c` を外した")), "{:?}", p.adjustments);
}

#[test]
fn a_plan_hands_over_with_cut_and_two_others() {
    // D-58 (open-design: one primary transition and one or two accents): code keeps dip and the
    // look's accent (the base profile's is flash); the planner's other hand-overs become cuts
    let f = ws();
    let shot = |id: &str, beat: &str, t: &str| {
        let mut s = plan_shot(id, beat, 3.0, "well");
        s["transition"] = json!(t);
        s
    };
    let fake = FakeClaude::new(json!({"rationale": "r", "grade": "none", "letterbox": 0, "shots": [
        shot("a", "hook", "glitch"), shot("b", "establish", "whiteout"), shot("c", "escalate", "flash"),
        shot("d", "escalate", "dip"), shot("e", "title", "cut")]}));
    let p = ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    let kinds: Vec<&str> = p.shots.iter().map(|s| s["transition"].as_str().unwrap_or("cut")).collect();
    assert_eq!(kinds, ["cut", "cut", "flash", "dip", "cut"]);
    assert!(p.adjustments.iter().any(|a| a == "つなぎを cut のほか 2 種（flash, dip）に絞った: a glitch, b whiteout"), "{:?}", p.adjustments);
    // the planner was told what each hand-over says and where it goes, in one wording
    let text = request_text(&fake.calls.borrow()[0]);
    assert!(text.contains("## Hand-overs between shots") && text.contains("whiteout (up to white and back"), "{text}");
    assert!(!text.contains("on each short evidence shot of a count"), "the old, contradicting line is gone");
}
