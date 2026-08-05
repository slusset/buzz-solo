# Buzz Solo — Upstream Peel & Launch Preparation v0.1

Status: phases 0–4 landed — peel complete
Date: 2026-07-30

## Motive

CI run [30570059944](https://github.com/slusset/buzz/actions/runs/30570059944)
on the first specs-only PR lit up the full inherited lane matrix — desktop
builds, Windows, mobile, hosted Postgres/Redis e2e — and failed on a
dependency-pin drift in a lane this fork does not ship (Desktop Build (macOS):
root `Cargo.lock` pins `mesh-llm-sdk` by tag at one commit while
`desktop/src-tauri/Cargo.lock` pins the same version by rev at another).
None of it was caused by the PR. The infrastructure had drifted from the
project's nature: this fork's center is the sovereign laptop node
(`buzz-local-relay` + `buzz-cli` + context/handoff contracts) with a
Cloudflare rendezvous replica — not Block's hosted relay, desktop, and
mobile release machinery.

The peel is done in phases so each step is a reviewable commit and
everything retired stays recoverable from git history.

## Phase 0 — CI tells the truth (landed with this spec)

`ci.yml` rewritten to the Solo surface, every lane unconditional (the
`dorny/paths-filter` job ran with `token: ''` and `fetch-depth: 2`, over-firing
on unreliable local diffs — with five cheap lanes there is nothing left worth
filtering):

- **Rust Lint** — `just fmt-check`, `just clippy` (workspace)
- **Unit Tests** — `just test-unit` plus
  `cargo nextest run -p buzz-local-relay -p buzz-cli` (the Solo center,
  previously absent from CI entirely)
- **Sovereign Contracts** — `scripts/test-buzz-handoff-contract.sh`,
  `scripts/test-buzz-ctx-graph.sh` (relay-free, previously local-only)
- **Cloudflare Portable Relay** — `just cloudflare-check`
- **Security** — `cargo-deny check`

Retired from `ci.yml`: paths-filter, desktop core/smoke/e2e/integration,
backend-integration and relay-e2e (hosted Postgres/Redis world), web, mobile,
Windows rust, server cross-compile, dead-token guard, macOS desktop build,
and the release/mobile contract steps that guarded workflows slated for
Phase 1 retirement.

## Phase 1 — retire upstream-only workflows (landed)

These serve Block's release and deploy machinery and cannot succeed on this
fork (Block secrets, ECR, signing certs, Buildkite hand-offs). PR-triggering
ones actively pollute checks on Rust PRs:

| Workflow | Fires on fork? | Serves |
|---|---|---|
| `docker.yml` | PRs touching `crates/**`, `Cargo.*`; pushes to main | Relay image → ECR |
| `helm-chart.yml` | PRs touching `deploy/charts/buzz/**`; pushes to main | Staging Helm chart |
| `push-gateway-helm-chart.yml` | PRs touching its chart | Push-gateway chart |
| `benchmark-harbor.yml` | PRs touching `benchmarks/**` | Upstream bench harness |
| `auto-tag-on-release-pr-merge.yml` | PR close on main | Upstream release tags |
| `release.yml` | `v*` tags | Desktop release pipeline |
| `sprig.yml` | pushes to main, `sprig-v*` tags | Sprig harness bundle |
| `linux-canary.yml`, `windows-canary.yml`, `signed-macos-canary.yml`, `mobile-release-candidate.yml` | dispatch-only | Client canaries |

Delete all eleven. Node runtime releases are already signed `node/vX.Y.Z`
git tags per `node-release-distribution-v0.1.md` — no workflow involved.
If a Solo release pipeline is ever wanted (e.g. attaching built `buzz`
binaries to node tags), it gets written fresh against that spec, not
adapted from `release.yml`. The contract scripts that guarded these
workflows (`test-release-ref-contract.sh`, `test-signed-canary-contract.sh`,
`test-mobile-*.sh`, `verify-release-ref.sh`) are orphaned by this phase
and go with their surfaces in Phase 3.

## Phase 2 — identity (landed)

- Repository renamed to `slusset/buzz-solo` (GitHub redirects the old
  slug). README rewritten to the Buzz Solo statement (sovereign node,
  portable relay, context/handoff/journal contracts); CONTRIBUTING
  retitled and repointed.
- `upstream` (`block/buzz`) stays a read-only remote for selective
  cherry-picks; upstream `main` is not a merge source. The GitHub fork
  relationship is kept (harmless, keeps cherry-pick UX); detaching via
  support remains possible later.
- `AGENTS.md` ecosystem section rewritten: the five-repo Block table
  replaced with the Solo runtime-surface table and a retiring-surfaces
  notice. Deeper doc sweep (RELEASING.md, TESTING.md, VISION docs, the
  desktop/mobile sections of AGENTS.md) goes with Phase 3, when the
  surfaces they document leave the tree.

## Phase 3 — code peel (landed)

Build-graph findings that shaped the cut:

- **Crate graph is clean.** The Solo center (`buzz-local-relay`, `buzz-cli`)
  closes over only `buzz-core`, `buzz-auth`, `buzz-sdk`, `buzz-ws-client`,
  `buzz-persona`. No workspace crate depends on anything under `desktop/`,
  `mobile/`, or `web/` (desktop was its own Cargo workspace). All crates
  stay; the desktop sidecar/harness crates (`buzz-acp`, `buzz-agent`,
  `buzz-dev-mcp`, `sprig`, `buzz-pair-relay`, `buzz-pairing-cli`) lose their
  main consumer but remain buildable and get their own decision with the
  hosted stack.
- **The relay serves web UIs from optional runtime paths** (`BUZZ_WEB_DIR`,
  `BUZZ_ADMIN_WEB_DIR`), not embedded assets — removing `web/` cannot break
  the relay build. Consequence: the Docker image no longer bundles a web UI,
  so invite-landing and repo-browser routes are unavailable from it.
  `admin-web/` stays (hosted-stack operator dashboard).

Removed: `desktop/`, `mobile/`, `web/`, `benchmarks/`, `deploy/charts/`,
`patches/` (both patches were desktop/web-only), `RELEASING.md`, the
client-only VISION docs (`VISION_SOVEREIGN.md` stays), the orphaned
release/canary/mobile contract scripts from Phase 1, and the desktop/mobile
dev scripts. Tooling updated in the same commit: `justfile` (composite
targets now mirror the five CI lanes; `test-unit` gained the Solo suites),
`lefthook.yml`, `pnpm-workspace.yaml` + lockfile (packages: `admin-web`,
`cloudflare/portable-relay`), `Dockerfile` (admin bundle only, Solo OCI
labels), and the docs sweep (AGENTS/CONTRIBUTING/TESTING/ARCHITECTURE).

The hosted relay stack initially stayed pending its own decision; that
decision landed as Phase 4 below.

## Phase 4 — hosted stack shutdown (landed)

The owner's decision: the sovereign workflow never touches the hosted
stack — the laptop node is `buzz-local-relay`, the drain leg is its
`buzz-relay-pull` bin, and the rendezvous is the Cloudflare adapter — so
the "shadow relay" retires. Dependency check: every dependent of a
hosted-stack crate is itself in the hosted stack; the keep-set closure is
unchanged, and the `mesh-llm` git dependency (source of the original CI
pin drift) left with `buzz-relay`, its only consumer.

Removed: crates `buzz-relay`, `buzz-db`, `buzz-pubsub`, `buzz-search`,
`buzz-audit`, `buzz-media`, `buzz-workflow`, `buzz-conformance`,
`buzz-push-gateway`, `buzz-relay-mesh`, `buzz-admin`, `buzz-test-client`;
`admin-web/`, `deploy/`, `migrations/`, `schema/`, the Docker files and
compose stack, `GOVERNANCE.md` (pointed at Block governance), and the
relay-serving dev scripts (`run-tests.sh` integration lanes, seeds,
relay launchers, DB maintenance SQL). Workspace `Cargo.toml` shed the
hosted dependency block (sqlx, redis, iroh, opentelemetry/metrics stacks,
the aws-creds fork pin, the CI profile). `.env.example`, `justfile`,
`lefthook.yml`, and the pnpm workspace (now just
`cloudflare/portable-relay`) were reduced to match, and
ARCHITECTURE.md/TESTING.md were rewritten as Solo documents.

Kept deliberately: the agent harness (`buzz-acp`, `buzz-agent`,
`buzz-dev-mcp`, `sprig`), pairing (`buzz-pair-relay`,
`buzz-pairing-cli`), the nostr git tools, and `examples/` — small,
buildable, and plausibly part of the Solo agent story; they get their own
look if they go stale. The local dev Docker stack was shut down before
the compose file left the tree; its volumes remain until removed manually
(`docker volume ls | grep buzz`).

## Non-goals

- No change to PrincipalDomain, PrincipalNode, profiles, or journal semantics.
- No history rewrite; upstream ancestry remains shared so cherry-picks stay
  cheap.
