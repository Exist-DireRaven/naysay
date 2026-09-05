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

### naysay v0.7.0 — 2026-09-05

**naysay remembers what you decided — and asks whether it is still
true.** The archive becomes an engine: past decisions now enter the
prompt, carry lifecycle, and can overturn present plans. Shaped by the
second external review's v0.7 spec (D-023) and logged as D-025/D-026.

#### Added

- **Assumption lifecycle registry** (`.naysay/assumptions.json`) —
  every assumption a premortem/spec emits is tracked: normalized claim,
  status (UNKNOWN/VALID/QUESTIONED/INVALIDATED), first/last decision,
  optional note. Deterministic matching (normalized text), deterministic
  transitions (postmortem outcomes or explicit `decisions verify`).
- **Decision memory in prompts** — premortem and spec prompts now carry
  a `DECISION MEMORY` block: prior verdicts on similar ideas
  (deterministic retrieval, top 3) with assumption statuses, plus
  MEMORY RULES: a prior DON'T BUILD on the same idea must be either
  justified by a material change or repeated.
- **Postmortem assumption updates** — with `--parent`, the prompt lists
  the parent premortem's assumptions and instructs the model to emit
  `ASSUMPTION VALID|INVALIDATED: <claim>` lines; the registry flips
  accordingly (existing entries only — postmortems cannot invent
  assumptions).
- **`naysay decisions assumptions`** — the registry listing, with an
  explicit unverified-assumption risk warning.
- **`naysay decisions verify <claim> <STATUS>`** — manual lifecycle
  flip with optional note.
- **`decisions relevant` conflict annotation** — rows whose prior
  verdict was DON'T BUILD are flagged as repeats requiring
  justification.
- **8 unit tests** — normalize/merge, status flips (existing-only),
  memory-context relevance gating, risk lines with status/age,
  postmortem update application. Total: 85.

#### Notes

- No new commands beyond `decisions assumptions|verify`; the engine
  works through the EXISTING premortem/spec/postmortem surface (D-026).
- No vector DB, no MCP, no Web UI (D-019/D-023 rejections stand).
- LOC: main.rs ~3150, tui.rs ~2840, store.rs ~870 — all inside the
  guardrail.

---
### naysay v0.6.1 — 2026-09-05

#### Added

- **Windows executable icon** — `assets/naysay.ico` (multi-size 16-256)
  is embedded into the .exe at build time via `winresource`, so
  Explorer/Taskbar show the naysay icon. CI release builds embed it
  (MSVC runner); local GNU-toolchain builds skip gracefully when the
  resource toolchain is unavailable (the user's mingw64 gcc currently
  fails to spawn cc1 — a toolchain issue, not a naysay one).
- **Folder icon** — the repo folder ships `naysay.ico` +
  `desktop.ini` (`IconResource`) so Explorer shows the icon for the
  checked-out folder too.

---
### naysay v0.6.0 — 2026-09-05

Full line editing + native wide-char rendering. Both defects were
reported by the user within one session and share one root cause:
ratatui's cell layer mishandles wide (CJK) chars on the inline path.

#### Added

- **Full line editing** — the input is cursor-addressable: ←/→ move
  by char (CJK = one step), Home/End jump, Delete removes forward,
  and typing inserts AT the cursor. `hlp` + ← + `e` = `help`. The
  visible window pins to the cursor when the input overflows the row
  (`input_window`), and cursor placement is display-width based.
- **Native wide-char rendering** — the transcript and the input row
  bypass ratatui's cell layer for text: ratatui reserves the space
  and draws the ASCII prompt; each pre-wrapped row prints as ONE
  contiguous crossterm `Print` (colors ride on spans). The terminal
  renders wide chars itself — no per-cell MoveTo, no follower-space
  gaps (see D-024 for the root cause found in ratatui 0.29's
  `insert_before` draw path).

#### Fixed

- **CJK gaps in input and transcript** ("仿 生 机 械 臂") — root
  cause: `Buffer::set_stringn` resets the follower cell after every
  wide char, and `insert_before`'s `draw_lines` prints every cell
  without the diff skip logic, so a blank lands after each CJK char.
- **Cursor could not move inside typed text** — `hlp` could not
  become `help` without deleting `lp`.
- 2 new unit tests (cursor mid-word insertion via `handle_key`
  simulation, `input_window` overflow pinning). Total: 77.

---
### naysay v0.5.1 — 2026-09-05

#### Fixed

