#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_api::{FileSystemHost, RuntimeStorageHost, RuntimeStorageWriteHost};
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_util::stream::ReverseStream::ReverseStream;
use std::path::Path;
use uuid::Uuid;

use super::ChatServiceCore::getMimeTypeFromPath;

const ATTACHMENT_UPLOAD_MAX_CHUNK_BYTES: usize = 1024 * 1024;

/// Imports attachments through the shared policy into their final file or source location.
#[derive(Clone)]
pub struct AttachmentTransferManager {
    storageHost: Arc<dyn RuntimeStorageHost>,
    storageWriteHost: Arc<dyn RuntimeStorageWriteHost>,
    fileSystemHost: Arc<dyn FileSystemHost>,
}

impl AttachmentTransferManager {
    /// Binds attachment writes to the receiving runtime's Host, not the picker device.
    pub fn getInstance(hostManager: &HostManager) -> Result<Self, String> {
        Ok(Self {
            storageHost: hostManager
                .runtimeStorageHost
                .clone()
                .ok_or("RuntimeStorageHost is not registered for attachment uploads")?,
            storageWriteHost: hostManager
                .runtimeStorageWriteHost
                .clone()
                .ok_or("RuntimeStorageWriteHost is not registered for attachment uploads")?,
            fileSystemHost: hostManager
                .fileSystemHost
                .clone()
                .ok_or("FileSystemHost is not registered for attachment uploads")?,
        })
    }

    /// Exposes this chat's editable limits before the client opens a file stream.
    pub fn selectedAttachmentPolicy(&self, chatId: String) -> Result<String, String> {
        let policy = self.chatPolicy(&chatId)?;
        serde_json::to_string(&policy).map_err(|e| e.to_string())
    }

    fn chatPolicy(&self, chatId: &str) -> Result<operit_store::FileSyncPolicy::FileSyncPolicy, String> {
        let manager = operit_store::repository::ChatHistoryManager::ChatHistoryManager::default()
            .map_err(|e| e.to_string())?;
        let base = manager
            .fileDirectoryForChat(chatId)
            .map_err(|e| e.to_string())?;
        let storage = if let Some(relative) = base
            .strip_prefix("/operitapp/workspaces/")
            .or_else(|| base.strip_prefix("/app/workspaces/"))
        {
            format!("workspaces/{relative}")
        } else if base.starts_with("/operitapp/outputs/") {
            operit_store::ChatOutputStore::ChatOutputStore::storagePath(&base)?
        } else {
            // External mounts keep their own files. App-managed attachments use
            // the conversation output area's policy in that case.
            let output = manager
                .ensureOutputDirectory(chatId)
                .map_err(|e| e.to_string())?;
            operit_store::ChatOutputStore::ChatOutputStore::storagePath(&output)?
        };
        operit_store::FileSyncPolicy::FileSyncPolicy::ensure(self.storageHost.as_ref(), &storage)?;
        operit_store::FileSyncPolicy::FileSyncPolicy::load(self.storageHost.as_ref(), &storage)
    }

