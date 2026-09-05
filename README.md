# naysay

**Your coding agent will happily build the wrong thing.**
**naysay asks: "should you build it at all?"**
**你的 coding agent 会愉快地去做不该做的东西。**
**naysay 问：你到底该不该做它？**

*A decision layer between human intent and agent execution.*
*naysay remembers what you decided — and asks whether it is still true.*
*人类意图与 agent 执行之间的决策层。*
**naysay 记住你做过的决定——并追问它们今天还成立吗。**

[English](#english) · [中文](#中文)

```
idea → premortem → decision → spec → agent → postmortem
```

---

<a id="english"></a>

## English

```
$ naysay premortem "an open-source workflow automation platform"

── premortem: an open-source workflow automation platform ──

Assume this is the autopsy report, six months from now.

1. Cause of death (the most likely one):
   The WASM plugin self-bootstrap system was built before any users
   existed. The day-zero signal: you find yourself explaining *why
   plugins are needed*, not what users can do.

2. Ranked killers (three secondary causes):
   • Scope explosion. The v1 design doc has 16 chapters; the
     implementation will likely touch 6.
   • Agent integration nobody asked for. The home-grown ZCode is the
     real competitor; your WASM abstraction is invisible to it.
   • The marketing site shipped before the MVP — usually a sign the
     team gave up on the MVP.

3. Scope autopsy: imagined features never touched / real feature everything depended on
   • Never touched: Tauri desktop + WASM plugins + self-bootstrap —
     built for a person who doesn't exist.
   • The single thing everything depended on: the smallest command
     that turns a chat into a spec.

4. The version that survived: cut the desktop, cut the plugins, cut
   the bootstrap. Keep one CLI: type an idea, get a spec that has been
   interrogated, ready for any agent.
   Success criteria: spec in under 30 seconds; a decision in under 10
   minutes.

5. Verdict: do NOT build as originally planned. Build the smallest
   version above, run it for two weeks, see if real intent emerges.

Verdict — the only line that earns the red accent.
```

That autopsy is real — it's naysay run on its own predecessor, and
the smallest surviving version it named is this tool.

### Why

Two pains, both amplified by agents:

1. **Agents execute, they don't interrogate.** You say "do this" and
   Claude Code / ZCode / Codex say "ok, started." The pushback step
   is your job — and it's the step most people skip because they want
   momentum. naysay exists to be the one thing that pushes back.

2. **No artifact between "idea" and "agent eats it."** The gap between
   "I want to do X" and "agent receives X" is where the costliest
   decisions get made. A vague prompt produces a vague plan, and agents
   happily fill vague prompts with their own opinions. naysay fills
   that gap with `spec` — a hardened version of your idea an agent
   can't misinterpret.

This is upstream of agents, not a competitor: **naysay's output is the
agent's input.**

### Install

```bash
# From crates.io (recommended)
cargo install naysay

# From a release — grab a prebuilt binary (Windows / macOS / Linux)
# from https://github.com/Exist-DireRaven/naysay/releases
# Unzip, put it on your PATH.

# From source
git clone https://github.com/Exist-DireRaven/naysay && cd naysay
cargo build --release
# binary at target/release/naysay[.exe]
```

First run? naysay walks you through a provider picker — Ollama (local,
free, no key), DeepSeek, GLM, OpenAI, MiniMax, OpenRouter, or any
custom OpenAI-compatible endpoint. Your choice lands in `naysay.toml`,
the key goes to the OS keyring, and the TUI launches. Existing setups
can still switch providers by editing `naysay.toml` or using
`naysay key set`.

```bash
naysay premortem "your idea here"
```

### The five-minute loop

```bash
# 1. cheap breadth: angles you haven't considered
naysay seed "a stock monitoring system"

# 2. depth on the one that sparked
naysay drill "noise budget"

# 3. the interrogation — worth building, at what scope?
naysay premortem "build a stock monitoring system" --save autopsy.md

# 4. if it survives: the hand-off artifact
naysay spec "build a stock monitoring system (minimal version)" --save spec.md
#    open Claude Code / ZCode / Codex in an empty dir: "execute spec.md"

# 5. weeks later — done or dead, feed the decision log
naysay postmortem "stock monitoring system" --save postmortem.md
```

The same loop works conversationally: `naysay` (or `naysay repl`) opens
an interactive session that remembers your last turns — follow-ups
like "what about X?" or "drill into #2" just work. `/context N`
widens the memory window, `/clear` wipes it, and `naysay --continue`
picks up yesterday's session where it left off.

### Commands

```
naysay                        interactive transcript (default)
naysay --continue             resume your most recent session
naysay repl                   plain REPL (scriptable, pipeable)
naysay premortem <idea>       assume it died in 6 months — read the autopsy
naysay spec <idea>            harden an idea into a spec for your agent
naysay postmortem <idea>      it's over — the review + decision-log entry
naysay seed <topic>           brainstorm 8 angles
naysay drill <idea>           drill into one angle
naysay explain <file>         walk through unfamiliar code
naysay key set|status|delete  manage API key in OS keyring
naysay sessions list|show     browse past sessions
naysay doctor                 diagnose setup problems (config / key / network)
```

Global flags: `--save <PATH>` (write to file), `--json` (machine output),
`--sound` (8-bit chimes in the interactive UI), `--tui` (explicit
interactive mode).

```bash
naysay spec "scrape zhihu trending" --save spec.md
naysay explain ./src/scraper.rs --json | jq .explanation
```

### The interactive UI

No chrome, no full-screen takeover: the conversation is a transcript
that scrolls in your terminal's own scrollback, Claude-Code style. The
only live region is two rows at the bottom — `>` input plus one dim
status line — and when you quit, the transcript stays readable right
where a terminal transcript should stay. `Ctrl+↑/↓` recalls previous
inputs, `Tab` completes commands, `@path` inlines a file and `@dir`
inlines a whole source tree (budgeted), `/resume [file]` replays a past
session, and the status line shows a live token meter for every call.

### Providers: any OpenAI-compatible endpoint

naysay speaks the OpenAI chat-completions wire format. On first run it
writes `%LOCALAPPDATA%\naysay\naysay.toml` (platform equivalent
elsewhere) with commented examples — switch provider by uncommenting
one block. The token meter makes the cost visible; a local Ollama
model makes it zero.

```bash
# CI escape hatch — no file editing:
NAYSAY_CHAT_URL=https://api.deepseek.com/chat/completions \
NAYSAY_MODEL=deepseek-chat \
NAYSAY_API_KEY=sk-... \
naysay premortem "your idea"

# offline + private — see naysay.toml for the Ollama block
```

### FAQ

**Why a separate tool? Claude Code can brainstorm too.**
It can — but the executor asking "should we do this?" is the contractor
recommending a bigger house. naysay is structurally on your side,
costs fractions of a cent per interrogation (or nothing, on Ollama),
and keeps a decision log your agent sessions never will.

**Where does my data go?**
To the provider you configure, and nowhere else. No telemetry, no
analytics. Sessions are local JSONL files you can read and delete.

**Windows-only?**
Primary platform is Windows (some sound niceties are Windows-only);
Linux and macOS build and run fine.

**Why is the binary ~9 MB?**
Single-file Rust with tokio + reqwest + ratatui inside. No runtime
dependencies, no installer.

### How this project is built

naysay is developed by a human working with AI coding agents, under
rules that keep ownership human: every design decision is logged in
[DECISIONS.md](DECISIONS.md) (17 entries and counting), the codebase
is mapped in [CODEMAP.md](CODEMAP.md), and the agents that help
maintain it are bound by [AGENTS.md](AGENTS.md). The premortem in
this README killed this project's own predecessor — the lineage is
part of the product. Start with [CONTRIBUTING.md](CONTRIBUTING.md) if
you want in.

### naysay's own decision record

The tool runs on itself. Current state, queryable in this repo:

```
logged decisions   : 26 (DECISIONS.md D-001 … D-026)
kill cases         : 2 published (examples/) — incl. this tool's predecessor
survivor           : the tool you are reading
assumption registry: live (UNKNOWN → VALID/INVALIDATED lifecycle)
memory injection   : premortem/spec prompts carry prior verdicts + risks
calibration        : naysay calibration   (once real loops exist)
```

The most interesting number is not the downloads — it is the list of
things that were **not** built: a Tauri desktop app, a WASM plugin
sandbox, a self-bootstrap installer, an AI calendar, a workflow
platform. Every one has a documented autopsy.

### License

[MIT](LICENSE)

---

<a id="中文"></a>

## 中文

```
$ naysay premortem "做一个开源工作流自动化平台"

── premortem: 做一个开源工作流自动化平台 ──

假设这是六个月后的尸检报告。

1. 死因(最可能的那一种):
   WASM 插件自举系统在做之前没有用户。Day 0 你看到的最早信号:你发现
   自己在解释"为什么需要插件",而不是解释"用户能做什么"。

2. 死因排名(三种次可能):
   • 范围爆炸。v1 设计书里有 16 章,实现里可能用到 6 章。
   • Agent 集成没人在意。MiniMax 自家的 ZCode 才是真正的对手,你的
     WASM 抽象对它毫无意义。
   • 文档网站先于 MVP 写完了——这通常意味着团队放弃了 MVP。

3. 范围尸检:从未被触及的设想功能 / 被依赖的真实功能
   • 从未触及:Tauri 桌面端 + WASM 插件 + 自举更新 — 整个体系是
     给一个不存在的人准备的。
   • 唯一被依赖的:那个让 AI 从对话中生成 spec 的最小命令。

4. 幸存的版本:砍掉桌面端、砍掉插件系统、砍掉自举更新。只留一个
   CLI:输入想法,输出被审问过的 spec 给任何 agent。
   成功条件:30 秒内产出 spec,任何人都能在 10 分钟内决定做不做。

5. 判决:不要以原计划构建。改做上述最小版本,跑两周看有没有真实
   用户意图。
```

上面的尸检是真实的——naysay 跑在它自己的前身之上,得出的最小幸存版本就是它自己。

### 为什么

agent 时代被两个痛点放大:

1. **Agent 只执行,不审问。** 你说"做这个",Claude Code / ZCode / Codex 立刻答"好的,开始了"。审问这一步本该是你的,但人想要 momentum 的时候最容易跳过——naysay 存在的全部意义,就是把"那个说不的声音"做实。

2. **"想法"和"agent 接到它"之间没有产物。** "我想做 X" 到 "agent 收到 X" 的空隙里,代价最大的决策都在这里发生。prompt 模糊,产出的计划就模糊,而 agent 非常乐意用自己想法填满模糊的 prompt。naysay 用 `spec` 命令填上这个空隙——把想法硬化成 agent 没法误读的形态。

naysay 不是 agent 的竞品,**它是 agent 的上游**:naysay 的输出,就是 agent 的输入。

### 安装

```bash
# crates.io(推荐)
cargo install naysay

# 从 release 取:https://github.com/Exist-DireRaven/naysay/releases
# 下载 Windows / macOS / Linux 三平台二进制,解压并加入 PATH。

# 从源码构建
git clone https://github.com/Exist-DireRaven/naysay && cd naysay
cargo build --release
# 二进制在 target/release/naysay[.exe]
```

首次运行会进入 provider 选择器——Ollama(本地、免费、免 key)、
DeepSeek、GLM、OpenAI、MiniMax、OpenRouter,或任意自定义 OpenAI
兼容端点。选择写入 `naysay.toml`,key 存入系统 keyring,然后直接
进 TUI。老用户仍可编辑 `naysay.toml` 换 provider,或用
`naysay key set`。

```bash
naysay premortem "你的想法"
```

### 五分钟循环

```bash
# 1. 便宜的开阔:你没想到的角度
naysay seed "做个股票监控系统"

# 2. 钻进触发兴趣的那一条
naysay drill "噪音预算"

# 3. 审问 — 这事值不值得做?做多大?
naysay premortem "做个股票监控系统" --save autopsy.md

# 4. 活下来的:交付物
naysay spec "做个股票监控系统(最小版)" --save spec.md
#    把 spec.md 喂给 Claude Code / ZCode / Codex:"execute spec.md"

# 5. 几周后 — 做成或死了,回填决策日志
naysay postmortem "股票监控系统" --save postmortem.md
```

也可以会话式地跑:直接 `naysay` 或 `naysay repl`,会话会记住最近几轮——"X 怎么办?" / "钻 #2" 这种追问直接可用。`/context N` 加宽窗口,`/clear` 清空,`naysay --continue` 接上昨天的会话。

### 命令

```
naysay                        交互式会话(默认)
naysay --continue             接着上一次的会话继续
naysay repl                   纯 REPL(可脚本化、可管道)
naysay premortem <idea>       假设它六个月后死了 — 看尸检
naysay spec <idea>            把想法硬化成 agent 能用的 spec
naysay postmortem <idea>      项目结束了 — 复盘 + 决策日志
naysay seed <topic>           8 个角度
naysay drill <idea>           钻进一条
naysay explain <file>         走读陌生代码
naysay key set|status|delete  管理 keyring 里的 API key
naysay sessions list|show     浏览历史会话
naysay doctor                 配置 / key / 网络诊断
```

全局 flag:`--save <PATH>`(写入文件)、`--json`(机器输出)、`--sound`(交互界面 8-bit 音效)、`--tui`(显式进交互模式)。

```bash
naysay spec "做知乎热榜爬虫" --save spec.md
naysay explain ./src/scraper.rs --json | jq .explanation
```

### 交互界面

无边框、无全屏接管——对话就是终端自己的 scrollback 里的转录稿,Claude Code 风格。唯一的活动区是底部两行:`>` 输入行 + 一行暗色状态。退出后转录稿保留在原地。`Ctrl+↑/↓` 召回历史输入,`Tab` 补全命令,`@path` 引入文件,`@dir` 引入整棵源码树(有预算),`/resume [file]` 回到过去的会话,状态行显示每轮的 token 表。

### Provider:任何 OpenAI 兼容端点

naysay 说 OpenAI chat-completions 线协议。首次运行会写 `%LOCALAPPDATA%\naysay\naysay.toml`,里面是带注释的配置范例——取消注释切 provider。token 表让成本可见,本地 Ollama 让它归零。

```bash
# CI 逃生口:无需改文件
NAYSAY_CHAT_URL=https://api.deepseek.com/chat/completions \
NAYSAY_MODEL=deepseek-chat \
NAYSAY_API_KEY=sk-... \
naysay premortem "你的想法"

# 离线 + 私有 — 见 naysay.toml 里的 Ollama 配置块
```

### FAQ

**已经有 Claude Code 了,为什么要单独的 naysay?**
Claude Code 能 brainstorm,但执行者问"这事该做吗"等于承包商建议盖更大的房子。naysay 结构性地站在你这边,每次审问花几分钱(Ollama 上免费),而且保留 agent 会话里永远不会有的一份决策日志。

**数据去哪了?**
去你配置的 provider,仅此而已。无 telemetry、无分析。session 是本地 JSONL,你可以读、可以删。

**只有 Windows?**
主平台是 Windows(部分音效是 Windows 专用);Linux 和 macOS 同样能编译和运行。

**为什么二进制 ~9 MB?**
单文件 Rust,内嵌 tokio + reqwest + ratatui。无运行时依赖、无安装器。

### 这项目怎么做的

naysay 由一个人类与 AI 协作开发,规则保证所有权留在人这一边:每条设计决定都入 [DECISIONS.md](DECISIONS.md)(17 条,还在长),代码地图在 [CODEMAP.md](CODEMAP.md),协作的 agent 受 [AGENTS.md](AGENTS.md) 约束。本 README 开头的 premortem 杀掉了项目自己的前身——血统本身就是产品的一部分。想参与从 [CONTRIBUTING.md](CONTRIBUTING.md) 开始。

### naysay 自己的决策记录

这个工具跑在自己身上。当前状态，本仓库内可查：

```
已入档决策   : 23 条（DECISIONS.md D-001 … D-023）
杀掉的项目   : 2 个已发布案例（examples/）—— 包括本工具的前身
幸存者       : 你正在读的这个工具
calibration  : naysay calibration（等真实决策闭环积累）
```

最有意思的数字不是下载量——是那份**没有被建造**的清单：Tauri 桌面
端、WASM 插件沙箱、自举安装器、AI 日历、工作流平台。每一个都有
带文档的尸检报告。

### 协议

[MIT](LICENSE)
