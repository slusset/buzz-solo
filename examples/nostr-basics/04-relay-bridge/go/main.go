package main

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/gorilla/websocket"
)

type readResult struct {
	data []byte
	err  error
}

func readMessages(conn *websocket.Conn, output chan<- readResult) {
	for {
		_, data, err := conn.ReadMessage()
		output <- readResult{data: data, err: err}
		if err != nil {
			return
		}
	}
}

func bridgeKinds() []int {
	value := os.Getenv("NOSTR_BRIDGE_KINDS")
	if value == "" {
		return []int{1}
	}
	var result []int
	for _, part := range strings.Split(value, ",") {
		var kind int
		if _, err := fmt.Sscanf(strings.TrimSpace(part), "%d", &kind); err == nil {
			result = append(result, kind)
		}
	}
	if len(result) == 0 {
		return []int{1}
	}
	return result
}

func main() {
	if len(os.Args) < 3 {
		panic("usage: nostr-04-relay-bridge <source-ws-url> <sink-ws-url>")
	}
	sourceURL, sinkURL := os.Args[1], os.Args[2]
	if sourceURL == sinkURL {
		panic("source and sink must be different relay URLs")
	}

	source, _, err := websocket.DefaultDialer.Dial(sourceURL, nil)
	if err != nil {
		panic(err)
	}
	defer source.Close()
	sink, _, err := websocket.DefaultDialer.Dial(sinkURL, nil)
	if err != nil {
		panic(err)
	}
	defer sink.Close()

	const subscriptionID = "bridge-source"
	filter := map[string]any{"kinds": bridgeKinds()}
	if err := source.WriteJSON([]any{"REQ", subscriptionID, filter}); err != nil {
		panic(err)
	}
	fmt.Printf("subscribed to %s: %v\nforwarding signed events to %s\n", sourceURL, filter, sinkURL)

	sourceMessages := make(chan readResult)
	sinkMessages := make(chan readResult)
	go readMessages(source, sourceMessages)
	go readMessages(sink, sinkMessages)

	for {
		select {
		case result := <-sourceMessages:
			if result.err != nil {
				fmt.Println("source closed:", result.err)
				_ = source.WriteJSON([]any{"CLOSE", subscriptionID})
				return
			}
			var envelope []json.RawMessage
			if err := json.Unmarshal(result.data, &envelope); err != nil || len(envelope) == 0 {
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
				var eventMeta struct {
					ID string `json:"id"`
				}
				if err := json.Unmarshal(envelope[2], &eventMeta); err != nil {
					panic(err)
				}
				// Keep the received event object byte-for-byte intact. The bridge
				// forwards the original signature; it does not rebuild or re-sign it.
				if err := sink.WriteJSON([]any{"EVENT", json.RawMessage(envelope[2])}); err != nil {
					panic(err)
				}
				fmt.Println("forwarded EVENT", eventMeta.ID)
			case "EOSE":
				fmt.Println("source EOSE: historical events are complete")
			case "NOTICE", "CLOSED":
				fmt.Println("source", label, string(result.data))
			}
		case result := <-sinkMessages:
			if result.err != nil {
				fmt.Println("sink closed:", result.err)
				return
			}
			var envelope []json.RawMessage
			if err := json.Unmarshal(result.data, &envelope); err != nil || len(envelope) == 0 {
				continue
			}
			var label string
			_ = json.Unmarshal(envelope[0], &label)
			if label == "OK" || label == "NOTICE" || label == "CLOSED" {
				fmt.Println("sink", label, string(result.data))
			}
		}
	}
}
