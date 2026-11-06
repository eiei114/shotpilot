//! Typefaces for the burned-in text (D-52): the catalogue in `references/fonts/catalog.json`,
//! fetching a family at one weight from Google Fonts into a cache, and asking a font file
//! whether it has the glyphs a line needs.
//!
//! Only families in the catalogue are fetched: every one is under the SIL Open Font License
//! (Kosugi and Kosugi Maru: Apache 2.0), so the files may be cached and used in a video. The
//! system fonts (Yu Mincho, Yu Gothic, ...) are read where they are and never copied.
//!
//! Fetching is off unless the binary turns it on (`allow_fetch`): the CLI and the app do, the
//! library and the tests do not, so a test never reaches the network.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use serde_json::Value;

use crate::error::{Error, Result};

const CATALOG: &str = include_str!("../../../references/fonts/catalog.json");

static FETCH: AtomicBool = AtomicBool::new(false);

/// Let `resolve` download a missing family (the CLI and the app call this at start).
pub fn allow_fetch(on: bool) {
    FETCH.store(on, Ordering::Relaxed);
}

fn fetch_allowed() -> bool {
    FETCH.load(Ordering::Relaxed) && std::env::var("SHOTPILOT_FONT_FETCH").map_or(true, |v| v != "0")
}

/// One family of the catalogue.
#[derive(Debug, Clone)]
pub struct Family {
    pub family: String,
    /// `ja` or `latin`.
    pub script: String,
    /// mincho, gothic, maru, brush, pop, serif, sans-condensed, pixel, ...
    pub class: String,
    pub weights: Vec<u16>,
    /// Japanese families: `full` (JIS levels 1 and 2), `level1`, `partial` or `none` (kana only).
    pub kanji: String,
    pub registers: Vec<String>,
    pub roles: Vec<String>,
    /// What the face is for, in a line (Japanese).
    pub note: String,
}

impl Family {
    /// The available weight nearest to `w`.
    pub fn nearest_weight(&self, w: u16) -> u16 {
        self.weights.iter().copied().min_by_key(|x| (*x as i32 - w as i32).abs()).unwrap_or(400)
    }
}

/// An overused face and what to use instead (open-design's ban list and genre clichés).
#[derive(Debug, Clone)]
pub struct Overused {
    pub family: String,
    pub why: String,
    pub instead: Vec<String>,
}

pub struct Catalog {
    pub families: Vec<Family>,
    pub overused: Vec<Overused>,
    /// The registers each look draws on (`looks`: look id -> registers).
    pub looks: Vec<(String, Vec<String>)>,
}

pub fn catalog() -> &'static Catalog {
    static CELL: OnceLock<Catalog> = OnceLock::new();
    CELL.get_or_init(|| {
        let v: Value = serde_json::from_str(CATALOG).unwrap_or(Value::Null);
        let strs = |x: &Value| -> Vec<String> { x.as_array().into_iter().flatten().filter_map(|s| s.as_str().map(str::to_string)).collect() };
        let fam = |e: &Value| Family {
            family: e["family"].as_str().unwrap_or("").to_string(),
            script: e["script"].as_str().unwrap_or("").to_string(),
            class: e["class"].as_str().unwrap_or("").to_string(),
            weights: e["weights"].as_array().into_iter().flatten().filter_map(|w| w.as_u64()).map(|w| w as u16).collect(),
            kanji: e["coverage"]["kanji"].as_str().unwrap_or(if e["script"] == "latin" { "none" } else { "full" }).to_string(),
            registers: strs(&e["registers"]),
            roles: strs(&e["roles"]),
            note: e["note"].as_str().unwrap_or("").to_string(),
        };
        let families = v["japanese"].as_array().into_iter().flatten().chain(v["latin"].as_array().into_iter().flatten()).map(fam).collect();
        let overused = v["overused"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|e| Overused {
                family: e["family"].as_str().unwrap_or("").to_string(),
                why: e["why"].as_str().unwrap_or("").to_string(),
                instead: strs(&e["instead"]),
            })
            .collect();
        let looks = v["looks"].as_object().into_iter().flatten().map(|(k, regs)| (k.clone(), strs(regs))).collect();
        Catalog { families, overused, looks }
    })
}

