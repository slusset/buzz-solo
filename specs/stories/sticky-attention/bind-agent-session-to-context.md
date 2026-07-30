---
id: bind-agent-session-to-context
type: story
refs:
  journey: specs/journeys/maintain-a-durable-context-through-agent-sessions.md
  persona: specs/personas/sovereign-builder.md
  steps: [1, 2, 6]
---

# Story: Bind an agent session to one durable context

## Narrative

As a sovereign builder,
I want a harness session to discover exactly one opted-in durable context,
So that integrations remain portable without taking ownership of context
identity or policy.

## Acceptance Criteria

- [ ] Context identity, enabled hooks, linked-directory roles, disclosure
  policy, and checkpoint limits come from context-owned configuration.
- [ ] The generic harness adapter contains no context identifier or
  context-specific filesystem path.
- [ ] A working directory inside the context root resolves to that context.
- [ ] A working directory inside a declared linked repository resolves to the
  owning context without following the link during artifact inventory.
- [ ] Zero matching contexts leave the session unbound and write no lifecycle
  residue.
- [ ] Multiple matching contexts fail closed with an explicit ambiguity result
  and write no lifecycle residue.
- [ ] The unchanged adapter can serve another opted-in context while keeping
  session bindings isolated.

## Notes

GitHub Copilot CLI is the first validated harness adapter, not part of the
domain identity. Other harnesses satisfy the same contract through their own
lifecycle surfaces.
