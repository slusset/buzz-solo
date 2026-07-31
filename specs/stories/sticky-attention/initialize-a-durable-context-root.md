---
id: initialize-a-durable-context-root
type: story
refs:
  journey: specs/journeys/steward-a-domain-context.md
  persona: specs/personas/domain-steward.md
  steps: [1, 2]
---

# Story: Initialize a durable context root

## Narrative

As a domain steward,
I want to initialize a context root with one command and link existing
repositories into it,
So that the domain has a canonical on-disk and journal home without moving
or copying any code.

## Acceptance Criteria

- [ ] `init` creates `.context/context.yaml` (identity, h boundary,
  disclosure policy, checkpoint policy) and a charter stub; identity is
  minted once and never re-derived from the path.
- [ ] After init, `artifacts.yaml` and `current-work.yaml` are generated
  projections of journal heads; on divergence the journal wins.
- [ ] `link-repo` records a symlink plus target metadata; inventory hashes
  the target string and never traverses the link.
- [ ] A session opened inside the root or a linked repository discovers
  this context; ambiguous membership fails closed.
- [ ] Discovery honors `${DURABLE_CONTEXT_HOME:-~/DurableContext}`.
- [ ] Nothing in init or linking publishes, syncs, or replicates.

## Notes

Migrating an existing hand-authored root (the fielded VH pattern) is init
over a populated directory: authored `context.yaml` is respected;
generated files are regenerated.
