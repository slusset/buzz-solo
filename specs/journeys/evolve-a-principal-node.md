---
id: evolve-a-principal-node
type: journey
refs:
  persona: specs/personas/domain-architect.md
---

# Journey: Evolve a Principal Domain and Principal Node coherently

## Actor

The domain architect reviews an operational friction or proposed capability
that crosses Principal Domain, Principal Node, runtime instance, relay, host,
release, and interface boundaries.

Source Persona: `specs/personas/domain-architect.md`

## Trigger

A working deployment reveals that a process, host script, service manager,
adapter, or interface has absorbed responsibility belonging to a Principal
Domain or one of its authorized Principal Nodes.

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

### 2. Separate domain, node, and execution identity

- **User intent**: Establish what persists when keys rotate, nodes move, and
  processes restart.
- **System response**: The root context and journal authority are named the
  Principal Domain; each stable authorized operational representative is a
  Principal Node; each process execution is a disposable Node Runtime
  Instance.
- **Next**: Declare how a Principal Node receives authority.

### 3. Authorize the Principal Node

- **User intent**: Ensure a running node represents the domain by explicit
  authority rather than shared filesystem access or convention.
- **System response**: A current domain-root authority signs the Principal
  Node authorization. The domain identifier remains stable when its root
  verification key rotates, and revocation prevents new node operations
  without rewriting history.
- **Next**: Assign each responsibility to a layer.

### 4. Assign responsibility to one layer

- **User intent**: Prevent a host or interface from becoming accidental domain
  authority.
- **System response**: Each responsibility is assigned to the Principal
  Domain, Principal Node, Node Runtime Instance, portable relay core, host
  adapter, release/migration plane, or interface adapter. Cross-layer work is
  expressed through a named port.
- **Next**: Declare what the host may provide.

### 5. Declare host capabilities without granting authority

- **User intent**: Support macOS, Linux, foreground operation, and future hosts
  without changing node semantics.
- **System response**: The host signs a capability claim; the Principal Node
  binds that claim under its domain authorization. Missing capabilities
  produce an explicit degradation or refusal; host signals never authorize a
  state transition.
- **Next**: Model one PrincipalNode-owned synchronization procedure.

### 6. Model one Principal-Node procedure

- **User intent**: Ensure startup, operator requests, commit notifications,
  peer wakes, and recovery ticks cannot drift into separate workflows.
- **System response**: Every trigger enters the same guarded Principal Node
  state machine, regardless of which Runtime Instance receives it.
  Synchronization re-evaluates current declarations, preserves exact event
  envelopes, and commits a source-bound cursor only after durable,
  checkpoint-safe outcomes.
- **Next**: Define coherence evidence.

### 7. Make the boundary observable across time scales

- **User intent**: Know whether intent, code, release, host binding, and live
  runtime topology still agree.
- **System response**: Merge-time, runtime, and resurrection-time invariants
  verify domain authorization, node identity, runtime-instance binding, host
  capability binding, and state ownership. They produce independent `ok`,
  `drift`, `violation`, or `unknown` observations with bounded evidence.
- **Next**: Review the complete specification chain.

### 8. Review before implementation

- **User intent**: Commit to the semantic boundary before choosing concrete
  Rust modules, processes, or OS integrations.
- **System response**: Persona, journey, stories, domain models, architecture
  decision, capability scope, and traceability links are reviewed together.
  Unresolved choices remain explicit and implementation does not begin.
- **Next**: Hand the accepted narrative and model to behavior-contract work.

## Outcomes

- **Success**: One Principal Domain can authorize many stable Principal Nodes;
  each Principal Node owns its continuity and synchronization semantics across
  disposable Runtime Instances; hosts provide mechanical capabilities; and
  release and runtime evidence remain independently verifiable.
- **Failure modes**:
  - the Principal Domain is identified directly by a rotatable root key;
  - a Runtime Instance or host process is mistaken for the Principal Node;
  - a Principal Node operates without current domain authorization;
  - the portable relay core absorbs Principal Node orchestration;
  - launchd, systemd, or a shell script owns cursors, stream selection, or
    retry semantics;
  - a wake signal bypasses agreement or authorization evaluation;
  - host paths or process labels become node identity;
  - release metadata is treated as authority over journal replay;
  - one coherence score conceals a violation or an unknown observation;
  - implementation begins while responsibility remains ambiguous.

## Related Stories

- `specs/stories/principal-node/authorize-principal-node.md`
- `specs/stories/principal-node/place-synchronization-inside-principal-node.md`
- `specs/stories/principal-node/declare-host-capabilities-without-domain-authority.md`
- `specs/stories/principal-node/observe-principal-boundary-coherence.md`
- `specs/stories/principal-node/promote-compatible-node-runtime.md`

## E2E Coverage

- Planned: Principal Domain/Node boundary behavior and host-adapter conformance
  features
  after specification review.
