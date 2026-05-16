use std::fs::{File, OpenOptions};

use fs2::FileExt;

use crate::errors::{CsuError, Result};
use crate::fs_util;
use crate::paths;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

pub struct LockGuard {
    file: File,
}

impl LockGuard {
    pub fn acquire() -> Result<Self> {
        paths::ensure_backup_root()?;
        let path = paths::lock_file()?;
        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true).truncate(false);
        #[cfg(unix)]
        options.mode(fs_util::PRIVATE_FILE_MODE);
        let file = options.open(&path)?;
        fs_util::set_permissions(&path, fs_util::PRIVATE_FILE_MODE)?;
        file.try_lock_exclusive().map_err(|_| CsuError::LockHeld)?;
        Ok(Self { file })
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

pub struct Rollback<'a> {
    store: &'a dyn crate::storage::CredentialStore,
    original_creds: Option<crate::models::Credentials>,
    original_config: Option<serde_json::Value>,
    original_state: Option<crate::account_state::Snapshot>,
    committed: bool,
}

impl<'a> Rollback<'a> {
    pub fn new(
        store: &'a dyn crate::storage::CredentialStore,
        original_creds: crate::models::Credentials,
        original_config: serde_json::Value,
        original_state: crate::account_state::Snapshot,
    ) -> Self {
        Self {
            store,
            original_creds: Some(original_creds),
            original_config: Some(original_config),
            original_state: Some(original_state),
            committed: false,
        }
    }

    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for Rollback<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Some(creds) = self.original_creds.take() {
            let _ = self.store.write_active(&creds);
        }
        if let Some(cfg) = self.original_config.take() {
            let _ = crate::config::save_global(&cfg);
        }
        if let Some(state) = self.original_state.take() {
            let _ = state.apply();
        }
    }
}
