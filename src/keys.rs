use anyhow::Result;
use nostr::{
    Event, Keys,
    event::{EventBuilder, Kind, Tag, TagKind},
    nips::nip01::Metadata,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufReader, Write},
    path::{Path, PathBuf},
    str::FromStr,
};
use tempfile::NamedTempFile;
use thiserror::Error;
use tracing::{error, warn};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::{KIND_APPLICATION_HANDLER, KIND_JOB_REQUEST};

#[derive(Error, Debug)]
pub enum KeyProfileError {
    #[error("Keys file does not exist at {0}")]
    NotFound(PathBuf),

    #[error("Failed to open keys file at {0}: {1}")]
    FileOpen(PathBuf, #[source] std::io::Error),

    #[error("Keys file already exists at {0}")]
    AlreadyExists(PathBuf),

    #[error("Failed to parse keys file at {0}: {1}")]
    FileParse(PathBuf, #[source] serde_json::Error),

    #[error("Failed to serialize keys: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error during key write: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to persist keys to disk: {0}")]
    Persist(#[from] tempfile::PersistError),

    #[error("Failed to build or sign nostr event: {0}")]
    NostrBuilder(#[from] nostr::event::builder::Error),

    #[error("Invalid secret key for identifier: {0}")]
    InvalidSecretKey(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyProfile {
    key: String,
    identifier: String,
    pub metadata: Option<Event>,
    pub application_handler: Option<Event>,

    #[serde(skip)]
    path: Option<PathBuf>,
}

impl KeyProfile {
    pub fn init<P: AsRef<str>>(
        path_str: P,
        generate: bool,
        identifier_tag: Option<String>,
    ) -> Result<Self, KeyProfileError> {
        let path = PathBuf::from(path_str.as_ref());

        if path.exists() {
            let file = File::open(&path).map_err(|e| KeyProfileError::FileOpen(path.clone(), e))?;
            let reader = BufReader::new(file);
            let mut profile: KeyProfile = serde_json::from_reader(reader)
                .map_err(|e| KeyProfileError::FileParse(path.clone(), e))?;
            profile.path = Some(path.clone());

            if !profile.identifier.trim().is_empty() {
                if let Some(new_id) = identifier_tag {
                    warn!(
                        "Provided identifier '{}' is being ignored because the keys file already contains identifier '{}'.",
                        new_id, profile.identifier
                    );
                }
            } else {
                profile.identifier = identifier_tag.unwrap_or_else(|| {
                    warn!(
                        "Missing NIP-89 application handler identifier in key file, generating UUID."
                    );
                    Uuid::new_v4().to_string()
                });
                profile.persist()?;
            }

            Ok(profile)
        } else if generate {
            let keys = Keys::generate();
            let secret = keys.secret_key();
            let identifier = match identifier_tag {
                Some(identifier) => identifier,
                None => {
                    warn!(
                        "Missing NIP-89 application handler identifier in key file, generating UUID."
                    );
                    Uuid::new_v4().to_string()
                }
            };

            let profile = KeyProfile {
                key: secret.to_secret_hex(),
                identifier,
                metadata: None,
                application_handler: None,
                path: Some(path.clone()),
            };

            profile.atomic_write(&path)?;
            Ok(profile)
        } else {
            Err(KeyProfileError::NotFound(path))
        }
    }

    pub fn keys(&self) -> Result<Keys, KeyProfileError> {
        Keys::from_str(&self.key)
            .map_err(|_| KeyProfileError::InvalidSecretKey(self.identifier.clone()))
    }

    fn atomic_write<P: AsRef<Path>>(&self, path: P) -> Result<(), KeyProfileError> {
        let json = serde_json::to_string(self)?;

        let dir = path.as_ref().parent().unwrap_or_else(|| Path::new("."));
        let mut temp_file = NamedTempFile::new_in(dir)?;

        temp_file.write_all(json.as_bytes())?;
        temp_file.as_file_mut().sync_all()?;

        #[cfg(unix)]
        {
            fs::set_permissions(temp_file.path(), fs::Permissions::from_mode(0o600))?;
        }

        temp_file.persist(path)?;
        Ok(())
    }

    fn persist(&self) -> Result<(), KeyProfileError> {
        match &self.path {
            Some(p) => self.atomic_write(p),
            None => Err(KeyProfileError::NotFound(PathBuf::from("[unknown path]"))),
        }
    }

    pub async fn build_metadata(
        &mut self,
        metadata: &Metadata,
    ) -> Result<Option<Event>, KeyProfileError> {
        if self.metadata.is_none() {
            let keys = self.keys()?;
            let event = EventBuilder::metadata(metadata).sign(&keys).await?;
            self.metadata = Some(event.clone());
            self.persist()?;
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }

    pub async fn build_application_handler(&mut self) -> Result<Option<Event>, KeyProfileError> {
        if self.application_handler.is_none() {
            let keys = self.keys()?;
            let kind_0_content = self
                .metadata
                .as_ref()
                .expect(&format!(
                    "The kind 0 metadata must be initialized before kind {} descriptor",
                    KIND_APPLICATION_HANDLER.to_string()
                ))
                .content
                .clone();

            let tags: Vec<Tag> = vec![
                Tag::custom(TagKind::Custom("k".into()), [KIND_JOB_REQUEST.to_string()]),
                Tag::identifier(self.identifier.to_string()),
            ];

            let event = EventBuilder::new(Kind::Custom(KIND_APPLICATION_HANDLER), kind_0_content)
                .tags(tags)
                .sign(&keys)
                .await?;

            self.application_handler = Some(event.clone());
            self.persist()?;
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }
}
