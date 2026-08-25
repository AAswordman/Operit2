use std::sync::Arc;

use operit_host_api::{
    HostManager::HostManager, RuntimeSqliteHost, RuntimeStorageHost, RuntimeStorageWriteHost,
};
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use crate::data::backup::Operit1SnapshotImportManager::{
    observeOperit1SnapshotImportProgress, publishOperit1SnapshotImportProgress,
    Operit1ModelConfigImportResult, Operit1ModelConfigSnapshotPreview,
    Operit1SnapshotImportManager, Operit1SnapshotImportProgress, Operit1SnapshotImportResult,
    Operit1SnapshotPreview,
};
use crate::data::backup::RawSnapshotBackupManager::{
    RawSnapshotBackupManager, RawSnapshotManifest,
};
use crate::services::ArchiveTransferManager::{ArchiveTransferManager, StagedArchive};

/// Applies snapshot-specific parsing and restore operations to a staged archive.
#[derive(Clone)]
pub struct SnapshotImportManager {
    archiveTransferManager: ArchiveTransferManager,
    storageHost: Arc<dyn RuntimeStorageHost>,
    storageWriteHost: Arc<dyn RuntimeStorageWriteHost>,
    sqliteHost: Arc<dyn RuntimeSqliteHost>,
}

impl SnapshotImportManager {
    /// Creates a snapshot importer bound to the runtime owner's archive transfer service.
    #[allow(non_snake_case)]
    pub fn getInstance(hostManager: &HostManager) -> Result<Self, String> {
        Ok(Self {
            archiveTransferManager: ArchiveTransferManager::getInstance(hostManager)?,
            storageHost: hostManager.runtimeStorageHost.clone().ok_or_else(|| {
                "RuntimeStorageHost is not registered for snapshot operations".to_string()
            })?,
            storageWriteHost: hostManager.runtimeStorageWriteHost.clone().ok_or_else(|| {
                "RuntimeStorageWriteHost is not registered for snapshot restore".to_string()
            })?,
            sqliteHost: hostManager.runtimeSqliteHost.clone().ok_or_else(|| {
                "RuntimeSqliteHost is not registered for snapshot operations".to_string()
            })?,
        })
    }

    /// Exports all raw runtime storage into a portable snapshot archive.
    #[allow(non_snake_case)]
    pub fn exportRawSnapshot(&self) -> Result<Vec<u8>, String> {
        RawSnapshotBackupManager::new(self.storageHost.clone(), self.storageWriteHost.clone())
            .exportSnapshot()
    }

    /// Reads raw snapshot metadata from a sealed archive without changing runtime storage.
    #[allow(non_snake_case)]
    pub fn inspectRawSnapshot(
        &self,
        archive: StagedArchive,
    ) -> Result<RawSnapshotManifest, String> {
        RawSnapshotBackupManager::new(self.storageHost.clone(), self.storageWriteHost.clone())
            .inspectSnapshotSource(self.archiveTransferManager.openStagedArchive(&archive)?)
    }

    /// Restores one sealed raw snapshot after closing the active database handle.
    #[allow(non_snake_case)]
    pub fn restoreRawSnapshot(&self, archive: StagedArchive) -> Result<(), String> {
        let source = self.archiveTransferManager.openStagedArchive(&archive)?;
        AppDatabase::closeDatabase();
        RawSnapshotBackupManager::new(self.storageHost.clone(), self.storageWriteHost.clone())
            .restoreSnapshotSource(source)
    }

    /// Previews an Operit1 model-configuration snapshot from a sealed archive.
    #[allow(non_snake_case)]
    pub fn inspectOperit1ModelConfigSnapshot(
        &self,
        archive: StagedArchive,
    ) -> Result<Operit1ModelConfigSnapshotPreview, String> {
        self.operit1Importer().inspectModelConfigSnapshotSource(
            self.archiveTransferManager.openStagedArchive(&archive)?,
        )
    }

    /// Previews an Operit1 full snapshot from a sealed archive.
    #[allow(non_snake_case)]
    pub fn inspectOperit1Snapshot(
        &self,
        archive: StagedArchive,
    ) -> Result<Operit1SnapshotPreview, String> {
        self.operit1Importer()
            .inspectSnapshotSource(self.archiveTransferManager.openStagedArchive(&archive)?)
    }

    /// Imports an Operit1 model configuration from a sealed archive into the selected profile.
    #[allow(non_snake_case)]
    pub fn importOperit1ModelConfigSnapshot(
        &self,
        archive: StagedArchive,
        configId: String,
        modelId: String,
    ) -> Result<Operit1ModelConfigImportResult, String> {
        self.operit1Importer().importModelConfigSnapshotSource(
            self.archiveTransferManager.openStagedArchive(&archive)?,
            configId,
            modelId,
        )
    }

    /// Imports one sealed Operit1 snapshot and publishes progress events.
    #[allow(non_snake_case)]
    pub fn importOperit1Snapshot(
        &self,
        archive: StagedArchive,
    ) -> Result<Operit1SnapshotImportResult, String> {
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress {
            stage: "parse".to_string(),
            title: "解析快照".to_string(),
            detail: "正在读取 Operit1 快照内容。".to_string(),
            progress: 0.04,
            active: true,
        });
        self.operit1Importer()
            .importSnapshotSource(self.archiveTransferManager.openStagedArchive(&archive)?)
    }

    /// Observes the latest Operit1 snapshot import progress state.
    #[allow(non_snake_case)]
    pub fn operit1SnapshotImportProgressFlow(
        &self,
    ) -> operit_store::PreferencesDataStore::StateFlow<Operit1SnapshotImportProgress> {
        observeOperit1SnapshotImportProgress()
    }

    /// Creates the portable Operit1 consumer over this runtime owner's storage hosts.
    fn operit1Importer(&self) -> Operit1SnapshotImportManager {
        Operit1SnapshotImportManager::new(
            RuntimeStorePaths::default(),
            self.storageHost.clone(),
            self.storageWriteHost.clone(),
            self.sqliteHost.clone(),
        )
    }
}
