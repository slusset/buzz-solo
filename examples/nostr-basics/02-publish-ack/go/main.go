package main

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/gorilla/websocket"
	nostr "github.com/nbd-wtf/go-nostr"
)

func main() {
	relay := "ws://127.0.0.1:3100"
	if len(os.Args) > 1 {
		relay = os.Args[1]
	}
	content := os.Getenv("NOSTR_CONTENT")
	if content == "" {
		content = "hello from the NIP-01 Go publisher"
	}

	secretKey := nostr.GeneratePrivateKey()
	publicKey, err := nostr.GetPublicKey(secretKey)
	if err != nil {
		panic(err)
	}
	event := nostr.Event{PubKey: publicKey, CreatedAt: nostr.Now(), Kind: 1, Content: content}
	if err := event.Sign(secretKey); err != nil {
		panic(err)
	}

	conn, _, err := websocket.DefaultDialer.Dial(relay, nil)
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	if err := conn.WriteJSON([]any{"EVENT", event}); err != nil {
		panic(err)
	}
	fmt.Println("sent EVENT", event.ID)

	for {
		_, data, err := conn.ReadMessage()
		if err != nil {
			panic(err)
		}
		var envelope []json.RawMessage
		if err := json.Unmarshal(data, &envelope); err != nil || len(envelope) == 0 {
			continue
		}
		var label string
		if err := json.Unmarshal(envelope[0], &label); err != nil {
			continue
		}
		switch label {
		case "OK":
			if len(envelope) < 4 {
				continue
			}
			var id string
			var accepted bool
			var reason string
			_ = json.Unmarshal(envelope[1], &id)
			_ = json.Unmarshal(envelope[2], &accepted)
			_ = json.Unmarshal(envelope[3], &reason)
			if id == event.ID {
				fmt.Printf("relay OK: accepted=%t reason=%q\n", accepted, reason)
				return
			}
		case "NOTICE":
			fmt.Println("relay NOTICE:", string(envelope[1]))
		default:
			fmt.Println("relay", label, string(data))
		}
	}
}
