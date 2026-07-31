<h1 align="center">Buzz Solo 🐝</h1>

<p align="center">
  <strong>A sovereign personal node — one owner, many nodes, one journal.</strong>
</p>

<p align="center">
  <a href="specs/TELOS.md">Telos</a> ·
  <a href="specs/architecture/">Specs</a> ·
  <a href="crates/buzz-local-relay/README.md">Local relay</a> ·
  <a href="crates/buzz-cli/CONTEXT.md">Context CLI</a> ·
  <a href="LICENSE">Apache 2.0</a>
</p>

---

## What this is

Buzz Solo is a solo-first derivative of [Block's Buzz](https://github.com/block/buzz).
Upstream builds a workspace: one relay, one community, many members. This
project inverts it: **one owner, many nodes, one journal**. Your laptop runs a
durable relay that is the source of truth for your working context; replicas
(another laptop, a Cloudflare Durable Object rendezvous) carry the streams you
explicitly declare, governed by signed declarations rather than server
configuration.

Everything is ordinary Nostr events, signed and verifiable. The fork stays
protocol-compatible with upstream; the shared-vocabulary conversation lives in
[block/buzz#3805](https://github.com/block/buzz/issues/3805).

## What runs today

- **`buzz-local-relay`** — a durable single-process relay: verified signed
  events, portable NDJSON persistence, no Postgres/Redis/Docker. Stop it,
  restart it, and the effective history recovers from the log.
- **`buzz-cli`** — the profile-driven node CLI. Encrypted context heads
  (NIP-AE engrams), h-bounded session records, journal sync to replicas,
  witness beacon pulses, declaration-governed status, content-addressed
  artifact custody, and the journal **handoff lifecycle**
  (`open → claim → return → close`) for delegating work across nodes.
- **`cloudflare/portable-relay`** — the Durable Object rendezvous replica.
  Peer trust, exports, and readers rehydrate from declaration heads
  (provisionally kind 30700); a redeploy with blank env vars retains trust
  from the journal.
- **Contracts** — relay-free contract suites for the handoff lifecycle and
  the context graph (`scripts/test-buzz-handoff-contract.sh`,
  `scripts/test-buzz-ctx-graph.sh`), run on every CI push.
- **Node releases** — signed `node/vX.Y.Z` git tags per
  [node-release-distribution-v0.1](specs/architecture/node-release-distribution-v0.1.md).
  No release pipeline, no artifacts service: the tag signature is the channel.

## Quick start

Needs [Hermit](https://cashapp.github.io/hermit/) (or Rust 1.88+ and `just`).
No Docker, Postgres, or Redis.

```bash
git clone https://github.com/slusset/buzz-solo.git && cd buzz-solo
. ./bin/activate-hermit
just local-relay
```

The relay listens on `ws://127.0.0.1:3000` and writes verified signed events
to `.buzz-local/events.ndjson`. Use `--ephemeral` for a disposable in-memory
run, or `--data /path/to/events.ndjson` to place the portable log elsewhere.

Build the node CLI:

```bash
cargo build --release -p buzz-cli
./target/release/buzz context doctor
```

See the [local relay guide](crates/buzz-local-relay/README.md) and the
[managed context CLI guide](crates/buzz-cli/CONTEXT.md) for profiles,
identity roles, migration, and replication.

## Specs

The project is spec-driven; design lands in [`specs/architecture/`](specs/architecture/)
before (or with) code. Start with:

- [portable-relay-boundary](specs/architecture/portable-relay-boundary.md) —
  the node/replica boundary
- [journal-handoff-v0.1](specs/architecture/journal-handoff-v0.1.md) —
  cross-node work delegation
- [sovereign-sync-agreement-v0.1](specs/architecture/sovereign-sync-agreement-v0.1-draft.md) —
  declaration-governed relay-to-relay trust
- [node-release-distribution-v0.1](specs/architecture/node-release-distribution-v0.1.md) —
  signed-tag runtime releases
- [buzz-solo-upstream-peel-v0.1](specs/architecture/buzz-solo-upstream-peel-v0.1.md) —
  how this fork is shedding its upstream inheritance

## CI

Five lanes, all unconditional: Rust lint, unit tests (including the
`buzz-local-relay` + `buzz-cli` suites), the sovereign contracts, the
Cloudflare portable-relay checks, and `cargo-deny`. That is the whole
surface — if CI is green, the node is sound.

## Lineage

The upstream inheritance — desktop, mobile, and web clients, the hosted
multi-tenant relay stack, and Block's release machinery — was removed in
[the upstream peel](specs/architecture/buzz-solo-upstream-peel-v0.1.md);
everything stays recoverable from git history. `upstream` (`block/buzz`)
remains a read-only remote for selective cherry-picks, and the protocol
conversation continues at
[block/buzz#3805](https://github.com/block/buzz/issues/3805).

---

<p align="center">
  <sub>Buzz Solo 🐝</sub><br>
  <sub>Apache 2.0 · Derived from <a href="https://github.com/block/buzz">Buzz</a> by Block, Inc.</sub>
</p>
