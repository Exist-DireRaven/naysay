# Changelog

All notable changes to `naysay` are documented here. Versions follow
[Semantic Versioning](https://semver.org/).

[English](#english) · [中文](#中文)

`naysay` was previously distributed as `pair` (v0.1 → v1.3, 2026-08-24 →
2026-08-25). The 1.3-era pair code is the foundation naysay v0.1 is
rebuilt on; the rename + re-positioning is large enough that the version
counter resets. Historical pair entries are preserved below for lineage.

---

<a id="english"></a>

## English

### naysay v0.3.1 — 2026-09-05

#### Fixed

- **Stale version banner.** The first-run setup box kept saying
  `naysay v0.1` through four releases because the version was
  hand-written in three places. Every banner now derives from
  `CARGO_PKG_VERSION` at compile time (`crate::VERSION`), and the
  setup box computes its padding so future version strings of any
  length stay frame-aligned. Found by a user asking exactly the right
  question: "why does 0.3 still say v0.1?"

---

### naysay v0.3.0 — 2026-09-05

Decision memory. The store is a directory of JSON files under
`.naysay/decisions/` in the working directory; three pure-read query
commands; best-effort auto-save in the three verdict commands. No LLM
calls needed for any query.

#### Added

- **`.naysay/decisions/` store** — every `premortem` / `spec` /
  `postmortem` call auto-saves a record: 12-hex id, timestamp, idea,
  full body, and the structured sections extracted by substring scan
  (assumptions, evidence, unknowns, failure conditions, confidence).
  Save failures print to stderr and never break the command.
- **`naysay decisions by-id <id>`** — print one record. Accepts the
  bare id or the full file stem.
- **`naysay decisions link <id>`** — walk the parent chain and print
  the decision lineage as a tree.
- **`naysay decisions unknowns`** — the UNKNOWNS inventory: every
  "what we don't know" bullet across all stored premortems, oldest
  first. Works on day one with zero LLM calls and no API key.
- **REPL aliases** — `d-by-id`, `d-link`, `d-unknowns` inside the
  plain REPL.
- **5 unit tests** — id uniqueness/format, section extraction
  (bullets + numbered lists + missing headings), confidence parsing
  (fraction and percent), save/load roundtrip with parent linking.
  Total: 61.

#### Notes

- The store is cwd-local by design (D-021): the user decides which
  directory is a project. No git hooks, no sync, no server.
- See `examples/003-decision-memory.md` for the self-review.

---

### naysay v0.2.0 — 2026-09-05

Structured output, zero new surface. Same six commands, same six
prompts, same CLI — only the prompt templates grow.

### Changed

- **`premortem` now ends with a structured section** in addition to
  the existing autopsy:
  - `ASSUMPTIONS` — 3–5 things the build depends on being true, each
    specific enough to be wrong.
  - `EVIDENCE` — for each assumption, what would prove it true and
    what would prove it false. "None yet" is an acceptable answer;
    inventing data is not.
  - `UNKNOWNS` — 2–4 things that would flip the verdict if they
    turned out a certain way.
  - `CONFIDENCE` — a 0..1 number for the verdict itself. "0.5 means
    you would change your mind for a free coffee. 0.9 means you
    would bet money on it."
- **`spec` now includes** `Assumptions`, `Failure Conditions`, and
  `Risk Budget` sections in addition to the existing ones. Failure
  conditions are deal-breakers, not bug lists ("latency > 2s" is
  one; "the user dislikes the icon" is not).
- **`postmortem` now ends with a `CALIBRATION` section** — the
  difference between the original premortem verdict and the actual
  outcome. This is the single most useful sentence in the whole
  document: it teaches whether the premortem process itself was
  calibrated or not.

### Notes

- 56/56 tests pass, fmt clean, clippy `-D warnings` clean.
- Binary is still single-file, no runtime deps, ~9 MB.
- No new commands, no new flags, no new dependencies, no new types.
  See DECISIONS.md D-020 for why this is the point.

---

### naysay v0.1.0 — 2026-09-04

First naysay release. Built on pair v1.3.

#### Changed

- **Renamed binary, package, keyring service, data directory** from
  `pair` to `naysay`. The legacy `pair` keyring entry and
  `MINIMAX_API_KEY` env var are still read for compatibility, so
  existing installs continue to work without re-`key set`.
- **Repositioned for the agent era.** New system prompt makes
  "interrogate before committing" the default posture. Pair's
  "thinking partner" framing invited direct competition with Claude
  Code / ZCode; naysay occupies the upstream step they leave empty.
- **Configurable provider** via `<data_dir>/naysay.toml`. MiniMax
  stays the default; the file ships commented examples for OpenAI,
  DeepSeek, and a local Ollama server. `NAYSAY_CHAT_URL` /
  `NAYSAY_MODEL` env vars override the file (CI escape hatch).
- **`build` command removed.** Replaced by `premortem` (was this
  idea worth building?) and `spec` (how will the agent execute
  it?). See `DECISIONS.md` D-002.
- **TLS backend: rustls → native-tls.** A mingw toolchain update
  broke `ring`'s C build (rustls's crypto backend) and could not be
  repaired in-tree; on Windows, native-tls means Schannel, which
  is equally capable for HTTPS chat calls and shrinks the binary
  ~3 MB. See `DECISIONS.md` D-012.
- **Inline transcript UI** — the full-screen three-pane TUI
  (borders, history pane, status bar, alternate screen) is gone.
  The conversation is now a transcript printed into the terminal's
  own scrollback; the only live region is a two-row strip (`>`
  input + dim status line). Quitting leaves the transcript
  readable in place, and the terminal's native PageUp/scrollback
  works on it. Streaming deltas are no longer rendered live — the
  response arrives as a finished document, with a spinner + live
  character count carrying the liveness in the status line.
  Scrollback state survives `/clear` (it belongs to the terminal);
  only the model's context is wiped.

#### Added

- **`postmortem <idea> [notes]`** — the project is over; the review
  plus a self-contained decision-log entry to paste into
  DECISIONS.md. In the plain REPL, `postmortem <idea> -- what
  happened` passes context. Closes the seed → premortem → spec →
  postmortem loop.
- **Input history recall** — `Ctrl+↑` / `Ctrl+↓` walk previously
  submitted inputs (plain `↑`/`↓` still scroll the history pane);
  capped at 100 entries; typing cancels the recall.
- **Idle command hints** — the status line advertises the verdict
  family whenever the input is empty, so the killer commands are
  discoverable without opening `help`.
- **Transcript UI session logging** — the transcript UI now logs
  both user and assistant turns in the same JSONL format the
  plain REPL uses (previously the transcript UI logged nothing,
  so `naysay sessions` missed most usage). This is also the
  foundation for a future `--continue` session resume.
- **Chinese verdict highlighting** — `is_verdict_line` matches
  判决 / 结论 / 决定 in addition to the English keywords, so
  verdict lines in Chinese replies light up red.
- **`--continue` + `/resume [file]`** — session resume.
  `naysay --continue` launches the TUI with the most recent
  session's turns replayed into the conversation (the model picks
  them up as normal context via the usual 3-turn window);
  `/resume` does the same mid-session, with an optional file
  argument resolved like `sessions show`. New turns append to the
  resumed file, so a continued session stays in one piece. Input
  recall (Ctrl+Up/Down) is seeded from the resumed turns.
