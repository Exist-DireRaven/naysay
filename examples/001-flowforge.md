# 001 — FlowForge (the predecessor that became naysay)

> **Decision: KILL.** Replaced with a 3400-line single binary that does
> one thing well.

## The idea

Build a complete visual workflow automation platform for non-technical
users: drag-and-drop node editor, 30+ node types, Tauri desktop app,
WASM plugin sandbox, self-bootstrap installer, AI node generator,
marketplace, cloud sync. The plan was 16 chapters and ~6 months.

## What naysay said

> **Cause of death (the most likely):** the WASM plugin self-bootstrap
> system was built before any users existed. Day-zero signal: you find
> yourself explaining *why plugins are needed*, not what users can do.
>
> **Ranked killers:**
> 1. Scope explosion. 16 chapters designed, 6 chapters implementable.
> 2. Agent integration nobody asked for. The home-grown ZCode is the
>    real competitor; the WASM abstraction is invisible to it.
> 3. Marketing site shipped before MVP — usually a sign the team gave
>    up on the MVP.

## The verdict

Don't build as originally planned. Build the smallest version above,
run it for two weeks, see if real intent emerges.

## What actually happened

The smallest version was 3400 lines of Rust with 6 commands
(`seed / drill / premortem / spec / postmortem / explain`) and a
single terminal binary. The marketing site was deleted. The Tauri
desktop, WASM plugin sandbox, and self-bootstrap installer were never
touched. The project was renamed to `naysay` and repositioned:
instead of building a workflow platform, it interrogates the
decisions that would lead to one.

Two years later, the surviving version is the one naysay is now.

## What this case teaches

- The largest cost in a project is the parts nobody ends up using.
- "I might need extensibility later" is the most expensive sentence
  in software.
- Renaming the project is cheap. Killing a 6-month plan is expensive.
  Do the cheap thing first.

---

*This file is part of the codebase. New kill cases get their own
three-digit number. Add to `examples/README.md` when you do.*