/// Faces that can set a title (role `title`, not overused), for the planner's choice (D-59).
pub fn title_faces() -> Vec<&'static Family> {
    catalog().families.iter().filter(|f| f.roles.iter().any(|r| r == "title") && overused(&f.family).is_none()).collect()
}

/// The face a title may take (D-59): a catalogue family with the title role, not overused, with
/// glyphs for the titles it will set (a Latin face sets only a Latin title, a kana face no
/// kanji), at its weight nearest 700. The error says why not, for a note.
pub fn title_spec(name: &str, titles: &[&str]) -> std::result::Result<Spec, String> {
    if let Some(o) = overused(name) {
        return Err(format!("{} is overused (instead: {})", o.family, o.instead.join(", ")));
    }
    let Some(f) = family(name) else { return Err(format!("{} is not in the font catalogue", name.trim())) };
    if !f.roles.iter().any(|r| r == "title") {
        return Err(format!("{} is not a title face", f.family));
    }
    let non_latin = |t: &str| t.chars().any(|c| c.is_alphabetic() && c as u32 > 0x24F);
    let kanji = |t: &str| t.chars().any(|c| matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF));
    if f.script == "latin" && titles.iter().any(|t| non_latin(t)) {
        return Err(format!("{} has no Japanese glyphs for the title", f.family));
    }
    if f.kanji == "none" && titles.iter().any(|t| kanji(t)) {
        return Err(format!("{} has no kanji for the title", f.family));
    }
    Ok(Spec { family: f.family.clone(), weight: f.nearest_weight(700) })
}

/// The title faces a look may reach for: those of its registers, and the worn and broken ones
/// whatever the look (a damaged world shows in its title). Without a look, every title face.
pub fn title_text(look: &str) -> String {
    let regs: Vec<String> = catalog().looks.iter().find(|(k, _)| k == look).map(|(_, r)| r.clone()).unwrap_or_default();
    let mut out = String::from(
        "\n## Title faces (`title_face`)\nThe look sets the faces of the cards and lines. The title may take its own face \
         from this list when the pitch, the plan or the place calls for one: a worn or broken face for a damaged place \
         or a story told through damaged things, a scrawl for a note someone left, a carved or burned face for what \
         the place went through. Leave it empty to keep the look's display face. A Latin face (latin) sets only a \
         Latin title; a face without kanji (kana) sets no kanji.\n",
    );
    for f in title_faces() {
        let reach = regs.is_empty() || f.registers.iter().any(|r| r == "worn" || regs.contains(r));
        if !reach {
            continue;
        }
        let script = if f.script == "latin" { "latin" } else if f.kanji == "none" { "kana" } else { "ja" };
        out.push_str(&format!("- `{}` ({}, {}; {}): {}\n", f.family, script, f.class, f.registers.join("/"), f.note));
    }
    out
}

pub fn family(name: &str) -> Option<&'static Family> {
    catalog().families.iter().find(|f| f.family.eq_ignore_ascii_case(name.trim()))
}

pub fn overused(name: &str) -> Option<&'static Overused> {
    catalog().overused.iter().find(|o| o.family.eq_ignore_ascii_case(name.trim()))
}

/// A family at one weight, written `Family:weight` in shots.yaml (`Shippori Mincho:800`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub family: String,
    pub weight: u16,
}

impl Spec {
    /// Parses `Family` or `Family:weight`; the family must be in the catalogue, the weight
    /// snaps to the nearest one it has.
    pub fn parse(s: &str) -> std::result::Result<Spec, String> {
        let s = s.trim();
        let (name, weight) = match s.rsplit_once(':') {
            Some((n, w)) if w.trim().chars().all(|c| c.is_ascii_digit()) && !w.trim().is_empty() => {
                (n.trim(), w.trim().parse::<u16>().map_err(|_| format!("bad weight in '{s}'"))?)
            }
            _ => (s, 400),
        };
        let Some(f) = family(name) else {
            return Err(format!("'{name}' is not in references/fonts/catalog.json (only those can be fetched and used)"));
        };
        Ok(Spec { family: f.family.clone(), weight: f.nearest_weight(weight) })
    }

