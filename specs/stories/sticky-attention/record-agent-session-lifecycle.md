---
id: record-agent-session-lifecycle
type: story
refs:
  journey: specs/journeys/maintain-a-durable-context-through-agent-sessions.md
  persona: specs/personas/sovereign-builder.md
  steps: [3, 5]
---

# Story: Record agent-session lifecycle residue

## Narrative

As a sovereign builder,
I want agent-session starts, completions, and interruptions recorded
automatically,
So that attention continuity survives both normal exits and missing end
callbacks without copying the conversation.

## Acceptance Criteria

- [ ] Session identity is namespaced by context ID, adapter ID, and
  harness-local session ID so two harnesses cannot collide.
- [ ] A bound session start appends an attributable record carrying context ID,
  adapter ID, harness session ID, phase, timestamp, and only optional metadata
  permitted by context disclosure policy.
- [ ] A normal end callback appends a completion record with its timestamp and
  a bounded completion-reason code.
- [ ] Prompts, responses, transcripts, credentials, error bodies, and final
  messages never enter lifecycle residue.
- [ ] Session-end capture is best effort; failure is surfaced and does not
  fabricate success.
- [ ] On a later bound start, an older active session is marked interrupted
  only when the adapter supplies positive abandonment evidence; concurrent and
  unverifiable sessions remain active and visible.
- [ ] Reconciliation is idempotent and never rewrites a completed session.
- [ ] A delayed completion may correct an interrupted effective state only when
  it proves the session ended before reconciliation; the interruption evidence
  remains in history.
- [ ] Lifecycle capture and reconciliation are local durability operations and
  never trigger synchronization or publication.

## Notes

Next-start reconciliation is the correctness backstop for harnesses whose end
callbacks cannot be guaranteed during process termination.
