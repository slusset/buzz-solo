# 03 — subscriptions and filters

This lesson is read-oriented. It sends a `REQ` with a kind `1` filter, prints
events until the relay's `EOSE`, continues listening briefly for live events,
then sends `CLOSE`.

Set `NOSTR_AUTHOR` to a lowercase 64-character pubkey to add an author filter.
The filter's `limit` applies only to the initial historical response; it does
not cap later live events.

```bash
NOSTR_AUTHOR=<pubkey> cargo run --manifest-path examples/nostr-basics/03-subscribe-filter/rust/Cargo.toml -- ws://127.0.0.1:3100
cd examples/nostr-basics/03-subscribe-filter/go
go mod tidy
NOSTR_AUTHOR=<pubkey> go run . ws://127.0.0.1:3100
```

Without `NOSTR_AUTHOR`, the example asks for the latest few kind `1` events.
