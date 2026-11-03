//! How Shotpilot reaches Claude.
//!
//! Two ways, picked in the app's settings:
//! - Claude Code: run the user's `claude` CLI headless (`-p`, stream-json in and out),
//!   which is what the Agent SDK drives under the hood. It uses the account already
//!   logged in to Claude Code, so no API key is needed, and no Node or Python either.
//! - API key: call the Messages API directly (`ai::HttpClaude`).
//!
//! The choice (and the key, if any) lives in a small JSON file in the user's config
//! folder, next to the app's last-project file. The key is never sent back to the UI.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

use crate::ai::{Claude, HttpClaude};
use crate::bail;
use crate::engines::find_on_path;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Provider {
    /// Claude Code if it is installed, else an API key from the environment.
    #[default]
    Auto,
    ClaudeCode,
    ApiKey,
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Provider::Auto => "auto",
            Provider::ClaudeCode => "claude-code",
            Provider::ApiKey => "api-key",
        }
    }

    pub fn parse(s: &str) -> Result<Provider> {
        Ok(match s {
            "auto" | "" => Provider::Auto,
            "claude-code" => Provider::ClaudeCode,
            "api-key" => Provider::ApiKey,
            _ => bail!("unknown provider '{s}' (auto, claude-code, api-key)"),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub provider: Provider,
    pub api_key: Option<String>,
    /// How hard Claude thinks (low..max). None = the default (`ai::DEFAULT_EFFORT`).
    pub effort: Option<String>,
}

impl Settings {
    /// The effort requests go out with: `SHOTPILOT_EFFORT`, else the saved setting, else the default.
    pub fn effective_effort(&self) -> String {
        std::env::var("SHOTPILOT_EFFORT")
            .ok()
            .filter(|e| crate::ai::valid_effort(e))
            .or_else(|| self.effort.clone())
            .unwrap_or_else(|| crate::ai::DEFAULT_EFFORT.to_string())
    }
}

impl Settings {
    /// `<config>/shotpilot/settings.json` (%APPDATA% on Windows).
    pub fn default_path() -> Option<PathBuf> {
        let base = std::env::var_os("APPDATA")
            .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("shotpilot").join("settings.json"))
    }

    /// A missing or unreadable file means defaults; settings never block startup.
    pub fn load(path: Option<&Path>) -> Settings {
        let Some(v) = path.and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<Value>(&t).ok())
        else {
            return Settings::default();
        };
        Settings {
            provider: v["provider"].as_str().and_then(|s| Provider::parse(s).ok()).unwrap_or_default(),
            api_key: v["api_key"].as_str().filter(|s| !s.is_empty()).map(str::to_string),
            effort: v["effort"].as_str().filter(|e| crate::ai::valid_effort(e)).map(str::to_string),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let v = json!({"provider": self.provider.name(), "api_key": self.api_key, "effort": self.effort});
        let io = |e: std::io::Error| Error::user(format!("cannot write {}: {e}", path.display()));
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(&v).unwrap_or_default()).map_err(io)
    }
}

fn env_key() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty())
}

/// The `claude` command: on PATH, or where the native installer puts it.
pub fn claude_cli() -> Option<PathBuf> {
    find_on_path("claude").or_else(|| {
        let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
        let bin = PathBuf::from(home).join(".local").join("bin");
        ["claude.exe", "claude"].iter().map(|n| bin.join(n)).find(|p| p.is_file())
    })
}

/// Sends every request at one effort level, whichever way Claude is reached.
pub struct WithEffort {
    pub inner: Box<dyn Claude>,
    pub effort: String,
}

impl Claude for WithEffort {
    fn ask(&self, request: &Value) -> Result<String> {
        let mut r = request.clone();
        r["output_config"]["effort"] = json!(self.effort);
        self.inner.ask(&r)
    }
}

/// The client the settings call for (at the configured effort), or a message saying what is missing.
pub fn make_claude(settings: &Settings) -> Result<Box<dyn Claude>> {
    let inner = make_client(settings)?;
    Ok(Box::new(WithEffort { inner, effort: settings.effective_effort() }))
}

