//! Recently popular teasers as styles to follow (D-39): four per genre, measured and
//! analysed. Picking one shapes the skeleton (hook, title, where the cutting is dense) and
//! hands the planner its analysis; it is editorial, so nothing re-renders.

mod common;

use common::{actions, ws};
use serde_json::{json, Value};
use shotpilot::craft::{self_review_words, skeleton};
use shotpilot::model::BriefCfg;
use shotpilot::refs::{catalogue, genres, pace, planner_text, style, styles, styles_for};

fn brief(genre: &str, style: &str) -> BriefCfg {
    BriefCfg { genre: genre.into(), style: style.into(), ..Default::default() }
}

#[test]
fn four_recent_styles_per_genre_each_analysed() {
    assert_eq!(styles().len(), 4 * genres().len());
    for g in genres() {
        let mine = styles_for(g.id);
        assert_eq!(mine.len(), 4, "{}", g.id);
        for s in mine {
            assert!(s.uploaded.as_str() >= "2025-01", "{} {}", s.id, s.uploaded);
            assert!((20.0..=100.0).contains(&s.duration), "{} {}", s.id, s.duration);
            assert!(s.summary.ends_with('。') && s.summary.chars().count() >= 10, "{}: {}", s.id, s.summary);
            let prose = s.prose();
            assert!(!prose.contains("auto:") && !prose.contains("分析: 未"), "{}", s.id);
            for section in ["## 2.", "## 4.", "## 7.", "### Do", "### Avoid"] {
                assert!(prose.contains(section), "{} lacks {section}", s.id);
            }
            assert!(!s.cuts().is_empty() && s.density().is_some(), "{}", s.id);
            // D-22: the analysis reaches the planner, so it never asks it to look at its own work
            let text = planner_text(&brief(g.id, s.id));
            assert!(text.contains("## Style reference: Inspired by"), "{}", s.id);
            assert!(self_review_words(&text).is_empty(), "{}: {:?}", s.id, self_review_words(&text));
        }
    }
    assert!(style("yt-nope").is_none());
    let cat = catalogue();
    assert_eq!(cat["genres"].as_array().unwrap().len(), genres().len());
    assert!(cat["genres"].as_array().unwrap().iter().all(|g| g["styles"].as_array().unwrap().len() == 4
        && !g["headline"].as_str().unwrap().is_empty()));
}

#[test]
fn a_style_sets_the_pace_of_the_skeleton() {
    let s = &styles()[0];
    let p = pace(&brief(s.genre, s.id));
    assert_eq!(p.hook, s.first_cut().unwrap());
    assert_eq!(p.title, s.last_shot().unwrap());
    assert_eq!(p.density, s.density().unwrap());
    // without a style, the genre's measured shape
    assert_ne!(pace(&brief(s.genre, "")).density, p.density);

    // the shape moves time: a style that cuts fastest at the end gives its last middle shot
    // less time than a style that settles at the end
    let by_end = |s: &&shotpilot::refs::Style| s.density().unwrap()[3];
    let busy_end = styles().iter().max_by(|a, b| by_end(a).total_cmp(&by_end(b))).unwrap();
    let calm_end = styles().iter().min_by(|a, b| by_end(a).total_cmp(&by_end(b))).unwrap();
    let last_mid = |b: &BriefCfg| {
        let slots = skeleton(b, 45.0, Some(9));
        let total: f64 = slots.iter().map(|x| x.duration).sum();
        assert!((total - 45.0).abs() < 0.6, "{total} {slots:?}");
        slots[slots.len() - 2].duration
    };
    let (busy, calm) = (last_mid(&brief("", busy_end.id)), last_mid(&brief("", calm_end.id)));
    assert!(busy < calm, "{} ({:?}) {busy} vs {} ({:?}) {calm}", busy_end.id, busy_end.density(), calm_end.id, calm_end.density());
}

#[test]
fn picking_a_style_is_editorial_and_reaches_the_plan() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    let s = &styles_for("cozy")[0];
    f.ws.set_brief(&[("genre".to_string(), json!("コージー")), ("style".to_string(), json!(s.id))]).unwrap();
    assert_eq!(f.ws.defn.brief.style, s.id);
    let out = f.ws.render(&[], "previz", false, None).unwrap();
    assert!(actions(&out).values().all(|a| a == "reused"));

    struct Plan(std::cell::RefCell<Vec<Value>>);
    impl shotpilot::ai::Claude for Plan {
        fn ask(&self, request: &Value) -> shotpilot::Result<String> {
            self.0.borrow_mut().push(request.clone());
            let shot = |id: &str, beat: &str, anchor: &str| json!({"id": id, "beat": beat, "purpose": "p", "duration": 5.0,
                "anchor": anchor, "offset": [-400, -300, 100], "look_at": "", "focal_length": 35, "move": "static",
                "move_amount": 0, "time_of_day": "default", "notes": "", "appear": [], "caption": "", "transition": "cut"});
            Ok(json!({"rationale": "r", "grade": "warm", "letterbox": 0, "script": [],
                      "shots": [shot("a", "hook", "well"), shot("b", "establish", "gate"), shot("c", "escalate", "door"), shot("d", "title", "well")]}).to_string())
        }
    }
    let claude = Plan(Default::default());
    let p = shotpilot::ai::plan(&f.ws.defn, None, None, None, None, &claude).unwrap();
    let user = claude.0.borrow()[0]["messages"][0]["content"].to_string();
    assert!(user.contains(&format!("Inspired by {}", s.name)), "the planner gets the style");
    assert!(p.references.iter().any(|r| r.starts_with("style: ")), "{:?}", p.references);
    // the new grades are part of what the planner may answer, and the finish takes them
    assert!(p.finish.iter().any(|(k, v)| k == "grade" && v == "warm"), "{:?}", p.finish);
    f.ws.set_finish(&p.finish).unwrap();
    assert_eq!(f.ws.defn.finish.grade, "warm");
    // no style again: back to the genre's measured shape
    f.ws.set_brief(&[("style".to_string(), Value::Null)]).unwrap();
    assert_eq!(f.ws.defn.brief.style, "");
}
