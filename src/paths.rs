use std::env;
use std::path::{Path, PathBuf};

use directories::BaseDirs;

use crate::errors::{CsuError, Result};
use crate::fs_util;

const MANAGED_MARKER: &str = ".csu-managed";

fn home() -> Result<PathBuf> {
    BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .ok_or(CsuError::NoHomeDir)
}

pub fn claude_config_home() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CLAUDE_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }
    Ok(home()?.join(".claude"))
}

pub fn credentials_file() -> Result<PathBuf> {
    Ok(claude_config_home()?.join(".credentials.json"))
}

pub fn global_config_file() -> Result<PathBuf> {
    let legacy = claude_config_home()?.join(".config.json");
    if legacy.exists() {
        return Ok(legacy);
    }
    Ok(home()?.join(".claude.json"))
}

pub fn backup_root() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CSU_BACKUP_DIR") {
        return Ok(PathBuf::from(dir));
    }
    Ok(home()?.join(".csu-backup"))
}

pub fn sequence_file() -> Result<PathBuf> {
    Ok(backup_root()?.join("sequence.json"))
}

pub fn lock_file() -> Result<PathBuf> {
    Ok(backup_root()?.join(".lock"))
}

pub fn configs_dir() -> Result<PathBuf> {
    Ok(backup_root()?.join("configs"))
}

pub fn creds_dir() -> Result<PathBuf> {
    Ok(backup_root()?.join("credentials"))
}

pub fn state_dir() -> Result<PathBuf> {
    Ok(backup_root()?.join("state"))
}

pub fn ensure_backup_root() -> Result<PathBuf> {
    let root = backup_root()?;
    create_managed_dir(&root)?;
    create_managed_dir(&configs_dir()?)?;
    create_managed_dir(&creds_dir()?)?;
    create_managed_dir(&state_dir()?)?;
    Ok(root)
}

pub fn config_backup_file(slot: u32, email: &str) -> Result<PathBuf> {
    Ok(configs_dir()?.join(format!(
        ".claude-config-{slot}-{}.json",
        safe_component(email)
    )))
}

pub fn creds_backup_file(slot: u32, email: &str) -> Result<PathBuf> {
    Ok(creds_dir()?.join(format!(".creds-{slot}-{}.json", safe_component(email))))
}

pub fn state_backup_file(slot: u32, email: &str, name: &str) -> Result<PathBuf> {
    Ok(state_dir()?.join(format!(".state-{slot}-{}-{name}", safe_component(email))))
}

pub fn keyring_backup_account(slot: u32, email: &str) -> String {
    format!("account-{slot}-{}", safe_component(email))
}

pub fn backup_root_uses_env() -> bool {
    env::var_os("CSU_BACKUP_DIR").is_some()
}

pub fn validate_backup_root_for_purge(root: &Path) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    let canonical = root.canonicalize()?;
    if canonical.parent().is_none() || canonical == home()? {
        return Err(CsuError::UnsafeBackupRoot(root.to_path_buf()));
    }
    let default_root = !backup_root_uses_env()
        && canonical.file_name().and_then(|name| name.to_str()) == Some(".csu-backup");
    if !default_root && !canonical.join(MANAGED_MARKER).is_file() {
        return Err(CsuError::UnsafeBackupRoot(root.to_path_buf()));
    }
    Ok(())
}

pub fn managed_marker_file(dir: &Path) -> PathBuf {
    dir.join(MANAGED_MARKER)
}

fn create_managed_dir(path: &Path) -> Result<()> {
    fs_util::create_private_dir_all(path)?;
    fs_util::write_private(&managed_marker_file(path), b"csu\n")?;
    Ok(())
}

fn safe_component(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'@' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    if out.is_empty() {
        "unknown".into()
    } else {
        out
    }
}
