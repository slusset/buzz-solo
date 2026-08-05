# Principal Domain and Principal Node Boundary v0.1

Status: proposed — terminology adopted; review before behavior contracts
Date: 2026-08-01

## Decision

Buzz Solo distinguishes three identities that the earlier node-runtime draft
combined:

1. The **Principal Domain** is the stable, identity-bound root context. It owns
   the logical journal, domain declarations and policy, nested bounded-context
   heads, and authorization of Principal Nodes.
2. A **Principal Node** is a stable operational principal authorized to
   represent exactly one Principal Domain. It owns node-local continuity:
   source-bound cursors, synchronization sessions, runtime checkpoints, host
   binding, selected release, and coherence observations.
3. A **Node Runtime Instance** is one ephemeral process execution of a selected
   release for a Principal Node. It may disappear, restart, or move hosts
   without changing Principal Node or Principal Domain identity.

The existing “root node key” becomes the current **domain-root verification
key**. It authorizes the Principal Domain and its node bindings, but the raw key
is not the permanent Principal Domain identifier. Rotation changes the active
verification method without creating a new domain or rewriting history.

The Principal Node is the portable application and authority boundary around
the relay core. It owns synchronization, cursor commitment, retry
classification, agreement evaluation, coherence, and compatibility gates.
The Node Runtime Instance executes those decisions but acquires no authority
merely by running. Host adapters provide mechanical capabilities and likewise
acquire no domain authority.

This resolves “synchronization belongs inside the relay” precisely:
synchronization belongs to the Principal Node representing the Principal
Domain. It does not belong in the OS service manager or an ephemeral process,
and it does not enlarge the deterministic portable relay core.

## Ubiquitous language

| Term | Meaning | Identity and authority |
| --- | --- | --- |
| Principal Domain | Root context and durable domain aggregate spanning its journal and nested contexts. | Stable `PrincipalDomainId`; current domain-root key authorizes changes. |
| Domain-root verification key | Rotatable verification method currently authorized to govern a Principal Domain. | Authority, not permanent domain identity. |
| Principal Node | Stable operational representative of exactly one Principal Domain. | Stable `PrincipalNodeId` plus current domain authorization. |
| Principal Node authorization | Domain-root-signed binding of one Principal Node to one Principal Domain and scope. | Required for new node-owned operations. |
| Node Runtime Instance | One process executing one verified release for a Principal Node on a bound host. | Ephemeral; no independent domain authority. |
| Host capability claim | Host-signed declaration of mechanical capabilities and opaque references. | A claim until a Principal Node binds it. |
| Host binding | Principal-Node selection of a verified host capability claim for a bounded purpose and validity interval. | Permits mechanical calls; grants no domain authority to the host. |
| Portable relay core | Deterministic verification, classification, reduction, filtering, and relay decisions. | Meaning of events; unaware of Principal Domain/Node orchestration. |
| Runtime release | Signed executable provenance, integrity, and compatibility evidence. | Never authority over journal replay or append. |
| Interface adapter | CLI, agent, desktop, device, or API translation into commands and queries. | Only the actor evidence presented with the request. |

“Relay node principal” in the portable identity profile is the protocol-facing
projection of `PrincipalNodeId`. It is not a second entity. Owner identity,
Principal Domain, Principal Node, runtime instance, event author, transport
key, host attestation key, and custodian remain distinct claims.

## Relationship and cardinality

```text
Principal Domain
  stable root context + logical journal
  current domain-root authority
       |
       | authorizes 1..N over time or topology
       v
Principal Node
  stable operational identity + continuity state
       |
       | selects 0..1 active instance
       v
Node Runtime Instance
  verified release + composition root
       |
       | binds one host capability claim
       v
Host Adapter
```

One Principal Domain may authorize several Principal Nodes without merging
their node-local cursors, checkpoints, host bindings, or process identities.
One Principal Node represents exactly one Principal Domain. A Principal Node
may have no active Runtime Instance while offline and may create successive
instances across restart, promotion, recovery, or host migration.

