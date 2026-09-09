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

### naysay v0.3.0 — 2026-09-09 *(unpublished)*

**Onboarding runs where the need is, not where the program started.**
`naysay check "…"` on a fresh machine answered `no API key` and stopped,
while `naysay` with no arguments walked you through the provider picker —
the picker simply lived inside the TUI's launch path. It now runs before any
model call, on every surface. Per D-034.

#### Added

- **The TUI is a workspace, not a chat log (D-035 M2).** Three panes:
  the active session and the decision store on the left, the streaming
  transcript in the centre, the current decision's verdict, confidence,
  assumptions and linked records on the right. The transcript is in-memory
  (no more scrollback), scrolls with ↑↓ / PgUp / PgDn, and `Home`/`End`
  with an empty input jump to the oldest row and back to the tail —
  submitting returns to the tail. The right pane reloads after every write.
  `Ctrl+F` focuses the left pane's store filter; every whitespace-separated
  term must match a row.
- **`src/text.rs` + `src/workspace.rs`.** Pure text layout and the
  three-pane view moved out of `tui.rs`; it is 2615 lines. The boundary is a
  design choice, not a quota — the line ceiling was retired the same day
  (D-039).
- **A worked example in the README.** One real run — key setup, `seed`,
  `drill`, `premortem`, `spec`, then `postmortem --parent` and calibration —
  with the actual output, in both languages. It is the tutorial the first-run
  flow never had.

#### Changed

- **The LOC guardrail is retired (D-039).** `main.rs` ≤ 4000 and `tui.rs` ≤
  3000 LOC is withdrawn. Module boundaries are argued on cohesion and
  comprehension; `CODEMAP.md` plus review is the signal, not a number.
  D-023's rejected list is untouched.

- **`ensure_key()`** — the picker is a property of needing a model, not of
  launching the TUI. CLI and REPL commands run it before their first
  request; the TUI still runs it before taking the terminal.
- **TTY-gated.** Under a pipe, a redirect, or `--json`, the run fails fast
  with the exact env vars and a pointer to `naysay doctor` instead of
  prompting — script behaviour is unchanged.
- **`probe_has_key()` honours a custom `api_key_env`.** It knew only
  `NAYSAY_API_KEY` / `MINIMAX_API_KEY` and the keyring, so a user with
  `DEEPSEEK_API_KEY` configured in `naysay.toml` would have been asked to
  pick a provider they had already configured.
- **No re-onboarding on a wrong key.** Only a missing key triggers the
  picker; an auth failure surfaces the provider's own error.

#### Fixed

- **Assumption extraction no longer assumes English or bare headings.** A
  record whose section headings were bold (`**ASSUMPTIONS**:`) or Chinese
  (`**假设**:`) extracted zero assumptions; only `### ASSUMPTIONS` matched.
  Headings are now matched after stripping `#`, `*`, `_` and a trailing
  colon, and Chinese aliases are accepted. Per D-038.
- **`decisions relevant` finds Chinese ideas.** Retrieval scored with Jaccard
  against idea-plus-body, so a one-line Chinese query capped at 0.07 and
  returned nothing. It now uses the overlap coefficient and CJK character
  bigrams: the same query returns its record at 0.55.
- **stderr is silent while the TUI owns the terminal.** `decision-store:`
  notes were unconditional and would have printed into the middle of the
  fullscreen workspace; they are now gated by `TUI_ACTIVE` like the retry
  notes.
- **A stream that fails mid-answer no longer leaves a duplicate entry.**
  `apply_event` cleared `streaming` before matching on it, so the
  "replace the partial entry" arm was unreachable and the error landed as a
  second entry beside the partial text.
- **A finished stream ends at `[DONE]`.** The reader loop only exited when
  the connection closed and skipped `[DONE]` as "not content", so a provider
  that keeps the connection open after the sentinel (MiniMax's legacy
  endpoint) hung the TUI forever. A 120 s silence is now an error instead of
  an endless "thinking". Per D-041.
