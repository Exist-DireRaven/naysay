# DECISIONS — the design log

# DECISIONS — 设计日志

English-only by design — the design log is read mostly by humans (and the occasional agent) who already work in code; English keeps the audience widest. The README and other user-facing docs are bilingual.

This file is part of the codebase. If you change the rules, change this file.
本文件是代码库的一部分。规则若改,本文件随之改。



This file is the audit trail. Every non-obvious choice in naysay is here,
with the reasoning captured at the time it was made. Future maintainers
(including me, three months from now) can read this and know *why* the
code looks the way it does.

The companion file `CODEMAP.md` answers "what does this do?".
`DECISIONS.md` answers "why does this exist?".

---

## D-001 · Name: pair → naysay (2026-09-04)

**Decision:** Rename the binary from `pair` to `naysay`. Bump version
back to `0.1.0`.

**Why:**
1. *pair* described a function (brainstorm together). naysay describes a
   posture (interrogate before committing). The new posture is the one
   the tool actually takes.
2. *pair* invited direct comparison with Claude Code / ZCode (which can
   also "pair" with you). naysay occupies the space they leave empty.
3. The *yes* / *naysay* symmetry gives a one-line identity: "$ yes" is the
   classic Unix tool that never stops saying yes. naysay is its dialectical
   opposite.

**Trade-off:** Version resets to 0.1.0 even though the binary carries
~3400 lines of mature code. This is intentional: the positioning
changed enough that old CHANGELOG entries read like a different product.

## D-002 · Drop the `build` command (2026-09-04)

**Decision:** Remove `pair build` (the 5-step "reasoning first, code last"
planner) in favor of `premortem` + `spec`.

