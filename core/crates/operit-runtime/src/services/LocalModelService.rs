#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use operit_host_api::HostManager::HostManager;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_host_api::{HttpDownloadControl, HttpHost, RuntimeStorageHost};
use operit_local_models::LocalEngineCatalog::LocalEngineCatalog;
use operit_local_models::LocalEngineDownload::{
    LocalEngineDownloadProgress, LocalEngineDownloadProgressCallback, LocalEngineInstallRequest,
    LocalEngineInstaller,
};
use operit_local_models::LocalEngineManifest::{
    LocalEngineDelivery, LocalEngineManifest, LocalPlatformTarget,
};
use operit_local_models::LocalModelCatalog::LocalModelCatalog;
use operit_local_models::LocalModelDownload::{
    LocalModelDownloadProgress, LocalModelDownloadProgressCallback, LocalModelInstallRequest,
    LocalModelInstaller,
};
use operit_local_models::LocalModelManifest::LocalModelManifest;
use operit_local_models::LocalModelRegistry::{
    InstalledLocalEngine, InstalledLocalModel, LocalModelRegistrySnapshot,
};
use operit_local_models::LocalModelRegistryStore::LocalModelRegistryStore;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelCatalogStatus {
    pub manifest: LocalModelManifest,
    pub installedModel: Option<InstalledLocalModel>,
    pub engineManifest: Option<LocalEngineManifest>,
    pub installedEngine: Option<InstalledLocalEngine>,
    pub platformCompatible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelBundleInstallResult {
    pub installedModel: InstalledLocalModel,
    pub installedEngine: InstalledLocalEngine,
    pub modelDownloadedBytes: u64,
    pub engineDownloadedBytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalModelInstallPhase {
    Preparing,
    Engine,
    Model,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelInstallStatus {
    pub operationId: String,
    pub modelId: String,
    pub version: String,
    pub phase: LocalModelInstallPhase,
    pub currentFile: Option<String>,
    pub downloadedBytes: u64,
    pub totalBytes: u64,
    pub error: Option<String>,
}

#[derive(Clone)]
struct LocalModelInstallOperation {
    status: LocalModelInstallStatus,
    engineControl: HttpDownloadControl,
    modelControl: HttpDownloadControl,
}

static LOCAL_MODEL_INSTALL_OPERATIONS: OnceLock<
    Mutex<BTreeMap<String, LocalModelInstallOperation>>,
> = OnceLock::new();

#[derive(Clone)]
/// Manages the local model and engine asset repository.
pub struct LocalModelService {
    runtimeRoot: PathBuf,
    runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
    httpHost: Arc<dyn HttpHost>,
}

impl LocalModelService {
    /// Creates the local model service from application host context.
    pub fn getInstance(context: &HostManager) -> Result<Self, String> {
        let runtimeStorageHost = context
            .runtimeStorageHost
            .as_ref()
            .ok_or_else(|| "RuntimeStorageHost is required for local models".to_string())?;
        let runtimeRoot = runtimeStorageHost
            .runtimeRootDir()
            .ok_or_else(|| "RuntimeStorageHost runtime root is not configured".to_string())?;
        let httpHost = context
            .httpHost
            .as_ref()
            .ok_or_else(|| "HttpHost is required for local model downloads".to_string())?
            .clone();
        Ok(Self {
            runtimeRoot,
            runtimeStorageHost: runtimeStorageHost.clone(),
            httpHost,
        })
    }

    /// Returns model catalog entries with installed model and engine state.
    pub fn getCatalogStatus(&self) -> Result<Vec<LocalModelCatalogStatus>, String> {
        let target = LocalPlatformTarget::current()?;
        let registry = self.registryStore()?.read().map_err(errorString)?;
        let mut statuses = Vec::new();
        for manifest in LocalModelCatalog::manifests() {
            let installedModel = registry
                .getInstalledModel(&manifest.id, &manifest.version)
                .cloned();
            let engineManifest = self.engineManifestForModel(&manifest)?;
            let platformCompatible = manifest.supportsPlatform(&target.platform)
                && engineManifest
                    .as_ref()
                    .and_then(|engine| engine.artifactForTarget(&target))
                    .is_some();
            let installedEngine = match manifest.engineRequirement.as_ref() {
                Some(requirement) => registry
                    .getInstalledEngine(&requirement.engineId, &requirement.version, &target)
                    .filter(|installed| {
                        engineManifest
                            .as_ref()
                            .and_then(|engine| engine.artifactForTarget(&target))
                            == Some(&installed.artifact)
                    })
                    .cloned(),
                None => None,
            };
            statuses.push(LocalModelCatalogStatus {
                manifest,
                installedModel,
                engineManifest,
                installedEngine,
                platformCompatible,
            });
        }
        Ok(statuses)
    }

    /// Returns the shared local model and engine registry snapshot.
    pub fn getRegistry(&self) -> Result<LocalModelRegistrySnapshot, String> {
        self.registryStore()?.read().map_err(errorString)
    }

    /// Returns the current local engine platform target.
    pub fn getPlatformTarget(&self) -> Result<LocalPlatformTarget, String> {
        LocalPlatformTarget::current()
    }

    /// Installs one model and its exact platform engine dependency.
    pub fn installModel(
        &self,
        modelId: String,
        version: String,
    ) -> Result<LocalModelBundleInstallResult, String> {
        let manifest = self.catalogModel(&modelId, &version)?;
        self.requirePlatformCompatibleModel(&manifest)?;
        let requirement = manifest.engineRequirement.as_ref().ok_or_else(|| {
            format!(
                "local model engine requirement is missing: {}@{}",
                manifest.id, manifest.version
            )
        })?;
        let engineManifest = self.catalogEngine(&requirement.engineId, &requirement.version)?;
        let target = LocalPlatformTarget::current()?;
        let engineArtifact = engineManifest.artifactForTarget(&target).ok_or_else(|| {
            format!(
                "local engine artifact is unavailable for target: {}@{}#{}",
                engineManifest.id,
                engineManifest.version,
                target.storageSegment()
            )
        })?;
        let engineInstaller = self.engineInstaller()?;
        let currentRegistry = engineInstaller.readRegistry().map_err(errorString)?;
        let engineInstalled = currentRegistry
            .getInstalledEngine(&engineManifest.id, &engineManifest.version, &target)
            .map(|installed| installed.artifact == *engineArtifact)
            .unwrap_or(false);
        let engineDownloadBytes = match (&engineArtifact.delivery, engineInstalled) {
            (_, true) | (LocalEngineDelivery::Embedded, false) => 0,
            (LocalEngineDelivery::DownloadArchive, false) => engineArtifact.byteSize,
        };
        let totalBytes = engineDownloadBytes
            .checked_add(manifest.declaredByteSize())
            .ok_or_else(|| {
                format!(
                    "local model installation byte total overflowed: {}",
                    manifest.registryKey()
                )
            })?;
        let operationId = installOperationId(&manifest.id, &manifest.version);
        let (engineControl, modelControl) = registerInstallOperation(
            operationId.clone(),
            manifest.id.clone(),
            manifest.version.clone(),
            totalBytes,
        )?;
        let result = self.performInstall(
            manifest,
            engineManifest,
            target,
            engineInstaller,
            engineInstalled,
            operationId.clone(),
            engineControl,
            modelControl,
        );
        match result {
            Ok(result) => {
                completeInstallOperation(&operationId, &result)?;
                Ok(result)
            }
            Err(error) => {
                failInstallOperation(&operationId, &error)?;
                Err(error)
            }
        }
    }

    /// Returns every installation operation retained by this runtime process.
    pub fn getInstallStatuses(&self) -> Result<Vec<LocalModelInstallStatus>, String> {
        let operations = installOperations()
            .lock()
            .map_err(|error| format!("local model install operation lock poisoned: {error}"))?;
        Ok(operations
            .values()
            .map(|operation| operation.status.clone())
            .collect())
    }

    /// Returns one installation operation by exact model id and version.
    pub fn getInstallStatus(
        &self,
        modelId: String,
        version: String,
    ) -> Result<Option<LocalModelInstallStatus>, String> {
        let operationId = installOperationId(modelId.trim(), version.trim());
        let operations = installOperations()
            .lock()
            .map_err(|error| format!("local model install operation lock poisoned: {error}"))?;
        Ok(operations
            .get(&operationId)
            .map(|operation| operation.status.clone()))
    }

    /// Requests cancellation for one active model and engine installation.
    pub fn cancelInstall(
        &self,
        modelId: String,
        version: String,
    ) -> Result<LocalModelInstallStatus, String> {
        let operationId = installOperationId(modelId.trim(), version.trim());
        let mut operations = installOperations()
            .lock()
            .map_err(|error| format!("local model install operation lock poisoned: {error}"))?;
        let operation = operations
            .get_mut(&operationId)
            .ok_or_else(|| format!("local model install operation not found: {operationId}"))?;
        if isTerminalInstallPhase(&operation.status.phase) {
            return Err(format!(
                "local model install operation is not active: {operationId}"
            ));
        }
        operation.engineControl.cancel();
        operation.modelControl.cancel();
        operation.status.phase = LocalModelInstallPhase::Cancelling;
        Ok(operation.status.clone())
    }

    /// Performs one registered model bundle installation.
    fn performInstall(
        &self,
        manifest: LocalModelManifest,
        engineManifest: LocalEngineManifest,
        target: LocalPlatformTarget,
        engineInstaller: LocalEngineInstaller,
        engineInstalled: bool,
        operationId: String,
        engineControl: HttpDownloadControl,
        modelControl: HttpDownloadControl,
    ) -> Result<LocalModelBundleInstallResult, String> {
        let (installedEngine, engineDownloadedBytes) = match engineInstalled {
            true => (
                engineInstaller
                    .verifyInstalledEngine(
                        &engineManifest.id,
                        &engineManifest.version,
                        &target,
                        currentTimeMillis(),
                    )
                    .map_err(errorString)?,
                0,
            ),
            false => {
                let result = engineInstaller
                    .install(
                        LocalEngineInstallRequest {
                            manifest: engineManifest,
                            target,
                            installedAtMs: currentTimeMillis(),
                        },
                        engineControl.clone(),
                        engineInstallProgressCallback(operationId.clone()),
                    )
                    .map_err(errorString)?;
                (result.installedEngine, result.downloadedBytes)
            }
        };
        if engineControl.isCancelled() || modelControl.isCancelled() {
            return Err(format!("local model installation cancelled: {operationId}"));
        }
        let modelResult = self.modelInstaller()?.install(
            LocalModelInstallRequest {
                manifest,
                installedAtMs: currentTimeMillis(),
            },
            modelControl,
            modelInstallProgressCallback(operationId, engineDownloadedBytes),
        );
        let modelResult = modelResult.map_err(errorString)?;
        Ok(LocalModelBundleInstallResult {
            installedModel: modelResult.installedModel,
            installedEngine,
            modelDownloadedBytes: modelResult.downloadedBytes,
            engineDownloadedBytes,
        })
    }

    /// Verifies one installed model and its exact platform engine.
    pub fn verifyModel(
        &self,
        modelId: String,
        version: String,
    ) -> Result<LocalModelCatalogStatus, String> {
        let manifest = self.catalogModel(&modelId, &version)?;
        self.requirePlatformCompatibleModel(&manifest)?;
        self.modelInstaller()?
            .verifyInstalledModel(&modelId, &version, currentTimeMillis())
            .map_err(errorString)?;
        let requirement = manifest.engineRequirement.as_ref().ok_or_else(|| {
            format!("local model engine requirement is missing: {modelId}@{version}")
        })?;
        self.engineInstaller()?
            .verifyInstalledEngine(
                &requirement.engineId,
                &requirement.version,
                &LocalPlatformTarget::current()?,
                currentTimeMillis(),
            )
            .map_err(errorString)?;
        self.catalogStatus(&modelId, &version)
    }

    /// Deletes one installed or retained local model download from the asset repository.
    pub fn deleteModel(&self, modelId: String, version: String) -> Result<(), String> {
        let operationId = installOperationId(&modelId, &version);
        removeTerminalInstallOperation(&operationId)?;
        let manifest = self.catalogModel(&modelId, &version)?;
        let target = LocalPlatformTarget::current()?;
        let modelInstaller = self.modelInstaller()?;
        let registry = modelInstaller.readRegistry().map_err(errorString)?;
        if registry.getInstalledModel(&modelId, &version).is_some() {
            modelInstaller
                .deleteInstalledModel(&modelId, &version)
                .map_err(errorString)?;
        } else {
            modelInstaller
                .removePendingInstallArtifacts(&manifest)
                .map_err(errorString)?;
        }
        if let Some(requirement) = manifest.engineRequirement.as_ref() {
            if registry
                .getInstalledEngine(&requirement.engineId, &requirement.version, &target)
                .is_none()
            {
                let engineManifest =
                    self.catalogEngine(&requirement.engineId, &requirement.version)?;
                self.engineInstaller()?
                    .removePendingInstallArtifacts(&engineManifest, &target)
                    .map_err(errorString)?;
            }
        }
        Ok(())
    }

    /// Deletes one engine target that is not required by installed models.
    pub fn deleteEngine(&self, engineId: String, version: String) -> Result<(), String> {
        let target = LocalPlatformTarget::current()?;
        let registry = self.modelInstaller()?.readRegistry().map_err(errorString)?;
        let dependentModels = registry
            .installedModels
            .iter()
            .filter(|model| match model.manifest.engineRequirement.as_ref() {
                Some(requirement) => {
                    requirement.engineId == engineId.trim() && requirement.version == version.trim()
                }
                None => false,
            })
            .map(|model| model.manifest.registryKey())
            .collect::<Vec<_>>();
        if !dependentModels.is_empty() {
            return Err(format!(
                "local engine is required by installed models: {}",
                dependentModels.join(", ")
            ));
        }
        self.engineInstaller()?
            .deleteInstalledEngine(&engineId, &version, &target)
            .map_err(errorString)?;
        Ok(())
    }

    /// Returns one catalog status by exact model id and version.
    fn catalogStatus(
        &self,
        modelId: &str,
        version: &str,
    ) -> Result<LocalModelCatalogStatus, String> {
        self.getCatalogStatus()?
            .into_iter()
            .find(|status| status.manifest.id == modelId && status.manifest.version == version)
            .ok_or_else(|| format!("local model catalog entry not found: {modelId}@{version}"))
    }

    /// Returns one model manifest from the built-in catalog.
    fn catalogModel(&self, modelId: &str, version: &str) -> Result<LocalModelManifest, String> {
        LocalModelCatalog::manifests()
            .into_iter()
            .find(|manifest| manifest.id == modelId.trim() && manifest.version == version.trim())
            .ok_or_else(|| format!("local model catalog entry not found: {modelId}@{version}"))
    }

    /// Returns one engine manifest from the built-in catalog.
    fn catalogEngine(&self, engineId: &str, version: &str) -> Result<LocalEngineManifest, String> {
        LocalEngineCatalog::manifests()
            .into_iter()
            .find(|manifest| manifest.id == engineId.trim() && manifest.version == version.trim())
            .ok_or_else(|| format!("local engine catalog entry not found: {engineId}@{version}"))
    }

    /// Returns the engine catalog manifest referenced by one model.
    fn engineManifestForModel(
        &self,
        model: &LocalModelManifest,
    ) -> Result<Option<LocalEngineManifest>, String> {
        match model.engineRequirement.as_ref() {
            Some(requirement) => self
                .catalogEngine(&requirement.engineId, &requirement.version)
                .map(Some),
            None => Ok(None),
        }
    }

    /// Requires one catalog model to have a driver for the current platform.
    fn requirePlatformCompatibleModel(&self, model: &LocalModelManifest) -> Result<(), String> {
        let target = LocalPlatformTarget::current()?;
        if !model.supportsPlatform(&target.platform) {
            return Err(format!(
                "local model driver is unavailable for target: {}#{}",
                model.registryKey(),
                target.storageSegment()
            ));
        }
        Ok(())
    }

    /// Creates a model installer for this runtime root.
    fn modelInstaller(&self) -> Result<LocalModelInstaller, String> {
        LocalModelInstaller::forRuntimeStorage(
            self.runtimeStorageHost.clone(),
            self.httpHost.clone(),
        )
        .map_err(errorString)
    }

    /// Creates an engine installer for this runtime root.
    fn engineInstaller(&self) -> Result<LocalEngineInstaller, String> {
        LocalEngineInstaller::forRuntimeStorage(
            self.runtimeStorageHost.clone(),
            self.httpHost.clone(),
        )
        .map_err(errorString)
    }

    /// Creates the registry store for this runtime root.
    fn registryStore(&self) -> Result<LocalModelRegistryStore, String> {
        Ok(LocalModelRegistryStore::forRuntimeStorage(
            self.runtimeStorageHost.clone(),
        ))
    }
}

/// Converts one displayable error into a string.
fn errorString(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Returns the shared installation operation table.
fn installOperations() -> &'static Mutex<BTreeMap<String, LocalModelInstallOperation>> {
    LOCAL_MODEL_INSTALL_OPERATIONS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Builds the stable operation id for one exact model release.
fn installOperationId(modelId: &str, version: &str) -> String {
    format!("{}@{}", modelId.trim(), version.trim())
}

/// Returns whether one installation phase is terminal.
fn isTerminalInstallPhase(phase: &LocalModelInstallPhase) -> bool {
    matches!(
        phase,
        LocalModelInstallPhase::Cancelled
            | LocalModelInstallPhase::Completed
            | LocalModelInstallPhase::Failed
    )
}

/// Registers one active installation and returns its shared control tokens.
fn registerInstallOperation(
    operationId: String,
    modelId: String,
    version: String,
    totalBytes: u64,
) -> Result<(HttpDownloadControl, HttpDownloadControl), String> {
    let mut operations = installOperations()
        .lock()
        .map_err(|error| format!("local model install operation lock poisoned: {error}"))?;
    if let Some(operation) = operations.get(&operationId) {
        if !isTerminalInstallPhase(&operation.status.phase) {
            return Err(format!(
                "local model install operation is already active: {operationId}"
            ));
        }
    }
    let engineControl = HttpDownloadControl::new();
    let modelControl = HttpDownloadControl::new();
    operations.insert(
        operationId.clone(),
        LocalModelInstallOperation {
            status: LocalModelInstallStatus {
                operationId,
                modelId,
                version,
                phase: LocalModelInstallPhase::Preparing,
                currentFile: None,
                downloadedBytes: 0,
                totalBytes,
                error: None,
            },
            engineControl: engineControl.clone(),
            modelControl: modelControl.clone(),
        },
    );
    Ok((engineControl, modelControl))
}

/// Marks one installation as complete with its exact downloaded byte count.
fn completeInstallOperation(
    operationId: &str,
    result: &LocalModelBundleInstallResult,
) -> Result<(), String> {
    let mut operations = installOperations()
        .lock()
        .map_err(|error| format!("local model install operation lock poisoned: {error}"))?;
    let operation = operations
        .get_mut(operationId)
        .ok_or_else(|| format!("local model install operation not found: {operationId}"))?;
    operation.status.phase = LocalModelInstallPhase::Completed;
    operation.status.currentFile = None;
    operation.status.downloadedBytes = result
        .engineDownloadedBytes
        .checked_add(result.modelDownloadedBytes)
        .ok_or_else(|| format!("local model installed byte total overflowed: {operationId}"))?;
    operation.status.error = None;
    Ok(())
}

/// Marks one installation as cancelled or failed from its control state.
fn failInstallOperation(operationId: &str, errorMessage: &str) -> Result<(), String> {
    let mut operations = installOperations()
        .lock()
        .map_err(|error| format!("local model install operation lock poisoned: {error}"))?;
    let operation = operations
        .get_mut(operationId)
        .ok_or_else(|| format!("local model install operation not found: {operationId}"))?;
    let cancelled = operation.engineControl.isCancelled() || operation.modelControl.isCancelled();
    operation.status.phase = match cancelled {
        true => LocalModelInstallPhase::Cancelled,
        false => LocalModelInstallPhase::Failed,
    };
    operation.status.currentFile = None;
    operation.status.error = match cancelled {
        true => None,
        false => Some(errorMessage.to_string()),
    };
    Ok(())
}

/// Removes one terminal installation operation after its retained files are deleted.
fn removeTerminalInstallOperation(operationId: &str) -> Result<(), String> {
    let mut operations = installOperations()
        .lock()
        .map_err(|error| format!("local model install operation lock poisoned: {error}"))?;
    if let Some(operation) = operations.get(operationId) {
        if !isTerminalInstallPhase(&operation.status.phase) {
            return Err(format!(
                "local model installation must be cancelled before deletion: {operationId}"
            ));
        }
    }
    operations.remove(operationId);
    Ok(())
}

/// Updates one active installation from a concrete download progress event.
fn updateInstallProgress(
    operationId: &str,
    phase: LocalModelInstallPhase,
    currentFile: Option<String>,
    downloadedBytes: Option<u64>,
) {
    let mut operations = installOperations()
        .lock()
        .expect("local model install operation lock must remain available");
    let operation = operations
        .get_mut(operationId)
        .expect("local model install operation must exist while progress is emitted");
    if isTerminalInstallPhase(&operation.status.phase) {
        return;
    }
    if operation.status.phase != LocalModelInstallPhase::Cancelling {
        operation.status.phase = phase;
    }
    operation.status.currentFile = currentFile;
    if let Some(downloadedBytes) = downloadedBytes {
        operation.status.downloadedBytes = operation.status.downloadedBytes.max(downloadedBytes);
    }
}

/// Maps engine download progress into the shared bundle operation state.
fn engineInstallProgressCallback(operationId: String) -> LocalEngineDownloadProgressCallback {
    Arc::new(move |progress| match progress {
        LocalEngineDownloadProgress::Started { .. } => {
            updateInstallProgress(&operationId, LocalModelInstallPhase::Engine, None, Some(0))
        }
        LocalEngineDownloadProgress::Downloading {
            downloadedBytes, ..
        } => updateInstallProgress(
            &operationId,
            LocalModelInstallPhase::Engine,
            None,
            Some(downloadedBytes),
        ),
        LocalEngineDownloadProgress::Extracting { .. } => {
            updateInstallProgress(&operationId, LocalModelInstallPhase::Engine, None, None)
        }
        LocalEngineDownloadProgress::Completed {
            downloadedBytes, ..
        } => updateInstallProgress(
            &operationId,
            LocalModelInstallPhase::Engine,
            None,
            Some(downloadedBytes),
        ),
    })
}

/// Maps model download progress into the shared bundle operation state.
fn modelInstallProgressCallback(
    operationId: String,
    engineDownloadedBytes: u64,
) -> LocalModelDownloadProgressCallback {
    Arc::new(move |progress| match progress {
        LocalModelDownloadProgress::Started { .. } => updateInstallProgress(
            &operationId,
            LocalModelInstallPhase::Model,
            None,
            Some(engineDownloadedBytes),
        ),
        LocalModelDownloadProgress::FileStarted { relativePath, .. } => updateInstallProgress(
            &operationId,
            LocalModelInstallPhase::Model,
            Some(relativePath),
            Some(engineDownloadedBytes),
        ),
        LocalModelDownloadProgress::FileProgress {
            relativePath,
            downloadedBytes,
            ..
        }
        | LocalModelDownloadProgress::FileDownloaded {
            relativePath,
            downloadedBytes,
            ..
        } => updateInstallProgress(
            &operationId,
            LocalModelInstallPhase::Model,
            Some(relativePath),
            Some(
                engineDownloadedBytes
                    .checked_add(downloadedBytes)
                    .expect("local model progress byte total must fit in u64"),
            ),
        ),
        LocalModelDownloadProgress::Extracting { .. } => {
            updateInstallProgress(&operationId, LocalModelInstallPhase::Model, None, None)
        }
        LocalModelDownloadProgress::Completed {
            downloadedBytes, ..
        } => updateInstallProgress(
            &operationId,
            LocalModelInstallPhase::Model,
            None,
            Some(
                engineDownloadedBytes
                    .checked_add(downloadedBytes)
                    .expect("local model progress byte total must fit in u64"),
            ),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_OPERATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    /// Verifies engine and model callbacks publish monotonic aggregate bytes.
    #[test]
    fn installProgressAggregatesAcrossEngineAndModelDownloads() {
        let operationId = uniqueTestOperationId("progress");
        registerInstallOperation(
            operationId.clone(),
            operationId.clone(),
            "1".to_string(),
            300,
        )
        .unwrap();

        let engineProgress = engineInstallProgressCallback(operationId.clone());
        engineProgress(LocalEngineDownloadProgress::Downloading {
            downloadedBytes: 100,
            totalBytes: 100,
        });
        let modelProgress = modelInstallProgressCallback(operationId.clone(), 100);
        modelProgress(LocalModelDownloadProgress::FileProgress {
            relativePath: "model.bin".to_string(),
            fileBytes: 80,
            totalFileBytes: 200,
            downloadedBytes: 180,
            totalBytes: 200,
        });
        modelProgress(LocalModelDownloadProgress::FileProgress {
            relativePath: "model.bin".to_string(),
            fileBytes: 40,
            totalFileBytes: 200,
            downloadedBytes: 40,
            totalBytes: 200,
        });

        let status = testInstallStatus(&operationId);
        assert_eq!(status.phase, LocalModelInstallPhase::Model);
        assert_eq!(status.currentFile.as_deref(), Some("model.bin"));
        assert_eq!(status.downloadedBytes, 280);
        assert_eq!(status.totalBytes, 300);
        removeTestInstallOperation(&operationId);
    }

    /// Verifies duplicate active operations are rejected by stable operation id.
    #[test]
    fn duplicateActiveInstallOperationIsRejected() {
        let operationId = uniqueTestOperationId("duplicate");
        registerInstallOperation(
            operationId.clone(),
            operationId.clone(),
            "1".to_string(),
            10,
        )
        .unwrap();
        let error = registerInstallOperation(
            operationId.clone(),
            operationId.clone(),
            "1".to_string(),
            10,
        )
        .unwrap_err();

        assert_eq!(
            error,
            format!("local model install operation is already active: {operationId}")
        );
        removeTestInstallOperation(&operationId);
    }

    /// Verifies cancellation tokens determine the terminal cancelled state.
    #[test]
    fn cancelledControlProducesCancelledInstallStatus() {
        let operationId = uniqueTestOperationId("cancelled");
        let (engineControl, _) = registerInstallOperation(
            operationId.clone(),
            operationId.clone(),
            "1".to_string(),
            10,
        )
        .unwrap();
        engineControl.cancel();
        failInstallOperation(&operationId, "download interrupted").unwrap();

        let status = testInstallStatus(&operationId);
        assert_eq!(status.phase, LocalModelInstallPhase::Cancelled);
        assert_eq!(status.error, None);
        removeTestInstallOperation(&operationId);
    }

    /// Builds one unique operation id for parallel runtime tests.
    fn uniqueTestOperationId(label: &str) -> String {
        let sequence = TEST_OPERATION_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        format!("test-{label}-{}-{sequence}@1", std::process::id())
    }

    /// Returns one test operation status from the shared table.
    fn testInstallStatus(operationId: &str) -> LocalModelInstallStatus {
        installOperations()
            .lock()
            .unwrap()
            .get(operationId)
            .unwrap()
            .status
            .clone()
    }

    /// Removes one test operation from the shared table.
    fn removeTestInstallOperation(operationId: &str) {
        installOperations().lock().unwrap().remove(operationId);
    }
}