The reference topology permits one active Runtime Instance per Principal Node.
This is enforced as a semantic lease or selection, not by assuming one PID or
one operating-system process forever. Auxiliary workers are allowed only
through the selected instance's composition root and cannot independently
interpret or commit Principal Node state.

## Architectural shape

```text
 CLI / agents / desktop / devices
                 |
        Principal Node inbound ports
                 |
    +------------v---------------------------------------+
    | Principal Node application boundary               |
    | auth | lifecycle | sync | cursors | coherence     |
    +--------+----------------------+--------------------+
             |                      |
      Principal Domain ports   relay/transport/host/release ports
             |                      |
    +--------v---------+      +-----v--------------------+
    | root context and |      | concrete adapters wired  |
    | journal policy   |      | by Node Runtime Instance |
    +------------------+      +--------------------------+
```

The semantic dependency direction points inward. The Principal Domain and
Principal Node models know no OS, process, network, or installer APIs. The
Node Runtime Instance is the composition root that selects concrete adapters.

## Authority chain

A new operation is admissible only when each relevant link verifies:

1. the stable Principal Domain identifier resolves to a current domain-root
   authority;
2. a current domain-root-signed authorization binds the Principal Node to that
   domain and permits the operation class;
3. the Principal Node has selected a compatible signed runtime release;
4. the Principal Node has bound a verified host capability claim for the
   operation purpose and current validity interval, satisfying release and
   operation requirements;
5. the Runtime Instance matches the selected release, Principal Node, and host
   binding;
6. event, declaration, stream-agreement, transport, and destination-policy
   evidence independently authorize the requested domain transition.

No later link can manufacture an earlier one. A valid process cannot create a
Principal Node. A valid host claim cannot create domain authorization. A valid
release cannot validate an event. A domain-root key does not automatically
become an event-author, transport, node-checkpoint, or host-attestation key.

Revoking Principal Node authorization prevents new node-owned operations after
the next authority evaluation. It does not erase accepted events, receipts,
checkpoints, or the historical fact that the node was once authorized.

## Responsibility allocation

| Responsibility | Owner | Explicitly not owner |
| --- | --- | --- |
| Root context, logical journal authority, domain policy | Principal Domain | Principal Node, runtime, host |
| Authorize or revoke a Principal Node | Principal Domain root authority | Host, runtime release, transport key |
| Node-local cursors, checkpoints, host and release selection | Principal Node | Principal Domain journal, host scheduler |
| Verify and classify signed event envelopes | Portable relay core | Principal Node scheduler, host, interface |
| Apply destination ingest and return receipts | Relay sink plus destination policy | Source, host wake layer |
| Derive stream agreement and transport readiness | Principal Node using current signed declarations | Runtime Instance, host, transport |
| Select streams and normalize triggers | Principal Node | launchd/systemd, pulse transport |
| Classify retries and checkpoint-safe completion | Principal Node | host scheduler, source cursor encoding |
| Commit a source-bound cursor | Principal Node through its durable cursor-store port | Runtime memory, host service job |
| Execute the selected application and adapters | Node Runtime Instance | Principal Domain identity |
| Start, stop, restart, and wake a process | Host supervision/clock-wake adapter | Relay core |
| Resolve paths and durable placement | Host placement adapter | Domain model |
| Resolve signing capabilities | Host custody adapter under Principal Node policy | Release manifest |
| Produce and sign runtime artifacts | Release/migration plane | Running instance |
| Verify release compatibility before mutation | Principal Node at instance selection | Installer alone |
| Translate user or agent intent | Interface adapter | Host integration |

## Synchronization is Principal-Node-owned

Startup, durable journal commits, peer wakes, recovery ticks, operator
requests, and retries all create the same `SyncSession` command for a Principal
Node. A trigger answers only **when may the node evaluate?** It never answers
**what is allowed?**

For each attempt, the Principal Node:

