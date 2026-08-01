# Coherence Monitoring v0.1

Status: draft
Date: 2026-07-31

## Decision

Testing in Buzz Solo is **coherence observation at three time scales**:
merge-time (the five CI lanes), runtime (this spec), and resurrection-time
([the resurrection drill](resurrection-drill-v0.1.md)). Coherence monitoring
is the runtime scale: the node continuously observes whether its
architectural evidence still agrees —

- **intent** — TELOS and the spec chain;
- **model and code** — stories, domain models, behavior contracts, and the
  repository at the running node's version;
- **release and host binding** — verified runtime bytes, compatibility claims,
  and declared host capabilities;
- **runtime** — the living state: journal, projections, declarations,
  replicas, sessions.

Two rules are absolute:

1. **Monitoring never mutates.** Findings are observations; reconciliation
   is always a separate, explicit act. The monitor holds the same
   read-only posture as the context explorer and the same fail-closed
   philosophy as everything else.
2. **Observation and recording are separate.** Evaluation is read-only.
   After it completes, an authorized node identity may record the finding as
   metadata-only witness residue through the normal append path. Recording is
   never implicit and cannot change the subject that was observed.

Coherence is a vector of named invariant observations. Every observation is
`ok`, `drift`, `violation`, or `unknown`; one score may not average away a
critical violation or missing evidence.

## Invariant catalog

Each invariant names its existing machinery; monitoring is mostly the
promotion of existing checks from "runs when invoked" to "runs on a cadence"
with an optional, separately authorized recording step.

### journal-integrity

Replaying the NDJSON journal produces the same effective set the live
node serves (the portable boundary's replay invariant, observed
recurringly); the journal is append-only since the last observation;
signature spot-checks pass on sampled events.

### projection-agreement

Generated `.context/` files (`artifacts.yaml`, `current-work.yaml`) match
their journal heads — the "journal wins" rule with a watcher. Artifact
manifests match disk per the existing `status` operation
(changed/removed/untracked is already a coherence probe wearing a CLI
costume). Divergence is a finding, never an auto-repair.

### declaration-coherence

Each replica's *actual* admitted set matches the current declaration
heads: grants honored that were revoked, grants declared but not
rehydrated, exports flowing outside any declaration. Generalizes the
existing drift-steward machinery from stream drift to trust drift.

### replication-coherence

Stream-head equality between Nest and each replica within the expected
lag window; drain-leg cursor health (advancing, resumable); beacon
pulses observed from both nodes (`buzz context pulse` is the existing
heartbeat — its absence is itself a finding).

### runtime-boundary-coherence

The live component graph agrees with the
[node-runtime boundary](node-runtime-boundary-v0.1.md): one selected runtime
owns synchronization and cursor decisions. Independently supervised pull,
push, cursor, or coherence jobs are violations when they interpret domain
state rather than act as adapters through node-runtime ports.

### host-capability-coherence

The active verified host capability manifest satisfies the runtime's required
profile. Placement, custody, supervision, clock/wake, session, and attestation
claims resolve to bounded evidence. Missing evidence is `unknown`, never an
inferred success; optional loss is explicit degradation.

### release-coherence

The running executable digest, source revision, supported state/profile
schemas, and required host profile agree with verified release evidence. A
valid release proves provenance, integrity, and compatibility claims; it never
proves authority to replay or append journal state.

### session-coherence

Sessions marked active whose namespaced identity can be checked against
host facts: where the host adapter can confirm a dead process, the
finding carries `host_process_confirmed_dead` evidence ready for the
existing reconcile operation — but the monitor only *reports* it.

### intent-coherence

The telos-facing monitor: event kinds observed in the journal that no
spec names; sovereign-surface behavior (declarations, handoffs, engrams,
replication) whose spec artifact is missing or contradicted. Checkable
mechanically (kinds observed vs. kinds documented in `kind.rs` and the
chain) — the monitor that keeps the spec chain honest in the direction
merge-time CI cannot.

