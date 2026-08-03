use std::error::Error;

use futures_util::{SinkExt, StreamExt};
use nostr::{EventBuilder, Keys, Kind};
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let relay = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:3100".to_string());
    let content = std::env::var("NOSTR_CONTENT")
        .unwrap_or_else(|_| "hello from the NIP-01 Rust publisher".to_string());
    let keys = Keys::generate();
    let event = EventBuilder::new(Kind::TextNote, content).sign_with_keys(&keys)?;
    let event_id = event.id.to_hex();

    println!("connecting to {relay}");
    let (mut ws, _) = connect_async(&relay).await?;
    ws.send(Message::Text(
        serde_json::json!(["EVENT", event]).to_string().into(),
    ))
    .await?;
    println!("sent EVENT {event_id}");

    while let Some(message) = ws.next().await {
        let message = message?;
        let Message::Text(text) = message else {
            continue;
        };
        let value: Value = serde_json::from_str(text.as_ref())?;
        let Some(array) = value.as_array() else {
            continue;
        };
        match array.first().and_then(Value::as_str) {
            Some("OK") if array.get(1).and_then(Value::as_str) == Some(&event_id) => {
                let accepted = array.get(2).and_then(Value::as_bool).unwrap_or(false);
                let reason = array.get(3).and_then(Value::as_str).unwrap_or("");
                println!("relay OK: accepted={accepted} reason={reason:?}");
                break;
            }
            Some("NOTICE") => println!("relay NOTICE: {}", array.get(1).unwrap_or(&Value::Null)),
            Some(other) => println!("relay {other}: {value}"),
            None => {}
        }
    }

    Ok(())
}
