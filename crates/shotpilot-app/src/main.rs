//! Shotpilot desktop app.
//!
//! One native window over the same loop as the CLI: the core library serves the UI on
//! a private loopback port inside this process, and the window shows it. No browser,
//! no Python, no sidecar process. shots.yaml stays the source of truth, so the CLI and
//! a text editor can edit the same project while the window is open.
//!
//! The window starts on the home (recent projects, a new project from a prompt), or on
//! a project when one is given: a path argument, or shots.yaml in the current folder.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use rfd::{MessageDialog, MessageLevel};
use shotpilot::server::{Pick, Server};
use tauri::{WebviewUrl, WebviewWindow, WebviewWindowBuilder};

fn main() {
    // the look's typefaces are fetched from Google Fonts the first time a cut needs them
    shotpilot::fonts::allow_fetch(true);
    let window: Arc<OnceLock<WebviewWindow>> = Arc::new(OnceLock::new());
    let server = match Server::bind_home("127.0.0.1", 0) {
        Ok(s) => s.with_picker(picker(Arc::clone(&window))),
        Err(e) => return fail(&e.msg),
    };
    let mut url = format!("{}home", server.url());
    if let Some(p) = start_project() {
        match server.open(&p) {
            Ok(_) => url = server.url(),
            Err(e) => fail(&e.msg), // the home still opens, with the project list
        }
    }
    server.spawn();

    let result = tauri::Builder::default()
        .setup(move |app| {
            let w = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                .title("Shotpilot")
                .inner_size(1280.0, 820.0)
                .min_inner_size(720.0, 480.0)
                // the pages name the open project in their title; the window follows
                .on_document_title_changed(|w, title| {
                    let _ = w.set_title(if title.trim().is_empty() { "Shotpilot" } else { title.as_str() });
                })
                .build()?;
            let _ = window.set(w);
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(e) = result {
        fail(&format!("could not open the window: {e}"));
    }
}

/// A project to open at start: a path argument (shots.yaml or its folder), else
/// shots.yaml in the current folder. None starts on the home.
fn start_project() -> Option<PathBuf> {
    if let Some(arg) = std::env::args_os().nth(1) {
        return Some(PathBuf::from(arg));
    }
    let here = PathBuf::from("shots.yaml");
    here.is_file().then(|| std::path::absolute(here).ok()).flatten()
}

/// Native file dialogs for the home, parented to the window so they open on top.
fn picker(window: Arc<OnceLock<WebviewWindow>>) -> shotpilot::server::Picker {
    Box::new(move |pick| {
        let mut d = rfd::FileDialog::new();
        if let Some(w) = window.get() {
            d = d.set_parent(w);
        }
        match pick {
            Pick::Folder { title, start } => {
                d = d.set_title(title);
                if let Some(s) = start {
                    d = d.set_directory(s);
                }
                d.pick_folder()
            }
            Pick::Uproject { start } => {
                d = d.set_title("Shotpilot: UE プロジェクト（.uproject）を選ぶ").add_filter("Unreal Engine project", &["uproject"]);
                if let Some(s) = start {
                    d = d.set_directory(s);
                }
                d.pick_file()
            }
            Pick::Image { start } => {
                d = d.set_title("Shotpilot: ロゴ画像を選ぶ（透明の背景の PNG がよい）").add_filter("画像", &["png", "jpg", "jpeg", "webp"]);
                if let Some(s) = start {
                    d = d.set_directory(s);
                }
                d.pick_file()
            }
            Pick::Video { start } => {
                d = d.set_title("Shotpilot: 参照にするティザーの動画を選ぶ（180 秒まで）")
                    .add_filter("動画", &["mp4", "mov", "m4v", "webm", "mkv"]);
                if let Some(s) = start {
                    d = d.set_directory(s);
                }
                d.pick_file()
            }
        }
    })
}

fn fail(msg: &str) {
    MessageDialog::new().set_level(MessageLevel::Error).set_title("Shotpilot").set_description(msg).show();
}
