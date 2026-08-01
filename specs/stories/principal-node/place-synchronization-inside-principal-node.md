---
id: place-synchronization-inside-principal-node
type: story
refs:
  journey: specs/journeys/evolve-a-principal-node.md
  persona: specs/personas/domain-architect.md
  steps: [1, 2, 4, 6]
---

# Story: Place synchronization inside the Principal Node

## Narrative

As a domain architect,
I want synchronization to be one Principal-Node-owned transition procedure,
So that host scheduling and transport choices cannot redefine continuity.

## Acceptance Criteria

- [ ] The Principal Node owns synchronization lifecycle, stream
  selection, agreement evaluation, cursor use, retry classification, and
  completion evidence.
- [ ] Startup, journal commits, peer wakes, recovery ticks, and operator
  requests enter the same synchronization procedure.
- [ ] Replacing or restarting a Node Runtime Instance does not create a new
  cursor lineage or synchronization policy.
- [ ] A host signal conveys only that the node may evaluate work; it carries no
  source identity, stream authority, cursor, or policy decision.
- [ ] Every synchronization attempt re-evaluates current declaration heads and
  authenticated transport evidence.
- [ ] Exact signed event envelopes cross the replication boundary unchanged.
- [ ] A source-bound cursor advances only after every covered record has a
  durable checkpoint-safe outcome.
- [ ] A failed, rejected, or ambiguous record cannot create a permanent gap.
- [ ] Restart resumes from Principal-Node-owned durable cursor state without
  consulting Runtime Instance memory or a host scheduler's history.
- [ ] The portable relay core remains unaware of scheduling, retry cadence, and
  host lifecycle.

## Notes

The current pull/push binaries and host jobs may remain as compatibility
adapters during migration, but neither they nor a Runtime Instance are the
identity or policy owner of the procedure.