## Mechanism

- `buzz node coherence [--invariant <id>] [--deep]` — one-shot run;
  `buzz context doctor` remains the operator-facing summary and gains a
  coherence section. (Namespace per the tooling spec: node-level
  commands never take a root.)
- Scheduled runs enter the node runtime through its clock/wake port. The node
  owns cadence and the invariant procedure; launchd, systemd, a foreground
  loop, or another host mechanism only delivers “evaluate now” with declared
  timing properties.
- Observations carry invariant id, subject, status (`ok | drift | violation |
  unknown`), criticality, observer revision, and observed/expected references
  (event IDs, head addresses, cursor positions, release digests — never payload
  copies or host-private identifiers).
- Recording an observation is a separate authorized command. Its witness event
  is metadata-only and follows the same forbidden-list posture as lifecycle
  residue.
- A finding that repeats does not re-append daily noise: findings are
  per-(invariant, subject) parameterized-replaceable heads; history of a
  drift lives in head replacements, and resolution replaces the head
  with a clean observation.

## Cadence tiers (default node policy, constrained by host capabilities)

| Tier | Invariants | Default |
|---|---|---|
| Heartbeat | pulses, cursor health | continuous / each drain cycle |
| Daily | projection-agreement, session-coherence, journal append-only | daily |
| Deep | full replay equality, signature sampling, declaration probe, intent-coherence | weekly |

Deep checks are local and bounded; anything requiring a network probe of a
replica declares it and stays within existing authenticated legs — the monitor
introduces no new transport. A host may report that it cannot meet a requested
cadence; the node records degradation or blocks a required profile rather than
silently changing policy.

## Conformance sketch

`coherence-monitoring-v0.1`:

- every catalog invariant is observable on demand;
- every result is a named four-state observation, and critical unknowns or
  violations remain individually visible;
- no observation code path mutates journal, projections, declarations,
  cursors, host binding, or replicas;
- a separate authorized recording path can land findings as metadata-only
  witness heads and render them in the explorer;
- a seeded incoherence of each class (a tampered projection file, a
  stale replica grant, an orphaned active session, an unspecced kind, an
  independently supervised sync job, a missing host capability, or release
  byte skew) is detected by the corresponding invariant within its tier's
  cadence;
- monitoring runs produce no publication or replication side effects.

## Non-goals

- Auto-repair of any kind (reconciliation stays explicit).
- Alerting infrastructure — observations may be recorded as journal residue;
  how a host surfaces them (explorer, doctor, push) is host/adapter policy.
- Monitoring other owners' nodes.

## Traceability

- Time scales: five CI lanes (merge),
  [`resurrection-drill-v0.1.md`](resurrection-drill-v0.1.md) (resurrection)
- Telos: [`../TELOS.md`](../TELOS.md) — intent/code/runtime coherence
- Drift precedent:
  [`../journeys/detect-and-reconcile-stream-drift.md`](../journeys/detect-and-reconcile-stream-drift.md),
  [`../features/sovereign-sync/steward-drift.feature`](../features/sovereign-sync/steward-drift.feature)
- Replay invariant: [`portable-relay-boundary.md`](portable-relay-boundary.md)
- Projections and status:
  [`durable-context-tooling-v0.1.md`](durable-context-tooling-v0.1.md),
  [`../contracts/agent-harness/durable-context-hooks.yaml`](../contracts/agent-harness/durable-context-hooks.yaml)
- Scheduling: [`node-host-boundary-v0.1.md`](node-host-boundary-v0.1.md)
  (clock/wake and supervision ports)
- Runtime ownership:
  [`node-runtime-boundary-v0.1.md`](node-runtime-boundary-v0.1.md)
- Observation model:
  [`../models/node-runtime/coherence-observation.model.yaml`](../models/node-runtime/coherence-observation.model.yaml)
