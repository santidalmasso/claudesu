use crate::errors::{CsuError, Result};
use crate::models::Credentials;
use crate::paths;

use super::CredentialStore;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use super::FileStore;

#[cfg(any(target_os = "linux", target_os = "windows"))]
use keyring::Entry;

pub struct SystemStore;

const SERVICE: &str = "Claude Code-credentials";

#[cfg(target_os = "macos")]
fn current_user_account() -> String {
    std::env::var("USER").unwrap_or_else(|_| "claude".to_string())
}

impl CredentialStore for SystemStore {
    #[cfg(target_os = "macos")]
    fn read_active(&self) -> Result<Credentials> {
        let account = current_user_account();
        let bytes = macos_get_secret(&account)?;
        serde_json::from_slice(&bytes)
            .map(Credentials)
            .map_err(|error| CsuError::JsonAt {
                origin: format!("credential entry (service={SERVICE}, account={account})"),
                error,
            })
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn read_active(&self) -> Result<Credentials> {
        FileStore.read_active()
    }

    #[cfg(target_os = "macos")]
    fn write_active(&self, creds: &Credentials) -> Result<()> {
        let account = current_user_account();
        if !macos_item_exists(&account)? {
            return Err(CsuError::CredentialStore(format!(
                "no Claude Code credential in the keychain for account '{account}' — \
                 log in with Claude Code before switching"
            )));
        }
        macos_set_secret(&account, &creds.to_compact_bytes()?)
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn write_active(&self, creds: &Credentials) -> Result<()> {
        FileStore.write_active(creds)
    }

    fn write_backup(&self, slot: u32, email: &str, creds: &Credentials) -> Result<()> {
        let account = paths::keyring_backup_account(slot, email);
        set_backup_secret(&account, &creds.to_compact_bytes()?)
    }

    fn read_backup(&self, slot: u32, email: &str) -> Result<Credentials> {
        let account = paths::keyring_backup_account(slot, email);
        let bytes = get_backup_secret(&account)?;
        serde_json::from_slice(&bytes)
            .map(Credentials)
            .map_err(|error| CsuError::JsonAt {
                origin: format!("credential backup entry (service={SERVICE}, account={account})"),
                error,
            })
    }

    fn delete_backup(&self, slot: u32, email: &str) -> Result<()> {
        delete_backup_secret(&paths::keyring_backup_account(slot, email))
    }
}

#[cfg(target_os = "macos")]
fn set_backup_secret(account: &str, value: &[u8]) -> Result<()> {
    macos_set_secret(account, value)
}

#[cfg(target_os = "macos")]
fn get_backup_secret(account: &str) -> Result<Vec<u8>> {
    macos_get_secret(account)
}

#[cfg(target_os = "macos")]
fn delete_backup_secret(account: &str) -> Result<()> {
    macos_delete_secret(account)
}

// Keychain access shells to /usr/bin/security on purpose: the item's ACL trusts
// the process that touched it, and going through /usr/bin/security shares one
// stable identity with Claude Code. Direct SecItem calls would rebind the ACL to
// csu's per-build identity and lock Claude Code out of its own credentials.
#[cfg(target_os = "macos")]
const SECURITY_BIN: &str = "/usr/bin/security";

#[cfg(target_os = "macos")]
const ERR_ITEM_NOT_FOUND: i32 = 44;

#[cfg(target_os = "macos")]
fn macos_get_secret(account: &str) -> Result<Vec<u8>> {
    use std::process::Command;
    let output = Command::new(SECURITY_BIN)
        .args(["find-generic-password", "-w", "-s", SERVICE, "-a", account])
        .output()
        .map_err(|error| CsuError::CredentialStore(error.to_string()))?;
    if output.status.success() {
        let mut bytes = output.stdout;
        while bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        return Ok(bytes);
    }
    if output.status.code() == Some(ERR_ITEM_NOT_FOUND) {
        return Err(CsuError::CredentialEntryMissing(account.to_string()));
    }
    Err(macos_security_error("find-generic-password", &output.stderr))
}

#[cfg(target_os = "macos")]
fn macos_set_secret(account: &str, value: &[u8]) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(SECURITY_BIN)
        .arg("-i")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CsuError::CredentialStore(error.to_string()))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| CsuError::CredentialStore("security stdin unavailable".to_string()))?;
    let command = format!(
        "add-generic-password -U -a \"{account}\" -s \"{SERVICE}\" -X {}\n",
        macos_hex(value)
    );
    stdin
        .write_all(command.as_bytes())
        .map_err(|error| CsuError::CredentialStore(error.to_string()))?;
    drop(stdin);
    let output = child
        .wait_with_output()
        .map_err(|error| CsuError::CredentialStore(error.to_string()))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(macos_security_error("add-generic-password", &output.stderr))
    }
}

#[cfg(target_os = "macos")]
fn macos_delete_secret(account: &str) -> Result<()> {
    use std::process::Command;
    let output = Command::new(SECURITY_BIN)
        .args(["delete-generic-password", "-s", SERVICE, "-a", account])
        .output()
        .map_err(|error| CsuError::CredentialStore(error.to_string()))?;
    if output.status.success() || output.status.code() == Some(ERR_ITEM_NOT_FOUND) {
        Ok(())
    } else {
        Err(macos_security_error("delete-generic-password", &output.stderr))
    }
}

#[cfg(target_os = "macos")]
fn macos_item_exists(account: &str) -> Result<bool> {
    use std::process::{Command, Stdio};
    let status = Command::new(SECURITY_BIN)
        .args(["find-generic-password", "-s", SERVICE, "-a", account])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| CsuError::CredentialStore(error.to_string()))?;
    Ok(status.success())
}

#[cfg(target_os = "macos")]
fn macos_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(target_os = "macos")]
fn macos_security_error(operation: &str, stderr: &[u8]) -> CsuError {
    let detail = String::from_utf8_lossy(stderr);
    CsuError::CredentialStore(format!("security {operation}: {}", detail.trim()))
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn set_backup_secret(account: &str, value: &[u8]) -> Result<()> {
    entry(account)?
        .set_secret(value)
        .map_err(|error| keyring_error(account, error))
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn get_backup_secret(account: &str) -> Result<Vec<u8>> {
    entry(account)?
        .get_secret()
        .map_err(|error| keyring_error(account, error))
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn delete_backup_secret(account: &str) -> Result<()> {
    match entry(account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(keyring_error(account, error)),
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn entry(account: &str) -> Result<Entry> {
    Entry::new(SERVICE, account).map_err(|error| keyring_error(account, error))
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn keyring_error(account: &str, error: keyring::Error) -> CsuError {
    match error {
        keyring::Error::NoEntry => CsuError::CredentialEntryMissing(account.to_string()),
        error => CsuError::CredentialStore(format!("{error:?}")),
    }
}
