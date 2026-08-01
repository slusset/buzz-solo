# macOS supervision templates

These templates are for the installed node runtime. They are intentionally not
loaded automatically: render them with absolute paths, inspect the result, and
then install them with `launchctl` as an explicit operational change.

The templates use `@NAME@` placeholders because launchd does not expand shell
variables inside `ProgramArguments`. At minimum, replace:

- `@BUZZ_RUNTIME_ROOT@` — normally `~/.local/lib/buzz/current`
- `@BUZZ_DATA_ROOT@` — the selected profile's durable data root
- `@BUZZ_STATE_ROOT@` — the selected profile's log/state root
- `@BUZZ_CREDENTIALS_ROOT@` — owner-only credential files
- `@BUZZ_PEER_TRUST@`, `@BUZZ_OWNER_PUBKEY@`, and `@BUZZ_NODE_LABEL@`
- `@BUZZ_RENDEZVOUS_URL@`, `@BUZZ_SOURCE_ID@`, and `@BUZZ_PULSE_WITNESS@`

The relay template passes `--artifacts` explicitly. This keeps the relay's
artifact store aligned with the profile instead of falling back to the legacy
`<journal>.artifacts` sidecar convention.
