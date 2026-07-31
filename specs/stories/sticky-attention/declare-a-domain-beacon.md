---
id: declare-a-domain-beacon
type: story
refs:
  journey: specs/journeys/steward-a-domain-context.md
  persona: specs/personas/domain-steward.md
  steps: [4, 6]
---

# Story: Declare a domain beacon

## Narrative

As a domain steward,
I want the domain to have one stable identity naming its context,
authoritative surfaces, and stewardship chain,
So that a future principal — human or agentic — can pick the domain up
from what the node knows.

## Acceptance Criteria

- [ ] The beacon is a parameterized-replaceable journal head
  (`d = beacon:{domain-slug}`) projected into the root as a readable file.
- [ ] It carries domain identity, context reference, surface references
  (URLs and titles only), stewardship (principal, collaborators, target
  handoff), current-work reference, and tooling provenance notes.
- [ ] Credentials, tokens, private keys, deployment secrets, and raw
  operational payloads are excluded by rule; a beacon containing them is
  rejected, not redacted.
- [ ] Replacing the beacon never destroys residue; stewardship changes are
  visible as head history.
- [ ] A stewardship handoff composes the beacon with the journal-handoff
  lifecycle; the beacon itself never transfers custody.
- [ ] "Domain beacon" is always qualified against beacon pulses
  (kind 20700) and the rotating-beacons exploration — three distinct
  concepts sharing a word.

## Notes

Demonstration fixture: the OR Temperature Control domain (VH), with its
Confluence/Jira surfaces and Periop-team target handoff — zero PHI or
credentials in any event, enforced by checkpoint policy patterns.
