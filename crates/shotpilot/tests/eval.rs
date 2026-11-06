//! The evaluation harness (eval.rs) with a fake Claude: scoring is deterministic code.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use shotpilot::ai::{self, Claude};
use shotpilot::eval;
use shotpilot::{Result, Workspace};

const CASES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../eval/cases");

fn case(id: &str) -> PathBuf {
    Path::new(CASES).join(id).join("shots.yaml")
}

struct Fake {
    payload: Value,
    calls: RefCell<Vec<Value>>,
}

impl Fake {
    fn new(payload: Value) -> Fake {
        Fake { payload, calls: RefCell::new(Vec::new()) }
    }
}

impl Claude for Fake {
    fn ask(&self, request: &Value) -> Result<String> {
        self.calls.borrow_mut().push(request.clone());
        Ok(self.payload.to_string())
    }
}

fn shot(id: &str, beat: &str, duration: f64, anchor: &str, focal: f64) -> Value {
    json!({"id": id, "beat": beat, "purpose": "p", "duration": duration, "anchor": anchor,
           "offset": [-300, 0, 120], "look_at": "", "aim": "base", "focal_length": focal, "move": "static",
           "move_amount": 0, "time_of_day": "default", "notes": "", "appear": []})
}

fn line(id: &str, kind: &str, ja: &str, dur: f64, after: &str) -> Value {
    json!({"id": id, "kind": kind, "ja": ja, "en": "", "head": "", "from": "", "dur": dur, "after": after,
           "on": "", "at": 0, "motion": ""})
}

fn reply(shots: Vec<Value>, script: Vec<Value>) -> Value {
    json!({"rationale": "r", "script": script, "shots": shots, "grade": "none", "letterbox": 0, "motion": {}, "title_face": ""})
}

fn plan_for(id: &str, payload: &Value) -> (Workspace, ai::Planned) {
    let ws = Workspace::open(case(id)).unwrap();
    let p = ai::plan(&ws.defn, None, None, None, None, &Fake::new(payload.clone())).unwrap();
    (ws, p)
}

const ANCHORS: [&str; 5] = ["gate", "well", "window", "door", "lantern"];
const MOVES: [&str; 3] = ["static", "dolly_in", "crane_up"];

/// Shots as (beat, seconds), cycling anchors, lenses and moves so the shot check stays quiet.
fn shots_of(spec: &[(&str, f64)]) -> Vec<Value> {
    spec.iter()
        .enumerate()
        .map(|(i, (beat, d))| {
            let mut s = shot(&format!("s{:02}", i + 1), beat, *d, ANCHORS[i % 5], [24.0, 50.0, 35.0, 85.0][i % 4]);
            s["move"] = json!(MOVES[(i / 2) % 3]);
            s["move_amount"] = json!(if i / 2 % 3 == 0 { 0 } else { 80 });
            s["offset"] = json!([-300 - 40 * (i % 3) as i64, 60 * (i % 4) as i64 - 90, 120]);
            s
        })
        .collect()
}

/// The horror case's plan: rhythm close to the style (a slow opening, a fast third quarter,
/// a short close) with one rule card and the title card.
fn good_horror() -> Value {
    let mut spec = vec![("hook", 4.5), ("establish", 4.5), ("establish", 3.5)];
    spec.extend([("escalate", 3.5), ("escalate", 3.5)]);
    spec.extend(std::iter::repeat(("escalate", 2.0)).take(6));
    spec.extend([("escalate", 3.0), ("title", 2.0)]);
    let shots = shots_of(&spec);
    let n = shots.len();
    let script = vec![
        line("rule1", "rule", "井戸をのぞくな", 2.5, "s03"),
        line("title", "title", "夜の中庭", 1.6, &format!("s{n:02}")),
    ];
    reply(shots, script)
}


const IDS: [&str; 6] = ["horror", "cozy", "strategy", "rpg", "action", "adventure"];

#[test]
fn every_case_loads_and_points_at_real_references() {
    for id in IDS {
        let ws = Workspace::open(case(id)).unwrap();
        let b = &ws.defn.brief;
        assert_eq!(shotpilot::refs::genre_for(&b.genre).map(|g| g.id), Some(id), "{id}: genre {:?}", b.genre);
        assert_eq!(shotpilot::refs::style(&b.style).map(|s| s.genre), Some(id), "{id}: style {:?}", b.style);
        assert!(shotpilot::designsys::get(&b.design).is_some(), "{id}: look {:?}", b.design);
        assert!((30.0..=45.0).contains(&ws.defn.teaser.target_duration), "{id}");
        assert!(!ws.defn.stage.summary.is_empty() && ws.defn.stage.anchors.len() == ws.defn.anchors.len(), "{id}: stage");
        assert!(!ws.defn.shots.is_empty() && !b.core.is_empty() && !ws.defn.teaser.pitch.is_empty(), "{id}");
        assert!(eval::target(&ws.defn).source.starts_with("style:"), "{id}");
    }
}