**Why:**
1. `build` was naysay doing naysay's competitor's job. Step 5 ("write the
   actual code") is where Claude Code lives, and naysay has no business
   competing with the agent on code generation.
2. The premortem / spec split better matches the user decision:
   *should I do this?* (premortem) → *how will I do this?* (spec).
3. The build-style "reasoning first" cadence lives inside spec now —
   every spec section exists because a hand-wavy version invites agent
   improvisation.

**Trade-off:** Anyone with `pair build` muscle memory needs to relearn.
Acceptable: that audience was the developer, not the world.

## D-003 · Configurable provider via `naysay.toml` (2026-09-04)

**Decision:** Replace the hard-coded MiniMax endpoint with a
`naysay.toml` provider block. MiniMax remains the default; the file is
written on first run with commented examples.

**Why:**
1. The original claim ("partner in your terminal") assumed MiniMax. With
   the agent-economy positioning, users will want local Ollama, DeepSeek,
   OpenAI — whichever they trust with their ideas.
2. naysay already speaks the OpenAI wire format, so this is purely a
   config layer, not a protocol layer.
3. Bad TOML → defaults (same contract as prompts.toml). A misconfigured
   config never blocks startup.

**Trade-off:** More docs to read on first run. Mitigated by the
fully-commented template.

## D-004 · `premortem` is the brand (2026-09-04)

**Decision:** `premortem` ships as a first-class command, not buried in
a list of 12 "thinking tools". The README opens with a premortem of
*naysay's predecessor*.

**Why:**
1. The premortem frame is the single most underused tool in the
   brainstorm family. Most people can list features; few can list the
   ways their idea could die.
2. The killer demo for naysay is *running it on its own predecessor* —
   that output is the README's first screen, because it does the job of
   a feature list without being one. "Here's what killed v1" beats
   "here's what v2 does" every time.
3. The tool eats its own cooking on day one. The first entries in the
   decision log are decisions this tool would have forced on its
   predecessor.

## D-005 · Position upstream of agents, not alongside (2026-09-04)

**Decision:** `spec` produces an artifact designed to be handed to a
coding agent. naysay does not generate code. naysay does not run an
agent loop. naysay stops at the artifact.

**Why:**
1. Tooling is fragmenting: Claude Code, ZCode, Codex, Cursor. A workflow
   tool that picks a side loses everyone on the other sides.
2. The decision step is upstream of all of them. If naysay produces a
   spec good enough that *any* agent can run it, naysay is tool-agnostic.
3. This is a structural fix for the earlier positioning problem: when
   pair claimed "thinking partner" it competed with agents on adjacent
   ground. naysay occupies adjacent *upstream* ground instead.

## D-006 · Single binary, zero runtime deps (carried from pair)

**Decision:** No Tauri, no React, no Vite, no Node, no Electron.

**Why:** The premise is "the voice that says no" — that voice should be
the simplest thing that runs. Anyone on a fresh machine should be able
to `./naysay` and have it work.

The cost: ratatui gives a less rich UI than a web frontend. Acceptable
because the UI is intentionally minimal (history + input + status, three
panes). Everything complex lives in the prompts.

## D-007 · OpenAI wire format, hand-rolled SSE parser (carried from pair)

**Decision:** Speak the OpenAI chat-completions protocol directly.
Parse SSE in a 50-line hand-rolled function, not via an SDK.

**Why:** Every LLM provider speaks a *slight* variant of OpenAI's wire
format. Adopting the SDK locks you to OpenAI's choices. Adopting the
protocol gives you the freedom to point at any compatible endpoint
(see D-003).

The SSE parser has 9 unit tests covering every shape of chunk
finish-reason chunks emit (and they emit them in surprising ways).
The cost of writing it was a Friday afternoon; the cost of being
locked to a provider SDK is permanent.

## D-008 · Streaming by default in TUI, non-streaming for CLI/REPL
(carried from pair)

**Decision:** TUI uses `call_llm_stream`. CLI / REPL / `--save` /
`--json` paths use `call_llm` (single-shot).

**Why:** Streaming in a TUI makes the response feel alive. Streaming to
stdout is decorative at best and annoying at worst — the consumer
script probably wants to know the call is done, not watch tokens arrive.
Splitting on "is there a human watching this in real-time" is the cleanest
cut.

## D-009 · OS keyring, env var escape hatch (carried from pair)

**Decision:** API key lives in the OS keyring (Windows Credential
Manager / macOS Keychain / Linux Secret Service). The env var
configured in `naysay.toml` (`api_key_env`, defaults to
`NAYSAY_API_KEY`) overrides the keyring.

**Why:** CI / scripting should never have to touch the keyring. An env
var override is the simplest possible escape hatch and the cheapest
one to document.

`MINIMAX_API_KEY` is also read as a legacy fallback so existing pair
installations keep working after the rename.

## D-010 · Decide with the user, not for them (2026-09-04)

**Decision:** This file is the first artifact the user (me) created in
the rename. Going forward, every non-trivial change adds a new D entry
*before* the code lands, written in the user's voice.

**Why:** "Code I wrote" and "code I own" are not the same thing. The
test for ownership is: can I justify this design choice in front of
someone who hasn't seen the codebase? If I can't, I don't own it yet.

This file is the cure for that.
## D-011 · `postmortem` closes the loop (2026-09-04)

**Decision:** Ship `postmortem <idea>` as the fourth verdict command.
Its section 5 is a self-contained markdown entry formatted for a
DECISIONS.md file.

**Why:**
1. The README's pipeline diagram promised it before it existed — a
   documentation-implementation gap, which is exactly the kind of rot
   this tool exists to prevent.
2. A premortem without a postmortem is prediction without calibration.
   Only by re-reading the autopsy after the fact do you learn whether
   your day-one fears were the right ones.
3. The decision log only compounds if every ended project feeds it.
   postmortem's output is designed to be pasted in verbatim.

**Trade-off:** Without user notes the model must speculate; the prompt
forces it to label guesses as guesses and name the evidence it needs.

## D-012 · TLS backend: rustls → native-tls (2026-09-04)

**Decision:** Switch reqwest from `rustls-tls` to `native-tls`.

**Why:** A mingw toolchain update broke `ring`'s C build (rustls's
crypto backend) — compiler-family detection failed and could not be
repaired in-tree without replacing the toolchain. On Windows,
native-tls means Schannel: equally capable for HTTPS chat calls, one
less C toolchain in the build graph, and the binary shrinks ~3 MB.

