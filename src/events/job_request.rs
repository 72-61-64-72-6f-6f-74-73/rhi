use anyhow::Result;
use nostr::event::{Event, EventId, Tag, TagKind};
use nostr::filter::{Alphabet, SingleLetterTag};
use nostr::{event::Kind, key::Keys};
use nostr_sdk::Client;
use nostr_sdk::RelayPoolNotification;
use tracing::{info, warn};

use crate::KIND_JOB_REQUEST;
use crate::handlers::job_request_order::{JobRequestOrderError, handle_job_request_order};
use crate::handlers::job_request_preview::handle_job_request_preview;
use crate::handlers::job_request_quote::handle_job_request_quote;
use crate::utils::nostr::{
    NostrTagsResolveError, nostr_event_job_feedback, nostr_filter_kind, nostr_filter_new_events,
    nostr_tag_at_value, nostr_tag_first_value, nostr_tag_relays_parse, nostr_tag_slice,
    nostr_tags_resolve,
};
use crate::utils::unit::MassUnitError;

#[derive(thiserror::Error, Debug)]
pub enum JobRequestError {
    #[error("Order: {0}")]
    JobRequestOrder(#[from] JobRequestOrderError),

    #[error("{0}")]
    MassUnit(#[from] MassUnitError),

    #[error("{0}")]
    NostrTagsResolve(#[from] NostrTagsResolveError),

    #[error("Invalid job request input type: {0}")]
    InvalidInputType(String),

    #[error("Invalid job request input marker: {0}")]
    InvalidInputMarker(String),

    #[error("Deserialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Failure to process request")]
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobRequestInputType {
    Url,
    Event,
    Job,
    Text,
}

impl TryFrom<&str> for JobRequestInputType {
    type Error = JobRequestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "url" => Ok(Self::Url),
            "event" => Ok(Self::Event),
            "job" => Ok(Self::Job),
            "text" => Ok(Self::Text),
            other => Err(JobRequestError::InvalidInputType(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobRequestInputMarker {
    Order,
    Quote,
    Preview,
}

impl TryFrom<&str> for JobRequestInputMarker {
    type Error = JobRequestError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "order" => Ok(Self::Order),
            "quote" => Ok(Self::Quote),
            "preview" => Ok(Self::Preview),
            other => Err(JobRequestError::InvalidInputMarker(other.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JobRequestInput {
    pub data: String,
    pub input_type: JobRequestInputType,
    pub relay: Option<String>,
    pub marker: Option<JobRequestInputMarker>,
}

#[derive(Debug, Clone)]
pub struct JobRequest {
    pub id: EventId,
    pub inputs: Vec<JobRequestInput>,
    pub output: Option<String>,
    pub bid_msat: Option<u64>,
    pub relays: Vec<String>,
    pub service_providers: Vec<String>,
    pub params: Vec<(String, String)>,
    pub hashtags: Vec<String>,
    pub tags: Vec<Tag>,
}

pub async fn subscriber(keys: Keys, relays: Vec<String>) -> Result<()> {
    info!("Starting subscriber for kind {}", KIND_JOB_REQUEST);
    let client = Client::new(keys.clone());

    for relay in &relays {
        client.add_relay(relay).await?;
    }

    let filter = nostr_filter_new_events(nostr_filter_kind(KIND_JOB_REQUEST));

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
                        let _ = handle_error(err, event, keys, client, None).await;
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
    _keys: Keys,
    client: Client,
    _job_req: Option<JobRequest>,
) -> Result<()> {
    warn!("job_request handle_error error {}", error);
    warn!("job_request handle_error event {:?}", { event.clone() });

    let builder = nostr_event_job_feedback(&event, error, "error", None)?;
    let event_id = client.send_event_builder(builder).await?;

    warn!("job_request handle_error sent feedback {:?}", {
        event_id.clone()
    });
    Ok(())
}

async fn handle_event(event: Event, keys: Keys, client: Client) -> Result<(), JobRequestError> {
    let job_req = parse_event(&event, &keys)?;
    for job_req_input in &job_req.inputs {
        let marker = job_req_input
            .marker
            .as_ref()
            .ok_or_else(|| JobRequestError::InvalidInputMarker(job_req.id.to_string()))?;

        match marker {
            JobRequestInputMarker::Order => {
                process_job_request(
                    handle_job_request_order,
                    event.clone(),
                    keys.clone(),
                    client.clone(),
                    job_req.clone(),
                    job_req_input.clone(),
                )
                .await;
            }
            JobRequestInputMarker::Quote => {
                process_job_request(
                    handle_job_request_quote,
                    event.clone(),
                    keys.clone(),
                    client.clone(),
                    job_req.clone(),
                    job_req_input.clone(),
                )
                .await;
            }
            JobRequestInputMarker::Preview => {
                process_job_request(
                    handle_job_request_preview,
                    event.clone(),
                    keys.clone(),
                    client.clone(),
                    job_req.clone(),
                    job_req_input.clone(),
                )
                .await;
            }
        }
    }

    Ok(())
}

fn parse_event(event: &Event, keys: &Keys) -> Result<JobRequest, JobRequestError> {
    let tags = nostr_tags_resolve(event, keys)?;
    let mut inputs = vec![];
    let mut output = None;
    let mut bid_msat = None;
    let mut relays = vec![];
    let mut providers = vec![];
    let mut params = vec![];
    let mut hashtags = vec![];

    for tag in &tags {
        match tag.kind() {
            TagKind::SingleLetter(l) if l == SingleLetterTag::lowercase(Alphabet::I) => {
                if let Some(vals) = nostr_tag_slice(tag, 1) {
                    match &vals[..] {
                        [data, input_type, relay, marker, ..] => {
                            let data = data.clone();
                            let input_type = JobRequestInputType::try_from(input_type.as_str())?;
                            let relay = relay.clone();
                            let marker = JobRequestInputMarker::try_from(marker.as_str())?;
                            inputs.push(JobRequestInput {
                                data,
                                input_type,
                                relay: Some(relay),
                                marker: Some(marker),
                            });
                        }
                        _ => continue,
                    }
                }
            }

            TagKind::SingleLetter(l) if l == SingleLetterTag::lowercase(Alphabet::T) => {
                if let Some(val) = nostr_tag_first_value(tag, "t") {
                    hashtags.push(val);
                }
            }

            TagKind::Custom(ref k) if k == "output" => {
                output = nostr_tag_first_value(tag, k);
            }

            TagKind::Custom(ref k) if k == "bid" => {
                bid_msat = nostr_tag_first_value(tag, k).and_then(|s| s.parse().ok());
            }

            TagKind::Custom(k) if k == "param" => {
                if let Some(vals) = nostr_tag_slice(tag, 1) {
                    if vals.len() >= 2 {
                        params.push((vals[0].clone(), vals[1].clone()));
                    }
                }
            }

            TagKind::Relays => {
                if let Some(urls) = nostr_tag_relays_parse(tag) {
                    relays = urls.into_iter().map(|u| u.to_string()).collect();
                }
            }

            TagKind::SingleLetter(l) if l == SingleLetterTag::lowercase(Alphabet::P) => {
                if let Some(pk) = nostr_tag_at_value(tag, 1) {
                    providers.push(pk);
                }
            }

            _ => {}
        }
    }

    Ok(JobRequest {
        id: event.id,
        inputs,
        output,
        bid_msat,
        relays,
        service_providers: providers,
        tags,
        params,
        hashtags,
    })
}

async fn process_job_request<F, Fut>(
    handler: F,
    event: Event,
    keys: Keys,
    client: Client,
    job_req: JobRequest,
    job_req_input: JobRequestInput,
) where
    F: FnOnce(Event, Keys, Client, JobRequest, JobRequestInput) -> Fut,
    Fut: std::future::Future<Output = Result<(), JobRequestError>>,
{
    let error_event = event.clone();
    let error_job_req = job_req.clone();
    let error_keys = keys.clone();
    let error_client = client.clone();

    if let Err(err) = handler(
        event,
        keys.clone(),
        client.clone(),
        job_req.clone(),
        job_req_input.clone(),
    )
    .await
    {
        let _ = handle_error(
            err,
            error_event,
            error_keys,
            error_client,
            Some(error_job_req),
        )
        .await;
    }
}
