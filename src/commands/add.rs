use chrono::Utc;

use crate::account_state;
use crate::config;
use crate::errors::{CsuError, Result};
use crate::lock::LockGuard;
use crate::models::Account;
use crate::paths;
use crate::sequence;
use crate::storage;

pub fn run(slot: Option<u32>) -> Result<String> {
    paths::ensure_backup_root()?;
    let _guard = LockGuard::acquire()?;

    let store = storage::default_store();
    let creds = store.read_active()?;
    let config = config::load_global()?;
    let oauth = config::view_oauth_account(&config).ok_or(CsuError::MissingOauthAccount)?;

    let email = oauth
        .email_address
        .clone()
        .ok_or_else(|| CsuError::InvalidCredentials("oauthAccount.emailAddress missing".into()))?;

    let mut seq = sequence::load()?;

    if let Some(existing) = seq.contains_email(&email) {
        return Err(CsuError::AccountAlreadyExists(format!(
            "{email} is already stored at slot {existing}",
        )));
    }

    let slot_num = match slot {
        Some(n) => {
            if n == 0 {
                return Err(CsuError::InvalidSlot);
            }
            if seq.get_slot(n).is_some() {
                return Err(CsuError::SlotTaken(n));
            }
            n
        }
        None => seq.next_slot(),
    };

    store.write_backup(slot_num, &email, &creds)?;
    config::save_backup(slot_num, &email, &config)?;
    account_state::Snapshot::capture()?.save_as_backup(slot_num, &email)?;

    let account = Account {
        email: email.clone(),
        uuid: oauth.account_uuid.clone(),
        organization_uuid: oauth.organization_uuid.clone(),
        organization_name: oauth.organization_name.clone(),
        added: Utc::now(),
    };

    seq.accounts.insert(slot_num.to_string(), account);
    if !seq.sequence.contains(&slot_num) {
        seq.sequence.push(slot_num);
        seq.sequence.sort_unstable();
    }
    seq.active_account_number = Some(slot_num);
    sequence::save(&seq)?;

    Ok(format!("added {email} at slot {slot_num}"))
}
