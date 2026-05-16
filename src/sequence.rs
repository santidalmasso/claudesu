use std::fs;

use crate::errors::{CsuError, Result};
use crate::fs_util;
use crate::models::SequenceFile;
use crate::paths;

pub fn load() -> Result<SequenceFile> {
    let path = paths::sequence_file()?;
    if !path.exists() {
        return Ok(SequenceFile::default());
    }
    let bytes = fs::read(&path)?;
    if bytes.is_empty() {
        return Ok(SequenceFile::default());
    }
    let seq: SequenceFile =
        serde_json::from_slice(&bytes).map_err(|error| crate::errors::CsuError::JsonAt {
            origin: format!("sequence file at {}", path.display()),
            error,
        })?;
    Ok(seq)
}

pub fn save(seq: &SequenceFile) -> Result<()> {
    paths::ensure_backup_root()?;
    let path = paths::sequence_file()?;
    let bytes = serde_json::to_vec_pretty(seq)?;
    fs_util::write_private(&path, &bytes)?;
    Ok(())
}

pub fn resolve(seq: &SequenceFile, who: &str) -> Result<u32> {
    if let Ok(n) = who.parse::<u32>() {
        if seq.get_slot(n).is_some() {
            return Ok(n);
        }
        return Err(CsuError::AccountNotFound(who.to_string()));
    }
    seq.contains_email(who)
        .ok_or_else(|| CsuError::AccountNotFound(who.to_string()))
}
