# CODEMAP — what every function does

# CODEMAP — 每个函数做什么

English-only by design — same audience rationale as DECISIONS.md. The README is the bilingual entry point.

This file is part of the codebase. If you change the rules, change this file.
本文件是代码库的一部分。规则若改,本文件随之改。



Companion to `DECISIONS.md` (which answers "why?"). This answers
"what?".

Codebase at v0.11.0: ~9400 lines across `src/main.rs` + `src/tui.rs` +
`src/store.rs` + `src/text.rs` + `src/workspace.rs`.
If you can read all three files end-to-end with this map in hand, you own the
tool. If you can't, that's the part to study next.

---

## `src/main.rs` (≈ 3490 lines)

### CLI layer

| symbol | what it does |
|--------|--------------|
| `Cli` | clap struct — global flags `--save` / `--json` / `--tui` / `--sound` / `--music`, plus optional subcommand. |
| `Command` | subcommand enum: `Repl` / `Seed` / `Drill` / `Premortem` / `Spec` / `Explain` / `Key` / `Sessions` / `Doctor`. |
| `KeyAction` | nested subcommand under `key`: `Set` / `Status` / `Delete`. |
| `SessionsAction` | nested subcommand under `sessions`: `List` / `Show`. |
| `main` | tokio entry point. `install_panic_hook` → parse CLI → dispatch. `--tui` overrides no-subcommand; `--continue` resolves the newest session up front (missing session warns and starts fresh) and applies to both TUI and REPL. |

### Launch path (no subcommand)

