use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_host_api::TimeUtils::tryCurrentTimeMillis;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::RuntimeStorageHost::{defaultRuntimeStorageHost, runtimeStoragePath};
use crate::RuntimeStorePaths::RuntimeStorePaths;

#[derive(Debug, Error)]
pub enum SyncOperationStoreError {
    #[error("host error: {0}")]
    Host(#[from] operit_host_api::HostError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncClock {
    pub sequences: BTreeMap<String, i64>,
}

impl SyncClock {
    pub fn empty() -> Self {
        Self {
            sequences: BTreeMap::new(),
        }
    }

    pub fn sequenceFor(&self, deviceId: &str) -> i64 {
        match self.sequences.get(deviceId) {
            Some(sequence) => *sequence,
            None => 0,
        }
    }

    pub fn setSequence(&mut self, deviceId: impl Into<String>, sequence: i64) {
        self.sequences.insert(deviceId.into(), sequence);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SyncOperation {
    pub opId: String,
    pub originDeviceId: String,
    pub sequence: i64,
    pub domain: String,
    pub entityType: String,
    pub entityId: String,
    pub operation: String,
    pub payload: Value,
    pub createdAt: i64,
    pub schemaVersion: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewSyncOperation {
    pub domain: String,
    pub entityType: String,
    pub entityId: String,
    pub operation: String,
    pub payload: Value,
}

#[derive(Clone)]
pub struct SyncOperationStore {
    storageHost: Arc<dyn RuntimeStorageHost>,
    rootPath: String,
}

impl SyncOperationStore {
    pub fn new(storageHost: Arc<dyn RuntimeStorageHost>, rootPath: impl Into<String>) -> Self {
        Self {
            storageHost,
            rootPath: rootPath.into(),
        }
    }

    pub fn native(paths: RuntimeStorePaths) -> Self {
        Self::new(
            defaultRuntimeStorageHost(),
            runtimeStoragePath(&paths.root_dir().join("sync")),
        )
    }

    pub fn appendLocalOperation(
        &self,
        originDeviceId: &str,
        operation: NewSyncOperation,
    ) -> Result<SyncOperation, SyncOperationStoreError> {
        let mut clock = self.localClock()?;
        let sequence = clock.sequenceFor(originDeviceId) + 1;
        let op = SyncOperation {
            opId: format!("{originDeviceId}:{sequence}"),
            originDeviceId: originDeviceId.to_string(),
            sequence,
            domain: operation.domain,
            entityType: operation.entityType,
            entityId: operation.entityId,
            operation: operation.operation,
            payload: operation.payload,
            createdAt: currentTimeMillis()?,
            schemaVersion: 1,
        };
        self.appendOperation(&op)?;
        clock.setSequence(originDeviceId.to_string(), sequence);
        self.writeLocalClock(&clock)?;
        Ok(op)
    }

    pub fn localDeviceId(&self) -> Result<String, SyncOperationStoreError> {
        let path = self.localDeviceIdPath();
        if self.storageHost.exists(&path)? {
            let content = String::from_utf8(self.storageHost.readBytes(&path)?)
                .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
            let value = content.trim().to_string();
            if !value.is_empty() {
                return Ok(value);
            }
        }
        let now = currentTimeMillis()?;
        let mut hasher = DefaultHasher::new();
        self.rootPath.hash(&mut hasher);
        now.hash(&mut hasher);
        let deviceId = format!("core-{now}-{:016x}", hasher.finish());
        self.storageHost.writeBytes(&path, deviceId.as_bytes())?;
        Ok(deviceId)
    }

    pub fn appendOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SyncOperationStoreError> {
        let mut operations = self.operationsForDevice(&operation.originDeviceId)?;
        let clock = self.localClock()?;
        let alreadyExists = operations
            .iter()
            .any(|existing| existing.opId == operation.opId);
        if alreadyExists {
            self.observeOperation(operation)?;
            return Ok(());
        }
        if operation.sequence <= clock.sequenceFor(&operation.originDeviceId) {
            return Ok(());
        }
        operations.push(operation.clone());
        let mut operations = compactSyncOperations(operations);
        operations.sort_by(|left, right| left.sequence.cmp(&right.sequence));
        let mut content = String::new();
        for operation in operations {
            content.push_str(&serde_json::to_string(&operation)?);
            content.push('\n');
        }
        self.storageHost.writeBytes(
            &self.operationsPath(&operation.originDeviceId),
            content.as_bytes(),
        )?;
        self.registerDevice(&operation.originDeviceId)?;
        self.observeOperation(operation)?;
        Ok(())
    }

    pub fn operationsSince(
        &self,
        clock: &SyncClock,
        domains: &[String],
        limit: usize,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        let domainSet = domains.iter().cloned().collect::<BTreeSet<_>>();
        let mut out = Vec::new();
        for deviceId in self.devices()? {
            for operation in self.operationsForDevice(&deviceId)? {
                if operation.sequence <= clock.sequenceFor(&deviceId) {
                    continue;
                }
                if !domainSet.is_empty() && !domainSet.contains(&operation.domain) {
                    continue;
                }
                out.push(operation);
            }
        }
        out.sort_by(|left, right| {
            left.createdAt
                .cmp(&right.createdAt)
                .then(left.originDeviceId.cmp(&right.originDeviceId))
                .then(left.sequence.cmp(&right.sequence))
        });
        let mut out = compactSyncOperations(out);
        out.truncate(limit);
        Ok(out)
    }

    pub fn localClock(&self) -> Result<SyncClock, SyncOperationStoreError> {
        self.readJson(&self.clockPath())
    }

    pub fn writeLocalClock(&self, clock: &SyncClock) -> Result<(), SyncOperationStoreError> {
        self.writeJson(&self.clockPath(), clock)
    }

    #[allow(non_snake_case)]
    pub fn observeOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SyncOperationStoreError> {
        let mut clock = self.localClock()?;
        if operation.sequence > clock.sequenceFor(&operation.originDeviceId) {
            clock.setSequence(operation.originDeviceId.clone(), operation.sequence);
            self.writeLocalClock(&clock)?;
        }
        Ok(())
    }

    fn operationsForDevice(
        &self,
        deviceId: &str,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        let path = self.operationsPath(deviceId);
        if !self.storageHost.exists(&path)? {
            return Ok(Vec::new());
        }
        let content = String::from_utf8(self.storageHost.readBytes(&path)?)
            .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
        let mut operations = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            operations.push(serde_json::from_str(trimmed)?);
        }
        Ok(operations)
    }

    fn devices(&self) -> Result<Vec<String>, SyncOperationStoreError> {
        self.readJson(&self.devicesPath())
    }

    fn registerDevice(&self, deviceId: &str) -> Result<(), SyncOperationStoreError> {
        let mut devices = self.devices()?;
        if devices.iter().any(|existing| existing == deviceId) {
            return Ok(());
        }
        devices.push(deviceId.to_string());
        devices.sort();
        self.writeJson(&self.devicesPath(), &devices)
    }

    fn readJson<T>(&self, path: &str) -> Result<T, SyncOperationStoreError>
    where
        T: serde::de::DeserializeOwned + Default,
    {
        if !self.storageHost.exists(path)? {
            return Ok(T::default());
        }
        let content = String::from_utf8(self.storageHost.readBytes(path)?)
            .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
        if content.trim().is_empty() {
            return Ok(T::default());
        }
        Ok(serde_json::from_str(&content)?)
    }

    fn writeJson<T>(&self, path: &str, value: &T) -> Result<(), SyncOperationStoreError>
    where
        T: serde::Serialize,
    {
        let content = serde_json::to_vec_pretty(value)?;
        self.storageHost.writeBytes(path, &content)?;
        Ok(())
    }

    fn clockPath(&self) -> String {
        format!("{}/clocks.json", self.rootPath)
    }

    fn devicesPath(&self) -> String {
        format!("{}/devices.json", self.rootPath)
    }

    fn localDeviceIdPath(&self) -> String {
        format!("{}/local_device_id", self.rootPath)
    }

    fn operationsPath(&self, deviceId: &str) -> String {
        format!(
            "{}/operations/{}.jsonl",
            self.rootPath,
            storageSafeId(deviceId)
        )
    }
}

impl Default for SyncClock {
    fn default() -> Self {
        Self::empty()
    }
}

fn currentTimeMillis() -> Result<i64, SyncOperationStoreError> {
    tryCurrentTimeMillis().map_err(SyncOperationStoreError::Message)
}

fn storageSafeId(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[allow(non_snake_case)]
pub fn compactSyncOperations(operations: Vec<SyncOperation>) -> Vec<SyncOperation> {
    let mut latestUpserts = BTreeMap::<(String, String, String, String), i64>::new();
    for operation in &operations {
        if operation.operation == "upsert" {
            let key = syncEntityKey(operation);
            let sequence = latestUpserts.entry(key).or_insert(operation.sequence);
            if operation.sequence > *sequence {
                *sequence = operation.sequence;
            }
        }
    }

    let mut compacted = Vec::with_capacity(operations.len());
    for operation in operations {
        if operation.operation == "upsert" {
            let key = syncEntityKey(&operation);
            if latestUpserts.get(&key).copied() != Some(operation.sequence) {
                continue;
            }
        }
        compacted.push(operation);
    }
    compacted
}

#[allow(non_snake_case)]
fn syncEntityKey(operation: &SyncOperation) -> (String, String, String, String) {
    (
        operation.originDeviceId.clone(),
        operation.domain.clone(),
        operation.entityType.clone(),
        operation.entityId.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use operit_host_api::{HostError, RuntimeStorageEntry};
    use serde_json::{json, Value};

    #[derive(Clone, Default)]
    struct MemoryStorageHost {
        files: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    }

    impl RuntimeStorageHost for MemoryStorageHost {
        fn readBytes(&self, path: &str) -> operit_host_api::HostResult<Vec<u8>> {
            let files = self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            match files.get(path) {
                Some(content) => Ok(content.clone()),
                None => Err(HostError::new(format!(
                    "missing runtime storage file: {path}"
                ))),
            }
        }

        fn writeBytes(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
            let mut files = self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            files.insert(path.to_string(), content.to_vec());
            Ok(())
        }

        fn delete(&self, path: &str, _recursive: bool) -> operit_host_api::HostResult<()> {
            let mut files = self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            files.remove(path);
            Ok(())
        }

        fn exists(&self, path: &str) -> operit_host_api::HostResult<bool> {
            let files = self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            Ok(files.contains_key(path))
        }

        fn list(&self, prefix: &str) -> operit_host_api::HostResult<Vec<RuntimeStorageEntry>> {
            let files = self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?;
            Ok(files
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

    fn operation(
        sequence: i64,
        entityType: &str,
        entityId: &str,
        operationName: &str,
        payload: Value,
    ) -> SyncOperation {
        SyncOperation {
            opId: format!("device-a:{sequence}"),
            originDeviceId: "device-a".to_string(),
            sequence,
            domain: "chat".to_string(),
            entityType: entityType.to_string(),
            entityId: entityId.to_string(),
            operation: operationName.to_string(),
            payload,
            createdAt: sequence,
            schemaVersion: 1,
        }
    }

    fn operationFromOrigin(
        originDeviceId: &str,
        sequence: i64,
        entityType: &str,
        entityId: &str,
        operationName: &str,
        payload: Value,
    ) -> SyncOperation {
        SyncOperation {
            opId: format!("{originDeviceId}:{sequence}"),
            originDeviceId: originDeviceId.to_string(),
            sequence,
            domain: "chat".to_string(),
            entityType: entityType.to_string(),
            entityId: entityId.to_string(),
            operation: operationName.to_string(),
            payload,
            createdAt: sequence,
            schemaVersion: 1,
        }
    }

    fn sequences(operations: &[SyncOperation]) -> Vec<i64> {
        operations
            .iter()
            .map(|operation| operation.sequence)
            .collect()
    }

    #[test]
    fn compact_keeps_latest_upsert_for_each_entity() {
        let compacted = compactSyncOperations(vec![
            operation(1, "message", "chat-1:1", "upsert", json!({"content": "a"})),
            operation(2, "message", "chat-1:1", "upsert", json!({"content": "ab"})),
            operation(
                3,
                "message",
                "chat-1:2",
                "upsert",
                json!({"content": "other"}),
            ),
            operation(4, "chat", "chat-1", "upsert", json!({"title": "New Chat"})),
        ]);

        assert_eq!(sequences(&compacted), vec![2, 3, 4]);
        assert_eq!(compacted[0].payload["content"], "ab");
    }

    #[test]
    fn compact_keeps_delete_transactions_between_upserts() {
        let compacted = compactSyncOperations(vec![
            operation(
                1,
                "message",
                "chat-1:1",
                "upsert",
                json!({"content": "old"}),
            ),
            operation(2, "message", "chat-1:1", "delete", json!({"deleted": true})),
            operation(
                3,
                "message",
                "chat-1:1",
                "upsert",
                json!({"content": "new"}),
            ),
        ]);

        assert_eq!(sequences(&compacted), vec![2, 3]);
        assert_eq!(compacted[0].operation, "delete");
        assert_eq!(compacted[1].payload["content"], "new");
    }

    #[test]
    fn append_and_export_compact_repeated_stream_snapshots() {
        let host = Arc::new(MemoryStorageHost::default());
        let store = SyncOperationStore::new(host, "sync-test");

        store
            .appendOperation(&operation(
                1,
                "message",
                "chat-1:1",
                "upsert",
                json!({"content": "h"}),
            ))
            .unwrap();
        store
            .appendOperation(&operation(
                2,
                "message",
                "chat-1:1",
                "upsert",
                json!({"content": "he"}),
            ))
            .unwrap();
        store
            .appendOperation(&operation(
                3,
                "message",
                "chat-1:1",
                "upsert",
                json!({"content": "hello"}),
            ))
            .unwrap();

        let operations = store
            .operationsSince(&SyncClock::empty(), &["chat".to_string()], 100)
            .unwrap();

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].sequence, 3);
        assert_eq!(operations[0].payload["content"], "hello");
    }

    #[test]
    fn stress_compacts_many_stream_snapshots_to_one_exported_upsert() {
        let host = Arc::new(MemoryStorageHost::default());
        let store = SyncOperationStore::new(host, "sync-stress");

        for sequence in 1..=2_000 {
            store
                .appendOperation(&operation(
                    sequence,
                    "message",
                    "chat-1:1",
                    "upsert",
                    json!({"content": format!("token-{sequence}")}),
                ))
                .unwrap();
        }

        let operations = store
            .operationsSince(&SyncClock::empty(), &["chat".to_string()], 100)
            .unwrap();

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].sequence, 2_000);
        assert_eq!(operations[0].payload["content"], "token-2000");
    }

    #[test]
    fn stress_compacts_many_entities_without_cross_entity_loss() {
        let mut operations = Vec::new();
        let mut sequence = 1;
        for deviceIndex in 0..4 {
            let deviceId = format!("device-{deviceIndex}");
            for round in 0..80 {
                for entityIndex in 0..30 {
                    let entityId = format!("chat-{deviceIndex}:{entityIndex}");
                    operations.push(operationFromOrigin(
                        &deviceId,
                        sequence,
                        "message",
                        &entityId,
                        "upsert",
                        json!({"round": round, "entity": entityId}),
                    ));
                    sequence += 1;
                }
            }
            operations.push(operationFromOrigin(
                &deviceId,
                sequence,
                "message",
                &format!("chat-{deviceIndex}:deleted"),
                "delete",
                json!({"deleted": true}),
            ));
            sequence += 1;
        }

        let compacted = compactSyncOperations(operations);
        let expectedUpserts = 4 * 30;
        let expectedDeletes = 4;

        assert_eq!(compacted.len(), expectedUpserts + expectedDeletes);
        assert_eq!(
            compacted
                .iter()
                .filter(|operation| operation.operation == "delete")
                .count(),
            expectedDeletes
        );
        assert!(compacted
            .iter()
            .filter(|operation| operation.operation == "upsert")
            .all(|operation| operation.payload["round"] == 79));
    }

    #[test]
    #[ignore]
    fn stress_ultra_compacts_many_entities_without_cross_entity_loss() {
        let deviceCount = 4;
        let entityCount = 30;
        let updateRounds = 8_000;
        let mut operations = Vec::new();
        let mut sequence = 1;
        for deviceIndex in 0..deviceCount {
            let deviceId = format!("device-{deviceIndex}");
            for round in 0..updateRounds {
                for entityIndex in 0..entityCount {
                    let entityId = format!("chat-{deviceIndex}:{entityIndex}");
                    operations.push(operationFromOrigin(
                        &deviceId,
                        sequence,
                        "message",
                        &entityId,
                        "upsert",
                        json!({"round": round, "entity": entityId}),
                    ));
                    sequence += 1;
                }
            }
            operations.push(operationFromOrigin(
                &deviceId,
                sequence,
                "message",
                &format!("chat-{deviceIndex}:deleted"),
                "delete",
                json!({"deleted": true}),
            ));
            sequence += 1;
        }

        let rawCount = operations.len();
        let compacted = compactSyncOperations(operations);
        let expectedUpserts = deviceCount * entityCount;
        let expectedDeletes = deviceCount;

        eprintln!(
            "sync operation ultra stress: raw_operations={rawCount}, compacted_operations={}",
            compacted.len()
        );
        assert_eq!(
            rawCount,
            deviceCount * entityCount * updateRounds + deviceCount
        );
        assert_eq!(compacted.len(), expectedUpserts + expectedDeletes);
        assert_eq!(
            compacted
                .iter()
                .filter(|operation| operation.operation == "delete")
                .count(),
            expectedDeletes
        );
        assert!(compacted
            .iter()
            .filter(|operation| operation.operation == "upsert")
            .all(|operation| operation.payload["round"] == updateRounds - 1));
    }
}
