# Development and runtime environments

Buzz has two environments on one host:

1. The development environment is the repository checkout. It builds and
   tests code and uses the `dev` profile, port `3100`, and isolated data.
2. The runtime environment is an immutable package built from a clean,
   versioned checkout. Supervision points at its `current` symlink; profiles
   and durable journal state remain outside the package.

## Development

From the repository root:

```bash
just init-dev-profile
just local-relay
```

The development relay uses:

```text
profile:    ${XDG_CONFIG_HOME:-$HOME/.config}/buzz/profiles/dev.toml
data:       ${XDG_DATA_HOME:-$HOME/.local/share}/buzz-local-relay/dev/
HTTP/WS:    127.0.0.1:3100
```

Set `BUZZ_DEV_PRIVATE_KEY` to a development-only Nostr key before using
profile-driven write commands. The development profile has no rendezvous
relay, so replication is opt-in rather than an accidental side effect of
running a checkout.

Use `just local-relay --ephemeral` for disposable experiments. The direct
`buzz-local-relay` binary also defaults to the XDG `default` data root, but
production services should always pass explicit `--data`, `--artifacts`, and
identity paths.

## Build and install a runtime

The package contains the `buzz` CLI, local relay, replication helpers, and
profile-compatible context scripts:

```bash
just ci
BUZZ_NODE_VERSION=0.2.1 just node-build
just node-install dist/node/0.2.1/$(rustc -vV | sed -n 's/^host: //p')
```

The builder requires a clean checkout, uses `Cargo.lock`, embeds the Git
revision, and writes `release.json`. The installer copies the package into:

```text
${BUZZ_RUNTIME_ROOT:-$HOME/.local/lib/buzz}/releases/<version>/
```

It then atomically advances `current`. It never overwrites an existing
versioned release. Restart supervised services explicitly after an install.

## Live state and supervision

Keep the live profile and state outside the runtime package:

```text
~/.config/buzz/profiles/default.toml
~/.config/buzz/credentials/
~/.local/share/buzz-local-relay/default/
~/.local/state/buzz/default/logs/
~/.local/lib/buzz/current/
```

The macOS relay template in `ops/macos/launchd/` is a rendering input, not an
auto-installed service. Replace every placeholder, inspect the generated plist,
and only then load it with `launchctl`. The template passes `--artifacts`
explicitly so the relay and profile use the same artifact directory.

## Legacy migration boundary

`~/.buzz-local` is durable state, not a build directory. First change service
and runtime paths while leaving that directory in place. Run `buzz context
doctor --offline`, verify the journal and replication behavior, and only then
perform a separately backed-up state migration into the XDG data root. Do not
delete the legacy directory merely because a new profile exists.
