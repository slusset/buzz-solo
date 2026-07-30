# Buzz Solo — Upstream Peel & Launch Preparation v0.1

Status: phases 0–2 landed, phase 3 proposed
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

## Phase 3 — code peel (needs dependency analysis first)

Candidate removals: `desktop/`, `mobile/`, `web/`, `benchmarks/`,
`deploy/charts/`, and hosted-relay-only crates. **Not mechanical**: the Solo
center still compiles against shared crates (`buzz-core`, `buzz-sdk`,
`buzz-ws-client`, `buzz-auth`, …), `justfile` and pre-commit/pre-push hooks
reference the client surfaces, and the pnpm workspace spans desktop/web/
cloudflare. Each removal needs a build-graph check before it lands. The
hosted relay stack (`buzz-relay`, `buzz-db`, `buzz-pubsub`, …) stays for now
— the drain leg and interop tests still exercise it — and gets its own
decision later.

## Non-goals

- No change to the sovereign node runtime, profiles, or journal semantics.
- No history rewrite; upstream ancestry remains shared so cherry-picks stay
  cheap.