    pub fn label(&self) -> String {
        format!("{}:{}", self.family, self.weight)
    }

    fn file_name(&self) -> String {
        let slug: String = self.family.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        format!("{slug}-{}.ttf", self.weight)
    }
}

/// Where fetched fonts are kept: `$SHOTPILOT_FONTS`, else `<config>/shotpilot/fonts`.
pub fn cache_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("SHOTPILOT_FONTS").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    crate::connect::Settings::default_path().and_then(|p| p.parent().map(|d| d.join("fonts")))
}

/// The cached file of a spec, if it is there.
pub fn local(spec: &Spec) -> Option<PathBuf> {
    let p = cache_dir()?.join(spec.file_name());
    p.is_file().then_some(p)
}

/// The file for a spec: the cache, else (when fetching is allowed) Google Fonts.
pub fn resolve(spec: &Spec) -> Result<PathBuf> {
    if let Some(p) = local(spec) {
        return Ok(p);
    }
    if !fetch_allowed() {
        crate::bail!("{} is not downloaded yet", spec.label());
    }
    fetch(spec)
}

/// Fetch the faces of a finish in the background (when fetching is allowed), so the first
/// cut after a look is picked does not wait for them. Failures are left for the cut to report.
pub fn prefetch(specs: &[Spec]) {
    if !fetch_allowed() {
        return;
    }
    let missing: Vec<Spec> = specs.iter().filter(|s| local(s).is_none()).cloned().collect();
    if missing.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        for s in missing {
            let _ = fetch(&s);
        }
    });
}

/// Fetch the faces now (the CLI waits for them); returns what could not be had.
pub fn fetch_now(specs: &[Spec]) -> Vec<String> {
    specs.iter().filter_map(|s| resolve(s).err().map(|e| format!("{}: {}", s.label(), e.msg))).collect()
}

/// Download one family at one weight. Google Fonts' CSS API answers a client that does not
/// ask for WOFF2 with one TrueType file per weight (variable families come pinned to it).
fn fetch(spec: &Spec) -> Result<PathBuf> {
    let dir = cache_dir().ok_or_else(|| Error::user("no folder to keep fonts in (set SHOTPILOT_FONTS)"))?;
    std::fs::create_dir_all(&dir)?;
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(120)).build();
    let css_url = format!("https://fonts.googleapis.com/css2?family={}:wght@{}", spec.family.replace(' ', "+"), spec.weight);
    let css = agent
        .get(&css_url)
        .call()
        .map_err(|e| Error::user(format!("could not reach Google Fonts for {}: {e}", spec.label())))?
        .into_string()
        .map_err(|e| Error::user(format!("Google Fonts: {e}")))?;
    let Some(url) = ttf_url(&css) else {
        crate::bail!("Google Fonts sent no TrueType file for {}", spec.label());
    };
    let resp = agent.get(&url).call().map_err(|e| Error::user(format!("could not download {}: {e}", spec.label())))?;
    let mut bytes = Vec::new();
    resp.into_reader().take(64 << 20).read_to_end(&mut bytes)?;
    if !is_font(&bytes) {
        crate::bail!("the file for {} is not a font", spec.label());
    }
    let dst = dir.join(spec.file_name());
    let tmp = dir.join(format!("{}.part", spec.file_name()));
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &dst)?;
    let readme = dir.join("README.txt");
    if !readme.exists() {
        let _ = std::fs::write(
            &readme,
            "Fonts fetched by shotpilot from Google Fonts (fonts.google.com). Each is under the SIL Open Font \
             License 1.1 (Kosugi, Kosugi Maru: Apache 2.0); see the family's page for its licence text.\n",
        );
    }
    Ok(dst)
}

/// The first `url(...)` of a TrueType source in a Google Fonts stylesheet.
pub fn ttf_url(css: &str) -> Option<String> {
    css.split("url(").skip(1).filter_map(|s| s.split(')').next()).map(|u| u.trim_matches(['\'', '"']).to_string()).find(|u| u.ends_with(".ttf"))
}

