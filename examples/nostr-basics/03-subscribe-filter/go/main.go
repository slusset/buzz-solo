package main

import (
	"encoding/json"
	"fmt"
	"os"
	"time"

	"github.com/gorilla/websocket"
	nostr "github.com/nbd-wtf/go-nostr"
)

func main() {
	relay := "ws://127.0.0.1:3100"
	if len(os.Args) > 1 {
		relay = os.Args[1]
	}
	const subscriptionID = "learn-nip01"

	filter := map[string]any{"kinds": []int{1}, "limit": 5}
	if author := os.Getenv("NOSTR_AUTHOR"); author != "" {
		filter["authors"] = []string{author}
	}

	conn, _, err := websocket.DefaultDialer.Dial(relay, nil)
	if err != nil {
		panic(err)
	}
	defer conn.Close()
	if err := conn.WriteJSON([]any{"REQ", subscriptionID, filter}); err != nil {
		panic(err)
	}
	fmt.Printf("sent REQ %s: %v\n", subscriptionID, filter)

	_ = conn.SetReadDeadline(time.Now().Add(15 * time.Second))
	for {
		_, data, err := conn.ReadMessage()
		if err != nil {
			if netError, ok := err.(interface{ Timeout() bool }); ok && netError.Timeout() {
				fmt.Println("listen window elapsed")
				break
			}
			panic(err)
		}

		var envelope []json.RawMessage
		if err := json.Unmarshal(data, &envelope); err != nil || len(envelope) == 0 {
			continue
		}
		var label string
		_ = json.Unmarshal(envelope[0], &label)
		switch label {
		case "EVENT":
			if len(envelope) < 3 {
				continue
			}
			var subID string
			_ = json.Unmarshal(envelope[1], &subID)
			if subID != subscriptionID {
				continue
			}
			var event nostr.Event
			if err := json.Unmarshal(envelope[2], &event); err != nil {
				panic(err)
			}
			fmt.Printf("EVENT %s kind=%d author=%s content=%q\n", event.ID, event.Kind, event.PubKey, event.Content)
		case "EOSE":
			fmt.Println("EOSE: historical events are complete")
		case "CLOSED":
			fmt.Println("CLOSED:", string(data))
			return
		case "NOTICE":
			fmt.Println("NOTICE:", string(data))
		}
	}

	_ = conn.WriteJSON([]any{"CLOSE", subscriptionID})
}
