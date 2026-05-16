use std::fs;
use std::io;
use std::path::Path;

use dialoguer::Confirm;

use crate::errors::Result;
use crate::lock::LockGuard;
use crate::paths;
use crate::sequence;
use crate::storage;

pub fn run(yes: bool) -> Result<String> {
    if !yes {
        let prompt =
            "purge removes ALL csu-managed account backups. current active credentials are kept. continue?";
        let confirmed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;
        if !confirmed {
            return Ok("aborted".into());
        }
    }

    let root = paths::backup_root()?;
    paths::validate_backup_root_for_purge(&root)?;

    let _guard = LockGuard::acquire()?;

    if let Ok(seq) = sequence::load() {
        let store = storage::default_store();
        for (slot_str, account) in &seq.accounts {
            if let Ok(slot) = slot_str.parse::<u32>() {
                let _ = store.delete_backup(slot, &account.email);
            }
        }
    }

    paths::validate_backup_root_for_purge(&root)?;
    if root.exists() && !paths::backup_root_uses_env() {
        fs::remove_dir_all(&root)?;
    } else if root.exists() {
        purge_managed_files(&root)?;
    }

    Ok("purged all csu state".into())
}

fn purge_managed_files(root: &Path) -> Result<()> {
    remove_file_if_exists(&paths::sequence_file()?)?;
    remove_file_if_exists(&paths::lock_file()?)?;
    purge_dir(&paths::configs_dir()?, ".claude-config-")?;
    purge_dir(&paths::creds_dir()?, ".creds-")?;
    purge_dir(&paths::state_dir()?, ".state-")?;
    remove_file_if_exists(&paths::managed_marker_file(root))?;
    let _ = fs::remove_dir(root);
    Ok(())
}

fn purge_dir(dir: &Path, prefix: &str) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    let marker = paths::managed_marker_file(dir);
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if path == marker || name.to_string_lossy().starts_with(prefix) {
            remove_file_if_exists(&path)?;
        }
    }

    let _ = fs::remove_dir(dir);
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
