//! `naysay` — the voice that says no before your coding agents say yes.
//!
//! Lineage: `pair` v0.1→v1.3 (brainstorm/drill/explain TUI), renamed and
//! repositioned for the agent era. pair asked "what could I build?";
//! naysay asks "should this be built at all, and at what scope?".
//!
//!   - `naysay seed <topic>`     → angles you haven't considered
//!   - `naysay drill <idea>`     → one angle, broken into sub-points
//!   - `naysay premortem <idea>` → assume it died in 6 months; read the autopsy first
//!   - `naysay spec <idea>`      → harden a surviving idea into a spec for your agent
//!   - `naysay explain <file>`   → open the black box, line by line
//!   - `naysay`                  → TUI;  `naysay repl` → plain REPL
//!   - `naysay.toml`             → any OpenAI-compatible endpoint
//!     (MiniMax default; OpenAI, DeepSeek, local Ollama, …)

mod tui;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::sync::{Mutex, OnceLock};

/// The crate version, for every user-visible banner. Deriving it here (not
/// hand-writing "v0.1" in three places) is the fix for the stale-banner bug
/// found in v0.3.0: the first-run box kept saying v0.1 through four
/// releases because nothing tied it to the build.
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

// ─── CLI definition ─────────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "naysay",
    version,
    long_version = concat!(
        env!("CARGO_PKG_VERSION"),
        " (",
        env!("NAYSAY_GIT_TAG"),
        " ",
        env!("NAYSAY_GIT_HASH"),
        ")",
    ),
    about = "The voice that says no before your coding agents say yes — runs the TUI by default, prompts for your API key if missing"
)]
struct Cli {
    /// Save output to a file instead of stdout
    #[arg(long, global = true, value_name = "PATH")]
    save: Option<String>,

    /// Output as JSON (machine-readable)
    #[arg(long, global = true)]
    json: bool,

    /// Launch the full-screen TUI REPL instead of the plain one
    #[arg(long, global = true)]
    tui: bool,

    /// Play 8-bit sound effects on TUI events (TUI mode only, off by default)
    #[arg(long, global = true)]
    sound: bool,

    /// Loop an 8-bit bassline in the background while the TUI is open (Windows only)
    #[arg(long, global = true)]
    music: bool,

    /// Resume the most recent session (TUI and REPL): replay its turns into
    /// the conversation and append new turns to the same session log
    #[arg(long = "continue", global = true)]
    continue_last: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Plain REPL mode (read-eval-print loop, type subcommands inline)
    Repl,
    /// Generate 8 angles you haven't thought of for a topic
    Seed {
        /// Topic to brainstorm around (quote if it has spaces)
        topic: String,
    },
    /// Drill one level deeper into a single idea
    Drill {
        /// The idea text to drill into (paste from a seed line)
        idea: String,
    },
    /// Assume it died six months from now — read the autopsy before committing
    Premortem {
        /// The idea to interrogate
        idea: String,
    },
    /// Harden a surviving idea into a spec your coding agent can execute
    Spec {
        /// The idea to spec out
        idea: String,
    },
    /// The project shipped or died — write the postmortem and the decision-log entry
    Postmortem {
        /// The idea/project being reviewed
        idea: String,
        /// What actually happened (optional context; without it the model says what evidence it would need)
        notes: Option<String>,
    },
    /// Explain a file line-by-line
    Explain {
        /// Path to the file
        path: String,
    },
    /// Manage API key in OS keyring
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },
    /// Browse past REPL sessions
    Sessions {
        #[command(subcommand)]
        action: SessionsAction,
    },
    /// Query the local decision store (.naysay/decisions/)
    Decisions {
        #[command(subcommand)]
        action: DecisionsAction,
    },
    /// Diagnose common setup problems (key, sessions dir, network)
    Doctor,
}

#[derive(Subcommand)]
enum DecisionsAction {
    /// Print one decision record by id
    ById {
        /// id (e.g. abc12345ef67) or full stem (e.g. premortem-abc12345ef67)
        id: String,
    },
    /// Show the decision chain walking up from a child id
    Link {
        /// child id
        child: String,
    },
    /// List every UNKNOWNS bullet across all stored premortems
    Unknowns,
}

#[derive(Subcommand)]
enum SessionsAction {
    /// List recent session files
    List,
    /// Print the contents of one session
    Show {
        /// Session file name (e.g. session-1755900000.jsonl) or epoch seconds
        file: String,
    },
}

#[derive(Subcommand)]
enum KeyAction {
    /// Save your API key to the OS keyring
    Set,
    /// Check whether an API key is configured
    Status,
    /// Remove the API key from the OS keyring
    Delete,
}

// ─── LLM wire types ──────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub(crate) struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize, Default)]
struct ChatChunkChoice {
    /// Optional so we don't choke on finish_reason-only chunks that
    /// arrive at the tail of an OpenAI-compatible stream. These look like
    /// `{"choices":[{"index":0,"finish_reason":"stop"}]}` — no `delta`
    /// at all. Treating them as fatal would lose partial content.
    #[serde(default)]
    delta: ChatChoiceDelta,
}

#[derive(Deserialize, Default)]
struct ChatChoiceDelta {
    #[serde(default)]
    content: String,
}

#[derive(Deserialize)]
struct ChatChunk {
    choices: Vec<ChatChunkChoice>,
    /// Some OpenAI-compatible servers append a `usage` object to the final
    /// chunk of a stream. Optional everywhere: absent → no display.
    #[serde(default)]
    usage: Option<Usage>,
}

/// Token accounting for one LLM call, as reported by the server. Drives the
/// "· N tok" notes — the cheap-thinking claim is only credible if the user
/// can see the meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub(crate) struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

impl Usage {
    pub(crate) fn total(&self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

/// Last usage reported by an LLM call, stored by `call_llm_with_model` /
/// `call_llm_stream` and consumed by the UI layer nearest to the user:
/// the six command functions print it to stderr (CLI + REPL), the TUI task
/// reads it into `TuiEvent::Result`. Safe from races: naysay never issues
/// concurrent LLM calls (the TUI's busy flag + sequential CLI/REPL paths).
static LAST_USAGE: Mutex<Option<Usage>> = Mutex::new(None);

pub(crate) fn store_last_usage(u: Usage) {
    if let Ok(mut slot) = LAST_USAGE.lock() {
        *slot = Some(u);
    }
}

pub(crate) fn take_last_usage() -> Option<Usage> {
    LAST_USAGE.lock().ok().and_then(|mut slot| slot.take())
}

/// CLI/REPL note: one dim line on stderr, so stdout stays pipe-clean.
pub(crate) fn note_usage_stderr() {
    if let Some(u) = take_last_usage() {
        eprintln!(
            "· {} tok (prompt {} + completion {})",
            u.total(),
            u.prompt_tokens,
            u.completion_tokens
        );
    }
}

// ─── Prompts (externalized to TOML) ─────────────────────────────────────────────────────

/// Per-command prompt overrides, loaded from `prompts.toml` in the data dir.
/// If the file is missing or malformed, every field falls back to the
/// embedded default — the TUI never fails to start because of a bad config.
///
/// To customize, copy the keys you want to change into the file. Anything
/// you omit keeps the default. Example:
///
///     [prompts]
///     angles = "Give me 8 angles on {topic}, each under 10 words."
///     steps = "Plan to {goal} in 5 numbered steps."
#[derive(Deserialize, Default)]
pub(crate) struct Prompts {
    pub angles: Option<String>,
    pub questions: Option<String>,
    pub contrarian: Option<String>,
    pub use_cases: Option<String>,
    pub premortem: Option<String>,
    pub spec: Option<String>,
    pub pros: Option<String>,
    pub cons: Option<String>,
    pub risks: Option<String>,
    pub steps: Option<String>,
    pub examples: Option<String>,
    pub explain: Option<String>,
    pub summarize: Option<String>,
    pub freeform: Option<String>,
}

#[derive(Deserialize, Default)]
struct PromptsFile {
    #[serde(default)]
    prompts: Prompts,
}

impl Prompts {
    /// Load overrides from `<data_dir>/prompts.toml`. Missing file or parse
    /// errors → empty Prompts (all None → caller falls back to defaults).
    ///
    /// On first run (no file present), write a documented template so users
    /// discover the customization point. The template has every key commented
    /// out — they uncomment what they want to change and the default still
    /// applies to anything left blank.
    pub(crate) fn load() -> Self {
        let path = match data_dir() {
            Ok(dir) => dir.join("prompts.toml"),
            Err(_) => return Self::default(),
        };
        if !path.exists() {
            let _ = std::fs::write(&path, PROMPTS_TEMPLATE);
        }
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Self::default(), // read failed after create? fall back
        };
        toml::from_str::<PromptsFile>(&raw)
            .map(|f| f.prompts)
            .unwrap_or_default()
    }

    /// Resolve `key` against overrides, falling back to `default`.
    pub(crate) fn get<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        let opt = match key {
            "angles" => &self.angles,
            "questions" => &self.questions,
            "contrarian" => &self.contrarian,
            "use_cases" => &self.use_cases,
            "premortem" => &self.premortem,
            "spec" => &self.spec,
            "pros" => &self.pros,
            "cons" => &self.cons,
            "risks" => &self.risks,
            "steps" => &self.steps,
            "examples" => &self.examples,
            "explain" => &self.explain,
            "summarize" => &self.summarize,
            "freeform" => &self.freeform,
            _ => &None,
        };
        opt.as_deref().unwrap_or(default)
    }
}

/// Template written to `<data_dir>/prompts.toml` on first run. Every key is
/// present but commented out — the user uncomments the ones they want to
/// customize. The template documents the placeholder contract for each
/// command (e.g. `{topic}` is the only variable injected by the TUI).
const PROMPTS_TEMPLATE: &str = "\
# naysay prompts — uncomment any line to override the default for that command.
# Placeholders ({topic}, {idea}, {goal}, etc.) are replaced at call time.
# Anything left commented falls back to the embedded default, so it's safe to
# delete keys you don't care about.

[prompts]
# angles = \"Brainstorm 5 angles on {topic}.\"
# questions = \"Ask 5 deep questions about {topic}.\"
# contrarian = \"Steel-man the opposite of: {claim}\"
# use_cases = \"User scenarios for {thing}.\"
# premortem = \"Autopsy for {idea}.\"
# spec = \"Spec for {idea}.\"
# pros = \"Strengths of {idea}.\"
# cons = \"Weaknesses of {idea}.\"
# risks = \"Failure modes for {idea}.\"
# steps = \"Actionable plan for {goal}.\"
# examples = \"Real-world examples of {concept}.\"
# explain = \"Walk through this file.\\nFile: {path}\\nContent:\\n{content}\"
# summarize = \"Summarize this file.\\nFile: {path}\\nContent:\\n{content}\"
";

// ─── Provider config (naysay.toml) ──────────────────────────────────────────────────────

/// Provider configuration, loaded from `<data_dir>/naysay.toml`.
///
/// naysay speaks the OpenAI chat-completions wire format, so any
/// OpenAI-compatible endpoint works: MiniMax (default), OpenAI, DeepSeek,
/// OpenRouter, or a local Ollama / LM Studio server. `chat_url` is the FULL
/// chat-completions URL — no path guessing, no per-provider adapters.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Config {
    /// Full chat-completions endpoint (OpenAI wire format).
    pub chat_url: String,
    /// Default model id. The TUI can still switch at runtime via `/model <name>`.
    pub model: String,
    /// Name of the env var the API key is read from.
    pub api_key_env: String,
}

pub(crate) const DEFAULT_CHAT_URL: &str = "https://api.minimax.chat/v1/text/chatcompletion_v2";
pub(crate) const DEFAULT_MODEL: &str = "MiniMax-Text-01";
pub(crate) const DEFAULT_API_KEY_ENV: &str = "NAYSAY_API_KEY";
const KEYRING_SERVICE: &str = "naysay";
const KEYRING_USER: &str = "api-key";

impl Default for Config {
    fn default() -> Self {
        Self {
            chat_url: DEFAULT_CHAT_URL.into(),
            model: DEFAULT_MODEL.into(),
            api_key_env: DEFAULT_API_KEY_ENV.into(),
        }
    }
}

impl Config {
    /// Parse a naysay.toml body, surfacing TOML errors instead of silently
    /// falling back. Doctor uses this to report a malformed file; the
    /// runtime path stays forgiving via `parse`.
    fn parse_strict(raw: &str) -> Result<Self, toml::de::Error> {
        #[derive(Deserialize)]
        struct File {
            #[serde(default)]
            provider: Provider,
        }
        #[derive(Deserialize, Default)]
        struct Provider {
            chat_url: Option<String>,
            model: Option<String>,
            api_key_env: Option<String>,
        }
        let file = toml::from_str::<File>(raw)?;
        let d = Self::default();
        Ok(Self {
            chat_url: file.provider.chat_url.unwrap_or(d.chat_url),
            model: file.provider.model.unwrap_or(d.model),
            api_key_env: file.provider.api_key_env.unwrap_or(d.api_key_env),
        })
    }