- **Clipped, gapped transcript (the serious one).** The inline
  transcript estimated wrapped row heights with `ceil(columns / width)`
  and inserted that many rows — but real terminals wrap at word
  boundaries, so every long line lost its tail and mis-estimates left
  blank gaps. The transcript now **pre-wraps** each logical line into
  exact physical rows (word-aware, display-width based, style
  preserving, CJK correct) before inserting: inserted row count ==
  visible row count, always. Found by a user whose premortem output
  was visibly shredded.
- **Input overflow.** Typing past the terminal width used to hide the
  end of the input; the row now shows the tail (cursor stays on it).
- **Verdict highlight survived markdown.** `**5. Verdict**` and
  `## Verdict` now light up red like the plain forms.
- **Zombie process hygiene**: a smoke-test TUI left running held the
  release binary lock; release builds now fail with a clear culprit
  instead of a mystery.

#### Added

- 7 unit tests: word-boundary wrap (incl. the exact reported
  130-char case), CJK 2-column accounting, long-word hard split,
  style preservation across rows, empty-line row count,
  input tail-scroll, verdict-under-markdown. Total: 78.

---
### naysay v0.5.0 — 2026-09-05

The archive becomes memory. One theme: past decisions start shaping
present ones. Shaped by a second external review of v0.4 (8.0/10 —
see DECISIONS.md D-023).

#### Added

- **`naysay decisions relevant "<idea>"`** — deterministic retrieval
  over the decision store: Jaccard token overlap between the query
  and each record's idea+body, ranked, top 10. No LLM, no network,
  no dependencies. Retrieval is deterministic; interpretation is
  the LLM's job (the architectural boundary suggested in review).
- **`--parent <ID>`** on premortem / spec / postmortem — decision
  revision lineage is now writable: DEC-001 → REVISIT → DEC-023.
  The `parent` field existed since v0.3; nothing wrote it until now.
- **`naysay calibration`** — the honest minimal version. premortem
  prompts now end with a structured `VERDICT: BUILD|DON'T BUILD`
  line; postmortem prompts open CALIBRATION with a structured
  `OUTCOME: BUILT|KILLED|ABANDONED|UNKNOWN` line; the command links
  premortems to child postmortems and reports held / wrong /
  overridden, with an explicit caveat while the corpus is small.
- **`src/store.rs`** — Decision leaves the CLI file (~290 lines
  extracted). First execution of the LOC guardrail (main.rs ≤ 4000,
  tui.rs ≤ 3000): split before the boundary, not after.
- **README "naysay's own decision record"** — the self-experiment is
  public: 23 logged decisions, 2 published kill cases, the lineage
  from killed predecessor to shipped tool.
- **6 unit tests** — verdict/outcome extraction (3), tokenize +
  Jaccard bounds and ordering (2), verdict-vs-outcome classification
  across all four cells (1). Total: 71.

#### Notes

- LOC: main.rs ~3100, tui.rs ~2600, store.rs ~470 — all inside the
  guardrail.
- Deliberately deferred (D-023): MODEL CONFIDENCE rename, JSON
  schema output, richer verdict taxonomy, `naysay check`
  engineering-decision mode, full calibration dashboard. Each has a
  documented re-open condition.

---

### naysay v0.4.0 — 2026-09-05

Interactive provider picker at first run. No new commands, no new
flags, no new dependencies — presets are data, not abstraction
(D-022).

#### Added

- **First-run provider picker** — the setup box now offers seven
  paths: Ollama (local, free, no key), DeepSeek, GLM (Zhipu),
  OpenAI, MiniMax, OpenRouter (also carries Claude models), and
  Custom (any OpenAI-compatible endpoint). The choice is written to
  `naysay.toml`, the key goes to the OS keyring, and the TUI
  launches against the chosen provider. Existing users with a key
  skip it entirely.
- **Claude honesty note** — the picker states plainly that
  Anthropic's API is not OpenAI-compatible and routes Claude usage
  through OpenRouter.
- **6 unit tests** — choice parsing, preset well-formedness (URL
  scheme, model, env name, key-only-for-remote rule), TOML
  roundtrip through `Config::parse_strict` + `validate`, banner
  alignment across version lengths. Total: 65.

#### Fixed

- **Stale version banner.** The first-run setup box kept saying
  `naysay v0.1` through four releases because the version was
  hand-written in three places. Every banner now derives from
  `CARGO_PKG_VERSION` at compile time (`crate::VERSION`), and the
  setup box computes its padding so future version strings of any
  length stay frame-aligned. Found by a user asking exactly the
  right question: "why does 0.3 still say v0.1?"

---

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

### naysay v0.7.0 — 2026-09-05

