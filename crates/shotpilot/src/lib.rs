//! Shotpilot: shot design and capture lane for UE5 teasers.
//!
//! Define shots in `shots.yaml`; only the shots whose pixels changed are re-rendered.

pub mod ai;
pub mod connect;
pub mod craft;
pub mod cut;
pub mod engines;
pub mod error;
pub mod finish;
pub mod fingerprint;
pub mod geometry;
pub mod library;
pub mod model;
pub mod moodboard;
pub mod previz;
pub mod scout;
pub mod server;
pub mod shotcheck;
pub mod sound;
pub mod soundcheck;
pub mod state;
pub mod sync;
pub mod templates;
pub mod workspace;
pub mod yamlio;

pub use error::{Error, Result};
pub use workspace::Workspace;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
