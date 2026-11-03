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
/// "horror RPG" is horror and "action RPG" is rpg.
const GENRES: &[(&str, &str, &str, &str)] =
    &[genre!("horror"), genre!("cozy"), genre!("rpg"), genre!("action"), genre!("adventure")];

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
    out
}

/// Markdown one heading level down, to sit under a `##` section of the prompt.
fn nested(md: &str) -> String {
    md.lines().map(|l| if l.starts_with('#') { format!("#{l}") } else { l.to_string() }).collect::<Vec<_>>().join("\n")
}

/// Measured hook and title lengths (seconds) for a brief: the genre's medians, or the
/// all-teaser medians when the brief names no known genre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pace {
    /// Median time to the first cut.
    pub hook: f64,
    /// Median length of the final shot.
    pub title: f64,
}

pub fn pace(brief: &BriefCfg) -> Pace {
    let r = genre_for(&brief.genre).unwrap_or_else(craft);
    let all = craft();
    let pick = |k: &str, fallback: f64| r.median(k).or_else(|| all.median(k)).unwrap_or(fallback);
    Pace { hook: pick("first_cut", 3.0), title: pick("last_shot", 4.5) }
}