    /// Field-level validation beyond TOML well-formedness. Returns one
    /// human-readable issue per problem; empty = healthy.
    fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if !self.chat_url.starts_with("http://") && !self.chat_url.starts_with("https://") {
            issues.push(format!(
                "chat_url must start with http:// or https:// (got `{}`)",
                self.chat_url
            ));
        }
        if self.model.trim().is_empty() {
            issues.push("model is empty — every provider needs a model id".to_string());
        }
        let env_ok = !self.api_key_env.trim().is_empty()
            && self
                .api_key_env
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
        if !env_ok {
            issues.push(format!(
                "api_key_env must be an env-var name (A-Z, 0-9, _; got `{}`)",
                self.api_key_env
            ));
        }
        issues
    }

    /// Parse a naysay.toml body. Malformed TOML → defaults (same contract
    /// as prompts.toml: a bad config file must never stop the tool from
    /// starting).
    fn parse(raw: &str) -> Self {
        #[derive(Deserialize, Default)]
        struct File {
            #[serde(default)]
            provider: Provider,
        }
        #[derive(Deserialize, Default)]
        struct Provider {
            chat_url: Option<String>,
            model: Option<String>,
            api_key_env: Option<String>,
        }
        let file = toml::from_str::<File>(raw).unwrap_or_default();
        let d = Self::default();
        Self {
            chat_url: file.provider.chat_url.unwrap_or(d.chat_url),
            model: file.provider.model.unwrap_or(d.model),
            api_key_env: file.provider.api_key_env.unwrap_or(d.api_key_env),
        }
    }

    /// Load config from `<data_dir>/naysay.toml`, writing a documented
    /// template on first run. `NAYSAY_CHAT_URL` / `NAYSAY_MODEL` env vars
    /// override the file (CI escape hatch, same role as `api_key_env`).
    fn load() -> Self {
        let mut cfg = match data_dir() {
            Ok(dir) => {
                let path = dir.join("naysay.toml");
                if !path.exists() {
                    let _ = std::fs::write(&path, CONFIG_TEMPLATE);
                }
                std::fs::read_to_string(&path)
                    .map(|raw| Self::parse(&raw))
                    .unwrap_or_default()
            }
            Err(_) => Self::default(),
        };
        if let Ok(u) = std::env::var("NAYSAY_CHAT_URL") {
            if !u.is_empty() {
                cfg.chat_url = u;
            }
        }
        if let Ok(m) = std::env::var("NAYSAY_MODEL") {
            if !m.is_empty() {
                cfg.model = m;
            }
        }
        cfg
    }
}

/// Process-wide config, initialized on first access. Every code path touches
/// it before its first LLM call, so no explicit init step is needed.
pub(crate) static CONFIG: OnceLock<Config> = OnceLock::new();
pub(crate) fn config() -> &'static Config {
    CONFIG.get_or_init(Config::load)
}

/// Host of a chat URL, for display: `https://api.x.com/v1/chat` → `api.x.com`.
/// Falls back to the input when it doesn't look like a URL.
pub(crate) fn endpoint_host(url: &str) -> &str {
    let after_scheme = url.split("://").nth(1).unwrap_or(url);
    after_scheme.split('/').next().unwrap_or(url)
}

/// Template written to `<data_dir>/naysay.toml` on first run.
const CONFIG_TEMPLATE: &str = "\
# naysay provider config — any OpenAI-compatible chat endpoint works.
# Written on first run. Uncomment what you need, then restart naysay.
# Per-invocation env overrides: NAYSAY_CHAT_URL, NAYSAY_MODEL.

# MiniMax (default — leave everything commented to keep using it)
# chat_url = \"https://api.minimax.chat/v1/text/chatcompletion_v2\"
# model = \"MiniMax-Text-01\"

# OpenAI
# chat_url = \"https://api.openai.com/v1/chat/completions\"
# model = \"gpt-4o-mini\"

# DeepSeek
# chat_url = \"https://api.deepseek.com/chat/completions\"
# model = \"deepseek-chat\"

# Local Ollama (offline, private, free)
# chat_url = \"http://localhost:11434/v1/chat/completions\"
# model = \"qwen2.5:7b\"

[provider]
# chat_url = \"https://api.openai.com/v1/chat/completions\"
# model = \"gpt-4o-mini\"
# api_key_env = \"NAYSAY_API_KEY\"
";

/// Heuristic language hint for the user message. Cheap detection: any CJK
/// ideograph or kana in the text → "Respond in Chinese". SYSTEM_PROMPT
/// already says "match the user's language", but cheap models default to
/// English anyway — this hint lives at the top of every user message so it
/// cannot be ignored.
///
/// The whole text is scanned, not just the first letter: REPL input starts
/// with an ASCII command word (`premortem 做个爬虫`), so a first-letter
/// heuristic would never fire for the very lines that need it. English
/// text essentially never contains CJK chars, so scanning everything has
/// no practical false-positive path. Returns an empty string otherwise —
/// the model falls back to its own judgement.
pub(crate) fn detect_language_hint(text: &str) -> &'static str {
    for c in text.chars() {
        let cp = c as u32;
        // CJK Unified Ideographs, Extension A, hiragana + katakana
        let is_cjk = (0x4E00..=0x9FFF).contains(&cp)
            || (0x3400..=0x4DBF).contains(&cp)
            || (0x3040..=0x30FF).contains(&cp);
        if is_cjk {
            return "[Respond in Chinese.] ";
        }
        // Cyrillic, Arabic, etc. could go here.
    }
    ""
}

/// System prompt sent with every call. Defines the interrogator role, then
/// per-command prompts add task-specific framing. Key points borrowed from
/// how Claude Code / Codex operate:
///   * no "Sure!" / "I'd be happy to..." — never pad responses
///   * think carefully before responding, then commit
///   * match the user's depth — terse questions get terse answers, deep
///     questions get deep answers
///   * never invent specifics; say when you don't know
const SYSTEM_PROMPT: &str = "\
You are naysay, the voice in the user's terminal that says no before their \
coding agents say yes. Agents execute; you interrogate. The user came here \
to stress-test an idea BEFORE handing it to an agent — surface what that \
agent will trip over, and what the user is avoiding seeing.

Rules:
- Push back by default. Agreement must be earned with specifics, not \
  granted out of politeness.
- Be concrete. \"it depends\" and \"there are risks\" are worthless — name \
  the actual failure mode, the actual number, the actual order of magnitude.
- Never open with greetings, preambles, or self-introductions. \
  Start with the substance.
- Match the user's language. If the user writes in Chinese, reply in \
  Chinese. If they write in English, reply in English. Only switch \
  language when the user explicitly asks for it.
- Match the user's depth. A terse question gets a terse answer. A deep \
  question gets a deep one.
- Never invent specific facts, file paths, or code you haven't seen. \
  If you don't know, say so.
- Don't pad responses with headers, bullet summaries of what you just \
  said, or restatements of the question.
- Think carefully before responding, then commit. No hedging.
- When the user asks for ideas or options, give genuinely interesting \
  ones — not obvious ones.
- Output plain text. No markdown unless the user asks for markdown.";

// ─── Entry point ─────────────────────────────────────────────────────────────────────────

/// Install a global panic hook that writes to a log file. If the TUI panics
/// (or anything else in naysay), the user can find the backtrace at
/// `%LOCALAPPDATA%\naysay\panic.log` (Windows) or equivalent on other OSes —
/// even if the console window closes before they can read the error.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let path = data_dir().ok().map(|d| d.join("panic.log"));
        if let Some(p) = path {
            let bt = std::backtrace::Backtrace::force_capture();
            let body = format!(
                "naysay panic at {}\n\n{}\n\n--- backtrace ---\n{}\n",
                chrono_unix_now(),
                info,
                bt
            );
            let _ = std::fs::write(&p, body);
            eprintln!("\npanicked. details written to {}", p.display());
        } else {
            eprintln!("\npanicked: {info}");
        }
    }));
}

fn chrono_unix_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("epoch={}", d.as_secs()))
        .unwrap_or_else(|_| "unknown".into())
}

#[tokio::main]
async fn main() -> Result<()> {
    install_panic_hook();
    let cli = Cli::parse();

    // --continue resolves to the newest session file up front, so a missing
    // session can be reported before any UI opens.
    let resume = if cli.continue_last {
        match latest_session() {
            Ok(p) => Some(p),
            Err(_) => {
                eprintln!("(no previous session found — starting fresh)");
                None
            }
        }
    } else {
        None
    };

    // --tui overrides default behavior (no subcommand → TUI instead of REPL)
    if cli.tui {
        return tui::run(cli.sound, cli.music, resume).await;
    }

    match cli.command {
        // No subcommand → interactive TUI launch (handles key setup itself)
        None => launch_interactive(cli.sound, cli.music, resume).await,
        Some(Command::Repl) => repl(resume).await,
        Some(Command::Seed { topic }) => {
            seed(&topic, &[], cli.save.as_deref(), cli.json).await?;
            Ok(())
        }
        Some(Command::Drill { idea }) => {
            drill(&idea, &[], cli.save.as_deref(), cli.json).await?;
            Ok(())
        }
        Some(Command::Premortem { idea }) => {
            premortem(&idea, &[], cli.save.as_deref(), cli.json).await?;
            Ok(())
        }
        Some(Command::Spec { idea }) => {
            spec(&idea, &[], cli.save.as_deref(), cli.json).await?;
            Ok(())
        }
        Some(Command::Postmortem { idea, notes }) => {
            postmortem(&idea, notes.as_deref(), &[], cli.save.as_deref(), cli.json).await?;
            Ok(())
        }
        Some(Command::Explain { path }) => {
            explain(&path, &[], cli.save.as_deref(), cli.json).await?;
            Ok(())
        }
        Some(Command::Key { action }) => match action {
            KeyAction::Set => key_set(),
            KeyAction::Status => key_status(),
            KeyAction::Delete => key_delete(),
        },
        Some(Command::Sessions { action }) => match action {
            SessionsAction::List => sessions_list(),
            SessionsAction::Show { file } => sessions_show(&file),
        },
        Some(Command::Decisions { action }) => match action {
            DecisionsAction::ById { id } => run_d_by_id(&id),
            DecisionsAction::Link { child } => run_d_link(&child),
            DecisionsAction::Unknowns => run_d_unknowns(),
        },
        Some(Command::Doctor) => doctor().await,
    }
}

// ─── Launch path (no subcommand) ────────────────────────────────────────────────────────

/// Default entry path. Checks for an API key (env → keyring), prompts for one
/// if missing, then launches the TUI. This is what runs when the user
/// double-clicks `naysay.exe` or types `naysay` with no arguments.
///
/// Goal: a non-technical user can double-click the binary and end up in the
/// TUI without needing to know what an "API key" or "environment variable" is.
/// First-run provider presets. Data, not abstraction (D-022): each row is
/// just the three fields `naysay.toml` already supports, pre-filled. A new
/// provider is a new row, never new code.
struct ProviderPreset {
    label: &'static str,
    chat_url: &'static str,
    model: &'static str,
    api_key_env: &'static str,
    needs_key: bool,
    note: &'static str,
}

const PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        label: "Ollama",
        chat_url: "http://localhost:11434/v1/chat/completions",
        model: "qwen2.5:7b",
        api_key_env: "NAYSAY_API_KEY",
        needs_key: false,
        note: "local, free, offline — no key needed",
    },
    ProviderPreset {
        label: "DeepSeek",
        chat_url: "https://api.deepseek.com/chat/completions",
        model: "deepseek-chat",
        api_key_env: "DEEPSEEK_API_KEY",
        needs_key: true,
        note: "platform.deepseek.com",
    },
    ProviderPreset {
        label: "GLM (Zhipu)",
        chat_url: "https://open.bigmodel.cn/api/paas/v4/chat/completions",
        model: "glm-4-flash",
        api_key_env: "ZHIPU_API_KEY",
        needs_key: true,
        note: "open.bigmodel.cn — glm-4-flash is free",
    },
    ProviderPreset {
        label: "OpenAI",
        chat_url: "https://api.openai.com/v1/chat/completions",
        model: "gpt-4o-mini",
        api_key_env: "OPENAI_API_KEY",
        needs_key: true,
        note: "platform.openai.com",
    },
    ProviderPreset {
        label: "MiniMax",
        chat_url: "https://api.minimax.chat/v1/text/chatcompletion_v2",
        model: "MiniMax-Text-01",
        api_key_env: "MINIMAX_API_KEY",
        needs_key: true,
        note: "api.minimax.chat — the previous default",
    },
    ProviderPreset {
        label: "OpenRouter",
        chat_url: "https://openrouter.ai/api/v1/chat/completions",
        model: "anthropic/claude-3.5-sonnet",
        api_key_env: "OPENROUTER_API_KEY",
        needs_key: true,
        note: "openrouter.ai — also carries Claude models",
    },
];

