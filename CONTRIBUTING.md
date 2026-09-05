# Contributing to naysay · 参与贡献

[English](#english) · [中文](#中文)

---

<a id="english"></a>

## English

Thanks for wanting to make naysay better. This document tells you how
to set up, how we review, and the two house rules that make this
project unusual.

### The house rules

1. **Run the premortem on your own PR.** naysay is a tool that
   interrogates ideas before they become code — use it on your own
   proposal: `naysay premortem "<what your PR does>"`. If it surfaces
   a killer you can't answer, the PR isn't ready. This sounds like a
   gimmick until the first time it saves you a week.
2. **Design decisions go in DECISIONS.md.** Every non-obvious choice
   ships with a `D-###` entry (dated, with reasoning and trade-offs)
   *before* or together with the code. A reviewer should never have
   to ask "why is it this way?" — the answer is already in the log.
   This project is developed by a human working with AI agents; the
   decision log is how ownership stays human. See also
   [AGENTS.md](AGENTS.md) if you're an agent.

### Setup

```bash
git clone https://github.com/Exist-DireRaven/naysay
cd naysay
cargo build
cargo test
```

Requirements: Rust 1.75+ (stable). Windows is the primary platform
(some TUI niceties are Windows-only and degrade gracefully
elsewhere); Linux and macOS build and run with a reduced feature set.

Configure a provider to test against:

```bash
naysay key set                     # stores in the OS keyring
# or point at a local Ollama — free, no key:
#   naysay.toml → [provider] chat_url = "http://localhost:11434/v1/chat/completions"
```

### Before you open a PR

```bash
cargo fmt                          # formatting is checked in CI
cargo clippy                       # zero warnings is the bar, -D warnings in CI
cargo test                         # all green, and tests only grow
```

CI runs fmt + clippy + test on Windows, Linux, and macOS. A PR that
lowers test count or introduces clippy warnings will be asked to fix
it.

### Commit style

[Conventional Commits](https://www.conventionalcommits.org/):
`feat:`, `fix:`, `docs:`, `chore:`, `test:`. One logical change per
commit.

### Reporting bugs

Open a GitHub issue with the bug template. Run `naysay doctor` first
and paste the output — it answers half the questions we'd ask anyway.
Never paste your API key anywhere; see [SECURITY.md](SECURITY.md) for
key-leak handling.

### What we will not merge

The [RuFlow list](DECISIONS.md) — plugin systems, GUI frontends,
provider abstraction layers, telemetry. naysay is deliberately small:
one binary, no runtime deps, ~4k lines. Features that grow it past
comprehension get rejected regardless of quality.

---

<a id="中文"></a>

## 中文

感谢你想让 naysay 更好。这份文档讲怎么搭环境、怎么被 review、以及
让这个项目与众不同的两条家规。

### 家规

1. **给自己的 PR 先过一遍 premortem。** naysay 是把想法在变代码前审
   问一遍的工具——用它审问自己的提案:
   `naysay premortem "<你的 PR 要做什么>"`。如果它指出了一个你答不
   上的杀手,PR 还没准备好。这听起来像噱头,直到第一次它真的替你省了
   一周时间。
2. **设计决定入 DECISIONS.md。** 每个非显然的选择,都要带一条 `D-###`
   记录(带日期、带取舍),**先于或随同**代码一起提交。reviewer 不应
   该需要再问"为什么这么写"——答案就在日志里。本项目是一个人 + AI
   agent 协作开发,决策日志是所有权留在人这一边的机制。如果你是
   agent,看 [AGENTS.md](AGENTS.md)。

### 搭环境

```bash
git clone https://github.com/Exist-DireRaven/naysay
cd naysay
cargo build
cargo test
```

要求:Rust 1.75+ stable。主平台是 Windows(部分 TUI 优化是
Windows 专属,在其它平台降级运行);Linux 和 macOS 都能编译,功能
略减。

准备一个 provider 用来测试:

```bash
naysay key set                     # 存入系统 keyring
# 或本地 Ollama —— 免费、无需 key:
#   naysay.toml → [provider] chat_url = "http://localhost:11434/v1/chat/completions"
```

### 开 PR 前

```bash
cargo fmt                          # CI 会强制
cargo clippy                       # 零警告是底线,CI 用 -D warnings
cargo test                         # 全绿,且测试数量只增不减
```

CI 在 Windows / Linux / macOS 三平台跑 fmt + clippy + test。降
测试数、引入 clippy 警告的 PR 会被打回。

### Commit 规范

[Conventional Commits](https://www.conventionalcommats.org/):
`feat:`、`fix:`、`docs:`、`chore:`、`test:`。一个逻辑变更一个
commit。

### 报 bug

用 GitHub issue + bug 模板。先跑 `naysay doctor` 把输出贴上——它
能回答我们一半会反问的问题。**永远不要粘贴 API key**;处理流程
见 [SECURITY.md](SECURITY.md)。

### 不合并的清单

[RuFlow 列表](DECISIONS.md):插件系统、GUI 抽象层、provider 适配
层、telemetry。naysay 故意做小:一个二进制、零运行时依赖、约 4k 行。
让它长大到"超出理解范围"的功能,无论质量高低都不收。
