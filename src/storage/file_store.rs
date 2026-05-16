use std::fs;

use crate::errors::{CsuError, Result};
use crate::fs_util;
use crate::models::Credentials;
use crate::paths;

use super::CredentialStore;

pub struct FileStore;

impl CredentialStore for FileStore {
    fn read_active(&self) -> Result<Credentials> {
        let path = paths::credentials_file()?;
        if !path.exists() {
            return Err(CsuError::CredentialsMissing(path));
        }
        let bytes = fs::read(&path)?;
        serde_json::from_slice(&bytes)
            .map(Credentials)
            .map_err(|error| CsuError::JsonAt {
                origin: format!("active credentials at {}", path.display()),
                error,
            })
    }

    fn write_active(&self, creds: &Credentials) -> Result<()> {
        let path = paths::credentials_file()?;
        if let Some(parent) = path.parent() {
            fs_util::create_private_dir_all(parent)?;
        }
        let bytes = creds.to_compact_bytes()?;
        fs_util::write_private(&path, &bytes)?;
        Ok(())
    }

    fn write_backup(&self, slot: u32, email: &str, creds: &Credentials) -> Result<()> {
        paths::ensure_backup_root()?;
        let path = paths::creds_backup_file(slot, email)?;
        let bytes = creds.to_compact_bytes()?;
        fs_util::write_private(&path, &bytes)?;
        Ok(())
    }

    fn read_backup(&self, slot: u32, email: &str) -> Result<Credentials> {
        let path = paths::creds_backup_file(slot, email)?;
        if !path.exists() {
            return Err(CsuError::CredentialsMissing(path));
        }
        let bytes = fs::read(&path)?;
        serde_json::from_slice(&bytes)
            .map(Credentials)
            .map_err(|error| CsuError::JsonAt {
                origin: format!("credentials backup for slot {slot} at {}", path.display()),
                error,
            })
    }

    fn delete_backup(&self, slot: u32, email: &str) -> Result<()> {
        let path = paths::creds_backup_file(slot, email)?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
