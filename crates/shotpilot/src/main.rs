//! shotpilot command line.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use shotpilot::ai;
use shotpilot::connect::{self, Settings};
use shotpilot::error::Result;
use shotpilot::fingerprint as fp;
use shotpilot::model::{self, fmt_g};
use shotpilot::workspace::EditReport;
use shotpilot::{sound, soundcheck};
use shotpilot::{bail, templates, yamlio, Workspace};

#[derive(Parser)]
#[command(name = "shotpilot", version, about = "Shot design and capture lane for UE5 teasers.")]
struct Cli {
    /// shot definition
    #[arg(short, long, default_value = "shots.yaml", global = true)]
    file: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// write a starter shots.yaml
    Init {
        /// mock (no engine), ue5, or three (a sample game drawn in Edge or Chrome, D-72)
        #[arg(long, default_value = "mock", value_parser = ["mock", "ue5", "three"])]
        engine: String,
        #[arg(long)]
        uproject: Option<String>,
        /// e.g. /Game/Maps/Courtyard
        #[arg(long)]
        map: Option<String>,
        #[arg(long)]
        name: Option<String>,
        /// no sample shots
        #[arg(long)]
        empty: bool,
        #[arg(long)]
        force: bool,
    },
    /// check shots.yaml
    Validate,
    /// what is fresh, stale or out of sync
    Status {
        #[arg(long)]
        json: bool,
    },
    /// push changed shots into engine sequences
    Build {
        shots: Vec<String>,
        /// rebuild even if unchanged / overwrite editor edits
        #[arg(long)]
        force: bool,
    },
    /// render stale shots: render [previz|still|preview|final] [shot ...]
    Render {
        #[arg(value_name = "QUALITY/SHOT")]
        targets: Vec<String>,
        /// re-render even if a matching take exists
        #[arg(long)]
        force: bool,
    },
    /// bring Sequencer edits back into shots.yaml
    Pull {
        /// how to resolve conflicts
        #[arg(long, value_parser = ["engine", "yaml"])]
        prefer: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// import anchors from tagged level actors
    Scan {
        #[arg(long, default_value = "shotpilot")]
        tag: String,
        #[arg(long, default_value = "SP_")]
        prefix: String,
    },
    /// assemble the teaser from existing takes (no rendering)
    Cut {
        /// best (default) | previz | still | preview | final
        #[arg(default_value = "best")]
        quality: String,
        #[arg(long)]
        no_video: bool,
    },
    /// lint the shot list against trailer craft rules (P0 blocks approval)
    Check {
        #[arg(long)]
        json: bool,
    },
    /// sign off a shot's framing; any edit that changes its picture withdraws it
    Approve {
        #[arg(required_unless_present = "script")]
        shots: Vec<String>,
        /// withdraw the approval
        #[arg(long)]
        undo: bool,
        /// approve the script (its text and the cut it is the spine of) instead of shots
        #[arg(long)]
        script: bool,
    },
    /// read the level's plan (walls, things, lamps, storeys) for the blocking map. UE only; reads, never saves the level
    Level,
    /// photograph every anchor from four sides into scout/ (stage 0, location scouting).
    /// With no anchors yet, brings them in from the level first (and the mesh inventory)
    Scout {
        anchors: Vec<String>,
    },
    /// the planning turn: what teaser to make, its look and its playbooks (written to brief:)
    Discover {
        /// what you want, in your own words (default: teaser.pitch)
        #[arg(default_value = "")]
        request: String,
        /// answers to the questions Claude asked last time, as text
        #[arg(long, default_value = "")]
        answers: String,
    },
    /// the look: list the design systems | design <id> (use it) | design <id> --fork (copy its DESIGN.md here to edit)
    Design {
        id: Option<String>,
        #[arg(long)]
        fork: bool,
    },
    /// the teaser playbooks the planning turn can pick from
    Skills,
    /// borrow a reference teaser's rhythm and hand-overs (D-71): measure a video (up to 180 s),
    /// write its lengths into finish.motion, compare the latest cut with it
    Reference {
        /// the video to measure; with none, list the references measured so far
        video: Option<String>,
        /// measure only: leave the brief and finish.motion as they are
        #[arg(long)]
        measure_only: bool,
        /// borrow a reference measured before
        #[arg(long = "use")]
        use_id: Option<String>,
        /// measure the latest cut the same way and pair its frames with the reference's
        #[arg(long)]
        compare: bool,
    },
    /// concept frames (D-46): make the pictures the plan describes | --match: find their places in the level
    Concept {
        #[arg(long)]
        r#match: bool,
    },
    /// the first pass in one go: plan, scout, stage, mood board, script + shots, render, cut
    Produce {
        /// first render quality (previz needs no engine)
        #[arg(long, default_value = "previz")]
        quality: String,
        /// extra direction for the cut
        #[arg(long, default_value = "")]
        note: String,
        /// skip the mood board
        #[arg(long)]
        no_mood: bool,
        /// stop after the script, the shots and their previz, to check them first
        #[arg(long, conflicts_with = "resume")]
        review: bool,
        /// continue after a review: sound, render, cut
        #[arg(long)]
        resume: bool,
    },
    /// read the set from the scout photos with Claude and write `stage:` (stage 0)
    Stage,
    /// the meshes the project has: assets [filter]  |  assets --scan (asks the engine)
    Assets {
        filter: Option<String>,
        #[arg(long)]
        scan: bool,
    },
    /// props shots may make appear: prop add <name> <asset>  |  prop rm <name>
    Prop {
        #[arg(value_parser = ["add", "rm"])]
        action: String,
        name: String,
        asset: Option<String>,
    },
    /// generate mood-board images from the brief (Codex, or an OpenAI API key)
    Moodboard {
        /// art direction, e.g. "cold blue, red lantern, fog"
        #[arg(long, default_value = "")]
        direction: String,
        #[arg(short, long, default_value_t = 4)]
        n: usize,
    },
    /// paint near-real storyboard frames over the previz layout (preview only; never in the cut)
    Storyboard {
        /// shots to paint (all when empty)
        shots: Vec<String>,
        /// art direction, e.g. "cold blue, red lantern, fog"
        #[arg(long, default_value = "")]
        direction: String,
        /// repaint even if the frame matches the shot
        #[arg(long)]
        force: bool,
    },
    /// what the trailer is for: brief kind=teaser genre=ホラー must_show='[井戸, 窓]'
    Brief {
        assignments: Vec<String>,
    },
    /// what is burned in at the cut: finish logo=logo.png title_size=0.18 fonts='{display: "Shippori Mincho:800"}'
    Finish {
        assignments: Vec<String>,
    },
    /// edit fields of one shot: set s03 camera.lens.focal_length=50 duration=5
    Set {
        shot: String,
        #[arg(required = true)]
        assignments: Vec<String>,
    },
    /// reorder shots: order s06 s01 ...  |  order --move s06 --to 1
    Order {
        ids: Vec<String>,
        #[arg(long = "move")]
        move_: Option<String>,
        #[arg(long, default_value_t = 1)]
        to: usize,
    },
    /// draft a shot list with Claude from the pitch and anchors
    Plan {
        #[arg(long)]
        pitch: Option<String>,
        #[arg(long)]
        duration: Option<f64>,
        #[arg(long)]
        shots: Option<usize>,
        /// extra direction for the cut (e.g. "slow burn, no jump scares")
        #[arg(long)]
        note: Option<String>,
        /// print the brief, skeleton and craft rules Claude would get, without calling it
        #[arg(long)]
        spec: bool,
        #[arg(long)]
        write: bool,
        #[arg(long)]
        replace: bool,
    },
    /// score the planner against the collected teasers: eval [CASES...] [--models a,b] [--runs N] [--candidates N] [--out DIR] [--replay DIR]
    Eval {
        /// projects (shots.yaml or its folder); default: eval/cases/*/shots.yaml
        cases: Vec<PathBuf>,
        /// models to compare, comma separated (default: the current model)
        #[arg(long, value_delimiter = ',')]
        models: Vec<String>,
        #[arg(long, default_value_t = 1)]
        runs: usize,
        /// plans written per run, the best one kept by code score (as SHOTPILOT_CANDIDATES does)
        #[arg(long)]
        candidates: Option<usize>,
        /// where results.jsonl, replies/ and summary.md go (default: eval/runs/<time>)
        #[arg(long)]
        out: Option<PathBuf>,
        /// re-score the saved replies in this folder with the current code (no model calls)
        #[arg(long)]
        replay: Option<PathBuf>,
    },
    /// apply a note with Claude: revise "3番目はもっと寄って"
    Revise {
        note: String,
        /// limit edits to this shot (repeatable)
        #[arg(long)]
        shot: Vec<String>,
        /// change only this part (every shot unless --shot): camera, light, or finish (text burned in at the
        /// cut, hand-overs and camera effects; on every shot, also the script's words and the grade)
        #[arg(long, value_parser = ["camera", "light", "finish"])]
        aspect: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    /// mix the BGM and sound effects and check the mix against launch films (never re-renders).
    /// with assignments, edit `audio:` first: sound bgm.file=audio/bgm.wav bgm.gain=-6 gain=0
    Sound {
        assignments: Vec<String>,
        #[arg(long)]
        json: bool,
        /// design the music and effects afresh from the sound library (Claude), then mix
        #[arg(long)]
        design: bool,
        /// list the effects shotpilot makes in code (D-103), or those matching some words
        #[arg(long, num_args = 0..=1, default_missing_value = "", value_name = "WORDS")]
        made: Option<String>,
        /// make one effect in code into audio/made/ and print its path, e.g. --make 'impact/deep?dur=2'
        #[arg(long, value_name = "ID")]
        make: Option<String>,
    },
    /// the release plan (D-47): promo  |  promo post <shot> [--clip]  |  promo derive <video> [--shots]
    ///  |  promo update [version]  (an update trailer made from these shots; only changed cuts re-render)
    Promo {
        /// post | derive | update (none = show the checklists)
        action: Option<String>,
        /// the shot (post), the video row (derive) or the version name (update)
        target: Option<String>,
        /// post: a one-cut clip instead of a still
        #[arg(long)]
        clip: bool,
        /// derive: carry the shots and takes over, not only the groundwork
        #[arg(long)]
        shots: bool,
        #[arg(long)]
        json: bool,
    },
    /// Claude Code hooks (D-66): run by Claude Code in the game's folder, never blocks.
    /// session-start: what is out of date now  |  post-tool: what the last tool call changed
    Hook {
        #[arg(value_parser = ["session-start", "post-tool"])]
        event: String,
    },
    /// Claude Code integration (D-66): install the Skill and the hooks into the game's folder
    Claude {
        #[arg(value_parser = ["install"])]
        action: String,
        /// the game's folder (default: the folder of project.uproject, else this project's)
        #[arg(long)]
        dir: Option<PathBuf>,
        /// overwrite a Skill that was edited by hand
        #[arg(long)]
        force: bool,
        /// only print the hooks block to paste into .claude/settings.json
        #[arg(long)]
        print: bool,
    },
    /// list takes of one shot
    Takes { shot: String },
    /// history of builds, renders, pulls and edits
    Log {
        #[arg(short, default_value_t = 30)]
        n: usize,
    },
    /// delete old takes
    Gc {
        #[arg(long, default_value_t = 2)]
        keep: usize,
        #[arg(long)]
        dry_run: bool,
    },
}

/// The release plan as text: one block per video with its checklist, then the posts.
fn print_promo(v: &Value) {
    let s = |x: &Value| x.as_str().unwrap_or("").to_string();
    println!("release plan: {}{}", s(&v["plan"]), if v["saved"] == false { " (not saved yet)" } else { "" });
    for video in v["videos"].as_array().into_iter().flatten() {
        let due = match video["days"].as_i64() {
            Some(d) if d < 0 => format!("  {} ({} days late)", s(&video["due"]), -d),
            Some(d) => format!("  {} (in {d} days)", s(&video["due"])),
            None => String::new(),
        };
        let here = if video["here"] == true { "  [this project]" } else if video["project"].is_null() { "  [not started]" } else { "" };
        println!("
{} {} — {}  {}/{}{due}{here}", s(&video["id"]), s(&video["title"]), s(&video["kind_ja"]),
                 video["done"], video["total"]);
        for c in video["checks"].as_array().into_iter().flatten() {
            let mark = if c["done"] == true { "x" } else { " " };
            let auto = if c["auto"] == true { " (auto)" } else { "" };
            let detail = Some(s(&c["detail"])).filter(|d| !d.is_empty()).map(|d| format!(" — {d}")).unwrap_or_default();
            println!("  [{mark}] {}{auto}{detail}", s(&c["text"]));
        }
    }
    let changed = v["changed"].as_array().cloned().unwrap_or_default();
    if !changed.is_empty() {
        println!("
game changed under {} cut(s): {}", changed.len(),
                 changed.iter().map(|c| s(&c["id"])).collect::<Vec<_>>().join(", "));
        println!("  `shotpilot promo update v1.1` starts an update trailer from these shots");
    }
    let p = &v["posting"];
    println!("
posts: every {} days{}; next {} (in {} days)", p["every_days"],
             Some(s(&p["channel"])).filter(|c| !c.is_empty()).map(|c| format!(" on {c}")).unwrap_or_default(),
             s(&p["next"]), p["days"]);
    for c in p["candidates"].as_array().into_iter().flatten().take(3) {
        let tag = if c["retake"] == true { " (retaken: the game changed)" } else if c["new"] == true { " (new)" } else { "" };
        println!("  candidate {}{tag}: `shotpilot promo post {}`", s(&c["shot"]), s(&c["shot"]));
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // the look's typefaces are fetched from Google Fonts the first time a cut needs them
    shotpilot::fonts::allow_fetch(true);
    match run(cli) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("shotpilot: {e}");
            ExitCode::from(2)
        }
    }
}

/// One Claude Code hook call: read what Claude Code sends on stdin, say what changed.
fn run_hook(file: &PathBuf, event: &str) -> Result<()> {
    use std::io::Read;
    let mut input = String::new();
    if !std::io::stdin().is_terminal() {
        std::io::stdin().read_to_string(&mut input)?;
    }
    let input: Value = serde_json::from_str(&input).unwrap_or(Value::Null);
    let ws = ws(file)?;
    let root = shotpilot::claudecode::game_root(&ws);
    let shots = shotpilot::claudecode::shots_arg(&ws, &root).replace("\"${CLAUDE_PROJECT_DIR}/", "").replace('"', "");
    match event {
        "session-start" => {
            if let Some(text) = shotpilot::claudecode::session_start(&ws, &shots)? {
                println!("{text}");
            }
        }
        _ => {
            let edited = shotpilot::claudecode::edited_file(&input);
            if let Some(text) = shotpilot::claudecode::post_tool(&ws, &shots, edited.as_deref())? {
                println!("{}", shotpilot::claudecode::post_tool_json(&text));
            }
        }
    }
    Ok(())
}

fn ws(cli_file: &PathBuf) -> Result<Workspace> {
    Workspace::open(cli_file)
}

fn print_edit_report(rep: &EditReport) {
    if !rep.order.is_empty() {
        println!("new order: {}", rep.order.join(" > "));
    }
    println!("edited: {}", if rep.edited.is_empty() { "-".into() } else { rep.edited.join(", ") });
    if rep.audio {
        println!("audio: changed (re-mixed at the next cut; nothing re-renders)");
    }
    println!("now stale (need re-render): {}", if rep.stale.is_empty() { "nothing".into() } else { rep.stale.join(", ") });
}

/// key=value pairs; an empty value means null (remove).
fn parse_assignments(raw: &[String]) -> Result<Vec<(String, Value)>> {
    let mut parsed = Vec::new();
    for a in raw {
        let Some((k, v)) = a.split_once('=') else { bail!("expected key=value, got '{a}'") };
        let value = if v.trim().is_empty() { Value::Null } else { yamlio::parse_str(v, &format!("value of {k}"))? };
        parsed.push((k.trim().to_string(), value));
    }
    Ok(parsed)
}

/// "T_Enemy_D, M_Enemy +3" from a list of content packages.
fn short_list(packages: &[Value]) -> String {
    let names: Vec<&str> = packages.iter().filter_map(Value::as_str).map(shotpilot::content::short).collect();
    match names.len() {
        0..=3 => names.join(", "),
        n => format!("{} +{}", names[..3].join(", "), n - 3),
    }
}

/// What a reference lent `finish.motion` (D-71).
fn print_borrowed(notes: &[String], id: &str) {
    if notes.is_empty() {
        return;
    }
    println!("borrowed from `{id}` into finish.motion (no shot re-renders; run `shotpilot cut` to see it):");
    for n in notes {
        println!("  {n}");
    }
}

fn dash(v: &[String]) -> String {
    if v.is_empty() { "-".into() } else { v.join(", ") }
}

/// A client that only reports why no real one could be made, so a run records the error per call.
struct FailingClaude(String);

impl ai::Claude for FailingClaude {
    fn ask(&self, _request: &Value) -> Result<String> {
        Err(shotpilot::Error::user(self.0.clone()))
    }
}

/// The eval cases: explicit projects (a shots.yaml or its folder), else eval/cases/*/shots.yaml
/// found from the current folder upwards.
fn eval_cases(given: &[PathBuf]) -> Result<Vec<PathBuf>> {
    if !given.is_empty() {
        return Ok(given.iter().map(|p| if p.is_dir() { p.join("shots.yaml") } else { p.clone() }).collect());
    }
    let mut dir = std::env::current_dir()?;
    loop {
        let cases = dir.join("eval").join("cases");
        if cases.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(&cases)?
                .filter_map(|e| e.ok().map(|e| e.path().join("shots.yaml")))
                .filter(|p| p.is_file())
                .collect();
            found.sort();
            return Ok(found);
        }
        if !dir.pop() {
            bail!("no eval/cases folder found; pass the projects to score");
        }
    }
}

