//! Teaser skills (D-43), after open-design's skills: one playbook per kind of teaser
//! (`references/skills/<id>/SKILL.md`, YAML front matter + a Markdown body).
//!
//! The planning turn reads the catalogue (name and description) and picks the skills that fit
//! the request; the picks go to `brief.skills`. When the shot list is written, the planner gets
//! each picked skill's whole body. `main` skills shape the whole teaser (one, sometimes two);
//! `utility` skills shape one part (the opening, the title, the end card, a count).

use std::sync::OnceLock;

use serde_json::{json, Value};

use crate::model::{str_of, BriefCfg};

macro_rules! skill {
    ($id:literal) => {
        ($id, include_str!(concat!("../../../references/skills/", $id, "/SKILL.md")))
    };
}

const SKILLS: &[(&str, &str)] = &[
    skill!("announce-teaser"),
    skill!("mood-teaser"),
    skill!("gameplay-first"),
    skill!("release-date"),
    skill!("demo-teaser"),
    skill!("character-reveal"),
    skill!("world-tour"),
    skill!("threat-reveal"),
    skill!("rules-teaser"),
    skill!("diary-teaser"),
    skill!("verb-montage"),
    skill!("one-take"),
    skill!("update-teaser"),
    skill!("mystery-hook"),
    skill!("cold-open"),
    skill!("title-drop"),
    skill!("end-card"),
    skill!("count-run"),
];

#[derive(Debug)]
pub struct Skill {
    pub id: &'static str,
    /// Display name for the app (D-41), e.g. "規則で見せる".
    pub ja_name: String,
    pub description: String,
    pub triggers: Vec<String>,
    /// main | utility
    pub mode: String,
    /// teaser | gameplay | any
    pub kind: String,
    /// Genre package ids it suits (empty: any).
    pub genres: Vec<String>,
    pub example_prompt: String,
    /// The Markdown body after the front matter.
    pub body: &'static str,
}

impl Skill {
    pub fn is_main(&self) -> bool {
        self.mode == "main"
    }

    /// The H1 of the body.
    pub fn title(&self) -> &str {
        self.body.lines().find(|l| l.starts_with("# ")).map(|l| l[2..].trim()).unwrap_or(self.id)
    }

    pub fn to_json(&self) -> Value {
        json!({"id": self.id, "name": self.ja_name, "title": self.title(), "description": self.description, "mode": self.mode,
               "kind": self.kind, "genres": self.genres, "example": self.example_prompt})
    }
}

/// Split `---\n<yaml>\n---\n<body>`.
fn split_front(text: &'static str) -> (&'static str, &'static str) {
    let t = text.strip_prefix("---").unwrap_or(text);
    match t.find("\n---") {
        Some(i) => {
            let body = &t[i + 4..];
            (&t[..i], body.strip_prefix('\n').unwrap_or(body))
        }
        None => ("", text),
    }
}

pub fn all() -> &'static [Skill] {
    static CELL: OnceLock<Vec<Skill>> = OnceLock::new();
    CELL.get_or_init(|| {
        SKILLS
            .iter()
            .map(|(id, text)| {
                let (front, body) = split_front(text);
                let m = crate::yamlio::parse_str(front, id).unwrap_or(Value::Null);
                let sp = &m["sp"];
                let list = |v: &Value| v.as_array().into_iter().flatten().map(str_of).collect::<Vec<_>>();
                Skill {
                    id,
                    ja_name: m["ja_name"].as_str().unwrap_or(id).to_string(),
                    description: m["description"].as_str().unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" "),
                    triggers: list(&m["triggers"]),
                    mode: sp["mode"].as_str().unwrap_or("main").to_string(),
                    kind: sp["kind"].as_str().unwrap_or("any").to_string(),
                    genres: list(&sp["genres"]),
                    example_prompt: sp["example_prompt"].as_str().unwrap_or("").to_string(),
                    body,
                }
            })
            .collect()
    })
}

pub fn get(id: &str) -> Option<&'static Skill> {
    all().iter().find(|s| s.id == id)
}

/// The skills active for a brief: the ones it names, else one main skill by kind.
pub fn active(brief: &BriefCfg) -> Vec<&'static Skill> {
    let named: Vec<&Skill> = brief.skills.iter().filter_map(|id| get(id)).collect();
    if named.iter().any(|s| s.is_main()) {
        return named;
    }
    let fallback = if brief.is_gameplay() { "gameplay-first" } else { "announce-teaser" };
    let mut out: Vec<&Skill> = get(fallback).into_iter().collect();
    out.extend(named);
    out
}

/// The catalogue the planning turn chooses from (open-design's `{{SKILL_CATALOG}}`).
pub fn catalogue_text() -> String {
    all()
        .iter()
        .map(|s| {
            let genres = if s.genres.is_empty() { "any genre".to_string() } else { s.genres.join(", ") };
            format!("- `{}` ({}, {}, {}): {}", s.id, s.mode, s.kind, genres, s.description)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The whole body of each active skill, for the planner (open-design's `## Active skill`).
pub fn planner_text(brief: &BriefCfg) -> String {
    active(brief)
        .iter()
        .map(|s| format!("\n## Active skill — {}\nFollow this skill's workflow.\n\n{}\n", s.id, s.body.trim()))
        .collect()
}
