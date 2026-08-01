---
id: promote-compatible-node-runtime
type: story
refs:
  journey: specs/journeys/evolve-a-sovereign-node-runtime.md
  persona: specs/personas/domain-architect.md
  steps: [5, 6, 7]
---

# Story: Promote a compatible node runtime

## Narrative

As a domain architect,
I want a signed runtime release to declare the state and adapter contracts it
can safely continue,
So that installation, migration, rollback, and resurrection are verifiable
transitions rather than host folklore.

## Acceptance Criteria

- [ ] A release identifies its source revision, runtime version, supported
  profile and state schemas, and required host capability profile.
- [ ] Verification establishes release provenance and byte integrity before the
  runtime is selected.
- [ ] Compatibility is checked against the node context and host capabilities
  before the new runtime mutates durable state.
- [ ] A migration has an explicit precondition, postcondition, recovery point,
  and irreversible boundary if one exists.
- [ ] Rollback changes executable selection but never rolls the journal or a
  committed replication cursor backward implicitly.
- [ ] The running runtime can report evidence that matches the selected release
  manifest.
- [ ] A resurrection drill verifies the same compatibility claims on a fresh
  host adapter.
- [ ] Private keys, journals, cursors, and mutable host state are not embedded
  in the release artifact.

## Notes

Release signatures attest provenance and integrity. They do not authorize
journal transitions or replace replay verification.