fn make_client(settings: &Settings) -> Result<Box<dyn Claude>> {
    let key = settings.api_key.clone().or_else(env_key);
    match settings.provider {
        Provider::ApiKey => match key {
            Some(k) => Ok(Box::new(HttpClaude::with_key(k))),
            None => bail!("no API key yet: enter one in the settings, or set ANTHROPIC_API_KEY"),
        },
        Provider::ClaudeCode => match claude_cli() {
            Some(exe) => Ok(Box::new(CliClaude::new(exe))),
            None => bail!("Claude Code (the `claude` command) was not found; install it and log in, or use an API key"),
        },
        Provider::Auto => {
            if let Some(exe) = claude_cli() {
                Ok(Box::new(CliClaude::new(exe)))
            } else if let Some(k) = key {
                Ok(Box::new(HttpClaude::with_key(k)))
            } else {
                bail!("Claude is not connected: install Claude Code and log in, or enter an API key in the settings")
            }
        }
    }
}

/// What the settings screen shows. Never includes the key itself.
pub fn describe(settings: &Settings) -> Value {
    let cli = claude_cli();
    let key = settings.api_key.as_deref();
    let active = match settings.provider {
        Provider::ApiKey => (key.is_some() || env_key().is_some()).then_some("api-key"),
        Provider::ClaudeCode => cli.is_some().then_some("claude-code"),
        Provider::Auto => {
            if cli.is_some() {
                Some("claude-code")
            } else if key.is_some() || env_key().is_some() {
                Some("api-key")
            } else {
                None
            }
        }
    };
    json!({
        "provider": settings.provider.name(),
        "active": active,
        "claude_code": cli.map(|p| p.display().to_string()),
        "api_key": key.map(|k| format!("…{}", &k[k.len().saturating_sub(4)..])),
        "env_key": env_key().is_some(),
        "effort": settings.effective_effort(),
        "effort_saved": settings.effort,
        "effort_env": std::env::var("SHOTPILOT_EFFORT").ok().filter(|e| crate::ai::valid_effort(e)),
        "efforts": crate::ai::EFFORTS,
    })
}

// --------------------------------------------------------------------------- Claude Code

/// Runs one request through `claude -p`. Tools are off and nothing is saved as a
/// session; the answer comes back as the CLI's `structured_output`.
pub struct CliClaude {
    exe: PathBuf,
    /// Neutral working folder, so a game repo's CLAUDE.md does not leak into the prompt.
    cwd: PathBuf,
}

impl CliClaude {
    pub fn new(exe: PathBuf) -> CliClaude {
        CliClaude { exe, cwd: std::env::temp_dir() }
    }

    /// The command line for a Messages API request body (the user turn goes on stdin).
    pub fn args(request: &Value) -> Vec<String> {
        let mut a: Vec<String> = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
                                  "--tools", "", "--no-session-persistence"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut opt = |flag: &str, v: Option<String>| {
            if let Some(v) = v {
                a.push(flag.into());
                a.push(v);
            }
        };
        opt("--model", request["model"].as_str().map(str::to_string));
        opt("--effort", request["output_config"]["effort"].as_str().map(str::to_string));
        opt("--system-prompt", request["system"].as_str().map(str::to_string));
        let schema = &request["output_config"]["format"]["schema"];
        opt("--json-schema", (!schema.is_null()).then(|| schema.to_string()));
        a
    }
}

/// The final answer from `claude -p --output-format stream-json` output.
pub fn parse_cli_output(stdout: &str) -> Result<String> {
    let result = stdout
        .lines()
        .rev()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
        .find(|v| v["type"] == "result");
    let Some(r) = result else {
        bail!("Claude Code returned no result");
    };
    if r["is_error"].as_bool().unwrap_or(false) || r["subtype"].as_str().is_some_and(|s| s != "success") {
        let why = r["result"].as_str().filter(|s| !s.is_empty()).or(r["subtype"].as_str()).unwrap_or("unknown error");
        bail!("Claude Code: {why}");
    }
    match &r["structured_output"] {
        Value::Null => Ok(r["result"].as_str().unwrap_or("").to_string()),
        v => Ok(v.to_string()),
    }
}

