//! Assemble the teaser in definition order from existing takes.
//!
//! Nothing is rendered here. Each take is encoded once into a cached segment.mp4 inside
//! its own take folder; the cut is a stream-copy concat of those segments. Reordering
//! shots or swapping one take therefore costs a concat, not a render.

use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

use crate::bail;
use crate::engines::find_on_path;
use crate::error::Result;
use crate::fingerprint as fp;
use crate::model::{fmt_g, num, Definition, ResolvedShot};
use crate::sound::Mixed;
use crate::state::{now_iso, Take};

pub struct Pick {
    pub shot: ResolvedShot,
    pub take: Take,
    pub quality: String,
}

pub fn ffmpeg() -> Option<PathBuf> {
    find_on_path("ffmpeg")
}

/// Keep a console child (ffmpeg, UnrealEditor-Cmd) from opening its own window.
/// The desktop app has no console, so Windows would otherwise pop one up per call.
pub fn no_window(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

pub fn run_ffmpeg(args: &[String]) -> Result<()> {
    let Some(exe) = ffmpeg() else { bail!("ffmpeg not found on PATH") };
    let out = no_window(&mut Command::new(exe)).args(["-hide_banner", "-loglevel", "error", "-y"]).args(args).output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("ffmpeg failed: {}", err.trim().chars().take(2000).collect::<String>());
    }
    Ok(())
}

pub fn segment(take: &Take, w: i64, h: i64, fps: i64) -> Result<PathBuf> {
    let seg = take.dir.join(format!("segment_{w}x{h}_{fps}.mp4"));
    if seg.exists() {
        return Ok(seg);
    }
    let frames = take.frames();
    let files = take.files();
    let total = take.info.get("shot_frames").and_then(Value::as_i64).unwrap_or(1);
    let tfps = take.info.get("fps").and_then(num).unwrap_or(fps as f64);
    let mut lines = Vec::new();
    for (i, (f, name)) in frames.iter().zip(&files).enumerate() {
        let next = frames.get(i + 1).copied().unwrap_or(total);
        lines.push(format!("file '{name}'"));
        lines.push(format!("duration {:.6}", (next - f) as f64 / tfps));
    }
    if let Some(last) = files.last() {
        lines.push(format!("file '{last}'")); // concat demuxer needs the last file twice for its duration
    }
    let lst = take.dir.join("frames.txt");
    std::fs::write(&lst, lines.join("\n") + "\n")?;
    let vf = format!(
        "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,fps={fps},format=yuv420p"
    );
    let tmp = take.dir.join(format!("segment_{w}x{h}_{fps}.tmp.mp4"));
    let args: Vec<String> = [
        "-f", "concat", "-safe", "0", "-i", &lst.display().to_string(), "-vf", &vf, "-frames:v", &total.to_string(),
        "-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-r", &fps.to_string(), &tmp.display().to_string(),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    run_ffmpeg(&args)?;
    std::fs::rename(&tmp, &seg)?;
    Ok(seg)
}

/// Every pick's segment, in pick order. Missing ones are encoded a few at a time: each
/// take has its own folder, and for small frames an encode is mostly ffmpeg starting up.
fn segments(picks: &[Pick], w: i64, h: i64, fps: i64) -> Result<Vec<PathBuf>> {
    let jobs = std::thread::available_parallelism().map(|n| (n.get() / 2).clamp(1, 4)).unwrap_or(2);
    let mut out = Vec::with_capacity(picks.len());
    for batch in picks.chunks(jobs) {
        let made: Vec<Result<PathBuf>> = std::thread::scope(|sc| {
            let handles: Vec<_> = batch.iter().map(|p| sc.spawn(move || segment(&p.take, w, h, fps))).collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_else(|_| Err(crate::error::Error::user("segment encode panicked"))))
                .collect()
        });
        for r in made {
            out.push(r?);
        }
    }
    Ok(out)
}

/// `os.path.relpath` for two absolute paths, with forward slashes.
pub fn relpath(target: &Path, base: &Path) -> String {
    let t: Vec<Component> = target.components().collect();
    let b: Vec<Component> = base.components().collect();
    let common = t.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = std::iter::repeat("..".to_string()).take(b.len() - common).collect();
    parts.extend(t[common..].iter().map(|c| c.as_os_str().to_string_lossy().to_string()));
    if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    }
}

