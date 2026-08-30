use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::convert::Infallible;
#[cfg(not(target_arch = "wasm32"))]
use std::net::SocketAddr;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
#[cfg(not(target_arch = "wasm32"))]
use std::task::{Context, Poll};

use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use axum::body::Body;
#[cfg(not(target_arch = "wasm32"))]
use axum::body::Bytes;
#[cfg(not(target_arch = "wasm32"))]
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
#[cfg(not(target_arch = "wasm32"))]
use axum::extract::{Json, Path as AxumPath, State};
#[cfg(not(target_arch = "wasm32"))]
use axum::http::{HeaderMap, StatusCode};
#[cfg(not(target_arch = "wasm32"))]
use axum::response::{IntoResponse, Response};
#[cfg(not(target_arch = "wasm32"))]
use axum::routing::{get, post};
#[cfg(not(target_arch = "wasm32"))]
use axum::Router;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
#[cfg(not(target_arch = "wasm32"))]
use futures_util::Stream as FuturesStream;
use hmac::{Hmac, Mac};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};

use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostManager::{defaultHttpHost, defaultWebSocketHost};
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_host_api::{
    HttpRequestData, RuntimeStorageHost, TimeUtils::currentTimeMillis, WebSocketClosedCallback,
    WebSocketHost, WebSocketMessageCallback, WebSocketOpenedCallback, WebSocketRequestData,
};
use operit_link::CoreLinkClient;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError,
    CoreLinkPushSession, CorePushRequest, CoreWatchRequest,
};
#[cfg(not(target_arch = "wasm32"))]
use operit_runtime::services::RuntimeHostInteractionService::{
    publishOwnerWebAccessPairing, RuntimeHostInteractionWebAccessPairingPayload,
};
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceDeviceProfile, CoreSpaceStore};
use operit_store::PreferencesDataStore::{
    emptyPreferences, stringPreferencesKey, CoreNodeStateStore, Flow, Preferences,
    PreferencesDataStoreError,
};
use operit_util::RuntimeStorageLayout::{
    RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH, RUNTIME_LINK_ACCESS_IDENTITY_PATH,
    RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH,
    RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH, RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
};

pub mod CoreNodePeerLink;

const LINK_HTTP_READ_TIMEOUT_SECONDS: u64 = 120;
const SESSION_INFO_READ_TIMEOUT_SECONDS: u64 = 10;

#[cfg(not(target_arch = "wasm32"))]
use CoreNodePeerLink::{
    encodePeerFrame, receivePeerFrame, registerPeerLink, PeerConnection, PeerFrameBatch,
    PeerFrameSender,
};
use CoreNodePeerLink::{CoreNodeLinkClient, CoreNodeTransportClient};
use CoreNodePeerLink::{PeerChannelOpenEnvelope, PeerFrame};

#[cfg(test)]
mod tests;

type HmacSha256 = Hmac<Sha256>;
pub const REMOTE_PAIRING_SERVICE_VERSION: i32 = 1;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct RemoteLinkServerConfig {
    pub bindAddress: String,
    pub token: String,
    pub deviceId: String,
    pub deviceInfo: RemoteDeviceInfo,
    pub webAccess: Option<RemoteWebAccessConfig>,
    pub printStartupInfo: bool,
    pub accessStore: LinkAccessStore,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct RemoteLinkServer;

/// Describes the standalone static Web Access server.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct StaticWebAccessServerConfig {
    pub bindAddress: String,
    pub shutdownToken: String,
    pub webRoot: PathBuf,
    pub readAsset: Arc<dyn Fn(&Path) -> Result<Vec<u8>, String> + Send + Sync>,
    pub linkControl: Option<StaticWebAccessControlConfig>,
    pub printStartupInfo: bool,
}

/// Configures the pairing-only control endpoints hosted beside static Web Access.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct StaticWebAccessControlConfig {
    pub token: String,
    pub deviceId: String,
    pub deviceInfo: RemoteDeviceInfo,
    pub accessStore: LinkAccessStore,
}

/// Hosts the static Web Access bundle without exposing a Core route.
#[cfg(not(target_arch = "wasm32"))]
pub struct StaticWebAccessServer;

/// Stores the state needed by standalone static Web Access handlers.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct StaticWebAccessState {
    shutdownToken: String,
    shutdownSender: Arc<StdMutex<Option<oneshot::Sender<()>>>>,
    webRoot: PathBuf,
    readAsset: Arc<dyn Fn(&Path) -> Result<Vec<u8>, String> + Send + Sync>,
    control: Option<StaticWebAccessControlState>,
}