/// Is a key already configured anywhere? Deliberately does NOT touch
/// `config()`: at first run the OnceLock must stay unfrozen until the
/// picker has written the chosen provider to naysay.toml, so the TUI's
/// first config read sees the choice.
fn probe_has_key() -> bool {
    for name in [DEFAULT_API_KEY_ENV, "MINIMAX_API_KEY"] {
        if std::env::var(name).map(|v| !v.is_empty()).unwrap_or(false) {
            return true;
        }
    }
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        if matches!(entry.get_password(), Ok(k) if !k.is_empty()) {
            return true;
        }
    }
    false
}

/// "1" -> 0, " 3 " -> 2, out-of-range or garbage -> None.
fn parse_provider_choice(input: &str, max: usize) -> Option<usize> {
    let n: usize = input.trim().parse().ok()?;
    if n >= 1 && n <= max {
        Some(n - 1)
    } else {
        None
    }
}

fn is_valid_env_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// The `[provider]` table body written on first run.
fn provider_toml_body(chat_url: &str, model: &str, api_key_env: &str) -> String {
    format!(
        "[provider]\nchat_url = \"{}\"\nmodel = \"{}\"\napi_key_env = \"{}\"\n",
        chat_url, model, api_key_env
    )
}

/// One row of the first-run ASCII box: `  |   {text}<pad>|`. The frame's
/// inner width is fixed (50 columns); padding is computed so version bumps
/// can never break the alignment again.
fn setup_box_row(text: &str) -> String {
    let inner = 50usize;
    let lead = 3usize;
    let pad = inner.saturating_sub(lead + text.chars().count());
    format!("  |   {text}{}|", " ".repeat(pad))
}

async fn launch_interactive(
    sound: bool,
    music: bool,
    resume: Option<std::path::PathBuf>,
) -> Result<()> {
    // First-run: if no key is configured anywhere, walk the user through
    // the provider picker. The probe avoids config() on purpose — see
    // probe_has_key.
    if !probe_has_key() {
        eprintln!();
        eprintln!("  +--------------------------------------------------+");
        eprintln!("  |                                                  |");
        eprintln!(
            "{}",
            setup_box_row(&format!(
                "naysay v{VERSION} — says no before your agents do"
            ))
        );
        eprintln!("  |                                                  |");
        eprintln!("  |   first-time setup                               |");
        eprintln!("  |                                                  |");
        eprintln!("  +--------------------------------------------------+");
        eprintln!();
        eprintln!("   Pick a provider (any OpenAI-compatible endpoint works):");
        eprintln!();
        for (i, p) in PRESETS.iter().enumerate() {
            eprintln!("   [{:>2}] {:<12} {}", i + 1, p.label, p.note);
        }
        let custom_row = format!(
            "[{:>2}] {:<12} {}",
            PRESETS.len() + 1,
            "Custom",
            "any OpenAI-compatible endpoint"
        );
        eprintln!("   {custom_row}");
        eprintln!();
        eprintln!("   (Claude note: Anthropic's API is not OpenAI-compatible —");
        eprintln!("    pick OpenRouter and a claude-* model to use it.)");
        eprintln!();

        let choice = loop {
            eprint!("   provider [1-{}] > ", PRESETS.len() + 1);
            std::io::Write::flush(&mut std::io::stderr()).ok();
            let mut line = String::new();
            std::io::stdin()
                .lock()
                .read_line(&mut line)
                .context("read provider choice")?;
            match parse_provider_choice(line.trim(), PRESETS.len() + 1) {
                Some(n) => break n,
                None => {
                    eprintln!("   pick a number between 1 and {}", PRESETS.len() + 1)
                }
            }
        };

        let (chat_url, model, api_key_env, needs_key) = if choice < PRESETS.len() {
            let p = &PRESETS[choice];
            (
                p.chat_url.to_string(),
                p.model.to_string(),
                p.api_key_env.to_string(),
                p.needs_key,
            )
        } else {
            let url = loop {
                eprint!("   chat url (e.g. https://host/v1/chat/completions) > ");
                std::io::Write::flush(&mut std::io::stderr()).ok();
                let mut line = String::new();
                std::io::stdin()
                    .lock()
                    .read_line(&mut line)
                    .context("read url")?;
                let line = line.trim().to_string();
                if line.starts_with("http://") || line.starts_with("https://") {
                    break line;
                }
                eprintln!("   (must start with http:// or https://)");
            };
            let model = loop {
                eprint!("   model id > ");
                std::io::Write::flush(&mut std::io::stderr()).ok();
                let mut line = String::new();
                std::io::stdin()
                    .lock()
                    .read_line(&mut line)
                    .context("read model")?;
                let line = line.trim().to_string();
                if !line.is_empty() && !line.contains('"') {
                    break line;
                }
                eprintln!("   (model id required, no quotes)");
            };
            eprint!("   api key env name [NAYSAY_API_KEY] > ");
            std::io::Write::flush(&mut std::io::stderr()).ok();
            let mut env_line = String::new();
            std::io::stdin()
                .lock()
                .read_line(&mut env_line)
                .context("read env name")?;
            let env_name = env_line.trim().to_string();
            let env_name = if is_valid_env_name(&env_name) {
                env_name
            } else {
                DEFAULT_API_KEY_ENV.to_string()
            };
            (url, model, env_name, true)
        };

        let key = if needs_key {
            loop {
                eprint!("   api key  > ");
                std::io::Write::flush(&mut std::io::stderr()).ok();
                let mut line = String::new();
                std::io::stdin()
                    .lock()
                    .read_line(&mut line)
                    .context("read api key")?;
                let key = line.trim().to_string();
                if !key.is_empty() {
                    break key;
                }
                eprintln!("   (key required)");
            }
        } else {
            // Ollama ignores the key; a placeholder keeps load_api_key happy.
            "ollama-local".to_string()
        };

        // Persist the choice for every future run.
        let dir = data_dir()?;
        let path = dir.join("naysay.toml");
        std::fs::write(&path, provider_toml_body(&chat_url, &model, &api_key_env))
            .context("write naysay.toml")?;

        // Keyring for future processes.
        if needs_key {
            if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
                match entry.set_password(&key) {
                    Ok(()) => eprintln!("   + key stored in OS keyring"),
                    Err(e) => eprintln!("   (keyring unavailable: {e} — env var still set)"),
                }
            }
        } else {
            eprintln!("   + no key needed for local models");
        }

        // Env for THIS process: config() initializes later (first call in
        // tui::run) and reads the file we just wrote, but the env vars are
        // belt-and-braces for the legacy MINIMAX_API_KEY priority rule.
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var(&api_key_env, &key);
        }

        eprintln!();
        eprintln!("   + provider: {model}  ·  {}", endpoint_host(&chat_url));
        eprintln!("   launching TUI...");
        std::thread::sleep(std::time::Duration::from_millis(1200));
    }

    tui::run(sound, music, resume).await
}

// ─── v0.1 seed ───────────────────────────────────────────────────────────────────────────

async fn seed(
    topic: &str,
    history: &[Message],
    save_path: Option<&str>,
    json: bool,
) -> Result<String> {
    let prompt = format!(
        "The user wants to brainstorm around this topic: {topic}\n\n\
         Surface angles they probably haven't considered. Pick a number of \
         angles that fits the topic (5-10 usually). Each angle: short, \
         specific, surprising — not generic."
    );

    let content = call_llm(&prompt, history, 1200, 0.7).await?;
    note_usage_stderr();
    emit_output(
        "seed",
        save_path,
        json,
        &content,
        |c| format!("\n── seed: {topic} ──\n\n{c}\n"),
        |c| {
            serde_json::json!({
                "topic": topic,
                "kind": "seed",
                "lines": c.lines().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>(),
            })
        },
    )?;
    Ok(content)
}

// ─── v0.2 drill ──────────────────────────────────────────────────────────────────────────

async fn drill(
    idea: &str,
    history: &[Message],
    save_path: Option<&str>,
    json: bool,
) -> Result<String> {
    let prompt = format!(
        "The user picked this idea to drill into: {idea}\n\n\
         Break it into 3-5 actionable sub-points. Each sub-point should be \
         concrete and specific — something the user can actually do, not a \
         vague restatement of the parent idea."
    );

    let content = call_llm(&prompt, history, 800, 0.6).await?;
    note_usage_stderr();
    emit_output(
        "drill",
        save_path,
        json,
        &content,
        |c| format!("\n── drill: {idea} ──\n\n{c}\n"),
        |c| {
            serde_json::json!({
                "idea": idea,
                "kind": "drill",
                "lines": c.lines().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>(),
            })
        },
    )?;
    Ok(content)
}

// ─── premortem ──────────────────────────────────────────────────────────────────────────

async fn premortem(
    idea: &str,
    history: &[Message],
    save_path: Option<&str>,
    json: bool,
) -> Result<String> {
    let prompt = format!(
        "The user is about to commit to building this: {idea}

         Run the premortem. It is six months in the future and this project          is dead — abandoned, unmaintained, or alive but ignored. Write the          autopsy:

         1. Cause of death — the single most likely killer, stated bluntly.
         2. Ranked killers — 3-5 probable causes of death, each with the          early warning sign that was already visible on day one.
         3. Scope autopsy — which imagined features were never touched, and          which single feature everything actually depended on.
         4. The version that survived — the smallest cut of this idea that          dodges every cause of death above.
         5. Verdict — build it (at what scope) or don't (and what to do          instead).

         After the autopsy, add a short STRUCTURED section:

         ASSUMPTIONS — 3-5 things the build depends on being true. Be          specific ('a person will run this 3x/week', not 'people will want          this'). If you cannot name the assumption, name why you can't.

         EVIDENCE — for each assumption: what would prove it true? what          would prove it false? Use only known data; if you have none, say          'none yet' rather than inventing.

         UNKNOWNS — 2-4 things that, if they turned out a certain way,          would flip the verdict. Be specific about the direction of the flip.

         CONFIDENCE — a number 0..1 for the verdict itself. 0.5 means you          would change your mind for a free coffee. 0.9 means you would          bet money on it. Pick a number; do not say 'medium'.

         Be specific to this idea. Generic startup advice is worthless here."
    );

    let content = call_llm(&prompt, history, 1500, 0.6).await?;
    note_usage_stderr();
    emit_output(
        "premortem",
        save_path,
        json,
        &content,
        |c| {
            format!(
                "
── premortem: {idea} ──

{c}
"
            )
        },
        |c| {
            serde_json::json!({
                "idea": idea,
                "kind": "premortem",
                "autopsy": c,
            })
        },
    )?;
    if let Err(e) = save_decision("premortem", idea, &content, None) {
        eprintln!("decision-store: save failed: {e}");
    } else {
        eprintln!("decision-store: saved premortem under .naysay/decisions/");
    }
    Ok(content)
}

// ─── spec ───────────────────────────────────────────────────────────────────────────────

async fn spec(
    idea: &str,
    history: &[Message],
    save_path: Option<&str>,
    json: bool,
) -> Result<String> {
    let prompt = format!(
        "The user wants to hand this to a coding agent to execute: {idea}

         Write the spec the agent will receive. Assume the agent is capable          but has zero context, and will take the path of least resistance          wherever the spec is vague. Sections:

         # Goal — one paragraph: what exists when this is done, and for whom.
         # Non-goals — what this is NOT. Anything unlisted here, the agent          will build on a whim.
         # Assumptions — 2-4 things the build depends on being true. Be          specific; 'users will want this' is not an assumption, it is a hope.
         # Success criteria — 3-5 concrete, checkable conditions.
         # Failure conditions — 2-4 conditions under which the build is          considered failed regardless of whether it runs. A failure          condition is a deal-breaker; not a bug list. Examples: 'latency          > 2s', 'requires paid infrastructure', 'user cannot interpret          output without documentation'.
         # Risk budget — the worst case the user is willing to absorb          (e.g. '1 weekend of my time, $20 of infra, then kill').
         # Constraints — language, platform, budget, things that must not change.
         # Milestones — ordered; each one independently runnable or checkable.
         # Open questions — what the user must decide; the agent should ask,          not guess.

         Be concrete. A vague spec means the agent improvises, and          improvisation is where rework is born."
    );

    let content = call_llm(&prompt, history, 2000, 0.4).await?;
    note_usage_stderr();
    emit_output(
        "spec",
        save_path,
        json,
        &content,
        |c| {
            format!(
                "
── spec: {idea} ──

{c}
"
            )
        },
        |c| {
            serde_json::json!({
                "idea": idea,
                "kind": "spec",
                "spec": c,
            })
        },
    )?;
    if let Err(e) = save_decision("spec", idea, &content, None) {
        eprintln!("decision-store: save failed: {e}");
    } else {
        eprintln!("decision-store: saved spec under .naysay/decisions/");
    }
    Ok(content)
}

