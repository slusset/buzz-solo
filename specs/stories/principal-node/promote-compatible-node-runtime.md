---
id: promote-compatible-node-runtime
type: story
refs:
  journey: specs/journeys/evolve-a-principal-node.md
  persona: specs/personas/domain-architect.md
  steps: [6, 7, 8]
---

# Story: Promote a compatible node runtime

## Narrative

As a domain architect,
I want a signed runtime release to declare which Principal Node state and
adapter contracts it can safely continue,
So that selecting a new Runtime Instance is a verifiable transition rather
than host folklore.

## Acceptance Criteria

- [ ] A release identifies its source revision, runtime version, supported
  profile and state schemas, and required host capability profile.
- [ ] Verification establishes release provenance and byte integrity before the
  runtime is selected.
- [ ] Compatibility is checked against the Principal Domain, Principal Node
  continuity state, and bound host capabilities before the new Runtime
  Instance mutates durable state.
- [ ] A migration has an explicit precondition, postcondition, recovery point,
  and irreversible boundary if one exists.
- [ ] Rollback changes executable selection but never rolls the journal or a
  committed replication cursor backward implicitly.
- [ ] The Runtime Instance can report evidence that matches the Principal
  Node's selected release manifest.
- [ ] A resurrection drill verifies the same compatibility claims on a fresh
  host adapter.
- [ ] Private keys, journals, cursors, and mutable host state are not embedded
  in the release artifact.

## Notes

Release signatures attest provenance and integrity. Selecting a release does
not authorize a Principal Node, journal transition, or replay result.
