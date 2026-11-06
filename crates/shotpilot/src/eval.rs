//! Evaluation harness: how close does a planned cut come to the teasers it should reproduce?
//!
//! Quality here is measured by code, never by asking a model to review itself (D-22). A
//! plan ([`ai::Planned`]) is applied to the project in memory, resolved, linted with the
//! shot check and compared with a [`Target`] built from the brief: the measured rhythm of
//! the teaser style the brief follows, else its genre package, else every teaser. The
//! runner calls the planner over cases x models x runs and keeps every raw reply, so
//! [`replay`] can re-score the saved replies with the current code and zero model calls.
//!
//! Score weights (total 0..100):
//! - `contract` 20: the plan parses and resolves; each code repair in `Planned::adjustments`
//!   (beat order, durations clamped, runtime scaled, lines dropped) costs 8 points of this part.
//! - `checks` 30: P0 findings cost 40 points of this part each, P1 8, P2 2.
//! - `structure` 35: piece count 20, mean piece 15, first cut 15, last shot 10, quarter density
//!   25, beat shares 15 (the hook / establish / escalate / title split of the runtime against
//!   the measured teaser's, `refs::beats`). Bands are scored with [`band_score`].
//! - `text` 15: each script line rule broken costs 12 points of this part, each line the
//!   code had to lengthen for reading time 4.
//! - `pass` = parses, no P0, total >= 70.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::ai::{self, Claude};
use crate::error::{Error, Result};
use crate::model::{self, Definition};
use crate::refs;
use crate::script::{self, Piece};
use crate::shotcheck::{self, Severity};
use crate::yamlio::Doc;
use crate::{bail, Workspace};

pub const W_CONTRACT: f64 = 20.0;
pub const W_CHECKS: f64 = 30.0;
pub const W_STRUCTURE: f64 = 35.0;
pub const W_TEXT: f64 = 15.0;
pub const PASS_TOTAL: f64 = 70.0;

/// The note `ai::plan` writes when it lengthens a line for reading time (D-49).
const LENGTHENED: &str = "文字が読めるように";

/// Line rules of TEASER-TYPE.md section 10 (the numbers `craft::script_text` gives the planner).
const MAX_JA_CHARS: usize = 15;
const MAX_EN_WORDS: usize = 6;
/// Kinds read as headlines: no closing 。
const HEADLINE_KINDS: &[&str] = &["card", "rule", "count", "title"];

// --------------------------------------------------------------------------- target

/// A measured range: inside `lo..=hi` is full marks, `mid` is the typical value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub lo: f64,
    pub mid: f64,
    pub hi: f64,
}

impl Band {
    /// A single measurement with the +-20% noise of the frame measurements.
    fn around(v: f64) -> Band {
        Band { lo: v * 0.8, mid: v, hi: v * 1.2 }
    }


    fn to_json(self) -> Value {
        json!({"lo": r2(self.lo), "mid": r2(self.mid), "hi": r2(self.hi)})
    }
}

/// What a plan is compared with.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    /// "style:<id>", "genre:<id>" or "all".
    pub source: String,
    /// Runtime of the measured teaser(s), seconds.
    pub duration: f64,
    /// Cuts (pieces of the cut) per second of runtime.
    pub cuts_per_sec: Band,
    /// Mean piece length, seconds.
    pub asl: Band,
    pub first_cut: Band,
    pub last_shot: Band,
    /// Cuts by quarter, normalised to mean 1.
    pub density: [f64; 4],
    /// Share of the runtime each beat takes (hook, establish, escalate, title).
    pub beats: Option<[f64; 4]>,
}

impl Target {
    /// The shot count a plan of `runtime` seconds should have.
    pub fn count_for(&self, runtime: f64) -> Band {
        Band {
            lo: self.cuts_per_sec.lo * runtime,
            mid: self.cuts_per_sec.mid * runtime,
            hi: self.cuts_per_sec.hi * runtime,
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "source": self.source, "duration": r2(self.duration), "cuts_per_sec": self.cuts_per_sec.to_json(),
            "asl": self.asl.to_json(), "first_cut": self.first_cut.to_json(), "last_shot": self.last_shot.to_json(),
            "density": self.density.map(r2), "beats": self.beats.map(|b| b.map(r2)),
        })
    }
}

