//! Comment-preserving YAML I/O.
//!
//! Humans edit shots.yaml by hand; `set`, `pull`, `revise`, `order` and `scan` edit it
//! by machine. Machine edits are line splices: only the lines of the shots (or anchor
//! entries) being changed are rewritten, so every other comment, blank line and key
//! order survives byte for byte. Every splice is re-parsed and checked against the
//! intended value before it may be written.

use std::path::Path;

use serde_json::{Map, Number, Value};
use yaml_rust2::{Yaml, YamlLoader};

use crate::bail;
use crate::error::{Error, Result};

// --------------------------------------------------------------------------- parse

pub fn parse_str(text: &str, origin: &str) -> Result<Value> {
    let docs = YamlLoader::load_from_str(text)
        .map_err(|e| Error::user(format!("{origin}: invalid YAML: {e}")))?;
    Ok(docs.into_iter().next().map(to_json).unwrap_or(Value::Null))
}

pub fn load(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path)?;
    parse_str(&text, &path.display().to_string())
}

fn to_json(y: Yaml) -> Value {
    match y {
        Yaml::Real(s) => match s.parse::<f64>().ok().and_then(Number::from_f64) {
            Some(n) => Value::Number(n),
            None => Value::String(s),
        },
        Yaml::Integer(i) => Value::Number(i.into()),
        Yaml::String(s) => Value::String(s),
        Yaml::Boolean(b) => Value::Bool(b),
        Yaml::Array(a) => Value::Array(a.into_iter().map(to_json).collect()),
        Yaml::Hash(h) => {
            let mut m = Map::new();
            for (k, v) in h {
                let key = match k {
                    Yaml::String(s) | Yaml::Real(s) => s,
                    Yaml::Integer(i) => i.to_string(),
                    Yaml::Boolean(b) => b.to_string(),
                    Yaml::Null => "null".into(),
                    other => format!("{other:?}"),
                };
                m.insert(key, to_json(v));
            }
            Value::Object(m)
        }
        Yaml::Alias(_) | Yaml::Null | Yaml::BadValue => Value::Null,
    }
}

// --------------------------------------------------------------------------- emit

pub fn fmt_float(f: f64) -> String {
    if f.is_nan() {
        return ".nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { ".inf".into() } else { "-.inf".into() };
    }
    let s = format!("{f}");
    if s.contains(['.', 'e', 'E']) {
        s
    } else {
        format!("{s}.0")
    }
}

fn plain_ok(s: &str) -> bool {
    if s.is_empty() || s.trim() != s || s.chars().any(|c| c.is_control()) {
        return false;
    }
    if s.starts_with(|c: char| "-?:,[]{}#&*!|>'\"%@`".contains(c)) {
        return false;
    }
    if s.contains(": ") || s.contains(" #") || s.ends_with(':') {
        return false;
    }
    // Anything that would read back as a number, bool or null must be quoted.
    matches!(YamlLoader::load_from_str(&format!("k: {s}")).ok().as_deref(),
        Some([Yaml::Hash(h)]) if h.values().next() == Some(&Yaml::String(s.to_string())))
}

pub fn emit_str(s: &str) -> String {
    if plain_ok(s) {
        s.to_string()
    } else {
        serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\""))
    }
}

pub fn emit_scalar(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => match (n.as_i64(), n.as_u64()) {
            (Some(i), _) => i.to_string(),
            (_, Some(u)) => u.to_string(),
            _ => fmt_float(n.as_f64().unwrap_or(0.0)),
        },
        Value::String(s) => emit_str(s),
        Value::Array(_) | Value::Object(_) => emit_flow(v),
    }
}

/// Flow style (`[1, 2]`, `{a: 1}`) for small inline values.
pub fn emit_flow(v: &Value) -> String {
    match v {
        Value::Array(a) => format!("[{}]", a.iter().map(emit_flow).collect::<Vec<_>>().join(", ")),
        Value::Object(m) => {
            let parts: Vec<String> = m.iter().map(|(k, x)| format!("{}: {}", emit_str(k), emit_flow(x))).collect();
            format!("{{{}}}", parts.join(", "))
        }
        _ => emit_scalar(v),
    }
}

fn short_numeric(a: &[Value]) -> bool {
    a.len() <= 4 && a.iter().all(Value::is_number)
}

fn pad(n: usize) -> String {
    " ".repeat(n)
}