#[test]
fn the_target_falls_back_from_style_to_genre_to_all_teasers() {
    let mut ws = Workspace::open(case("horror")).unwrap();
    let style = eval::target(&ws.defn);
    assert_eq!(style.source, "style:yt-fAXzLmsbrd8");
    assert_eq!(style.first_cut.mid, 4.5);
    ws.defn.brief.style = String::new();
    let genre = eval::target(&ws.defn);
    assert_eq!(genre.source, "genre:horror");
    assert!(genre.first_cut.lo < genre.first_cut.mid && genre.first_cut.mid < genre.first_cut.hi);
    ws.defn.brief.genre = String::new();
    let all = eval::target(&ws.defn);
    assert_eq!(all.source, "all");
    assert!((all.density.iter().sum::<f64>() - 4.0).abs() < 1e-6);
}

#[test]
fn bands_are_full_inside_and_fall_to_zero_at_about_three_times() {
    let b = eval::Band { lo: 8.0, mid: 10.0, hi: 12.0 };
    assert_eq!(eval::band_score(8.0, &b), 1.0);
    assert_eq!(eval::band_score(12.0, &b), 1.0);
    assert!((eval::band_score(13.0, &b) - 0.75).abs() < 1e-9);
    assert!((eval::band_score(14.0, &b) - 0.5).abs() < 1e-9);
    assert_eq!(eval::band_score(16.0, &b), 0.0);
    assert_eq!(eval::band_score(0.0, &b), 0.0);
}

#[test]
fn a_plan_close_to_the_target_passes_with_a_high_total() {
    let (ws, p) = plan_for("horror", &good_horror());
    let s = eval::score(&ws.defn, &p).unwrap();
    assert!(s.pass && s.total >= 90.0, "{}", serde_json::to_string_pretty(&s.to_json()).unwrap());
    assert_eq!(s.facts["p0"], 0);
    assert_eq!(s.facts["target"]["source"], "style:yt-fAXzLmsbrd8");
    assert_eq!(s.facts["density"].as_array().unwrap().len(), 4);
    assert_eq!(s.to_json()["parts"].as_object().unwrap().len(), 4);
}

#[test]
fn a_plan_whose_anchor_does_not_exist_scores_zero() {
    let (ws, mut p) = plan_for("horror", &good_horror());
    p.shots[0]["camera"]["anchor"] = json!("nowhere");
    let s = eval::score(&ws.defn, &p).unwrap();
    assert_eq!(s.total, 0.0);
    assert!(!s.pass);
    assert!(s.notes[0].contains("nowhere"), "{:?}", s.notes);
}

#[test]
fn a_title_first_plan_fails_on_the_p0() {
    let all_title = reply(shots_of(&[("title", 4.0), ("title", 4.0), ("title", 4.0), ("title", 4.0)]), vec![]);
    let (ws, p) = plan_for("horror", &all_title);
    let s = eval::score(&ws.defn, &p).unwrap();
    assert!(!s.pass);
    assert!(s.facts["p0"].as_u64().unwrap() >= 1);
    assert!(s.notes.iter().any(|n| n.contains("open.title")), "{:?}", s.notes);
    assert!(s.parts["checks"] < 70.0);
}

#[test]
fn a_plan_far_from_the_rhythm_scores_lower_on_structure() {
    let slow = reply(shots_of(&[("hook", 8.0), ("establish", 8.0), ("escalate", 8.0), ("escalate", 8.0), ("title", 8.0)]), vec![]);
    let (ws, far) = plan_for("horror", &slow);
    let (_, near) = plan_for("horror", &good_horror());
    let (far, near) = (eval::score(&ws.defn, &far).unwrap(), eval::score(&ws.defn, &near).unwrap());
    assert!(far.parts["structure"] + 30.0 < near.parts["structure"], "{} vs {}", far.parts["structure"], near.parts["structure"]);
    assert!(far.total < near.total);
}

#[test]
fn script_lines_that_break_the_rules_cost_text_points() {
    let mut payload = good_horror();
    payload["script"][0] = line("rule1", "rule", "井戸をのぞいてはいけないという決まりが２階にある。", 6.0, "s03");
    let (ws, p) = plan_for("horror", &payload);
    let s = eval::score(&ws.defn, &p).unwrap();
    let v = s.facts["text_violations"].as_array().unwrap();
    assert_eq!(v.len(), 3, "{v:?}"); // too long, closing full stop, full-width digit
    assert!(s.parts["text"] <= 64.0);
}