fn band_of(r: &refs::Reference, metric: &str) -> Option<Band> {
    r.band(metric).map(|(lo, mid, hi)| Band { lo, mid, hi })
}

/// The reproduction target for a project's brief: the style it names, else its genre
/// package, else the all-genre craft numbers. Where a style lacks a number, the genre's
/// (or every teaser's) stands in, as `refs::pace` does.
pub fn target(defn: &Definition) -> Target {
    let all = refs::craft();
    let genre = refs::genre_for(&defn.brief.genre);
    let base = genre.unwrap_or(all);
    let pick = |m: &str| band_of(base, m).or_else(|| band_of(all, m));
    let epm = pick("edits_per_min").unwrap_or(Band { lo: 8.0, mid: 15.0, hi: 25.0 });
    let mut t = Target {
        source: genre.map_or("all".to_string(), |g| format!("genre:{}", g.id)),
        duration: pick("duration").map_or(45.0, |b| b.mid),
        cuts_per_sec: Band { lo: epm.lo / 60.0, mid: epm.mid / 60.0, hi: epm.hi / 60.0 },
        // mean piece length: runtime / cuts = 60 / edits per minute (more edits, shorter pieces)
        asl: Band { lo: 60.0 / epm.hi, mid: 60.0 / epm.mid, hi: 60.0 / epm.lo },
        first_cut: pick("first_cut").unwrap_or(Band::around(3.0)),
        last_shot: pick("last_shot").unwrap_or(Band::around(4.5)),
        density: base.density_by_quarter().or_else(|| all.density_by_quarter()).unwrap_or([1.0; 4]),
        beats: refs::beats(&defn.brief).map(|b| b.share),
    };
    if let Some(s) = refs::style(&defn.brief.style) {
        t.source = format!("style:{}", s.id);
        if s.duration > 0.0 {
            t.duration = s.duration;
        }
        if let (Some(n), true) = (s.shot_count(), s.duration > 0.0) {
            t.cuts_per_sec = Band::around(n / s.duration);
        }
        if let Some(a) = s.asl() {
            t.asl = Band::around(a);
        }
        if let Some(v) = s.first_cut() {
            t.first_cut = Band::around(v);
        }
        if let Some(v) = s.last_shot() {
            t.last_shot = Band::around(v);
        }
        let q = s.density().or_else(|| {
            let c = s.cuts_by_quarter()?;
            let mean = c.iter().sum::<f64>() / 4.0;
            (mean > 0.0).then(|| c.map(|v| v / mean))
        });
        if let Some(q) = q {
            t.density = q;
        }
    }
    t
}

/// 1 inside the band, falling linearly to 0 at twice the band's half width beyond its edge
/// (about 3x the band, as the measurements carry about +-20% noise).
pub fn band_score(x: f64, b: &Band) -> f64 {
    let outside = if x < b.lo {
        b.lo - x
    } else if x > b.hi {
        x - b.hi
    } else {
        return 1.0;
    };
    let half = (b.hi - b.lo) / 2.0;
    if half <= 0.0 {
        return 0.0;
    }
    (1.0 - outside / (2.0 * half)).clamp(0.0, 1.0)
}

/// Mean absolute difference of two beat splits: <= 0.04 is full marks, >= 0.16 none.
fn beats_score(got: &[f64; 4], want: &[f64; 4]) -> (f64, f64) {
    let d = got.iter().zip(want).map(|(a, b)| (a - b).abs()).sum::<f64>() / 4.0;
    (d, (1.0 - (d - 0.04) / 0.12).clamp(0.0, 1.0))
}

/// Mean absolute difference of two quarter profiles: <= 0.25 is full marks, >= 0.75 none.
fn density_score(got: &[f64; 4], want: &[f64; 4]) -> (f64, f64) {
    let d = got.iter().zip(want).map(|(a, b)| (a - b).abs()).sum::<f64>() / 4.0;
    (d, (1.0 - (d - 0.25) / 0.5).clamp(0.0, 1.0))
}

// --------------------------------------------------------------------------- score

#[derive(Debug, Clone)]
pub struct Score {
    /// 0..100
    pub total: f64,
    pub pass: bool,
    pub parts: BTreeMap<String, f64>,
    pub facts: Value,
    pub notes: Vec<String>,
}

