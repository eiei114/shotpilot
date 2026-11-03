//! Shotpilot desktop app.
//!
//! One native window over the same loop as the CLI: the core library serves the UI on
//! a private loopback port inside this process, and the window shows it. No browser,
//! no Python, no sidecar process. shots.yaml stays the source of truth, so the CLI and
//! a text editor can edit the same project while the window is open.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use shotpilot::server::Server;
use shotpilot::templates;
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn main() {
    let Some(shots) = choose_project() else { return };
    remember(&shots);
    let server = match Server::bind(&shots, "127.0.0.1", 0) {
        Ok(s) => s,
        Err(e) => return fail(&e.msg),
    };
    let url = server.url();
    let title = match shotpilot::model::load(&shots) {
        Ok(d) => format!("Shotpilot — {}", d.project.name),
        Err(_) => "Shotpilot".to_string(),
    };
    server.spawn();

    let result = tauri::Builder::default()
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                .title(title)
                .inner_size(1280.0, 820.0)
                .min_inner_size(720.0, 480.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(e) = result {
        fail(&format!("could not open the window: {e}"));
    }
}

/// The project to open: a path argument (shots.yaml or its folder), else shots.yaml in
/// the current folder, else a folder picker. Offers to create the sample project when
/// the chosen folder has no shots.yaml yet.
fn choose_project() -> Option<PathBuf> {
    if let Some(arg) = std::env::args_os().nth(1) {
        let p = PathBuf::from(arg);
        let p = if p.is_dir() { p.join("shots.yaml") } else { p };
        if p.is_file() {
            return Some(p);
        }
        return offer_sample(p.parent().unwrap_or(Path::new(".")));
    }
    let here = PathBuf::from("shots.yaml");
    if here.is_file() {
        return std::path::absolute(here).ok();
    }
    let mut dialog = rfd::FileDialog::new().set_title("Shotpilot: shots.yaml のあるフォルダを選ぶ");
    if let Some(last) = last_project().and_then(|p| p.parent().and_then(Path::parent).map(Path::to_path_buf)) {
        dialog = dialog.set_directory(last);
    }
    let dir = dialog.pick_folder()?;
    let p = dir.join("shots.yaml");
    if p.is_file() {
        Some(p)
    } else {
        offer_sample(&dir)
    }
}

fn offer_sample(dir: &Path) -> Option<PathBuf> {
    let answer = MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title("Shotpilot")
        .set_description(format!(
            "{} に shots.yaml がありません。\nサンプル（UE なしで動く mock エンジン、6ショット）を作りますか？",
            dir.display()
        ))
        .set_buttons(MessageButtons::YesNo)
        .show();
    if answer != MessageDialogResult::Yes {
        return None;
    }
    let name = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "shotpilot".into());
    let p = dir.join("shots.yaml");
    let text = templates::render(templates::DEMO, &name, "mock", "null", "null");
    if let Err(e) = std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&p, text)) {
        fail(&format!("could not write {}: {e}", p.display()));
        return None;
    }
    // placeholder BGM and effects the demo refers to; the demo still opens without them
    let _ = shotpilot::sound::write_demo_audio(dir);
    let gi = dir.join(".gitignore");
    if !gi.exists() {
        let _ = std::fs::write(gi, templates::GITIGNORE);
    }
    Some(p)
}

fn config_file() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("XDG_CONFIG_HOME")).map(PathBuf::from)?;
    Some(base.join("shotpilot").join("last_project.txt"))
}

fn last_project() -> Option<PathBuf> {
    std::fs::read_to_string(config_file()?).ok().map(|s| PathBuf::from(s.trim()))
}

fn remember(shots: &Path) {
    if let (Some(f), Ok(abs)) = (config_file(), std::path::absolute(shots)) {
        let _ = f.parent().map(std::fs::create_dir_all);
        let _ = std::fs::write(f, abs.display().to_string());
    }
}

fn fail(msg: &str) {
    MessageDialog::new().set_level(MessageLevel::Error).set_title("Shotpilot").set_description(msg).show();
}