// ─── postmortem ─────────────────────────────────────────────────────────────────────────

async fn postmortem(
    idea: &str,
    notes: Option<&str>,
    history: &[Message],
    save_path: Option<&str>,
    json: bool,
) -> Result<String> {
    let notes_block = match notes {
        Some(n) => format!("What the user remembers about how it went:\n{n}\n"),
        None => "The user provided no notes. Reason from the idea itself and \
                 common patterns, and where the real outcome matters, say what \
                 evidence you would need instead of inventing it.\n"
            .to_string(),
    };

    let prompt = format!(
        "The project \"{idea}\" is over — shipped, killed, or quietly abandoned.\n\n\
         {notes_block}\n\
         Write the postmortem:\n\n\
         1. What actually happened — one paragraph, stated plainly. If you are \
         guessing, say so explicitly.\n\
         2. Predicted vs actual — which failure modes (or successes) were \
         foreseeable on day one? Name them as if a premortem had been run \
         before the first commit.\n\
         3. The decisive moment — the single decision that determined the \
         outcome. What was the alternative at that fork?\n\
         4. Cost accounting — time, money, and attention spent vs value \
         extracted. Use only numbers the user gave; otherwise name the \
         numbers you need from them.\n\
         5. Decision-log entry — 3-5 lines in markdown, self-contained, \
         written to paste into a DECISIONS.md: what was tried, what \
         happened, what to do differently next time.\n\n\
         Be specific to this project. Blame decisions, not people."
    );

    let content = call_llm(&prompt, history, 1500, 0.5).await?;
    note_usage_stderr();
    emit_output(
        "postmortem",
        save_path,
        json,
        &content,
        |c| format!("\n── postmortem: {idea} ──\n\n{c}\n"),
        |c| {
            serde_json::json!({
                "idea": idea,
                "kind": "postmortem",
                "notes": notes,
                "postmortem": c,
            })
        },
    )?;
    if let Err(e) = save_decision("postmortem", idea, &content, None) {
        eprintln!("decision-store: save failed: {e}");
    } else {
        eprintln!("decision-store: saved postmortem under .naysay/decisions/");
    }
    Ok(content)
}

// ─── v0.5 explain ──────────────────────────────────────────────────────────────────────

async fn explain(
    path: &str,
    history: &[Message],
    save_path: Option<&str>,
    json: bool,
) -> Result<String> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;

    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    // Default MiniMax model has ~32k token context; keep input comfortably below.
    let max_chars = 24_000;
    let truncated = if content.len() > max_chars {
        eprintln!(
            "(file is {} chars; truncating to first {})",
            content.len(),
            max_chars
        );
        &content[..max_chars]
    } else {
        &content
    };

    let prompt = format!(
        "Walk a developer through this file. Be concrete: name the actual \
         functions, structs, or blocks that matter. Skip boilerplate (imports, \
         constants, blank lines). For each non-trivial chunk, give intent \
         (what it's for) and mechanism (how it works) in one or two sentences \
         each. End with 2-4 questions the reader should think about — only \
         questions the code actually raises.\n\n\
         File: {path}\n\
         Language hint: {ext}\n\n\
         Content:\n```\n{truncated}\n```"
    );

    let explanation = call_llm(&prompt, history, 2500, 0.4).await?;
    note_usage_stderr();
    emit_output(
        "explain",
        save_path,
        json,
        &explanation,
        |c| format!("\n━━ explain: {path} ━━\n\n{c}\n"),
        |c| {
            serde_json::json!({
                "file": path,
                "kind": "explain",
                "explanation": c,
            })
        },
    )?;
    Ok(explanation)
}

// ─── v0.3 REPL ───────────────────────────────────────────────────────────────────────────

/// Mutable state for the plain REPL: conversation memory, context window,
/// and the session log. The TUI keeps the equivalent in `TuiState`.
struct ReplState {
    /// Full conversation as wire messages. User turns carry the language
    /// hint from birth (same convention as the TUI's build_context), so an
    /// old turn can't drag the model back to English.
    history: Vec<Message>,
    /// How many user/assistant pairs are sent with each call (0..=10).
    context_turns: usize,
    /// Session JSONL; user inputs are logged by the loop, assistant
    /// responses by `record`.
    session_path: Option<PathBuf>,
}

impl ReplState {
    /// The message window to send with the next call: the last
    /// `context_turns` user/assistant pairs, oldest first.
    fn context(&self) -> Vec<Message> {
        let take = (self.context_turns * 2).min(self.history.len());
        self.history[self.history.len() - take..].to_vec()
    }

    /// Append one completed exchange to the memory and the session log.
    /// The user side is the full input line (matching what the TUI sends as
    /// context); it gets the language hint here, once, at birth.
    fn record(&mut self, user_line: &str, assistant: &str) {
        self.history.push(Message {
            role: "user".into(),
            content: format!("{}{user_line}", detect_language_hint(user_line)),
        });
        self.history.push(Message {
            role: "assistant".into(),
            content: assistant.into(),
        });
        if let Some(ref p) = self.session_path {
            log_event(p, "assistant", assistant);
        }
    }
}

async fn repl(resume: Option<std::path::PathBuf>) -> Result<()> {
    // Try to open a session log; non-fatal if it fails.
    let mut st = ReplState {
        history: Vec::new(),
        context_turns: 3,
        session_path: open_session_log().ok(),
    };

    println!("naysay — the voice that says no first");
    if let Some(ref p) = st.session_path {
        println!("(logging to {})", p.display());
    }

    // --continue: replay the previous session into REPL memory so
    // follow-ups keep their context. Replayed user turns get the language
    // hint here too, and new turns append to the same session file.
    if let Some(ref path) = resume {
        match load_session_records(path) {
            Ok(records) => {
                let n = records.len();
                for r in &records {
                    let is_user = r.kind == "user";
                    st.history.push(Message {
                        role: if is_user { "user" } else { "assistant" }.into(),
                        content: if is_user {
                            format!("{}{}", detect_language_hint(&r.text), r.text)
                        } else {
                            r.text.clone()
                        },
                    });
                }
                st.session_path = Some(path.clone());
                println!(
                    "(resumed {n} turns from {} — new turns append to the same session)",
                    path.display()
                );
            }
            Err(e) => eprintln!("resume failed: {e:#}"),
        }
    }

    println!("type `help` for commands, `quit` to exit\n");

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        // Prompt
        print!("naysay> ");
        io::stdout().flush().context("flush stdout")?;

        // Read line
        let mut line = String::new();
        let n = reader.read_line(&mut line).context("read stdin")?;
        if n == 0 {
            // EOF (Ctrl-D / piped input ended)
            println!();
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Log user input (best-effort)
        if let Some(ref p) = st.session_path {
            log_input(p, line);
        }

        // Dispatch
        match dispatch_repl(line, &mut st).await {
            Ok(ReplAction::Continue) => {}
            Ok(ReplAction::Exit) => break,
            Err(e) => eprintln!("error: {e:#}"),
        }
    }

    println!("bye.");
    Ok(())
}

enum ReplAction {
    Continue,
    Exit,
}

async fn dispatch_repl(line: &str, st: &mut ReplState) -> Result<ReplAction> {
    // Naive split: first word = command, rest = args
    let mut parts = line.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim();

    match cmd {
        "help" | "?" => {
            let help_text = "commands:\n  \
                 premortem <idea>    assume it died in 6 months — the autopsy\n  \
                 postmortem <idea>   it's over — the review + decision-log entry\n  \
                 spec <idea>         harden an idea into a spec for your agent\n  \
                 seed <topic>        brainstorm 8 angles\n  \
                 drill <idea>        drill into an idea\n  \
                 explain <file>      walk through code\n  \
                 /context N          prior turns the AI sees, 0..=10 (now {n})\n  \
                 /clear              wipe REPL conversation memory\n  \
                 key set|status|del  manage API key\n  \
                 sessions list|show  browse past sessions\n  \
                 quit | exit         leave naysay\n\n\
                 the AI sees your last few turns, so follow-ups work:\n\
                 \"what about X?\" or \"drill into #2\"."
                .replace("{n}", &st.context_turns.to_string());
            println!("{help_text}");
            Ok(ReplAction::Continue)
        }
        "quit" | "exit" | "q" => Ok(ReplAction::Exit),
        "seed" => {
            if rest.is_empty() {
                eprintln!("usage: seed <topic>");
            } else {
                let ctx = st.context();
                let content = seed(rest, &ctx, None, false).await?;
                st.record(line, &content);
            }
            Ok(ReplAction::Continue)
        }
        "drill" => {
            if rest.is_empty() {
                eprintln!("usage: drill <idea>");
            } else {
                let ctx = st.context();
                let content = drill(rest, &ctx, None, false).await?;
                st.record(line, &content);
            }
            Ok(ReplAction::Continue)
        }
        "premortem" => {
            if rest.is_empty() {
                eprintln!("usage: premortem <idea>");
            } else {
                let ctx = st.context();
                let content = premortem(rest, &ctx, None, false).await?;
                st.record(line, &content);
            }
            Ok(ReplAction::Continue)
        }
        "spec" => {
            if rest.is_empty() {
                eprintln!("usage: spec <idea>");
            } else {
                let ctx = st.context();
                let content = spec(rest, &ctx, None, false).await?;
                st.record(line, &content);
            }
            Ok(ReplAction::Continue)
        }
        "postmortem" => {
            if rest.is_empty() {
                eprintln!("usage: postmortem <idea> [notes]");
            } else {
                // `postmortem <idea> -- notes text` keeps one-arg ergonomics
                // while allowing context; anything after " -- " is the notes.
                let (idea, notes) = match rest.split_once(" -- ") {
                    Some((i, n)) => (i.trim(), Some(n.trim())),
                    None => (rest, None),
                };
                let ctx = st.context();
                let content = postmortem(idea, notes, &ctx, None, false).await?;
                st.record(line, &content);
            }
            Ok(ReplAction::Continue)
        }
        "explain" => {
            if rest.is_empty() {
                eprintln!("usage: explain <file>");
            } else {
                let ctx = st.context();
                let content = explain(rest, &ctx, None, false).await?;
                st.record(line, &content);
            }
            Ok(ReplAction::Continue)
        }
        "/context" | ":context" => {
            // Mirror the TUI's /context: bare shows the current window,
            // N (0..=10) sets it.
            if rest.is_empty() {
                println!("context = {} turns", st.context_turns);
            } else {
                match rest.parse::<usize>() {
                    Ok(n) if n <= 10 => {
                        st.context_turns = n;
                        println!("context = {n} turns");
                    }
                    Ok(n) => eprintln!("/context N: N must be 0..=10 (got {n})"),
                    Err(e) => eprintln!("/context N: not a number ({e})"),
                }
            }
            Ok(ReplAction::Continue)
        }
        "/clear" | ":clear" => {
            let dropped = st.history.len();
            st.history.clear();
            println!("(cleared {dropped} remembered messages)");
            Ok(ReplAction::Continue)
        }
        "key" => {
            // `key set` | `key status` | `key delete`
            let r = match rest {
                "set" => key_set(),
                "status" => key_status(),
                "delete" | "del" => key_delete(),
                "" => {
                    eprintln!("usage: key <set|status|delete>");
                    Ok(())
                }
                other => {
                    eprintln!("unknown key action: `{other}`. use set/status/delete");
                    Ok(())
                }
            };
            if let Err(e) = r {
                eprintln!("error: {e:#}");
            }
            Ok(ReplAction::Continue)
        }
        "sessions" => {
            let r = match rest {
                "list" | "" => sessions_list(),
                other if !other.is_empty() => sessions_show(other),
                _ => {
                    eprintln!("usage: sessions list | sessions show <file>");
                    Ok(())
                }
            };
            if let Err(e) = r {
                eprintln!("error: {e:#}");
            }
            Ok(ReplAction::Continue)
        }
        "d-by-id" if rest.is_empty() => {
            eprintln!("usage: d-by-id <id>");
            Ok(ReplAction::Continue)
        }
        "d-by-id" => {
            run_d_by_id(rest).map_err(|e| anyhow::anyhow!("{e:#}"))?;
            Ok(ReplAction::Continue)
        }
        "d-link" if rest.is_empty() => {
            eprintln!("usage: d-link <child-id>");
            Ok(ReplAction::Continue)
        }
        "d-link" => {
            run_d_link(rest).map_err(|e| anyhow::anyhow!("{e:#}"))?;
            Ok(ReplAction::Continue)
        }
        "d-unknowns" => {
            run_d_unknowns().map_err(|e| anyhow::anyhow!("{e:#}"))?;
            Ok(ReplAction::Continue)
        }
        other => {
            eprintln!("unknown command: `{other}`. type `help` for the list.");
            Ok(ReplAction::Continue)
        }
    }
}

