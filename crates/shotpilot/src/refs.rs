//! Measured teaser references, embedded at build time (D-29).
//!
//! open-design hands the model a design system before it writes. Here the same role is
//! played by `references/craft/` (what holds for any game teaser) and
//! `references/genres/<id>/` (one genre), both measured from published teasers with
//! ffmpeg (`research_data/tools/`). Each package is prose (`*.md`, read by people and by
//! the planner) plus numbers (`tokens.json`, read by code).
//!
//! The planner gets the prose and a short line of the numbers. The skeleton takes its hook
//! and title lengths from the same numbers, so the text and the code cannot disagree.

use crate::model::BriefCfg;
use serde_json::Value;
use std::sync::OnceLock;

const CRAFT_DOC: &str = include_str!("../../../references/craft/TEASER-CRAFT.md");
const CRAFT_TOKENS: &str = include_str!("../../../references/craft/tokens.json");
const BASELINE_DOC: &str = include_str!("../../../references/craft/TEASER-BASELINE.md");

/// The teaser baseline (D-49): checkable values, what to avoid, and defect -> decision, as
/// open-design's Design baseline. The header above the first `##` is for maintainers.
pub fn baseline_text() -> String {
    let body = BASELINE_DOC.find("\n## ").map_or(BASELINE_DOC, |i| &BASELINE_DOC[i + 1..]);
    format!("\n## Teaser baseline (checkable values, what to avoid, defect -> decision)\n{}\n", nested(body.trim_end()))
}

const TYPE_DOC: &str = include_str!("../../../references/craft/TEASER-TYPE.md");
const MOTION_DOC: &str = include_str!("../../../references/motion/MOTION.md");

/// The writing side of the motion rules (D-53): section 6 of references/motion/MOTION.md.
/// The vocabulary's numbers stay with the code (motion.rs), like `type_text`.
pub fn motion_text() -> String {
    let body = MOTION_DOC.find("\n## 6.").map_or("", |i| &MOTION_DOC[i + 1..]);
    let body = body.split_once('\n').map_or("", |(_, b)| b);
    format!(
        "\n## Text motion (which entrance a line asks for; the look's profile moves the rest)\n{}\n\n## Camera effects\n{}",
        nested(body.trim()),
        crate::fx::guide()
    )
}

/// The writing side of the type rules (D-52): section 10 of TEASER-TYPE.md, the only part a
/// writer of the script acts on. Like open-design injecting the craft sections a skill
/// requires, the rest of the file (sizes, faces, tracking) stays with the code that draws.
pub fn type_text() -> String {
    let body = TYPE_DOC.find("\n## 10.").map_or("", |i| &TYPE_DOC[i + 1..]);
    let body = body.split_once('\n').map_or("", |(_, b)| b);
    format!("\n## On-screen type (how the script's lines are written; the look sets the faces and sizes)\n{}\n", nested(body.trim()))
}

macro_rules! genre {
    ($id:literal) => {
        (
            $id,
            include_str!(concat!("../../../references/genres/", $id, "/GENRE.md")),
            include_str!(concat!("../../../references/genres/", $id, "/manifest.json")),
            include_str!(concat!("../../../references/genres/", $id, "/tokens.json")),
        )
    };
}

/// Genre packages in match order: the first whose alias appears in `brief.genre` wins, so
/// "horror RPG" is horror, "tactics RPG" is strategy and "action RPG" is rpg.
const GENRES: &[(&str, &str, &str, &str)] = &[
    genre!("horror"),
    genre!("cozy"),
    genre!("strategy"),
    genre!("rpg"),
    genre!("action"),
    genre!("adventure"),
];

const AUTO_END: &str = "<!-- auto:end -->";

/// One reference package: prose plus measured numbers.
#[derive(Debug)]
pub struct Reference {
    pub id: &'static str,
    /// Display name, e.g. "ホラー".
    pub name: String,
    /// How many teasers were measured.
    pub n: u64,
    pub aliases: Vec<String>,
    doc: &'static str,
    tokens: Value,
}

impl Reference {
    fn median(&self, metric: &str) -> Option<f64> {
        self.tokens["metrics"][metric]["median"].as_f64()
    }

    /// The hand-written part of the document (below the generated tables).
    pub fn prose(&self) -> &'static str {
        self.doc.split_once(AUTO_END).map(|(_, rest)| rest).unwrap_or(self.doc).trim()
    }
}

