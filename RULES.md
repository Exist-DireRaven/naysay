# RULES — what naysay is committed to

One page. Currently binding rules only, each pointing at the decision that
established it. The reasoning and the lineage live in
[DECISIONS.md](DECISIONS.md), which is append-only — nothing there is deleted.

*Last reconciled against D-039. If this file and an entry disagree, the entry
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
- **Module boundaries are argued, not budgeted.** Extract when a file has
  grown beyond comprehension or lost cohesion — never to satisfy a line
  count. The LOC guardrail (`main.rs` ≤ 4000, `tui.rs` ≤ 3000) is retired.
  — D-023, D-039

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
