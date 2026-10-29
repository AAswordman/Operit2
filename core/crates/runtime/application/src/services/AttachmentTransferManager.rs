#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_api::{FileSystemHost, RuntimeStorageHost, RuntimeStorageWriteHost};
use operit_model::AttachmentInfo::AttachmentInfo;
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_util::stream::ReverseStream::ReverseStream;
use operit_util::{
    OperitPaths,
    RuntimeStorageLayout::{
        RUNTIME_CHAT_ATTACHMENTS_DIR_PATH, RUNTIME_CHAT_ATTACHMENT_UPLOADS_DIR_PATH,
    },
};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::ChatServiceCore::getMimeTypeFromPath;

const ATTACHMENT_UPLOAD_MAX_CHUNK_BYTES: usize = 1024 * 1024;

/// Receives selected files into cleanable Host staging storage with bounded memory.
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

    /// Writes ordered chunks into one private Host write session and publishes only at EOF.
    pub async fn writeAttachmentUpload(
        &self,
        uploadId: String,
        fileName: String,
        expectedByteLength: Option<i64>,
        mut bytes: ReverseStream<Vec<u8>>,
    ) -> Result<(), String> {
        validateLength(expectedByteLength)?;
        let storagePath = attachmentStoragePath(&uploadId, &fileName)?;
        let mut writer = self
            .storageWriteHost
            .createWriteSession(&storagePath)
            .map_err(|error| error.to_string())?;
        let result = async {
            let mut received = 0i64;
            while let Some(chunk) = bytes.recv().await {
                if chunk.len() > ATTACHMENT_UPLOAD_MAX_CHUNK_BYTES {
                    return Err(format!("Attachment chunk exceeds {ATTACHMENT_UPLOAD_MAX_CHUNK_BYTES} bytes"));
                }
                received = received.checked_add(chunk.len() as i64)
                    .ok_or("Attachment byte length overflow")?;
                if expectedByteLength.is_some_and(|expected| received > expected) {
                    return Err("Attachment exceeds its declared byte length".to_string());
                }
                writer.writeChunk(&chunk).map_err(|error| error.to_string())?;
            }
            if expectedByteLength.is_some_and(|expected| received != expected) {
                return Err(format!("Attachment byte length mismatch: expected {expectedByteLength:?}, got {received}"));
            }
            Ok(())
        }.await;
        match result {
            Ok(()) => writer.commit().map_err(|error| error.to_string()),
            Err(error) => {
                if let Err(cleanup) = writer.discard() {
                    operit_host_api::logHostError("AttachmentUpload", &cleanup.to_string());
                }
                Err(error)
            }
        }
    }

    /// Returns verified file metadata after the reverse stream has committed successfully.
    pub fn completeAttachmentUpload(
        &self,
        uploadId: String,
        fileName: String,
        expectedByteLength: Option<i64>,
    ) -> Result<AttachmentInfo, String> {
        validateLength(expectedByteLength)?;
        let storagePath = attachmentStoragePath(&uploadId, &fileName)?;
        let runtimeRoot = self
            .storageHost
            .runtimeRootDir()
            .ok_or("Attachment Host does not expose its runtime storage root")?;
        let path = OperitPaths::runtimePathFromRoot(&runtimeRoot, &storagePath)?;
        let stored = self
            .fileSystemHost
            .fileExists(&path.to_string_lossy())
            .map_err(|error| error.to_string())?;
        if !stored.exists || stored.isDirectory || stored.size < 0 {
            return Err("Uploaded attachment is not a committed file".to_string());
        }
        if expectedByteLength.is_some_and(|expected| expected != stored.size) {
            return Err("Uploaded attachment byte length mismatch".to_string());
        }
        let noMediaMarker = path
            .parent()
            .ok_or("Uploaded attachment path has no parent directory")?
            .join(".nomedia");
        self.fileSystemHost
            .writeFile(&noMediaMarker.to_string_lossy(), "", false)
            .map_err(|error| error.to_string())?;
        Ok(AttachmentInfo {
            nodeId: CoreNodeIdentityStore::new(self.storageHost.clone())
                .identity()
                .ok()
                .map(|identity| identity.nodeId),
            mimeType: getMimeTypeFromPath(std::path::Path::new(&fileName)).to_string(),
            fileName,
            filePath: path.to_string_lossy().into_owned(),
            fileSize: stored.size,
            content: String::new(),
        })
    }

    /// Removes a completed but unattached upload after cancellation or registration failure.
    pub fn discardAttachmentUpload(
        &self,
        uploadId: String,
        fileName: String,
    ) -> Result<(), String> {
        let file = attachmentStoragePath(&uploadId, &fileName)?;
        let (path, _) = file
            .rsplit_once('/')
            .ok_or("Attachment upload path has no parent")?;
        if self
            .storageHost
            .exists(path)
            .map_err(|error| error.to_string())?
        {
            self.storageHost
                .delete(path, true)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
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

/// Keeps unregistered uploads isolated from persistent chat attachments.
fn attachmentStoragePath(uploadId: &str, fileName: &str) -> Result<String, String> {
    validateAttachmentFileName(fileName)?;
    let id = Uuid::parse_str(uploadId).map_err(|_| "Invalid attachment upload identity")?;
    Ok(format!(
        "{RUNTIME_CHAT_ATTACHMENT_UPLOADS_DIR_PATH}/{}/{}",
        id.simple(),
        fileName
    ))
}

/// Builds one attachment destination without allowing a chat id to escape its directory.
fn chatAttachmentStoragePath(
    chatId: &str,
    attachmentId: &str,
    fileName: &str,
) -> Result<String, String> {
    validateAttachmentFileName(chatId)?;
    validateAttachmentFileName(fileName)?;
    let id = Uuid::parse_str(attachmentId).map_err(|_| "Invalid attachment identity")?;
    Ok(format!(
        "{RUNTIME_CHAT_ATTACHMENTS_DIR_PATH}/{chatId}/{}/{fileName}",
        id.simple()
    ))
}

/// Prepares a dedicated directory while retaining the original file name.
pub(crate) fn prepareChatAttachmentPath(
    fileSystem: &dyn FileSystemHost,
    chatId: &str,
    attachmentId: &str,
    fileName: &str,
) -> Result<PathBuf, String> {
    let storagePath = chatAttachmentStoragePath(chatId, attachmentId, fileName)?;
    let path = OperitPaths::runtimePathFromRoot(&OperitPaths::operitRootDir()?, &storagePath)?;
    if fileSystem
        .fileExists(&path.to_string_lossy())
        .map_err(|error| error.to_string())?
        .exists
    {
        return Err("An existing attachment must not be overwritten".into());
    }
    let directory = path
        .parent()
        .ok_or("Attachment path has no parent directory")?;
    fileSystem
        .makeDirectory(&directory.to_string_lossy(), true)
        .map_err(|error| error.to_string())?;
    let marker = directory.join(".nomedia");
    fileSystem
        .writeFile(&marker.to_string_lossy(), "", false)
        .map_err(|error| error.to_string())?;
    Ok(path)
}

/// Promotes only an exact committed upload into its receiving chat's attachment directory.
pub(crate) fn adoptUploadedAttachment(
    fileSystem: &dyn FileSystemHost,
    attachment: &mut AttachmentInfo,
    chatId: &str,
) -> Result<(), String> {
    let uploadRoot = OperitPaths::chatAttachmentUploadsDir()?;
    let relative = Path::new(&attachment.filePath)
        .strip_prefix(&uploadRoot)
        .map_err(|_| "Uploaded attachment is outside the staging directory")?;
    let segments = relative.components().collect::<Vec<_>>();
    let [std::path::Component::Normal(id), std::path::Component::Normal(name)] =
        segments.as_slice()
    else {
        return Err("Uploaded attachment must name one staged file".into());
    };
    if name.to_str() != Some(attachment.fileName.as_str()) {
        return Err("Uploaded attachment file name does not match its metadata".into());
    }
    let id = id.to_str().ok_or("Invalid attachment upload identity")?;
    let destination = prepareChatAttachmentPath(fileSystem, chatId, id, &attachment.fileName)?;
    fileSystem
        .moveFile(&attachment.filePath, &destination.to_string_lossy())
        .map_err(|error| error.to_string())?;
    let sourceDirectory = Path::new(&attachment.filePath)
        .parent()
        .ok_or("Attachment upload has no parent")?;
    if let Err(error) = fileSystem.deleteFile(&sourceDirectory.to_string_lossy(), true) {
        operit_host_api::logHostError("AttachmentUpload", &error.to_string());
    }
    attachment.filePath = destination.to_string_lossy().into_owned();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keeps uploads transient and rejects attempts to traverse out of staging.
    #[test]
    fn upload_paths_are_transient_and_cannot_escape_the_attachment_directory() {
        let id = Uuid::new_v4().simple().to_string();
        assert!(attachmentStoragePath(&id, "文档.apk")
            .unwrap()
            .starts_with("runtime/temp/clean_on_exit/chat_attachment_uploads/"));
        for name in [
            "", " ", ".", "..", ".nomedia", "../bad", "a/b", "a\\b", "a\0b", "C:bad",
        ] {
            assert!(attachmentStoragePath(&id, name).is_err(), "{name:?}");
        }
        assert!(attachmentStoragePath("../bad", "file.apk").is_err());
        assert!(validateLength(Some(-1)).is_err());
        assert!(validateLength(None).is_ok());
        assert!(validateLength(Some(0)).is_ok());
    }

    /// Isolates duplicate names by chat and attachment identity while keeping original names.
    #[test]
    fn persistent_paths_are_grouped_and_cannot_escape_the_chat_directory() {
        let id = Uuid::new_v4().simple().to_string();
        let path = chatAttachmentStoragePath("chat-a", &id, "report.pdf").unwrap();
        assert_eq!(
            path,
            format!("runtime/data/chat_attachments/chat-a/{id}/report.pdf")
        );
        assert_ne!(
            path,
            chatAttachmentStoragePath("chat-b", &id, "report.pdf").unwrap()
        );
        assert_ne!(
            path,
            chatAttachmentStoragePath("chat-a", &Uuid::new_v4().to_string(), "report.pdf").unwrap()
        );
        for chat in ["", ".", "..", "../other-chat", "a/b", "a\\b", "C:chat"] {
            assert!(
                chatAttachmentStoragePath(chat, &id, "report.pdf").is_err(),
                "{chat:?}"
            );
        }
        assert!(chatAttachmentStoragePath("chat-a", "../invalid", "report.pdf").is_err());
        assert!(chatAttachmentStoragePath("chat-a", &id, "../report.pdf").is_err());
    }
}