**Trade-off:** Loss of the "pure Rust, cross-compiles anywhere" property
rustls advertised. Accepted: naysay already has Windows-only code
(Win32 Beep, console control handler), so Schannel's platform coupling
adds no new constraint. If cross-compilation ever matters, revisit.

## D-013 · The TUI logs what the REPL logs (2026-09-04)

**Decision:** The TUI writes both user and assistant turns to the same
JSONL session format the plain REPL uses. Both sides are logged, not
just the prompts.

**Why:** `naysay sessions` was blind to most actual usage. And a
decision-log tool that doesn't keep its own decision history would be
a bad joke. Logging both sides is also the prerequisite for the next
feature on the roadmap: `--continue` session resume.

## D-014 · Session resume, TUI-first (2026-09-04)

**Decision:** `--continue` and `/resume [file]` replay a session's turns
into the TUI conversation, and new turns append to the resumed file
rather than forking a new one. The plain REPL accepts the flag but only
prints a note — it has no conversation memory to replay into.

**Why:**
1. pair's v1.0 "known limitations" listed "no session resume — closing
   a REPL drops context". Six weeks of that limitation is what made the
   tool a toy: a thinking partner you have to re-brief every morning is
   an acquaintance, not a partner.
2. Appending to the same file keeps `sessions list` honest — one
   continuous engagement reads as one session, and the next `--continue`
   picks up the whole arc.
3. TUI-first because the TUI already logs both sides (D-013); the plain
   REPL logs only user turns, so a replay there would be half a memory.
   When the REPL gains conversation memory, resume follows naturally.

**Trade-off:** A long session replayed whole can flood the history pane.
Mitigated: the viewport starts at the bottom, and build_context still
only sends the last N turns to the model — the pane shows history the
model doesn't re-read.

## D-015 · REPL gains memory; deliberately no freeform (2026-09-04)

**Decision:** The plain REPL now keeps conversation memory (default 3
turns, `/context N` 0..=10, `/clear` to wipe). The six command functions
take a `history: &[Message]` parameter and return the response text, so
the REPL records both sides in the session log and replays a session on
`--continue`. Freeform input is deliberately NOT added to the REPL: an
unrecognized command still errors.

**Why:**
1. The REPL was second-class: every command was a stateless single-shot,
   follow-ups were impossible, and its session logs held only user turns
   — so a resumed session there would have been half a memory.
2. Logging the assistant side completes the invariant "every session is
   fully replayable" for both interfaces (D-013).
3. Freeform stays out of the REPL on purpose: the REPL is the scripted,
   pipped-input mode (`echo ... | naysay repl`), where a typo should
   fail loudly rather than silently become an LLM call. The TUI owns
   freeform. Revisit with an explicit `--freeform` flag if ever needed.

**Trade-off:** `context()` clones the message window per call — six
cloned messages, irrelevant next to the HTTP round-trip.

## D-016 · Inline transcript UI — no chrome (2026-09-04)

**Decision:** Replace the full-screen three-pane TUI with an inline
transcript. Finished conversation turns are printed into the terminal's
own scrollback (ratatui `Viewport::Inline` + `insert_before`); the only
live region is a two-row strip: `> ` input and one dim status line.
Responses are no longer streamed live into a pane — they arrive as a
finished document, and the status line carries the liveness (spinner +
character count) instead.

**Why:**
1. The full-screen TUI borrowed the terminal and gave it back empty:
   alternate screen means the conversation evaporated on quit. Claude
   Code's defining UI property is the opposite — the transcript *is* the
   terminal buffer, so PageUp and scrollback work natively and the
   session stays readable after exit. A decision-log tool that erases
   its own history on exit was fighting its thesis.
2. Apple-design translation to a terminal: content is the interface.
   No borders, no boxes, no title bars — hierarchy comes from spacing
   and weight, the single red accent stays reserved for verdicts, and
   everything metadata is dim. Two live rows are the entire chrome.
3. Streaming-into-a-pane was eye candy; an autopsy report reads as a
   document. Watching tokens arrive does not improve a premortem. The
   character counter in the status line answers the only real question
   streaming answered ("is it alive?").

