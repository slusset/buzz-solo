# Buzz Solo — development task runner

set dotenv-load := true

# List all available tasks
default:
    @just --list

# ─── Dev Environment ─────────────────────────────────────────────────────────

# Install required dev tools via Hermit and create .env (safe to re-run)
bootstrap:
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{justfile_directory()}}/bin:$PATH"
    # Hermit's bin/ symlinks auto-download pinned tool versions on first use.
    # Running each tool once triggers the download if not already cached.
    echo "Ensuring toolchain via Hermit..."
    cargo --version &
    node --version &
    pnpm --version &
    wait
    if [[ ! -f .env ]]; then
        cp .env.example .env
        echo "Created .env from .env.example — review it before running just local-relay."
    fi

# Bootstrap tools, install JS deps, and install git hooks
setup: bootstrap
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{justfile_directory()}}/bin:$PATH"
    pnpm install
    just hooks

# Install git hooks via lefthook (dispatches from the shared .git/hooks dir so all
# linked worktrees inherit the same hooks without a worktree-relative .hooks path)
hooks:
    #!/usr/bin/env bash
    set -euo pipefail
    # Use the Hermit-pinned lefthook (bin/lefthook self-downloads on first use):
    # works with no pre-installed lefthook and guarantees the pinned version
    # rather than whatever happens to be on PATH.
    export PATH="{{justfile_directory()}}/bin:$PATH"
    # --path-format=absolute guarantees an absolute path from every invocation context:
    # without it, --git-common-dir returns ".git" from the main checkout and a
    # relative hooksPath would break linked-worktree dispatch just like .hooks did.
    HOOKS_DIR="$(git rev-parse --path-format=absolute --git-common-dir)/hooks"
    git config --local core.hooksPath "$HOOKS_DIR"
    lefthook install --force

# ─── Build & Check ───────────────────────────────────────────────────────────

# Build the Rust workspace
build:
    cargo build --workspace

# Build the Rust workspace in release mode
build-release:
    cargo build --workspace --release

# Run repo lint and formatting checks
check: fmt-check clippy cloudflare-check handoff-check graph-check

# Format all Rust code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Run clippy with warnings as errors
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run all checks suitable for CI / pre-push (no infra needed)
ci: check test-unit security

# ─── Test ─────────────────────────────────────────────────────────────────────

# Run the test suite (alias for test-unit — no external infrastructure exists)
test: test-unit

# Run unit tests (no infra needed)
test-unit:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v cargo-nextest &>/dev/null; then
        cargo nextest run -p buzz-core -p buzz-auth --lib
        # The Solo center: durable local relay + node CLI. Infra-free
        # (in-process servers on ephemeral ports), so it belongs here.
        cargo nextest run -p buzz-local-relay -p buzz-cli
    else
        cargo test -p buzz-core -p buzz-auth --lib
        cargo test -p buzz-local-relay -p buzz-cli
    fi

# Dependency policy (advisories, licenses, sources)
security:
    cargo-deny check

# ─── Run ──────────────────────────────────────────────────────────────────────

# Initialize the isolated XDG development profile and data directories.
init-dev-profile:
    ./scripts/init-buzz-dev-profile.sh

# Start the durable development relay (no external services).
# The default is XDG_DATA_HOME/buzz-local-relay/dev on port 3100. Pass
# --ephemeral for a disposable in-memory run or explicit relay flags as needed.
local-relay *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{justfile_directory()}}/bin:$PATH"
    data_root="${BUZZ_DEV_DATA_ROOT:-${XDG_DATA_HOME:-$HOME/.local/share}/buzz-local-relay/dev}"
    mkdir -p "$data_root/artifacts" "$data_root/cursors"
    cargo run -p buzz-local-relay -- \
        --bind "${BUZZ_DEV_BIND_ADDR:-127.0.0.1:3100}" \
        --data "$data_root/sovereign.ndjson" \
        --artifacts "$data_root/artifacts" \
        {{ARGS}}

# Build an immutable, manifest-bearing node runtime package.
node-build:
    ./scripts/build-node-release.sh

# Install a previously built node runtime package and safely update current.
node-install package:
    ./scripts/install-node-release.sh "{{package}}"

# ─── Cloudflare ──────────────────────────────────────────────────────────────

cloudflare_dir := "cloudflare/portable-relay"

# Run Cloudflare portable-relay checks (binding types, typecheck, lint, tests)
cloudflare-check:
    cd {{cloudflare_dir}} && pnpm run check

# ─── Sovereign Contracts ─────────────────────────────────────────────────────

# Journal handoff lifecycle, custody, and runner safety contract
handoff-check:
    ./scripts/test-buzz-handoff-contract.sh

# Context graph renderer conformance
graph-check:
    ./scripts/test-buzz-ctx-graph.sh

# ─── Utilities ────────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# Check the Rust workspace compiles without producing binaries
check-compile:
    cargo check --workspace --all-targets

# ─── Agent Harness ────────────────────────────────────────────────────────────

# Run a goose agent connected to a relay (foreground)
goose relay="ws://localhost:3000" agents="1" heartbeat="0" prompt="" key="$BUZZ_PRIVATE_KEY":
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{justfile_directory()}}/bin:$PATH"
    source ./scripts/_goose-env.sh "{{relay}}" "{{key}}" "{{agents}}" "{{heartbeat}}" "{{prompt}}"
    exec env "${env_args[@]}" ./target/release/buzz-acp

# Run a goose agent in the background (screen session named 'goose-agent-N')
goose-bg relay="ws://localhost:3000" agents="1" heartbeat="0" prompt="" key="$BUZZ_PRIVATE_KEY":
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{justfile_directory()}}/bin:$PATH"
    source ./scripts/_goose-env.sh "{{relay}}" "{{key}}" "{{agents}}" "{{heartbeat}}" "{{prompt}}"
    screen -dmS goose-agent-{{agents}} bash -c "$(printf '%q ' env "${env_args[@]}") ./target/release/buzz-acp"
    echo "Agent running in screen session 'goose-agent-{{agents}}'. Attach with: screen -r goose-agent-{{agents}}"