1. re-evaluates its current Principal Domain authorization;
2. resolves the configured source stream and exact source-bound cursor;
3. derives current agreement and transport readiness from signed declaration
   heads and authenticated transport evidence;
4. asks a replication source for exactly one bounded page and accepts the
   source's unchanged-or-advanced classification of its opaque `next_cursor`;
5. blocks an unauthenticated peer as `transport_not_ready`, while cursor-load,
   transport-unavailable, projection-unavailable, source-unavailable, and
   malformed-source-batch failures durably record distinct evaluation-failure
   evidence before exposing `failed`;
6. treats an empty advanced page as successful `scan_progress` (or `caught_up`
   when the source says caught up), commits no receipt evidence, and treats an
   empty non-caught-up unchanged page as malformed to prevent livelock;
7. delivers exact signed envelopes through the destination's normal sink;
8. classifies every receipt under checkpoint-safety rules;
9. for a cursor-advancing completion, atomically compares the expected
   continuity cursor and commits the exact source-issued cursor together with
   the immutable completed terminal summary through Principal Node continuity
   after every selected returned record is durably checkpoint-safe, or with
   zero receipts when the page contains only source-filtered scan progress;
10. for completed no-work, blocked, failed, or cancelled outcomes, commits the
   immutable terminal summary without a cursor mutation before exposing the
   terminal state.

Zero-record scan progress is safe because a source stream's selection predicate
is bound to its immutable `SourceStreamId`; excluded durable source records do
not require sink receipts. Redefining that predicate requires a new stream
identity and fresh cursor lineage. The Principal Node does not parse or order
the opaque token, and an initial no-position equivalence is declared by the
source rather than synthesized by the destination.

Continuity transitions are committed facts. If continuity persistence is
unavailable or reports an ambiguous result, the attempt returns a typed pending
continuity result containing its prior lifecycle state and exact immutable
candidate commit. Retrying that candidate uses the same `SyncSession` ID and
either commits it or observes `already_committed_same`; conflicting content
fails closed and no new retry session is created.

Replacing a Runtime Instance does not create a new cursor lineage, selection,
or retry policy. The host clock/wake port may deliver “evaluate now,” and a
pulse transport may report that peer state may have advanced. Neither signal
carries trusted source identity, selection, declaration status, or cursor
content. Missed wakes converge through Principal-Node-owned recovery policy.

The current `buzz-relay-pull`, `buzz-relay-push`, `scripts/buzz-drain`, and
separately supervised host jobs are transitional compatibility adapters. No
new synchronization semantics may be added to them. They are retired only
after equivalent Principal Node behavior and resurrection evidence pass.

## Host capability claim and binding

Host integration is a two-step relationship:

1. A host attestation identity signs an immutable capability claim describing
   supervision, placement, custody, clock/wake, sessions, and attestation.
2. A currently authorized Principal Node verifies and binds that claim for a
   named capability profile, purpose, validity interval, and Runtime Instance
   selection.

The host claim proves what the host says it can do. The Principal Node binding
proves that this node chose to rely on it. Neither signature grants authority
over Principal Domain policy, stream selection, cursor safety, or event
admission.

Paths, service labels, process IDs, credential references, and timer mechanisms
remain host-local facts. When recorded in a checkpoint or observation, they
appear only as bounded claim digests or stable references permitted by
disclosure policy.

## State placement follows meaning

The models classify state before asking a host where to place it:

| State class | Examples | Owner and recovery |
| --- | --- | --- |
| Principal Domain history | Signed events, declarations, root bindings, nested context heads | Domain journal; authoritative and replayable. |
| Principal Node continuity | Cursors, terminal sync summaries, signed checkpoints, migration markers | Node-owned durable data; never reconstructed from process or scheduler history. |
| Configuration | Profiles, role references, selections, capability requirements | Operator-controlled input; secrets appear only by opaque reference. |
| Host claim and binding | Signed claim, claim digest, placement and credential references | Host-local claim plus Principal Node binding; never domain authority. |
| Operational state | Logs, detailed attempt diagnostics, unrecorded observations, process handles | Disposable or reconstructible unless explicitly promoted through an authorized append. |
| Runtime installation | Versioned executable/adapter artifacts and release manifest | Reproducible from verified release evidence; contains no mutable domain or node state. |

