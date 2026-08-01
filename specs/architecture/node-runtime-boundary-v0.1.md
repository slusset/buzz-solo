# Sovereign Node Runtime Boundary v0.1

Status: proposed — review before behavior contracts or implementation
Date: 2026-08-01

## Decision

Buzz Solo introduces a first-class **sovereign node runtime** between the
portable relay core and host-specific integration.

The node runtime is the portable application boundary that operates one
identity-bound node context. It owns runtime lifecycle, synchronization,
source-bound cursors, retry classification, agreement evaluation,
coherence observation, and compatibility gates. It coordinates these rules
through ports without depending on launchd, systemd, filesystem conventions,
network libraries, keychain APIs, or user-interface frameworks.

The host adapter provides declared mechanical capabilities: supervision,
placement, custody, clock and wake delivery, session signals, and attestation.
A host can make a capability available; it cannot decide what the node may
synchronize, which cursor is safe, whether a declaration matches, or which
domain transition is admissible.

The portable relay core remains smaller than the node runtime. It owns signed
event verification, classification, reduction, filters, and normative relay
decisions. It exposes replication source and sink ports, but it does not own
scheduling, peer relationships, cursor persistence policy, retries, release
selection, or host lifecycle.

This resolves the phrase “synchronization belongs inside the relay” precisely:
synchronization belongs inside the sovereign node product boundary, in the
node runtime surrounding the portable relay core. It does not belong in the
OS service manager, and it does not enlarge the deterministic relay kernel.

## Terms that must remain distinct

| Term | Meaning | Durable authority |
| --- | --- | --- |
| Node context | The identity-bound aggregate: journal, profile references, cursors, declarations, context heads, and checkpoints. | Journal and verified node-owned stores. |
| Node runtime | The portable application that operates one node context through ports. | None by existence; it exercises authority derived from current signed state and configured identities. |
| Portable relay core | Deterministic event and query semantics used by runtime adapters. | Normative relay decisions after policy and durable append. |
| Host adapter | OS/runtime integration implementing declared mechanical capabilities. | None over domain transitions. |
| Runtime release | Signed evidence identifying executable provenance, integrity, and compatibility claims. | None over journal replay or append. |
| Interface adapter | CLI, agent, desktop, device, or API translation into node commands and queries. | Only the actor capability presented with a request. |

A node context can survive the loss of a runtime installation or host. A
runtime release can be replaced without replacing the node. A host can wake a
runtime without authorizing work. A healthy relay process is not, by itself,
evidence that the node boundary is coherent.

## Architectural shape

```text
 CLI / agents / desktop / devices
                 |
          inbound node ports
                 |
    +------------v------------------------------------+
    | sovereign node runtime                          |
    | lifecycle | sync | cursors | coherence | gates |
    +-------+-------------+----------------+----------+
            |             |                |
       relay ports   peer/artifact     host capability ports
            |        transport ports   + release evidence
    +-------v-------+     |                |
    | portable relay|  transport       launchd/systemd/
    | core          |  adapters        foreground/keychain/XDG
    +---------------+  + storage       adapters
```

The semantic dependency direction points inward. Outer adapters know the
ports they implement. The node-runtime application knows port abstractions and
domain models. The portable relay core knows neither the node application nor
any adapter.

## Responsibility allocation

| Responsibility | Owner | Explicitly not owner |
| --- | --- | --- |
| Verify and classify signed event envelopes | Portable relay core | Node scheduler, host, interface |
| Apply destination ingest and return receipts | Relay sink plus destination policy | Source, host wake layer |
| Derive stream agreement and transport readiness | Node runtime using current signed declarations | Host adapter, transport |
| Select streams and normalize triggers | Node runtime | launchd/systemd, pulse transport |
| Classify retries and checkpoint-safe completion | Node runtime | host scheduler, source cursor encoding |
| Commit a source-bound cursor | Node runtime through a durable cursor-store port | host service job |
| Start, stop, restart, and wake an executable | Host supervision/clock-wake adapter | relay core |
| Resolve paths and durable placement | Host placement adapter | domain model |
| Resolve signing capabilities | Host custody adapter under node policy | release manifest |
| Produce and sign runtime artifacts | Release/migration plane | running node runtime |
| Verify release compatibility before state mutation | Node runtime at startup/promotion | installer alone |
| Translate user or agent intent | Interface adapter | host integration |

## Synchronization is one node-owned procedure

