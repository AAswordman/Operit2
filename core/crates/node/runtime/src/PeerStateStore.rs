//! 节点通信持久化。沿用原 link_access 路径和 Preferences 格式，不恢复旧握手或 HTTP 接口。
use crate::NodeServices::{PairedPeer, PeerTransport};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use operit_host_api::RuntimeStorageHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_link::protocol::LinkDeviceInfo;
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, CoreNodeStateStore, Preferences, PreferencesDataStoreError,
    PREFERENCES_SCHEMA_VERSION_KEY_NAME,
};
use operit_util::RuntimeStorageLayout::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy)]
pub(crate) enum StoredDirection {
    Inbound,
    Outbound,
}

/// Stores node-local listener binding and discovery preferences.
#[derive(Clone, Serialize, Deserialize)]
pub struct PeerHostConfig {
    /// Network bind address. Port zero requests Host allocation; automatic mode
    /// persists the selected port for subsequent reconnects and restarts.
    pub bindAddress: String,
    /// 显式暴露的传输方式；空列表不启动监听。
    #[serde(default)]
    pub transports: Vec<operit_peer_link::PeerTransport>,
    pub discoveryEnabled: bool,
    /// Automatic reuses an assigned port or requests a new one on collision.
    /// Fixed requires a nonzero port and never changes it after a bind failure.
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
    const OUTBOUND_PREFERENCES_VERSION: u32 = 2;
    const HOST_CONFIG_PREFERENCES_VERSION: u32 = 4;

