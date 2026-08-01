---
id: domain-architect
type: persona
---

# Persona: Domain architect

## Role

The person accountable for keeping Buzz's intent, domain language, runtime
boundaries, and operational evidence coherent as the system evolves. The
domain architect works across the portable relay, sovereign node runtime,
host adapters, release process, and interfaces without allowing any one
implementation detail to become accidental authority.

## Goals

- Assign each state transition and operational responsibility to one explicit
  bounded context.
- Keep domain policy inside the node while allowing hosts and transports to
  vary through declared capabilities.
- Express architecture choices as models, invariants, ports, and observable
  conformance claims rather than diagrams alone.
- Make release compatibility, migration, rollback, and resurrection part of
  the trust model.
- Detect when code or live process topology no longer matches the declared
  architecture.
- Let several authorized actors evolve logically singular state without
  losing attribution, admissibility, or a verifiable history.

## Frustrations

- Shell scripts and service managers quietly accumulate domain workflow.
- A working process topology is mistaken for a coherent architecture.
- Host paths, credentials, scheduling, and protocol decisions become entangled
  until migration is the first time the boundaries are tested.
- Crate and module boundaries do not always reveal which layer owns a rule.
- A single green status can hide an unknown or violated critical invariant.
- Release engineering is treated as packaging even when runtime identity and
  state compatibility depend on it.

## Context

- Tech comfort: high in architecture and distributed systems; actively
  learning Rust and its tooling
- Usage frequency: continuous design work with focused reviews before each
  sovereign-surface implementation or release
- Key devices: development workstation, consumer nodes, and recovery hosts

## Quotes

> "If several actors can change one state, show me which transitions each may
> propose, what evidence accompanies them, and which boundary decides whether
> the result is admissible."
