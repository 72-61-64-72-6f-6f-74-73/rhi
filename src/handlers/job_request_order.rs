use anyhow::Result;
use nostr::{event::Event, key::Keys};
use nostr_sdk::Client;
use serde::Deserialize;
use thiserror::Error;
use tracing::info;

use crate::{
    events::job_request::{JobRequest, JobRequestError, JobRequestInput},
    models::event_classified::EventClassified,
    utils::nostr::nostr_fetch_event_by_id,
};

#[derive(Debug, Error)]
pub enum JobRequestOrderError {
    #[error("Failure to parse the reference event {0}")]
    ReferenceEventParse(String),

    #[error("Failure to fetch the reference event {0}")]
    ReferenceEventFetch(String),

    #[error("Reference event not found {0}")]
    ReferenceEventMissing(String),
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
    _event: Event,
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

    info!("handle_job_request_order ref_event: {:?}", ref_event);

    let ref_classified = EventClassified::from_event(ref_event)
        .map_err(|_| JobRequestOrderError::ReferenceEventParse(order_data.event.id.clone()))?;

    info!(
        "handle_job_request_order ref_classified: {:?}",
        ref_classified
    );

    Ok(())
}
