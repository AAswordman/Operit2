//! An isolated real SQLite namespace for tests whose business files stay in memory.
use std::path::PathBuf;
use operit_host_api::{HostResult, RuntimeSqliteConnection, RuntimeSqliteHost};
use operit_host_native_storage::NativeRuntimeStorageHost;

pub(crate) struct TestTransactionJournal {
    root: PathBuf,
    host: NativeRuntimeStorageHost,
}

impl Default for TestTransactionJournal {
    fn default() -> Self {
        let root = std::env::temp_dir().join(format!("operit-transaction-test-{}", uuid::Uuid::new_v4()));
        let host = NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces"));
        Self { root, host }
    }
}

impl TestTransactionJournal {
    pub(crate) fn open(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
        self.host.openSqliteDatabase(&format!("runtime/journals/{path}"))
    }
}

impl Drop for TestTransactionJournal {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.root); }
}