/// Holds identity and authenticated session state for pairing-only endpoints.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct StaticWebAccessControlState {
    token: String,
    keySecret: Arc<StaticSecret>,
    keyPublic: String,
    deviceId: String,
    deviceInfo: RemoteDeviceInfo,
    pairings: Arc<Mutex<BTreeMap<String, PendingPairing>>>,
    sessions: Arc<Mutex<BTreeMap<String, RemoteSession>>>,
    accessStore: LinkAccessStore,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemotePairingCodeRecord {
    pub pairingId: String,
    pub pairingServiceVersion: i32,
    pub clientDeviceId: String,
    pub clientDeviceInfo: RemoteDeviceInfo,
    pub pairingCode: String,
    pub createdAt: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedRemoteSessionRecord {
    pub deviceId: String,
    pub deviceInfo: RemoteDeviceInfo,
    pub pairingServiceVersion: i32,
    pub sessionSecret: String,
}

const LINK_ACCESS_RECORD_KEY: &str = "record";
const LINK_ACCESS_BIND_ADDRESS_KEY: &str = "bindAddress";
const LINK_ACCESS_TOKEN_KEY: &str = "token";
const LINK_ACCESS_WEB_ACCESS_ENABLED_KEY: &str = "webAccessEnabled";
const LINK_ACCESS_DISCOVERY_ENABLED_KEY: &str = "discoveryEnabled";
const LINK_ACCESS_PORT_MODE_KEY: &str = "portMode";
const LINK_ACCESS_UPDATED_AT_KEY: &str = "updatedAt";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkAccessIdentity {
    pub deviceId: String,
    pub deviceInfo: RemoteDeviceInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkAccessHostConfig {
    pub bindAddress: String,
    pub token: String,
    pub webAccessEnabled: bool,
    pub discoveryEnabled: bool,
    pub portMode: LinkAccessHostPortMode,
    pub updatedAt: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LinkAccessHostPortMode {
    #[serde(rename = "automatic")]
    Automatic,
    #[serde(rename = "fixed")]
    Fixed,
}

#[derive(Clone)]
pub struct LinkAccessStore {
    storage: Arc<dyn RuntimeStorageHost>,
}

impl LinkAccessStore {
    /// Creates the Link Access datastore for the current runtime host.
    #[allow(non_snake_case)]
    pub fn getInstance(context: &operit_host_api::HostManager::HostManager) -> Self {
        let storage = context
            .runtimeStorageHost
            .clone()
            .expect("LinkAccessStore requires a RuntimeStorageHost");
        Self::new(storage)
    }

    /// Creates the repository that owns Link Access records for one runtime.
    pub fn new(storage: Arc<dyn RuntimeStorageHost>) -> Self {
        Self { storage }
    }

    /// Initializes and returns the runtime's persisted Link device identity.
    pub fn initializeIdentity(
        &self,
        deviceInfo: RemoteDeviceInfo,
    ) -> Result<LinkAccessIdentity, String> {
        let coreNodeIdentity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        let store = self.dataStore(RUNTIME_LINK_ACCESS_IDENTITY_PATH);
        let preferences = self.readPreferences(&store)?;
        let encoded = requiredPreference(
            &preferences,
            LINK_ACCESS_RECORD_KEY,
            RUNTIME_LINK_ACCESS_IDENTITY_PATH,
        )?;
        let record: serde_json::Value =
            serde_json::from_str(&encoded).map_err(|error| error.to_string())?;
        let persistedDeviceId = record
            .get("deviceId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "Link Access identity is missing deviceId".to_string())?;
        if persistedDeviceId != coreNodeIdentity.nodeId {
            return Err(format!(
                "device identity mismatch: current={}, link={persistedDeviceId}",
                coreNodeIdentity.nodeId
            ));
        }
        let identity = if let Some(persistedDeviceInfo) = record.get("deviceInfo") {
            LinkAccessIdentity {
                deviceId: coreNodeIdentity.nodeId,
                deviceInfo: serde_json::from_value(persistedDeviceInfo.clone())
                    .map_err(|error| error.to_string())?,
            }
        } else {
            let identity = LinkAccessIdentity {
                deviceId: coreNodeIdentity.nodeId,
                deviceInfo,
            };
            writeSingleRecord(&store, &identity)?;
            identity
        };
        CoreSpaceStore::new(self.storage.clone()).writeLocalDeviceProfile(
            identity.deviceInfo.displayName(),
            identity.deviceInfo.platform.clone(),
            identity.deviceInfo.model.clone(),
            operit_runtime::CORE_VERSION.to_string(),
        )?;
        self.syncPairedDeviceProfiles()?;
        Ok(identity)
    }

    /// Updates and returns the runtime's Link device information for its stable identity.
    #[allow(non_snake_case)]
    pub fn updateIdentityDeviceInfo(
        &self,
        deviceInfo: RemoteDeviceInfo,
    ) -> Result<LinkAccessIdentity, String> {
        let coreNodeIdentity = CoreNodeIdentityStore::new(self.storage.clone()).initialize()?;
        let identity = LinkAccessIdentity {
            deviceId: coreNodeIdentity.nodeId,
            deviceInfo,
        };
        writeSingleRecord(
            &self.dataStore(RUNTIME_LINK_ACCESS_IDENTITY_PATH),
            &identity,
        )?;
        CoreSpaceStore::new(self.storage.clone()).writeLocalDeviceProfile(
            identity.deviceInfo.displayName(),
            identity.deviceInfo.platform.clone(),
            identity.deviceInfo.model.clone(),
            operit_runtime::CORE_VERSION.to_string(),
        )?;
        self.syncPairedDeviceProfiles()?;
        Ok(identity)
    }

    /// Returns every accepted inbound session owned by this runtime.
    pub fn inboundSessions(&self) -> Result<BTreeMap<String, AcceptedRemoteSessionRecord>, String> {
        self.readRecordMap(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH)
    }

    /// Observes every accepted inbound session owned by this runtime.
    #[allow(non_snake_case)]
    pub fn inboundSessionsFlow(&self) -> Flow<BTreeMap<String, AcceptedRemoteSessionRecord>> {
        self.recordMapFlow(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH)
    }

    /// Persists one accepted inbound session owned by this runtime.
    pub fn saveInboundSession(
        &self,
        sessionId: String,
        record: AcceptedRemoteSessionRecord,
    ) -> Result<(), String> {
        self.validateInboundSessionRecord(&sessionId, &record)?;
        self.writePairedDeviceProfile(&record.deviceId, &record.deviceInfo)?;
        self.writeMapRecord(
            RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,
            &sessionId,
            &record,
        )
    }

    /// Removes one accepted inbound session owned by this runtime.
    pub fn removeInboundSession(&self, sessionId: &str) -> Result<(), String> {
        let sessions = self.inboundSessions()?;
        sessions
            .get(sessionId)
            .ok_or_else(|| format!("accepted remote session does not exist: {sessionId}"))?;
        self.removeMapRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, sessionId)
    }

    /// Returns every named outbound session owned by this runtime.
    pub fn outboundSessions(&self) -> Result<BTreeMap<String, PairedRemoteSessionRecord>, String> {
        self.readRecordMap(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
    }

    /// Observes every named outbound session owned by this runtime.
    #[allow(non_snake_case)]
    pub fn outboundSessionsFlow(&self) -> Flow<BTreeMap<String, PairedRemoteSessionRecord>> {
        self.recordMapFlow(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
    }

    /// Persists one named outbound session owned by this runtime.
    pub fn saveOutboundSession(
        &self,
        name: String,
        record: PairedRemoteSessionRecord,
    ) -> Result<(), String> {
        self.validateOutboundSessionRecord(&name, &record)?;
        self.writePairedDeviceProfile(&record.coreDeviceId, &record.remoteDeviceInfo)?;
        self.writeMapRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, &name, &record)
    }

    /// Writes synchronized device profiles for every stored pairing endpoint.
    #[allow(non_snake_case)]
    pub fn syncPairedDeviceProfiles(&self) -> Result<(), String> {
        for record in self.inboundSessions()?.into_values() {
            self.writePairedDeviceProfile(&record.deviceId, &record.deviceInfo)?;
        }
        for record in self.outboundSessions()?.into_values() {
            self.writePairedDeviceProfile(&record.coreDeviceId, &record.remoteDeviceInfo)?;
        }
        Ok(())
    }

    /// Removes one named outbound session owned by this runtime.
    pub fn removeOutboundSession(&self, name: &str) -> Result<(), String> {
        let sessions = self.outboundSessions()?;
        sessions
            .get(name)
            .ok_or_else(|| format!("paired remote session does not exist: {name}"))?;
        self.removeMapRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, name)
    }

    /// Returns every pending pairing owned by this runtime.
    pub fn pendingPairings(&self) -> Result<BTreeMap<String, RemotePairingCodeRecord>, String> {
        self.readRecordMap(RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH)
    }

    /// Persists one pending pairing owned by this runtime.
    pub fn savePendingPairing(&self, record: RemotePairingCodeRecord) -> Result<(), String> {
        self.writeMapRecord(
            RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH,
            &record.pairingId.clone(),
            &record,
        )
    }

    /// Removes one pending pairing owned by this runtime.
    pub fn removePendingPairing(&self, pairingId: &str) -> Result<(), String> {
        self.removeMapRecord(RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH, pairingId)
    }

    /// Returns every pending outbound pairing initiated by this runtime.
    #[allow(non_snake_case)]
    pub fn pendingOutboundPairings(
        &self,
    ) -> Result<BTreeMap<String, PendingOutboundPairingRecord>, String> {
        self.readRecordMap(RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH)
    }

    /// Persists one pending outbound pairing initiated by this runtime.
    #[allow(non_snake_case)]
    pub fn savePendingOutboundPairing(
        &self,
        pairingId: String,
        record: PendingOutboundPairingRecord,
    ) -> Result<(), String> {
        self.writeMapRecord(
            RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
            &pairingId,
            &record,
        )
    }

    /// Removes one pending outbound pairing after it has completed or been cancelled.
    #[allow(non_snake_case)]
    pub fn removePendingOutboundPairing(&self, pairingId: &str) -> Result<(), String> {
        self.removeMapRecord(
            RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
            pairingId,
        )
    }

    /// Persists the active Link Access host configuration for this runtime.
    pub fn saveHostConfig(&self, config: LinkAccessHostConfig) -> Result<(), String> {
        writeHostConfigPreferences(
            &self.dataStore(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH),
            &config,
        )
    }

    /// Initializes and returns the active Link Access host configuration.
    pub fn initializeHostConfig(&self) -> Result<LinkAccessHostConfig, String> {
        let store = self.dataStore(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH);
        let preferences = self.readPreferences(&store)?;
        if !preferences.entries().is_empty() {
            return hostConfigFromPreferences(&preferences);
        }
        let config = LinkAccessHostConfig {
            bindAddress: "0.0.0.0:37194".to_string(),
            token: link_access_token(),
            webAccessEnabled: false,
            discoveryEnabled: false,
            portMode: LinkAccessHostPortMode::Automatic,
            updatedAt: currentTimeMillis(),
        };
        writeHostConfigPreferences(&store, &config)?;
        Ok(config)
    }

    /// Reads the active Link Access host configuration for this runtime.
    pub fn hostConfig(&self) -> Result<LinkAccessHostConfig, String> {
        hostConfigFromPreferences(
            &self.readPreferences(&self.dataStore(RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH))?,
        )
    }

    /// Creates one local datastore for a Link Access preferences path.
    fn dataStore(&self, path: &str) -> CoreNodeStateStore {
        CoreNodeStateStore::newWithStorage(self.storage.clone(), path)
    }

    /// Writes the Space device profile carried by one paired endpoint record.
    #[allow(non_snake_case)]
    fn writePairedDeviceProfile(
        &self,
        deviceId: &str,
        deviceInfo: &RemoteDeviceInfo,
    ) -> Result<(), String> {
        let spaceStore = CoreSpaceStore::new(self.storage.clone());
        let displayName = deviceInfo.displayName();
        let profiles = spaceStore.deviceProfiles()?;
        let profile = match profiles.get(deviceId) {
            Some(profile)
                if profile.displayName == displayName
                    && profile.platform == deviceInfo.platform
                    && profile.model == deviceInfo.model =>
            {
                return Ok(());
            }
            Some(profile) => CoreSpaceDeviceProfile {
                nodeId: deviceId.to_string(),
                displayName,
                userName: profile.userName.clone(),
                platform: deviceInfo.platform.clone(),
                model: deviceInfo.model.clone(),
                coreVersion: profile.coreVersion.clone(),
                updatedAt: unix_millis(),
            },
            None => CoreSpaceDeviceProfile {
                nodeId: deviceId.to_string(),
                displayName,
                userName: String::new(),
                platform: deviceInfo.platform.clone(),
                model: deviceInfo.model.clone(),
                coreVersion: None,
                updatedAt: unix_millis(),
            },
        };
        spaceStore.importDeviceProfiles(vec![profile])
    }

    /// Reads one Link Access preferences snapshot.
    fn readPreferences(&self, store: &CoreNodeStateStore) -> Result<Preferences, String> {
        store.data().map_err(|error| error.to_string())
    }

    /// Reads every keyed record from a Link Access datastore.
    fn readRecordMap<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<BTreeMap<String, T>, String> {
        let preferences = self.readPreferences(&self.dataStore(path))?;
        recordMapFromPreferences(preferences).map_err(|error| error.to_string())
    }

    /// Observes every keyed record stored at one Link Access preferences path.
    #[allow(non_snake_case)]
    fn recordMapFlow<T>(&self, path: &str) -> Flow<BTreeMap<String, T>>
    where
        T: serde::de::DeserializeOwned + 'static,
    {
        self.dataStore(path)
            .dataFlow()
            .mapResult(recordMapFromPreferences)
    }

    /// Validates one inbound record against every persisted direction for the same device.
    #[allow(non_snake_case)]
    fn validateInboundSessionRecord(
        &self,
        sessionId: &str,
        record: &AcceptedRemoteSessionRecord,
    ) -> Result<(), String> {
        for (existingSessionId, existing) in self.inboundSessions()? {
            if existingSessionId != sessionId
                && existing.deviceId == record.deviceId
                && existing.deviceInfo != record.deviceInfo
            {
                return Err(format!(
                    "paired device {} has conflicting device information",
                    record.deviceId
                ));
            }
        }
        for existing in self.outboundSessions()?.into_values() {
            if existing.coreDeviceId == record.deviceId
                && existing.remoteDeviceInfo != record.deviceInfo
            {
                return Err(format!(
                    "paired device {} has conflicting device information",
                    record.deviceId
                ));
            }
        }
        Ok(())
    }

    /// Validates one outbound record against every persisted direction for the same device.
    #[allow(non_snake_case)]
    fn validateOutboundSessionRecord(
        &self,
        name: &str,
        record: &PairedRemoteSessionRecord,
    ) -> Result<(), String> {
        for (existingName, existing) in self.outboundSessions()? {
            if existingName != name && existing.coreDeviceId == record.coreDeviceId {
                return Err(format!(
                    "multiple outgoing pairings target device {}",
                    record.coreDeviceId
                ));
            }
            if existing.coreDeviceId == record.coreDeviceId
                && existing.remoteDeviceInfo != record.remoteDeviceInfo
            {
                return Err(format!(
                    "paired device {} has conflicting device information",
                    record.coreDeviceId
                ));
            }
        }
        for existing in self.inboundSessions()?.into_values() {
            if existing.deviceId == record.coreDeviceId
                && existing.deviceInfo != record.remoteDeviceInfo
            {
                return Err(format!(
                    "paired device {} has conflicting device information",
                    record.coreDeviceId
                ));
            }
        }
        Ok(())
    }

    /// Writes one keyed record into a Link Access datastore.
    fn writeMapRecord<T: Serialize>(
        &self,
        path: &str,
        name: &str,
        value: &T,
    ) -> Result<(), String> {
        let encoded = serde_json::to_string(value).map_err(|error| error.to_string())?;
        self.dataStore(path)
            .edit(|preferences| {
                preferences.set(&stringPreferencesKey(name), encoded.clone());
            })
            .map_err(|error| error.to_string())
    }

    /// Removes one keyed record from a Link Access datastore.
    fn removeMapRecord(&self, path: &str, name: &str) -> Result<(), String> {
        self.dataStore(path)
            .edit(|preferences| {
                preferences.remove(&stringPreferencesKey(name));
            })
            .map_err(|error| error.to_string())
    }
}

/// Decodes every keyed JSON record from one preferences snapshot.
#[allow(non_snake_case)]
fn recordMapFromPreferences<T: serde::de::DeserializeOwned>(
    preferences: Preferences,
) -> Result<BTreeMap<String, T>, PreferencesDataStoreError> {
    let mut records = BTreeMap::new();
    for (name, encoded) in preferences.entries() {
        records.insert(name, serde_json::from_str(&encoded)?);
    }
    Ok(records)
}

/// Writes one single-record datastore snapshot.
fn writeSingleRecord<T: Serialize>(store: &CoreNodeStateStore, value: &T) -> Result<(), String> {
    let mut preferences = emptyPreferences();
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_RECORD_KEY),
        serde_json::to_string(value).map_err(|error| error.to_string())?,
    );
    store
        .replace(preferences)
        .map_err(|error| error.to_string())
}

/// Reads one required preference string.
fn requiredPreference(preferences: &Preferences, key: &str, path: &str) -> Result<String, String> {
    preferences
        .get(&stringPreferencesKey(key))
        .cloned()
        .ok_or_else(|| format!("Link Access store {path} is missing key {key}"))
}

/// Reads one required boolean preference string.
fn requiredBoolPreference(
    preferences: &Preferences,
    key: &str,
    path: &str,
) -> Result<bool, String> {
    requiredPreference(preferences, key, path)?
        .parse::<bool>()
        .map_err(|error| error.to_string())
}

/// Reads one required integer preference string.
fn requiredI64Preference(preferences: &Preferences, key: &str, path: &str) -> Result<i64, String> {
    requiredPreference(preferences, key, path)?
        .parse::<i64>()
        .map_err(|error| error.to_string())
}

/// Converts persisted host config preferences into the typed model.
fn hostConfigFromPreferences(preferences: &Preferences) -> Result<LinkAccessHostConfig, String> {
    Ok(LinkAccessHostConfig {
        bindAddress: requiredPreference(
            preferences,
            LINK_ACCESS_BIND_ADDRESS_KEY,
            RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH,
        )?,
        token: requiredPreference(
            preferences,
            LINK_ACCESS_TOKEN_KEY,
            RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH,
        )?,
        webAccessEnabled: requiredBoolPreference(
            preferences,
            LINK_ACCESS_WEB_ACCESS_ENABLED_KEY,
            RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH,
        )?,
        discoveryEnabled: requiredBoolPreference(
            preferences,
            LINK_ACCESS_DISCOVERY_ENABLED_KEY,
            RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH,
        )?,
        portMode: hostPortModeFromPreference(&requiredPreference(
            preferences,
            LINK_ACCESS_PORT_MODE_KEY,
            RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH,
        )?)?,
        updatedAt: requiredI64Preference(
            preferences,
            LINK_ACCESS_UPDATED_AT_KEY,
            RUNTIME_LINK_ACCESS_HOST_CONFIG_PATH,
        )?,
    })
}

/// Persists one host config through the local datastore API.
fn writeHostConfigPreferences(
    store: &CoreNodeStateStore,
    config: &LinkAccessHostConfig,
) -> Result<(), String> {
    let mut preferences = emptyPreferences();
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_BIND_ADDRESS_KEY),
        config.bindAddress.clone(),
    );
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_TOKEN_KEY),
        config.token.clone(),
    );
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_WEB_ACCESS_ENABLED_KEY),
        config.webAccessEnabled.to_string(),
    );
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_DISCOVERY_ENABLED_KEY),
        config.discoveryEnabled.to_string(),
    );
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_PORT_MODE_KEY),
        hostPortModePreference(&config.portMode).to_string(),
    );
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_UPDATED_AT_KEY),
        config.updatedAt.to_string(),
    );
    store
        .replace(preferences)
        .map_err(|error| error.to_string())
}

/// Returns the persisted literal for one host port mode.
fn hostPortModePreference(value: &LinkAccessHostPortMode) -> &'static str {
    match value {
        LinkAccessHostPortMode::Automatic => "automatic",
        LinkAccessHostPortMode::Fixed => "fixed",
    }
}

