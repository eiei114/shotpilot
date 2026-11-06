//! Claude Code integration (D-66): the hooks tell Claude which teaser cuts went out of date,
//! and say nothing when nothing did; the installer adds the Skill and the hooks to the game's
//! folder without touching anything else there.

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use common::{launches, ws, Fixture};
use serde_json::{json, Value};
use shotpilot::claudecode;

fn write(f: &Fixture, rel: &str, text: &str) {
    let p = f.ws.root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

/// A demo project rendered at `still`, with the mock engine recording which content files
/// each shot showed (the well shows in some shots, not all).
fn rendered() -> Fixture {
    let mut f = ws();
    write(&f, "content/well.txt", "well texture v1");
    write(&f, "content/lantern.txt", "lantern v1");
    f.ws.render(&[], "still", false, None).unwrap();
    f
}

fn reopen(f: &mut Fixture) {
    f.ws = shotpilot::Workspace::open(f.path()).unwrap();
}

#[test]
fn nothing_out_of_date_means_no_words() {
    let f = rendered();
    assert!(claudecode::outdated(&f.ws).unwrap().iter().all(|o| o.quality == "previz"), "only previz was never asked for");
    assert_eq!(claudecode::session_start(&f.ws, "shots.yaml").unwrap(), None);
    assert_eq!(claudecode::post_tool(&f.ws, "shots.yaml", None).unwrap(), None);
}

#[test]
fn a_game_change_names_only_the_cuts_that_showed_it() {
    let mut f = rendered();
    assert_eq!(claudecode::session_start(&f.ws, "shots.yaml").unwrap(), None);
    // nothing changed: the post-tool hook stays quiet
    assert_eq!(claudecode::post_tool(&f.ws, "shots.yaml", None).unwrap(), None);

    write(&f, "content/well.txt", "well texture v2, repainted");
    reopen(&mut f);
    let out = claudecode::outdated(&f.ws).unwrap();
    let game: Vec<&str> = out.iter().filter(|o| !o.packages.is_empty()).map(|o| o.shot.as_str()).collect();
    assert!(game.contains(&"s02_well"), "{game:?}");
    assert!(game.len() < f.ws.defn.shot_ids().len(), "only the cuts with the well: {game:?}");

    let said = claudecode::post_tool(&f.ws, "shots.yaml", None).unwrap().expect("the change is reported");
    assert!(said.contains("ゲームの素材が変わった") && said.contains("well"), "{said}");
    for id in f.ws.defn.shot_ids() {
        assert_eq!(said.contains(&id), game.contains(&id.as_str()), "{id} in:\n{said}");
    }
    assert!(said.contains("shotpilot --file shots.yaml render still"), "{said}");
    // said once: the next tool call has nothing new to say
    assert_eq!(claudecode::post_tool(&f.ws, "shots.yaml", None).unwrap(), None);

    // a new session hears it again (it is still out of date)
    let again = claudecode::session_start(&f.ws, "shots.yaml").unwrap().expect("still out of date");
    assert!(again.contains("s02_well"));

    // re-shoot: only those cuts render, and the hooks fall silent
    let outcomes = f.ws.render(&[], "still", false, None).unwrap();
    for o in &outcomes {
        assert_eq!(o.action, if game.contains(&o.shot_id.as_str()) { "rendered" } else { "reused" }, "{}", o.shot_id);
    }
    assert_eq!(claudecode::session_start(&f.ws, "shots.yaml").unwrap(), None);
}

#[test]
fn an_edit_to_shots_yaml_says_what_it_means_for_rerendering() {
    let mut f = rendered();
    claudecode::session_start(&f.ws, "shots.yaml").unwrap();
    let path = f.path();

    // a purpose is editorial: no cut needs a new take
    let text = f.text().replace("purpose: 井戸のクローズアップ。何かがいる気配", "purpose: 井戸のクローズアップ。何かが潜む気配");
    f.write(&text);
    reopen(&mut f);
    let said = claudecode::post_tool(&f.ws, "shots.yaml", Some(&path)).unwrap().expect("the edit is explained");
    assert!(said.contains("撮り直しは無い"), "{said}");

    // a lens is picture: only that cut is out of date
    f.ws.set_fields("s03_window", &[("camera.lens.focal_length".to_string(), json!(85))]).unwrap();
    reopen(&mut f);
    let said = claudecode::post_tool(&f.ws, "shots.yaml", Some(&path)).unwrap().expect("the edit is explained");
    assert!(said.contains("s03_window") && said.contains("定義が変わった"), "{said}");
    for id in f.ws.defn.shot_ids().into_iter().filter(|i| i != "s03_window") {
        assert!(!said.contains(&id), "{id} did not change:\n{said}");
    }
    // the same file, unchanged: nothing more to say
    assert_eq!(claudecode::post_tool(&f.ws, "shots.yaml", Some(&path)).unwrap(), None);
}

#[test]
fn install_adds_the_skill_and_hooks_and_keeps_the_rest() {
    let f = ws();
    let root = f.ws.root.clone();
    let settings = root.join(".claude").join("settings.json");
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    let mine = json!({
        "permissions": {"allow": ["Bash(cargo test *)"]},
        "hooks": {"PostToolUse": [{"matcher": "Write", "hooks": [{"type": "command", "command": "my-formatter"}]}]},
        "model": "opus"
    });
    std::fs::write(&settings, serde_json::to_string_pretty(&mine).unwrap()).unwrap();

    let rep = claudecode::install(&f.ws, &root, false).unwrap();
    assert!(rep.skill_written);
    assert_eq!(rep.hooks_added, vec!["SessionStart", "PostToolUse"]);
    let skill = std::fs::read_to_string(&rep.skill).unwrap();
    assert!(skill.starts_with("---\nname: shotpilot-teaser\n"), "frontmatter first");
    assert!(skill.contains("shotpilot --file shots.yaml status") && !skill.contains("{{SHOTS}}"));

    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
    let keys: Vec<&String> = doc.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["permissions", "hooks", "model"], "order and other keys kept");
    let post = doc["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(post.len(), 2, "the developer's own hook stays");
    assert_eq!(post[0]["hooks"][0]["command"], "my-formatter");
    let cmd = post[1]["hooks"][0]["command"].as_str().unwrap();
    assert_eq!(cmd, "shotpilot --file \"${CLAUDE_PROJECT_DIR}/shots.yaml\" hook post-tool");
    assert!(doc["hooks"]["SessionStart"][0]["hooks"][0]["command"].as_str().unwrap().ends_with("hook session-start"));

    // twice: nothing is added again
    let rep = claudecode::install(&f.ws, &root, false).unwrap();
    assert!(!rep.skill_written && rep.hooks_added.is_empty());
    let doc2: Value = serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
    assert_eq!(doc, doc2);

    // a Skill edited by hand is kept unless forced
    std::fs::write(&rep.skill, "my own notes").unwrap();
    assert!(!claudecode::install(&f.ws, &root, false).unwrap().skill_written);
    assert_eq!(std::fs::read_to_string(&rep.skill).unwrap(), "my own notes");
    assert!(claudecode::install(&f.ws, &root, true).unwrap().skill_written);
}

#[test]
fn install_refuses_to_overwrite_settings_it_cannot_read() {
    let f = ws();
    let settings = f.ws.root.join(".claude").join("settings.json");
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(&settings, "{ // a comment\n}").unwrap();
    let err = claudecode::install(&f.ws, &f.ws.root, false).unwrap_err();
    assert!(err.msg.contains("not valid JSON"), "{}", err.msg);
    assert_eq!(std::fs::read_to_string(&settings).unwrap(), "{ // a comment\n}");
}

/// The binary as Claude Code runs it: JSON on stdin, JSON (or nothing) on stdout, exit 0.
fn hook(args: &[&str], stdin: &Value) -> (i32, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_shotpilot"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).to_string())
}

#[test]
fn the_hook_speaks_claude_codes_json_and_never_fails_the_session() {
    let mut f = rendered();
    let file = f.path().to_string_lossy().to_string();
    let input = json!({"hook_event_name": "PostToolUse", "tool_name": "Bash", "tool_input": {"command": "ls"}, "cwd": f.ws.root});
    let state = f.ws.root.join(".shotpilot").join("state.json");
    let before = (std::fs::read(&state).unwrap(), f.text(), launches(&f.ws).len());
    let (code, out) = hook(&["--file", &file, "hook", "session-start"], &json!({"hook_event_name": "SessionStart", "source": "startup"}));
    assert_eq!((code, out.trim()), (0, ""));

    write(&f, "content/lantern.txt", "lantern v2, brighter");
    reopen(&mut f);
    let (code, out) = hook(&["--file", &file, "hook", "post-tool"], &input);
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(out.trim()).unwrap_or_else(|e| panic!("{e}: {out}"));
    assert_eq!(v["hookSpecificOutput"]["hookEventName"], "PostToolUse");
    assert!(v["hookSpecificOutput"]["additionalContext"].as_str().unwrap().contains("lantern"));
    // the hooks report; they never render, and never write the definition or the takes' state
    assert_eq!((std::fs::read(&state).unwrap(), f.text(), launches(&f.ws).len()), before);

    // no project there: still exit 0, and silent
    let missing = f.ws.root.join("nope").join("shots.yaml").to_string_lossy().to_string();
    let (code, out) = hook(&["--file", &missing, "hook", "post-tool"], &input);
    assert_eq!((code, out.trim()), (0, ""));
}
