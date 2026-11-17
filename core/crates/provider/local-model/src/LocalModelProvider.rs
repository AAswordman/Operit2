use std::collections::BTreeMap;
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_util::AppLogger::AppLogger;
use operit_util::RuntimeStorageLayout::RUNTIME_ROOT_PATH_PREFIX;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::LocalEngineManifest::{LocalEngineArtifact, LocalPlatform, LocalPlatformTarget};
use crate::LocalInference::{
    LocalInferenceError, LocalModelSelection, LocalSttRequest, LocalSttResponse, LocalTtsRequest,
    LocalTtsResponse,
};
use crate::LocalModelManifest::{LocalModelDriver, LocalModelKind};
use crate::LocalModelRegistry::{InstalledLocalEngine, InstalledLocalModel};
use crate::LocalModelRegistryStore::LocalModelRegistryStore;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum LocalModelProviderError {
    #[error("local model registry error: {0}")]
    Registry(String),
    #[error("local model is not installed: {0}@{1}")]
    ModelNotInstalled(String, String),
    #[error("local model kind mismatch: expected {expected}, got {actual}")]
    ModelKindMismatch { expected: String, actual: String },
    #[error("local model has no executable driver metadata: {0}@{1}")]
    DriverMissing(String, String),
    #[error("local model has no engine requirement: {0}@{1}")]
    EngineRequirementMissing(String, String),
    #[error("local engine is not installed: {0}@{1} for {2}")]
    EngineNotInstalled(String, String, String),
    #[error("local engine executable is not declared: {0}")]
    EngineExecutableMissing(String),
    #[error("local runtime file is missing: {0}")]
    RuntimeFileMissing(String),
    #[error("local driver does not support this request: {0}")]
    UnsupportedDriver(String),
    #[error("local inference process failed: {0}")]
    Process(String),
    #[error("local inference output is invalid: {0}")]
    InvalidOutput(String),
    #[error("local inference request is invalid: {0}")]
    InvalidRequest(String),
    #[error("local inference storage error: {0}")]
    Storage(String),
}