fn parse(tokens: &str) -> Value {
    serde_json::from_str(tokens).unwrap_or(Value::Null)
}

/// The numbers over every measured teaser.
pub fn craft() -> &'static Reference {
    static CELL: OnceLock<Reference> = OnceLock::new();
    CELL.get_or_init(|| {
        let tokens = parse(CRAFT_TOKENS);
        Reference {
            id: "all",
            name: "all genres".into(),
            n: tokens["n"].as_u64().unwrap_or(0),
            aliases: Vec::new(),
            doc: CRAFT_DOC,
            tokens,
        }
    })
}

/// Every genre package, in match order.
pub fn genres() -> &'static [Reference] {
    static CELL: OnceLock<Vec<Reference>> = OnceLock::new();
    CELL.get_or_init(|| {
        GENRES
            .iter()
            .map(|(id, doc, manifest, tokens)| {
                let m = parse(manifest);
                Reference {
                    id,
                    name: m["name"].as_str().unwrap_or(id).to_string(),
                    n: m["n"].as_u64().unwrap_or(0),
                    aliases: m["aliases"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_lowercase).collect(),
                    doc,
                    tokens: parse(tokens),
                }
            })
            .collect()
    })
}

/// The genre package a brief's free-text genre points at ("一人称ホラー" -> horror).
pub fn genre_for(text: &str) -> Option<&'static Reference> {
    let low = text.trim().to_lowercase();
    if low.is_empty() {
        return None;
    }
    let all = genres();
    all.iter()
        .find(|g| g.id == low)
        .or_else(|| all.iter().find(|g| g.aliases.iter().any(|a| low.contains(a.as_str()))))
}

fn g(v: Option<f64>) -> String {
    v.map(|x| crate::model::fmt_g(crate::geometry::round_to(x, if x.abs() >= 100.0 { 0 } else { 1 })))
        .unwrap_or_else(|| "?".into())
}

fn quarters(r: &Reference, key: &str) -> String {
    r.tokens["profiles"][key].as_array().map(|q| q.iter().map(|v| g(v.as_f64())).collect::<Vec<_>>().join(" -> ")).unwrap_or_default()
}

/// The numbers a planner needs, in one short paragraph. `base` adds the all-genre median.
fn numbers(r: &Reference, base: Option<&Reference>) -> String {
    let m = |k: &str, unit: &str| {
        let own = g(r.median(k));
        match base {
            Some(b) => format!("{own}{unit} (all: {}{unit})", g(b.median(k))),
            None => format!("{own}{unit}"),
        }
    };
    let flag = |k: &str| g(r.tokens["flags"][k].as_f64().map(|v| v * 100.0));
    format!(
        "Medians over {n} teasers: runtime {dur}s; cuts per minute {epm}; median shot {shot}; first cut at {first}; \
         final shot {last}; dips to black per minute {blk}, {tail}% dip to black in the last third; \
         brightness {luma} (of 235); saturation {sat}; loudness {lufs} LUFS, range {lra}. \
         Cut density by quarter (1 = the teaser's own average): {eq}. Loudness by quarter against the whole (LU): {lq}.",
        n = r.n,
        dur = g(r.median("duration")),
        epm = m("edits_per_min", ""),
        shot = m("median_shot", "s"),
        first = m("first_cut", "s"),
        last = m("last_shot", "s"),
        blk = m("blacks_per_min", ""),
        tail = flag("black_in_tail"),
        luma = m("luma", ""),
        sat = m("sat", ""),
        lufs = g(r.median("lufs")),
        lra = m("lra", " LU"),
        eq = quarters(r, "edits_by_quarter"),
        lq = quarters(r, "loudness_by_quarter_vs_integrated"),
    )
}

/// What the planner reads about published teasers: the universal craft, and the genre
/// package when the brief names one.
pub fn planner_text(brief: &BriefCfg) -> String {
    let all = craft();
    let mut out = format!(
        "## Measured teaser craft ({} published game teasers)\n\
         How published teasers are cut, measured frame by frame. A shot here is one camera take, so a teaser has \
         about half as many shots as the cut counts below; the skeleton already reflects that.\n{}\n\n{}\n",
        all.n,
        numbers(all, None),
        nested(all.prose())
    );
    if let Some(r) = genre_for(&brief.genre) {
        out.push_str(&format!(
            "\n## Genre reference: {} ({}, {} teasers)\nThis genre against all teasers.\n{}\n\n{}\n",
            r.id,
            r.name,
            r.n,
            numbers(r, Some(all)),
            nested(r.prose())
        ));
    }
    if let Some(s) = style(&brief.style) {
        out.push_str(&style_text(s));
    }
    out
}

