//! The D-13 loop against a model: long random sequences of edits, undos, renders, game
//! changes and approvals, checked after every step against what the loop promises.
//!
//! The model knows only the spec, never the fingerprint code: which fields change a shot's
//! pixels (focal length, duration and the time of day here) and which are editorial (purpose,
//! beat, notes, order, caption, transition, exposure, fx, sound, brief). It also reopens the
//! project now and then, so everything it checks has to survive in shots.yaml and state.json. From that alone it predicts which shots render,
//! which reuse a take, what goes stale, which approvals hold, and what undo brings back. A
//! failure prints the seed and every step so far: a counterexample with its repro steps.
//!
//! `SHOTPILOT_MODEL_SEED=<n>` replays one seed; `SHOTPILOT_MODEL_RUNS` and
//! `SHOTPILOT_MODEL_STEPS` run more and longer sequences locally.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{launches, ws, Fixture};
use serde_json::{json, Value};

/// The demo's `defaults.lens.focal_length`: what a shot without its own lens is shot at.
const DEFAULT_FOCAL: f64 = 35.0;
/// The demo's `defaults.environment.time_of_day`.
const DEFAULT_TIME: &str = "dusk";
const TIMES: &[&str] = &["dusk", "night", "dawn"];
const EXPOSURES: &[f64] = &[-0.3, 0.2];
const BEATS: &[&str] = &["hook", "establish", "escalate", "title"];
const FOCALS: &[f64] = &[18.0, 24.0, 35.0, 50.0, 85.0];
const DURATIONS: &[f64] = &[3.0, 3.5, 4.0, 5.0];
const PURPOSES: &[&str] = &["門の外から覗く", "何かがいる気配", "影が一瞬だけ映る"];
const NOTES: &[&str] = &["上1/3を空ける", "音で落とす"];
const CAPTIONS: &[&str] = &["誰もいないはずだった", "七人目"];
const TRANSITIONS: &[&str] = &["cut", "dip"];
const GAINS: &[f64] = &[-2.0, -4.0, -6.0];
const GENRES: &[&str] = &["ホラー", "ミステリー"];
/// The demo's anchors; each has a content file the mock engine reports when it is in view.
const ANCHORS: &[&str] = &["gate", "well", "window", "lantern", "door"];
const QUALITIES: &[&str] = &["previz", "still"];

/// xorshift64*: a reproducible sequence without a crate.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

/// One shot as the model sees it: the raw values it writes, and its picture.
#[derive(Clone, Debug, PartialEq)]
struct Shot {
    focal: Option<f64>,
    duration: f64,
    time: Option<String>,
    purpose: String,
    beat: String,
    exposure: Option<f64>,
    notes: Option<String>,
    caption: Option<String>,
    transition: Option<String>,
    fx: bool,
    sfx: Option<(Value, f64)>,
}

/// What reaches the pixels.
type Picture = (u64, u64, String);

impl Shot {
    fn picture(&self) -> Picture {
        (self.focal.unwrap_or(DEFAULT_FOCAL).to_bits(), self.duration.to_bits(),
         self.time.clone().unwrap_or_else(|| DEFAULT_TIME.to_string()))
    }
}

/// A take the model knows about: the picture it shows, and (engine takes) the content files
/// it showed with their versions then.
#[derive(Clone, Debug)]
struct Take {
    picture: Picture,
    deps: BTreeMap<String, u32>,
}

enum Undo {
    Shot(String, Box<Shot>),
    Order(Vec<String>),
}

struct Model {
    order: Vec<String>,
    shots: BTreeMap<String, Shot>,
    /// (shot, quality) -> every take rendered so far (never overwritten)
    takes: BTreeMap<(String, String), Vec<Take>>,
    content: BTreeMap<String, u32>,
    undo: Vec<Undo>,
    approved: BTreeMap<String, Picture>,
    script: Option<Vec<(String, String, String, u64)>>,
}

impl Model {
    /// What the script approval is tied to (D-38): each shot's id, beat, purpose and length, in order.
    fn script_state(&self) -> Vec<(String, String, String, u64)> {
        self.order
            .iter()
            .map(|id| {
                let s = &self.shots[id];
                (id.clone(), s.beat.clone(), s.purpose.clone(), s.duration.to_bits())
            })
            .collect()
    }

