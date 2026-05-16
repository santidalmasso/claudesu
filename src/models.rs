use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub email: String,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(rename = "organizationUuid", default)]
    pub organization_uuid: Option<String>,
    #[serde(rename = "organizationName", default)]
    pub organization_name: Option<String>,
    #[serde(default = "Utc::now")]
    pub added: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SequenceFile {
    #[serde(rename = "activeAccountNumber", default)]
    pub active_account_number: Option<u32>,
    #[serde(default)]
    pub sequence: Vec<u32>,
    #[serde(default)]
    pub accounts: BTreeMap<String, Account>,
}

impl SequenceFile {
    pub fn next_slot(&self) -> u32 {
        (1..)
            .find(|n| !self.accounts.contains_key(&n.to_string()))
            .unwrap_or(1)
    }

    pub fn contains_email(&self, email: &str) -> Option<u32> {
        self.accounts
            .iter()
            .find(|(_, a)| a.email.eq_ignore_ascii_case(email))
            .and_then(|(k, _)| k.parse().ok())
    }

    pub fn find_by_uuid(&self, uuid: &str) -> Option<u32> {
        self.accounts
            .iter()
            .find(|(_, a)| a.uuid.as_deref() == Some(uuid))
            .and_then(|(k, _)| k.parse().ok())
    }

    pub fn get_slot(&self, slot: u32) -> Option<&Account> {
        self.accounts.get(&slot.to_string())
    }

    pub fn next_after(&self, slot: u32) -> Option<u32> {
        let idx = self.sequence.iter().position(|s| *s == slot)?;
        Some(self.sequence[(idx + 1) % self.sequence.len()])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Credentials(pub serde_json::Value);

impl Credentials {
    pub fn to_compact_bytes(&self) -> crate::errors::Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.0)?)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OauthAccountView {
    #[serde(rename = "emailAddress", default)]
    pub email_address: Option<String>,
    #[serde(rename = "accountUuid", default)]
    pub account_uuid: Option<String>,
    #[serde(rename = "organizationUuid", default)]
    pub organization_uuid: Option<String>,
    #[serde(rename = "organizationName", default)]
    pub organization_name: Option<String>,
}
