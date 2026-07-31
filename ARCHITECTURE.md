# Buzz Solo Architecture

One owner, many nodes, one journal. This document is the short map; the
authoritative detail lives in [`specs/architecture/`](specs/architecture/).

## Topology

```
┌──────────────────────────────┐        ┌──────────────────────────────┐
│  Sovereign node (laptop)     │        │  Rendezvous replica          │
│  buzz-local-relay            │◄──────►│  Cloudflare Durable Object   │
│  · verified signed events    │  sync  │  (cloudflare/portable-relay) │
│  · durable NDJSON journal    │        │  · declaration-rehydrated    │
│  · declaration-loaded trust  │        │    trust / exports / readers │
└──────────────┬───────────────┘        └──────────────▲───────────────┘
               │                                       │
      ┌────────▼────────┐                     ┌────────┴────────┐
      │  buzz (CLI)     │                     │  Peer nodes     │
      │  profile-driven │                     │  (other laptops)│
      └─────────────────┘                     └─────────────────┘
```

Every arrow carries ordinary signed Nostr events. There is no server-side
configuration of trust: **sync declarations** (provisionally kind 30700,
[sovereign-sync-agreement](specs/architecture/sovereign-sync-agreement-v0.1-draft.md))
are addressable events that declare which streams a peer may read or write,
and both the laptop relay and the Cloudflare adapter rehydrate peer trust,
exports, and readers from declaration heads at startup. A redeploy with
blank environment variables retains trust from the journal.

## The journal

The node's source of truth is an append-only NDJSON log of verified signed
events (`~/.buzz-local/sovereign.ndjson` in the standard install). Restarting
the relay recovers the effective history from the log. On top of it:

- **Context heads** — NIP-AE engrams (kind 30174, NIP-44 encrypted,
  slug-addressed). Only ciphertext leaves the machine.
- **Session records** — plaintext kind-1 events bounded by an `h` context,
  attested by the NIP-OA agent capability.
- **Handoffs** — a delegation lifecycle (`open → claim → return → close`)
  of ordinary kind-1 events inside the same `h`-scoped contexts, with
  content-addressed artifact custody and byte-verified returns
  ([journal-handoff-v0.1](specs/architecture/journal-handoff-v0.1.md)).
- **Beacon pulses** — kind 20700 witness statements emitted by both nodes
  ([beacon-pulse](specs/architecture/beacon-pulse-v0.2-draft.md)).

## The CLI

`buzz` is profile-driven (`~/.config/buzz/profiles/*.toml`): a profile
resolves identity roles (journal author, replication transport, relay
witness, steward, artifact roles), relays, the push cursor, and stream
selection. `buzz context …` covers load/save of context heads, session
records, sync, pulse, status, handoff lifecycle, artifact custody, and graph
projection. See [crates/buzz-cli/CONTEXT.md](crates/buzz-cli/CONTEXT.md).

## Crate map

**Solo center** — `buzz-local-relay` (relay + replication/handoff/pulse
bins) · `buzz-cli`

**Shared protocol** — `buzz-core` (zero-I/O types, NIP-01 filters, Schnorr
verify, kind registry) · `buzz-auth` (NIP-42/98 verification) · `buzz-sdk`
(typed event builders) · `buzz-ws-client` (NIP-42 WebSocket client) ·
`buzz-persona` (persona packs)

**Agent harness** — `buzz-acp` (ACP ↔ relay bridge) · `buzz-agent` ·
`buzz-dev-mcp` · `sprig` (all-in-one bundle)

**Interop** — `git-sign-nostr` · `git-credential-nostr` ·
`buzz-pair-relay` / `buzz-pairing-cli` (NIP-AB device pairing)

## Design principles

**Event kinds are the only switch.** Every action is a Nostr event with a
kind integer from `buzz-core/src/kind.rs`. Adding a feature means defining
a kind, not an endpoint.

**Declarations over configuration.** Anything governing inter-node behavior
(trust, exports, read grants) is a signed, addressable event that survives
redeploys and is auditable in the journal.

**Releases are signatures.** Node runtime releases are signed `node/vX.Y.Z`
git tags ([node-release-distribution](specs/architecture/node-release-distribution-v0.1.md));
there is no artifact pipeline to trust.

## Lineage

Derived from [block/buzz](https://github.com/block/buzz). The upstream
inheritance (clients, hosted multi-tenant relay, release machinery) was
removed in [the upstream peel](specs/architecture/buzz-solo-upstream-peel-v0.1.md);
everything remains recoverable from git history, and `upstream` stays a
read-only cherry-pick remote. Protocol compatibility discussion:
[block/buzz#3805](https://github.com/block/buzz/issues/3805).