XDG directories, macOS Library locations, container mounts, and future device
stores are host-adapter mappings for these classes. A directory name does not
decide the identity, ownership, or authority of its bytes.

## Ports

Ports are named for the capability required by the Principal Node, not the
technology providing it.

### Inbound Principal Node ports

- **Node lifecycle** — bind, activate, quiesce, take offline, inspect readiness.
- **Synchronization request** — request evaluation with a trigger class; wake
  callers cannot provide policy decisions.
- **Coherence query** — evaluate one invariant or render the current vector.
- **Operator transition** — explicit promotion, migration, reconciliation,
  skip, or recovery decisions with actor evidence.

### Principal Domain ports

- **Domain authorization** — resolve current domain-root authority and current
  Principal Node authorization.
- **Domain journal** — append, replay, and query exact signed history.
- **Declaration projection** — current effective owner declarations and
  matched-agreement derivation.
- **Root-context projection** — derive nested bounded-context and policy heads.

### Relay and continuity ports

- **Relay ingest/query** — normal signed-event decisions and explicit queries.
- **Replication source/sink** — bounded exact envelopes and source-bound
  receipts as defined by the portable relay boundary.
- **Sync continuity** — durable cursor and terminal-summary reads, immutable
  non-cursor terminal commits, and idempotent atomic compare-and-commit of an
  exact source-bound cursor with its completed summary; storage supplies
  atomicity but never interprets the token or changes a pending candidate.
- **Checkpoint store** — signed Principal Node checkpoints bound to selected
  release, domain journal head, cursor heads, active profile digest, and host
  claim/binding digests. Signatures attest evidence and never make invalid
  state admissible.

### Transport, artifact, host, and release ports

- **Peer transport** — authenticated delivery and bounded reads; transport
  identity is evidence, not policy ownership.
- **Artifact custody** — content-addressed probe, fetch, and upload under
  reference-gated authorization.
- **Supervision** — Runtime Instance start, stop, and process health.
- **Placement** — config, data, state, cache, and runtime locations plus
  durability properties.
- **Custody** — signing interfaces and verification ceremonies by opaque
  reference.
- **Clock/wake** — monotonic and wall clocks plus scheduled and event wakes.
- **Session signals** — OS and harness lifecycle evidence.
- **Host attestation** — capability-claim signatures and authenticator facts.
- **Release verifier** — source revision, artifact digest, signature, supported
  schemas, and required host capability profile before instance selection.

## Rust and hexagonal architecture

Rust makes the distinction enforceable through the Cargo dependency graph and
types rather than naming conventions alone.

```text
Principal Domain kernel
          ^
          |
Principal Node application + ports
          ^
          |
Node Runtime Instance composition root
          |
          +-- relay, cursor, transport, host, release, CLI adapters
```

The crate or module consuming a capability owns its port trait. Concrete
adapters depend inward:

- Principal Domain types contain no process, host, transport, release, path,
  or environment-variable code;
- Principal Node use cases depend on narrow traits such as
  `DomainAuthority`, `SyncContinuity`, `ClockWake`, `HostCapabilities`,
  `ReplicationTransport`, and `ReleaseVerifier`;
- the Runtime Instance composition root chooses concrete implementations and
  translates startup/OS failures;
- launchd, systemd, XDG, keychain, Cloudflare, WebSocket, and CLI code live in
  adapter crates or modules;
- adapter crates may depend on Principal Domain/Node crates; the domain and
  application crates may not depend on adapters;
- Cargo features may select adapter availability but may not place conditional
  OS behavior inside domain rules.

