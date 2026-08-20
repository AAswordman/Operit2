use super::*;

use operit_host_api::{HostError, RuntimeStorageEntry};
use operit_util::RuntimeStorageLayout::RUNTIME_SPACE_TOPOLOGY_DIR_PATH;
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

    /// Appends bytes to one in-memory runtime storage entry.
    fn appendBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        self.files
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?
            .entry(path.to_string())
            .or_default()
            .extend_from_slice(content);
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

/// Verifies Link Access storage no longer creates a global execution route.
#[test]
fn link_access_store_constructs_without_global_route() {
    let _store = LinkAccessStore::new(Arc::new(MemoryStorageHost::default()));
}

/// Creates one accepted inbound session record for topology lifecycle tests.
#[allow(non_snake_case)]
fn acceptedSession(peerNodeId: &str) -> AcceptedRemoteSessionRecord {
    AcceptedRemoteSessionRecord {
        deviceId: peerNodeId.to_string(),
        deviceInfo: RemoteDeviceInfo {
            platform: "test".to_string(),
            model: "peer".to_string(),
        },
        pairingServiceVersion: 1,
        sessionSecret: "inbound-secret".to_string(),
    }
}

/// Creates one outbound session record targeting the same direct CoreNode.
#[allow(non_snake_case)]
fn outboundSession(localNodeId: &str, peerNodeId: &str) -> PairedRemoteSessionRecord {
    PairedRemoteSessionRecord {
        baseUrl: "http://peer.invalid".to_string(),
        sessionId: "outbound-session".to_string(),
        deviceId: localNodeId.to_string(),
        coreDeviceId: peerNodeId.to_string(),
        remoteDeviceInfo: RemoteDeviceInfo {
            platform: "test".to_string(),
            model: "peer".to_string(),
        },
        pairingServiceVersion: 1,
        sessionSecret: "outbound-secret".to_string(),
        transport: LinkTransportPreference::Http,
    }
}

/// Verifies pairing session persistence never reads the active Peer Link topology projection.
#[test]
fn session_persistence_does_not_read_space_topology() {
    let storage = Arc::new(MemoryStorageHost::default());
    CoreNodeIdentityStore::new(storage.clone())
        .writeNodeId("node-a".to_string())
        .expect("local CoreNode identity must be written");
    CoreSpaceStore::new(storage.clone())
        .initialize()
        .expect("local Space must initialize");
    storage
        .writeBytes(
            &format!("{RUNTIME_SPACE_TOPOLOGY_DIR_PATH}/node-a.preferences.json"),
            b"",
        )
        .expect("empty topology fixture must be written");
    let accessStore = LinkAccessStore::new(storage.clone());
    accessStore
        .saveInboundSession("inbound-1".to_string(), acceptedSession("node-b"))
        .expect("first inbound session must persist");
    accessStore
        .saveInboundSession("inbound-2".to_string(), acceptedSession("node-b"))
        .expect("second inbound session must persist");
    accessStore
        .saveOutboundSession("outbound".to_string(), outboundSession("node-a", "node-b"))
        .expect("outbound session must persist");
    accessStore
        .removeInboundSession("inbound-1")
        .expect("first inbound session must be removed");
    accessStore
        .removeInboundSession("inbound-2")
        .expect("second inbound session must be removed");
    accessStore
        .removeOutboundSession("outbound")
        .expect("outbound session must be removed");
    assert!(accessStore
        .inboundSessions()
        .expect("inbound sessions must read")
        .is_empty());
    assert!(accessStore
        .outboundSessions()
        .expect("outbound sessions must read")
        .is_empty());
}
