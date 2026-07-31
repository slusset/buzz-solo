# Skill Distribution Loop v0.1

Status: draft — initial loop via GitHub issues
Date: 2026-07-31

## Decision

Skills discovered or authored on any node flow upstream to the home node
through a named intake, land in this repository under `skills/<name>/`, and
distribute back out to every node by ordinary `git pull` of `main`. The
repository is the distribution channel; the journal records adoption.

The v0.1 intake is a GitHub issue (`skill-submission` template) on
`slusset/buzz-solo`. This is deliberately low-tech: the coordination surface
between the Vanderbilt Health domain and Nest is GitHub issues for now, and
skills ride the same surface. A journal-native intake (skills as signed
events or handoff artifacts) is a named later phase, not v0.1.

## Topology and names

| Name | Role | Where |
|---|---|---|
| **Nest** | Buzz Solo Durable Context home node — the sovereign journal's home | Ted's laptop (`~/.buzz-local`, `~/Projects/buzz`) |
| **Vanderbilt Health domain** | Split-identity satellite host and its agent | VUMC → Vanderbilt Health machine (`~/DurableContext`) |
| **Rendezvous** | Availability replica | Cloudflare portable relay (standing question: rotating beacons may change its role — open exploration, not v0.1) |

Existing stream labels (`ted-laptop/sovereign`) are unchanged; renaming
operational labels is a deliberate migration, not a side effect of naming.

## The loop

```
Vanderbilt Health domain                    Nest (home node)
────────────────────────                    ─────────────────
1. author/discover skill
   (~/DurableContext)
2. exercise it locally
3. submit: skill-submission ─────────────►  4. review: portability,
   issue on slusset/buzz-solo                  host assumptions,
                                               naming, no secrets
                                            5. land: PR adds
                                               skills/<name>/SKILL.md
                                               (+ aux files), merges
                                               to main
6. pull main, install into  ◄─────────────  (distribution = git)
   local harness skill dir
7. confirm adoption: close the
   issue; journal residue on the
   adopting node
```

Rules:

- **`skills/<name>/SKILL.md` is the canonical form.** Frontmatter carries
  `name`, `description`, and — new for this loop — `origin` (authoring
  node/host) and `host_assumptions` (paths, tools, capabilities the skill
  expects, by reference only). A skill whose assumptions a host cannot meet
  is skipped by that host, not broken.
- **No secrets, ever.** Skills reference credentials and key material by
  profile/custody reference; a submission containing material is rejected
  at intake. Same forbidden-list posture as lifecycle residue in the
  durable-context-hooks contract.
- **Review happens once, at the home node.** Satellites do not exchange
  skills laterally; the loop is a star through `main` so every node runs
  the same reviewed version. Git history is the version record.
- **Adoption is residue.** When a node installs or updates a skill, its
  agent logs a one-line journal record (`skill:<name>` adopted at
  `<commit>`), so `buzz context sessions` shows which node runs what.
- **Skills that change node behavior get specs.** A skill that encodes new
  sovereign-surface behavior (declarations, handoffs, engrams) is the
  *packaging*; the behavior itself still lands in `specs/architecture/`
  first, per the standing rule.

## Repository layout

```
skills/
  README.md            # this loop, in one screen
  <name>/SKILL.md      # canonical skill
  <name>/…             # aux files the skill needs
```

Harness-specific installation (e.g. copying into `~/.claude/skills/` or a
harness plugin dir) is a host-adapter concern
([node-host-boundary](node-host-boundary-v0.1.md) session port); v0.1
installation is manual copy or symlink by the node's operator/agent.

## Later phases (named, not specified)

- **Journal-native intake**: skill submission as a handoff
  (`open → claim → return → close`) with the skill as a content-addressed
  return artifact — replacing the GitHub issue leg once the handoff
  tooling is routine across both nodes.
- **Binding contexts to nodes**: binding the shared durable context
  (Buzz Evolution) to each node so skill adoption residue and context
  briefs travel through replication instead of issue comments.
- **Rotating beacons**: if the Rendezvous role changes, the distribution
  loop is unaffected — it rides git, not the replica.

## Traceability

- Session surface:
  [`../contracts/agent-harness/durable-context-hooks.yaml`](../contracts/agent-harness/durable-context-hooks.yaml)
- Host boundary: [`node-host-boundary-v0.1.md`](node-host-boundary-v0.1.md)
- Handoff lifecycle (phase-2 intake): [`journal-handoff-v0.1.md`](journal-handoff-v0.1.md)
- Intake template: `.github/ISSUE_TEMPLATE/skill-submission.md`
