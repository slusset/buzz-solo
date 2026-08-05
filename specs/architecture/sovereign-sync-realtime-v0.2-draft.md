# Sovereign sync realtime v0.2 (draft)

Status: design recorded, implementation deferred. Realtime replication is
relationship-layer work and stays gated behind the Buzz Solo
(sticky-attention) milestone. Proposed by the downstream consumer node in
journal record `6e0f4231…` after the first closed release loop; the pull
half has a working precedent in `a36cb7b79` (drain rendezvous on beacon
transitions).

## Purpose

Reduce replication latency between sovereign nodes from the polling
interval to near-realtime, without introducing any new authority, custody
path, or replication code path. Realtime is a latency optimization over
the existing scheduled recovery — never a correctness dependency.

## Design

Three triggers, one procedure owned by the
[PrincipalNode](principal-node-boundary-v0.1.md):

1. **Commit-triggered push.** A durable local journal commit enqueues an
   immediate, debounced synchronization request using the same declared
   source, stream filters, and PrincipalNode-owned source-bound cursor as recovery.
   The debounce window coalesces bursts; every request enters the same
   `SyncSession` lifecycle.

2. **Pulse-triggered pull (peer wake).** When the rendezvous journal head
   advances, it wakes authenticated peer sessions over their existing
   standing sockets — the same sessions the pulse layer already witnesses
   under `coherence.sessions`. A woken peer asks its PrincipalNode, through the
   active RuntimeInstance, to evaluate the same drain procedure as recovery. This extends the transition-driven
   drain precedent (`a36cb7b79`) from beacon transitions to head-advance wake.

3. **Recovery tick as backstop.** The PrincipalNode owns the recovery cadence
   and requests wakes through its RuntimeInstance's host clock/wake capability. A launchd timer,
   systemd timer, foreground loop, or platform alarm may deliver the tick, but
   none owns a second synchronization procedure. A lost wake degrades to
   recovery-tick latency, never to data loss.

## Invariants

- **Wake signals carry no authority.** A pulse or commit trigger conveys
  "look now," nothing else. Every triggered drain re-evaluates the
  declared export/admit/read agreements exactly as a scheduled drain
  does; no trigger can widen a selection or bypass a grant.
- **One code path.** Realtime and recovery replication are the same
  procedure with different triggers. There is no separate fast path to
  drift from the recovery path.
- **Cursor integrity.** All triggers share the PrincipalNode-owned, source-bound cursor
  and stream selection. No trigger may fall back to a default cursor or an
  unselected stream set — the `context sync` cursor/selection
  passthrough defect reported in `6e0f4231…` is a hard precondition for
  this work, not a parallel concern.
- **Fail silent, converge on node policy.** Missed wakes are not errors and
  are not retried at the wake layer; the recovery tick converges them.
- **Pulse-layer discipline.** Wake signals are ephemeral,
  conversation-not-record, per beacon pulse v0.2. Durable state changes
  belong to the journal; the wake layer never becomes a second source of
  truth.

## Out of scope

- New event kinds or transport mechanisms; the existing pulse semantics
  and authenticated sockets suffice.
- Any change to agreement evaluation, artifact custody, or checkpoint
  semantics.
- Unattended execution concerns; claim exclusivity is a separate track.

## Sequencing

1. Precondition: profile cursor and stream-selection passthrough in
   `context sync` (defect fix, lands with the managed context CLI).
2. Commit-triggered push on the development node.
3. Head-advance peer wake at the rendezvous.
4. Only then, tune backstop intervals down if the realtime path proves
   reliable — never remove them.

## Traceability

- PrincipalNode owner: [`principal-node-boundary-v0.1.md`](principal-node-boundary-v0.1.md)
- Sync session model:
  [`../models/principal-node/sync-session.model.yaml`](../models/principal-node/sync-session.model.yaml)
- Sync lifecycle:
  [`../models/principal-node/sync-session.lifecycle.yaml`](../models/principal-node/sync-session.lifecycle.yaml)
- Host clock/wake capability:
  [`node-host-boundary-v0.1.md`](node-host-boundary-v0.1.md)
