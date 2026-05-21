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
    macos_keychain_set(SERVICE, account, value)
}

#[cfg(target_os = "macos")]
fn get_backup_secret(account: &str) -> Result<Vec<u8>> {
    macos_keychain_get(SERVICE, account)
}

#[cfg(target_os = "macos")]
fn delete_backup_secret(account: &str) -> Result<()> {
    macos_keychain_delete(SERVICE, account)
}

// Update an existing item, or add it when absent. The lookup never requests the
// password buffer, so updating needs only modify-content authorization, not
// decrypt — refreshing a backup whose ACL grants one but not the other won't
// prompt or fall through to a duplicate-item add.
#[cfg(target_os = "macos")]
fn macos_keychain_set(service: &str, account: &str, value: &[u8]) -> Result<()> {
    use security_framework::os::macos::keychain::SecKeychain;
    match macos_find_item_ref_no_decrypt(service, account)? {
        Some(item_ref) => macos_set_item_password(item_ref, account, value),
        None => {
            let keychain = SecKeychain::default().map_err(|error| {
                macos_keychain_error("SecKeychainCopyDefault", error.code(), account)
            })?;
            keychain
                .add_generic_password(service, account, value)
                .map_err(|error| {
                    macos_keychain_error("SecKeychainAddGenericPassword", error.code(), account)
                })
        }
    }
}

// FFI, not `-w`: a single non-printable byte makes `find-generic-password -w`
// emit ASCII-hex instead of the raw value, which then fails JSON parsing.
#[cfg(target_os = "macos")]
fn macos_keychain_get(service: &str, account: &str) -> Result<Vec<u8>> {
    use security_framework::os::macos::passwords::find_generic_password;
    use security_framework_sys::base::errSecItemNotFound;
    let (password, _item) = find_generic_password(None, service, account).map_err(|error| {
        if error.code() == errSecItemNotFound {
            CsuError::CredentialEntryMissing(account.to_string())
        } else {
            macos_keychain_error("SecKeychainFindGenericPassword", error.code(), account)
        }
    })?;
    Ok(password.to_vec())
}

#[cfg(target_os = "macos")]
fn macos_keychain_delete(service: &str, account: &str) -> Result<()> {
    use core_foundation::base::TCFType;
    use security_framework::os::macos::keychain_item::SecKeychainItem;
    use security_framework_sys::base::errSecSuccess;
    use security_framework_sys::keychain_item::SecKeychainItemDelete;
    let Some(item_ref) = macos_find_item_ref_no_decrypt(service, account)? else {
        return Ok(());
    };
    let item = unsafe { SecKeychainItem::wrap_under_create_rule(item_ref) };
    let status = unsafe { SecKeychainItemDelete(item.as_concrete_TypeRef()) };
    if status == errSecSuccess {
        Ok(())
    } else {
        Err(macos_keychain_error("SecKeychainItemDelete", status, account))
    }
}

#[cfg(target_os = "macos")]
fn macos_set_item_password(
    item_ref: security_framework_sys::base::SecKeychainItemRef,
    account: &str,
    value: &[u8],
) -> Result<()> {
    use core_foundation::base::TCFType;
    use security_framework::os::macos::keychain_item::SecKeychainItem;
    let mut item = unsafe { SecKeychainItem::wrap_under_create_rule(item_ref) };
    item.set_password(value).map_err(|error| {
        macos_keychain_error("SecKeychainItemModifyAttributesAndData", error.code(), account)
    })
}

// Read / probe-exists shell out to /usr/bin/security so Claude Code's ACL keeps
// trusting the same identity. Write uses SecKeychainItemModifyAttributesAndData
// (what `add-generic-password -U` calls internally): it mutates data without
// touching the access object, and the payload never lands in argv.
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

// Active write is update-only: csu must never create the Claude Code item, or
// the new item's ACL would bind to csu and lock Claude Code out.
#[cfg(target_os = "macos")]
fn macos_set_secret(account: &str, value: &[u8]) -> Result<()> {
    let item_ref = macos_find_item_ref_no_decrypt(SERVICE, account)?
        .ok_or_else(|| CsuError::CredentialEntryMissing(account.to_string()))?;
    macos_set_item_password(item_ref, account, value)
}

