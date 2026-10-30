use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use operit_host_api::{
    HostError, HostResult, HostSecretStore, RuntimeSqliteConnection, RuntimeSqliteHost,
    RuntimeStorageEntry, RuntimeStorageHost, RuntimeStorageWriteHost, RuntimeStorageWriteSession,
};
use uuid::Uuid;

use operit_host_native_common::NativeRuntimeStorageHost;

#[derive(Clone, Debug)]
pub struct LinuxRuntimeStorageHost {
    storage: NativeRuntimeStorageHost,
    secretRoot: PathBuf,
}

impl LinuxRuntimeStorageHost {
    /// Returns the default Linux runtime data root.
    #[allow(non_snake_case)]
    pub fn defaultRuntimeRoot() -> PathBuf {
        if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(xdg_data_home).join("operit2").join("runtime");
        }
        let home = env::var_os("HOME").expect("HOME is required for Operit2 runtime storage");
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("operit2")
            .join("runtime")
    }

    /// Returns the default Linux workspace collection root.
    #[allow(non_snake_case)]
    pub fn defaultWorkspaceRoot() -> PathBuf {
        if let Some(xdg_data_home) = env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(xdg_data_home)
                .join("operit2")
                .join("workspaces");
        }
        let home = env::var_os("HOME").expect("HOME is required for Operit2 runtime storage");
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("operit2")
            .join("workspaces")
    }

    /// Creates a Linux runtime storage host with explicit roots.
    #[allow(non_snake_case)]
    pub fn new(runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            storage: NativeRuntimeStorageHost::new(runtimeRoot, workspaceRoot),
            secretRoot: defaultHostSecretRoot(),
        }
    }

    /// Reads one Linux host secret from the private secret directory.
    fn readSecretFile(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        let path = self.secretPath(key)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        self.validateSecretFile(&path, &metadata)?;
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        Ok(Some(content))
    }

    /// Writes one Linux host secret through an atomic private file replacement.
    fn writeSecretFile(&self, key: &str, content: &[u8]) -> HostResult<()> {
        self.ensureSecretRoot()?;
        let path = self.secretPath(key)?;
        let temporaryPath = self.secretRoot.join(format!(
            ".{}.{}.tmp",
            validateSecretKey(key)?,
            Uuid::new_v4()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temporaryPath)?;
        file.write_all(content)?;
        file.sync_all()?;
        fs::rename(temporaryPath, path)?;
        self.syncSecretRoot()
    }

    /// Deletes one Linux host secret from the private secret directory.
    fn deleteSecretFile(&self, key: &str) -> HostResult<()> {
        let path = self.secretPath(key)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        self.validateSecretFile(&path, &metadata)?;
        fs::remove_file(path)?;
        self.syncSecretRoot()
    }

    /// Ensures the Linux host secret directory exists with private permissions.
    fn ensureSecretRoot(&self) -> HostResult<()> {
        fs::create_dir_all(&self.secretRoot)?;
        let metadata = fs::symlink_metadata(&self.secretRoot)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(HostError::new(format!(
                "Linux host secret root must be a directory: {}",
                self.secretRoot.display()
            )));
        }
        fs::set_permissions(&self.secretRoot, fs::Permissions::from_mode(0o700))?;
        Ok(())
    }

    /// Resolves one validated secret key beneath the Linux host secret directory.
    fn secretPath(&self, key: &str) -> HostResult<PathBuf> {
        Ok(self.secretRoot.join(validateSecretKey(key)?))
    }

    /// Validates that one persisted Linux host secret is a private regular file.
    fn validateSecretFile(&self, path: &Path, metadata: &fs::Metadata) -> HostResult<()> {
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(HostError::new(format!(
                "Linux host secret must be a regular file: {}",
                path.display()
            )));
        }
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(HostError::new(format!(
                "Linux host secret has insecure permissions: {}",
                path.display()
            )));
        }
        Ok(())
    }

    /// Synchronizes Linux host secret directory metadata after a mutation.
    fn syncSecretRoot(&self) -> HostResult<()> {
        File::open(&self.secretRoot)?.sync_all()?;
        Ok(())
    }
}

/// Returns the per-user Linux directory used for private host secrets.
fn defaultHostSecretRoot() -> PathBuf {
    if let Some(xdg_state_home) = env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(xdg_state_home)
            .join("operit2")
            .join("secrets");
    }
    let home = env::var_os("HOME").expect("HOME is required for Operit2 host secrets");
    PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("operit2")
        .join("secrets")
}

impl RuntimeStorageHost for LinuxRuntimeStorageHost {
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

impl RuntimeStorageWriteHost for LinuxRuntimeStorageHost {
    /// Opens a streaming writer owned by the same shared storage host.
    fn createWriteSession(&self, path: &str) -> HostResult<Box<dyn RuntimeStorageWriteSession>> {
        self.storage.createWriteSession(path)
    }
}

impl RuntimeSqliteHost for LinuxRuntimeStorageHost {
    /// Opens a database through the shared storage boundary.
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.storage.openSqliteDatabase(path)
    }
}

impl HostSecretStore for LinuxRuntimeStorageHost {
    /// Reads a private Linux host secret.
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        self.readSecretFile(key)
    }

    /// Writes a private Linux host secret.
    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        self.writeSecretFile(key, content)
    }

    /// Deletes a private Linux host secret.
    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        self.deleteSecretFile(key)
    }
}

/// Validates a private host secret key.
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
