import io

with io.open("src/tui.rs", encoding="utf-8") as f:
    s = f.read()

fails = []
def rep(old, new, count=1):
    global s
    n = s.count(old)
    if n != count:
        fails.append("count %d != %d: %r" % (n, count, old[:70]))
        return
    s = s.replace(old, new)

# Banner block
banner_block_old = '''    state.history.push(HistoryEntry::Info(format!(
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
        .push(HistoryEntry::Info("  Esc / Ctrl+C     quit".into()));'''
banner_block_new = '''    let host = endpoint_host(&config().chat_url);
    let banner = [
        ui_text::BANNER_HEADER.replace("{model}", &state.model).replace("{host}", &host),
        ui_text::BANNER_INTRO.to_string(),
        ui_text::BANNER_VERDICT.to_string(),
        ui_text::BANNER_GENERATION.to_string(),
        ui_text::BANNER_ANALYSIS.to_string(),
        ui_text::BANNER_READING.to_string(),
        ui_text::BANNER_SESSION.to_string(),
        ui_text::BANNER_HELP.to_string(),
    ];
    for line in banner {
        state.history.push(HistoryEntry::Info(line));
    }
    if let Some(ref p) = state.session_path {
        state.history.push(HistoryEntry::Info(
            ui_text::BANNER_LOGGING.replace("{path}", &p.display().to_string()),
        ));
    }
    state.history.push(HistoryEntry::Info(ui_text::BANNER_QUIT.to_string()));'''
rep(banner_block_old, banner_block_new)

# Resume OK info
rep('''                state.history.push(HistoryEntry::Info(format!(
                    "[ok] resumed {resumed_turns} turns from {} — new turns append to the same session",
                    path.display()
                )));''',
    '''                state.history.push(HistoryEntry::Info(
                    ui_text::BANNER_RESUME
                        .replace("{n}", &resumed_turns.to_string())
                        .replace("{path}", &path.display().to_string()),
                ));''')
rep('''                state
                    .history
                    .push(HistoryEntry::Error(format!("resume failed: {e}")));''',
    '''                state.history.push(HistoryEntry::Error(
                    ui_text::RESUME_FAILED.replace("{err}", &format!("{e}")),
                ));''')

# /clear
rep('''        state.history.push(HistoryEntry::Info(format!(
            "[ok] cleared {dropped} remembered entries (the transcript above stays in scrollback)"
        )));''',
    '''        state.history.push(HistoryEntry::Info(
            ui_text::CLEARED.replace("{n}", &dropped.to_string()),
        ));''')

# /resume busy / none
rep('''            state.history.push(HistoryEntry::Error(
                "wait for the current call to finish, then /resume".into(),
            ));''',
    '''            state.history.push(HistoryEntry::Error(ui_text::WAIT_RESUME.into()));''')
rep('''            state.history.push(HistoryEntry::Error(
                "no session found to resume (start one first)".into(),
            ));''',
    '''            state.history.push(HistoryEntry::Error(ui_text::RESUME_NONE.into()));''')

# Regenerate / export
rep('''                                        state.history.push(HistoryEntry::Info(
                                            format!("[↻] regenerating: {cmd}"),
                                        ));''',
    '''                                        state.history.push(HistoryEntry::Info(
                                            ui_text::REGEN_INFO.replace("{cmd}", &cmd),
                                        ));''')
rep('''                                    Ok(path) => {
                                        state.history.push(HistoryEntry::Info(
                                            format!("[ok] exported conversation to {}", path.display()),
                                        ));
                                    }
                                    Err(e) => {
                                        state.history.push(HistoryEntry::Error(
                                            format!("export failed: {e}"),
                                        ));
                                    }''',
    '''                                    Ok(path) => {
                                        state.history.push(HistoryEntry::Info(
                                            ui_text::EXPORTED
                                                .replace("{path}", &path.display().to_string()),
                                        ));
                                    }
                                    Err(e) => {
                                        state.history.push(HistoryEntry::Error(
                                            ui_text::EXPORT_FAILED.replace("{err}", &format!("{e}")),
                                        ));
                                    }''')

# Tab completion status
rep('''            state.status = format!("no command starts with '{prefix}'");''',
    '''            state.status = ui_text::TAB_NO_MATCH.replace("{prefix}", prefix);''')
rep('''                state.status = format!("{} options — Tab again to cycle", candidates.len());''',
    '''                state.status = ui_text::TAB_CYCLE.replace("{n}", &candidates.len().to_string());''')

