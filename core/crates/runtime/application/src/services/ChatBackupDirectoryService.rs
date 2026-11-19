#![allow(non_snake_case)]

use operit_host_api::HostManager::HostManager;
use operit_host_api::RuntimeStorageHost;
use operit_store::ChatDatabaseSnapshot;
use operit_vfs::MountRegistry::MountRegistry;
use operit_vfs::VisualFileSystem::VisualFileSystem;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Identifies one verified standalone SQLite chat snapshot for the restore UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatBackupSnapshot {
    pub name: String,
    pub createdAt: i64,
    pub chatCount: i64,
    pub messageCount: i64,
}

/// Accesses explicitly selected backup folders through the owning filesystem Host.
#[derive(Clone)]
pub struct ChatBackupDirectoryService {
    storage: Arc<dyn RuntimeStorageHost>,
    files: VisualFileSystem,
}

impl ChatBackupDirectoryService {
    /// Binds directory operations to the same storage and VFS as the local runtime.
    pub fn getInstance(host: &HostManager) -> Result<Self, String> {
        Ok(Self {
            storage: host
                .runtimeStorageHost
                .clone()
                .ok_or("Backup storage Host is unavailable")?,
            files: host.vfs()?,
        })
    }

    /// Verifies the selected directory without recreating a missing or revoked destination.
    pub fn checkDirectory(&self, directory: String) -> Result<(), String> {
        let root = self.directory(&directory)?;
        self.files.listFiles(&root)?;
        let name = format!(".operit-backup-probe-{}", uuid::Uuid::new_v4().simple());
        let path = format!("{root}/{name}");
        self.files.writeFile(&path, &name, false)?;
        let checked = self.files.readFile(&path).and_then(|read| {
            if read == name {
                Ok(())
            } else {
                Err("Backup directory write verification failed".into())
            }
        });
        self.files.deleteFile(&path, false)?;
        checked
    }

    /// Captures SQLite pages and publishes their verified bytes through the external mount.
    pub fn createBackup(
        &self,
        directory: String,
        name: String,
    ) -> Result<Option<ChatBackupSnapshot>, String> {
        validateName(&name)?;
        let root = self.directory(&directory)?;
        let destination = format!("{root}/{name}");
        if self.files.fileExists(&destination)?.exists {
            return Err("Backup filename already exists".into());
        }
        let temporary = format!(
            "{root}/.operit-backup-{}.tmp",
            uuid::Uuid::new_v4().simple()
        );
        let stage = stagingPath();
        let result = (|| {
            let createdAt = operit_host_api::TimeUtils::currentTimeMillis();
            let Some(info) = ChatDatabaseSnapshot::create(&stage, createdAt)? else {
                return Ok(None);
            };
            self.files.copyFile(&stage, &temporary, false)?;
            let expected = self.digest(&stage)?;
            if self.digest(&temporary)? != expected {
                return Err("Backup copy verification failed".into());
            }
            self.files.moveFile(&temporary, &destination)?;
            if self.digest(&destination)? != expected {
                return Err("Published backup verification failed".into());
            }
            Ok(Some(snapshot(name, info)))
        })();
        self.cleanup(&temporary)?;
        self.cleanup(&stage)?;
        result
    }

    /// Lists direct files; callers may only prune their exact source and backup naming scheme.
    pub fn listBackups(&self, directory: String) -> Result<Vec<String>, String> {
        let root = self.directory(&directory)?;
        Ok(self
            .files
            .listFiles(&root)?
            .into_iter()
            .filter(|entry| !entry.isDirectory)
            .map(|entry| entry.name)
            .collect())
    }

    /// Verifies a snapshot in isolated staging without interpreting conversation records.
    pub fn inspectBackup(
        &self,
        directory: String,
        name: String,
    ) -> Result<ChatBackupSnapshot, String> {
        validateName(&name)?;
        let root = self.directory(&directory)?;
        let stage = stagingPath();
        let result = (|| {
            self.files
                .copyFile(&format!("{root}/{name}"), &stage, false)?;
            ChatDatabaseSnapshot::inspect(&stage).map(|info| snapshot(name, info))
        })();
        self.cleanup(&stage)?;
        result
    }