// ─── LLM helper ─────────────────────────────────────────────────────────────────────────

/// Send `prompt` (and optional prior turns) to the LLM and return the
/// assistant's reply text.
///
/// The system prompt is always prepended. `history` is the conversation so
/// far — older turns first, newest turn last. Pass `&[]` for a single-turn
/// call. Pass the last N user/assistant pairs to give the LLM context
/// across a multi-turn session.
/// Set while the TUI owns the terminal. Retry backoff notes go to stderr,
/// which would garble the TUI's two live rows — so retries are silent in
/// TUI mode (its status line already shows liveness).
static TUI_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn set_tui_active(active: bool) {
    use std::sync::atomic::Ordering;
    TUI_ACTIVE.store(active, Ordering::SeqCst);
}

/// Backoff for attempt `n` (0-based): 1s, 2s, 4s…
fn backoff_secs(attempt: u32) -> u64 {
    1 << attempt
}

/// Retryable statuses: rate limiting and server-side errors. Client errors
/// (4xx other than 429) fail immediately — the request is wrong, not busy.
fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

const MAX_RETRIES: u32 = 2;

/// POST the chat request with basic resilience: 429 and 5xx responses are
/// retried up to `MAX_RETRIES` times with exponential backoff. The response
/// body is never consumed before the retry decision, so a retry is always
/// safe to issue.
async fn post_chat_with_retry(req: &ChatRequest) -> Result<reqwest::Response> {
    let api_key = load_api_key().context(format!(
        "no API key — set {} or run `naysay key set`",
        config().api_key_env
    ))?;
    let client = Client::builder()
        // Bound the worst case ("hung forever on connect") without capping
        // total response time — long generations are legitimate.
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .context("build http client")?;

    let mut attempt: u32 = 0;
    loop {
        let resp = client
            .post(&config().chat_url)
            .bearer_auth(&api_key)
            .json(req)
            .send()
            .await
            .context("HTTP request failed — check your connection, or run `naysay doctor`")?;
        let status = resp.status();
        if !is_retryable_status(status) || attempt >= MAX_RETRIES {
            return Ok(resp);
        }
        let wait = std::time::Duration::from_secs(backoff_secs(attempt));
        if !TUI_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
            eprintln!(
                "\u{b7} {status} — retrying in {:?} (attempt {}/{})",
                wait,
                attempt + 1,
                MAX_RETRIES
            );
        }
        tokio::time::sleep(wait).await;
        attempt += 1;
    }
}

pub(crate) async fn call_llm(
    prompt: &str,
    history: &[Message],
    max_tokens: u32,
    temperature: f32,
) -> Result<String> {
    call_llm_with_model(&config().model, prompt, history, max_tokens, temperature).await
}

/// Inner implementation. Takes the model name explicitly so the TUI can pass
/// its user-selected value (set via `/model`). CLI/REPL callers use the
/// `call_llm` wrapper which always pins to the configured default model.
pub(crate) async fn call_llm_with_model(
    model: &str,
    prompt: &str,
    history: &[Message],
    max_tokens: u32,
    temperature: f32,
) -> Result<String> {
    let mut messages: Vec<Message> = Vec::with_capacity(history.len() + 2);
    messages.push(Message {
        role: "system".into(),
        content: SYSTEM_PROMPT.into(),
    });
    messages.extend_from_slice(history);
    let user_msg = format!("{}{prompt}", detect_language_hint(prompt));
    messages.push(Message {
        role: "user".into(),
        content: user_msg,
    });

    let req = ChatRequest {
        model: model.into(),
        messages,
        max_tokens,
        temperature,
        stream: false,
    };

    let resp = post_chat_with_retry(&req)
        .await?
        .error_for_status()
        .context("endpoint returned non-2xx")?
        .json::<ChatResponse>()
        .await
        .context("failed to parse response JSON")?;

    if let Some(u) = resp.usage {
        store_last_usage(u);
    }

    let content = resp
        .choices
        .into_iter()
        .next()
        .context("response had no choices")?
        .message
        .content;

    Ok(content)
}

/// Streaming variant of `call_llm`. Emits each content delta to `on_delta`
/// as it arrives from the server. Returns the full concatenated response on
/// success, or the first error encountered.
///
/// The wire protocol is Server-Sent Events: lines like
///     data: {"choices":[{"delta":{"content":"hello"}}]}
/// separated by blank lines, with a final `data: [DONE]` to mark end-of-stream.
/// The endpoint, model, and auth are shared with `call_llm`.
pub(crate) async fn call_llm_stream<F>(
    model: &str,
    prompt: &str,
    history: &[Message],
    max_tokens: u32,
    temperature: f32,
    mut on_delta: F,
) -> Result<String>
where
    F: FnMut(&str) + Send,
{
    use futures_util::StreamExt;

    let mut messages: Vec<Message> = Vec::with_capacity(history.len() + 2);
    messages.push(Message {
        role: "system".into(),
        content: SYSTEM_PROMPT.into(),
    });
    messages.extend_from_slice(history);
    let user_msg = format!("{}{prompt}", detect_language_hint(prompt));
    messages.push(Message {
        role: "user".into(),
        content: user_msg,
    });

    let req = ChatRequest {
        model: model.into(),
        messages,
        max_tokens,
        temperature,
        stream: true,
    };

    let resp = post_chat_with_retry(&req)
        .await?
        .error_for_status()
        .context("endpoint returned non-2xx")?;

    let mut byte_stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut full = String::new();
    let mut sink = |delta: &str| {
        full.push_str(delta);
        on_delta(delta);
    };

    let mut last_usage: Option<Usage> = None;
    while let Some(chunk) = byte_stream.next().await {
        let bytes = chunk.context("failed reading response chunk")?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));
        match drain_sse_events(&mut buffer, &mut sink) {
            Ok(u) => {
                if u.is_some() {
                    last_usage = u;
                }
            }
            Err(e) => return Err(anyhow::anyhow!("SSE parser failed mid-stream: {e}")),
        }
    }
    if let Some(u) = last_usage {
        store_last_usage(u);
    }

    Ok(full)
}

/// Drain complete SSE events from `buffer`, calling `on_delta` for each
/// content piece. The buffer keeps any partial trailing event; the next
/// call (with more bytes) finishes it.
///
/// SSE spec: events are separated by blank lines (`\n\n`). Each event is a
/// series of `field: value` lines. We only care about `data:` lines; other
/// fields (`event:`, `id:`, `retry:`) are ignored. The terminal event has
/// payload `[DONE]` and is also ignored.
fn drain_sse_events(
    buffer: &mut String,
    on_delta: &mut dyn FnMut(&str),
) -> Result<Option<Usage>, String> {
    let mut usage: Option<Usage> = None;
    while let Some(idx) = buffer.find("\n\n") {
        let event: String = buffer.drain(..idx + 2).collect();
        for line in event.lines() {
            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            if payload.is_empty() || payload == "[DONE]" {
                continue;
            }
            // Malformed JSON must not abort the stream. The server may
            // emit a finish-reason-only chunk at the end (`{"choices":[
            // {"finish_reason":"stop"}]}` with no `delta` key) or even
            // genuinely broken JSON; either way, deltas already received
            // are still in `full` on the caller side, and skipping
            // preserves them.
            let parsed: ChatChunk = match serde_json::from_str(payload) {
                Ok(c) => c,
                Err(e) => {
                    // Only log at debug verbosity — this used to be a
                    // hard error before, now it's expected noise.
                    eprintln!("[naysay] skip unparseable SSE chunk: {e}");
                    continue;
                }
            };
            if let Some(u) = parsed.usage {
                usage = Some(u);
            }
            if let Some(choice) = parsed.choices.into_iter().next() {
                let piece = choice.delta.content;
                if !piece.is_empty() {
                    on_delta(&piece);
                }
            }
        }
    }
    Ok(usage)
}

/// Strip leading "1." / "1、" / "1:" / "- " / "* " markers if the model emitted them,
/// and prefix each non-empty line with "N. ".
/// (Kept for the test suite — production paths use raw line iteration now.)
#[allow(dead_code)]
fn number_lines(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cleaned = line
            .trim_start_matches(|c: char| {
                c.is_ascii_digit() || matches!(c, '.' | ',' | '、' | ':' | '-' | '*' | ' ')
            })
            .trim();
        out.push(format!("{}. {}", i + 1, cleaned));
    }
    out
}

/// Single output dispatcher used by seed/drill/explain.
/// - json=false: render via `pretty(content)` and write to file or stdout
/// - json=true: build a JSON object via `json_builder(content)` and serialize
fn emit_output(
    label: &str,
    save_path: Option<&str>,
    json: bool,
    content: &str,
    pretty: impl Fn(&str) -> String,
    json_builder: impl Fn(&str) -> serde_json::Value,
) -> Result<()> {
    let payload = if json {
        serde_json::to_string_pretty(&json_builder(content))
            .with_context(|| format!("serialize {label} as JSON"))?
    } else {
        pretty(content)
    };

    match save_path {
        Some(p) => {
            std::fs::write(p, &payload).with_context(|| format!("write output to {p}"))?;
            eprintln!("✓ saved {label} output to {p}");
        }
        None => print!("{payload}"),
    }
    Ok(())
}

// ─── v0.7 credential storage (OS keyring) ──────────────────────────────────────

pub(crate) fn load_api_key() -> Result<String> {
    let env_name = config().api_key_env.clone();

    // 1. Configured env var wins (escape hatch for CI / scripted use)
    if let Ok(k) = std::env::var(&env_name) {
        if !k.is_empty() {
            return Ok(k);
        }
    }
    // 2. Legacy pair-era env var, so old setups keep working after the rename
    if let Ok(k) = std::env::var("MINIMAX_API_KEY") {
        if !k.is_empty() {
            return Ok(k);
        }
    }

    // 3. OS keyring
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("failed to access OS keyring")?;
    match entry.get_password() {
        Ok(k) => Ok(k),
        Err(keyring::Error::NoEntry) => Err(anyhow::anyhow!(
            "no API key configured. Run `naysay key set` or set {env_name}"
        )),
        Err(e) => Err(anyhow::anyhow!("keyring read failed: {e}")),
    }
}

fn key_set() -> Result<()> {
    print!("API key (input hidden): ");
    io::stdout().flush()?;
    let mut key = String::new();
    io::stdin()
        .lock()
        .read_line(&mut key)
        .context("read input")?;
    let key = key.trim().to_string();
    if key.is_empty() {
        anyhow::bail!("empty key");
    }
    if key.len() < 8 {
        eprintln!("(warning: key looks very short)");
    }

    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("failed to access OS keyring")?;
    entry
        .set_password(&key)
        .context("failed to write to OS keyring")?;

    println!("✓ saved to OS keyring (service=`{KEYRING_SERVICE}`, user=`{KEYRING_USER}`)");
    Ok(())
}

fn key_status() -> Result<()> {
    // Env vars win for status display too (configured name first, then legacy)
    for name in [config().api_key_env.as_str(), "MINIMAX_API_KEY"] {
        if let Ok(k) = std::env::var(name) {
            if !k.is_empty() {
                println!("✓ {name} env var is set ({} chars)", k.len());
                return Ok(());
            }
        }
    }

    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("failed to access OS keyring")?;
    match entry.get_password() {
        Ok(k) => println!("✓ OS keyring has key ({} chars)", k.len()),
        Err(keyring::Error::NoEntry) => println!("✗ no API key configured"),
        Err(e) => return Err(anyhow::anyhow!("keyring read failed: {e}")),
    }
    Ok(())
}

fn key_delete() -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("failed to access OS keyring")?;
    match entry.delete_credential() {
        Ok(()) => println!("✓ removed from OS keyring"),
        Err(keyring::Error::NoEntry) => println!("(no key to delete)"),
        Err(e) => return Err(anyhow::anyhow!("keyring delete failed: {e}")),
    }
    Ok(())
}

// ─── v0.11 doctor ──────────────────────────────────────────────────────────────────────