Use distinct newtypes for `PrincipalDomainId`, `DomainRootKey`,
`PrincipalNodeId`, `RuntimeInstanceId`, `SourceStreamId`, `TransportPrincipal`,
and opaque cursor tokens. A compiler-visible distinction prevents possession
of one identity from being accidentally accepted as another.

The exact crate split, trait signatures, async strategy, and error enums are
deferred to implementation design. Conformance is dependency direction and
observable behavior, not maximum crate count.

## Boundary coherence

Coherence is a vector of named observations. One green aggregate score cannot
hide a critical failure.

### Merge-time

- `principal-dependency-direction` — domain/application code contains no
  inward dependency on runtime, host, transport, installer, or interfaces.
- `domain-node-authorization` — every Principal Node operation requires a
  current binding to exactly one Principal Domain.
- `single-sync-procedure` — every trigger reaches the same Principal Node
  behavior contract and lifecycle.
- `cursor-ownership` — only Principal Node application code can authorize
  source-bound cursor commitment.
- `runtime-instance-ephemerality` — no process, PID, host, or release value is
  used as Principal Domain or Principal Node identity.
- `adapter-conformance` — foreground and OS adapters pass the same host
  capability contract.

### Runtime

- `domain-node-coherence` — the Principal Node's current authorization, scope,
  and domain-root authority verify.
- `principal-node-coherence` — one stable Principal Node owns continuity and
  synchronization state across instance replacement.
- `runtime-instance-coherence` — the selected process matches the Principal
  Node, release, composition-root revision, and host binding.
- `host-capability-coherence` — requirements match the bound verified host
  claim; missing evidence is `unknown`, not `ok`.
- `release-coherence` — running bytes, source revision, schemas, and host
  profile match verified release evidence.
- `replication-coherence` — declaration heads, transport evidence, cursors,
  receipts, and expected lag are mutually consistent.

### Resurrection-time

- a fresh host can reconstitute the same Principal Domain and Principal Node;
- a newly created Runtime Instance receives no identity from its PID or host;
- replay, cursor, and context heads match the signed Principal Node checkpoint;
- variant K synchronization is Principal-Node-owned rather than a recreated
  host script;
- changing host mechanisms does not change admissible transition results.

Observation is read-only. Recording an observation is a separate authorized
metadata-only append. A monitor never repairs a projection, changes domain or
node authorization, advances a cursor, or rewrites host binding.

## Release, checkpoint, and migration consequences

A runtime release eventually declares version, source revision, artifact
digest, signing evidence, supported schemas, required host capability profile,
and migration compatibility. It contains no mutable Principal Domain or Node
state.

Before first mutation, the Principal Node verifies the selected release against
its domain authorization, continuity state, and host binding. A Runtime
Instance is then created for that exact selection. A migration defines
preconditions, postconditions, a recovery point, and any irreversible boundary.
Rollback changes instance/release selection; it never implicitly rolls the
domain journal or committed cursors backward.

A Principal Node checkpoint binds:

- `PrincipalDomainId`, current authorization event, and domain journal head;
- `PrincipalNodeId` and checkpoint signer;
- selected release and Runtime Instance evidence;
- source-bound cursor heads;
- active profile digest;
- host capability claim and binding digests.

The Principal Node's configured checkpoint key signs the canonical digest.
That key may rotate through node authorization policy without changing
`PrincipalNodeId`. A checkpoint signature attests observed continuity and
cannot make invalid journal, cursor, release, or host evidence admissible.

## Migration sequence

This decision still authorizes specifications only. After review and behavior
contracts, migration proceeds in reversible slices:

1. introduce `PrincipalDomain`, root authority, and explicit Principal Node
   authorization around existing state with no behavior change;
2. introduce `PrincipalNodeId` independently of host, process, and release;
3. place cursor access and terminal sync summaries behind Principal Node ports;
4. route pull and push triggers into one `SyncSession` procedure;
5. model the current process as a `NodeRuntimeInstance` composition root;
6. add host capability claim/binding ports with a foreground adapter;
7. supervise the selected Runtime Instance while Principal Node continuity
   remains durable outside it;