/// Markdown one heading level down, to sit under a `##` section of the prompt.
fn nested(md: &str) -> String {
    md.lines().map(|l| if l.starts_with('#') { format!("#{l}") } else { l.to_string() }).collect::<Vec<_>>().join("\n")
}

/// The measured pace a brief's skeleton follows: hook and title lengths (seconds) and how
/// dense the cutting is in each quarter. From the style the brief follows (D-39), else
/// the genre's medians, else every teaser's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pace {
    /// Time to the first cut.
    pub hook: f64,
    /// Length of the final shot.
    pub title: f64,
    /// Edit density by quarter against the average (1 = average).
    pub density: [f64; 4],
}

impl Reference {
    fn density(&self) -> Option<[f64; 4]> {
        let q: Vec<f64> = self.tokens["profiles"]["edits_by_quarter"].as_array()?.iter().filter_map(Value::as_f64).collect();
        relative(&q)
    }
}

pub fn pace(brief: &BriefCfg) -> Pace {
    let r = genre_for(&brief.genre).unwrap_or_else(craft);
    let all = craft();
    let pick = |k: &str, fallback: f64| r.median(k).or_else(|| all.median(k)).unwrap_or(fallback);
    let base = Pace {
        hook: pick("first_cut", 3.0),
        title: pick("last_shot", 4.5),
        density: r.density().or_else(|| all.density()).unwrap_or([1.0; 4]),
    };
    match style(&brief.style) {
        Some(s) => Pace {
            hook: s.first_cut().unwrap_or(base.hook),
            title: s.last_shot().unwrap_or(base.title),
            density: s.density().unwrap_or(base.density),
        },
        None => base,
    }
}

// --------------------------------------------------------------------------- styles (D-39)

macro_rules! style {
    ($genre:literal, $id:literal) => {
        (
            $genre,
            $id,
            include_str!(concat!("../../../references/teasers/", $id, "/TEASER.md")),
            include_str!(concat!("../../../references/teasers/", $id, "/manifest.json")),
            include_str!(concat!("../../../references/teasers/", $id, "/tokens.json")),
        )
    };
}

/// Recently popular teasers to imitate ("流行りの型"), four per genre package: game teasers
/// uploaded in 2025 or later, 20-100 s, among the most viewed of their genre (a mix of
/// indie and well-known), each analysed from its contact sheet (TEASER.md sections 1-7).
const STYLES: &[(&str, &str, &str, &str, &str)] = &[
    style!("horror", "yt-fAXzLmsbrd8"),
    style!("horror", "yt-yd6oGFqE1F8"),
    style!("horror", "yt-IkXbe0zTAck"),
    style!("horror", "yt-LnydjhGRH1s"),
    style!("cozy", "yt-ZLXfWsIWog8"),
    style!("cozy", "yt-QtGJNp2QWEw"),
    style!("cozy", "yt-CP61GovN7ZE"),
    style!("cozy", "yt-vmVVsImRjHo"),
    style!("strategy", "yt-fGgsNvxaA04"),
    style!("strategy", "yt-UpH6HB-Km_0"),
    style!("strategy", "yt--uDHQlmGstM"),
    style!("strategy", "yt-QW_7oiMLM6o"),
    style!("rpg", "yt-9lwef2jan-Q"),
    style!("rpg", "yt-3idSGSoMMn8"),
    style!("rpg", "yt-AZmYsFsGVMk"),
    style!("rpg", "yt-ayW7oCXggoE"),
    style!("action", "yt-luWunttkCgY"),
    style!("action", "yt-0lEO0u1_ooY"),
    style!("action", "yt-h8AcT7N-OoU"),
    style!("action", "yt-olOTdoW6XsQ"),
    style!("adventure", "yt-qDFEeeLy6ws"),
    style!("adventure", "yt-GUnZa0ghIow"),
    style!("adventure", "yt-q21jZaUOa4Q"),
    style!("adventure", "yt-7SFCD_MMQ6Q"),
];

