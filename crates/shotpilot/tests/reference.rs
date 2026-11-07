//! A reference teaser (D-71): measured once, it lends the cut its hand-over lengths and tells
//! the planner where its hand-overs fall; its pictures never reach the cut and nothing
//! re-renders. After open-design's "replicate a reference video".

mod common;

use common::{measured_reference, set, ws};
use serde_json::json;

#[test]
fn a_measured_reference_lends_its_lengths_and_nothing_re_renders() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    measured_reference(&f.ws, "demo");
    let notes = f.ws.use_reference("demo").unwrap();
    assert_eq!(notes.len(), 5, "{notes:?}"); // dip, flash, whiteout, accent, cards
    assert_eq!(f.ws.defn.brief.reference_teaser, "demo");
    let m = f.ws.defn.finish.motion.to_value();
    assert_eq!((m["dip"].clone(), m["flash"].clone(), m["accent"].clone()), (json!(0.08), json!(0.13), json!("flash")));
    assert_eq!(m["whiteout"], json!({"rise": 0.33, "peak": 0.34, "decay": 0.92}));
    assert_eq!(m["card"], json!({"in": "fade", "in_s": 0.54, "out": "cut", "out_s": 0.0}));
    // drawn with the measured seconds as they are
    let mo = shotpilot::motion::resolve(&f.ws.defn.finish.motion);
    assert!(!mo.plain);
    assert_eq!((mo.dip, mo.flash, mo.whiteout.decay, mo.card.enter.as_str()), (0.08, 0.13, 0.92, "fade"));
    // editorial: every take stays fresh
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));

    // the planner hears its rhythm and where its hand-overs fall, never its content
    let text = shotpilot::reference::planner_text(&f.ws.defn);
    assert!(text.contains("## Reference teaser `demo`") && text.contains("never its pictures, words or story"), "{text}");
    assert!(text.contains("40s long; the picture changes every 3.25s on median (11 cuts)"), "{text}");
    assert!(text.contains("2 times through black (into black by a cut 1 of them)") && text.contains("at 25%, 75%"), "{text}");
    assert!(text.contains("2 white flashes, at 50%, 53%") && text.contains("1 whiteouts, at 90%"), "{text}");
    assert!(text.contains("2 lines of text on black, at 3%, 60%"), "{text}");
    assert!(shotpilot::craft::self_review_words(&text).is_empty());

    // a look applied afterwards keeps what was measured on top of its own motion
    f.ws.set_brief(&set(&[("design", json!("quiet-dread"))])).unwrap();
    f.ws.apply_look().unwrap();
    let m = f.ws.defn.finish.motion.to_value();
    assert_eq!((m["profile"].clone(), m["dip"].clone(), m["accent"].clone()), (json!("horror"), json!(0.08), json!("flash")));

    // unknown or unsafe ids are refused
    assert!(f.ws.use_reference("nope").unwrap_err().msg.contains("no measured reference `nope`"));
    assert!(f.ws.set_brief(&set(&[("reference_teaser", json!("../x"))])).unwrap_err().msg.contains("brief.reference_teaser"));
    // with no reference named, the planner hears nothing about one
    f.ws.set_brief(&set(&[("reference_teaser", json!(""))])).unwrap();
    assert!(shotpilot::reference::planner_text(&f.ws.defn).is_empty());
    // a new project keeps the copies of the reference's pictures out of git; its numbers stay in
    let gi = shotpilot::templates::GITIGNORE;
    assert!(gi.contains("reference/*/proxy.mp4\n") && gi.contains("reference/*/compare/\n") && !gi.contains("facts.json"), "{gi}");
}