**Trade-offs:**
- `handle_key` lost Up/Down/PageUp scrolling — the terminal owns the
  buffer now, which is the point, but in-flight responses can't be
  scrolled away from (they don't exist yet; the status line shows
  progress instead).
- `line_height` estimates wrapped height with our own `display_width`;
  if a terminal disagrees with our width tables, insert heights drift by
  a row. Accepted: the same table drives the cursor fix (D-010-era), and
  the common cases (ASCII + CJK) are exact.
- `/clear` resets the `flushed` cursor along with history — a stale
  cursor past the new length would silently suppress every future
  flush. The scrollback above survives; only the model's memory is
  wiped.

## D-017 · Open-source release — three completenesses (2026-09-05)

**Decision:** Promote naysay from "private tool that works" to "public
open-source project": product polish (token meter, 429/5xx retry,
`@dir` inlining, doctor config validation), engineering completeness
(git history, LICENSE, CI on three platforms, automated releases), and
community completeness (CONTRIBUTING / SECURITY / templates / README
finalized). First release: v0.1.0 — not 1.0.

**Why:**
1. "Complete" for an open-source project is three separate claims: a
   stranger can *use* it, *trust* it, and *participate* in it. Each has
   its own deliverables; shipping one without the others wastes the
   first impression.
2. v0.1.0 over 1.0.0: honest versioning is the first signal a stranger
   reads. 1.0 promises stability we haven't earned.
3. The token meter serves the positioning directly — "interrogation
   should be cheap" is only credible if the meter is visible.
4. The four polish items all passed their own premortem: none grows the
   tool past ~150 lines each, none adds a dependency, all deepen the
   existing value instead of adding surface.

**Trade-offs:**
- Retry covers 429/5xx only; connection-level failures still fail fast
  (distinguishing the two needs a mock server, which is disproportionate
  to the value at this size).
- `@dir` uses an allowlist of extensions and a 60k-char budget — a
  denylist would inline binaries as mojibake; a bigger budget would
  blow 32k-token contexts.
- CI installs libssl-dev on Ubuntu (native-tls, D-012) — documented
  cost of that decision.

## D-018 · Clippy "0 warnings" must be CI-verified, not local-verified (2026-09-05)

**Decision:** The "clippy `-D warnings` clean" check must be re-validated
whenever the Rust toolchain version used by CI changes. Local
verification against the user's installed rustc is not enough.

**Why:**
1. Clippy lints are added and modified in every rust release.
   A codebase that passes clippy under rust 1.85 may fail under
   1.98 (this is what caught naysay v0.1.0 — `EXPORT_TITLE` and
   `EXPORT_TS_TAG` were never used, and `needless_borrow` flagged
   `.replace("{x}", &x)` on a `&str`-pattern).
2. The convention "0 warnings" is a promise to future contributors
   and to the README. If the promise is verified only locally, it
   can silently rot when CI's toolchain moves forward.
3. Fixing this is cheap: pin rustc in CI via `rust-toolchain.toml` or
   `dtolnay/rust-toolchain@stable` action; rerun the original v0.1.0
   commit to confirm the diagnosis if it ever recurs.

**Trade-off:** Pinning the toolchain is intentionally not done yet —
the user is on a moving-edge Windows toolchain and pinning would
silently break the install. Until pinning is chosen, every release
must include "CI clippy green on the current stable" as an explicit
check.

## D-019 · Roadmap under discipline (2026-09-05)

**Decision:** Adopt a 3-version roadmap shaped by what an external
review surfaced. Reject the temptation to do everything in v0.2.

### The full review (paraphrased)

A reviewer offered 27 recommendations across UX, product, architecture,
and community. They are useful. Many of them are also Featuritis
disguised as insight. The same reviewer closed with "do not add 30
features." That warning is the most important thing in the review.

### What we adopt now (v0.1.x polish)

- **README front-page hook** — first 10 seconds must answer "what is
  this and why would I want it", not "how do I install it".
  Single-page edit. Shipped in this commit.