    /// Native attachments use their original source for large files and a bounded
    /// direct write for small files, sharing the picker upload policy.
    pub fn importNativeAttachment(
        &self,
        chatId: String,
        filePath: String,
    ) -> Result<AttachmentInfo, String> {
        let source = self
            .fileSystemHost
            .fileExists(&filePath)
            .map_err(|e| e.to_string())?;
        if !source.exists || source.isDirectory || source.size < 0 {
            return Err("Attachment source is not a readable file".into());
        }
        let name = Path::new(&filePath)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid attachment filename")?
            .to_string();
        validateAttachmentFileName(&name)?;
        let policy = self.chatPolicy(&chatId)?;
        if source.size as u64 > policy.maxFileBytes.min(policy.maxTotalBytes) {
            let reference = self.registerAttachmentSource(serde_json::json!({
                    "namespace": "/virtual/local/attachments", "backend": "native", "root": filePath, "name": name,
                }).to_string(), name, source.size)?;
            return self.createReferenceAttachment(chatId, reference);
        }
        let id = Uuid::new_v4().simple().to_string();
        let path = selectedAttachmentStoragePath(&chatId, &id, &name)?;
        let mut writer = self
            .storageWriteHost
            .createWriteSession(&path)
            .map_err(|e| e.to_string())?;
        let result = (|| {
            let mut offset = 0u64;
            while offset < source.size as u64 {
                let length = (source.size as u64 - offset).min(64 * 1024) as usize;
                let bytes = self
                    .fileSystemHost
                    .readFileBytesRange(&filePath, offset, length)
                    .map_err(|e| e.to_string())?;
                if bytes.is_empty() || bytes.len() > length {
                    return Err("Attachment source changed while reading".to_string());
                }
                writer.writeChunk(&bytes).map_err(|e| e.to_string())?;
                offset += bytes.len() as u64;
            }
            if !self
                .fileSystemHost
                .readFileBytesRange(&filePath, offset, 1)
                .map_err(|e| e.to_string())?
                .is_empty()
            {
                return Err("Attachment source grew while reading".into());
            }
            Ok(())
        })();
        match result {
            Ok(()) => writer.commit().map_err(|e| e.to_string())?,
            Err(error) => {
                let _ = writer.discard();
                return Err(error);
            }
        }
        self.completeSelectedAttachment(chatId, id, name, Some(source.size))
    }

    /// Registers an authorized, reusable source without copying any file bytes.
    pub fn registerAttachmentSource(
        &self,
        sourceJson: String,
        fileName: String,
        expectedByteLength: i64,
    ) -> Result<String, String> {
        validateAttachmentFileName(&fileName)?;
        validateLength(Some(expectedByteLength))?;
        let source: operit_tools::files::MountRegistry::MountSource =
            serde_json::from_str(&sourceJson).map_err(|e| e.to_string())?;
        let mount =
            operit_tools::files::MountRegistry::MountRegistry::withStorage(self.storageHost.clone())
                .register(
                    &source.namespace,
                    &source.backend,
                    &source.root,
                    &source.name,
                )?;
        let mapper = operit_tools::files::PathMapper::PathMapper::new(
            self.storageHost
                .runtimeRootDir()
                .ok_or("Missing runtime root")?,
            self.storageHost
                .workspaceRootDir()
                .ok_or("Missing workspace root")?,
            self.storageHost.clone(),
        );
        let path = mount.vfsPath();
        let resolved = mapper.resolve(&path)?;
        let metadata = self
            .fileSystemHost
            .fileExists(&resolved.physicalPath)
            .map_err(|e| e.to_string())?;
        if !metadata.exists || metadata.isDirectory || metadata.size != expectedByteLength {
            return Err("Selected source is unavailable or its size changed".into());
        }
        let reference = operit_store::FileSyncPolicy::FileReference::new(
            self.storageHost.clone(),
            path,
            fileName,
            metadata.size,
            None,
        )?;
        String::from_utf8(reference.bytes()?).map_err(|e| e.to_string())
    }

    /// Uses the selected-file pipeline for bytes already supplied by a paste operation.
    pub(crate) async fn importBytesAttachment(
        &self, chatId: String, fileName: String, bytes: &[u8],
    ) -> Result<AttachmentInfo, String> {
        let size = i64::try_from(bytes.len()).map_err(|_| "Attachment size overflow")?;
        let policy = self.chatPolicy(&chatId)?;
        if size as u64 > policy.maxTotalBytes {
            return Err("Attachment exceeds the configured retention limit".into());
        }
        let large = size as u64 > policy.maxFileBytes;
        let id = Uuid::new_v4().simple().to_string();
        let (mut sender, input) = ReverseStream::channel();
        let producer = async {
            for chunk in bytes.chunks(ATTACHMENT_UPLOAD_MAX_CHUNK_BYTES) {
                sender.send(chunk.to_vec()).await?;
            }
            sender.send(Vec::new()).await?;
            sender.close();
            Ok::<_, String>(())
        };
        let consumer = async {
            if large {
                self.writeAttachmentSource(id.clone(), fileName.clone(), Some(size),
                    i64::try_from(policy.maxTotalBytes).unwrap_or(i64::MAX), input).await
            } else {
                self.writeSelectedAttachment(chatId.clone(), id.clone(), fileName.clone(), Some(size), input).await
            }
        };
        let (produced, written) = tokio::join!(producer, consumer);
        written?;
        produced?;
        if large {
            let reference = self.completeAttachmentSource(id, fileName, Some(size))?;
            self.createReferenceAttachment(chatId, reference)
        } else {
            self.completeSelectedAttachment(chatId, id, fileName, Some(size))
        }
    }