impl Claude for CliClaude {
    fn ask(&self, request: &Value) -> Result<String> {
        let mut cmd = Command::new(&self.exe);
        cmd.args(CliClaude::args(request))
            .current_dir(&self.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            // the app has no console; don't flash one for the child
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = cmd.spawn().map_err(|e| Error::user(format!("cannot start {}: {e}", self.exe.display())))?;
        let line = json!({"type": "user", "message": {"role": "user", "content": request["messages"][0]["content"]}});
        let mut stdin = child.stdin.take().expect("piped");
        // write on a thread: the request can be megabytes (an image) and the child
        // starts printing before it has read all of it
        let writer = std::thread::spawn(move || writeln!(stdin, "{line}"));
        let out = child.wait_with_output().map_err(|e| Error::user(format!("Claude Code: {e}")))?;
        let _ = writer.join();
        let stdout = String::from_utf8_lossy(&out.stdout);
        match parse_cli_output(&stdout) {
            Ok(text) => Ok(text),
            Err(e) if !out.status.success() => {
                let err = String::from_utf8_lossy(&out.stderr);
                let tail = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
                bail!("{}{}", e.msg, if tail.is_empty() { String::new() } else { format!(" ({tail})") })
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_maps_to_cli_flags() {
        let req = crate::ai::request("SYS", "hi", json!({"type": "object"}));
        let a = CliClaude::args(&req);
        let after = |f: &str| a.iter().position(|x| x == f).map(|i| a[i + 1].clone());
        assert_eq!(after("--system-prompt").as_deref(), Some("SYS"));
        assert_eq!(after("--json-schema").as_deref(), Some("{\"type\":\"object\"}"));
        assert_eq!(after("--model").as_deref(), Some(crate::ai::model_name().as_str()));
        assert_eq!(after("--effort").as_deref(), Some("medium"));
        assert_eq!(after("--tools").as_deref(), Some(""));
        assert!(a.contains(&"--no-session-persistence".to_string()));
    }

    #[test]
    fn cli_output_gives_the_structured_answer() {
        let ok = "{\"type\":\"system\",\"subtype\":\"init\"}\n\
                  {\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"{}\",\"structured_output\":{\"a\":1}}\n";
        assert_eq!(parse_cli_output(ok).unwrap(), "{\"a\":1}");
        let bad = "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":true,\"result\":\"Not logged in\"}";
        assert!(parse_cli_output(bad).unwrap_err().msg.contains("Not logged in"));
        assert!(parse_cli_output("").is_err());
    }

    #[test]
    fn settings_round_trip_and_hide_the_key() {
        let dir = std::env::temp_dir().join(format!("sp-settings-{}", std::process::id()));
        let p = dir.join("settings.json");
        let s = Settings { provider: Provider::ApiKey, api_key: Some("sk-ant-secret-1234".into()), ..Default::default() };
        s.save(&p).unwrap();
        let back = Settings::load(Some(&p));
        assert_eq!(back.provider, Provider::ApiKey);
        assert_eq!(back.api_key.as_deref(), Some("sk-ant-secret-1234"));
        let shown = describe(&back).to_string();
        assert!(!shown.contains("secret"));
        assert!(shown.contains("…1234"));
        assert_eq!(Settings::load(Some(&dir.join("missing.json"))).provider, Provider::Auto);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn effort_is_saved_and_applied_to_every_request() {
        let dir = std::env::temp_dir().join(format!("sp-effort-{}", std::process::id()));
        let p = dir.join("settings.json");
        Settings { effort: Some("xhigh".into()), ..Default::default() }.save(&p).unwrap();
        assert_eq!(Settings::load(Some(&p)).effort.as_deref(), Some("xhigh"));
        std::fs::write(&p, r#"{"effort": "turbo"}"#).unwrap(); // unknown levels fall back to the default
        assert_eq!(Settings::load(Some(&p)).effort, None);
        let _ = std::fs::remove_dir_all(dir);

        struct Echo;
        impl Claude for Echo {
            fn ask(&self, request: &Value) -> Result<String> {
                Ok(request["output_config"]["effort"].as_str().unwrap_or("").to_string())
            }
        }
        let c = WithEffort { inner: Box::new(Echo), effort: "high".into() };
        let req = crate::ai::request("s", "u", json!({"type": "object"}));
        assert_eq!(req["output_config"]["effort"], crate::ai::DEFAULT_EFFORT);
        assert_eq!(c.ask(&req).unwrap(), "high");
        // and Claude Code gets it as --effort
        let mut r = req.clone();
        r["output_config"]["effort"] = json!("high");
        let args = CliClaude::args(&r);
        let i = args.iter().position(|a| a == "--effort").unwrap();
        assert_eq!(args[i + 1], "high");
    }
}
