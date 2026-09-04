# Changelog

All notable changes to `naysay` are documented here. Versions follow
[Semantic Versioning](https://semver.org/).

`naysay` was previously distributed as `pair` (v0.1 → v1.3, 2026-08-24 →
2026-08-25). The 1.3-era pair code is the foundation naysay v0.1 is
rebuilt on; the rename + re-positioning is large enough that the version
counter resets. Historical pair entries are preserved below for lineage.

---

## naysay v0.1.0 — 2026-09-04

First naysay release. Built on pair v1.3.

### Changed

- **Inline transcript UI** — the full-screen three-pane TUI (borders,
  history pane, status bar, alternate screen) is gone. The conversation
  is now a transcript printed into the terminal's own scrollback; the
  only live region is a two-row strip (`>` input + dim status line).
  Quitting leaves the transcript readable in place, and the terminal's
  native PageUp/scrollback works on it. Streaming deltas are no longer
  rendered live — the response arrives as a finished document, with a
  spinner + live character count carrying the liveness in the status
  line. Scrollback state survives `/clear` (it belongs to the terminal);
  only the model's context is wiped.
- **Renamed binary, package, keyring service, data directory** from
  `pair` to `naysay`. The legacy `pair` keyring entry and
  `MINIMAX_API_KEY` env var are still read for compatibility, so
  existing installs continue to work without re-`key set`.
- **Repositioned for the agent era.** New system prompt makes
  "interrogate before committing" the default posture. Pair's "thinking
  partner" framing invited direct competition with Claude Code / ZCode;
  naysay occupies the upstream step they leave empty.
- **Configurable provider** via `<data_dir>/naysay.toml`. MiniMax stays
  the default; the file ships commented examples for OpenAI, DeepSeek,
  and a local Ollama server. `NAYSAY_CHAT_URL` / `NAYSAY_MODEL` env vars
  override the file (CI escape hatch).
- **`build` command removed.** Replaced by `premortem` (was this idea
  worth building?) and `spec` (how will the agent execute it?). See
  `DECISIONS.md` D-002.
- **TLS backend: rustls → native-tls.** A mingw toolchain update broke
  `ring`'s C build (rustls's crypto backend) and could not be repaired
  in-tree; on Windows, native-tls means Schannel, which is equally
  capable for HTTPS chat calls and shrinks the binary ~3 MB. See
  `DECISIONS.md` D-012.

### Added

- **`--continue` + `/resume [file]`** — session resume. `naysay --continue`
  launches the TUI with the most recent session's turns replayed into the
  conversation (the model picks them up as normal context via the usual
  3-turn window); `/resume` does the same mid-session, with an optional
  file argument resolved like `sessions show`. New turns append to the
  resumed file, so a continued session stays in one piece. Input recall
  (Ctrl+Up/Down) is seeded from the resumed turns.
- **REPL conversation memory** — the plain REPL now remembers the last 3
  turns (configurable `/context N` 0..=10, `/clear` to wipe), so
  follow-ups like "what about X?" work outside the TUI too. The six
  command functions take `history: &[Message]` and return the response
  text; the REPL logs both sides of every exchange and replays a session
  on `--continue`. An unrecognized command still errors (no freeform in
  scripted mode — see DECISIONS.md D-015).
- **`postmortem <idea> [notes]`** — the project is over; the review plus
  a self-contained decision-log entry to paste into DECISIONS.md. In the
  plain REPL, `postmortem <idea> -- what happened` passes context.
  Closes the seed → premortem → spec → postmortem loop.
- **Input history recall** — `Ctrl+↑` / `Ctrl+↓` walk previously
  submitted inputs (plain `↑`/`↓` still scroll the history pane); capped
  at 100 entries; typing cancels the recall.
- **Idle command hints** — the status line advertises the verdict family
  whenever the input is empty, so the killer commands are discoverable
  without opening `help`.
- **TUI session logging** — the transcript UI now logs both user and assistant
  turns in the same JSONL format the plain REPL uses (previously the TUI
  logged nothing, so `naysay sessions` missed most usage). This is also
  the foundation for a future `--continue` session resume.
- **Chinese verdict highlighting** — `is_verdict_line` matches
  判决 / 结论 / 决定 in addition to the English keywords, so verdict
  lines in Chinese replies light up red.
- **Token meter** — the `usage` object in LLM responses (non-streaming
  and streaming final chunk) is now parsed and displayed: CLI/REPL print
  one stderr note per call, the interactive status line shows
  `ready (1.2s · 812 tok)`. Absent usage degrades silently.
- **Retry with backoff** — 429 and 5xx responses are retried up to 2
  times (1s, 2s exponential backoff) before surfacing. Retries are
  silent while the interactive UI owns the terminal. Connect timeout
  added (10s) to bound the hung-connection worst case.
