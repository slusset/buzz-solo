---
id: evolve-a-sovereign-node-runtime
type: journey
refs:
  persona: specs/personas/domain-architect.md
---

# Journey: Evolve a sovereign node runtime coherently

## Actor

The domain architect reviews an operational friction or proposed capability
that crosses relay, node, host, release, and interface boundaries.

Source Persona: `specs/personas/domain-architect.md`

## Trigger

A working deployment reveals that a host script, service manager, adapter, or
interface has absorbed responsibility for a domain transition that should
remain portable and attributable.

## Preconditions

- TELOS and the existing specification chain are available.
- The current runtime and host topology can be observed without mutation.
- Durable state, credentials, installed runtime files, and host state are
  distinguishable.
- The proposed change has not yet been implemented.

## Flow

### 1. Treat operational friction as boundary evidence

- **User intent**: Understand what the difficult operation revealed instead of
  recording it only as deployment inconvenience.
- **System response**: The current flow is described in terms of actors,
  transitions, authority, durable state, host effects, and release evidence.
- **Next**: Identify the state whose continuity is at stake.

### 2. Name the state and authorized transitions

- **User intent**: Establish what remains logically singular even when several
  actors or replicas participate.
- **System response**: The specification identifies the aggregate, immutable
  evidence, derived state, commands, guards, and cursor or checkpoint rules.
- **Next**: Assign each responsibility to a bounded context.

### 3. Assign responsibility to one layer

- **User intent**: Prevent a host or interface from becoming accidental domain
  authority.
- **System response**: Each responsibility is assigned to the portable relay
  core, sovereign node runtime, host adapter, release/migration plane, or an
  interface adapter. Cross-layer work is expressed through a named port.
- **Next**: Declare what the host may provide.

### 4. Declare host capabilities without granting authority

- **User intent**: Support macOS, Linux, foreground operation, and future hosts
  without changing node semantics.
- **System response**: The host declares supervision, placement, custody,
  clock/wake, session, and attestation capabilities. Missing capabilities
  produce an explicit degradation or refusal; host signals never authorize a
  state transition.
- **Next**: Model the runtime lifecycle.

### 5. Model one portable runtime procedure

- **User intent**: Ensure startup, operator requests, commit notifications,
  peer wakes, and recovery ticks cannot drift into separate workflows.
- **System response**: Every trigger enters the same guarded node-runtime state
  machine. Synchronization re-evaluates current declarations, preserves exact
  event envelopes, and commits a source-bound cursor only after durable,
  checkpoint-safe outcomes.
- **Next**: Define coherence evidence.

### 6. Make the boundary observable across time scales

- **User intent**: Know whether intent, code, release, host binding, and live
  runtime topology still agree.
- **System response**: Merge-time, runtime, and resurrection-time invariants
  produce independent `ok`, `drift`, `violation`, or `unknown` observations
  with bounded evidence. No aggregate score can hide a critical violation.
- **Next**: Review the complete specification chain.

### 7. Review before implementation

- **User intent**: Commit to the semantic boundary before choosing concrete
  Rust modules, processes, or OS integrations.
- **System response**: Persona, journey, stories, domain models, architecture
  decision, capability scope, and traceability links are reviewed together.
  Unresolved choices remain explicit and implementation does not begin.
- **Next**: Hand the accepted narrative and model to behavior-contract work.

## Outcomes

- **Success**: One portable node-runtime boundary owns synchronization and
  coherence semantics; hosts provide mechanical capabilities; release and
  runtime evidence are independently verifiable; and the implementation can
  later be tested against the same model on different hosts.
- **Failure modes**:
  - the portable relay core absorbs node orchestration;
  - launchd, systemd, or a shell script owns cursors, stream selection, or
    retry semantics;
  - a wake signal bypasses agreement or authorization evaluation;
  - host paths or process labels become node identity;
  - release metadata is treated as authority over journal replay;
  - one coherence score conceals a violation or an unknown observation;
  - implementation begins while responsibility remains ambiguous.

## Related Stories

- `specs/stories/node-runtime/place-synchronization-inside-node-runtime.md`
- `specs/stories/node-runtime/declare-host-capabilities-without-domain-authority.md`
- `specs/stories/node-runtime/observe-boundary-coherence.md`
- `specs/stories/node-runtime/promote-compatible-node-runtime.md`

## E2E Coverage

- Planned: node-runtime boundary behavior and host-adapter conformance features
  after specification review.