/// One real teaser as a style to follow: its measured rhythm and its analysis.
#[derive(Debug)]
pub struct Style {
    pub id: &'static str,
    /// The genre package it belongs to.
    pub genre: &'static str,
    pub name: String,
    pub channel: String,
    pub url: String,
    /// "2026-10"
    pub uploaded: String,
    pub views: u64,
    /// Seconds.
    pub duration: f64,
    /// The first sentence of the analysis: what to imitate, in one line.
    pub summary: String,
    doc: &'static str,
    tokens: Value,
}

impl Style {
    /// The analysis (sections 1-7), below the measured block.
    pub fn prose(&self) -> &'static str {
        self.doc.split_once(AUTO_END).map(|(_, rest)| rest).unwrap_or(self.doc).trim()
    }

    /// Times of its cuts (s).
    pub fn cuts(&self) -> Vec<f64> {
        self.tokens["cut_times_s"].as_array().into_iter().flatten().filter_map(Value::as_f64).collect()
    }

    /// Its spans of black (s).
    pub fn blacks(&self) -> Vec<[f64; 2]> {
        self.tokens["black_intervals_s"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|b| Some([b.get(0)?.as_f64()?, b.get(1)?.as_f64()?]))
            .collect()
    }

    pub fn first_cut(&self) -> Option<f64> {
        self.tokens["edits"]["first_s"].as_f64()
    }

    pub fn last_shot(&self) -> Option<f64> {
        self.tokens["edits"]["last_s"].as_f64()
    }

    pub fn edits_per_min(&self) -> Option<f64> {
        self.tokens["edits"]["per_min"].as_f64()
    }

    /// Edit density by quarter against its own average (1 = average).
    pub fn density(&self) -> Option<[f64; 4]> {
        let q: Vec<f64> = self.tokens["edits"]["by_quarter"].as_array()?.iter().filter_map(Value::as_f64).collect();
        relative(&q)
    }

    /// What the app shows on the style's card.
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "id": self.id, "genre": self.genre, "name": self.name, "channel": self.channel, "url": self.url,
            "uploaded": self.uploaded, "views": self.views, "duration": self.duration, "summary": self.summary,
            "cuts": self.cuts(), "blacks": self.blacks(), "edits_per_min": self.edits_per_min(),
        })
    }
}

/// Four quarter counts as ratios to their mean (None when they are not four or all zero).
fn relative(q: &[f64]) -> Option<[f64; 4]> {
    if q.len() != 4 {
        return None;
    }
    let mean = q.iter().sum::<f64>() / 4.0;
    (mean > 0.0).then(|| [q[0] / mean, q[1] / mean, q[2] / mean, q[3] / mean])
}

/// Every style, in genre order.
pub fn styles() -> &'static [Style] {
    static CELL: OnceLock<Vec<Style>> = OnceLock::new();
    CELL.get_or_init(|| {
        STYLES
            .iter()
            .map(|(genre, id, doc, manifest, tokens)| {
                let m = parse(manifest);
                let t = parse(tokens);
                let name = m["name"].as_str().unwrap_or(id);
                let name = name.split(" | ").next().unwrap_or(name).trim().to_string();
                let up = m["source"]["uploadDate"].as_str().unwrap_or("");
                let summary = doc
                    .split_once(AUTO_END)
                    .and_then(|(_, rest)| rest.split_once("## 1."))
                    .and_then(|(_, s)| s.split_once('\n'))
                    .map(|(_, s)| s.trim().split('。').next().unwrap_or("").trim().to_string() + "。")
                    .unwrap_or_default();
                Style {
                    id,
                    genre,
                    name,
                    channel: m["source"]["channel"].as_str().unwrap_or("").to_string(),
                    url: m["source"]["url"].as_str().unwrap_or("").to_string(),
                    uploaded: if up.len() >= 6 { format!("{}-{}", &up[..4], &up[4..6]) } else { String::new() },
                    views: m["source"]["viewCount"].as_u64().unwrap_or(0),
                    duration: t["duration_s"].as_f64().unwrap_or(0.0),
                    summary,
                    doc,
                    tokens: t,
                }
            })
            .collect()
    })
}

