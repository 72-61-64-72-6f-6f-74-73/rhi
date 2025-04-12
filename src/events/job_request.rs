use anyhow::Result;
use nostr::{event::Kind, filter::Filter, key::Keys, types::Timestamp};
use nostr_sdk::Client;
use nostr_sdk::RelayPoolNotification;
use tracing::info;

use crate::KIND_JOB_REQUEST;

pub async fn subscriber(keys: Keys, relays: Vec<String>) -> Result<()> {
    info!("Starting subscriber for kind {}", KIND_JOB_REQUEST);
    let client = Client::new(keys.clone());

    for relay in &relays {
        client.add_relay(relay).await?;
    }

    let filter = Filter::new()
        .kind(Kind::Custom(KIND_JOB_REQUEST))
        .since(Timestamp::now());

    client.connect().await;
    client.subscribe(filter, None).await?;

    let mut notifications = client.notifications();

    while let Ok(n) = notifications.recv().await {
        if let RelayPoolNotification::Event { event, .. } = n {
            if event.kind == Kind::Custom(KIND_JOB_REQUEST) {
                info!("Receieved job request event: {:?}", { event.clone() });
            }
        }
    }

    client.disconnect().await;

    Ok(())
}
