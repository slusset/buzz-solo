# 02 — publish and acknowledge

This lesson adds one relay connection. The client signs a kind `1` event and
sends the NIP-01 client-to-relay frame:

```json
["EVENT", {"id":"...", "pubkey":"...", "created_at":0, "kind":1, "tags":[], "content":"...", "sig":"..."}]
```

The relay answers with `OK`, whose boolean says whether it accepted the event.
An `OK` response is not a new event and the relay does not replace the event's
author or signature.

```bash
cargo run --manifest-path examples/nostr-basics/02-publish-ack/rust/Cargo.toml -- ws://127.0.0.1:3100
cd examples/nostr-basics/02-publish-ack/go && go mod tidy && go run . ws://127.0.0.1:3100
```
