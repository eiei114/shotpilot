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
    /// Any OpenAI-compatible `/v1/chat/completions` server (Ollama, vLLM, LM Studio, OpenRouter).
    /// Configured by `SHOTPILOT_OPENAI_BASE_URL` and optionally `SHOTPILOT_OPENAI_KEY`.
    OpenAi,
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Provider::Auto => "auto",
            Provider::ClaudeCode => "claude-code",
            Provider::ApiKey => "api-key",
            Provider::OpenAi => "openai",
        }
    }

    pub fn parse(s: &str) -> Result<Provider> {
        Ok(match s {
            "auto" | "" => Provider::Auto,
            "claude-code" => Provider::ClaudeCode,
            "api-key" => Provider::ApiKey,
            "openai" => Provider::OpenAi,
            _ => bail!("unknown provider '{s}' (auto, claude-code, api-key, openai)"),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub provider: Provider,
    pub api_key: Option<String>,
    /// How hard Claude thinks (low..max). None = the default (`ai::DEFAULT_EFFORT`).
    pub effort: Option<String>,
    /// OpenAI API key, for mood-board images (Claude does not make images).
    pub openai_api_key: Option<String>,
    /// How mood-board images are made: auto (Codex if installed, else the key), codex, openai-api.
    pub image_provider: String,
}

pub const IMAGE_PROVIDERS: &[&str] = &["auto", "codex", "openai-api"];

/// The `codex` command, when installed.
pub fn codex_cli() -> Option<PathBuf> {
    find_on_path("codex")
}

fn env_openai() -> Option<String> {
    std::env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty())
}

/// The image model for the mood board, or a message saying what is missing.
pub fn make_image_gen(settings: &Settings) -> Result<Box<dyn crate::moodboard::ImageGen>> {
    let key = settings.openai_api_key.clone().or_else(env_openai);
    let codex = || codex_cli().and_then(crate::moodboard::CodexImages::new);
    match settings.image_provider.as_str() {
        "codex" => match codex() {
            Some(c) => Ok(Box::new(c)),
            None => bail!("Codex (the `codex` command) was not found; install it and log in, or use an OpenAI API key"),
        },
        "openai-api" => match key {
            Some(k) => Ok(Box::new(crate::moodboard::OpenAiImages::new(k))),
            None => bail!("no OpenAI API key yet: enter one in the connection settings (or set OPENAI_API_KEY)"),
        },
        _ => {
            if let Some(c) = codex() {
                Ok(Box::new(c))
            } else if let Some(k) = key {
                Ok(Box::new(crate::moodboard::OpenAiImages::new(k)))
            } else {
                bail!("no image model: install Codex and log in, or enter an OpenAI API key in the connection settings")
            }
        }
    }
}

/// Which image backend the settings resolve to right now (for the UI).
pub fn active_image_provider(settings: &Settings) -> Option<&'static str> {
    let key = settings.openai_api_key.is_some() || env_openai().is_some();
    let codex = codex_cli().is_some();
    match settings.image_provider.as_str() {
        "codex" => codex.then_some("codex"),
        "openai-api" => key.then_some("openai-api"),
        _ => if codex { Some("codex") } else if key { Some("openai-api") } else { None },
    }
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
            openai_api_key: v["openai_api_key"].as_str().filter(|s| !s.is_empty()).map(str::to_string),
            image_provider: v["image_provider"].as_str().filter(|s| IMAGE_PROVIDERS.contains(s)).unwrap_or("auto").to_string(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let v = json!({"provider": self.provider.name(), "api_key": self.api_key, "effort": self.effort,
                       "openai_api_key": self.openai_api_key, "image_provider": self.image_provider});
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

/// Sends every request at one effort level, whichever way Claude is reached. A quick call
/// (chip suggestions, D-62) asks for `suggest::EFFORT` itself and keeps it: its chips have to
/// arrive in seconds whatever effort planning runs at.
pub struct WithEffort {
    pub inner: Box<dyn Claude>,
    pub effort: String,
}

impl Claude for WithEffort {
    fn ask(&self, request: &Value) -> Result<String> {
        let mut r = request.clone();
        if r["output_config"]["effort"] != crate::suggest::EFFORT {
            r["output_config"]["effort"] = json!(self.effort);
        }
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
    // SHOTPILOT_PROVIDER overrides the saved choice (handy for CLI runs against a local server)
    let provider = std::env::var("SHOTPILOT_PROVIDER")
        .ok()
        .and_then(|p| Provider::parse(p.trim()).ok())
        .filter(|p| *p != Provider::Auto)
        .unwrap_or(settings.provider);
    match provider {
        Provider::OpenAi => Ok(Box::new(OpenAiClaude::from_env()?)),
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
        Provider::OpenAi => OpenAiConfig::env_base().map(|_| "openai"),
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
        "openai_key": settings.openai_api_key.as_deref().map(|k| format!("…{}", &k[k.len().saturating_sub(4)..])),
        "openai_env": env_openai().is_some(),
        "image_model": crate::moodboard::image_model(),
        "image_provider": if settings.image_provider.is_empty() { "auto" } else { settings.image_provider.as_str() },
        "image_active": active_image_provider(settings),
        "codex": codex_cli().map(|p| p.display().to_string()),
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

// --------------------------------------------------------------------------- OpenAI-compatible

/// Which `response_format` a request goes out with; a server that rejects one gets the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Schema,
    Object,
    Plain,
}

/// Explicit configuration for [`OpenAiClaude`]; the environment is read only in [`OpenAiClaude::from_env`].
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// Up to and including `/v1`, e.g. `http://localhost:11434/v1`.
    pub base: String,
    pub key: Option<String>,
    /// Send images; when false each image becomes an "[image omitted]" text part.
    pub vision: bool,
    pub temperature: f64,
    pub max_tokens: u64,
    /// Seconds to wait before the first retry (doubles each time).
    pub backoff_secs: u64,
}

impl OpenAiConfig {
    fn env_base() -> Option<String> {
        std::env::var("SHOTPILOT_OPENAI_BASE_URL").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }
}

/// Runs the Messages-shaped requests of `ai.rs` against `POST {base}/chat/completions`.
pub struct OpenAiClaude {
    cfg: OpenAiConfig,
}

struct SendFail {
    retry: bool,
    format_rejected: bool,
    err: Error,
}

impl OpenAiClaude {
    pub fn new(cfg: OpenAiConfig) -> OpenAiClaude {
        OpenAiClaude { cfg }
    }

    pub fn from_env() -> Result<OpenAiClaude> {
        let Some(base) = OpenAiConfig::env_base() else {
            bail!("set SHOTPILOT_OPENAI_BASE_URL (e.g. http://localhost:11434/v1) to use an OpenAI-compatible server");
        };
        let env = |k: &str| std::env::var(k).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let vision_off = env("SHOTPILOT_VISION").is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "0" | "false" | "off" | "no"));
        Ok(OpenAiClaude::new(OpenAiConfig {
            base,
            key: env("SHOTPILOT_OPENAI_KEY"),
            vision: !vision_off,
            temperature: env("SHOTPILOT_TEMPERATURE").and_then(|t| t.parse().ok()).unwrap_or(0.2),
            max_tokens: env("SHOTPILOT_MAX_TOKENS").and_then(|t| t.parse().ok()).unwrap_or(16000),
            backoff_secs: 2,
        }))
    }

    /// The chat-completions body for a Messages API request. Anthropic-only fields are dropped.
    fn translate(&self, request: &Value, format: Format) -> Value {
        let mut system = request["system"].as_str().unwrap_or("").to_string();
        let schema = &request["output_config"]["format"]["schema"];
        if !schema.is_null() && format != Format::Schema {
            system = schema_in_system(&system, schema);
        }
        let user = match &request["messages"][0]["content"] {
            Value::Array(blocks) => Value::Array(
                blocks
                    .iter()
                    .filter_map(|b| match b["type"].as_str() {
                        Some("text") => Some(json!({"type": "text", "text": b["text"]})),
                        Some("image") if self.cfg.vision => {
                            let src = &b["source"];
                            let url = format!("data:{};base64,{}", src["media_type"].as_str().unwrap_or("image/png"), src["data"].as_str().unwrap_or(""));
                            Some(json!({"type": "image_url", "image_url": {"url": url}}))
                        }
                        Some("image") => Some(json!({"type": "text", "text": "[image omitted]"})),
                        _ => None,
                    })
                    .collect(),
            ),
            other => other.clone(),
        };
        let mut body = json!({
            "model": request["model"],
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}],
            "temperature": self.cfg.temperature,
            "max_tokens": self.cfg.max_tokens,
            "stream": false,
        });
        match format {
            Format::Schema if !schema.is_null() => {
                body["response_format"] = json!({"type": "json_schema", "json_schema": {"name": "reply", "schema": schema, "strict": false}});
            }
            Format::Object => body["response_format"] = json!({"type": "json_object"}),
            _ => {}
        }
        body
    }

    fn send_once(&self, body: &str) -> std::result::Result<String, SendFail> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(30))
            .timeout_read(std::time::Duration::from_secs(600))
            .build();
        let mut req = agent.post(&format!("{}/chat/completions", self.cfg.base.trim_end_matches('/'))).set("content-type", "application/json");
        if let Some(k) = &self.cfg.key {
            req = req.set("authorization", &format!("Bearer {k}"));
        }
        match req.send_string(body) {
            Ok(r) => r.into_string().map_err(|e| SendFail {
                retry: true,
                format_rejected: false,
                err: Error::user(format!("OpenAI-compatible server: reply broke: {e}")),
            }),
            Err(ureq::Error::Status(code, r)) => {
                let text = r.into_string().unwrap_or_default();
                let excerpt: String = text.chars().take(300).collect();
                let lower = text.to_lowercase();
                let rejected = (code == 400 || code == 422) && (lower.contains("response_format") || lower.contains("json_schema"));
                Err(SendFail {
                    retry: code == 429 || code >= 500,
                    format_rejected: rejected,
                    err: Error::user(format!("OpenAI-compatible server error {code}: {}", excerpt.trim())),
                })
            }
            Err(e) => Err(SendFail {
                retry: true,
                format_rejected: false,
                err: Error::user(format!("could not reach the OpenAI-compatible server: {e}")),
            }),
        }
    }

    fn send(&self, body: &str) -> std::result::Result<String, SendFail> {
        let mut wait = self.cfg.backoff_secs;
        let mut attempt = 0;
        loop {
            match self.send_once(body) {
                Err(f) if f.retry && attempt < 3 => {
                    eprintln!("shotpilot: {}; retrying in {wait}s", f.err.msg);
                    std::thread::sleep(std::time::Duration::from_secs(wait));
                    wait *= 2;
                    attempt += 1;
                }
                other => return other,
            }
        }
    }
}

