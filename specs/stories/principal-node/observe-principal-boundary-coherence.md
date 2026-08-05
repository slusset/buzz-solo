---
id: observe-principal-boundary-coherence
type: story
refs:
  journey: specs/journeys/evolve-a-principal-node.md
  persona: specs/personas/domain-architect.md
  steps: [2, 3, 4, 6, 7]
---

# Story: Observe Principal Domain and Node coherence

## Narrative

As a domain architect,
I want boundary decisions to produce independently inspectable coherence
observations,
So that a running process cannot appear healthy while domain authorization,
Principal Node identity, host binding, release, or durable state disagree.

## Acceptance Criteria

- [ ] Coherence is reported as a set of named invariant observations, not one
  weighted score.
- [ ] Every observation is `ok`, `drift`, `violation`, or `unknown` and names
  its subject, observer revision, and bounded evidence references.
- [ ] A critical violation or unknown remains visible even when every other
  invariant is `ok`.
- [ ] Principal-boundary coherence detects independently supervised domain jobs,
  host-owned cursor mutation, or a live component graph that contradicts the
  declared Principal Node boundary.
- [ ] Domain-node coherence verifies that the Principal Node has current
  authorization from exactly one Principal Domain.
- [ ] Runtime-instance coherence verifies that an executing process matches the
  Principal Node's selected release and host binding without becoming its
  identity.
- [ ] Host-capability coherence compares node requirements with the active
  signed host capability claim and PrincipalNode binding.
- [ ] Release coherence compares the running runtime and adapter revisions with
  verified release evidence without treating the release as journal authority.
- [ ] Observation reads never repair, reconcile, advance cursors, or alter the
  state being measured.
- [ ] Recording an observation is a separate metadata-only append that contains
  no payload copies, credentials, or host-private identifiers.
- [ ] Merge-time, runtime, and resurrection-time observations use the same
  invariant identifiers where they make the same claim.

## Notes

Coherence is evidence about agreement among identities, authority, and
artifacts. It is not a health percentage and it is not an automatic repair
mechanism.