    fn fresh(&self, id: &str, q: &str) -> bool {
        let p = self.shots[id].picture();
        self.takes.get(&(id.to_string(), q.to_string())).into_iter().flatten().any(|t| {
            t.picture == p && (q == "previz" || t.deps.iter().all(|(f, v)| self.content.get(f) == Some(v)))
        })
    }

    fn status(&self, id: &str, q: &str) -> &'static str {
        match self.takes.get(&(id.to_string(), q.to_string())) {
            None => "none",
            Some(_) if self.fresh(id, q) => "fresh",
            Some(_) => "stale",
        }
    }

    fn approval(&self, id: &str) -> &'static str {
        match self.approved.get(id) {
            None => "none",
            Some(p) if *p == self.shots[id].picture() => "approved",
            Some(_) => "outdated",
        }
    }

    fn script_approval(&self) -> &'static str {
        match &self.script {
            None => "none",
            Some(s) if *s == self.script_state() => "approved",
            Some(_) => "outdated",
        }
    }
}

fn read_model(f: &Fixture) -> Model {
    let mut shots = BTreeMap::new();
    for s in &f.ws.defn.shots {
        let id = s["id"].as_str().unwrap().to_string();
        let sfx = s["sfx"].as_array().and_then(|a| a.first()).map(|e| (e.clone(), e["gain"].as_f64().unwrap_or(0.0)));
        shots.insert(id, Shot {
            focal: s["camera"]["lens"]["focal_length"].as_f64(),
            duration: s["duration"].as_f64().unwrap(),
            time: s["environment"]["time_of_day"].as_str().map(str::to_string),
            purpose: s["purpose"].as_str().unwrap_or("").to_string(),
            beat: s["beat"].as_str().unwrap_or("").to_string(),
            exposure: None,
            notes: s["notes"].as_str().map(str::to_string),
            caption: None,
            transition: None,
            fx: false,
            sfx,
        });
    }
    Model {
        order: f.ws.defn.shot_ids(),
        shots,
        takes: BTreeMap::new(),
        content: ANCHORS.iter().map(|a| (format!("content/{a}.txt"), 1)).collect(),
        undo: Vec::new(),
        approved: BTreeMap::new(),
        script: None,
    }
}

fn write_content(f: &Fixture, file: &str, version: u32) {
    let p = f.ws.root.join(file);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, format!("{file} version {version}")).unwrap();
}

/// A random value different from `now`.
fn other<T: Clone + PartialEq>(rng: &mut Rng, xs: &[T], now: &T) -> T {
    let choices: Vec<&T> = xs.iter().filter(|x| *x != now).collect();
    choices[rng.below(choices.len())].clone()
}

