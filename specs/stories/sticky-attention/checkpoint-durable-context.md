---
id: checkpoint-durable-context
type: story
refs:
  journey: specs/journeys/maintain-a-durable-context-through-agent-sessions.md
  persona: specs/personas/sovereign-builder.md
  steps: [4]
---

# Story: Checkpoint a durable context safely

## Narrative

As a sovereign builder,
I want explicit context checkpoints to preserve accepted artifact drift,
So that durable memory advances without traversing repositories, capturing
secrets, or implying publication.

## Acceptance Criteria

- [ ] Status reports changed, removed, and untracked root artifacts without
  writing journal events or replacing the manifest.
- [ ] Checkpoint inventory never follows symbolic-link targets.
- [ ] Linked repositories contribute remote, branch, and commit metadata only;
  credentials are removed from remotes and working-tree contents are not
  context artifacts.
- [ ] Sensitive filenames, text matching context-owned sensitive-content
  patterns, binary content, and files above the context-owned size limit are
  rejected with explicit findings.
- [ ] Accepted changed artifacts are durable before the manifest is replaced.
- [ ] Removed artifacts produce durable tombstones before the manifest is
  replaced.
- [ ] The manifest is replaced atomically and journaled last.
- [ ] A failed artifact write leaves the previous manifest head authoritative.
- [ ] Status and checkpoint never invoke replication, synchronization, or
  publication.

## Notes

A checkpoint label is metadata, not a substitute for artifact content or a
request to publish.
