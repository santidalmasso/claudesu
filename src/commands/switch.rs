use crate::account_state;
use crate::config;
use crate::errors::{CsuError, Result};
use crate::live::{self, ActiveState};
use crate::lock::{LockGuard, Rollback};
use crate::models::SequenceFile;
use crate::sequence;
use crate::storage;

pub fn run_next() -> Result<String> {
    let _guard = LockGuard::acquire()?;
    let seq = sequence::load()?;
    let active = resolve_active(&seq)?;
    let target = seq.next_after(active).ok_or(CsuError::NoActiveAccount)?;
    apply_switch(seq, active, target)
}

pub fn run_to(who: &str) -> Result<String> {
    let _guard = LockGuard::acquire()?;
    let seq = sequence::load()?;
    let target = sequence::resolve(&seq, who)?;
    let active = resolve_active(&seq)?;
    apply_switch(seq, active, target)
}

fn resolve_active(seq: &SequenceFile) -> Result<u32> {
    match live::detect(seq)? {
        ActiveState::Slot(slot) => Ok(slot),
        ActiveState::Unstored { email } => Err(CsuError::UnstoredActiveAccount(email)),
        ActiveState::LoggedOut => Err(CsuError::NoActiveAccount),
    }
}

fn apply_switch(mut seq: SequenceFile, active: u32, target: u32) -> Result<String> {
    if target == active {
        let email = seq
            .get_slot(target)
            .map(|a| a.email.clone())
            .unwrap_or_default();
        if seq.active_account_number != Some(active) {
            seq.active_account_number = Some(active);
            sequence::save(&seq)?;
        }
        return Ok(format!("already on slot {target} ({email})"));
    }

    let active_account = seq
        .get_slot(active)
        .cloned()
        .ok_or_else(|| CsuError::AccountNotFound(active.to_string()))?;
    let target_account = seq
        .get_slot(target)
        .cloned()
        .ok_or_else(|| CsuError::AccountNotFound(target.to_string()))?;

    let store = storage::default_store();
    let store_ref: &dyn storage::CredentialStore = store.as_ref();

    let active_creds = store_ref.read_active()?;
    let active_config = config::load_global()?;
    let active_state = account_state::Snapshot::capture()?;

    store_ref.write_backup(active, &active_account.email, &active_creds)?;
    config::save_backup(active, &active_account.email, &active_config)?;
    active_state.save_as_backup(active, &active_account.email)?;

    let target_creds = store_ref.read_backup(target, &target_account.email)?;
    let target_config = config::load_backup(target, &target_account.email)?;

    let merged = config::merge_oauth_account(&active_config, &target_config)?;

    let rollback = Rollback::new(store_ref, active_creds, active_config, active_state);

    store_ref.write_active(&target_creds)?;
    config::save_global(&merged)?;
    account_state::Snapshot::from_backup(target, &target_account.email)?.apply()?;

    seq.active_account_number = Some(target);
    sequence::save(&seq)?;

    rollback.commit();

    Ok(format!(
        "switched to slot {target} ({}). Restart Claude Code to pick up the change.",
        target_account.email,
    ))
}