impl From<LocalModelProviderError> for LocalInferenceError {
    /// Converts a provider error into the public inference error contract.
    fn from(value: LocalModelProviderError) -> Self {
        Self::RequestFailed(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ResolvedLocalModelRuntime {
    pub model: InstalledLocalModel,
    pub engine: InstalledLocalEngine,
    pub modelDirectory: String,
    pub engineDirectory: String,
    pub engineRuntimeDirectory: String,
}

#[derive(Clone)]
pub struct LocalModelProvider {
    registryStore: LocalModelRegistryStore,
    storageHost: Arc<dyn RuntimeStorageHost>,
}

impl LocalModelProvider {
    /// Creates a local provider backed by the runtime storage host.
    pub fn forRuntimeStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
    ) -> Result<Self, LocalModelProviderError> {
        Ok(Self {
            registryStore: LocalModelRegistryStore::forRuntimeStorage(storageHost.clone()),
            storageHost,
        })
    }

    /// Resolves one installed model together with its exact platform engine.
    pub fn resolve(
        &self,
        selection: &LocalModelSelection,
        expectedKind: LocalModelKind,
    ) -> Result<ResolvedLocalModelRuntime, LocalModelProviderError> {
        let registry = self
            .registryStore
            .read()
            .map_err(|error| LocalModelProviderError::Registry(error.to_string()))?;
        let model = registry
            .getInstalledModel(&selection.modelId, &selection.version)
            .cloned()
            .ok_or_else(|| {
                LocalModelProviderError::ModelNotInstalled(
                    selection.modelId.clone(),
                    selection.version.clone(),
                )
            })?;
        if model.manifest.kind != expectedKind {
            return Err(LocalModelProviderError::ModelKindMismatch {
                expected: modelKindName(&expectedKind).to_string(),
                actual: modelKindName(&model.manifest.kind).to_string(),
            });
        }
        let requirement = model.manifest.engineRequirement.as_ref().ok_or_else(|| {
            LocalModelProviderError::EngineRequirementMissing(
                model.manifest.id.clone(),
                model.manifest.version.clone(),
            )
        })?;
        let target = LocalPlatformTarget::current().map_err(LocalModelProviderError::Process)?;
        if !model.manifest.supportsPlatform(&target.platform) {
            return Err(LocalModelProviderError::UnsupportedDriver(format!(
                "{}#{}",
                model.manifest.registryKey(),
                target.storageSegment()
            )));
        }
        let engine = registry
            .getInstalledEngine(&requirement.engineId, &requirement.version, &target)
            .cloned()
            .ok_or_else(|| {
                LocalModelProviderError::EngineNotInstalled(
                    requirement.engineId.clone(),
                    requirement.version.clone(),
                    target.storageSegment(),
                )
            })?;
        let modelDirectory = self.runtimeLayoutPath(&model.storagePath)?;
        let engineDirectory = self.runtimeLayoutPath(&engine.storagePath)?;
        // The archive root marker denotes the installation directory, not a VFS segment.
        let archiveRoot = match engine.artifact.archiveRoot.as_str() {
            "." => "",
            relative => relative,
        };
        let engineRuntimeDirectory =
            operit_vfs::VfsPath::joinVfsPath(&engineDirectory, archiveRoot)
                .map_err(LocalModelProviderError::Storage)?;
        Ok(ResolvedLocalModelRuntime {
            model,
            engine,
            modelDirectory,
            engineDirectory,
            engineRuntimeDirectory,
        })
    }

    /// Returns the virtual address of a declared runtime storage entry.
    fn runtimeLayoutPath(&self, storagePath: &str) -> Result<String, LocalModelProviderError> {
        operit_vfs::VfsPath::runtimeDataPath(storagePath).map_err(LocalModelProviderError::Storage)
    }
}

/// Returns the stable display name for one local model kind.
fn modelKindName(kind: &LocalModelKind) -> &'static str {
    match kind {
        LocalModelKind::SpeechToText => "SpeechToText",
        LocalModelKind::TextToSpeech => "TextToSpeech",
        LocalModelKind::Chat => "Chat",
        LocalModelKind::Embedding => "Embedding",
    }
}

/// Builds a serializable driver options map for host execution.
pub fn localDriverOptions(
    runtime: &ResolvedLocalModelRuntime,
) -> Result<BTreeMap<String, Value>, LocalModelProviderError> {
    let driver = runtime.model.manifest.driver.as_ref().ok_or_else(|| {
        LocalModelProviderError::DriverMissing(
            runtime.model.manifest.id.clone(),
            runtime.model.manifest.version.clone(),
        )
    })?;
    let value = serde_json::to_value(driver)
        .map_err(|error| LocalModelProviderError::InvalidOutput(error.to_string()))?;
    let object = value
        .as_object()
        .ok_or_else(|| LocalModelProviderError::InvalidOutput(value.to_string()))?;
    Ok(object
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalEngineCatalog::LocalEngineCatalog;
    use crate::LocalModelCatalog::LocalModelCatalog;
    use crate::LocalModelManifest::LocalModelManifest;
    use crate::LocalModelRegistry::LocalModelRegistrySnapshot;
    use crate::LocalModelRegistryStore::testRuntimeStorageHost;
    use crate::LocalModelStorage::{buildLocalEngineStoragePath, buildLocalModelStoragePath};

    /// Resolves an installed model through a persisted registry without loading model weights.
    fn resolveInstalledRuntime(
        manifest: LocalModelManifest,
        engineManifest: crate::LocalEngineManifest::LocalEngineManifest,
        archiveRoot: &str,
    ) -> Result<ResolvedLocalModelRuntime, LocalModelProviderError> {
        let root = std::env::temp_dir().join(format!(
            "operit-local-provider-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let storage = testRuntimeStorageHost(root.clone());
        let target = LocalPlatformTarget::current().unwrap();
        let mut artifact = engineManifest.artifactForTarget(&target).unwrap().clone();
        artifact.archiveRoot = archiveRoot.to_string();
        let selection = LocalModelSelection {
            modelId: manifest.id.clone(),
            version: manifest.version.clone(),
        };
        let kind = manifest.kind.clone();
        let mut registry = LocalModelRegistrySnapshot::empty();
        registry.upsert(InstalledLocalModel {
            storagePath: buildLocalModelStoragePath(
                &kind,
                &manifest.engine,
                &manifest.id,
                &manifest.version,
            )
            .unwrap(),
            manifest,
            installedAtMs: 1,
            verifiedAtMs: Some(1),
        });
        registry.upsertEngine(InstalledLocalEngine {
            storagePath: buildLocalEngineStoragePath(
                &engineManifest.id,
                &engineManifest.version,
                &target,
            )
            .unwrap(),
            manifest: engineManifest,
            artifact,
            installedAtMs: 1,
            verifiedAtMs: Some(1),
        });
        LocalModelRegistryStore::forRuntimeStorage(storage.clone())
            .write(&registry)
            .unwrap();
        let result = LocalModelProvider::forRuntimeStorage(storage)
            .unwrap()
            .resolve(&selection, kind);
        std::fs::remove_dir_all(root).unwrap();
        result
    }

    /// Keeps embedded llama.cpp and MNN roots resolvable before native inference starts.
    #[test]
    fn installedChatEnginesResolveInstallationRoot() {
        let qwen = LocalModelCatalog::manifests()
            .into_iter()
            .find(|model| model.id == "qwen3.5-2b-gguf")
            .unwrap();
        for (model, engine) in [
            (qwen, LocalEngineCatalog::llamaCpp()),
            (LocalModelCatalog::qwen3Mnn(), LocalEngineCatalog::mnn()),
        ] {
            let target = LocalPlatformTarget::current().unwrap();
            let archiveRoot = engine
                .artifactForTarget(&target)
                .unwrap()
                .archiveRoot
                .clone();
            assert_eq!(archiveRoot, ".");
            let runtime = resolveInstalledRuntime(model, engine, &archiveRoot).unwrap();
            assert_eq!(runtime.engineRuntimeDirectory, runtime.engineDirectory);
            assert!(runtime.modelDirectory.starts_with("/operitapp/data/"));
        }
    }

    /// Preserves both installation-root and nested archive layouts in the VFS namespace.
    #[test]
    fn installedEngineArchiveDirectoriesStayWithinVfs() {
        for archiveRoot in ["", ".", "sherpa-onnx/lib"] {
            let runtime = resolveInstalledRuntime(
                LocalModelCatalog::qwen3LlamaCpp(),
                LocalEngineCatalog::llamaCpp(),
                archiveRoot,
            )
            .unwrap();
            let expected = match archiveRoot {
                "" | "." => runtime.engineDirectory.clone(),
                _ => format!("{}/{archiveRoot}", runtime.engineDirectory),
            };
            assert_eq!(runtime.engineRuntimeDirectory, expected);
        }
    }

    /// Rejects traversal and embedded dot segments rather than weakening shared VFS validation.
    #[test]
    fn installedEngineArchiveDirectoriesRejectInvalidSegments() {
        for archiveRoot in ["..", "../outside", "lib/../../outside", "lib/./nested"] {
            let error = resolveInstalledRuntime(
                LocalModelCatalog::qwen3LlamaCpp(),
                LocalEngineCatalog::llamaCpp(),
                archiveRoot,
            )
            .unwrap_err();
            assert!(
                matches!(error, LocalModelProviderError::Storage(_)),
                "{archiveRoot}: {error}"
            );
        }
    }
}