- **The keyring was a mock store.** `keyring` 3 ships no keystore by default,
  so `key set` printed `✓ saved to OS keyring` and the credential was gone
  by the next command. The platform stores are now enabled explicitly
  (`windows-native` / `apple-native` / `linux-native-sync-persistent`). Per
  D-040.
- **Assumptions are extracted from the shape the model actually writes.**
  `**ASSUMPTIONS**` followed by a blank line and a numbered list extracted
  nothing, because a blank line ended the section; the same bug dropped
  `confidence` when the value sat on the line under `**CONFIDENCE**`.
- **Parent links resolve in both id forms.** `decisions relevant` prints
  `kind-hex` while the store compares the bare hex, so a `--parent` copied
  from the listing never matched and calibration saw no linked pair.
- **`doctor`'s hint names the configured variable.** It always said
  `NAYSAY_API_KEY`, even when `naysay.toml` expected a different
  `api_key_env` — following the hint could leave the key unread.

#### Notes

- No new command, no new dependency — TTY detection is
  `std::io::IsTerminal`.
- 84/84 tests pass, clippy and fmt clean, release build verified.
- Verified end to end: a fresh `naysay check webui` with no key shows the
  picker; the same command with stdin piped fails fast with the actionable
  message.
- **Workspace milestone:** 91/91 tests pass (three new: width-aware
  truncation, store filtering, and a `TestBackend` render of all three
  panes), `cargo clippy -- -D warnings` and `cargo fmt --check` clean,
  release build verified, and the three panes confirmed in a real terminal
  window.
- **Tutorial run:** 95/95 tests pass, clippy and fmt clean, release build
  verified. Verified live: the `[DONE]`-then-open stub completes instead of
  hanging, the silent stub aborts at 120 s, `key set`/`status`/`delete`
  round-trip through Windows Credential Manager across processes, and the
  full loop above ran end to end against `deepseek-chat` (seed 1012 tok,
  premortem 3747 tok, spec, postmortem 1940 tok).
- **Not published.** 0.2.0 is the current crates.io release. D-019's rule is
  that the next version does not start until the current one has been used,
  and this onboarding has not been used once yet.

---

> **Version-line note (2026-09-09).** `0.1.0` was the only published release.
> `0.2.0`–`0.10.0` were internal iterations that never reached crates.io;
> they are archived as a git bundle plus a source zip. This release publishes
> their combined result as **0.2.0** — the second public release. The internal
> entries below are kept for lineage, and their version numbers do not
> correspond to published artifacts.

### naysay v0.2.0 — 2026-09-09 · first public release since 0.1.0

Everything the internal 0.2–0.10 line built, shipped as one version.

#### Added

- **`naysay check <decision>`** — the engineering-decision interrogation:
  the actual problem, existing coverage, minimum form, six-month failure
  mode, verdict. Ends with `VERDICT: BUILD | DON'T BUILD` so `calibration`
  can link it to a later postmortem. Deliberately cheaper than premortem
  (900 max tokens) because it runs many times per project.
- **Decision memory** — every verdict writes a record to
  `.naysay/decisions/` and a step to its decision session. Premortem and
  check prompts carry prior verdicts on similar ideas plus
  assumption-risk lines.
- **Assumption lifecycle registry** (`.naysay/assumptions.json`) —
  `UNKNOWN → VALID / QUESTIONED / INVALIDATED`, fed by the structured
  `ASSUMPTIONS` block and flipped by postmortems or `decisions verify`.
- **`decisions` / `session` / `calibration` subcommands** — query the
  store, walk a decision's lineage, inspect the exploration tree, and
  compare premortem verdicts against real outcomes.
- **Structured output** — `ASSUMPTIONS / EVIDENCE / UNKNOWNS / CONFIDENCE`
  on premortem, `Assumptions / Failure conditions / Risk budget` on spec,
  `OUTCOME:` on postmortem.
- **Interactive provider picker** — Ollama, DeepSeek, GLM, OpenAI, MiniMax,
  OpenRouter, or any OpenAI-compatible endpoint, written to `naysay.toml`
  with the key in the OS keyring.
- **Full line editing and native wide-char rendering** in the TUI.

#### Fixed

