# Examples

This directory contains reference material for building on Buzz beyond the desktop app and AI agents.

## `countdown-bot/`

A small non-AI bot that connects directly to the Buzz relay over WebSocket, authenticates with NIP-42, subscribes to one channel, and replies to deterministic commands like `!countdown 5` and `!fib 8`.

It demonstrates two identity paths:

1. **Standalone bot identity** — the bot authenticates with its own key and must be explicitly admitted to closed/allowlisted relays.
2. **Owner-attested / agent OAuth path** — the bot authenticates with its own key while presenting the same `BUZZ_AUTH_TAG` NIP-OA credential that Buzz agents receive from the owner/agent OAuth flow, so a relay can admit it because its owner is already a relay member.

See [`countdown-bot/README.md`](countdown-bot/README.md) for usage.

## `meadow-core/`

A persona-pack example for Buzz agents.

## `nostr-basics/`

A paired Rust and Go learning path for NIP-01. The projects start with signed
event envelopes, then move through `EVENT`/`OK`, `REQ`/`EVENT`/`EOSE`/`CLOSE`,
and finally a small relay bridge that behaves as a client to two relays.

See [`nostr-basics/README.md`](nostr-basics/README.md). The bridge is useful
for visualizing `client <-> relay <-> peer-relay`, but NIP-01 does not define a
relay-to-relay protocol: the bridge is an ordinary client process with two
WebSocket connections.
