//! 节点通信持久化。沿用原 link_access 路径和 Preferences 格式，不恢复旧握手或 HTTP 接口。
use crate::NodeServices::{PairedPeer, PeerTransport};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use operit_host_api::RuntimeStorageHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use ring::rand::{SecureRandom, SystemRandom};
use operit_link::protocol::LinkDeviceInfo;
use operit_store::PreferencesDataStore::{stringPreferencesKey, CoreNodeStateStore, Preferences, PreferencesDataStoreError, PREFERENCES_SCHEMA_VERSION_KEY_NAME};
use operit_util::RuntimeStorageLayout::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy)]
pub(crate) enum StoredDirection {
    Inbound,
    Outbound,
}

/// 原监听配置；token 只供本地 runtime 使用，不生成 UI DTO，也不实现 Debug。
#[derive(Clone, Serialize, Deserialize)]
pub struct PeerHostConfig {
    pub bindAddress: String,
    pub token: String,
    /// 显式暴露的传输方式；空列表不启动监听。
    #[serde(default)]
    pub transports: Vec<operit_peer_link::PeerTransport>,
    pub discoveryEnabled: bool,
    pub portMode: PeerHostPortMode,
    pub updatedAt: i64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerHostPortMode {
    #[serde(rename = "automatic")]
    Automatic,
    #[serde(rename = "fixed")]
    Fixed,
}

/// 配对服务沿用原版本及原入站/出站凭证，不按重构后的传输实现另分版本。
pub(crate) const PAIRING_SERVICE_VERSION: u32 = 1;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StoredInbound {
    pub deviceId: String,
    pub deviceInfo: LinkDeviceInfo,
    pub pairingServiceVersion: u32,
    pub sessionSecret: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StoredOutbound {
    pub endpoint: String,
    pub sessionId: String,
    pub deviceId: String,
    pub peerNodeId: String,
    pub peerDeviceInfo: LinkDeviceInfo,
    pub pairingServiceVersion: u32,
    pub sessionSecret: String,
    pub transport: String,
}

#[derive(Clone)]
pub struct PeerStateStore {
    storage: Arc<dyn RuntimeStorageHost>,
}
impl PeerStateStore {
    const OUTBOUND_PREFERENCES_VERSION: u32 = 1;
    const HOST_CONFIG_PREFERENCES_VERSION: u32 = 1;

    /// Creates the node-local peer preference store over the supplied storage host.
    pub fn new(storage: Arc<dyn RuntimeStorageHost>) -> Self {
        Self { storage }
    }
    /// Opens peer preferences with their declared storage schema.
    fn store(&self, path: &str) -> CoreNodeStateStore {
        let store = CoreNodeStateStore::newWithStorage(self.storage.clone(), path);
        if path == RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH {
            store.withSchema(
                Self::HOST_CONFIG_PREFERENCES_VERSION,
                Self::migrateHostConfigPreferences,
            )
        } else if path == RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH {
            store.withSchema(
                Self::OUTBOUND_PREFERENCES_VERSION,
                Self::migrateOutboundPreferences,
            )
        } else {
            store
        }
    }
    /// Initializes listener defaults in the preference schema transaction.
    fn migrateHostConfigPreferences(
        version: u32,
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        match version {
            0 => {
                if preferences.entries().is_empty() {
                    let initial = PeerHostConfig {
                        bindAddress: "0.0.0.0:37195".into(),
                        token: newHostPairingToken()?,
                        transports: vec![PeerTransport::Http, PeerTransport::WebSocket],
                        discoveryEnabled: true,
                        portMode: PeerHostPortMode::Fixed,
                        updatedAt: currentTimeMillis(),
                    };
                    writeHostConfigPreferences(preferences, &initial)?;
                } else {
                    // Legacy exposure flags do not declare any active transport.
                    if preferences.get(&stringPreferencesKey("transports")).is_none() {
                        preferences.set(&stringPreferencesKey("transports"), "[]".into());
                    }
                    decodeHostConfig(preferences).map_err(PreferencesDataStoreError::Message)?;
                }
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration {
                from,
                to: from + 1,
            }),
        }
    }

    /// Migrates outbound preferences one schema version at a time.
    fn migrateOutboundPreferences(
        version: u32,
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        match version {
            0 => Self::normalizeOutboundSessionFields(preferences),
            from => Err(PreferencesDataStoreError::MissingMigration {
                from,
                to: from + 1,
            }),
        }
    }

    /// Renames persisted remote session fields without changing pairing credentials.
    fn normalizeOutboundSessionFields(
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        for (name, encoded) in preferences.entries() {
            if name == PREFERENCES_SCHEMA_VERSION_KEY_NAME {
                continue;
            }
            let mut record: Value = serde_json::from_str(&encoded)?;
            let fields = record.as_object_mut().ok_or_else(|| {
                PreferencesDataStoreError::Message(format!("Invalid outbound record: {name}"))
            })?;
            for (old, new) in [
                ("baseUrl", "endpoint"),
                ("coreDeviceId", "peerNodeId"),
                ("remoteDeviceInfo", "peerDeviceInfo"),
            ] {
                if let Some(value) = fields.remove(old) {
                    if fields.get(new).is_some_and(|current| current != &value) {
                        return Err(PreferencesDataStoreError::Message(format!(
                            "Conflicting outbound field {new}: {name}"
                        )));
                    }
                    fields.insert(new.into(), value);
                }
            }
            fields
                .entry("transport")
                .or_insert_with(|| Value::String("http".into()));
            serde_json::from_value::<StoredOutbound>(record.clone())?;
            preferences.set(&stringPreferencesKey(&name), serde_json::to_string(&record)?);
        }
        Ok(())
    }

    fn preferences(&self, path: &str) -> Result<Preferences, String> {
        self.store(path).data().map_err(|error| error.to_string())
    }
    pub(crate) fn records<T: DeserializeOwned>(&self, path: &str) -> Result<BTreeMap<String, T>, String> {
        self.preferences(path)?
            .iterEntries()
            .filter(|(name, _)| *name != PREFERENCES_SCHEMA_VERSION_KEY_NAME)
            .map(|(name, encoded)| {
                let record = serde_json::from_str(encoded)
                    .map_err(|_| format!("Invalid peer state record at {path}, key {name}"))?;
                Ok((name.to_owned(), record))
            })
            .collect()
    }

    /// Inspect only the version before decoding a record; do not build a JSON
    /// Value tree (notably one allocation per secret byte) for every pending.
    pub(crate) fn versionedRecords<T: DeserializeOwned>(&self, path: &str, version: u32) -> Result<BTreeMap<String, T>, String> {
        #[derive(Deserialize)]
        struct Header { version: Option<u32> }
        let prefs = self.preferences(path)?;
        let mut records = BTreeMap::new();
        for (name, encoded) in prefs.iterEntries() {
            if name == PREFERENCES_SCHEMA_VERSION_KEY_NAME { continue; }
            let header: Header = serde_json::from_str(encoded).map_err(|e| e.to_string())?;
            if header.version == Some(version) {
                records.insert(name.to_owned(), serde_json::from_str(encoded).map_err(|e| e.to_string())?);
            }
        }
        Ok(records)
    }

    /// Expiry cleanup and insertion are one durable transaction. Unknown or
    /// malformed records are retained; a failed write leaves the old state intact.
    pub(crate) fn putPending<T: Serialize>(&self, path: &str, id: &str, value: &T, now: i64, capacity: usize) -> Result<(), String> {
        #[derive(Deserialize)]
        struct Header { version: u32, expires: i64 }
        let encoded = serde_json::to_string(value).map_err(|e| e.to_string())?;
        self.store(path).try_edit_result(|prefs| {
            let mut expired = Vec::new();
            let mut active = 0;
            for (name, record) in prefs.iterEntries() {
                if name == PREFERENCES_SCHEMA_VERSION_KEY_NAME { continue; }
                if let Ok(header) = serde_json::from_str::<Header>(record) {
                    if header.version == PAIRING_SERVICE_VERSION && header.expires <= now {
                        expired.push(name.to_owned());
                        continue;
                    }
                }
                if name != id { active += 1; }
            }
            if active >= capacity {
                return Err(PreferencesDataStoreError::Message("Pending pairing capacity reached".into()));
            }
            for name in expired { prefs.remove(&stringPreferencesKey(&name)); }
            prefs.set(&stringPreferencesKey(id), encoded);
            Ok(())
        }).map_err(|e: PreferencesDataStoreError| e.to_string())
    }


    /// Call once at boot, before serving requests on a clock without an epoch.
    /// An uptime-based expiry from a previous boot is not a valid future lease.
    /// Drop only current-version IN-FLIGHT pairings, never saved authorizations.
    pub fn discardPendingPairingsForUnsynchronizedClock(&self, now: i64) -> Result<usize, String> {
        if now >= 1_577_836_800_000 { return Ok(0); } // 2020-01-01, plausible epoch
        #[derive(Deserialize)]
        struct Header { version: Option<u32> }
        let mut removed = 0;
        for path in [RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
            RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH] {
            self.store(path).edit(|prefs| {
                let names = prefs.iterEntries().filter_map(|(name, record)| {
                    if name == PREFERENCES_SCHEMA_VERSION_KEY_NAME { return None; }
                    let header = serde_json::from_str::<Header>(record).ok()?;
                    (header.version == Some(PAIRING_SERVICE_VERSION)).then(|| name.to_owned())
                }).collect::<Vec<_>>();
                removed += names.len();
                for name in names { prefs.remove(&stringPreferencesKey(&name)); }
            }).map_err(|e| e.to_string())?;
        }
        Ok(removed)
    }

    /// 在原 Preferences 文件中提交一个事务/凭证；不创建平行的存储目录。
    pub(crate) fn putRecord<T: Serialize>(&self, path: &str, id: &str, value: &T) -> Result<(), String> {
        let encoded = serde_json::to_string(value).map_err(|e| e.to_string())?;
        self.store(path).edit(|prefs| prefs.set(&stringPreferencesKey(id), encoded))
            .map_err(|e| e.to_string())
    }
    pub(crate) fn deleteRecord(&self, path: &str, id: &str) -> Result<(), String> {
        self.store(path).edit(|prefs| { prefs.remove(&stringPreferencesKey(id)); })
            .map_err(|e| e.to_string())
    }

    /// Refreshes the host-owned platform while preserving the configured name and unknown fields.
    pub fn deviceInfo(&self, supplied: LinkDeviceInfo, replace: bool) -> Result<LinkDeviceInfo, String> {
        use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
        use operit_store::PreferencesDataStore::PreferencesDataStoreError;
        if supplied.platform.trim().is_empty() || supplied.model.trim().is_empty() {
            return Err("Host device platform and model must not be empty".to_string());
        }
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        self.store(RUNTIME_LINK_ACCESS_IDENTITY_PATH).try_edit_result(|preferences| {
            let key = stringPreferencesKey("record");
            let encoded = preferences.get(&key).ok_or_else(||
                PreferencesDataStoreError::Message("Node identity is missing".into()))?;
            let mut record: Value = serde_json::from_str(encoded)?;
            if record.get("deviceId").and_then(Value::as_str) != Some(identity.nodeId.as_str()) {
                return Err(PreferencesDataStoreError::Message("Node identity changed".into()));
            }
            let persisted = record.get("deviceInfo").cloned()
                .map(serde_json::from_value::<LinkDeviceInfo>).transpose()?;
            let mut current = supplied.clone();
            if !replace {
                if let Some(info) = persisted { current.model = info.model; }
            }
            let info = record.as_object_mut().ok_or_else(||
                PreferencesDataStoreError::Message("Node identity is not an object".into()))?
                .entry("deviceInfo").or_insert_with(|| serde_json::json!({}));
            let object = info.as_object_mut().ok_or_else(||
                PreferencesDataStoreError::Message("Device info is not an object".into()))?;
            object.insert("platform".into(), current.platform.clone().into());
            object.insert("model".into(), current.model.clone().into());
            preferences.set(&key, serde_json::to_string(&record)?);
            Ok::<_, PreferencesDataStoreError>(current)
        }).map_err(|error| error.to_string())
    }

    /// Reads listener preferences after their versioned initialization.
    pub fn hostConfig(&self) -> Result<Option<PeerHostConfig>, String> {
        let preferences = self.preferences(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)?;
        decodeHostConfig(&preferences).map(Some)
    }
    /// Reads the listener token persisted by the declared preference schema.
    pub fn localPairingToken(&self) -> Result<String, String> {
        let config = self.hostConfig()?.ok_or("Node listener is not configured")?;
        if config.token.trim().is_empty() {
            return Err("Node pairing token is not configured".into());
        }
        Ok(config.token)
    }

    /// Saves explicit listener settings without removing unrelated preference keys.
    pub fn saveHostConfig(&self, config: &PeerHostConfig) -> Result<(), String> {
        self.store(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
            .try_edit_result(|preferences| writeHostConfigPreferences(preferences, config))
            .map_err(|error: PreferencesDataStoreError| error.to_string())
    }

    /// Rotates the stored listener token in one node-local preference transaction.
    pub fn refreshLocalPairingToken(&self) -> Result<String, String> {
        self.store(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
            .try_edit_result(|preferences| {
                decodeHostConfig(preferences).map_err(PreferencesDataStoreError::Message)?;
                let token = newHostPairingToken()?;
                preferences.set(&stringPreferencesKey("token"), token.clone());
                preferences.set(&stringPreferencesKey("updatedAt"), currentTimeMillis().to_string());
                Ok::<String, PreferencesDataStoreError>(token)
            })
            .map_err(|error: PreferencesDataStoreError| error.to_string())
    }

    /// 从原入站/出站文件分别读取授权，只在 UI 投影中按节点归并，不合并凭证或反向授权。
    pub fn pairedPeers(&self, localNodeId: &str) -> Result<Vec<PairedPeer>, String> {
        let mut peers = BTreeMap::<String, PairedPeer>::new();
        for (_, record) in
            self.records::<StoredInbound>(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH)?
        {
            validateCredential(record.pairingServiceVersion, &record.sessionSecret)?;
            mergePeer(
                &mut peers,
                &record.deviceId,
                &record.deviceInfo,
                StoredDirection::Inbound,
            )?;
        }
        for (_, record) in
            self.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)?
        {
            validateCredential(record.pairingServiceVersion, &record.sessionSecret)?;
            if record.deviceId != localNodeId
                || record.endpoint.is_empty()
                || record.sessionId.is_empty()
                || !matches!(
                    record.transport.as_str(),
                    "http" | "ws" | "tcp" | "serial" | "bluetooth"
                )
            {
                return Err("Invalid outbound peer identity or transport".into());
            }
            mergePeer(
                &mut peers,
                &record.peerNodeId,
                &record.peerDeviceInfo,
                StoredDirection::Outbound,
            )?;
        }
        Ok(peers.into_values().collect())
    }

    /// 设备级撤销的持久化部分：清除两个方向、全部渠道和该设备的待确认事务。
    /// 调用方 runtime 必须串行化配对/撤销并失效内存凭证、关闭连接；跨文件写失败必须报错，
    /// 不能将这里当成跨文件原子事务或在失败后继续授予旧连接访问权。重复调用可安全重试。
    pub fn removePairedPeer(&self, nodeId: &str) -> Result<(), String> {
        if nodeId.trim().is_empty() {
            return Err("Invalid paired node id".into());
        }
        let paths = [
            (RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, "/deviceId"),
            (RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, "/peerNodeId"),
            (RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH, "/clientDeviceId"),
            (
                RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
                "/state/peerNodeId",
            ),
        ];
        // 先验证四份原记录，避免发现损坏数据前就删除其中一个方向。
        let records = paths
            .iter()
            .map(|(path, pointer)| {
                let keys = self
                    .records::<Value>(path)?
                    .into_iter()
                    .filter_map(|(key, record)| {
                        (record.pointer(pointer).and_then(Value::as_str) == Some(nodeId)


                            || (path == &RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH
                                && record.get("peerNodeId").and_then(Value::as_str) == Some(nodeId)))
                            .then_some(key)
                    })
                    .collect::<Vec<_>>();
                Ok((*path, keys))
            })
            .collect::<Result<Vec<_>, String>>()?;
        for (path, keys) in records {
            if keys.is_empty() {
                continue;
            }
            self.store(path)
                .edit(|preferences| {
                    for key in &keys {
                        preferences.remove(&stringPreferencesKey(key));
                    }
                })
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// 原待确认记录保留读取，但旧事务不能绕过新协议 token/配对码校验而直接完成。
    /// 不向 UI 返回这些原始数据：旧出站状态可能包含临时密钥。
    pub(crate) fn pendingRecords(
        &self,
        direction: StoredDirection,
    ) -> Result<BTreeMap<String, Value>, String> {
        self.records(match direction {
            StoredDirection::Inbound => RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
            StoredDirection::Outbound => RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
        })
    }
}
/// Generates a listener credential using the runtime secure random source.
fn newHostPairingToken() -> Result<String, PreferencesDataStoreError> {
    let mut bytes = [0; 32];
    SystemRandom::new().fill(&mut bytes).map_err(|_| {
        PreferencesDataStoreError::Message("Listener token random source failed".into())
    })?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

/// Writes listener fields into the existing node-local preference snapshot.
fn writeHostConfigPreferences(
    preferences: &mut Preferences,
    config: &PeerHostConfig,
) -> Result<(), PreferencesDataStoreError> {
    for (key, value) in [
        ("bindAddress", config.bindAddress.clone()),
        ("token", config.token.clone()),
        ("discoveryEnabled", config.discoveryEnabled.to_string()),
        ("transports", serde_json::to_string(&config.transports)?),
        ("portMode", match config.portMode {
            PeerHostPortMode::Automatic => "automatic",
            PeerHostPortMode::Fixed => "fixed",
        }.into()),
        ("updatedAt", config.updatedAt.to_string()),
    ] {
        preferences.set(&stringPreferencesKey(key), value);
    }
    Ok(())
}

/// Decodes all listener fields strictly after preference schema migration.
fn decodeHostConfig(preferences: &Preferences) -> Result<PeerHostConfig, String> {
    let get = |key| {
        preferences
            .get(&stringPreferencesKey(key))
            .cloned()
            .ok_or_else(|| format!("Peer host config is missing {key}"))
    };
    let parseBool = |key| {
        get(key)?
            .parse::<bool>()
            .map_err(|_| format!("Invalid peer host config field {key}"))
    };
    Ok(PeerHostConfig {
        bindAddress: get("bindAddress")?,
        token: get("token")?,
        discoveryEnabled: parseBool("discoveryEnabled")?,
        transports: serde_json::from_str(&get("transports")?)
            .map_err(|error| format!("Invalid peer host config field transports: {error}"))?,
        portMode: match get("portMode")?.as_str() {
            "automatic" => PeerHostPortMode::Automatic,
            "fixed" => PeerHostPortMode::Fixed,
            _ => return Err("Invalid peer host config field portMode".into()),
        },
        updatedAt: get("updatedAt")?
            .parse()
            .map_err(|_| "Invalid peer host config field updatedAt".to_string())?,
    })
}
fn validateCredential(version: u32, secret: &str) -> Result<(), String> {
    if version <= 0
        || BASE64
            .decode(secret)
            .map_err(|_| "Invalid peer credential encoding")?
            .len()
            != 32
    {
        return Err("Invalid peer credential version or length".into());
    }
    Ok(())
}
fn mergePeer(
    peers: &mut BTreeMap<String, PairedPeer>,
    nodeId: &str,
    info: &LinkDeviceInfo,
    direction: StoredDirection,
) -> Result<(), String> {
    if nodeId.trim().is_empty() {
        return Err("Invalid paired node id".into());
    }
    let peer = peers.entry(nodeId.into()).or_insert_with(|| PairedPeer {
        nodeId: nodeId.into(),
        displayName: info.displayName(),
        inbound: false,
        outbound: false,
    });
    if peer.displayName != info.displayName() {
        return Err("Conflicting paired device metadata".into());
    }
    match direction {
        StoredDirection::Inbound => peer.inbound = true,
        StoredDirection::Outbound => peer.outbound = true,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::{HostError, HostResult, RuntimeStorageEntry};
    use operit_store::PreferencesDataStore::emptyPreferences;
    use std::sync::Mutex;
    #[derive(Default)]
    struct Storage(Mutex<BTreeMap<String, Vec<u8>>>, std::sync::atomic::AtomicBool);
    impl RuntimeStorageHost for Storage {
        fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
            None
        }
        fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
            None
        }
        fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            self.0
                .lock()
                .unwrap()
                .get(path)
                .cloned()
                .ok_or_else(|| HostError::new("missing"))
        }
        fn writeBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
            if self.1.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(HostError::new("injected write failure"));
            }
            self.0.lock().unwrap().insert(path.into(), bytes.into());
            Ok(())
        }
        fn appendBytes(&self, _: &str, _: &[u8]) -> HostResult<()> {
            Err(HostError::new("unused"))
        }
        fn delete(&self, path: &str, _: bool) -> HostResult<()> {
            self.0.lock().unwrap().remove(path);
            Ok(())
        }
        fn exists(&self, path: &str) -> HostResult<bool> {
            Ok(self.0.lock().unwrap().contains_key(path))
        }
        fn list(&self, _: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
            Ok(vec![])
        }
    }
    fn writeRecord(store: &PeerStateStore, path: &str, key: &str, record: Value) {
        let mut preferences = emptyPreferences();
        preferences.set(&stringPreferencesKey(key), record.to_string());
        CoreNodeStateStore::newWithStorage(store.storage.clone(), path).replace(preferences).unwrap();
    }
    #[test]
    fn unsynchronized_boot_discards_only_inflight_pairings_and_keeps_credentials() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        store.putRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, "keep", &serde_json::json!({"credential":true})).unwrap();
        store.putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, "keep", &serde_json::json!({"credential":true})).unwrap();
        let inbound = storage.readBytes(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).unwrap();
        let outbound = storage.readBytes(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).unwrap();
        for path in [RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH, RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH] {
            store.putRecord(path, "stale", &serde_json::json!({"version":1,"expires":999999})).unwrap();
            store.putRecord(path, "future", &serde_json::json!({"version":2,"expires":999999})).unwrap();
        }
        assert_eq!(store.discardPendingPairingsForUnsynchronizedClock(1_800_000_000_000).unwrap(), 0);
        assert_eq!(store.discardPendingPairingsForUnsynchronizedClock(10).unwrap(), 2);
        for path in [RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH, RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH] {
            assert_eq!(store.records::<Value>(path).unwrap().keys().map(String::as_str).collect::<Vec<_>>(), ["future"]);
        }
        assert_eq!(storage.readBytes(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).unwrap(), inbound);
        assert_eq!(storage.readBytes(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).unwrap(), outbound);
    }

    #[test]
    fn repeated_expired_pairings_remain_bounded_and_keep_credentials() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let path = RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH;
        store.putRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, "credential", &serde_json::json!({"keep":true})).unwrap();
        let credential = storage.readBytes(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).unwrap();
        for now in 0..100 {
            store.putPending(path, &format!("pending-{now}"), &serde_json::json!({"version":1,"expires":now+1}), now, 2).unwrap();
            assert_eq!(store.records::<Value>(path).unwrap().len(), 1);
        }
        assert_eq!(storage.readBytes(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).unwrap(), credential);
    }

    #[test]
    fn pending_capacity_and_failed_commit_leave_old_state_unchanged() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let path = RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH;
        let pending = serde_json::json!({"version":1,"expires":10});
        store.putPending(path, "a", &pending, 0, 1).unwrap();
        let before = storage.readBytes(path).unwrap();
        assert!(store.putPending(path, "b", &pending, 0, 1).is_err());
        storage.1.store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(store.putPending(path, "b", &pending, 10, 1).is_err());
        assert_eq!(storage.readBytes(path).unwrap(), before);
        assert!(store.records::<Value>(path).unwrap().contains_key("a"));
        assert!(!store.records::<Value>(path).unwrap().contains_key("b"));
    }

    #[test]
    fn cleanup_retains_unknown_and_malformed_records_without_value_tree() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        let path = RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH;
        store.putRecord(path, "future", &serde_json::json!({"version":2,"expires":0})).unwrap();
        store.putRecord(path, "old", &serde_json::json!({"version":1,"expires":0})).unwrap();
        store.putRecord(path, "legacy", &serde_json::json!({"old":"unchanged"})).unwrap();
        store.putPending(path, "new", &serde_json::json!({"version":1,"expires":100}), 1, 4).unwrap();
        assert_eq!(store.records::<Value>(path).unwrap().keys().map(String::as_str).collect::<Vec<_>>(), ["future", "legacy", "new"]);
        assert_eq!(store.versionedRecords::<Value>(path, 1).unwrap().keys().map(String::as_str).collect::<Vec<_>>(), ["new"]);
    }

    #[test]
    fn original_pairing_credentials_are_used_without_repairing() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        let secret = BASE64.encode([9; 32]);
        writeRecord(&store, RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, "in-session", serde_json::json!({
            "deviceId": "peer", "deviceInfo": {"platform":"test", "model":"peer"},
            "pairingServiceVersion": 1, "sessionSecret": secret
        }));
        writeRecord(&store, RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, "saved-name", serde_json::json!({
            "baseUrl":"http://peer:37194", "sessionId":"out-session", "deviceId":"local",
            "coreDeviceId":"peer", "remoteDeviceInfo":{"platform":"test", "model":"peer"},
            "pairingServiceVersion":1, "sessionSecret":secret
        }));
        let inbound = store.records::<StoredInbound>(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).unwrap();
        assert_eq!(inbound["in-session"].sessionSecret, secret);
        let outbound = store.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).unwrap();
        let record = &outbound["saved-name"];
        assert_eq!(record.pairingServiceVersion, PAIRING_SERVICE_VERSION);
        assert_eq!(record.sessionId, "out-session");
        assert_eq!(record.sessionSecret, secret);
        assert_eq!(record.transport, "http");
        let encoded = serde_json::to_value(record).unwrap();
        assert_eq!(encoded["peerNodeId"], "peer");
        assert!(encoded.get("coreDeviceId").is_none());
        let peers = store.pairedPeers("local").unwrap();
        assert_eq!(peers.len(), 1);
        assert!(peers[0].inbound && peers[0].outbound);
        store.removePairedPeer("peer").unwrap();
        assert!(store.pairedPeers("local").unwrap().is_empty());
    }

    #[test]
    fn outbound_schema_migration_is_persisted_once_and_preserves_unknown_fields() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        writeRecord(&store, RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, "named", serde_json::json!({
            "baseUrl":"http://peer:37194", "sessionId":"session", "deviceId":"local",
            "coreDeviceId":"peer", "remoteDeviceInfo":{"platform":"test", "model":"peer"},
            "pairingServiceVersion":1, "sessionSecret":BASE64.encode([8;32]), "future":42
        }));
        assert_eq!(store.store(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).schemaVersion().unwrap(), 1);
        let record = store.records::<Value>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).unwrap().remove("named").unwrap();
        assert_eq!(record["endpoint"], "http://peer:37194");
        assert_eq!(record["peerNodeId"], "peer");
        assert_eq!(record["future"], 42);
        assert!(record.get("baseUrl").is_none());
        assert!(record.get("coreDeviceId").is_none());
        assert!(record.get("remoteDeviceInfo").is_none());
        let before = storage.0.lock().unwrap().clone();
        let reopened = PeerStateStore::new(storage.clone());
        reopened.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).unwrap();
        assert_eq!(before, *storage.0.lock().unwrap());
        // Current-schema malformed data must fail, not receive a permanent serde fallback.
        let mut preferences = store.preferences(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).unwrap();
        let mut invalid = record;
        invalid.as_object_mut().unwrap().remove("transport");
        preferences.set(&stringPreferencesKey("named"), invalid.to_string());
        store.store(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).replace(preferences).unwrap();
        assert!(store.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).is_err());
    }

    #[test]
    fn failed_outbound_migration_does_not_rewrite_or_advance_schema() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        writeRecord(&store, RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, "conflict", serde_json::json!({
            "baseUrl":"old", "endpoint":"different"
        }));
        let before = storage.0.lock().unwrap().clone();
        assert!(store.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).is_err());
        assert_eq!(before, *storage.0.lock().unwrap());
    }

    #[test]
    fn outbound_migration_reports_missing_steps_and_rejects_newer_schemas() {
        let mut preferences = emptyPreferences();
        assert!(matches!(
            PeerStateStore::migrateOutboundPreferences(1, &mut preferences),
            Err(PreferencesDataStoreError::MissingMigration { from: 1, to: 2 })
        ));
        let storage = Arc::new(Storage::default());
        preferences.set(&stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME), "2".into());
        CoreNodeStateStore::newWithStorage(storage.clone(), RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
            .replace(preferences).unwrap();
        let before = storage.0.lock().unwrap().clone();
        let store = PeerStateStore::new(storage.clone());
        assert!(matches!(
            store.store(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).data(),
            Err(PreferencesDataStoreError::SchemaVersionTooNew { actual: 2, expected: 1 })
        ));
        assert_eq!(before, *storage.0.lock().unwrap());
    }

    /// Updates host-owned platform metadata without replacing the configured name or unknown fields.
    #[test]
    fn original_identity_and_unknown_fields_survive_device_info_updates() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        writeRecord(&store, RUNTIME_LINK_ACCESS_IDENTITY_PATH, "record", serde_json::json!({
            "deviceId": "stable-node", "future": "preserved",
            "deviceInfo": { "platform": "old", "model": "old-name", "extra": 7 }
        }));
        let supplied = LinkDeviceInfo { platform: "new".into(), model: "new-name".into() };
        let initialized = store.deviceInfo(supplied.clone(), false).unwrap();
        assert_eq!(initialized.model, "old-name");
        assert_eq!(initialized.platform, "new");
        assert_eq!(store.deviceInfo(supplied, true).unwrap().model, "new-name");
        let records = store.records::<Value>(RUNTIME_LINK_ACCESS_IDENTITY_PATH).unwrap();
        assert_eq!(records["record"]["deviceId"], "stable-node");
        assert_eq!(records["record"]["future"], "preserved");
        assert_eq!(records["record"]["deviceInfo"]["extra"], 7);
        writeRecord(&store, RUNTIME_LINK_ACCESS_IDENTITY_PATH, "record", serde_json::json!({
            "deviceId": "stable-node", "deviceInfo": "broken"
        }));
        let before = store.preferences(RUNTIME_LINK_ACCESS_IDENTITY_PATH).unwrap().entries();
        assert!(store.deviceInfo(LinkDeviceInfo { platform: "new".into(), model: "name".into() }, true).is_err());
        assert_eq!(before, store.preferences(RUNTIME_LINK_ACCESS_IDENTITY_PATH).unwrap().entries());
    }

    /// Refreshes a previously empty runtime platform without changing identity or the device name.
    #[test]
    fn startup_refreshes_platform_without_replacing_persisted_device_name() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        writeRecord(&store, RUNTIME_LINK_ACCESS_IDENTITY_PATH, "record", serde_json::json!({
            "deviceId": "stable-node", "future": "preserved",
            "deviceInfo": { "platform": "", "model": "wasm32", "extra": 7 }
        }));
        let supplied = LinkDeviceInfo { platform: "web".into(), model: "Chrome 154".into() };
        let current = store.deviceInfo(supplied.clone(), false).unwrap();
        assert_eq!(current.platform, "web");
        assert_eq!(current.model, "wasm32");
        let records = store.records::<Value>(RUNTIME_LINK_ACCESS_IDENTITY_PATH).unwrap();
        assert_eq!(records["record"]["deviceId"], "stable-node");
        assert_eq!(records["record"]["future"], "preserved");
        assert_eq!(records["record"]["deviceInfo"]["platform"], "web");
        assert_eq!(records["record"]["deviceInfo"]["extra"], 7);
        let before = store.preferences(RUNTIME_LINK_ACCESS_IDENTITY_PATH).unwrap().entries();
        assert_eq!(store.deviceInfo(supplied, false).unwrap(), current);
        assert_eq!(before, store.preferences(RUNTIME_LINK_ACCESS_IDENTITY_PATH).unwrap().entries());
    }

    /// Rejects incomplete host metadata before creating or modifying identity preferences.
    #[test]
    fn empty_host_device_metadata_never_changes_identity_storage() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let invalid = [
            LinkDeviceInfo { platform: "".into(), model: "wasm32".into() },
            LinkDeviceInfo { platform: "web".into(), model: " ".into() },
        ];
        for supplied in &invalid {
            assert!(store.deviceInfo(supplied.clone(), false).is_err());
            assert!(storage.0.lock().unwrap().is_empty());
        }
        writeRecord(&store, RUNTIME_LINK_ACCESS_IDENTITY_PATH, "record", serde_json::json!({
            "deviceId": "stable-node", "deviceInfo": { "platform": "web", "model": "My browser" }
        }));
        let before = storage.0.lock().unwrap().clone();
        for supplied in invalid {
            assert!(store.deviceInfo(supplied, true).is_err());
            assert_eq!(before, *storage.0.lock().unwrap());
        }
    }

    /// Verifies first reads initialize listener defaults exactly once through the schema.
    #[test]
    fn listener_schema_initializes_http_websocket_discovery_and_fixed_binding_once() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let first = store.hostConfig().unwrap().unwrap();
        assert_eq!(first.bindAddress, "0.0.0.0:37195");
        assert_eq!(first.transports, vec![PeerTransport::Http, PeerTransport::WebSocket]);
        assert!(first.discoveryEnabled);
        assert_eq!(first.portMode, PeerHostPortMode::Fixed);
        assert_eq!(base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&first.token).unwrap().len(), 32);
        assert_eq!(store.store(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH).schemaVersion().unwrap(), 1);
        let before = storage.0.lock().unwrap().clone();
        let second = PeerStateStore::new(storage.clone()).hostConfig().unwrap().unwrap();
        assert_eq!(first.token, second.token);
        assert_eq!(first.updatedAt, second.updatedAt);
        assert_eq!(before, *storage.0.lock().unwrap());
    }

    /// Verifies simultaneous first reads share one persisted listener credential.
    #[test]
    fn concurrent_listener_reads_share_the_schema_initialization_transaction() {
        let storage = Arc::new(Storage::default());
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let workers = (0..8).map(|_| {
            let storage = storage.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let store = PeerStateStore::new(storage);
                barrier.wait();
                store.hostConfig().unwrap().unwrap().token
            })
        }).collect::<Vec<_>>();
        let tokens = workers.into_iter().map(|worker| worker.join().unwrap()).collect::<Vec<_>>();
        assert!(tokens.iter().all(|token| token == &tokens[0]));
        assert_eq!(storage.0.lock().unwrap().len(), 1);
    }

    /// Verifies token rotation only changes authentication data and its update timestamp.
    #[test]
    fn listener_token_rotation_preserves_binding_transports_discovery_and_unknown_keys() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let mut config = store.hostConfig().unwrap().unwrap();
        config.bindAddress = "127.0.0.1:48123".into();
        config.transports = vec![PeerTransport::Tcp];
        config.discoveryEnabled = false;
        store.saveHostConfig(&config).unwrap();
        store.store(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH).edit(|preferences| {
            preferences.set(&stringPreferencesKey("extra"), "preserved".into());
        }).unwrap();
        let before = store.preferences(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH).unwrap();
        let newToken = store.refreshLocalPairingToken().unwrap();
        assert_ne!(newToken, config.token);
        assert_eq!(base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&newToken).unwrap().len(), 32);
        let after = store.preferences(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH).unwrap();
        for (key, value) in before.entries() {
            if key != "token" && key != "updatedAt" {
                assert_eq!(after.get(&stringPreferencesKey(&key)), Some(&value));
            }
        }
        assert_eq!(after.get(&stringPreferencesKey("token")), Some(&newToken));
        assert_eq!(store.hostConfig().unwrap().unwrap().portMode, PeerHostPortMode::Fixed);
    }

    /// Verifies missing current-schema fields and unknown future versions fail unchanged.
    #[test]
    fn listener_schema_rejects_incomplete_current_and_newer_preferences() {
        for version in [1, 2] {
            let storage = Arc::new(Storage::default());
            let mut preferences = emptyPreferences();
            preferences.set(&stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME), version.to_string());
            preferences.set(&stringPreferencesKey("token"), "original-token".into());
            CoreNodeStateStore::newWithStorage(storage.clone(), RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
                .replace(preferences).unwrap();
            let before = storage.0.lock().unwrap().clone();
            let store = PeerStateStore::new(storage.clone());
            assert!(store.hostConfig().is_err());
            assert!(store.refreshLocalPairingToken().is_err());
            assert_eq!(before, *storage.0.lock().unwrap());
        }
    }

    /// Verifies that reading a token preserves initialized listener preferences.
    #[test]
    fn local_token_reads_the_persisted_configuration_without_rewriting_it() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        let initialized = store.hostConfig().unwrap().unwrap();
        assert_eq!(store.localPairingToken().unwrap(), initialized.token);
        let mut config = PeerHostConfig {
            bindAddress: "0.0.0.0:37194".into(), token: "saved-token".into(),
            discoveryEnabled: false, transports: vec![PeerTransport::Tcp],
            portMode: PeerHostPortMode::Fixed, updatedAt: 12,
        };
        store.saveHostConfig(&config).unwrap();
        let before = store.preferences(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH).unwrap().entries();
        assert_eq!(store.localPairingToken().unwrap(), "saved-token");
        assert_eq!(before, store.preferences(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH).unwrap().entries());
        config.token = " ".into();
        store.saveHostConfig(&config).unwrap();
        assert!(store.localPairingToken().is_err());
    }

    /// Verifies version-zero listener preferences retain explicit settings and unknown fields.
    #[test]
    fn original_host_config_is_read_and_updated_in_place_without_losing_fields() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let mut preferences = emptyPreferences();
        for (key, value) in [
            ("bindAddress", "127.0.0.1:37194"),
            ("token", "old-token"),
            ("webAccessEnabled", "true"),
            ("discoveryEnabled", "false"),
            ("portMode", "fixed"),
            ("updatedAt", "123"),
            ("extra", "keep"),
        ] {
            preferences.set(&stringPreferencesKey(key), value.into());
        }
        CoreNodeStateStore::newWithStorage(storage.clone(), RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
            .replace(preferences)
            .unwrap();
        let mut config = store.hostConfig().unwrap().unwrap();
        let before = storage.0.lock().unwrap().clone();
        store.hostConfig().unwrap().unwrap();
        assert!(config.transports.is_empty(), "obsolete flags must not enable listeners");
        assert_eq!(config.token, "old-token");
        assert_eq!(config.portMode, PeerHostPortMode::Fixed);
        assert_eq!(
            storage.0.lock().unwrap().clone(),
            before,
            "reading must not rewrite original files"
        );
        config.discoveryEnabled = true;
        store.saveHostConfig(&config).unwrap();
        assert_eq!(
            store
                .preferences(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
                .unwrap()
                .get(&stringPreferencesKey("extra"))
                .unwrap(),
            "keep"
        );
    }
    #[test]
    fn original_directional_records_do_not_create_reverse_authorization() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,
            "session",
            serde_json::json!({
                "deviceId":"incoming", "deviceInfo":{"platform":"test","model":"one"}, "pairingServiceVersion":1, "sessionSecret":BASE64.encode([1;32])
            }),
        );
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
            "saved-name",
            serde_json::json!({
                "endpoint":"tcp-address", "sessionId":"other-session", "deviceId":"local", "peerNodeId":"outgoing", "peerDeviceInfo":{"platform":"test","model":"two"}, "pairingServiceVersion":1, "sessionSecret":BASE64.encode([2;32]), "transport":"tcp"
            }),
        );
        let peers = store.pairedPeers("local").unwrap();
        assert!(peers[0].inbound && !peers[0].outbound);
        assert!(!peers[1].inbound && peers[1].outbound);
        assert!(store.pairedPeers("wrong-local").is_err());
    }
    #[test]
    fn device_revocation_clears_both_directions_all_channels_and_pending_transactions() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        for (path, record) in [
            (
                RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,
                serde_json::json!({"deviceId":"board"}),
            ),
            (
                RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
                serde_json::json!({"peerNodeId":"board"}),
            ),
            (
                RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
                serde_json::json!({"clientDeviceId":"board"}),
            ),
            (
                RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
                serde_json::json!({"state":{"peerNodeId":"board"}}),
            ),
        ] {
            let other = serde_json::json!({"deviceId":"other", "peerNodeId":"other", "clientDeviceId":"other", "state":{"peerNodeId":"other"}});
            store
                .store(path)
                .edit(|preferences| {
                    preferences.set(&stringPreferencesKey("channel-a"), record.to_string());
                    preferences.set(&stringPreferencesKey("channel-b"), record.to_string());
                    preferences.set(&stringPreferencesKey("other"), other.to_string());
                })
                .unwrap();
        }
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_IDENTITY_PATH,
            "record",
            serde_json::json!({"deviceId":"local"}),
        );
        let identity = storage
            .readBytes(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
            .unwrap();
        store.removePairedPeer("board").unwrap();
        store.removePairedPeer("board").unwrap();
        for path in [
            RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,
            RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
            RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
            RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
        ] {
            assert_eq!(
                store
                    .records::<Value>(path)
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>(),
                ["other"]
            );
        }
        assert_eq!(
            storage
                .readBytes(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
                .unwrap(),
            identity
        );
    }

    /// Verifies invalid listener preferences fail without modifying stored records.
    #[test]
    fn malformed_config_is_not_replaced_and_pending_records_are_retained() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let mut p = emptyPreferences();
        p.set(&stringPreferencesKey("token"), "secret".into());
        CoreNodeStateStore::newWithStorage(storage.clone(), RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
            .replace(p)
            .unwrap();
        for (direction, path) in [
            (
                StoredDirection::Inbound,
                RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
            ),
            (
                StoredDirection::Outbound,
                RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
            ),
        ] {
            writeRecord(
                &store,
                path,
                "pending",
                serde_json::json!({"old-state":"retained"}),
            );
            assert_eq!(
                store.pendingRecords(direction).unwrap()["pending"]["old-state"],
                "retained"
            );
        }
        let before = storage.0.lock().unwrap().clone();
        assert!(store.hostConfig().is_err());
        assert_eq!(*storage.0.lock().unwrap(), before);
    }
}
