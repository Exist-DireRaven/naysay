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

### naysay v0.1.0 — 2026-09-04

首个 naysay 正式发布。在 pair v1.3 之上构建。

#### 变更

- **二进制、包、keyring 服务、数据目录改名**:`pair` → `naysay`。
  仍读旧 keyring 条目和 `MINIMAX_API_KEY` 环境变量,老用户无需重设。
- **为 agent 时代重新定位**。新的 system prompt 把"开工前先审问"
  作为默认姿态。pair 的"思考伙伴"框架直接和 Claude Code / ZCode
  撞赛道;naysay 占它们留空的上游那一步。
- **可换 provider**,通过 `<data_dir>/naysay.toml`。MiniMax 仍是默认,
  文件里给 OpenAI / DeepSeek / 本地 Ollama 配了带注释的范例块。
  `NAYSAY_CHAT_URL` / `NAYSAY_MODEL` 环境变量能覆盖文件(CI 逃生口)。
- **移除 `build` 命令**。由 `premortem`(这事值不值得做)和
  `spec`(agent 怎么执行)替换。详见 DECISIONS.md D-002。
- **TLS 后端:rustls → native-tls**。mingw 工具链更新导致 `ring`
  的 C 构建坏掉,在不替换工具链的情况下无法修复;在 Windows 上
  native-tls 等价 Schannel,对 HTTPS 调用同样胜任,二进制还瘦
  ~3 MB。详见 DECISIONS.md D-012。
- **行内转录界面**——原来的全屏三面板 TUI(边框 / 历史窗格 / 状态
  条 / alternate screen)没了。对话现在直接打印进终端自己的
  scrollback;唯一活动区是底部两行(`>` 输入 + 暗色状态行)。退出后
  转录稿留在原地,终端原生的 PageUp/scrollback 就能翻。流式逐字
  不再实时渲染——响应整份到达,活跃感由状态行的 spinner + 实时
  字符数承担。`/clear` 只清模型记忆,scrollback 属于终端,留下
  来看。

#### 新增

- **`postmortem <idea> [notes>`** — 项目结束了:复盘 + 一段
  可直接粘贴进 DECISIONS.md 的决策日志条目。纯 REPL 用
  `postmortem <idea> -- 实际情况` 传上下文。补完 seed → premortem →
  spec → postmortem 的闭环。
- **输入历史回溯** — `Ctrl+↑` / `Ctrl+↓` 翻历史(普通 `↑`/`↓`
  仍滚动历史窗格);上限 100 条;打字取消回溯状态。
- **空闲时命令提示** — 输入为空时状态行展示 verdict 系列,杀手锏
  命令不用开 help 也能找到。
- **转录界面会话日志** — TUI 现在也按 JSONL 记两边对话(之前
  TUI 完全不记,`naysay sessions` 看不到大部分真实使用)。这也是
  未来 `--continue` 会话恢复的基础。
- **中文判决高亮** — `is_verdict_line` 在判决 / 结论 / 决定 时也
  触发红字加粗。
- **`--continue` + `/resume [file]`** — 会话恢复。`naysay --continue`
  把最近一次会话的 turns 重放进对话(模型通过 3-turn 窗口自然读到);
  `/resume` 在会话中途做同样的事,文件名参数解析规则和
  `sessions show` 一样。新 turns 追加到同一会话文件。输入召回
  (Ctrl+↑/↓) 从恢复的 turns 里播种。
- **REPL 对话记忆** — 纯 REPL 现在记最近 3 轮(`/context N` 0..=10,
  `/clear` 清空),所以 "X 怎么办?" 这种追问 REPL 也能用。六个命令
  函数接受 `history: &[Message]` 并返回响应文本;REPL 双向记
  日志并在 `--continue` 时回放。未识别命令仍然报错(脚本化模式
  不要 freeform——见 DECISIONS.md D-015)。
- **`premortem <idea>`** — 假设想法六个月后死了,写出尸检报告
  (死因 / 排名死因 / 范围尸检 / 幸存版本 / 判决)。杀手锏 demo:
  对 `FlowForge` 跑这一条,输出就是 README 的开屏画面。
- **`spec <idea>`** — 产出专门喂给 coding agent 的工件。章节:目标 /
  非目标 / 成功标准 / 约束 / 里程碑 / 待决问题。变成管线的
  `agent's input` 那一半。
- **`Config` + `naysay.toml`** — endpoint / model / env-var 名。
  `OnceLock` 单例。坏 TOML 回退默认(同 `prompts.toml` 的契约)。
- **`endpoint_host`** helper,给启动横幅和 doctor 用。
- **token 表** — LLM 响应里的 `usage`(非流式 + 流式末块)现在
  被解析展示:CLI/REPL 每调用后在 stderr 打一行,交互界面状态
  行显示 `ready (1.2s · 812 tok)`。无 usage 时静默退化。
- **重试 + 退避** — 429 和 5xx 最多重试 2 次(1s, 2s 指数退避)。
  交互界面占着终端时重试静默。新增 10s 连接超时,防止挂死。
- **`@dir` 目录内联** — `@./src/` 把目录下每个文本文件打包内联
  (扩展名白名单、跳过 vendor/build 目录、60k 字符总预算、超出
  内联报告)。
- **`doctor` 配置校验** — 新增第一项检查:`naysay.toml` 严格
  TOML 解析 + 字段校验(URL scheme、模型 id、env-var 名),附
  修复提示。
- **单元测试** — `Config::parse`(3)、`endpoint_host`(2)、语言
  检测(4)、display width(3)、verdict 匹配(5)、context 语言
  hint(2)、session-record 解析(3)、newest-session 选择(2)、
  session-arg 解析(1)、REPL context/record(3)、usage 解析(5)、
  retry/backoff(2)、config 校验(1)、@dir 收集(3)。总数:56。
- **`AGENTS.md`** — 协作 agent 的规则。agent 出主意,人决策。
- **`DECISIONS.md`** — 设计日志。代码落地之前,每个非显然决定
  都入档。
- **`CODEMAP.md`** — `main.rs` + `tui.rs` 的逐函数地图。

#### 备注

- 仍然是单文件、零运行时依赖,~9 MB(native-tls 后更小)。
- ~3800 行,比 pair v1.3 的 3381 略多:`build`(230 行)移除了,
  但 `premortem` / `spec` / `postmortem` / provider config /
  语言 hint 层都加了。

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