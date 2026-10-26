use operit_store::ExtensionStore::{ExtensionStore, PackageEnvironment};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct EnvPreferences {
    store: ExtensionStore,
}

impl EnvPreferences {
    /// Opens package environments through the common installation ownership store.
    pub fn getInstance() -> Self {
        Self {
            store: ExtensionStore::default(),
        }
    }

    /// Reads the environment shared by an installation and its subpackages.
    pub fn getPackageEnvironment(&self, packageName: &str) -> Result<PackageEnvironment, String> {
        self.store.packageEnvironment(packageName)
    }

    /// Updates explicitly edited keys without replacing unrelated package environment values.
    pub fn editPackageEnvironment(
        &self,
        packageName: &str,
        changes: BTreeMap<String, Option<String>>,
    ) -> Result<(), String> {
        self.store.editPackageEnvironment(packageName, changes)
    }

    /// Reads a persisted variable from its explicitly selected package installation.
    pub fn getEnv(&self, packageName: &str, key: &str) -> Result<Option<String>, String> {
        let environment = self.store.packageEnvironment(packageName)?;
        Ok(environment.variables.get(key.trim()).cloned())
    }

    /// Writes a variable to its package installation and removes explicitly empty values.
    pub fn setEnv(&self, packageName: &str, key: &str, value: &str) -> Result<(), String> {
        self.store.editPackageEnvironment(
            packageName,
            BTreeMap::from([(
                key.trim().to_string(),
                (!value.is_empty()).then(|| value.to_string()),
            )]),
        )
    }

    /// Removes a variable from its explicitly selected package installation.
    pub fn removeEnv(&self, packageName: &str, key: &str) -> Result<(), String> {
        self.store.editPackageEnvironment(
            packageName,
            BTreeMap::from([(key.trim().to_string(), None)]),
        )
    }
}