#[derive(Debug)]
pub struct CutInfo {
    pub dir: PathBuf,
    pub seconds: f64,
    pub shots: usize,
    pub video: Option<PathBuf>,
    /// The mix laid under the video (cut/sound/mix.wav), when there is sound.
    pub audio: Option<PathBuf>,
    /// The sound check of that mix.
    pub check: Option<Value>,
    /// Things the user should know (e.g. sound skipped and why).
    pub notes: Vec<String>,
}

/// Assemble the cut. With `mixed`, the video carries the mix as its soundtrack.
pub fn write_cut(defn: &Definition, picks: &[Pick], out_dir: &Path, video: bool, mixed: Option<&Mixed>)
    -> Result<CutInfo> {
    std::fs::create_dir_all(out_dir)?;
    let fps = defn.teaser.fps;
    let (mut w, mut h) = fp::resolution(defn, &fp::quality(defn, "preview")?);
    if picks.iter().all(|p| p.quality == "final") {
        (w, h) = defn.teaser.resolution;
    }

    let mut t = 0i64;
    let mut rows = Vec::new();
    for p in picks {
        let s = &p.shot;
        let files = p.take.files();
        rows.push(json!({
            "order": rows.len() + 1, "id": s.id, "beat": s.beat, "purpose": s.purpose, "notes": s.notes,
            "duration": s.duration, "frames": s.frames, "record_in": t, "record_out": t + s.frames,
            "quality": p.quality, "fingerprint": p.take.fingerprint(),
            "take_dir": relpath(&p.take.dir, out_dir),
            "files": files, "frame_numbers": p.take.frames(),
            "camera": {"start": s.start.to_json(), "end": s.end.to_json(), "curve": s.curve},
            "lens": s.lens, "environment": s.environment, "markers": s.markers,
        }));
        t += s.frames;
    }
    let total_seconds = crate::geometry::round_to(t as f64 / fps as f64, 3);
    let manifest = json!({
        "generated": now_iso(), "project": defn.project.name, "title": defn.teaser.title,
        "fps": fps, "total_frames": t, "total_seconds": total_seconds,
        "target_seconds": defn.teaser.target_duration, "shots": rows,
        "audio": mixed.map(|m| json!({
            "mix": relpath(&m.wav, out_dir), "key": m.key,
            "verdict": m.check["verdict"], "summary": m.check["summary"],
        })),
    });
    std::fs::write(out_dir.join("manifest.json"), serde_json::to_string_pretty(&manifest)?)?;
    std::fs::write(out_dir.join("spec.md"), spec_md(defn, &manifest))?;
    std::fs::write(out_dir.join("index.html"), storyboard_html(defn, &manifest))?;

    let mut info = CutInfo {
        dir: out_dir.to_path_buf(),
        seconds: total_seconds,
        shots: picks.len(),
        video: None,
        audio: mixed.map(|m| m.wav.clone()),
        check: mixed.map(|m| m.check.clone()),
        notes: Vec::new(),
    };
    // finishing (D-25): the cut in order, script cards between shots and lines over them
    let pieces = cut_pieces(defn, picks, fps);
    let finishing = crate::finish::needed(&defn.finish, &pieces);

    if video && ffmpeg().is_some() {
        let segs = segments(picks, w, h, fps)?;
        let lst = out_dir.join("segments.txt");
        let body: String = segs.iter().map(|p| format!("file '{}'\n", p.display().to_string().replace('\\', "/"))).collect();
        std::fs::write(&lst, body)?;
        let out = out_dir.join("cut.mp4");
        if finishing {
            info.seconds = finish_pass(defn, &pieces, &lst, mixed.map(|m| m.wav.as_path()), out_dir, &out, w, h, fps, &mut info.notes)?;
            info.video = Some(out);
            return Ok(info);
        }
        let mut args: Vec<String> =
            ["-f", "concat", "-safe", "0", "-i", &lst.display().to_string()].map(String::from).to_vec();
        match mixed {
            // the picture is stream-copied either way; only the soundtrack is encoded. The mix
            // is already at `audio.loudness` (sound.rs), so it ships exactly as it was checked.
            Some(m) => args.extend(
                ["-i", &m.wav.display().to_string(), "-map", "0:v", "-map", "1:a", "-c:v", "copy",
                 "-c:a", "aac", "-b:a", "192k", "-ar", AUDIO_RATE, "-shortest"]
                    .map(String::from),
            ),
            None => args.extend(["-c", "copy"].map(String::from)),
        }
        args.push(out.display().to_string());
        run_ffmpeg(&args)?;
        info.video = Some(out);
    }
    Ok(info)
}

