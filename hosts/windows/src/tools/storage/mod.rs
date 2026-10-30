use std::env;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::slice;

use operit_host_api::{
    HostError, HostResult, HostSecretStore, RuntimeSqliteConnection, RuntimeSqliteHost,
    RuntimeStorageEntry, RuntimeStorageHost, RuntimeStorageWriteHost, RuntimeStorageWriteSession,
};
use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND, FILETIME};
use windows_sys::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
    CRED_TYPE_GENERIC,
};

use operit_host_native_common::NativeRuntimeStorageHost;

#[derive(Clone, Debug)]
pub struct WindowsRuntimeStorageHost {
    storage: NativeRuntimeStorageHost,
}

impl WindowsRuntimeStorageHost {
    /// Returns the default Windows runtime data root.
    #[allow(non_snake_case)]
    pub fn defaultRuntimeRoot() -> PathBuf {
        let appdata =
            env::var_os("APPDATA").expect("APPDATA is required for Operit2 runtime storage");
        PathBuf::from(appdata).join("Operit2").join("runtime")
    }

    /// Returns the default Windows workspace collection root.
    #[allow(non_snake_case)]
    pub fn defaultWorkspaceRoot() -> PathBuf {
        let appdata =
            env::var_os("APPDATA").expect("APPDATA is required for Operit2 runtime storage");
        PathBuf::from(appdata).join("Operit2").join("workspaces")
    }

    /// Creates a Windows runtime storage host with explicit roots.
    #[allow(non_snake_case)]
    pub fn new(runtimeRoot: PathBuf, workspaceRoot: PathBuf) -> Self {
        Self {
            storage: NativeRuntimeStorageHost::new(runtimeRoot, workspaceRoot),
        }
    }
}

impl RuntimeStorageHost for WindowsRuntimeStorageHost {
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

impl RuntimeStorageWriteHost for WindowsRuntimeStorageHost {
    /// Opens a streaming writer owned by the same shared storage host.
    fn createWriteSession(&self, path: &str) -> HostResult<Box<dyn RuntimeStorageWriteSession>> {
        self.storage.createWriteSession(path)
    }
}

impl RuntimeSqliteHost for WindowsRuntimeStorageHost {
    /// Opens a database through the shared storage boundary.
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.storage.openSqliteDatabase(path)
    }
}

impl HostSecretStore for WindowsRuntimeStorageHost {
    /// Reads a secret from Windows Credential Manager.
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        let target = windowsSecretTargetName(key)?;
        let mut credential = null_mut::<CREDENTIALW>();
        let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
        if ok == 0 {
            let error = unsafe { GetLastError() };
            if error == ERROR_NOT_FOUND {
                return Ok(None);
            }
            return Err(lastWindowsCredentialError("read Windows credential"));
        }
        let credentialRef = unsafe { &*credential };
        let content = unsafe {
            slice::from_raw_parts(
                credentialRef.CredentialBlob,
                credentialRef.CredentialBlobSize as usize,
            )
            .to_vec()
        };
        unsafe { CredFree(credential.cast()) };
        Ok(Some(content))
    }

    /// Writes a secret into Windows Credential Manager.
    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        let mut target = windowsSecretTargetName(key)?;
        let mut userName = widestring("Operit2")?;
        let mut credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            Comment: null_mut(),
            LastWritten: FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: content.len() as u32,
            CredentialBlob: content.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: null_mut(),
            TargetAlias: null_mut(),
            UserName: userName.as_mut_ptr(),
        };
        let ok = unsafe { CredWriteW(&mut credential, 0) };
        if ok == 0 {
            return Err(lastWindowsCredentialError("write Windows credential"));
        }
        Ok(())
    }

    /// Deletes a secret from Windows Credential Manager.
    fn deleteSecret(&self, key: &str) -> HostResult<()> {
        let target = windowsSecretTargetName(key)?;
        let ok = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok == 0 {
            let error = unsafe { GetLastError() };
            if error != ERROR_NOT_FOUND {
                return Err(lastWindowsCredentialError("delete Windows credential"));
            }
        }
        Ok(())
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

/// Builds the Windows Credential Manager target name for a host secret key.
fn windowsSecretTargetName(key: &str) -> HostResult<Vec<u16>> {
    let name = validateSecretKey(key)?;
    widestring(&format!("Operit2:{name}"))
}

/// Converts Rust text into a nul-terminated Windows wide string.
fn widestring(value: &str) -> HostResult<Vec<u16>> {
    if value.encode_utf16().any(|unit| unit == 0) {
        return Err(HostError::new("Windows credential text contains nul"));
    }
    Ok(value.encode_utf16().chain(std::iter::once(0)).collect())
}

/// Converts the last Windows credential error into a host error.
fn lastWindowsCredentialError(action: &str) -> HostError {
    HostError::new(format!("{action} failed: Windows error {}", unsafe {
        GetLastError()
    }))
}