impl Score {
    pub fn to_json(&self) -> Value {
        json!({"total": r2(self.total), "pass": self.pass, "parts": self.parts.iter().map(|(k, v)| (k.clone(), json!(r2(*v)))).collect::<serde_json::Map<_, _>>(),
               "facts": self.facts, "notes": self.notes})
    }

    fn zero(note: String) -> Score {
        let parts = ["contract", "checks", "structure", "text"].iter().map(|k| (k.to_string(), 0.0)).collect();
        Score { total: 0.0, pass: false, parts, facts: json!({}), notes: vec![note] }
    }

    fn p0(&self) -> f64 {
        self.facts["p0"].as_f64().unwrap_or(0.0)
    }
}

fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The project with the plan applied, in memory (nothing is written).
fn apply(defn: &Definition, plan: &ai::Planned) -> Result<Definition> {
    let mut value = Doc::load(&defn.path)?.value()?;
    let top = value.as_object_mut().ok_or_else(|| Error::user("shots.yaml: top level must be a mapping"))?;
    top.insert("shots".into(), Value::Array(plan.shots.clone()));
    top.insert("script".into(), Value::Array(plan.script.clone()));
    let mut finish = match top.get("finish") {
        Some(Value::Object(m)) => m.clone(),
        _ => serde_json::Map::new(),
    };
    for (k, v) in &plan.finish {
        finish.insert(k.clone(), v.clone());
    }
    top.insert("finish".into(), Value::Object(finish));
    model::parse(&value, &defn.path)
}

/// Pieces of the cut in order: (id, start s, length s, is_shot, kind).
struct Piece1 {
    start: f64,
    len: f64,
    shot: bool,
    kind: String,
    /// The shot's beat; a card belongs to the beat of the shot before it (hook before any).
    beat: usize,
}

fn pieces(d: &Definition, shots: &[model::ResolvedShot]) -> Vec<Piece1> {
    let fps = d.teaser.fps.max(1);
    let mut beat = 0;
    script::layout(&d.script, shots, fps)
        .iter()
        .map(|it| {
            let (shot, kind) = match it.piece {
                Piece::Shot(i) => {
                    let b = shots[i].beat.as_deref().and_then(|b| refs::BEAT_ORDER.iter().position(|x| *x == b));
                    beat = b.unwrap_or(beat);
                    (true, "shot".to_string())
                }
                Piece::Slot(i) => (false, d.script[i].kind.clone()),
            };
            Piece1 { start: it.start as f64 / fps as f64, len: it.frames as f64 / fps as f64, shot, kind, beat }
        })
        .collect()
}

/// The length of the final piece that is not black: what the measurement calls the last
/// shot. The references were cut-detected (research_data/tools/build_refs.py), so a title
/// card on black after the last shot is a shot of its own there, and so it is here.
fn closing_len(p: &[Piece1]) -> f64 {
    p.iter().rev().find(|x| x.shot || x.kind != "black").map_or(0.0, |x| x.len)
}

fn text_violations(d: &Definition) -> Vec<String> {
    let mut out = Vec::new();
    for l in &d.script {
        let chars = l.ja.chars().filter(|c| !c.is_whitespace()).count();
        if l.kind != "title" && chars > MAX_JA_CHARS {
            out.push(format!("{}: {} characters (max {MAX_JA_CHARS})", l.id, chars));
        }
        let words = l.en.split_whitespace().count();
        if words > MAX_EN_WORDS {
            out.push(format!("{}: {} English words (max {MAX_EN_WORDS})", l.id, words));
        }
        if HEADLINE_KINDS.contains(&l.kind.as_str()) && l.ja.trim_end().ends_with('。') {
            out.push(format!("{}: headline ends with 。", l.id));
        }
        if l.ja.chars().chain(l.en.chars()).any(|c| ('０'..='９').contains(&c)) {
            out.push(format!("{}: full-width digits", l.id));
        }
    }
    out
}

