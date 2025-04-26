use anyhow::Result;
use nostr::{
    event::{Event, Tag, TagKind},
    key::Keys,
};
use nostr_sdk::{Client, client::Error as NostrClientError};
use serde::Deserialize;
use thiserror::Error;
use tracing::info;

use crate::{
    events::job_request::{JobRequest, JobRequestError, JobRequestInput},
    models::event_classified::EventClassified,
    utils::nostr::{nostr_event_job_result, nostr_fetch_event_by_id, nostr_send_event},
};

#[derive(Debug, Error)]
pub enum JobRequestOrderError {
    #[error("Failed to parse reference event: {0}")]
    ParseReference(String),

    #[error("Failed to fetch reference event: {0}")]
    FetchReference(String),

    #[error("Reference event not found: {0}")]
    MissingReference(String),

    #[error("Reference event does not meet request requirements: {0}")]
    MissingRequested(String),

    #[error("Failed to send job response")]
    ResponseSend(#[from] NostrClientError),

    #[error("Request cannot be satisfied: {0}")]
    Unsatisfiable(String),
}

#[derive(Debug, Deserialize)]
pub struct JobRequestOrderDataQuantity {
    pub amount: f64,
    pub unit: String,
    pub count: u32,
    pub mass_g: f64,
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct JobRequestOrderDataPrice {
    pub amount: f64,
    pub currency: String,
    pub quantity_amount: f64,
    pub quantity_unit: String,
}

#[derive(Debug, Deserialize)]
pub struct JobRequestOrderDataOrder {
    pub price: JobRequestOrderDataPrice,
    pub quantity: JobRequestOrderDataQuantity,
}

#[derive(Debug, Deserialize)]
pub struct JobRequestOrderDataEvent {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct JobRequestOrderData {
    pub event: JobRequestOrderDataEvent,
    pub order: JobRequestOrderDataOrder,
}

pub async fn handle_job_request_order(
    event_job_request: Event,
    _keys: Keys,
    client: Client,
    _job_req: JobRequest,
    job_req_input: JobRequestInput,
) -> Result<(), JobRequestError> {
    let order_data: JobRequestOrderData = serde_json::from_str(&job_req_input.data)
        .map_err(|e| JobRequestOrderError::ParseReference(e.to_string()))?;

    let ref_id = &order_data.event.id;
    let ref_event = nostr_fetch_event_by_id(client.clone(), ref_id)
        .await
        .map_err(|_| JobRequestOrderError::FetchReference(ref_id.clone()))?;

    let ref_classified = EventClassified::from_event(&ref_event)
        .map_err(|_| JobRequestOrderError::ParseReference(ref_id.clone()))?;

    let order_result = ref_classified.calculate_order(&order_data.order)?;

    let payload = serde_json::to_string(&order_result)?;
    let tags = vec![Tag::custom(
        TagKind::custom("e_ref"),
        [ref_event.id.to_hex()],
    )];

    let job_result_event =
        nostr_event_job_result(&event_job_request, payload, 0, None, Some(tags))?;

    let job_result_event_id = nostr_send_event(client, job_result_event).await?;

    info!("job request order result sent: {:?}", job_result_event_id);

    Ok(())
}
