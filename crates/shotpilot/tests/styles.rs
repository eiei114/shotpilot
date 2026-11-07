//! Recently popular teasers as styles to follow (D-39): four per genre, measured and
//! analysed. Picking one shapes the skeleton (hook, title, where the cutting is dense) and
//! hands the planner its analysis; it is editorial, so nothing re-renders.

mod common;

use common::{actions, ws};
use serde_json::{json, Value};
use shotpilot::craft::{self_review_words, skeleton};
use shotpilot::model::BriefCfg;
use shotpilot::refs::{beats, catalogue, genres, pace, planner_text, style, styles, styles_for};

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
fn every_style_names_where_its_beats_fall() {
    for s in styles() {
        let b = s.beats().unwrap_or_else(|| panic!("{}: section 2 lacks a hook/establish/escalate/title line with its seconds", s.id));
        assert!((b.share.iter().sum::<f64>() - 1.0).abs() < 1e-9, "{} {b:?}", s.id);
        assert!(b.share.iter().all(|x| *x > 0.02) && b.rate.iter().all(|r| *r > 0.0), "{} {b:?}", s.id);
        // the pieces inside the beats add up to the measured cuts (within the ones that open a beat)
        let pieces: f64 = (0..4).map(|i| b.rate[i] * b.share[i] * s.duration).sum();
        let measured = s.cuts().len() as f64 + 1.0;
        assert!((pieces - measured).abs() <= 4.0, "{}: {pieces} pieces in the beats, {measured} measured", s.id);
        if std::env::var("SHOTPILOT_PRINT").is_ok() {
            println!("{} {:?} {:?} pieces {pieces:.0}/{measured}", s.id, b.share.map(|x| (x * 100.0).round()), b.rate.map(|x| (x * 100.0).round() / 100.0));
        }
    }
    // without a style, the genre's styles on average; without a genre, every style
    let g = &genres()[0];
    assert_eq!(beats(&brief(g.id, "")).unwrap().source, format!("genre:{}", g.id));
    assert_eq!(beats(&brief("", "")).unwrap().source, "all");
    assert_eq!(beats(&brief("", styles()[0].id)).unwrap().source, styles()[0].id);
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

    // the skeleton copies the style's beats: as many pieces as it cuts to at this length (the
    // shots and the black and title cards after them), the first cut where it lands, and
    // the hook and the middle beats sharing the time as they do there
    for s in styles() {
        let b = brief(s.genre, s.id);
        let beats = s.beats().unwrap();
        let slots = skeleton(&b, 45.0, None);
        let total: f64 = slots.iter().map(|x| x.duration).sum();
        assert!((total + shotpilot::craft::card_reserve(&b, 45.0) - 45.0).abs() < 0.3, "{}: {total} {slots:?}", s.id);
        assert!(slots.iter().all(|x| (1.0..=8.0).contains(&x.duration)), "{}: {slots:?}", s.id);
        let want: f64 = (0..4).map(|i| beats.rate[i] * beats.share[i] * 45.0).sum();
        let got = (slots.len() + 2) as f64;
        // a skeleton has at most MAX_SHOTS and a shot holds 1-8 s, so the fastest and the
        // slowest styles are met as near as that allows
        let cap = shotpilot::craft::MAX_SHOTS as f64 + 2.0;
        let floor = (total / 8.0).ceil() + 2.0;
        let want = want.clamp(floor, cap);
        assert!((got - want).abs() <= 0.3 * want + 1.0, "{}: {got} pieces for {want:.1} measured", s.id);
        assert_eq!(slots[0].duration, ((p_hook(s) * 4.0).round() / 4.0), "{}", s.id);
        let secs = |beat: &str| slots.iter().filter(|x| x.beat == beat).map(|x| x.duration).sum::<f64>();
        let hook = beats.share[0] * 45.0;
        // (a hook of one piece there is the first shot alone; its beat's time goes to the middle)
        let hook_shots = slots.iter().filter(|x| x.beat == "hook").count();
        let limit = shotpilot::shotcheck::HOOK_LONG_TEASER;
        if hook_shots > 1 && (slots[0].duration + 1.0..=limit - 0.5).contains(&hook) {
            assert!((secs("hook") - hook).abs() <= 0.5, "{}: hook {} vs {hook}", s.id, secs("hook"));
        }
        // and never past the shot check's hook.long
        assert!(secs("hook") < limit, "{}: hook {}", s.id, secs("hook"));
        // escalation is the faster of the middle beats where the style's is (as the shot check
        // asks); a style that escalates slower is not turned around (the time goes as measured)
        let mean = |beat: &str| secs(beat) / slots.iter().filter(|x| x.beat == beat).count() as f64;
        if beats.rate[2] >= beats.rate[1] {
            assert!(mean("escalate") < mean("establish"), "{}: {slots:?}", s.id);
        }
    }
    // a style that cuts faster asks for more shots
    let pieces = |s: &&shotpilot::refs::Style| {
        let b = s.beats().unwrap();
        (0..4).map(|i| b.rate[i] * b.share[i]).sum::<f64>()
    };
    let fast = styles().iter().max_by(|a, b| pieces(a).total_cmp(&pieces(b))).unwrap();
    let slow = styles().iter().min_by(|a, b| pieces(a).total_cmp(&pieces(b))).unwrap();
    let n = |s: &shotpilot::refs::Style| skeleton(&brief("", s.id), 45.0, None).len();
    assert!(n(fast) > n(slow) + 4, "{} {} vs {} {}", fast.id, n(fast), slow.id, n(slow));
    // a shot count asked for is kept
    assert_eq!(skeleton(&brief("", fast.id), 45.0, Some(9)).len(), 9);
}