/// One `key: value` entry of a block mapping at `indent`.
pub fn emit_entry(key: &str, val: &Value, indent: usize) -> Vec<String> {
    let head = format!("{}{}:", pad(indent), emit_str(key));
    match val {
        Value::Object(m) if !m.is_empty() => {
            let mut out = vec![head];
            out.extend(emit_map(m, indent + 2));
            out
        }
        Value::Array(a) if !a.is_empty() && !short_numeric(a) => {
            let mut out = vec![head];
            for item in a {
                out.extend(emit_item(item, indent + 2));
            }
            out
        }
        _ => vec![format!("{head} {}", emit_scalar(val))],
    }
}

pub fn emit_map(m: &Map<String, Value>, indent: usize) -> Vec<String> {
    m.iter().flat_map(|(k, v)| emit_entry(k, v, indent)).collect()
}

/// A sequence item: dash at `indent`, content at `indent + 2`.
pub fn emit_item(v: &Value, indent: usize) -> Vec<String> {
    match v {
        Value::Object(m) if !m.is_empty() => {
            let mut lines = emit_map(m, indent + 2);
            lines[0] = format!("{}- {}", pad(indent), &lines[0][indent + 2..]);
            lines
        }
        _ => vec![format!("{}- {}", pad(indent), emit_flow(v))],
    }
}

/// Serialize a value as a standalone YAML document (block style).
pub fn dumps(v: &Value) -> String {
    let lines = match v {
        Value::Object(m) if !m.is_empty() => emit_map(m, 0),
        Value::Array(a) if !a.is_empty() => a.iter().flat_map(|x| emit_item(x, 0)).collect(),
        _ => vec![emit_flow(v)],
    };
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

// --------------------------------------------------------------------------- line helpers

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

fn is_comment(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

fn is_trivia(line: &str) -> bool {
    is_blank(line) || is_comment(line)
}

fn is_item(line: &str, indent: usize) -> bool {
    indent_of(line) == indent && {
        let t = line.trim_start();
        t == "-" || t.starts_with("- ")
    }
}

/// Split `content  # comment` outside quotes. Returns (content, comment-with-hash).
fn split_eol_comment(s: &str) -> (&str, Option<&str>) {
    let (mut sq, mut dq) = (false, false);
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'\'' if !dq => sq = !sq,
            b'"' if !sq => dq = !dq,
            b'#' if !sq && !dq && (i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') => {
                return (s[..i].trim_end(), Some(&s[i..]));
            }
            _ => {}
        }
    }
    (s.trim_end(), None)
}

// --------------------------------------------------------------------------- document

/// One item replacement: (index, new value, end-of-line comments by top-level key).
pub type ItemEdit = (usize, Value, Vec<(String, String)>);

/// A YAML file held as lines, edited by splicing.
pub struct Doc {
    lines: Vec<String>,
    crlf: bool,
    origin: String,
}

struct Block {
    key_line: usize,
    end: usize, // exclusive; trailing column-0 comments/blank lines belong to what follows
    inline: bool,
}

struct ItemSpan {
    pre: Vec<String>,
    body: Vec<String>,
}

struct SeqLayout {
    indent: usize,
    items: Vec<ItemSpan>,
    tail: Vec<String>,
}

impl Doc {
    pub fn from_text(text: &str, origin: &str) -> Doc {
        let crlf = text.contains("\r\n");
        let mut lines: Vec<String> = text.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
        if lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        Doc { lines, crlf, origin: origin.to_string() }
    }

    pub fn load(path: &Path) -> Result<Doc> {
        Ok(Doc::from_text(&std::fs::read_to_string(path)?, &path.display().to_string()))
    }

    pub fn text(&self) -> String {
        let nl = if self.crlf { "\r\n" } else { "\n" };
        let mut s = self.lines.join(nl);
        s.push_str(nl);
        s
    }

