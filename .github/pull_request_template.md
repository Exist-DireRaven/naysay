<!--
  House rules (CONTRIBUTING.md):
  1. Run the premortem on your own PR before opening it.
  2. Non-obvious design choices ship with a DECISIONS.md entry.
-->

## What does this PR do?

<!-- One paragraph. -->

## Related issue

<!-- "Fixes #N" or "None" -->

## Premortem (self-review)

<!-- Run: naysay premortem "<what this PR does>" — paste the verdict line
     (section 5) or summarize it. What's the most likely way this bites us
     in six months? -->

## Checklist

- [ ] `cargo fmt` applied
- [ ] `cargo clippy` — zero warnings
- [ ] `cargo test` — all green, test count not lower
- [ ] CODEMAP.md updated if functions changed
- [ ] DECISIONS.md entry added for any non-obvious choice
- [ ] CHANGELOG.md entry added (Unreleased section)