| symbol | what it does |
|--------|--------------|
| `decisions relevant <idea>` / `run_d_relevant` | Deterministic retrieval: Jaccard overlap between the query tokens and each record's idea+body tokens. No LLM — interpretation is the caller's job (D-023). |
| `naysay calibration` / `run_calibration` | Links premortem verdicts (incl. the v0.5 structured `VERDICT: BUILD|DON'T BUILD` line) to child postmortem outcomes (`OUTCOME: BUILT|KILLED|ABANDONED|UNKNOWN`). Prints an honesty caveat when the corpus is too small. |
| `extract_verdict` / `extract_outcome` / `classify_verdict_outcome` / `tokenize` / `relevance_score` | Pure helpers behind the two features — all unit-tested. |
| `ensure_key` | First-run onboarding, wherever a model is needed (D-034). If no key (probed via env + `naysay.toml`'s `api_key_env` + keyring, without touching `config()`), and the run may be interactive, walks the provider picker: 6 presets (Ollama / DeepSeek / GLM / OpenAI / MiniMax / OpenRouter) + Custom. Writes `naysay.toml`, saves the key to the keyring, sets the env var for this process. Under a pipe, a redirect, or `--json` it fails fast with the env-var escape hatch instead of prompting. Called from `post_chat_with_retry` (CLI/REPL) and from `launch_interactive` (TUI). |
| `launch_interactive` | `naysay` with no subcommand: `ensure_key(true)`, then hand the terminal to the TUI. |
| `ProviderPreset` / `PRESETS` / `parse_provider_choice` / `provider_toml_body` | The picker's data and pure helpers. Presets are the three naysay.toml fields pre-filled — a new provider is a new row, never new code (D-022). |
| `install_panic_hook` | Writes a panic backtrace to `<data_dir>/panic.log` and eprintln's the path. Keeps debugging possible even when the console closes. |

### LLM wire types (OpenAI chat-completions)

| symbol | what it does |
|--------|--------------|
| `Message` | One turn of conversation, role + content. Serialized verbatim. |
| `ChatRequest` | POST body. Fields: `model`, `messages`, `max_tokens`, `temperature`, optional `stream`. |
| `ChatChoice` / `ChatChoiceMessage` | Non-streaming response body. |
| `ChatChunk` / `ChatChunkChoice` / `ChatChoiceDelta` | Streaming chunk body. `delta` and `content` are `#[serde(default)]` so a missing-field chunk (which finish_reason chunks emit) doesn't abort parsing. `usage` optional — many providers append it to the final chunk. |
| `Usage` | Token accounting for one call (prompt + completion), `total()`. Surfaced via the `LAST_USAGE` store so the UI layer nearest the user can show the meter. |
| `store_last_usage` / `take_last_usage` / `note_usage_stderr` | Set by both call paths after a response; the six command functions print the stderr note (CLI/REPL), the TUI task packs it into `TuiEvent::Result`. |
| `ChatResponse` | Wrapper for non-streaming parse. |

### Provider configuration

| symbol | what it does |
|--------|--------------|
| `Config` | chat_url + model + api_key_env. Loaded from `<data_dir>/naysay.toml`. `OnceLock` for process-wide singleton. A malformed or invalid file is fatal — `config_error()` carries the reason and `main` refuses to run (D-043). |
| `Config::parse_strict` | Parses TOML, surfacing errors. Missing fields → defaults; malformed → `Err` (D-043). |
| `Config::load` | Reads the file, writes the template on first run, applies env-var overrides, validates, returns `Result`. |
| `config()` | Public accessor. First call initializes. |
| `endpoint_host` | Display helper. `https://api.x.com/v1/chat` → `api.x.com`. |
| `CONFIG_TEMPLATE` | What `naysay.toml` looks like on first run — all the provider examples commented out for discoverability. |
| `DEFAULT_CHAT_URL` / `DEFAULT_MODEL` / `DEFAULT_API_KEY_ENV` | Fallback values. MiniMax defaults. |
| `KEYRING_SERVICE` / `KEYRING_USER` | `naysay` / `api-key`. Independent of provider. |

### Prompt externalization

| symbol | what it does |
|--------|--------------|
| `Prompts` | Optional overrides for every command's prompt template (incl. `postmortem`). One field per command. |
| `PromptsFile` | TOML wrapper — the file has a `[prompts]` table. |
| `Prompts::load` | Reads `<data_dir>/prompts.toml`, writes template on first run, falls back to defaults on any error. |
| `Prompts::overrides` / `get` | The key table every surface resolves through; `get` returns the user override or the default. A contract test keeps every key documented in `prompts.toml` and honoured by all surfaces. |
| `PROMPTS_TEMPLATE` | What `prompts.toml` looks like on first run — every key shown but commented. |

### System prompt

| symbol | what it does |
|--------|--------------|
| `SYSTEM_PROMPT` | Single global system prompt. Defines the "interrogate before committing" role and 8 tone rules. Always prepended by both `call_llm` and `call_llm_stream`. |

### Command implementations (CLI/REPL paths)

| symbol | what it does |
|--------|--------------|
| `seed` | `seed <topic>` — 5-10 angles the user probably hasn't considered. Calls `call_llm`, formats with `── topic ──` header. |
| `drill` | `drill <idea>` — 3-5 actionable sub-points. Calls `call_llm`. |
| `premortem` | `premortem <idea>` — assume the idea died in 6 months, write the autopsy (cause, ranked killers, scope autopsy, surviving version, verdict). |
| `check` | `check <decision>` — v0.9 engineering-decision entry point: the same interrogation aimed at a dependency / abstraction / rewrite that is about to become code. Shorter and cheaper than premortem (900 max tokens, temp 0.4) because it runs many times per project, not once. Writes a `check` record to the store and a session step. |
| `spec` | `spec <idea>` — produce a spec the agent can't misinterpret: goal / non-goals / success criteria / constraints / milestones / open questions. |
| `postmortem` | `postmortem <idea> [notes]` — the project is over; write the review (what happened, predicted-vs-actual, decisive moment, cost accounting) plus a self-contained decision-log entry. |
| `explain` | `explain <file>` — read the file, send to LLM with "walk through this file" framing. Truncates to 24k chars. |

### Plain REPL

| symbol | what it does |
|--------|--------------|
| `ReplState` | REPL conversation memory: `history` (full, user turns carry the language hint from birth), `context_turns` (0..=10), `session_path`. `context()` returns the last N pairs; `record()` appends an exchange to memory + logs the assistant side. |
| `repl` | Stdin reader. Opens a session log, replays a resumed session when given one (`--continue`), prints the `naysay>` prompt, dispatches each line via `dispatch_repl`, logs user input. |
| `dispatch_repl` | Naive `command + rest` split over a `&mut ReplState`. Recognized: `help` / `quit` / `seed` / `drill` / `premortem` / `check` / `spec` / `postmortem` / `explain` / `/context` / `/clear` / `key` / `sessions`. LLM-backed commands send the context window and call `record()` afterwards. Unknown → error (no freeform in scripted mode). |

### LLM HTTP

| symbol | what it does |
|--------|--------------|
| `call_llm` | Wrapper. Pins to `config().model`. System prompt + history + user prompt → POST → parse → content. |
| `call_llm_with_model` | Inner. Takes model name explicitly so the TUI can pass `/model`-chosen values. POSTs via `post_chat_with_retry`, stores usage. |
| `post_chat_with_retry` | Resilient POST: 429/5xx retried up to 2x (1s/2s backoff), body never consumed before the retry decision. Silent while the TUI owns the terminal (`TUI_ACTIVE` flag); 10s connect timeout. |
| `is_retryable_status` / `backoff_secs` | Retry policy, pure functions (unit-tested). |
| `call_llm_stream` | Streaming twin. Same request shape but `stream: true`, then `bytes_stream` → `drain_sse_events` per chunk; stops at `[DONE]` and fails after 120 s of silence (D-041). |
| `drain_sse_events` | Hand-rolled SSE parser. Drains complete events from buffer (keeps partial trailing event). Emits deltas, reports usage and the terminal `[DONE]`. Tolerates missing-delta, malformed JSON, finish_reason chunks. **Has 9 unit tests.** |

### Output formatting

| symbol | what it does |
|--------|--------------|
| `emit_output` | Shared by `seed` / `drill` / `premortem` / `spec` / `explain`. `json=false` → pretty-printed text → stdout or `--save` file. `json=true` → JSON object → stdout or file. |

### Credential management

| symbol | what it does |
|--------|--------------|
| `load_api_key` | Lookup chain: configured env var → legacy `MINIMAX_API_KEY` → keyring. First hit wins. |
| `key_set` | Read input → write to keyring. |
| `key_status` | Display where the key is coming from. |
| `key_delete` | Remove from keyring. |

### Doctor

| symbol | what it does |
|--------|--------------|
| `doctor` | Three checks: API key (loaded?), sessions dir (writable?), chat endpoint (reachable?). Exits non-zero on failure. |

### Storage

| symbol | what it does |
|--------|--------------|
| `data_dir` | OS data dir joined with `naysay`. Creates if missing. |
| `session_dir` | `data_dir/sessions`. |
| `open_session_log` | Touch a `session-<epoch>.jsonl` file in the sessions dir. |
| `log_input` | Append one user-input record (JSONL). Delegates to `log_event`. |
| `log_event` | Append one record (`kind`: "user" or "assistant"). Both sides are logged so `--continue` can replay the whole conversation. |
| `SessionRecord` / `load_session_records` | One replayed turn / parse a session JSONL into turns. Skips malformed lines, unknown kinds, and empty texts — resume never fails on one bad line. |
| `latest_session` / `newest_session_in` | Path of the most recent session (lexicographic = chronological on `session-<epoch>.jsonl`). Pure dir-scanner split out for tests. |
| `resolve_session_arg` / `resolve_session_in` | `/resume` and `sessions show` argument → path (digits, filename, or dotted name). Pure helper split out for tests. |
| `sessions_list` | List recent session files. |
| `sessions_show` | Pretty-print one session. |

### Tests (`#[cfg(test)] mod tests`)

22 tests, ~310 lines:

- **`number_lines`** (8 tests) — strips various leading markers from LLM output.
- **`sse_*`** (9 tests) — single event, multiple events, `[DONE]`, empty
  delta, partial event kept in buffer, `event:`/`id:` extras, invalid
  JSON skipped, missing-`delta` field skipped, finish_reason after
  content delivers content.
- **`config_*`** (3 tests) — empty → defaults, provider table overrides,
  malformed → defaults.
- **`endpoint_host`** (2 tests) — scheme stripping, bare-string fallback.
- **`line_height`** — no dedicated tests yet; covered indirectly by display_width tests (it wraps using the same widths).

---

## `src/tui.rs` (≈ 2630 lines)

### Entry + lifecycle

| symbol | what it does |
|--------|--------------|
| `run` | UI entry. Loads config, sanity-checks API key, sets up Windows console ctrl handler (Ctrl+C graceful exit), pushes boot-sequence history, replays a resumed session when given one (`--continue`: turns into history, user turns into input recall, session log reused). Dispatches to `workspace::run` (default); with `--inline` it falls through to the pre-workspace path: raw mode, inline viewport (no alternate screen), flush finished entries to scrollback, render the two-row live strip, restore terminal on exit. |
| `debug_log` | Append-only debug log to `<data_dir>/session.log` (best-effort, never panics). Tagged at every phase boundary so a "TUI flashed and exited" is diagnosable. |
| `ctrl_c_pressed` | Reads the Windows console handler's flag; polled by both loops. |
| `win_console::install` | Sets a Windows console control handler so Ctrl+C → graceful exit instead of SIGKILL. No-op on non-Windows. |
| `CTRL_C_PRESSED` | Atomic flag. Polled by the render loop. |

### State

| symbol | what it does |
|--------|--------------|
| `TuiState` | All mutable UI state: history, busy flag, status string, tick counter, call count, last_command (for `r` regeneration), tab-completion scratchpad, streaming index, context_turns (0..=10), current model, input_history + recall_idx (Ctrl+Up/Down input recall), session_path (JSONL logging), flushed (scrollback cursor — how many history entries are already printed; /clear resets it with the history). |
| `CompletionState` | Tab-completion scratchpad. Reset on any non-Tab keypress. |
| `HistoryEntry` | `User(String)` / `Ai(String)` / `Error(String)` / `Info(String)`. |
| `TuiEvent` | `Delta(String)` (one streamed chunk) / `Result(Result<(String, Duration), String>)` (final outcome + elapsed time). |
| `KeyAction` | `None` / `Quit` / `Submit(String)` / `Save` / `Regenerate`. |

### Input

| symbol | what it does |
|--------|--------------|
| `handle_key` | All key handling. Cursor-addressable editing: chars insert at `state.cursor`, Backspace/Delete/←/→/Home/End, Enter submits. While busy: only quit keys. |
| `edit_text` | Insert / delete / move at the cursor, shared by the main input and the workspace's store-filter field. |
| `apply_completion` | Tab completion on first word. First Tab: longest-common-prefix extension. Repeated Tab: cycle through candidates. |
| `longest_common_prefix` | Helper. |
| `submit_line` | Dispatch a command. Routes to `help` / `/context` / `/clear` / `/model` / `/resume [file]` / the curated command map, else freeform. `@path` inlining before send. Every submission lands in `input_history` and the session log; an LLM response is logged on `Result(Ok)`. Spawns an async task that does the LLM call and pushes `Delta` events into `tx`. |
| `inline_files` | Substitute `@path` tokens with file contents (truncated to 24k chars); `@dir` inlines a whole source tree. Returns `(expanded, InlineReport)` so the UI can confirm what was loaded. |
| `collect_dir_files` / `walk_for_inline` / `inline_wanted` | The `@dir` engine: recursive walk, sorted per directory, extension allowlist, vendor/build dirs skipped, 60k-char budget, overflow reported inline. |

### LLM command set (TUI variants)

| symbol | what it does |
|--------|--------------|
| `run_angles` | `angles <topic>` — streaming. |
| `run_questions` | `questions <topic>` — deep questions. |
| `run_contrarian` | `contrarian <claim>` — steelman the opposite. |
| `run_use_cases` | `use-cases <thing>` — concrete user scenarios. |
| `run_premortem` | `premortem <idea>` — autopsy. Since v0.10 writes a record + session step, prepends `resolve()` memory, and emits `VERDICT:` plus the structured sections. |
| `run_check` | `check <decision>` — v0.9 pre-existence check for an engineering decision; v0.10 gives it the same store write and memory as premortem. |
| `run_spec` | `spec <idea>` — agent-ready spec; v0.10 adds the store write, memory, and the CLI's `Assumptions / Failure conditions / Risk budget` sections. |
| `run_postmortem` | `postmortem <idea>` — the review + decision-log entry; v0.10 adds the store write and memory. |
| `with_memory` | Prepend `resolve()` context to a TUI verdict prompt — the TUI's half of the shared decision memory (v0.10 / D-031). |
| `run_pros` | `pros <idea>` — genuine strengths. |
| `run_cons` | `cons <idea>` — genuine weaknesses. |
| `run_risks` | `risks <idea>` — failure modes. |
| `run_steps` | `steps <goal>` — actionable plan. |
| `run_examples` | `examples <concept>` — real-world instances. |
| `run_explain` | `explain <file>` — file walkthrough. |
| `run_summarize` | `summarize <file>` — short overview. |
| `run_freeform` | Anything not matching a command — passed through. |
| `verify_and_format` | Shared post-processing for run_X: empty-detection + `── kind: arg ──` header. |
| `enrich_error` | Classify an error string and append a "Try:" line so the user always has a next step. |
| `build_context` | Pull the last N user/assistant pairs out of history → `Vec<Message>` for context. |

### Rendering

| symbol | what it does |
|--------|--------------|
| `render` | Draws the two-row live strip: ASCII `> ` prompt + dim status row. The input TEXT is printed natively after draw — ratatui never renders wide chars on the live path. |
| `Assumption` / `load_registry` / `save_registry` / `register_assumptions` / `apply_status_updates` / `verify_assumption` / `assumptions_path` | v0.7 assumption lifecycle: registry at `.naysay/assumptions.json`, claims normalized (lowercase + whitespace collapse), statuses UNKNOWN/VALID/QUESTIONED/INVALIDATED. Postmortems flip via `ASSUMPTION VALID|INVALIDATED:` lines; `decisions verify` flips manually. |
| `memory_context` / `memory_context_block` / `assumption_risk_lines` | The DECISION MEMORY block injected into premortem/spec prompts: top-3 relevant prior verdicts + tracked-assumption risk lines + MEMORY RULES (a prior DON'T BUILD must be justified or repeated). |
| `parent_assumption_context` | The parent premortem's assumptions formatted as the postmortem's status-update checklist. |
| `status_text` | The dim status line text (busy/idle/typing), extracted pure for testing. |
| `flush_pending` | Two-phase print: ratatui `insert_before` reserves blank rows above the viewport (owns scrolling), then each pre-wrapped row prints as ONE contiguous crossterm `Print` with span colors — wide chars render natively, no follower-space gaps. In-flight streaming entry flushes on its `Result` event. |
| `line_height` | Estimated wrapped row count of a Line at a given width (display_width-based, so CJK wraps on the same accounting terminals use). Drives insert heights. |
| `entry_to_lines` | One `HistoryEntry` → `Vec<Line<'_>>` for the scrollback transcript. User turns as `> cmd`, AI turns verbatim with verdict lines in red, errors prefixed `!`, info dim. |
| `apply_event` | Apply a `TuiEvent` to state. `Delta` → append to the streaming entry. `Result` → finalize, set elapsed, clear busy. |

### Export + sound

| symbol | what it does |
|--------|--------------|
| `export_conversation` | Ctrl+S → write a markdown transcript to cwd (`naysay-<epoch>.md`). |
| `play_sound` | Win32 `Beep` for submit / success / error. Off by default. No-op on non-Windows. |
| `play_background_music` | Looping bassline (`--music` flag). No-op on non-Windows. |

---

## `src/store.rs` (≈ 2070 lines)

The decision store, the assumption registry and the decision sessions. All
deterministic — no LLM calls (D-023) — and cwd-local plain JSON (D-021):
`.naysay/decisions/`, `.naysay/assumptions.json`, `.naysay/sessions/`.

### Records

| symbol | what it does |
|--------|--------------|
| `DecisionRecord` | One saved decision: id, kind, ts, idea, parent, body, the extracted sections, confidence, verdict, outcome, `schema_version` (absent = legacy). |
| `decisions_dir` / `make_decision_id` | `.naysay/decisions/`; 12 hex chars from wall-clock nanos, retried on collision. |
| `save_decision_to` / `save_decision` / `save_verdict` | Write one record; `save_verdict` also records the session step (D-031). `parent` is normalized through `bare_id`. |
| `bare_id` | Both printed id forms (`hex` and `kind-hex`) resolve to the bare hex, so links and calibration pair up either way. |
| `write_atomic` / `SCHEMA_VERSION` | Every store write goes through temp-file + rename; `SCHEMA_VERSION` is stamped on new records (D-044). |
| `read_record_by_id` / `load_all_records` | Lookup by either id form; load the whole store sorted by time. Unreadable records are counted and reported, never silently dropped. |
| `run_d_by_id` / `run_d_unknowns` / `run_d_link` | `decisions show` / `unknowns` / `link`. |

### Extraction (substring scans, never validated)

| symbol | what it does |
|--------|--------------|
| `heading_key` / `heading_matches` | Normalize a heading (`#`, `*`, `_`, `:` stripped) and match it plus the Chinese aliases. |
| `extract_section` | The bullet/numbered list under a heading; blank lines belong to the section, which is the shape models actually write. |
| `extract_confidence` / `confidence_number` | 0..=100 from the CONFIDENCE line or the next non-empty line; fractions scale. |
| `extract_verdict` / `extract_outcome` | The `VERDICT:` / `OUTCOME:` lines. |
| `is_cjk` / `tokenize` / `relevance_score` | CJK bigrams plus word tokens; overlap coefficient. |
| `classify_verdict_outcome` | verdict × outcome → held / wrong / overridden / unknown. |
| `run_calibration` / `run_d_relevant` | `calibration` pairs linked records; `decisions relevant` scores the store. |

### Assumption registry (v0.7)

| symbol | what it does |
|--------|--------------|
| `Assumption` | Claim (normalized key) + display + UNKNOWN/VALID/QUESTIONED/INVALIDATED + provenance. |
| `assumptions_path` / `load_registry` / `save_registry` | `.naysay/assumptions.json`, rewritten whole on change. |
| `normalize_claim` | Lowercase, collapse whitespace, strip trailing punctuation — the matching key. |
| `register_assumptions` / `apply_status_updates` | Register or restate claims; flip statuses from `ASSUMPTION VALID|INVALIDATED:` lines. |
| `assumption_risk_lines` / `parent_assumption_context` | The risk lines for prompt injection and the postmortem's status-update checklist. |
| `verify_assumption` / `run_d_assumptions` / `run_d_verify` | `decisions assumptions` / `verify`. |

### Decision sessions (v0.8)

| symbol | what it does |
|--------|--------------|
| `Op` / `SessionStep` / `DecisionSession` | The operation enum, one recorded step (input, full output, digest, parent_seq, saved_ref), and the session around one root idea. |
| `output_digest_of` | First 2 non-empty lines, capped at 240 chars — the currency of context assembly. |
| `save_decision_session` / `load_decision_session` / `list_decision_sessions` | `.naysay/sessions/ds-<epoch>.json`. |
| `current_session_pointer` / `load_current_session` / `save_current_session` / `set_current_session` / `clear_current_session` | The `.naysay/session-current` pointer. |
| `current_project_root` / `session_matches_project` | Sessions record the cwd they belong to; a foreign session is not injected (legacy sessions are grandfathered) — D-044. |
| `assemble_session_block` / `record_session_step` | Build the context block for one op; append a step (auto-create is the caller's choice, D-037). |
| `run_session_start` / `list` / `show` / `resume` / `close` / `run_context_manifest` | The `session` and `context` subcommands. |

### Context resolver (v0.8)

| symbol | what it does |
|--------|--------------|
| `SelectedContext` | What one operation will see, by source: the assembled text plus provenance counts and warnings. |
| `resolve` | The single place that decides context per op — session exploration (project-checked), top-N historical verdicts, assumption risks, MEMORY RULES. |

---

## `src/workspace.rs` (≈ 720 lines)

The fullscreen three-pane workspace (D-035): exploration and the store left,
transcript centre, current decision's verdict and assumptions right. Owns its
own alternate-screen lifecycle; all conversation state and command dispatch
live in `tui.rs`.

| symbol | what it does |
|--------|--------------|
| `run` | Enter alternate screen + raw mode, install a panic hook that restores the terminal, run the event loop, restore on exit. |
| `event_loop` | Drain `TuiEvent`s (a finished call also reloads the store snapshots), draw, poll keys, tick the spinner. |
| `handle_workspace_key` | On top of `tui::handle_key`: transcript scrolling (↑↓ / PgUp / PgDn, plus Home/End when the input is empty), Ctrl+F to focus the store filter, and the filter's own editing keys. |
| `draw` | Three-column layout; the centre column is transcript / input row / status row. |
| `draw_transcript` | In-memory transcript: `entry_to_lines` + `wrap_entry_lines` per frame, scrolled from the tail; cursor placed by display width so CJK lands right. |
| `draw_exploration` | Store filter, active session steps (newest 8), and the filtered store list (newest 20) with the shown record marked. |
| `draw_decision` | The shown record: kind/id/age, idea, verdict (red for DON'T BUILD), confidence, assumptions with registry lifecycle marks, linked parent/child records. |
| `View` | Workspace-local view state: scroll offset, filter text + cursor + focus, and the session / record / registry snapshots. `reload` re-reads all three; `current_record` picks the session's last saved decision; `filtered` applies the term filter. |
| `clip` / `record_matches` / `status_mark` / `age_days` / `prefix_width` | Pure helpers (unit-tested): width-aware truncation, filter matching, assumption lifecycle marks, age in days, filter-cursor column. |

Tests (3): `clip_*`, `store_filter_*`, and a `TestBackend` render test that
asserts all three panes draw.

---

## `src/text.rs` (≈ 325 lines)

Pure text layout, extracted from `tui.rs` for the workspace work (D-035 M2).
No terminal state, no I/O — every function here is unit-tested.

| symbol | what it does |
|--------|--------------|
| `char_width` | Display width of one char: 2 for CJK / Hangul / fullwidth ranges, else 1 — the accounting terminals use for cursor placement and wrapping. |
| `display_width` | Sum of `char_width` over a string. Canonical measure; production callers use `char_width` directly. |
| `byte_index_of_char` | Char index → byte offset (cursor positions are char-based because CJK chars are one cursor step each). |
| `byte_prefix` | Largest char-boundary-safe prefix ≤ N bytes — `&s[..n]` panics on mixed ASCII/CJK input. |
| `input_window` | Visible window of the input around the cursor, plus the display width of the text before it, so the cursor lands correctly. Pins to the cursor on overflow. |
| `flatten_line` / `row_from` | Internal wrap plumbing: `Line` → (style, char) stream and back into one owned row, merging adjacent same-style chars. |
| `wrap_line_to_width` | Word-aware wrap of one line into rows that each fit `width` columns, styles preserved; unbreakable runs hard-split. |
| `wrap_entry_lines` | Wrap every logical line of an entry; always returns at least one row. |

Tests (8): `width_*` (3) and `wrap_*` (5).

---

## Where to start reading

If you've never seen the code:

1. `main.rs` → `SYSTEM_PROMPT` (the whole personality lives there).
2. `main.rs` → `Config` (how a request actually leaves the machine).
3. `main.rs` → `drain_sse_events` + its 9 tests (the only genuinely subtle
   function in the codebase).
4. `tui.rs` → `run` + `submit_line` (the architecture).
5. `tui.rs` → `run_premortem` and `run_spec` (the brand-bearing commands).
6. `workspace.rs` → `draw` + `draw_decision` (what the user actually sees).

If you want to change something:

- *Brand voice?* `SYSTEM_PROMPT` and the per-command `DEFAULT` consts.
- *New command?* Add a `run_X` here + a `Command::X { ... }` in `Cli` + a
  match arm in `submit_line` + a match arm in `dispatch_repl`. ~30 lines
  total.
- *New provider?* Add a commented example to `CONFIG_TEMPLATE`. Zero code.
- *Different storage?* `data_dir` and friends — single fn.