# Go implementation

The Go version uses a raw WebSocket connection so the NIP-01 envelopes stay
visible. It reads `NOSTR_AUTHOR` and applies it to the filter when present.

```bash
go mod tidy
go run . ws://127.0.0.1:3100
```
