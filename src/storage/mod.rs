use crate::errors::Result;
use crate::models::Credentials;

pub trait CredentialStore {
    fn read_active(&self) -> Result<Credentials>;

    fn write_active(&self, creds: &Credentials) -> Result<()>;

    fn write_backup(&self, slot: u32, email: &str, creds: &Credentials) -> Result<()>;

    fn read_backup(&self, slot: u32, email: &str) -> Result<Credentials>;

    fn delete_backup(&self, slot: u32, email: &str) -> Result<()>;
}

mod file_store;
pub use file_store::FileStore;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod system;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub use system::SystemStore;

pub fn default_store() -> Box<dyn CredentialStore> {
    if std::env::var_os("CSU_FORCE_FILE_STORE").is_some() {
        return Box::new(FileStore);
    }
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    {
        Box::new(SystemStore)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Box::new(FileStore)
    }
}