**naysay 记住你做过的决定——并追问它们今天还成立吗。** 档案变成
引擎：过去的决策进入 prompt、带上生命周期、能够推翻当前计划。
形状来自对 v0.4 的第二次外部审评的 v0.7 规格(D-023)，入档为
D-025/D-026。

#### 新增

- **假设生命周期注册表** (`.naysay/assumptions.json`) —
  premortem/spec 产出的每条假设都被追踪：规范化 claim、状态
  (UNKNOWN/VALID/QUESTIONED/INVALIDATED)、首次/最近决策、可选备注。
  确定性匹配(规范化文本)、确定性翻转(postmortem 结果或显式
  `decisions verify`)。
- **prompt 注入决策记忆** — premortem 和 spec 的 prompt 现在携带
  `DECISION MEMORY` 块：相似想法的历史判决(确定性检索，前 3 条)
  及假设状态，外加 MEMORY RULES：同一想法历史上的 DON'T BUILD
  必须被"什么实质变了"证明，否则重复 DON'T BUILD。
- **postmortem 假设状态更新** — 带 `--parent` 时，prompt 列出父
  premortem 的假设并指示模型输出 `ASSUMPTION VALID|INVALIDATED:
  <claim>` 行；注册表相应翻转(仅限已存在条目——postmortem 不能
  凭空发明假设)。
- **`naysay decisions assumptions`** — 注册表清单，带未验证假设的
  风险警告。
- **`naysay decisions verify <claim> <STATUS>`** — 手动生命周期
  翻转，可带备注。
- **`decisions relevant` 冲突注记** — 历史 verdict 为 DON'T BUILD
  的行被标记为"重复需要论证"。
- **8 个单元测试** — 规范化/合并、状态翻转(仅限已存在)、记忆
  上下文相关性门控、风险行状态/年龄、postmortem 更新应用。
  总数:85。

#### 备注

- 除 `decisions assumptions|verify` 外无新命令——引擎通过既有的
  premortem/spec/postmortem 表面工作(D-026)。
- 无 vector DB、无 MCP、无 Web UI(D-019/D-023 的拒绝依然成立)。
- LOC：main.rs ~3150，tui.rs ~2840，store.rs ~870——都在警戒线内。

---
### naysay v0.6.1 — 2026-09-05

#### 新增

- **Windows 可执行文件图标** — `assets/naysay.ico`(16-256 多尺寸)在
  构建期通过 `winresource` 嵌入 .exe,资源管理器/任务栏显示 naysay
  图标。CI 发布构建嵌入(MSVC runner);本地 GNU 工具链若资源工具
  不可用则优雅跳过(本机 mingw64 gcc 目前无法启动 cc1——工具链
  问题,非 naysay)。
- **文件夹图标** — 仓库自带 `naysay.ico` + `desktop.ini`
  (`IconResource`),检出后资源管理器同样显示文件夹图标。

---
### naysay v0.6.0 — 2026-09-05

完整行编辑 + 原生宽字符渲染。两个缺陷都由用户在一个会话内报告，
且共享同一根因：ratatui 的 cell 层在 inline 路径上对宽字符(CJK)
处理不当。

#### 新增

- **完整行编辑** — 输入有了可移动光标：←/→ 按字符移动(CJK 一步)，
  Home/End 跳转，Delete 向前删，打字在光标处插入。`hlp` + ← + `e`
  = `help`。输入溢出时可见窗口钉住光标(`input_window`)，光标定位
  按显示宽度。
- **原生宽字符渲染** — 转录稿和输入行对文本绕开 ratatui 的 cell
  层：ratatui 负责预留空间和画 ASCII 提示符；每条预换行行用一次
  连续的 crossterm `Print` 打印(颜色随 span)。宽字符由终端自己
  渲染——无逐 cell MoveTo、无 follower 空格(根因见 D-024:
  ratatui 0.29 `insert_before` 打印路径的缺陷)。

#### 修复

- **输入框和转录稿的中文空隙**("仿 生 机 械 臂")——根因:
  `Buffer::set_stringn` 在每个宽字符后重置 follower 单元,而
  `insert_before` 的 `draw_lines` 无差别打印每个 cell(不走 diff
  跳过逻辑),于是每个中文字后面跟一个真实的空格。
- **光标无法移进已输入文本** — `hlp` 没法不删 `lp` 变成 `help`。
- 2 个新单元测试(`handle_key` 模拟的光标中插、`input_window`
  溢出钉住)。总数:77。

---
### naysay v0.5.1 — 2026-09-05

#### 修复

