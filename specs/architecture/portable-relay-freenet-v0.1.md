# Portable Relay Freenet v0.1

Status: draft — design recorded, implementation not started
Date: 2026-07-30

## Decision

`portable-relay-freenet-v0.1` is a proposed third implementation of the
[portable relay boundary](portable-relay-boundary.md): an exported stream set
resident on the [Freenet](https://freenet.org) peer-to-peer network as a WASM
contract, bridged to the sovereign journal through the existing replication
source/sink ports.

Unlike the laptop and Cloudflare adapters, the Freenet adapter is **not a
client-facing relay endpoint in v0.1**. Its coordination atom is one
*exported stream set* — the same unit the drain legs and the rendezvous
replica move today — held as replicated, self-validating contract state.
The laptop journal remains the sole source of truth; the Freenet contract is
an expendable replica, exactly as the portable boundary already assumes of
every non-laptop leg.

This is a portability and sovereignty experiment, not a production-hosting
claim, and not a replacement for the Cloudflare rendezvous.

## Why this boundary exists

Every replica Buzz Solo runs today is either owned hardware (a laptop) or
rented platform (a Cloudflare Durable Object). Freenet offers a third
category: **network-resident state with no operator** — replicated by
interested peers, validated by the contract itself, addressed by the hash of
its code and parameters.

The fit is structural, not aspirational:

- Freenet contracts must "provide a mechanism to merge any two valid
  states" (summary/delta exchange, CRDT-like by convention). Buzz Solo's
  effective-event reducer **is already that mechanism**: regular events form
  a grow-only set keyed by event ID; replaceable and parameterized
  replaceable events are last-write-wins on `(author, kind[, d])` with
  `created_at` ordering and event-ID tie-break. The portable boundary's
  reducer semantics translate to `update_state` without redesign.
- Sync declarations (provisionally kind 30700,
  [sovereign-sync-agreement](sovereign-sync-agreement-v0.1-draft.md)) become
  **enforcement instead of configuration**: the contract's validation
  admits an update only if it is signed by a currently-declared grantee.
  Today the Cloudflare adapter rehydrates trust from declaration heads and
  applies it in adapter code; on Freenet the network itself refuses
  undeclared writes.
- Freenet subscriptions (`SUB`/`UPD` push on contract state change) give the
  drain leg an event-driven feed where the rendezvous leg polls on a
  schedule today.
- Freenet **delegates** (local-only WASM holding key material, per-caller
  attested) are the structural sibling of the node profile's identity roles
  and the NIP-OA agent capability.

No Nostr-on-Freenet bridge exists anywhere as of 2026-07-30 (verified:
the only public crossover is freenet-core discussion #619). This spec, if
implemented, is first of its kind — the same posture as the kind-30700
proposal in [block/buzz#3805](https://github.com/block/buzz/issues/3805).

## Platform snapshot (informative, volatile)

Recorded 2026-07-30; re-verify before implementation:

- Network publicly live since ~2026-03; freenet-core v0.2.115 with
  near-daily patch releases; contract API **pre-1.0 and still moving**
  (the Locutus-era `validate_delta` is gone; deltas arrive via
  `Vec<UpdateData>` into `update_state`).
- Contract interface (`freenet-stdlib` 0.8.5): `validate_state`,
  `update_state`, `summarize_state`, `get_state_delta` over raw byte
  wrappers; contract key = BLAKE3 hash of WASM code **plus** parameters.
- Node API: local process, WebSocket `/v1/contract/command` (default
  `:7509`); `freenet local` runs an isolated dev node. No documented
  supported pattern for embedding the node in-process.
- No hard documented per-contract state size limit; peers evict contracts
  with poor request-rate-to-cost ratios. Cross-contract reads not yet
  shipped.

## Contract design

### Parameters

```text
{
  owner_pubkey:  32-byte x-only BIP-340 public key (the stream owner),
  stream_label:  operator-assigned source stream ID (policy label, not a
                 credential — mirrors the replication port rule),
  boundary_h:    optional h-tag context the stream is scoped to
}
```

Different parameters → different contract instance → different network key.
One contract per exported stream set keeps the coordination atom aligned
with the replication port's unit.

### State and merge

State is the effective event set of the exported stream, exactly as the
portable reducer defines it:

- regular events: grow-only set keyed by event ID;
- replaceable / parameterized replaceable: one winner per
  `(author, kind[, d])`, ordered by `created_at`, event-ID tie-break;
- ephemeral kinds `20000..29999`: **never enter contract state** (the
  replication port already forbids exporting them; beacon pulses stay
  off-Freenet).

`summarize_state` returns the event-ID set of regular events (compact
encoding) plus the `(address, created_at, event_id)` head of each
replaceable address. `get_state_delta` returns exactly the signed envelopes
the summary lacks or supersedes. `update_state` verifies and merges. Merge
is commutative, associative, and idempotent by construction — the reducer
invariants the conformance suite already checks.

### Validation

`validate_state` / `update_state` admit an event only when all hold:

1. the event ID and BIP-340 Schnorr signature verify (the same
   verification `buzz-core` performs; requires `secp256k1` compiled to
   `wasm32-unknown-unknown` inside the contract — a build-feasibility
   gate for phase 1);
2. the author is the `owner_pubkey`, **or** is granted by the current
   declaration head in state;
3. the event carries the contract's `boundary_h` when one is declared.

Declaration ordering rule: within one update batch, kind-30700 declaration
events signed by `owner_pubkey` are applied before all other admission
checks, so a batch may carry a new declaration plus events it admits.
Revocation is not retroactive: events admitted under an earlier declaration
remain in state (history is signed and immutable); a revoked grantee simply
cannot add more.

Spam resistance falls out of admission: the contract accepts writes only
from declared keys, so Ghost Keys and other network-level anti-flood
primitives are unnecessary for this contract class.

### Confidentiality rule

Freenet contract state is world-readable. Therefore the export policy is
fail-closed: **a stream may be exported to a Freenet contract only if every
event in it is ciphertext (NIP-AE engrams, NIP-44 payloads) or explicitly
marked public by the owner.** Grantee-scoped plaintext reads (session
records under read grants) stay on legs that enforce read policy — the
laptop and the Cloudflare identity profile. Encryption is the only read
control on this leg; the spec makes that explicit rather than implied.

## Bridge adapter

A Rust bridge (candidate home: a `buzz-local-relay` bin alongside
`buzz-relay-pull`, e.g. `buzz-freenet-sync`) connects the laptop node to a
local Freenet node over `/v1/contract/command` and implements both
replication ports:

- **source → contract**: reads bounded records in journal order from the
  laptop replication source, submits them as contract updates, persists the
  opaque cursor only after the node acknowledges the update — the same
  checkpoint-safety rule as every other leg;
- **contract → sink**: subscribes to the contract; incoming deltas pass
  through the destination's normal pipeline (independent ID + signature
  verification, duplicate/replacement classification, durable append) —
  Freenet acceptance never bypasses destination verification, mirroring the
  replication port's fail-closed rule.

The bridge is an orchestrator over declared ports; nothing inside
`buzz-core` learns about Freenet. Runtime dependencies stay outside the
portable layer, exactly as the boundary demands of Cloudflare APIs today.

A delegate holding the replication-transport key (signing updates without
exposing the key to the bridge UI surface) is a phase-3 exploration, not a
v0.1 requirement — the bridge may initially sign with the profile's
existing replication identity.

## Capability claim

An implementation MUST NOT claim `portable-relay-core-v0.1` in v0.1 — the
contract is not a NIP-01/HTTP relay surface. It targets:

- `portable-relay-replication-v0.1` (both directions, evidenced against the
  laptop reference);
- `portable-relay-freenet-v0.1` (this profile):
  - contract state equals the effective set the laptop reducer produces for
    the same exported records, byte-exact on signed envelopes;
  - merge of any two independently-evolved valid states converges;
  - undeclared-author updates are refused by the contract, not the bridge;
  - ephemeral kinds and non-ciphertext gated streams are refused at export;
  - cursor resume survives bridge and Freenet node restarts.

A client-facing NIP-01 read surface backed by contract state (which would
put `portable-relay-core-v0.1` in reach) is a named later phase, not part
of this claim.

## Phases

1. **Feasibility spike** — compile BIP-340 verification + the reducer to
   `wasm32-unknown-unknown` under `freenet-stdlib`; measure contract size
   and per-event validation cost in wasmtime. Abort criteria: secp256k1
   verification impractical in the contract sandbox.
2. **Local-node prototype** — contract + bridge against `freenet local`;
   run the shared signed conformance vector through the round trip
   (laptop → contract → second laptop profile) and diff effective sets
   against `portable_conformance.rs` expectations.
3. **Network experiment** — publish one ciphertext-only stream contract to
   the live network; measure propagation, subscription latency versus the
   five-minute drain schedule, and eviction behavior over idle weeks.
   Availability finding to record honestly: a solo owner's contract has an
   audience of one, so liveness with the laptop offline likely requires an
   always-on subscribed peer — the same presence requirement the
   Cloudflare rendezvous satisfies today, minus the platform.
4. **Later, separately named** — NIP-01 read surface over contract state;
   handoff lifecycle as its own contract class (the
   [journal-handoff](journal-handoff-v0.1.md) reducer's
   open→claim→return→close rules expressed as update validation);
   delegate-held identity roles.

## Non-goals

- Replacing the laptop journal as source of truth, in any phase.
- Replacing the Cloudflare rendezvous while Freenet's contract API is
  pre-1.0 and eviction behavior is unmeasured.
- Artifact custody on Freenet (undocumented state size limits; R2-backed
  custody stays as specified).
- Anonymity claims (Freenet itself makes none).
- Any federation, discovery, or multi-owner semantics.

## Risks

- **API churn**: the contract trait has already changed shape since the
  Locutus era; pin `freenet-stdlib` and expect rework before 1.0.
- **Eviction**: interest-based replication may drop an audience-of-one
  contract; phase 3 exists to measure this, not assume it away.
- **First-of-kind**: no prior Nostr-on-Freenet art to lean on; validation
  subtleties (e.g. declaration rotation ordering) have no reference
  implementation to compare against — the conformance vector is the only
  ground truth.

## Traceability

- Boundary: [`portable-relay-boundary.md`](portable-relay-boundary.md)
  (replication source/sink ports, reducer invariants)
- Trust vocabulary:
  [`sovereign-sync-agreement-v0.1-draft.md`](sovereign-sync-agreement-v0.1-draft.md)
- Handoff reducer (phase-4 candidate):
  [`journal-handoff-v0.1.md`](journal-handoff-v0.1.md)
- Sibling adapter:
  [`portable-relay-cloudflare-v0.1.md`](portable-relay-cloudflare-v0.1.md)
- Conformance reference:
  `crates/buzz-local-relay/tests/portable_conformance.rs`,
  `crates/buzz-local-relay/tests/replication_port.rs`
- Platform (volatile, re-verify): freenet.org/build/manual
  (contract-interface, delegates, architecture, tutorial), docs.rs
  `freenet-stdlib` 0.8.5, github.com/freenet/freenet-core (releases,
  discussion #619)
