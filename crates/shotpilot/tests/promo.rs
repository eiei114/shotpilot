//! The release plan (D-47): the videos a game puts out after the teaser, as checklists,
//! started from the current project; and regular posts exported from takes as they are.
//!
//! The point of the update trailer: it is made from the teaser's shots, so after the game
//! changes only the cuts that showed the change render again, in the new project too.

mod common;

use std::collections::BTreeSet;

use chrono::NaiveDate;
use common::{actions, launches, ws, Fixture};
use serde_json::{json, Map, Value};
use shotpilot::promo::{self, Carry, Plan};
use shotpilot::Workspace;

fn write(f: &Fixture, rel: &str, text: &str) {
    let p = f.ws.root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn day(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn obj(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

fn check<'a>(video: &'a Value, id: &str) -> &'a Value {
    video["checks"].as_array().unwrap().iter().find(|c| c["id"] == id).unwrap()
}

#[test]
fn a_new_project_starts_with_this_video_and_the_usual_next_ones() {
    let f = ws();
    let v = promo::view(&f.ws, day("2026-11-05")).unwrap();
    assert_eq!(v["saved"], false, "nothing is written until something changes");
    assert!(!f.ws.root.join(promo::FILE).exists());
    let kinds: Vec<&str> = v["videos"].as_array().unwrap().iter().map(|x| x["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["announce", "demo", "launch"]);
    let here = &v["videos"][0];
    assert_eq!((here["here"].as_bool(), here["exists"].as_bool(), here["state"].as_str()), (Some(true), Some(true), Some("doing")));
    // the made-in-code checks read the project: nothing approved or cut yet
    assert_eq!(check(here, "approved")["done"], false);
    assert_eq!(check(here, "cut")["done"], false);
    assert_eq!(check(here, "game")["done"], true);
    // a row without a project cannot judge them yet
    assert_eq!(check(&v["videos"][1], "cut")["detail"], "プロジェクトを作ると判定します");
}

#[test]
fn hand_checks_tick_and_made_in_code_ones_refuse() {
    let f = ws();
    let mut plan = Plan::open(&f.ws).unwrap();
    plan.set_check("v1", "mute", true).unwrap();
    assert!(plan.set_check("v1", "cut", true).unwrap_err().msg.contains("自動"));
    assert!(plan.set_check("v1", "nope", true).is_err());
    plan.set_video("v2", &obj(json!({"due": "2027-2-01", "title": "Next Fest デモ"}))).unwrap();
    assert!(plan.set_video("v2", &obj(json!({"due": "来月"}))).is_err());
    plan.save().unwrap();

    let v = promo::view(&f.ws, day("2027-01-30")).unwrap();
    assert_eq!(v["saved"], true);
    let fest = v["videos"].as_array().unwrap().iter().find(|x| x["id"] == "v2").unwrap();
    assert_eq!((fest["title"].as_str(), fest["due"].as_str(), fest["days"].as_i64()), (Some("Next Fest デモ"), Some("2027-02-01"), Some(2)));
    // dated rows first
    assert_eq!(v["videos"][0]["id"], "v2");
    let here = v["videos"].as_array().unwrap().iter().find(|x| x["id"] == "v1").unwrap();
    assert_eq!(check(here, "mute")["done"], true);
    // the plan is editorial: no shot goes stale
    assert!(f.ws.status().unwrap().iter().all(|r| r.build == "new"));
}

#[test]
fn an_update_trailer_reuses_every_cut_the_game_did_not_change() {
    let mut f = ws();
    write(&f, "content/well.txt", "well v1");
    write(&f, "content/lantern.txt", "lantern v1");
    f.ws.render(&[], "still", false, None).unwrap();
    let all: BTreeSet<String> = f.ws.defn.shot_ids().into_iter().collect();

    // the game changes: the well is repainted
    write(&f, "content/well.txt", "well v2, repainted");
    let changed: BTreeSet<String> =
        promo::changed_cuts(&f.ws).unwrap().iter().map(|c| c["id"].as_str().unwrap().to_string()).collect();
    assert!(!changed.is_empty() && changed != all, "{changed:?}");

    let (id, shots) = promo::start_update(&f.ws, "v1.1", "2027-04-01").unwrap();
    assert_ne!(shots.parent().unwrap(), f.ws.root.as_path(), "a project of its own, next to this one");
    let mut up = Workspace::open(&shots).unwrap();
    assert_eq!(up.defn.brief.kind, "gameplay");
    assert_eq!(up.defn.shot_ids(), f.ws.defn.shot_ids(), "the update trailer starts from the teaser's shots");
    // the copied takes still watch the same game: only the changed cuts are out of date
    let stale: BTreeSet<String> = up.status().unwrap().into_iter().filter(|r| r.quality("still") == "stale").map(|r| r.id).collect();
    assert_eq!(stale, changed);
    let out = up.render(&[], "still", false, None).unwrap();
    for (sid, a) in actions(&out) {
        assert_eq!(a, if changed.contains(&sid) { "rendered" } else { "reused" }, "{sid}");
    }
    assert_eq!(launches(&up).len(), 1);
    // the teaser's own takes were not touched
    assert_eq!(promo::changed_cuts(&f.ws).unwrap().len(), changed.len());

    // one plan for both: the row points at the new project, and the new project shares the plan
    let mine = promo::view(&f.ws, day("2026-11-05")).unwrap();
    let row = mine["videos"].as_array().unwrap().iter().find(|v| v["id"] == id.as_str()).unwrap().clone();
    assert_eq!((row["kind"].as_str(), row["exists"].as_bool(), row["update"].as_str()), (Some("update"), Some(true), Some("v1.1")));
    let theirs = promo::view(&up, day("2026-11-05")).unwrap();
    assert_eq!(theirs["linked"], true);
    assert_eq!(theirs["plan"], mine["plan"]);
    let row2 = theirs["videos"].as_array().unwrap().iter().find(|v| v["id"] == id.as_str()).unwrap();
    assert_eq!(row2["here"], true);
    // a second project for the same row is refused
    assert!(promo::derive(&f.ws, &id, Carry::Shots).is_err());
}

#[test]
fn a_milestone_video_takes_the_groundwork_but_no_shots() {
    let f = ws();
    write(&f, "scout/well/front.png", "photo");
    let shots = promo::derive(&f.ws, "v3", Carry::Groundwork).unwrap();
    let w = Workspace::open(&shots).unwrap();
    assert!(w.defn.shot_ids().is_empty());
    assert_eq!(w.defn.project.name, "ローンチトレーラー");
    assert_eq!(w.defn.anchors.len(), f.ws.defn.anchors.len());
    assert!(w.root.join("scout/well/front.png").is_file());
    assert!(!w.root.join("render").exists());
    let link: Value = serde_json::from_str(&std::fs::read_to_string(w.root.join(promo::FILE)).unwrap()).unwrap();
    assert!(link["link"].as_str().unwrap().ends_with("promo.json"));
}

#[test]
fn a_post_is_a_take_as_it_is_and_the_next_one_is_due_after_the_cadence() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    let v = promo::view(&f.ws, day("2026-11-05")).unwrap();
    let p = &v["posting"];
    assert_eq!((p["every_days"].as_i64(), p["next"].as_str()), (Some(7), Some("2026-11-05")));
    let cands = p["candidates"].as_array().unwrap();
    assert_eq!(cands.len(), f.ws.defn.shot_ids().len());
    assert!(cands[0]["thumb"].as_str().unwrap().starts_with("/files/render/"));

    let id = promo::export_post(&f.ws, "s04_lantern", false, day("2026-11-05")).unwrap();
    let plan = Plan::open(&f.ws).unwrap();
    let post = plan.posts().into_iter().find(|x| x["id"] == id.as_str()).unwrap();
    assert_eq!((post["shot"].as_str(), post["kind"].as_str(), post["done"].as_bool()), (Some("s04_lantern"), Some("still"), Some(false)));
    assert!(f.ws.root.join(post["file"].as_str().unwrap()).is_file());
    assert!(post["file"].as_str().unwrap().starts_with("promo/posts/"));

    let mut plan = Plan::open(&f.ws).unwrap();
    plan.set_post(&id, &obj(json!({"done": true}))).unwrap();
    plan.set_schedule(&obj(json!({"every_days": 14, "channel": "X"}))).unwrap();
    plan.save().unwrap();
    let v = promo::view(&f.ws, day("2026-11-10")).unwrap();
    let p = &v["posting"];
    assert_eq!((p["next"].as_str(), p["days"].as_i64(), p["channel"].as_str()), (Some("2026-11-19"), Some(9), Some("X")));
    assert_eq!(p["posts"][0]["url"].as_str().map(|u| u.starts_with("/files/promo/posts/")), Some(true));
    let posted = p["candidates"].as_array().unwrap().iter().find(|c| c["shot"] == "s04_lantern").unwrap();
    assert_eq!(posted["posted"], true);
    // a post never touches the definition
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));
}