/// The system prompt for a model that cannot be held to a schema: the schema goes in as text.
/// Shared by the OpenAI-compatible fallbacks and the eval's `+noschema` models.
pub fn schema_in_system(system: &str, schema: &Value) -> String {
    format!("{system}\n\nReply with a single JSON object (no other text) that matches this JSON schema:\n{schema}")
}

/// Drops `<think>...</think>` reasoning (also a lone closing tag) and surrounding blanks.
pub fn strip_reasoning(text: &str) -> String {
    let mut out = text.to_string();
    while let (Some(a), Some(b)) = (out.find("<think>"), out.find("</think>")) {
        if b < a {
            break;
        }
        out.replace_range(a..b + "</think>".len(), "");
    }
    if let Some(b) = out.rfind("</think>") {
        out = out[b + "</think>".len()..].to_string();
    }
    out.trim().to_string()
}

fn message_text(reply: &str) -> Result<String> {
    let v: Value = serde_json::from_str(reply).map_err(|e| Error::user(format!("OpenAI-compatible server returned malformed JSON: {e}")))?;
    let text = match &v["choices"][0]["message"]["content"] {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join(""),
        _ => String::new(),
    };
    let text = strip_reasoning(&text);
    if text.is_empty() {
        let why = v["choices"][0]["finish_reason"].as_str().unwrap_or("no text");
        bail!("OpenAI-compatible server returned no text ({why})");
    }
    Ok(text)
}

