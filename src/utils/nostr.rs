use std::borrow::Cow;

use anyhow::Result;
use nostr::{
    event::{Event, EventBuilder, EventId, Kind, Tag, TagKind, TagStandard},
    filter::Filter,
    key::{Keys, PublicKey},
    nips::{
        nip04,
        nip90::{DataVendingMachineStatus, JobFeedbackData},
    },
    types::{RelayUrl, Timestamp},
};
use nostr_sdk::Client;
use nostr_sdk::RelayPoolNotification;
use thiserror::Error;

use crate::events::job_request::JobRequestError;

pub fn nostr_kind(kind: u16) -> Kind {
    Kind::Custom(kind)
}

pub fn nostr_filter_kind(kind: u16) -> Filter {
    Filter::new().kind(Kind::Custom(kind))
}

pub fn nostr_filter_new_events(filter: Filter) -> Filter {
    filter.since(Timestamp::now())
}

pub fn nostr_tag_first_value(tag: &Tag, key: &str) -> Option<String> {
    if tag.kind() == TagKind::custom(key) {
        tag.content().map(|v| v.to_string())
    } else {
        None
    }
}

pub fn nostr_tag_at_value(tag: &Tag, index: usize) -> Option<String> {
    tag.as_slice().get(index).cloned()
}

pub fn nostr_tag_slice(tag: &Tag, start: usize) -> Option<Vec<String>> {
    tag.as_slice().get(start..).map(|s| s.to_vec())
}

pub fn nostr_tag_relays_parse(tag: &Tag) -> Option<&Vec<RelayUrl>> {
    match tag.as_standardized()? {
        TagStandard::Relays(urls) => Some(urls),
        _ => None,
    }
}

pub fn nostr_tags_match<'a>(tag: &'a Tag) -> Option<(&'a str, &'a [String])> {
    if let TagKind::Custom(Cow::Borrowed(key)) = tag.kind() {
        Some((key, &tag.as_slice()[1..]))
    } else {
        None
    }
}

pub fn nostr_tag_match_l(tag: &Tag) -> Option<(&str, f64)> {
    let values = tag.as_slice();

    if values.len() >= 3 && values[0].eq_ignore_ascii_case("l") {
        if let Ok(value) = values[1].parse::<f64>() {
            return Some((values[2].as_str(), value));
        }
    }

    None
}

pub fn nostr_tag_match_location(tag: &Tag) -> Option<(&str, &str, &str)> {
    let values = tag.as_slice();

    if values.len() >= 4 && values[0] == "location" {
        Some((values[1].as_str(), values[2].as_str(), values[3].as_str()))
    } else {
        None
    }
}

pub fn nostr_tag_match_geohash(tag: &Tag) -> Option<String> {
    match tag.as_standardized()? {
        TagStandard::Geohash(geohash) => Some(geohash.clone()),
        _ => None,
    }
}

pub fn nostr_tag_match_title(tag: &Tag) -> Option<String> {
    match tag.as_standardized()? {
        TagStandard::Title(title) => Some(title.clone()),
        _ => None,
    }
}

pub fn nostr_tag_match_summary(tag: &Tag) -> Option<String> {
    match tag.as_standardized()? {
        TagStandard::Summary(summary) => Some(summary.clone()),
        _ => None,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NostrEventError {
    #[error("Failed to build job result event: {0}")]
    BuildError(#[from] nostr::event::builder::Error),
}

pub fn nostr_event_job_result(
    job_request: &Event,
    payload: impl Into<String>,
    millisats: u64,
    bolt11: Option<String>,
    tags: Option<Vec<Tag>>,
) -> Result<EventBuilder, NostrEventError> {
    let builder = EventBuilder::job_result(job_request.clone(), payload, millisats, bolt11)?
        .tags(tags.unwrap_or_default());
    Ok(builder)
}

pub fn nostr_event_job_feedback(
    job_request: &Event,
    error: JobRequestError,
    status: &str,
    tags: Option<Vec<Tag>>,
) -> Result<EventBuilder, NostrEventError> {
    let status = status
        .parse::<DataVendingMachineStatus>()
        .unwrap_or(DataVendingMachineStatus::Error);
    let feedback_data =
        JobFeedbackData::new(&job_request.clone(), status).extra_info(error.to_string());
    let builder = EventBuilder::job_feedback(feedback_data).tags(tags.unwrap_or_default());
    Ok(builder)
}

pub async fn nostr_fetch_event_by_id(client: Client, id: &str) -> Result<Option<Event>> {
    let event_id = EventId::from_hex(id)?;
    let filter = Filter::new().id(event_id);

    client.connect().await;
    client.subscribe(filter, None).await?;

    let mut notifications = client.notifications();
    while let Ok(n) = notifications.recv().await {
        if let RelayPoolNotification::Event { event, .. } = n {
            if event.id == event_id {
                return Ok(Some(*event));
            }
        }
    }

    Ok(None)
}

#[derive(Debug, Error)]
pub enum NostrTagsResolveError {
    #[error("Missing public key tag in encrypted event: {0:?}")]
    MissingPTag(nostr::Event),

    #[error("Encrypted event recipient mismatch")]
    NotRecipient,

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Failed to parse decrypted tag JSON: {0}")]
    ParseError(#[from] serde_json::Error),
}

pub fn nostr_tags_resolve(event: &Event, keys: &Keys) -> Result<Vec<Tag>, NostrTagsResolveError> {
    if event.tags.iter().any(|t| t.kind() == TagKind::Encrypted) {
        let recipient = event
            .tags
            .iter()
            .find_map(|tag| {
                if tag.kind() == TagKind::p() {
                    tag.content()?.parse::<PublicKey>().ok()
                } else {
                    None
                }
            })
            .ok_or_else(|| NostrTagsResolveError::MissingPTag(event.clone()))?;

        if recipient != keys.public_key() {
            return Err(NostrTagsResolveError::NotRecipient.into());
        }

        let cleartext = nip04::decrypt(keys.secret_key(), &event.pubkey, &event.content)
            .map_err(|e| NostrTagsResolveError::DecryptionError(e.to_string()))?;

        let decrypted_tags: nostr::event::tag::list::Tags = serde_json::from_str(&cleartext)?;

        Ok(decrypted_tags.to_vec())
    } else {
        Ok(event.clone().tags.to_vec())
    }
}
