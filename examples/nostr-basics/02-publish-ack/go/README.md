# Go implementation

This version uses `go-nostr` to sign the event and `gorilla/websocket` only for
the transport. The `EVENT` and `OK` arrays are assembled and decoded directly.

```bash
go mod tidy
go run . ws://127.0.0.1:3100
```