/// Parses one host port mode preference literal.
fn hostPortModeFromPreference(value: &str) -> Result<LinkAccessHostPortMode, String> {
    match value {
        "automatic" => Ok(LinkAccessHostPortMode::Automatic),
        "fixed" => Ok(LinkAccessHostPortMode::Fixed),
        other => Err(format!("invalid Link Access host port mode: {other}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct RemoteWebAccessConfig {
    pub token: String,
    pub shutdownToken: String,
    pub webRoot: PathBuf,
    pub readAsset: Arc<dyn Fn(&Path) -> Result<Vec<u8>, String> + Send + Sync>,
}

#[cfg(not(target_arch = "wasm32"))]
impl StaticWebAccessServer {
    /// Binds and serves the standalone static Web Access bundle.
    pub async fn serve(config: StaticWebAccessServerConfig) -> Result<(), String> {
        let address: SocketAddr = config
            .bindAddress
            .parse()
            .map_err(|error| format!("invalid bind address: {error}"))?;
        let listener = TcpListener::bind(address)
            .await
            .map_err(|error| error.to_string())?;
        Self::serveWithListener(config, listener, address).await
    }

    /// Serves the static bundle from an already bound listener.
    #[allow(non_snake_case)]
    pub async fn serveWithListener(
        config: StaticWebAccessServerConfig,
        listener: TcpListener,
        address: SocketAddr,
    ) -> Result<(), String> {
        let (shutdownSender, shutdownReceiver) = oneshot::channel::<()>();
        let control = if let Some(config) = config.linkControl.clone() {
            CoreSpaceStore::new(config.accessStore.storage.clone()).initialize()?;
            let keySecret = Arc::new(StaticSecret::random_from_rng(OsRng));
            let keyPublic = public_key_to_string(&PublicKey::from(keySecret.as_ref()));
            let sessions = Arc::new(Mutex::new(BTreeMap::new()));
            for (sessionId, session) in config.accessStore.inboundSessions()? {
                sessions.lock().await.insert(
                    sessionId,
                    RemoteSession {
                        deviceId: session.deviceId,
                        deviceInfo: session.deviceInfo,
                        pairingServiceVersion: session.pairingServiceVersion,
                        sessionSecret: BASE64
                            .decode(session.sessionSecret.as_bytes())
                            .map_err(|error| error.to_string())?,
                    },
                );
            }
            Some(StaticWebAccessControlState {
                token: config.token,
                keySecret,
                keyPublic,
                deviceId: config.deviceId,
                deviceInfo: config.deviceInfo,
                pairings: Arc::new(Mutex::new(BTreeMap::new())),
                sessions,
                accessStore: config.accessStore,
            })
        } else {
            None
        };
        let state = StaticWebAccessState {
            shutdownToken: config.shutdownToken,
            shutdownSender: Arc::new(StdMutex::new(Some(shutdownSender))),
            webRoot: config.webRoot,
            readAsset: config.readAsset,
            control,
        };
        let mut app = Router::new();
        if state.control.is_some() {
            app = app
                .route("/link/hello", get(static_web_access_hello))
                .route("/link/pair/start", post(static_web_access_pair_start))
                .route("/link/pair/finish", post(static_web_access_pair_finish))
                .route("/link/session", post(static_web_access_session_info))
                .route("/link/space/adopt", post(static_web_access_space_adopt));
        }
        app = app
            .route("/", get(static_web_access_index))
            .route("/client/web-access/close", post(static_web_access_close))
            .route("/*path", get(static_web_access_asset));
        let app = app.with_state(state);
        if config.printStartupInfo {
            println!("static web access server listening on http://{address}");
        }
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdownReceiver.await;
            })
            .await
            .map_err(|error| error.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct RemoteLinkState {
    coreNodeTransport: Arc<dyn CoreNodeTransportClient>,
    token: String,
    keySecret: Arc<StaticSecret>,
    keyPublic: String,
    deviceId: String,
    deviceInfo: RemoteDeviceInfo,
    pairings: Arc<Mutex<BTreeMap<String, PendingPairing>>>,
    sessions: Arc<Mutex<BTreeMap<String, RemoteSession>>>,
    accessStore: LinkAccessStore,
    webAccess: Option<RemoteWebAccessState>,
}

#[derive(Clone)]
struct SharedAccessCoreClient {
    core: Arc<StdMutex<Box<dyn CoreNodeLinkClient + Send>>>,
}

impl SharedAccessCoreClient {
    /// Executes one local call on the runtime scheduler.
    async fn callOnRuntime(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        if let Err(error) = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "link-access-call",
            Box::new(move || {
                Box::pin(async move {
                    let mut client = core
                        .lock()
                        .expect("Link Access core mutex poisoned")
                        .cloneCoreNodeLinkClient();
                    let response = client.call(request).await;
                    let _ = sender.send(response);
                })
            }),
        ) {
            return CoreCallResponse::err(requestId, CoreLinkError::internal(error.to_string()));
        }
        receiver.await.unwrap_or_else(|error| {
            CoreCallResponse::err(requestId, CoreLinkError::internal(error.to_string()))
        })
    }

    /// Reads one local watch snapshot on the runtime scheduler.
    #[allow(non_snake_case)]
    async fn watchSnapshotOnRuntime(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "link-access-watch-snapshot",
                Box::new(move || {
                    Box::pin(async move {
                        let mut client = core
                            .lock()
                            .expect("Link Access core mutex poisoned")
                            .cloneCoreNodeLinkClient();
                        let response = client.watchSnapshot(request).await;
                        let _ = sender.send(response);
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }

    /// Opens one local watch on the runtime scheduler.
    async fn watchOnRuntime(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "link-access-watch",
                Box::new(move || {
                    Box::pin(async move {
                        let mut client = core
                            .lock()
                            .expect("Link Access core mutex poisoned")
                            .cloneCoreNodeLinkClient();
                        let response = client.watch(request).await;
                        let _ = sender.send(response);
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }

    /// Opens one local reverse stream on the runtime scheduler.
    #[allow(non_snake_case)]
    async fn openPushOnRuntime(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "link-access-push-open",
                Box::new(move || {
                    Box::pin(async move {
                        let mut client = core
                            .lock()
                            .expect("Link Access core mutex poisoned")
                            .cloneCoreNodeLinkClient();
                        let response = client.openPush(request).await;
                        let _ = sender.send(response);
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }
}

#[async_trait]
impl CoreNodeTransportClient for SharedAccessCoreClient {
    /// Executes one local call through the runtime scheduler.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.callOnRuntime(request).await
    }

    /// Reads one local watch snapshot through the runtime scheduler.
    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.watchSnapshotOnRuntime(request).await
    }

    /// Opens one local watch through the runtime scheduler.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.watchOnRuntime(request).await
    }

    /// Opens one local push through the runtime scheduler.
    #[allow(non_snake_case)]
    async fn openPush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.openPushOnRuntime(request).await
    }

    /// Executes one routed call through the runtime scheduler.
    #[allow(non_snake_case)]
    async fn routedCall(
        &self,
        previousNodeId: String,
        request: CoreNodePeerLink::RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        let requestId = request.payload.requestId.clone();
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        if let Err(error) = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "link-access-routed-call",
            Box::new(move || {
                Box::pin(async move {
                    let mut client = core
                        .lock()
                        .expect("Link Access core mutex poisoned")
                        .cloneCoreNodeLinkClient();
                    let response = client.routedCall(previousNodeId, request).await;
                    let _ = sender.send(response);
                })
            }),
        ) {
            return CoreCallResponse::err(requestId, CoreLinkError::internal(error.to_string()));
        }
        receiver.await.unwrap_or_else(|error| {
            CoreCallResponse::err(requestId, CoreLinkError::internal(error.to_string()))
        })
    }

    /// Reads one routed watch snapshot through the runtime scheduler.
    #[allow(non_snake_case)]
    async fn routedWatchSnapshot(
        &self,
        previousNodeId: String,
        request: CoreNodePeerLink::RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "link-access-routed-watch-snapshot",
                Box::new(move || {
                    Box::pin(async move {
                        let mut client = core
                            .lock()
                            .expect("Link Access core mutex poisoned")
                            .cloneCoreNodeLinkClient();
                        let response = client.routedWatchSnapshot(previousNodeId, request).await;
                        let _ = sender.send(response);
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }

    /// Opens one routed watch through the runtime scheduler.
    #[allow(non_snake_case)]
    async fn routedWatch(
        &self,
        previousNodeId: String,
        request: CoreNodePeerLink::RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "link-access-routed-watch",
                Box::new(move || {
                    Box::pin(async move {
                        let mut client = core
                            .lock()
                            .expect("Link Access core mutex poisoned")
                            .cloneCoreNodeLinkClient();
                        let response = client.routedWatch(previousNodeId, request).await;
                        let _ = sender.send(response);
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }

    /// Opens one routed push through the runtime scheduler.
    #[allow(non_snake_case)]
    async fn routedOpenPush(
        &self,
        previousNodeId: String,
        request: CoreNodePeerLink::RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "link-access-routed-push-open",
                Box::new(move || {
                    Box::pin(async move {
                        let mut client = core
                            .lock()
                            .expect("Link Access core mutex poisoned")
                            .cloneCoreNodeLinkClient();
                        let response = client.routedOpenPush(previousNodeId, request).await;
                        let _ = sender.send(response);
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }
}

/// Wraps a cloneable CoreNode router for use by Send-safe Peer Link callbacks.
#[allow(non_snake_case)]
pub fn coreNodeTransportClient(
    core: impl CoreNodeLinkClient + Send + 'static,
) -> Arc<dyn CoreNodeTransportClient> {
    Arc::new(SharedAccessCoreClient {
        core: Arc::new(StdMutex::new(Box::new(core))),
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct RemoteWebAccessState {
    shutdownToken: String,
    shutdownSender: Arc<StdMutex<Option<oneshot::Sender<()>>>>,
    webRoot: PathBuf,
    readAsset: Arc<dyn Fn(&Path) -> Result<Vec<u8>, String> + Send + Sync>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
struct PendingPairing {
    pairingServiceVersion: i32,
    clientDeviceId: String,
    clientDeviceInfo: RemoteDeviceInfo,
    clientPublicKey: String,
    pairingCode: String,
    serverNonce: String,
    clientNonce: String,
    sharedSecret: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
struct RemoteSession {
    deviceId: String,
    deviceInfo: RemoteDeviceInfo,
    pairingServiceVersion: i32,
    sessionSecret: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
struct VerifiedRemoteSession {
    sessionId: String,
    deviceId: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDeviceInfo {
    pub platform: String,
    pub model: String,
}

impl RemoteDeviceInfo {
    /// Describes the local CLI device with its runtime role.
    #[allow(non_snake_case)]
    pub fn nativeCli(role: &str) -> Result<Self, String> {
        let hostname =
            std::env::var("HOSTNAME").map_err(|error| format!("HOSTNAME unavailable: {error}"))?;
        let hostname = hostname.trim();
        if hostname.is_empty() {
            return Err("HOSTNAME is empty".to_string());
        }
        Ok(Self {
            platform: std::env::consts::OS.to_string(),
            model: format!("{}-{}(cli)-{}", hostname, role, std::env::consts::ARCH),
        })
    }

    pub fn native() -> Self {
        Self {
            platform: std::env::consts::OS.to_string(),
            model: std::env::consts::ARCH.to_string(),
        }
    }

    pub fn displayName(&self) -> String {
        format!("{}-{}", self.platform, self.model)
    }
}

/// Describes the live device space summary exposed during nearby discovery.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteDeviceSpaceInfo {
    pub spaceId: String,
    pub spaceName: String,
    pub spaceRevision: i64,
    pub deviceCount: usize,
    pub userName: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HelloResponse {
    pub protocolVersion: i32,
    pub pairingServiceVersion: i32,
    pub coreDeviceId: String,
    pub coreDeviceInfo: RemoteDeviceInfo,
    pub deviceSpace: RemoteDeviceSpaceInfo,
    pub corePublicKey: String,
    pub transports: Vec<String>,
    pub pairingRequired: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairStartRequest {
    pub pairingServiceVersion: i32,
    pub tokenHash: String,
    pub clientDeviceId: String,
    pub clientDeviceInfo: RemoteDeviceInfo,
    pub clientPublicKey: String,
    pub clientNonce: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairStartResponse {
    pub pairingId: String,
    pub pairingServiceVersion: i32,
    pub coreDeviceId: String,
    pub coreDeviceInfo: RemoteDeviceInfo,
    pub corePublicKey: String,
    pub serverNonce: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairFinishRequest {
    pub pairingId: String,
    pub pairingCode: String,
    pub clientProof: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairFinishResponse {
    pub sessionId: String,
    pub pairingServiceVersion: i32,
    pub coreProof: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteSessionInfoEnvelope {
    pub nonce: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteSpaceAdoptEnvelope {
    pub space: CoreSpace,
    pub deviceProfiles: Vec<CoreSpaceDeviceProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteSessionInfoResponse {
    pub protocolVersion: i32,
    pub pairingServiceVersion: i32,
    pub coreDeviceId: String,
    pub coreDeviceInfo: RemoteDeviceInfo,
    pub clientDeviceId: String,
    pub clientDeviceInfo: RemoteDeviceInfo,
    pub transports: Vec<String>,
    pub deviceSpace: CoreSpace,
    pub deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    pub nonce: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteWsEnvelope {
    pub protocolVersion: i32,
    pub sessionId: String,
    pub deviceId: String,
    pub signature: String,
    pub requestId: String,
    #[serde(with = "serde_bytes")]
    pub payloadBytes: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum RemoteWsPayload {
    SessionInfo(RemoteSessionInfoEnvelope),
    PeerChannelOpen(PeerChannelOpenEnvelope),
    PeerChannelClose(String),
    PeerFrame { channelId: String, frame: PeerFrame },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum RemoteWsResponse {
    SessionInfo(RemoteSessionInfoResponse),
    PeerOpened(String),
    PeerFrame(PeerFrame),
    PeerClosed(String),
    PeerAccepted,
    Error(CoreLinkError),
}

/// Selects the concrete carrier used by one paired remote session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkTransportPreference {
    Http,
    #[serde(rename = "ws")]
    WebSocket,
}

/// Migrates pre-transport session records to their original HTTP carrier.
fn defaultLinkTransportPreference() -> LinkTransportPreference {
    LinkTransportPreference::Http
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedRemoteSessionRecord {
    pub baseUrl: String,
    pub sessionId: String,
    pub deviceId: String,
    pub coreDeviceId: String,
    pub remoteDeviceInfo: RemoteDeviceInfo,
    pub pairingServiceVersion: i32,
    pub sessionSecret: String,
    #[serde(default = "defaultLinkTransportPreference")]
    pub transport: LinkTransportPreference,
}

impl PairedRemoteSessionRecord {
    /// Returns this paired session with an updated remote endpoint.
    #[allow(non_snake_case)]
    pub fn withBaseUrl(&self, baseUrl: impl Into<String>) -> Self {
        Self {
            baseUrl: baseUrl.into().trim_end_matches('/').to_string(),
            sessionId: self.sessionId.clone(),
            deviceId: self.deviceId.clone(),
            coreDeviceId: self.coreDeviceId.clone(),
            remoteDeviceInfo: self.remoteDeviceInfo.clone(),
            pairingServiceVersion: self.pairingServiceVersion,
            sessionSecret: self.sessionSecret.clone(),
            transport: self.transport.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairStartState {
    pub pairingId: String,
    pub pairingServiceVersion: i32,
    pub clientDeviceId: String,
    pub clientDeviceInfo: RemoteDeviceInfo,
    pub clientPublicKey: String,
    pub coreDeviceId: String,
    pub coreDeviceInfo: RemoteDeviceInfo,
    pub clientNonce: String,
    pub serverNonce: String,
    pub sharedSecret: Vec<u8>,
}

/// Stores the client-side state needed to finish one outbound pairing after user confirmation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingOutboundPairingRecord {
    pub baseUrl: String,
    pub state: PairStartState,
}

#[derive(Clone, Debug)]
pub struct RemoteLinkClient {
    baseUrl: String,
}

#[cfg(not(target_arch = "wasm32"))]
impl RemoteLinkServer {
    /// Binds and serves one authenticated Link endpoint from its configured address.
    pub async fn serve(
        core: impl CoreNodeLinkClient + Send + 'static,
        config: RemoteLinkServerConfig,
    ) -> Result<(), String> {
        let address: SocketAddr = config
            .bindAddress
            .parse()
            .map_err(|error| format!("invalid bind address: {error}"))?;
        let listener = TcpListener::bind(address)
            .await
            .map_err(|error| error.to_string())?;
        Self::serveWithListener(core, config, listener, address).await
    }

    /// Serves one authenticated Link endpoint from an already bound listener.
    #[allow(non_snake_case)]
    pub async fn serveWithListener(
        core: impl CoreNodeLinkClient + Send + 'static,
        config: RemoteLinkServerConfig,
        listener: TcpListener,
        address: SocketAddr,
    ) -> Result<(), String> {
        CoreSpaceStore::new(config.accessStore.storage.clone()).initialize()?;
        let keySecret = Arc::new(StaticSecret::random_from_rng(OsRng));
        let keyPublic = public_key_to_string(&PublicKey::from(keySecret.as_ref()));
        let webAccessConfig = config.webAccess.clone();
        let (shutdownSender, shutdownReceiver) = oneshot::channel::<()>();
        let sessions = Arc::new(Mutex::new(BTreeMap::new()));
        let acceptedSessions = config.accessStore.inboundSessions()?;
        for (sessionId, session) in acceptedSessions.iter() {
            sessions.lock().await.insert(
                sessionId.clone(),
                RemoteSession {
                    deviceId: session.deviceId.clone(),
                    deviceInfo: session.deviceInfo.clone(),
                    pairingServiceVersion: session.pairingServiceVersion,
                    sessionSecret: BASE64
                        .decode(session.sessionSecret.as_bytes())
                        .map_err(|error| error.to_string())?,
                },
            );
        }
        let webAccess = webAccessConfig.clone().map(|value| RemoteWebAccessState {
            shutdownToken: value.shutdownToken,
            shutdownSender: Arc::new(StdMutex::new(Some(shutdownSender))),
            webRoot: value.webRoot,
            readAsset: value.readAsset,
        });
        let core = Arc::new(StdMutex::new(
            Box::new(core) as Box<dyn CoreNodeLinkClient + Send>
        ));
        let state = RemoteLinkState {
            coreNodeTransport: Arc::new(SharedAccessCoreClient { core }),
            token: config.token.clone(),
            keySecret,
            keyPublic,
            deviceId: config.deviceId.clone(),
            deviceInfo: config.deviceInfo.clone(),
            pairings: Arc::new(Mutex::new(BTreeMap::new())),
            sessions,
            accessStore: config.accessStore.clone(),
            webAccess,
        };
        let mut app = Router::new()
            .route("/link/hello", get(hello))
            .route("/link/pair/start", post(pair_start))
            .route("/link/pair/finish", post(pair_finish))
            .route("/link/session", post(session_info))
            .route("/link/space/adopt", post(space_adopt))
            .route("/link/peer/channel/events", post(peer_channel_events))
            .route("/link/peer/channel/frame", post(peer_channel_frame))
            .route("/link/ws", get(ws));
        if webAccessConfig.is_some() {
            app = app
                .route("/", get(web_access_index))
                .route("/*path", get(web_access_asset))
                .route("/client/web-access/close", post(web_access_close));
        }
        let app = app.with_state(state);
        if config.printStartupInfo {
            println!("operit link server listening on http://{address}");
            println!("link token: {}", config.token);
        }
        if webAccessConfig.is_some() {
            return axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdownReceiver.await;
                })
                .await
                .map_err(|error| error.to_string());
        }
        axum::serve(listener, app)
            .await
            .map_err(|error| error.to_string())
    }
}

impl RemoteLinkClient {
    pub fn new(baseUrl: impl Into<String>) -> Self {
        Self {
            baseUrl: baseUrl.into().trim_end_matches('/').to_string(),
        }
    }

    pub async fn hello(&self, tokenHash: &str) -> Result<HelloResponse, String> {
        decodeRemoteHttpJson(remoteHttpRequest(
            "GET",
            format!("{}/link/hello", self.baseUrl),
            vec![(
                "x-operit-link-token-hash".to_string(),
                tokenHash.to_string(),
            )],
            Vec::new(),
        )?)
    }

    pub async fn pairStart(
        &self,
        tokenHash: &str,
        clientDeviceId: String,
        clientDeviceInfo: RemoteDeviceInfo,
    ) -> Result<PairStartState, String> {
        let clientSecret = StaticSecret::random_from_rng(OsRng);
        let clientPublic = PublicKey::from(&clientSecret);
        if clientDeviceId.trim().is_empty() {
            return Err("pairing client device id must not be empty".to_string());
        }
        let clientNonce = Uuid::new_v4().to_string();
        let request = PairStartRequest {
            pairingServiceVersion: REMOTE_PAIRING_SERVICE_VERSION,
            tokenHash: tokenHash.to_string(),
            clientDeviceId: clientDeviceId.clone(),
            clientDeviceInfo: clientDeviceInfo.clone(),
            clientPublicKey: public_key_to_string(&clientPublic),
            clientNonce: clientNonce.clone(),
        };
        let response: PairStartResponse = decodeRemoteHttpJson(remoteHttpJsonRequest(
            format!("{}/link/pair/start", self.baseUrl),
            &request,
        )?)?;
        let corePublic = parse_public_key(&response.corePublicKey)?;
        let sharedSecret = clientSecret.diffie_hellman(&corePublic).as_bytes().to_vec();
        Ok(PairStartState {
            pairingId: response.pairingId,
            pairingServiceVersion: response.pairingServiceVersion,
            clientDeviceId,
            clientDeviceInfo,
            clientPublicKey: public_key_to_string(&clientPublic),
            coreDeviceId: response.coreDeviceId,
            coreDeviceInfo: response.coreDeviceInfo,
            clientNonce,
            serverNonce: response.serverNonce,
            sharedSecret,
        })
    }

    pub async fn pairFinish(
        &self,
        state: &PairStartState,
        pairingCode: &str,
    ) -> Result<PairedRemoteSession, String> {
        let clientProof = proof(
            &state.sharedSecret,
            &state.clientNonce,
            &state.serverNonce,
            "client",
        );
        let request = PairFinishRequest {
            pairingId: state.pairingId.clone(),
            pairingCode: pairingCode.trim().to_string(),
            clientProof,
        };
        let response: PairFinishResponse = decodeRemoteHttpJson(remoteHttpJsonRequest(
            format!("{}/link/pair/finish", self.baseUrl),
            &request,
        )?)?;
        let expectedCoreProof = proof(
            &state.sharedSecret,
            &state.clientNonce,
            &state.serverNonce,
            "core",
        );
        if response.coreProof != expectedCoreProof {
            return Err("core proof mismatch".to_string());
        }
        Ok(PairedRemoteSession {
            baseUrl: self.baseUrl.clone(),
            sessionId: response.sessionId,
            deviceId: state.clientDeviceId.clone(),
            coreDeviceId: state.coreDeviceId.clone(),
            remoteDeviceInfo: state.coreDeviceInfo.clone(),
            pairingServiceVersion: response.pairingServiceVersion,
            transport: LinkTransportPreference::Http,
            sessionSecret: session_secret(
                &state.sharedSecret,
                &state.clientNonce,
                &state.serverNonce,
            ),
        })
    }
}

/// Executes one authenticated Link HTTP request through the configured runtime host.
fn remoteHttpRequest(
    method: &str,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
) -> Result<Vec<u8>, String> {
    remoteHttpRequestWithReadTimeout(method, url, headers, body, LINK_HTTP_READ_TIMEOUT_SECONDS)
}

/// Executes one authenticated Link HTTP request with an explicit host read window.
fn remoteHttpRequestWithReadTimeout(
    method: &str,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    readTimeoutSeconds: u64,
) -> Result<Vec<u8>, String> {
    let response = defaultHttpHost()
        .executeHttpRequest(HttpRequestData {
            url: url.clone(),
            method: method.to_string(),
            headers,
            body,
            formFields: Vec::new(),
            fileParts: Vec::new(),
            connectTimeoutSeconds: 10,
            readTimeoutSeconds,
            followRedirects: false,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        })
        .map_err(|error| error.to_string())?;
    if !(200..300).contains(&response.statusCode) {
        let description =
            remoteHttpErrorDescription(method, response.statusCode, &url, &response.body);
        return Err(description);
    }
    Ok(response.body)
}

/// Formats one failed Link HTTP response while preserving structured session-auth metadata.
fn remoteHttpErrorDescription(method: &str, statusCode: i32, url: &str, body: &[u8]) -> String {
    let base = format!("Link HTTP {method} {url} failed with status {statusCode}");
    let Ok(error) = operit_link::decodeLink::<CoreLinkError>(body) else {
        return format!("{base}: {}", String::from_utf8_lossy(body));
    };
    if let Some(reason) = coreLinkRemoteSessionAuthReason(&error) {
        return format!(
            "REMOTE_SESSION_AUTH/{reason}: {base}: {}: {}",
            error.code, error.message
        );
    }
    format!("{base}: {}: {}", error.code, error.message)
}

/// Formats one structured Link error and retains remote-session auth metadata.
fn remoteCoreLinkErrorDescription(error: &CoreLinkError) -> String {
    if let Some(reason) = coreLinkRemoteSessionAuthReason(&error) {
        return format!(
            "REMOTE_SESSION_AUTH/{reason}: {}: {}",
            error.code, error.message
        );
    }
    format!("{}: {}", error.code, error.message)
}

/// Extracts the stable auth reason from one structured remote-session Link error.
fn coreLinkRemoteSessionAuthReason(error: &CoreLinkError) -> Option<&str> {
    let Some(operit_link::CoreValue::Map(details)) = error.details.as_ref() else {
        return None;
    };
    let Some(operit_link::CoreValue::String(errorType)) = details.get("type") else {
        return None;
    };
    if errorType != "remote_session_auth" {
        return None;
    }
    let Some(operit_link::CoreValue::String(reason)) = details.get("authReason") else {
        return None;
    };
    Some(reason.as_str())
}

/// Returns the structured auth reason preserved in one remote Link error string.
pub fn remoteSessionAuthReason(error: &str) -> Option<&str> {
    let rest = error.strip_prefix("REMOTE_SESSION_AUTH/")?;
    let (reason, _) = rest.split_once(": ")?;
    Some(reason)
}

/// Encodes one JSON Link control request and executes it through the runtime HTTP host.
fn remoteHttpJsonRequest<T: Serialize>(url: String, request: &T) -> Result<Vec<u8>, String> {
    remoteHttpRequest(
        "POST",
        url,
        vec![("content-type".to_string(), "application/json".to_string())],
        serde_json::to_vec(request).map_err(|error| error.to_string())?,
    )
}

/// Decodes a JSON Link control response from host HTTP response bytes.
fn decodeRemoteHttpJson<T: serde::de::DeserializeOwned>(bytes: Vec<u8>) -> Result<T, String> {
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

#[derive(Clone)]
pub struct PairedRemoteSession {
    baseUrl: String,
    pub sessionId: String,
    pub deviceId: String,
    pub coreDeviceId: String,
    pub remoteDeviceInfo: RemoteDeviceInfo,
    pub pairingServiceVersion: i32,
    pub transport: LinkTransportPreference,
    sessionSecret: Vec<u8>,
}

/// Owns one authenticated WebSocket used by a single Link carrier operation.
pub(crate) struct RemoteWsConnection {
    host: Arc<dyn WebSocketHost>,
    streamId: String,
    sessionId: String,
    deviceId: String,
    sessionSecret: Vec<u8>,
    receiver: Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>,
}

impl RemoteWsConnection {
    /// Opens one authenticated WebSocket through the configured host capability.
    pub(crate) async fn open(
        session: &PairedRemoteSession,
        streamLabel: &str,
    ) -> Result<Arc<Self>, String> {
        let streamId = format!("link-ws-{streamLabel}-{}", Uuid::new_v4().simple());
        let (messageSender, messageReceiver) = tokio::sync::mpsc::unbounded_channel();
        let (openedSender, openedReceiver) = tokio::sync::oneshot::channel();
        let openedSignal = Arc::new(StdMutex::new(Some(openedSender)));
        let openedForClose = openedSignal.clone();
        let openedForOpen = openedSignal.clone();
        let messageCallback: WebSocketMessageCallback = Arc::new(move |message| {
            let _ = messageSender.send(message);
        });
        let openedCallback: WebSocketOpenedCallback = Arc::new(move || {
            if let Some(sender) = openedForOpen
                .lock()
                .expect("WebSocket open signal lock poisoned")
                .take()
            {
                let _ = sender.send(Ok(()));
            }
        });
        let closedCallback: WebSocketClosedCallback = Arc::new(move |result| {
            if let Some(sender) = openedForClose
                .lock()
                .expect("WebSocket open signal lock poisoned")
                .take()
            {
                let _ = sender.send(result);
            }
        });
        let openResult = defaultWebSocketHost().openWebSocket(
            streamId.clone(),
            WebSocketRequestData {
                url: webSocketUrl(&session.baseUrl)?,
                headers: Vec::new(),
                connectTimeoutSeconds: 10,
                ignoreSsl: false,
            },
            openedCallback,
            messageCallback,
            closedCallback,
        );
        if let Err(error) = openResult {
            return Err(error.to_string());
        }
        openedReceiver
            .await
            .map_err(|error| format!("WebSocket open signal closed: {error}"))??;
        Ok(Arc::new(Self {
            host: defaultWebSocketHost(),
            streamId,
            sessionId: session.sessionId.clone(),
            deviceId: session.deviceId.clone(),
            sessionSecret: session.sessionSecret.clone(),
            receiver: Mutex::new(messageReceiver),
        }))
    }

    /// Sends one signed Link payload through the opened WebSocket.
    pub(crate) fn sendPayload(&self, payload: RemoteWsPayload) -> Result<(), String> {
        let payloadBytes = operit_link::encodeLink(&payload).map_err(|error| error.to_string())?;
        let envelope = RemoteWsEnvelope {
            protocolVersion: 3,
            sessionId: self.sessionId.clone(),
            deviceId: self.deviceId.clone(),
            signature: sign(&self.sessionSecret, &payloadBytes),
            requestId: Uuid::new_v4().to_string(),
            payloadBytes,
        };
        let body = operit_link::encodeLink(&envelope).map_err(|error| error.to_string())?;
        self.host
            .sendWebSocketMessage(&self.streamId, body)
            .map_err(|error| error.to_string())
    }

    /// Waits for one typed WebSocket response from the remote Link endpoint.
    pub(crate) async fn nextResponse(&self) -> Result<RemoteWsResponse, String> {
        let mut receiver = self.receiver.lock().await;
        let message = receiver
            .recv()
            .await
            .ok_or_else(|| "WebSocket closed before producing a response".to_string())?;
        operit_link::decodeLink(&message).map_err(|error| error.to_string())
    }

    /// Closes the host-owned WebSocket carrier.
    pub(crate) fn close(&self) -> Result<(), String> {
        self.host
            .closeWebSocket(&self.streamId)
            .map_err(|error| error.to_string())
    }
}

/// Converts one authenticated HTTP Link endpoint into its WebSocket endpoint.
fn webSocketUrl(baseUrl: &str) -> Result<String, String> {
    if let Some(rest) = baseUrl.strip_prefix("https://") {
        return Ok(format!("wss://{rest}/link/ws"));
    }
    if let Some(rest) = baseUrl.strip_prefix("http://") {
        return Ok(format!("ws://{rest}/link/ws"));
    }
    Err("Link base URL must use http:// or https://".to_string())
}

impl PairedRemoteSession {
    #[allow(non_snake_case)]
    pub fn exportRecord(&self) -> PairedRemoteSessionRecord {
        PairedRemoteSessionRecord {
            baseUrl: self.baseUrl.clone(),
            sessionId: self.sessionId.clone(),
            deviceId: self.deviceId.clone(),
            coreDeviceId: self.coreDeviceId.clone(),
            remoteDeviceInfo: self.remoteDeviceInfo.clone(),
            pairingServiceVersion: self.pairingServiceVersion,
            sessionSecret: BASE64.encode(&self.sessionSecret),
            transport: self.transport.clone(),
        }
    }

    #[allow(non_snake_case)]
    pub fn fromRecord(record: PairedRemoteSessionRecord) -> Result<Self, String> {
        Ok(Self {
            baseUrl: record.baseUrl.trim_end_matches('/').to_string(),
            sessionId: record.sessionId,
            deviceId: record.deviceId,
            coreDeviceId: record.coreDeviceId,
            remoteDeviceInfo: record.remoteDeviceInfo,
            pairingServiceVersion: record.pairingServiceVersion,
            transport: record.transport,
            sessionSecret: BASE64
                .decode(record.sessionSecret)
                .map_err(|error| error.to_string())?,
        })
    }

    #[allow(non_snake_case)]
    pub async fn sessionInfo(&self) -> Result<RemoteSessionInfoResponse, String> {
        match self.transport {
            LinkTransportPreference::Http => self.sessionInfoHttp().await,
            LinkTransportPreference::WebSocket => self.sessionInfoWebSocket().await,
        }
    }

    /// Adopts one joined Space projection through the authenticated pairing control plane.
    pub async fn adoptDeviceSpace(
        &self,
        space: CoreSpace,
        deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    ) -> Result<CoreSpace, String> {
        let body = operit_link::encodeLink(&RemoteSpaceAdoptEnvelope {
            space,
            deviceProfiles,
        })
        .map_err(|error| error.to_string())?;
        operit_link::decodeLink(&self.signedRemotePost("space/adopt", body)?)
            .map_err(|error| error.to_string())
    }

    /// Reads session metadata through the HTTP Link carrier.
    #[allow(non_snake_case)]
    async fn sessionInfoHttp(&self) -> Result<RemoteSessionInfoResponse, String> {
        let body = operit_link::encodeLink(&RemoteSessionInfoEnvelope {
            nonce: Uuid::new_v4().to_string(),
        })
        .map_err(|error| error.to_string())?;
        operit_link::decodeLink(&self.signedRemotePostWithReadTimeout(
            "session",
            body,
            SESSION_INFO_READ_TIMEOUT_SECONDS,
        )?)
        .map_err(|error| error.to_string())
    }

    /// Reads session metadata through a dedicated WebSocket carrier.
    #[allow(non_snake_case)]
    async fn sessionInfoWebSocket(&self) -> Result<RemoteSessionInfoResponse, String> {
        let connection = RemoteWsConnection::open(self, "session").await?;
        connection.sendPayload(RemoteWsPayload::SessionInfo(RemoteSessionInfoEnvelope {
            nonce: Uuid::new_v4().to_string(),
        }))?;
        let response = tokio::select! {
            response = connection.nextResponse() => response?,
            delay = defaultHostRuntimeTaskSchedulerHost().waitForHostRuntimeDelay(SESSION_INFO_READ_TIMEOUT_SECONDS * 1000) => {
                let _ = connection.close();
                delay.map_err(|error| error.to_string())?;
                return Err("Remote session info timed out".to_string());
            }
        };
        let _ = connection.close();
        match response {
            RemoteWsResponse::SessionInfo(value) => Ok(value),
            RemoteWsResponse::Error(error) => Err(remoteCoreLinkErrorDescription(&error)),
            _ => Err("unexpected WebSocket session response".to_string()),
        }
    }

    /// Sends one signed HTTP Link request with the standard host read window.
    fn signedRemotePost(&self, path: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
        self.signedRemotePostWithReadTimeout(path, body, LINK_HTTP_READ_TIMEOUT_SECONDS)
    }

    /// Sends one signed HTTP Link request with an explicit host read window.
    fn signedRemotePostWithReadTimeout(
        &self,
        path: &str,
        body: Vec<u8>,
        readTimeoutSeconds: u64,
    ) -> Result<Vec<u8>, String> {
        remoteHttpRequestWithReadTimeout(
            "POST",
            format!("{}/link/{path}", self.baseUrl),
            vec![
                ("x-operit-link-version".to_string(), "3".to_string()),
                ("x-operit-session".to_string(), self.sessionId.clone()),
                ("x-operit-device".to_string(), self.deviceId.clone()),
                (
                    "x-operit-signature".to_string(),
                    sign(&self.sessionSecret, &body),
                ),
            ],
            body,
            readTimeoutSeconds,
        )
    }
}

/// Returns the authenticated CoreNode and its current live Space identity.
#[cfg(not(target_arch = "wasm32"))]
async fn hello(State(state): State<RemoteLinkState>, headers: HeaderMap) -> Response {
    if !token_matches(&state, &headers) {
        return unauthorized("invalid token");
    }
    let spaceStore = CoreSpaceStore::new(state.accessStore.storage.clone());
    let space = match spaceStore.initialize() {
        Ok(space) => space,
        Err(error) => return internal_server_error(error),
    };
    let profiles = match spaceStore.deviceProfiles() {
        Ok(profiles) => profiles,
        Err(error) => return internal_server_error(error),
    };
    let profile = match profiles.get(&state.deviceId) {
        Some(profile) => profile,
        None => return internal_server_error("Current device profile is not initialized"),
    };
    Json(HelloResponse {
        protocolVersion: 3,
        pairingServiceVersion: REMOTE_PAIRING_SERVICE_VERSION,
        coreDeviceId: state.deviceId,
        coreDeviceInfo: state.deviceInfo,
        deviceSpace: RemoteDeviceSpaceInfo {
            spaceId: space.spaceId,
            spaceName: space.spaceName,
            spaceRevision: space.spaceRevision,
            deviceCount: space.members.len(),
            userName: profile.userName.clone(),
        },
        corePublicKey: state.keyPublic,
        transports: vec!["http".to_string(), "ws".to_string()],
        pairingRequired: true,
    })
    .into_response()
}

#[cfg(not(target_arch = "wasm32"))]
async fn pair_start(
    State(state): State<RemoteLinkState>,
    Json(request): Json<PairStartRequest>,
) -> Response {
    if !token_hash_matches(&state, &request.tokenHash) {
        return unauthorized("invalid token");
    }
    let clientPublic = match parse_public_key(&request.clientPublicKey) {
        Ok(value) => value,
        Err(error) => return bad_request(error),
    };
    let sharedSecret = state
        .keySecret
        .diffie_hellman(&clientPublic)
        .as_bytes()
        .to_vec();
    let pairingId = Uuid::new_v4().to_string();
    let pairingCode = pairing_code();
    let serverNonce = Uuid::new_v4().to_string();
    eprintln!(
        "operit link pairing code for {}: {}",
        request.clientDeviceId, pairingCode
    );
    let pairingRecord = RemotePairingCodeRecord {
        pairingId: pairingId.clone(),
        pairingServiceVersion: request.pairingServiceVersion,
        clientDeviceId: request.clientDeviceId.clone(),
        clientDeviceInfo: request.clientDeviceInfo.clone(),
        pairingCode: pairingCode.clone(),
        createdAt: unix_millis(),
    };
    if let Err(error) = state.accessStore.savePendingPairing(pairingRecord.clone()) {
        return internal_server_error(error);
    }
    state.pairings.lock().await.insert(
        pairingId.clone(),
        PendingPairing {
            pairingServiceVersion: request.pairingServiceVersion,
            clientDeviceId: request.clientDeviceId,
            clientDeviceInfo: request.clientDeviceInfo,
            clientPublicKey: request.clientPublicKey,
            pairingCode,
            serverNonce: serverNonce.clone(),
            clientNonce: request.clientNonce,
            sharedSecret,
        },
    );
    publishOwnerWebAccessPairing(RuntimeHostInteractionWebAccessPairingPayload {
        pairingId: pairingRecord.pairingId,
        clientDeviceId: pairingRecord.clientDeviceId,
        clientPlatform: pairingRecord.clientDeviceInfo.platform,
        clientModel: pairingRecord.clientDeviceInfo.model,
        pairingCode: pairingRecord.pairingCode,
        createdAt: pairingRecord.createdAt,
    });
    Json(PairStartResponse {
        pairingId,
        pairingServiceVersion: REMOTE_PAIRING_SERVICE_VERSION,
        coreDeviceId: state.deviceId,
        coreDeviceInfo: state.deviceInfo,
        corePublicKey: state.keyPublic,
        serverNonce,
    })
    .into_response()
}

#[cfg(not(target_arch = "wasm32"))]
async fn pair_finish(
    State(state): State<RemoteLinkState>,
    Json(request): Json<PairFinishRequest>,
) -> Response {
    let Some(pairing) = state.pairings.lock().await.get(&request.pairingId).cloned() else {
        return bad_request("pairing not found");
    };
    if pairing.pairingCode != request.pairingCode.trim() {
        return unauthorized("invalid pairing code");
    }
    let expectedClientProof = proof(
        &pairing.sharedSecret,
        &pairing.clientNonce,
        &pairing.serverNonce,
        "client",
    );
    if request.clientProof != expectedClientProof {
        return unauthorized("invalid client proof");
    }
    let sessionId = request.pairingId.clone();
    let sessionSecret = session_secret(
        &pairing.sharedSecret,
        &pairing.clientNonce,
        &pairing.serverNonce,
    );
    let record = AcceptedRemoteSessionRecord {
        deviceId: pairing.clientDeviceId.clone(),
        deviceInfo: pairing.clientDeviceInfo.clone(),
        pairingServiceVersion: pairing.pairingServiceVersion,
        sessionSecret: BASE64.encode(sessionSecret.as_slice()),
    };
    if let Err(error) = state
        .accessStore
        .saveInboundSession(sessionId.clone(), record)
    {
        return internal_server_error(error);
    }
    if let Err(error) = state.accessStore.removePendingPairing(&request.pairingId) {
        return internal_server_error(error);
    }
    state.sessions.lock().await.insert(
        sessionId.clone(),
        RemoteSession {
            deviceId: pairing.clientDeviceId,
            deviceInfo: pairing.clientDeviceInfo,
            pairingServiceVersion: pairing.pairingServiceVersion,
            sessionSecret,
        },
    );
    state.pairings.lock().await.remove(&request.pairingId);
    Json(PairFinishResponse {
        sessionId,
        pairingServiceVersion: pairing.pairingServiceVersion,
        coreProof: proof(
            &pairing.sharedSecret,
            &pairing.clientNonce,
            &pairing.serverNonce,
            "core",
        ),
    })
    .into_response()
}

#[cfg(not(target_arch = "wasm32"))]
async fn session_info(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let verified = match verify_session(&state, &headers, &body).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let envelope = match operit_link::decodeLink::<RemoteSessionInfoEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => {
            return encode_link_response(
                StatusCode::BAD_REQUEST,
                CoreLinkError::new("BAD_REQUEST", error.to_string()),
            );
        }
    };
    let spaceStore = CoreSpaceStore::new(state.accessStore.storage.clone());
    let space = match spaceStore.initialize() {
        Ok(value) => value,
        Err(error) => return internal_server_error(error),
    };
    let deviceProfiles = match spaceStore.deviceProfilesForCurrentSpace() {
        Ok(value) => value,
        Err(error) => return internal_server_error(error),
    };
    let sessions = state.sessions.lock().await;
    let Some(session) = sessions.get(&verified.sessionId) else {
        return encode_link_response(
            StatusCode::UNAUTHORIZED,
            remote_session_auth_error("invalid session", "invalid_session"),
        );
    };
    encode_link_response(
        StatusCode::OK,
        RemoteSessionInfoResponse {
            protocolVersion: 3,
            pairingServiceVersion: session.pairingServiceVersion,
            coreDeviceId: state.deviceId,
            coreDeviceInfo: state.deviceInfo,
            clientDeviceId: session.deviceId.clone(),
            clientDeviceInfo: session.deviceInfo.clone(),
            transports: vec!["http".to_string(), "ws".to_string()],
            deviceSpace: space,
            deviceProfiles,
            nonce: envelope.nonce,
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
struct ServerPeerFrameSender {
    sender: StdMutex<Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>>>,
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl PeerFrameSender for ServerPeerFrameSender {
    /// Queues one length-prefixed frame for the connected CoreNode.
    async fn send(&self, frame: PeerFrame) -> Result<(), String> {
        self.sender
            .lock()
            .map_err(|error| error.to_string())?
            .as_ref()
            .ok_or_else(|| "Peer Link response stream is closed".to_string())?
            .send(encodePeerFrame(&frame)?)
            .map_err(|_| "Peer Link response stream is closed".to_string())
    }

    /// Ends the server response stream owned by this carrier.
    fn close(&self) {
        let _ = self
            .sender
            .lock()
            .expect("Peer Link response sender lock poisoned")
            .take();
    }
}

/// Adapts queued Peer Link frames into an Axum response body stream.
#[cfg(not(target_arch = "wasm32"))]
struct ServerPeerFrameStream {
    receiver: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    connection: Arc<PeerConnection>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FuturesStream for ServerPeerFrameStream {
    type Item = Result<Bytes, Infallible>;

    /// Polls the next queued Peer Link frame.
    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver)
            .poll_recv(context)
            .map(|item| item.map(|bytes| Ok(Bytes::from(bytes))))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for ServerPeerFrameStream {
    /// Removes the active Peer Link when its HTTP response stream closes.
    fn drop(&mut self) {
        operit_util::AppLogger::AppLogger::w(
            "PeerCarrierTrace",
            "server_response_stream_drop reason=Peer Link response stream closed",
        );
        self.connection
            .close("Peer Link response stream closed".to_string());
    }
}

/// Opens the server-to-client event stream for one authenticated direct Peer Link.
#[cfg(not(target_arch = "wasm32"))]
async fn peer_channel_events(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let verified = match verify_session(&state, &headers, &body).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let envelope = match operit_link::decodeLink::<PeerChannelOpenEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => {
            return encode_link_response(
                StatusCode::BAD_REQUEST,
                CoreLinkError::new("BAD_REQUEST", error.to_string()),
            );
        }
    };
    if envelope.channelId.trim().is_empty() {
        return encode_link_response(
            StatusCode::BAD_REQUEST,
            CoreLinkError::new("BAD_REQUEST", "Peer Link channel id must not be empty"),
        );
    }
    let spaceStore = CoreSpaceStore::new(state.accessStore.storage.clone());
    match spaceStore.contains(verified.deviceId.clone()) {
        Ok(true) => {}
        Ok(false) => {
            return encode_link_response(
                StatusCode::FORBIDDEN,
                CoreLinkError::new(
                    "SPACE_MEMBER_REQUIRED",
                    "Paired device is not in this device space",
                ),
            );
        }
        Err(error) => return internal_server_error(error),
    }
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let connection = PeerConnection::new(
        state.deviceId.clone(),
        verified.deviceId,
        envelope.channelId,
        Arc::new(ServerPeerFrameSender {
            sender: StdMutex::new(Some(sender)),
        }),
        state.coreNodeTransport.clone(),
        Some(spaceStore),
    );
    if let Err(error) = registerPeerLink(connection.clone()) {
        return encode_link_response(
            StatusCode::CONFLICT,
            CoreLinkError::new("PEER_LINK_ALREADY_ACTIVE", error),
        );
    }
    Response::builder()
        .header("content-type", "application/msgpack-seq")
        .body(Body::from_stream(ServerPeerFrameStream {
            receiver,
            connection,
        }))
        .expect("Peer Link channel response must build")
}

/// Receives one ordered batch of client-to-server frames for an active Peer Link.
#[cfg(not(target_arch = "wasm32"))]
async fn peer_channel_frame(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let verified = match verify_session(&state, &headers, &body).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let batch = match operit_link::decodeLink::<PeerFrameBatch>(&body) {
        Ok(value) => value,
        Err(error) => {
            return encode_link_response(
                StatusCode::BAD_REQUEST,
                CoreLinkError::new("PEER_FRAME_BATCH_INVALID", error.to_string()),
            );
        }
    };
    if batch.frames.is_empty() {
        return encode_link_response(
            StatusCode::BAD_REQUEST,
            CoreLinkError::new(
                "PEER_FRAME_BATCH_EMPTY",
                "Peer frame batch must not be empty",
            ),
        );
    }
    for frame in batch.frames {
        if let Err(error) = receivePeerFrame(&state.deviceId, &verified.deviceId, frame).await {
            return encode_link_response(
                StatusCode::BAD_REQUEST,
                CoreLinkError::new("PEER_FRAME_REJECTED", error),
            );
        }
    }
    encode_link_response(StatusCode::OK, serde_json::json!({ "ok": true }))
}

#[cfg(not(target_arch = "wasm32"))]
async fn web_access_index(State(state): State<RemoteLinkState>) -> Response {
    let Some(webAccess) = state.webAccess.as_ref() else {
        return bad_request("web access is not enabled");
    };
    serve_web_access_file(webAccess, "index.html")
}

#[cfg(not(target_arch = "wasm32"))]
async fn web_access_asset(
    State(state): State<RemoteLinkState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let Some(webAccess) = state.webAccess.as_ref() else {
        return bad_request("web access is not enabled");
    };
    serve_web_access_file(webAccess, &path)
}

#[cfg(not(target_arch = "wasm32"))]
async fn web_access_close(State(state): State<RemoteLinkState>, headers: HeaderMap) -> Response {
    let Some(webAccess) = state.webAccess.as_ref() else {
        return bad_request("web access is not enabled");
    };
    let token = header_string(&headers, "x-operit-web-access-shutdown-token");
    if token.as_deref() != Some(webAccess.shutdownToken.as_str()) {
        return unauthorized("invalid web access shutdown token");
    }
    let sender = webAccess
        .shutdownSender
        .lock()
        .expect("web access shutdown mutex poisoned")
        .take();
    let Some(sender) = sender else {
        return bad_request("web access close already requested");
    };
    if sender.send(()).is_err() {
        return bad_request("web access shutdown receiver is closed");
    }
    Json(serde_json::json!({"ok": true})).into_response()
}

/// Serves the standalone Web Access index document.
#[cfg(not(target_arch = "wasm32"))]
async fn static_web_access_index(State(state): State<StaticWebAccessState>) -> Response {
    serve_static_web_access_file(&state, "index.html")
}

/// Serves one standalone Web Access asset.
#[cfg(not(target_arch = "wasm32"))]
async fn static_web_access_asset(
    State(state): State<StaticWebAccessState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    serve_static_web_access_file(&state, &path)
}

/// Stops the standalone Web Access server after validating its control token.
#[cfg(not(target_arch = "wasm32"))]
async fn static_web_access_close(
    State(state): State<StaticWebAccessState>,
    headers: HeaderMap,
) -> Response {
    let token = header_string(&headers, "x-operit-web-access-shutdown-token");
    if token.as_deref() != Some(state.shutdownToken.as_str()) {
        return unauthorized("invalid web access shutdown token");
    }
    let sender = state
        .shutdownSender
        .lock()
        .expect("static web access shutdown mutex poisoned")
        .take();
    let Some(sender) = sender else {
        return bad_request("web access close already requested");
    };
    if sender.send(()).is_err() {
        return bad_request("web access shutdown receiver is closed");
    }
    Json(serde_json::json!({"ok": true})).into_response()
}

/// Returns the pairing control state attached to the static Web Access server.
#[cfg(not(target_arch = "wasm32"))]
fn static_web_access_control(state: &StaticWebAccessState) -> &StaticWebAccessControlState {
    state
        .control
        .as_ref()
        .expect("static Web Access pairing control is not configured")
}

/// Returns the authenticated device and Space metadata used during pairing.
#[cfg(not(target_arch = "wasm32"))]
async fn static_web_access_hello(
    State(state): State<StaticWebAccessState>,
    headers: HeaderMap,
) -> Response {
    let control = static_web_access_control(&state);
    if header_string(&headers, "x-operit-link-token-hash").as_deref()
        != Some(link_token_hash(&control.token).as_str())
    {
        return unauthorized("invalid token");
    }
    let spaceStore = CoreSpaceStore::new(control.accessStore.storage.clone());
    let space = match spaceStore.initialize() {
        Ok(value) => value,
        Err(error) => return internal_server_error(error),
    };
    let profiles = match spaceStore.deviceProfiles() {
        Ok(value) => value,
        Err(error) => return internal_server_error(error),
    };
    let Some(profile) = profiles.get(&control.deviceId) else {
        return internal_server_error("Current device profile is not initialized");
    };
    Json(HelloResponse {
        protocolVersion: 3,
        pairingServiceVersion: REMOTE_PAIRING_SERVICE_VERSION,
        coreDeviceId: control.deviceId.clone(),
        coreDeviceInfo: control.deviceInfo.clone(),
        deviceSpace: RemoteDeviceSpaceInfo {
            spaceId: space.spaceId,
            spaceName: space.spaceName,
            spaceRevision: space.spaceRevision,
            deviceCount: space.members.len(),
            userName: profile.userName.clone(),
        },
        corePublicKey: control.keyPublic.clone(),
        transports: vec!["http".to_string(), "ws".to_string()],
        pairingRequired: true,
    })
    .into_response()
}

/// Begins one pairing transaction without creating any Core route capability.
#[cfg(not(target_arch = "wasm32"))]
async fn static_web_access_pair_start(
    State(state): State<StaticWebAccessState>,
    Json(request): Json<PairStartRequest>,
) -> Response {
    let control = static_web_access_control(&state);
    if request.tokenHash != link_token_hash(&control.token) {
        return unauthorized("invalid token");
    }
    let clientPublic = match parse_public_key(&request.clientPublicKey) {
        Ok(value) => value,
        Err(error) => return bad_request(error),
    };
    let sharedSecret = control
        .keySecret
        .diffie_hellman(&clientPublic)
        .as_bytes()
        .to_vec();
    let pairingId = Uuid::new_v4().to_string();
    let pairingCode = pairing_code();
    let serverNonce = Uuid::new_v4().to_string();
    eprintln!(
        "operit link pairing code for {}: {}",
        request.clientDeviceId, pairingCode
    );
    let pairingRecord = RemotePairingCodeRecord {
        pairingId: pairingId.clone(),
        pairingServiceVersion: request.pairingServiceVersion,
        clientDeviceId: request.clientDeviceId.clone(),
        clientDeviceInfo: request.clientDeviceInfo.clone(),
        pairingCode: pairingCode.clone(),
        createdAt: unix_millis(),
    };
    if let Err(error) = control
        .accessStore
        .savePendingPairing(pairingRecord.clone())
    {
        return internal_server_error(error);
    }
    control.pairings.lock().await.insert(
        pairingId.clone(),
        PendingPairing {
            pairingServiceVersion: request.pairingServiceVersion,
            clientDeviceId: request.clientDeviceId.clone(),
            clientDeviceInfo: request.clientDeviceInfo.clone(),
            clientPublicKey: request.clientPublicKey,
            pairingCode,
            serverNonce: serverNonce.clone(),
            clientNonce: request.clientNonce,
            sharedSecret,
        },
    );
    publishOwnerWebAccessPairing(RuntimeHostInteractionWebAccessPairingPayload {
        pairingId: pairingRecord.pairingId,
        clientDeviceId: pairingRecord.clientDeviceId,
        clientPlatform: pairingRecord.clientDeviceInfo.platform,
        clientModel: pairingRecord.clientDeviceInfo.model,
        pairingCode: pairingRecord.pairingCode,
        createdAt: pairingRecord.createdAt,
    });
    Json(PairStartResponse {
        pairingId,
        pairingServiceVersion: REMOTE_PAIRING_SERVICE_VERSION,
        coreDeviceId: control.deviceId.clone(),
        coreDeviceInfo: control.deviceInfo.clone(),
        corePublicKey: control.keyPublic.clone(),
        serverNonce,
    })
    .into_response()
}

/// Completes one pairing transaction and persists its authenticated session.
#[cfg(not(target_arch = "wasm32"))]
async fn static_web_access_pair_finish(
    State(state): State<StaticWebAccessState>,
    Json(request): Json<PairFinishRequest>,
) -> Response {
    let control = static_web_access_control(&state);
    let Some(pairing) = control
        .pairings
        .lock()
        .await
        .get(&request.pairingId)
        .cloned()
    else {
        return bad_request("pairing not found");
    };
    if pairing.pairingCode != request.pairingCode.trim() {
        return unauthorized("invalid pairing code");
    }
    let expectedClientProof = proof(
        &pairing.sharedSecret,
        &pairing.clientNonce,
        &pairing.serverNonce,
        "client",
    );
    if request.clientProof != expectedClientProof {
        return unauthorized("invalid client proof");
    }
    let sessionId = request.pairingId.clone();
    let sessionSecret = session_secret(
        &pairing.sharedSecret,
        &pairing.clientNonce,
        &pairing.serverNonce,
    );
    let record = AcceptedRemoteSessionRecord {
        deviceId: pairing.clientDeviceId.clone(),
        deviceInfo: pairing.clientDeviceInfo.clone(),
        pairingServiceVersion: pairing.pairingServiceVersion,
        sessionSecret: BASE64.encode(sessionSecret.as_slice()),
    };
    if let Err(error) = control
        .accessStore
        .saveInboundSession(sessionId.clone(), record)
    {
        return internal_server_error(error);
    }
    if let Err(error) = control.accessStore.removePendingPairing(&request.pairingId) {
        return internal_server_error(error);
    }
    control.sessions.lock().await.insert(
        sessionId.clone(),
        RemoteSession {
            deviceId: pairing.clientDeviceId,
            deviceInfo: pairing.clientDeviceInfo,
            pairingServiceVersion: pairing.pairingServiceVersion,
            sessionSecret,
        },
    );
    control.pairings.lock().await.remove(&request.pairingId);
    Json(PairFinishResponse {
        sessionId,
        pairingServiceVersion: pairing.pairingServiceVersion,
        coreProof: proof(
            &pairing.sharedSecret,
            &pairing.clientNonce,
            &pairing.serverNonce,
            "core",
        ),
    })
    .into_response()
}

/// Returns one authenticated session summary for pairing clients.
#[cfg(not(target_arch = "wasm32"))]
async fn static_web_access_session_info(
    State(state): State<StaticWebAccessState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let control = static_web_access_control(&state);
    let verified = match verify_static_web_access_session(control, &headers, &body).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let envelope = match operit_link::decodeLink::<RemoteSessionInfoEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => {
            return encode_link_response(
                StatusCode::BAD_REQUEST,
                CoreLinkError::new("BAD_REQUEST", error.to_string()),
            )
        }
    };
    let spaceStore = CoreSpaceStore::new(control.accessStore.storage.clone());
    let space = match spaceStore.initialize() {
        Ok(value) => value,
        Err(error) => return internal_server_error(error),
    };
    let deviceProfiles = match spaceStore.deviceProfilesForCurrentSpace() {
        Ok(value) => value,
        Err(error) => return internal_server_error(error),
    };
    let sessions = control.sessions.lock().await;
    let Some(session) = sessions.get(&verified.sessionId) else {
        return encode_link_response(
            StatusCode::UNAUTHORIZED,
            remote_session_auth_error("invalid session", "invalid_session"),
        );
    };
    encode_link_response(
        StatusCode::OK,
        RemoteSessionInfoResponse {
            protocolVersion: 3,
            pairingServiceVersion: session.pairingServiceVersion,
            coreDeviceId: control.deviceId.clone(),
            coreDeviceInfo: control.deviceInfo.clone(),
            clientDeviceId: session.deviceId.clone(),
            clientDeviceInfo: session.deviceInfo.clone(),
            transports: vec!["http".to_string(), "ws".to_string()],
            deviceSpace: space,
            deviceProfiles,
            nonce: envelope.nonce,
        },
    )
}

/// Applies one authenticated Space projection through the standalone pairing server.
#[cfg(not(target_arch = "wasm32"))]
async fn static_web_access_space_adopt(
    State(state): State<StaticWebAccessState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let control = static_web_access_control(&state);
    if let Err(response) = verify_static_web_access_session(control, &headers, &body).await {
        return response;
    }
    let envelope = match operit_link::decodeLink::<RemoteSpaceAdoptEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => {
            return encode_link_response(
                StatusCode::BAD_REQUEST,
                CoreLinkError::new("BAD_REQUEST", error.to_string()),
            )
        }
    };
    let spaceStore = CoreSpaceStore::new(control.accessStore.storage.clone());
    if let Err(error) = spaceStore.importDeviceProfiles(envelope.deviceProfiles) {
        return encode_link_response(StatusCode::CONFLICT, CoreLinkError::internal(error));
    }
    match spaceStore.adopt(envelope.space) {
        Ok(space) => encode_link_response(StatusCode::OK, space),
        Err(error) => encode_link_response(StatusCode::CONFLICT, CoreLinkError::internal(error)),
    }
}

/// Applies one authenticated Space projection during the pairing join handshake.
#[cfg(not(target_arch = "wasm32"))]
async fn space_adopt(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    let envelope = match operit_link::decodeLink::<RemoteSpaceAdoptEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => {
            return encode_link_response(
                StatusCode::BAD_REQUEST,
                CoreLinkError::new("BAD_REQUEST", error.to_string()),
            )
        }
    };
    let spaceStore = CoreSpaceStore::new(state.accessStore.storage.clone());
    if let Err(error) = spaceStore.importDeviceProfiles(envelope.deviceProfiles) {
        return encode_link_response(StatusCode::CONFLICT, CoreLinkError::internal(error));
    }
    match spaceStore.adopt(envelope.space) {
        Ok(space) => encode_link_response(StatusCode::OK, space),
        Err(error) => encode_link_response(StatusCode::CONFLICT, CoreLinkError::internal(error)),
    }
}

/// Verifies one session request against the pairing-only control state.
#[cfg(not(target_arch = "wasm32"))]
async fn verify_static_web_access_session(
    state: &StaticWebAccessControlState,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<VerifiedRemoteSession, Response> {
    if header_string(headers, "x-operit-link-version").as_deref() != Some("3") {
        return Err(encode_link_response(
            StatusCode::BAD_REQUEST,
            CoreLinkError::new(
                "LINK_VERSION_MISMATCH",
                "Link protocol version 3 is required",
            ),
        ));
    }
    let Some(sessionId) = header_string(headers, "x-operit-session") else {
        return Err(encode_link_response(
            StatusCode::UNAUTHORIZED,
            CoreLinkError::new("UNAUTHORIZED", "missing session"),
        ));
    };
    let Some(deviceId) = header_string(headers, "x-operit-device") else {
        return Err(encode_link_response(
            StatusCode::UNAUTHORIZED,
            CoreLinkError::new("UNAUTHORIZED", "missing device"),
        ));
    };
    let Some(signature) = header_string(headers, "x-operit-signature") else {
        return Err(encode_link_response(
            StatusCode::UNAUTHORIZED,
            CoreLinkError::new("UNAUTHORIZED", "missing signature"),
        ));
    };
    let records = state.accessStore.inboundSessions().map_err(|error| {
        encode_link_response(StatusCode::UNAUTHORIZED, CoreLinkError::internal(error))
    })?;
    let Some(record) = records.get(&sessionId) else {
        return Err(encode_link_response(
            StatusCode::UNAUTHORIZED,
            remote_session_auth_error("invalid session", "invalid_session"),
        ));
    };
    let session = accepted_session_from_record(record)
        .map_err(|error| encode_link_response(StatusCode::UNAUTHORIZED, error))?;
    if session.deviceId != deviceId {
        return Err(encode_link_response(
            StatusCode::UNAUTHORIZED,
            remote_session_auth_error("device mismatch", "device_mismatch"),
        ));
    }
    if sign(&session.sessionSecret, body) != signature {
        return Err(encode_link_response(
            StatusCode::UNAUTHORIZED,
            remote_session_auth_error("signature mismatch", "signature_mismatch"),
        ));
    }
    Ok(VerifiedRemoteSession {
        sessionId,
        deviceId,
    })
}

#[cfg(not(target_arch = "wasm32"))]
async fn ws(State(state): State<RemoteLinkState>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| handle_ws(socket, state))
        .into_response()
}

#[cfg(not(target_arch = "wasm32"))]
async fn handle_ws(mut socket: WebSocket, state: RemoteLinkState) {
    while let Some(Ok(message)) = socket.recv().await {
        match message {
            Message::Binary(bytes) => {
                let envelope = match operit_link::decodeLink::<RemoteWsEnvelope>(&bytes) {
                    Ok(value) => value,
                    Err(error) => {
                        let response = operit_link::encodeLink(RemoteWsResponse::Error(
                            CoreLinkError::new("BAD_REQUEST", error.to_string()),
                        ))
                        .expect("RemoteWsResponse must serialize");
                        let _ = socket.send(Message::Binary(response)).await;
                        continue;
                    }
                };
                let payload =
                    match operit_link::decodeLink::<RemoteWsPayload>(&envelope.payloadBytes) {
                        Ok(value) => value,
                        Err(error) => {
                            let response = operit_link::encodeLink(RemoteWsResponse::Error(
                                CoreLinkError::new("BAD_REQUEST", error.to_string()),
                            ))
                            .expect("RemoteWsResponse must serialize");
                            let _ = socket.send(Message::Binary(response)).await;
                            continue;
                        }
                    };
                match payload {
                    #[cfg(not(target_arch = "wasm32"))]
                    RemoteWsPayload::PeerChannelOpen(request) => {
                        handle_ws_peer(&mut socket, &state, envelope, request).await;
                        return;
                    }
                    RemoteWsPayload::SessionInfo(_) => {
                        let response = handle_ws_binary(&state, &bytes).await;
                        let _ = socket.send(Message::Binary(response)).await;
                    }
                    _ => {
                        let response =
                            operit_link::encodeLink(RemoteWsResponse::Error(CoreLinkError::new(
                                "WS_STREAM_MODE_REMOVED",
                                "Legacy remote Link streams are removed",
                            )))
                            .expect("RemoteWsResponse must serialize");
                        let _ = socket.send(Message::Binary(response)).await;
                    }
                }
            }
            Message::Close(frame) => {
                let _ = socket.send(Message::Close(frame)).await;
                break;
            }
            _ => {}
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn send_ws_response(
    socket: &mut WebSocket,
    response: RemoteWsResponse,
) -> Result<(), String> {
    let bytes = operit_link::encodeLink(response).map_err(|error| error.to_string())?;
    socket
        .send(Message::Binary(bytes))
        .await
        .map_err(|error| error.to_string())
}

/// Verifies one signed WebSocket request and returns its authenticated session.
#[cfg(not(target_arch = "wasm32"))]
async fn verify_ws_envelope(
    state: &RemoteLinkState,
    envelope: &RemoteWsEnvelope,
) -> Result<VerifiedRemoteSession, CoreLinkError> {
    if envelope.protocolVersion != 3 {
        return Err(CoreLinkError::new(
            "LINK_VERSION_MISMATCH",
            "Link protocol version 3 is required",
        ));
    }
    verify_session_parts(
        state,
        &envelope.sessionId,
        &envelope.deviceId,
        &envelope.signature,
        &envelope.payloadBytes,
    )
    .await
}

/// Sends and receives one authenticated Peer Link over a WebSocket connection.
#[cfg(not(target_arch = "wasm32"))]
async fn handle_ws_peer(
    socket: &mut WebSocket,
    state: &RemoteLinkState,
    envelope: RemoteWsEnvelope,
    request: PeerChannelOpenEnvelope,
) {
    let verified = match verify_ws_envelope(state, &envelope).await {
        Ok(value) => value,
        Err(error) => {
            let _ = send_ws_response(socket, RemoteWsResponse::Error(error)).await;
            return;
        }
    };
    if request.channelId.trim().is_empty() {
        let _ = send_ws_response(
            socket,
            RemoteWsResponse::Error(CoreLinkError::new(
                "BAD_REQUEST",
                "Peer Link channel id must not be empty",
            )),
        )
        .await;
        return;
    }
    let spaceStore = CoreSpaceStore::new(state.accessStore.storage.clone());
    match spaceStore.contains(verified.deviceId.clone()) {
        Ok(true) => {}
        Ok(false) => {
            let _ = send_ws_response(
                socket,
                RemoteWsResponse::Error(CoreLinkError::new(
                    "SPACE_MEMBER_REQUIRED",
                    "Paired device is not in this device space",
                )),
            )
            .await;
            return;
        }
        Err(error) => {
            let _ = send_ws_response(
                socket,
                RemoteWsResponse::Error(CoreLinkError::internal(error)),
            )
            .await;
            return;
        }
    }
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let connection = PeerConnection::new(
        state.deviceId.clone(),
        verified.deviceId.clone(),
        request.channelId.clone(),
        Arc::new(WsPeerFrameSender {
            sender: StdMutex::new(Some(sender)),
        }),
        state.coreNodeTransport.clone(),
        Some(spaceStore),
    );
    if let Err(error) = registerPeerLink(connection.clone()) {
        let _ = send_ws_response(
            socket,
            RemoteWsResponse::Error(CoreLinkError::new("PEER_LINK_ALREADY_ACTIVE", error)),
        )
        .await;
        return;
    }
    if send_ws_response(
        socket,
        RemoteWsResponse::PeerOpened(request.channelId.clone()),
    )
    .await
    .is_err()
    {
        connection.close("Peer WebSocket closed before opening".to_string());
        return;
    }
    loop {
        tokio::select! {
            frame = receiver.recv() => {
                let Some(frame) = frame else { return; };
                if send_ws_response(socket, RemoteWsResponse::PeerFrame(frame)).await.is_err() {
                    connection.close("Peer WebSocket send failed".to_string());
                    return;
                }
            }
            message = socket.recv() => {
                let Some(Ok(message)) = message else {
                    connection.close("Peer WebSocket closed".to_string());
                    return;
                };
                let Message::Binary(bytes) = message else { continue; };
                let incoming = match operit_link::decodeLink::<RemoteWsEnvelope>(&bytes) {
                    Ok(value) => value,
                    Err(error) => {
                        let _ = send_ws_response(socket, RemoteWsResponse::Error(CoreLinkError::new("BAD_REQUEST", error.to_string()))).await;
                        continue;
                    }
                };
                if verify_ws_envelope(state, &incoming).await.is_err() {
                    let _ = send_ws_response(socket, RemoteWsResponse::Error(CoreLinkError::new("UNAUTHORIZED", "invalid WebSocket session"))).await;
                    connection.close("Peer WebSocket authentication failed".to_string());
                    return;
                }
                let payload = match operit_link::decodeLink::<RemoteWsPayload>(&incoming.payloadBytes) {
                    Ok(value) => value,
                    Err(error) => {
                        let _ = send_ws_response(socket, RemoteWsResponse::Error(CoreLinkError::new("BAD_REQUEST", error.to_string()))).await;
                        continue;
                    }
                };
                match payload {
                    RemoteWsPayload::PeerFrame { channelId, frame } if channelId == request.channelId => {
                        if let Err(error) = receivePeerFrame(&state.deviceId, &verified.deviceId, frame).await {
                            let _ = send_ws_response(socket, RemoteWsResponse::Error(CoreLinkError::new("PEER_FRAME_REJECTED", error))).await;
                            connection.close("Peer frame rejected".to_string());
                            return;
                        }
                    }
                    RemoteWsPayload::PeerChannelClose(channelId) if channelId == request.channelId => {
                        connection.close("Peer WebSocket closed by owner".to_string());
                        let _ = send_ws_response(socket, RemoteWsResponse::PeerClosed(channelId)).await;
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Sends server-originated Peer frames through a WebSocket writer queue.
#[cfg(not(target_arch = "wasm32"))]
struct WsPeerFrameSender {
    sender: StdMutex<Option<tokio::sync::mpsc::UnboundedSender<PeerFrame>>>,
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl PeerFrameSender for WsPeerFrameSender {
    /// Queues one ordered Peer frame for the active WebSocket.
    async fn send(&self, frame: PeerFrame) -> Result<(), String> {
        let sender = self
            .sender
            .lock()
            .map_err(|error| error.to_string())?
            .as_ref()
            .cloned()
            .ok_or_else(|| "Peer WebSocket sender is closed".to_string())?;
        sender
            .send(frame)
            .map_err(|_| "Peer WebSocket sender is closed".to_string())
    }

    /// Closes the server-to-client WebSocket frame queue.
    fn close(&self) {
        let _ = self
            .sender
            .lock()
            .expect("Peer WebSocket sender lock poisoned")
            .take();
    }
}

/// Decodes one signed websocket envelope and encodes its response.
#[cfg(not(target_arch = "wasm32"))]
async fn handle_ws_binary(state: &RemoteLinkState, bytes: &[u8]) -> Vec<u8> {
    let response = match operit_link::decodeLink::<RemoteWsEnvelope>(bytes) {
        Ok(envelope) => handle_ws_envelope(state, envelope).await,
        Err(error) => RemoteWsResponse::Error(CoreLinkError::new("BAD_REQUEST", error.to_string())),
    };
    operit_link::encodeLink(&response).expect("RemoteWsResponse must serialize")
}

/// Verifies the raw websocket payload bytes and dispatches the decoded payload.
#[cfg(not(target_arch = "wasm32"))]
async fn handle_ws_envelope(
    state: &RemoteLinkState,
    envelope: RemoteWsEnvelope,
) -> RemoteWsResponse {
    if envelope.protocolVersion != 3 {
        return RemoteWsResponse::Error(CoreLinkError::new(
            "LINK_VERSION_MISMATCH",
            "Link protocol version 3 is required",
        ));
    }
    let payload = match operit_link::decodeLink::<RemoteWsPayload>(&envelope.payloadBytes) {
        Ok(value) => value,
        Err(error) => {
            return RemoteWsResponse::Error(CoreLinkError::new("BAD_REQUEST", error.to_string()))
        }
    };
    let verified = match verify_session_parts(
        state,
        &envelope.sessionId,
        &envelope.deviceId,
        &envelope.signature,
        &envelope.payloadBytes,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return RemoteWsResponse::Error(error),
    };
    match payload {
        RemoteWsPayload::SessionInfo(request) => {
            let spaceStore = CoreSpaceStore::new(state.accessStore.storage.clone());
            let space = match spaceStore.initialize() {
                Ok(value) => value,
                Err(error) => return RemoteWsResponse::Error(CoreLinkError::internal(error)),
            };
            let deviceProfiles = match spaceStore.deviceProfilesForCurrentSpace() {
                Ok(value) => value,
                Err(error) => return RemoteWsResponse::Error(CoreLinkError::internal(error)),
            };
            let sessions = state.sessions.lock().await;
            let Some(session) = sessions.get(&envelope.sessionId) else {
                return RemoteWsResponse::Error(remote_session_auth_error(
                    "invalid session",
                    "invalid_session",
                ));
            };
            RemoteWsResponse::SessionInfo(RemoteSessionInfoResponse {
                protocolVersion: 3,
                pairingServiceVersion: session.pairingServiceVersion,
                coreDeviceId: state.deviceId.clone(),
                coreDeviceInfo: state.deviceInfo.clone(),
                clientDeviceId: session.deviceId.clone(),
                clientDeviceInfo: session.deviceInfo.clone(),
                transports: vec!["http".to_string(), "ws".to_string()],
                deviceSpace: space,
                deviceProfiles,
                nonce: request.nonce,
            })
        }
        _ => RemoteWsResponse::Error(CoreLinkError::new(
            "WS_STREAM_MODE_REMOVED",
            "Legacy remote Link streams are removed",
        )),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn verify_session(
    state: &RemoteLinkState,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<VerifiedRemoteSession, Response> {
    if header_string(headers, "x-operit-link-version").as_deref() != Some("3") {
        return Err(encode_link_response(
            StatusCode::BAD_REQUEST,
            CoreLinkError::new(
                "LINK_VERSION_MISMATCH",
                "Link protocol version 3 is required",
            ),
        ));
    }
    let Some(sessionId) = header_string(headers, "x-operit-session") else {
        return Err(encode_link_response(
            StatusCode::UNAUTHORIZED,
            CoreLinkError::new("UNAUTHORIZED", "missing session"),
        ));
    };
    let Some(deviceId) = header_string(headers, "x-operit-device") else {
        return Err(encode_link_response(
            StatusCode::UNAUTHORIZED,
            CoreLinkError::new("UNAUTHORIZED", "missing device"),
        ));
    };
    let Some(signature) = header_string(headers, "x-operit-signature") else {
        return Err(encode_link_response(
            StatusCode::UNAUTHORIZED,
            CoreLinkError::new("UNAUTHORIZED", "missing signature"),
        ));
    };
    verify_session_parts(state, &sessionId, &deviceId, &signature, body)
        .await
        .map_err(|error| encode_link_response(StatusCode::UNAUTHORIZED, error))
}

#[cfg(not(target_arch = "wasm32"))]
async fn verify_session_parts(
    state: &RemoteLinkState,
    sessionId: &str,
    deviceId: &str,
    signature: &str,
    body: &[u8],
) -> Result<VerifiedRemoteSession, CoreLinkError> {
    let records = state
        .accessStore
        .inboundSessions()
        .map_err(CoreLinkError::internal)?;
    let Some(record) = records.get(sessionId) else {
        return Err(remote_session_auth_error(
            "invalid session",
            "invalid_session",
        ));
    };
    let session = accepted_session_from_record(record)?;
    if session.deviceId != deviceId {
        return Err(remote_session_auth_error(
            "device mismatch",
            "device_mismatch",
        ));
    }
    if sign(&session.sessionSecret, body) != signature {
        return Err(remote_session_auth_error(
            "signature mismatch",
            "signature_mismatch",
        ));
    }
    Ok(VerifiedRemoteSession {
        sessionId: sessionId.to_string(),
        deviceId: deviceId.to_string(),
    })
}

/// Creates a structured unauthorized error for a remote session auth failure.
#[cfg(not(target_arch = "wasm32"))]
fn remote_session_auth_error(message: &'static str, auth_reason: &'static str) -> CoreLinkError {
    CoreLinkError::withDetails(
        "UNAUTHORIZED",
        message,
        operit_link::CoreValue::Map(BTreeMap::from([
            (
                "type".to_string(),
                operit_link::CoreValue::String("remote_session_auth".to_string()),
            ),
            (
                "authReason".to_string(),
                operit_link::CoreValue::String(auth_reason.to_string()),
            ),
            (
                "resetWebAccessSession".to_string(),
                operit_link::CoreValue::Bool(true),
            ),
        ])),
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn accepted_session_from_record(
    record: &AcceptedRemoteSessionRecord,
) -> Result<RemoteSession, CoreLinkError> {
    Ok(RemoteSession {
        deviceId: record.deviceId.clone(),
        deviceInfo: record.deviceInfo.clone(),
        pairingServiceVersion: record.pairingServiceVersion,
        sessionSecret: BASE64
            .decode(record.sessionSecret.as_bytes())
            .map_err(|error| CoreLinkError::new("INVALID_SESSION_STORE", error.to_string()))?,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn token_matches(state: &RemoteLinkState, headers: &HeaderMap) -> bool {
    header_string(headers, "x-operit-link-token-hash")
        .map(|value| token_hash_matches(state, &value))
        .unwrap_or(false)
}

pub fn link_token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    BASE64.encode(hasher.finalize())
}

#[cfg(not(target_arch = "wasm32"))]
fn token_hash_matches(state: &RemoteLinkState, tokenHash: &str) -> bool {
    tokenHash == link_token_hash(&state.token)
}

#[cfg(not(target_arch = "wasm32"))]
fn header_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string)
}

fn parse_public_key(value: &str) -> Result<PublicKey, String> {
    let bytes = BASE64.decode(value).map_err(|error| error.to_string())?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "public key must be 32 bytes".to_string())?;
    Ok(PublicKey::from(bytes))
}

fn public_key_to_string(value: &PublicKey) -> String {
    BASE64.encode(value.as_bytes())
}

#[cfg(not(target_arch = "wasm32"))]
fn pairing_code() -> String {
    let bytes = Uuid::new_v4().as_u128();
    format!("{:06}", (bytes % 1_000_000) as u32)
}

fn link_access_token() -> String {
    let mut bytes = [0u8; 18];
    OsRng.fill_bytes(&mut bytes);
    format!("ow-{}", URL_SAFE_NO_PAD.encode(bytes))
}

/// Returns the host-owned Unix clock used by Link Access records.
fn unix_millis() -> i64 {
    currentTimeMillis()
}

fn proof(sharedSecret: &[u8], clientNonce: &str, serverNonce: &str, role: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sharedSecret);
    hasher.update(clientNonce.as_bytes());
    hasher.update(serverNonce.as_bytes());
    hasher.update(role.as_bytes());
    BASE64.encode(hasher.finalize())
}

fn session_secret(sharedSecret: &[u8], clientNonce: &str, serverNonce: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(sharedSecret);
    hasher.update(clientNonce.as_bytes());
    hasher.update(serverNonce.as_bytes());
    hasher.update(b"session");
    hasher.finalize().to_vec()
}

fn sign(sessionSecret: &[u8], body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(sessionSecret).expect("HMAC accepts any session secret length");
    mac.update(body);
    BASE64.encode(mac.finalize().into_bytes())
}

#[cfg(not(target_arch = "wasm32"))]
fn unauthorized(message: impl Into<String>) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(CoreLinkError::new("UNAUTHORIZED", message.into())),
    )
        .into_response()
}

#[cfg(not(target_arch = "wasm32"))]
fn bad_request(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(CoreLinkError::new("BAD_REQUEST", message.into())),
    )
        .into_response()
}
#[cfg(not(target_arch = "wasm32"))]
fn internal_server_error(message: impl Into<String>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(CoreLinkError::new("INTERNAL_SERVER_ERROR", message.into())),
    )
        .into_response()
}

/// Encodes a typed Link response as MessagePack bytes.
#[cfg(not(target_arch = "wasm32"))]
fn encode_link_response(status: StatusCode, value: impl Serialize) -> Response {
    match operit_link::encodeLink(value) {
        Ok(bytes) => Response::builder()
            .status(status)
            .header("content-type", "application/msgpack")
            .body(Body::from(bytes))
            .expect("Link response must build"),
        Err(error) => internal_server_error(error.to_string()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn serve_web_access_file(webAccess: &RemoteWebAccessState, path: &str) -> Response {
    let relativePath = match sanitize_web_asset_path(path) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let fullPath = webAccess.webRoot.join(&relativePath);
    if !fullPath.starts_with(&webAccess.webRoot) {
        return bad_request("web asset path escapes web root");
    }
    let bytes = match (webAccess.readAsset)(&fullPath) {
        Ok(value) => value,
        Err(error) => {
            return (
                StatusCode::NOT_FOUND,
                Json(CoreLinkError::new("NOT_FOUND", error.to_string())),
            )
                .into_response();
        }
    };
    let contentType = content_type_for_path(&fullPath);
    Response::builder()
        .header("content-type", contentType)
        .header("cross-origin-opener-policy", "same-origin")
        .header("cross-origin-embedder-policy", "require-corp")
        .header("cross-origin-resource-policy", "same-origin")
        .body(Body::from(bytes))
        .expect("web asset response must build")
}

/// Reads and returns one asset from the standalone Web Access root.
#[cfg(not(target_arch = "wasm32"))]
fn serve_static_web_access_file(webAccess: &StaticWebAccessState, path: &str) -> Response {
    let relativePath = match sanitize_web_asset_path(path) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let fullPath = webAccess.webRoot.join(&relativePath);
    if !fullPath.starts_with(&webAccess.webRoot) {
        return bad_request("web asset path escapes web root");
    }
    let bytes = match (webAccess.readAsset)(&fullPath) {
        Ok(value) => value,
        Err(error) => {
            return (
                StatusCode::NOT_FOUND,
                Json(CoreLinkError::new("NOT_FOUND", error.to_string())),
            )
                .into_response();
        }
    };
    let contentType = content_type_for_path(&fullPath);
    Response::builder()
        .header("content-type", contentType)
        .header("cross-origin-opener-policy", "same-origin")
        .header("cross-origin-embedder-policy", "require-corp")
        .header("cross-origin-resource-policy", "same-origin")
        .body(Body::from(bytes))
        .expect("web asset response must build")
}

#[cfg(not(target_arch = "wasm32"))]
fn sanitize_web_asset_path(path: &str) -> Result<PathBuf, Response> {
    let normalized = path.trim_start_matches('/');
    if normalized.is_empty() {
        return Ok(PathBuf::from("index.html"));
    }
    let relative = PathBuf::from(normalized);
    if relative
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(bad_request("invalid web asset path"));
    }
    Ok(relative)
}

#[cfg(not(target_arch = "wasm32"))]
fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}
