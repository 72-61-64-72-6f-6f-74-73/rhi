use tracing::info;
use anyhow::Result;

fn init_tracing() {
    tracing_subscriber::fmt::init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    info!("Started");

    Ok(())
}