    /// Persists just the locator in the receiving chat; its source may be another node.
    pub fn createReferenceAttachment(
        &self,
        chatId: String,
        referenceJson: String,
    ) -> Result<AttachmentInfo, String> {
        self.chatPolicy(&chatId)?;
        if referenceJson.len() > operit_store::FileSyncPolicy::MAX_POLICY_BYTES {
            return Err("File reference is too large".into());
        }
        let reference: operit_store::FileSyncPolicy::FileReference =
            serde_json::from_str(&referenceJson).map_err(|e| e.to_string())?;
        reference.validate()?;
        let name = format!(
            "{}{}",
            reference.fileName,
            operit_store::FileSyncPolicy::REFERENCE_SUFFIX
        );
        let path = selectedAttachmentStoragePath(&chatId, &Uuid::new_v4().simple().to_string(), &name)?;
        let bytes = reference.bytes()?;
        self.storageHost
            .writeBytesAtomically(&path, &bytes)
            .map_err(|error| error.to_string())?;
        self.persistentInfo(&path, name, bytes.len() as i64)
    }

    /// Streams once to the final durable location. Ephemeral large selections are
    /// retained on the source node; only their small locator is Space-synchronized.
    pub async fn writeSelectedAttachment(
        &self,
        chatId: String,
        uploadId: String,
        fileName: String,
        expectedByteLength: Option<i64>,
        mut bytes: ReverseStream<Vec<u8>>,
    ) -> Result<(), String> {
        let policy = self.chatPolicy(&chatId)?;
        let path = selectedAttachmentStoragePath(&chatId, &uploadId, &fileName)?;
        self.persistSelection(
            &path,
            expectedByteLength,
            policy.maxFileBytes.min(policy.maxTotalBytes),
            &mut bytes,
        )
        .await
    }

    pub fn completeSelectedAttachment(
        &self,
        chatId: String,
        uploadId: String,
        fileName: String,
        expectedByteLength: Option<i64>,
    ) -> Result<AttachmentInfo, String> {
        let path = selectedAttachmentStoragePath(&chatId, &uploadId, &fileName)?;
        let length = self.committedLength(&path, expectedByteLength)?;
        self.persistentInfo(&path, fileName, length)
    }

    /// Local-only storage is necessary for browser blobs and other ephemeral selections.
    pub async fn writeAttachmentSource(
        &self,
        uploadId: String,
        fileName: String,
        expectedByteLength: Option<i64>,
        maxBytes: i64,
        mut bytes: ReverseStream<Vec<u8>>,
    ) -> Result<(), String> {
        if maxBytes < 0 {
            return Err("Invalid attachment retention limit".into());
        }
        let path = sourceAttachmentPath(&uploadId, &fileName)?;
        self.persistSelection(&path, expectedByteLength, maxBytes as u64, &mut bytes)
            .await
    }

