//! shotpilot command line.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use shotpilot::ai::{self, HttpClaude};
use shotpilot::error::Result;
use shotpilot::fingerprint as fp;
use shotpilot::model::{self, fmt_g};
use shotpilot::workspace::EditReport;
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
        #[arg(long, default_value = "mock", value_parser = ["mock", "ue5"])]
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
        #[arg(long)]
        write: bool,
        #[arg(long)]
        replace: bool,
    },
    /// apply a note with Claude: revise "3番目はもっと寄って"
    Revise {
        note: String,
        /// limit edits to this shot (repeatable)
        #[arg(long)]
        shot: Vec<String>,
        #[arg(long)]
        yes: bool,
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("shotpilot: {e}");
            ExitCode::from(2)
        }
    }
}

fn ws(cli_file: &PathBuf) -> Result<Workspace> {
    Workspace::open(cli_file)
}

fn print_edit_report(rep: &EditReport) {
    if !rep.order.is_empty() {
        println!("new order: {}", rep.order.join(" > "));
    }
    println!("edited: {}", if rep.edited.is_empty() { "-".into() } else { rep.edited.join(", ") });
    println!("now stale (need re-render): {}", if rep.stale.is_empty() { "nothing".into() } else { rep.stale.join(", ") });
}

fn dash(v: &[String]) -> String {
    if v.is_empty() { "-".into() } else { v.join(", ") }
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
            let tpl = if empty { templates::EMPTY } else { templates::DEMO };
            std::fs::write(file, templates::render(tpl, &name, &engine, &uproject, &map))?;
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
            println!("{:>2}  {:<idw$}  {:<9}  {:>5}  {:<6}  {}", "#", "shot", "beat", "sec", "engine", qh.join("  "));
            for r in &rows {
                let marks: Vec<String> = qs
                    .iter()
                    .map(|q| {
                        let m = match r.quality(q) {
                            "fresh" => "fresh",
                            "stale" => "STALE",
                            _ => "-",
                        };
                        format!("{m:<7}")
                    })
                    .collect();
                println!("{:>2}  {:<idw$}  {:<9}  {:>5}  {:<6}  {}", r.index + 1, r.id, r.beat.as_deref().unwrap_or("-"),
                         fmt_g(r.duration), r.build, marks.join("  "));
            }
            let total: f64 = rows.iter().map(|r| r.duration).sum();
            let target = ws.defn.teaser.target_duration;
            let diff = total - target;
            println!("\ntotal {}s / target {}s ({}{}s)", fmt_g(total), fmt_g(target), if diff >= 0.0 { "+" } else { "" }, fmt_g(diff));
            println!("engine: synced = sequence asset matches shots.yaml, dirty = needs build, new = never built");
            println!("takes:  fresh = take exists for the current definition, STALE = only older takes exist");
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
            let outcomes = ws.render(&targets, &quality, force, None)?;
            for o in &outcomes {
                let extra = match o.action.as_str() {
                    "rendered" => format!("{:.1}s", o.seconds),
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
        }
        Cmd::Set { shot, assignments } => {
            let mut parsed = Vec::new();
            for a in &assignments {
                let Some((k, v)) = a.split_once('=') else { bail!("expected key=value, got '{a}'") };
                let value = if v.trim().is_empty() {
                    Value::Null
                } else {
                    yamlio::parse_str(v, &format!("value of {k}"))?
                };
                parsed.push((k.trim().to_string(), value));
            }
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
        Cmd::Plan { pitch, duration, shots, write, replace } => {
            let mut ws = ws(file)?;
            let claude = HttpClaude::from_env()?;
            let (planned, rationale) = ai::plan(&ws.defn, pitch.as_deref(), duration, shots, &claude)?;
            let total: f64 = planned.iter().filter_map(|s| s["duration"].as_f64()).sum();
            println!("# {} shots, {}s\n# {rationale}\n", planned.len(), fmt_g(total));
            print!("{}", yamlio::dumps(&json!({"shots": planned})));
            if write {
                if !ws.defn.shots.is_empty() && !replace {
                    bail!("shots.yaml already has shots; use --replace to overwrite them");
                }
                ws.replace_all_shots(&planned)?;
                println!("wrote {} shots to {}", planned.len(), ws.path.display());
            } else {
                println!("(preview only; add --write to put these in shots.yaml)");
            }
        }
        Cmd::Revise { note, shot, yes } => {
            let mut ws = ws(file)?;
            let scope: Option<Vec<String>> = if shot.is_empty() {
                None
            } else {
                Some(ws.select(&shot)?.into_iter().map(|s| s.id).collect())
            };
            let claude = HttpClaude::from_env()?;
            let rev = ai::revise(&ws.defn, &note, scope.as_deref(), &claude)?;
            if rev.edits.is_empty() && rev.order.is_none() {
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
            print_edit_report(&ws.edit_definition(&rev.edits, rev.order.as_deref(), "revise", &note)?);
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
