use std::path::PathBuf;
use std::sync::Arc;

use crate::LocalModelManifest::LocalModelManifest;
use crate::LocalModelRegistry::LocalModelRegistrySnapshot;

pub trait LocalModelRuntimeSupport: Send + Sync {
    /// Returns the root directory used for local model runtime data.
    fn localModelDataDir(&self) -> Result<PathBuf, String>;

    /// Returns the current installed local model registry snapshot.
    fn installedLocalModels(&self) -> Result<LocalModelRegistrySnapshot, String>;

    /// Persists the supplied installed local model registry snapshot.
    fn saveInstalledLocalModels(&self, snapshot: LocalModelRegistrySnapshot) -> Result<(), String>;

    /// Returns available local model manifests known to the runtime.
    fn availableLocalModelManifests(&self) -> Result<Vec<LocalModelManifest>, String>;
}

#[derive(Clone)]
pub struct LocalModelRuntimeContext {
    support: Arc<dyn LocalModelRuntimeSupport>,
}

impl LocalModelRuntimeContext {
    /// Creates a local model runtime context from a caller-owned support implementation.
    pub fn new(support: Arc<dyn LocalModelRuntimeSupport>) -> Self {
        Self { support }
    }

    /// Returns the runtime support implementation for local model operations.
    pub fn support(&self) -> &dyn LocalModelRuntimeSupport {
        self.support.as_ref()
    }

    /// Clones the shared runtime support implementation.
    pub fn sharedSupport(&self) -> Arc<dyn LocalModelRuntimeSupport> {
        self.support.clone()
    }
}
