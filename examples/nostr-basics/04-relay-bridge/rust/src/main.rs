use std::error::Error;

use futures_util::{SinkExt, StreamExt};
use nostr::Event;
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const SUBSCRIPTION_ID: &str = "bridge-source";

fn kinds() -> Vec<u16> {
    std::env::var("NOSTR_BRIDGE_KINDS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|part| part.trim().parse::<u16>().ok())
                .collect()
        })
        .filter(|values: &Vec<u16>| !values.is_empty())
        .unwrap_or_else(|| vec![1])
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let source_url = std::env::args()
        .nth(1)
        .ok_or("usage: nostr-04-relay-bridge <source-ws-url> <sink-ws-url>")?;
    let sink_url = std::env::args()
        .nth(2)
        .ok_or("usage: nostr-04-relay-bridge <source-ws-url> <sink-ws-url>")?;
    if source_url == sink_url {
        return Err("source and sink must be different relay URLs".into());
    }

    let (source, _) = connect_async(&source_url).await?;
    let (sink, _) = connect_async(&sink_url).await?;
    let (mut source_write, mut source_read) = source.split();
    let (mut sink_write, mut sink_read) = sink.split();

    let filter = json!({"kinds": kinds()});
    source_write
        .send(Message::Text(
            json!(["REQ", SUBSCRIPTION_ID, filter]).to_string().into(),
        ))
        .await?;
    println!("subscribed to {source_url}: {filter}");
    println!("forwarding signed events to {sink_url}");

    loop {
        tokio::select! {
            source_message = source_read.next() => {
                let Some(message) = source_message else { break; };
                let Message::Text(text) = message? else { continue; };
                let value: Value = serde_json::from_str(text.as_ref())?;
                let Some(array) = value.as_array() else { continue; };
                match array.first().and_then(Value::as_str) {
                    Some("EVENT") if array.get(1).and_then(Value::as_str) == Some(SUBSCRIPTION_ID) => {
                        let raw_event = array.get(2).ok_or("source EVENT has no event payload")?;
                        let event: Event = serde_json::from_value(raw_event.clone())?;
                        let id = event.id.to_hex();
                        sink_write.send(Message::Text(
                            json!(["EVENT", event]).to_string().into(),
                        )).await?;
                        println!("forwarded EVENT {id}");
                    }
                    Some("EOSE") => println!("source EOSE: historical events are complete"),
                    Some("NOTICE") => println!("source NOTICE: {value}"),
                    Some("CLOSED") => println!("source CLOSED: {value}"),
                    _ => {}
                }
            }
            sink_message = sink_read.next() => {
                let Some(message) = sink_message else { break; };
                let Message::Text(text) = message? else { continue; };
                let value: Value = serde_json::from_str(text.as_ref())?;
                match value.as_array().and_then(|array| array.first()).and_then(Value::as_str) {
                    Some("OK") | Some("NOTICE") | Some("CLOSED") => println!("sink: {value}"),
                    _ => {}
                }
            }
        }
    }

    source_write
        .send(Message::Text(
            json!(["CLOSE", SUBSCRIPTION_ID]).to_string().into(),
        ))
        .await?;
    Ok(())
}