    pub fn value(&self) -> Result<Value> {
        parse_str(&self.text(), &self.origin)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let tmp = path.with_extension("yaml.tmp");
        std::fs::write(&tmp, self.text())?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    fn find_block(&self, key: &str) -> Option<Block> {
        let prefix = format!("{key}:");
        let key_line = self.lines.iter().position(|l| {
            l.strip_prefix(&prefix).is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t']))
        })?;
        let rest = &self.lines[key_line][prefix.len()..];
        let inline = !split_eol_comment(rest).0.trim().is_empty();
        let mut end = key_line + 1;
        while end < self.lines.len() {
            let l = &self.lines[end];
            let t = l.trim_start();
            if is_blank(l) || l.starts_with([' ', '\t', '#']) || t == "-" || l.starts_with("- ") {
                end += 1;
            } else {
                break;
            }
        }
        while end > key_line + 1 && {
            let l = &self.lines[end - 1];
            is_blank(l) || l.starts_with('#')
        } {
            end -= 1;
        }
        Some(Block { key_line, end, inline })
    }

    fn seq_layout(&self, b: &Block) -> Option<SeqLayout> {
        if b.inline {
            return None;
        }
        let body = &self.lines[b.key_line + 1..b.end];
        let first = body.iter().position(|l| !is_trivia(l))?;
        if !is_item(&body[first], indent_of(&body[first])) {
            return None;
        }
        let indent = indent_of(&body[first]);
        let starts: Vec<usize> = (0..body.len()).filter(|&i| is_item(&body[i], indent)).collect();
        // anything shallower than the dashes means this is not a plain block sequence
        if body.iter().any(|l| !is_trivia(l) && indent_of(l) < indent) {
            return None;
        }
        let mut items = Vec::new();
        let mut pre: Vec<String> = body[..starts[0]].to_vec();
        let mut tail = Vec::new();
        for (k, &s) in starts.iter().enumerate() {
            let next = starts.get(k + 1).copied().unwrap_or(body.len());
            let mut stop = next;
            while stop > s + 1 && {
                let l = &body[stop - 1];
                is_blank(l) || (is_comment(l) && indent_of(l) <= indent)
            } {
                stop -= 1;
            }
            items.push(ItemSpan { pre: std::mem::take(&mut pre), body: body[s..stop].to_vec() });
            let trivia = body[stop..next].to_vec();
            if k + 1 < starts.len() {
                pre = trivia;
            } else {
                tail = trivia;
            }
        }
        Some(SeqLayout { indent, items, tail })
    }

    fn write_seq(&mut self, b: &Block, layout: &SeqLayout) {
        let mut out = Vec::new();
        for it in &layout.items {
            out.extend(it.pre.iter().cloned());
            out.extend(it.body.iter().cloned());
        }
        out.extend(layout.tail.iter().cloned());
        self.lines.splice(b.key_line + 1..b.end, out);
    }

    /// Rewrite a whole top-level sequence from values (used when the layout is not
    /// a plain block sequence, e.g. `shots: []`). Comments inside it are lost.
    fn rewrite_seq(&mut self, key: &str, items: &[Value]) {
        let mut out = Vec::new();
        for (i, it) in items.iter().enumerate() {
            if i > 0 && items.len() > 1 && it.is_object() {
                out.push(String::new());
            }
            out.extend(emit_item(it, 2));
        }
        match self.find_block(key) {
            Some(b) => {
                let comment = split_eol_comment(&self.lines[b.key_line][key.len() + 1..]).1.map(str::to_string);
                let head = if items.is_empty() { format!("{key}: []") } else { format!("{key}:") };
                let head = match comment {
                    Some(c) => format!("{head}  {c}"),
                    None => head,
                };
                let mut lines = vec![head];
                lines.extend(out);
                self.lines.splice(b.key_line..b.end, lines);
            }
            None => {
                if self.lines.last().is_some_and(|l| !is_blank(l)) {
                    self.lines.push(String::new());
                }
                self.lines.push(if items.is_empty() { format!("{key}: []") } else { format!("{key}:") });
                self.lines.extend(out);
            }
        }
    }

    fn verify(&self, expected: &Value, what: &str) -> Result<()> {
        let got = self.value()?;
        if &got != expected {
            bail!("internal error: the {what} edit to {} did not round-trip; nothing was written", self.origin);
        }
        Ok(())
    }

    fn seq_values(&self, key: &str) -> Result<(Value, Vec<Value>)> {
        let doc = self.value()?;
        let items = match doc.get(key) {
            Some(Value::Array(a)) => a.clone(),
            None | Some(Value::Null) => Vec::new(),
            Some(_) => bail!("{}: `{key}` must be a list", self.origin),
        };
        Ok((doc, items))
    }

    /// Replace sequence items by index (`key_comments`: end-of-line comments to put on
    /// top-level keys of that item, e.g. a note on `camera:` after a pull).
    pub fn replace_items(&mut self, key: &str, edits: &[ItemEdit]) -> Result<()> {
        let (mut expected, mut items) = self.seq_values(key)?;
        for (i, v, _) in edits {
            if *i >= items.len() {
                bail!("internal error: no item {i} in `{key}`");
            }
            items[*i] = v.clone();
        }
        expected[key] = Value::Array(items.clone());
        let layout = self.find_block(key).and_then(|b| self.seq_layout(&b).map(|l| (b, l)));
        match layout {
            Some((b, mut l)) if l.items.len() == items.len() => {
                for (i, v, comments) in edits {
                    let mut body = emit_item(v, l.indent);
                    for (k, c) in comments {
                        add_key_comment(&mut body, l.indent + 2, k, c);
                    }
                    l.items[*i].body = body;
                }
                self.write_seq(&b, &l);
            }
            _ => self.rewrite_seq(key, &items),
        }
        self.verify(&expected, "item")
    }

    /// Reorder sequence items: `perm[k]` is the old index that goes to position k.
    /// Each item moves with the comments directly above it; blank-line spacing stays put.
    pub fn reorder_items(&mut self, key: &str, perm: &[usize]) -> Result<()> {
        let (mut expected, items) = self.seq_values(key)?;
        let mut sorted = perm.to_vec();
        sorted.sort_unstable();
        if sorted != (0..items.len()).collect::<Vec<_>>() {
            bail!("internal error: not a permutation of `{key}`");
        }
        let new_items: Vec<Value> = perm.iter().map(|&i| items[i].clone()).collect();
        expected[key] = Value::Array(new_items.clone());
        let layout = self.find_block(key).and_then(|b| self.seq_layout(&b).map(|l| (b, l)));
        match layout {
            Some((b, l)) if l.items.len() == items.len() => {
                let blanks: Vec<Vec<String>> =
                    l.items.iter().map(|it| it.pre.iter().filter(|x| is_blank(x)).cloned().collect()).collect();
                let comments: Vec<Vec<String>> =
                    l.items.iter().map(|it| it.pre.iter().filter(|x| !is_blank(x)).cloned().collect()).collect();
                let moved = SeqLayout {
                    indent: l.indent,
                    items: perm
                        .iter()
                        .enumerate()
                        .map(|(k, &i)| ItemSpan {
                            pre: blanks[k].iter().chain(comments[i].iter()).cloned().collect(),
                            body: l.items[i].body.clone(),
                        })
                        .collect(),
                    tail: l.tail.clone(),
                };
                self.write_seq(&b, &moved);
            }
            _ => self.rewrite_seq(key, &new_items),
        }
        self.verify(&expected, "reorder")
    }

    /// Replace the whole sequence (e.g. a fresh plan). Comments above the key survive.
    pub fn set_items(&mut self, key: &str, items: &[Value]) -> Result<()> {
        let (mut expected, _) = self.seq_values(key)?;
        if !expected.is_object() {
            expected = Value::Object(Map::new());
        }
        expected[key] = Value::Array(items.to_vec());
        self.rewrite_seq(key, items);
        self.verify(&expected, "list")
    }

    /// Add or update entries of a top-level mapping (e.g. anchors), keeping the
    /// comments on untouched entries and on updated single-line entries.
    pub fn upsert_entries(&mut self, key: &str, entries: &[(String, Value)]) -> Result<()> {
        let mut expected = self.value()?;
        if !expected.is_object() {
            expected = Value::Object(Map::new());
        }
        let mut merged = match expected.get(key) {
            Some(Value::Object(m)) => m.clone(),
            None | Some(Value::Null) => Map::new(),
            Some(_) => bail!("{}: `{key}` must be a mapping", self.origin),
        };
        for (k, v) in entries {
            merged.insert(k.clone(), v.clone());
        }
        expected[key] = Value::Object(merged.clone());

        let block = self.find_block(key);
        let child_indent = block.as_ref().filter(|b| !b.inline).and_then(|b| {
            self.lines[b.key_line + 1..b.end].iter().find(|l| !is_trivia(l)).map(|l| indent_of(l))
        });
        match (block, child_indent) {
            (Some(b), Some(ci)) if ci > 0 => {
                let mut end = b.end;
                for (k, v) in entries {
                    let head = format!("{}:", emit_str(k));
                    let found = (b.key_line + 1..end).find(|&i| {
                        let l = &self.lines[i];
                        indent_of(l) == ci && l.trim_start().strip_prefix(&head).is_some_and(|r| r.is_empty() || r.starts_with(' '))
                    });
                    let mut new = emit_entry(k, v, ci);
                    match found {
                        Some(i) => {
                            let mut stop = i + 1;
                            while stop < end && (is_blank(&self.lines[stop]) || indent_of(&self.lines[stop]) > ci) {
                                stop += 1;
                            }
                            while stop > i + 1 && is_trivia(&self.lines[stop - 1]) {
                                stop -= 1;
                            }
                            if new.len() == 1 && stop == i + 1 {
                                if let (_, Some(c)) = split_eol_comment(&self.lines[i]) {
                                    new[0] = format!("{}  {c}", new[0]);
                                }
                            }
                            let n = new.len();
                            self.lines.splice(i..stop, new);
                            end = end + n - (stop - i);
                        }
                        None => {
                            let mut at = end;
                            while at > b.key_line + 1 && is_trivia(&self.lines[at - 1]) {
                                at -= 1;
                            }
                            let n = new.len();
                            self.lines.splice(at..at, new);
                            end += n;
                        }
                    }
                }
            }
            (block, _) => {
                let mut lines = vec![if merged.is_empty() { format!("{key}: {{}}") } else { format!("{key}:") }];
                lines.extend(emit_map(&merged, 2));
                match block {
                    Some(b) => {
                        self.lines.splice(b.key_line..b.end, lines);
                    }
                    None => {
                        if self.lines.last().is_some_and(|l| !is_blank(l)) {
                            self.lines.push(String::new());
                        }
                        self.lines.extend(lines);
                    }
                }
            }
        }
        self.verify(&expected, "mapping")
    }
}

fn add_key_comment(body: &mut [String], indent: usize, key: &str, comment: &str) {
    let head = format!("{}:", emit_str(key));
    for line in body.iter_mut() {
        let content = line.trim_start().trim_start_matches("- ");
        let at = line.len() - content.len();
        if at == indent && content.starts_with(&head) {
            let one_line = comment.replace(['\n', '\r'], " ");
            line.push_str(&format!("  # {one_line}"));
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SRC: &str = "# top comment\nversion: 1\nanchors:\n  well: [0, 0, 80]   # the well\n  gate: [1, 2, 3]\n\n# shots below\nshots:\n  - id: a\n    duration: 5   # five\n\n  # about b\n  - id: b\n    duration: 2\n\n  - id: c\n    duration: 1\n# trailing\nother: x\n";

    #[test]
    fn scalars_roundtrip() {
        for s in ["yes", "123", "1.5", "null", "a: b", "#x", "", " pad", "日本語", "-x", "true", "~"] {
            let v = json!({ "k": s });
            assert_eq!(parse_str(&dumps(&v), "t").unwrap(), v, "{s}");
        }
        let v = json!({"a": 5.0, "b": 5, "c": [1, 2.5], "d": {"e": []}, "f": [{"x": 1}, {"y": "z"}]});
        assert_eq!(parse_str(&dumps(&v), "t").unwrap(), v);
    }

    #[test]
    fn replace_keeps_other_comments() {
        let mut d = Doc::from_text(SRC, "t");
        d.replace_items("shots", &[(1, json!({"id": "b", "duration": 3}), vec![])]).unwrap();
        let t = d.text();
        assert!(t.contains("duration: 5   # five") && t.contains("  # about b\n  - id: b\n    duration: 3\n"));
        assert!(t.contains("# trailing\nother: x"));
    }

    #[test]
    fn reorder_moves_comments_with_items() {
        let mut d = Doc::from_text(SRC, "t");
        d.reorder_items("shots", &[2, 0, 1]).unwrap();
        let v = d.value().unwrap();
        let ids: Vec<&str> = v["shots"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["c", "a", "b"]);
        assert!(d.text().contains("  # about b\n  - id: b"));
        assert!(d.text().contains("# top comment"));
    }

    #[test]
    fn upsert_keeps_comments() {
        let mut d = Doc::from_text(SRC, "t");
        d.upsert_entries("anchors", &[("well".into(), json!([1.0, 0.0, 80.0])), ("door".into(), json!([5.0, 5.0, 5.0]))])
            .unwrap();
        let t = d.text();
        assert!(t.contains("  well: [1.0, 0.0, 80.0]  # the well"));
        assert!(t.contains("  door: [5.0, 5.0, 5.0]\n\n# shots below"));
    }

    #[test]
    fn empty_flow_list_is_rewritten() {
        let mut d = Doc::from_text("version: 1\nshots: []  # none yet\n", "t");
        d.set_items("shots", &[json!({"id": "a"}), json!({"id": "b"})]).unwrap();
        assert_eq!(d.value().unwrap()["shots"][1]["id"], "b");
        assert!(d.text().starts_with("version: 1\nshots:  # none yet\n"));
    }

    #[test]
    fn key_comment_is_added() {
        let mut d = Doc::from_text(SRC, "t");
        let node = json!({"id": "a", "camera": {"location": [0, 0, 0]}, "duration": 5});
        d.replace_items("shots", &[(0, node, vec![("camera".into(), "pulled".into())])]).unwrap();
        assert!(d.text().contains("    camera:  # pulled\n"));
    }
}
