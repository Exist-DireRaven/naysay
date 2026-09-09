//! The verdict prompt templates, in one place.
//!
//! The CLI/REPL path and the TUI path used to carry separate copies of these
//! four prompts, and they drifted: the TUI's premortem stopped at section 5
//! and never emitted `ASSUMPTIONS` / `VERDICT:`, so a record written from the
//! default surface could not feed the assumption registry (D-031). One text
//! per command, read by both surfaces (D-035, milestone 1).
//!
//! `{idea}` is the only placeholder. `prompts.toml` can still override any of
//! these by key — see `Prompts::get`.

pub(crate) const PREMORTEM: &str = "\
The user is about to commit to building this: {idea}\n\n\
Run the premortem. It is six months in the future and this project is dead — \
abandoned, unmaintained, or alive but ignored. Write the autopsy:\n\n\
1. Cause of death — the single most likely killer, stated bluntly.\n\
2. Ranked killers — 3-5 probable causes of death, each with the early warning \
sign that was already visible on day one.\n\
3. Scope autopsy — which imagined features were never touched, and which \
single feature everything actually depended on.\n\
4. The version that survived — the smallest cut of this idea that dodges \
every cause of death above.\n\
5. Verdict — build it (at what scope) or don't (and what to do instead). \
Then end the whole output with a final line of exactly `VERDICT: BUILD` or \
`VERDICT: DON'T BUILD` — no other words on that line.\n\n\
After the autopsy, add a short STRUCTURED section:\n\n\
ASSUMPTIONS — 3-5 things the build depends on being true. Be specific \
('a person will run this 3x/week', not 'people will want this'). If you \
cannot name the assumption, name why you can't.\n\n\
EVIDENCE — for each assumption: what would prove it true? what would prove it \
false? Use only known data; if you have none, say 'none yet' rather than \
inventing.\n\n\
UNKNOWNS — 2-4 things that, if they turned out a certain way, would flip the \
verdict. Be specific about the direction of the flip.\n\n\
CONFIDENCE — a number 0..1 for the verdict itself. 0.5 means you would change \
your mind for a free coffee. 0.9 means you would bet money on it. Pick a \
number; do not say 'medium'.\n\n\
Be specific to this idea. Generic startup advice is worthless here.";

pub(crate) const CHECK: &str = "\
The user is about to make this engineering decision: {idea}\n\n\
Run the pre-existence check. This decision is about to become code, a \
dependency, or an abstraction — interrogate it before it exists:\n\n\
1. The actual problem — one sentence: what need does this serve? If the \
decision is a solution looking for a problem, say so here.\n\
2. Existing coverage — what already does this: in this repo, in the \
dependencies already installed, or in a tool already on PATH? Name it. If \
something covers most of it, say how much.\n\
3. Minimum form — the smallest version that satisfies the need. What could be \
deleted from the proposal and still work?\n\
4. Six-month failure mode — how this becomes maintenance debt: the dependency \
that rots, the abstraction nobody calls, the code the next agent has to read.\n\
5. Verdict — build it, reuse what exists, or don't build at all. Then end the \
whole output with a final line of exactly `VERDICT: BUILD` or \
`VERDICT: DON'T BUILD` — no other words on that line.\n\n\
After the check, add a short ASSUMPTIONS section: 3-5 things this decision \
depends on being true, each specific enough to be checkable.\n\n\
Be specific to this decision. Generic engineering advice is worthless here.";

pub(crate) const SPEC: &str = "\
The user wants to hand this to a coding agent to execute: {idea}\n\n\
Write the spec the agent will receive. Assume the agent is capable but has \
zero context, and will take the path of least resistance wherever the spec is \
vague. Sections:\n\n\
# Goal — one paragraph: what exists when this is done, and for whom.\n\
# Non-goals — what this is NOT. Anything unlisted here, the agent will build \
on a whim.\n\
# Assumptions — 2-4 things the build depends on being true. Be specific; \
'users will want this' is not an assumption, it is a hope.\n\
# Success criteria — 3-5 concrete, checkable conditions.\n\
# Failure conditions — 2-4 conditions under which the build is considered \
failed regardless of whether it runs. A failure condition is a deal-breaker; \
not a bug list.\n\
# Risk budget — the worst case the user is willing to absorb (e.g. \
'1 weekend of my time, $20 of infra, then kill').\n\
# Constraints — language, platform, budget, things that must not change.\n\
# Milestones — ordered; each one independently runnable or checkable.\n\
# Open questions — what the user must decide; the agent should ask, not \
guess.\n\n\
Be concrete. A vague spec means the agent improvises, and improvisation is \
where rework is born.";

pub(crate) const POSTMORTEM: &str = "\
The project \"{idea}\" is over — shipped, killed, or quietly abandoned.\n\n\
{notes}\n\
Write the postmortem:\n\n\
1. What actually happened — one paragraph, stated plainly. If you are \
guessing, say so explicitly.\n\
2. Predicted vs actual — which failure modes (or successes) were foreseeable \
on day one? Name them as if a premortem had been run before the first \
commit.\n\
3. The decisive moment — the single decision that determined the outcome. \
What was the alternative at that fork?\n\
4. Cost accounting — time, money, and attention spent vs value extracted. \
Use only numbers the user gave; otherwise name the numbers you need from \
them.\n\
5. Decision-log entry — 3-5 lines in markdown, self-contained, written to \
paste into a DECISIONS.md: what was tried, what happened, what to do \
differently next time.\n\n\
After the postmortem, add a short CALIBRATION section. Its first line must be \
exactly one of: `OUTCOME: BUILT`, `OUTCOME: KILLED`, `OUTCOME: ABANDONED`, \
`OUTCOME: UNKNOWN`.\n\n\
CALIBRATION — if a premortem was run for this project, did the verdict hold? \
If you said BUILD and it was killed, or KILL and it shipped, that is the most \
useful sentence in this whole document. One paragraph: what was the original \
confidence, what actually happened, and what the gap teaches about the \
premortem process itself. If no premortem exists, name two things you would \
have warned against that the project proved right about.\n\n\
Be specific to this project. Blame decisions, not people.";

#[cfg(test)]
mod tests {
    use super::*;

    /// D-031's drift was a missing section, not a missing template: the TUI's
    /// premortem stopped before `ASSUMPTIONS` / `VERDICT:`, so its records
    /// could not feed the registry. These markers are the extractors'
    /// contract, and one text now serves both surfaces.
    #[test]
    fn verdict_templates_carry_the_extractable_markers() {
        assert!(PREMORTEM.contains("VERDICT: BUILD") && PREMORTEM.contains("ASSUMPTIONS"));
        assert!(PREMORTEM.contains("CONFIDENCE"));
        assert!(CHECK.contains("VERDICT: BUILD") && CHECK.contains("ASSUMPTIONS"));
        assert!(SPEC.contains("# Failure conditions") && SPEC.contains("# Risk budget"));
        assert!(POSTMORTEM.contains("OUTCOME: BUILT"));
        assert!(
            POSTMORTEM.contains("{notes}"),
            "postmortem needs its notes slot"
        );
        for t in [PREMORTEM, CHECK, SPEC, POSTMORTEM] {
            assert!(t.contains("{idea}"), "template lost its {{idea}} slot");
        }
    }
}
