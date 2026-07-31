# AGENTS.md — AI Agent Contributor Guide

This guide is for AI agents contributing to the Buzz Solo codebase. It covers
agent-specific context and conventions. For general contributor info (setup,
code style, PR process, architecture), see [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Ecosystem

This repo (`slusset/buzz-solo`) is the whole project: a solo-first derivative
of [block/buzz](https://github.com/block/buzz) — one owner, many nodes, one
journal. There is no companion build or deploy repo. The runtime surfaces are:

| Surface | Where |
|---------|-------|
| Sovereign node (durable local relay) | `crates/buzz-local-relay` |
| Node CLI (context, journal, handoffs, sync) | `crates/buzz-cli` |
| Cloudflare rendezvous replica | `cloudflare/portable-relay` |
| Node runtime releases | signed `node/vX.Y.Z` git tags ([spec](specs/architecture/node-release-distribution-v0.1.md)) |

`upstream` (`block/buzz`) is a read-only remote kept for selective
cherry-picks; upstream `main` is not a merge source. The desktop, mobile, and
web clients left the tree in phase 3 of
[the peel](specs/architecture/buzz-solo-upstream-peel-v0.1.md) — resurrect
from git history if a client surface returns. The hosted multi-tenant relay
stack remains in-tree pending its own decision.

---

## Repo Structure

```
crates/
  # Solo center
  buzz-local-relay    # Durable single-process relay — verified events, NDJSON log
  buzz-cli            # Agent-first node CLI — context, journal, handoffs, sync
  # Shared protocol + client plumbing
  buzz-core           # Core types, event verification, filter matching, kind registry
  buzz-sdk            # Typed Nostr event builders
  buzz-ws-client      # Shared NIP-42 WebSocket client (connect, auth, publish)
  buzz-persona        # Agent persona packs
  # Hosted relay stack (stays pending its own peel decision)
  buzz-relay          # WebSocket relay server; also hosts git + huddle audio
  buzz-db             # Postgres event store and data access layer
  buzz-auth           # Authentication and authorization
  buzz-pubsub         # Redis pub/sub fan-out, presence, typing indicators
  buzz-search         # Postgres FTS full-text search
  buzz-audit          # Hash-chain audit log
  buzz-media          # Blossom/S3 media storage
  buzz-workflow       # YAML-as-code workflow engine (evalexpr conditions)
  buzz-conformance    # Multi-tenant replay conformance checker
  buzz-push-gateway   # Push notification gateway
  buzz-relay-mesh     # Mesh compute routing (buzz-relay dependency)
  buzz-admin          # Operator CLI for relay administration
  buzz-test-client    # Integration test client and E2E test suite
  # Agent harness + interop
  buzz-acp            # ACP harness bridging Buzz events to AI agents
  buzz-agent          # Minimal ACP-compliant agent
  buzz-dev-mcp        # Developer MCP server — shell + file-edit tools
  sprig               # All-in-one harness bundling ACP, agent, and dev MCP
  buzz-pair-relay     # Ephemeral sidecar relay for NIP-AB device pairing
  buzz-pairing-cli    # CLI for NIP-AB device pairing interop testing
  git-sign-nostr      # Sign git objects with a Nostr key
  git-credential-nostr # Git credential helper for Nostr-authed push/fetch

admin-web/            # Read-only relay admin dashboard (hosted stack)
cloudflare/portable-relay/  # Durable Object rendezvous replica
specs/                # Telos + architecture specs — design lands here first
migrations/           # SQL migrations (auto-applied on relay startup)
schema/               # Desired-state schema for the hosted relay
scripts/              # Dev tooling + sovereign contract suites
examples/             # Example bots (countdown-bot)
deploy/compose/       # Self-host Compose bundle for the hosted relay
.env.example          # Config template — copy to .env before running
```

---

## Getting Started

```bash
. ./bin/activate-hermit   # activate hermit toolchain (Rust, Node, etc.)
just local-relay          # durable Solo relay at ws://127.0.0.1:3000 — no Docker
just ci                   # run before any PR
```

For the hosted relay stack (Postgres/Redis/MinIO via Docker):

```bash
cp .env.example .env
just setup
just relay                # hosted relay at ws://localhost:3000
```

---

## Quality Gates

Run `just ci` before every PR — it runs `check` (fmt + clippy + cloudflare +
sovereign contracts) + `test-unit` + `security` (cargo-deny). This mirrors the
five CI lanes exactly; if `just ci` passes, CI passes.

Run `just test` for integration tests if you touched `buzz-relay`,
`buzz-db`, or `buzz-auth` — these require a running Postgres and Redis.

**Pre-commit hooks** are installed by `just setup` (or `just hooks`) and
auto-fix Rust formatting via `stage_fixed`. **Pre-push hooks** run branch-skew,
`test-unit`, the sovereign contracts, and the Cloudflare checks in parallel.
Before agents run Git or hooks, activate the repo's Hermit environment
(`. ./bin/activate-hermit`); do not rewrite hook commands to compensate for an
unconfigured shell `PATH`.

**Commit with `git commit -s`.** The required **DCO Check** fails any PR with a
commit missing a `Signed-off-by` trailer, and `just hooks` installs a
`commit-msg` hook that adds it to commits you create locally (`git rebase` and
`git cherry-pick` still need `--signoff`) — if you build commit commands
programmatically, include `-s` every time. To repair a branch that already has
unsigned commits: `git rebase --signoff <base>`, then force-push.

Additional rules:
- No `unsafe` code
- Do not introduce new `unwrap()` or `expect()` in production paths — use `?` and proper error types
- New public API must have doc comments

---

## Key Patterns

**Nostr-first HTTP surface**: The primary API is NIP-29 over WebSocket. The
relay also exposes a narrow HTTP surface: NIP-11/NIP-05 metadata,
`POST /events`, `POST /query`, `POST /count`, workflow webhooks at
`/hooks/{id}`, Blossom media, git smart HTTP, git policy hooks, and health
probes. These HTTP paths all preserve the same host-derived community boundary.

**Prefer Nostr events over new HTTP endpoints**: For new feature work, model
the operation as a Nostr event (new kind in `buzz-core/src/kind.rs`, handler
in the relay) rather than adding endpoint-specific JSON APIs. If you find
yourself reaching for a new HTTP endpoint, first check whether an event kind
would do the job — it usually will, and you get realtime fan-out, NIP-29
scoping, and the existing auth pipeline for free.

Reference https://github.com/nostr-protocol/nips

**Event kinds**: All event kind integers are defined in
`buzz-core/src/kind.rs`. New features get new kind integers — add them here
first, then implement handling in the relay.

**Channel scoping**: Channels use `h` tags (NIP-29 group tag), not `e` tags.
Filters and queries must scope to `h` tags when operating within a channel.
Sovereign journal records carry the same `h` boundary
(`context.default_h` in the node profile).

**Agent-facing operations go in `buzz-cli`**: New agent-facing features belong
in `buzz-cli` — add a subcommand there first, then wire the REST/WebSocket
call in `client.rs`. `buzz-dev-mcp` (shell + file tools for `buzz-agent`) is
separate.

**Specs before code**: Design lands in `specs/architecture/` before (or with)
the implementation. Sovereign-surface changes (declarations, handoffs,
engrams, replication) must stay consistent with their spec; update the spec in
the same PR when behavior changes.

**Workflow conditions**: `buzz-workflow` uses
[evalexpr](https://docs.rs/evalexpr) for condition evaluation. Keep expressions
simple and testable.

**Thread counters**: `reply_count` and `descendant_count` are materialized on
thread root events. Any code that inserts replies must update these counters —
check existing reply handlers for the pattern.

---

## Agent CLI (`buzz-cli`)

`buzz` is the agent-first CLI. Auth env vars
(`BUZZ_RELAY_URL`, `BUZZ_PRIVATE_KEY`, `BUZZ_AUTH_TAG`) are auto-injected
by the ACP harness into managed agent subprocesses. In development, set
`BUZZ_PRIVATE_KEY` and `BUZZ_RELAY_URL` in your environment manually.
Sovereign-node operation is profile-driven
(`~/.config/buzz/profiles/*.toml`) — see
[crates/buzz-cli/CONTEXT.md](crates/buzz-cli/CONTEXT.md).

### Building the CLI

```bash
cargo build --release -p buzz-cli
```

Binary location: `./target/release/buzz`. Add `./target/release` to `PATH`
or invoke with the full path.

All reads return sig-stripped JSON arrays; all writes return
`{event_id, accepted, message}`; creates add the entity ID. Exit codes:
0=ok, 1=input error, 2=network/relay, 3=auth, 4=other, 5=write conflict (NIP-33 LWW).

`--format compact` is a **global** flag — it goes before the subcommand:
`buzz --format compact channels list`, NOT `buzz channels list --format compact`.

See `crates/buzz-cli/TESTING.md` for the full live-testing runbook.

---

## Testing

```bash
just test-unit    # unit tests incl. the Solo center — no infrastructure needed
just test         # full integration suite (requires Postgres + Redis)
just handoff-check graph-check   # sovereign contract suites (relay-free)
```

E2E tests live in `crates/buzz-test-client/tests/`:
- `e2e_relay.rs` — WebSocket relay protocol
- `e2e_media.rs` — media upload/download (Blossom)
- `e2e_media_extended.rs` — extended media scenarios
- `e2e_nostr_interop.rs` — Nostr interop (NIP-50 search, NIP-10 threads, NIP-17 gift wraps)

See [TESTING.md](TESTING.md) for the live local-relay runbook.

---

## Common Gotchas

1. **Kind `39000` for channel metadata, not `41`** — kind 41 is NIP-01 (unused). All kinds defined in `buzz-core/src/kind.rs`.
2. **Relay queries must specify `kinds`** — omitting `kinds` triggers the p-gate (403). Always include explicit kind filters.
3. **`messages search` must include `--kinds`** — an open-ended search (no kinds) hits the relay p-gate and returns 403. Pass at least `--kinds 9,45001,45003` to scope the query.
4. **Worktrees: `cd` in the same command** — shell CWD doesn't persist between tool calls. Use `cd /path && cargo build` as one command.
5. **`echo` with a bare `=`-prefixed word breaks under zsh** — zsh expands `=foo` as a command path. Quote such arguments in scripts meant for interactive shells.

---

## See Also

- [CONTRIBUTING.md](CONTRIBUTING.md) — setup, code style, PR process, how to add event kinds / CLI subcommands
- [TESTING.md](TESTING.md) — live relay + CLI testing runbook
- [ARCHITECTURE.md](ARCHITECTURE.md) — system design and component relationships
- [specs/architecture/](specs/architecture/) — the spec set, including [the upstream peel](specs/architecture/buzz-solo-upstream-peel-v0.1.md)
- [README.md](README.md) — project overview and quick start