    /// Restores only chat tables atomically while leaving other runtime data and the source intact.
    pub fn restoreBackup(
        &self,
        directory: String,
        name: String,
    ) -> Result<ChatBackupSnapshot, String> {
        validateName(&name)?;
        let root = self.directory(&directory)?;
        let stage = stagingPath();
        let result = (|| {
            self.files
                .copyFile(&format!("{root}/{name}"), &stage, false)?;
            ChatDatabaseSnapshot::restore(&stage).map(|info| snapshot(name, info))
        })();
        self.cleanup(&stage)?;
        result
    }

    /// Removes one owned backup filename while rejecting traversal and unrelated files.
    pub fn removeBackup(&self, directory: String, name: String) -> Result<(), String> {
        validateName(&name)?;
        let root = self.directory(&directory)?;
        self.files.deleteFile(&format!("{root}/{name}"), false)
    }

    /// Hashes bounded file ranges to verify physical copies without materializing the database.
    fn digest(&self, path: &str) -> Result<Vec<u8>, String> {
        let size = u64::try_from(self.files.fileExists(path)?.size).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut offset = 0;
        while offset < size {
            let bytes = self.files.readFileBytesRange(
                path,
                offset,
                (size - offset).min(64 * 1024) as usize,
            )?;
            if bytes.is_empty() {
                return Err("Unexpected end of database snapshot".into());
            }
            offset += bytes.len() as u64;
            hash.update(bytes);
        }
        Ok(hash.finalize().to_vec())
    }

    /// Removes an operation-owned temporary file after closing all SQLite handles.
    fn cleanup(&self, path: &str) -> Result<(), String> {
        if self.files.fileExists(path)?.exists {
            self.files.deleteFile(path, false)?;
        }
        Ok(())
    }

    /// Resolves an existing user mount without registering sources or interpreting platform paths.
    fn directory(&self, directory: &str) -> Result<String, String> {
        let mount = MountRegistry::withStorage(self.storage.clone())
            .list()?
            .into_iter()
            .find(|mount| mount.vfsPath() == directory)
            .ok_or("Select an external folder and mount it as the backup directory")?;
        let root = mount.vfsPath();
        for live in ["/operitapp/data", "/operitapp/workspaces"] {
            if self.files.isWithin(live, &root)? {
                return Err(
                    "Choose a backup directory outside application data and workspaces".into(),
                );
            }
        }
        let metadata = self.files.fileExists(&root)?;
        if !metadata.exists || !metadata.isDirectory {
            return Err("The selected backup directory no longer exists".into());
        }
        Ok(root)
    }
}

/// Limits deletion and publication to direct chat database snapshot filenames.
fn validateName(name: &str) -> Result<(), String> {
    if !name.starts_with("operit-chats-")
        || !name.ends_with(".sqlite")
        || name.chars().any(|value| matches!(value, '/' | '\\' | '\0'))
    {
        return Err("Invalid chat backup filename".into());
    }
    Ok(())
}

/// Allocates isolated runtime-owned staging for a physical SQLite image.
fn stagingPath() -> String {
    format!(
        "/operitapp/data/temp/clean_on_exit/chat_backups/{}.sqlite",
        uuid::Uuid::new_v4().simple()
    )
}

/// Projects database header metadata into a lightweight restore-list entry.
fn snapshot(
    name: String,
    info: ChatDatabaseSnapshot::ChatDatabaseSnapshotInfo,
) -> ChatBackupSnapshot {
    ChatBackupSnapshot {
        name,
        createdAt: info.createdAt,
        chatCount: info.chatCount,
        messageCount: info.messageCount,
    }
}
