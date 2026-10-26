#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_api::{RuntimeStorageHost, RuntimeStorageWriteHost};
use operit_tools::files::MountRegistry::{MountRegistry, MountSource};
use operit_util::stream::ReverseStream::ReverseStream;
use operit_util::RuntimeStorageLayout::RUNTIME_CLEAN_ON_EXIT_DIR_PATH;
use uuid::Uuid;

const MAX_CHUNK_BYTES: usize = 1024 * 1024;
const VFS_SELECTION_ROOT: &str = "/app/data/temp/clean_on_exit/file_picker/";

/// Publishes host-selected files and directories through the runtime's VFS.
#[derive(Clone)]
pub struct FileSelectionService {
    storage: Arc<dyn RuntimeStorageHost>,
    writers: Arc<dyn RuntimeStorageWriteHost>,
}

impl FileSelectionService {
    /// Binds selected resources to the runtime that owns the file interfaces.
    pub fn getInstance(host: &HostManager) -> Result<Self, String> {
        Ok(Self {
            storage: host
                .runtimeStorageHost
                .clone()
                .ok_or("RuntimeStorageHost is not registered for file selection")?,
            writers: host
                .runtimeStorageWriteHost
                .clone()
                .ok_or("RuntimeStorageWriteHost is not registered for file selection")?,
        })
    }

    /// Allocates a VFS destination while preserving the selected display filename.
    pub fn createFile(&self, fileName: String) -> Result<String, String> {
        validateFileName(&fileName)?;
        Ok(format!(
            "{VFS_SELECTION_ROOT}{}/{fileName}",
            Uuid::new_v4().simple()
        ))
    }

    /// Commits a selected file only after its bounded stream matches the declared size.
    pub async fn writeFile(
        &self,
        path: String,
        expectedByteLength: i64,
        mut bytes: ReverseStream<Vec<u8>>,
    ) -> Result<(), String> {
        if expectedByteLength < 0 {
            return Err("Selected file byte length must not be negative".into());
        }
        let storagePath = selectionStoragePath(&path)?;
        let mut writer = self
            .writers
            .createWriteSession(&storagePath)
            .map_err(|error| error.to_string())?;
        let result = async {
            let mut received = 0i64;
            while let Some(chunk) = bytes.recv().await {
                if chunk.len() > MAX_CHUNK_BYTES {
                    return Err(format!("Selected file chunk exceeds {MAX_CHUNK_BYTES} bytes"));
                }
                received = received.checked_add(chunk.len() as i64)
                    .ok_or("Selected file byte length overflow")?;
                if received > expectedByteLength {
                    return Err("Selected file exceeds its declared byte length".into());
                }
                writer.writeChunk(&chunk).map_err(|error| error.to_string())?;
            }
            if received != expectedByteLength {
                return Err(format!("Selected file byte length mismatch: expected {expectedByteLength}, got {received}"));
            }
            Ok(())
        }.await;
        match result {
            Ok(()) => writer.commit().map_err(|error| error.to_string()),
            Err(error) => {
                if let Err(cleanup) = writer.discard() {
                    operit_host_api::logHostError("FileSelection", &cleanup.to_string());
                }
                Err(error)
            }
        }
    }

    /// Removes a selected file when its picker operation fails before delivery.
    pub fn deleteFile(&self, path: String) -> Result<(), String> {
        let storagePath = selectionStoragePath(&path)?;
        if self
            .storage
            .exists(&storagePath)
            .map_err(|error| error.to_string())?
        {
            self.storage
                .delete(&storagePath, false)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Registers an authorized folder source and returns its public VFS mount root.
    pub fn mountDirectory(&self, sourceJson: String) -> Result<String, String> {
        let source: MountSource = serde_json::from_str(&sourceJson)
            .map_err(|error| format!("Invalid selected directory source: {error}"))?;
        MountRegistry::withStorage(self.storage.clone())
            .register(
                &source.namespace,
                &source.backend,
                &source.root,
                &source.name,
            )
            .map(|mount| mount.vfsPath())
    }
}

/// Requires one display filename without path traversal or embedded separators.
fn validateFileName(name: &str) -> Result<(), String> {
    if name.trim().is_empty()
        || matches!(name, "." | "..")
        || name.chars().any(|value| matches!(value, '/' | '\\' | '\0'))
    {
        return Err("Selected file name must be a single non-empty filename".into());
    }
    Ok(())
}

/// Restricts streamed writes and cleanup to the file-selection temporary directory.
fn selectionStoragePath(path: &str) -> Result<String, String> {
    let relative = path
        .strip_prefix(VFS_SELECTION_ROOT)
        .ok_or("Selected file path is outside its VFS directory")?;
    let (id, name) = relative
        .split_once('/')
        .ok_or("Invalid selected file path")?;
    let parsedId = Uuid::parse_str(id).map_err(|_| "Invalid selected file identity")?;
    if parsedId.simple().to_string() != id {
        return Err("Selected file identity must use its canonical form".into());
    }
    validateFileName(name)?;
    Ok(format!(
        "{RUNTIME_CLEAN_ON_EXIT_DIR_PATH}/file_picker/{id}/{name}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keeps selected file destinations inside the registered ephemeral storage tree.
    #[test]
    fn selectedPathsPreserveNamesAndRejectEscapes() {
        let id = Uuid::new_v4().simple().to_string();
        let path = format!("{VFS_SELECTION_ROOT}{id}/世界书.json");
        assert_eq!(
            selectionStoragePath(&path).unwrap(),
            format!("{RUNTIME_CLEAN_ON_EXIT_DIR_PATH}/file_picker/{id}/世界书.json")
        );
        for name in ["", " ", ".", "..", "../a", "a/b", "a\\b", "a\0b"] {
            assert!(validateFileName(name).is_err());
        }
        for path in [
            "/private/var/mobile/tmp/book.json",
            "/app/data/config/a.json",
            "/app/data/temp/clean_on_exit/file_picker/../a.json",
        ] {
            assert!(selectionStoragePath(path).is_err());
        }
    }
}
