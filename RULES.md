# RULES — what naysay is committed to

One page. Currently binding rules only, each pointing at the decision that
established it. The reasoning and the lineage live in
[DECISIONS.md](DECISIONS.md), which is append-only — nothing there is deleted.

*Last reconciled against D-036. If this file and an entry disagree, the entry
wins and this file is wrong.*

## What naysay is

- **Upstream of agents, never alongside them.** `spec` produces an artifact
  for a coding agent; naysay generates no code and runs no agent loop. — D-005
- **The output is the agent's input.** naysay stops at the artifact. — D-005
- **`premortem` is the brand.** First-class, and the README opens with a real
  premortem of the predecessor. — D-004
- **`check` is the frequent entry point.** Engineering decisions, cheaper
  than premortem, meant to run many times per project. — D-030
- **Two repositories.** The human CLI here; the agent-facing skill, hooks and
  plugin in `naysay-agent`. — D-033

## Architecture floor

- **Single binary, zero runtime dependencies.** No Tauri, React, Vite, Node,
  Electron — and no new crate without an explicit decision. — D-006
- **OpenAI wire format, hand-rolled SSE parser.** No SDK. — D-007
- **Streaming in the TUI; single-shot for CLI / REPL / `--save` / `--json`.** — D-008
- **Key in the OS keyring; the `api_key_env` variable overrides it.** — D-009
- **The store is plain JSON files in the working directory.** No index, no
  database, no migration. — D-021

## Governance

- **A decision is logged before the code lands**, in the owner's voice. — D-010
- **A new feature must name what it displaces.** If it does not say what it
  replaces, it is rejected by default. — D-019
- **No featuritis.** The next version does not start until the current one
  has been used. — D-019
- **"Clippy 0 warnings" is verified in CI**, not locally. — D-018
- **CODEMAP.md is updated in the same edit** as any function change. — AGENTS.md
- **Guardrails:** `main.rs` ≤ 4000 LOC, `tui.rs` ≤ 3000 LOC. Crossing either
  line stops feature work and starts module extraction. — D-023

## Rejected, and staying rejected

A browser UI, an MCP server, cloud sync, team dashboards, plugins, agent
orchestration, a vector store, SaaS. — D-023

These get re-proposed regularly. Reject them again unless a new entry
overturns D-023 explicitly.

## The decision loop

- **Retrieval is deterministic; interpretation is the LLM's job.** — D-023
- **Assumptions are tracked entities** with a lifecycle:
  UNKNOWN → VALID / QUESTIONED / INVALIDATED. — D-025
- **Prior verdicts are data, not instructions.** A prior DON'T BUILD is
  precedent, never binding. — D-028
- **Every surface feeds the loop.** TUI, REPL and CLI all write records,
  record session steps, and read memory. — D-031
- **Verdicts are machine-readable:** `VERDICT: BUILD | DON'T BUILD` on
  premortem and check, `OUTCOME: …` on postmortem, so `calibration` can link
  them. — D-020, D-023

## Onboarding and surfaces

- **Onboarding belongs to needing a model, not to launching the TUI.** It is
  TTY-gated: a pipe, a redirect or `--json` fails fast with the env-var
  escape hatch instead of prompting. — D-034
- **A missing key prompts; a wrong key does not.** An auth failure must never
  masquerade as "not configured yet". — D-034
- **One prompt text per command.** CLI and TUI both read `src/prompts.rs`. — D-035
- **The TUI is a workspace, not a chat log:** exploration and store left,
  transcript centre, verdict and assumptions right. — D-035
- **One language per surface.** UI strings, code, comments and docs are
  English; the README keeps its Chinese section as a translation. — D-035

## Open, deferred

- **Standalone CLI commands inherit an auto-created session.** Workaround:
  `naysay session close`. Re-opens the first time a run answers the wrong
  question. — D-032
- **The inline transcript survives behind a flag** until the workspace has
  been used for a week. — D-035
- **The picker still lists Ollama among seven cloud providers** instead of
  making "local, free, no key" the first fork. — D-034

---

<a id="中文"></a>