- `premortem` wrote two decision records per run (v0.3–v0.10).
- The assumption registry wrote to a sibling of the intended directory.

#### Known

- **D-032:** standalone CLI commands auto-create a decision session, so a
  later standalone command on an unrelated idea is injected with the first
  one's context. Workaround: `naysay session close`.

---

### naysay v0.10.0 — 2026-09-09 *(internal, unpublished)*

**The decision loop closes on the surface people actually use.** Until this
release the TUI — the default entry point — remembered nothing: no records,
no memory injection. Now all four verdict commands write to the store and
read from it, and their prompts carry the structured sections the CLI has
emitted since v0.2. Per D-031.

#### Changed

- **The TUI writes decisions.** `run_premortem`, `run_spec`,
  `run_postmortem` and `run_check` call `store::save_verdict` (record +
  session step) and prepend `resolve()` context, exactly like the CLI path.
- **TUI prompt parity.** The TUI's premortem and check templates now end
  with `VERDICT: BUILD | DON'T BUILD`; premortem gains the
  `ASSUMPTIONS / EVIDENCE / UNKNOWNS / CONFIDENCE` block; spec gains
  `Assumptions / Failure conditions / Risk budget`. The assumption registry
  and `calibration` are now fed from both surfaces.
- **`prompts.toml` key `check`** is covered by a test that the override
  actually resolves.

#### Dogfood

Every claim above was exercised against the live tool before this entry was
written:

- `naysay check "<the v0.10 plan>"` → `VERDICT: BUILD` — the first real
  record (`check-a7ae928531f6`) and the first real assumptions in the
  registry.
- `naysay check "<the image-to-GIF decision>"` → `VERDICT: DON'T BUILD` —
  use ffmpeg, don't write a pipeline. The Gifkit case, answered in 900
  tokens.
- `naysay check "<make naysay a skill>"` → `VERDICT: DON'T BUILD` — reuse
  what exists rather than build a new skill. Consistent with the two
  premortems of the same idea earlier the same day.

#### Known gap

- **D-032:** standalone CLI commands auto-create a decision session, so a
  later standalone command on an unrelated idea is injected with the first
  one's context. Found by dogfooding — the first GIF check answered the
  previous TUI question. Deferred with a stated re-open condition; the
  workaround today is `naysay session close`.

#### Notes

- 84/84 tests pass (two new: `op_kind_round_trips_including_check`,
  `prompts_check_key_resolves_override_and_default`),
  `cargo clippy -- -D warnings` clean, `cargo fmt --check` clean, release
  build verified.
- LOC: main.rs 3312, tui.rs 2928, store.rs ~1780. Both D-023 guardrails
  hold; `tui.rs` is at 98% of its ceiling.

---

### naysay v0.9.0 — 2026-09-09

**The decision loop gains its most frequent entry point.** The premortem
fires once per project; most decisions are smaller and more frequent — a
dependency, an abstraction, a rewrite. `naysay check` is that entry
point, logged as the "first candidate" in D-023 and never shipped until
now.

#### Added

- **`naysay check <decision>`** — the engineering-decision
  interrogation. Five sections: the actual problem, existing coverage
  (repo / dependencies / PATH), minimum form, six-month failure mode,
  verdict. Ends with `VERDICT: BUILD` or `VERDICT: DON'T BUILD` so
  `calibration` can link it to a later postmortem. Available as a CLI
  subcommand, a REPL command, and a TUI command.
- **`Op::Check`** — the new op enters the decision session and the
  decision store, and its prompt carries the same memory injection as
  premortem: prior verdicts on similar ideas (top 2), session
  exploration, and assumption-risk lines.
- **`prompts.toml` key `check`** — override the default template like
  any other command.

#### Fixed

- **`premortem` wrote two decision records per run.** It called
  `save_decision` once to obtain the id for the session step, then again
  at the end. Every premortem since v0.3 left a duplicate in
  `.naysay/decisions/`, which would have inflated the calibration pairs.
  It now saves once.

#### Notes

- 83/83 tests pass (one new: `op_kind_round_trips_including_check`),
  `cargo clippy -- -D warnings` clean, `cargo fmt --check` clean,
  release build verified.