- **examples/ kill-case library** — `001-flowforge.md` is live.
  Brand differentiation: naysay showcases what it **stopped**, not
  what it shipped. New cases get a three-digit number and the
  convention `idea → premortem → decision → what actually happened`.
  Single directory. Zero new dependencies.

### What we accept as future versions (do not start until the current
one is used)

- **v0.2 — structured decisions.** `assumption` / `evidence` /
  `unknown` / `confidence` / `kill criteria` / `success criteria`
  as optional sections inside existing commands' output. The CLI
  surface does not grow; the structured output becomes parseable
  by the user's tools. **No new UI.**
- **v0.3 — decision memory.** Local store (`.naysay/decisions/`)
  linking premortem → spec → postmortem by id. Query:
  "have we made this decision before?" and "what assumptions from
  older decisions are now invalid?" This is where the project
  starts paying for itself over years, not weeks.
- **v0.4 — agent integration.** MCP server, Git hook emitting
  decision records on commit, CI integration. The core thesis:
  agents must learn to consume a naysay decision artifact, not
  replace naysay.

### What we explicitly reject (would regress the project)

- "MCP server / GitHub App / VS Code / Codex / web UI / CI" as
  distinct frontends. The Decision Model is internal until v0.3
  proves it's worth sharing.
- "Build KILL DEFER VALIDATE UNKNOWN" as a richer verdict
  taxonomy in v0.2. Premortem already says BUILD-or-not in plain
  language; the new vocabulary is a UX cost without a user.
- "Decision Debt" terminology as a first-class concept. Add the
  example, not the vocabulary. Terms should follow observed
  patterns, not lead them.
- "Calibration" (track prediction accuracy, beat the no-tool
  baseline). This is the most ambitious suggestion and therefore
  the most dangerous — it is the right thing eventually, but
  doing it before we have decisions we believe in is putting
  the cart before the horse. Deferred to v0.5+ pending a real
  decision corpus.

### How we enforce this

Any new D-entry in this file proposing a feature in v0.2–v0.4 must
identify which rejected item (if any) it competes with. If the
proposal does not say what it displaces, it is by default
rejected.

## D-020 · v0.2 — Structured output, zero new surface (2026-09-05)

**Decision:** For v0.2, expand the existing commands' prompt templates
with structured sections (`ASSUMPTIONS / EVIDENCE / UNKNOWNS / CONFIDENCE`
in premortem; `Failure Conditions / Risk Budget` in spec;
`CALIBRATION` in postmortem). Do **not** add new CLI commands, new flags,
new files, new dependencies, or new types. CLI surface unchanged.

**Why:**
1. D-019 promised "v0.2 — structured decisions. The CLI surface does
   not grow; the structured output becomes parseable by the user's
   tools. No new UI." This commit is the literal execution of that
   promise. Anything that grows the CLI in v0.2 violates it.
2. The value of structured output is in the **contract** with the
   user, not in the implementation. A user who wants to grep
   `^CONFIDENCE` in a saved spec can do that today, with no
   schema, no parser, no new code.
3. The risk of v0.2 is the same as every other v0.x: premature
   abstraction. Adding a `Decision` type with serde derives, a TOML
   schema for prompts, an MCP server, all of these are easy to write
   and wrong to ship before anyone has actually parsed an
   `ASSUMPTIONS` block. Do the boring thing first.
4. The trade-off is real: the LLM may format things inconsistently.
   That is acceptable — the contract is "the LLM will try this
   format", not "the format is guaranteed". A user who needs
   machine-readable output is using `--json`, which already exists.

**Trade-off:** If the LLM produces malformed structured output,
downstream parsing breaks silently. The mitigation is in v0.3:
decision memory that re-validates each structured block on write.

### What v0.2 ships (in this commit)

- `premortem` output now ends with an `ASSUMPTIONS / EVIDENCE / UNKNOWNS /
  CONFIDENCE` block, in addition to the existing autopsy sections.
- `spec` output now includes `Assumptions / Failure Conditions / Risk
  Budget`, in addition to existing sections.
- `postmortem` output now ends with a `CALIBRATION` block.

### What v0.2 deliberately does not ship

- No new command, no new flag, no new file beyond
  `examples/002-*.md` and this decision log.
