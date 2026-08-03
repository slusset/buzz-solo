package main

import (
	"encoding/json"
	"fmt"

	nostr "github.com/nbd-wtf/go-nostr"
)

func main() {
	secretKey := nostr.GeneratePrivateKey()
	publicKey, err := nostr.GetPublicKey(secretKey)
	if err != nil {
		panic(err)
	}

	event := nostr.Event{
		PubKey:    publicKey,
		CreatedAt: nostr.Now(),
		Kind:      1,
		Tags:      nostr.Tags{{"t", "nip-01"}},
		Content:   "hello from the NIP-01 Go lab",
	}
	if err := event.Sign(secretKey); err != nil {
		panic(err)
	}

	var canonical any
	if err := json.Unmarshal(event.Serialize(), &canonical); err != nil {
		panic(err)
	}
	canonicalJSON, err := json.Marshal(canonical)
	if err != nil {
		panic(err)
	}
	eventJSON, err := json.MarshalIndent(event, "", "  ")
	if err != nil {
		panic(err)
	}

	fmt.Println("pubkey:", event.PubKey)
	fmt.Println("canonical preimage:", string(canonicalJSON))
	fmt.Println("event id:", event.ID)
	fmt.Println("event JSON:", string(eventJSON))
	fmt.Println("id verifies:", event.CheckID())
	signatureValid, err := event.CheckSignature()
	if err != nil {
		panic(err)
	}
	fmt.Println("signature verifies:", signatureValid)
}
