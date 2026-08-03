# Go implementation

The bridge uses one raw WebSocket connection to the source and another to the
sink. A reader goroutine feeds each connection into a small `select` loop so
source events and sink acknowledgements remain visible.

```bash
go mod tidy
go run . ws://127.0.0.1:3100 ws://127.0.0.1:3200
```