/// Burn in the finish: concat the segments, then one encode with letterbox, dips, captions,
/// title and end card; the mix (padded with silence over the end card) as the soundtrack.
#[allow(clippy::too_many_arguments)]
fn finish_pass(defn: &Definition, pieces: &[crate::finish::CutPiece], segments: &Path, mix: Option<&Path>, out_dir: &Path,
               out: &Path, w: i64, h: i64, fps: i64, notes: &mut Vec<String>) -> Result<f64> {
    let fonts = crate::finish::faces(&defn.finish, notes);
    if fonts.is_none() {
        // the text is left out (cards stay as black, bars and dips still apply) rather than failing the cut
        notes.push("no font with Japanese glyphs was found; text was left out (set finish.font)".into());
    }
    let mut cfg = defn.finish.clone();
    // the logo is a file next to shots.yaml; without it, the title is set in type
    cfg.logo = match &defn.finish.logo {
        Some(rel) => {
            let p = defn.root().join(rel);
            if p.is_file() {
                Some(p.display().to_string())
            } else {
                notes.push(format!("finish.logo: {} was not found; the title was set in type", p.display()));
                None
            }
        }
        None => None,
    };
    let text_dir = out_dir.join("finish");
    std::fs::create_dir_all(&text_dir)?;
    let (graph, total) = crate::finish::filter_graph(&cfg, pieces, w, h, fps, fonts.as_ref(), &text_dir, notes)?;
    let mut graph = graph;
    let mut args: Vec<String> = ["-f", "concat", "-safe", "0", "-i", &segments.display().to_string()].map(String::from).to_vec();
    if let Some(m) = mix {
        args.extend(["-i".to_string(), m.display().to_string()]);
        graph.push_str(";[1:a]apad[a]"); // already normalized (sound.rs); silence over the end card
    }
    args.extend(["-filter_complex".to_string(), graph, "-map".to_string(), "[v]".to_string()]);
    if mix.is_some() {
        args.extend(["-map", "[a]", "-c:a", "aac", "-b:a", "192k", "-ar", AUDIO_RATE].map(String::from));
    }
    args.extend(["-c:v", "libx264", "-preset", "veryfast", "-crf", "18", "-r", &fps.to_string(), "-t", &format!("{total:.3}")]
        .map(String::from));
    args.push(out.display().to_string());
    run_ffmpeg(&args)?;
    if let Some((_, d)) = &defn.finish.end_card {
        notes.push(format!("end card adds {}s after the last shot", fmt_g(*d)));
    }
    Ok(crate::geometry::round_to(total, 3))
}

/// Sample rate of the cut's soundtrack (the mix's own rate; said out loud so no filter or
/// encoder default picks another).
const AUDIO_RATE: &str = "48000";

/// The cut as finishing sees it: shots (with their place in the plain concatenation) and
/// script slots, in cut order (script.rs layout).
pub fn cut_pieces(defn: &Definition, picks: &[Pick], fps: i64) -> Vec<crate::finish::CutPiece> {
    use crate::finish::{CutPiece, ShotPiece};
    let shots: Vec<ResolvedShot> = picks.iter().map(|p| p.shot.clone()).collect();
    let mut concat = Vec::with_capacity(shots.len());
    let mut t = 0i64;
    for s in &shots {
        concat.push(t as f64 / fps as f64);
        t += s.frames;
    }
    crate::script::layout(&defn.script, &shots, fps)
        .into_iter()
        .map(|it| match it.piece {
            crate::script::Piece::Shot(k) => {
                let s = &shots[k];
                let raw = defn.shots.get(s.index).cloned().unwrap_or(Value::Null);
                CutPiece::Shot(ShotPiece {
                    concat_start: concat[k],
                    duration: s.frames as f64 / fps as f64,
                    beat: s.beat.clone(),
                    caption: crate::finish::caption(&raw),
                    transition: crate::finish::transition(&raw),
                    exposure: crate::finish::exposure(&raw),
                    fx: crate::fx::of(&raw),
                    overlays: crate::script::overlays(&defn.script, &s.id).into_iter().cloned().collect(),
                })
            }
            crate::script::Piece::Slot(i) => CutPiece::Slot(defn.script[i].clone(), it.frames as f64 / fps as f64),
        })
        .collect()
}

