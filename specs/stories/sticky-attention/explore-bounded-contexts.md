---
id: explore-bounded-contexts
type: story
refs:
  journey: specs/journeys/steward-a-domain-context.md
  persona: specs/personas/domain-steward.md
  steps: [5]
---

# Story: Explore bounded contexts like a filesystem

## Narrative

As a domain steward,
I want to browse everything the node knows — all contexts, then any one in
depth — as a navigable tree,
So that visibility is looking around, not composing queries.

## Acceptance Criteria

- [ ] `explore` with no argument lists every bounded context ordered by
  warmth, with freshness, open-loop count, and active sessions inline.
- [ ] `explore <context|root>` expands one context: heads, current-work,
  open loops, sessions (active/interrupted), linked repositories,
  artifact manifest, declared skills, beacon.
- [ ] The projection is deterministic: same events, same tree, on every
  surface; nothing is stored as authority.
- [ ] Exploration is read-only — it never creates residue, moves the
  attention cursor, or replaces a head.
- [ ] Disclosure policy applies: the explorer never renders content the
  context policy withholds from the requesting surface.
- [ ] Works from a context root, a linked repository, or anywhere (with
  explicit argument).

## Notes

A TUI and a read-only virtual-filesystem mount are later phases; the CLI
tree is the contract.
