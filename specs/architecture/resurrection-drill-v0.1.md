# Resurrection Drill v0.1

Status: draft
Date: 2026-07-31

## Decision

The strongest claim this architecture makes — *the node can be reborn from
identity plus network* — is verified by a scheduled drill, not asserted.
The drill stands up the sovereign context on a **fresh rented host** (a
Digital Ocean droplet is the reference target) and is the named acceptance
test for the `node-host-migration-v0.1` conformance profile in
[node-host-boundary-v0.1](node-host-boundary-v0.1.md). It also forces the
first real second host adapter: Linux, systemd user units, XDG placement —
the capability manifest earning its keep on unfamiliar soil.

A resurrection performed once is an anecdote; performed on schedule, it is
a property. Cadence: **quarterly minimum**, and after any change to
hydration, custody, or replication code paths.

## Two variants, two claims

The drill runs both variants; they verify different things and their
difference *is* one of the measurements.

### Variant R — restore (disaster recovery)

Carry the node context artifact to the fresh host and hydrate:

1. Provision the droplet; install nothing by hand beyond the bootstrap.
2. Fetch the runtime by its signed `node/vX.Y.Z` tag; **verify the tag
   signature before executing anything**
   ([node-release-distribution](node-release-distribution-v0.1.md)).
3. Transport the node context artifact (manifest + journal + sealed
   envelope) via an existing authenticated leg (artifact custody or
   direct copy).
4. Hydrate per the host boundary: placement → release and host-capability
   compatibility gate → journal replay verified against the manifest's
   `journal_head` (fail closed on mismatch) → custody provisioning →
   supervision registration of the composed node runtime.
5. Append a witnessed `node-hydrated` record from the new host.

**Claim proven**: laptop dies, backup exists → node reborn byte-exact.

### Variant K — key-only (resurrection from the network)

Arrive with nothing but key material:

1. Provision the droplet; fetch and verify the runtime as above.
2. Instantiate a profile from the carried identity; authenticate to
   Rendezvous.
3. Start the node runtime with the fresh host adapter and request recovery.
   The runtime evaluates current declarations and drains every stream the
   identity is entitled to read into the empty journal through its ordinary
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

- The **owner root key never touches rented metal.** It stays on
  hardware at Nest. This is a rule, not a recommendation: a drill run
  that places root material on the droplet is an automatic failure.
- Each drill mints a **scoped drill identity**; a declaration grants it
  read on the exported streams for the drill window. This exercises the
  kind-30700 vocabulary exactly as proposed upstream: trust as signed,
  auditable data.
- Teardown **revokes the grant** — so every drill also exercises
  revocation, the half of the declaration lifecycle that otherwise never
  runs in anger.
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
- variant R only: replay equality against the manifest's journal head;
- runtime-boundary coherence: synchronization and cursors are node-owned and
  the fresh host supplies only declared capabilities;
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
  (`node-host-migration-v0.1`, node context manifest, hydration)
- Runtime boundary:
  [`node-runtime-boundary-v0.1.md`](node-runtime-boundary-v0.1.md)
- Runtime channel: [`node-release-distribution-v0.1.md`](node-release-distribution-v0.1.md)
- Trust vocabulary: [`sovereign-sync-agreement-v0.1-draft.md`](sovereign-sync-agreement-v0.1-draft.md)
- Recovery procedure:
  [`../models/node-runtime/sync-session.lifecycle.yaml`](../models/node-runtime/sync-session.lifecycle.yaml)
- Sibling scale: [`coherence-monitoring-v0.1.md`](coherence-monitoring-v0.1.md)
- Drill-not-assertion precedent: recovery envelope rule in
  [`node-host-boundary-v0.1.md`](node-host-boundary-v0.1.md) passkey profile