fn run(seed: u64, steps: usize, seen: &mut BTreeMap<&'static str, usize>) {
    let mut f = ws();
    for a in ANCHORS {
        write_content(&f, &format!("content/{a}.txt"), 1);
    }
    let mut m = read_model(&f);
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut log: Vec<String> = Vec::new();
    // steps queued by an earlier step (an "undo everything", a cut that needs previz first),
    // taken from the end
    let mut queued: Vec<usize> = Vec::new();
    let mut previz_all = false;

    macro_rules! promise {
        ($cond:expr, $($what:tt)+) => {
            if !$cond {
                panic!("seed {seed}, step {}: {}\nsteps so far:\n  {}\nreplay: SHOTPILOT_MODEL_SEED={seed} cargo test --test model",
                       log.len(), format!($($what)+), log.join("\n  "));
            }
        };
    }

    for _ in 0..steps {
        let id = rng.pick(&m.order).clone();
        let before = m.shots[&id].clone();
        let roll = queued.pop().unwrap_or_else(|| rng.below(122));
        // ---------------------------------------------------------------- definition edits
        let edit: Option<(Vec<(String, Value)>, Shot)> = match roll {
            0..=11 => {
                let mut s = before.clone();
                let now = before.focal.unwrap_or(f64::NAN);
                s.focal = Some(other(&mut rng, FOCALS, &now));
                Some((vec![("camera.lens.focal_length".into(), json!(s.focal))], s))
            }
            12..=19 => {
                let mut s = before.clone();
                s.duration = other(&mut rng, DURATIONS, &before.duration);
                Some((vec![("duration".into(), json!(s.duration))], s))
            }
            20..=25 => {
                let mut s = before.clone();
                s.purpose = other(&mut rng, &PURPOSES.iter().map(|p| p.to_string()).collect::<Vec<_>>(), &before.purpose);
                Some((vec![("purpose".into(), json!(s.purpose))], s))
            }
            26..=29 => {
                let mut s = before.clone();
                s.notes = Some(other(&mut rng, &NOTES.iter().map(|p| p.to_string()).collect::<Vec<_>>(), &before.notes.clone().unwrap_or_default()));
                Some((vec![("notes".into(), json!(s.notes))], s))
            }
            30..=33 => {
                let mut s = before.clone();
                s.caption = Some(other(&mut rng, &CAPTIONS.iter().map(|p| p.to_string()).collect::<Vec<_>>(), &before.caption.clone().unwrap_or_default()));
                Some((vec![("caption".into(), json!(s.caption))], s))
            }
            34..=36 => {
                let mut s = before.clone();
                s.transition = Some(other(&mut rng, &TRANSITIONS.iter().map(|p| p.to_string()).collect::<Vec<_>>(), &before.transition.clone().unwrap_or_default()));
                Some((vec![("transition".into(), json!(s.transition))], s))
            }
            37..=39 => {
                let mut s = before.clone();
                s.fx = !before.fx;
                Some((vec![("fx".into(), if s.fx { json!(["chroma"]) } else { Value::Null })], s))
            }
            100..=105 => {
                let mut s = before.clone();
                let now = before.time.clone().unwrap_or_default();
                s.time = Some(other(&mut rng, &TIMES.iter().map(|t| t.to_string()).collect::<Vec<_>>(), &now));
                Some((vec![("environment.time_of_day".into(), json!(s.time))], s))
            }
            106..=109 => {
                let mut s = before.clone();
                s.exposure = Some(other(&mut rng, EXPOSURES, &before.exposure.unwrap_or(0.0)));
                Some((vec![("exposure".into(), json!(s.exposure))], s))
            }
            110..=113 => {
                let mut s = before.clone();
                s.beat = other(&mut rng, &BEATS.iter().map(|b| b.to_string()).collect::<Vec<_>>(), &before.beat);
                Some((vec![("beat".into(), json!(s.beat))], s))
            }
            _ => None,
        };
        if let Some((assign, after)) = edit {
            log.push(format!("set {id} {}", assign.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(" ")));
            let rep = f.ws.set_fields(&id, &assign).unwrap_or_else(|e| panic!("seed {seed}: {} failed: {}", log.last().unwrap(), e.msg));
            let want: Vec<String> = if after.picture() != before.picture() { vec![id.clone()] } else { vec![] };
            promise!(rep.stale == want, "an edit of {id} reported stale {:?}, the picture says {want:?}", rep.stale);
            m.shots.insert(id.clone(), after);
            m.undo.push(Undo::Shot(id.clone(), Box::new(before)));
        } else {
            match roll {
                40..=44 => {
                    let mut order = m.order.clone();
                    while order == m.order {
                        for i in (1..order.len()).rev() {
                            order.swap(i, rng.below(i + 1));
                        }
                    }
                    log.push(format!("order {}", order.join(",")));
                    let rep = f.ws.edit_definition(&[], Some(&order), "order", "").unwrap();
                    promise!(rep.stale.is_empty(), "reordering made {:?} stale", rep.stale);
                    m.undo.push(Undo::Order(std::mem::replace(&mut m.order, order)));
                }
                45..=48 => {
                    // a sound edit: the gain of the first effect of a shot that has one
                    let with: Vec<String> = m.order.iter().filter(|s| m.shots[*s].sfx.is_some()).cloned().collect();
                    let sid = rng.pick(&with).clone();
                    let was = m.shots[&sid].clone();
                    let (entry, gain) = was.sfx.clone().unwrap();
                    let g = other(&mut rng, GAINS, &gain);
                    let mut e = entry.clone();
                    e["gain"] = json!(g);
                    log.push(format!("sound {sid} gain={g}"));
                    let rep = f.ws.edit_sound(&[(sid.clone(), Some(json!([e])))], None, "").unwrap();
                    promise!(rep.stale.is_empty(), "a sound edit made {:?} stale", rep.stale);
                    let mut now = was.clone();
                    now.sfx = Some((entry, g));
                    m.shots.insert(sid.clone(), now);
                    m.undo.push(Undo::Shot(sid, Box::new(was)));
                }
                49..=51 => {
                    let g = *rng.pick(GENRES);
                    log.push(format!("brief genre={g}"));
                    f.ws.set_brief(&[("genre".into(), json!(g))]).unwrap();
                }
                52..=61 => {
                    log.push("undo".into());
                    match (f.ws.undo(), m.undo.pop()) {
                        (Err(e), None) => {
                            *seen.entry("undo with nothing left").or_default() += 1;
                            promise!(e.msg.contains("nothing to undo"), "undo with nothing to undo said: {}", e.msg)
                        }
                        (Err(e), Some(_)) => promise!(false, "undo refused: {}", e.msg),
                        (Ok(_), None) => promise!(false, "undo did something when nothing was left to undo"),
                        (Ok((rep, _)), Some(Undo::Order(order))) => {
                            *seen.entry("undo of a reorder").or_default() += 1;
                            promise!(rep.stale.is_empty(), "undoing an order made {:?} stale", rep.stale);
                            m.order = order;
                        }
                        (Ok((rep, _)), Some(Undo::Shot(sid, was))) => {
                            if was.picture() != m.shots[&sid].picture() {
                                *seen.entry("undo of a picture edit").or_default() += 1;
                            }
                            let want: Vec<String> =
                                if was.picture() != m.shots[&sid].picture() { vec![sid.clone()] } else { vec![] };
                            promise!(rep.stale == want, "undo on {sid} reported stale {:?}, the picture says {want:?}", rep.stale);
                            m.shots.insert(sid, *was);
                        }
                    }
                }
                62..=79 => {
                    let forced = std::mem::take(&mut previz_all);
                    let q = if forced { "previz" } else { *rng.pick(QUALITIES) };
                    let only: Vec<String> = if !forced && rng.below(3) == 0 { vec![id.clone()] } else { vec![] };
                    log.push(format!("render {q} {}", if only.is_empty() { "all".to_string() } else { only.join(",") }));
                    let launched = launches(&f.ws).len();
                    let out = f.ws.render(&only, q, false, None).unwrap();
                    let targets: Vec<String> = if only.is_empty() { m.order.clone() } else { only.clone() };
                    let got: BTreeMap<String, String> = out.iter().map(|o| (o.shot_id.clone(), o.action.clone())).collect();
                    let mut rendered = BTreeSet::new();
                    for t in &targets {
                        let want = if m.fresh(t, q) { "reused" } else { "rendered" };
                        *seen.entry(if want == "reused" { "a take reused" } else { "a shot rendered" }).or_default() += 1;
                        promise!(got.get(t).map(String::as_str) == Some(want), "render {q}: {t} was {:?}, the model says {want}", got.get(t));
                        if want == "rendered" {
                            rendered.insert(t.clone());
                        }
                    }
                    promise!(got.len() == targets.len(), "render {q} touched {:?}, asked for {targets:?}", got.keys());
                    // what the engine was launched for: exactly the shots that rendered, once
                    let after = launches(&f.ws);
                    if q == "previz" || rendered.is_empty() {
                        promise!(after.len() == launched, "render {q} launched the engine with nothing to render on it");
                    } else {
                        promise!(after.len() == launched + 1, "render {q} launched the engine {} times", after.len() - launched);
                        let shots: BTreeSet<String> = after.last().unwrap()["shots"].as_array().unwrap().iter()
                            .map(|k| k.as_str().unwrap().split(':').next().unwrap().to_string()).collect();
                        promise!(shots == rendered, "the engine was launched for {shots:?}, the model renders {rendered:?}");
                    }
                    for o in out.iter().filter(|o| o.action == "rendered") {
                        let take = f.ws.state.takes_for(&o.shot_id, q, &o.fingerprint).into_iter().next().unwrap();
                        let deps: BTreeMap<String, u32> = take.info.get("deps").and_then(Value::as_array).into_iter().flatten()
                            .filter_map(|d| d["file"].as_str()).map(|file| (file.to_string(), m.content[file])).collect();
                        if q != "previz" {
                            promise!(!deps.is_empty(), "the still take of {} showed no content", o.shot_id);
                        }
                        m.takes.entry((o.shot_id.clone(), q.to_string())).or_default().push(Take { picture: m.shots[&o.shot_id].picture(), deps });
                    }
                }
                80..=85 => {
                    let file = format!("content/{}.txt", rng.pick(ANCHORS));
                    let v = other(&mut rng, &[1, 2, 3], &m.content[&file]);
                    log.push(format!("game {file} -> v{v}"));
                    write_content(&f, &file, v);
                    m.content.insert(file, v);
                }
                86..=91 => {
                    log.push(format!("approve {id}"));
                    match f.ws.approve(&id, true) {
                        Ok(()) => {
                            *seen.entry("a shot approved").or_default() += 1;
                            m.approved.insert(id.clone(), m.shots[&id].picture());
                        }
                        Err(e) => {
                            *seen.entry("approval refused for a P0").or_default() += 1;
                            promise!(e.msg.contains("P0"), "approving {id} failed: {}", e.msg)
                        }
                    }
                }
                92..=95 => {
                    log.push("approve script".into());
                    f.ws.approve_script(true).unwrap();
                    m.script = Some(m.script_state());
                }
                96..=97 | 118..=121 => {
                    log.push(format!("undo everything ({} edits)", m.undo.len()));
                    queued.extend(std::iter::repeat(52).take(m.undo.len() + 1)); // an undo per edit, then one more
                    continue;
                }
                114..=117 => {
                    // everything the loop knows has to come back from the files
                    log.push("reopen".into());
                    *seen.entry("reopened").or_default() += 1;
                    f.ws = shotpilot::Workspace::open(&f.ws.path).unwrap();
                }
                _ => {
                    // the cut follows the order and renders nothing
                    if !m.order.iter().all(|s| m.fresh(s, "previz")) {
                        // render the previz of every shot first, then come back to the cut
                        queued.extend([98, 62]);
                        previz_all = true;
                        continue;
                    }
                    log.push("cut previz".into());
                    *seen.entry("a cut").or_default() += 1;
                    let launched = launches(&f.ws).len();
                    f.ws.cut("previz", false).unwrap();
                    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(f.ws.root.join("cut/previz/manifest.json")).unwrap()).unwrap();
                    let ids: Vec<String> = manifest["shots"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap().to_string()).collect();
                    promise!(ids == m.order, "the cut runs {ids:?}, the order is {:?}", m.order);
                    promise!(launches(&f.ws).len() == launched, "the cut launched the engine");
                }
            }
        }

        // ---------------------------------------------------------------- after every step
        promise!(f.ws.defn.shot_ids() == m.order, "the order is {:?}, the model has {:?}", f.ws.defn.shot_ids(), m.order);
        for st in f.ws.status().unwrap() {
            for q in QUALITIES {
                promise!(st.quality(q) == m.status(&st.id, q), "{} at {q} is {}, the model says {}", st.id, st.quality(q), m.status(&st.id, q));
            }
            promise!(st.approval == m.approval(&st.id), "{} approval is {}, the model says {}", st.id, st.approval, m.approval(&st.id));
            if m.approval(&st.id) == "outdated" {
                *seen.entry("an approval dropped by the picture").or_default() += 1;
            }
            // stale because the game changed under a take of this very picture (D-37)
            let p = m.shots[&st.id].picture();
            let game = m.takes.get(&(st.id.clone(), "still".to_string())).is_some_and(|ts| ts.iter().any(|t| t.picture == p));
            if game && m.status(&st.id, "still") == "stale" {
                *seen.entry("stale because the game changed").or_default() += 1;
            }
        }
        if m.script_approval() == "outdated" {
            *seen.entry("the script approval dropped").or_default() += 1;
        }
        promise!(f.ws.script_approval() == m.script_approval(), "the script approval is {}, the model says {}", f.ws.script_approval(), m.script_approval());
    }
}

fn env(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

/// What the sequences must have gone through between them, so a change to the mix of
/// steps cannot quietly stop testing a promise.
const MUST_SEE: &[&str] = &[
    "a shot rendered", "a take reused", "undo of a picture edit", "undo of a reorder", "undo with nothing left", "a shot approved",
    "an approval dropped by the picture", "the script approval dropped", "stale because the game changed",
    "reopened", "a cut",
];

#[test]
fn any_sequence_of_edits_rerenders_exactly_the_shots_whose_picture_changed() {
    let steps = env("SHOTPILOT_MODEL_STEPS").unwrap_or(60) as usize;
    let mut seen = BTreeMap::new();
    if let Some(seed) = env("SHOTPILOT_MODEL_SEED") {
        return run(seed, steps, &mut seen);
    }
    for seed in 0..env("SHOTPILOT_MODEL_RUNS").unwrap_or(6) {
        run(seed, steps, &mut seen);
    }
    eprintln!("model: {seen:?}");
    let missed: Vec<&&str> = MUST_SEE.iter().filter(|k| !seen.contains_key(**k)).collect();
    assert!(missed.is_empty(), "the random sequences never went through {missed:?}: {seen:?}");
}
