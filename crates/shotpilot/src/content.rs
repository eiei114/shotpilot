//! What a take saw of the game, so that a change to the game re-renders only the shots
//! that showed it.
//!
//! The render fingerprint covers the shot definition (camera, lens, light, props). It does
//! not cover the game itself: a new enemy texture changes the picture of every shot the
//! enemy appears in, and of no other. So the engine reports, per rendered shot, the content
//! files behind what was on screen (meshes, materials, textures, Blueprints), and the take
//! keeps them with their size, time and SHA-256 (`deps` in take.json). A take stays fresh
//! while its definition fingerprint matches and none of those files changed. Put a file
//! back and the old take matches again (takes are never overwritten).
//!
//! Hashing is cheap to repeat: an unchanged size and modification time short-circuit it,
//! and hashes of changed files are cached for this process.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::state::Take;

/// (size, modified ns) of a file, or None when it is gone.
fn stamp(path: &Path) -> Option<(u64, u128)> {
    let m = std::fs::metadata(path).ok()?;
    let t = m.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    Some((m.len(), t))
}

/// path -> (size, modified ns, SHA-256), for this process.
type HashCache = Mutex<HashMap<PathBuf, (u64, u128, String)>>;

fn cache() -> &'static HashCache {
    static CELL: OnceLock<HashCache> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(HashMap::new()))
}

/// SHA-256 of a file, reusing the hash while its size and time stay the same.
pub fn hash_file(path: &Path) -> Option<String> {
    let (len, t) = stamp(path)?;
    if let Some((l, tt, h)) = cache().lock().ok()?.get(path) {
        if *l == len && *tt == t {
            return Some(h.clone());
        }
    }
    let mut f = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let h: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if let Ok(mut c) = cache().lock() {
        c.insert(path.to_path_buf(), (len, t, h.clone()));
    }
    Some(h)
}

/// Where a recorded file lives: paths under the project folder are kept relative to it,
/// so the project can move.
fn resolve(root: &Path, file: &str) -> PathBuf {
    let p = Path::new(file);
    if p.is_absolute() { p.to_path_buf() } else { root.join(p) }
}

fn relative(root: &Path, file: &Path) -> String {
    let shown = file.strip_prefix(root).unwrap_or(file);
    shown.to_string_lossy().replace('\\', "/")
}

/// The deps a take keeps, from what the engine reported (`[{package, file}]`): each file
/// that exists, hashed now. Files that cannot be read are left out (the engine reported a
/// package that is not a file on disk, such as engine content).
pub fn record(root: &Path, reported: &[Value]) -> Vec<Value> {
    let mut seen = std::collections::BTreeMap::new();
    for d in reported {
        let Some(file) = d["file"].as_str().filter(|f| !f.is_empty()) else { continue };
        let path = resolve(root, file);
        let (Some((len, t)), Some(sha)) = (stamp(&path), hash_file(&path)) else { continue };
        let package = d["package"].as_str().unwrap_or(file).to_string();
        seen.insert(relative(root, &path), json!({
            "package": package, "file": relative(root, &path), "size": len, "mtime": t.to_string(), "sha256": sha,
        }));
    }
    seen.into_values().collect()
}

/// The packages of a take whose files changed (or went away) since it was rendered. Empty
/// for a take that recorded none (previz, or an engine that does not report them).
pub fn changed(root: &Path, take: &Take) -> Vec<String> {
    let mut out = Vec::new();
    for d in take.info.get("deps").and_then(Value::as_array).into_iter().flatten() {
        let Some(file) = d["file"].as_str() else { continue };
        let path = resolve(root, file);
        let package = d["package"].as_str().unwrap_or(file).to_string();
        let same = match stamp(&path) {
            None => false,
            Some((len, t)) if Some(len) == d["size"].as_u64() && Some(t.to_string().as_str()) == d["mtime"].as_str() => true,
            Some(_) => hash_file(&path).as_deref() == d["sha256"].as_str(),
        };
        if !same {
            out.push(package);
        }
    }
    out
}

/// Short names for the app and the CLI: "/Game/Enemy/T_Enemy_D" -> "T_Enemy_D".
pub fn short(package: &str) -> &str {
    package.rsplit(['/', '\\']).next().unwrap_or(package)
}