// Locate an item by service+account without requesting the password buffer
// (NULL data pointers), mirroring how `add-generic-password -U` finds a
// duplicate. Returns Ok(None) when absent.
#[cfg(target_os = "macos")]
fn macos_find_item_ref_no_decrypt(
    service: &str,
    account: &str,
) -> Result<Option<security_framework_sys::base::SecKeychainItemRef>> {
    use security_framework_sys::base::{errSecItemNotFound, errSecSuccess};
    use security_framework_sys::keychain::SecKeychainFindGenericPassword;
    use std::ptr;
    let mut item_ref = ptr::null_mut();
    let status = unsafe {
        SecKeychainFindGenericPassword(
            ptr::null(),
            service.len() as u32,
            service.as_ptr().cast(),
            account.len() as u32,
            account.as_ptr().cast(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut item_ref,
        )
    };
    if status == errSecItemNotFound {
        Ok(None)
    } else if status == errSecSuccess {
        Ok(Some(item_ref))
    } else {
        Err(macos_keychain_error(
            "SecKeychainFindGenericPassword",
            status,
            account,
        ))
    }
}

#[cfg(target_os = "macos")]
fn macos_keychain_error(op: &str, status: i32, account: &str) -> CsuError {
    use security_framework_sys::base::errSecAuthFailed;
    const ERR_SEC_INTERACTION_NOT_ALLOWED: i32 = -25308;
    if status == errSecAuthFailed || status == ERR_SEC_INTERACTION_NOT_ALLOWED {
        CsuError::CredentialStore(format!(
            "macOS Keychain denied access to '{account}' (OSStatus {status}). \
             This typically happens once after upgrading csu — macOS prompts to \
             allow the new binary to use the existing Keychain item. Re-run the \
             command and click 'Always Allow' on the dialog."
        ))
    } else {
        CsuError::CredentialStore(format!(
            "{op}: OSStatus {status} (account '{account}')"
        ))
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

// Exercises the real macOS keychain FFI against a throwaway service name so it
// never touches the live "Claude Code-credentials" item. Marked #[ignore]
// because it writes to the login keychain (which must be unlocked) — run with
// `cargo test -- --ignored` on a Mac before shipping keychain changes.
#[cfg(all(test, target_os = "macos"))]
mod macos_keychain_tests {
    use super::*;

    struct Cleanup {
        service: String,
        account: String,
    }

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = macos_keychain_delete(&self.service, &self.account);
        }
    }

    #[test]
    #[ignore = "touches the macOS login keychain; run with `cargo test -- --ignored`"]
    fn keychain_round_trip_preserves_raw_bytes() {
        let service = format!("csu-selftest-{}", std::process::id());
        let account = "round-trip";
        let _cleanup = Cleanup {
            service: service.clone(),
            account: account.to_string(),
        };

        // Embedded control byte (0x01): the case where `find-generic-password
        // -w` would emit ASCII-hex and corrupt the read. FFI must return it raw.
        let created: &[u8] = b"{\"token\":\"a\x01b\",\"n\":1}";
        macos_keychain_set(&service, account, created).expect("create");
        assert_eq!(
            macos_keychain_get(&service, account).expect("read after create"),
            created,
            "stored bytes must round-trip exactly, including the control byte",
        );

        // Update must replace the value in place (the create-vs-update path).
        let updated: &[u8] = b"{\"token\":\"c\x02d\",\"n\":22}";
        macos_keychain_set(&service, account, updated).expect("update");
        assert_eq!(
            macos_keychain_get(&service, account).expect("read after update"),
            updated,
            "update must overwrite the stored bytes",
        );

        // Delete removes it, and deleting again is a silent no-op.
        macos_keychain_delete(&service, account).expect("delete");
        assert!(matches!(
            macos_keychain_get(&service, account),
            Err(CsuError::CredentialEntryMissing(_))
        ));
        macos_keychain_delete(&service, account).expect("delete is idempotent");
    }
}
