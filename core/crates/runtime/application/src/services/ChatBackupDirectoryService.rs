#![allow(non_snake_case)]

use operit_host_api::HostManager::HostManager;
use operit_host_api::RuntimeStorageHost;
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::sync::SqlChatSyncStore::SqlChatSyncStore;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_store::SqliteStore::SqliteStoreError;
use operit_vfs::MountRegistry::MountRegistry;
use operit_vfs::VisualFileSystem::VisualFileSystem;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Identifies a database backup by its filename and timestamp.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatBackupSnapshot {
    pub name: String,
    pub createdAt: i64,
}

/// Copies database files through the selected folder's filesystem Host.
#[derive(Clone)]
pub struct ChatBackupDirectoryService {
    storage: Arc<dyn RuntimeStorageHost>,
    files: VisualFileSystem,
}

impl ChatBackupDirectoryService {
    /// Binds directory operations to the local runtime's storage and VFS.
    pub fn getInstance(host: &HostManager) -> Result<Self, String> {
        Ok(Self {
            storage: host
                .runtimeStorageHost
                .clone()
                .ok_or("Backup storage Host is unavailable")?,
            files: host.vfs()?,
        })
    }

    /// Checks that the selected folder is still accessible without writing probe files.
    pub fn checkDirectory(&self, directory: String) -> Result<(), String> {
        self.files.listFiles(&self.directory(&directory)?)?;
        Ok(())
    }

    /// Closes the database, copies its file directly to the selected folder, and reopens it.
    pub fn createBackup(
        &self,
        directory: String,
        name: String,
    ) -> Result<ChatBackupSnapshot, String> {
        validateName(&name)?;
        let info = snapshot(name.clone())?;
        let destination = format!("{}/{}", self.directory(&directory)?, name);
        if self.files.fileExists(&destination)?.exists {
            return Err("Backup filename already exists".into());
        }
        let database = AppDatabase::default().map_err(|e| e.to_string())?;
        let path = RuntimeStorePaths::default().sqlite_database_path();
        let result = database.store().withDatabaseFile(|| {
            self.files
                .copyFile(&path.to_string_lossy(), &destination, false)
                .map_err(SqliteStoreError::Message)
        });
        if let Err(error) = result {
            if self.files.fileExists(&destination)?.exists {
                self.files.deleteFile(&destination, false)?;
            }
            return Err(error.to_string());
        }
        Ok(info)
    }

    /// Lists direct files without opening or scanning any database.
    pub fn listBackups(&self, directory: String) -> Result<Vec<String>, String> {
        Ok(self
            .files
            .listFiles(&self.directory(&directory)?)?
            .into_iter()
            .filter(|entry| !entry.isDirectory)
            .map(|entry| entry.name)
            .collect())
    }

    /// Copies beside the database and atomically renames it over the closed live file.
    pub fn restoreBackup(
        &self,
        directory: String,
        name: String,
    ) -> Result<ChatBackupSnapshot, String> {
        validateName(&name)?;
        let info = snapshot(name.clone())?;
        let source = format!("{}/{}", self.directory(&directory)?, name);
        let database = AppDatabase::default().map_err(|e| e.to_string())?;
        let previous = SqlChatSyncStore::default()
            .map_err(|e| e.to_string())?
            .restoration()
            .map_err(|e| e.to_string())?;
        let path = RuntimeStorePaths::default().sqlite_database_path();
        let path = path.to_string_lossy();
        let temporary = format!("{path}.restore-{}", uuid::Uuid::new_v4().simple());
        let result = (|| {
            self.files.copyFile(&source, &temporary, false)?;
            database
                .restoreDatabaseFile(|| {
                    self.files
                        .moveFile(&temporary, &path)
                        .map_err(SqliteStoreError::Message)
                })
                .map_err(|e| e.to_string())
        })();
        if self.files.fileExists(&temporary)?.exists {
            self.files.deleteFile(&temporary, false)?;
        }
        result?;
        SqlChatSyncStore::default()
            .map_err(|e| e.to_string())?
            .establishRestoreBaseline(previous)
            .map_err(|e| e.to_string())?;
        Ok(info)
    }

    /// Removes one backup file selected by the retention policy.
    pub fn removeBackup(&self, directory: String, name: String) -> Result<(), String> {
        validateName(&name)?;
        self.files
            .deleteFile(&format!("{}/{}", self.directory(&directory)?, name), false)
    }

    /// Resolves an existing external mount outside live application storage.
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

/// Restricts file operations to direct backup filenames.
fn validateName(name: &str) -> Result<(), String> {
    if !name.starts_with("operit-chats-")
        || !name.ends_with(".sqlite")
        || name.chars().any(|value| matches!(value, '/' | '\\' | '\0'))
    {
        return Err("Invalid chat backup filename".into());
    }
    Ok(())
}

/// Reads the creation timestamp from the backup filename without accessing its contents.
fn snapshot(name: String) -> Result<ChatBackupSnapshot, String> {
    let createdAt = name
        .strip_suffix(".sqlite")
        .and_then(|stem| stem.rsplit('-').next())
        .ok_or("Backup timestamp is missing")?
        .parse::<i64>()
        .map_err(|e| e.to_string())?;
    Ok(ChatBackupSnapshot { name, createdAt })
}
