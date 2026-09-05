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
