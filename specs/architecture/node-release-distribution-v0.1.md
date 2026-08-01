# Node release distribution v0.1

Status: active. First release: `node/v0.1.0`. The consumer-side update
command and `doctor` closure land with the managed context CLI
(handoff `075aa020…`); until then the consumer procedure below is manual.

## Purpose

One-way distribution of node runtime releases — relay, CLIs, skills, signing
tooling, and host-adapter assets — from a single development node to
downstream consumer nodes, over ordinary Git, with releases attributable to
the release-signing role associated with the development PrincipalNode. The
release signer does not become the PrincipalDomain or PrincipalNode identity.

Roles are fixed by policy, not capability:

- **Development PrincipalNode** (`ted-laptop`): the only PrincipalNode where
  runtime changes happen. Builds, specs, and releases originate here.
- **Consumer PrincipalNodes** (downstream hosts): verify, check out, rebuild, test,
  and generate feedback. Consumer nodes never edit installed executables;
  they configure profiles.

## Channel

- Repository: the Git fork both nodes already use.
- Release = an annotated tag in the `node/vX.Y.Z` namespace.
- Every release tag is signed with the pinned development release key (NIP-GS,
  BIP-340), publicly attested through the hardware binding statement. That key
  may currently overlap another operational role, but its release signature is
  provenance evidence only and never domain-root or PrincipalNode authorization.

Release key (pinned):

```text
9c2fd8696a630bdf27c3d54394739bb3cbbf81b7cf7fa1e205a806eca33fa90e
```

## PrincipalNode boundary consequence

The signed release is evidence selected by a PrincipalNode and executed by a
[NodeRuntimeInstance](principal-node-boundary-v0.1.md). It proves provenance
and integrity and may declare compatibility; it never identifies a
PrincipalDomain or PrincipalNode and never authorizes journal replay, event
admission, declaration changes, or cursor movement.

The current v0.1 channel proves the signed tag and source revision. The target
promotion contract, to be specified behaviorally before implementation, adds
a canonical release manifest declaring:

- runtime version, source revision, and artifact digest;
- supported profile, journal, cursor, checkpoint, host-claim, and host-binding schemas;
- the required host capability profile;
- migration identifiers and compatibility direction.

Before a promoted RuntimeInstance first mutates durable state, the
PrincipalNode compatibility gate must match that evidence against the current
PrincipalDomain state, PrincipalNode checkpoint, and active host binding. A
migration must name a precondition, postcondition, recovery point, and any
irreversible boundary. Rollback selects different verified executable bytes
and creates a new RuntimeInstance; it never rolls journal or committed cursor
state backward implicitly.

## Consumer procedure

On each update check:

1. `git fetch origin --tags`
2. Select the newest `node/v*` tag.
3. Verify the signature and **pin the key explicitly** — `TRUST_FULLY` is
   advisory (config-match-only) and is never sufficient:

   ```bash
   git verify-tag --raw <tag> 2>&1 \
     | grep -q "GOODSIG 9c2fd8696a630bdf27c3d54394739bb3cbbf81b7cf7fa1e205a806eca33fa90e"
   ```

4. Check out the verified tag; never track a moving branch.
5. Rebuild from source (`cargo install --locked --path …` / the repo's
   install target). No prebuilt artifacts at two-node scale.
6. Run `doctor` once available; it must attest the installed executables
   match the checked-out release.
7. Append a signed session record to the shared context: tag, result, and
   any defect found. This residue is the feedback channel.

## Trust bootstrap

The verifier (`git-sign-nostr`) is built from the same repository it
verifies. First installation on a new consumer node is trust-on-first-use;
every subsequent update is verified by the previous installation's
verifier. TOFU happens once per node and should be recorded as a session
record when it happens.

## What never travels through this channel

Private keys, journals, replication cursors, profiles, artifact stores, and
all durable relay state. Git moves capability; the node keeps memory.
Distribution of context is the sovereign-sync layer's job, not this
channel's.

## Verification of the loop

The development node observes update health from the journal, not from the
consumer's machine: each consumer update produces a signed session record
that drains home through the existing shared streams. An update that leaves
no residue is indistinguishable from an update that never ran — silence is
a finding. Folding installed-version reporting into the steward's cycle is
the v0.2 candidate that makes version skew a witnessed condition rather
than a checkable one.

## Out of scope (v0.1)

- Automatic/unattended updates on consumer nodes.
- Prebuilt release artifacts and multi-platform binaries.
- Multi-key or threshold release signing.
- Revocation of a compromised release key (rotate by publishing a new
  binding statement and re-pinning consumers manually).

## Traceability

- Principal Domain and Principal Node boundary:
  [`principal-node-boundary-v0.1.md`](principal-node-boundary-v0.1.md)
- Promotion story:
  [`../stories/principal-node/promote-compatible-node-runtime.md`](../stories/principal-node/promote-compatible-node-runtime.md)
- PrincipalNode model:
  [`../models/principal-node/principal-node.model.yaml`](../models/principal-node/principal-node.model.yaml)
- RuntimeInstance model:
  [`../models/principal-node/node-runtime-instance.model.yaml`](../models/principal-node/node-runtime-instance.model.yaml)