- **REPL conversation memory** — the plain REPL now remembers the
  last 3 turns (configurable `/context N` 0..=10, `/clear` to
  wipe), so follow-ups like "what about X?" work outside the
  TUI too. The six command functions take `history: &[Message]`
  and return the response text; the REPL logs both sides of every
  exchange and replays a session on `--continue`. An unrecognized
  command still errors (no freeform in scripted mode — see
  DECISIONS.md D-015).
- **`premortem <idea>`** — assume the idea died in six months,
  write the autopsy (cause / ranked killers / scope autopsy /
  surviving version / verdict). The killer demo: this command on
  `FlowForge` is the README's opening screen.
- **`spec <idea>`** — produces an artifact designed to be handed
  to a coding agent. Sections: goal / non-goals / success
  criteria / constraints / milestones / open questions. Becomes
  the `agent's input` half of the new pipeline.
- **`Config` type + `naysay.toml`** — endpoint, model, env-var
  name. `OnceLock` singleton. Bad TOML falls back to defaults
  (same contract as `prompts.toml`).
- **`endpoint_host`** helper for the boot sequence and doctor
  output.
- **Token meter** — the `usage` object in LLM responses
  (non-streaming and streaming final chunk) is now parsed and
  displayed: CLI/REPL print one stderr note per call, the
  interactive status line shows `ready (1.2s · 812 tok)`. Absent
  usage degrades silently.
- **Retry with backoff** — 429 and 5xx responses are retried up
  to 2 times (1s, 2s exponential backoff) before surfacing.
  Retries are silent while the interactive UI owns the terminal.
  Connect timeout added (10s) to bound the hung-connection worst
  case.
- **`@dir` directory inlining** — `@./src/` inlines every text
  file under a directory (extension allowlist, vendor/build dirs
  skipped, 60k-char total budget, overflow reported inline).
- **`doctor` config validation** — new first check: strict TOML
  parse of `naysay.toml` plus field validation (URL scheme,
  model id, env-var name), with fix-or-delete hints.
- **Unit tests** — `Config::parse` (3), `endpoint_host` (2),
  language detection (4), display width (3), verdict matching (5),
  context language hints (2), session-record parsing (3),
  newest-session selection (2), session-arg resolution (1), REPL
  context/record (3), usage parsing (5), retry/backoff (2),
  config validation (1), @dir collection (3). Total: 56, up from
  pair's 16.
