use anyhow::Result;
use clap::Parser;
use nostr::{Filter, Keys, Kind, Timestamp};
use nostr_sdk::{Client, RelayPoolNotification};
use tokio::signal::unix::{SignalKind, signal};
use tracing::{error, info};

fn init_tracing() {
    tracing_subscriber::fmt::init();
}

async fn subscribe(relays: Vec<String>) -> Result<()> {
    info!("Subscription started for kind 5300");

    let keys = Keys::generate();
    let client = Client::new(keys);
    for relay in relays.iter() {
        client.add_relay(relay).await?;
    }
    client.connect().await;

    let filter = Filter::new()
        .kind(Kind::Custom(5300))
        .since(Timestamp::now());

    client.subscribe(filter, None).await?;

    let mut notifications = client.notifications();

    while let Ok(notification) = notifications.recv().await {
        match notification {
            RelayPoolNotification::Event { event, .. } => {
                info!("Event received {:?}", { event.clone() });
            }
            RelayPoolNotification::Message { .. } => {}
            RelayPoolNotification::Shutdown => {}
        }
    }

    client.disconnect().await;

    Ok(())
}

#[derive(Parser)]
#[command(
    about = env!("CARGO_PKG_DESCRIPTION"),
    author = env!("CARGO_PKG_AUTHORS"),
    version = env!("CARGO_PKG_VERSION")
)]
pub struct Args {
    #[arg(long, help = "Adds nostr relays to the subscription", required = true)]
    pub relays: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let args = Args::parse();
    let relays = args.relays.clone();

    info!("Starting");

    tokio::spawn(async move {
        loop {
            if let Err(e) = subscribe(relays.clone()).await {
                error!("Error on subscription: {e}");
            }
        }
    });

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    tokio::select! {
        _ = sigterm.recv() => {
            info!("Received SIGTERM. Shutting down...");
        },
        _ = sigint.recv() => {
            info!("Received SIGINT. Shutting down...");
        }
    }

    Ok(())
}
