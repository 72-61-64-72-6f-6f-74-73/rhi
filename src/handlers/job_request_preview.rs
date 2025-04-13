use anyhow::Result;
use nostr::{event::Event, key::Keys};
use nostr_sdk::Client;
use tracing::info;

use crate::events::job_request::{JobRequest, JobRequestError};

pub async fn handle_job_request_preview(
    event: Event,
    job_req: JobRequest,
    keys: Keys,
    client: Client,
) -> Result<(), JobRequestError> {
    info!("handle_job_request_preview job_req: {:?}", job_req);

    Ok(())
}
