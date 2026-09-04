# CODEMAP — what every function does

Companion to `DECISIONS.md` (which answers "why?"). This answers
"what?".

Codebase at v0.1.0: ~3400 lines across `src/main.rs` + `src/tui.rs`.
If you can read both files end-to-end with this map in hand, you own the
tool. If you can't, that's the part to study next.

---

## `src/main.rs` (≈ 1750 lines)

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
| `launch_interactive` | First-launch experience. If no key (env or keyring), prints an ASCII setup box, prompts for one, sets the env var for this process, verifies it loaded, then drops into the TUI. Goal: double-click → TUI without jargon. |
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
| `Config` | chat_url + model + api_key_env. Loaded from `<data_dir>/naysay.toml`. `OnceLock` for process-wide singleton. |
| `Config::parse` | Parses TOML. Malformed → defaults. Missing fields → defaults. |
| `Config::load` | Reads file, writes template on first run, applies env-var overrides, returns. |
| `config()` | Public accessor. First call initializes. |
| `endpoint_host` | Display helper. `https://api.x.com/v1/chat` → `api.x.com`. |
| `CONFIG_TEMPLATE` | What `naysay.toml` looks like on first run — all the provider examples commented out for discoverability. |
| `DEFAULT_CHAT_URL` / `DEFAULT_MODEL` / `DEFAULT_API_KEY_ENV` | Fallback values. MiniMax defaults. |
| `KEYRING_SERVICE` / `KEYRING_USER` | `naysay` / `api-key`. Independent of provider. |

### Prompt externalization

| symbol | what it does |
|--------|--------------|
| `Prompts` | Optional overrides for every command's prompt template. One field per command. |
| `PromptsFile` | TOML wrapper — the file has a `[prompts]` table. |
| `Prompts::load` | Reads `<data_dir>/prompts.toml`, writes template on first run, falls back to defaults on any error. |
| `Prompts::get` | Lookup by key, returns user override or default. |
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
| `spec` | `spec <idea>` — produce a spec the agent can't misinterpret: goal / non-goals / success criteria / constraints / milestones / open questions. |
| `postmortem` | `postmortem <idea> [notes]` — the project is over; write the review (what happened, predicted-vs-actual, decisive moment, cost accounting) plus a self-contained decision-log entry. |
| `explain` | `explain <file>` — read the file, send to LLM with "walk through this file" framing. Truncates to 24k chars. |

### Plain REPL

| symbol | what it does |
|--------|--------------|
| `ReplState` | REPL conversation memory: `history` (full, user turns carry the language hint from birth), `context_turns` (0..=10), `session_path`. `context()` returns the last N pairs; `record()` appends an exchange to memory + logs the assistant side. |
| `repl` | Stdin reader. Opens a session log, replays a resumed session when given one (`--continue`), prints the `naysay>` prompt, dispatches each line via `dispatch_repl`, logs user input. |
| `dispatch_repl` | Naive `command + rest` split over a `&mut ReplState`. Recognized: `help` / `quit` / `seed` / `drill` / `premortem` / `spec` / `postmortem` / `explain` / `/context` / `/clear` / `key` / `sessions`. LLM-backed commands send the context window and call `record()` afterwards. Unknown → error (no freeform in scripted mode). |

### LLM HTTP

| symbol | what it does |
|--------|--------------|
| `call_llm` | Wrapper. Pins to `config().model`. System prompt + history + user prompt → POST → parse → content. |
| `call_llm_with_model` | Inner. Takes model name explicitly so the TUI can pass `/model`-chosen values. POSTs via `post_chat_with_retry`, stores usage. |
| `post_chat_with_retry` | Resilient POST: 429/5xx retried up to 2x (1s/2s backoff), body never consumed before the retry decision. Silent while the TUI owns the terminal (`TUI_ACTIVE` flag); 10s connect timeout. |
| `is_retryable_status` / `backoff_secs` | Retry policy, pure functions (unit-tested). |
| `call_llm_stream` | Streaming twin. Same request shape but `stream: true`, then `bytes_stream` → `drain_sse_events` per chunk. |
| `drain_sse_events` | Hand-rolled SSE parser. Drains complete events from buffer (keeps partial trailing event). Emits deltas. Tolerates missing-delta, malformed JSON, finish_reason chunks. **Has 9 unit tests.** |

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

## `src/tui.rs` (≈ 1640 lines)

### Entry + lifecycle

| symbol | what it does |
|--------|--------------|
| `run` | UI entry. Loads config, sanity-checks API key, sets up Windows console ctrl handler (Ctrl+C graceful exit), pushes boot-sequence history, replays a resumed session when given one (`--continue`: turns into history, user turns into input recall, session log reused), enables raw mode with an **inline viewport** (no alternate screen), runs the main loop — flush finished entries to scrollback, render the two-row live strip, restore terminal on exit (transcript stays in scrollback). |
| `debug_log` | Append-only debug log to `<data_dir>/session.log` (best-effort, never panics). Tagged at every phase boundary so a "TUI flashed and exited" is diagnosable. |
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
| `handle_key` | All key handling. Plain chars → input. Backspace / Enter / Esc / Tab / Ctrl+S / Ctrl+C / `r`. While busy: only quit keys (the terminal owns scrolling). |
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
| `run_premortem` | `premortem <idea>` — autopsy. |
| `run_spec` | `spec <idea>` — agent-ready spec. |
| `run_postmortem` | `postmortem <idea>` — the review + decision-log entry. |
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
| `render` | Draws the two-row live strip only: `> ` input row + dim status row (spinner + char count while busy, command cheat-sheet when idle). Places the cursor at the end of typed input (display-width aware). History is never re-rendered here — it lives in scrollback. |
| `flush_pending` | Prints every finished history entry to the terminal's scrollback via `insert_before`, one batch per frame. The in-flight streaming entry stays unflushed until its `Result` event finalizes it. |
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

## Where to start reading

If you've never seen the code:

1. `main.rs` → `SYSTEM_PROMPT` (the whole personality lives there).
2. `main.rs` → `Config` (how a request actually leaves the machine).
3. `main.rs` → `drain_sse_events` + its 9 tests (the only genuinely subtle
   function in the codebase).
4. `tui.rs` → `run` + `submit_line` (the architecture).
5. `tui.rs` → `run_premortem` and `run_spec` (the brand-bearing commands).
6. `tui.rs` → `render` (what the user actually sees).

If you want to change something:

- *Brand voice?* `SYSTEM_PROMPT` and the per-command `DEFAULT` consts.
- *New command?* Add a `run_X` here + a `Command::X { ... }` in `Cli` + a
  match arm in `submit_line` + a match arm in `dispatch_repl`. ~30 lines
  total.
- *New provider?* Add a commented example to `CONFIG_TEMPLATE`. Zero code.
- *Different storage?* `data_dir` and friends — single fn.