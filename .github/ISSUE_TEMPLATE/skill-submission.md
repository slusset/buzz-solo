---
name: Skill submission
about: Flow a skill discovered or authored on a satellite node upstream
labels: skill
---

**Skill name**
Short kebab-case name.

**Authored on**
Which node/host created it? (e.g. Vanderbilt Health, `~/DurableContext`)

**What it does**
One-paragraph description: trigger, behavior, output.

**Skill content**
Paste the full `SKILL.md` (and any auxiliary files) in fenced blocks, or
link a branch/PR adding it under `skills/<name>/`.

**Host assumptions**
Anything host-specific it relies on (paths, tools, credentials by
*reference* only — never material). Portable skills should declare
assumptions so other nodes can adapt them.

**Verification**
How was it exercised on the originating host?

---

Intake per `specs/architecture/skill-distribution-loop-v0.1.md`: an issue is
the submission; a maintainer (or the submitting agent, once authorized)
turns it into a PR adding `skills/<name>/SKILL.md`; merged skills are
distributed by nodes pulling `main`.
