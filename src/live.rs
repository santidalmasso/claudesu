use crate::config;
use crate::errors::{CsuError, Result};
use crate::lock::LockGuard;
use crate::models::SequenceFile;
use crate::sequence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveState {
    Slot(u32),
    Unstored { email: String },
    LoggedOut,
}

pub fn detect(seq: &SequenceFile) -> Result<ActiveState> {
    let config = match config::load_global() {
        Ok(config) => config,
        Err(CsuError::ConfigMissing(_)) => return Ok(ActiveState::LoggedOut),
        Err(error) => return Err(error),
    };

    let Some(oauth) = config::view_oauth_account(&config) else {
        return Ok(ActiveState::LoggedOut);
    };

    if let Some(uuid) = oauth.account_uuid.as_deref() {
        if let Some(slot) = seq.find_by_uuid(uuid) {
            return Ok(ActiveState::Slot(slot));
        }
    }

    if let Some(email) = oauth.email_address {
        if let Some(slot) = seq.contains_email(&email) {
            return Ok(ActiveState::Slot(slot));
        }
        return Ok(ActiveState::Unstored { email });
    }

    Ok(ActiveState::LoggedOut)
}

pub fn detect_and_reconcile() -> Result<(SequenceFile, ActiveState)> {
    let seq = sequence::load()?;
    let state = detect(&seq)?;

    if let ActiveState::Slot(slot) = state {
        if seq.active_account_number != Some(slot) {
            if let Ok(_guard) = LockGuard::acquire() {
                let mut fresh = sequence::load()?;
                fresh.active_account_number = Some(slot);
                sequence::save(&fresh)?;
                return Ok((fresh, state));
            }
        }
    }

    Ok((seq, state))
}
