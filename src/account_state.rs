use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::errors::Result;
use crate::fs_util;
use crate::paths;

pub const SCOPED_FILES: &[&str] = &["remote-settings.json", "policy-limits.json"];

pub struct Snapshot {
    files: Vec<(&'static str, Option<Vec<u8>>)>,
}

impl Snapshot {
    pub fn capture() -> Result<Self> {
        let home = paths::claude_config_home()?;
        let mut files = Vec::with_capacity(SCOPED_FILES.len());
        for name in SCOPED_FILES {
            files.push((*name, read_optional(&home.join(name))?));
        }
        Ok(Self { files })
    }

    pub fn from_backup(slot: u32, email: &str) -> Result<Self> {
        let mut files = Vec::with_capacity(SCOPED_FILES.len());
        for name in SCOPED_FILES {
            let path = paths::state_backup_file(slot, email, name)?;
            files.push((*name, read_optional(&path)?));
        }
        Ok(Self { files })
    }

    pub fn apply(&self) -> Result<()> {
        let home = paths::claude_config_home()?;
        for (name, content) in &self.files {
            write_or_delete(&home.join(name), content.as_deref())?;
        }
        Ok(())
    }

    pub fn save_as_backup(&self, slot: u32, email: &str) -> Result<()> {
        paths::ensure_backup_root()?;
        for (name, content) in &self.files {
            let path = paths::state_backup_file(slot, email, name)?;
            write_or_delete(&path, content.as_deref())?;
        }
        Ok(())
    }
}

pub fn delete_backup(slot: u32, email: &str) -> Result<()> {
    for name in SCOPED_FILES {
        fs_util::remove_if_exists(&paths::state_backup_file(slot, email, name)?)?;
    }
    Ok(())
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn write_or_delete(path: &Path, content: Option<&[u8]>) -> Result<()> {
    match content {
        Some(bytes) => fs_util::write_private(path, bytes),
        None => fs_util::remove_if_exists(path),
    }
}
