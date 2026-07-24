use super::*;

use operit_host_api::{HostError, RuntimeStorageEntry};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct MemoryStorageHost {
    files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl RuntimeStorageHost for MemoryStorageHost {
    /// Does not expose a physical runtime root for in-memory test storage.
    fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }

    /// Does not expose a physical workspace root for in-memory test storage.
    fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
        None
    }

    /// Reads one in-memory runtime storage entry.
    fn readBytes(&self, path: &str) -> operit_host_api::HostResult<Vec<u8>> {
        let files = self
            .files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        files
            .get(path)
            .cloned()
            .ok_or_else(|| HostError::new(format!("missing runtime storage entry: {path}")))
    }

    /// Writes one in-memory runtime storage entry.
    fn writeBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        self.files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?
            .insert(path.to_string(), content.to_vec());
        Ok(())
    }

    /// Removes one in-memory runtime storage entry.
    fn delete(&self, path: &str, _recursive: bool) -> operit_host_api::HostResult<()> {
        self.files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?
            .remove(path);
        Ok(())
    }

    /// Checks whether one in-memory runtime storage entry exists.
    fn exists(&self, path: &str) -> operit_host_api::HostResult<bool> {
        Ok(self
            .files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?
            .contains_key(path))
    }

    /// Lists in-memory runtime storage entries with the requested prefix.
    fn list(&self, prefix: &str) -> operit_host_api::HostResult<Vec<RuntimeStorageEntry>> {
        Ok(self
            .files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?
            .iter()
            .filter(|(path, _)| path.starts_with(prefix))
            .map(|(path, content)| RuntimeStorageEntry {
                path: path.clone(),
                isDirectory: false,
                size: content.len() as i64,
            })
            .collect())
    }
}

/// Verifies local initialization, named remote persistence, and remote route validation.
#[test]
fn routing_config_records_and_validates_runtime_route() {
    let store = LinkAccessStore::new(Arc::new(MemoryStorageHost::default()));

    let config = store
        .initializeRoutingConfig()
        .expect("routing config must initialize");

    assert_eq!(config.route, LinkAccessRoute::Local);
    assert_eq!(
        store
            .routingConfig()
            .expect("routing config must persist")
            .route,
        LinkAccessRoute::Local
    );

    let config = LinkAccessRoutingConfig {
        route: LinkAccessRoute::Remote {
            sessionName: "desktop".to_string(),
        },
        updatedAt: 123,
    };

    store
        .saveRoutingConfig(config.clone())
        .expect("remote routing config must persist");

    assert_eq!(
        store
            .routingConfig()
            .expect("remote routing config must read"),
        config
    );

    let config = LinkAccessRoutingConfig {
        route: LinkAccessRoute::Remote {
            sessionName: " ".to_string(),
        },
        updatedAt: 123,
    };

    let error = store
        .saveRoutingConfig(config)
        .expect_err("blank remote session name must be rejected");

    assert_eq!(error, "remote Link route requires a paired session name");
}
