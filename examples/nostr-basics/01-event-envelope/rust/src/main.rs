use std::error::Error;

use nostr::{EventBuilder, JsonUtil, Keys, Kind, Tag};
use serde_json::json;

fn main() -> Result<(), Box<dyn Error>> {
    let keys = Keys::generate();
    let tag = Tag::parse(["t", "nip-01"])?;
    let event = EventBuilder::new(Kind::TextNote, "hello from the NIP-01 Rust lab")
        .tags([tag])
        .sign_with_keys(&keys)?;

    let canonical = json!([
        0,
        event.pubkey.to_hex(),
        event.created_at.as_secs(),
        event.kind.as_u16(),
        event.tags,
        event.content,
    ]);
    let canonical_json = serde_json::to_string(&canonical)?;

    println!("pubkey: {}", event.pubkey.to_hex());
    println!("canonical preimage: {canonical_json}");
    println!("event id: {}", event.id.to_hex());
    println!("event JSON: {}", event.as_json());
    println!("id verifies: {}", event.verify_id());
    println!("signature verifies: {}", event.verify_signature());

    Ok(())
}
