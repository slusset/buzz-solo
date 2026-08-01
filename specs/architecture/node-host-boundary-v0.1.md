# Node Host Boundary v0.1

Status: draft — design exploration
Date: 2026-07-31

## Decision

Each Principal Node binds a RuntimeInstance to a host machine through declared
**host capability ports**, the same way the portable relay core binds to
runtimes through relay ports. A host adapter declares what its host can do —
supervision, placement, secret custody, clock/wake delivery, session signals,
and attestation — and the RuntimeInstance consumes those capabilities without
letting PrincipalNode application policy touch OS APIs directly.

The consumer of these ports is a
[NodeRuntimeInstance](principal-node-boundary-v0.1.md) executing for one
authorized PrincipalNode. The host never runs a parallel domain workflow: it
may start, stop, and wake a RuntimeInstance, but it does not own PrincipalNode
authorization, synchronization, cursor advancement, retry semantics,
agreement evaluation, checkpointing, or coherence policy.

Two things become first-class that are conventions today:

1. **The user session is a signal, not a ritual.** OS login/lock and agent
   session start/end reach the node through a session-signal port. The
   [durable-context-hooks](../contracts/agent-harness/durable-context-hooks.yaml)
   contract stays the authoritative agent-session surface; the host adapter
   is its carrier.
2. **Principal continuity has an artifact form.** A signed,
   content-addressed **principal context artifact** combines PrincipalDomain
   reconstruction evidence with a PrincipalNode checkpoint: authorization,
   journal and cursor heads, selected release, profile digest, host binding,
   custody requirements, and context heads. Host migration and disaster
   recovery become defined operations instead of folklore.

A **passkey profile** names WebAuthn/FIDO2 user verification as the
mechanism that seals key material and gates capability minting.

## Why this boundary exists

The 2026-08-01 XDG migration exposed how the node had bound to exactly one host
by convention: legacy `~/.buzz-local` paths, hand-written launchd plists,
plaintext key files resolved by `reference` paths, separate pull/push jobs, an
agent skill that loads and saves context by discipline, and a NIP-OA
capability that silently expires after 30 days and is rebound by hand. Moving
the files made placement clearer, but did not itself declare the host contract.
The arrangement works because there is one machine and one careful operator;
without a capability boundary it does not travel:

- a second host (Linux box, new laptop, future phone) has no defined
  hydration path — the peel left "many nodes" as topology without a
  mechanism for standing one up from the journal;
- hosts differ in what they *can* do (Secure Enclave vs TPM vs nothing;
  launchd vs systemd; Touch ID vs security key vs no authenticator), and
  today those differences are invisible until something fails;
- the human is bound to the node socially, not cryptographically: nothing
  ties "the owner is present and consenting" to unsealing engram keys or
  minting an agent capability.

## Layered bounded contexts

The question "what is the bounded context around the portable local relay"
resolves into a stable domain, an authorized node, and an ephemeral execution
inside the host context:

