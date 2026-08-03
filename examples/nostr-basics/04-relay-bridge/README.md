# 04 — relay bridge

This is the first example that resembles `client <-> relay <-> peer-relay`.
The process opens two independent WebSocket connections:

```text
source relay ──EVENT──▶ bridge/client ──EVENT──▶ sink relay
```

It subscribes to kind `1` events on the source, forwards the received signed
event object unchanged to the sink, and prints the sink's `OK` responses. It
forwards the original event; it does not generate a new signature.

NIP-01 itself defines client-to-relay messages, not relay-to-relay forwarding.
Treat this bridge as a policy-bearing application component: in a real system
you would add allowlists, deduplication, backpressure, retry rules, and an
explicit trust boundary before forwarding events.

Run it with two relay URLs:

```bash
cargo run --manifest-path examples/nostr-basics/04-relay-bridge/rust/Cargo.toml -- \
  ws://127.0.0.1:3100 ws://127.0.0.1:3200

cd examples/nostr-basics/04-relay-bridge/go
go mod tidy
go run . ws://127.0.0.1:3100 ws://127.0.0.1:3200
```

The source and sink should be different relay instances. Both implementations
use `NOSTR_BRIDGE_KINDS` as an optional comma-separated list, defaulting to
kind `1`.