8. retain compatibility entry points until receipt, cursor, restart, and
   resurrection evidence agree;
9. retire independent pull/push host jobs.

At every slice, Principal Domain journal and Principal Node cursor heads remain
recoverable from the existing migration backup and checkpoint process.

## Adopted choices and remaining review

Adopted in this revision:

- `PrincipalDomain`, `PrincipalNode`, and `NodeRuntimeInstance` are the
  canonical names;
- domain identity is stable across domain-root key rotation;
- a Principal Domain may authorize many Principal Nodes; each Principal Node
  represents exactly one domain;
- one semantic instance selection is required, not one permanent OS process;
- host integration uses a host-signed claim plus Principal Node binding;
- cursors, terminal sync summaries, and checkpoints are durable Principal Node
  continuity; detailed diagnostics are operational state;
- checkpoints bind domain, node, release/instance, cursor, profile, and host
  evidence and are signed by a rotatable Principal Node checkpoint key.

Remaining for the behavior-contract review:

- the event kind/address form for Principal Domain root and Principal Node
  authorization declarations;
- revocation timing for an already-running operation at a checkpoint boundary;
- whether the canonical release manifest extends `release.json` or is a
  separately signed artifact referenced by both tag and installation.

## Non-goals

- No new Nostr event kind or transport protocol in this decision.
- No multi-owner Principal Domain semantics.
- No automatic repair, declaration mutation, or cursor skipping.
- No requirement that adapters share an async runtime or server framework.
- No final Rust crate decomposition or process manager format yet.
- No conversion of host or process facts into domain authority.
- No unattended runtime promotion.

## Traceability

- Telos: [`../TELOS.md`](../TELOS.md)
- Persona: [`../personas/domain-architect.md`](../personas/domain-architect.md)
- Journey:
  [`../journeys/evolve-a-principal-node.md`](../journeys/evolve-a-principal-node.md)
- Stories:
  [`../stories/principal-node/authorize-principal-node.md`](../stories/principal-node/authorize-principal-node.md),
  [`../stories/principal-node/place-synchronization-inside-principal-node.md`](../stories/principal-node/place-synchronization-inside-principal-node.md),
  [`../stories/principal-node/declare-host-capabilities-without-domain-authority.md`](../stories/principal-node/declare-host-capabilities-without-domain-authority.md),
  [`../stories/principal-node/observe-principal-boundary-coherence.md`](../stories/principal-node/observe-principal-boundary-coherence.md),
  [`../stories/principal-node/promote-compatible-node-runtime.md`](../stories/principal-node/promote-compatible-node-runtime.md)
- Principal Domain model:
  [`../models/principal-domain/principal-domain.model.yaml`](../models/principal-domain/principal-domain.model.yaml)
- Principal Node model:
  [`../models/principal-node/principal-node.model.yaml`](../models/principal-node/principal-node.model.yaml)
- Principal Node lifecycle:
  [`../models/principal-node/principal-node.lifecycle.yaml`](../models/principal-node/principal-node.lifecycle.yaml)
- Runtime Instance model:
  [`../models/principal-node/node-runtime-instance.model.yaml`](../models/principal-node/node-runtime-instance.model.yaml)
- Runtime Instance lifecycle:
  [`../models/principal-node/node-runtime-instance.lifecycle.yaml`](../models/principal-node/node-runtime-instance.lifecycle.yaml)
- Synchronization model:
  [`../models/principal-node/sync-session.model.yaml`](../models/principal-node/sync-session.model.yaml)
- Coherence model:
  [`../models/principal-node/coherence-observation.model.yaml`](../models/principal-node/coherence-observation.model.yaml)
- Host capability model:
  [`../models/node-host/host-capability-claim.model.yaml`](../models/node-host/host-capability-claim.model.yaml)
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