fn is_font(b: &[u8]) -> bool {
    b.len() > 12 && matches!(&b[..4], [0, 1, 0, 0] | b"OTTO" | b"true")
}

/// Whether a font file has a glyph for every visible character of `text`. Single fonts only
/// (TrueType / OpenType); a collection (.ttc) or an unreadable file answers `None`.
pub fn covers(path: &Path, text: &str) -> Option<bool> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<PathBuf, Option<Cmap>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut cache = cache.lock().ok()?;
    let cmap = cache.entry(path.to_path_buf()).or_insert_with(|| std::fs::read(path).ok().and_then(|b| Cmap::read(&b)));
    let cmap = cmap.as_ref()?;
    Some(text.chars().filter(|c| !c.is_whitespace() && !c.is_control()).all(|c| cmap.has(c as u32)))
}

/// The characters of `text` a font has no glyph for (for notes), in order, without repeats.
pub fn missing(path: &Path, text: &str) -> String {
    let Some(cmap) = std::fs::read(path).ok().and_then(|b| Cmap::read(&b)) else { return String::new() };
    let mut out = String::new();
    for c in text.chars().filter(|c| !c.is_whitespace() && !c.is_control()) {
        if !cmap.has(c as u32) && !out.contains(c) {
            out.push(c);
        }
    }
    out
}

/// A font's character map (format 4 or 12), enough to ask "is this character there".
#[derive(Debug, Clone)]
pub struct Cmap {
    /// Ranges (first, last) of characters that map to a glyph other than 0.
    ranges: Vec<(u32, u32)>,
}

impl Cmap {
    pub fn has(&self, c: u32) -> bool {
        let i = self.ranges.partition_point(|r| r.1 < c);
        self.ranges.get(i).is_some_and(|r| r.0 <= c)
    }

    /// Reads the best Unicode subtable of a TrueType / OpenType file.
    pub fn read(b: &[u8]) -> Option<Cmap> {
        if !is_font(b) {
            return None;
        }
        let n = u16_at(b, 4)? as usize;
        let cmap = (0..n).map(|i| 12 + i * 16).find(|&r| b.get(r..r + 4) == Some(b"cmap")).and_then(|r| u32_at(b, r + 8))? as usize;
        let subtables = u16_at(b, cmap + 2)? as usize;
        let mut best: Option<(u8, usize)> = None;
        for i in 0..subtables {
            let rec = cmap + 4 + i * 8;
            let (platform, encoding) = (u16_at(b, rec)?, u16_at(b, rec + 2)?);
            let off = cmap + u32_at(b, rec + 4)? as usize;
            let rank = match (platform, encoding, u16_at(b, off)?) {
                (3, 10, 12) | (0, 4, 12) | (0, 6, 12) => 3,
                (3, 1, 4) | (0, 3, 4) | (0, 1, 4) | (0, 0, 4) => 2,
                _ => 0,
            };
            let better = match best {
                Some((r, _)) => rank > r,
                None => rank > 0,
            };
            if better {
                best = Some((rank, off));
            }
        }
        let (_, off) = best?;
        let mut ranges = match u16_at(b, off)? {
            12 => format12(b, off)?,
            4 => format4(b, off)?,
            _ => return None,
        };
        ranges.sort_unstable();
        let mut merged: Vec<(u32, u32)> = Vec::new();
        for r in ranges {
            match merged.last_mut() {
                Some(last) if r.0 <= last.1.saturating_add(1) => last.1 = last.1.max(r.1),
                _ => merged.push(r),
            }
        }
        Some(Cmap { ranges: merged })
    }
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    b.get(o..o + 2).map(|x| u16::from_be_bytes([x[0], x[1]]))
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    b.get(o..o + 4).map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]]))
}