/// Score a plan against the project it was made for.
pub fn score(defn: &Definition, plan: &ai::Planned) -> Result<Score> {
    let applied = apply(defn, plan).and_then(|d| model::resolve_all(&d).map(|s| (d, s)));
    let (d, shots) = match applied {
        Ok(x) => x,
        Err(e) => return Ok(Score::zero(format!("does not parse or resolve: {}", e.msg))),
    };
    let mut notes = Vec::new();

    // contract
    let (lengthened, repairs): (Vec<&String>, Vec<&String>) = plan.adjustments.iter().partition(|a| a.contains(LENGTHENED));
    let contract = (100.0 - 8.0 * repairs.len() as f64).max(0.0);
    notes.extend(repairs.iter().map(|a| format!("code repaired: {a}")));

    // checks
    let found = shotcheck::check(&d, &shots);
    let n = |s: Severity| found.iter().filter(|f| f.severity == s).count();
    let (p0, p1, p2) = (n(Severity::P0), n(Severity::P1), n(Severity::P2));
    let checks = (100.0 - 40.0 * p0 as f64 - 8.0 * p1 as f64 - 2.0 * p2 as f64).max(0.0);
    notes.extend(found.iter().filter(|f| f.severity == Severity::P0).map(|f| format!("P0 {}: {}", f.rule, f.message)));

    // structure
    let t = target(defn);
    let p = pieces(&d, &shots);
    let runtime = p.last().map_or(0.0, |x| x.start + x.len);
    let n_shots = shots.len();
    let n_pieces = p.len();
    let first_cut = p.first().map_or(0.0, |x| x.len);
    // an end card is cut after everything else (cut.rs)
    let last_shot = d.finish.end_card.as_ref().map_or_else(|| closing_len(&p), |(_, secs)| *secs);
    let mean_shot = if n_pieces > 0 { runtime / n_pieces as f64 } else { 0.0 };
    let mut q = [0.0f64; 4];
    for x in p.iter().skip(1) {
        let i = ((x.start / runtime.max(0.001)) * 4.0).floor().clamp(0.0, 3.0) as usize;
        q[i] += 1.0;
    }
    let qmean = q.iter().sum::<f64>() / 4.0;
    let density = if qmean > 0.0 { q.map(|v| v / qmean) } else { [0.0; 4] };
    let want_count = t.count_for(runtime);
    let (dist, dscore) = density_score(&density, &t.density);
    let mut split = [0.0f64; 4];
    for x in &p {
        split[x.beat] += x.len / runtime.max(0.001);
    }
    // without measured beats the part is neither won nor lost
    let (beats_dist, bscore) = t.beats.map_or((0.0, 1.0), |want| beats_score(&split, &want));
    let sub = [
        ("count", 20.0, band_score(n_pieces as f64, &want_count)),
        ("mean_shot", 15.0, band_score(mean_shot, &t.asl)),
        ("first_cut", 15.0, band_score(first_cut, &t.first_cut)),
        ("last_shot", 10.0, band_score(last_shot, &t.last_shot)),
        ("density", 25.0, dscore),
        ("beats", 15.0, bscore),
    ];
    let structure = sub.iter().map(|(_, w, s)| w * s).sum::<f64>();
    let structure_parts: serde_json::Map<String, Value> = sub.iter().map(|(k, _, s)| (k.to_string(), json!(r2(*s)))).collect();

    // text
    let violations = text_violations(&d);
    let text = (100.0 - 12.0 * violations.len() as f64 - 4.0 * lengthened.len() as f64).max(0.0);
    notes.extend(violations.iter().map(|v| format!("text: {v}")));

    let mut parts = BTreeMap::new();
    parts.insert("contract".to_string(), contract);
    parts.insert("checks".to_string(), checks);
    parts.insert("structure".to_string(), structure);
    parts.insert("text".to_string(), text);
    let total = (W_CONTRACT * contract + W_CHECKS * checks + W_STRUCTURE * structure + W_TEXT * text) / 100.0;
    let pass = p0 == 0 && total >= PASS_TOTAL;

    let facts = json!({
        "shots": n_shots, "pieces": n_pieces, "runtime": r2(runtime), "first_cut": r2(first_cut), "last_shot": r2(last_shot),
        "mean_shot": r2(mean_shot), "density": density.map(r2), "density_distance": r2(dist),
        "beats": split.map(r2), "beats_distance": r2(beats_dist),
        "target": t.to_json(), "target_count": want_count.to_json(), "structure_parts": structure_parts,
        "p0": p0, "p1": p1, "p2": p2, "findings": found.iter().map(|f| format!("{} {}", f.severity.as_str(), f.rule)).collect::<Vec<_>>(), "adjustments": plan.adjustments.len(), "lengthened_lines": lengthened.len(),
        "text_violations": violations,
    });
    Ok(Score { total, pass, parts, facts, notes })
}

