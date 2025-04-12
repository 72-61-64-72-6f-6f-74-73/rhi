use anyhow::Result;
use config::{Config, ConfigError, File};
use nostr::Metadata;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, warn};

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("Configuration loading failed: {0}")]
    Load(#[from] ConfigError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub metadata: Metadata,
}

impl Settings {
    pub fn load(config_path: &Option<String>) -> Result<Self, SettingsError> {
        let default = Self::default();

        match Self::load_from_file(config_path) {
            Ok(settings) => Ok(settings),
            Err(err) if config_path.is_none() => {
                warn!("Could not read config file: {err}. Using default configuration.",);
                Ok(default)
            }
            Err(err) => Err(err),
        }
    }

    fn load_from_file(config_path: &Option<String>) -> Result<Self, SettingsError> {
        let path = config_path.as_deref().unwrap_or("config.toml");

        let config = Config::builder()
            .add_source(File::with_name(path).required(false))
            .build()?
            .try_deserialize::<Settings>()?;

        Ok(config)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            metadata: Metadata {
                name: Some("rhi".to_string()),
                ..Default::default()
            },
        }
    }
}