fn run(cli: Cli) -> Result<u8> {
    let file = &cli.file;
    match cli.cmd {
        Cmd::Init { engine, uproject, map, name, empty, force } => {
            if file.exists() && !force {
                bail!("{} already exists (use --force to overwrite)", file.display());
            }
            let abs = std::path::absolute(file)?;
            let name = name.unwrap_or_else(|| {
                abs.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or("shotpilot".into())
            });
            let uproject = uproject.map(|u| serde_json::to_string(&u).unwrap_or_default()).unwrap_or("null".into());
            let map = map.map(|m| yamlio::emit_str(&m)).unwrap_or("null".into());
            let three = engine == "three";
            let tpl = match (empty, three) {
                (true, _) => templates::EMPTY,
                (false, true) => templates::THREE_VILLAGE,
                (false, false) => templates::DEMO,
            };
            std::fs::write(file, templates::render(tpl, &name, &engine, &uproject, &map))?;
            if !empty {
                if let Some(dir) = abs.parent() {
                    // placeholder music, air and effects the sample refers to
                    if three {
                        sound::write_village_audio(dir)?;
                    } else {
                        sound::write_demo_audio(dir)?;
                    }
                }
            }
            if three {
                // the game the shots film (an existing game/ is left as it is)
                let game = abs.parent().map(|p| p.join("game")).unwrap_or_else(|| "game".into());
                if shotpilot::threegame::write(&game)? {
                    println!("wrote the sample game in {}", game.display());
                }
            }
            let gi = abs.parent().map(|p| p.join(".gitignore")).unwrap_or_else(|| ".gitignore".into());
            if !gi.exists() {
                std::fs::write(gi, templates::GITIGNORE)?;
            }
            model::load(&abs)?; // validate
            println!("wrote {}. next: shotpilot render previz && shotpilot cut", file.display());
        }
        Cmd::Validate => {
            let ws = ws(file)?;
            let shots = ws.resolved()?;
            let total: f64 = shots.iter().map(|s| s.duration).sum();
            println!("ok: {} shots, {}s (target {}s), engine={}", shots.len(), fmt_g(total),
                     fmt_g(ws.defn.teaser.target_duration), ws.defn.project.engine);
            for w in ws.structure_warnings()? {
                println!("  note: {w}");
            }
        }
        Cmd::Status { json } => {
            let ws = ws(file)?;
            let rows = ws.status()?;
            if json {
                let v: Vec<Value> = rows.iter().map(|r| r.to_json()).collect();
                println!("{}", serde_json::to_string_pretty(&v)?);
                return Ok(0);
            }
            let qs = fp::quality_names(&ws.defn);
            let idw = rows.iter().map(|r| r.id.len()).max().unwrap_or(4).max(4);
            let qh: Vec<String> = qs.iter().map(|q| format!("{q:<7}")).collect();
            println!("{:>2}  {:<idw$}  {:<9}  {:>5}  {:<6}  {:<8}  {}", "#", "shot", "beat", "sec", "engine", "framing", qh.join("  "));
            for r in &rows {
                let marks: Vec<String> = qs
                    .iter()
                    .map(|q| {
                        let game = r.changed.iter().any(|(k, _)| k == q);
                        let m = match r.quality(q) {
                            "fresh" => "fresh",
                            "stale" if game => "GAME",
                            "stale" => "STALE",
                            _ => "-",
                        };
                        format!("{m:<7}")
                    })
                    .collect();
                let framing = match r.approval.as_str() {
                    "approved" => "ok",
                    "outdated" => "CHANGED",
                    _ => "-",
                };
                println!("{:>2}  {:<idw$}  {:<9}  {:>5}  {:<6}  {:<8}  {}", r.index + 1, r.id, r.beat.as_deref().unwrap_or("-"),
                         fmt_g(r.duration), r.build, framing, marks.join("  "));
            }
            let total: f64 = rows.iter().map(|r| r.duration).sum();
            let target = ws.defn.teaser.target_duration;
            let diff = total - target;
            println!("\ntotal {}s / target {}s ({}{}s)", fmt_g(total), fmt_g(target), if diff >= 0.0 { "+" } else { "" }, fmt_g(diff));
            println!("engine: synced = sequence asset matches shots.yaml, dirty = needs build, new = never built");
            println!("takes:  fresh = take exists for the current definition, STALE = only older takes exist,");
            println!("        GAME = the game content the take showed changed since (re-render shoots only those)");
            for r in rows.iter().filter(|r| !r.changed.is_empty()) {
                let mut pkgs: Vec<Value> = r.changed.iter().flat_map(|(_, p)| p.iter().map(|x| json!(x))).collect();
                pkgs.dedup();
                let qs: Vec<&str> = r.changed.iter().map(|(q, _)| q.as_str()).collect();
                println!("game changed under {} ({}): {}", r.id, qs.join(", "), short_list(&pkgs));
            }
            println!("framing: ok = approved, CHANGED = the picture changed after approval (shotpilot approve)");
            for w in ws.structure_warnings()? {
                println!("note: {w}");
            }
        }
        Cmd::Build { shots, force } => {
            let mut ws = ws(file)?;
            let rep = ws.build(&shots, force)?;
            if rep.skipped.is_some() && rep.built.is_empty() {
                println!("nothing to build: engine already matches shots.yaml");
                return Ok(0);
            }
            println!("built: {}", dash(&rep.built));
            if !rep.diverged.is_empty() {
                println!("NOT overwritten (edited in the editor): {}. run `shotpilot pull` to keep those edits, \
                          or `build --force` to discard them", rep.diverged.join(", "));
            }
            if !rep.error.is_empty() {
                println!("errors: {}", rep.error.join(", "));
                return Ok(1);
            }
        }
        Cmd::Render { mut targets, force } => {
            let mut ws = ws(file)?;
            let mut quality = "previz".to_string();
            if targets.first().is_some_and(|t| fp::quality_names(&ws.defn).contains(t)) {
                quality = targets.remove(0);
            }
            // the shots that render because the game changed under their take, and what changed
            let mut game_changed = serde_json::Map::new();
            let outcomes = {
                let mut on = |event: &str, data: &Value| {
                    if event == "plan" {
                        game_changed = data["game_changed"].as_object().cloned().unwrap_or_default();
                    }
                };
                ws.render(&targets, &quality, force, Some(&mut on))?
            };
            for o in &outcomes {
                let extra = match o.action.as_str() {
                    "rendered" => match game_changed.get(&o.shot_id).and_then(Value::as_array) {
                        Some(p) => format!("{:.1}s  (game changed: {})", o.seconds, short_list(p)),
                        None => format!("{:.1}s", o.seconds),
                    },
                    "failed" => o.detail.clone(),
                    _ => String::new(),
                };
                println!("  {:<8} {:<20} {quality}:{}  {extra}", o.action, o.shot_id, &o.fingerprint[..8]);
            }
            let count = |a: &str| outcomes.iter().filter(|o| o.action == a).count();
            println!("{quality}: {} rendered, {} reused, {} failed", count("rendered"), count("reused"), count("failed"));
            if count("failed") > 0 {
                return Ok(1);
            }
        }
        Cmd::Pull { prefer, dry_run } => {
            let mut ws = ws(file)?;
            let decisions = ws.pull(prefer.as_deref(), dry_run)?;
            if decisions.is_empty() {
                println!("nothing has been pushed to the engine yet");
                return Ok(0);
            }
            for d in &decisions {
                let mut lab = match d.kind.as_str() {
                    "clean" => "unchanged",
                    "engine_edit" if dry_run => "would pull (edited in engine)",
                    "engine_edit" => "PULLED (edited in engine)",
                    "agree" => "in sync",
                    "conflict" => "CONFLICT (changed in both)",
                    "missing" => "missing in engine",
                    _ => "never pushed",
                }
                .to_string();
                if d.kind.as_str() == "conflict" {
                    if let Some(p) = &prefer {
                        lab.push_str(&format!(" -> kept {p}"));
                    }
                }
                println!("  {:<20} {lab}", d.shot_id);
                for c in &d.changes {
                    println!("      {c}");
                }
            }
            if decisions.iter().any(|d| d.kind.as_str() == "conflict") && prefer.is_none() {
                println!("resolve conflicts with `shotpilot pull --prefer engine` or `--prefer yaml`");
                return Ok(1);
            }
        }
        Cmd::Scan { tag, prefix } => {
            let mut ws = ws(file)?;
            let rep = ws.scan(&tag, &prefix)?;
            println!("anchors added: {}; moved: {}; unchanged: {}", dash(&rep.added), dash(&rep.moved), rep.same.len());
        }
        Cmd::Promo { action, target, clip, shots, json } => {
            use shotpilot::promo;
            let ws = ws(file)?;
            let today = promo::today();
            let target = target.unwrap_or_default();
            match action.as_deref() {
                None => {}
                Some("post") => {
                    let id = promo::export_post(&ws, &target, clip, today)?;
                    println!("post {id}: {} exported to {}/", target, promo::POSTS_DIR);
                }
                Some("derive") => {
                    let carry = if shots { promo::Carry::Shots } else { promo::Carry::Groundwork };
                    println!("made {}", promo::derive(&ws, &target, carry)?.display());
                }
                Some("update") => {
                    let (id, p) = promo::start_update(&ws, &target, "")?;
                    println!("made {id}: {}", p.display());
                }
                Some(other) => bail!("promo: unknown action '{other}' (post / derive / update)"),
            }
            let v = promo::view(&ws, today)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else if action.is_none() {
                print_promo(&v);
            }
        }
        Cmd::Cut { quality, no_video } => {
            let mut ws = ws(file)?;
            let info = ws.cut(&quality, !no_video)?;
            println!("cut: {} shots, {}s -> {}", info.shots, fmt_g(info.seconds), info.dir.display());
            println!("  storyboard: {}", info.dir.join("index.html").display());
            println!("  spec:       {}", info.dir.join("spec.md").display());
            match &info.video {
                Some(v) => println!("  video:      {}", v.display()),
                None if !no_video => println!("  video:      skipped (ffmpeg not found)"),
                None => {}
            }
            if let (Some(a), Some(c)) = (&info.audio, &info.check) {
                println!("  sound:      {} ({})", a.display(), c["verdict_ja"].as_str().unwrap_or(""));
            }
            for n in &info.notes {
                println!("  note:       {n}");
            }
        }
        Cmd::Check { json } => {
            let ws = ws(file)?;
            let found = ws.check()?;
            if json {
                let v: Vec<Value> = found.iter().map(|f| f.to_json()).collect();
                println!("{}", serde_json::to_string_pretty(&v)?);
                return Ok(0);
            }
            for s in ws.resolved()? {
                if let Some(fr) = shotpilot::shotcheck::framing(&ws.defn, &s) {
                    let pct = |v: f64| if v.is_finite() { format!("{}%", (v * 100.0).round()) } else { "inside".into() };
                    println!("  {:<24} {:<10} fills {} -> {} of the frame height ({}{})", s.id, fr.subject, pct(fr.fill.0),
                             pct(fr.fill.1), shotpilot::shotcheck::size_name(fr.fill.0.min(fr.fill.1)),
                             if fr.known { "" } else { ", size unknown" });
                }
            }
            if found.is_empty() {
                println!("ok: no findings");
            }
            for f in &found {
                println!("{}
      fix: {}", f.line(), f.fix);
            }
            let p0 = found.iter().filter(|f| f.severity == shotpilot::shotcheck::Severity::P0).count();
            return Ok(if p0 > 0 { 1 } else { 0 });
        }
        Cmd::Approve { shots, undo, script: true } => {
            let mut ws = ws(file)?;
            if !shots.is_empty() {
                bail!("approve --script takes no shot ids");
            }
            ws.approve_script(!undo)?;
            println!("script: {}", ws.script_approval());
        }
        Cmd::Approve { shots, undo, script: false } => {
            let mut ws = ws(file)?;
            for s in ws.select(&shots)? {
                ws.approve(&s.id, !undo)?;
                println!("{} {}", if undo { "withdrawn:" } else { "approved:" }, s.id);
            }
        }
        Cmd::Level => {
            let mut ws = ws(file)?;
            let n = ws.read_level()?;
            let floors = shotpilot::scout::load_level(&ws.root).map(|l| l["floors"].to_string()).unwrap_or_default();
            println!("level plan: {n} shapes, floors at {floors} -> scout/level.json");
        }
        Cmd::Scout { anchors } => {
            let mut ws = ws(file)?;
            // Claude picks the places by the plan when it is connected; the scout runs without it
            let claude = connect::make_claude(&Settings::load(Settings::default_path().as_deref())).ok();
            let rep = ws.scout(&anchors, claude.as_deref())?;
            if let Some(s) = &rep.scan {
                let how = if s.auto { "picked from the level" } else { "tagged in the level" };
                println!("anchors {how}: {}", dash(&s.added));
            }
            if let Some(n) = rep.assets {
                println!("assets inventoried: {n}");
            }
            for (name, views) in rep.index["anchors"].as_object().into_iter().flatten() {
                let files: Vec<&str> = views.as_array().into_iter().flatten().filter_map(|v| v["file"].as_str()).collect();
                println!("{name:<16} {}", files.join("  "));
            }
        }
        Cmd::Produce { quality, note, no_mood, review, resume } => {
            let mut ws = ws(file)?;
            let settings = Settings::load(Settings::default_path().as_deref());
            let claude = connect::make_claude(&settings)?;
            let images = if no_mood || resume { None } else { connect::make_image_gen(&settings).ok() };
            let opts = shotpilot::produce::ProduceOpts { quality, direction: note, mood: !no_mood, library: None, review, stop: Vec::new() }
                .with_user_library();
            let mut print = |e: &str, d: &Value| {
                if e == "step" && d["status"] != "start" {
                    println!("{:<10} {:<8} {}", d["label"].as_str().unwrap_or(""), d["status"].as_str().unwrap_or(""),
                             d["detail"].as_str().unwrap_or(""));
                } else if e == "step" {
                    println!("{} ...", d["label"].as_str().unwrap_or(""));
                }
            };
            let out = if resume {
                shotpilot::produce::resume(&mut ws, claude.as_ref(), &opts, &mut print)?
            } else {
                shotpilot::produce::produce(&mut ws, claude.as_ref(), images.as_deref(), &opts, &mut print)?
            };
            println!("\n{}", out["rationale"].as_str().unwrap_or(""));
            for n in out["notes"].as_array().into_iter().flatten() {
                println!("note: {}", n.as_str().unwrap_or(""));
            }
            if let Some(v) = out["video"].as_str() {
                println!("video: {v}");
            }
            if out["stopped"] == "review" {
                println!("next: check the script and the shots (status / check), fix them, approve the script \
                          (shotpilot approve --script), then shotpilot produce --resume");
            } else {
                println!("next: check the shots (shotpilot check), approve or point-fix them, then render final");
            }
        }
        Cmd::Stage => {
            let mut ws = ws(file)?;
            let claude = connect::make_claude(&Settings::load(Settings::default_path().as_deref()))?;
            let st = ai::understand_stage(&ws.defn, claude.as_ref())?;
            ws.set_stage(&st)?;
            println!("{}\n", st.summary);
            for (n, d) in &st.anchors {
                println!("- {n}: {d}");
            }
            let mood: Vec<String> = st.mood.iter().map(|(f, a)| format!("{f}={a}")).collect();
            println!("\nmood board bases: {}", mood.join(", "));
        }
        Cmd::Assets { filter, scan } => {
            let mut ws = ws(file)?;
            let list = if scan { ws.scan_assets()? } else { shotpilot::scout::load_assets(&ws.root) };
            if list.is_empty() {
                println!("no inventory yet: run `shotpilot assets --scan`");
            }
            let f = filter.unwrap_or_default().to_lowercase();
            for a in list.iter().filter(|a| a.asset.to_lowercase().contains(&f)) {
                let size = a.size.map(|s| format!("{} x {} x {} cm", fmt_g(s[0]), fmt_g(s[1]), fmt_g(s[2]))).unwrap_or_else(|| "-".into());
                println!("{:<9} {:<22} {}   (prop add {} {})", a.kind, size, a.asset, a.suggested_name(), a.asset);
            }
        }
        Cmd::Prop { action, name, asset } => {
            let mut ws = ws(file)?;
            if action == "add" {
                let Some(asset) = asset else { bail!("prop add <name> <asset path>") };
                ws.add_prop(&name, &asset, None, None)?;
                println!("added prop {name}: {asset}");
            } else {
                ws.remove_prop(&name)?;
                println!("removed prop {name}");
            }
        }
        Cmd::Moodboard { direction, n } => {
            let mut ws = ws(file)?;
            let gen = connect::make_image_gen(&Settings::load(Settings::default_path().as_deref()))?;
            let made = ws.generate_moodboard(gen.as_ref(), &direction, n.clamp(1, 4))?;
            for m in &made {
                println!("added {m}");
            }
            println!("mood board: {} image(s); plan shows them to Claude", ws.defn.brief.images.len());
        }
        Cmd::Storyboard { shots, direction, force } => {
            let mut ws = ws(file)?;
            let gen = connect::make_image_gen(&Settings::load(Settings::default_path().as_deref()))?;
            for o in ws.generate_storyboard(gen.as_ref(), &shots, &direction, force)? {
                let rel = o.file.strip_prefix(&ws.root).unwrap_or(&o.file).display().to_string();
                println!("{:<14} {:<5} {rel}", o.shot_id, o.action);
            }
            println!("storyboard frames are a preview: the cut uses only takes rendered from the game");
        }
        Cmd::Discover { request, answers } => {
            let mut ws = ws(file)?;
            let settings = Settings::load(Settings::default_path().as_deref());
            let claude = connect::make_claude(&settings)?;
            let d = shotpilot::ai::discover(&ws.defn, &request, &answers, true, claude.as_ref())?;
            println!("{}", d.say);
            if d.is_question() {
                println!("\n{}", d.form["title"].as_str().unwrap_or("questions"));
                for q in d.form["questions"].as_array().into_iter().flatten() {
                    let opts: Vec<String> = q["options"].as_array().into_iter().flatten()
                        .map(|o| format!("{} ({})", o["label"].as_str().unwrap_or(""), o["value"].as_str().unwrap_or(""))).collect();
                    println!("- {} [{}] default: {}", q["label"].as_str().unwrap_or(""), q["id"].as_str().unwrap_or(""), q["default"].as_str().unwrap_or(""));
                    if !opts.is_empty() {
                        println!("    {}", opts.join(" / "));
                    }
                }
                println!("\nanswer with: shotpilot discover \"<request>\" --answers \"<id>: <answer>; ...\"");
                return Ok(1);
            }
            ws.apply_discovery(&d, &request, &serde_json::Value::Null)?;
            println!("\ndirection: {}\ndesign:    {}\nskills:    {}\n\n{}", d.direction, dash(std::slice::from_ref(&d.design)), dash(&d.skills), d.plan);
        }
        Cmd::Design { id, fork } => {
            let mut ws = ws(file)?;
            match id {
                None => {
                    for d in shotpilot::designsys::all() {
                        let mark = if d.id == ws.defn.brief.design { "*" } else { " " };
                        println!("{mark} {:<18} {} [{}] {}", d.id, d.name, d.genres.join(", "), d.description);
                    }
                    if shotpilot::designsys::project_doc(&ws.root).is_some() {
                        println!("  (this project has its own {}: it wins for the prose)", shotpilot::designsys::PROJECT_FILE);
                    }
                }
                Some(id) => {
                    let Some(d) = shotpilot::designsys::get(&id) else {
                        shotpilot::bail!("unknown design system '{id}' (shotpilot design lists them)");
                    };
                    ws.set_brief(&[("design".to_string(), serde_json::json!(d.id))])?;
                    // the look's typefaces, title size and motion; nothing re-renders
                    ws.set_finish(&shotpilot::designsys::look_entries(d))?;
                    for missing in shotpilot::fonts::fetch_now(&ws.defn.finish.fonts.specs()) {
                        println!("  font not fetched ({missing}); the cut will use the system font");
                    }
                    if fork {
                        let dst = ws.root.join(shotpilot::designsys::PROJECT_FILE);
                        if dst.exists() {
                            shotpilot::bail!("{} already exists; edit it, or remove it to fork again", dst.display());
                        }
                        std::fs::write(&dst, d.doc)?;
                        println!("copied {} to {} (edit it into your game's brand)", d.id, dst.display());
                    }
                    println!("design: {} ({})", d.id, d.name);
                }
            }
        }
        Cmd::Concept { r#match } => {
            let mut ws = ws(file)?;
            let settings = Settings::load(Settings::default_path().as_deref());
            if r#match {
                let claude = connect::make_claude(&settings)?;
                let m = ws.match_concept(claude.as_ref(), None)?;
                println!("rounds: {}; places added: {}; frames with a place: {}; still open: {}", m.rounds, dash(&m.added), m.found, dash(&m.open));
            } else {
                let gen = connect::make_image_gen(&settings)?;
                let made = ws.generate_concept(gen.as_ref())?;
                println!("made {} picture(s)", made.len());
            }
            for f in shotpilot::concept::load(&ws.root) {
                let at = f.found.as_ref().map(|p| format!("{}/{} ({})", p.anchor, p.side, p.fit)).unwrap_or_else(|| "-".into());
                println!("  {:<4} {:<10} {:<40} {}", f.id, f.beat, f.file.as_deref().unwrap_or("(no picture)"), at);
            }
        }
        Cmd::Reference { video, measure_only, use_id, compare } => {
            let mut ws = ws(file)?;
            let mut told = false;
            if let Some(v) = &video {
                let (r, notes) = ws.add_reference(std::path::Path::new(v), !measure_only)?;
                println!("measured {} -> reference/{}/ (facts.json, contact.jpg, proxy.mp4)", r.source, r.id);
                for (k, v) in r.measure.rows() {
                    println!("  {k:<14} {v}");
                }
                print_borrowed(&notes, &r.id);
                told = true;
            } else if let Some(id) = &use_id {
                let notes = ws.use_reference(id)?;
                print_borrowed(&notes, id);
                told = true;
            }
            if compare {
                let c = ws.compare_reference(use_id.as_deref().filter(|_| video.is_none()))?;
                println!("{:<14} {:<44} cut", "", format!("reference `{}`", c.reference.id));
                for ((k, a), (_, b)) in c.reference.measure.rows().into_iter().zip(c.cut.rows()) {
                    println!("{k:<14} {a:<44} {b}");
                }
                for p in &c.pairs {
                    println!("  {}  {:<9} reference {}s / cut {}s", p.file, p.label, shotpilot::model::fmt_g(p.at_ref), shotpilot::model::fmt_g(p.at_cut));
                }
                println!("pairs: {}", c.dir.join("sheet.jpg").display());
                told = true;
            }
            if !told {
                let active = ws.defn.brief.reference_teaser.clone();
                let all = shotpilot::reference::list(&ws.root);
                if all.is_empty() {
                    println!("no reference teaser yet: shotpilot reference <video>");
                }
                for r in all {
                    let mark = if r.id == active { "*" } else { " " };
                    println!("{mark} {:<24} {:<30} {}s, {} cuts", r.id, r.source, shotpilot::model::fmt_g(shotpilot::geometry::round_to(r.measure.secs, 1)), r.measure.cuts.len());
                }
            }
        }
        Cmd::Skills => {
            let ws = ws(file)?;
            let active: Vec<&str> = shotpilot::skills::active(&ws.defn.brief).iter().map(|s| s.id).collect();
            for s in shotpilot::skills::all() {
                let mark = if active.contains(&s.id) { "*" } else { " " };
                println!("{mark} {:<17} {:<8} {}", s.id, s.mode, s.description);
            }
        }
        Cmd::Brief { assignments } => {
            let mut ws = ws(file)?;
            if !assignments.is_empty() {
                ws.set_brief(&parse_assignments(&assignments)?)?;
            }
            let b = &ws.defn.brief;
            println!("kind:       {}", b.kind);
            println!("genre:      {}", b.genre);
            println!("audience:   {}", b.audience);
            println!("core:       {}", b.core);
            println!("must_show:  {}", b.must_show.join(", "));
            println!("references: {}", b.references.join(", "));
            if !b.notes.is_empty() {
                println!("notes:      {}", b.notes);
            }
            let missing = b.missing();
            if !missing.is_empty() {
                println!("missing:    {} (the planner works better with these)", missing.join(", "));
            }
        }
        Cmd::Finish { assignments } => {
            let mut ws = ws(file)?;
            if !assignments.is_empty() {
                ws.set_finish(&parse_assignments(&assignments)?)?;
            }
            let f = &ws.defn.finish;
            println!("grade:      {}", f.grade);
            println!("letterbox:  {}", f.letterbox.map(fmt_g).unwrap_or_else(|| "-".into()));
            println!("typeface:   {} (the system font when a face is missing)", f.typeface);
            for (role, spec) in
                [("display", &f.fonts.display), ("body", &f.fonts.body), ("latin", &f.fonts.latin), ("title", &f.fonts.title)]
            {
                let line = match spec {
                    Some(s) if shotpilot::fonts::local(s).is_some() => format!("{} (downloaded)", s.label()),
                    Some(s) => format!("{} (downloaded at the next cut)", s.label()),
                    None => "-".into(),
                };
                println!("{:<11} {line}", format!("{role}:"));
            }
            println!("title_size: {}", f.title_size.map(fmt_g).unwrap_or_else(|| format!("{} (default)", fmt_g(shotpilot::finish::TITLE_SIZE))));
            println!("logo:       {}", f.logo.as_deref().unwrap_or("-"));
            let m = shotpilot::motion::resolve(&f.motion);
            let avoid = if m.avoid.is_empty() { String::new() } else { format!(", never {}", m.avoid.join(" ")) };
            println!("motion:     {} ({}{avoid}); card {} / title {} / dip {}s / accent {}",
                     m.profile, m.energy, m.card.enter, m.title.enter, fmt_g(shotpilot::geometry::round_to(m.dip, 2)), m.accent);
        }
        Cmd::Set { shot, assignments } => {
            let parsed = parse_assignments(&assignments)?;
            let mut ws = ws(file)?;
            let sid = ws.select_one(&shot)?.id;
            print_edit_report(&ws.set_fields(&sid, &parsed)?);
        }
        Cmd::Order { ids, move_, to } => {
            let mut ws = ws(file)?;
            let all = ws.defn.shot_ids();
            let new: Vec<String> = if let Some(m) = move_ {
                let sid = ws.select_one(&m)?.id;
                let mut rest: Vec<String> = all.iter().filter(|i| **i != sid).cloned().collect();
                let pos = to.clamp(1, all.len()) - 1;
                rest.insert(pos, sid);
                rest
            } else {
                let mut new: Vec<String> = ws.select(&ids)?.into_iter().map(|s| s.id).collect();
                new.dedup();
                for i in &all {
                    if !new.contains(i) {
                        new.push(i.clone());
                    }
                }
                new
            };
            print_edit_report(&ws.edit_definition(&[], Some(&new), "order", "")?);
        }
        Cmd::Plan { pitch, duration, shots, note, spec, write, replace } => {
            let mut ws = ws(file)?;
            if spec {
                let target = duration.unwrap_or(ws.defn.teaser.target_duration);
                let slots = shotpilot::craft::skeleton(&ws.defn.brief, target, shots);
                println!("{}", shotpilot::craft::plan_spec(&ws.defn, &slots));
                return Ok(0);
            }
            let claude = connect::make_claude(&Settings::load(Settings::default_path().as_deref()))?;
            let p = ai::plan(&ws.defn, pitch.as_deref(), duration, shots, note.as_deref(), claude.as_ref())?;
            let planned = p.shots;
            let total: f64 = planned.iter().filter_map(|s| s["duration"].as_f64()).sum();
            println!("# {} shots, {}s\n# {}\n", planned.len(), fmt_g(total), p.rationale);
            println!("# references: {}", p.references.join(", "));
            for a in &p.adjustments {
                println!("# adjusted by code: {a}");
            }
            if !p.candidates.is_empty() {
                let list: Vec<String> = p.candidates.iter().map(|c| c.map_or("failed".into(), fmt_g)).collect();
                println!("# picked by code score from {} plans: {}", list.len(), list.join(" / "));
            }
            print!("{}", yamlio::dumps(&json!({"script": p.script, "shots": planned})));
            if write {
                if !ws.defn.shots.is_empty() && !replace {
                    bail!("shots.yaml already has shots; use --replace to overwrite them");
                }
                ws.replace_plan(&planned, Some(&p.script))?;
                ws.set_finish(&p.finish)?;
                println!("wrote {} shots to {}", planned.len(), ws.path.display());
            } else {
                println!("(preview only; add --write to put these in shots.yaml)");
            }
        }
        Cmd::Eval { cases, models, runs, candidates, out, replay } => {
            if let Some(dir) = replay {
                let report = shotpilot::eval::replay(&dir)?;
                print!("{}", report.to_markdown("Replay"));
                return Ok(0);
            }
            let cases = eval_cases(&cases)?;
            if let Some(n) = candidates {
                // read by ai::plan on this (single) thread
                std::env::set_var("SHOTPILOT_CANDIDATES", n.to_string());
            }
            let out = out.unwrap_or_else(|| PathBuf::from("eval").join("runs").join(chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()));
            let settings = Settings::load(Settings::default_path().as_deref());
            let make = |_model: &str| -> Box<dyn ai::Claude> {
                match connect::make_claude(&settings) {
                    Ok(c) => c,
                    Err(e) => Box::new(FailingClaude(e.msg)),
                }
            };
            let report = shotpilot::eval::run(&cases, &models, runs, &out, &make)?;
            print!("{}", report.to_markdown("Eval summary"));
            println!("results: {}", out.display());
        }
        Cmd::Revise { note, shot, aspect, yes } => {
            let mut ws = ws(file)?;
            let scope: Option<Vec<String>> = if !shot.is_empty() {
                Some(ws.select(&shot)?.into_iter().map(|s| s.id).collect())
            } else if aspect.is_some() {
                // one part of every shot (the app's "cameras only"): the script stays
                Some(ws.defn.shot_ids())
            } else {
                None
            };
            let claude = connect::make_claude(&Settings::load(Settings::default_path().as_deref()))?;
            let rev = match &aspect {
                Some(a) => {
                    let ids = scope.as_deref().unwrap_or_default();
                    let on = if ids.len() == 1 { format!("shot {}", ids[0]) } else { format!("all {} shots", ids.len()) };
                    let mut pointer = ai::Pointer {
                        fields: ai::aspect_fields(a),
                        context: Some(format!("{on}; the director points at this part of the shot: {a}")),
                        ..Default::default()
                    };
                    if a == "finish" {
                        // as the app: the whole teaser's finish gets every line and the grade, a shot its own lines
                        let whole = shot.is_empty();
                        pointer.texts = Some(ws.defn.script.iter()
                            .filter(|l| whole || l.on.as_ref().is_some_and(|o| ids.contains(o)))
                            .map(|l| l.id.clone()).collect());
                        pointer.finish = whole;
                    }
                    ai::revise_at(&ws.defn, &note, scope.as_deref(), &pointer, claude.as_ref())?
                }
                None => ai::revise(&ws.defn, &note, scope.as_deref(), claude.as_ref())?,
            };
            if rev.edits.is_empty() && rev.order.is_none() && rev.audio.is_none() && rev.texts.is_empty() && rev.finish.is_empty() {
                println!("no change: {}", rev.summary);
                return Ok(0);
            }
            println!("# {}", rev.summary);
            for (sid, node) in &rev.edits {
                println!("\n## {sid}\n- before:\n{}- after:\n{}", yamlio::dumps(ws.defn.find(sid)?), yamlio::dumps(node));
            }
            if let Some(o) = &rev.order {
                println!("order: {}", o.join(" > "));
            }
            if let Some(a) = &rev.audio {
                println!("\n## audio\n- before:\n{}- after:\n{}", yamlio::dumps(&Value::Object(ws.defn.audio.clone())),
                         yamlio::dumps(&Value::Object(a.clone())));
            }
            for (id, set) in &rev.texts {
                let fields: Vec<String> = set.iter().map(|(k, v)| format!("{k}={v}")).collect();
                println!("\n## script {id}: {}", fields.join(", "));
            }
            if !rev.finish.is_empty() {
                let fields: Vec<String> = rev.finish.iter().map(|(k, v)| format!("{k}={v}")).collect();
                println!("\n## finish: {}", fields.join(", "));
            }
            if !yes {
                if !std::io::stdin().is_terminal() {
                    println!("(not applied; pass --yes to apply non-interactively)");
                    return Ok(0);
                }
                print!("apply? [y/N] ");
                std::io::Write::flush(&mut std::io::stdout())?;
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer)?;
                if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
                    println!("not applied");
                    return Ok(0);
                }
            }
            print_edit_report(&ws.apply_revision(&rev, "revise", &note)?);
        }
        Cmd::Sound { assignments, json, design, made, make } => {
            if let Some(words) = made {
                let bank = shotpilot::forge::bank();
                let found = bank.search(&words, usize::MAX);
                if json {
                    let rows: Vec<Value> = found.iter().map(|m| json!({"id": format!("{}{}", shotpilot::forge::PREFIX, m.id),
                        "dur_s": m.dur_s, "kinds": m.kinds, "dur": m.dur_knob, "pitch": m.pitch.is_some(), "note": m.note})).collect();
                    println!("{}", serde_json::to_string_pretty(&rows)?);
                } else {
                    println!("{} of {} sounds shotpilot can make ({})", found.len(), bank.sounds.len(), bank.version);
                    for m in found {
                        println!("  {:<28} {:>5.1}s  {:<22} {}", m.id, m.dur_s, m.kinds.join(","), m.note);
                    }
                }
                return Ok(0);
            }
            let mut ws = ws(file)?;
            if let Some(r) = make {
                let bank = shotpilot::forge::bank();
                let r = if r.starts_with(shotpilot::forge::PREFIX) { r } else { format!("{}{r}", shotpilot::forge::PREFIX) };
                let p = bank.make(&r, &ws.root)?;
                for n in &p.notes {
                    println!("note: {n}");
                }
                println!("{}", p.file);
                println!("its moment is {:.2}s into the file: to land it at the start of s03, `shotpilot set s03 'sfx=[{{file: {}, at: {}}}]'`",
                         p.moment_s, p.file, fmt_g(shotpilot::geometry::round_to(-p.moment_s.min(sound::LEAD_MAX), 2)));
                return Ok(0);
            }
            if design {
                let lib = shotpilot::library::Library::open(shotpilot::library::default_root().as_deref())?;
                let claude = connect::make_claude(&Settings::load(Settings::default_path().as_deref()))?;
                let d = shotpilot::sounddesign::design(&ws, &lib, claude.as_ref())?;
                shotpilot::sounddesign::apply(&mut ws, &d)?;
                if !json {
                    println!("{}\n", d.rationale);
                    for n in &d.notes {
                        println!("note: {n}");
                    }
                }
            }
            if !assignments.is_empty() {
                let rep = ws.set_audio(&parse_assignments(&assignments)?)?;
                if !json {
                    print_edit_report(&rep);
                }
            }
            let Some(m) = ws.mix()? else {
                if json {
                    println!("null");
                } else {
                    println!("no sound yet. put files in {}/ and set audio.bgm or a shot's sfx, e.g.", ws.root.join("audio").display());
                    println!("  shotpilot sound bgm.file=audio/bgm.wav bgm.gain=-6");
                    println!("  shotpilot set s03 'sfx=[{{file: audio/hit.wav, at: 0}}]'");
                }
                return Ok(0);
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&m.to_json())?);
                return Ok(0);
            }
            println!("mix: {} ({}s{})", m.wav.display(), fmt_g(m.total), if m.reused { ", cached" } else { "" });
            for l in &m.lanes {
                let owner = l["shot"].as_str().map(|s| format!(" @ {s}")).unwrap_or_default();
                println!("  {:<4} {:<14} {:>7}s  {:>+5} dB  {}{owner}", l["role"].as_str().unwrap_or(""),
                         l["label"].as_str().unwrap_or(""), soundcheck::fmt_t(l["from"].as_f64().unwrap_or(0.0)),
                         l["gain"].as_f64().unwrap_or(0.0), l["file"].as_str().unwrap_or(""));
            }
            for o in sound::orphans(&ws.defn, &ws.resolved()?) {
                println!("warning: {}", o.text());
            }
            println!("{}", soundcheck::text(&m.check));
        }
        Cmd::Hook { event } => {
            // a hook reports facts and never stands in the way: any failure is silent (exit 0),
            // since Claude Code shows a non-zero exit to the developer or feeds it to Claude
            let _ = run_hook(file, &event);
        }
        Cmd::Claude { action: _, dir, force, print } => {
            let ws = ws(file)?;
            let root = match dir {
                Some(d) if d.is_dir() => std::path::absolute(&d)?,
                Some(d) => bail!("{} is not a folder", d.display()),
                None => shotpilot::claudecode::game_root(&ws),
            };
            if print {
                let hooks = shotpilot::claudecode::hooks_json(&shotpilot::claudecode::shots_arg(&ws, &root));
                println!("{}", serde_json::to_string_pretty(&json!({ "hooks": hooks }))?);
                return Ok(0);
            }
            let rep = shotpilot::claudecode::install(&ws, &root, force)?;
            println!("game folder: {}", rep.game_root.display());
            println!("skill:    {} ({})", rep.skill.display(), if rep.skill_written { "written" } else { "kept; --force to overwrite" });
            println!("hooks:    {} ({})", rep.settings.display(),
                     if rep.hooks_added.is_empty() { "already there".to_string() } else { format!("added {}", rep.hooks_added.join(", ")) });
            println!("Claude Code now hears which teaser cuts went out of date when a session starts and after each tool call.");
            println!("The hooks run `shotpilot`: keep it on PATH (cargo install --path crates/shotpilot).");
        }
        Cmd::Takes { shot } => {
            let ws = ws(file)?;
            let s = ws.select_one(&shot)?;
            for q in fp::quality_names(&ws.defn) {
                let cur = fp::render_fingerprint(&ws.defn, &s, &q)?;
                for t in ws.state.list_takes(&s.id, &q) {
                    let flag = if t.fingerprint() == cur { "*" } else { " " };
                    let fp12 = &t.fingerprint()[..12.min(t.fingerprint().len())];
                    println!(" {flag} {q:<8} {fp12}  {}  {} frames  {}s  {}", t.created(), t.files().len(),
                             t.info.get("seconds").cloned().unwrap_or(json!(0)), t.dir.display());
                }
            }
            println!("(* = matches the current definition)");
        }
        Cmd::Log { n } => {
            let ws = ws(file)?;
            let h = ws.state.history();
            for e in &h[h.len().saturating_sub(n)..] {
                let detail: serde_json::Map<String, Value> = e
                    .as_object()
                    .map(|m| {
                        m.iter()
                            .filter(|(k, v)| {
                                *k != "at" && *k != "action" && !v.is_null() && **v != json!([]) && **v != json!("")
                                    && **v != json!(false)
                            })
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .unwrap_or_default();
                println!("{}  {:<8} {}", e["at"].as_str().unwrap_or(""), e["action"].as_str().unwrap_or(""), Value::Object(detail));
            }
        }
        Cmd::Gc { keep, dry_run } => {
            let mut ws = ws(file)?;
            let removed = ws.gc(keep, dry_run)?;
            println!("{} {} old take(s)", if dry_run { "would remove" } else { "removed" }, removed.len());
            for p in removed {
                println!("  {}", p.display());
            }
        }
    }
    Ok(0)
}