- No new dependency.
- No new type. The model is still called with `&[Message]`, returns
  `Result<String>`. Output is still a String with newlines.
- No change to the public JSON schema for `--json` (yet — that lands
  when at least three real users need it).

## D-021 · v0.3 — Decision Memory, again zero new surface (2026-09-05)

**Decision:** v0.3 adds a local decision store and three pure query
commands. No new flag, no new LLM-backed command, no new dependency,
no new user-facing surface beyond the store and the queries.

### Premortem (of v0.3 itself)

**Cause of death (most likely):** turning memory into a framework —
MCP server, vector embeddings, semantic search, GraphRAG. The first
reviewer already said "don't add 30 features" (D-019). Memory's
value is its existence, not its cleverness.

**Ranked killers:**
1. Schema-mania: forcing the LLM to emit JSON-shaped output that
   downstream code parses, in v0.3 itself rather than after v0.3 has
   proven the store earns its keep.
2. Repo-coupling: requiring `.naysay/` to live inside a git repo
   with a particular layout. The user might not have a git repo
   when first running.
3. State-on-disk bugs: silent corruption, partial writes, race
   conditions when two REPLs run in parallel. Premature robustness
   writes more code than the feature.

**Version that survived:** three query commands that read the
store (`d-by-id`, `d-link`, `d-unknowns`) plus auto-write on
`premortem` / `spec` / `postmortem` (8-char hex id, appended to
content). The store is a directory of JSON files under `.naysay/
decisions/` in the cwd; nothing more.

### What v0.3 ships

- `.naysay/decisions/` directory created on first save.
- `premortem` / `spec` / `postmortem` (when `--save` or no `--save`
  but running interactively, i.e. TUI) write a JSON record to the
  store with: id, timestamp, idea, full text, plus extracted
  fields (assumptions, evidence, unknowns, confidence, failure
  conditions, risk budget, calibration, predecessor id).
- Three pure-read commands: `d-by-id <id>` (print one record),
  `d-link <a> <b>` (show a→b chain of related decisions),
  `d-unknowns` (list every UNKNOWN/UNKNOWNS across all stored
  premortems — the "what we don't know" inventory).
- `d-` prefix aliases for the long names (parity with `seed` /
  `angles`, `drill` / `pros`).

### What v0.3 deliberately does not ship

- Vector search. Plain text grep is enough for thousands of records.
- Auto-link by content similarity. The user names the link when
  they have it; until then, "no link" is a correct answer.
- Git hooks / repo-side validation. The store is cwd-local; the
  user chooses when to commit it.
- Multi-store / sync / cloud. One cwd, one store.
- Schema-validated LLM output. The id + timestamp + raw text +
  extracted fields are stored as a single JSON file per record;
  nothing in the runtime parses the body.

### What this is not

v0.3 is the smallest change that makes naysay's decisions
**queryable** in a useful way. It is not a knowledge graph, not a
calibration engine, not a learning loop. Those come after at least
one real user has produced at least ten real records.

## D-022 · Interactive provider picker (v0.4.0) (2026-09-05)

**Decision:** First-run setup becomes an interactive picker: six provider
presets (Ollama, DeepSeek, GLM, OpenAI, MiniMax, OpenRouter) plus a
custom path. The choice is written to `naysay.toml`, the key goes to
the OS keyring, and the TUI launches against the chosen provider.

**Why:**
1. The old box assumed MiniMax and demanded a key before doing
   anything — even local Ollama users had to type a fake key, with no
   hint that was acceptable. A tool whose pitch is "interrogate ideas
   for fractions of a cent" was introducing itself with a signup wall.
2. Presets are **data, not abstraction** (the D-003 promise holds):
   each preset is the three fields naysay.toml already supports,
   pre-filled. A new provider is a new row in a const table, never
   new code. The "no provider abstraction layer" rule survives
   because there is still exactly one wire format and one code path.
3. **Claude honesty**: Anthropic's API is not OpenAI-compatible, so
   there is no Claude preset — the menu says so and points at
   OpenRouter (which is OpenAI-compatible and carries Claude models).
   Pretending otherwise would be the first lie in the docs.

