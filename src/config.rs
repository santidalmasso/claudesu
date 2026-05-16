use std::fs;

use serde_json::{Map, Value};

use crate::errors::{CsuError, Result};
use crate::fs_util;
use crate::models::OauthAccountView;
use crate::paths;

pub fn load_global() -> Result<Value> {
    let path = paths::global_config_file()?;
    if !path.exists() {
        return Err(CsuError::ConfigMissing(path));
    }
    let bytes = fs::read(&path)?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| CsuError::JsonAt {
        origin: format!("global config at {}", path.display()),
        error,
    })?;
    Ok(value)
}

pub fn save_global(value: &Value) -> Result<()> {
    let path = paths::global_config_file()?;
    let bytes = serde_json::to_vec_pretty(value)?;
    fs_util::write_private(&path, &bytes)?;
    Ok(())
}

pub fn save_backup(slot: u32, email: &str, value: &Value) -> Result<()> {
    paths::ensure_backup_root()?;
    let path = paths::config_backup_file(slot, email)?;
    let mut backup = Map::new();
    backup.insert("oauthAccount".to_string(), extract_oauth_account(value)?);
    let bytes = serde_json::to_vec_pretty(&Value::Object(backup))?;
    fs_util::write_private(&path, &bytes)?;
    Ok(())
}

pub fn load_backup(slot: u32, email: &str) -> Result<Value> {
    let path = paths::config_backup_file(slot, email)?;
    if !path.exists() {
        return Err(CsuError::ConfigMissing(path));
    }
    let bytes = fs::read(&path)?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| CsuError::JsonAt {
        origin: format!("config backup for slot {slot} at {}", path.display()),
        error,
    })?;
    Ok(value)
}

pub fn delete_backup(slot: u32, email: &str) -> Result<()> {
    let path = paths::config_backup_file(slot, email)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn extract_oauth_account(config: &Value) -> Result<Value> {
    config
        .get("oauthAccount")
        .cloned()
        .ok_or(CsuError::MissingOauthAccount)
}

pub fn view_oauth_account(config: &Value) -> Option<OauthAccountView> {
    let section = config.get("oauthAccount")?;
    serde_json::from_value(section.clone()).ok()
}

pub fn merge_oauth_account(base: &Value, other: &Value) -> Result<Value> {
    let incoming = extract_oauth_account(other)?;
    let mut merged = base.clone();
    match &mut merged {
        Value::Object(map) => {
            map.insert("oauthAccount".to_string(), incoming);
        }
        _ => {
            let mut map = serde_json::Map::new();
            map.insert("oauthAccount".to_string(), incoming);
            merged = Value::Object(map);
        }
    }
    Ok(merged)
}