- **`@dir` directory inlining** — `@./src/` inlines every text file
  under a directory (extension allowlist, vendor/build dirs skipped,
  60k-char total budget, overflow reported inline).
- **`doctor` config validation** — new first check: strict TOML parse
  of `naysay.toml` plus field validation (URL scheme, model id,
  env-var name), with fix-or-delete hints.
- **`premortem <idea>`** — assume the idea died in six months, write the
  autopsy (cause / ranked killers / scope autopsy / surviving version /
  verdict). The killer demo: this command on `FlowForge` is the README's
  opening screen.
- **`spec <idea>`** — produces an artifact designed to be handed to a
  coding agent. Sections: goal / non-goals / success criteria /
  constraints / milestones / open questions. Becomes the `agent's input`
  half of the new pipeline.
- **`Config` type + `naysay.toml`** — endpoint, model, env-var name.
  `OnceLock` singleton. Bad TOML falls back to defaults (same contract
  as `prompts.toml`).
- **`endpoint_host`** helper for the boot sequence and doctor output.
- **Unit tests** — `Config::parse` (3), `endpoint_host` (2), language
  detection (4), display width (3), verdict matching (5), context
  language hints (2), session-record parsing (3), newest-session
  selection (2), session-arg resolution (1), REPL context/record (3),
  usage parsing (5), retry/backoff (2), config validation (1),
  @dir collection (3). Total: 56, up from pair's 16.
- **`AGENTS.md`** — rules for AI agents that help maintain naysay. The
  agent advises; the user decides.
- **`DECISIONS.md`** — design log. Every non-obvious choice recorded
  before the code lands.
- **`CODEMAP.md`** — function-by-function map of `main.rs` + `tui.rs`.

### Notes

- Binary is still single-file, no runtime deps, ~8.6 MB (native-tls).
- ~3800 lines, up from pair v1.3's 3381: `build` (230 lines) was removed
  but `premortem`/`spec`/`postmortem`, provider config, and the language
  hint layer were added.

---

## pair v1.3 — 2026-08-25 *(predecessor)*

Quality-of-life polish on top of v1.2. Six independent improvements.

### Added

- **Streaming LLM responses (SSE)** — the TUI now renders each token as
  it arrives from the server instead of waiting for the whole response.
  Adds `call_llm_stream()` next to `call_llm()`. Wire protocol is SSE;
  implementation uses `reqwest`'s `bytes_stream()` plus a hand-rolled
  SSE parser.
- **`Ctrl+S` — export conversation** — writes the current history to
  `pair-<unix-timestamp>.md` in the cwd.
- **`r` — regenerate last command** — when the input box is empty,
  pressing `r` re-dispatches the most recently submitted command.
- **Tab completion** — first Tab extends the typed prefix to the longest
  common prefix of matching commands; subsequent Tabs cycle through
  candidates.
- **Externalized prompts (`prompts.toml`)** — every command's prompt
  template can be overridden without recompiling. Written to
  `<data_dir>/prompts.toml` on first run.
- **Configurable context window (`/context N`)** — `/context` (bare)
  shows current value; `/context N` sets it (range 0..=10).
- **`/clear`** — wipe the visible history. Boot banner replayed.
- **Inline `@file` references** — type `@./path/to/file.rs` anywhere in
  the input and the file's contents are substituted into the prompt.
- **`/model [name]`** — show or switch the LLM used for subsequent
  calls.
- **Match the user's language** — system prompt rule: reply in
  whichever language the user types in.
- **SSE parser tolerates missing-delta and malformed chunks** — chunks
  with no `delta` field (the typical finish_reason tail) are silently
  skipped. Malformed JSON is also skipped instead of being fatal.
- **SSE parser unit tests** — extracted `drain_sse_events(buffer,
  on_delta)`; 7 tests covering single event, multiple events, `[DONE]`,
  empty delta, partial event spanning chunks, extras lines, invalid
  JSON. Test count 16 → 23.
- **Actionable error messages** — every propagated error gets a `Try: …`
  line appended, classified by content.

## pair v1.2 — 2026-08-24 *(predecessor)*

TUI polish. Hacker aesthetic. Granular commands. Freeform mode.

## pair v1.1 — 2026-08-24 *(predecessor)*

TUI polish + 8-bit sound effects.

## pair v1.0 — 2026-08-24 *(predecessor)*

First public release. One binary, no runtime deps, ~9 MB.

## pair v0.1 → v0.11 — 2026-08-24 *(predecessor)*

Original feature progression: `seed` (v0.1) → `drill` (v0.2) → REPL
(v0.3) → `build` (v0.4) → `explain` (v0.5) → `--save`/`--json`
(v0.6) → keyring (v0.7) → sessions (v0.8) → TUI (v0.9) → unit tests
(v0.10) → doctor + `--version` (v0.11). See `git log` for pair-era
commits.