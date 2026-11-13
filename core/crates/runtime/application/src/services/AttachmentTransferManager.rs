#![allow(non_snake_case)]
use operit_vfs::VisualFileSystem::VisualFileSystem;

use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_api::{RuntimeStorageHost, RuntimeStorageWriteHost};
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_util::stream::ReverseStream::ReverseStream;
use std::path::Path;
use uuid::Uuid;

use super::ChatServiceCore::getMimeTypeFromPath;

const ATTACHMENT_UPLOAD_MAX_CHUNK_BYTES: usize = 1024 * 1024;
// Leave room for reference suffixes and the Host writer's private staging filename.
const ATTACHMENT_FILE_NAME_MAX_BYTES: usize = 180;

/// Imports attachments through the shared policy into their final file or source location.
#[derive(Clone)]
pub struct AttachmentTransferManager {
    storageHost: Arc<dyn RuntimeStorageHost>,
    storageWriteHost: Arc<dyn RuntimeStorageWriteHost>,
    filesystem: VisualFileSystem,
}

impl AttachmentTransferManager {
    /// Binds attachment writes to the receiving runtime's Host, not the picker device.
    pub fn getInstance(hostManager: &HostManager) -> Result<Self, String> {
        Ok(Self {
            filesystem: hostManager.vfs()?,
            storageHost: hostManager
                .runtimeStorageHost
                .clone()
                .ok_or("RuntimeStorageHost is not registered for attachment uploads")?,
            storageWriteHost: hostManager
                .runtimeStorageWriteHost
                .clone()
                .ok_or("RuntimeStorageWriteHost is not registered for attachment uploads")?,
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
        let filePath = self.filesystem.resolveResourcePath(None, &filePath)?;
        let source = self
            .filesystem
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
        let name = attachmentStorageFileName(&name)?;
        let policy = self.chatPolicy(&chatId)?;
        if source.size as u64 > policy.maxFileBytes.min(policy.maxTotalBytes) {
            let reference = operit_store::FileSyncPolicy::FileReference::new(
                self.storageHost.clone(), filePath, name, source.size, None,
            )?;
            let reference = String::from_utf8(reference.bytes()?).map_err(|error| error.to_string())?;
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
                    .filesystem
                    .readFileBytesRange(&filePath, offset, length)
                    .map_err(|e| e.to_string())?;
                if bytes.is_empty() || bytes.len() > length {
                    return Err("Attachment source changed while reading".to_string());
                }
                writer.writeChunk(&bytes).map_err(|e| e.to_string())?;
                offset += bytes.len() as u64;
            }
            if !self
                .filesystem
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
        let fileName = attachmentStorageFileName(&fileName)?;
        validateLength(Some(expectedByteLength))?;
        let source: operit_vfs::MountRegistry::MountSource =
            serde_json::from_str(&sourceJson).map_err(|e| e.to_string())?;
        let mount =
            operit_vfs::MountRegistry::MountRegistry::withStorage(self.storageHost.clone())
                .register(
                    &source.namespace,
                    &source.backend,
                    &source.root,
                    &source.name,
                )?;
        let path = mount.vfsPath();
        let metadata = self
            .filesystem
            .fileExists(&path)?;
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
        let name = attachmentStorageFileName(&format!(
            "{}{}",
            reference.fileName,
            operit_store::FileSyncPolicy::REFERENCE_SUFFIX
        ))?;
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
        let fileName = attachmentStorageFileName(&fileName)?;
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
        let fileName = attachmentStorageFileName(&fileName)?;
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
        Ok(AttachmentInfo {
            vfsPath: Some(vfsPath.clone()),
            nodeId: Some(
                CoreNodeIdentityStore::new(self.storageHost.clone())
                    .identity()?
                    .nodeId,
            ),
            filePath: vfsPath.clone(),
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
        attachmentStorageFileName(&fileName)?;
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

/// Keeps portable names unchanged; UUID-owned directories make a fixed fallback unique.
fn attachmentStorageFileName(fileName: &str) -> Result<String, String> {
    validateAttachmentFileName(fileName)?;
    if fileName.len() <= ATTACHMENT_FILE_NAME_MAX_BYTES
        && operit_util::FileUtils::FileUtils::isPortableFileName(fileName)
    {
        return Ok(fileName.to_string());
    }
    // Preserve locator semantics even when a client uploads an existing reference file.
    let (base, suffix) = fileName
        .strip_suffix(operit_store::FileSyncPolicy::REFERENCE_SUFFIX)
        .map_or((fileName, ""), |base| {
            (base, operit_store::FileSyncPolicy::REFERENCE_SUFFIX)
        });
    let base = base.trim_end_matches([' ', '.']);
    let extension = base
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .filter(|extension| {
            !extension.is_empty()
                && extension.len() <= 16
                && extension.bytes().all(|value| value.is_ascii_alphanumeric())
        });
    Ok(match extension {
        Some(extension) => format!("attachment.{extension}{suffix}"),
        None => format!("attachment{suffix}"),
    })
}

/// Rejects invalid declared lengths before starting a Host write.
fn validateLength(length: Option<i64>) -> Result<(), String> {
    if length.is_some_and(|length| length < 0) {
        return Err("Attachment byte length must not be negative".to_string());
    }
    Ok(())
}

fn sourceAttachmentPath(id: &str, name: &str) -> Result<String, String> {
    let name = attachmentStorageFileName(name)?;
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
    let fileName = attachmentStorageFileName(fileName)?;
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

    #[test]
    fn portable_attachment_names_are_preserved() {
        for name in [
            "需求文档.pdf",
            "image.jpg",
            ".env",
            "a & b=1.txt",
            "COM10.txt",
        ] {
            assert_eq!(attachmentStorageFileName(name).unwrap(), name);
        }
        let boundary = format!("{}.jpg", "a".repeat(ATTACHMENT_FILE_NAME_MAX_BYTES - 4));
        assert_eq!(attachmentStorageFileName(&boundary).unwrap(), boundary);
    }

    #[test]
    fn nonportable_attachment_names_use_a_bounded_idempotent_fallback() {
        for (name, expected) in [
            ("_cgi-bin_mmwebwx-bin_webwxgetmsgimg??&MsgID=123&skey=example&mmweb_appid=wx_webfilehelper.jpg", "attachment.jpg"),
            ("photo?.jpg", "attachment.jpg"),
            ("photo*.PNG", "attachment.PNG"),
            ("report<draft>.pdf", "attachment.pdf"),
            ("report|draft.txt", "attachment.txt"),
            ("report\"draft.txt", "attachment.txt"),
            ("photo.jpg ", "attachment.jpg"),
            ("photo.jpg.", "attachment.jpg"),
            ("CON", "attachment"),
            ("nul.txt", "attachment.txt"),
            ("PrN .pdf", "attachment.pdf"),
            ("AUX.png", "attachment.png"),
            ("COM1.txt", "attachment.txt"),
            ("LPT9.txt", "attachment.txt"),
            ("COM¹.txt", "attachment.txt"),
            ("LPT².txt", "attachment.txt"),
            ("COM³.txt", "attachment.txt"),
            ("CONIN$", "attachment"),
            ("CONOUT$", "attachment"),
            ("bad?.invalid-extension", "attachment"),
        ] {
            let actual = attachmentStorageFileName(name).unwrap();
            assert_eq!(actual, expected, "{name:?}");
            assert_eq!(attachmentStorageFileName(&actual).unwrap(), actual);
        }
        for name in [
            format!("{}.pdf", "a".repeat(181)),
            format!("{}.jpg", "图".repeat(90)),
        ] {
            let actual = attachmentStorageFileName(&name).unwrap();
            assert!(actual.starts_with("attachment."));
            assert!(actual.len() <= ATTACHMENT_FILE_NAME_MAX_BYTES);
        }
        let id = Uuid::new_v4().simple().to_string();
        assert!(sourceAttachmentPath(&id, "photo?.jpg")
            .unwrap()
            .ends_with("/attachment.jpg"));
        let other = Uuid::new_v4().simple().to_string();
        assert_ne!(
            sourceAttachmentPath(&id, "photo?.jpg").unwrap(),
            sourceAttachmentPath(&other, "photo?.jpg").unwrap()
        );
    }

    #[test]
    fn fallback_preserves_file_reference_suffixes() {
        let suffix = operit_store::FileSyncPolicy::REFERENCE_SUFFIX;
        for name in ["photo?.jpg".to_string(), format!("{}.pdf", "a".repeat(170))] {
            let actual = attachmentStorageFileName(&format!("{name}{suffix}")).unwrap();
            assert!(actual.starts_with("attachment."));
            assert!(actual.ends_with(suffix));
            assert!(actual.len() <= ATTACHMENT_FILE_NAME_MAX_BYTES);
            assert_eq!(attachmentStorageFileName(&actual).unwrap(), actual);
        }
    }

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
