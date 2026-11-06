//! Shotpilot: shot design and capture lane for UE5 teasers.
//!
//! Define shots in `shots.yaml`; only the shots whose pixels changed are re-rendered.

pub mod ai;
pub mod claudecode;
pub mod concept;
pub mod connect;
pub mod craft;
pub mod content;
pub mod cut;
pub mod designsys;
pub mod engines;
pub mod error;
pub mod eval;
pub mod finish;
pub mod fonts;
pub mod fx;
pub mod fingerprint;
pub mod geometry;
pub mod library;
pub mod model;
pub mod motion;
pub mod moodboard;
pub mod previz;
pub mod produce;
pub mod projects;
pub mod promo;
pub mod refs;
pub mod scout;
pub mod script;
pub mod server;
pub mod shotcheck;
pub mod skills;
pub mod sound;
pub mod soundcheck;
pub mod sounddesign;
pub mod state;
pub mod storyboard;
pub mod suggest;
pub mod sync;
pub mod templates;
pub mod workspace;
pub mod yamlio;

pub use error::{Error, Result};
pub use workspace::Workspace;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