impl Claude for OpenAiClaude {
    fn ask(&self, request: &Value) -> Result<String> {
        let has_schema = !request["output_config"]["format"]["schema"].is_null();
        let formats: &[Format] = if has_schema { &[Format::Schema, Format::Object, Format::Plain] } else { &[Format::Plain] };
        for (i, f) in formats.iter().enumerate() {
            let body = self.translate(request, *f).to_string();
            match self.send(&body) {
                Ok(reply) => return message_text(&reply),
                Err(fail) if fail.format_rejected && i + 1 < formats.len() => continue,
                Err(fail) => return Err(fail.err),
            }
        }
        bail!("OpenAI-compatible server rejected every request format")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    type Seen = Arc<Mutex<Vec<(String, Value)>>>;

    /// Serves the scripted (status, body) replies in order; records (headers, body) of each request.
    fn fake_server(replies: Vec<(u16, String)>) -> (String, Seen) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        std::thread::spawn(move || {
            for (status, body) in replies {
                let Ok((mut sock, _)) = listener.accept() else { return };
                let mut reader = BufReader::new(sock.try_clone().unwrap());
                let (mut head, mut len) = (String::new(), 0usize);
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    head.push_str(&line);
                }
                let mut buf = vec![0u8; len];
                reader.read_exact(&mut buf).unwrap();
                log.lock().unwrap().push((head, serde_json::from_slice(&buf).unwrap_or(Value::Null)));
                let resp = format!("HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
                let _ = sock.write_all(resp.as_bytes());
            }
        });
        (base, seen)
    }

