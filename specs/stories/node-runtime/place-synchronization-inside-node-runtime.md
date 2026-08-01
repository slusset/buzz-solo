---
id: place-synchronization-inside-node-runtime
type: story
refs:
  journey: specs/journeys/evolve-a-sovereign-node-runtime.md
  persona: specs/personas/domain-architect.md
  steps: [1, 2, 3, 5]
---

# Story: Place synchronization inside the node runtime

## Narrative

As a domain architect,
I want synchronization to be one node-owned state transition procedure,
So that host scheduling and transport choices cannot redefine continuity.

## Acceptance Criteria

- [ ] The sovereign node runtime owns synchronization lifecycle, stream
  selection, agreement evaluation, cursor use, retry classification, and
  completion evidence.
- [ ] Startup, journal commits, peer wakes, recovery ticks, and operator
  requests enter the same synchronization procedure.
- [ ] A host signal conveys only that the node may evaluate work; it carries no
  source identity, stream authority, cursor, or policy decision.
- [ ] Every synchronization attempt re-evaluates current declaration heads and
  authenticated transport evidence.
- [ ] Exact signed event envelopes cross the replication boundary unchanged.
- [ ] A source-bound cursor advances only after every covered record has a
  durable checkpoint-safe outcome.
- [ ] A failed, rejected, or ambiguous record cannot create a permanent gap.
- [ ] Restart resumes from node-owned durable cursor state without consulting a
  host scheduler's history.
- [ ] The portable relay core remains unaware of scheduling, retry cadence, and
  host lifecycle.

## Notes

The current pull/push binaries and host jobs may remain as compatibility
adapters during migration, but they are not the target owner of the procedure.