- **转录稿被截断 + 出现空隙(严重)。** 行内转录用
  `ceil(列数 / 宽度)` 估算换行行数再插入——但真实终端按词边界
  换行，于是每行长句丢尾巴，估算偏差留下空行。现在转录稿
  **预换行**：按显示宽度做词感知切分(保留样式、CJK 按 2 列)，
  切成精确物理行再插入——插入行数 == 可见行数，永远一致。
  发现者是一位 premortem 输出被明显撕碎的用户。
- **输入溢出。** 输入超过终端宽度时行尾被藏住；现在显示尾部
  (光标跟随)。
- **判决高亮穿过 markdown。** `**5. Verdict**` 和 `## Verdict`
  与普通形式一样变红。
- **僵尸进程卫生**：冒烟测试留下的 TUI 占着 release 二进制的锁；
  现在构建失败会给出明确元凶而非谜团。

#### 新增

- 7 个单元测试：词边界换行(含被报告的 130 字符真实案例)、CJK
  双列记账、超长词硬切、跨行样式保留、空行行数、输入尾部滚动、
  markdown 下的判决匹配。总数:78。

---
### naysay v0.5.0 — 2026-09-05

档案变成记忆。一个主题：过去的决定开始影响现在的决定。形状来自
对 v0.4 的第二次外部审评(8.0/10 — 见 DECISIONS.md D-023)。

#### 新增

- **`naysay decisions relevant "<idea>"`** — 决策存储上的确定性
  检索：查询与每条记录的 idea+正文做 Jaccard 词集重叠，排序取
  前 10。零 LLM、零网络、零依赖。检索是确定性的；解读交给
  LLM——审评提出的架构边界，原样采纳。
- **`--parent <ID>`** 加到 premortem / spec / postmortem — 决策
  修订谱系可写：DEC-001 → REVISIT → DEC-023。`parent` 字段
  v0.3 就存在，直到现在才有人写它。
- **`naysay calibration`** — 诚实极简版。premortem prompt 末尾
  现在输出结构化 `VERDICT: BUILD|DON'T BUILD` 行;postmortem 的
  CALIBRATION 段以结构化 `OUTCOME: BUILT|KILLED|ABANDONED|
  UNKNOWN` 行开头;命令把 premortem 和子 postmortem 链起来，
  报告 held / wrong / overridden，语料不足时打上明确的诚实
  免责。
- **`src/store.rs`** — Decision 脱离 CLI 文件(约 290 行迁出)。
  LOC 警戒线(main.rs ≤ 4000，tui.rs ≤ 3000)的第一次执行：
  在撞线之前拆，而不是之后。
- **README "naysay 自己的决策记录"** — 自我实验公开：23 条
  入档决策、2 个已发布杀项目案例、从前身被杀到工具上线的
  谱系。
- **6 个单元测试** — verdict/outcome 提取(3)、tokenize +
  Jaccard 边界与排序(2)、四象限分类(1)。总数:71。

#### 备注

- LOC：main.rs ~3100，tui.rs ~2600，store.rs ~470——都在警戒线内。
- 刻意延后(D-023)：MODEL CONFIDENCE 改名、JSON schema 输出、
  更丰富的判决分类、`naysay check` 工程决策模式、完整 calibration
  面板。每一条都有书面的重开条件。

---
### naysay v0.4.0 — 2026-09-05

首次运行变成交互式 provider 选择器。无新命令、无新 flag、无新依赖
——presets 是数据,不是抽象(D-022)。

#### 新增

- **首启 provider 选择器** — setup 框现在提供七条路:Ollama(本地、
  免费、免 key)、DeepSeek、GLM(智谱)、OpenAI、MiniMax、OpenRouter
  (也承载 Claude 模型)、Custom(任意 OpenAI 兼容端点)。选择写入
  `naysay.toml`,key 进系统 keyring,TUI 直接以所选 provider 启动。
  已有 key 的老用户完全跳过。
- **Claude 诚实注记** — 选择器明说 Anthropic 的 API 不是 OpenAI
  兼容格式,Claude 用法指向 OpenRouter。
- **6 个单元测试** — 选项解析、preset 合规(URL scheme、model、
  env 名、key 仅限远程规则)、TOML 经 `Config::parse_strict` +
  `validate` 回环、banner 跨版本长度对齐。总数:65。

#### 修复

- **版本横幅过期。** 首启 setup 框连着四个版本都显示 `naysay
  v0.1`——版本号在三处被手写死了。现在所有横幅从
  `CARGO_PKG_VERSION` 编译期派生(`crate::VERSION`),setup 框补宽
  动态计算,任意长度的版本号都不会破框。发现者是一位用户问出了
  最准的问题:"为什么 0.3 还显示 v0.1?"

---

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