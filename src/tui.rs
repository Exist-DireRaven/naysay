//! Inline transcript UI for naysay — no full-screen chrome.
//!
//! Claude Code's lesson: the best terminal UI is no terminal UI. The
//! conversation is a transcript that scrolls naturally in the terminal's
//! own scrollback (printed via ratatui's inline viewport `insert_before`),
//! and the only live region is two lines at the bottom: `>` input + one
//! dim status line. Quit, and the transcript stays readable right where
//! a terminal transcript should stay.
//!
//! Async LLM calls run in `tokio::spawn` tasks; results stream back
//! through an `mpsc::channel`. Finished entries are flushed to scrollback
//! by `flush_pending`; the in-flight response is not displayed live —
//! an autopsy report arrives as a document, not as chatter. The status
//! line carries the liveness instead: spinner + character count.
//!
//! v1.2 additions on top of v1.1:
//!   - Banner moved out of content into the history pane's title bar
//!     (fixes the "middle is misaligned" complaint — content was wrapping
//!     around a 53-char ASCII box that didn't fit narrow terminals)
//!   - `--music` flag: looping 8-bit bassline in a background tokio task
//!   - Empty-response detection: shows `(no content)` instead of a blank block
//!   - Errors rendered with the full chain (was being truncated to first line)
//!   - "ready in X.Xs" feedback after each LLM call so the user knows it worked
//!
//! Sound module is a no-op on non-Windows targets — cross-compiles cleanly.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Position;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{Terminal, TerminalOptions, Viewport};

use crate::{call_llm_stream, config, endpoint_host, load_api_key, open_session_log, Prompts};

/// Display width of `text` in terminal columns. CJK ideographs, kana, and
/// fullwidth punctuation are 2 columns; everything else is 1.
///
/// ratatui renders each character using its Unicode width property when
/// drawing Paragraph content, so the cursor must follow the same
/// accounting. Counting `chars()` only gives 1 column per glyph and lands
/// the cursor inside a wide character — visually mid-glyph for Chinese
/// users. This is the same heuristic every terminal emulator uses for
/// cursor placement; we don't pull in `unicode-width` because a narrow
/// inline range check covers 99% of real prompts.
fn display_width(text: &str) -> usize {
    let mut w = 0usize;
    for c in text.chars() {
        let cp = c as u32;
        let wide = (0x1100..=0x115F).contains(&cp)        // Hangul Jamo
            || (0x2E80..=0x303E).contains(&cp)             // CJK Radicals + Symbols
            || (0x3041..=0x33FF).contains(&cp)             // Hiragana, Katakana, CJK symbols
            || (0x3400..=0x4DBF).contains(&cp)             // CJK Extension A
            || (0x4E00..=0x9FFF).contains(&cp)             // CJK Unified Ideographs
            || (0xA000..=0xA4CF).contains(&cp)             // Yi
            || (0xAC00..=0xD7A3).contains(&cp)             // Hangul Syllables
            || (0xF900..=0xFAFF).contains(&cp)             // CJK Compatibility
            || (0xFE30..=0xFE4F).contains(&cp)             // CJK Compatibility Forms
            || (0xFF00..=0xFF60).contains(&cp)             // Fullwidth Forms
            || (0xFFE0..=0xFFE6).contains(&cp)             // Fullwidth signs
            || (0x20000..=0x2FFFD).contains(&cp)          // CJK Ext B–F + supplement
            || (0x30000..=0x3FFFD).contains(&cp);
        w += if wide { 2 } else { 1 };
    }
    w
}

/// Append one line to the session debug log. Best-effort — never panics,
/// never blocks. Used to diagnose "TUI flashed and exited" issues.
fn debug_log(msg: &str) {
    if let Ok(dir) = crate::data_dir() {
        let path = dir.join("session.log");
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Use OpenOptions::append then explicitly sync — without sync, kills
        // mid-write lose buffered data. This is the difference between
        // "diagnosable" and "I have no idea why it died".
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            use std::io::Write;
            let _ = writeln!(f, "[{ts}] {msg}");
            let _ = f.flush();
        }
    }
}

// ─── Windows console control handler ──────────────────────────────────────────────────

/// Set when the user presses Ctrl+C (or Ctrl+Break). The render loop polls
/// this flag and exits gracefully when set, instead of letting the default
/// Windows handler SIGKILL the process and lose the terminal cleanup.
static CTRL_C_PRESSED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
mod win_console {
    use super::CTRL_C_PRESSED;
    use std::sync::atomic::Ordering;

    /// Console control handler. Returning 1 (= TRUE) tells Windows we
    /// handled the event and the process should NOT be terminated.
    unsafe extern "system" fn handler(ctrl_type: u32) -> i32 {
        // CTRL_C_EVENT = 0, CTRL_BREAK_EVENT = 1
        if ctrl_type == 0 || ctrl_type == 1 {
            CTRL_C_PRESSED.store(true, Ordering::SeqCst);
            1 // handled
        } else {
            // CTRL_CLOSE_EVENT (2), CTRL_LOGOFF_EVENT (5), CTRL_SHUTDOWN_EVENT (6)
            // can't be intercepted — let the default handler run.
            0
        }
    }

    extern "system" {
        fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
    }

