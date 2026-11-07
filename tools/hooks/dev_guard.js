#!/usr/bin/env node
// Claude Code hooks for working on Shotpilot itself (wired in .claude/settings.json).
//
//   post-tool  after Edit/Write/MultiEdit: remembers when a file the D-13 loop rests on was
//              touched, and reports right away UI strings missing from the English
//              dictionary (D-51), which are easy to forget and only show up in English mode.
//   stop       when Claude is about to end its turn and a loop file was touched, runs the
//              loop's tests (the demo's claim and the invariant table). If they fail, the
//              turn does not end: Claude gets the failures, once.
//
// Its own errors never fail the session (exit 0 and say nothing).
const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const repo = process.env.CLAUDE_PROJECT_DIR || path.resolve(__dirname, "..", "..");
// one mark per session: several sessions may share this working tree
const markOf = (input) => path.join(repo, "target", `claude-touched-loop-${String(input.session_id || "local").replace(/[^\w-]/g, "")}.txt`);

// Files whose edits can change what re-renders, what is reused or what the invariants say.
const LOOP = [
  "crates/shotpilot/src/fingerprint.rs",
  "crates/shotpilot/src/state.rs",
  "crates/shotpilot/src/content.rs",
  "crates/shotpilot/src/workspace.rs",
  "crates/shotpilot/src/model.rs",
  "crates/shotpilot/src/yamlio.rs",
  "crates/shotpilot/src/engines.rs",
  "crates/shotpilot/src/script.rs",
  "crates/shotpilot/tests/loop.rs",
  "crates/shotpilot/tests/model.rs",
  "crates/shotpilot/tests/game_changes.rs",
  "AGENTS.md",
];
const TESTS = ["test", "-q", "-p", "shotpilot", "--test", "loop", "--test", "model", "--test", "game_changes", "--test", "invariants"];

function readInput() {
  try {
    return JSON.parse(fs.readFileSync(0, "utf8") || "{}");
  } catch {
    return {};
  }
}

function rel(file) {
  if (!file) return "";
  const abs = path.isAbsolute(file) ? file : path.join(repo, file);
  return path.relative(repo, abs).split(path.sep).join("/");
}

function edited(input) {
  const ti = input.tool_input || {};
  return rel(ti.file_path || ti.path || "");
}

function postTool(input) {
  const file = edited(input);
  if (!file) return;
  const mark = markOf(input);
  if (LOOP.includes(file)) {
    fs.mkdirSync(path.dirname(mark), { recursive: true });
    const seen = fs.existsSync(mark) ? fs.readFileSync(mark, "utf8").split("\n").filter(Boolean) : [];
    if (!seen.includes(file)) fs.writeFileSync(mark, [...seen, file].join("\n") + "\n");
  }
  const ui = /^crates\/shotpilot\/(web\/[^/]+\.(html|js)|src\/[^/]+\.rs)$/.test(file);
  if (!ui) return;
  const r = spawnSync("node", [path.join(repo, "tools", "i18n_extract.js"), "--check"], { cwd: repo, encoding: "utf8" });
  if (r.status !== 0) return;
  let missing = [];
  try {
    missing = Object.keys(JSON.parse(r.stdout));
  } catch {
    return;
  }
  if (!missing.length) return;
  const shown = missing.slice(0, 5).map((k) => `「${k}」`).join("、");
  const more = missing.length > 5 ? ` ほか ${missing.length - 5} 件` : "";
  const text =
    `i18n（D-51）: 画面の文言 ${missing.length} 件が crates/shotpilot/web/i18n-en.json に無い。英語モードでは日本語のまま出る: ${shown}${more}。` +
    "一覧は `node tools/i18n_extract.js`。{0} などの穴は日本語と同じ組にする。";
  process.stdout.write(JSON.stringify({ hookSpecificOutput: { hookEventName: "PostToolUse", additionalContext: text } }));
}

function stop(input) {
  const mark = markOf(input);
  if (!fs.existsSync(mark)) return;
  const touched = fs.readFileSync(mark, "utf8").split("\n").filter(Boolean);
  const r = spawnSync("cargo", TESTS, { cwd: repo, encoding: "utf8", maxBuffer: 64 << 20 });
  const out = `${r.stdout || ""}\n${r.stderr || ""}`;
  if (r.status === 0) {
    fs.rmSync(mark, { force: true });
    return;
  }
  const lines = out.split(/\r?\n/);
  const keep = lines.filter((l) => /FAILED|panicked|^error|SHOTPILOT_MODEL_SEED|assertion|left:|right:|-->/.test(l)).slice(0, 40);
  const reason =
    `D-13 のループのテストが落ちている（このターンで ${touched.join(", ")} を変えた）。` +
    "部分修正ループはデモの主張そのもので、落としたまま先に進まない（AGENTS.md「テスト」）。\n" +
    `再現: cargo ${TESTS.join(" ")}\n` +
    keep.join("\n");
  if (input.stop_hook_active) {
    // already sent back once this turn: tell the developer and let the turn end
    process.stdout.write(JSON.stringify({ systemMessage: `dev_guard: ループのテストがまだ落ちている（cargo ${TESTS.join(" ")}）` }));
    return;
  }
  process.stdout.write(JSON.stringify({ decision: "block", reason }));
}

try {
  const input = readInput();
  const mode = process.argv[2];
  if (mode === "post-tool") postTool(input);
  else if (mode === "stop") stop(input);
} catch {
  // never fail the session
}
process.exit(0);
