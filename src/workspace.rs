//! Fullscreen three-pane workspace (D-035): exploration and the decision
//! store left, the transcript centre, the current decision's verdict and
//! assumptions right.
//!
//! The inline transcript this replaces is not gone — `--inline` still
//! renders it until the workspace has been used for a week (M5).
//!
//! The transcript is in-memory: `TuiState::history` is wrapped into rows
//! for the centre pane every frame. Nothing is printed to the terminal's
//! scrollback, so exiting loses it — `--save` and the session JSONL are the
//! durable copies (the trade D-035 accepted).

use std::io;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::store::{self, Assumption, DecisionRecord};
use crate::text::{char_width, display_width, input_window, wrap_entry_lines};
use crate::tui::{
    apply_event, ctrl_c_pressed, debug_log, edit_text, entry_to_lines, export_conversation,
    handle_key, status_text, submit_line, ui_text, HistoryEntry, KeyAction, TuiEvent, TuiState,
    ACCENT_RED, MUTED,
};
use crate::Prompts;

/// Store rows the left pane keeps on screen — a browser, not a database view.
const STORE_ROWS: usize = 20;
/// Session steps the left pane shows, newest kept — the store list below
/// must not be pushed off screen by a long exploration.
const SESSION_ROWS: usize = 8;
/// Transcript rows moved per PageUp / PageDown.
const PAGE: u16 = 10;

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum Focus {
    #[default]
    Input,
    Search,
}

/// Workspace-local view state. Nothing here is conversation state, so it
/// dies with the window; the store snapshots reload after every write.
#[derive(Default)]
struct View {
    /// Transcript offset in lines from the tail. 0 = following the stream.
    scroll: u16,
    /// Left-pane store filter, edited while `focus == Focus::Search`.
    search: String,
    search_cursor: usize,
    focus: Focus,
    session: Option<store::DecisionSession>,
    records: Vec<DecisionRecord>,
    registry: Vec<Assumption>,
}

impl View {
    /// Re-read the session pointer, the decision store and the assumption
    /// registry. Called on start, after every submit, and on every result.
    fn reload(&mut self) {
        self.session = store::load_current_session();
        let dir = store::decisions_dir().ok();
        self.records = dir
            .as_deref()
            .map(store::load_all_records)
            .unwrap_or_default();
        self.registry = dir.as_deref().map(store::load_registry).unwrap_or_default();
    }

    /// The decision the right pane shows: the newest one the session saved,
    /// else the newest record in the store.
    fn current_record(&self) -> Option<&DecisionRecord> {
        let saved = self.session.as_ref().and_then(|s| {
            s.steps
                .iter()
                .rev()
                .find_map(|step| step.saved_ref.as_deref())
        });
        if let Some(id) = saved {
            if let Some(rec) = self.records.iter().find(|r| r.id == id) {
                return Some(rec);
            }
        }
        self.records.iter().max_by_key(|r| r.ts)
    }

    /// Store rows matching the filter, newest first. Every whitespace-
    /// separated term must appear somewhere in the row.
    fn filtered(&self) -> Vec<&DecisionRecord> {
        let terms: Vec<String> = self
            .search
            .split_whitespace()
            .map(|t| t.to_lowercase())
            .collect();
        let mut rows: Vec<&DecisionRecord> = self
            .records
            .iter()
            .filter(|r| terms.iter().all(|t| record_matches(r, t)))
            .collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.ts));
        rows
    }
}

fn record_matches(r: &DecisionRecord, term: &str) -> bool {
    format!(
        "{} {} {} {}",
        r.kind,
        r.id,
        r.idea,
        r.verdict.as_deref().unwrap_or("")
    )
    .to_lowercase()
    .contains(term)
}

/// Handles the loop needs that are not view state. Bundled so the loop
/// keeps a readable argument list.
struct Ctx<'a> {
    rx: &'a mpsc::Receiver<TuiEvent>,
    tx: &'a mpsc::Sender<TuiEvent>,
    prompts: &'a Arc<Prompts>,
    sound_enabled: bool,
}

