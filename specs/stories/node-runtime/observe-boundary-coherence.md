---
id: observe-boundary-coherence
type: story
refs:
  journey: specs/journeys/evolve-a-sovereign-node-runtime.md
  persona: specs/personas/domain-architect.md
  steps: [2, 3, 5, 6]
---

# Story: Observe node-boundary coherence

## Narrative

As a domain architect,
I want boundary decisions to produce independently inspectable coherence
observations,
So that a running system cannot appear healthy while intent, code, host
binding, release, or durable state disagree.

## Acceptance Criteria

- [ ] Coherence is reported as a set of named invariant observations, not one
  weighted score.
- [ ] Every observation is `ok`, `drift`, `violation`, or `unknown` and names
  its subject, observer revision, and bounded evidence references.
- [ ] A critical violation or unknown remains visible even when every other
  invariant is `ok`.
- [ ] Runtime-boundary coherence detects independently supervised domain jobs,
  host-owned cursor mutation, or a live component graph that contradicts the
  declared node-runtime boundary.
- [ ] Host-capability coherence compares node requirements with the active
  signed host capability manifest.
- [ ] Release coherence compares the running runtime and adapter revisions with
  verified release evidence without treating the release as journal authority.
- [ ] Observation reads never repair, reconcile, advance cursors, or alter the
  state being measured.
- [ ] Recording an observation is a separate metadata-only append that contains
  no payload copies, credentials, or host-private identifiers.
- [ ] Merge-time, runtime, and resurrection-time observations use the same
  invariant identifiers where they make the same claim.

## Notes

Coherence is evidence about agreement among artifacts. It is not a health
percentage and it is not an automatic repair mechanism.
