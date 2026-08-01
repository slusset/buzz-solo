# Contributing to Buzz Solo

Welcome, and thank you for your interest in contributing! Buzz Solo is an
open-source, solo-first derivative of [Block's Buzz](https://github.com/block/buzz).
This guide will help you get from zero to a merged pull request.

If you have questions that aren't answered here, [open an issue](https://github.com/slusset/buzz-solo/issues/new).

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Before You Open a PR](#before-you-open-a-pr)
3. [Setting Up the Development Environment](#setting-up-the-development-environment)
4. [Running Tests](#running-tests)
5. [Code Style](#code-style)
6. [Making a Pull Request](#making-a-pull-request)
7. [Architecture Overview](#architecture-overview)
8. [Ecosystem](#ecosystem)
9. [How to Add a New Event Kind](#how-to-add-a-new-event-kind)
10. [How to Add a New MCP Tool](#how-to-add-a-new-mcp-tool)
11. [How to Add a New API Endpoint](#how-to-add-a-new-api-endpoint)
12. [License and CLA](#license-and-cla)

---

## Code of Conduct

This project follows the [Contributor Covenant v2.1](CODE_OF_CONDUCT.md).
By participating you agree to uphold these standards. Please report
unacceptable behavior to **conduct@buzz-relay.org**.

---

## Before You Open a PR

Before starting, search [open PRs](https://github.com/slusset/buzz-solo/pulls) and [open issues](https://github.com/slusset/buzz-solo/issues) for duplicates — someone may already be working on the same thing. When you open your PR, link the closest existing one in the description (or say "none found").

For anything beyond a small fix, opening an issue first is strongly recommended. Describe the problem and proposed solution so a maintainer can acknowledge the approach before you build — it avoids two people building the same thing in parallel.

Buzz is an agent platform, so AI-assisted PRs are welcome. No need to disclose the tools you used, but you own and must have reviewed the final code. Submissions that are clearly unreviewed may be closed with a pointer here.

We squash-merge, so your PR title becomes the commit subject in `main`. Use [Conventional Commits](https://www.conventionalcommits.org/) format: `feat(mcp): add get_feed_actions tool`. The type prefix (`feat`, `fix`, `docs`, `refactor`, `test`, `chore`) is required. See the [Commit Messages](#commit-messages) section for the full reference.

Every commit needs a Developer Certificate of Origin sign-off, so commit with `git commit -s` — it appends the `Signed-off-by` trailer that certifies you wrote the change and can contribute it. The required **DCO Check** blocks merge without it on every commit, and it's the most common reason new PRs stall. If you already pushed unsigned commits, run `git rebase --signoff main` and force-push. Running `just hooks` installs a `commit-msg` hook that adds the trailer to commits created by `git commit` and `git merge`; other flows need their own flag — `git rebase --signoff`, `git cherry-pick -s`.

We review as capacity allows — focused PRs that follow this guide move fastest.

---

## Setting Up the Development Environment

### Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.88+ | Install via [rustup](https://rustup.rs/) |
| Node.js | 24+ | Required for the Cloudflare portable-relay checks and `just ci` |
| pnpm | 10+ | Required for the Cloudflare portable-relay checks and `just ci` |
| `just` | latest | Task runner — `cargo install just` |
| `lefthook` | 2.1.3 (Hermit-pinned) | Auto-installed by `just hooks` — no manual install needed |
| `sqlx` migrations | workspace crate | `just migrate` applies embedded migrations from `migrations/` |

This repo uses [Hermit](https://cashapp.github.io/hermit/) for toolchain
pinning. Activate it once per shell session:

```bash
. ./bin/activate-hermit
```

Hermit pins Rust, `just`, Node, pnpm, and other tools to the versions in
`bin/`. Each tool is downloaded on first use. You can also run `just bootstrap`
(which `just setup` calls automatically) to pre-download all required tools
upfront. If you don't use Hermit, ensure your toolchain meets the minimum
versions in the table above.

### First-Time Setup

```bash
# 1. Clone the repo
git clone https://github.com/slusset/buzz-solo.git
cd buzz-solo

# 2. Activate Hermit (optional but recommended)
. ./bin/activate-hermit

# 3. Bootstrap tools + infrastructure
just setup

# 4. Install Git hooks (optional, recommended)
just hooks
```

`just setup` runs `just bootstrap` first — it copies `.env.example` to `.env`
if it doesn't already exist, and invokes `cargo`, `node`, and `pnpm` to trigger
Hermit's lazy tool download (each tool is fetched once on first invocation and
cached thereafter). You can also run `just bootstrap` independently at any time;
it is safe to re-run.

`just setup` then installs the JS workspace dependencies (pnpm) and git
hooks. Nothing else is required — the Solo relay needs no external services.

### Running the Relay

```bash
just local-relay   # isolated XDG dev relay on port 3100 — no Docker/services
```

---

## Running Tests

```bash
just test-unit   # buzz-core + buzz-auth libs, buzz-local-relay, buzz-cli
just handoff-check graph-check   # sovereign contract suites (bash + jq)
```

All tests are self-contained: the local-relay suites start in-process
servers on ephemeral ports. See [TESTING.md](TESTING.md) for the live
relay + CLI runbook.

### CI Gate

Before opening a PR, run the full CI gate locally:

```bash
just ci
# Runs: check (fmt + clippy + cloudflare + sovereign contracts) + unit tests + cargo-deny
```

This mirrors the CI lanes exactly. PRs that fail `just ci` will not be
merged. If it fails on formatting, `just fmt` fixes Rust formatting in place.

---

## Code Style

### Formatting

We use `rustfmt` with default settings. Format your code before committing:

```bash
cargo fmt --all
```

To check without modifying:

```bash
cargo fmt --all -- --check
```

### Linting

We use `clippy` with warnings-as-errors:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Fix all clippy warnings before submitting a PR. If you believe a warning is
a false positive, add a targeted `#[allow(...)]` with a comment explaining
why.

### No Unsafe Code

All crates enforce `#![deny(unsafe_code)]`. Do not add unsafe blocks. If you
believe unsafe is genuinely necessary, open an issue first to discuss the
approach.

### Error Handling

- Use `thiserror` for library error types.
- Use `anyhow` for binary / application-level error propagation.
- Do not use `unwrap()` or `expect()` in production code paths. Use `?` or
  explicit error handling. `unwrap()` is acceptable in tests.

### Logging and Tracing

Use the `tracing` crate for all instrumentation. Prefer structured fields
over string interpolation:

```rust
// Good
tracing::info!(channel_id = %id, event_kind = kind, "Event ingested");

// Avoid
tracing::info!("Event ingested: channel={id} kind={kind}");
```

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(mcp): add get_feed_actions tool
fix(auth): reject expired NIP-42 challenges
docs(agents): document workflow MCP tools
refactor(db): extract channel queries into channel.rs
test(workflow): add approval gate integration test
```

The type prefix (`feat`, `fix`, `docs`, `refactor`, `test`, `chore`) is
required. The scope (in parentheses) is optional but encouraged.

---

## Making a Pull Request

### What a Good PR Looks Like

1. **Focused** — one logical change per PR. If you're fixing a bug and
   refactoring a module, split them into two PRs.

2. **Tested** — new behavior has tests. Bug fixes include a regression test.
   If a test is impractical, explain why in the PR description.

3. **Documented** — public APIs, new event kinds, new MCP tools, and new
   config variables are documented. Update `README.md`, `AGENTS.md`, or
   the relevant spec in `specs/architecture/` as appropriate.

4. **CI passing** — `just ci` passes locally before you push.

5. **Clear description** — the PR description explains:
   - What problem this solves (or what feature it adds)
   - How it was implemented (key decisions, trade-offs)
   - How to test it manually (if applicable)
   - Any follow-up work deferred to a future PR

### Review Process

- We prioritize focused PRs that follow this guide and review as capacity allows.
- Address review comments by pushing new commits (don't force-push during
  review; it makes it hard to see what changed).
- Once approved, a maintainer will squash-merge your PR.

---

## Architecture Overview

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full system design and
[AGENTS.md](AGENTS.md#repo-structure) for the complete crate map. The key
design principles:

**The journal is the single source of truth.** The sovereign node appends
verified signed events to a durable log; replicas carry only the streams
that signed declarations export to them.

**Event kinds are the only switch.** Every action in the system — a message,
a reaction, a workflow step, a canvas update — is a Nostr event with a kind
integer. Adding a new feature means defining a new kind. No breaking changes
to existing clients.

---

## Ecosystem

This repo (`slusset/buzz-solo`) is the whole project — there are no
companion build or deploy repositories. See
[AGENTS.md § Ecosystem](AGENTS.md#ecosystem) for the runtime surface table
and the fork's relationship to upstream `block/buzz`.

**Contributors:** Fork `slusset/buzz-solo`, open a PR, and CI runs
automatically. No special access is required.

Node runtime releases are signed `node/vX.Y.Z` git tags — see
[node-release-distribution-v0.1](specs/architecture/node-release-distribution-v0.1.md).

---

## How to Add a New Event Kind

1. **Define the kind constant** in `buzz-core/src/kind.rs`:

   ```rust
   /// My new event kind — description of what it represents.
   pub const KIND_MY_FEATURE: u32 = 4XXXX;
   ```

   Pick a kind number in the appropriate sub-range defined in `kind.rs`.
   Check the `ALL_KINDS` array for collisions. Each sub-range is documented
   with comments in the file.

2. **Define the payload type** in the appropriate module in `buzz-core/src/`
   (e.g., alongside `event.rs`) if the content field is structured JSON:

   ```rust
   #[derive(Debug, Serialize, Deserialize)]
   pub struct MyFeaturePayload {
       pub field_one: String,
       pub field_two: Option<u64>,
   }
   ```

3. **Handle the kind where it matters** — the local relay
   (`crates/buzz-local-relay`) for node-side behavior, the CLI
   (`crates/buzz-cli`) for the agent-facing surface, and the Cloudflare
   adapter (`cloudflare/portable-relay`) if replicas must understand it.

4. **Write tests** — a unit test for payload serialization in `buzz-core`
   and coverage in the crate that handles the kind.

5. **Document** — `kind.rs` is the authoritative registry of all kind
   numbers, and sovereign-surface kinds get (or update) a spec in
   `specs/architecture/`.

---

## HTTP Endpoints

Prefer a signed Nostr event over any new HTTP surface. The local relay
exposes only the core Nostr event/query surface plus health probes; the
Cloudflare adapter mirrors it. If you believe something needs HTTP, write
the spec first (`specs/architecture/`) and raise it in an issue.

---

## License and CLA

Buzz is licensed under the **Apache License, Version 2.0**. See
[LICENSE](LICENSE) for the full text.

By submitting a pull request, you agree that your contribution is licensed
under the Apache 2.0 license and that you have the right to submit it.

If your employer has rights to intellectual property you create, you may need
their sign-off. When in doubt, check with your legal team.

---

*Thank you for contributing to Buzz. Every bug report, documentation fix,
and code contribution makes the project better for everyone. 🐝*