pub(crate) async fn run(
    state: &mut TuiState,
    rx: &mpsc::Receiver<TuiEvent>,
    tx: &mpsc::Sender<TuiEvent>,
    prompts: &Arc<Prompts>,
    sound_enabled: bool,
) -> Result<()> {
    let mut input = String::new();
    let mut view = View::default();
    view.reload();
    debug_log("workspace::run entered");
    let ctx = Ctx {
        rx,
        tx,
        prompts,
        sound_enabled,
    };

    let mut terminal = setup()?;
    let result = event_loop(&mut terminal, state, &mut input, &mut view, &ctx).await;
    restore(&mut terminal);
    debug_log(&format!("workspace loop ended: {:?}", result.is_ok()));
    result
}

fn setup() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode().context("enable raw mode")?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen, Hide).context("enter alternate screen")?;
    let terminal = Terminal::new(CrosstermBackend::new(out)).context("ratatui terminal")?;
    install_panic_restore();
    Ok(terminal)
}

fn restore(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen, Show);
    let _ = terminal.show_cursor();
}

/// A panic must not strand the user in the alternate screen with raw mode
/// on and no prompt: restore the terminal first, then run the normal hook
/// (which writes panic.log).
fn install_panic_restore() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
            prev(info);
        }));
    });
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut TuiState,
    input: &mut String,
    view: &mut View,
    ctx: &Ctx<'_>,
) -> Result<()> {
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();
    loop {
        // Drain pending LLM results; a finished call is also the moment the
        // store may have grown, so the panes reload then.
        let mut finished = false;
        while let Ok(evt) = ctx.rx.try_recv() {
            finished |= matches!(evt, TuiEvent::Result(_));
            apply_event(state, evt, ctx.sound_enabled);
        }
        if finished {
            view.reload();
        }
        if ctrl_c_pressed() {
            debug_log("Ctrl+C signal received — leaving workspace");
            return Ok(());
        }

        terminal.draw(|f| draw(f, state, input, view))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match handle_workspace_key(key, state, input, view) {
                    KeyAction::None => {}
                    KeyAction::Quit => return Ok(()),
                    KeyAction::Submit(line) => {
                        submit_line(
                            line,
                            state,
                            ctx.tx,
                            ctx.sound_enabled,
                            Arc::clone(ctx.prompts),
                        );
                        // A new turn means the user wants to watch it stream.
                        view.scroll = 0;
                        view.reload();
                    }
                    KeyAction::Save => match export_conversation(&state.history) {
                        Ok(path) => state.history.push(HistoryEntry::Info(format!(
                            "[ok] exported conversation to {}",
                            path.display()
                        ))),
                        Err(e) => state.history.push(HistoryEntry::Error(
                            ui_text::EXPORT_FAILED.replace("{err}", &format!("{e}")),
                        )),
                    },
                    KeyAction::Regenerate => {
                        if let Some(cmd) = state.last_command.clone() {
                            if !state.busy {
                                state
                                    .history
                                    .push(HistoryEntry::Info(format!("[↻] regenerating: {cmd}")));
                                submit_line(
                                    cmd,
                                    state,
                                    ctx.tx,
                                    ctx.sound_enabled,
                                    Arc::clone(ctx.prompts),
                                );
                            }
                        }
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            state.tick = state.tick.wrapping_add(1);
            last_tick = Instant::now();
        }
    }
}

/// Workspace keys on top of the shared input handling: transcript scrolling,
/// the store-filter field, and Ctrl+F to focus it.
fn handle_workspace_key(
    key: KeyEvent,
    state: &mut TuiState,
    input: &mut String,
    view: &mut View,
) -> KeyAction {
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q'))
    {
        return KeyAction::Quit;
    }

    if view.focus == Focus::Search {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => view.focus = Focus::Input,
            _ => {
                edit_text(&key, &mut view.search, &mut view.search_cursor);
            }
        }
        return KeyAction::None;
    }

    match key.code {
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            view.focus = Focus::Search;
            return KeyAction::None;
        }
        // The inline build left these to the terminal's own scrollback;
        // a fullscreen workspace has to own them.
        KeyCode::PageUp => {
            view.scroll = view.scroll.saturating_add(PAGE);
            return KeyAction::None;
        }
        KeyCode::PageDown => {
            view.scroll = view.scroll.saturating_sub(PAGE);
            return KeyAction::None;
        }
        KeyCode::Up if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            view.scroll = view.scroll.saturating_add(1);
            return KeyAction::None;
        }
        KeyCode::Down if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            view.scroll = view.scroll.saturating_sub(1);
            return KeyAction::None;
        }
        // With an empty input Home/End have nothing to edit, so they jump to
        // the oldest row and back to the tail.
        KeyCode::Home if input.is_empty() => {
            view.scroll = u16::MAX;
            return KeyAction::None;
        }
        KeyCode::End if input.is_empty() => {
            view.scroll = 0;
            return KeyAction::None;
        }
        _ => {}
    }
    handle_key(key, input, state)
}

