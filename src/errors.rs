use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CsuError {
    #[error("no active account in sequence")]
    NoActiveAccount,

    #[error("Claude Code is logged into {0}, which csu hasn't stored — run `csu add` first")]
    UnstoredActiveAccount(String),

    #[error("account not found: {0}")]
    AccountNotFound(String),

    #[error("account already exists: {0}")]
    AccountAlreadyExists(String),

    #[error("slot {0} already in use")]
    SlotTaken(u32),

    #[error("credentials not found at {0} — is Claude Code logged in?")]
    CredentialsMissing(PathBuf),

    #[error("config not found at {0}")]
    ConfigMissing(PathBuf),

    #[error("secure credential entry not found for {0}")]
    CredentialEntryMissing(String),

    #[error("credential store error: {0}")]
    CredentialStore(String),

    #[error("invalid credentials JSON: {0}")]
    InvalidCredentials(String),

    #[error("no oauthAccount section in config")]
    MissingOauthAccount,

    #[error("cannot determine home directory")]
    NoHomeDir,

    #[error("another csu process is running (lock held)")]
    LockHeld,

    #[error("slot numbers start at 1")]
    InvalidSlot,

    #[error("refusing to purge unsafe backup root: {0}")]
    UnsafeBackupRoot(PathBuf),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("failed to parse {origin}: {error}")]
    JsonAt {
        origin: String,
        #[source]
        error: serde_json::Error,
    },

    #[error("prompt error: {0}")]
    Prompt(String),
}

impl From<dialoguer::Error> for CsuError {
    fn from(e: dialoguer::Error) -> Self {
        CsuError::Prompt(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CsuError>;
