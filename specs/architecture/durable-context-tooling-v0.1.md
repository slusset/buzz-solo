# Durable Context Tooling v0.1

Status: draft — incorporates field report slusset/buzz-solo#3
Date: 2026-07-31

## Decision

The DurableContext directory pattern that emerged on the Vanderbilt Health
domain (`~/DurableContext/{DomainName}`) becomes a first-class Buzz Solo
concept: a durable context has an **on-disk root**, a **journal presence**,
and a **stewardship identity**, and the CLI grows a command surface to
initialize, discover, inspect, checkpoint, and explore contexts. This spec
integrates issue [#3](https://github.com/slusset/buzz-solo/issues/3) into
the sticky-attention chain rather than beside it.

Answering #3's first open question directly: **DurableContext is core, not
a plugin** — but most of it already was. The field pattern independently
converged on what the chain specs require: working-directory binding with
fail-closed ambiguity, linked repositories never traversed, metadata-only
session residue, manifest-last checkpoints, explicit-request-only
publication. That convergence is the strongest evidence yet that the specs
describe the right invariants. What #3 adds is the *on-disk form*, the
*resumable current-work head*, *per-context skills*, the *domain beacon*,
and — from the owner — *context visibility tooling* (the explorer).

## Already specified (mapping, not new work)

| #3 concept | Owning spec artifact |
|---|---|
| Bind context ID to working directory, fail closed | `bind-agent-session-to-context` story; `ResolveContextBinding` |
| Linked repos without traversal | `CheckpointPolicy` (`follow_symbolic_links: false`, `repository_capture: metadata_only`) |
| Metadata-only lifecycle records | `LifecycleResidue` allowed/forbidden lists |
| Checkpoint, manifest last | `checkpoint-is-manifest-last` rule; `ArtifactManifestHead` |
| Reconcile interrupted sessions | `AbandonmentEvidence`; `reconcile` operation |
| Never publish implicitly | `local-does-not-mean-published` rule; hooks `publication: forbidden` |
| Disclosure boundary | `ContextOptIn.disclosure_policy` |
| Sensitive-content rejection (PHI, secrets, keys, exports) | `CheckpointPolicy` sensitive patterns — PHI patterns are a policy *instance* the VH domain supplies, not a new mechanism |

The `.context/` files in the field pattern (`context.yaml`,
`artifacts.yaml`, `current-work.yaml`, `sessions/*`) are the on-disk
representation of `ContextOptIn`, `ArtifactManifestHead`, the current-work
head, and `AgentSession` records respectively. v0.1 blesses that layout as
the canonical root shape:

```text
{root}/
  .context/
    context.yaml        # ContextOptIn: identity, h, disclosure, links, skills
    artifacts.yaml      # generated manifest projection (see below)
    current-work.yaml   # resumable current-work head projection
    sessions/           # session binding records
  {slug}-context-charter.md
  {linked-repo} -> /absolute/target
```

**Generated, not authored** (#3's second open question): after
initialization, `.context/artifacts.yaml` and `current-work.yaml` are
projections of journal heads — hand-editable during migration of existing
roots, but the journal head wins on divergence and tooling regenerates the
files. `context.yaml` is the one authored document (it *is* the opt-in).

## New: current-work head

A per-context parameterized-replaceable head (`d = current-work`) holding
the resumable handoff: current initiative, **next safe action**,
authoritative artifact references, relevant skills. It is the bridge
between the context brief (derived, never stored) and stewardship handoff
(which needs a stored, deliberate statement of "what now"). Replacing it
never destroys residue, per the standing distillation rule.

## New: per-context skills

`context.yaml` may declare `skills:` — names resolved against the
repository's `skills/` tree per
[skill-distribution-loop-v0.1](skill-distribution-loop-v0.1.md). The
context declares *which* skills apply; the loop governs *where skills come
from*. Tooling provenance notes (e.g. "local `mcp-atlassian` fork, OAuth
remote disabled, bug patched and submitted upstream") are ordinary context
metadata — captured as residue or charter text, never as credentials.

Layer split, adopting #3's recommendation verbatim: relay identity,
transport keys, rendezvous, and replication stay **out** of DurableContext
manifests (they belong to the node profile and the
[node-host-boundary](node-host-boundary-v0.1.md)); the durable-context
layer owns filesystem lifecycle only. The three-skill split
(`buzz` / `sovereign-context` / `durable-context`) mirrors the three nested
bounded contexts already drawn in the host-boundary spec.

## New: domain beacon

A stable stewardship identity for a domain — the entry point a future
principal (human or agentic) resumes from. Minimum machine-readable shape
(#3's fourth open question), as a parameterized-replaceable journal head
(`d = beacon:{domain-slug}`) projected into the root as
`{slug}-beacon.yaml`:

```text
{
  domain:        display name + slug,
  context:       context ID (h) + root location hint,
  stewardship:   { principal, collaborators, target_handoff },
  surfaces:      [ authoritative external references — repos, Confluence,
                   Jira, runbooks — URLs and titles only ],
  current_work:  reference to the current-work head,
  provenance:    tooling notes and limitations,
  excluded:      credentials, tokens, keys, deployment secrets, raw
                 operational payloads — by rule, not by review
}
```

Naming note: a **domain beacon** is a stewardship identity; a **beacon
pulse** (kind 20700) is a node witness statement; the **rotating beacons**
idea is an open rendezvous exploration. Three different things; the word
is overloaded and the specs must always qualify it.

Handoff of a domain is then a composition of existing mechanisms: the
beacon names the domain, the current-work head says what now, and the
[journal-handoff](journal-handoff-v0.1.md) lifecycle transfers custody —
nothing new to invent, which is the point of the chain.

## New: context explorer

Context visibility and management — "a file system explorer for context."
A read-only deterministic projection (same rule as the brief: same events,
same view, never stored as authority) over all bounded contexts:

```text
buzz context explore                 # all contexts by warmth
buzz context explore <context|root>  # one context, expanded
```

Tree shape: context → freshness/warmth → heads (current understanding) →
current-work → open loops → active/interrupted sessions → linked repos →
artifact manifest → declared skills → beacon. Sources are the journal and
`.context` roots; the explorer never mutates. A TUI and an actual virtual
filesystem (mounting the projection read-only, making "explorer" literal)
are named later phases; the CLI tree ships first.

## CLI surface and the namespace collision

#3 proposes `buzz context init|status|bind-session|checkpoint|reconcile|
link-repo|journal`. The `buzz context` namespace already carries the
node-level surface (`load|save|log|sessions|sync|pulse|status|handoff|
artifact|graph|doctor`) — and `status` collides outright (declaration
state vs directory state).

Resolution: directory-scoped operations take a **root-first grammar**
under the same namespace — `init`, `bind`, `checkpoint`, `reconcile`,
`link-repo`, `explore` all accept `[root]` (default: discover from cwd),
while node-level commands never do. The collision is resolved by renaming
the node-level `status` to `buzz node status` in a CLI change that ships
with the first tooling slice; until then the directory form is
`buzz context explore` (which subsumes #3's `status` intent). `journal`
(#3) is already `buzz context log`.

Discovery honors `${DURABLE_CONTEXT_HOME:-~/DurableContext}` for
symlink-target matching, exactly as fielded.

## Open-question dispositions (#3)

1. Core or plugin → **core** (above).
2. Hand-editable manifests → **generated after init**, journal wins.
3. Skills modeling → **manifest references into the repo skill tree**;
   journal residue records adoption; no third mechanism.
4. Beacon shape → **the head above**; v0.1 minimum, extensible.
5. Copilot hooks vs CLI split → the hooks contract owns lifecycle
   callbacks; the CLI owns everything invocable without a harness. A
   harness adapter calls the same CLI operations the builder can.

## Demonstration fixture

The **OR Temperature Control** context (VH domain) is the acceptance
fixture: init from the existing root, linked `metasys-*` repos discovered,
beacon declared with its Confluence/Jira surfaces and stewardship chain
(steward: Ted Slusser; PM: Vikas Jain; target handoff: Periop team),
explorer renders the whole domain — with zero PHI, credentials, or raw
exports in any journal event, enforced by policy patterns.

## Next steps (named)

- Extend `bounded-attention-context.model.yaml` with `LinkedRepository`,
  `CurrentWorkHead`, and `DomainBeacon` entities + explorer commands.
- `context-explorer.feature` behavior file; fixture from OR Temperature
  Control (sanitized).
- CLI implementation slice: `init` / `explore` first (read-mostly),
  binding/checkpoint next (they have the most invariants to honor).
- GitHub-identity DX note from #3 (`gh` multi-account, `GH_TOKEN`
  override) lands in CONTRIBUTING as a workflow note, not a spec.

## Traceability

- Field report: [slusset/buzz-solo#3](https://github.com/slusset/buzz-solo/issues/3)
- Personas: [`../personas/sovereign-builder.md`](../personas/sovereign-builder.md),
  [`../personas/domain-steward.md`](../personas/domain-steward.md)
- Journey: [`../journeys/steward-a-domain-context.md`](../journeys/steward-a-domain-context.md)
- Stories: `../stories/sticky-attention/initialize-a-durable-context-root.md`,
  `../stories/sticky-attention/explore-bounded-contexts.md`,
  `../stories/sticky-attention/declare-a-domain-beacon.md`
- Chain: [`../contracts/agent-harness/durable-context-hooks.yaml`](../contracts/agent-harness/durable-context-hooks.yaml),
  [`../models/sticky-attention/bounded-attention-context.model.yaml`](../models/sticky-attention/bounded-attention-context.model.yaml)
- Adjacent: [`skill-distribution-loop-v0.1.md`](skill-distribution-loop-v0.1.md),
  [`node-host-boundary-v0.1.md`](node-host-boundary-v0.1.md),
  [`journal-handoff-v0.1.md`](journal-handoff-v0.1.md)