fn draw(f: &mut Frame, state: &TuiState, input: &str, view: &View) {
    let cols = Layout::horizontal([
        Constraint::Percentage(22),
        Constraint::Min(24),
        Constraint::Percentage(28),
    ])
    .split(f.area());
    draw_exploration(f, cols[0], view);
    draw_transcript(f, cols[1], state, input, view);
    draw_decision(f, cols[2], view);
}

fn draw_transcript(f: &mut Frame, area: Rect, state: &TuiState, input: &str, view: &View) {
    let title = if view.scroll == 0 {
        ui_text::WS_TITLE_TRANSCRIPT.to_string()
    } else {
        ui_text::WS_TITLE_SCROLLED.replace("{n}", &view.scroll.to_string())
    };
    let block = Block::bordered().title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);

    // Transcript: pre-wrapped rows, scrolled from the tail.
    let lines = transcript_lines(state, rows[0].width as usize);
    let max_scroll = lines.len().saturating_sub(rows[0].height as usize);
    let offset = max_scroll.saturating_sub(view.scroll as usize);
    f.render_widget(
        Paragraph::new(lines).scroll((offset.min(u16::MAX as usize) as u16, 0)),
        rows[0],
    );

    // Input row, with the terminal cursor placed by display width so CJK
    // lands on the right column.
    let max_text = (rows[1].width as usize).saturating_sub(3);
    let (window, prefix_w) = input_window(input, state.cursor, max_text);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("> ", Style::default().fg(MUTED)),
            Span::raw(window),
        ])),
        rows[1],
    );
    if !state.busy && view.focus == Focus::Input {
        let x = rows[1]
            .x
            .saturating_add(2)
            .saturating_add(prefix_w as u16)
            .min(rows[1].right().saturating_sub(1));
        f.set_cursor_position(Position::new(x, rows[1].y));
    }

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            status_text(state, input),
            Style::default().fg(MUTED),
        ))),
        rows[2],
    );
}

fn draw_exploration(f: &mut Frame, area: Rect, view: &View) {
    let block = Block::bordered().title(ui_text::WS_TITLE_EXPLORATION);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let width = inner.width as usize;

    let focused = view.focus == Focus::Search;
    let label_style = if focused {
        Style::default().fg(ACCENT_RED)
    } else {
        Style::default().fg(MUTED)
    };
    let mut lines: Vec<Line<'static>> = vec![Line::from(vec![
        Span::styled(ui_text::WS_FILTER, label_style),
        Span::raw(clip(
            &view.search,
            width.saturating_sub(display_width(ui_text::WS_FILTER)),
        )),
    ])];

    lines.push(Line::from(""));
    match &view.session {
        Some(s) => {
            let head = format!("{} · ", s.id);
            lines.push(Line::from(Span::styled(
                format!(
                    "{head}{}",
                    clip(&s.root_idea, width.saturating_sub(display_width(&head)))
                ),
                Style::default().fg(MUTED),
            )));
            let first = s.steps.len().saturating_sub(SESSION_ROWS);
            for step in &s.steps[first..] {
                let prefix = format!("{:>2} {:<10} ", step.seq, step.op.as_str());
                lines.push(Line::from(format!(
                    "{prefix}{}",
                    clip(&step.input, width.saturating_sub(display_width(&prefix))),
                )));
            }
        }
        None => lines.push(Line::from(Span::styled(
            ui_text::WS_NO_SESSION,
            Style::default().fg(MUTED),
        ))),
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        ui_text::WS_STORE,
        Style::default().fg(MUTED),
    )));
    let rows = view.filtered();
    if rows.is_empty() {
        lines.push(Line::from(Span::styled(
            ui_text::WS_NO_MATCH,
            Style::default().fg(MUTED),
        )));
    } else {
        let current = view.current_record().map(|r| r.id.clone());
        for r in rows.iter().take(STORE_ROWS) {
            let marker = if Some(&r.id) == current.as_ref() {
                "▸ "
            } else {
                "  "
            };
            let prefix = format!("{marker}{}-{} ", r.kind, r.id);
            lines.push(Line::from(format!(
                "{prefix}{}",
                clip(&r.idea, width.saturating_sub(display_width(&prefix))),
            )));
        }
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    if focused {
        let x = inner
            .x
            .saturating_add(display_width(ui_text::WS_FILTER) as u16)
            .saturating_add(prefix_width(&view.search, view.search_cursor))
            .min(inner.right().saturating_sub(1));
        f.set_cursor_position(Position::new(x, inner.y));
    }
}

