use serde::{Deserialize, Serialize};

use crate::LocalEngineManifest::{LocalEngineArtifact, LocalEngineManifest, LocalPlatformTarget};
use crate::LocalModelManifest::{LocalModelInstallSource, LocalModelKind, LocalModelManifest};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct InstalledLocalModel {
    pub manifest: LocalModelManifest,
    pub storagePath: String,
    pub installedAtMs: i64,
    pub verifiedAtMs: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct InstalledLocalEngine {
    pub manifest: LocalEngineManifest,
    pub artifact: LocalEngineArtifact,
    pub storagePath: String,
    pub installedAtMs: i64,
    pub verifiedAtMs: Option<i64>,
}

impl InstalledLocalEngine {
    /// Returns the stable registry key for this installed engine target.
    pub fn registryKey(&self) -> String {
        format!(
            "{}@{}#{}",
            self.manifest.id,
            self.manifest.version,
            self.artifact.target.storageSegment()
        )
    }
}

impl InstalledLocalModel {
    /// Returns the stable registry key for this installed model.
    pub fn registryKey(&self) -> String {
        self.manifest.registryKey()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalModelRegistrySnapshot {
    pub installedModels: Vec<InstalledLocalModel>,
    #[serde(default)]
    pub installedEngines: Vec<InstalledLocalEngine>,
}

impl LocalModelRegistrySnapshot {
    /// Creates an empty local model registry snapshot.
    pub fn empty() -> Self {
        Self {
            installedModels: Vec::new(),
            installedEngines: Vec::new(),
        }
    }

    /// Returns the installed model matching the supplied model id and version.
    pub fn getInstalledModel(&self, modelId: &str, version: &str) -> Option<&InstalledLocalModel> {
        let modelId = modelId.trim();
        let version = version.trim();
        self.installedModels
            .iter()
            .find(|model| model.manifest.id == modelId && model.manifest.version == version)
    }

    /// Returns installed models matching the supplied local model kind.
    pub fn modelsByKind(&self, kind: LocalModelKind) -> Vec<&InstalledLocalModel> {
        self.installedModels
            .iter()
            .filter(|model| model.manifest.kind == kind)
            .collect()
    }

    /// Returns the installed engine matching an exact id, version, and target.
    pub fn getInstalledEngine(
        &self,
        engineId: &str,
        version: &str,
        target: &LocalPlatformTarget,
    ) -> Option<&InstalledLocalEngine> {
        let engineId = engineId.trim();
        let version = version.trim();
        self.installedEngines.iter().find(|engine| {
            engine.manifest.id == engineId
                && engine.manifest.version == version
                && engine.artifact.target == *target
        })
    }

    /// Adds or replaces one installed local model by model id and version.
    pub fn upsert(&mut self, installedModel: InstalledLocalModel) {
        self.installedModels.retain(|existing| {
            existing.manifest.id != installedModel.manifest.id
                || existing.manifest.version != installedModel.manifest.version
        });
        self.installedModels.push(installedModel);
    }

    /// Adds or replaces one installed local engine by id, version, and target.
    pub fn upsertEngine(&mut self, installedEngine: InstalledLocalEngine) {
        self.installedEngines.retain(|existing| {
            existing.manifest.id != installedEngine.manifest.id
                || existing.manifest.version != installedEngine.manifest.version
                || existing.artifact.target != installedEngine.artifact.target
        });
        self.installedEngines.push(installedEngine);
    }

    /// Removes one installed local model by model id and version.
    pub fn remove(&mut self, modelId: &str, version: &str) -> bool {
        let before = self.installedModels.len();
        let modelId = modelId.trim();
        let version = version.trim();
        self.installedModels
            .retain(|model| model.manifest.id != modelId || model.manifest.version != version);
        self.installedModels.len() != before
    }

    /// Removes one installed local engine by id, version, and target.
    pub fn removeEngine(
        &mut self,
        engineId: &str,
        version: &str,
        target: &LocalPlatformTarget,
    ) -> bool {
        let before = self.installedEngines.len();
        let engineId = engineId.trim();
        let version = version.trim();
        self.installedEngines.retain(|engine| {
            engine.manifest.id != engineId
                || engine.manifest.version != version
                || engine.artifact.target != *target
        });
        self.installedEngines.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalModelManifest::{
        LocalEngineKind, LocalModelFile, LocalModelKind, LocalModelManifest, LocalModelSource,
        LocalModelSourceKind,
    };

    /// Verifies registry records are replaced by model id and version.
    #[test]
    fn upsertReplacesMatchingInstalledModel() {
        let mut registry = LocalModelRegistrySnapshot::empty();
        registry.upsert(installedModel("model-a", "v1", 100));
        registry.upsert(installedModel("model-a", "v1", 200));

        assert_eq!(registry.installedModels.len(), 1);
        assert_eq!(registry.installedModels[0].installedAtMs, 200);
    }

    /// Verifies registry removal targets the exact model id and version.
    #[test]
    fn removeDeletesExactInstalledModel() {
        let mut registry = LocalModelRegistrySnapshot::empty();
        registry.upsert(installedModel("model-a", "v1", 100));
        registry.upsert(installedModel("model-a", "v2", 200));

        assert!(registry.remove("model-a", "v1"));
        assert_eq!(registry.installedModels.len(), 1);
        assert_eq!(registry.installedModels[0].manifest.version, "v2");
    }

    /// Builds an installed model fixture for registry tests.
    fn installedModel(modelId: &str, version: &str, installedAtMs: i64) -> InstalledLocalModel {
        InstalledLocalModel {
            manifest: manifest(modelId, version),
            storagePath: format!("runtime/models/local/stt/sherpa_ncnn/{modelId}/{version}"),
            installedAtMs,
            verifiedAtMs: None,
        }
    }

    /// Builds a manifest fixture for registry tests.
    fn manifest(modelId: &str, version: &str) -> LocalModelManifest {
        LocalModelManifest {
            id: modelId.to_string(),
            version: version.to_string(),
            displayName: modelId.to_string(),
            description: "test model".to_string(),
            kind: LocalModelKind::SpeechToText,
            engine: LocalEngineKind::SherpaNcnn,
            license: "test".to_string(),
            homepage: "https://example.test/model".to_string(),
            languages: vec!["en".to_string()],
            tags: Vec::new(),
            engineRequirement: None,
            driver: None,
            sources: vec![LocalModelSource {
                id: "main".to_string(),
                kind: LocalModelSourceKind::DirectHttp,
                repository: "test".to_string(),
                revision: version.to_string(),
                baseUrl: "https://example.test/model".to_string(),
            }],
            installSource: LocalModelInstallSource::Files,
            files: vec![LocalModelFile {
                relativePath: "model.bin".to_string(),
                sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
                    .to_string(),
                byteSize: 5,
                sourceId: "main".to_string(),
            }],
        }
    }
}
