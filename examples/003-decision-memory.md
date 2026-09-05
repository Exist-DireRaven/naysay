# 003 — naysay's own decision memory (v0.3)

> **Decision: SHIP.** The smallest change that makes naysay's decisions
> queryable across sessions.

## The idea

After v0.2 taught the prompts to emit structured sections
(ASSUMPTIONS / EVIDENCE / UNKNOWNS / CONFIDENCE / Failure Conditions /
Risk Budget / CALIBRATION), the output still evaporated: a user who
saved a premortem to a file had to remember where they put it, and
"what did I say I didn't know last month?" had no answer.

## What naysay said

> **Cause of death (most likely):** turning memory into a framework —
> vector search, MCP server, graph store. The first external review
> already warned against it, and the D-019 rejection list exists
> precisely for this moment.
>
> **The version that survived:** a directory of JSON files under
> `.naysay/decisions/` in the working directory, three pure-read query
> commands (`decisions by-id|link|unknowns`), and best-effort
> auto-save in the three verdict commands. No embeddings. No server.
> No schema enforcement. Grep is the API.

## The verdict

Ship it. Memory's value is its existence, not its cleverness.

## What actually happened

Shipped as v0.3.0. The store is 6 new functions and ~200 lines,
all in `main.rs`:

- `save_decision` auto-fires on every premortem/spec/postmortem
  (failure is printed to stderr and never breaks the command).
- `decisions by-id <id>` prints the raw record.
- `decisions unknowns` walks every stored premortem and prints the
  UNKNOWNS inventory — the "what we don't know" list across all
  decisions ever made in this directory.
- `decisions link <id>` walks the parent chain and prints the
  decision lineage as a tree.

The design test: could a user get value on day one with zero LLM
calls? `decisions unknowns` on an empty store prints "(no unknowns
recorded)" and exits 0 — no API key, no network, no setup.

## What this case teaches

- The first version of memory should be so boring it embarrasses you.
- A store you can `grep` is a store you can trust.
- If the query commands need a network call, the design is wrong.

---

*This file is part of the codebase. New kill cases get their own
three-digit number. Add to `examples/README.md` when you do.*
