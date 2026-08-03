# 01 — event envelope

This is the offline starting point. It creates a keypair, builds a kind `1`
text note, prints the canonical NIP-01 serialization used to derive the event
ID, and verifies both the ID and Schnorr signature.

There is no relay connection. The important distinction is:

```text
event id = sha256(canonical [0, pubkey, created_at, kind, tags, content])
signature = Schnorr-sign(event id, private key)
```

Run either implementation:

```bash
cargo run --manifest-path examples/nostr-basics/01-event-envelope/rust/Cargo.toml
cd examples/nostr-basics/01-event-envelope/go && go run .
```
