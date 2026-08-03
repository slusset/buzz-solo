# Go implementation

This version uses `go-nostr` for secp256k1/Schnorr details but prints the same
NIP-01 fields as the Rust implementation. The library is intentionally kept at
the edge of the example so the canonical event preimage remains visible.

```bash
go mod tidy
go run .
```
