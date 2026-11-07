//! The invariants in AGENTS.md each name the tests that keep them (D-65).
//!
//! An invariant written only in prose broke once (D-64: a path Claude was called on got none
//! of the spec, though AGENTS.md said every path must). So every item of "守ること（不変条件）"
//! ends with `守るテスト:` and the names of #[test] functions that fail when it breaks, and
//! this test holds the list to the code: an item without names, or a name no test has (a
//! renamed or deleted test), fails here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Every `fn` that has `#[test]` above it, in the given folders (recursively).
fn test_names(dirs: &[PathBuf]) -> BTreeSet<String> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    for d in dirs {
        walk(d, &mut files);
    }
    let mut names = BTreeSet::new();
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        for (i, l) in lines.iter().enumerate() {
            if !l.trim_start().starts_with("#[test]") {
                continue;
            }
            // the fn after the attribute (other attributes such as #[ignore] may sit between)
            for next in lines.iter().skip(i + 1).take(6) {
                let t = next.trim_start().trim_start_matches("pub ");
                if let Some(rest) = t.strip_prefix("fn ") {
                    names.insert(rest.split(['(', '<']).next().unwrap_or("").to_string());
                    break;
                }
            }
        }
    }
    names
}

/// The items of "守ること（不変条件）", one string per item.
fn invariants(agents: &str) -> Vec<String> {
    let text = agents.replace("\r\n", "\n");
    let section = text.split("## 守ること（不変条件）\n").nth(1).expect("AGENTS.md has the invariants section");
    let section = section.split("\n## ").next().unwrap_or("");
    section.lines().filter(|l| l.starts_with("- ")).map(str::to_string).collect()
}

/// The backticked names after `守るテスト:` up to the end of that sentence.
fn named_tests(item: &str) -> Option<Vec<String>> {
    let after = item.split("守るテスト:").nth(1)?;
    let sentence = after.split('。').next().unwrap_or("");
    Some(sentence.split('`').skip(1).step_by(2).map(str::to_string).collect())
}

#[test]
fn every_invariant_names_the_tests_that_keep_it() {
    let agents = std::fs::read_to_string(repo().join("AGENTS.md")).unwrap();
    let items = invariants(&agents);
    assert!(items.len() >= 30, "found only {} invariants; did the section heading change?", items.len());
    let crates = repo().join("crates");
    let known = test_names(&[crates.join("shotpilot").join("src"), crates.join("shotpilot").join("tests"),
                             crates.join("shotpilot-app").join("src")]);
    let mut problems = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let head: String = item.chars().take(40).collect();
        match named_tests(item) {
            None => problems.push(format!("[{}] {head}…: no `守るテスト:`", i + 1)),
            Some(names) if names.is_empty() => problems.push(format!("[{}] {head}…: `守るテスト:` names no test", i + 1)),
            Some(names) => {
                for n in names.iter().filter(|n| !known.contains(*n)) {
                    problems.push(format!("[{}] {head}…: no #[test] fn `{n}`", i + 1));
                }
            }
        }
    }
    assert!(problems.is_empty(), "AGENTS.md invariants and their tests disagree:\n{}", problems.join("\n"));
}

#[test]
fn the_ledger_reader_finds_names_and_misses() {
    // the parser itself: names come from the 守るテスト sentence only
    let item = "- 何か。`not_a_test` は説明。 守るテスト: `a_test`、`b_test`。 人間が守る: `c`。";
    assert_eq!(named_tests(item), Some(vec!["a_test".to_string(), "b_test".to_string()]));
    assert_eq!(named_tests("- 守るテストの無い項目。"), None);
    let found = invariants("x\n## 守ること（不変条件）\n\n- one\n- two\n\n## テスト\n- three\n");
    assert_eq!(found, ["- one", "- two"]);
}
