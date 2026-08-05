---
id: declare-host-capabilities-without-domain-authority
type: story
refs:
  journey: specs/journeys/evolve-a-principal-node.md
  persona: specs/personas/domain-architect.md
  steps: [3, 4, 5, 6]
---

# Story: Declare host capabilities without domain authority

## Narrative

As a domain architect,
I want each host adapter to declare the mechanical capabilities it provides,
So that a Principal Node can move without trusting the host to make domain
decisions.

## Acceptance Criteria

- [ ] A host capability claim declares supervision, placement, custody,
  clock/wake, session, and attestation capabilities.
- [ ] The host attestation identity signs the claim, and an authorized
  Principal Node explicitly binds it for a bounded purpose and validity
  interval before use.
- [ ] Host labels, paths, service names, credential identifiers, and timer
  mechanisms never become node identity or journal authority.
- [ ] A host adapter can start, stop, wake, place, and surface the node only
  through declared ports.
- [ ] A host adapter cannot select replication streams, interpret cursors,
  broaden grants, reconcile findings, or append domain events on its own
  authority.
- [ ] Missing required capabilities cause an explicit blocked or degraded
  Principal Node state rather than an implicit fallback.
- [ ] Capability declarations contain references and properties, never private
  key material or reusable credentials.
- [ ] A foreground adapter can satisfy the minimum profile without launchd or
  systemd.
- [ ] Two conforming host adapters produce the same node-visible outcomes for
  the capabilities they both declare.

## Notes

Host capabilities make OS differences explicit. They do not make the host a
Principal Domain, Principal Node, owner, policy author, or event author.