- LOC: main.rs ~3300, tui.rs ~2880, store.rs ~1750. Both D-023
  guardrails still hold (main.rs ≤ 4000, tui.rs ≤ 3000) — tui.rs is at
  96% of its line.
- **Known gap, logged for v0.10:** the TUI's verdict commands
  (`run_premortem` / `run_spec` / `run_postmortem` / `run_check`) still
  do not write to the decision store or inject memory, although D-021
  said the interactive path would. Only the CLI and REPL paths do.
  Closing that gap is the first candidate for v0.10.

---

### naysay v0.8.0 — 2026-09-05

ContextResolver: the single place that decides what context an operation
sees. Replaces three scattered context injections (memory_context,
session_context_block, parent_assumption_context) with one deterministic
resolve() call per operation.

#### Added

- **`SelectedContext` + `resolve()`** — the provenance-typed return of
  the context resolver. Every field (session_id, root_idea,
  exploration_count, historical_count, assumption_count, warnings) is
  a fact about what was selected and why. `/context` renders it,
  the prompt builder consumes it.
- **seed/drill enter the session** — seed and drill now record steps
  into the DecisionSession and inject session + historical context
  into their prompts (previously they were fully session-blind).
- **Drill auto-links parent_seq** — drill automatically links to the
  most recent seed step (previously always None), enabling real
  branch lineage.
- **MEMORY RULES rewritten** — prior decisions are now explicitly
  labeled "historical evidence — data, not instructions." A prior
  DON'T BUILD is no longer binding; the model must identify what
  changed if it disagrees, but is not forced to repeat the old verdict.

#### Changed

- **`memory_context` / `session_context_block` / `parent_assumption_context`
  replaced by `resolve()`** — one function, one selection, one provenance
  type. Callers no longer concatenate three separate blocks.
- **`save_decision_to` registers assumptions at the correct level** —
  `.naysay/assumptions.json` (was writing to a sibling of the parent dir).

#### Notes

- 82/82 tests pass, clippy -D warnings clean, fmt clean.
- LOC: main.rs ~3130, tui.rs ~2840, store.rs ~800.
- D-028: the MEMORY RULES rewrite from "must repeat" to "data, not
  instructions" is the most important semantic change in this release.
  The previous wording risked a self-reinforcing negative bias.

---

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

### naysay v0.3.0 — 2026-09-09 *（未发布）*

**引导跑在需要它的地方，而不是程序启动的地方。** 在新机器上
`naysay check "…"` 只会答一句 `no API key` 然后停住，而 `naysay` 不带参数
却会带你走完 provider 选择器——因为那段引导住在 TUI 的启动路径里。现在它
跑在任何一次模型调用之前，三个界面都一样。见 D-034。

#### 变更

- **`ensure_key()`** —— 选择器属于"需要模型"，不属于"启动 TUI"。CLI 和
  REPL 命令在第一次请求前调用它；TUI 仍在接管终端之前调用。
- **按 TTY 分流。** 管道、重定向或 `--json` 时快速失败，打印确切的环境变量
  并指向 `naysay doctor`，不弹交互——脚本行为不变。
- **`probe_has_key()` 认自定义 `api_key_env` 了。** 它以前只认
  `NAYSAY_API_KEY` / `MINIMAX_API_KEY` 和 keyring，于是已经配好
  `DEEPSEEK_API_KEY` 的人反而会被要求再选一次 provider。
- **key 错时不重新引导。** 只有缺 key 才触发选择器；认证失败直接抛出
  provider 自己的错误。

#### 备注

- 不加命令、不加依赖——TTY 判断用 `std::io::IsTerminal`。
- 84/84 测试通过，clippy 与 fmt 干净，release 构建已验证。
- 端到端验证过：新机器上 `naysay check webui` 会显示选择器；同一条命令
  在 stdin 被管道化时快速失败并给出可操作的提示。
- **未发布。** crates.io 上目前是 0.2.0。D-019 的规矩是当前版本被用过之前
  不开下一版，而这段新引导一次都还没被用过。

---

