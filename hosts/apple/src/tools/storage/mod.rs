use std::env;
use std::path::PathBuf;

#[cfg(any(target_os = "ios", target_os = "macos"))]
use operit_host_api::HostSecretStore;
use operit_host_api::{
    HostError, HostResult, RuntimeSqliteConnection, RuntimeSqliteHost, RuntimeStorageEntry,
    RuntimeStorageHost, RuntimeStorageWriteHost, RuntimeStorageWriteSession,
};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

use operit_host_native_common::NativeRuntimeStorageHost;

#[derive(Clone, Debug)]
pub struct AppleRuntimeStorageHost {
    storage: NativeRuntimeStorageHost,
}

impl AppleRuntimeStorageHost {
    /// Returns the default Apple runtime data root.
    #[allow(non_snake_case)]
    pub fn defaultRuntimeRoot() -> PathBuf {
        let base = appleApplicationSupportDirectory();
        base.join("Operit2").join("runtime")
    }

    /// Returns the default Apple workspace collection root.
    #[allow(non_snake_case)]
    pub fn defaultWorkspaceRoot() -> PathBuf {
        let base = appleApplicationSupportDirectory();
        base.join("Operit2").join("workspaces")
    }

    /// Creates an Apple runtime storage host with explicit runtime and workspace roots.
    #[allow(non_snake_case)]
    pub fn new(runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            storage: NativeRuntimeStorageHost::new(runtimeRoot, workspaceRoot),
        }
    }
}

impl RuntimeStorageHost for AppleRuntimeStorageHost {
    fn openTransactionDatabase(&self, path: &str) -> operit_host_api::HostResult<Box<dyn operit_host_api::RuntimeSqliteConnection>> {
        RuntimeSqliteHost::openSqliteDatabase(self, path)
    }

    /// Returns the shared runtime storage root.
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        self.storage.runtimeRootDir()
    }

    /// Returns the shared workspace storage root.
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        self.storage.workspaceRootDir()
    }

    /// Reads a file through the shared storage boundary.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.storage.readBytes(path)
    }

    /// Reads a bounded range through the shared storage boundary.
    fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
        self.storage.readBytesRange(path, offset, length)
    }

    /// Publishes a complete file through the shared atomic writer.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.storage.writeBytes(path, content)
    }

    /// Flushes and publishes a private catalog through the shared atomic writer.
    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.storage.writeBytesAtomically(path, content)
    }

    /// Appends bytes through the shared storage boundary.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        self.storage.appendBytes(path, content)
    }

    /// Flushes a file and its parent directory through the shared host.
    fn flushFile(&self, path: &str) -> HostResult<()> {
        self.storage.flushFile(path)
    }

    /// Deletes an entry through the shared storage boundary.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        self.storage.delete(path, recursive)
    }

    /// Tests an entry through the shared storage boundary.
    fn exists(&self, path: &str) -> HostResult<bool> {
        self.storage.exists(path)
    }

    /// Lists logical entries without exposing host-owned staging files.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        self.storage.list(prefix)
    }
}

impl RuntimeStorageWriteHost for AppleRuntimeStorageHost {
    /// Opens a streaming writer owned by the same shared storage host.
    fn createWriteSession(&self, path: &str) -> HostResult<Box<dyn RuntimeStorageWriteSession>> {
        self.storage.createWriteSession(path)
    }
}

impl RuntimeSqliteHost for AppleRuntimeStorageHost {
    /// Opens a database through the shared storage boundary.
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.storage.openSqliteDatabase(path)
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl HostSecretStore for AppleRuntimeStorageHost {
    /// Reads secret bytes from Apple Keychain.
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        let account = validateSecretKey(key)?;
        match get_generic_password(APPLE_SECRET_SERVICE, &account) {
            Ok(content) => Ok(Some(content)),
            Err(error) if error.code() == -25300 => Ok(None),
            Err(error) => Err(HostError::new(format!(
                "read Apple Keychain item failed: {error}"
            ))),
        }
    }

    /// Writes secret bytes into Apple Keychain.
    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        let account = validateSecretKey(key)?;
        set_generic_password(APPLE_SECRET_SERVICE, &account, content)
            .map_err(|error| HostError::new(format!("write Apple Keychain item failed: {error}")))
    }

    /// Deletes secret bytes from Apple Keychain.
    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        let account = validateSecretKey(key)?;
        match delete_generic_password(APPLE_SECRET_SERVICE, &account) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == -25300 => Ok(()),
            Err(error) => Err(HostError::new(format!(
                "delete Apple Keychain item failed: {error}"
            ))),
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
const APPLE_SECRET_SERVICE: &str = "Operit2.HostSecretStore";

/// Validates and returns a platform-safe host secret key.
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn validateSecretKey(key: &str) -> HostResult<String> {
    if key.is_empty()
        || key.chars().any(|character| {
            !(character.is_ascii_alphanumeric()
                || character == '.'
                || character == '_'
                || character == '-')
        })
    {
        return Err(HostError::new(format!("invalid host secret key: {key}")));
    }
    Ok(key.to_string())
}

/// Returns the Apple application support directory.
#[allow(non_snake_case)]
fn appleApplicationSupportDirectory() -> PathBuf {
    let home = env::var_os("HOME").expect("HOME is required for Apple runtime storage");
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
}
