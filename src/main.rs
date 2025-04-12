use anyhow::Result;
use clap::Parser;
use nostr::{Filter, Keys, Kind, Timestamp, event::Event, nips::nip01::Metadata};
use nostr_sdk::{Client, RelayPoolNotification};
use rhi::{KIND_JOB_REQUEST, keys::KeyProfile};
use tokio::signal::unix::{SignalKind, signal};
use tracing::{error, info};

struct ConfigMetadata {
    name: String,
    nip_05: Option<String>,
}

fn init_tracing() {
    tracing_subscriber::fmt::init();
}

async fn subscribe(keys: Keys, relays: Vec<String>) -> Result<()> {
    let client = Client::new(keys);
    for relay in relays.iter() {
        client.add_relay(relay).await?;
    }
    client.connect().await;

    let filter = Filter::new()
        .kind(Kind::Custom(KIND_JOB_REQUEST))
        .since(Timestamp::now());

    client.subscribe(filter, None).await?;

    info!("Subscription started for kind {}", {
        KIND_JOB_REQUEST.to_string()
    });

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
    #[arg(long, help = "Adds the keys profiles file path.", required = true)]
    pub keys: String,

    #[arg(long, help = "Adds nostr relays to the subscription.", required = true)]
    pub relays: Vec<String>,

    #[arg(
        long,
        help = "(Optional) Sets flag to generate keys if none are found.",
        required = false
    )]
    pub generate_keys: bool,

    #[arg(
        long,
        help = "(Optional) Adds the application handler identifier tag (NIP-89)",
        required = false
    )]
    pub identifier: Option<String>,

    #[arg(
        long,
        help = "(Optional) Adds the domain name (NIP-05) to metadata",
        required = false
    )]
    pub nip05_domain: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let args = Args::parse();
    let relays = args.relays.clone();

    info!("Starting");

    let mut key_profile = KeyProfile::init(args.keys, args.generate_keys, args.identifier)?;

    let keys = key_profile.keys()?;

    let config = ConfigMetadata {
        name: "rhi".to_string(),
        nip_05: args.nip05_domain,
    };

    let metadata = Metadata {
        name: Some(config.name.clone()),
        display_name: None,
        picture: None,
        nip05: config
            .nip_05
            .as_ref()
            .map(|domain| format!("{}@{}", config.name, domain)),
        ..Default::default()
    };

    let mut events: Vec<Event> = vec![];

    if let Some(event) = key_profile.build_metadata(&metadata).await? {
        events.push(event);
    }

    if let Some(event) = key_profile.build_application_handler().await? {
        events.push(event);
    }

    if !events.is_empty() {
        let client = Client::new(keys.clone());
        for relay in relays.iter() {
            client.add_relay(relay).await?;
        }
        client.connect().await;
        for event in events {
            client.send_event(&event).await?;
            info!("Sent kind {} event for key profile", {
                event.clone().kind
            })
        }
        client.disconnect().await;
    }

    let keys_sub = keys.clone();
    let relays_sub = relays.clone();

    tokio::spawn(async move {
        loop {
            if let Err(e) = subscribe(keys_sub.clone(), relays_sub.clone()).await {
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