**Implementation trap worth recording:** `config()` is a OnceLock. The
picker must run BEFORE the first `config()` call, or the frozen config
would ignore the just-written naysay.toml. Hence `probe_has_key()`
checks env + keyring directly and never touches config(); the picker
itself reads only statics. First `config()` call happens in tui::run,
after env vars (`api_key_env` = key) and the toml are both in place.

**Trade-offs:**
- Preset models drift (gpt-4o-mini will age). Accepted: the file is
  user-editable, and the picker's job is a working first call, not a
  current-events feed.
- Ollama users get a placeholder key ("ollama-local") stored to
  keyring so `load_api_key` passes; the server ignores it. Slightly
  impure, far better than a fake-key prompt.
- Smoke-testing the picker writes to the REAL `%LOCALAPPDATA%` unless
  the test overrides `LOCALAPPDATA`. Learned the hard way; future
  smoke tests must isolate the data dir.

## D-023 · v0.4 external review — the archive-to-memory turn (2026-09-05)

**Decision:** A second external review scored v0.4 at 8.0/10 and made
one criticism that outranks everything else in it: **Decision Memory is
currently a Decision Archive** — `by-id`/`link`/`unknowns` answer
"what did I store?", not "how do past decisions shape this one?". v0.5
therefore does ONE thematic thing: turn the archive toward memory.

### Adopted in v0.5 (this release)

1. **`decisions relevant <idea>`** — deterministic retrieval (token
   overlap, no LLM, no dependencies). The review's architectural
   boundary is adopted verbatim: *retrieval is deterministic;
   interpretation is the LLM's job*. v0.5 ships only the retrieval
   half; piping it into an LLM is a user exercise until v0.6.
2. **`--parent <ID>`** on premortem / spec / postmortem — decision
   revision lineage (DEC-001 → REVISIT → DEC-023) becomes writable.
   The `parent` field existed since v0.3 but nothing wrote it.
3. **`naysay calibration`** — the honest minimal version. premortem
   prompts now emit a structured `VERDICT: BUILD|DON'T BUILD` line;
   postmortem prompts emit `OUTCOME: BUILT|KILLED|ABANDONED|UNKNOWN`.
   The calibration command links premortems to their child
   postmortems and reports agreement. It prints an explicit caveat
   when there are fewer linked pairs than a statistic deserves.
4. **`src/store.rs`** — Decision leaves the CLI file. main.rs was
   ~3000 lines and climbing; the review proposed a guardrail
   (split at 4000/3000 LOC). We split **before** hitting it, because
   moving code after the boundary is refactoring, moving it before is
   hygiene.
5. **README "naysay's own decision record"** — the self-experiment is
   now public: every logged decision, the kill cases, the lineage.

### Deferred (logged, with the condition that re-opens them)

- **MODEL CONFIDENCE rename** (CONFIDENCE ≠ TRUTH): correct criticism,
  but the extraction protocol is stable and stored records already use
  CONFIDENCE. Renames happen when calibration output exists to show
  the *contrast* between model confidence and calibrated confidence —
  otherwise the rename renames a number nobody uses yet.
- **JSON schema / structured output API**: parser drift is real
  ("Confidence: high" vs "0.8"), but schema-locking the LLM output
  before the decision model stabilizes optimizes the wrong layer.
  Re-open when ≥3 real users parse the records programmatically.
- **Verdict taxonomy** (BUILD/KILL/DEFER/VALIDATE/REVISE): the premortem
  prompt already asks for "build it at what scope or don't" — the
  richer vocabulary lands when the calibration data shows the binary
  verdict is actually lossy.
- **`naysay check` (engineering-decision mode)**: the usage-frequency
  argument is the strongest point in the review, but it competes with
  the relevant-retrieval work for the same release. Logged as the
  first candidate for v0.6.
- **Full calibration dashboard**: needs a corpus. The v0.5 command
  prints what exists and says so.

### Rejected (unchanged from D-019)

MCP / web UI / cloud sync / team dashboards / plugins / agent
orchestration / vector DB / SaaS. The review agrees; noted twice now.

