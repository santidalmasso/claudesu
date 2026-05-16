use dialoguer::Confirm;

use crate::account_state;
use crate::config;
use crate::errors::{CsuError, Result};
use crate::lock::LockGuard;
use crate::sequence;
use crate::storage;

pub fn run(who: &str, yes: bool) -> Result<String> {
    let _guard = LockGuard::acquire()?;
    let mut seq = sequence::load()?;
    let slot = sequence::resolve(&seq, who)?;
    let account = seq
        .get_slot(slot)
        .cloned()
        .ok_or_else(|| CsuError::AccountNotFound(slot.to_string()))?;

    if !yes {
        let prompt = format!(
            "remove slot {slot} ({}) — this deletes its backup credentials. continue?",
            account.email,
        );
        let confirmed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;
        if !confirmed {
            return Ok("aborted".into());
        }
    }

    let store = storage::default_store();
    store.delete_backup(slot, &account.email)?;
    config::delete_backup(slot, &account.email)?;
    account_state::delete_backup(slot, &account.email)?;

    seq.accounts.remove(&slot.to_string());
    seq.sequence.retain(|s| *s != slot);
    if seq.active_account_number == Some(slot) {
        seq.active_account_number = None;
    }
    sequence::save(&seq)?;

    Ok(format!("removed slot {slot} ({})", account.email))
}