# Status busy + ready
rep('''            state.status = format!("thinking [{kind}]…");''',
    '''            state.status = ui_text::STATUS_BUSY.replace("{kind}", kind);''')
rep('''            state.status = format!("ready ({secs:.1}s{tok})");''',
    '''            state.status = ui_text::STATUS_READY
                .replace("{secs}", &format!("{secs:.1}"))
                .replace("{tok}", &tok);''')

# Render status row: replace the three branch literals
old_render = '''    let status_text = if state.busy {
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
            if state.status.is_empty() { "ready" } else { &state.status },
        )
    } else {
        format!(
            "{} · tab completes · enter sends",
            if state.status.is_empty() { "ready" } else { &state.status },
        )
    };'''
new_render = '''    let status_text = if state.busy {
        let chars = state
            .streaming
            .and_then(|idx| state.history.get(idx))
            .map(|e| match e {
                HistoryEntry::Ai(s) => s.chars().count().to_string(),
                _ => String::new(),
            })
            .unwrap_or_default();
        ui_text::STATUS_READY_BUSY
            .replace("{spinner}", spinner)
            .replace("{chars}", &chars)
    } else if input.is_empty() {
        let s = if state.status.is_empty() { "ready" } else { &state.status };
        ui_text::STATUS_READY_IDLE.replace("{status}", s)
    } else {
        let s = if state.status.is_empty() { "ready" } else { &state.status };
        ui_text::STATUS_READY_TYPING.replace("{status}", s)
    };'''
rep(old_render, new_render)

# Help text: the giant replacement
old_help = '''            let help_text = "commands:\\n  \\
                 premortem <idea>    assume it died in 6 months — the autopsy\\n  \\
                 postmortem <idea>   it's over — the review + decision-log entry\\n  \\
                 spec <idea>         harden an idea into a spec for your agent\\n  \\
                 seed <topic>        brainstorm 8 angles\\n  \\
                 drill <idea>        drill into an idea\\n  \\
                 explain <file>      walk through code\\n  \\
                 /context N          prior turns the AI sees, 0..=10 (now {n})\\n  \\
                 /clear              wipe REPL conversation memory\\n  \\
                 key set|status|del  manage API key\\n  \\
                 sessions list|show  browse past sessions\\n  \\
                 quit | exit         leave naysay\\n\\n\\
                 the AI sees your last few turns, so follow-ups work:\\n\\
                 \\"what about X?\\" or \\"drill into #2\\".".replace("{n}", &st.context_turns.to_string());'''
new_help = '''            let help_text = ui_text::HELP
                .replace("{n}", &st.context_turns.to_string())
                .replace("{m}", &state_model_dummy(st));
            println!("{help_text}");'''
rep(old_help, new_help)

# Actually need a `state_model_dummy` helper or just inline. Simpler: substitute empty for {m} in HELP since REPL doesn't show current model.
new_help = '''            let help_text = ui_text::HELP
                .replace("{n}", &st.context_turns.to_string())
                .replace("{m}", &config().model);
            println!("{help_text}");'''
rep(new_help, new_help)
# ^ supersedes the previous one. Good.

# export_conversation header literals
rep('''        writeln!(f, "# naysay conversation\\n")?;''',
    '''        writeln!(f, "{}", ui_text::EXPORT_TITLE)?;''')
rep('''        writeln!(f, "_exported at epoch {ts}_\\n")?;''',
    '''        writeln!(f, "{}", ui_text::EXPORT_TS_TAG.replace("{ts}", &ts.to_string()))?;''')
rep('''            HistoryEntry::User(s) => {
                writeln!(f, "**you**\\n\\n{s}\\n")?;
            }
            HistoryEntry::Ai(s) => {
                writeln!(f, "**naysay**\\n\\n{s}\\n")?;
            }
            HistoryEntry::Error(s) => {
                writeln!(f, "> ⚠ {s}\\n")?;
            }''',
    '''            HistoryEntry::User(s) => {
                writeln!(f, "{}\\n\\n{s}\\n", ui_text::EXPORT_USER_HEAD)?;
            }
            HistoryEntry::Ai(s) => {
                writeln!(f, "{}\\n\\n{s}\\n", ui_text::EXPORT_AI_HEAD)?;
            }
            HistoryEntry::Error(s) => {
                writeln!(f, "{}", ui_text::EXPORT_ERR_HEAD.replace("{err}", s))?;
            }''')

with io.open("src/tui.rs", "w", encoding="utf-8", newline="\n") as f:
    f.write(s)

print("tui.rs ui_text migrations:", len(fails))
for x in fails: print(" ", x)
