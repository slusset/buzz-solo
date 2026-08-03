# NIP-01 basics in Rust and Go

This directory is a deliberately small, wire-oriented Nostr lab. Every lesson
has two standalone projects:

```text
lesson/
  rust/   # uses this repository's workspace nostr crate
  go/     # uses go-nostr plus a small WebSocket client
```

The code favors explicit JSON arrays and WebSocket frames so the protocol is
visible while learning. It is not intended to replace a production relay SDK.

## Learning path

| Lesson | What to watch | Rust | Go |
| --- | --- | --- | --- |
| 01 event envelope | keypair, canonical event preimage, SHA-256 id, Schnorr signature, verification | [`01-event-envelope/rust`](01-event-envelope/rust) | [`01-event-envelope/go`](01-event-envelope/go) |
| 02 publish and ack | `EVENT` from client to relay and `OK` back from relay | [`02-publish-ack/rust`](02-publish-ack/rust) | [`02-publish-ack/go`](02-publish-ack/go) |
| 03 subscribe and filter | `REQ`, filter matching, historical `EVENT`, `EOSE`, live updates, `CLOSE` | [`03-subscribe-filter/rust`](03-subscribe-filter/rust) | [`03-subscribe-filter/go`](03-subscribe-filter/go) |
| 04 relay bridge | one process acting as a client to a source relay and a sink relay | [`04-relay-bridge/rust`](04-relay-bridge/rust) | [`04-relay-bridge/go`](04-relay-bridge/go) |

The default relay URL is `ws://127.0.0.1:3100`, matching the local relay
development convention in this repository. Each network lesson accepts a
relay URL as its first argument where appropriate.

```bash
. ./bin/activate-hermit
just local-relay

cargo run --manifest-path examples/nostr-basics/02-publish-ack/rust/Cargo.toml
go run ./examples/nostr-basics/02-publish-ack/go
```

The Go toolchain is not part of the Rust workspace. From a Go lesson directory,
run `go mod tidy` once and then use `go run .`.

## The mental model

NIP-01 gives clients and relays a common event object and a small WebSocket
vocabulary. A client normally opens one WebSocket per relay and fans out its
publishes or subscriptions. Relays do not share a canonical global database,
so different relays can have different views of the same pubkey's events.

The bridge in lesson 04 makes that last point tangible. It subscribes to one
relay, receives signed events, and sends the same signed event objects to a
second relay. It does not re-sign or invent a relay identity. In a larger Buzz
design, this is the seam where a portable relay, a local relay, or a peer node
can be composed, but the forwarding policy still belongs to the application.

## Interface experiments

These examples intentionally keep the protocol core independent from the UI.
That leaves clean future adapters for:

- a SwiftUI/macOS app that renders the `EVENT`, `EOSE`, `OK`, and `NOTICE`
  messages;
- a Rust library exposed through a C ABI or a small local JSONL process;
- a Go service that owns relay connections while a native client navigates the
  context.

The next useful UI exercise is to wrap lesson 03 in a JSONL command/event
boundary rather than putting Swift-specific behavior in the Nostr layer.