/// Diagnostics: config validity, API key, sessions dir writability, and
/// endpoint reachability. Exits non-zero if any check fails so it can be
/// piped to CI / scripts.
async fn doctor() -> Result<()> {
    println!("naysay doctor\n");

    let mut failures = 0;

    // 1. Config (naysay.toml)
    print!("  [1/4] Config (naysay.toml)  ");
    let cfg_path = data_dir().ok().map(|d| d.join("naysay.toml"));
    match cfg_path.filter(|p| p.exists()) {
        None => {
            println!("\u{2713} no file (embedded defaults)");
        }
        Some(path) => {
            let raw = std::fs::read_to_string(&path).unwrap_or_default();
            match Config::parse_strict(&raw) {
                Err(e) => {
                    println!("\u{2717} malformed TOML: {e}");
                    println!("         hint: fix or delete {}", path.display());
                    failures += 1;
                }
                Ok(_) => {
                    let issues = config().validate();
                    if issues.is_empty() {
                        println!("\u{2713} ok ({})", path.display());
                    } else {
                        println!("\u{2717} {} issue(s):", issues.len());
                        for i in issues {
                            println!("           - {i}");
                        }
                        failures += 1;
                    }
                }
            }
        }
    }

    // 2. API key
    print!("  [2/4] API key ............. ");
    match load_api_key() {
        Ok(k) => {
            let source = if std::env::var(&config().api_key_env)
                .ok()
                .filter(|v| !v.is_empty())
                .is_some()
            {
                format!("{} env", config().api_key_env)
            } else if std::env::var("MINIMAX_API_KEY")
                .ok()
                .filter(|v| !v.is_empty())
                .is_some()
            {
                "MINIMAX_API_KEY env (legacy)".to_string()
            } else {
                "OS keyring".to_string()
            };
            println!("✓ ok ({source}, {} chars)", k.len());
        }
        Err(e) => {
            println!("✗ fail — {e:#}");
            println!("         hint: run `naysay key set` or set NAYSAY_API_KEY");
            failures += 1;
        }
    }

    // 3. Sessions dir
    print!("  [3/4] Sessions dir ........ ");
    match session_dir() {
        Ok(p) => {
            // Probe write by creating + deleting a sentinel file.
            let probe = p.join(".doctor-write-probe");
            match std::fs::write(&probe, "ok") {
                Ok(()) => {
                    let _ = std::fs::remove_file(&probe);
                    println!("✓ writable ({})", p.display());
                }
                Err(e) => {
                    println!("✗ not writable: {e}");
                    println!("         hint: check permissions on {}", p.display());
                    failures += 1;
                }
            }
        }
        Err(e) => {
            println!("✗ fail — {e:#}");
            failures += 1;
        }
    }

    // 4. Endpoint reachability — short timeout, GET (some servers reject HEAD).
    print!("  [4/4] Chat endpoint ...... ");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| anyhow::anyhow!("build http client: {e}"))?;
    match client.get(&config().chat_url).send().await {
        Ok(r) => {
            // Any HTTP response (even 4xx) means we reached the server.
            println!("✓ reachable (HTTP {})", r.status().as_u16());
        }
        Err(e) => {
            println!("✗ unreachable: {e}");
            println!("         hint: check your network / proxy / firewall");
            failures += 1;
        }
    }

    println!();
    if failures == 0 {
        println!("✓ all checks passed");
        Ok(())
    } else {
        println!("✗ {failures} check(s) failed");
        std::process::exit(1);
    }
}

// ─── v0.8 session persistence ────────────────────────────────────────────────────

use std::path::PathBuf;

/// Local data directory for sessions and similar state.
fn data_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("APPDATA"))
        .context("neither LOCALAPPDATA nor APPDATA is set")?;

    #[cfg(target_os = "macos")]
    let base = {
        let home = std::env::var("HOME").context("HOME not set")?;
        format!("{home}/Library/Application Support")
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var("XDG_DATA_HOME")
        .or_else(|_| std::env::var("HOME").map(|h| format!("{h}/.local/share")))
        .context("XDG_DATA_HOME and HOME both unset")?;

    let dir = PathBuf::from(base).join("naysay");
    std::fs::create_dir_all(&dir).context("create naysay data dir")?;
    Ok(dir)
}

fn session_dir() -> Result<PathBuf> {
    let dir = data_dir()?.join("sessions");
    std::fs::create_dir_all(&dir).context("create sessions dir")?;
    Ok(dir)
}

/// Open a new session log file. Returns the path so the REPL / TUI can
/// append user inputs and print it on exit.
pub(crate) fn open_session_log() -> Result<PathBuf> {
    let dir = session_dir()?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = dir.join(format!("session-{ts}.jsonl"));
    // Touch the file so it exists even before first input.
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .context("open session log")?;
    Ok(path)
}

/// Append one user input line to the session log (best-effort).
pub(crate) fn log_input(path: &PathBuf, line: &str) {
    log_event(path, "user", line);
}

/// Append one event (kind: "user" | "assistant") to the session log
/// (best-effort). Both sides are logged so a future `--continue` can
/// replay the whole conversation, not just the prompts.
pub(crate) fn log_event(path: &PathBuf, kind: &str, text: &str) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let record = serde_json::json!({
        "t": ts,
        "kind": kind,
        "text": text,
    });
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(path) {
        let _ = writeln!(f, "{record}");
    }
}

fn sessions_list() -> Result<()> {
    let dir = session_dir()?;
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .context("read sessions dir")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "jsonl").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("(no sessions yet — start one with `naysay`)");
        return Ok(());
    }

    println!("{:<25} {:>10}  PATH", "FILE", "BYTES");
    for e in entries.iter().rev().take(20) {
        let meta = e.metadata().ok();
        let size = meta.map(|m| m.len()).unwrap_or(0);
        let name = e.file_name().to_string_lossy().into_owned();
        println!("{name:<25} {size:>10}  {}", e.path().display());
    }
    Ok(())
}

/// One replayed turn from a session log.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionRecord {
    pub kind: String,
    pub text: String,
}

/// Parse a session JSONL file into (kind, text) records. Skips malformed
/// lines, empty texts, and unknown kinds (boot markers etc.) — resume must
/// never fail because of one bad line.
pub(crate) fn load_session_records(path: &std::path::Path) -> Result<Vec<SessionRecord>> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let kind = v["kind"].as_str().unwrap_or("").to_string();
        if kind != "user" && kind != "assistant" {
            continue;
        }
        let text = v["text"].as_str().unwrap_or("").to_string();
        if text.is_empty() {
            continue;
        }
        out.push(SessionRecord { kind, text });
    }
    Ok(out)
}

/// Path of the newest session in `dir`, or `None` when there is none.
/// Sessions are named `session-<epoch>.jsonl`, so lexicographic order is
/// chronological. Non-jsonl files are ignored.
fn newest_session_in(dir: &std::path::Path) -> Option<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "jsonl").unwrap_or(false))
        .map(|e| e.path())
        .collect();
    entries.sort();
    entries.pop()
}

/// Path of the most recent session file, for `--continue` and bare `/resume`.
pub(crate) fn latest_session() -> Result<PathBuf> {
    let dir = session_dir()?;
    newest_session_in(&dir).context("no sessions yet")
}

/// Resolve a `/resume <arg>` / `sessions show <arg>` argument to a session
/// file path. Accepts bare digits ("1787486027"), a full filename
/// ("session-1787486027.jsonl"), or any filename containing a dot.
pub(crate) fn resolve_session_arg(input: &str) -> Result<PathBuf> {
    let dir = session_dir()?;
    Ok(resolve_session_in(&dir, input))
}

/// Pure path math for `resolve_session_arg`, split out so tests don't touch
/// the real sessions dir.
fn resolve_session_in(dir: &std::path::Path, input: &str) -> PathBuf {
    if input.contains('.') {
        dir.join(input)
    } else {
        dir.join(format!("session-{input}.jsonl"))
    }
}

fn sessions_show(input: &str) -> Result<()> {
    let path = resolve_session_arg(input)?;

    if !path.exists() {
        anyhow::bail!("session not found: {}", path.display());
    }

    let content =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;

    println!("\n━━ {} ━━\n", path.display());
    for line in content.lines() {
        // Try to pretty-print each JSONL record; fall back to raw line.
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => {
                let kind = v["kind"].as_str().unwrap_or("?");
                let text = v["text"].as_str().unwrap_or("");
                let arrow = match kind {
                    "user" => "▸ you",
                    "assistant" => "◂ naysay",
                    _ => "·",
                };
                println!("{arrow} {text}");
            }
            Err(_) => println!("{line}"),
        }
    }
    println!();
    Ok(())
}

// ─── v0.3 decision store ──────────────────────────────────────────────────────────────────────

/// One saved decision. A flat JSON per file under `.naysay/decisions/
/// <kind>-<id>.json`. No schema-version field: the shape may drift across
/// naysay versions because the user owns the file and grep is the only
/// API surface this store promises.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DecisionRecord {
    /// 12 hex chars. Minted from a hash of wall-clock nanos; collision
    /// retries up to 8 times before giving up.
    pub id: String,
    /// "premortem" | "spec" | "postmortem"
    pub kind: String,
    /// epoch seconds at write time
    pub ts: u64,
    pub idea: String,
    /// Optional predecessor id, wired by the user in a future revision.
    pub parent: Option<String>,
    /// Full body exactly as the model produced it. Never parsed.
    pub body: String,
    /// The structured sections v0.2 taught the prompts to emit. Extracted
    /// by substring scan; empty when the model skipped them. Not validated.
    pub assumptions: Vec<String>,
    pub evidence: Vec<String>,
    pub unknowns: Vec<String>,
    pub failure_conditions: Vec<String>,
    /// 0..=100 when the model emitted a confidence number.
    pub confidence: Option<u8>,
}

/// The store lives in the current working directory: `.naysay/decisions/`.
/// Cwd-local by design (D-021): the user chooses which directory is a
/// project, and therefore which decisions belong together.
fn decisions_dir() -> std::io::Result<PathBuf> {
    let dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".naysay")
        .join("decisions");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 12 hex chars from a hash of wall-clock nanos. The caller retries on
/// collision; at hundreds of records a collision is vanishingly rare.
fn make_decision_id(nanos: u128) -> String {
    let mut h: u64 = (nanos as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    let bytes = h.to_be_bytes();
    let mut out = String::with_capacity(12);
    for b in &bytes[..6] {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Extract the bullet list under a section heading. Forgiving by design:
/// accepts `## HEADING`, `# HEADING`, and bare `HEADING:`; grabs `- ` /
/// `* ` / `1. ` bullets until a blank line or the next heading. Returns
/// an empty list when the heading is absent — never an error.
fn extract_section(body: &str, heading: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section = false;
    let want = heading.trim().trim_end_matches(':').to_ascii_lowercase();
    for raw in body.lines() {
        let line = raw.trim_end();
        if !in_section {
            let head = line.trim_start_matches('#').trim().trim_end_matches(':');
            if head.to_ascii_lowercase() == want {
                in_section = true;
            }
        } else {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                in_section = false;
                continue;
            }
            if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
                out.push(rest.trim().to_string());
            } else {
                let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
                if !digits.is_empty() && digits.len() <= 3 {
                    let after = &t[digits.len()..];
                    if let Some(rest) = after.strip_prefix(". ").or_else(|| after.strip_prefix(')'))
                    {
                        out.push(rest.trim().to_string());
                        continue;
                    }
                }
                in_section = false;
            }
        }
    }
    out
}

/// Parse the confidence number out of a line mentioning CONFIDENCE.
/// Accepts "0.62" and "62"; returns 0..=100.
fn extract_confidence(body: &str) -> Option<u8> {
    for raw in body.lines() {
        let t = raw.trim();
        if !t.to_uppercase().contains("CONFIDENCE") {
            continue;
        }
        let mut digits = String::new();
        let mut seen_dot = false;
        for ch in t.chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
            } else if ch == '.' && !seen_dot && !digits.is_empty() {
                digits.push(ch);
                seen_dot = true;
            } else if !digits.is_empty() {
                break;
            }
        }
        if let Ok(v) = digits.parse::<f64>() {
            let scaled = if v <= 1.0 { v * 100.0 } else { v };
            return Some(scaled.round().clamp(0.0, 100.0) as u8);
        }
    }
    None
}