### The new guardrail (standing rule)

**main.rs ≤ 4000 LOC, tui.rs ≤ 3000 LOC.** Crossing either line stops
feature work and starts module extraction. src/store.rs is the first
execution of this rule.

## D-024 · Native rendering for wide chars + full line editing (v0.6.0) (2026-09-05)

**Decision:** Two user-visible defects fixed by changing HOW text reaches
the terminal, not what the text is:

1. **CJK gaps** ("仿 生 机 械 臂"): every wide char in the transcript and
   input row was followed by a printed blank. Root cause is in
   ratatui 0.29's inline path: `Buffer::set_stringn` resets the follower
   cell after a wide char, and `insert_before`'s `draw_lines` prints EVERY
   cell without the diff skip logic — the follower blank becomes a real
   printed space.
2. **No cursor movement**: input was append-only (`push` + `pop`).

**Fix:**
- The transcript and the input row now bypass ratatui's cell layer for
  text: ratatui reserves the space (`insert_before` with an empty
  buffer / draws only the ASCII prompt), then the pre-wrapped rows are
  printed as ONE contiguous crossterm `Print` per row. The terminal
  renders wide chars itself — no per-cell MoveTo, no follower cells,
  no gaps. Colors ride on spans via `SetForegroundColor`.
- Full line editing: `cursor: usize` (char index — CJK is one step),
  ←/→/Home/End/Delete, insertion at the cursor. The visible window
  pins to the cursor on overflow (`input_window`), cursor placement is
  display-width based.

**Trade-offs:**
- The transcript is written outside ratatui's buffers, so ratatui
  doesn't know it exists — acceptable: scrollback is append-only and
  the live region (2 rows) stays under ratatui's control.
- Status line keeps a few width-1 non-ASCII glyphs (`·`, `│`) which
  ratatui renders cell-perfect; only wide chars needed the bypass.
- Tests cover the pure helpers (`input_window`, cursor editing via
  `handle_key` simulation, wrap rows); the terminal output itself is
  verified by smoke runs, not unit tests.

## D-025 · Assumption lifecycle registry (v0.7) (2026-09-05)

**Decision:** Assumptions become first-class tracked entities with a
lifecycle (UNKNOWN → VALID/QUESTIONED/INVALIDATED), stored in
`.naysay/assumptions.json`, matched by normalized claim text. The
postmortem can flip statuses (via structured `ASSUMPTION VALID|INVALIDATED:`
lines in its output); `decisions assumptions` lists the registry with
unverified-age warnings; `decisions verify` flips manually.

**Why:** the second review's sharpest point — "Decision Memory is an
Archive" — is fixed not by smarter search but by giving the smallest
unit of a decision (the assumption) a lifecycle. "You are building on
an assumption first seen 19 days ago and never verified" is a sentence
no coding agent says today.

**Trade-offs:** claim matching is deterministic (normalized text), so
rephrased assumptions create separate entries — accepted, because fuzzy
matching hallucinate-merges distinct claims. Entries are created only
by premortem/spec (facts about what was assumed) and flipped only by
postmortems or explicit user command.

## D-026 · Decision memory injection into prompts (v0.7) (2026-09-05)

**Decision:** premortem and spec prompts now carry a `DECISION MEMORY`
block — prior verdicts on similar ideas (deterministic Jaccard
retrieval, top 3) plus tracked-assumption risk lines — and MEMORY RULES
that make the model treat prior verdicts as facts: a prior DON'T BUILD
on the same idea must be either justified by a material change or
repeated. postmortem prompts carry the parent premortem's assumptions
with instructions to emit structured status updates.

**Why:** this is the review's S1/S2/S4 made concrete. S1: history can
change the verdict because the model is instructed to treat a prior
DON'T BUILD as binding unless something material changed. S2: conflicts
surface with what/why/what-would-resolve structure. S4: a postmortem's
status flips are visible to the next premortem through the registry.

**Trade-offs:** the injected block costs tokens (bounded: top 3 records
+ top 5 assumptions). The memory rules bias the model toward repeating
prior kills — deliberate: the burden of proof belongs to the builder.
