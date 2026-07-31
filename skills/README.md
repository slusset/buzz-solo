# Skills

Reviewed, distributable agent skills for Buzz Solo nodes. Canonical form:
`<name>/SKILL.md` with `name`, `description`, `origin`, and
`host_assumptions` frontmatter.

The loop (full spec:
[skill-distribution-loop-v0.1](../specs/architecture/skill-distribution-loop-v0.1.md)):

1. A node authors or discovers a skill and exercises it locally.
2. It flows upstream via a **skill-submission issue** on this repo.
3. Review at the home node (Nest): portability, host assumptions, no
   secret material — then a PR lands it here and merges to `main`.
4. Nodes pull `main` and install into their harness's skill directory
   (manual copy/symlink in v0.1).
5. Adoption leaves a one-line journal record on the adopting node.

Satellites never exchange skills laterally — the loop is a star through
`main`, so every node runs the same reviewed version.