    fn reply(content: &str) -> (u16, String) {
        (200, json!({"choices": [{"message": {"role": "assistant", "content": content}, "finish_reason": "stop"}]}).to_string())
    }

    fn client(base: String, vision: bool) -> OpenAiClaude {
        OpenAiClaude::new(OpenAiConfig { base, key: Some("k-secret".into()), vision, temperature: 0.2, max_tokens: 1234, backoff_secs: 0 })
    }

    fn image_request() -> Value {
        let mut req = crate::ai::request("SYS", "hi", json!({"type": "object"}));
        req["messages"][0]["content"] = json!([
            {"type": "text", "text": "look"},
            {"type": "image", "source": {"type": "base64", "media_type": "image/jpeg", "data": "QUJD"}},
        ]);
        req
    }

    #[test]
    fn openai_request_is_translated() {
        let (base, seen) = fake_server(vec![reply("{\"a\":1}")]);
        let out = client(base, true).ask(&image_request()).unwrap();
        assert_eq!(out, "{\"a\":1}");
        let seen = seen.lock().unwrap();
        let (head, body) = &seen[0];
        assert!(head.starts_with("POST /v1/chat/completions"), "{head}");
        assert!(head.to_lowercase().contains("authorization: bearer k-secret"));
        assert_eq!(body["model"], crate::ai::model_name());
        assert_eq!(body["messages"][0], json!({"role": "system", "content": "SYS"}));
        assert_eq!(body["messages"][1]["content"][0], json!({"type": "text", "text": "look"}));
        assert_eq!(body["messages"][1]["content"][1]["image_url"]["url"], "data:image/jpeg;base64,QUJD");
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["response_format"]["json_schema"]["schema"], json!({"type": "object"}));
        assert_eq!(body["temperature"], 0.2);
        assert_eq!(body["max_tokens"], 1234);
        assert_eq!(body["stream"], false);
        for gone in ["thinking", "output_config", "fallbacks"] {
            assert!(body.get(gone).is_none(), "{gone} must not be sent");
        }
    }

    #[test]
    fn rejected_response_format_falls_back_to_json_object_then_plain() {
        let bad = (400, json!({"error": {"message": "response_format.json_schema is not supported"}}).to_string());
        let (base, seen) = fake_server(vec![bad.clone(), reply("{\"ok\":true}")]);
        assert_eq!(client(base, true).ask(&image_request()).unwrap(), "{\"ok\":true}");
        {
            let seen = seen.lock().unwrap();
            assert_eq!(seen.len(), 2);
            assert_eq!(seen[1].1["response_format"], json!({"type": "json_object"}));
            assert!(seen[1].1["messages"][0]["content"].as_str().unwrap().contains("\"type\":\"object\""));
        }
        let (base, seen) = fake_server(vec![bad.clone(), bad, reply("{}")]);
        assert_eq!(client(base, true).ask(&image_request()).unwrap(), "{}");
        assert!(seen.lock().unwrap()[2].1.get("response_format").is_none());
        // an unrelated 400 is an error, not a fallback
        let (base, seen) = fake_server(vec![(400, "context length exceeded".into())]);
        let e = client(base, true).ask(&image_request()).unwrap_err();
        assert!(e.msg.contains("400") && e.msg.contains("context length"), "{}", e.msg);
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn reasoning_preamble_is_stripped() {
        assert_eq!(strip_reasoning("<think>hmm {\"x\":0}</think>\n{\"a\":1}"), "{\"a\":1}");
        assert_eq!(strip_reasoning("thinking...</think>{\"a\":1}"), "{\"a\":1}");
        assert_eq!(strip_reasoning("Here you go: {\"a\":1}"), "Here you go: {\"a\":1}");
        let (base, _) = fake_server(vec![reply("<think>plan</think>{\"a\":1}")]);
        assert_eq!(client(base, true).ask(&image_request()).unwrap(), "{\"a\":1}");
        let (base, _) = fake_server(vec![reply("Sure! {\"a\":1}")]);
        assert_eq!(client(base, true).ask(&image_request()).unwrap(), "Sure! {\"a\":1}");
    }

    #[test]
    fn vision_off_drops_images() {
        let (base, seen) = fake_server(vec![reply("{}")]);
        client(base, false).ask(&image_request()).unwrap();
        let body = seen.lock().unwrap()[0].1.clone();
        assert_eq!(body["messages"][1]["content"][1], json!({"type": "text", "text": "[image omitted]"}));
        assert!(!body.to_string().contains("image_url"));
    }

    #[test]
    fn openai_is_a_provider() {
        assert_eq!(Provider::parse("openai").unwrap(), Provider::OpenAi);
        assert_eq!(Provider::OpenAi.name(), "openai");
    }

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
    fn openai_key_is_kept_but_never_shown() {
        let dir = std::env::temp_dir().join(format!("sp-openai-{}", std::process::id()));
        let p = dir.join("settings.json");
        Settings { openai_api_key: Some("sk-proj-secret-5678".into()), ..Default::default() }.save(&p).unwrap();
        let back = Settings::load(Some(&p));
        assert_eq!(back.openai_api_key.as_deref(), Some("sk-proj-secret-5678"));
        let shown = describe(&back).to_string();
        assert!(!shown.contains("secret") && shown.contains("…5678"));
        assert!(make_image_gen(&back).is_ok());
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
        // a quick call (chip suggestions) keeps its own low effort whatever planning runs at
        let mut quick = req.clone();
        quick["output_config"]["effort"] = json!(crate::suggest::EFFORT);
        assert_eq!(c.ask(&quick).unwrap(), crate::suggest::EFFORT);
        // and Claude Code gets it as --effort
        let mut r = req.clone();
        r["output_config"]["effort"] = json!("high");
        let args = CliClaude::args(&r);
        let i = args.iter().position(|a| a == "--effort").unwrap();
        assert_eq!(args[i + 1], "high");
    }
}