/// Core save, parameterized by directory so tests can use a temp dir.
fn save_decision_to(
    dir: &std::path::Path,
    kind: &str,
    idea: &str,
    body: &str,
    parent: Option<&str>,
    nanos: u128,
) -> std::io::Result<String> {
    std::fs::create_dir_all(dir)?;
    for _ in 0..8 {
        let id = make_decision_id(nanos.wrapping_add(1));
        let path = dir.join(format!("{}-{}.json", kind, id));
        if path.exists() {
            continue;
        }
        let rec = DecisionRecord {
            id: id.clone(),
            kind: kind.to_string(),
            ts: (nanos / 1_000_000_000) as u64,
            idea: idea.to_string(),
            parent: parent.map(|s| s.to_string()),
            body: body.to_string(),
            assumptions: extract_section(body, "ASSUMPTIONS"),
            evidence: extract_section(body, "EVIDENCE"),
            unknowns: extract_section(body, "UNKNOWNS"),
            failure_conditions: extract_section(body, "FAILURE CONDITIONS"),
            confidence: extract_confidence(body),
        };
        let json = serde_json::to_string_pretty(&rec).map_err(std::io::Error::other)?;
        std::fs::write(&path, json)?;
        return Ok(id);
    }
    Err(std::io::Error::other(
        "could not mint a fresh decision id after 8 tries",
    ))
}

/// Save into the cwd store. Best-effort: callers print the error and move
/// on — a failed save must never break the command's primary output.
fn save_decision(
    kind: &str,
    idea: &str,
    body: &str,
    parent: Option<&str>,
) -> std::io::Result<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = decisions_dir()?;
    save_decision_to(&dir, kind, idea, body, parent, now)
}

fn read_record_by_id(dir: &std::path::Path, id: &str) -> Option<DecisionRecord> {
    let short = id.splitn(2, '-').last().unwrap_or(id);
    let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem == id || stem.ends_with(&format!("-{}", short)) {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if let Ok(rec) = serde_json::from_str::<DecisionRecord>(&raw) {
                    return Some(rec);
                }
            }
        }
    }
    None
}

// ─── v0.3 query commands ───────────────────────────────────────────────────────────────────

fn run_d_by_id(id: &str) -> Result<()> {
    let dir = decisions_dir().context("decision store not accessible")?;
    let Some(rec) = read_record_by_id(&dir, id) else {
        anyhow::bail!("no decision found for id: {id}");
    };
    let json =
        serde_json::to_string_pretty(&rec).map_err(|e| anyhow::anyhow!("serialize record: {e}"))?;
    println!("{json}");
    Ok(())
}

fn run_d_unknowns() -> Result<()> {
    let dir = decisions_dir().context("decision store not accessible")?;
    let mut rows: Vec<DecisionRecord> = Vec::new();
    let entries = std::fs::read_dir(&dir).context("read decision store")?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(rec) = serde_json::from_str::<DecisionRecord>(&raw) {
                if !rec.unknowns.is_empty() {
                    rows.push(rec);
                }
            }
        }
    }
    rows.sort_by_key(|r| r.ts);
    let mut total = 0usize;
    for rec in &rows {
        total += rec.unknowns.len();
        println!(
            "# {}-{}  ({} unknown{})",
            rec.kind,
            rec.id,
            rec.unknowns.len(),
            if rec.unknowns.len() == 1 { "" } else { "s" }
        );
        for u in &rec.unknowns {
            println!("  - {u}");
        }
        println!();
    }
    if total == 0 {
        println!("(no unknowns recorded)");
    }
    Ok(())
}

fn run_d_link(child: &str) -> Result<()> {
    let dir = decisions_dir().context("decision store not accessible")?;
    let Some(target) = read_record_by_id(&dir, child) else {
        anyhow::bail!("no decision found for: {child}");
    };
    println!("# {}: {}", target.kind, target.idea);
    println!();
    let mut current = Some(target);
    let mut depth = 0usize;
    while let Some(rec) = current.take() {
        for _ in 0..depth {
            print!("  ");
        }
        println!("└─ {}-{} (ts={})", rec.kind, rec.id, rec.ts);
        if let Some(parent_id) = rec.parent.as_deref() {
            current = read_record_by_id(&dir, parent_id);
        }
        depth += 1;
    }
    Ok(())
}

// ─── tests ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── number_lines ──────────────────────────────────────────────────────────────────────

    #[test]
    fn number_lines_bare_lines() {
        let out = number_lines("foo\nbar\nbaz");
        assert_eq!(out, vec!["1. foo", "2. bar", "3. baz"]);
    }

    #[test]
    fn number_lines_strips_ascii_dot_prefix() {
        let out = number_lines("1. foo\n2. bar");
        assert_eq!(out, vec!["1. foo", "2. bar"]);
    }

    #[test]
    fn number_lines_strips_chinese_separator() {
        // "1、" is the typical Chinese list marker.
        let out = number_lines("1、 foo\n2、 bar");
        assert_eq!(out, vec!["1. foo", "2. bar"]);
    }

    #[test]
    fn number_lines_strips_dash_bullet() {
        let out = number_lines("- foo\n- bar");
        assert_eq!(out, vec!["1. foo", "2. bar"]);
    }

    #[test]
    fn number_lines_strips_star_bullet() {
        let out = number_lines("* foo\n* bar");
        assert_eq!(out, vec!["1. foo", "2. bar"]);
    }

    #[test]
    fn number_lines_preserves_emoji() {
        let out = number_lines("1. 💡 idea one\n2. 🎯 idea two");
        assert_eq!(out, vec!["1. 💡 idea one", "2. 🎯 idea two"]);
    }

    #[test]
    fn number_lines_empty_lines_dont_increment_number() {
        // Original line indices are preserved even when lines are skipped.
        let out = number_lines("foo\n\nbar");
        assert_eq!(out, vec!["1. foo", "3. bar"]);
    }

    #[test]
    fn number_lines_input_is_only_whitespace() {
        let out = number_lines("   \n\t\n  ");
        assert!(out.is_empty());
    }

    // ─── SSE parser ────────────────────────────────────────────────────────────

    /// Helper: drive `drain_sse_events` with `input`, collect every delta,
    /// and return (joined_content, leftover_buffer_after_drain).
    fn run_drain(input: &str) -> (String, String) {
        let mut buf = input.to_string();
        let mut pieces: Vec<String> = Vec::new();
        let mut sink = |p: &str| pieces.push(p.to_string());
        drain_sse_events(&mut buf, &mut sink).unwrap();
        (pieces.join(""), buf)
    }

    #[test]
    fn sse_single_complete_event() {
        let chunk = "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n";
        let (out, rest) = run_drain(chunk);
        assert_eq!(out, "hello");
        assert_eq!(rest, "");
    }

    #[test]
    fn sse_multiple_events_one_buffer() {
        let chunk = "data: {\"choices\":[{\"delta\":{\"content\":\"foo\"}}]}\n\n\
                     data: {\"choices\":[{\"delta\":{\"content\":\"bar\"}}]}\n\n";
        let (out, rest) = run_drain(chunk);
        assert_eq!(out, "foobar");
        assert_eq!(rest, "");
    }

    #[test]
    fn sse_done_marker_is_ignored() {
        let chunk = "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\n\
                     data: [DONE]\n\n";
        let (out, _) = run_drain(chunk);
        assert_eq!(out, "x");
    }

    #[test]
    fn sse_empty_content_is_ignored() {
        // Final chunk in some implementations is `delta: {}` with no content.
        let chunk = "data: {\"choices\":[{\"delta\":{}}]}\n\n";
        let (out, _) = run_drain(chunk);
        assert_eq!(out, "");
    }

    #[test]
    fn sse_partial_event_kept_in_buffer() {
        // Event spans two chunks: first chunk has no terminator, second does.
        let mut buf = String::from("data: {\"choices\":[{\"delta\":{\"content\":\"hel");
        let mut pieces: Vec<String> = Vec::new();
        // Scope each sink borrow so we can also read `pieces` between calls.
        {
            let mut sink = |p: &str| pieces.push(p.to_string());
            drain_sse_events(&mut buf, &mut sink).unwrap();
        }
        assert_eq!(pieces, Vec::<String>::new());
        assert_eq!(buf, "data: {\"choices\":[{\"delta\":{\"content\":\"hel");

        buf.push_str("lo\"}}]}\n\n");
        {
            let mut sink = |p: &str| pieces.push(p.to_string());
            drain_sse_events(&mut buf, &mut sink).unwrap();
        }
        assert_eq!(pieces, vec!["hello".to_string()]);
        assert_eq!(buf, "");
    }

    #[test]
    fn sse_event_with_extras_lines() {
        // SSE allows `event:` / `id:` lines alongside `data:`. They must be
        // ignored by the parser, not fed to serde_json.
        let chunk = "event: message\n\
                     id: 42\n\
                     data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n";
        let (out, _) = run_drain(chunk);
        assert_eq!(out, "ok");
    }

    #[test]
    fn sse_invalid_json_is_skipped() {
        // Malformed JSON must NOT abort the stream — would lose partial
        // content the user has already seen. The bad chunk is dropped and
        // any subsequent valid chunks still deliver.
        let chunk = "data: {not json\n\n\
                     data: {\"choices\":[{\"delta\":{\"content\":\"recovered\"}}]}\n\n";
        let (out, _) = run_drain(chunk);
        assert_eq!(out, "recovered");
    }

    #[test]
    fn sse_chunk_with_usage_is_captured() {
        // Many OpenAI-compatible servers append a usage object to the final
        // chunk of a stream; the parser must surface it, not drop it.
        let chunk = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}],\"usage\":{",
            "\"prompt_tokens\":604,\"completion_tokens\":208,\"total_tokens\":812}}