# 中文

> 这是上面英文版的中文翻译，不是第二份事实来源——两者冲突时以英文版和
> [DECISIONS.md](DECISIONS.md) 为准。

## naysay 是什么

- **在 agent 的上游，不在它旁边。** `spec` 产出的是交给 coding agent 的
  工件；naysay 不生成代码，也不跑 agent 循环。— D-005
- **naysay 的输出就是 agent 的输入。** 它在工件处停下。— D-005
- **`premortem` 是品牌。** 一等公民，README 开篇就是它对自己前身的真实
  premortem。— D-004
- **`check` 是高频入口。** 面向工程决策，比 premortem 便宜，一个项目里要
  跑很多次。— D-030
- **两个仓库。** 人类用的 CLI 在这里；面向 agent 的 skill、hook、plugin
  在 `naysay-agent`。— D-033

## 架构底线

- **单二进制、零运行时依赖。** 不要 Tauri、React、Vite、Node、Electron；
  没有明确决定不加新 crate。— D-006
- **OpenAI 线协议，手写 SSE 解析器。** 不用 SDK。— D-007
- **TUI 流式；CLI / REPL / `--save` / `--json` 单次。** — D-008
- **key 存系统 keyring；`api_key_env` 指定的环境变量可覆盖。** — D-009
- **存储就是工作目录里的普通 JSON 文件。** 没有索引、没有数据库、没有迁移。
  — D-021

## 治理

- **决定先入档，代码后落地**，且用所有者的口吻写。— D-010
- **新功能必须说清它替代了什么。** 不说，默认拒绝。— D-019
- **不要 featuritis。** 当前版本没被用过，就不开下一版。— D-019
- **「clippy 零告警」由 CI 验证**，不靠本地。— D-018
- **改函数就同步改 CODEMAP.md**，同一次编辑里。— AGENTS.md
- **守卫线：** `main.rs` ≤ 4000 行，`tui.rs` ≤ 3000 行。越线就停止加功能、
  开始抽模块。— D-023

## 已拒绝，且继续拒绝

浏览器界面、MCP 服务、云同步、团队看板、插件、agent 编排、向量库、SaaS。
— D-023

这些会反复被重新提出来。除非有新决定明确推翻 D-023，否则继续拒绝。

## 决策循环

- **检索是确定性的，解读是 LLM 的事。** — D-023
- **假设是一等实体**，带生命周期：UNKNOWN → VALID / QUESTIONED /
  INVALIDATED。— D-025
- **历史判决是数据，不是指令。** 以前的 DON'T BUILD 是先例，永远不是约束。
  — D-028
- **每个界面都喂这个循环。** TUI、REPL、CLI 都写记录、写 session step、
  读记忆。— D-031
- **判决是机器可读的：** premortem 和 check 结尾是
  `VERDICT: BUILD | DON'T BUILD`，postmortem 结尾是 `OUTCOME: …`，这样
  `calibration` 才能把它们连起来。— D-020、D-023

## 引导与界面

- **引导属于「需要模型」，不属于「启动 TUI」。** 按 TTY 分流：管道、重定向
  或 `--json` 时快速失败并给出环境变量逃生口，不弹交互。— D-034
- **缺 key 才引导；key 错不引导。** 认证失败绝不能被伪装成「还没配置」。
  — D-034
- **每个命令只有一份 prompt 文本。** CLI 和 TUI 都读 `src/prompts.rs`。
  — D-035
- **TUI 是工作台，不是聊天记录：** 左边探索与存储，中间转录，右边判决与
  假设。— D-035
- **每个界面一种语言。** UI 文案、代码、注释和文档一律英文；README 保留
  中文段作为翻译。— D-035

## 开放、已延后

- **CLI 独立命令会继承自动创建的 session。** 绕过方式：
  `naysay session close`。第一次答错问题时重新打开。— D-032
- **内联转录保留在开关后面**，直到工作台被用满一周。— D-035
- **选择器仍把 Ollama 和七个云端并列**，没有把「本地、免费、免 key」做成
  第一个岔路口。— D-034
