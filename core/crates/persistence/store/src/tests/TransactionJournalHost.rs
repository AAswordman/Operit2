//! An isolated real SQLite namespace for tests whose business files stay in memory.
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, Once};
use std::sync::atomic::{AtomicUsize, Ordering};
use operit_host_api::{HostError, HostSecretStore};
use operit_host_api::{HostResult, RuntimeSqliteConnection, RuntimeSqliteHost};
use operit_host_native_storage::NativeRuntimeStorageHost;

pub(crate) struct TransactionJournalHost {
    root: PathBuf,
    host: NativeRuntimeStorageHost,
}

impl Default for TransactionJournalHost {
    fn default() -> Self {
        installTestHostSecrets();
        let root = std::env::temp_dir().join(format!("operit-transaction-test-{}", uuid::Uuid::new_v4()));
        let host = NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces"));
        Self { root, host }
    }
}

impl TransactionJournalHost {
    pub(crate) fn open(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.host.openSqliteDatabase(&format!("runtime/journals/{path}"))
    }
}

impl Drop for TransactionJournalHost {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); }
}

/// Registers one stable in-memory host secret interface for encrypted store fixtures.
#[allow(non_snake_case)]
fn installTestHostSecrets() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| crate::RuntimeStorageHost::setDefaultHostSecretStore(Arc::new(MemorySecretStore::default())));
}

#[derive(Clone, Default)]
pub(crate) struct MemorySecretStore {
    secrets: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    writes: Arc<AtomicUsize>,
}

impl MemorySecretStore {
    #[allow(non_snake_case)]
    pub(crate) fn writeCount(&self) -> usize { self.writes.load(Ordering::SeqCst) }
}

impl HostSecretStore for MemorySecretStore {
    /// Reads secret bytes from the in-memory test secret store.
    fn readSecret(&self, key: &str) -> operit_host_api::HostResult<Option<Vec<u8>>> {
        let secrets = self
            .secrets
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(secrets.get(key).cloned())
    }

    /// Writes secret bytes into the in-memory test secret store.
    fn writeSecret(&self, key: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        secrets.insert(key.to_string(), content.to_vec());
        self.writes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Deletes secret bytes from the in-memory test secret store.
    fn deleteSecret(&self, key: &str) -> operit_host_api::HostResult<()> {
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        secrets.remove(key);
        Ok(())
    }
}
