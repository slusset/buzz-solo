# Testing

## Automated Tests

```bash
just test-unit       # buzz-core + buzz-auth libs, buzz-local-relay, buzz-cli
just handoff-check   # journal handoff lifecycle / custody / runner contract
just graph-check     # context graph renderer conformance
just cloudflare-check
just ci              # everything CI runs (check + test-unit + cargo-deny)
```

No test needs external infrastructure: the local-relay suites start
in-process servers on ephemeral ports, and the contract suites are pure
bash + jq against repo scripts.

## Live Local Relay

```bash
. ./bin/activate-hermit
just init-dev-profile
just local-relay                     # ws://127.0.0.1:3100, XDG dev log
just local-relay --ephemeral         # disposable in-memory run
```

Build the CLI and drive it:

```bash
cargo build --release -p buzz-cli
export BUZZ_RELAY_URL=ws://127.0.0.1:3100
export BUZZ_PRIVATE_KEY=<hex key>    # dev key; never a real identity
./target/release/buzz --help
```

See [crates/buzz-cli/TESTING.md](crates/buzz-cli/TESTING.md) for the full
live-testing runbook and [crates/buzz-cli/CONTEXT.md](crates/buzz-cli/CONTEXT.md)
for profile-driven sovereign-node operation (`buzz context doctor` verifies
a configured node end to end).

## Replication / rendezvous

The Cloudflare portable relay has its own check suite (`just
cloudflare-check`: binding types, typecheck, lint, tests). Cross-node
replication and handoff behavior is covered by the buzz-local-relay
integration tests (`tests/replication_port.rs`, `tests/portable_conformance.rs`,
`tests/artifact_store.rs`, `tests/identity_adapter.rs`) and the sovereign
contract scripts.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Tests pass locally but CI fails | Forgot to run `just ci` | `just ci` mirrors the five CI lanes |
| CLI writes report node unreachable | Laptop relay down | `launchctl kickstart -k gui/$UID/com.buzz.local-relay` |
| Stale `BUZZ_*` env vars | Inherited from an old shell | `unset BUZZ_AUTH_TAG BUZZ_RELAY_URL BUZZ_PRIVATE_KEY` |
