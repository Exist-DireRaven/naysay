# 002 — Realtime AI calendar (KILLED)

> **Decision: KILL.** Cost of building > expected value at 2 weeks.

## The idea

A calendar app that auto-schedules your day from a continuous stream
of inbox messages, Slack pings, and GitHub notifications. Targets
people whose job is "keeping many small commitments from falling
through the cracks." AI would triage incoming noise into the calendar
as tentative blocks; one click accepts, one click rejects, two clicks
overrides. Premium tier: $20/month for cross-account inbox integration.

## What naysay said

> **Cause of death:** the product targets a pain that is *the user's
> identity*, not a job. Anyone whose job is "keep many small things
> from falling through the cracks" already has a system that works
> for them — usually three of them at once. An AI calendar adds a
> fourth. The migration cost is not software; it is admitting that
> the existing system is wrong, which the user will not.
>
> **Ranked killers:**
> 1. Distribution: the user is locked to a calendar app already.
>    Switching cost is measured in missed meetings, not features.
> 2. Trust: the AI must triage *correctly* on day one. "I moved your
>    investor call to next Tuesday" is a non-recoverable error.
> 3. Pricing: $20/month is a hard cap. The people who would pay
>    $20/month are the people whose existing system already works.
> 4. The pain is real but the user has a coping mechanism that
>    produces no audible signal of failure. They will not switch.

## The verdict

Don't build. The 2-week MVP will pass every internal demo and fail
every cold-start because the people in the demos were the
implementers' friends, not the target user.

## What actually happened

After the premortem, the implementer built a 1-day CLI prototype
(calendar.txt + cron). It worked for them, personally, for a week.
They never built the second iteration. Two months later they
discovered the same workflow was being served by *existing* keyboard
shortcuts in their email client.

The product was never built. No time, money, or attention was lost
beyond the 1-day prototype.

## What this case teaches

- "I would use this" is not the same as "I would pay for this."
- A working prototype is not a market.
- The best outcomes of a premortem are the projects that never start.

---

*This file is part of the codebase. New kill cases get their own
three-digit number. Add to `examples/README.md` when you do.*