// --------------------------------------------------------------------------- runner

/// Sets the model on every request and keeps the request and the raw reply of the last call.
pub struct WithModel<'a> {
    inner: &'a dyn Claude,
    model: String,
    request: std::cell::RefCell<Option<Value>>,
    reply: std::cell::RefCell<Option<String>>,
}

impl<'a> WithModel<'a> {
    pub fn new(inner: &'a dyn Claude, model: &str) -> WithModel<'a> {
        WithModel { inner, model: model.to_string(), request: Default::default(), reply: Default::default() }
    }

    pub fn reply(&self) -> Option<String> {
        self.reply.borrow().clone()
    }

    pub fn spec_hash(&self) -> String {
        self.request.borrow().as_ref().map(spec_hash).unwrap_or_default()
    }
}

/// A model id ending in this is asked without a schema to hold it to: the schema goes into
/// the system prompt as text, as an OpenAI-compatible server without `json_schema` gets it.
/// Measures how the code's repairs hold up without constrained decoding.
pub const NO_SCHEMA: &str = "+noschema";

impl Claude for WithModel<'_> {
    fn ask(&self, request: &Value) -> Result<String> {
        let mut r = request.clone();
        let (model, free) = match self.model.strip_suffix(NO_SCHEMA) {
            Some(m) => (m, true),
            None => (self.model.as_str(), false),
        };
        if !model.is_empty() {
            r["model"] = json!(model);
        }
        let schema = r["output_config"]["format"]["schema"].clone();
        if free && !schema.is_null() {
            r["system"] = json!(crate::connect::schema_in_system(r["system"].as_str().unwrap_or(""), &schema));
            if let Some(cfg) = r["output_config"].as_object_mut() {
                cfg.remove("format");
            }
        }
        *self.request.borrow_mut() = Some(r.clone());
        let text = self.inner.ask(&r)?;
        *self.reply.borrow_mut() = Some(text.clone());
        Ok(text)
    }
}

/// Answers every request with a saved reply.
struct Replay(String);

impl Claude for Replay {
    fn ask(&self, _request: &Value) -> Result<String> {
        Ok(self.0.clone())
    }
}

/// SHA-256 (first 12 hex) of the system text, every text block of the user turn and the
/// output schema; images are left out. Ties a score to the exact spec the model was given.
pub fn spec_hash(request: &Value) -> String {
    let mut h = Sha256::new();
    h.update(request["system"].as_str().unwrap_or("").as_bytes());
    match &request["messages"][0]["content"] {
        Value::String(s) => h.update(s.as_bytes()),
        Value::Array(blocks) => {
            for b in blocks.iter().filter(|b| b["type"] == "text") {
                h.update(b"\n--\n");
                h.update(b["text"].as_str().unwrap_or("").as_bytes());
            }
        }
        _ => {}
    }
    h.update(b"\n--\n");
    h.update(request["output_config"]["format"]["schema"].to_string().as_bytes());
    format!("{:x}", h.finalize())[..12].to_string()
}

#[derive(Debug, Clone)]
pub struct Row {
    pub case: String,
    pub model: String,
    pub run: usize,
    pub secs: f64,
    pub ok: bool,
    pub error: Option<String>,
    pub score: Option<Score>,
    pub spec_hash: String,
    pub reply_file: Option<String>,
}

impl Row {
    pub fn total(&self) -> f64 {
        self.score.as_ref().map_or(0.0, |s| s.total)
    }

    pub fn passed(&self) -> bool {
        self.score.as_ref().is_some_and(|s| s.pass)
    }

    pub fn to_json(&self) -> Value {
        json!({"case": self.case, "model": self.model, "run": self.run, "secs": r2(self.secs), "ok": self.ok, "error": self.error,
               "score": self.score.as_ref().map(Score::to_json), "spec_hash": self.spec_hash, "reply_file": self.reply_file})
    }
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub rows: Vec<Row>,
}

/// Per-model figures.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelStat {
    pub model: String,
    pub runs: usize,
    pub ok_pct: f64,
    pub pass_pct: f64,
    pub mean_total: f64,
    pub stdev_total: f64,
    pub mean_p0: f64,
    pub mean_secs: f64,
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
}

