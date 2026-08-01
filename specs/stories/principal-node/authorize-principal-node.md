---
id: authorize-principal-node
type: story
refs:
  journey: specs/journeys/evolve-a-principal-node.md
  persona: specs/personas/domain-architect.md
  steps: [2, 3, 4]
---

# Story: Authorize a Principal Node

## Narrative

As a domain architect,
I want a Principal Domain to authorize each Principal Node explicitly,
So that filesystem access, a running process, or possession of a transport key
cannot silently become authority to represent the domain.

## Acceptance Criteria

- [ ] A Principal Domain has a stable identifier established by its root
  declaration and distinct from the current domain-root verification key.
- [ ] Rotating the domain-root verification key preserves Principal Domain
  identity and historical signatures.
- [ ] A Principal Node has a stable identifier distinct from host, process,
  release, event-author, and transport identities.
- [ ] A current domain-root authority signs the authorization binding one
  Principal Node to exactly one Principal Domain and a declared scope.
- [ ] One Principal Domain may authorize multiple Principal Nodes over time or
  topology without merging their node-local cursors or host bindings.
- [ ] Revocation prevents the Principal Node from initiating new domain
  operations while preserving prior events, receipts, and checkpoints.
- [ ] A Node Runtime Instance may exercise only the authority of the Principal
  Node whose verified release and host binding it executes.
- [ ] Possessing the domain-root, node, transport, or host key never implies
  any of the other identity roles.

## Notes

The existing “root node key” becomes the current domain-root verification key.
It authorizes the Principal Domain but is not the permanent domain identifier.
