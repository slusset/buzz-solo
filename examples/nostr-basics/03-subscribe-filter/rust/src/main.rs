use std::error::Error;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use nostr::Event;
use serde_json::{json, Value};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let relay = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:3100".to_string());
    let subscription_id = "learn-nip01";
    let mut filter = json!({"kinds": [1], "limit": 5});
    if let Ok(author) = std::env::var("NOSTR_AUTHOR") {
        filter["authors"] = json!([author]);
    }

    let (mut ws, _) = connect_async(&relay).await?;
    ws.send(Message::Text(
        json!(["REQ", subscription_id, filter]).to_string().into(),
    ))
    .await?;
    println!("sent REQ {subscription_id}: {filter}");

    let listening = timeout(Duration::from_secs(15), async {
        while let Some(message) = ws.next().await {
            let Message::Text(text) = message? else {
                continue;
            };
            let value: Value = serde_json::from_str(text.as_ref())?;
            let Some(array) = value.as_array() else {
                continue;
            };
            match array.first().and_then(Value::as_str) {
                Some("EVENT") if array.get(1).and_then(Value::as_str) == Some(subscription_id) => {
                    if let Some(raw_event) = array.get(2) {
                        let event: Event = serde_json::from_value(raw_event.clone())?;
                        println!(
                            "EVENT {} kind={} author={} content={:?}",
                            event.id.to_hex(),
                            event.kind.as_u16(),
                            event.pubkey.to_hex(),
                            event.content
                        );
                    }
                }
                Some("EOSE") => println!("EOSE: historical events are complete"),
                Some("CLOSED") => {
                    println!("CLOSED: {value}");
                    break;
                }
                Some("NOTICE") => println!("NOTICE: {value}"),
                _ => {}
            }
        }
        Ok::<(), Box<dyn Error>>(())
    })
    .await;

    if listening.is_err() {
        println!("listen window elapsed; sending CLOSE");
    }
    ws.send(Message::Text(
        json!(["CLOSE", subscription_id]).to_string().into(),
    ))
    .await?;
    Ok(())
}