fn draw_decision(f: &mut Frame, area: Rect, view: &View) {
    let block = Block::bordered().title(ui_text::WS_TITLE_DECISION);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let width = inner.width as usize;

    let mut lines: Vec<Line<'static>> = Vec::new();
    let Some(rec) = view.current_record() else {
        lines.push(Line::from(Span::styled(
            ui_text::WS_NO_DECISION,
            Style::default().fg(MUTED),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        return;
    };

    lines.push(Line::from(Span::styled(
        format!("{}-{} · {}d ago", rec.kind, rec.id, age_days(rec.ts)),
        Style::default().fg(MUTED),
    )));
    lines.push(Line::from(clip(&rec.idea, width)));
    lines.push(Line::from(""));

    let (verdict, verdict_style) = match rec.verdict.as_deref() {
        Some("DON'T BUILD") => (
            "DON'T BUILD",
            Style::default().fg(ACCENT_RED).add_modifier(Modifier::BOLD),
        ),
        Some(v) => (v, Style::default().add_modifier(Modifier::BOLD)),
        None => (ui_text::WS_NONE, Style::default().fg(MUTED)),
    };
    lines.push(Line::from(vec![
        Span::styled(ui_text::WS_VERDICT, Style::default().fg(MUTED)),
        Span::styled(verdict.to_string(), verdict_style),
    ]));
    if let Some(c) = rec.confidence {
        lines.push(Line::from(vec![
            Span::styled(ui_text::WS_CONFIDENCE, Style::default().fg(MUTED)),
            Span::raw(c.to_string()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        ui_text::WS_ASSUMPTIONS,
        Style::default().fg(MUTED),
    )));
    if rec.assumptions.is_empty() {
        lines.push(Line::from(Span::styled(
            ui_text::WS_NONE_EXTRACTED,
            Style::default().fg(MUTED),
        )));
    } else {
        for claim in &rec.assumptions {
            let status = view
                .registry
                .iter()
                .find(|a| a.claim == store::normalize_claim(claim))
                .map(|a| a.status.as_str())
                .unwrap_or("UNKNOWN");
            let (mark, style) = status_mark(status);
            lines.push(Line::from(vec![
                Span::styled(format!("{mark} "), style),
                Span::raw(clip(claim, width.saturating_sub(2))),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        ui_text::WS_LINKED,
        Style::default().fg(MUTED),
    )));
    let mut linked = false;
    if let Some(pid) = &rec.parent {
        if let Some(parent) = view.records.iter().find(|r| r.id == store::bare_id(pid)) {
            lines.push(linked_line("parent", parent, width));
            linked = true;
        }
    }
    for child in view
        .records
        .iter()
        .filter(|r| r.parent.as_deref().map(store::bare_id) == Some(rec.id.as_str()))
    {
        lines.push(linked_line("child", child, width));
        linked = true;
    }
    if !linked {
        lines.push(Line::from(Span::styled(
            ui_text::WS_NONE,
            Style::default().fg(MUTED),
        )));
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn transcript_lines(state: &TuiState, width: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    for entry in &state.history {
        out.extend(wrap_entry_lines(&entry_to_lines(entry), width));
    }
    if out.is_empty() {
        out.push(Line::from(""));
    }
    out
}

fn linked_line(label: &str, rec: &DecisionRecord, width: usize) -> Line<'static> {
    let prefix = format!("{label} {}-{} ", rec.kind, rec.id);
    Line::from(format!(
        "{prefix}{}",
        clip(&rec.idea, width.saturating_sub(display_width(&prefix))),
    ))
}

/// Assumption lifecycle mark. Only INVALIDATED earns the accent — the rest
/// stay dim so the one red thing on screen stays meaningful.
fn status_mark(status: &str) -> (&'static str, Style) {
    match status {
        "INVALIDATED" => ("x", Style::default().fg(ACCENT_RED)),
        "VALID" => ("+", Style::default()),
        _ => ("?", Style::default().fg(MUTED)),
    }
}

fn age_days(ts: u64) -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(ts) / 86_400
}

/// Display width of the text before `cursor` — the search field's cursor
/// column.
fn prefix_width(text: &str, cursor: usize) -> u16 {
    text.chars().take(cursor).map(char_width).sum::<usize>() as u16
}

/// Truncate to `max` display columns, appending `…` when anything was cut.
fn clip(text: &str, max: usize) -> String {
    if display_width(text) <= max {
        return text.to_string();
    }
    let mut out = String::new();
    let mut w = 0usize;
    for c in text.chars() {
        let cw = char_width(c);
        if w + cw > max.saturating_sub(1) {
            break;
        }
        w += cw;
        out.push(c);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn record(kind: &str, id: &str, idea: &str, verdict: Option<&str>) -> DecisionRecord {
        DecisionRecord {
            id: id.into(),
            kind: kind.into(),
            ts: 0,
            idea: idea.into(),
            parent: None,
            body: String::new(),
            assumptions: Vec::new(),
            evidence: Vec::new(),
            unknowns: Vec::new(),
            failure_conditions: Vec::new(),
            confidence: None,
            verdict: verdict.map(str::to_string),
            outcome: None,
            schema_version: 0,
        }
    }

    #[test]
    fn clip_never_exceeds_width_and_marks_cuts() {
        assert_eq!(clip("hello", 10), "hello");
        assert_eq!(clip("hello world", 8), "hello w…");
        // CJK is two columns, so four chars do not fit in seven columns.
        assert_eq!(clip("做一个爬虫", 7), "做一个…");
        assert_eq!(display_width(&clip("做一个爬虫", 7)), 7);
        assert_eq!(clip("", 0), "");
    }

    #[test]
    fn store_filter_requires_every_term() {
        let mut view = View::default();
        view.records = vec![
            record(
                "premortem",
                "premortem-1",
                "stock monitor for retail",
                Some("DON'T BUILD"),
            ),
            record("spec", "spec-2", "stock monitor for retail", None),
            record("check", "check-3", "billing rewrite", Some("BUILD")),
        ];
        view.search = "stock".into();
        assert_eq!(view.filtered().len(), 2);
        view.search = "stock spec".into();
        assert_eq!(view.filtered().len(), 1);
        // Verdict text is searchable: both "BUILD" and "DON'T BUILD" match.
        view.search = "build".into();
        assert_eq!(view.filtered().len(), 2);
        view.search = "zzz".into();
        assert!(view.filtered().is_empty());
    }

    #[test]
    fn draw_renders_three_panes() {
        let state = TuiState::default();
        let view = View::default();
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|f| draw(f, &state, "", &view)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("transcript"), "centre pane missing");
        assert!(text.contains("exploration"), "left pane missing");
        assert!(text.contains("decision"), "right pane missing");
        assert!(text.contains("no active session"), "left pane empty state");
        assert!(
            text.contains("no decision saved yet"),
            "right pane empty state"
        );
    }

    #[test]
    fn empty_input_home_end_move_the_transcript() {
        let mut state = TuiState::default();
        let mut input = String::new();
        let mut view = View::default();
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);

        handle_workspace_key(key(KeyCode::PageUp), &mut state, &mut input, &mut view);
        assert_eq!(view.scroll, PAGE);
        handle_workspace_key(key(KeyCode::Home), &mut state, &mut input, &mut view);
        assert_eq!(view.scroll, u16::MAX, "Home jumps to the oldest row");
        handle_workspace_key(key(KeyCode::End), &mut state, &mut input, &mut view);
        assert_eq!(view.scroll, 0, "End returns to the tail");

        // With text in the input, Home/End edit the line instead.
        input.push_str("x");
        handle_workspace_key(key(KeyCode::Home), &mut state, &mut input, &mut view);
        assert_eq!(view.scroll, 0);
        assert_eq!(state.cursor, 0);
    }
}