"
        );
        let mut buf = chunk.to_string();
        let mut pieces: Vec<String> = Vec::new();
        let mut sink = |p: &str| pieces.push(p.to_string());
        let usage = drain_sse_events(&mut buf, &mut sink).unwrap();
        assert_eq!(
            usage,
            Some(Usage {
                prompt_tokens: 604,
                completion_tokens: 208
            })
        );
    }

    #[test]
    fn sse_without_usage_returns_none() {
        let chunk = "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}

";
        let mut buf = chunk.to_string();
        let mut sink = |p: &str| {};
        assert_eq!(drain_sse_events(&mut buf, &mut sink).unwrap(), None);
    }

    #[test]
    fn usage_total_sums_prompt_and_completion() {
        assert_eq!(Usage::default().total(), 0);
        assert_eq!(
            Usage {
                prompt_tokens: 604,
                completion_tokens: 208
            }
            .total(),
            812
        );
    }

    #[test]
    fn response_json_usage_is_parsed_when_present() {
        let body = r#"{"choices":[{"message":{"content":"hi"}}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#;
        let resp: ChatResponse = serde_json::from_str(body).unwrap();
        assert_eq!(
            resp.usage,
            Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5
            })
        );
    }

    #[test]
    fn response_json_without_usage_parses_as_none() {
        let body = r#"{"choices":[{"message":{"content":"hi"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.usage, None);
    }

    #[test]
    fn sse_missing_delta_field_is_skipped() {
        // Tail of an OpenAI-compatible stream often emits a finish_reason
        // chunk with no `delta` key at all: `{"choices":[{"finish_reason":
        // "stop","index":0}]}`. This used to be a fatal error and lost
        // everything the user had already received.
        let chunk = "data: {\"choices\":[{\"index\":0,\"finish_reason\":\"stop\"}]}\n\n";
        let (out, _) = run_drain(chunk);
        assert_eq!(out, "");
    }

    #[test]
    fn sse_finish_reason_after_content_delivers_content() {
        // The realistic shape: some content chunks, then a finish_reason
        // chunk at the end. All content must arrive; the finish chunk is
        // silently skipped.
        let chunk = "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n\
                     data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n\
                     data: {\"choices\":[{\"index\":0,\"finish_reason\":\"stop\"}]}\n\n\
                     data: [DONE]\n\n";
        let (out, _) = run_drain(chunk);
        assert_eq!(out, "hello world");
    }
    // ─── config parse ──────────────────────────────────────────────────────────

    #[test]
    fn config_parse_empty_is_defaults() {
        let cfg = Config::parse("");
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn config_parse_overrides_provider_table() {
        let cfg = Config::parse(
            "[provider]\nchat_url = \"http://localhost:11434/v1/chat/completions\"\nmodel = \"llama3\"\n",
        );
        assert_eq!(cfg.chat_url, "http://localhost:11434/v1/chat/completions");
        assert_eq!(cfg.model, "llama3");
        assert_eq!(cfg.api_key_env, DEFAULT_API_KEY_ENV);
    }

    #[test]
    fn config_parse_malformed_falls_back_to_defaults() {
        let cfg = Config::parse("not [ valid toml {{{");
        assert_eq!(cfg, Config::default());
    }

    // ─── endpoint_host ─────────────────────────────────────────────────────────

    #[test]
    fn endpoint_host_strips_scheme_and_path() {
        assert_eq!(endpoint_host(DEFAULT_CHAT_URL), "api.minimax.chat");
        assert_eq!(
            endpoint_host("http://localhost:11434/v1/chat/completions"),
            "localhost:11434"
        );
    }

    #[test]
    fn endpoint_host_falls_back_on_bare_string() {
        assert_eq!(endpoint_host("localhost"), "localhost");
    }

    // ─── detect_language_hint ──────────────────────────────────────────────────    #[test]
    fn detect_chinese_hint_for_cjk_prompts() {
        assert!(detect_language_hint("做知乎热榜爬虫").starts_with("[Respond in Chinese.]"));
        assert!(detect_language_hint("分析电商评论").starts_with("[Respond in Chinese.]"));
    }

    #[test]
    fn detect_chinese_hint_for_japanese_prompts() {
        // Hiragana characters — we still answer in Chinese (closest fallback).
        // Real Japanese support would need more locales; cheap heuristic
        // covers the dominant CJK case.
        assert!(detect_language_hint("こんにちは").starts_with("[Respond in Chinese.]"));
    }

    #[test]
    fn detect_empty_hint_for_latin_prompts() {
        assert_eq!(detect_language_hint("make a stock monitor"), "");
        assert_eq!(detect_language_hint("hello world"), "");
    }

    #[test]
    fn detect_empty_hint_for_digit_or_punct_only() {
        assert_eq!(detect_language_hint(""), "");
        assert_eq!(detect_language_hint("1234.56"), "");
        assert_eq!(detect_language_hint("?!@#"), "");
    }

    #[test]
    fn backoff_doubles_per_attempt() {
        assert_eq!(backoff_secs(0), 1);
        assert_eq!(backoff_secs(1), 2);
        assert_eq!(backoff_secs(2), 4);
    }

    #[test]
    fn retryable_statuses_are_429_and_5xx_only() {
        assert!(is_retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(reqwest::StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(!is_retryable_status(reqwest::StatusCode::UNAUTHORIZED));
        assert!(!is_retryable_status(reqwest::StatusCode::BAD_REQUEST));
        assert!(!is_retryable_status(reqwest::StatusCode::OK));
    }

    #[test]
    fn config_validate_flags_bad_fields() {
        let mut cfg = Config::default();
        assert!(cfg.validate().is_empty());
        cfg.chat_url = "api.example.com".into();
        cfg.model = "  ".into();
        cfg.api_key_env = "bad-name".into();
        let issues = cfg.validate();
        assert_eq!(issues.len(), 3, "{issues:?}");
    }

    // ─── decision store (v0.3) ─────────────────────────────────────────────────

    fn tmp_store(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("naysay-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    const SAMPLE_BODY: &str = "1. Cause of death — scope.\n\
2. Ranked killers — over-engineering.\n\
\n\
ASSUMPTIONS:\n\
- a person will run this 3x/week\n\
- setup takes under 10 minutes\n\
\n\
EVIDENCE:\n\
- none yet\n\
\n\
UNKNOWNS:\n\
- retention after week 2\n\
- willingness to pay\n\
\n\
CONFIDENCE: 0.62\n";

    #[test]
    fn decision_id_is_12_hex_and_unique_per_nanos() {
        let a = make_decision_id(1_000_000_000);
        let b = make_decision_id(2_000_000_000);
        assert_eq!(a.len(), 12);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn extract_section_grabs_bullets_until_blank_line() {
        let out = extract_section(SAMPLE_BODY, "ASSUMPTIONS");
        assert_eq!(
            out,
            vec![
                "a person will run this 3x/week",
                "setup takes under 10 minutes"
            ]
        );
    }

    #[test]
    fn extract_section_handles_numbered_lists_and_missing_headings() {
        let body = "UNKNOWNS:\n1. retention\n2. pricing\n\nEVIDENCE:\n- none";
        assert_eq!(
            extract_section(body, "UNKNOWNS"),
            vec!["retention", "pricing"]
        );
        assert!(extract_section("no headings here", "UNKNOWNS").is_empty());
    }

    #[test]
    fn extract_confidence_accepts_fraction_and_percent() {
        assert_eq!(extract_confidence("CONFIDENCE: 0.62"), Some(62));
        assert_eq!(extract_confidence("CONFIDENCE — 73"), Some(73));
        assert_eq!(extract_confidence("no confidence here"), None);
    }

    #[test]
    fn save_roundtrip_extracts_fields_and_link_finds_parent() {
        let dir = tmp_store("roundtrip");
        let id = save_decision_to(
            &dir,
            "premortem",
            "build x",
            SAMPLE_BODY,
            None,
            1_000_000_000,
        )
        .expect("save premortem");
        let rec = read_record_by_id(&dir, &id).expect("record readable");
        assert_eq!(rec.kind, "premortem");
        assert_eq!(rec.assumptions.len(), 2);
        assert_eq!(rec.unknowns.len(), 2);
        assert_eq!(rec.confidence, Some(62));

        // child record pointing at the premortem
        let child = save_decision_to(
            &dir,
            "postmortem",
            "build x (review)",
            "CALIBRATION: the verdict held.",
            Some(&id),
            2_000_000_000,
        )
        .expect("save postmortem");
        let child_rec = read_record_by_id(&dir, &child).expect("child readable");
        assert_eq!(child_rec.parent.as_deref(), Some(id.as_str()));
        // reading by full stem also works
        assert!(read_record_by_id(&dir, &format!("postmortem-{child}")).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ─── provider picker (v0.4) ─────────────────────────────────────────────────

    #[test]
    fn provider_choice_parses_ranges_only() {
        assert_eq!(parse_provider_choice("1", 7), Some(0));
        assert_eq!(parse_provider_choice(" 7 ", 7), Some(6));
        assert_eq!(parse_provider_choice("0", 7), None);
        assert_eq!(parse_provider_choice("8", 7), None);
        assert_eq!(parse_provider_choice("abc", 7), None);
        assert_eq!(parse_provider_choice("", 7), None);
    }

    #[test]
    fn provider_presets_are_well_formed() {
        for p in PRESETS {
            assert!(
                p.chat_url.starts_with("http://") || p.chat_url.starts_with("https://"),
                "{}: bad url",
                p.label
            );
            assert!(!p.model.is_empty(), "{}: empty model", p.label);
            assert!(
                is_valid_env_name(p.api_key_env),
                "{}: bad env name",
                p.label
            );
            // The key is only ever unused for local servers.
            assert_eq!((p.chat_url.starts_with("http://localhost")), !p.needs_key);
        }
        // Ollama must stay the no-key option — it is the whole point.
        assert!(!PRESETS[0].needs_key);
    }

    #[test]
    fn provider_toml_roundtrips_through_config_parse() {
        let body = provider_toml_body(
            "https://api.deepseek.com/chat/completions",
            "deepseek-chat",
            "DEEPSEEK_API_KEY",
        );
        let cfg = Config::parse_strict(&body).expect("valid toml");
        assert_eq!(cfg.chat_url, "https://api.deepseek.com/chat/completions");
        assert_eq!(cfg.model, "deepseek-chat");
        assert_eq!(cfg.api_key_env, "DEEPSEEK_API_KEY");
        assert!(cfg.validate().is_empty());
    }

    #[test]
    fn setup_box_row_keeps_frame_aligned_across_versions() {
        // The stale-banner bug: v0.1 vs v0.3.0 vs a hypothetical v0.10.0
        // must all produce rows of identical display width.
        let v1 = setup_box_row("naysay v0.1 — says no before your agents do");
        let v3 = setup_box_row("naysay v0.3.0 — says no before your agents do");
        let v10 = setup_box_row("naysay v0.10.0 — says no before your agents do");
        assert_eq!(v1.chars().count(), v3.chars().count());
        assert_eq!(v3.chars().count(), v10.chars().count());
        assert!(v1.starts_with("  |   "));
        assert!(v1.ends_with('|'));
    }

    #[test]
    fn detect_chinese_hint_after_ascii_command_word() {
        // The actual reported failure: REPL lines start with an ASCII
        // command word, so a first-letter heuristic never fired for the
        // lines that needed it.
        assert!(detect_language_hint("premortem 做个爬虫").starts_with("[Respond in Chinese.]"));
        assert!(detect_language_hint("spec 做知乎热榜爬虫").starts_with("[Respond in Chinese.]"));
    }

    // ─── session records / resume ──────────────────────────────────────────────

    /// Write `lines` to a temp .jsonl file and parse it back. Each call
    /// gets its own directory — tests run in parallel and must not share
    /// a fixture path.
    fn parse_fixture(name: &str, lines: &[&str]) -> Result<Vec<SessionRecord>> {
        let dir = std::env::temp_dir().join(format!("naysay-test-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).context("create temp dir")?;
        let path = dir.join("session-fix.jsonl");
        std::fs::write(&path, lines.join("\n")).context("write fixture")?;
        let out = load_session_records(&path);
        let _ = std::fs::remove_dir_all(&dir);
        out
    }

    #[test]
    fn session_records_parse_both_sides() {
        let recs = parse_fixture(
            "both-sides",
            &[
                "{\"t\":1,\"kind\":\"user\",\"text\":\"premortem x\"}",
                "{\"t\":2,\"kind\":\"assistant\",\"text\":\"the autopsy\"}",
            ],
        )
        .unwrap();
        assert_eq!(
            recs,
            vec![
                SessionRecord {
                    kind: "user".into(),
                    text: "premortem x".into()
                },
                SessionRecord {
                    kind: "assistant".into(),
                    text: "the autopsy".into()
                },
            ]
        );
    }

    #[test]
    fn session_records_skip_malformed_unknown_and_empty() {
        let recs = parse_fixture(
            "skip-bad",
            &[
                "not json at all",
                "{\"t\":3,\"kind\":\"info\",\"text\":\"[ok] boot marker\"}",
                "{\"t\":4,\"kind\":\"user\",\"text\":\"\"}",
                "",
                "{\"t\":5,\"kind\":\"user\",\"text\":\"kept\"}",
            ],
        )
        .unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].text, "kept");
    }

    #[test]
    fn session_records_missing_file_errors() {
        let bogus = std::env::temp_dir().join("naysay-nonexistent-session.jsonl");
        assert!(load_session_records(&bogus).is_err());
    }

    #[test]
    fn newest_session_in_picks_lexicographically_last_jsonl() {
        let dir = std::env::temp_dir().join(format!("naysay-test-new-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("session-100.jsonl"), "").unwrap();
        std::fs::write(dir.join("session-200.jsonl"), "").unwrap();
        std::fs::write(dir.join("notes.txt"), "").unwrap();
        let got = newest_session_in(&dir).unwrap();
        assert_eq!(
            got.file_name().unwrap().to_string_lossy(),
            "session-200.jsonl"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn newest_session_in_empty_dir_is_none() {
        let dir = std::env::temp_dir().join(format!("naysay-test-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(newest_session_in(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_session_in_digits_vs_filename() {
        let dir = std::path::Path::new("/tmp/sessions");
        assert_eq!(
            resolve_session_in(dir, "1787486027"),
            dir.join("session-1787486027.jsonl")
        );
        assert_eq!(
            resolve_session_in(dir, "session-1787486027.jsonl"),
            dir.join("session-1787486027.jsonl")
        );
        assert_eq!(
            resolve_session_in(dir, "weird.name"),
            dir.join("weird.name")
        );
    }

    // ─── ReplState ─────────────────────────────────────────────────────────────

    fn msg(role: &str, content: &str) -> Message {
        Message {
            role: role.into(),
            content: content.into(),
        }
    }

    #[test]
    fn repl_context_slices_last_pairs() {
        let st = ReplState {
            history: (0..8)
                .map(|i| {
                    msg(
                        if i % 2 == 0 { "user" } else { "assistant" },
                        &format!("m{i}"),
                    )
                })
                .collect(),
            context_turns: 2,
            session_path: None,
        };
        let ctx = st.context();
        assert_eq!(ctx.len(), 4);
        assert_eq!(ctx[0].content, "m4");
        assert_eq!(ctx[3].content, "m7");
    }

    #[test]
    fn repl_context_empty_history_and_zero_turns() {
        let mut st = ReplState {
            history: Vec::new(),
            context_turns: 3,
            session_path: None,
        };
        assert!(st.context().is_empty());
        st.context_turns = 0;
        st.history.push(msg("user", "x"));
        assert!(st.context().is_empty());
    }

    #[test]
    fn repl_record_hints_user_side_and_keeps_order() {
        let mut st = ReplState {
            history: Vec::new(),
            context_turns: 3,
            session_path: None,
        };
        st.record("premortem 做个爬虫", "the autopsy");
        st.record("make a stock monitor", "the angles");
        assert_eq!(st.history.len(), 4);
        assert!(st.history[0].content.starts_with("[Respond in Chinese.] "));
        assert_eq!(st.history[0].role, "user");
        assert_eq!(st.history[1].content, "the autopsy");
        // Latin turn gets no hint.
        assert_eq!(st.history[2].content, "make a stock monitor");
        assert_eq!(st.history[3].role, "assistant");
    }
}