// ----------------------------------------------------------------------- runner

struct Shared(std::rc::Rc<Fake>);

impl Claude for Shared {
    fn ask(&self, request: &Value) -> Result<String> {
        self.0.ask(request)
    }
}

fn totals(r: &eval::Report) -> Vec<String> {
    let mut v: Vec<String> = r.rows.iter().map(|x| format!("{}/{}/{}/{:.4}", x.case, x.model, x.run, x.total())).collect();
    v.sort();
    v
}

#[test]
fn run_writes_results_replies_and_a_summary_and_never_touches_the_cases() {
    let cases = [case("horror"), case("cozy")];
    let before: Vec<Vec<u8>> = cases.iter().map(|c| std::fs::read(c).unwrap()).collect();
    let out = tempfile::tempdir().unwrap();
    let fake = std::rc::Rc::new(Fake::new(good_horror()));
    let f2 = fake.clone();
    let make = move |_m: &str| -> Box<dyn Claude> { Box::new(Shared(f2.clone())) };
    let models = vec!["model-a".to_string(), "model-b".to_string()];
    let report = eval::run(&cases, &models, 2, out.path(), &make).unwrap();

    assert_eq!(report.rows.len(), 8);
    assert!(report.rows.iter().all(|r| r.ok && r.score.is_some()));
    let results = std::fs::read_to_string(out.path().join("results.jsonl")).unwrap();
    let rows: Vec<Value> = results.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(rows.len(), 8);
    for r in &rows {
        for k in ["case", "model", "run", "secs", "ok", "error", "score", "spec_hash", "reply_file"] {
            assert!(r.get(k).is_some(), "missing {k}");
        }
        assert!(out.path().join(r["reply_file"].as_str().unwrap()).is_file());
        assert_eq!(r["spec_hash"].as_str().unwrap().len(), 12);
    }
    assert_eq!(std::fs::read_dir(out.path().join("replies")).unwrap().count(), 8);
    // the model name reaches the request
    let seen: Vec<String> = fake.calls.borrow().iter().map(|c| c["model"].as_str().unwrap().to_string()).collect();
    assert_eq!(seen.iter().filter(|m| *m == "model-a").count(), 4);
    assert_eq!(seen.iter().filter(|m| *m == "model-b").count(), 4);
    // the same case under the same spec hashes alike; a different case does not
    let hash = |case: &str, model: &str, run: u64| {
        rows.iter().find(|r| r["case"] == case && r["model"] == model && r["run"] == run).unwrap()["spec_hash"].clone()
    };
    assert_eq!(hash("horror", "model-a", 1), hash("horror", "model-b", 2));
    assert_ne!(hash("horror", "model-a", 1), hash("cozy", "model-a", 1));

    let summary = std::fs::read_to_string(out.path().join("summary.md")).unwrap();
    for part in ["model-a", "model-b", "horror", "cozy", "pass %", "Spec hashes"] {
        assert!(summary.contains(part), "missing {part}:\n{summary}");
    }
    let stats = report.by_model();
    assert_eq!(stats.len(), 2);
    assert_eq!(stats[0].runs, 4);
    assert_eq!(stats[0].ok_pct, 100.0);
    assert!(stats[0].mean_total > 0.0);

    for (c, b) in cases.iter().zip(before) {
        assert_eq!(std::fs::read(c).unwrap(), b, "{} was written", c.display());
        assert!(!c.parent().unwrap().join(".shotpilot").exists());
    }

    // replay: the same totals from the saved replies, no model call
    let calls = fake.calls.borrow().len();
    let again = eval::replay(out.path()).unwrap();
    assert_eq!(fake.calls.borrow().len(), calls);
    assert_eq!(totals(&report), totals(&again));
    assert!(out.path().join("replay.jsonl").is_file() && out.path().join("replay.md").is_file());
}

#[test]
fn a_failing_call_is_recorded_not_fatal() {
    struct Down;
    impl Claude for Down {
        fn ask(&self, _r: &Value) -> Result<String> {
            Err(shotpilot::Error::user("no connection"))
        }
    }
    let out = tempfile::tempdir().unwrap();
    let make = |_m: &str| -> Box<dyn Claude> { Box::new(Down) };
    let report = eval::run(&[case("action")], &["m".to_string()], 1, out.path(), &make).unwrap();
    assert!(!report.rows[0].ok);
    assert_eq!(report.rows[0].error.as_deref(), Some("no connection"));
    assert!(report.rows[0].reply_file.is_none());
    assert_eq!(report.by_model()[0].ok_pct, 0.0);
}