#[test]
fn the_inferred_tempo_rules_are_calibrated_on_the_analysed_styles() {
    // hook.long sits at the analysed hooks' upper quartile: three in four teasers hold their
    // hook within it, and the median one does not need it
    let mut hooks: Vec<f64> = styles().iter().map(|s| s.beats().unwrap().share[0] * s.duration).collect();
    hooks.sort_by(f64::total_cmp);
    let limit = shotpilot::shotcheck::HOOK_LONG_TEASER;
    let within = hooks.iter().filter(|h| **h <= limit).count() as f64 / hooks.len() as f64;
    assert!((0.7..=0.85).contains(&within), "{within} of {hooks:?} within {limit}s");
    assert!(hooks[hooks.len() / 2] > 6.0, "{hooks:?}");

    // tempo.escalate stays for a brief with no style; following a style that escalates slower
    // than it establishes is not flagged, following one that escalates faster still is
    let rules = |f: &common::Fixture| f.ws.check().unwrap().iter().map(|x| x.rule.to_string()).collect::<Vec<_>>();
    let mut f = ws();
    assert!(rules(&f).contains(&"tempo.escalate".to_string()), "{:?}", rules(&f));
    let slows = styles().iter().find(|s| s.beats().unwrap().rate[2] < s.beats().unwrap().rate[1]).unwrap();
    let speeds = styles().iter().find(|s| s.beats().unwrap().rate[2] > s.beats().unwrap().rate[1]).unwrap();
    f.ws.set_brief(&[("style".to_string(), json!(slows.id))]).unwrap();
    assert!(!rules(&f).contains(&"tempo.escalate".to_string()), "{}: {:?}", slows.id, rules(&f));
    f.ws.set_brief(&[("style".to_string(), json!(speeds.id))]).unwrap();
    assert!(rules(&f).contains(&"tempo.escalate".to_string()), "{}: {:?}", speeds.id, rules(&f));
}

/// Where a style's skeleton makes its first cut: the measured first cut, inside 1-5 s.
fn p_hook(s: &shotpilot::refs::Style) -> f64 {
    s.first_cut().unwrap().clamp(1.0, 5.0)
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
