# Contributing to naysay

Thanks for wanting to make naysay better. This document tells you how to
set up, how we review, and the two house rules that make this project
unusual.

## The house rules

1. **Run the premortem on your own PR.** naysay is a tool that interrogates
   ideas before they become code — use it on your own proposal:
   `naysay premortem "<what your PR does>"`. If it surfaces a killer you
   can't answer, the PR isn't ready. This sounds like a gimmick until the
   first time it saves you a week.
2. **Design decisions go in DECISIONS.md.** Every non-obvious choice ships
   with a `D-###` entry (dated, with reasoning and trade-offs) *before* or
   together with the code. A reviewer should never have to ask "why is it
   this way?" — the answer is already in the log. This project is developed
   by a human working with AI agents; the decision log is how ownership
   stays human. See also [AGENTS.md](AGENTS.md) if you're an agent.

## Setup

```bash
git clone https://github.com/<owner>/naysay
cd naysay
cargo build
cargo test
```

Requirements: Rust 1.75+ (stable). Windows is the primary platform
(some TUI niceties are Windows-only and degrade gracefully elsewhere);
Linux and macOS build and run with a reduced feature set.

Configure a provider to test against:

```bash
naysay key set                     # stores in the OS keyring
# or point at a local Ollama — free, no key:
#   naysay.toml → [provider] chat_url = "http://localhost:11434/v1/chat/completions"
```

## Before you open a PR

```bash
cargo fmt                          # formatting is checked in CI
cargo clippy                       # zero warnings is the bar, -D warnings in CI
cargo test                         # all green, and tests only grow
```

CI runs fmt + clippy + test on Windows, Linux, and macOS. A PR that
lowers test count or introduces clippy warnings will be asked to fix it.

## Commit style

[Conventional Commits](https://www.conventionalcommits.org/): `feat:`,
`fix:`, `docs:`, `chore:`, `test:`. One logical change per commit.

## Reporting bugs

Open a GitHub issue with the bug template. Run `naysay doctor` first and
paste the output — it answers half the questions we'd ask anyway. Never
paste your API key anywhere; see [SECURITY.md](SECURITY.md) for key-leak
handling.

## What we will not merge

The [RuFlow list](DECISIONS.md) — plugin systems, GUI frontends, provider
abstraction layers, telemetry. naysay is deliberately small: one binary,
no runtime deps, ~4k lines. Features that grow it past comprehension get
rejected regardless of quality.