    pub fn install() {
        unsafe {
            let _ = SetConsoleCtrlHandler(Some(handler), 1);
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod win_console {
    pub fn install() {}
}

/// Entry point. The caller is responsible for being inside a tokio runtime.
///
/// * `sound_enabled` — play UI sound effects (submit blip, success arpeggio, error tone)
/// * `music_enabled` — play a looping 8-bit bassline in the background
/// * `resume` — session file to replay into the conversation (`--continue`).
///   New turns append to the same file, so a session stays in one piece.
pub async fn run(
    sound_enabled: bool,
    music_enabled: bool,
    resume: Option<std::path::PathBuf>,
) -> Result<()> {
    debug_log(&format!(
        "tui::run entered (sound={sound_enabled}, music={music_enabled})"
    ));

    // Install Windows console control handler so Ctrl+C becomes a graceful
    // quit instead of a SIGKILL.
    win_console::install();
    CTRL_C_PRESSED.store(false, Ordering::SeqCst);
    // Retry backoff notes go to stderr, which would garble the live rows —
    // silence them while the inline UI is up.
    crate::set_tui_active(true);

    // Sanity-check API key before opening the TUI — fail fast on misconfig.
    if let Err(e) = load_api_key() {
        debug_log(&format!("tui abort: no api key: {e}"));
        return Err(e).context("no API key — run `naysay key set` first");
    }
    debug_log("api key ok");

    if music_enabled {
        play_background_music();
        debug_log("background music task spawned");
    }

    let (tx, rx) = mpsc::channel::<TuiEvent>();
    let prompts = Arc::new(Prompts::load());
    debug_log("prompts loaded");
    let mut state = TuiState::new();
    // Session logging (user + assistant turns) — best-effort, same JSONL
    // format as the plain REPL so `naysay sessions` shows both. When
    // resuming, the previous file is reused so continued turns land in the
    // same session instead of forking a new one.
    if resume.is_none() {
        state.session_path = open_session_log().ok();
    }

    // First-launch: two sober lines. The header carries version + provider;
    // the menu lists the verdict family prominently because that is the brand.
    state.history.push(HistoryEntry::Info(format!(
        "naysay v0.1  ·  model: {}  ·  provider: {}",
        state.model,
        endpoint_host(&config().chat_url),
    )));
    state.history.push(HistoryEntry::Info(
        "type a command, or anything for freeform. verdict family first:".into(),
    ));
    state.history.push(HistoryEntry::Info(
        "  verdict    premortem <idea> | spec <idea> | postmortem <idea>".into(),
    ));
    state.history.push(HistoryEntry::Info(
        "  generation angles | questions | contrarian | use-cases".into(),
    ));
    state.history.push(HistoryEntry::Info(
        "  analysis   pros | cons | risks | steps | examples".into(),
    ));
    state.history.push(HistoryEntry::Info(
        "  reading    explain <file>  |  summarize <file>".into(),
    ));
    state.history.push(HistoryEntry::Info(
        "  session    /context N | /model <name> | /resume | /clear | Ctrl+S | r | Tab".into(),
    ));
    state
        .history
        .push(HistoryEntry::Info("  help       show all commands".into()));
    if let Some(ref p) = state.session_path {
        state.history.push(HistoryEntry::Info(format!(
            "  session    logging to {}",
            p.display()
        )));
    }
    state
        .history
        .push(HistoryEntry::Info("  Esc / Ctrl+C     quit".into()));

    // --continue: replay the resumed session's turns into the transcript.
    // build_context picks them up as ordinary pairs, so the model remembers
    // where the conversation left off; input recall is seeded too.
    if let Some(ref path) = resume {
        state.session_path = Some(path.clone());
        match crate::load_session_records(path) {
            Ok(records) => {
                let resumed_turns = records.len();
                for r in &records {
                    let entry = match r.kind.as_str() {
                        "user" => HistoryEntry::User(r.text.clone()),
                        _ => HistoryEntry::Ai(r.text.clone()),
                    };
                    state.history.push(entry);
                }
                for r in records.iter().filter(|r| r.kind == "user") {
                    state.input_history.push(r.text.clone());
                }
                if state.input_history.len() > 100 {
                    let excess = state.input_history.len() - 100;
                    state.input_history.drain(..excess);
                }
                state.history.push(HistoryEntry::Info(format!(
                    "[ok] resumed {resumed_turns} turns from {} — new turns append to the same session",
                    path.display()
                )));
            }
            Err(e) => {
                state
                    .history
                    .push(HistoryEntry::Error(format!("resume failed: {e}")));
            }
        }
    }

    let mut input = String::new();

    // Inline viewport: the terminal's own scrollback is the history pane.
    // Raw mode is still needed for key capture, but the screen is never
    // taken over — on quit, the transcript stays right where it printed.
    if let Err(e) = enable_raw_mode().context("enable raw mode") {
        debug_log(&format!("enable_raw_mode failed: {e:?}"));
        return Err(e);
    }
    debug_log("raw mode enabled");
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = match Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(2),
        },
    )
    .context("create inline terminal")
    {
        Ok(t) => t,
        Err(e) => {
            debug_log(&format!("inline terminal failed: {e:?}"));
            disable_raw_mode().ok();
            return Err(e);
        }
    };
    debug_log("inline terminal created");

    let result = (|| -> Result<()> {
        let tick_rate = Duration::from_millis(50);
        let mut last_tick = Instant::now();
        let mut frame_count: u64 = 0;
        debug_log("entering main loop");

        loop {
            // Drain any pending LLM results so the UI reflects them this frame.
            while let Ok(evt) = rx.try_recv() {
                debug_log(&format!(
                    "apply_event called: history_before={}",
                    state.history.len()
                ));
                apply_event(&mut state, evt, sound_enabled);
                debug_log(&format!(
                    "apply_event done: history_after={} status=`{}` busy={}",
                    state.history.len(),
                    state.status,
                    state.busy,
                ));
            }

            // Print every finished entry to the terminal's scrollback —
            // one insert for the whole pending batch, so the boot banner
            // and resume replay don't flicker line by line.
            if let Err(e) = flush_pending(&mut terminal, &mut state) {
                debug_log(&format!("flush_pending failed: {e:?}"));
                return Err(anyhow::anyhow!("scrollback flush: {e}"));
            }

            // Check the Ctrl+C flag set by the Windows console handler.
            // Without this, Ctrl+C terminates the process before cleanup runs
            // and the terminal stays in raw mode (and the user sees a flash).
            if CTRL_C_PRESSED.load(Ordering::SeqCst) {
                debug_log("Ctrl+C signal received — leaving loop gracefully");
                return Ok(());
            }

            // Render the two live rows (input + status)
            if let Err(e) = terminal.draw(|f| {
                render(f, &state, &input);
            }) {
                debug_log(&format!("terminal.draw failed: {e:?}"));
                return Err(anyhow::anyhow!("terminal draw: {e}"));
            }
            // While busy there is no input to type into — hide the cursor
            // so it doesn't sit parked on the input row.
            if state.busy {
                terminal.hide_cursor().ok();
            }

            // Timeout for events
            let elapsed = last_tick.elapsed();
            let poll_timeout = tick_rate.saturating_sub(elapsed);

            match event::poll(poll_timeout) {
                Ok(true) => {
                    let evt = match event::read() {
                        Ok(e) => e,
                        Err(e) => {
                            debug_log(&format!("event::read failed: {e:?}"));
                            return Err(anyhow::anyhow!("event read: {e}"));
                        }
                    };
                    if let Event::Key(key) = evt {
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }
                        match handle_key(key, &mut input, &mut state) {
                            KeyAction::None => {}
                            KeyAction::Quit => {
                                debug_log("Quit key received — leaving loop");
                                return Ok(());
                            }
                            KeyAction::Submit(line) => {
                                submit_line(
                                    line,
                                    &mut state,
                                    &tx,
                                    sound_enabled,
                                    Arc::clone(&prompts),
                                );
                            }
                            KeyAction::Save => match export_conversation(&state.history) {
                                Ok(path) => {
                                    state.history.push(HistoryEntry::Info(format!(
                                        "[ok] exported conversation to {}",
                                        path.display()
                                    )));
                                }
                                Err(e) => {
                                    state
                                        .history
                                        .push(HistoryEntry::Error(format!("export failed: {e}")));
                                }
                            },
                            KeyAction::Regenerate => {
                                if let Some(cmd) = state.last_command.clone() {
                                    if !state.busy {
                                        state.history.push(HistoryEntry::Info(format!(
                                            "[↻] regenerating: {cmd}"
                                        )));
                                        submit_line(
                                            cmd,
                                            &mut state,
                                            &tx,
                                            sound_enabled,
                                            Arc::clone(&prompts),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => {
                    debug_log(&format!("event::poll failed: {e:?}"));
                    return Err(anyhow::anyhow!("event poll: {e}"));
                }
            }

            if last_tick.elapsed() >= tick_rate {
                state.tick = state.tick.wrapping_add(1);
                last_tick = Instant::now();
                frame_count += 1;
                // Log every 10 frames (~500ms) so we can see if the loop is
                // actually running. With frame 100 we lose granularity.
                if frame_count <= 5 || frame_count.is_multiple_of(10) {
                    debug_log(&format!("frame {frame_count}"));
                }
            }
        }
    })();

    debug_log(&format!("loop ended: {:?}", result.is_ok()));

    // Always restore the terminal — even on error. No alternate screen to
    // leave: the transcript lives in the terminal's own scrollback and
    // stays readable after exit, which is the whole point of inline mode.
    crate::set_tui_active(false);
    disable_raw_mode().ok();
    println!();
    debug_log("terminal restored");

    result
}

// ─── State ──────────────────────────────────────────────────────────────────────────────

/// State for the Tab-cycling completion. Reset on any non-Tab key.
#[derive(Default)]
struct CompletionState {
    /// Word we are completing (the first token of `input`).
    prefix: String,
    /// Candidates that matched the prefix at first Tab.
    candidates: Vec<String>,
    /// Index into candidates for the next cycle step.
    cursor: usize,
}

/// Commands exposed to Tab completion. Kept in sync with `submit_line`'s
/// dispatch — anything you can type here you can tab-complete.
const COMMANDS: &[&str] = &[
    "angles",
    "seed",
    "questions",
    "contrarian",
    "use-cases",
    "usecases",
    "premortem",
    "spec",
    "postmortem",
    "pros",
    "drill",
    "cons",
    "risks",
    "steps",
    "examples",
    "explain",
    "summarize",
    "help",
    "quit",
];

/// Longest common prefix of a non-empty string slice.
fn longest_common_prefix(strs: &[String]) -> String {
    if strs.is_empty() {
        return String::new();
    }
    let first = &strs[0];
    let mut len = first.len();
    for s in &strs[1..] {
        let mut i = 0;
        while i < len && i < s.len() && s.as_bytes()[i] == first.as_bytes()[i] {
            i += 1;
        }
        len = i;
    }
    first[..len].to_string()
}

/// Tab completion for the first word of the input box.
///
/// Behavior:
///   * First Tab: collect candidates from COMMANDS that match the prefix.
///     If exactly one, fill it in. If multiple and they share a longer
///     common prefix than the typed prefix, extend to that. Otherwise, list
///     the candidates in the status bar for the user to see.
///   * Repeated Tab (while prefix is unchanged): cycle through candidates,
///     replacing the input with each in turn.
fn apply_completion(input: &mut String, state: &mut TuiState) {
    // Only complete the first word — arguments after a space are passed through.
    let first_space = input.find(' ');
    let (prefix, rest): (String, Option<String>) = match first_space {
        Some(i) => (input[..i].to_string(), Some(input[i..].to_string())),
        None => (input.clone(), None),
    };

    // Same prefix as last time → cycle to next candidate.
    if state.completion.prefix == prefix && !state.completion.candidates.is_empty() {
        let next = &state.completion.candidates[state.completion.cursor];
        let rebuilt = match rest {
            Some(suffix) => format!("{next}{suffix}"),
            None => next.clone(),
        };
        *input = rebuilt;
        state.completion.cursor = (state.completion.cursor + 1) % state.completion.candidates.len();
        return;
    }

    // Fresh prefix — collect matching commands.
    let candidates: Vec<String> = COMMANDS
        .iter()
        .filter(|c| c.starts_with(&prefix))
        .map(|s| s.to_string())
        .collect();

    match candidates.len() {
        0 => {
            // No match. Leave input alone, leave a hint in status.
            state.status = format!("no command starts with '{prefix}'");
        }
        1 => {
            // Single match — fill it in and clear cycling state.
            let only = candidates.into_iter().next().unwrap();
            let rebuilt = match rest {
                Some(suffix) => format!("{only}{suffix}"),
                None => only.clone(),
            };
            *input = rebuilt;
            state.completion = CompletionState::default();
        }
        _ => {
            // Multiple — try longest common prefix first.
            let lcp = longest_common_prefix(&candidates);
            if lcp.len() > prefix.len() {
                let rebuilt = match rest {
                    Some(suffix) => format!("{lcp}{suffix}"),
                    None => lcp.clone(),
                };
                *input = rebuilt;
                state.completion = CompletionState {
                    prefix: lcp,
                    candidates: candidates.clone(),
                    cursor: 0,
                };
                state.status = format!("{} options — Tab again to cycle", candidates.len());
            } else {
                // Same length prefix — cycle.
                let next = &candidates[0];
                let rebuilt = match rest {
                    Some(suffix) => format!("{next}{suffix}"),
                    None => next.clone(),
                };
                *input = rebuilt;
                state.completion = CompletionState {
                    prefix: prefix.to_string(),
                    candidates: candidates.clone(),
                    cursor: 1 % candidates.len(),
                };
                state.status = candidates.join("  ");
            }
        }
    }
}

#[derive(Default)]
struct TuiState {
    history: Vec<HistoryEntry>,
    busy: bool,
    status: String,
    /// Frame counter for spinner animation. Wraps; modulo handles overflow.
    tick: u64,
    /// When the session started. Reserved for future uptime display in the
    /// header; currently unused since we removed the `uptime_secs` readout.
    #[allow(dead_code)]
    session_start: Option<Instant>,
    /// Number of times the user submitted a command (seed/drill/explain).
    calls: u32,
    /// Last command submitted by the user. Pressing `r` re-dispatches this.
    /// Cleared on app exit; cleared when input box is edited (so `r` won't
    /// regenerate an unrelated earlier prompt after freeform).
    last_command: Option<String>,
    /// Tab completion scratchpad. Reset on any non-Tab keypress.
    completion: CompletionState,
    /// Index into `history` of the Ai entry currently being filled by a
    /// streaming response. `None` between calls. The render loop reads this
    /// to decide whether to show the spinner next to the entry.
    streaming: Option<usize>,
    /// Number of prior user/assistant pairs to include as context for each
    /// call. Configurable via `/context N` (default 3). Range 0..=10.
    context_turns: usize,
    /// Model name used for subsequent calls. Configurable via `/model <name>`
    /// (defaults to `Config::default().model`). Shown in the title bar so
    /// the user always knows what's being asked.
    model: String,
    /// Previously submitted inputs, oldest first. Ctrl+↑ / Ctrl+↓ walk this
    /// list; capped so a long session can't grow it without bound.
    input_history: Vec<String>,
    /// Position in `input_history` while recalling. `None` = not recalling;
    /// typing any character cancels the recall.
    recall_idx: Option<usize>,
    /// Session JSONL this TUI instance logs to (user + assistant turns).
    /// `None` when the sessions dir is unwritable — logging is best-effort.
    session_path: Option<std::path::PathBuf>,
    /// How many history entries have been printed to the terminal's
    /// scrollback. Everything from this index on is either not yet final
    /// (the in-flight streaming entry) or waiting for the next flush.
    flushed: usize,
}

impl TuiState {
    fn new() -> Self {
        Self {
            history: Vec::new(),
            busy: false,
            status: String::new(),
            tick: 0,
            session_start: Some(Instant::now()),
            calls: 0,
            last_command: None,
            completion: CompletionState::default(),
            streaming: None,
            context_turns: 3,
            model: config().model.clone(),
            input_history: Vec::new(),
            recall_idx: None,
            session_path: None,
            flushed: 0,
        }
    }
}

#[derive(Clone)]
enum HistoryEntry {
    User(String),
    Ai(String),
    Error(String),
    Info(String),
}

enum TuiEvent {
    /// A single streamed chunk from the LLM. Append to the in-flight entry.
    Delta(String),
    /// Stream finished (Ok = full content + elapsed + server-reported token
    /// usage when the provider sends it, Err = terminal failure).
    Result(Result<(String, Duration, Option<crate::Usage>), String>),
}

enum KeyAction {
    None,
    Quit,
    Submit(String),
    Save,
    Regenerate,
}

/// Spinner glyphs. Classic ASCII rotating bar instead of braille dots —
/// keeps the monospace hacker feel (every char is one display column).
const SPINNER_FRAMES: &[&str] = &["|", "/", "-", "\\", "|", "/", "-", "\\"];

// ─── Color system ──────────────────────────────────────────────────────────────────────────────

/// naysay is an interrogator, not a hacker toy. Default colors everywhere —
/// we only reach for the single accent color when we have something that
/// genuinely needs attention: the verdict in a premortem, the cause of
/// death in an autopsy, the user's own mistakes.
const ACCENT_RED: Color = Color::Red;
const MUTED: Color = Color::DarkGray;

// ─── Input handling ─────────────────────────────────────────────────────────────────────

fn handle_key(key: KeyEvent, input: &mut String, state: &mut TuiState) -> KeyAction {
    // Ctrl+C / Ctrl+Q always quits, even when busy.
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q'))
    {
        return KeyAction::Quit;
    }

    // Esc also quits.
    if key.code == KeyCode::Esc {
        return KeyAction::Quit;
    }

    // Ctrl+S exports the conversation to a markdown file.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        return KeyAction::Save;
    }

    // `r` regenerates the last command (only when input box is empty).
    if key.code == KeyCode::Char('r') && input.is_empty() {
        return KeyAction::Regenerate;
    }

    // Tab triggers command completion on the first word.
    if key.code == KeyCode::Tab {
        apply_completion(input, state);
        return KeyAction::None;
    }

    // While a request is in flight, ignore everything except quit.
    // Scrolling is the terminal's job now — PageUp/PageUp in the normal
    // buffer just works, which is half the reason inline mode exists.
    if state.busy {
        return KeyAction::None;
    }

    match key.code {
        KeyCode::Char(c) => {
            input.push(c);
            state.recall_idx = None;
            state.completion = CompletionState::default();
        }
        KeyCode::Backspace => {
            input.pop();
            state.recall_idx = None;
            state.completion = CompletionState::default();
        }
        KeyCode::Enter => {
            let line = std::mem::take(input);
            state.completion = CompletionState::default();
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                return KeyAction::None;
            }
            return KeyAction::Submit(trimmed);
        }
        // Recall previously submitted inputs. Ctrl+↑/↓ so the plain
        // Up/Down keep scrolling the history pane — terminal users expect
        // ↑ to mean "previous input", but this pane's scroll owns it, and
        // re-training either habit loses. The modifier splits the two.
        KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if !state.input_history.is_empty() {
                let idx = match state.recall_idx {
                    None => state.input_history.len() - 1,
                    Some(i) => i.saturating_sub(1),
                };
                state.recall_idx = Some(idx);
                *input = state.input_history[idx].clone();
                state.completion = CompletionState::default();
            }
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
            match state.recall_idx {
                None => {}
                Some(i) if i + 1 >= state.input_history.len() => {
                    // Walked past the newest — clear the input like a shell.
                    state.recall_idx = None;
                    input.clear();
                    state.completion = CompletionState::default();
                }
                Some(i) => {
                    state.recall_idx = Some(i + 1);
                    *input = state.input_history[i + 1].clone();
                    state.completion = CompletionState::default();
                }
            }
        }
        // Plain Up/Down and PageUp/PageDown: handled by the terminal's own
        // scrollback now. Nothing to do here.
        _ => {}
    }
    KeyAction::None
}

// ─── Submission + async dispatch ───────────────────────────────────────────────────────

fn submit_line(
    line: String,
    state: &mut TuiState,
    tx: &mpsc::Sender<TuiEvent>,
    sound_enabled: bool,
    prompts: Arc<Prompts>,
) {
    state.last_command = Some(line.clone());
    state.input_history.push(line.clone());
    if state.input_history.len() > 100 {
        state.input_history.remove(0);
    }
    state.recall_idx = None;
    if let Some(ref p) = state.session_path {
        crate::log_event(p, "user", &line);
    }
    state.history.push(HistoryEntry::User(line.clone()));

    match line.as_str() {
        "quit" | "exit" | ":quit" => {
            state.history.push(HistoryEntry::Info(
                "use Ctrl+C or Esc to quit the TUI".into(),
            ));
            return;
        }
        "help" | "?" | ":help" => {
            state.history.push(HistoryEntry::Ai(
                "generation ─────────────────────────────────────\n  \
                 angles <topic>         angles you haven't considered\n  \
                 questions <topic>      deep questions to ask\n  \
                 contrarian <claim>     steelman the opposite\n  \
                 use-cases <thing>      concrete user scenarios\n\n\
                 analysis ────────────────────────────────────────\n  \
                 pros <idea>            genuine strengths\n  \
                 cons <idea>            genuine weaknesses\n  \
                 risks <idea>           failure modes\n  \
                 steps <goal>           actionable plan\n  \
                 examples <concept>     real-world instances\n\n\
                 verdict ─────────────────────────────────────────\n  \
                 premortem <idea>       assume it died in 6 months\n  \
                 spec <idea>            spec for your coding agent\n  \
                 postmortem <idea>      it's over — review + decision-log\n\n\
                 reading ────────────────────────────────────────\n  \
                 explain <file>         code walkthrough\n  \
                 summarize <file>       brief overview\n\n\
                 session ────────────────────────────────────────\n  \
                 /context N             set prior turns to remember (now {n})\n  \
                 /model <name>          switch LLM (now {m})\n  \
                 /resume [file]         replay a past session into context\n  \
                 /clear                 wipe history pane\n  \
                 Ctrl+S                 save conversation to markdown\n  \
                 r                       regenerate last command\n  \
                 Tab                     complete command name\n\n\
                 the AI sees your last N turns, so you can ask follow-ups\n\
                 like \"what about X?\" or \"expand that\". any text that\n\
                 isn't a command goes to freeform.\n\
                 seed = angles alias. drill = pros alias.\n\
                 @<path> inlines a file or directory (budgeted) into the prompt.\n\
                 \"what's wrong with @./src/main.rs?\"\n\
                 prompts editable in <data_dir>/prompts.toml"
                    .replace("{n}", &state.context_turns.to_string())
                    .replace("{m}", &state.model),
            ));
            return;
        }
        _ => {}
    }

    // Parse the first word. If it matches a curated shortcut, use the
    // dedicated prompt for that intent. Otherwise treat the whole line as
    // freeform natural-language input and hand it straight to the LLM.
    let mut parts = line.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim();

    // /context [N] — show or set the number of prior turns included in each
    // LLM call. Bare `/context` shows current value. Range 0..=10.
    if cmd == "/context" || cmd == ":context" {
        if rest.is_empty() {
            state.history.push(HistoryEntry::Info(format!(
                "context = {} turn{}",
                state.context_turns,
                if state.context_turns == 1 { "" } else { "s" }
            )));
        } else {
            match rest.parse::<usize>() {
                Ok(n) if n <= 10 => {
                    state.context_turns = n;
                    state.history.push(HistoryEntry::Info(format!(
                        "context = {} turn{}",
                        n,
                        if n == 1 { "" } else { "s" }
                    )));
                }
                Ok(n) => {
                    state.history.push(HistoryEntry::Error(format!(
                        "/context N: N must be 0..=10 (got {n})"
                    )));
                }
                Err(e) => {
                    state.history.push(HistoryEntry::Error(format!(
                        "/context N: not a number ({e})"
                    )));
                }
            }
        }
        return;
    }
    // /clear — wipe the conversation memory. The transcript above stays in
    // the terminal's scrollback (it belongs to the terminal now); only the
    // model's context is forgotten. `flushed` resets with the history so
    // the new entries actually print — flushed is an index into history,
    // and a stale index past the new length would suppress every flush.
    if cmd == "/clear" || cmd == ":clear" {
        let dropped = state.history.len();
        state.history.clear();
        state.flushed = 0;
        state.last_command = None;
        state.streaming = None;
        state.history.push(HistoryEntry::Info(format!(
            "[ok] cleared {dropped} remembered entries (the transcript above stays in scrollback)"
        )));
        return;
    }

    // /model [name] — show or switch the model used for subsequent calls.
    // Bare `/model` prints the current value. With an argument, replaces it.
    // Any non-empty string is accepted (the API will reject unknown names
    // with a clearer error than we'd ever write here).
    if cmd == "/model" || cmd == ":model" {
        if rest.is_empty() {
            state
                .history
                .push(HistoryEntry::Info(format!("model = {}", state.model)));
        } else {
            state.model = rest.to_string();
            state.history.push(HistoryEntry::Info(format!(
                "model = {rest} (applies to next call)"
            )));
        }
        return;
    }

    // /resume [file] — replay a past session into the conversation. Bare
    // `/resume` picks the newest; an argument is resolved like
    // `sessions show` (digits, filename, or path). New turns append to the
    // resumed file so the session stays in one piece.
    if cmd == "/resume" || cmd == ":resume" {
        if state.busy {
            state.history.push(HistoryEntry::Error(
                "wait for the current call to finish, then /resume".into(),
            ));
            return;
        }
        let target = if rest.is_empty() {
            crate::latest_session().ok()
        } else {
            crate::resolve_session_arg(rest).ok()
        };
        let Some(path) = target else {
            state.history.push(HistoryEntry::Error(
                "no session found to resume (start one first)".into(),
            ));
            return;
        };
        match crate::load_session_records(&path) {
            Ok(records) => {
                let n = records.len();
                for r in &records {
                    let entry = match r.kind.as_str() {
                        "user" => HistoryEntry::User(r.text.clone()),
                        _ => HistoryEntry::Ai(r.text.clone()),
                    };
                    state.history.push(entry);
                }
                for r in records.iter().filter(|r| r.kind == "user") {
                    state.input_history.push(r.text.clone());
                }
                if state.input_history.len() > 100 {
                    let excess = state.input_history.len() - 100;
                    state.input_history.drain(..excess);
                }
                state.session_path = Some(path.clone());
                state.history.push(HistoryEntry::Info(format!(
                    "[ok] resumed {n} turns from {} — new turns append to the same session",
                    path.display()
                )));
            }
            Err(e) => {
                state
                    .history
                    .push(HistoryEntry::Error(format!("resume failed: {e}")));
            }
        }
        return;
    }

    // Map a recognized command name to its `(kind, arg)` tuple. `seed` and
    // `drill` are kept as silent aliases for `angles` / `pros` so old
    // muscle memory still works.
    let work: Option<(&'static str, String)> = match cmd {
        // Generation family
        "angles" if !rest.is_empty() => Some(("angles", rest.to_string())),
        "seed" if !rest.is_empty() => Some(("angles", rest.to_string())),
        "questions" if !rest.is_empty() => Some(("questions", rest.to_string())),
        "contrarian" if !rest.is_empty() => Some(("contrarian", rest.to_string())),
        "use-cases" | "usecases" if !rest.is_empty() => Some(("use-cases", rest.to_string())),
        // Verdict family
        "premortem" if !rest.is_empty() => Some(("premortem", rest.to_string())),
        "spec" if !rest.is_empty() => Some(("spec", rest.to_string())),
        "postmortem" if !rest.is_empty() => Some(("postmortem", rest.to_string())),
        // Analysis family
        "pros" if !rest.is_empty() => Some(("pros", rest.to_string())),
        "drill" if !rest.is_empty() => Some(("pros", rest.to_string())),
        "cons" if !rest.is_empty() => Some(("cons", rest.to_string())),
        "risks" if !rest.is_empty() => Some(("risks", rest.to_string())),
        "steps" if !rest.is_empty() => Some(("steps", rest.to_string())),
        "examples" if !rest.is_empty() => Some(("examples", rest.to_string())),
        // Comprehension family
        "explain" if !rest.is_empty() => Some(("explain", rest.to_string())),
        "summarize" if !rest.is_empty() => Some(("summarize", rest.to_string())),
        // Freeform fallback — anything else is straight-to-AI.
        _ if !cmd.is_empty() && !cmd.starts_with('/') => {
            Some(("freeform", line.trim().to_string()))
        }
        _ => None,
    };

    let Some((kind, arg)) = work else {
        state.history.push(HistoryEntry::Error(
            "type a command (angles, pros, cons, risks, steps, examples, questions, contrarian, use-cases, explain, summarize) or anything for freeform"
                .into(),
        ));
        return;
    };

    // @file inline: expand `@path` tokens into the actual file contents
    // before sending. Reports what was loaded (or failed) so the user can
    // spot typos in the history pane.
    let (arg, file_report) = inline_files(&arg);
    if !file_report.is_empty() {
        let ok: Vec<_> = file_report.iter().filter(|(_, _, r)| r.is_ok()).collect();
        let fail: Vec<_> = file_report.iter().filter(|(_, _, r)| r.is_err()).collect();
        let mut msg = String::from("[ok] inlined");
        if !ok.is_empty() {
            let names: Vec<String> = ok
                .iter()
                .map(|(p, n, _)| format!("{} ({} chars)", p, n))
                .collect();
            msg.push_str(&format!(" {}: {}", ok.len(), names.join(", ")));
        }
        if !fail.is_empty() {
            let names: Vec<String> = fail
                .iter()
                .map(|(p, _, e)| {
                    let err_msg = match e {
                        Ok(()) => unreachable!(),
                        Err(msg) => msg.clone(),
                    };
                    format!("{} ({})", p, err_msg)
                })
                .collect();
            msg.push_str(&format!("; failed {}: {}", fail.len(), names.join(", ")));
        }
        state.history.push(HistoryEntry::Info(msg));
    }

    state.busy = true;
    state.streaming = None;
    state.calls = state.calls.wrapping_add(1);
    state.status = format!("thinking [{kind}]...");

    play_sound(SoundKind::Submit, sound_enabled);

    let tx = tx.clone();
    let arg_for_task = arg.clone();
    let prompts_for_task = Arc::clone(&prompts);
    let model_for_task = state.model.clone();
    // Snapshot the most recent user/ai pairs as context for this call.
    // Clone the whole thing so the async task owns its data.
    let context_turns = state.context_turns;
    let context = build_context(&state.history, context_turns);
    tokio::spawn(async move {
        let started = Instant::now();
        debug_log(&format!(
            "llm call started: model=`{model_for_task}` kind={kind} arg=`{arg_for_task}`"
        ));
        // Wrap each chunk into a TuiEvent::Delta so the main loop can
        // append it to the in-flight Ai entry in real time.
        let tx_delta = tx.clone();
        let on_delta = |delta: &str| {
            let _ = tx_delta.send(TuiEvent::Delta(delta.to_string()));
        };
        let result = match kind {
            // Generation
            "angles" => {
                run_angles(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            "questions" => {
                run_questions(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            "contrarian" => {
                run_contrarian(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            "use-cases" => {
                run_use_cases(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            // Verdict
            "premortem" => {
                run_premortem(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            "spec" => {
                run_spec(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            "postmortem" => {
                run_postmortem(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            // Analysis
            "pros" => {
                run_pros(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            "cons" => {
                run_cons(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            "risks" => {
                run_risks(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            "steps" => {
                run_steps(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            "examples" => {
                run_examples(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            // Comprehension
            "explain" => {
                run_explain(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            "summarize" => {
                run_summarize(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            // Freeform
            "freeform" => {
                run_freeform(
                    &model_for_task,
                    &arg_for_task,
                    &context,
                    &prompts_for_task,
                    on_delta,
                )
                .await
            }
            _ => unreachable!(),
        };
        let elapsed = started.elapsed();
        match &result {
            Ok(content) => debug_log(&format!(
                "llm call ok: kind={kind} elapsed={:.2}s chars={}",
                elapsed.as_secs_f32(),
                content.chars().count(),
            )),
            Err(e) => debug_log(&format!(
                "llm call err: kind={kind} elapsed={:.2}s err={}",
                elapsed.as_secs_f32(),
                e.lines().next().unwrap_or(""),
            )),
        }
        let usage = crate::take_last_usage();
        let wrapped = result.map(|c| (c, elapsed, usage));
        let _ = tx.send(TuiEvent::Result(wrapped));
    });
}

// ─── Generation family ──────────────────────────────────────────────────────────────────

async fn run_angles<F: FnMut(&str) + Send>(
    model: &str,
    topic: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "The user wants to brainstorm around this topic: {topic}\n\n\
         Surface angles they probably haven't considered. Pick a number of \
         angles that fits the topic (5-10 usually). Each angle: short, \
         specific, surprising — not generic.";
    let template = prompts.get("angles", DEFAULT);
    let prompt = template.replace("{topic}", topic);
    let content = call_llm_stream(model, &prompt, history, 1200, 0.7, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("angles", topic, &content)
}

async fn run_questions<F: FnMut(&str) + Send>(
    model: &str,
    topic: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "The user wants to think harder about: {topic}\n\n\
         Generate 5-7 deep questions the user should ask themselves. The \
         questions should expose hidden assumptions, force a decision, or \
         surface something the user is likely overlooking. Skip obvious \
         questions — the kind that have obvious answers.";
    let template = prompts.get("questions", DEFAULT);
    let prompt = template.replace("{topic}", topic);
    let content = call_llm_stream(model, &prompt, history, 1000, 0.7, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("questions", topic, &content)
}

async fn run_contrarian<F: FnMut(&str) + Send>(
    model: &str,
    claim: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "The user holds this claim: {claim}\n\n\
         Make the strongest possible case AGAINST it. Steelman the opposition: \
         do not pick a strawman, address the strongest version of the \
         disagreement. Be specific and concrete, not generic. If the claim has \
         merit, acknowledge it briefly, then explain why you still disagree.";
    let template = prompts.get("contrarian", DEFAULT);
    let prompt = template.replace("{claim}", claim);
    let content = call_llm_stream(model, &prompt, history, 1200, 0.7, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("contrarian", claim, &content)
}

async fn run_use_cases<F: FnMut(&str) + Send>(
    model: &str,
    thing: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "For this: {thing}\n\n\
         Generate 4-7 concrete user scenarios. Each scenario should name a \
         specific kind of user, a specific situation, and what they would \
         concretely do with it. Skip the obvious 'power users' and 'casual \
         users' buckets; get specific.";
    let template = prompts.get("use_cases", DEFAULT);
    let prompt = template.replace("{thing}", thing);
    let content = call_llm_stream(model, &prompt, history, 1200, 0.7, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("use-cases", thing, &content)
}

// ─── Verdict family ─────────────────────────────────────────────────────────────────────

async fn run_premortem<F: FnMut(&str) + Send>(
    model: &str,
    idea: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "The user is about to commit to building this: {idea}\n\n\
         Run the premortem. It is six months in the future and this project \
         is dead — abandoned, unmaintained, or alive but ignored. Write the \
         autopsy:\n\n\
         1. Cause of death — the single most likely killer, stated bluntly.\n\
         2. Ranked killers — 3-5 probable causes of death, each with the \
         early warning sign that was already visible on day one.\n\
         3. Scope autopsy — which imagined features were never touched, and \
         which single feature everything actually depended on.\n\
         4. The version that survived — the smallest cut of this idea that \
         dodges every cause of death above.\n\
         5. Verdict — build it (at what scope) or don't (and what to do \
         instead).\n\n\
         Be specific to this idea. Generic startup advice is worthless here.";
    let template = prompts.get("premortem", DEFAULT);
    let prompt = template.replace("{idea}", idea);
    let content = call_llm_stream(model, &prompt, history, 1500, 0.6, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("premortem", idea, &content)
}

async fn run_spec<F: FnMut(&str) + Send>(
    model: &str,
    idea: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "The user wants to hand this to a coding agent to execute: {idea}\n\n\
         Write the spec the agent will receive. Assume the agent is capable \
         but has zero context, and will take the path of least resistance \
         wherever the spec is vague. Sections:\n\n\
         # Goal — one paragraph: what exists when this is done, and for whom.\n\
         # Non-goals — what this is NOT. Anything unlisted here, the agent \
         will build on a whim.\n\
         # Success criteria — 3-5 concrete, checkable conditions.\n\
         # Constraints — language, platform, budget, things that must not change.\n\
         # Milestones — ordered; each one independently runnable or checkable.\n\
         # Open questions — what the user must decide; the agent should ask, \
         not guess.\n\n\
         Be concrete. A vague spec means the agent improvises, and \
         improvisation is where rework is born.";
    let template = prompts.get("spec", DEFAULT);
    let prompt = template.replace("{idea}", idea);
    let content = call_llm_stream(model, &prompt, history, 2000, 0.4, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("spec", idea, &content)
}

async fn run_postmortem<F: FnMut(&str) + Send>(
    model: &str,
    idea: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str =
        "The project \"{idea}\" is over — shipped, killed, or quietly abandoned. \
         The user provided no notes, so reason from the idea itself and common \
         patterns, and where the real outcome matters, say what evidence you \
         would need instead of inventing it.\n\n\
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
         Be specific to this project. Blame decisions, not people.";
    let template = prompts.get("postmortem", DEFAULT);
    let prompt = template.replace("{idea}", idea);
    let content = call_llm_stream(model, &prompt, history, 1500, 0.5, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("postmortem", idea, &content)
}

// ─── Analysis family ──────────────────────────────────────────────────────────────────────

async fn run_pros<F: FnMut(&str) + Send>(
    model: &str,
    idea: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "What are the genuine strengths of: {idea}\n\n\
         3-7 concrete advantages. Skip obvious ones; focus on what someone \
         already familiar with the idea would still find valuable. Be \
         specific, not generic.";
    let template = prompts.get("pros", DEFAULT);
    let prompt = template.replace("{idea}", idea);
    let content = call_llm_stream(model, &prompt, history, 800, 0.6, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("pros", idea, &content)
}

async fn run_cons<F: FnMut(&str) + Send>(
    model: &str,
    idea: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "What are the genuine weaknesses of: {idea}\n\n\
         3-7 concrete disadvantages. Skip obvious ones. Be specific: name \
         what is actually broken or missing, not generic 'depends on context' \
         hand-waves.";
    let template = prompts.get("cons", DEFAULT);
    let prompt = template.replace("{idea}", idea);
    let content = call_llm_stream(model, &prompt, history, 800, 0.6, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("cons", idea, &content)
}

async fn run_risks<F: FnMut(&str) + Send>(
    model: &str,
    idea: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "What could go wrong with: {idea}\n\n\
         3-7 failure modes. For each, name what could fail, how likely it is \
         (rough), and how bad it would be. Skip generic risks like 'it might \
         not work'; be specific to this idea.";
    let template = prompts.get("risks", DEFAULT);
    let prompt = template.replace("{idea}", idea);
    let content = call_llm_stream(model, &prompt, history, 1000, 0.6, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("risks", idea, &content)
}

async fn run_steps<F: FnMut(&str) + Send>(
    model: &str,
    goal: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "Actionable plan for: {goal}\n\n\
         3-7 concrete steps in the right order. Each step should be specific \
         enough to actually do ('research X' is too vague; 'read these three \
         papers and write a 1-page summary' is specific). The user should be \
         able to start step 1 within 5 minutes of reading this.";
    let template = prompts.get("steps", DEFAULT);
    let prompt = template.replace("{goal}", goal);
    let content = call_llm_stream(model, &prompt, history, 1000, 0.5, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("steps", goal, &content)
}

async fn run_examples<F: FnMut(&str) + Send>(
    model: &str,
    concept: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    const DEFAULT: &str = "3-5 concrete examples of: {concept}\n\n\
         Each example should be a real, specific instance, not 'imagine \
         something like X'. Name names, dates, places, products. If you cannot \
         think of real ones, say so rather than inventing.";
    let template = prompts.get("examples", DEFAULT);
    let prompt = template.replace("{concept}", concept);
    let content = call_llm_stream(model, &prompt, history, 1000, 0.6, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("examples", concept, &content)
}

// ─── Comprehension family ──────────────────────────────────────────────────────────────

async fn run_explain<F: FnMut(&str) + Send>(
    model: &str,
    path: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    let _ = prompts;
    let content = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let max_chars = 24_000;
    let truncated = if content.len() > max_chars {
        format!(
            "{}\n... (truncated, file is {} chars)",
            &content[..max_chars],
            content.len()
        )
    } else {
        content
    };
    let prompt = format!(
        "Walk a developer through this file. Be concrete: name the actual \
         functions, structs, or blocks that matter. Skip boilerplate (imports, \
         constants, blank lines). For each non-trivial chunk, give intent \
         (what it is for) and mechanism (how it works) in one or two sentences \
         each. End with 2-4 questions the reader should think about, only \
         questions the code actually raises.\n\n\
         File: {path}\n\n\
         Content:\n```\n{truncated}\n```"
    );
    let body = call_llm_stream(model, &prompt, history, 2500, 0.4, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(enrich_error(&format!(
            "LLM returned an empty response ({len} chars after trim)",
            len = body.len(),
        )));
    }
    Ok(format!("\n── explain: {path} ──\n\n{body}\n"))
}

async fn run_summarize<F: FnMut(&str) + Send>(
    model: &str,
    path: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    let _ = prompts;
    let content = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let max_chars = 24_000;
    let truncated = if content.len() > max_chars {
        format!(
            "{}\n... (truncated, file is {} chars)",
            &content[..max_chars],
            content.len()
        )
    } else {
        content
    };
    let prompt = format!(
        "Summarize this file in 5-8 sentences. What does it do, what is its \
         role in the larger system, what are the key abstractions? Skip \
         boilerplate. The reader should be able to decide whether to read the \
         file in full after your summary.\n\n\
         File: {path}\n\n\
         Content:\n```\n{truncated}\n```"
    );
    let body = call_llm_stream(model, &prompt, history, 800, 0.4, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    verify_and_format("summarize", path, &body)
}

/// Shared post-processing for all run_X commands: empty-detection + header
/// formatting. Centralizes the "did the LLM return nothing?" check so each
/// command's prompt can stay focused on its intent.
fn verify_and_format(kind: &str, arg: &str, raw: &str) -> Result<String, String> {
    if raw.trim().is_empty() {
        return Err(enrich_error(&format!(
            "[{kind}] LLM returned an empty response ({len} chars after trim)",
            len = raw.len(),
        )));
    }
    Ok(format!("\n── {kind}: {arg} ──\n\n{raw}\n"))
}

/// Freeform natural-language input. Pass through to the LLM with minimal
/// framing — the system prompt already defines the role and tone.
async fn run_freeform<F: FnMut(&str) + Send>(
    model: &str,
    input: &str,
    history: &[crate::Message],
    prompts: &Prompts,
    on_delta: F,
) -> Result<String, String> {
    let _ = prompts;
    let content = call_llm_stream(model, input, history, 1500, 0.7, on_delta)
        .await
        .map_err(|e| enrich_error(&format!("{e:#}")))?;
    if content.trim().is_empty() {
        return Err(enrich_error("LLM returned an empty response"));
    }
    Ok(format!("\n{content}\n"))
}

/// Classify an error string by content and append a "Try:" line suggesting the
/// next command to run. Goal: never leave the user with a wall of text and no
/// obvious next step.
fn enrich_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let suggestion = if lower.contains("no api key")
        || lower.contains("unauthorized")
        || lower.contains("401")
    {
        "Try: `naysay key set` to store a valid API key"
    } else if lower.contains("429") || lower.contains("rate limit") {
        "Try: wait a moment, then re-run the command"
    } else if lower.contains("empty response") {
        "Try: `naysay doctor` to diagnose, or re-run the command"
    } else if lower.contains("no such file") || lower.contains("read ") && lower.contains(":") {
        "Try: check the file path exists"
    } else if lower.contains("timeout") || lower.contains("connection") || lower.contains("network")
    {
        "Try: check your internet connection, or `naysay doctor`"
    } else if lower.contains("model") || lower.contains("overloaded") {
        "Try: re-run the command, or `naysay doctor`"
    } else {
        "Try: `naysay doctor` for full diagnostics"
    };
    format!("{raw}\n\n{suggestion}")
}

/// Write the current conversation to a markdown file. Skips boot sequence
/// (Info lines), keeps user prompts and AI responses interleaved as a clean
/// transcript. Errors get folded into the prose as `> error:` blockquotes.
fn export_conversation(history: &[HistoryEntry]) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(format!("naysay-{ts}.md"));

    let mut f = std::fs::File::create(&path)?;
    writeln!(f, "# naysay conversation\n")?;
    writeln!(f, "_exported at epoch {ts}_\n")?;

    for entry in history {
        match entry {
            HistoryEntry::Info(_) => {} // skip boot lines, focus on dialog
            HistoryEntry::User(s) => {
                writeln!(f, "**you**\n\n{s}\n")?;
            }
            HistoryEntry::Ai(s) => {
                writeln!(f, "**naysay**\n\n{s}\n")?;
            }
            HistoryEntry::Error(s) => {
                writeln!(f, "> ⚠ {s}\n")?;
            }
        }
    }

    f.flush()?;
    Ok(path)
}

/// Substitute `@<path>` tokens in `line` with the file's contents, wrapped
/// in a `[file: …] … ``` block so the model can see what was inlined and
/// from where. Truncates each file to 24_000 chars to bound prompt size.
///
/// Returns `(expanded, report)` where `report` lists `(path, char_count,
/// status)` for each token the caller found — used to emit an `[ok] inlined
/// …` line into history so the user can confirm what actually went out.
///
/// Path syntax: `@` followed by non-whitespace chars (`[^\s@]+`). Email-like
/// addresses (foo@bar) are not matched because there's no leading `@`.
/// (path, char count, status) triple per inlined `@` token.
type InlineReport = Vec<(String, usize, Result<(), String>)>;

fn inline_files(line: &str) -> (String, InlineReport) {
    let max_chars = 24_000;
    let mut expanded = String::with_capacity(line.len());
    let mut report: Vec<(String, usize, Result<(), String>)> = Vec::new();
    let mut rest = line;

    while let Some(at_idx) = rest.find('@') {
        // Copy everything up to (and including) the `@`.
        expanded.push_str(&rest[..=at_idx]);
        // Look ahead at the chars after `@` until whitespace or end.
        let after_at = &rest[at_idx + 1..];
        let path_len = after_at
            .char_indices()
            .take_while(|(_, c)| !c.is_whitespace())
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);

        if path_len == 0 {
            // Lone `@` followed by whitespace — keep it literal and move on.
            rest = &rest[at_idx + 1..];
            // We already copied `…@` above; advance to just past the `@`.
            // (expanded already has `@` at the end, that's correct.)
            continue;
        }

        let path = &after_at[..path_len];
        let trimmed_path = path.trim_end_matches(|c: char| {
            // Strip common punctuation that often trails file refs
            // (`@src/main.rs.`, `@src/main.rs,`, `@src/main.rs?`).
            matches!(c, '.' | ',' | ';' | ':' | '?' | '!')
        });

        // Directories inline every text file inside (budgeted); plain files
        // inline as before.
        let meta = std::fs::metadata(trimmed_path);
        if meta.as_ref().map(|m| m.is_dir()).unwrap_or(false) {
            let files = collect_dir_files(std::path::Path::new(trimmed_path), DIR_INLINE_BUDGET);
            if files.is_empty() {
                let msg = "no readable text files found".to_string();
                expanded.push_str(&format!("[dir: {trimmed_path} — {msg}]"));
                report.push((trimmed_path.to_string(), 0, Err(msg)));
            } else {
                let mut total = 0usize;
                for (fp, content) in &files {
                    let truncated = if content.len() > max_chars {
                        format!(
                            "{}\n... (truncated, file is {} chars)",
                            &content[..max_chars],
                            content.len()
                        )
                    } else {
                        content.clone()
                    };
                    let n = truncated.chars().count();
                    total += n;
                    expanded.push_str(&format!("[file: {}]\n```\n{truncated}\n```", fp.display()));
                    report.push((fp.display().to_string(), n, Ok(())));
                }
                // Budget overflow is reported once, against the dir token,
                // so the user knows content was left out.
                let skipped = count_skipped_files(std::path::Path::new(trimmed_path));
                if skipped > files.len() {
                    let left = skipped - files.len();
                    expanded.push_str(&format!(
                        "[dir: {trimmed_path} — {left} file(s) skipped: inline budget exhausted]"
                    ));
                }
                report.push((trimmed_path.to_string(), total, Ok(())));
            }
        } else {
            match std::fs::read_to_string(trimmed_path) {
                Ok(content) => {
                    let truncated = if content.len() > max_chars {
                        format!(
                            "{}\n... (truncated, file is {} chars)",
                            &content[..max_chars],
                            content.len()
                        )
                    } else {
                        content.clone()
                    };
                    let total = truncated.chars().count();
                    expanded.push_str(&format!("[file: {trimmed_path}]\n```\n{truncated}\n```"));
                    report.push((trimmed_path.to_string(), total, Ok(())));
                }
                Err(e) => {
                    // Surface the error inline so the model can react, but keep
                    // the submission alive — the user might have a typo and
                    // want the LLM to ask for clarification.
                    expanded.push_str(&format!("[file: {trimmed_path} — could not read: {e}]"));
                    report.push((trimmed_path.to_string(), 0, Err(e.to_string())));
                }
            }
        }

        // Advance past the original `@path` token.
        rest = &after_at[path_len..];
    }

    expanded.push_str(rest);
    (expanded, report)
}

/// Total char budget across one `@dir` inline. Sized to stay comfortably
/// inside a 32k-token context alongside the prompt itself.
const DIR_INLINE_BUDGET: usize = 60_000;

/// Directory names never inlined — VCS internals, dependency trees, build
/// output. Everything here would be noise to the model at best.
const INLINE_SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".idea",
    ".vscode",
];

/// Extensions considered inline-worthy. Anything else (binaries, images,
/// lockfiles) is skipped rather than fed to the model as mojibake.
const INLINE_TEXT_EXTS: &[&str] = &[
    "rs", "py", "js", "ts", "tsx", "jsx", "md", "toml", "json", "yaml", "yml", "sh", "txt", "go",
    "c", "cpp", "h", "hpp", "css", "html", "sql", "rb", "java", "kt", "swift", "php", "lua",
];

fn inline_wanted(path: &std::path::Path) -> bool {
    let ext_ok = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| INLINE_TEXT_EXTS.contains(&e))
        .unwrap_or(false);
    ext_ok
}

/// Recursively collect readable text files under `dir`, oldest-order stable,
/// until `budget` chars are consumed. Deterministic order (sorted per
/// directory) so repeat invocations inline the same content.
fn collect_dir_files(dir: &std::path::Path, budget: usize) -> Vec<(std::path::PathBuf, String)> {
    let mut out: Vec<(std::path::PathBuf, String)> = Vec::new();
    let mut used = 0usize;
    walk_for_inline(dir, 0, &mut out, &mut used, budget);
    out
}

fn count_skipped_files(dir: &std::path::Path) -> usize {
    let mut n = 0;
    walk_count(dir, 0, &mut n);
    n
}

fn walk_count(dir: &std::path::Path, depth: usize, n: &mut usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            let skip = p
                .file_name()
                .and_then(|f| f.to_str())
                .map(|f| INLINE_SKIP_DIRS.contains(&f))
                .unwrap_or(false);
            if !skip {
                walk_count(&p, depth + 1, n);
            }
        } else if inline_wanted(&p) {
            *n += 1;
        }
    }
}

fn walk_for_inline(
    dir: &std::path::Path,
    depth: usize,
    out: &mut Vec<(std::path::PathBuf, String)>,
    used: &mut usize,
    budget: usize,
) {
    if depth > 8 || *used >= budget {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        if *used >= budget {
            return;
        }
        let p = e.path();
        if p.is_dir() {
            let skip = p
                .file_name()
                .and_then(|f| f.to_str())
                .map(|f| INLINE_SKIP_DIRS.contains(&f))
                .unwrap_or(false);
            if !skip {
                walk_for_inline(&p, depth + 1, out, used, budget);
            }
        } else if inline_wanted(&p) {
            if let Ok(content) = std::fs::read_to_string(&p) {
                let take = content.len().min(budget - *used);
                let take = take.min(content.len());
                let content = if take < content.len() {
                    content[..take].to_string()
                } else {
                    content
                };
                *used += content.len();
                out.push((p, content));
            }
        }
    }
}

/// Pull the last N user/assistant pairs out of the visible history and
/// shape them as `Message` structs to send to the LLM. Older turns first,
/// newest last. The current prompt (the thing the user just typed) is
/// appended separately by `call_llm` itself.
fn build_context(history: &[HistoryEntry], turns: usize) -> Vec<crate::Message> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut current_user: Option<String> = None;
    // Walk the history in order. For each User entry, look forward to find
    // the next Ai entry — that's one turn.
    for entry in history {
        match entry {
            HistoryEntry::User(s) => {
                // A new user message starts a new pair; if the previous
                // user had no AI response yet, drop it (LLM never answered).
                current_user = Some(s.clone());
            }
            HistoryEntry::Ai(s) => {
                if let Some(u) = current_user.take() {
                    pairs.push((u, s.clone()));
                }
            }
            _ => {}
        }
    }
    // Keep only the last `turns` pairs.
    let start = pairs.len().saturating_sub(turns);
    let mut msgs = Vec::with_capacity((pairs.len() - start) * 2);
    for (u, a) in &pairs[start..] {
        // Re-apply the language hint to historical user turns too. SYSTEM_PROMPT
        // says "match the user's language" but cheap models get pulled back to
        // whatever the prior assistant turn was — so a single English reply
        // poisons every Chinese prompt that follows. Adding the hint to every
        // historical user message makes it impossible to forget.
        let u_with_hint = format!("{}{u}", crate::detect_language_hint(u));
        msgs.push(crate::Message {
            role: "user".into(),
            content: u_with_hint,
        });
        msgs.push(crate::Message {
            role: "assistant".into(),
            content: a.clone(),
        });
    }
    msgs
}

fn apply_event(state: &mut TuiState, evt: TuiEvent, sound_enabled: bool) {
    match evt {
        TuiEvent::Delta(chunk) => {
            // First chunk: create the in-flight Ai entry. Subsequent chunks:
            // append to it. `streaming` tracks the entry index so the render
            // loop can put a spinner next to it.
            match state.streaming {
                None => {
                    state.history.push(HistoryEntry::Ai(chunk));
                    state.streaming = Some(state.history.len() - 1);
                }
                Some(idx) => {
                    if let Some(HistoryEntry::Ai(buf)) = state.history.get_mut(idx) {
                        buf.push_str(&chunk);
                    }
                }
            }
        }
        TuiEvent::Result(Ok((content, elapsed, usage))) => {
            state.busy = false;
            state.streaming = None;
            let secs = elapsed.as_secs_f32();
            // Token meter in the status line: the cheap-thinking claim is
            // only credible if the user can see the meter.
            let tok = usage
                .map(|u| format!(" · {} tok", u.total()))
                .unwrap_or_default();
            state.status = format!("ready ({secs:.1}s{tok})");
            // The Ai entry is already in history (filled via Delta chunks);
            // we don't push a duplicate here.
            if let Some(ref p) = state.session_path {
                crate::log_event(p, "assistant", &content);
            }
            play_sound(SoundKind::Success, sound_enabled);
        }
        TuiEvent::Result(Err(e)) => {
            state.busy = false;
            state.streaming = None;
            state.status = "ready (error)".into();
            // If we already started streaming, replace that partial entry
            // with the error so the user sees the failure in context.
            match state.streaming {
                Some(idx) if matches!(state.history.get(idx), Some(HistoryEntry::Ai(_))) => {
                    if let Some(HistoryEntry::Ai(buf)) = state.history.get_mut(idx) {
                        let prior = std::mem::take(buf);
                        let cleaned = e.trim().to_string();
                        *state.history.get_mut(idx).unwrap() =
                            HistoryEntry::Error(format!("{prior}\n\n[stream aborted: {cleaned}]"));
                    }
                }
                _ => {
                    state
                        .history
                        .push(HistoryEntry::Error(e.trim().to_string()));
                }
            }
            play_sound(SoundKind::Error, sound_enabled);
        }
    }
}

// ─── Render (two live rows) ────────────────────────────────────────────────────────────

/// Render the only live region: row 0 is the input line (`> …`), row 1 the
/// dim status line. Everything else in the conversation has already been
/// printed to the terminal's scrollback by `flush_pending` — the transcript
/// is not re-rendered here, ever.
fn render(f: &mut ratatui::Frame, state: &TuiState, input: &str) {
    let area = f.area();
    if area.height < 2 || area.width < 8 {
        return; // too small to be useful — avoid divide-by-weird layouts
    }
    let rows = ratatui::layout::Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    let status_row = ratatui::layout::Rect {
        y: area.y + 1,
        ..rows
    };

    // Input line: `> ` prompt + typed text, no box, no border.
    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(MUTED)),
        Span::raw(input.to_string()),
    ]);
    f.render_widget(Paragraph::new(input_line), rows);

    // Status line: everything here is metadata, so everything is dim.
    // While busy, the spinner + live char count carry the liveness that
    // streaming used to provide; idle, the line doubles as the command
    // cheat-sheet so the verdict family is always discoverable.
    let spinner = SPINNER_FRAMES[(state.tick / 4) as usize % SPINNER_FRAMES.len()];
    let status_text = if state.busy {
        let chars = state
            .streaming
            .and_then(|idx| state.history.get(idx))
            .map(|e| match e {
                HistoryEntry::Ai(s) => s.chars().count().to_string(),
                _ => String::new(),
            })
            .unwrap_or_default();
        format!("{spinner} thinking · {chars} chars · esc quits")
    } else if input.is_empty() {
        format!(
            "{} · verdict: premortem/spec/postmortem · ctrl+up/down history · tab · esc",
            if state.status.is_empty() {
                "ready"
            } else {
                &state.status
            },
        )
    } else {
        format!(
            "{} · tab completes · enter sends",
            if state.status.is_empty() {
                "ready"
            } else {
                &state.status
            },
        )
    };
    let status_line = Line::from(Span::styled(
        format!(" {status_text}"),
        Style::default().fg(MUTED),
    ));
    f.render_widget(Paragraph::new(status_line), status_row);

    // Cursor at the end of the typed input, on row 0. While busy the
    // cursor is hidden — there is nothing to type into.
    if !state.busy {
        // Display columns, not char count — CJK chars are 2 columns wide.
        let offset = 2 + display_width(input) as u16;
        let x = (area.x + offset).min(area.x + area.width.saturating_sub(1));
        f.set_cursor_position(Position::new(x, area.y));
    }
}

// ─── Scrollback flush ──────────────────────────────────────────────────────────────────

/// Estimated terminal-row height of `line` when wrapped at `width`
/// columns. Uses `display_width`, so CJK text wraps on the same accounting
/// ratatui and real terminals use. Empty lines still take one row.
fn line_height(line: &Line<'_>, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }
    let w: usize = line
        .spans
        .iter()
        .map(|sp| display_width(sp.content.as_ref()))
        .sum();
    if w == 0 {
        1
    } else {
        (w as u16).div_ceil(width).max(1)
    }
}

/// Print every finished history entry to the terminal's scrollback with a
/// single `insert_before` per batch. The in-flight streaming entry stays
/// unflushed until its `Result` event finalizes it.
fn flush_pending(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut TuiState,
) -> Result<()> {
    // One past the last entry we may flush: the streaming entry, if any,
    // is still being written by Delta events.
    let limit = match state.streaming {
        Some(idx) => idx,
        None => state.history.len(),
    };
    if state.flushed >= limit {
        return Ok(());
    }

    let width = terminal.size().map(|s| s.width).unwrap_or(80);
    let mut lines: Vec<Line<'_>> = Vec::new();
    for entry in &state.history[state.flushed..limit] {
        lines.extend(entry_to_lines(entry));
    }
    // u16::MAX ≈ 65k rows ≈ 900 screens — no realistic transcript hits the
    // ceiling. Accumulate in u32 and saturate so a pathological log degrades
    // to a clipped insert instead of an overflow panic.
    let height: u16 = lines
        .iter()
        .map(|l| line_height(l, width) as u32)
        .sum::<u32>()
        .min(u16::MAX as u32) as u16;
    if height == 0 {
        state.flushed = limit;
        return Ok(());
    }

    let x = 0u16;
    terminal
        .insert_before(height, |buf| {
            let mut y = 0u16;
            for line in &lines {
                let h = line_height(line, width);
                buf.set_line(x, y, line, width);
                y += h;
            }
        })
        .context("insert transcript into scrollback")?;
    state.flushed = limit;
    Ok(())
}

/// A line counts as a "verdict" when it carries the closing decision of an
/// autopsy or a spec — the moment that the red accent is earned. Match the
/// numbered leading items the prompts emit: "5." for premortem's verdict,
/// "Verdict" / "Open questions" headers, anything starting with a numbered
/// "6." in case the model reordered. Conservative on purpose — false
/// negatives lose a bit of emphasis, false positives turn unrelated text
/// into noise.
fn is_verdict_line(line: &str) -> bool {
    let t = line.trim_start();
    let lower = t.to_lowercase();
    lower.starts_with("verdict")
        || lower.starts_with("5.")
        || lower.starts_with("6.")
        || lower.starts_with("judgment")
        || lower.starts_with("decision")
        // The model replies in the user's language (see detect_language_hint),
        // so Chinese verdicts must light up too. Section 5 of the premortem
        // prompt is the verdict; 判决/结论/决定 cover how Chinese models
        // actually head that section.
        || t.starts_with("判决")
        || t.starts_with("结论")
        || t.starts_with("决定")
}

fn entry_to_lines(entry: &HistoryEntry) -> Vec<Line<'_>> {
    match entry {
        // A submitted command, transcript-style: `> premortem x`.
        HistoryEntry::User(s) => vec![Line::from(vec![
            Span::styled("> ", Style::default().fg(MUTED)),
            Span::raw(s.clone()),
        ])],
        HistoryEntry::Ai(s) => {
            if s.trim().is_empty() {
                return vec![Line::from(Span::styled(
                    "(no content)",
                    Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                ))];
            }
            // The report, verbatim. One accent, one job: verdict lines red.
            let mut lines: Vec<Line<'_>> = vec![Line::from("")]; // breathing room
            for line in s.lines() {
                if is_verdict_line(line) {
                    lines.push(Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(ACCENT_RED).add_modifier(Modifier::BOLD),
                    )));
                } else {
                    lines.push(Line::from(line.to_string()));
                }
            }
            lines.push(Line::from("")); // breathing room
            lines
        }
        HistoryEntry::Error(s) => s
            .lines()
            .map(|l| {
                Line::from(Span::styled(
                    format!("! {l}"),
                    Style::default().fg(ACCENT_RED),
                ))
            })
            .collect(),
        HistoryEntry::Info(s) => vec![Line::from(Span::styled(
            format!("  {s}"),
            Style::default().fg(MUTED),
        ))],
    }
}

// ─── Sound (8-bit via Windows Beep FFI) ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum SoundKind {
    Submit,
    Success,
    Error,
}

/// Play a one-shot UI sound effect. Returns immediately — the actual tones
/// run on a tokio worker thread so they don't block the render loop.
fn play_sound(kind: SoundKind, enabled: bool) {
    if !enabled {
        return;
    }
    tokio::spawn(async move {
        match kind {
            SoundKind::Submit => beep::tone(880, 25),
            SoundKind::Success => {
                beep::tone(523, 70);
                tokio::time::sleep(Duration::from_millis(75)).await;
                beep::tone(659, 70);
                tokio::time::sleep(Duration::from_millis(75)).await;
                beep::tone(784, 120);
            }
            SoundKind::Error => beep::tone(220, 180),
        }
    });
}

/// Spawn a background task that loops an 8-bit bassline forever.
/// Each note plays through the PC speaker, with a small gap. Loops until
/// the TUI exits (the task dies with the process).
fn play_background_music() {
    tokio::spawn(async move {
        // A simple ascending-descending arpeggio in C major. Repeats.
        // Pattern: C E G C(high) G E C — 16 steps, ~1.6s per loop.
        let pattern: &[(u32, u64)] = &[
            (262, 140), // C4
            (330, 140), // E4
            (392, 140), // G4
            (523, 200), // C5
            (392, 140), // G4
            (330, 140), // E4
            (262, 200), // C4 (sustain)
            (196, 280), // G3 (bass drop)
        ];
        loop {
            for &(freq, dur) in pattern {
                beep::tone(freq, dur as u32);
                // Small gap between notes — Beep already blocks for `dur`, so
                // we just need a tiny buffer to keep it from sounding buzzed.
                tokio::time::sleep(Duration::from_millis(15)).await;
            }
            // Pause before next loop so it doesn't feel relentless.
            tokio::time::sleep(Duration::from_millis(600)).await;
        }
    });
}

#[cfg(target_os = "windows")]
mod beep {
    extern "system" {
        fn Beep(dwFreq: u32, dwDuration: u32) -> i32;
    }

    /// Play a single tone through the PC speaker. Blocks the calling thread
    /// for `duration_ms`. On modern Windows without a PC speaker, this is a
    /// no-op that returns immediately.
    pub fn tone(freq: u32, duration_ms: u32) {
        unsafe {
            let _ = Beep(freq, duration_ms);
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod beep {
    /// No-op on non-Windows. The TUI stays silent and the rest of the code
    /// doesn't have to know about the platform.
    pub fn tone(_freq: u32, _duration_ms: u32) {}
}

// ─── Tests ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_detects_premortem_section_5() {
        assert!(is_verdict_line(
            "5. Verdict — build it at half scope or don't."
        ));
        assert!(is_verdict_line("5. Judgment"));
        assert!(is_verdict_line("5. Decision: skip the desktop build."));
    }

    #[test]
    fn verdict_detects_unnumbered_header() {
        assert!(is_verdict_line("Verdict: skip the desktop build."));
        assert!(is_verdict_line("verdict — build at half scope"));
        assert!(is_verdict_line("Decision: not now."));
        assert!(is_verdict_line("Judgment."));
    }

    #[test]
    fn verdict_ignores_unrelated_sections() {
        assert!(!is_verdict_line("1. Cause of death — ..."));
        assert!(!is_verdict_line("2. Ranked killers"));
        assert!(!is_verdict_line("Constraints: must run on Windows"));
        assert!(!is_verdict_line("# Goal — one paragraph"));
    }

    #[test]
    fn verdict_detects_chinese_headers() {
        // Models answering in Chinese (detect_language_hint) head the
        // verdict section with Chinese words; these must light up red too.
        assert!(is_verdict_line("判决：不要按原计划构建。"));
        assert!(is_verdict_line("结论 — 改做最小版本。"));
        assert!(is_verdict_line("决定：先跑两周再看。"));
        // But ordinary Chinese prose must not.
        assert!(!is_verdict_line("做一个开源工作流平台"));
    }

    #[test]
    fn verdict_tolerates_leading_whitespace() {
        // History pane wraps prose with leading spaces in some terminals;
        // the matcher must trim before deciding.
        assert!(is_verdict_line("   Verdict: ship the smallest cut."));
        assert!(is_verdict_line("\t5. Verdict."));
    }

    // ─── display_width ────────────────────────────────────────────────────────

    #[test]
    fn width_ascii_counts_one_per_char() {
        assert_eq!(display_width(""), 0);
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width("make a stock monitor"), 20);
    }

    #[test]
    fn width_cjk_counts_two_per_char() {
        // The actual bug — CJK chars must take 2 columns each or the
        // cursor lands mid-glyph when typing Chinese.
        assert_eq!(display_width("做爬虫"), 6); // 3 chars × 2 cols
        assert_eq!(display_width("做个知乎热榜爬虫"), 16); // 8 chars × 2 cols
    }

    #[test]
    fn width_mixed_strings() {
        assert_eq!(display_width("a做b"), 4); // 1 + 2 + 1
        assert_eq!(display_width("hello 世界"), 10); // 5 + 1 + 4
    }

    // ─── build_context applies language hint to historical user turns ────────

    fn history(user: &[&str], ai: &[&str]) -> Vec<HistoryEntry> {
        // Interleave user/ai — pairs must have one of each.
        let mut out = Vec::new();
        for (u, a) in user.iter().zip(ai.iter()) {
            out.push(HistoryEntry::User(u.to_string()));
            out.push(HistoryEntry::Ai(a.to_string()));
        }
        out
    }

    #[test]
    fn context_applies_hint_to_historical_chinese_turns() {
        // The actual reported bug: after one English reply, the model would
        // fall back to English even on a Chinese prompt. The fix was to add
        // the language hint to *every* historical user message, not just the
        // current one. Verify the hint sticks for old turns too.
        let h = history(
            &["做知乎热榜爬虫", "分析评论"],
            &["english reply one", "english reply two"],
        );
        let msgs = build_context(&h, 5);
        assert_eq!(msgs.len(), 4);
        // Both historical user messages must carry the hint prefix.
        assert!(msgs[0].content.starts_with("[Respond in Chinese.] "));
        assert!(msgs[2].content.starts_with("[Respond in Chinese.] "));
        // Assistant turns are unchanged.
        assert_eq!(msgs[1].content, "english reply one");
        assert_eq!(msgs[3].content, "english reply two");
    }

    // ─── collect_dir_files (@dir inline) ──────────────────────────────────────

    fn write_tmp(name: &str, rel: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("naysay-dir-{name}-{}", std::process::id()));
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, content).unwrap();
        dir
    }

    #[test]
    fn collect_dir_includes_text_files_recursively_and_skips_noise() {
        let dir = write_tmp("incl", "src/a.rs", "fn main() {}");
        std::fs::create_dir_all(dir.join("src/deep")).unwrap();
        std::fs::write(dir.join("src/deep/b.md"), "# doc").unwrap();
        std::fs::create_dir_all(dir.join("node_modules")).unwrap();
        std::fs::write(dir.join("node_modules/x.rs"), "junk").unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join(".git/y.rs"), "junk").unwrap();
        std::fs::write(dir.join("logo.bin"), [0u8, 1, 2]).unwrap();

        let files = collect_dir_files(&dir.join("src"), DIR_INLINE_BUDGET);
        let names: Vec<String> = files.iter().map(|(p, _)| p.display().to_string()).collect();
        assert_eq!(files.len(), 2, "{names:?}");
        assert!(names.iter().any(|n| n.ends_with("a.rs")));
        assert!(names.iter().any(|n| n.ends_with("b.md")));
        assert!(names
            .iter()
            .all(|n| !n.contains("node_modules") && !n.contains(".git")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn collect_dir_respects_budget_and_stays_deterministic() {
        let dir = write_tmp("budget", "big1.rs", &"x".repeat(700));
        std::fs::write(dir.join("big2.rs"), "y".repeat(700)).unwrap();
        let files = collect_dir_files(&dir, 1000);
        // Budget 1000 chars: the first (sorted) file takes 700, the second
        // only gets the remaining 300 — and the order is stable.
        assert_eq!(files.len(), 2);
        assert!(files[0].1.len() == 700 && files[1].1.len() == 300);
        let again = collect_dir_files(&dir, 1000);
        assert_eq!(files, again);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn collect_dir_empty_or_missing_is_empty() {
        let dir = std::env::temp_dir().join(format!("naysay-dir-none-{}", std::process::id()));
        assert!(collect_dir_files(&dir, 1000).is_empty());
        assert!(collect_dir_files(&dir.join("a.rs"), 1000).is_empty());
    }

    #[test]
    fn context_does_not_add_hint_to_latin_user_turns() {
        let h = history(&["make a stock monitor"], &["english reply"]);
        let msgs = build_context(&h, 1);
        assert_eq!(msgs[0].content, "make a stock monitor");
    }
}
