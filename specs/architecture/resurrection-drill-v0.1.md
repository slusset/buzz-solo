# Resurrection Drill v0.1

Status: draft
Date: 2026-07-31

## Decision

The strongest claim this architecture makes — *a PrincipalDomain and an
authorized PrincipalNode can be reconstructed from identity plus network* —
is verified by a scheduled drill, not asserted. The drill stands up the
sovereign context on a **fresh rented host** (a
Digital Ocean droplet is the reference target) and is the named acceptance
test for the `node-host-migration-v0.1` conformance profile in
[node-host-boundary-v0.1](node-host-boundary-v0.1.md). It also forces the
first real second host adapter: Linux, systemd user units, XDG placement —
the host capability claim and PrincipalNode binding earning their keep on
unfamiliar soil.

A resurrection performed once is an anecdote; performed on schedule, it is
a property. Cadence: **quarterly minimum**, and after any change to
hydration, custody, or replication code paths.

## Two variants, two claims

The drill runs both variants; they verify different things and their
difference *is* one of the measurements.

### Variant R — restore (disaster recovery)

Carry the principal context artifact to the fresh host and hydrate:

1. Provision the droplet; install nothing by hand beyond the bootstrap.
2. Fetch the runtime by its signed `node/vX.Y.Z` tag; **verify the tag
   signature before executing anything**
   ([node-release-distribution](node-release-distribution-v0.1.md)).
3. Transport the principal context artifact (checkpoint + journal + sealed
   envelope) via an existing authenticated leg (artifact custody or
   direct copy).
4. Hydrate per the host boundary: verify PrincipalDomain root authority and
   PrincipalNode authorization → placement → release, host-claim, and binding
   compatibility gate → journal replay verified against the checkpoint's
   `journal_head` (fail closed on mismatch) → custody provisioning → create
   and supervise a new NodeRuntimeInstance.
5. Append a witnessed `node-hydrated` record from the new host.

**Claim proven**: laptop dies, backup exists → node reborn byte-exact.

### Variant K — key-only (resurrection from the network)

Arrive with nothing but key material:

1. Provision the droplet; fetch and verify the runtime as above.
2. Instantiate a profile from the carried identity; authenticate to
   Rendezvous.
3. Authorize or recover a PrincipalNode, bind the verified fresh-host claim,
   launch a new RuntimeInstance, and request recovery. The PrincipalNode
   evaluates current declarations and drains every stream its transport role
   is entitled to read into the empty journal through the ordinary
   `SyncSession` procedure.
4. Unseal what the carried material can unseal (engram ciphertext drains
   like any stream; conversation keys arrive only via the sealed
   envelope or recovery ceremony — never via Rendezvous).
5. Produce the **gap report** (below).

**Claim proven**: laptop burns with no backup → this much survives.

## The gap report

The key-only variant's primary output is not pass/fail but a measured
inventory: streams, context heads, engrams, and artifacts recoverable
from the network versus the Nest journal's full inventory. The report is
a witness event (counts and head references, never payloads) and turns
"which streams should be exported?" from a philosophical question into a
number driven toward a target. **Drill failure includes the gap growing
unexpectedly** — a context that silently stopped exporting is exactly
what the drill exists to catch.

## The identity rule (decided by the owner, 2026-07-31)

Rented metal raises the question the identity roles were built for:
*which keys may touch the droplet?* The rule:

- The current **domain-root verification key never touches rented metal.** It stays on
  hardware at Nest. This is a rule, not a recommendation: a drill run
  that places root material on the droplet is an automatic failure. The
  PrincipalDomain ID remains stable if that verification key later rotates.
- Each drill authorizes a **scoped drill PrincipalNode** and grants its
  separate transport role read access to exported streams for the drill
  window. This exercises both PrincipalNode authorization and the kind-30700
  vocabulary as signed, auditable data.
- Teardown **revokes the stream grant and PrincipalNode authorization** — so
  every drill also exercises both revocation lifecycles.
- The sealed envelope travels only in variant R, and only under its
  custody rules (recovery ceremony to open; passkey-sealed material
  requires the authenticator, which never leaves the owner).

A true worst-case rehearsal (Nest destroyed, only the recovery envelope
survives) is variant K plus the recovery ceremony, and should be drilled
at least annually.

## Success criteria

All of the following, from the droplet:

- `buzz context doctor` green — profile, identities, relays, runtime;
- `buzz context load buzz/durable-context` returns the arrangement head;
- `buzz context explore` renders the recovered contexts;
- a session record logged on the droplet syncs and appears at Nest;
- beacon pulses witnessed from the drill node;
- the hydration/gap witness events are in the journal;
- variant R only: replay equality against the checkpoint's journal head;
- principal-boundary coherence: synchronization, cursors, release selection,
  and checkpoints are PrincipalNode-owned; the RuntimeInstance and fresh host
  supply only selected execution and declared capabilities;
- teardown complete: droplet destroyed, drill identity's grant revoked,
  revocation observed by
  [coherence monitoring](coherence-monitoring-v0.1.md)'s
  declaration-coherence invariant.

## Drill residue

Like every test in this architecture, the drill reports into the journal:
start, variant, runtime tag, gap report, success/failure per criterion,
teardown. A drill that cannot write its own residue has failed regardless
of what else worked.

## Non-goals

- The droplet is not a production replica; if a standing cloud node is
  ever wanted, that is a deliberate promotion with its own declarations,
  not drill leftovers.
- No federation or multi-owner semantics.
- Not a performance benchmark; timings are recorded as observations, not
  criteria, in v0.1.

## Traceability

- Acceptance profile: [`node-host-boundary-v0.1.md`](node-host-boundary-v0.1.md)
  (`node-host-migration-v0.1`, principal context artifact, hydration)
- Principal Domain and Principal Node boundary:
  [`principal-node-boundary-v0.1.md`](principal-node-boundary-v0.1.md)
- Runtime channel: [`node-release-distribution-v0.1.md`](node-release-distribution-v0.1.md)
- Trust vocabulary: [`sovereign-sync-agreement-v0.1-draft.md`](sovereign-sync-agreement-v0.1-draft.md)
- Recovery procedure:
  [`../models/principal-node/sync-session.lifecycle.yaml`](../models/principal-node/sync-session.lifecycle.yaml)
- Sibling scale: [`coherence-monitoring-v0.1.md`](coherence-monitoring-v0.1.md)
- Drill-not-assertion precedent: recovery envelope rule in
  [`node-host-boundary-v0.1.md`](node-host-boundary-v0.1.md) passkey profile