> **版本线说明（2026-09-09）。** 只有 `0.1.0` 真正发布过。`0.2.0`–`0.10.0`
> 是内部迭代，从未上 crates.io；它们已作为 git bundle + 源码 zip 归档。
> 本次把它们合并后的成果作为 **0.2.0** 发布——第二个公开发布版本。下面的
> 内部条目保留以记录血统，其版本号不对应任何已发布的产物。

### naysay v0.2.0 — 2026-09-09 · 0.1.0 之后的首次公开发布

内部 0.2–0.10 这条线造出来的东西，作为一版发布。

#### 新增

- **`naysay check <decision>`** — 工程决策审问：真实问题、已有覆盖、最小
  形态、六个月失败模式、判决。以 `VERDICT: BUILD | DON'T BUILD` 结尾，
  让 `calibration` 能连到之后的 postmortem。比 premortem 便宜（900
  max tokens），因为它要跑很多次。
- **决策记忆** — 每条判决写入 `.naysay/decisions/` 记录和 decision
  session 的一步；premortem 与 check 的 prompt 带相似想法的历史判决和
  假设风险行。
- **假设生命周期注册表**（`.naysay/assumptions.json`）——
  `UNKNOWN → VALID / QUESTIONED / INVALIDATED`，由结构化 `ASSUMPTIONS`
  段落喂入，由 postmortem 或 `decisions verify` 翻转。
- **`decisions` / `session` / `calibration` 子命令** — 查询存储、走决策
  血统、看探索树、把 premortem 判决和真实结果对照。
- **结构化输出** — premortem 的 `ASSUMPTIONS / EVIDENCE / UNKNOWNS /
  CONFIDENCE`、spec 的 `Assumptions / Failure conditions / Risk budget`、
  postmortem 的 `OUTCOME:`。
- **交互式 provider 选择器** — Ollama、DeepSeek、GLM、OpenAI、MiniMax、
  OpenRouter 或任意 OpenAI 兼容端点，写入 `naysay.toml`，key 进系统
  keyring。
- **TUI 完整行编辑 + 宽字符原生渲染**。

#### 修复

- `premortem` 每次运行写两条记录（v0.3–v0.10）。
- 假设注册表写到了目标目录的兄弟目录。

#### 已知

- **D-032：** CLI 独立命令会自动创建 decision session，之后一条关于无关
  想法的独立命令会被注入第一条的上下文。绕过方式：`naysay session close`。

---

### naysay v0.10.0 — 2026-09-09 *（内部，未发布）*

**决策循环在人真正会用的那个界面上闭合了。** 在此之前，TUI（默认入口）
什么都不记：不写记录、不注入记忆。现在四个判决命令都写入存储、也从存储
读取，prompt 带上了 CLI 从 v0.2 起就有的结构化段落。见 D-031。

#### 变更

- **TUI 开始写决策。** `run_premortem` / `run_spec` / `run_postmortem` /
  `run_check` 调用 `store::save_verdict`（记录 + session step）并前置
  `resolve()` 上下文，与 CLI 路径一致。
- **TUI prompt 对齐。** premortem 与 check 模板现在以
  `VERDICT: BUILD | DON'T BUILD` 结尾；premortem 补上
  `ASSUMPTIONS / EVIDENCE / UNKNOWNS / CONFIDENCE`；spec 补上
  `Assumptions / Failure conditions / Risk budget`。假设注册表和
  `calibration` 现在两个界面都喂得到。
- **`prompts.toml` 的 `check` 键**有了覆盖是否生效的测试。

#### 狗粮

本条目的每一条声明在写之前都对着真工具跑过：

- `naysay check "<v0.10 计划>"` → `VERDICT: BUILD`，第一条真实记录
  （`check-a7ae928531f6`）和注册表里的第一批真实假设。
- `naysay check "<图片转 GIF 的决定>"` → `VERDICT: DON'T BUILD` —— 用
  ffmpeg，别写管线。Gifkit 那个案例，900 tokens 答对。
- `naysay check "<把 naysay 做成 skill>"` → `VERDICT: DON'T BUILD` ——
  复用已有的，不要新建 skill。与当天早些时候同一想法的两次 premortem
  一致。