#[test]
fn a_noschema_model_gets_the_schema_as_text_and_the_same_model_otherwise() {
    let fake = Fake::new(good_horror());
    let ws = Workspace::open(case("horror")).unwrap();
    let held = eval::WithModel::new(&fake, "m1");
    ai::plan(&ws.defn, None, None, None, None, &held).unwrap();
    let free = eval::WithModel::new(&fake, &format!("m1{}", eval::NO_SCHEMA));
    ai::plan(&ws.defn, None, None, None, None, &free).unwrap();
    let calls = fake.calls.borrow();
    let (a, b) = (&calls[0], &calls[1]);
    assert_eq!(a["model"], "m1");
    assert_eq!(b["model"], "m1");
    let schema = &a["output_config"]["format"]["schema"];
    assert!(schema.is_object());
    assert!(b["output_config"]["format"].is_null(), "{}", b["output_config"]);
    let system = b["system"].as_str().unwrap();
    assert!(system.starts_with(a["system"].as_str().unwrap()) && system.ends_with(&schema.to_string()));
    // the same spec otherwise: only the system text moves
    assert_eq!(a["messages"], b["messages"]);
}

#[test]
fn the_score_compares_the_beats_and_the_last_piece_as_the_references_were_measured() {
    let (ws, p) = plan_for("horror", &good_horror());
    let s = eval::score(&ws.defn, &p).unwrap();
    let t = eval::target(&ws.defn);
    let want = t.beats.expect("the horror case follows a style with its beats written down");
    assert_eq!(want, shotpilot::refs::style(&ws.defn.brief.style).unwrap().beats().unwrap().share);
    let got: Vec<f64> = s.facts["beats"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    assert!((got.iter().sum::<f64>() - 1.0).abs() < 0.02, "{got:?}");
    // a plan that spends almost all its time in the hook is far from any measured split
    let lopsided = reply(shots_of(&[("hook", 8.0), ("hook", 8.0), ("hook", 8.0), ("establish", 2.0), ("escalate", 2.0), ("title", 2.0)]), vec![]);
    let (_, far) = plan_for("horror", &lopsided);
    let far = eval::score(&ws.defn, &far).unwrap();
    assert!(far.facts["beats_distance"].as_f64().unwrap() > s.facts["beats_distance"].as_f64().unwrap() + 0.1);
    assert!(far.facts["structure_parts"]["beats"].as_f64().unwrap() < 0.5);
    // a title card on black after the last shot is the last cut, as cut detection sees it
    let mut carded = good_horror();
    carded["script"] = json!([line("b", "black", "", 0.5, "s13"), line("t", "title", "ナナゴウシツ", 3.5, "b")]);
    let (_, p) = plan_for("horror", &carded);
    let s = eval::score(&ws.defn, &p).unwrap();
    assert_eq!(s.facts["last_shot"].as_f64().unwrap(), 3.5, "{}", s.facts);
}

#[test]
fn several_plans_are_written_and_code_keeps_the_best_one() {
    // three answers in turn: a slow one, a good one, a slow one with one shot fewer
    struct Turns(RefCell<Vec<Value>>, RefCell<usize>);
    impl Claude for Turns {
        fn ask(&self, _r: &Value) -> Result<String> {
            let mut i = self.1.borrow_mut();
            let v = self.0.borrow()[*i % 3].clone();
            *i += 1;
            Ok(v.to_string())
        }
    }
    let slow = reply(shots_of(&[("hook", 8.0), ("establish", 8.0), ("escalate", 8.0), ("escalate", 8.0), ("title", 8.0)]), vec![]);
    let slower = reply(shots_of(&[("hook", 8.0), ("establish", 8.0), ("escalate", 8.0), ("title", 8.0)]), vec![]);
    let turns = Turns(RefCell::new(vec![slow, good_horror(), slower]), RefCell::new(0));
    let ws = Workspace::open(case("horror")).unwrap();
    let p = ai::plan_best(&ws.defn, None, None, None, None, &turns, 3).unwrap();
    assert_eq!(*turns.1.borrow(), 3);
    assert_eq!(p.candidates.len(), 3);
    let kept = eval::score(&ws.defn, &p).unwrap();
    assert!(kept.pass, "{:?}", kept.notes);
    // the kept one is the good one: the highest of the three, and its reply is the one kept
    let best = p.candidates.iter().flatten().cloned().fold(f64::MIN, f64::max);
    assert_eq!((kept.total * 10.0).round() / 10.0, best, "{:?}", p.candidates);
    assert_eq!(serde_json::from_str::<Value>(&p.reply).unwrap(), good_horror());
    // one plan is one call and lists no candidates
    let p = ai::plan_best(&ws.defn, None, None, None, None, &Fake::new(good_horror()), 1).unwrap();
    assert!(p.candidates.is_empty());
}
