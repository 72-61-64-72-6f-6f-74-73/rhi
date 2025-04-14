use anyhow::Result;
use nostr::{event::Event, key::Keys};
use nostr_sdk::Client;
use serde::Deserialize;
use thiserror::Error;
use tracing::info;

use crate::{
    events::job_request::{JobRequest, JobRequestError, JobRequestInput},
    models::event_classified::EventClassified,
    utils::nostr::{NostrEventError, nostr_event_job_result, nostr_fetch_event_by_id},
};

#[derive(Debug, Error)]
pub enum JobRequestOrderError {
    #[error("Failure to parse the reference event {0}")]
    ReferenceEventParse(String),

    #[error("Failure to fetch the reference event {0}")]
    ReferenceEventFetch(String),

    #[error("Reference event not found {0}")]
    ReferenceEventMissing(String),

    #[error("Reference event does not satisfy requested {0}")]
    ReferenceEventMissingRequested(String),

    #[error("Failure building the job response")]
    ResponseEventBuildFailure(#[from] NostrEventError),

    #[error("Failure sending the job response")]
    ResponseEventSendFailure(#[from] nostr_sdk::client::Error),
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
    let order_data: JobRequestOrderData = serde_json::from_str(&job_req_input.data)?;

    info!("handle_job_request_order order_data: {:?}", order_data);

    let fetched_ref_event: Option<Event> =
        nostr_fetch_event_by_id(client.clone(), &order_data.event.id.clone())
            .await
            .map_err(|_| JobRequestOrderError::ReferenceEventFetch(order_data.event.id.clone()))?;
    let ref_event: &Event =
        fetched_ref_event
            .as_ref()
            .ok_or(JobRequestOrderError::ReferenceEventMissing(
                order_data.event.id.clone(),
            ))?;

    let ref_classified = EventClassified::from_event(ref_event)
        .map_err(|_| JobRequestOrderError::ReferenceEventParse(order_data.event.id.clone()))?;

    info!(
        "handle_job_request_order ref_classified: {:?}",
        ref_classified
    );

    if ref_classified.prices.is_empty() {
        return Err(JobRequestError::JobRequestOrder(
            JobRequestOrderError::ReferenceEventMissingRequested("price".to_string()),
        ));
    }

    if ref_classified.quantities.is_empty() {
        return Err(JobRequestError::JobRequestOrder(
            JobRequestOrderError::ReferenceEventMissingRequested("quantity".to_string()),
        ));
    }

    let payload = "your order was received!";
    let event_result = nostr_event_job_result(&event_job_request.clone(), payload, 0, None, None)
        .map_err(JobRequestOrderError::from)?;
    let event_id = client
        .send_event_builder(event_result)
        .await
        .map_err(JobRequestOrderError::from)?;

    info!("handle_job_request_order sent result {:?}", {
        event_id.clone()
    });
    Ok(())
}
