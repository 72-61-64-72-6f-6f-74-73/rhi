use anyhow::Result;
use nostr::event::Event;
use nostr::{event::Kind, filter::Filter, key::Keys, types::Timestamp};
use nostr_sdk::Client;
use nostr_sdk::RelayPoolNotification;
use tracing::{info, warn};

use crate::KIND_JOB_REQUEST;
use crate::utils::nostr::nostr_event_job_request_feedback;

#[derive(thiserror::Error, Debug)]
pub enum JobRequestError {
    #[error("Failure to process request.")]
    Failure,
}

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
                let event = (*event).clone();
                let keys = keys.clone();
                let client = client.clone();

                tokio::spawn(async move {
                    if let Err(err) =
                        handle_event(event.clone(), keys.clone(), client.clone()).await
                    {
                        let _ = handle_error(err, event, keys, client).await;
                    }
                });
            }
        }
    }

    client.disconnect().await;

    Ok(())
}

async fn handle_error(
    error: JobRequestError,
    event: Event,
    keys: Keys,
    client: Client,
) -> Result<()> {
    warn!("job_request handle_error {}", error);

    let builder = nostr_event_job_request_feedback(&event, error, "error", None)?;
    let event_id = client.send_event_builder(builder).await?;

    warn!("job_request handle_error sent feedback {:?}", {
        event_id.clone()
    });
    Ok(())
}

async fn handle_event(event: Event, keys: Keys, client: Client) -> Result<(), JobRequestError> {
    let t = std::time::Instant::now();
    if t.elapsed().as_nanos() % 2 == 0 {
        info!("job_request handle_event {:?}", { event.clone() });
    } else {
        return Err(JobRequestError::Failure);
    }

    Ok(())
}