    pub fn completeAttachmentSource(
        &self,
        uploadId: String,
        fileName: String,
        expectedByteLength: Option<i64>,
    ) -> Result<String, String> {
        let path = sourceAttachmentPath(&uploadId, &fileName)?;
        let size = self.committedLength(&path, expectedByteLength)?;
        let reference = operit_store::FileSyncPolicy::FileReference::new(
            self.storageHost.clone(),
            format!("/operitapp/data/{}", path.strip_prefix("runtime/").unwrap()),
            fileName,
            size,
            None,
        )?;
        String::from_utf8(reference.bytes()?).map_err(|e| e.to_string())
    }

    fn committedLength(&self, path: &str, expected: Option<i64>) -> Result<i64, String> {
        let parent = path.rsplit_once('/').ok_or("Invalid attachment path")?.0;
        let entry = self
            .storageHost
            .list(parent)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|entry| entry.path == path && !entry.isDirectory)
            .ok_or("Attachment has not been committed")?;
        if expected.is_some_and(|size| size != entry.size) {
            return Err("Committed attachment size changed".into());
        }
        Ok(entry.size)
    }

    async fn persistSelection(
        &self,
        path: &str,
        expected: Option<i64>,
        limit: u64,
        bytes: &mut ReverseStream<Vec<u8>>,
    ) -> Result<(), String> {
        validateLength(expected)?;
        if expected.is_some_and(|size| size as u64 > limit) {
            return Err(format!("This selection has no reusable source; retaining it would exceed the configured {limit} byte limit. Select a durable file on a CoreNode or adjust .operit/sync.json."));
        }
        if self.storageHost.exists(path).map_err(|e| e.to_string())? {
            return Err("Attachment destination is already occupied".into());
        }
        let mut writer = self
            .storageWriteHost
            .createWriteSession(path)
            .map_err(|e| e.to_string())?;
        let result = async {
            let mut received = 0i64;
            let mut complete = false;
            while let Some(chunk) = bytes.recv().await {
                if complete {
                    return Err("Attachment data followed its completion marker".into());
                }
                if chunk.is_empty() {
                    complete = true;
                    continue;
                }
                if chunk.len() > ATTACHMENT_UPLOAD_MAX_CHUNK_BYTES {
                    return Err("Attachment chunk exceeds one MiB".to_string());
                }
                received = received
                    .checked_add(chunk.len() as i64)
                    .ok_or("Attachment size overflow")?;
                if received as u64 > limit || expected.is_some_and(|expected| received > expected) {
                    return Err(
                        "Attachment exceeds its declared size or configured retention limit".into(),
                    );
                }
                writer.writeChunk(&chunk).map_err(|e| e.to_string())?;
            }
            if !complete {
                return Err("Attachment input ended without a successful completion marker".into());
            }
            if expected.is_some_and(|expected| received != expected) {
                return Err("Attachment byte length changed during upload".into());
            }
            Ok(())
        }
        .await;
        match result {
            Ok(()) => writer.commit().map_err(|e| e.to_string()),
            Err(error) => {
                let _ = writer.discard();
                Err(error)
            }
        }
    }

    fn persistentInfo(
        &self,
        path: &str,
        fileName: String,
        fileSize: i64,
    ) -> Result<AttachmentInfo, String> {
        let vfsPath = operit_store::WorkspaceFileSyncStore::managedVfsPath(path)?;
        let mapper = operit_tools::files::PathMapper::PathMapper::new(
            self.storageHost
                .runtimeRootDir()
                .ok_or("Missing runtime root")?,
            self.storageHost
                .workspaceRootDir()
                .ok_or("Missing workspace root")?,
            self.storageHost.clone(),
        );
        Ok(AttachmentInfo {
            vfsPath: Some(vfsPath.clone()),
            nodeId: Some(
                CoreNodeIdentityStore::new(self.storageHost.clone())
                    .identity()?
                    .nodeId,
            ),
            filePath: mapper.resolve(&vfsPath)?.nativePath()?,
            mimeType: getMimeTypeFromPath(Path::new(&fileName)).to_string(),
            fileName,
            fileSize,
            content: String::new(),
        })
    }

    /// Allocates an opaque upload identity without reading or buffering file content.
    pub fn beginAttachmentUpload(
        &self,
        fileName: String,
        expectedByteLength: Option<i64>,
    ) -> Result<String, String> {
        validateAttachmentFileName(&fileName)?;
        validateLength(expectedByteLength)?;
        Ok(Uuid::new_v4().simple().to_string())
    }

}