fn tc(frame: i64, fps: i64) -> String {
    let (s, f) = (frame / fps, frame % fps);
    format!("{:02}:{:02}:{:02}", s / 60, s % 60, f)
}

fn s(v: &Value) -> String {
    match v {
        Value::String(x) => x.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn spec_md(defn: &Definition, m: &Value) -> String {
    let fps = m["fps"].as_i64().unwrap_or(30);
    let shots = m["shots"].as_array().cloned().unwrap_or_default();
    let mut lines = vec![
        format!("# {} — shot spec", if defn.teaser.title.is_empty() { &defn.project.name } else { &defn.teaser.title }),
        String::new(),
        format!("- generated: {}", s(&m["generated"])),
        format!("- runtime: {}s (target {}s), {fps} fps, {} shots", m["total_seconds"], m["target_seconds"], shots.len()),
        format!("- map: {}", defn.project.map.as_deref().unwrap_or("-")),
        String::new(),
        "| # | shot | beat | in | out | sec | lens | env | take | purpose |".into(),
        "|---|---|---|---|---|---|---|---|---|---|".into(),
    ];
    for r in &shots {
        let env = r["environment"].get("time_of_day").map(s).unwrap_or_else(|| "-".into());
        let fp12: String = s(&r["fingerprint"]).chars().take(8).collect();
        lines.push(format!(
            "| {} | `{}` | {} | {} | {} | {} | {}mm f/{} | {} | {}:{} | {} |",
            r["order"], s(&r["id"]), if r["beat"].is_null() { "-".into() } else { s(&r["beat"]) },
            tc(r["record_in"].as_i64().unwrap_or(0), fps), tc(r["record_out"].as_i64().unwrap_or(0), fps),
            fmt_g(num(&r["duration"]).unwrap_or(0.0)), fmt_g(num(&r["lens"]["focal_length"]).unwrap_or(0.0)),
            fmt_g(num(&r["lens"]["aperture"]).unwrap_or(0.0)), env, s(&r["quality"]), fp12, s(&r["purpose"]),
        ));
    }
    lines.push(String::new());
    for r in &shots {
        let c = &r["camera"];
        lines.push(format!("## {}. {}", r["order"], s(&r["id"])));
        lines.push(String::new());
        lines.push(format!("- beat: {}", if r["beat"].is_null() { "-".into() } else { s(&r["beat"]) }));
        lines.push(format!("- purpose: {}", if s(&r["purpose"]).is_empty() { "-".into() } else { s(&r["purpose"]) }));
        lines.push(format!("- camera start: loc {} rot {}", c["start"]["location"], c["start"]["rotation"]));
        lines.push(format!("- camera end:   loc {} rot {} ({})", c["end"]["location"], c["end"]["rotation"], s(&c["curve"])));
        lines.push(format!("- lens: {}", r["lens"]));
        lines.push(format!("- environment: {}", r["environment"]));
        lines.push(format!("- frames: {}/ ({} files)", s(&r["take_dir"]), r["files"].as_array().map_or(0, Vec::len)));
        for mk in r["markers"].as_array().into_iter().flatten() {
            lines.push(format!("- marker @{}: {}", mk["frame"], s(&mk["label"])));
        }
        if !s(&r["notes"]).is_empty() {
            lines.push(format!("- notes: {}", s(&r["notes"])));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#x27;")
}

fn storyboard_html(defn: &Definition, m: &Value) -> String {
    let fps = m["fps"].as_i64().unwrap_or(30);
    let shots = m["shots"].as_array().cloned().unwrap_or_default();
    let mut cards = String::new();
    for r in &shots {
        let files: Vec<String> = r["files"].as_array().map(|a| a.iter().map(s).collect()).unwrap_or_default();
        let dir = s(&r["take_dir"]);
        let at = |i: usize| format!("{dir}/{}", files.get(i).cloned().unwrap_or_default());
        let (first, mid, last) = (at(0), at(files.len() / 2), at(files.len().saturating_sub(1)));
        let q = s(&r["quality"]);
        cards.push_str(&format!(
            r#"
<article class="card">
  <div class="frames"><img src="{}" alt=""><img class="mid" src="{}" alt=""><img src="{}" alt=""></div>
  <div class="meta">
    <div class="head"><span class="n">{}</span><code>{}</code>
      <span class="beat">{}</span>
      <span class="q q-{}">{}</span></div>
    <p class="purpose">{}</p>
    <p class="facts">{}s · {}mm · {}
      · {}</p>
  </div>
</article>"#,
            esc(&first), esc(&mid), esc(&last), r["order"], esc(&s(&r["id"])), esc(&s(&r["beat"])), esc(&q), esc(&q),
            esc(&s(&r["purpose"])), fmt_g(num(&r["duration"]).unwrap_or(0.0)),
            fmt_g(num(&r["lens"]["focal_length"]).unwrap_or(0.0)),
            esc(&r["environment"].get("time_of_day").map(s).unwrap_or_default()),
            tc(r["record_in"].as_i64().unwrap_or(0), fps),
        ));
    }
    let title = esc(if defn.teaser.title.is_empty() { &defn.project.name } else { &defn.teaser.title });
    let total = num(&m["total_seconds"]).unwrap_or(0.0);
    let target = num(&m["target_seconds"]).unwrap_or(0.0);
    let over = total - target;
    format!(
        r#"<!doctype html>
<html lang="ja"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} storyboard</title>
<style>
:root {{ --bg:#111; --fg:#eee; --mute:#999; --card:#1b1b1b; --line:#2a2a2a; }}
@media (prefers-color-scheme: light) {{ :root {{ --bg:#f6f6f4; --fg:#111; --mute:#666; --card:#fff; --line:#ddd; }} }}
body {{ margin:0; background:var(--bg); color:var(--fg); font:14px/1.5 system-ui, sans-serif; }}
header {{ padding:20px 16px 8px; }} h1 {{ margin:0; font-size:20px; }}
.sub {{ color:var(--mute); }}
main {{ display:grid; gap:12px; padding:16px; grid-template-columns:repeat(auto-fill,minmax(320px,1fr)); }}
.card {{ background:var(--card); border:1px solid var(--line); border-radius:8px; overflow:hidden; }}
.frames {{ display:grid; grid-template-columns:1fr 2fr 1fr; gap:2px; background:#000; }}
.frames img {{ width:100%; aspect-ratio:16/9; object-fit:cover; display:block; }}
.meta {{ padding:8px 10px; }} .head {{ display:flex; gap:8px; align-items:center; }}
.n {{ font-weight:700; }} .beat {{ color:var(--mute); font-size:12px; text-transform:uppercase; }} .purpose {{ margin:4px 0; }} .facts {{ margin:0; color:var(--mute); font-size:12px; }}
.q {{ margin-left:auto; font-size:11px; padding:1px 6px; border-radius:4px; border:1px solid var(--line); }}
.q-final {{ background:#1f6f3f; color:#fff; }} .q-preview {{ background:#2d4f8f; color:#fff; }}
.q-still {{ background:#6b4f9f; color:#fff; }} .q-previz {{ background:#555; color:#fff; }}
</style></head><body>
<header><h1>{title}</h1>
<div class="sub">{}s / target {}s ({}{:.1}s) · {} shots · {fps} fps · {}</div>
</header>
<main>{cards}</main>
</body></html>
"#,
        fmt_g(total), fmt_g(target), if over >= 0.0 { "+" } else { "" }, over, shots.len(), esc(&s(&m["generated"])),
    )
}
