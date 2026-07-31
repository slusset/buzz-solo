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
    if ! command -v docker &>/dev/null; then
        echo "Error: Docker is required but not installed."
        echo "Install it from https://docs.docker.com/get-docker/"
        exit 1
    fi
    if [[ ! -f .env ]]; then
        cp .env.example .env
        echo "Created .env from .env.example — review it before running just relay."
    fi

# Start Docker services, run migrations, install JS deps
setup: bootstrap
    ./scripts/dev-setup.sh

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

# Wipe development state and recreate a clean environment
[confirm("This will DELETE all development data. Continue? (y/N)")]
reset:
    ./scripts/dev-reset.sh --yes

# Stop all dev services (keep data)
down:
    docker compose down

# Show dev service status
ps:
    docker compose ps

# Tail all service logs
logs *ARGS:
    docker compose logs -f {{ARGS}}

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

# Ensure Docker dev services (Postgres, Redis, etc.) are running and healthy
_ensure-services:
    #!/usr/bin/env bash
    set -euo pipefail
    pg=$(docker inspect --format '{{"{{"}}.State.Health.Status{{"}}"}}' buzz-postgres 2>/dev/null || echo "not_found")
    redis=$(docker inspect --format '{{"{{"}}.State.Health.Status{{"}}"}}' buzz-redis 2>/dev/null || echo "not_found")
    if [[ "$pg" == "healthy" && "$redis" == "healthy" ]]; then
        echo "Services already healthy"
        exit 0
    fi
    echo "Starting services..."
    docker compose up -d || true
    echo -n "Waiting for services"
    for i in $(seq 1 40); do
        pg=$(docker inspect --format '{{"{{"}}.State.Health.Status{{"}}"}}' buzz-postgres 2>/dev/null || echo "not_found")
        redis=$(docker inspect --format '{{"{{"}}.State.Health.Status{{"}}"}}' buzz-redis 2>/dev/null || echo "not_found")
        if [[ "$pg" == "healthy" && "$redis" == "healthy" ]]; then
            echo " ready"
            exit 0
        fi
        echo -n "."
        sleep 3
    done
    echo " timed out"
    exit 1

# Apply database migrations and seed the local dev community if the dev database is running
_ensure-migrations: _ensure-services
    cargo run -p buzz-admin -- migrate
    ./scripts/seed-local-community.sh

# Run all checks suitable for CI / pre-push (no infra needed)
ci: check test-unit security

# ─── Test ─────────────────────────────────────────────────────────────────────

# Run all tests (unit + integration)
test:
    ./scripts/run-tests.sh all

# Run unit tests only (no infra needed)
test-unit:
    #!/usr/bin/env bash
    if command -v cargo-nextest &>/dev/null; then
        cargo nextest run -p buzz-core -p buzz-auth --lib
        # The Solo center: durable local relay + node CLI. Infra-free
        # (in-process servers on ephemeral ports), so it belongs here.
        cargo nextest run -p buzz-local-relay -p buzz-cli
        # buzz-db migrator/lint tests: pure SQL-parsing unit tests (no infra).
        # They guard the embedded-migrator invariant (exactly the consolidated
        # 0001; cutover/backfill stays an operator script, not startup state)
        # and the tenant-scoping lints. The Postgres-backed buzz-db tests are
        # #[ignore]d, so --lib runs only the infra-free set. Without this gate a
        # stray file in migrations/ or a broken lint ships green.
        cargo nextest run -p buzz-db --lib
        # Multi-tenant conformance gate (buzz-conformance): the independent
        # replay checker + golden fixtures. No infra — pure in-process trace
        # replay — so it belongs in the unit job. Run all targets (lib + the
        # tests/replay_fixtures.rs integration test), not just --lib.
        cargo nextest run -p buzz-conformance
        # Gateway unit and black-box HTTP tests are infra-free. Postgres-backed
        # contract/race tests run in the dedicated CI job below.
        cargo nextest run -p buzz-push-gateway
    else
        ./scripts/run-tests.sh unit
    fi

# Run integration tests only (starts services if needed)
test-integration:
    ./scripts/run-tests.sh integration

# Dependency policy (advisories, licenses, sources)
security:
    cargo-deny check

# ─── Run ──────────────────────────────────────────────────────────────────────

# Start the lightweight durable relay (no Docker or external services)
local-relay *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{justfile_directory()}}/bin:$PATH"
    cargo run -p buzz-local-relay -- {{ARGS}}

# Start the relay server (auto-starts Docker services if needed)
relay: bootstrap _ensure-migrations
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{justfile_directory()}}/bin:$PATH"
    cargo run -p buzz-relay

# Build and run the private read-only admin dashboard
admin: bootstrap _ensure-migrations
    #!/usr/bin/env bash
    set -euo pipefail
    export PATH="{{justfile_directory()}}/bin:$PATH"
    [[ -d node_modules ]] || pnpm install
    pnpm -C admin-web build
    export BUZZ_ADMIN_HOST="${BUZZ_ADMIN_HOST:-admin.localhost:3000}"
    export BUZZ_ADMIN_WEB_DIR="${BUZZ_ADMIN_WEB_DIR:-{{justfile_directory()}}/admin-web/dist}"
    echo "Admin dashboard: http://${BUZZ_ADMIN_HOST}/reports"
    cargo run -p buzz-relay

# Seed deterministic reports and product feedback for local admin dashboard review
admin-seed: _ensure-migrations
    ./scripts/seed-admin-dashboard.sh

# Run focused relay and browser checks for the read-only admin dashboard
admin-check: fmt-check
    cargo check -p buzz-relay --all-targets
    cargo test -p buzz-relay api::admin
    cargo test -p buzz-relay router::tests
    pnpm -C admin-web check
    pnpm -C admin-web exec playwright test

# Start the relay server in release mode
relay-release: _ensure-migrations
    cargo run -p buzz-relay --release

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

# ─── Database ─────────────────────────────────────────────────────────────────

# Apply database migrations
migrate: _ensure-migrations

# ─── Utilities ────────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# Check the Rust workspace compiles without producing binaries
check-compile:
    cargo check --workspace --all-targets

# ─── Agent Harness ────────────────────────────────────────────────────────────

# Run a goose agent connected to a Buzz relay (foreground)
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