fn stdev(v: &[f64]) -> f64 {
    let m = mean(v);
    if v.len() < 2 { 0.0 } else { (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt() }
}

impl Report {
    pub fn models(&self) -> Vec<String> {
        self.rows.iter().map(|r| r.model.clone()).collect::<BTreeSet<_>>().into_iter().collect()
    }

    pub fn cases(&self) -> Vec<String> {
        self.rows.iter().map(|r| r.case.clone()).collect::<BTreeSet<_>>().into_iter().collect()
    }

    pub fn spec_hashes(&self) -> BTreeSet<String> {
        self.rows.iter().filter(|r| !r.spec_hash.is_empty()).map(|r| r.spec_hash.clone()).collect()
    }

    pub fn by_model(&self) -> Vec<ModelStat> {
        self.models()
            .into_iter()
            .map(|m| {
                let rows: Vec<&Row> = self.rows.iter().filter(|r| r.model == m).collect();
                let pct = |f: &dyn Fn(&Row) -> bool| 100.0 * rows.iter().filter(|r| f(r)).count() as f64 / rows.len().max(1) as f64;
                let totals: Vec<f64> = rows.iter().map(|r| r.total()).collect();
                let p0s: Vec<f64> = rows.iter().map(|r| r.score.as_ref().map_or(0.0, Score::p0)).collect();
                let secs: Vec<f64> = rows.iter().map(|r| r.secs).collect();
                ModelStat {
                    model: m.clone(),
                    runs: rows.len(),
                    ok_pct: pct(&|r| r.ok),
                    pass_pct: pct(&|r| r.passed()),
                    mean_total: mean(&totals),
                    stdev_total: stdev(&totals),
                    mean_p0: mean(&p0s),
                    mean_secs: mean(&secs),
                }
            })
            .collect()
    }

    /// Mean total of one case x model cell.
    pub fn cell(&self, case: &str, model: &str) -> Option<f64> {
        let t: Vec<f64> = self.rows.iter().filter(|r| r.case == case && r.model == model).map(Row::total).collect();
        (!t.is_empty()).then(|| mean(&t))
    }

    pub fn to_markdown(&self, title: &str) -> String {
        let mut s = format!("# {title}\n\n## By model\n\n| model | runs | ok % | pass % | mean total | stdev | mean P0 | mean secs |\n|---|---|---|---|---|---|---|---|\n");
        for m in self.by_model() {
            s.push_str(&format!(
                "| {} | {} | {:.0} | {:.0} | {:.1} | {:.1} | {:.2} | {:.1} |\n",
                m.model, m.runs, m.ok_pct, m.pass_pct, m.mean_total, m.stdev_total, m.mean_p0, m.mean_secs
            ));
        }
        let models = self.models();
        s.push_str(&format!("\n## Mean total by case\n\n| case | {} |\n|---|{}\n", models.join(" | "), "---|".repeat(models.len())));
        for c in self.cases() {
            let cells: Vec<String> = models.iter().map(|m| self.cell(&c, m).map_or("-".into(), |v| format!("{v:.1}"))).collect();
            s.push_str(&format!("| {c} | {} |\n", cells.join(" | ")));
        }
        s.push_str(&format!("\n## Spec hashes\n\n{}\n", self.spec_hashes().into_iter().collect::<Vec<_>>().join(", ")));
        s
    }
}

fn slug(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' }).collect()
}

/// A case's id: the folder its shots.yaml sits in.
pub fn case_id(path: &Path) -> String {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let dir = abs.parent().and_then(Path::file_name).and_then(|n| n.to_str());
    match (abs.file_name().and_then(|n| n.to_str()), dir) {
        (Some("shots.yaml"), Some(d)) => d.to_string(),
        (Some(f), _) => f.trim_end_matches(".yaml").to_string(),
        _ => "case".into(),
    }
}

fn append_line(path: &Path, v: &Value) -> Result<()> {
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{v}")?;
    Ok(())
}

/// Plan every case with every model `runs` times, score each plan and write the results
/// under `out_dir`: `results.jsonl`, `replies/`, `summary.md`. The case files are only read.
pub fn run(cases: &[PathBuf], models: &[String], runs: usize, out_dir: &Path, make: &dyn Fn(&str) -> Box<dyn Claude>) -> Result<Report> {
    if cases.is_empty() {
        bail!("no cases: put projects under eval/cases/<id>/shots.yaml or pass their paths");
    }
    std::fs::create_dir_all(out_dir.join("replies"))?;
    let results = out_dir.join("results.jsonl");
    std::fs::write(&results, "")?;
    let mut report = Report::default();
    let models: Vec<String> = if models.is_empty() { vec![ai::model_name()] } else { models.to_vec() };
    for model_id in &models {
        let client = make(model_id);
        for case in cases {
            let ws = Workspace::open(case)?; // read-only
            let id = case_id(case);
            for run in 1..=runs.max(1) {
                let wrapped = WithModel::new(client.as_ref(), model_id);
                let t0 = Instant::now();
                let planned = ai::plan(&ws.defn, None, None, None, None, &wrapped);
                let secs = t0.elapsed().as_secs_f64();
                // the reply the kept plan was read from (with several candidates, not the last one)
                let kept = planned.as_ref().ok().map(|p| p.reply.clone()).filter(|r| !r.is_empty());
                let reply_file = match kept.or_else(|| wrapped.reply()) {
                    Some(text) => {
                        let rel = format!("replies/{id}-{}-{run}.json", slug(model_id));
                        let saved = json!({"case": id, "case_path": std::path::absolute(case)?.display().to_string(), "model": model_id, "run": run, "text": text});
                        std::fs::write(out_dir.join(&rel), serde_json::to_string_pretty(&saved)?)?;
                        Some(rel)
                    }
                    None => None,
                };
                let (ok, error, score) = match planned {
                    Ok(p) => match score(&ws.defn, &p) {
                        Ok(mut s) => {
                            if !p.candidates.is_empty() {
                                s.facts["candidates"] = json!(p.candidates);
                            }
                            (true, None, Some(s))
                        }
                        Err(e) => (false, Some(e.msg), None),
                    },
                    Err(e) => (false, Some(e.msg), None),
                };
                let row = Row { case: id.clone(), model: model_id.clone(), run, secs, ok, error, score, spec_hash: wrapped.spec_hash(), reply_file };
                append_line(&results, &row.to_json())?;
                report.rows.push(row);
            }
        }
    }
    std::fs::write(out_dir.join("summary.md"), report.to_markdown("Eval summary"))?;
    Ok(report)
}

/// Re-score every reply saved under `dir` with the current code, no model calls: the saved
/// text goes through `ai::plan` again, so repairs and scoring changes show up in the numbers.
/// Writes `replay.jsonl` and `replay.md` there.
pub fn replay(dir: &Path) -> Result<Report> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join("replies"))
        .map_err(|e| Error::user(format!("{}: no replies to replay ({e})", dir.display())))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let out = dir.join("replay.jsonl");
    std::fs::write(&out, "")?;
    let mut report = Report::default();
    for f in files {
        let saved: Value = serde_json::from_str(&std::fs::read_to_string(&f)?)?;
        let rel = format!("replies/{}", f.file_name().and_then(|n| n.to_str()).unwrap_or(""));
        let case_path = PathBuf::from(saved["case_path"].as_str().unwrap_or(""));
        let ws = Workspace::open(&case_path)?;
        let client = Replay(saved["text"].as_str().unwrap_or("").to_string());
        let wrapped = WithModel::new(&client, "");
        let t0 = Instant::now();
        let planned = ai::plan(&ws.defn, None, None, None, None, &wrapped);
        let secs = t0.elapsed().as_secs_f64();
        let (ok, error, score) = match planned {
            Ok(p) => match score(&ws.defn, &p) {
                Ok(s) => (true, None, Some(s)),
                Err(e) => (false, Some(e.msg), None),
            },
            Err(e) => (false, Some(e.msg), None),
        };
        let row = Row {
            case: saved["case"].as_str().unwrap_or("").to_string(),
            model: saved["model"].as_str().unwrap_or("").to_string(),
            run: saved["run"].as_u64().unwrap_or(0) as usize,
            secs, ok, error, score, spec_hash: wrapped.spec_hash(), reply_file: Some(rel),
        };
        append_line(&out, &row.to_json())?;
        report.rows.push(row);
    }
    std::fs::write(dir.join("replay.md"), report.to_markdown("Replay (current code, saved replies)"))?;
    Ok(report)
}
