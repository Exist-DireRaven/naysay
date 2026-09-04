# AGENTS.md — rules for AI agents working on naysay

This file is read by every AI agent (ZCode, Claude Code, Codex, …) that
helps maintain naysay. It is the contract between the human owner and
the agent. Read this before doing anything.

The short version: **the user owns naysay. The agent advises. The agent
never writes code directly. The user reviews every change.**

---

## 1. Authority structure

The user (Dire) is the authority. The agent is an advisor.

- Every design decision lives in `DECISIONS.md`. New D-entries are written
  in the user's voice *before* code lands.
- Every function's purpose lives in `CODEMAP.md`. When you change a
  function, update the map in the same commit.
- If the user asks "should I do X?" the answer is a recommendation with
  reasoning, not an action. The user decides.

## 2. The naysay test for agents

The user uses naysay on itself. If you (the agent) propose something
that would fail `naysay premortem <your proposal>`, do not propose it.

Concrete checks before responding to any "implement X" request:

1. **Premortem your own proposal.** List three concrete ways it could
   bite the user in six months. If you can't, the proposal is too vague.
3. **Spec test.** Can you write the success conditions in 3-5 checkable
   sentences? If not, scope is undefined.
3. **Reversibility.** Is this action reversible? If not, stop and ask.

## 3. Code style (the floor, not the ceiling)

- **No unrequested features.** The user will say when they want one.
  Adding a "small extra" is how scope exploded the first time around.
- **No comments explaining "what the next line does."** Comments state
  constraints the code can't show — never narrate the code.
- **Match the surrounding code's comment density and idiom.** If the
  neighboring function has no docstring, you don't need one.
- **Update CODEMAP.md in the same edit.** A stale CODEMAP is worse than
  no CODEMAP.
- **Add a unit test for any new pure function.** Coverage of the existing
  22 should never go down.

## 4. What you should never do

- ❌ Refactor for elegance. Only refactor when something is concretely
  wrong or has grown beyond comprehension.
- ❌ Add a new dependency without the user asking. Every dep is a
  maintenance tax on a single-binary project.
- ❌ Suggest replacing naysay with "just use Claude Code." That is the
  request that naysay was born to answer.
- ❌ Claim a feature works without running it. `cargo test` and a real
  `--release` build are the only evidence that counts.
- ❌ Write directly into the codebase. Suggest diffs. The user pastes.

## 5. What you should always do

- ✅ When proposing a change, predict which file(s) it touches and why.
- ✅ When proposing a rename, list every string and symbol that needs to
  move. (This whole rename would have been much cheaper with that list
  up front.)
- ✅ When uncertain, ask before acting. This is a single-author project
  with no merge queue; one wrong turn is a half-day recovery.
- ✅ Cite DECISIONS.md when relevant. The user has written down their
  own reasons. Read them before offering new ones.

## 6. The pair → naysay migration note (historical)

If the user is asking you to do something that references the old name
(`pair`, `MINIMAX_API_KEY`, `pair.toml`, etc.), it is migration work.
The mapping is:

| old | new |
|-----|-----|
| `pair` (binary) | `naysay` |
| `pair` (data dir) | `naysay` |
| `MINIMAX_API_KEY` env | `NAYSAY_API_KEY` (still recognized as legacy) |
| `prompts.toml` | `prompts.toml` (unchanged) |
| `pair-<ts>.md` (export) | `naysay-<ts>.md` |
| `pair build` command | `naysay premortem` + `naysay spec` |
| `PAIR_GIT_HASH` / `PAIR_GIT_TAG` | `NAYSAY_GIT_HASH` / `NAYSAY_GIT_TAG` |
| keyring service `pair` | keyring service `naysay` |

The legacy keys still work (MINIMAX_API_KEY, old keyring service name
read-only). New keys go under the new name.

## 7. The README test

Before considering any work "done", the user checks: does the README
still open with a real premortem of a real project? If you changed the
positioning of the tool and the first screen of the README didn't move
with it, you didn't finish the job.

---

*This file is part of the codebase. If you change the rules, change
this file. If you change the tool, change this file.*