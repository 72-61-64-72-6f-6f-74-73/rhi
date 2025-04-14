use anyhow::Result;
use nostr::{event::Event, key::Keys};
use nostr_sdk::Client;
use tracing::info;

use crate::events::job_request::{JobRequest, JobRequestError, JobRequestInput};

pub async fn handle_job_request_quote(
    _event: Event,
    _keys: Keys,
    _client: Client,
    job_req: JobRequest,
    _job_req_input: JobRequestInput,
) -> Result<(), JobRequestError> {
    info!("handle_job_request_quote job_req: {:?}", job_req);

    Ok(())
}
