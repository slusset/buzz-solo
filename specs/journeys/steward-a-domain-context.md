---
id: steward-a-domain-context
type: journey
refs:
  persona: specs/personas/domain-steward.md
---

# Journey: Steward a domain through a durable context

## Actor

The domain steward, working across agent harnesses and runtimes on a host
with a durable-context home (`${DURABLE_CONTEXT_HOME:-~/DurableContext}`).

Persona: `specs/personas/domain-steward.md`

## Trigger

A real-world domain (an integration, a system, an initiative) needs memory
that outlives sessions, harnesses, and eventually the steward.

## Preconditions

- The local sovereign relay is running.
- The steward can create a context root and link existing repositories.
- Publication and replication remain explicit-request-only throughout.

## Flow

### 1. Initialize the domain's context root

- **User intent**: Give the domain one durable home on disk and in the
  journal.
- **System response**: `buzz context init` creates the root's `.context/`
  opt-in (identity, `h` boundary, disclosure policy, checkpoint policy), a
  charter stub, and the context's journal presence. Nothing is published.
- **Next**: Link the domain's code.

### 2. Link repositories without absorbing them

- **User intent**: Associate the domain's repos and directories so sessions
  inside them bind to this context.
- **System response**: `buzz context link-repo` records each target as a
  symlink plus metadata; inventory hashes targets as strings and never
  traverses them. Discovery now resolves sessions opened inside a linked
  repo to this context, failing closed on ambiguity.
- **Next**: Work.

### 3. Work in the domain from any surface

- **User intent**: Let ordinary work accrete into the context regardless of
  which harness or runtime hosts the session.
- **System response**: Harness adapters carry the durable-context-hooks
  contract: metadata-only session records, checkpoints that write the
  manifest last, reconciliation on interrupted sessions. The current-work
  head holds the deliberate "what now" with a next safe action.
- **Next**: Make the domain findable.

### 4. Declare the domain beacon

- **User intent**: Give the domain one stable identity that names its
  context, authoritative surfaces, and stewardship chain.
- **System response**: A beacon head records domain, context reference,
  surfaces (URLs and titles only), stewardship (principal, collaborators,
  target handoff), and provenance notes — credentials and secrets excluded
  by rule. The beacon projects into the root as a readable file.
- **Next**: Keep visibility.

### 5. Explore what the node knows

- **User intent**: Look around — every stewarded context, and any one of
  them in depth — without composing queries.
- **System response**: `buzz context explore` renders the deterministic
  projection: contexts by warmth, then heads, current work, open loops,
  sessions, linked repos, artifacts, skills, beacon. Read-only, always
  recomputable, identical on every surface.
- **Next**: Eventually, hand off.

### 6. Hand the domain to the next principal

- **User intent**: Transfer stewardship as a defined operation.
- **System response**: The beacon and current-work head orient the
  successor; a journal handoff (`open → claim → return → close`) transfers
  custody with verifiable evidence. The domain's memory was never in the
  steward's head, so nothing is lost in the transfer.

## Outcome

The domain is resumable by any authorized principal — human or agentic —
from what the node knows: bound directories, attributed history, current
work, authoritative surfaces, and a stewardship chain, with sensitive
material excluded by policy rather than by hope.
