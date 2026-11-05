//! Design systems for teasers (D-45), after open-design's `design-systems/`.
//!
//! A design system is one look a teaser can wear: `DESIGN.md` (prose in nine sections, the
//! planner reads it whole and treats it as authoritative), `tokens.json` (the numbers code
//! uses: grade, letterbox, typeface, faces, title size, motion, card holds) and `manifest.json` (id, name, genres). The
//! bundled ones in `references/design-systems/` are generic looks, never a studio's identity.
//!
//! A project can have its own `DESIGN.md` next to shots.yaml (the game's brand). It wins over
//! a bundled one for the prose; the tokens still come from `brief.design` when it names one.
//! Everything here is editorial: it changes the cut, never a take.

use std::path::Path;
use std::sync::OnceLock;

use serde_json::{json, Value};

use crate::model::{num, str_of, Definition};

macro_rules! ds {
    ($id:literal) => {
        (
            $id,
            include_str!(concat!("../../../references/design-systems/", $id, "/DESIGN.md")),
            include_str!(concat!("../../../references/design-systems/", $id, "/manifest.json")),
            include_str!(concat!("../../../references/design-systems/", $id, "/tokens.json")),
        )
    };
}

const SYSTEMS: &[(&str, &str, &str, &str)] = &[
    ds!("quiet-dread"),
    ds!("vhs-found-footage"),
    ds!("cozy-pastel"),
    ds!("storybook"),
    ds!("grimdark"),
    ds!("epic-fantasy"),
    ds!("neon-action"),
    ds!("retro-pixel"),
    ds!("arthouse-minimal"),
    ds!("noir-mystery"),
    ds!("cold-scifi"),
    ds!("command-room"),
];

/// The file a project keeps its own design system in (next to shots.yaml).
pub const PROJECT_FILE: &str = "DESIGN.md";

#[derive(Debug)]
pub struct DesignSystem {
    pub id: &'static str,
    /// Display name, e.g. "静かな恐怖".
    pub name: String,
    /// Genre package id it was made for (horror, cozy, ...).
    pub category: String,
    /// One line for the app.
    pub description: String,
    /// Genre package ids it suits.
    pub genres: Vec<String>,
    pub doc: &'static str,
    pub tokens: Value,
}

impl DesignSystem {
    /// The title line of DESIGN.md without the heading mark.
    pub fn title(&self) -> &str {
        self.doc.lines().next().unwrap_or("").trim_start_matches('#').trim()
    }

    pub fn grade(&self) -> String {
        self.tokens["finish"]["grade"].as_str().unwrap_or("none").to_string()
    }

    /// Picture aspect inside bars; None for full frame.
    pub fn letterbox(&self) -> Option<f64> {
        num(&self.tokens["finish"]["letterbox"]).filter(|r| (1.0..=3.0).contains(r))
    }

    pub fn typeface(&self) -> String {
        self.tokens["finish"]["typeface"].as_str().unwrap_or("mincho").to_string()
    }

    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id, "name": self.name, "category": self.category, "description": self.description,
            "genres": self.genres, "title": self.title(), "finish": self.tokens["finish"],
            "palette": self.tokens["palette"], "type": self.tokens["type"],
        })
    }
}

pub fn all() -> &'static [DesignSystem] {
    static CELL: OnceLock<Vec<DesignSystem>> = OnceLock::new();
    CELL.get_or_init(|| {
        SYSTEMS
            .iter()
            .map(|(id, doc, manifest, tokens)| {
                let m: Value = serde_json::from_str(manifest).unwrap_or(Value::Null);
                DesignSystem {
                    id,
                    name: m["name"].as_str().unwrap_or(id).to_string(),
                    category: m["category"].as_str().unwrap_or("").to_string(),
                    description: m["description"].as_str().unwrap_or("").to_string(),
                    genres: m["genres"].as_array().into_iter().flatten().map(str_of).collect(),
                    doc,
                    tokens: serde_json::from_str(tokens).unwrap_or(Value::Null),
                }
            })
            .collect()
    })
}

pub fn get(id: &str) -> Option<&'static DesignSystem> {
    all().iter().find(|d| d.id == id)
}

/// The project's own DESIGN.md, when it has one.
pub fn project_doc(root: &Path) -> Option<String> {
    std::fs::read_to_string(root.join(PROJECT_FILE)).ok().filter(|t| !t.trim().is_empty())
}

/// What the planner reads about the look, in open-design's order: identity, the whole
/// DESIGN.md, then the tokens. Empty when no design system is active.
pub fn planner_text(defn: &Definition) -> String {
    let preset = get(&defn.brief.design);
    let own = project_doc(&defn.root());
    let (title, doc) = match (&own, preset) {
        (Some(d), _) => ("the game's own DESIGN.md".to_string(), d.as_str()),
        (None, Some(p)) => (format!("{} ({})", p.name, p.id), p.doc),
        (None, None) => return String::new(),
    };
    let mut out = format!(
        "\n## Active design system — {title}\n\
         Treat the following DESIGN.md as authoritative for grade, letterbox, typeface, card wording style, \
         holds, transitions and camera language. Do not invent a look outside it; where the brief or the \
         director's direction asks for something specific, that wins.\n\n{}\n",
        doc.trim()
    );
    if let Some(p) = preset {
        out.push_str(&format!(
            "\n## Active design system tokens\nUse these values for `grade`, `letterbox` and the card holds:\n```json\n{}\n```\n",
            serde_json::to_string_pretty(&p.tokens).unwrap_or_default()
        ));
    }
    out
}

/// One line per bundled design system, for the planning turn to choose from.
pub fn catalogue_text() -> String {
    all()
        .iter()
        .map(|d| format!("- `{}` {} [{}]: {}", d.id, d.name, d.genres.join(", "), d.title()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The finish values a design system sets (grade, letterbox, typeface, and its typefaces and
/// title size), as `finish:` entries.
pub fn finish_entries(d: &DesignSystem) -> Vec<(String, Value)> {
    let mut out = vec![
        ("grade".to_string(), json!(Some(d.grade()).filter(|g| g != "none"))),
        ("letterbox".to_string(), json!(d.letterbox())),
        ("typeface".to_string(), json!(d.typeface())),
    ];
    out.extend(look_entries(d));
    out
}

/// The look's typefaces (`type.display` / `body` / `latin`, "Family:weight" from the font
/// catalogue, D-52), title size and motion (`motion`: a profile of
/// references/motion/profiles.json and its overrides, D-53), as `finish:` entries.
pub fn look_entries(d: &DesignSystem) -> Vec<(String, Value)> {
    let t = &d.tokens["type"];
    let face = |k: &str| t[k].as_str().filter(|s| crate::fonts::Spec::parse(s).is_ok()).map(str::to_string);
    let fonts = json!({"display": face("display"), "body": face("body"), "latin": face("latin")});
    let motion = match &d.tokens["motion"] {
        Value::Object(m) if !m.is_empty() => Value::Object(m.clone()),
        _ => Value::Null,
    };
    vec![
        ("fonts".to_string(), fonts),
        ("title_size".to_string(), json!(num(&t["title_size"]).filter(|r| (0.04..=0.35).contains(r)))),
        ("motion".to_string(), motion),
    ]
}
