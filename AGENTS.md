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
cherry-picks; upstream `main` is not a merge source. The upstream
inheritance — clients, the hosted multi-tenant relay stack, and release
machinery — left the tree across the phases of
[the peel](specs/architecture/buzz-solo-upstream-peel-v0.1.md); resurrect
from git history if a surface returns.

---

## Repo Structure

```
crates/
  # Solo center
  buzz-principal-node  # Technology-neutral Principal Node application services
  buzz-local-relay    # Durable single-process relay — verified events, NDJSON log
  buzz-cli            # Agent-first node CLI — context, journal, handoffs, sync
  # Shared protocol + client plumbing
  buzz-core           # Core types, event verification, filter matching, kind registry
  buzz-auth           # NIP-42/98 verification
  buzz-sdk            # Typed Nostr event builders
  buzz-ws-client      # Shared NIP-42 WebSocket client (connect, auth, publish)
  buzz-persona        # Agent persona packs
  # Agent harness + interop
  buzz-acp            # ACP harness bridging Buzz events to AI agents
  buzz-agent          # Minimal ACP-compliant agent
  buzz-dev-mcp        # Developer MCP server — shell + file-edit tools
  sprig               # All-in-one harness bundling ACP, agent, and dev MCP
  buzz-pair-relay     # Ephemeral sidecar relay for NIP-AB device pairing
  buzz-pairing-cli    # CLI for NIP-AB device pairing interop testing
  git-sign-nostr      # Sign git objects with a Nostr key
  git-credential-nostr # Git credential helper for Nostr-authed push/fetch

cloudflare/portable-relay/  # Durable Object rendezvous replica
specs/                # Telos + architecture specs — design lands here first
scripts/              # Sovereign runtime scripts + contract suites
examples/             # countdown-bot, meadow-core persona pack
.env.example          # Dev env template for local relay + CLI runs
```

---

## Getting Started

```bash
. ./bin/activate-hermit   # activate hermit toolchain (Rust, Node, etc.)
just init-dev-profile
just local-relay          # isolated XDG dev relay at ws://127.0.0.1:3100
just ci                   # run before any PR
```

---

## Quality Gates

Run `just ci` before every PR — it runs `check` (fmt + clippy + cloudflare +
sovereign contracts) + `test-unit` + `security` (cargo-deny). This mirrors the
five CI lanes exactly; if `just ci` passes, CI passes.

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

**Events over endpoints**: model every operation as a signed Nostr event
(new kind in `buzz-core/src/kind.rs`) rather than an HTTP API. The local
relay exposes only the core Nostr event/query surface plus health probes.

Reference https://github.com/nostr-protocol/nips

**Event kinds**: all kind integers are defined in `buzz-core/src/kind.rs`.
New features get new kind integers — add them here first.

**Context boundary**: sovereign journal records carry an `h` tag
(`context.default_h` in the node profile); filters and queries scope to it.

**Agent-facing operations go in `buzz-cli`**: add a subcommand first, then
wire the relay call in `client.rs`. `buzz-dev-mcp` (shell + file tools for
`buzz-agent`) is separate.

**Specs before code**: design lands in `specs/architecture/` before (or
with) the implementation. Sovereign-surface changes (declarations, handoffs,
engrams, replication) must stay consistent with their spec; update the spec
in the same PR when behavior changes.

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

Binary location: `./target/release/buzz`. For an installed runtime, use
`just node-build` and `just node-install`; do not copy binaries into the
durable data directory.

All reads return sig-stripped JSON arrays; all writes return
`{event_id, accepted, message}`; creates add the entity ID. Exit codes:
0=ok, 1=input error, 2=network/relay, 3=auth, 4=other, 5=write conflict (NIP-33 LWW).

`--format compact` is a **global** flag — it goes before the subcommand:
`buzz --format compact channels list`, NOT `buzz channels list --format compact`.

See `crates/buzz-cli/TESTING.md` for the full live-testing runbook.

---

## Testing

```bash
just test-unit    # core/auth libs + the buzz-local-relay and buzz-cli suites
just handoff-check graph-check   # sovereign contract suites (relay-free)
just cloudflare-check
```

See [TESTING.md](TESTING.md) for the live local-relay runbook.

---

## Common Gotchas

1. **Kind `39000` for channel metadata, not `41`** — kind 41 is NIP-01 (unused). All kinds defined in `buzz-core/src/kind.rs`.
2. **Scope queries with explicit `kinds`** — relays that enforce hosted-style policy reject open-ended filters; the CLI conventions assume kind-scoped queries everywhere.
3. **Worktrees: `cd` in the same command** — shell CWD doesn't persist between tool calls. Use `cd /path && cargo build` as one command.
4. **`echo` with a bare `=`-prefixed word breaks under zsh** — zsh expands `=foo` as a command path. Quote such arguments in scripts meant for interactive shells.

---

## See Also

- [CONTRIBUTING.md](CONTRIBUTING.md) — setup, code style, PR process, how to add event kinds / CLI subcommands
- [TESTING.md](TESTING.md) — live relay + CLI testing runbook
- [ARCHITECTURE.md](ARCHITECTURE.md) — system design and component relationships
- [specs/architecture/](specs/architecture/) — the spec set, including [the upstream peel](specs/architecture/buzz-solo-upstream-peel-v0.1.md)
- [README.md](README.md) — project overview and quick start
