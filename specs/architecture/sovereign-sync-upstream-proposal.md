# Upstream proposal: sovereign sync agreements (posted as block/buzz#3805)

Source text for the upstream proposal, posted 2026-07-30 as
[block/buzz#3805](https://github.com/block/buzz/issues/3805)
("Proposal: a declaration vocabulary for relay-to-relay trust
(kind 30700)") after the prior venue, draft PR #2997 (portable relay
boundary RFC), was closed. The issue supersedes this file where they
drift. Everything claimed as "running" has live evidence on
`slusset/buzz#feature/local-relay`.

---

## Proposal: a declaration vocabulary for relay-to-relay trust (kind 30700)

This continues the earlier thread about sync between independently-owned
relays. We've been running an implementation for a while now and the
vocabulary has stabilized enough to propose for the shared registry.

Context note: I've closed my draft RFC PR #2997 (portable relay
boundary: laptop + Cloudflare reference adapters). That branch has grown
into a solo-first derivative of Buzz (working name: **Buzz Solo**)
rather than a mergeable change set — one owner, many
nodes, one journal, versus upstream's one relay, one community, many
members. It stays public and protocol-compatible on the fork; the
compatibility conversation moves here, and the asks below are
deliberately narrow.

### What it is

A **sync declaration** is one addressable event (provisionally kind
`30700`) expressing one half of a relay-to-relay relationship:

- `d = "<role>/<stream-id>"` where role is one of `export`, `admit`,
  `read`, `key-grant`, `steward`
- `n` tag names the **node** the declaration governs (one owner runs many
  nodes; journals replicate whole, so every copy carries declarations for
  every node and each node evaluates only its own)
- `p` tags carry counterparty/verification pubkeys; `e` tag pins the
  counterparty's declaration head (drift between heads breaks the match
  visibly)
- content carries `status`, a stable `principal` label, and role-specific
  fields (stream selection for `export`, powers for `steward`)

An **agreement** is a matched pair of declarations, one signed by each
party. No countersignature envelope: each side signs its own half, both
halves replicate over the streams they govern, and an unmatched or
drifted pair is an *observable* state rather than a hidden config
difference.

### Why upstream might care

1. **Config-as-events.** Our adapters derive their peer trust, stream
   exports, and reader grants by evaluating owner-signed declaration
   heads in their own store, with env/file config demoted to bootstrap.
   Operator trust survives redeploys because it lives in the journal, and
   every trust change is a signed, attributable, revocable event. The
   relay-mesh work already gives runtimes attested identities — this is
   the same idiom one level up, between independently-owned relays.
2. **It degrades to plain events.** Everything here is NIP-01 addressable
   events + filters. A relay that doesn't speak the vocabulary still
   custodies and serves it correctly. No new transport is required;
   replication ports are an optimization.
3. **Bindings map onto NIP-29.** A relationship between two relays is a
   two-member shared context scoped by an `h` tag; a community relay is
   the N-member case of the same object. An unmodified Buzz client can
   open a relay-to-relay binding as a channel. Solo-first nodes and
   community-first nodes join each other with one vocabulary.
4. **Blob custody inherits the vocabulary.** Artifact access follows
   reference: a content-addressed blob is served only to principals
   holding a read grant on a stream whose events `x`-tag it. No separate
   artifact ACLs.

### Running evidence (fork)

Updated since the earlier thread:

- Two independently-keyed laptops + a Cloudflare DO rendezvous run
  bidirectional selective sync governed entirely by declarations; the
  offer → admit → pin → drift → re-pin lifecycle is exercised end to
  end, and three shared streams now drain on a five-minute schedule
  under the same grants.
- Laptop relay and the Cloudflare adapter both rehydrate peer trust /
  exports / readers from declaration heads; a production redeploy with
  blank env vars retained trust from the journal.
- R2-backed artifact custody with reference-gated access is live behind
  the same grants.
- A delegation lifecycle (`open → claim → return → close`, ordinary
  kind-1 events inside the same `h`-scoped contexts) has run
  cross-node: work opened on one laptop was executed, returned with
  content-addressed result artifacts, independently byte-verified from
  the second laptop, and causally closed from there.
- A signed node-runtime release channel is live on the fork:
  `node/vX.Y.Z` annotated tags signed with the node key (BIP-340,
  verified by exact pinned pubkey). The consumer node verified a
  release, caught a real defect, reported it as a signed consumer
  record, and the fix shipped and was accepted — the distribution loop
  closed round-trip inside a day. Keys, journals, cursors, and profiles
  never travel through this channel.
- Enforcement strictness in practice (ask 4 below): we lean on
  observability. A read-only steward reports agreement drift (unmatched
  offers, unscoped exports) and lifecycle findings from the journal
  alone; invalid pre-hardening delegation records were retired via
  exact-event-ID archival acknowledgments rather than grandfathered —
  the archival state is observable and distinct from a valid close.
- Full draft spec: `specs/architecture/sovereign-sync-agreement-v0.1-draft.md`
  on the fork.

### Asks

1. **Registry**: assign (or bless the provisional) `30700` for sync
   declarations in `buzz-core/src/kind.rs`.
2. **Grammar**: comment on the `d = role/stream` + `n`-tag convention —
   especially whether the node-scoping tag collides with any planned use
   of `n`.
3. **NIP-29 mapping**: sanity-check the shared-context-as-binding framing
   with the group-model owners.
4. **Enforcement strictness**: the open question from before — should a
   relay refuse unmatched streams, or serve them and surface the
   mismatch? We currently treat strictness as adapter policy and lean on
   observability (a read-only steward agent reports drift); interested in
   whether upstream wants a normative position.

Happy to carve any of this into a NIP-style doc under `docs/nips/` if
there's appetite.

Related but out of scope here: the fork also carries a draft
harness-neutral spec chain for portable agent-session context hooks;
that conversation belongs under #3780 — pointer included only so the
pieces are visible together.
