use anyhow::Result;
use nostr::{
    event::{Event, EventBuilder, Kind, Tag, TagKind, TagStandard},
    filter::Filter,
    key::{Keys, PublicKey},
    nips::{
        nip04,
        nip90::{DataVendingMachineStatus, JobFeedbackData},
    },
    types::{RelayUrl, Timestamp},
};
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

pub fn nostr_event_job_request_feedback(
    event: &Event,
    error: JobRequestError,
    status: &str,
    tags: Option<Vec<Tag>>,
) -> anyhow::Result<EventBuilder> {
    let status = status
        .parse::<DataVendingMachineStatus>()
        .unwrap_or(DataVendingMachineStatus::Error);
    let feedback_data = JobFeedbackData::new(&event, status).extra_info(error.to_string());
    let builder = EventBuilder::job_feedback(feedback_data).tags(tags.unwrap_or_default());
    Ok(builder)
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

pub fn nostr_tags_resolve(event: &Event, keys: &Keys) -> Result<Vec<Tag>> {
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
