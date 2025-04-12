use nostr::{
    event::{Event, EventBuilder, Tag},
    nips::nip90::{DataVendingMachineStatus, JobFeedbackData},
};

use crate::events::job_request::JobRequestError;

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