#[test]
fn a_video_is_measured_and_a_cut_compared_with_it() {
    if shotpilot::cut::ffmpeg().is_none() {
        return; // measuring needs ffmpeg; the rest of the suite does not
    }
    let mut f = ws();
    // a small reference: a cut, a fade into black held 0.6 s, a cut back, a white flash
    let video = f.dir.path().join("Ref Teaser.mp4");
    let src = |kind: &str, d: f64| vec!["-f".to_string(), "lavfi".into(), "-i".into(), format!("{kind}s=320x180:r=24:d={d}")];
    let mut args: Vec<String> = Vec::new();
    for (kind, d) in [("testsrc2=", 2.5), ("smptebars=", 2.0), ("color=c=black:", 0.6), ("smptehdbars=", 2.0), ("color=c=white:", 0.167), ("testsrc2=", 2.0)] {
        args.extend(src(kind, d));
    }
    args.extend([
        "-filter_complex".into(),
        "[0:v]format=yuv420p[a];[1:v]format=yuv420p,fade=t=out:st=1.6:d=0.4[b];[2:v]format=yuv420p[c];[3:v]format=yuv420p[d];\
         [4:v]format=yuv420p[e];[5:v]format=yuv420p[g];[a][b][c][d][e][g]concat=n=6:v=1[v]"
            .into(),
        "-map".into(), "[v]".into(), "-c:v".into(), "libx264".into(), "-pix_fmt".into(), "yuv420p".into(),
        video.display().to_string(),
    ]);
    shotpilot::cut::run_ffmpeg(&args).unwrap();

    f.ws.render(&[], "previz", false, None).unwrap();
    let (r, notes) = f.ws.add_reference(&video, true).unwrap();
    assert_eq!(r.id, "ref-teaser");
    let m = &r.measure;
    assert!((m.secs - 9.25).abs() < 0.2, "{m:?}");
    assert_eq!(m.blacks.len(), 1, "{m:?}");
    assert!(m.blacks[0].fade_out >= 0.2 && (m.blacks[0].gap - 0.6).abs() < 0.2, "{m:?}");
    assert_eq!(m.flashes.len(), 1, "{m:?}");
    assert!(m.cuts.iter().any(|c| (c - 2.5).abs() < 0.1), "{:?}", m.cuts);
    for file in ["facts.json", "proxy.mp4", "contact.jpg"] {
        assert!(f.ws.root.join("reference/ref-teaser").join(file).is_file(), "{file}");
    }
    assert!(notes.iter().any(|n| n.starts_with("flash ")), "{notes:?}");
    assert_eq!(f.ws.defn.brief.reference_teaser, "ref-teaser");
    // measuring the same file again keeps its id
    assert_eq!(f.ws.add_reference(&video, false).unwrap().0.id, "ref-teaser");
    // nothing re-rendered
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));

    // the cut, measured the same way, paired with the reference at the same shares of each
    f.ws.cut("previz", true).unwrap();
    let c = f.ws.compare_reference(None).unwrap();
    assert!((c.cut.secs - 27.5).abs() < 0.3, "{:?}", c.cut);
    assert!(!c.pairs.is_empty() && c.pairs.iter().all(|p| c.dir.join(&p.file).is_file()), "{:?}", c.pairs);
    assert!(c.pairs.iter().any(|p| p.label == "black") && c.pairs.iter().any(|p| p.label == "flash"), "{:?}", c.pairs);
    assert!(c.dir.join("sheet.jpg").is_file() && c.dir.join("compare.json").is_file());
}

#[test]
fn switching_references_leaves_nothing_of_the_old_one() {
    let mut f = ws();
    measured_reference(&f.ws, "demo");
    // a calm one: fades through black, no flash, no whiteout, no text on black
    let calm = f.ws.root.join("reference/calm");
    std::fs::create_dir_all(&calm).unwrap();
    let facts = json!({"source": "calm.mp4", "secs": 30.0, "cuts": [5.0, 12.0],
                       "blacks": [{"at": 10.0, "gap": 0.6, "fade_out": 0.5, "fade_in": 0.5}], "flashes": [], "whiteouts": [], "texts": []});
    std::fs::write(calm.join("facts.json"), facts.to_string()).unwrap();
    f.ws.use_reference("demo").unwrap();
    // the director changes one of the borrowed lengths by hand
    let mut m = f.ws.defn.finish.motion.to_value();
    m["card"]["in_s"] = json!(0.8);
    f.ws.set_finish(&set(&[("motion", m)])).unwrap();
    f.ws.use_reference("calm").unwrap();
    let m = f.ws.defn.finish.motion.to_value();
    assert_eq!((m["dip"].clone(), m["accent"].clone()), (json!(0.5), json!("dip")), "{m}");
    assert!(m.get("flash").is_none() && m.get("whiteout").is_none(), "{m}");
    // what the director changed stays; the rest of what `demo` set is gone
    assert_eq!(m["card"], json!({"in_s": 0.8}), "{m}");
    assert_eq!(f.ws.defn.brief.reference_teaser, "calm");
}