#### 已知缺口

- **D-032：** CLI 独立命令会自动创建 decision session，于是之后一条关于
  无关想法的独立命令会被注入第一条的上下文。狗粮时发现——第一次 GIF
  检查回答的是上一条 TUI 的问题。已按"写出重新打开条件"的方式延后；
  当前的绕过方式是 `naysay session close`。

#### 备注

- 84/84 测试通过（新增 `op_kind_round_trips_including_check`、
  `prompts_check_key_resolves_override_and_default`），
  `cargo clippy -- -D warnings` 干净，`cargo fmt --check` 干净，
  release 构建已验证。
- LOC：main.rs 3312，tui.rs 2928，store.rs ~1780。D-023 两条守卫线
  成立；tui.rs 已到 98%。

---

### naysay v0.9.0 — 2026-09-09

**决策循环拿到了它最常用的入口。** premortem 每个项目只触发一次；
而大多数决定更小、更频繁——加一个依赖、抽一层抽象、重写一个模块。
`naysay check` 就是那个入口，D-023 里写下的 "first candidate"，自
v0.5 起挂着，一直没落地。

#### 新增

- **`naysay check <decision>`** — 工程决策审问。五段：真实问题、
  已有覆盖（仓库 / 依赖 / PATH）、最小形态、六个月后的失败模式、
  判决。结尾输出 `VERDICT: BUILD` 或 `VERDICT: DON'T BUILD`，让
  `calibration` 能把它和之后的 postmortem 连起来。CLI 子命令、
  REPL 命令、TUI 命令三处可用。
- **`Op::Check`** — 新操作进入决策会话与决策存储，prompt 与
  premortem 一样带记忆注入：相似想法的历史判决（取 2 条）、会话
  探索、假设风险行。
- **`prompts.toml` 新增 `check` 键** — 与其他命令一样可覆盖默认模板。

#### 修复

- **`premortem` 每次运行写两条决策记录。** 它先调一次
  `save_decision` 拿 id 给 session step，结尾又调了一次。v0.3 以来
  的每次 premortem 都在 `.naysay/decisions/` 里留下重复记录，会虚高
  calibration 的配对数量。现在只写一次。

#### 备注

- 83/83 测试通过（新增 `op_kind_round_trips_including_check`），
  `cargo clippy -- -D warnings` 干净，`cargo fmt --check` 干净，
  release 构建已验证。
- LOC：main.rs ~3300，tui.rs ~2880，store.rs ~1750。D-023 的两条
  守卫线仍成立（main.rs ≤ 4000、tui.rs ≤ 3000）——tui.rs 已到 96%。
- **已知缺口，记入 v0.10：** TUI 的判决类命令（`run_premortem` /
  `run_spec` / `run_postmortem` / `run_check`）仍不写决策存储、不注入
  记忆，尽管 D-021 说过交互路径会写。目前只有 CLI 和 REPL 路径会写。
  补上它是 v0.10 的第一候选。

---

### naysay v0.8.0 — 2026-09-05

ContextResolver：决定一次操作能看到什么上下文的唯一入口。替换了
三处分散注入（memory_context / session_context_block /
parent_assumption_context）。

#### 新增

- **`SelectedContext` + `resolve()`** — 带 provenance 的返回类型。
  每个字段都是事实。
- **seed / drill 进入 session** — 此前完全无状态；现在记录 step
  并注入 session + 历史决策上下文。
- **drill 自动链接 parent_seq** — drill 自动关联最近的 seed step
  （此前永远 None）。
- **MEMORY RULES 改写** — 历史决策明确标注为"历史证据——数据，
  非指令"。D-028。

#### 变更

- **三个注入函数合并为 `resolve()`** — 一个函数、一次选择、一个
  provenance 类型。
- **假设注册表路径修正** — 写到 `.naysay/assumptions.json`。

#### 备注

- 82/82 测试、clippy 清洁、fmt 干净。
- LOC：main.rs ~3130，tui.rs ~2840，store.rs ~760。

---

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