/// Rejects names that would escape an attachment directory or alias its metadata.
pub(crate) fn validateAttachmentFileName(fileName: &str) -> Result<(), String> {
    if fileName.trim().is_empty()
        || matches!(fileName, "." | ".." | ".nomedia")
        || fileName
            .chars()
            .any(|value| value.is_control() || matches!(value, '/' | '\\' | ':'))
    {
        return Err("Attachment filename must be a single non-empty filename".to_string());
    }
    Ok(())
}

/// Rejects invalid declared lengths before starting a Host write.
fn validateLength(length: Option<i64>) -> Result<(), String> {
    if length.is_some_and(|length| length < 0) {
        return Err("Attachment byte length must not be negative".to_string());
    }
    Ok(())
}

fn sourceAttachmentPath(id: &str, name: &str) -> Result<String, String> {
    validateAttachmentFileName(name)?;
    let id = Uuid::parse_str(id).map_err(|_| "Invalid attachment identity")?;
    Ok(format!(
        "runtime/data/attachment_sources/{}/{name}",
        id.simple()
    ))
}

/// The new input flow stores attachments beside the files the AI is working on.
fn selectedAttachmentStoragePath(
    chatId: &str,
    attachmentId: &str,
    fileName: &str,
) -> Result<String, String> {
    validateAttachmentFileName(fileName)?;
    let id = Uuid::parse_str(attachmentId).map_err(|_| "Invalid attachment identity")?;
    let manager = operit_store::repository::ChatHistoryManager::ChatHistoryManager::default()
        .map_err(|e| e.to_string())?;
    let base = manager
        .fileDirectoryForChat(chatId)
        .map_err(|e| e.to_string())?;
    let root = if let Some(relative) = base
        .strip_prefix("/operitapp/workspaces/")
        .or_else(|| base.strip_prefix("/app/workspaces/"))
    {
        format!("workspaces/{relative}")
    } else {
        let output = manager
            .ensureOutputDirectory(chatId)
            .map_err(|e| e.to_string())?;
        operit_store::ChatOutputStore::ChatOutputStore::storagePath(&output)?
    };
    Ok(format!(
        "{root}/.operit/attachments/{}/{fileName}",
        id.simple()
    ))
}

pub(crate) fn isSelectedAttachment(
    attachment: &AttachmentInfo,
    chatId: &str,
) -> Result<bool, String> {
    let Some(path) = &attachment.vfsPath else {
        return Ok(false);
    };
    let Some((parent, name)) = path.rsplit_once('/') else {
        return Ok(false);
    };
    if name != attachment.fileName {
        return Ok(false);
    }
    let Some((_, id)) = parent.rsplit_once('/') else {
        return Ok(false);
    };
    let expected = selectedAttachmentStoragePath(chatId, id, name)?;
    Ok(operit_store::WorkspaceFileSyncStore::managedVfsPath(&expected)? == *path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rejects names and identities that would escape the durable source directory.
    #[test]
    fn source_paths_cannot_escape_the_attachment_directory() {
        let id = Uuid::new_v4().simple().to_string();
        assert!(sourceAttachmentPath(&id, "文档.apk")
            .unwrap()
            .starts_with("runtime/data/attachment_sources/"));
        for name in [
            "", " ", ".", "..", ".nomedia", "../bad", "a/b", "a\\b", "a\0b", "C:bad",
        ] {
            assert!(sourceAttachmentPath(&id, name).is_err(), "{name:?}");
        }
        assert!(sourceAttachmentPath("../bad", "file.apk").is_err());
        assert!(validateLength(Some(-1)).is_err());
        assert!(validateLength(None).is_ok());
        assert!(validateLength(Some(0)).is_ok());
    }

}
