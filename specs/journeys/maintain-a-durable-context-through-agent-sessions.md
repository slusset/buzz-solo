---
id: maintain-a-durable-context-through-agent-sessions
type: journey
refs:
  persona: specs/personas/sovereign-builder.md
---

# Journey: Maintain a durable context through agent sessions

## Actor

The sovereign builder uses an accountable coding agent through any harness
that can expose session lifecycle callbacks and local tools.

Persona: `specs/personas/sovereign-builder.md`

## Trigger

The builder opens an agent session in a durable-context root or in a repository
the context explicitly links.

## Preconditions

- The local sovereign relay is running.
- The durable context owns its identity, directory bindings, disclosure policy,
  and explicit harness-integration opt-in.
- A generic harness adapter is installed on the host. It contains no context
  identifier or context-specific filesystem path.
- Lifecycle operations are local-only unless the builder separately requests
  publication through an authorized replication workflow.

## Flow

### 1. Enable the harness without surrendering context policy

- **User intent**: Let an agent harness participate in this context without
  making the adapter authoritative.
- **System response**: The context's own opt-in names enabled lifecycle hooks,
  checkpoint limits, sensitive-file policy, and linked-directory roles. The
  adapter reads this policy but cannot replace the context identity.
- **Next**: Resolve the session's context.

### 2. Bind the session to exactly one context

- **User intent**: Work from either the context root or a linked repository
  without manually naming a context.
- **System response**: The adapter walks upward from the working directory and
  compares it with enabled roots and declared linked directories. Exactly one
  match creates a binding; zero or multiple matches fail closed and write no
  lifecycle residue.
- **Next**: Record session start.

### 3. Record metadata-only session start

- **User intent**: Leave an attributable attention cursor without disclosing
  the conversation.
- **System response**: The adapter appends a signed start record carrying the
  context boundary, harness session ID, adapter ID, phase, timestamp, and any
  working-directory representation permitted by context disclosure policy.
  Prompts, responses, transcripts, credentials, error bodies, and final
  messages are excluded.
- **Next**: Work and inspect drift.
- **Contracts**: `on_session_start`, `POST /events`

### 4. Inspect and checkpoint intentional context drift

- **User intent**: Preserve context-owned artifacts and repository positions at
  a meaningful milestone.
- **System response**: Status reports drift without writing. On an explicit
  checkpoint, the adapter inventories root artifacts without following symbolic
  links, rejects configured sensitive filenames and content as well as binary
  and oversized files, records linked repositories as sanitized
  remote/branch/commit metadata only, writes changed artifacts and removal
  tombstones, then replaces the manifest last.
- **Next**: Continue or end the session.
- **Contracts**: `status`, `checkpoint`, `POST /events`

### 5. Complete or reconcile the session

- **User intent**: Preserve an honest lifecycle even when the harness exits
  unexpectedly.
- **System response**: A normal callback appends a metadata-only completion
  record with a bounded reason code. If no completion arrives, the next bound
  session may append an interruption record only when the adapter has positive
  abandonment evidence. Concurrent or unverifiable sessions remain active and
  visible. A delayed completion that proves it occurred before reconciliation
  corrects the effective state without erasing the interruption evidence.
- **Next**: Re-enter from the resulting attention cursor.
- **Contracts**: `on_session_end`, `reconcile`, `POST /events`

### 6. Reuse the adapter in another durable context

- **User intent**: Carry the integration pattern to another context without
  copying identity or policy.
- **System response**: The second context supplies its own identity and opt-in
  contract. The unchanged adapter discovers and serves it independently; a
  session never crosses context boundaries.
- **Next**: Continue working with the same lifecycle guarantees.

## Outcomes

- **Success**: Every bound session leaves minimal, attributable lifecycle
  residue; interruptions are visible; explicit checkpoints preserve accepted
  context drift; and the same adapter can serve multiple contexts without
  owning any of them.
- **Failure modes**:
  - adapter code hardcodes a context ID or path and silently serves the wrong
    boundary;
  - multiple contexts claim a linked directory and the adapter chooses one;
  - lifecycle events copy conversation or error content;
  - a later start falsely interrupts a concurrent session;
  - a missing end callback has no abandonment evidence and remains visibly
    unresolved;
  - checkpoint inventory traverses repository symlinks or captures secrets;
  - manifest replacement precedes artifact durability;
  - a local lifecycle action implicitly publishes or synchronizes state.

## Related Stories

- `specs/stories/sticky-attention/bind-agent-session-to-context.md`
- `specs/stories/sticky-attention/record-agent-session-lifecycle.md`
- `specs/stories/sticky-attention/checkpoint-durable-context.md`

## E2E Coverage

- Planned: adapter conformance scenarios in
  `specs/features/sticky-attention/agent-session-lifecycle.feature`