pub fn style(id: &str) -> Option<&'static Style> {
    let id = id.trim();
    styles().iter().find(|s| s.id == id)
}

/// The styles of one genre package.
pub fn styles_for(genre: &str) -> Vec<&'static Style> {
    styles().iter().filter(|s| s.genre == genre).collect()
}

/// The genre packages and their styles, for the app's pickers: name, size, the one line
/// that says what the genre's teasers do, a few measured numbers, and each style's card.
pub fn catalogue() -> Value {
    let all = craft();
    let genres: Vec<Value> = genres()
        .iter()
        .map(|r| {
            let headline = r
                .prose()
                .lines()
                .find_map(|l| l.strip_prefix("## 1. "))
                .map(|l| l.split_once(" — ").map(|(_, h)| h).unwrap_or(l).trim().to_string())
                .unwrap_or_default();
            serde_json::json!({
                "id": r.id, "name": r.name, "n": r.n, "headline": headline, "aliases": r.aliases,
                "edits_per_min": r.median("edits_per_min"), "luma": r.median("luma"),
                "first_cut": r.median("first_cut"), "last_shot": r.median("last_shot"),
                "styles": styles_for(r.id).iter().map(|s| s.to_json()).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::json!({"all": {"n": all.n, "edits_per_min": all.median("edits_per_min")}, "genres": genres})
}

/// What the planner reads about the style a brief follows.
fn style_text(s: &Style) -> String {
    let cuts = s.cuts();
    let quarters = s.density().map(|q| q.map(|v| g(Some(v))).join(" -> ")).unwrap_or_default();
    let blacks: Vec<String> = s.blacks().iter().map(|[a, b]| format!("{}-{}s", g(Some(*a)), g(Some(*b)))).collect();
    format!(
        "\n## Style reference: Inspired by {name} ({uploaded}, {views} views, {dur}s)\n\
         The director picked this recent teaser as the model. Follow its structure, rhythm, camera and use of text \
         and black, translated into this game's own world, anchors and script; never its story, characters or words. \
         Measured: {n} cuts, first cut at {first}s, final shot {last}s, cut density by quarter (1 = its average) \
         {quarters}{blacks}.\n\n{prose}\n",
        name = s.name,
        uploaded = s.uploaded,
        views = s.views,
        dur = g(Some(s.duration)),
        n = cuts.len(),
        first = g(s.first_cut()),
        last = g(s.last_shot()),
        blacks = if blacks.is_empty() { String::new() } else { format!(", black at {}", blacks.join(", ")) },
        prose = nested(s.prose()),
    )
}

// --------------------------------------------------------------------------- accessors for the eval harness

/// Additive read access to the measured numbers (eval.rs scores a plan against them).
impl Reference {
    /// `(p25, median, p75)` of a measured metric over this package's teasers.
    pub fn band(&self, metric: &str) -> Option<(f64, f64, f64)> {
        let m = &self.tokens["metrics"][metric];
        Some((m["p25"].as_f64()?, m["median"].as_f64()?, m["p75"].as_f64()?))
    }

    /// Edit density by quarter against the average (1 = average).
    pub fn density_by_quarter(&self) -> Option<[f64; 4]> {
        self.density()
    }
}

impl Style {
    /// Shots (cuts) it was measured to have.
    pub fn shot_count(&self) -> Option<f64> {
        self.tokens["shot_count"].as_f64()
    }

    /// Mean shot length (s).
    pub fn asl(&self) -> Option<f64> {
        self.tokens["asl_s"].as_f64()
    }

    /// Cuts in each quarter of its runtime, as counts.
    pub fn cuts_by_quarter(&self) -> Option<[f64; 4]> {
        let q: Vec<f64> = self.tokens["cuts_by_quarter"].as_array()?.iter().filter_map(Value::as_f64).collect();
        (q.len() == 4).then(|| [q[0], q[1], q[2], q[3]])
    }
}

// --------------------------------------------------------------------------- beats of a measured teaser

/// The four beats of the teaser type, in order.
pub const BEAT_ORDER: [&str; 4] = ["hook", "establish", "escalate", "title"];

/// Where a teaser's beats fall and how fast each one cuts: what the skeleton copies.
#[derive(Debug, Clone, PartialEq)]
pub struct Beats {
    /// Share of the runtime each beat takes (hook, establish, escalate, title; sums to 1).
    pub share: [f64; 4],
    /// Pieces (shots and cards) per second inside each beat.
    pub rate: [f64; 4],
    /// Where it came from: a style id, `genre:<id>` or `all`.
    pub source: String,
}

/// The first number in `s` (digits and one dot), with the rest of the string.
fn leading_number(s: &str) -> Option<(f64, &str)> {
    let s = s.trim_start();
    let end = s.char_indices().find(|(_, c)| !(c.is_ascii_digit() || *c == '.')).map_or(s.len(), |(i, _)| i);
    Some((s[..end].parse().ok()?, &s[end..]))
}

/// A beat line of section 2 of an analysis: `- **hook（0–3.7s）**: ...` -> ("hook", 0.0).
fn beat_line(line: &str) -> Option<(usize, f64)> {
    let rest = line.trim().strip_prefix("- **")?;
    let beat = BEAT_ORDER.iter().position(|b| rest.starts_with(b))?;
    let open = rest.find(['（', '('])?;
    let inside = &rest[open + rest[open..].chars().next()?.len_utf8()..];
    let (start, after) = leading_number(inside)?;
    // a range follows: "0–3.7s", "0-3.7", "0〜3.7"
    after.trim_start().starts_with(['–', '-', '~', '〜']).then_some((beat, start))
}

impl Style {
    /// When each beat starts (s), read from section 2 of the analysis. None unless all four
    /// are there in order (the first line of a beat counts).
    pub fn beat_starts(&self) -> Option<[f64; 4]> {
        let mut starts = [None; 4];
        for (beat, at) in self.prose().lines().filter_map(beat_line) {
            starts[beat].get_or_insert(at);
        }
        let s = [starts[0]?, starts[1]?, starts[2]?, starts[3]?];
        (s[0] <= 0.5 && s.windows(2).all(|w| w[0] < w[1]) && s[3] < self.duration).then_some(s)
    }

    /// Its beats: each one runs from its start to the next one's (gaps the analysis leaves go
    /// to the beat before), and its pieces are the measured cuts inside it plus the one it
    /// opens with.
    pub fn beats(&self) -> Option<Beats> {
        let starts = self.beat_starts()?;
        let cuts = self.cuts();
        let mut share = [0.0; 4];
        let mut rate = [0.0; 4];
        for i in 0..4 {
            let (a, b) = (starts[i], if i == 3 { self.duration } else { starts[i + 1] });
            // a cut within 0.3 s of a beat's start is the cut that opens it
            let inside = cuts.iter().filter(|c| **c > a + 0.3 && **c < b - 0.3).count() as f64;
            share[i] = (b - a) / self.duration;
            rate[i] = (inside + 1.0) / (b - a);
        }
        // a beat boundary the analysis puts between cuts counts one piece too many: scale so
        // the beats hold as many pieces as the teaser was measured to have
        let pieces: f64 = (0..4).map(|i| rate[i] * share[i] * self.duration).sum();
        let k = (cuts.len() as f64 + 1.0) / pieces;
        Some(Beats { share, rate: rate.map(|r| r * k), source: self.id.to_string() })
    }
}

/// The average of some styles' beats (None when none of them has its beats written down).
fn mean_beats(of: &[&'static Style], source: String) -> Option<Beats> {
    let all: Vec<Beats> = of.iter().filter_map(|s| s.beats()).collect();
    if all.is_empty() {
        return None;
    }
    let n = all.len() as f64;
    let mut b = Beats { share: [0.0; 4], rate: [0.0; 4], source };
    for x in &all {
        for i in 0..4 {
            b.share[i] += x.share[i] / n;
            b.rate[i] += x.rate[i] / n;
        }
    }
    Some(b)
}

/// The beats a brief's skeleton follows: the style it names, else the average of its genre's
/// styles, else of every style.
pub fn beats(brief: &BriefCfg) -> Option<Beats> {
    if let Some(b) = style(&brief.style).and_then(Style::beats) {
        return Some(b);
    }
    if let Some(b) = genre_for(&brief.genre).and_then(|r| mean_beats(&styles_for(r.id), format!("genre:{}", r.id))) {
        return Some(b);
    }
    mean_beats(&styles().iter().collect::<Vec<_>>(), "all".into())
}