```
┌────────────────────────────────────────────────────────────┐
│ HOST CONTEXT — machine, OS session, authenticators,        │
│ harnesses. Reached only through host capability ports.     │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ NODE RUNTIME INSTANCE — verified release + adapters   │ │
│  │  ┌──────────────────────┐ ┌─────────────────────────┐ │ │
│  │  │ PRINCIPAL NODE       │ │ PORTABLE RELAY CORE     │ │ │
│  │  │ sync, cursors,       │ │ events, filters,        │ │ │
│  │  │ checkpoints          │ │ decisions               │ │ │
│  │  └──────────┬───────────┘ └─────────────────────────┘ │ │
│  │             │ authorized by                          │ │
│  │  ┌──────────▼───────────────────────────────────────┐ │ │
│  │  │ PRINCIPAL DOMAIN — root context + journal       │ │ │
│  │  └──────────────────────────────────────────────────┘ │ │
│  └───────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

- The **relay core** is already bounded by
  [portable-relay-boundary](portable-relay-boundary.md).
- The **PrincipalDomain**, **PrincipalNode**, and **NodeRuntimeInstance** are
  bounded by [principal-node-boundary](principal-node-boundary-v0.1.md). The
  RuntimeInstance composes the application and consumes host ports; it is not
  another durable identity.
- The **attention contexts** inside the node are already bounded by the
  [bounded-attention-context model](../models/sticky-attention/bounded-attention-context.model.yaml):
  membership is the `h` tag, briefs are deterministic projections, and
  their filesystem artifacts are already content-addressed through the
  checkpoint manifest. Nothing here changes that.
- The **principal context artifact** is portable evidence, not a new aggregate.
  It carries what a host needs to reconstruct the PrincipalDomain and one
  authorized PrincipalNode without treating paths, processes, or keys as
  their stable identities.

Facts inside the host context (which launchd label, which keychain item,
which credential ID, which timer implementation) never appear in journal
events. Facts inside PrincipalDomain and PrincipalNode never depend on a
particular host's representation.

## Host capability ports

### Supervision

Ensure the selected NodeRuntimeInstance runs, restarts after crash and boot,
and reports process health. Adapters: launchd agent, systemd user unit,
foreground process, container, Durable Object alarm. Minimum conformance:
manual foreground invocation.

Supervision treats a RuntimeInstance as the process lifecycle unit while the
PrincipalNode remains the semantic lifecycle owner. Independently supervised
pull, push, cursor, or coherence jobs are transitional compatibility topology
and fail the target principal-boundary coherence invariant if they make domain
decisions outside the PrincipalNode.

### Placement

Resolve where config, durable node data, operational state, cache, installed
runtime releases, and content-addressed artifacts live. The current macOS
adapter uses the XDG-shaped layout under `~/.config`, `~/.local/share`,
`~/.local/state`, and `~/.local/lib`; another host may represent the same
roles differently. Placement is queried, never assumed — no other component
may hardcode a path. Placement also reports durability facts the principal
context artifact needs (filesystem identity, last-verified replay).

### Secret custody

Resolve a profile identity `reference` to a *signing capability*, not key
bytes where the class allows it. Declared custody classes:

| Class | Material location | User verification |
|---|---|---|
| `file` | plaintext file (today) | none |
| `os-keychain` | OS keychain / enclave-backed item | none or presence |
| `sealed` | file wrapped by a host-held KEK | per KEK policy |
| `passkey-sealed` | file wrapped by a passkey-PRF-derived KEK | user verification |
| `hardware` | non-exportable token (PIV/FIDO2) | presence or PIN+touch |

Every signing request carries a required verification level
(`none | presence | user-verification`); the port satisfies it with a
ceremony or fails closed. A custody class that cannot express the level
must refuse rather than silently downgrade — the same fail-closed posture
as the replication sink.

Hardware classes that cannot produce BIP-340 signatures (YubiKey, WebAuthn
authenticators) participate through the **attestation bridge** already in
operational use (`ssh-keygen -Y` binding of key A, serial 38031585): the
non-Nostr key attests the Nostr key, and the custody port records which
identities carry hardware attestation. Authenticators authorize and seal;
they never author events.

### Clock and wake

Provide wall and monotonic clocks plus scheduled and event-driven wake
delivery. The host chooses how a wake is delivered — launchd timer, systemd
timer, foreground loop, process callback, or platform alarm — and declares
its resolution and persistence properties.

The PrincipalNode owns cadence, debounce, retry classification, and what work a
wake causes. A wake means only “evaluate now.” It carries no trusted stream,
cursor, grant, peer policy, or reconciliation decision. A missed wake is
handled by PrincipalNode recovery policy through the current RuntimeInstance,
not by a second host-owned sync procedure.

### Session signals

Two signal families, one port:

**OS session** — login, unlock, lock, logout, shutdown. Node reactions are
policy, not hardcode; the sensible default:

- login/unlock → ensure supervision, optionally unseal `passkey-sealed`
  material if policy says unlock-implies-unseal (stricter policy defers to
  first use);
- lock/logout → **seal**: discard every unwrapped KEK and cached
  conversation key; the relay may keep serving public reads;
- shutdown → clean journal close, supervision handles restart.

**Agent session** — the host adapter implements the
[durable-context-hooks](../contracts/agent-harness/durable-context-hooks.yaml)
operations through each harness's native mechanism (hook scripts, plugin
callbacks, wrapper processes) and is the component that:

- injects the scoped environment (`BUZZ_RELAY_URL`, profile selection)
  into managed agent processes;
- requests capability minting at binding time (below);
- guarantees `on_session_end` best-effort delivery and feeds
  `host_process_confirmed_dead` abandonment evidence to reconciliation —
  the host is the only party that can actually confirm a dead process.

Context policy remains authoritative over binding, disclosure, and residue
exactly as the hooks contract states; the host adapter adds no authority,
only carriage.

### Attestation

What this host can prove: hardware-key attestation, WebAuthn credential
properties (authenticator attachment, UV capability), platform integrity
claims if any. Consumed when minting capabilities (a capability may record
the verification level that authorized it) and when hydrating a node onto
a new host (the arrival is witnessed, not assumed).

## Host capability claim and PrincipalNode binding

A host attestation identity signs one immutable capability claim per host
revision:

```text
{
  host_label,                      # presentation only, never identity
  supervision:  { kind, boot_persistent },
  placement:    { config, data, state, cache, runtime, artifact_store },
  custody:      [ {class, verification_levels, attestation} ],
  clock_wake:   { clocks, scheduled, event_driven, minimum_resolution },
  sessions:     { os_signals: [...], harness_adapters: [...] },
  attestation:  [ supported forms ]
}
```

Conformance is judged per claimed capability — a host that declares no
authenticator is conformant with core; it simply cannot claim the passkey
profile. This is how "different hosts have different capabilities" stays a
feature instead of an error: the PrincipalNode verifies the claim, signs a
bounded host binding, and degrades explicitly (e.g. refuses to place
`passkey-sealed` material on a host without an authenticator) rather than
discovering gaps at ceremony time. A host claim alone is never usable
PrincipalNode or PrincipalDomain authority.

## Passkey profile (`node-host-passkey-v0.1`)

WebAuthn/FIDO2 is the one user-verification mechanism that spans platform
authenticators (Touch ID today) and roaming keys (the existing YubiKeys)
with a uniform ceremony. The profile:

1. **Seal by PRF.** A discoverable credential with the PRF/`hmac-secret`
   extension evaluates a per-node salt to derive a KEK. Node key material
   and engram conversation keys at rest are wrapped by that KEK
   (`passkey-sealed` custody class). Unsealing *is* the ceremony —
   Touch ID or key tap — and sealing is discarding the unwrapped KEK.
2. **Gate minting.** NIP-OA agent capability minting and rebinding require
   a user-verification ceremony. The silent 30-day expiry stops being a
   maintenance chore and becomes the intended rhythm: rebinding is one
   tap, performed knowingly.
3. **Leave residue.** A ceremony may append a metadata-only witness record
   to the journal: purpose (`unseal | mint | hydrate`), verification
   level, hashed credential ID, timestamp. User presence becomes auditable
   history in the same journal as everything else. No WebAuthn payloads,
   challenges, or raw credential IDs enter events.
4. **Never author.** Passkey keys are ES256/EdDSA and stay outside event
   authorship entirely — consistent with
   [portable-relay-identity-v0.1](portable-relay-identity-v0.1.md):
   authentication evidence is never event history.
5. **Survive loss.** A sealed recovery envelope (offline-held recovery
   code wrapping the same KEK) is mandatory before `passkey-sealed` is
   allowed to be the *only* custody of any key. Losing an authenticator
   must cost a recovery ceremony, never the engram history.

## The principal context artifact

**Is the bounded context in artifact form? Yes — in three layers, two of
which already exist:**

1. **Attention-context artifacts** — already specced: checkpointed
   filesystem artifacts, content-addressed, manifest-last
   (`ArtifactManifestHead`).
2. **The journal** — already portable by construction: an NDJSON file of
   signed events is its own archive.
3. **The principal context artifact** — new, and the piece that makes domain
   and node continuity portable. It contains a PrincipalNode checkpoint and
   enough PrincipalDomain evidence for journal reconstruction:

```text
{
  principal_domain_id,
  domain_root_authority_ref,
  principal_node_id,
  principal_node_authorization_ref,
  checkpoint_signer_ref,
  runtime:        node/vX.Y.Z signed tag reference + RuntimeInstance id,
  journal_head:   { last_event_id, event_count, content_hash },
  profile_hash:   hash of the profile document (references only —
                  key material never enters the artifact),
  host_claim_hash,
  host_binding_hash,
  cursor_heads:   [ (source_stream_id, opaque_cursor_digest) ],
  custody_needs:  custody class + verification level per identity role,
  context_heads:  [ (h, head event ids) ]   # pointers into the journal,
  artifact_manifest_heads: [ ... ],         # pointers, not payloads
  sealed_envelope_ref:  content address of the wrapped-secrets blob,
                        travelling separately under custody rules
}
```

The artifact is **inventory and integrity, never self-authorizing**. Domain
authority comes from the domain-root chain; node authority comes from the
current PrincipalNode authorization; host use comes from claim plus binding.
The journal remains the source of durable domain history, and disagreement
with journal replay fails hydration closed.

**Hydration** (new host): verify domain and node authorization → fetch runtime
by signed tag → place journal and profile per the placement port → verify
replay against `journal_head` and cursor heads →
provision custody (unwrap the envelope via recovery or passkey ceremony;
re-provision and re-attest `hardware` classes, which never travel) →
verify the host claim → sign the PrincipalNode host binding → create a new
RuntimeInstance → register supervision → append a witnessed `node-hydrated`
record.
**Dehydration** is the inverse and may ride the existing artifact custody
legs for transport. Host migration and disaster recovery become the same
operation with different moods.

Bounded attention contexts are *not* re-serialized by the artifact — they
are h-scoped event sets inside the journal, and the artifact only points
at their heads. One logical journal, many contexts, one current checkpoint per
PrincipalNode.

## Conformance sketch

- `node-host-core-v0.1` — placement + `file` custody + manual
  supervision + foreground clock/wake + principal-context dehydrate/hydrate round trip
  on one host. (The current laptop setup, named.)
- `node-host-session-v0.1` — OS session seal/unseal policy honored;
  agent-session hooks carried per the durable-context-hooks contract with
  host-confirmed abandonment evidence.
- `node-host-passkey-v0.1` — PRF sealing, gated minting, ceremony
  residue, recovery envelope proven by drill (an actual restore, not an
  assertion).
- `node-host-migration-v0.1` — full dehydrate → transport → hydrate onto
  a second host with journal replay equality and re-attested identities.
  Named acceptance test:
  [the resurrection drill](resurrection-drill-v0.1.md).

## Non-goals

- No change to relay protocol semantics or the portable relay boundary.
- No host or RuntimeInstance ownership of PrincipalNode authorization,
  synchronization, cursor, retry, checkpoint, or coherence semantics.
- No multi-owner or shared-host semantics.
- No biometric identity claims — a ceremony proves *an enrolled
  authenticator verified its user*, nothing more.
- No browser-based WebAuthn RP requirement: the first adapter may use
  platform APIs (macOS LocalAuthentication / ASAuthorization) or a local
  FIDO2 library against the existing YubiKeys; the profile is about the
  ceremony semantics, not a web origin.

## Open questions

- PRF availability differs by platform and authenticator; the macOS
  adapter needs a spike to confirm Touch ID-backed PRF (or falls back to
  enclave-held KEK in `os-keychain` class with passkey gating only the
  *release policy*).
- Whether ceremony witness records belong in the default `h` boundary or
  a dedicated node-operations context.
- Whether the host capability claim itself should live in the journal as a
  replaceable event (host facts in events contradicts the layering rule
  above — leaning no: it stays a host-local signed file, referenced by
  hash from hydration records).

## Traceability

- Telos: [`../TELOS.md`](../TELOS.md)
- Relay boundary: [`portable-relay-boundary.md`](portable-relay-boundary.md)
- Principal Domain and Principal Node boundary:
  [`principal-node-boundary-v0.1.md`](principal-node-boundary-v0.1.md)
- Host capability claim model:
  [`../models/node-host/host-capability-claim.model.yaml`](../models/node-host/host-capability-claim.model.yaml)
- Identity profile: [`portable-relay-identity-v0.1.md`](portable-relay-identity-v0.1.md)
- Agent-session surface:
  [`../contracts/agent-harness/durable-context-hooks.yaml`](../contracts/agent-harness/durable-context-hooks.yaml)
- Inner bounded context:
  [`../models/sticky-attention/bounded-attention-context.model.yaml`](../models/sticky-attention/bounded-attention-context.model.yaml)
- Runtime channel: [`node-release-distribution-v0.1.md`](node-release-distribution-v0.1.md)
- Profile layout: `crates/buzz-cli/CONTEXT.md`
- Hardware precedent: YubiKey attestation bridge (`ssh-keygen -Y`,
  operational since 2026-07)