    /// Creates the node-local peer preference store over the supplied storage host.
    pub fn new(storage: Arc<dyn RuntimeStorageHost>) -> Self {
        Self { storage }
    }
    /// Opens peer preferences with their declared storage schema.
    fn store(&self, path: &str) -> CoreNodeStateStore {
        if path == RUNTIME_LINK_ACCESS_IDENTITY_PATH {
            return operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(
                self.storage.clone(),
            )
            .dataStore();
        }
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
            store.withSchema(2, |version, preferences| match version {
                0 => Ok(()),
                1 => {
                    for (key, value) in preferences.entries() {
                        if key == PREFERENCES_SCHEMA_VERSION_KEY_NAME {
                            continue;
                        }
                        let encoded = serde_json::from_value::<String>(value)?;
                        let record: Value = serde_json::from_str(&encoded)?;
                        preferences.set(&stringPreferencesKey(&key), record);
                    }
                    Ok(())
                }
                from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
            })
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
                        bindAddress: "0.0.0.0:0".into(),
                        transports: vec![PeerTransport::Http, PeerTransport::WebSocket],
                        discoveryEnabled: true,
                        portMode: PeerHostPortMode::Automatic,
                        updatedAt: currentTimeMillis(),
                    };
                    for (key, value) in [
                        ("bindAddress", initial.bindAddress),
                        ("transports", serde_json::to_string(&initial.transports)?),
                        ("discoveryEnabled", initial.discoveryEnabled.to_string()),
                        ("portMode", "automatic".to_owned()),
                        ("updatedAt", initial.updatedAt.to_string()),
                    ] {
                        preferences.set(&stringPreferencesKey(key), value);
                    }
                } else {
                    // Legacy exposure flags do not declare any active transport.
                    if preferences
                        .get(&stringPreferencesKey("transports"))
                        .is_none()
                    {
                        preferences.set(&stringPreferencesKey("transports"), "[]".to_owned());
                    }
                }
                Ok(())
            }
            1 => {
                for key in ["transports", "discoveryEnabled", "updatedAt"] {
                    let key = stringPreferencesKey(key);
                    let encoded = preferences.getTyped::<String>(&key)?.ok_or_else(|| {
                        PreferencesDataStoreError::Message(format!(
                            "Missing peer host field: {}",
                            key.name
                        ))
                    })?;
                    preferences.set(&key, serde_json::from_str::<Value>(&encoded)?);
                }
                decodeHostConfig(preferences).map_err(PreferencesDataStoreError::Message)?;
                Ok(())
            }
            2 => {
                decodeHostConfig(preferences).map_err(PreferencesDataStoreError::Message)?;
                preferences.remove(&stringPreferencesKey("token"));
                Ok(())
            }
            3 => {
                let config = decodeHostConfig(preferences)
                    .map_err(PreferencesDataStoreError::Message)?;
                // Old application and ESP32 defaults were persisted as fixed.
                // Keep custom bindings, disabled discovery and transport choices.
                let legacyDefault = config.discoveryEnabled
                    && config.portMode == PeerHostPortMode::Fixed
                    && ((config.bindAddress == "0.0.0.0:37195"
                        && config.transports == [PeerTransport::Http, PeerTransport::WebSocket])
                        || (config.bindAddress == "0.0.0.0:8765"
                            && (config.transports == [PeerTransport::Serial, PeerTransport::Tcp]
                                || config.transports == [PeerTransport::Serial])));
                if legacyDefault {
                    preferences.set(&stringPreferencesKey("bindAddress"), "0.0.0.0:0");
                    preferences.set(&stringPreferencesKey("portMode"), "automatic");
                }
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        }
    }

    /// Migrates outbound preferences one schema version at a time.
    fn migrateOutboundPreferences(
        version: u32,
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        match version {
            0 => Self::normalizeOutboundSessionFields(preferences),
            1 => {
                for (key, value) in preferences.entries() {
                    if key == PREFERENCES_SCHEMA_VERSION_KEY_NAME {
                        continue;
                    }
                    let encoded = serde_json::from_value::<String>(value)?;
                    let record: Value = serde_json::from_str(&encoded)?;
                    serde_json::from_value::<StoredOutbound>(record.clone())?;
                    preferences.set(&stringPreferencesKey(&key), record);
                }
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
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
            let mut record: Value =
                serde_json::from_str(&serde_json::from_value::<String>(encoded)?)?;
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
            preferences.set(
                &stringPreferencesKey(&name),
                serde_json::to_string(&record)?,
            );
        }
        Ok(())
    }

    /// Reads the complete preference object for schema and multi-field mutations.
    fn preferences(&self, path: &str) -> Result<Preferences, String> {
        self.store(path).data().map_err(|error| error.to_string())
    }
    /// Deletes old approval state without decoding it; also invalidates shared snapshots.
    pub(crate) fn clearSpaceApprovals(&self) -> Result<(), String> {
        let mut errors = Vec::new();
        for path in RUNTIME_SPACE_APPROVAL_STATE_PATHS {
            if let Err(error) = self.store(path).delete() { errors.push(format!("{path}: {error}")); }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
    }

    /// Reads only the requested record; another broken request has no bearing on this one.

    /// Read-only enumeration isolates malformed rows. Security cleanup uses the strict reader below.
    pub(crate) fn independentRecords<T: DeserializeOwned>(&self, path: &str) -> Result<BTreeMap<String, T>, String> {
        let mut records = BTreeMap::new();
        self.store(path).visitEncodedEntries(|name, encoded| {
            match serde_json::from_str(encoded.get()) {
                Ok(record) => { records.insert(name.to_owned(), record); },
                Err(error) => { operit_util::AppLogger::AppLogger::w("PeerStateStore",
                    &format!("Invalid peer state record at {path}, key {name}: {error}")); },
            }
            Ok(())
        }).map_err(|error| error.to_string())?;
        Ok(records)
    }

    pub(crate) fn records<T: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<BTreeMap<String, T>, String> {
        self.store(path).typedEntries::<T>()
            .map(|entries| entries.into_iter().collect())
            .map_err(|error| format!("Invalid peer state records at {path}: {error}"))
    }

    /// Decode a scan without the oversized tree nodes of large typed records.
    pub(crate) fn recordValues<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>, String> {
        self.store(path).typedEntries::<T>()
            .map(|entries| entries.into_iter().map(|(_, record)| record).collect())
            .map_err(|error| error.to_string())
    }

    /// Re-read one authorization from the current snapshot without decoding
    /// unrelated credentials or cloning their JSON trees.
    pub(crate) fn record<T: DeserializeOwned>(
        &self,
        path: &str,
        id: &str,
    ) -> Result<Option<T>, String> {
        self.store(path).typedEntry(id)
            .map_err(|error| format!("Invalid peer state record at {path}, key {id}: {error}"))
    }

    /// Inspect only the version before decoding a record; do not build a JSON
    /// Value tree (notably one allocation per secret byte) for every pending.
    pub(crate) fn versionedRecords<T: DeserializeOwned>(
        &self,
        path: &str,
        version: u32,
    ) -> Result<BTreeMap<String, T>, String> {
        #[derive(Deserialize)]
        struct Header {
            version: Option<u32>,
        }
        self.store(path).withEncodedEntries(|entries| {
            let mut records = BTreeMap::new();
            for (name, encoded) in entries {
                let bytes = encoded.get().as_bytes();
                let header: Header = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
                if header.version == Some(version) {
                    let record = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
                    records.insert(name, record);
                }
            }
            Ok(records)
        }).map_err(|error| error.to_string())?
    }

    /// Expiry cleanup and insertion are one durable transaction. Unknown or
    /// malformed records are retained; a failed write leaves the old state intact.
    pub(crate) fn putPending<T: Serialize>(
        &self,
        path: &str,
        id: &str,
        value: &T,
        now: i64,
        capacity: usize,
    ) -> Result<(), String> {
        #[derive(Deserialize)]
        struct Header {
            version: u32,
            expires: i64,
        }
        let encoded = serde_json::to_value(value).map_err(|e| e.to_string())?;
        self.store(path)
            .try_edit_result(|prefs| {
                let mut expired = Vec::new();
                let mut active = 0;
                for (name, record) in prefs.iterEntries() {
                    if name == PREFERENCES_SCHEMA_VERSION_KEY_NAME {
                        continue;
                    }
                    if let Ok(header) = Header::deserialize(record) {
                        if header.version == PAIRING_SERVICE_VERSION && header.expires <= now {
                            expired.push(name.to_owned());
                            continue;
                        }
                    }
                    if name != id {
                        active += 1;
                    }
                }
                if active >= capacity {
                    return Err(PreferencesDataStoreError::Message(
                        "Pending pairing capacity reached".into(),
                    ));
                }
                for name in expired {
                    prefs.remove(&stringPreferencesKey(&name));
                }
                prefs.set(&stringPreferencesKey(id), encoded);
                Ok(())
            })
            .map_err(|e: PreferencesDataStoreError| e.to_string())
    }

    /// Call once at boot, before serving requests on a clock without an epoch.
    /// An uptime-based expiry from a previous boot is not a valid future lease.
    /// Drop only current-version IN-FLIGHT pairings, never saved authorizations.
    pub fn discardPendingPairingsForUnsynchronizedClock(&self, now: i64) -> Result<usize, String> {
        if now >= 1_577_836_800_000 {
            return Ok(0);
        } // 2020-01-01, plausible epoch
        #[derive(Deserialize)]
        struct Header {
            version: Option<u32>,
        }
        let mut removed = 0;
        for path in [
            RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
            RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
        ] {
            self.store(path)
                .edit(|prefs| {
                    let names = prefs
                        .iterEntries()
                        .filter_map(|(name, record)| {
                            if name == PREFERENCES_SCHEMA_VERSION_KEY_NAME {
                                return None;
                            }
                            let header = Header::deserialize(record).ok()?;
                            (header.version == Some(PAIRING_SERVICE_VERSION))
                                .then(|| name.to_owned())
                        })
                        .collect::<Vec<_>>();
                    removed += names.len();
                    for name in names {
                        prefs.remove(&stringPreferencesKey(&name));
                    }
                })
                .map_err(|e| e.to_string())?;
        }
        Ok(removed)
    }

    /// 在原 Preferences 文件中提交一个事务/凭证；不创建平行的存储目录。
    pub(crate) fn putRecord<T: Serialize>(
        &self,
        path: &str,
        id: &str,
        value: &T,
    ) -> Result<(), String> {
        self.store(path)
            .setTypedEntry(id, value)
            .map_err(|e| e.to_string())
    }
    /// Mutable join receipts may normalize old whitespace together, so a
    /// partially upgraded file does not compress worse than either version.
    pub(crate) fn putCompactRecord<T: Serialize>(&self, path: &str, id: &str, value: &T) -> Result<(), String> {
        self.store(path).setTypedEntryCompactingJson(id, value).map_err(|e| e.to_string())
    }
    pub(crate) fn deleteRecord(&self, path: &str, id: &str) -> Result<(), String> {
        self.store(path).removeEntry(id).map_err(|e| e.to_string())
    }

    /// Refreshes the host-owned platform while preserving the configured name and unknown fields.
    pub fn deviceInfo(
        &self,
        supplied: LinkDeviceInfo,
        replace: bool,
    ) -> Result<LinkDeviceInfo, String> {
        use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
        use operit_store::PreferencesDataStore::PreferencesDataStoreError;
        if supplied.platform.trim().is_empty() || supplied.model.trim().is_empty() {
            return Err("Host device platform and model must not be empty".to_string());
        }
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        self.store(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
            .try_edit_result(|preferences| {
                let key = stringPreferencesKey("record");
                let encoded = preferences.get(&key).ok_or_else(|| {
                    PreferencesDataStoreError::Message("Node identity is missing".into())
                })?;
                let mut record: Value = serde_json::from_value(encoded.clone())?;
                if record.get("deviceId").and_then(Value::as_str) != Some(identity.nodeId.as_str())
                {
                    return Err(PreferencesDataStoreError::Message(
                        "Node identity changed".into(),
                    ));
                }
                let persisted = record
                    .get("deviceInfo")
                    .cloned()
                    .map(serde_json::from_value::<LinkDeviceInfo>)
                    .transpose()?;
                let mut current = supplied.clone();
                if !replace {
                    if let Some(info) = persisted {
                        // A user rename survives restarts, while the legacy
                        // architecture-derived CLI label is replaced by the
                        // supplied host-name label.
                        if !LinkDeviceInfo::isLegacyCliAutoModel(&info.model) {
                            current.model = info.model;
                        }
                    }
                }
                let info = record
                    .as_object_mut()
                    .ok_or_else(|| {
                        PreferencesDataStoreError::Message("Node identity is not an object".into())
                    })?
                    .entry("deviceInfo")
                    .or_insert_with(|| serde_json::json!({}));
                let object = info.as_object_mut().ok_or_else(|| {
                    PreferencesDataStoreError::Message("Device info is not an object".into())
                })?;
                object.insert("platform".into(), current.platform.clone().into());
                object.insert("model".into(), current.model.clone().into());
                // The pairing generation is minted once and changes only when the
                // device resets its pairing world (leaving a Space).
                let generation = object
                    .get("pairingIncarnation")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(newPairingIncarnation);
                object.insert("pairingIncarnation".into(), generation.clone().into());
                current.pairingIncarnation = Some(generation);
                preferences.set(&key, record);
                Ok::<_, PreferencesDataStoreError>(current)
            })
            .map_err(|error| error.to_string())
    }

    /// The device info as announced to peers: platform and model come from the
    /// host, while the pairing generation is always read from the identity
    /// record. A device that left a Space renewed its generation mid-run, and a
    /// stale in-memory copy would stamp every partner with the revoked
    /// generation — a removal of that device would then never match the pairing
    /// it made afterwards, leaving the credential behind.
    #[allow(non_snake_case)]
    pub fn announcedDeviceInfo(&self, supplied: &LinkDeviceInfo) -> LinkDeviceInfo {
        let mut info = supplied.clone();
        match self.pairingIncarnation() {
            Ok(Some(generation)) => info.pairingIncarnation = Some(generation),
            Ok(None) => {}
            Err(error) => {
                operit_util::AppLogger::AppLogger::w(
                    "PeerStateStore",
                    &format!("Pairing generation read failed error={error}"),
                );
            }
        }
        info
    }

    /// Returns the persisted local pairing generation, if one was minted.
    pub fn pairingIncarnation(&self) -> Result<Option<String>, String> {
        #[derive(Deserialize)]
        struct DeviceInfo {
            #[serde(default)]
            pairingIncarnation: Option<String>,
        }
        #[derive(Deserialize)]
        struct Identity {
            deviceInfo: Option<DeviceInfo>,
        }
        let record: Identity = self.store(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
            .typedEntry("record")
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Node identity is missing".to_string())?;
        Ok(record.deviceInfo.and_then(|info| info.pairingIncarnation))
    }

    /// Returns the local pairing generation, minting and persisting one on
    /// first use.
    #[allow(non_snake_case)]
    pub fn ensurePairingIncarnation(&self) -> Result<String, String> {
        self.editPairingIncarnation(None)
    }

    /// Replaces the local pairing generation with a fresh value. Called in the
    /// step that resets the pairing world (leaving a Space), so a removal that
    /// revoked the previous generation can never match credentials made after
    /// the reset.
    #[allow(non_snake_case)]
    pub fn renewPairingIncarnation(&self) -> Result<String, String> {
        self.editPairingIncarnation(Some(newPairingIncarnation()))
    }

    /// Applies one replacement to the persisted pairing generation: an explicit
    /// value replaces it, `None` keeps an existing one or mints.
    fn editPairingIncarnation(&self, replacement: Option<String>) -> Result<String, String> {
        use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
        use operit_store::PreferencesDataStore::PreferencesDataStoreError;
        let identity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        self.store(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
            .try_edit_result(|preferences| {
                let key = stringPreferencesKey("record");
                let encoded = preferences.get(&key).ok_or_else(|| {
                    PreferencesDataStoreError::Message("Node identity is missing".into())
                })?;
                let mut record: Value = serde_json::from_value(encoded.clone())?;
                if record.get("deviceId").and_then(Value::as_str) != Some(identity.nodeId.as_str())
                {
                    return Err(PreferencesDataStoreError::Message(
                        "Node identity changed".into(),
                    ));
                }
                let info = record
                    .as_object_mut()
                    .ok_or_else(|| {
                        PreferencesDataStoreError::Message("Node identity is not an object".into())
                    })?
                    .entry("deviceInfo")
                    .or_insert_with(|| serde_json::json!({}));
                let object = info.as_object_mut().ok_or_else(|| {
                    PreferencesDataStoreError::Message("Device info is not an object".into())
                })?;
                let generation = replacement
                    .clone()
                    .or_else(|| {
                        object
                            .get("pairingIncarnation")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(newPairingIncarnation);
                object.insert("pairingIncarnation".into(), generation.clone().into());
                preferences.set(&key, record);
                Ok::<_, PreferencesDataStoreError>(generation)
            })
            .map_err(|error| error.to_string())
    }

    /// Reads listener preferences after their versioned initialization.
    pub fn hostConfig(&self) -> Result<Option<PeerHostConfig>, String> {
        let preferences = self.preferences(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)?;
        decodeHostConfig(&preferences).map(Some)
    }
    /// Saves explicit listener settings without removing unrelated preference keys.
    pub fn saveHostConfig(&self, config: &PeerHostConfig) -> Result<(), String> {
        self.store(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
            .try_edit_result(|preferences| writeHostConfigPreferences(preferences, config))
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
                                && record.get("peerNodeId").and_then(Value::as_str)
                                    == Some(nodeId)))
                        .then_some(key)
                    })
                    .collect::<Vec<_>>();
                Ok((*path, keys))
            })
            .collect::<Result<Vec<_>, String>>()?;
        // Pairing revocation ends the consent lifecycle authorized by these
        // credentials, so a new pairing cannot replay a revoked review.
        crate::NodeSpaceService::space_join::purgePeerRecords(self, nodeId)?;
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

    /// Reads pending records without bypassing pairing-code confirmation.
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
/// Writes listener fields into the existing node-local preference snapshot.
fn writeHostConfigPreferences(
    preferences: &mut Preferences,
    config: &PeerHostConfig,
) -> Result<(), PreferencesDataStoreError> {
    for (key, value) in [
        ("bindAddress", Value::String(config.bindAddress.clone())),
        ("discoveryEnabled", Value::Bool(config.discoveryEnabled)),
        ("transports", serde_json::to_value(&config.transports)?),
        ("portMode", serde_json::to_value(config.portMode)?),
        ("updatedAt", Value::from(config.updatedAt)),
    ] {
        preferences.set(&stringPreferencesKey(key), value);
    }
    Ok(())
}

/// Decodes all listener fields strictly after preference schema migration.
fn decodeHostConfig(preferences: &Preferences) -> Result<PeerHostConfig, String> {
    let mut fields = serde_json::Map::new();
    for key in [
        "bindAddress",
        "discoveryEnabled",
        "transports",
        "portMode",
        "updatedAt",
    ] {
        let value = preferences
            .get(&stringPreferencesKey(key))
            .ok_or_else(|| format!("Peer host config is missing {key}"))?;
        fields.insert(key.to_owned(), value.clone());
    }
    serde_json::from_value(Value::Object(fields)).map_err(|error| error.to_string())
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
/// Mints one pairing generation. It only has to be fresh, not ordered: the
/// control log orders authority, and this value only separates the credentials
/// a device held while admitted from the ones it made after resetting.
fn newPairingIncarnation() -> String {
    format!("inc-{}", uuid::Uuid::new_v4().simple())
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
        pairingIncarnation: info.pairingIncarnation.clone(),
    });
    if peer.displayName != info.displayName() {
        return Err("Conflicting paired device metadata".into());
    }
    // Re-pairing requires an unpaired device on both ends, so the newest
    // handshake is the generation a later removal must match.
    if info.pairingIncarnation.is_some() {
        peer.pairingIncarnation = info.pairingIncarnation.clone();
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
    struct Storage(
        Mutex<BTreeMap<String, Vec<u8>>>,
        std::sync::atomic::AtomicBool,
    );
    impl RuntimeStorageHost for Storage {
        /// Confirms synchronous in-memory publication without copying or rewriting bytes.
        fn flushFile(&self, path: &str) -> operit_host_api::HostResult<()> {
            if !self.exists(path)? {
                return Err(operit_host_api::HostError::new("runtime storage entry not found"));
            }
            Ok(())
        }

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
        /// Atomically publishes one complete in-memory test entry.
        fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
            self.writeBytes(path, content)
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
        CoreNodeStateStore::newWithStorage(store.storage.clone(), path)
            .replace(preferences)
            .unwrap();
    }
    /// Projection reads validate their selected facts without decoding unrelated evidence.
    #[test]
    fn projection_scan_skips_evidence_and_rejects_malformed_selected_fields() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Projection { id: u32 }
        #[derive(Deserialize)]
        struct FullRecord { id: u32, payload: String }
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let path = RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH;
        for id in 0..8 {
            store.putRecord(path, &id.to_string(),
                &serde_json::json!({"id": id, "payload": {"evidence": ["x".repeat(2048)]}})).unwrap();
        }
        let mut records = store.recordValues::<Projection>(path).unwrap();
        records.sort_by_key(|record| record.id);
        assert_eq!(records, (0..8).map(|id| Projection { id }).collect::<Vec<_>>());
        assert!(store.recordValues::<FullRecord>(path).is_err());
        store.putRecord(path, "z-broken", &serde_json::json!({"payload": "evidence"})).unwrap();
        let before = storage.0.lock().unwrap().clone();
        assert!(store.recordValues::<Projection>(path).is_err());
        assert_eq!(*storage.0.lock().unwrap(), before);
    }

    #[test]
    fn single_record_reads_current_snapshot_and_rejects_malformed_credentials() {
        #[derive(Debug, Deserialize, Serialize, PartialEq)]
        struct Credential { deviceId: String, sessionSecret: String }
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        let path = RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH;
        let credential = Credential { deviceId: "peer".into(), sessionSecret: "first".into() };
        store.putRecord(path, "current", &credential).unwrap();
        assert_eq!(store.record::<Credential>(path, "current").unwrap(), Some(credential));
        assert!(store.record::<Credential>(path, "missing").unwrap().is_none());
        assert!(store.record::<Credential>(path, PREFERENCES_SCHEMA_VERSION_KEY_NAME).unwrap().is_none());
        store.putRecord(path, "current", &serde_json::json!({"invalid": true})).unwrap();
        assert!(store.record::<Credential>(path, "current").is_err());
        let replacement = Credential { deviceId: "peer".into(), sessionSecret: "renewed".into() };
        store.putRecord(path, "current", &replacement).unwrap();
        assert_eq!(store.record::<Credential>(path, "current").unwrap(), Some(replacement));
        store.deleteRecord(path, "current").unwrap();
        assert!(store.record::<Credential>(path, "current").unwrap().is_none());
    }

    #[test]
    fn unsynchronized_boot_discards_only_inflight_pairings_and_keeps_credentials() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        store
            .putRecord(
                RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,
                "keep",
                &serde_json::json!({"credential":true}),
            )
            .unwrap();
        store
            .putRecord(
                RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
                "keep",
                &serde_json::json!({"credential":true}),
            )
            .unwrap();
        let inbound = storage
            .readBytes(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH)
            .unwrap();
        let outbound = storage
            .readBytes(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
            .unwrap();
        for path in [
            RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
            RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
        ] {
            store
                .putRecord(
                    path,
                    "stale",
                    &serde_json::json!({"version":1,"expires":999999}),
                )
                .unwrap();
            store
                .putRecord(
                    path,
                    "future",
                    &serde_json::json!({"version":2,"expires":999999}),
                )
                .unwrap();
        }
        assert_eq!(
            store
                .discardPendingPairingsForUnsynchronizedClock(1_800_000_000_000)
                .unwrap(),
            0
        );
        assert_eq!(
            store
                .discardPendingPairingsForUnsynchronizedClock(10)
                .unwrap(),
            2
        );
        for path in [
            RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
            RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
        ] {
            assert_eq!(
                store
                    .records::<Value>(path)
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                ["future"]
            );
        }
        assert_eq!(
            storage
                .readBytes(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH)
                .unwrap(),
            inbound
        );
        assert_eq!(
            storage
                .readBytes(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
                .unwrap(),
            outbound
        );
    }

    #[test]
    fn repeated_expired_pairings_remain_bounded_and_keep_credentials() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let path = RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH;
        store
            .putRecord(
                RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,
                "credential",
                &serde_json::json!({"keep":true}),
            )
            .unwrap();
        let credential = storage
            .readBytes(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH)
            .unwrap();
        for now in 0..100 {
            store
                .putPending(
                    path,
                    &format!("pending-{now}"),
                    &serde_json::json!({"version":1,"expires":now+1}),
                    now,
                    2,
                )
                .unwrap();
            assert_eq!(store.records::<Value>(path).unwrap().len(), 1);
        }
        assert_eq!(
            storage
                .readBytes(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH)
                .unwrap(),
            credential
        );
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
        store
            .putRecord(
                path,
                "future",
                &serde_json::json!({"version":2,"expires":0}),
            )
            .unwrap();
        store
            .putRecord(path, "old", &serde_json::json!({"version":1,"expires":0}))
            .unwrap();
        store
            .putRecord(path, "legacy", &serde_json::json!({"old":"unchanged"}))
            .unwrap();
        store
            .putPending(
                path,
                "new",
                &serde_json::json!({"version":1,"expires":100}),
                1,
                4,
            )
            .unwrap();
        assert_eq!(
            store
                .records::<Value>(path)
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["future", "legacy", "new"]
        );
        assert_eq!(
            store
                .versionedRecords::<Value>(path, 1)
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["new"]
        );
    }

    #[test]
    fn original_pairing_credentials_are_used_without_repairing() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        let secret = BASE64.encode([9; 32]);
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,
            "in-session",
            serde_json::json!({
                "deviceId": "peer", "deviceInfo": {"platform":"test", "model":"peer"},
                "pairingServiceVersion": 1, "sessionSecret": secret
            }),
        );
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
            "saved-name",
            serde_json::json!({
                "baseUrl":"http://peer:37194", "sessionId":"out-session", "deviceId":"local",
                "coreDeviceId":"peer", "remoteDeviceInfo":{"platform":"test", "model":"peer"},
                "pairingServiceVersion":1, "sessionSecret":secret
            }),
        );
        let inbound = store
            .records::<StoredInbound>(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH)
            .unwrap();
        assert_eq!(inbound["in-session"].sessionSecret, secret);
        let outbound = store
            .records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
            .unwrap();
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
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
            "named",
            serde_json::json!({
                "baseUrl":"http://peer:37194", "sessionId":"session", "deviceId":"local",
                "coreDeviceId":"peer", "remoteDeviceInfo":{"platform":"test", "model":"peer"},
                "pairingServiceVersion":1, "sessionSecret":BASE64.encode([8;32]), "future":42
            }),
        );
        assert_eq!(
            store
                .store(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
                .schemaVersion()
                .unwrap(),
            2
        );
        let record = store
            .records::<Value>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
            .unwrap()
            .remove("named")
            .unwrap();
        assert_eq!(record["endpoint"], "http://peer:37194");
        assert_eq!(record["peerNodeId"], "peer");
        assert_eq!(record["future"], 42);
        assert!(record.get("baseUrl").is_none());
        assert!(record.get("coreDeviceId").is_none());
        assert!(record.get("remoteDeviceInfo").is_none());
        let before = storage.0.lock().unwrap().clone();
        let reopened = PeerStateStore::new(storage.clone());
        reopened
            .records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
            .unwrap();
        assert_eq!(before, *storage.0.lock().unwrap());
        // Current-schema malformed data must fail, not receive a permanent serde fallback.
        let mut preferences = store
            .preferences(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
            .unwrap();
        let mut invalid = record;
        invalid.as_object_mut().unwrap().remove("transport");
        preferences.set(&stringPreferencesKey("named"), invalid);
        store
            .store(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
            .replace(preferences)
            .unwrap();
        assert!(store
            .records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
            .is_err());
    }

    #[test]
    fn failed_outbound_migration_does_not_rewrite_or_advance_schema() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
            "conflict",
            serde_json::json!({
                "baseUrl":"old", "endpoint":"different"
            }),
        );
        let before = storage.0.lock().unwrap().clone();
        assert!(store
            .records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
            .is_err());
        assert_eq!(before, *storage.0.lock().unwrap());
    }

    #[test]
    fn outbound_migration_reports_missing_steps_and_rejects_newer_schemas() {
        let mut preferences = emptyPreferences();
        assert!(matches!(
            PeerStateStore::migrateOutboundPreferences(2, &mut preferences),
            Err(PreferencesDataStoreError::MissingMigration { from: 2, to: 3 })
        ));
        let storage = Arc::new(Storage::default());
        preferences.set(
            &stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME),
            3_u32,
        );
        CoreNodeStateStore::newWithStorage(
            storage.clone(),
            RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
        )
        .replace(preferences)
        .unwrap();
        let before = storage.0.lock().unwrap().clone();
        let store = PeerStateStore::new(storage.clone());
        assert!(matches!(
            store
                .store(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
                .data(),
            Err(PreferencesDataStoreError::SchemaVersionTooNew {
                actual: 3,
                expected: 2
            })
        ));
        assert_eq!(before, *storage.0.lock().unwrap());
    }

    /// Updates host-owned platform metadata without replacing the configured name or unknown fields.
    #[test]
    fn original_identity_and_unknown_fields_survive_device_info_updates() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_IDENTITY_PATH,
            "record",
            serde_json::json!({
                "deviceId": "stable-node", "future": "preserved",
                "deviceInfo": { "platform": "old", "model": "old-name", "extra": 7 }
            }),
        );
        let supplied = LinkDeviceInfo {
            platform: "new".into(),
            model: "new-name".into(),
            pairingIncarnation: None,
        };
        let initialized = store.deviceInfo(supplied.clone(), false).unwrap();
        assert_eq!(initialized.model, "old-name");
        assert_eq!(initialized.platform, "new");
        assert_eq!(store.deviceInfo(supplied, true).unwrap().model, "new-name");
        let records = store
            .records::<Value>(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
            .unwrap();
        assert_eq!(records["record"]["deviceId"], "stable-node");
        assert_eq!(records["record"]["future"], "preserved");
        assert_eq!(records["record"]["deviceInfo"]["extra"], 7);
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_IDENTITY_PATH,
            "record",
            serde_json::json!({
                "deviceId": "stable-node", "deviceInfo": "broken"
            }),
        );
        let before = store
            .preferences(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
            .unwrap()
            .entries();
        assert!(store
            .deviceInfo(
                LinkDeviceInfo {
                    platform: "new".into(),
                    model: "name".into(),
                    pairingIncarnation: None
                },
                true
            )
            .is_err());
        assert_eq!(
            before,
            store
                .preferences(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
                .unwrap()
                .entries()
        );
    }

    /// Refreshes a previously empty runtime platform without changing identity or the device name.
    #[test]
    fn startup_refreshes_platform_without_replacing_persisted_device_name() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_IDENTITY_PATH,
            "record",
            serde_json::json!({
                "deviceId": "stable-node", "future": "preserved",
                "deviceInfo": { "platform": "", "model": "wasm32", "extra": 7 }
            }),
        );
        let supplied = LinkDeviceInfo {
            platform: "web".into(),
            model: "Chrome 154".into(),
            pairingIncarnation: None,
        };
        let current = store.deviceInfo(supplied.clone(), false).unwrap();
        assert_eq!(current.platform, "web");
        assert_eq!(current.model, "wasm32");
        let records = store
            .records::<Value>(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
            .unwrap();
        assert_eq!(records["record"]["deviceId"], "stable-node");
        assert_eq!(records["record"]["future"], "preserved");
        assert_eq!(records["record"]["deviceInfo"]["platform"], "web");
        assert_eq!(records["record"]["deviceInfo"]["extra"], 7);
        let before = store
            .preferences(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
            .unwrap()
            .entries();
        assert_eq!(store.deviceInfo(supplied, false).unwrap(), current);
        assert_eq!(
            before,
            store
                .preferences(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
                .unwrap()
                .entries()
        );
    }

    /// Replaces the legacy CLI auto label with the host-name label while keeping identity.
    #[test]
    fn startup_replaces_legacy_cli_auto_model_with_the_supplied_host_name() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        let legacyModel = format!("server(cli)-{}", std::env::consts::ARCH);
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_IDENTITY_PATH,
            "record",
            serde_json::json!({
                "deviceId": "stable-node",
                "deviceInfo": { "platform": "linux", "model": legacyModel }
            }),
        );
        let supplied = LinkDeviceInfo {
            platform: "linux".into(),
            model: "server(cli)-build-01".into(),
            pairingIncarnation: None,
        };
        let current = store.deviceInfo(supplied, false).unwrap();
        assert_eq!(current.model, "server(cli)-build-01");
        let records = store
            .records::<Value>(RUNTIME_LINK_ACCESS_IDENTITY_PATH)
            .unwrap();
        assert_eq!(
            records["record"]["deviceInfo"]["model"],
            "server(cli)-build-01"
        );
    }

    /// Announces the persisted pairing generation even when the in-memory copy
    /// still carries the value minted at startup: a device that left a Space
    /// mid-run renewed its generation, and partners must record the new one.
    #[test]
    fn announced_device_info_reflects_a_renewed_pairing_generation() {
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_IDENTITY_PATH,
            "record",
            serde_json::json!({
                "deviceId": "stable-node", "deviceInfo": { "platform": "test", "model": "test" }
            }),
        );
        let startup = store
            .deviceInfo(
                LinkDeviceInfo {
                    platform: "test".into(),
                    model: "test".into(),
                    pairingIncarnation: None,
                },
                false,
            )
            .unwrap();
        let minted = startup.pairingIncarnation.clone();
        assert!(minted.is_some());
        let renewed = store.renewPairingIncarnation().unwrap();
        assert_ne!(minted, Some(renewed.clone()));
        // The startup-time copy is stale; the announcement must not be.
        assert_eq!(
            store.announcedDeviceInfo(&startup).pairingIncarnation,
            Some(renewed.clone()),
        );
        assert_eq!(store.pairingIncarnation().unwrap(), Some(renewed));
        assert_eq!(store.announcedDeviceInfo(&startup).platform, "test");
    }

    /// Rejects incomplete host metadata before creating or modifying identity preferences.
    #[test]
    fn empty_host_device_metadata_never_changes_identity_storage() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let invalid = [
            LinkDeviceInfo {
                platform: "".into(),
                model: "wasm32".into(),
                pairingIncarnation: None,
            },
            LinkDeviceInfo {
                platform: "web".into(),
                model: " ".into(),
                pairingIncarnation: None,
            },
        ];
        for supplied in &invalid {
            assert!(store.deviceInfo(supplied.clone(), false).is_err());
            assert!(storage.0.lock().unwrap().is_empty());
        }
        writeRecord(
            &store,
            RUNTIME_LINK_ACCESS_IDENTITY_PATH,
            "record",
            serde_json::json!({
                "deviceId": "stable-node", "deviceInfo": { "platform": "web", "model": "My browser" }
            }),
        );
        let before = storage.0.lock().unwrap().clone();
        for supplied in invalid {
            assert!(store.deviceInfo(supplied, true).is_err());
            assert_eq!(before, *storage.0.lock().unwrap());
        }
    }

    /// Verifies first reads initialize listener defaults exactly once through the schema.
    #[test]
    fn listener_schema_initializes_http_websocket_discovery_and_automatic_binding_once() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let first = store.hostConfig().unwrap().unwrap();
        assert_eq!(first.bindAddress, "0.0.0.0:0");
        assert_eq!(
            first.transports,
            vec![PeerTransport::Http, PeerTransport::WebSocket]
        );
        assert!(first.discoveryEnabled);
        assert_eq!(first.portMode, PeerHostPortMode::Automatic);
        assert_eq!(
            store
                .store(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
                .schemaVersion()
                .unwrap(),
            4
        );
        let before = storage.0.lock().unwrap().clone();
        let second = PeerStateStore::new(storage.clone())
            .hostConfig()
            .unwrap()
            .unwrap();
        assert_eq!(first.bindAddress, second.bindAddress);
        assert_eq!(first.updatedAt, second.updatedAt);
        assert_eq!(before, *storage.0.lock().unwrap());
    }

    /// Upgrades known old defaults once while preserving custom fixed listeners.
    #[test]
    fn listener_schema_migrates_legacy_defaults_without_overwriting_custom_bindings() {
        for (address, transports, discovery, mode, migrate) in [
            (
                "0.0.0.0:37195",
                vec![PeerTransport::Http, PeerTransport::WebSocket],
                true,
                PeerHostPortMode::Fixed,
                true,
            ),
            (
                "0.0.0.0:8765",
                vec![PeerTransport::Serial, PeerTransport::Tcp],
                true,
                PeerHostPortMode::Fixed,
                true,
            ),
            (
                "0.0.0.0:8765",
                vec![PeerTransport::Serial],
                true,
                PeerHostPortMode::Fixed,
                true,
            ),
            (
                "127.0.0.1:48123",
                vec![PeerTransport::Http, PeerTransport::WebSocket],
                true,
                PeerHostPortMode::Fixed,
                false,
            ),
            (
                "0.0.0.0:37195",
                vec![PeerTransport::Tcp],
                true,
                PeerHostPortMode::Fixed,
                false,
            ),
            (
                "0.0.0.0:37195",
                vec![PeerTransport::Http, PeerTransport::WebSocket],
                false,
                PeerHostPortMode::Fixed,
                false,
            ),
            (
                "0.0.0.0:37195",
                vec![PeerTransport::Http, PeerTransport::WebSocket],
                true,
                PeerHostPortMode::Automatic,
                false,
            ),
        ] {
            let storage = Arc::new(Storage::default());
            let store = PeerStateStore::new(storage.clone());
            let mut preferences = emptyPreferences();
            writeHostConfigPreferences(
                &mut preferences,
                &PeerHostConfig {
                    bindAddress: address.into(),
                    transports: transports.clone(),
                    discoveryEnabled: discovery,
                    portMode: mode,
                    updatedAt: 123,
                },
            )
            .unwrap();
            preferences.set(
                &stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME),
                3,
            );
            preferences.set(&stringPreferencesKey("extra"), "keep");
            CoreNodeStateStore::newWithStorage(storage.clone(), RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
                .replace(preferences)
                .unwrap();
            let config = store.hostConfig().unwrap().unwrap();
            assert_eq!(
                config.bindAddress,
                if migrate { "0.0.0.0:0" } else { address }
            );
            assert_eq!(
                config.portMode,
                if migrate {
                    PeerHostPortMode::Automatic
                } else {
                    mode
                }
            );
            assert_eq!(config.transports, transports);
            assert_eq!(config.discoveryEnabled, discovery);
            assert_eq!(config.updatedAt, 123);
            assert_eq!(
                store
                    .preferences(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
                    .unwrap()
                    .get(&stringPreferencesKey("extra"))
                    .unwrap(),
                "keep"
            );
            let before = storage.0.lock().unwrap().clone();
            store.hostConfig().unwrap();
            assert_eq!(before, *storage.0.lock().unwrap());
        }
    }

    /// Verifies simultaneous first reads share one persisted listener configuration.
    #[test]
    fn concurrent_listener_reads_share_the_schema_initialization_transaction() {
        let storage = Arc::new(Storage::default());
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let workers = (0..8)
            .map(|_| {
                let storage = storage.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let store = PeerStateStore::new(storage);
                    barrier.wait();
                    store.hostConfig().unwrap().unwrap().updatedAt
                })
            })
            .collect::<Vec<_>>();
        let timestamps = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert!(timestamps
            .iter()
            .all(|timestamp| timestamp == &timestamps[0]));
        assert_eq!(storage.0.lock().unwrap().len(), 1);
    }

    /// Verifies the versioned migration removes only the retired listener token.
    #[test]
    fn listener_schema_removes_token_and_preserves_settings_and_unrelated_keys() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let mut config = store.hostConfig().unwrap().unwrap();
        config.bindAddress = "127.0.0.1:48123".into();
        config.transports = vec![PeerTransport::Tcp];
        config.discoveryEnabled = false;
        store.saveHostConfig(&config).unwrap();
        let mut preferences = store
            .preferences(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
            .unwrap();
        preferences.set(
            &stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME),
            2,
        );
        preferences.set(&stringPreferencesKey("token"), "retired-token");
        preferences.set(&stringPreferencesKey("extra"), "preserved");
        CoreNodeStateStore::newWithStorage(storage.clone(), RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
            .replace(preferences.clone())
            .unwrap();
        let migrated = store.hostConfig().unwrap().unwrap();
        let after = store
            .preferences(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
            .unwrap();
        assert!(after.get(&stringPreferencesKey("token")).is_none());
        for (key, value) in preferences.entries() {
            if key == "token" || key == PREFERENCES_SCHEMA_VERSION_KEY_NAME {
                continue;
            }
            assert_eq!(after.get(&stringPreferencesKey(&key)), Some(&value));
        }
        assert_eq!(migrated.bindAddress, config.bindAddress);
        assert_eq!(migrated.transports, config.transports);
        assert_eq!(migrated.discoveryEnabled, config.discoveryEnabled);
        assert_eq!(migrated.updatedAt, config.updatedAt);
        assert_eq!(
            store
                .store(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
                .schemaVersion()
                .unwrap(),
            4
        );
    }

    /// Verifies missing current-schema fields and unknown future versions fail unchanged.
    #[test]
    fn listener_schema_rejects_incomplete_current_and_newer_preferences() {
        for version in [1, 2, 3, 4, 5] {
            let storage = Arc::new(Storage::default());
            let mut preferences = emptyPreferences();
            preferences.set(
                &stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME),
                version.to_string(),
            );
            preferences.set(&stringPreferencesKey("token"), "original-token");
            CoreNodeStateStore::newWithStorage(
                storage.clone(),
                RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH,
            )
            .replace(preferences)
            .unwrap();
            let before = storage.0.lock().unwrap().clone();
            let store = PeerStateStore::new(storage.clone());
            assert!(store.hostConfig().is_err());
            assert_eq!(before, *storage.0.lock().unwrap());
        }
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
            preferences.set(&stringPreferencesKey(key), value);
        }
        CoreNodeStateStore::newWithStorage(storage.clone(), RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH)
            .replace(preferences)
            .unwrap();
        let mut config = store.hostConfig().unwrap().unwrap();
        let before = storage.0.lock().unwrap().clone();
        store.hostConfig().unwrap().unwrap();
        assert!(
            config.transports.is_empty(),
            "obsolete flags must not enable listeners"
        );
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
    /// Revokes every native credential for one peer while preserving other peers.
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
                    preferences.set(&stringPreferencesKey("channel-a"), record.clone());
                    preferences.set(&stringPreferencesKey("channel-b"), record.clone());
                    preferences.set(&stringPreferencesKey("other"), other.clone());
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

    #[test]
    fn device_revocation_retires_join_requests_and_outcomes_but_preserves_other_peers() {
        use crate::NodeSpaceService::space_join::{INBOUND, INBOX, OUTBOUND, RESULTS};
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        for path in [INBOUND, OUTBOUND, INBOX] {
            for (id, applicant, target) in [
                ("applicant", "board", "local"),
                ("target", "local", "board"),
                ("other", "other", "local"),
            ] {
                store
                    .putRecord(
                        path,
                        id,
                        &serde_json::json!({"request": {
                            "applicantDeviceId": applicant, "targetDeviceId": target
                        }}),
                    )
                    .unwrap();
                store
                    .putRecord(RESULTS, id, &serde_json::json!({"decision": id}))
                    .unwrap();
            }
        }
        store.removePairedPeer("board").unwrap();
        store.removePairedPeer("board").unwrap();
        for path in [INBOUND, OUTBOUND, INBOX, RESULTS] {
            assert_eq!(
                store
                    .records::<Value>(path)
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                ["other"]
            );
        }
    }

    /// One malformed request must not prevent reading an unrelated request or displaying healthy rows.
    #[test]
    fn reading_one_request_does_not_decode_unrelated_corrupt_rows() {
        use crate::NodeSpaceService::space_join::INBOUND;
        let store = PeerStateStore::new(Arc::new(Storage::default()));
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Request { value: u32 }
        store.putRecord(INBOUND, "healthy", &serde_json::json!({"value": 7})).unwrap();
        store.putRecord(INBOUND, "broken", &serde_json::json!({"value": "invalid"})).unwrap();
        assert_eq!(store.record::<Request>(INBOUND, "healthy").unwrap(), Some(Request { value: 7 }));
        assert!(store.record::<Request>(INBOUND, "broken").unwrap_err().contains("broken"));
        assert_eq!(store.record::<Request>(INBOUND, "missing").unwrap(), None);
        let healthy = store.independentRecords::<Request>(INBOUND).unwrap();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy["healthy"], Request { value: 7 });
        assert!(store.records::<Request>(INBOUND).is_err());
        store.putRecord(INBOUND, "broken", &serde_json::json!({"value": 8})).unwrap();
        assert_eq!(store.independentRecords::<Request>(INBOUND).unwrap().len(), 2);
    }

    /// Validates every join record before changing any existing credential.
    #[test]
    fn malformed_join_record_aborts_revocation_before_deleting_any_peer_state() {
        use crate::NodeSpaceService::space_join::{INBOUND, OUTBOUND, RESULTS};
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        store
            .putRecord(
                RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,
                "credential",
                &serde_json::json!({"deviceId": "board"}),
            )
            .unwrap();
        store
            .putRecord(
                INBOUND,
                "request",
                &serde_json::json!({"request": {
                    "applicantDeviceId": "board", "targetDeviceId": "local"
                }}),
            )
            .unwrap();
        store
            .putRecord(RESULTS, "request", &serde_json::json!({"decision": true}))
            .unwrap();
        store
            .putRecord(OUTBOUND, "broken", &serde_json::json!({"request": null}))
            .unwrap();
        // Initialize credential schemas before comparing the revocation transaction's stored bytes.
        for path in [
            RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
            RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
            RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
        ] {
            store.records::<Value>(path).unwrap();
        }
        let before = storage.0.lock().unwrap().clone();
        assert!(store.removePairedPeer("board").is_err());
        assert_eq!(*storage.0.lock().unwrap(), before);
    }

    /// Verifies invalid listener preferences fail without modifying stored records.
    #[test]
    fn malformed_config_is_not_replaced_and_pending_records_are_retained() {
        let storage = Arc::new(Storage::default());
        let store = PeerStateStore::new(storage.clone());
        let mut p = emptyPreferences();
        p.set(&stringPreferencesKey("token"), "secret");
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
