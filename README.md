# naysay

[![CI](https://github.com/<owner>/naysay/actions/workflows/ci.yml/badge.svg)](https://github.com/<owner>/naysay/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/naysay.svg)](https://crates.io/crates/naysay)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**The voice that says no before your coding agents say yes.**

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

That autopsy is real — it's naysay run on its own predecessor, and the
smallest surviving version it named is this tool.

## Why

Two pains, both amplified by agents:

1. **Agents execute, they don't interrogate.** You say "do this" and
   Claude Code / ZCode / Codex say "ok, started." The pushback step is
   your job, and it's the step most people skip because they want
   momentum. naysay exists to be the one thing that pushes back.

2. **No artifact between "idea" and "agent eats it."** The gap between
   "I want to do X" and "agent receives X" is where the costliest
   decisions get made. A vague prompt produces a vague plan, and agents
   happily fill vague prompts with their own opinions. naysay fills that
   gap with `spec` — a hardened version of your idea an agent can't
   misinterpret.

This is upstream of agents, not a competitor: **naysay's output is the
agent's input.**

## Install

**From crates.io** (recommended):

```bash
cargo install naysay
```

**From a release** — grab a prebuilt binary (Windows / macOS / Linux)
from the [Releases page](https://github.com/<owner>/naysay/releases),
unzip, put it on your `PATH`.

**From source:**

```bash
git clone https://github.com/<owner>/naysay && cd naysay
cargo build --release
# binary at target/release/naysay[.exe]
```

**First run** — get a key from your provider ([MiniMax](https://api.minimax.chat)
is the default; OpenAI / DeepSeek / local Ollama work too), then:

```bash
naysay key set        # stored in your OS keyring
naysay premortem "your idea here"
```

## The five-minute loop

```bash
# 1. cheap breadth: angles you haven't considered
naysay seed "股票监控系统"

# 2. depth on the one that sparked
naysay drill "噪音预算"

# 3. the interrogation — is this worth building, at what scope?
naysay premortem "做一个股票监控系统" --save autopsy.md

# 4. if it survives: the hand-off artifact
naysay spec "做一个股票监控系统(最小版)" --save spec.md
#    now open Claude Code / ZCode / Codex in an empty dir: "execute spec.md"

# 5. weeks later — done or dead, feed the decision log
naysay postmortem "股票监控系统" --save postmortem.md
```

Steps 1–5 also work conversationally: `naysay` (or `naysay repl`)
starts an interactive session that remembers your last turns — follow-ups
like "what about X?" or "drill into #2" just work, `/context N` widens
the memory window, and `naysay --continue` picks up yesterday's session
where it left off.

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

```
naysay spec "做知乎热榜爬虫" --save spec.md
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
elsewhere) with commented examples — switch provider by uncommenting one
block. The token meter makes the cost visible; a local Ollama model makes
it zero.

```bash
# CI escape hatch — no file editing:
NAYSAY_CHAT_URL=https://api.deepseek.com/chat/completions \
NAYSAY_MODEL=deepseek-chat \
NAYSAY_API_KEY=sk-... \
naysay premortem "your idea"

# offline + private — see naysay.toml for the Ollama block
```

## FAQ

**Why a separate tool? Claude Code can brainstorm too.**
It can — but the executor asking "should we do this?" is the contractor
recommending a bigger house. naysay is structurally on your side, costs
fractions of a cent per interrogation (or nothing, on Ollama), and keeps
a decision log your agent sessions never will.

**Where does my data go?**
To the provider you configure, and nowhere else. No telemetry, no
analytics. Sessions are local JSONL files you can read and delete.

**Windows-only?**
Primary platform is Windows (some sound niceties are Windows-only);
Linux and macOS build and run fine.

**Why is the binary ~9 MB?**
Single-file Rust with tokio + reqwest + ratatui inside. No runtime
dependencies, no installer.

## How this project is built

naysay is developed by a human working with AI coding agents, under
rules that keep ownership human: every design decision is logged in
[DECISIONS.md](DECISIONS.md) (17 entries and counting), the codebase is
mapped in [CODEMAP.md](CODEMAP.md), and the agents that help maintain it
are bound by [AGENTS.md](AGENTS.md). The premortem in the README killed
this project's own predecessor — the lineage is part of the product.
Start with [CONTRIBUTING.md](CONTRIBUTING.md) if you want in.

## License

[MIT](LICENSE)