fn format12(b: &[u8], off: usize) -> Option<Vec<(u32, u32)>> {
    let groups = u32_at(b, off + 12)? as usize;
    let mut out = Vec::with_capacity(groups);
    for g in 0..groups {
        let o = off + 16 + g * 12;
        let (start, end, glyph) = (u32_at(b, o)?, u32_at(b, o + 4)?, u32_at(b, o + 8)?);
        // glyph 0 is .notdef: a group starting there only counts from its second character
        let start = if glyph == 0 { start.saturating_add(1) } else { start };
        if start <= end {
            out.push((start, end));
        }
    }
    Some(out)
}

fn format4(b: &[u8], off: usize) -> Option<Vec<(u32, u32)>> {
    let segs = u16_at(b, off + 6)? as usize / 2;
    let ends = off + 14;
    let starts = ends + segs * 2 + 2;
    let deltas = starts + segs * 2;
    let offsets = deltas + segs * 2;
    let mut out = Vec::new();
    for i in 0..segs {
        let (end, start) = (u16_at(b, ends + i * 2)? as u32, u16_at(b, starts + i * 2)? as u32);
        let (delta, range_off) = (u16_at(b, deltas + i * 2)?, u16_at(b, offsets + i * 2)? as usize);
        if start > end || start == 0xFFFF {
            continue;
        }
        if range_off == 0 {
            // every character maps; only the one whose glyph wraps to 0 is missing
            let zero = (0x10000 - delta as u32) & 0xFFFF;
            if (start..=end).contains(&zero) {
                if zero > start {
                    out.push((start, zero - 1));
                }
                if zero < end {
                    out.push((zero + 1, end));
                }
            } else {
                out.push((start, end));
            }
            continue;
        }
        let mut run: Option<(u32, u32)> = None;
        for c in start..=end {
            let at = offsets + i * 2 + range_off + (c - start) as usize * 2;
            let has = u16_at(b, at).is_some_and(|g| g != 0);
            match (has, run.as_mut()) {
                (true, Some(r)) if r.1 + 1 == c => r.1 = c,
                (true, _) => {
                    if let Some(r) = run.take() {
                        out.push(r);
                    }
                    run = Some((c, c));
                }
                (false, _) => {
                    if let Some(r) = run.take() {
                        out.push(r);
                    }
                }
            }
        }
        if let Some(r) = run {
            out.push(r);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal TrueType header with one cmap table holding `sub` (platform 3, encoding).
    pub(crate) fn font_with(sub: &[u8], encoding: u16) -> Vec<u8> {
        let mut b = vec![0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
        b.extend_from_slice(b"cmap");
        b.extend_from_slice(&[0, 0, 0, 0]);
        b.extend_from_slice(&28u32.to_be_bytes());
        b.extend_from_slice(&((12 + sub.len()) as u32).to_be_bytes());
        // cmap header: version 0, 1 subtable (3, encoding) at offset 12
        b.extend_from_slice(&[0, 0, 0, 1, 0, 3]);
        b.extend_from_slice(&encoding.to_be_bytes());
        b.extend_from_slice(&12u32.to_be_bytes());
        b.extend_from_slice(sub);
        b
    }

    fn format4_sub(segs: &[(u16, u16, i16)]) -> Vec<u8> {
        // (start, end, delta) with idRangeOffset 0, plus the closing 0xFFFF segment
        let mut all = segs.to_vec();
        all.push((0xFFFF, 0xFFFF, 1));
        let n = all.len() as u16;
        let mut s = vec![0, 4, 0, 0, 0, 0];
        s.extend_from_slice(&(n * 2).to_be_bytes());
        s.extend_from_slice(&[0; 6]);
        for (_, e, _) in &all {
            s.extend_from_slice(&e.to_be_bytes());
        }
        s.extend_from_slice(&[0, 0]);
        for (st, _, _) in &all {
            s.extend_from_slice(&st.to_be_bytes());
        }
        for (_, _, d) in &all {
            s.extend_from_slice(&d.to_be_bytes());
        }
        for _ in &all {
            s.extend_from_slice(&[0, 0]);
        }
        s
    }

    #[test]
    fn reads_which_characters_a_font_has() {
        // ASCII letters and hiragana, no kanji
        let sub = format4_sub(&[(0x41, 0x5A, -0x40), (0x3041, 0x3093, -0x3000)]);
        let c = Cmap::read(&font_with(&sub, 1)).unwrap();
        assert!(c.has('A' as u32) && c.has('Z' as u32) && c.has('あ' as u32) && c.has('ん' as u32));
        assert!(!c.has('a' as u32) && !c.has('七' as u32));

        let mut g12 = vec![0, 12, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 2];
        for (s, e, g) in [(0x20u32, 0x7E, 1), (0x4E00, 0x9FFF, 100)] {
            for x in [s, e, g] {
                g12.extend_from_slice(&x.to_be_bytes());
            }
        }
        let c = Cmap::read(&font_with(&g12, 10)).unwrap();
        assert!(c.has('七' as u32) && c.has('a' as u32) && !c.has('あ' as u32));
        assert!(Cmap::read(b"not a font at all").is_none());
    }

    #[test]
    fn specs_name_a_catalogue_family_and_one_of_its_weights() {
        assert_eq!(Spec::parse("Shippori Mincho:800").unwrap(), Spec { family: "Shippori Mincho".into(), weight: 800 });
        assert_eq!(Spec::parse("shippori mincho:750").unwrap().weight, 700); // nearest of 400..800 (ties go low)
        assert_eq!(Spec::parse("Dela Gothic One").unwrap().weight, 400);
        assert_eq!(Spec::parse("Marcellus:900").unwrap().weight, 400); // the only weight it has
        assert!(Spec::parse("Comic Sans MS").unwrap_err().contains("catalog"));
        assert_eq!(Spec::parse("Zen Kaku Gothic New:700").unwrap().file_name(), "ZenKakuGothicNew-700.ttf");
    }

    #[test]
    fn the_catalogue_knows_coverage_and_the_overused_faces() {
        assert_eq!(family("Monomaniac One").unwrap().kanji, "none");
        assert_eq!(family("Shippori Mincho").unwrap().kanji, "full");
        assert_eq!(catalog().families.iter().filter(|f| f.script == "ja").count(), 68);
        assert!(overused("Cinzel").unwrap().instead.contains(&"Marcellus".to_string()));
        assert!(overused("Marcellus").is_none());
    }

    #[test]
    fn finds_the_truetype_file_in_a_stylesheet() {
        let css = "@font-face {\n  src: url(https://fonts.gstatic.com/s/x/v1/abc.ttf) format('truetype');\n}";
        assert_eq!(ttf_url(css).as_deref(), Some("https://fonts.gstatic.com/s/x/v1/abc.ttf"));
        assert_eq!(ttf_url("src: url(https://x/y.woff2)"), None);
    }

    /// Guards D-52: fetching is off unless a binary calls `allow_fetch`, so a catalogue face
    /// that is not cached is reported as unavailable instead of being downloaded. The spec
    /// names a family that cannot be in any cache, so no env var needs to change.
    #[test]
    fn an_uncached_face_is_unavailable_without_allow_fetch() {
        assert!(!fetch_allowed(), "no test may turn fetching on");
        let spec = Spec { family: "Zz Not Cached Anywhere".into(), weight: 400 };
        let err = resolve(&spec).expect_err("must not resolve without a cache or a fetch");
        assert!(err.msg.contains("not downloaded yet"), "should refuse before any network call: {}", err.msg);
        let report = fetch_now(&[spec]);
        assert_eq!(report.len(), 1);
        assert!(report[0].contains("not downloaded yet"), "{report:?}");
        // prefetch is a no-op while fetching is off
        prefetch(&[Spec { family: "Zz Not Cached Anywhere".into(), weight: 400 }]);
    }

    /// Guards D-52: only families of the catalogue can be named, so nothing else is fetched.
    #[test]
    fn a_family_outside_the_catalogue_is_refused() {
        for name in ["Comic Sans MS", "Arial", "Totally Made Up Face:700", ""] {
            assert!(Spec::parse(name).is_err(), "{name} must be refused");
        }
        assert!(family("Totally Made Up Face").is_none());
        let first = catalog().families.first().expect("catalogue has families").family.clone();
        assert!(Spec::parse(&first).is_ok());
    }
}
