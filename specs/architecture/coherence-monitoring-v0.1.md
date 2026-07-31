# Coherence Monitoring v0.1

Status: draft
Date: 2026-07-31

## Decision

Testing in Buzz Solo is **coherence observation at three time scales**:
merge-time (the five CI lanes), runtime (this spec), and resurrection-time
([the resurrection drill](resurrection-drill-v0.1.md)). Coherence
monitoring is the runtime scale: the node continuously observes whether
its three artifacts still agree —

- **intent** — TELOS and the spec chain;
- **code** — the repository at the running node's version;
- **runtime** — the living state: journal, projections, declarations,
  replicas, sessions.

Two rules are absolute:

1. **Monitoring never mutates.** Findings are observations; reconciliation
   is always a separate, explicit act. The monitor holds the same
   read-only posture as the context explorer and the same fail-closed
   philosophy as everything else.
2. **Findings are journal residue.** A coherence observation lands as a
   metadata-only witness event in the journal, so the node's
   self-knowledge accumulates in the system it describes, is attributable
   and replayable like everything else, and renders in the explorer.

## Invariant catalog

Each invariant names its existing machinery; monitoring is mostly the
promotion of existing checks from "runs when invoked" to "runs on a
cadence and leaves residue."

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
- Scheduled runs ride the host adapter's supervision port (launchd
  periodic on Nest; systemd timers elsewhere) — cadence is host policy,
  not node code.
- Findings are witness events carrying: invariant id, severity
  (`info | drift | violation`), observed/expected references (event ids,
  head addresses, cursor positions — never payload copies), and the
  monitor's version. Metadata-only, same forbidden-list posture as
  lifecycle residue.
- A finding that repeats does not re-append daily noise: findings are
  per-(invariant, subject) parameterized-replaceable heads; history of a
  drift lives in head replacements, and resolution replaces the head
  with a clean observation.

## Cadence tiers (default policy, host-adjustable)

| Tier | Invariants | Default |
|---|---|---|
| Heartbeat | pulses, cursor health | continuous / each drain cycle |
| Daily | projection-agreement, session-coherence, journal append-only | daily |
| Deep | full replay equality, signature sampling, declaration probe, intent-coherence | weekly |

Deep checks are local and bounded; anything requiring a network probe of
a replica declares it and stays within existing authenticated legs — the
monitor introduces no new transport.

## Conformance sketch

`coherence-monitoring-v0.1`:

- every catalog invariant is observable on demand;
- no monitor code path mutates journal, projections, declarations, or
  replicas;
- findings land as metadata-only witness heads and render in the
  explorer;
- a seeded incoherence of each class (a tampered projection file, a
  stale replica grant, an orphaned active session, an unspecced kind) is
  detected by the corresponding invariant within its tier's cadence;
- monitoring runs produce no publication or replication side effects.

## Non-goals

- Auto-repair of any kind (reconciliation stays explicit).
- Alerting infrastructure — findings are journal residue; how a host
  surfaces them (explorer, doctor, push) is host/adapter policy.
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
  (supervision port)