Startup, durable journal commits, peer wakes, recovery ticks, operator
requests, and retries all create the same `SyncSession` command. A trigger
answers only **when should the node evaluate?** It never answers **what is
allowed?**

For each attempt, the node runtime:

1. resolves the configured source stream and its exact source-bound cursor;
2. derives current agreement and transport readiness from current declaration
   heads and authenticated transport evidence;
3. asks a replication source for a bounded batch;
4. delivers exact signed envelopes through the destination's normal sink;
5. classifies every receipt under checkpoint-safety rules;
6. persists the source-issued cursor unchanged only after every covered record
   is durably checkpoint-safe;
7. records completion, blocked, or retryable evidence in node-owned runtime
   state.

The host clock/wake port may deliver “evaluate now” at startup or a scheduled
instant. A pulse transport may deliver “peer state may have advanced.” Neither
signal carries trusted source identity, selection, declaration status, or
cursor content. Missed wakes converge through a node-owned recovery policy.

The reference topology is one long-lived supervised node service. The
semantic requirement is one composition root and one owner of synchronization
state, not necessarily one OS thread or one future executable forever.
Auxiliary workers are permitted only when they act through node-runtime ports
and cannot independently interpret or commit domain state.

The current `buzz-relay-pull`, `buzz-relay-push`, `scripts/buzz-drain`, and
separately supervised host jobs are **transitional compatibility adapters**.
They remain useful migration evidence, but they are not the target ownership
model. No new synchronization semantics may be added to those host jobs.

## State placement follows meaning

The node runtime classifies state before asking a host where to place it:

| State class | Examples | Authority and recovery |
| --- | --- | --- |
| Domain history | Signed events, declarations, context heads, witness residue | Journal-authoritative and replayable. |
| Node continuity | Source-bound cursors, runtime checkpoints, migration markers | Node-owned durable data; never reconstructed from host job history. |
| Configuration | Profiles, role references, stream selections, capability requirements | Operator-controlled input; secrets appear only by opaque reference. |
| Host binding | Capability manifest, placement resolution, credential references, service identity | Host-local signed evidence; never journal authority. |
| Operational state | Logs, unrecorded coherence observations, transient session diagnostics | Disposable or reconstructible unless explicitly promoted through an authorized domain append. |
| Runtime installation | Versioned executable and adapter artifacts, release manifest | Reproducible from verified release evidence; contains no mutable node state. |

XDG directories, macOS Library locations, container mounts, and future device
stores are adapter mappings for these classes. A directory name does not decide
the ownership or authority of the bytes it contains.

## Ports

Ports are named for the capability the node requires, not the technology that
happens to provide it.

### Inbound node ports

- **Node lifecycle** — start, quiesce, stop, inspect readiness.
- **Synchronization request** — request evaluation with a trigger class; no
  policy arguments are accepted from a wake caller.
- **Coherence query** — evaluate one invariant or render the current vector.
- **Operator transition** — explicit promotion, migration, reconciliation,
  skip, or recovery decisions with actor evidence.

### Relay and domain ports

- **Relay ingest/query** — normal signed-event decisions and explicit queries.
- **Replication source/sink** — bounded exact envelopes and source-bound
  receipts as defined by the portable relay boundary.
- **Declaration projection** — current effective owner declarations and
  matched-agreement derivation.
- **Cursor store** — durable get/compare/commit of opaque source-bound cursors;
  storage supplies atomicity but never interprets the token.
- **Checkpoint store** — durable runtime checkpoints tied to journal and cursor
  heads. Checkpoints are signed by a configured node checkpoint identity;
  signatures attest the observed heads and never make invalid state admissible.

### Transport and artifact ports

- **Peer transport** — authenticated delivery and bounded reads; transport
  identity is evidence, not policy ownership.
- **Artifact custody** — content-addressed probe, fetch, and upload under
  reference-gated authorization.

### Host capability ports

- **Supervision** — executable start, stop, restart, and process health.
- **Placement** — config, data, state, cache, and runtime locations plus
  durability properties.
- **Custody** — signing interfaces and verification ceremonies by opaque
  reference, never private key extraction where the class forbids it.
- **Clock/wake** — monotonic and wall clocks plus scheduled and event wakes.
- **Session signals** — OS and harness lifecycle evidence.
- **Attestation** — verifiable host and authenticator properties.

### Release evidence port

- **Release verifier** — verifies source revision, artifact digest, signature,
  supported schemas, and required host capability profile before selection.
  Verification supplies provenance and compatibility evidence, not domain
  authority.

## Rust and hexagonal architecture