- **`AGENTS.md`** — rules for AI agents that help maintain naysay.
  The agent advises; the user decides.
- **`DECISIONS.md`** — design log. Every non-obvious choice recorded
  before the code lands.
- **`CODEMAP.md`** — function-by-function map of `main.rs` +
  `tui.rs`.

#### Notes

- Binary is still single-file, no runtime deps, ~9 MB
  (native-tls).
- ~3800 lines, up from pair v1.3's 3381: `build` (230 lines)
  was removed but `premortem`/`spec`/`postmortem`, provider
  config, and the language hint layer were added.

---

<a id="中文"></a>

## 中文

### naysay v0.3.1 — 2026-09-05

#### 修复

- **版本横幅过期。** 首次运行的 setup 框连着四个版本都在显示
  `naysay v0.1`——因为版本号在三处被手写死了。现在所有横幅都从
  `CARGO_PKG_VERSION` 编译期派生(`crate::VERSION`),setup 框的
  补宽动态计算,任意长度的版本号都不会破坏对齐。发现者是一位用户
  问出了最准的问题:"为什么 0.3 还显示 v0.1?"

---

### naysay v0.3.0 — 2026-09-05

决策记忆。存储 = 工作目录下 `.naysay/decisions/` 的一组 JSON 文件;
三个纯读查询命令;三个 verdict 命令自动落盘(尽力而为)。查询全程
零 LLM 调用。

#### 新增

- **`.naysay/decisions/` 存储** — 每次 `premortem` / `spec` /
  `postmortem` 自动保存一条记录:12 位 hex id、时间戳、想法、完整
  正文、按子串扫描提取的结构化段。保存失败只打 stderr,绝不破坏
  命令本身。
- **`naysay decisions by-id <id>`** — 打印一条记录。
- **`naysay decisions link <id>`** — 沿 parent 链树形打印决策谱系。
- **`naysay decisions unknowns`** — UNKNOWNS 清单:所有已存
  premortem 里"我们不知道什么"的子弹。第一天就能用,零 API key。
- **REPL 别名** — `d-by-id` / `d-link` / `d-unknowns`。
- **5 个单元测试**。总数:61。

---

### naysay v0.2.0 — 2026-09-05

结构化输出,零新表面。同样的六个命令、六个 prompt、同一个 CLI——
只是 prompt 模板长了结构化段:

- `premortem` → `ASSUMPTIONS / EVIDENCE / UNKNOWNS / CONFIDENCE`
- `spec` → `Assumptions / Failure Conditions / Risk Budget`
- `postmortem` → `CALIBRATION`

无新命令、无新 flag、无新依赖、无新类型(DECISIONS.md D-020)。

---

### naysay v0.1.0 — 2026-09-04

首个 naysay 发布(在 pair v1.3 之上重建)。完整条目见上方 English 节;
要点:

- **改名 pair → naysay**,二进制 / keyring / 数据目录全换,旧凭据兼容。
- **为 agent 时代重新定位**:"开工前先审问"成为默认姿态。
- **`premortem` / `spec` / `postmortem` 三个 verdict 命令**上线;
  `build` 移除(DECISIONS.md D-002)。
- **provider 可换**(`naysay.toml`,MiniMax 默认,OpenAI / DeepSeek /
  本地 Ollama 范例),`NAYSAY_CHAT_URL` / `NAYSAY_MODEL` 环境变量覆盖。
- **行内转录界面**替代全屏 TUI:对话进终端 scrollback,唯一活动区
  是底部两行。
- **REPL 对话记忆**(`/context N` / `/clear`)、`--continue` 会话恢复、
  `naysay decisions` 决策存储之前身:session JSONL 双向日志。
- 56 项单元测试;单二进制 ~9 MB。

---

## pair v1.3 — 2026-08-25 *(predecessor / 前身)*

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

## pair v1.2 — 2026-08-24 *(predecessor / 前身)*

TUI polish. Hacker aesthetic. Granular commands. Freeform mode.

## pair v1.1 — 2026-08-24 *(predecessor / 前身)*

TUI polish + 8-bit sound effects.

## pair v1.0 — 2026-08-24 *(predecessor / 前身)*

First public release. One binary, no runtime deps, ~9 MB.

## pair v0.1 → v0.11 — 2026-08-24 *(predecessor / 前身)*

Original feature progression: `seed` (v0.1) → `drill` (v0.2) → REPL
(v0.3) → `build` (v0.4) → `explain` (v0.5) → `--save`/`--json`
(v0.6) → keyring (v0.7) → sessions (v0.8) → TUI (v0.9) → unit tests
(v0.10) → doctor + `--version` (v0.11). See `git log` for pair-era
commits.