Rust makes this boundary enforceable through the Cargo dependency graph and
the type system rather than naming conventions alone.

### Dependency rule

The crate or module that consumes a capability owns the port trait. Concrete
adapters depend inward on that trait.

```text
domain/event kernel  <-  node-runtime application  <-  composition root
                                  ^                         |
                                  |                         v
                           port implementations: relay, transport,
                           cursor store, host, release, interfaces
```

Consequences:

- domain types and relay decisions contain no `std::process`, OS service
  labels, environment-variable lookup, filesystem layout, HTTP client, or
  keychain code;
- node-runtime use cases depend on narrow traits such as `CursorStore`,
  `ClockWake`, `HostCapabilities`, `ReplicationTransport`, and
  `ReleaseVerifier`;
- launchd, systemd, XDG, keychain, Cloudflare, WebSocket, and CLI code
  implement ports in adapter crates or adapter modules;
- only the executable composition root chooses concrete implementations and
  wires them together;
- an adapter crate may depend on domain/application crates; a domain or
  application crate may not depend on an adapter crate;
- Cargo features may choose adapter availability but may not put conditional
  OS behavior inside domain rules.

The exact crate split is deferred to implementation design. Conformance is
about dependency direction and observable behavior, not maximizing crate
count. A module boundary is sufficient when Cargo-level isolation would add no
useful enforcement; a crate boundary is preferred where forbidden dependencies
or independently testable adapters need mechanical enforcement.

### Modeling guidance

- Use newtypes for identities that must not be confused: `NodePrincipal`,
  `SourceStreamId`, `OwnerIdentity`, `TransportPrincipal`, and cursor tokens.
- Keep source cursors opaque. The source adapter may encode them; destination,
  host, and interface code cannot parse or increment them.
- Represent domain refusals separately from transport and adapter failures so
  retry policy cannot reinterpret a policy violation as a network retry.
- Test application use cases with in-memory port implementations; test each
  real adapter against the same conformance contract.
- Keep startup wiring and OS error translation at the composition root.

These are architectural constraints. Concrete trait signatures, async
strategy, error enums, and crate names belong to the later implementation
design and behavior-contract pass.

## Boundary coherence

This decision becomes measurable through stable invariant identifiers.
Coherence is a vector; one green aggregate score cannot hide a critical
failure.

### Merge-time

- `runtime-dependency-direction` — the Cargo/module graph contains no inward
  dependency on host, transport, installer, or interface adapters.
- `single-sync-procedure` — every supported trigger reaches the same behavior
  contract and lifecycle.
- `cursor-ownership` — only node-runtime application code can authorize
  source-bound cursor commitment.
- `adapter-conformance` — foreground and at least one OS adapter pass the same
  host-capability contract.
- `spec-traceability` — sovereign-surface changes link intent, story, model,
  behavior contract, implementation, and evidence.

### Runtime

- `runtime-boundary-coherence` — the live component graph has one selected
  node runtime and no independently supervised process making synchronization
  policy or cursor decisions.
- `host-capability-coherence` — active requirements match a verified host
  capability manifest; missing evidence is `unknown`, not `ok`.
- `release-coherence` — the running bytes, source revision, schemas, and host
  profile match verified release evidence.
- `replication-coherence` — agreement heads, transport evidence, cursors,
  receipts, and expected lag are mutually consistent.

### Resurrection-time

- a fresh foreground or Linux host adapter can operate the same node context;
- replay, cursor, and context heads match the declared checkpoint;
- variant K synchronization is driven by the node runtime rather than a
  recreated host script;
- changing host mechanisms does not change node-visible transition results.

Observation is always read-only. Persisting an observation is a separate,
authorized metadata-only append. A monitor never repairs a projection,
changes a declaration, advances a cursor, or rewrites a host binding.

## Release and migration consequences

A runtime release must eventually declare:

- runtime version and source revision;
- artifact digest and signing evidence;
- supported profile, journal, cursor, checkpoint, and host-manifest schemas;
- required host capability profile;
- migration identifiers and compatibility direction.

Before first mutation, the node runtime verifies the selected release against
the existing node context and active host binding. A migration defines
preconditions, postconditions, a recovery point, and any irreversible
boundary. Rollback changes executable selection; it never implicitly rolls the
journal or committed cursors backward.

Release evidence cannot bypass event verification, declaration policy,
custody, or replay. Mutable node state and private material never enter the
release artifact.

## Migration sequence

This decision does not authorize implementation yet. After review and behavior
contracts, migration should proceed in independently reversible slices:

1. introduce the node-runtime application boundary around existing relay and
   CLI operations with no behavior change;
2. place cursor access behind the node-owned cursor-store port;
3. route pull and push triggers into one `SyncSession` procedure;
4. add clock/wake and capability-manifest host ports with a foreground adapter;
5. supervise the composed node runtime as one service;
6. retain compatibility entry points long enough to prove equivalent receipts
   and cursor heads;
7. retire independent pull/push host jobs only after runtime and resurrection
   coherence evidence passes.

At every slice, the journal and cursor heads remain recoverable from the
existing migration backup and checkpoint process.

## Questions for this review

The boundary decision is proposed; these choices should be settled before the
behavior-contract pass:

1. Is **sovereign node runtime** the right name for the application boundary,
   or would **relay runtime** communicate the product boundary more clearly?
2. Should conformance require one OS process, or retain this draft's weaker
   semantic rule: one composition root and one owner of synchronization state,
   with one long-lived service as the reference topology?
3. Which identity signs a host capability manifest: an owner-authorized host
   attestation key, the node owner directly, or a two-step host claim plus
   owner binding?
4. Which `SyncSession` evidence is durable node continuity data, and which is
   disposable operational telemetry? Cursor commitment itself is always
   durable.
5. What is the minimum runtime checkpoint: release + journal head + cursor
   heads, or must it also bind the active profile and host-manifest digests;
   and which node identity signs it?
6. Does the first canonical release manifest extend `release.json`, or become a
   separately signed manifest referenced by both the release tag and installed
   runtime?

## Non-goals

- No new Nostr event kind or transport protocol in this decision.
- No automatic repair, declaration mutation, or cursor skipping.
- No requirement that all adapters share an async runtime or server framework.
- No final Rust crate decomposition or process manager format yet.
- No conversion of host capability facts into journal authority.
- No unattended runtime promotion.

## Traceability

- Telos: [`../TELOS.md`](../TELOS.md)
- Persona: [`../personas/domain-architect.md`](../personas/domain-architect.md)
- Journey:
  [`../journeys/evolve-a-sovereign-node-runtime.md`](../journeys/evolve-a-sovereign-node-runtime.md)
- Stories:
  [`../stories/node-runtime/place-synchronization-inside-node-runtime.md`](../stories/node-runtime/place-synchronization-inside-node-runtime.md),
  [`../stories/node-runtime/declare-host-capabilities-without-domain-authority.md`](../stories/node-runtime/declare-host-capabilities-without-domain-authority.md),
  [`../stories/node-runtime/observe-boundary-coherence.md`](../stories/node-runtime/observe-boundary-coherence.md),
  [`../stories/node-runtime/promote-compatible-node-runtime.md`](../stories/node-runtime/promote-compatible-node-runtime.md)
- Runtime model:
  [`../models/node-runtime/node-runtime.model.yaml`](../models/node-runtime/node-runtime.model.yaml)
- Runtime lifecycle:
  [`../models/node-runtime/node-runtime.lifecycle.yaml`](../models/node-runtime/node-runtime.lifecycle.yaml)
- Synchronization model:
  [`../models/node-runtime/sync-session.model.yaml`](../models/node-runtime/sync-session.model.yaml)
- Synchronization lifecycle:
  [`../models/node-runtime/sync-session.lifecycle.yaml`](../models/node-runtime/sync-session.lifecycle.yaml)
- Coherence model:
  [`../models/node-runtime/coherence-observation.model.yaml`](../models/node-runtime/coherence-observation.model.yaml)
- Host capability model:
  [`../models/node-host/host-capability-manifest.model.yaml`](../models/node-host/host-capability-manifest.model.yaml)
- Cursor model:
  [`../models/shared/replication-cursor.model.yaml`](../models/shared/replication-cursor.model.yaml)
- Inner relay boundary: [`portable-relay-boundary.md`](portable-relay-boundary.md)
- Outer host boundary: [`node-host-boundary-v0.1.md`](node-host-boundary-v0.1.md)
- Synchronization agreements:
  [`sovereign-sync-agreement-v0.1-draft.md`](sovereign-sync-agreement-v0.1-draft.md)
- Runtime observation: [`coherence-monitoring-v0.1.md`](coherence-monitoring-v0.1.md)
- Release channel:
  [`node-release-distribution-v0.1.md`](node-release-distribution-v0.1.md)
- Resurrection evidence: [`resurrection-drill-v0.1.md`](resurrection-drill-v0.1.md)
