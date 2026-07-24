use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::net::SocketAddr;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::sync::Mutex as StdMutex;
use std::sync::Arc;

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
use futures_util::StreamExt;
use hmac::{Hmac, Mac};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpListener;
use tokio::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::oneshot;
use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};

#[cfg(not(target_arch = "wasm32"))]
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostManager::defaultHttpHost;
#[cfg(not(target_arch = "wasm32"))]
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_host_api::{HttpRequestData, RuntimeStorageHost, TimeUtils::currentTimeMillis};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError, CorePushItem,
    CorePushRequest, CoreWatchRequest,
};
use operit_link::CoreLinkClient;
#[cfg(not(target_arch = "wasm32"))]
use operit_link::CoreLinkTransportClient;
use operit_store::PreferencesDataStore::{
    emptyPreferences, stringPreferencesKey, Preferences, PreferencesDataStore,
};
#[cfg(not(target_arch = "wasm32"))]
use operit_runtime::services::RuntimeHostInteractionService::{
    withRuntimeHostInteractionOrigin, RuntimeHostInteractionRequestOrigin,
};

#[cfg(test)]
mod tests;

type HmacSha256 = Hmac<Sha256>;
pub const REMOTE_PAIRING_SERVICE_VERSION: i32 = 1;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct RemoteLinkServerConfig {
    pub bindAddress: String,
    pub token: String,
    pub localControlToken: Option<String>,
    pub deviceId: String,
    pub deviceInfo: RemoteDeviceInfo,
    pub webAccess: Option<RemoteWebAccessConfig>,
    pub printStartupInfo: bool,
    pub accessStore: LinkAccessStore,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct RemoteLinkServer;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemotePairingCodeRecord {
    pub pairingId: String,
    pub pairingServiceVersion: i32,
    pub clientDeviceId: String,
    pub clientDeviceInfo: RemoteDeviceInfo,
    pub pairingCode: String,
    pub createdAt: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AcceptedRemoteSessionRecord {
    pub deviceId: String,
    pub deviceInfo: RemoteDeviceInfo,
    pub pairingServiceVersion: i32,
    pub sessionSecret: String,
}

pub const LINK_ACCESS_IDENTITY_PATH: &str = "runtime/link_access/identity.preferences.json";
pub const LINK_ACCESS_INBOUND_SESSIONS_PATH: &str =
    "runtime/link_access/inbound_sessions.preferences.json";
pub const LINK_ACCESS_OUTBOUND_SESSIONS_PATH: &str =
    "runtime/link_access/outbound_sessions.preferences.json";
pub const LINK_ACCESS_PENDING_PAIRINGS_PATH: &str =
    "runtime/link_access/pending_pairings.preferences.json";
pub const LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH: &str =
    "runtime/link_access/pending_outbound_pairings.preferences.json";
pub const LINK_ACCESS_HOST_CONFIG_PATH: &str = "runtime/link_access/host_config.preferences.json";
pub const LINK_ACCESS_AUTO_SYNC_PATH: &str = "runtime/link_access/auto_sync.preferences.json";
pub const LINK_ACCESS_ROUTING_PATH: &str = "runtime/link_access/routing.preferences.json";

const LINK_ACCESS_RECORD_KEY: &str = "record";
const LINK_ACCESS_BIND_ADDRESS_KEY: &str = "bindAddress";
const LINK_ACCESS_TOKEN_KEY: &str = "token";
const LINK_ACCESS_WEB_ACCESS_ENABLED_KEY: &str = "webAccessEnabled";
const LINK_ACCESS_DISCOVERY_ENABLED_KEY: &str = "discoveryEnabled";
const LINK_ACCESS_PORT_MODE_KEY: &str = "portMode";
const LINK_ACCESS_AUTO_SYNC_REMOTE_NAMES_KEY: &str = "autoSyncRemoteNames";
const LINK_ACCESS_ROUTE_TYPE_KEY: &str = "routeType";
const LINK_ACCESS_REMOTE_SESSION_NAME_KEY: &str = "remoteSessionName";
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkAccessAutoSyncConfig {
    pub autoSyncRemoteNames: Vec<String>,
    pub updatedAt: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LinkAccessRoute {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "remote")]
    Remote { sessionName: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkAccessRoutingConfig {
    pub route: LinkAccessRoute,
    pub updatedAt: i64,
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
        let store = self.dataStore(LINK_ACCESS_IDENTITY_PATH);
        let preferences = self.readPreferences(&store)?;
        if !preferences.entries().is_empty() {
            return readPreferenceRecord(&preferences, LINK_ACCESS_RECORD_KEY, LINK_ACCESS_IDENTITY_PATH);
        }
        let identity = LinkAccessIdentity {
            deviceId: format!("core-{}", Uuid::new_v4()),
            deviceInfo,
        };
        writeSingleRecord(&store, &identity)?;
        Ok(identity)
    }

    /// Returns every accepted inbound session owned by this runtime.
    pub fn inboundSessions(&self) -> Result<BTreeMap<String, AcceptedRemoteSessionRecord>, String> {
        self.readRecordMap(LINK_ACCESS_INBOUND_SESSIONS_PATH)
    }

    /// Persists one accepted inbound session owned by this runtime.
    pub fn saveInboundSession(
        &self,
        sessionId: String,
        record: AcceptedRemoteSessionRecord,
    ) -> Result<(), String> {
        self.writeMapRecord(LINK_ACCESS_INBOUND_SESSIONS_PATH, &sessionId, &record)
    }

    /// Removes one accepted inbound session owned by this runtime.
    pub fn removeInboundSession(&self, sessionId: &str) -> Result<(), String> {
        self.removeMapRecord(LINK_ACCESS_INBOUND_SESSIONS_PATH, sessionId)
    }

    /// Returns every named outbound session owned by this runtime.
    pub fn outboundSessions(&self) -> Result<BTreeMap<String, PairedRemoteSessionRecord>, String> {
        self.readRecordMap(LINK_ACCESS_OUTBOUND_SESSIONS_PATH)
    }

    /// Persists one named outbound session owned by this runtime.
    pub fn saveOutboundSession(
        &self,
        name: String,
        record: PairedRemoteSessionRecord,
    ) -> Result<(), String> {
        self.writeMapRecord(LINK_ACCESS_OUTBOUND_SESSIONS_PATH, &name, &record)
    }

    /// Removes one named outbound session owned by this runtime.
    pub fn removeOutboundSession(&self, name: &str) -> Result<(), String> {
        self.removeMapRecord(LINK_ACCESS_OUTBOUND_SESSIONS_PATH, name)
    }

    /// Returns every pending pairing owned by this runtime.
    pub fn pendingPairings(&self) -> Result<BTreeMap<String, RemotePairingCodeRecord>, String> {
        self.readRecordMap(LINK_ACCESS_PENDING_PAIRINGS_PATH)
    }

    /// Persists one pending pairing owned by this runtime.
    pub fn savePendingPairing(&self, record: RemotePairingCodeRecord) -> Result<(), String> {
        self.writeMapRecord(
            LINK_ACCESS_PENDING_PAIRINGS_PATH,
            &record.pairingId.clone(),
            &record,
        )
    }

    /// Removes one pending pairing owned by this runtime.
    pub fn removePendingPairing(&self, pairingId: &str) -> Result<(), String> {
        self.removeMapRecord(LINK_ACCESS_PENDING_PAIRINGS_PATH, pairingId)
    }

    /// Returns every pending outbound pairing initiated by this runtime.
    #[allow(non_snake_case)]
    pub fn pendingOutboundPairings(
        &self,
    ) -> Result<BTreeMap<String, PendingOutboundPairingRecord>, String> {
        self.readRecordMap(LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH)
    }

    /// Persists one pending outbound pairing initiated by this runtime.
    #[allow(non_snake_case)]
    pub fn savePendingOutboundPairing(
        &self,
        pairingId: String,
        record: PendingOutboundPairingRecord,
    ) -> Result<(), String> {
        self.writeMapRecord(LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH, &pairingId, &record)
    }

    /// Removes one pending outbound pairing after it has completed or been cancelled.
    #[allow(non_snake_case)]
    pub fn removePendingOutboundPairing(&self, pairingId: &str) -> Result<(), String> {
        self.removeMapRecord(LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH, pairingId)
    }

    /// Persists the active Link Access host configuration for this runtime.
    pub fn saveHostConfig(&self, config: LinkAccessHostConfig) -> Result<(), String> {
        writeHostConfigPreferences(&self.dataStore(LINK_ACCESS_HOST_CONFIG_PATH), &config)
    }

    /// Initializes and returns the active Link Access host configuration.
    pub fn initializeHostConfig(&self) -> Result<LinkAccessHostConfig, String> {
        let store = self.dataStore(LINK_ACCESS_HOST_CONFIG_PATH);
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
        hostConfigFromPreferences(&self.readPreferences(&self.dataStore(LINK_ACCESS_HOST_CONFIG_PATH))?)
    }

    /// Initializes and returns this runtime's Link auto-sync configuration.
    #[allow(non_snake_case)]
    pub fn initializeAutoSyncConfig(&self) -> Result<LinkAccessAutoSyncConfig, String> {
        let store = self.dataStore(LINK_ACCESS_AUTO_SYNC_PATH);
        let preferences = self.readPreferences(&store)?;
        if !preferences.entries().is_empty() {
            return autoSyncConfigFromPreferences(&preferences);
        }
        let config = LinkAccessAutoSyncConfig {
            autoSyncRemoteNames: Vec::new(),
            updatedAt: currentTimeMillis(),
        };
        writeAutoSyncConfigPreferences(&store, &config)?;
        Ok(config)
    }

    /// Reads this runtime's Link auto-sync configuration.
    #[allow(non_snake_case)]
    pub fn autoSyncConfig(&self) -> Result<LinkAccessAutoSyncConfig, String> {
        autoSyncConfigFromPreferences(&self.readPreferences(&self.dataStore(LINK_ACCESS_AUTO_SYNC_PATH))?)
    }

    /// Persists this runtime's Link auto-sync configuration.
    #[allow(non_snake_case)]
    pub fn saveAutoSyncConfig(
        &self,
        config: LinkAccessAutoSyncConfig,
    ) -> Result<(), String> {
        writeAutoSyncConfigPreferences(&self.dataStore(LINK_ACCESS_AUTO_SYNC_PATH), &config)
    }

    /// Initializes and returns this runtime's Link request routing configuration.
    #[allow(non_snake_case)]
    pub fn initializeRoutingConfig(&self) -> Result<LinkAccessRoutingConfig, String> {
        let store = self.dataStore(LINK_ACCESS_ROUTING_PATH);
        let preferences = self.readPreferences(&store)?;
        if !preferences.entries().is_empty() {
            return routingConfigFromPreferences(&preferences);
        }
        let config = LinkAccessRoutingConfig {
            route: LinkAccessRoute::Local,
            updatedAt: currentTimeMillis(),
        };
        writeRoutingConfigPreferences(&store, &config)?;
        Ok(config)
    }

    /// Reads this runtime's Link request routing configuration.
    #[allow(non_snake_case)]
    pub fn routingConfig(&self) -> Result<LinkAccessRoutingConfig, String> {
        routingConfigFromPreferences(&self.readPreferences(&self.dataStore(LINK_ACCESS_ROUTING_PATH))?)
    }

    /// Persists this runtime's Link request routing configuration.
    #[allow(non_snake_case)]
    pub fn saveRoutingConfig(&self, config: LinkAccessRoutingConfig) -> Result<(), String> {
        validateRoutingConfig(&config)?;
        writeRoutingConfigPreferences(&self.dataStore(LINK_ACCESS_ROUTING_PATH), &config)
    }

    /// Creates one local datastore for a Link Access preferences path.
    fn dataStore(&self, path: &str) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(self.storage.clone(), path)
    }

    /// Reads one Link Access preferences snapshot.
    fn readPreferences(&self, store: &PreferencesDataStore) -> Result<Preferences, String> {
        store.data().map_err(|error| error.to_string())
    }

    /// Reads every keyed record from a Link Access datastore.
    fn readRecordMap<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<BTreeMap<String, T>, String> {
        let preferences = self.readPreferences(&self.dataStore(path))?;
        let mut records = BTreeMap::new();
        for (name, encoded) in preferences.entries() {
            records.insert(
                name,
                serde_json::from_str(&encoded).map_err(|error| error.to_string())?,
            );
        }
        Ok(records)
    }

    /// Writes one keyed record into a Link Access datastore.
    fn writeMapRecord<T: Serialize>(&self, path: &str, name: &str, value: &T) -> Result<(), String> {
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

/// Writes one single-record datastore snapshot.
fn writeSingleRecord<T: Serialize>(store: &PreferencesDataStore, value: &T) -> Result<(), String> {
    let mut preferences = emptyPreferences();
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_RECORD_KEY),
        serde_json::to_string(value).map_err(|error| error.to_string())?,
    );
    store
        .replace(preferences)
        .map_err(|error| error.to_string())
}

/// Reads one JSON-encoded preference value by key.
fn readPreferenceRecord<T: serde::de::DeserializeOwned>(
    preferences: &Preferences,
    key: &str,
    path: &str,
) -> Result<T, String> {
    let encoded = requiredPreference(preferences, key, path)?;
    serde_json::from_str(&encoded).map_err(|error| error.to_string())
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
fn requiredI64Preference(
    preferences: &Preferences,
    key: &str,
    path: &str,
) -> Result<i64, String> {
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
            LINK_ACCESS_HOST_CONFIG_PATH,
        )?,
        token: requiredPreference(preferences, LINK_ACCESS_TOKEN_KEY, LINK_ACCESS_HOST_CONFIG_PATH)?,
        webAccessEnabled: requiredBoolPreference(
            preferences,
            LINK_ACCESS_WEB_ACCESS_ENABLED_KEY,
            LINK_ACCESS_HOST_CONFIG_PATH,
        )?,
        discoveryEnabled: requiredBoolPreference(
            preferences,
            LINK_ACCESS_DISCOVERY_ENABLED_KEY,
            LINK_ACCESS_HOST_CONFIG_PATH,
        )?,
        portMode: hostPortModeFromPreference(&requiredPreference(
            preferences,
            LINK_ACCESS_PORT_MODE_KEY,
            LINK_ACCESS_HOST_CONFIG_PATH,
        )?)?,
        updatedAt: requiredI64Preference(
            preferences,
            LINK_ACCESS_UPDATED_AT_KEY,
            LINK_ACCESS_HOST_CONFIG_PATH,
        )?,
    })
}

/// Converts persisted auto-sync preferences into the typed model.
fn autoSyncConfigFromPreferences(
    preferences: &Preferences,
) -> Result<LinkAccessAutoSyncConfig, String> {
    Ok(LinkAccessAutoSyncConfig {
        autoSyncRemoteNames: serde_json::from_str(&requiredPreference(
            preferences,
            LINK_ACCESS_AUTO_SYNC_REMOTE_NAMES_KEY,
            LINK_ACCESS_AUTO_SYNC_PATH,
        )?)
        .map_err(|error| error.to_string())?,
        updatedAt: requiredI64Preference(
            preferences,
            LINK_ACCESS_UPDATED_AT_KEY,
            LINK_ACCESS_AUTO_SYNC_PATH,
        )?,
    })
}

/// Converts persisted Link routing preferences into the typed model.
fn routingConfigFromPreferences(
    preferences: &Preferences,
) -> Result<LinkAccessRoutingConfig, String> {
    let route = match requiredPreference(
        preferences,
        LINK_ACCESS_ROUTE_TYPE_KEY,
        LINK_ACCESS_ROUTING_PATH,
    )?
    .as_str()
    {
        "local" => LinkAccessRoute::Local,
        "remote" => LinkAccessRoute::Remote {
            sessionName: requiredPreference(
                preferences,
                LINK_ACCESS_REMOTE_SESSION_NAME_KEY,
                LINK_ACCESS_ROUTING_PATH,
            )?,
        },
        value => return Err(format!("invalid Link Access route type: {value}")),
    };
    let config = LinkAccessRoutingConfig {
        route,
        updatedAt: requiredI64Preference(
            preferences,
            LINK_ACCESS_UPDATED_AT_KEY,
            LINK_ACCESS_ROUTING_PATH,
        )?,
    };
    validateRoutingConfig(&config)?;
    Ok(config)
}

/// Persists one host config through the local datastore API.
fn writeHostConfigPreferences(
    store: &PreferencesDataStore,
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

/// Persists one auto-sync config through the local datastore API.
fn writeAutoSyncConfigPreferences(
    store: &PreferencesDataStore,
    config: &LinkAccessAutoSyncConfig,
) -> Result<(), String> {
    let mut preferences = emptyPreferences();
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_AUTO_SYNC_REMOTE_NAMES_KEY),
        serde_json::to_string(&config.autoSyncRemoteNames).map_err(|error| error.to_string())?,
    );
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_UPDATED_AT_KEY),
        config.updatedAt.to_string(),
    );
    store
        .replace(preferences)
        .map_err(|error| error.to_string())
}

/// Persists one Link routing configuration through the local datastore API.
fn writeRoutingConfigPreferences(
    store: &PreferencesDataStore,
    config: &LinkAccessRoutingConfig,
) -> Result<(), String> {
    let mut preferences = emptyPreferences();
    let (routeType, remoteSessionName) = match &config.route {
        LinkAccessRoute::Local => ("local", String::new()),
        LinkAccessRoute::Remote { sessionName } => ("remote", sessionName.clone()),
    };
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_ROUTE_TYPE_KEY),
        routeType.to_string(),
    );
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_REMOTE_SESSION_NAME_KEY),
        remoteSessionName,
    );
    preferences.set(
        &stringPreferencesKey(LINK_ACCESS_UPDATED_AT_KEY),
        config.updatedAt.to_string(),
    );
    store
        .replace(preferences)
        .map_err(|error| error.to_string())
}

/// Validates that a persisted Link route identifies a concrete remote session.
#[allow(non_snake_case)]
fn validateRoutingConfig(config: &LinkAccessRoutingConfig) -> Result<(), String> {
    if let LinkAccessRoute::Remote { sessionName } = &config.route {
        if sessionName.trim().is_empty() {
            return Err("remote Link route requires a paired session name".to_string());
        }
    }
    Ok(())
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
#[derive(Clone)]
struct RemoteLinkState {
    core: Arc<Mutex<SharedAccessCoreClient>>,
    linkDispatcher: operit_link::CoreLinkHttpDispatcher,
    token: String,
    localControlToken: Option<String>,
    keySecret: Arc<StaticSecret>,
    keyPublic: String,
    deviceId: String,
    deviceInfo: RemoteDeviceInfo,
    pairings: Arc<Mutex<BTreeMap<String, PendingPairing>>>,
    sessions: Arc<Mutex<BTreeMap<String, RemoteSession>>>,
    accessStore: LinkAccessStore,
    webAccess: Option<RemoteWebAccessState>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct SharedAccessCoreClient {
    core: Arc<Mutex<Box<dyn CoreLinkClient + Send>>>,
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl CoreLinkTransportClient for SharedAccessCoreClient {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        if let Err(error) = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "link-access-call",
            Box::new(move || {
                Box::pin(async move {
                    let response = core.lock().await.call(request).await;
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

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "link-access-watch-snapshot",
                Box::new(move || {
                    Box::pin(async move {
                        let response = core.lock().await.watchSnapshot(request).await;
                        let _ = sender.send(response);
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let (sender, receiver) = oneshot::channel();
        let core = self.core.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "link-access-watch",
                Box::new(move || {
                    Box::pin(async move {
                        let response = core.lock().await.watch(request).await;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HelloResponse {
    pub protocolVersion: i32,
    pub pairingServiceVersion: i32,
    pub coreDeviceId: String,
    pub coreDeviceInfo: RemoteDeviceInfo,
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
pub struct RemoteCallEnvelope {
    pub request: CoreCallRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteWatchEnvelope {
    pub request: CoreWatchRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteWatchChannelEnvelope {
    pub channelId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteWatchChannelOpenEnvelope {
    pub channelId: String,
    pub subscriptionId: String,
    pub request: CoreWatchRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteWatchChannelCloseEnvelope {
    pub channelId: String,
    pub subscriptionId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteWatchChannelOpenResponse {
    pub subscriptionId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteWatchChannelEvent {
    pub subscriptionId: String,
    pub event: CoreEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteSessionInfoEnvelope {
    pub nonce: String,
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
    pub nonce: String,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteWsEnvelope {
    pub protocolVersion: i32,
    pub sessionId: String,
    pub deviceId: String,
    pub signature: String,
    #[serde(with = "serde_bytes")]
    pub payloadBytes: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemotePushAccepted {
    pub pushId: String,
    pub sequence: u64,
}

#[cfg(not(target_arch = "wasm32"))]
struct RemotePushState {
    request: CorePushRequest,
    nextSequence: u64,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum RemoteWsPayload {
    SessionInfo(RemoteSessionInfoEnvelope),
    Call(RemoteCallEnvelope),
    WatchSnapshot(RemoteWatchEnvelope),
    PushOpen(CorePushRequest),
    PushItem(CorePushItem),
    PushClose(String),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum RemoteWsResponse {
    SessionInfo(RemoteSessionInfoResponse),
    Call(CoreCallResponse),
    WatchSnapshot(CoreEvent),
    PushOpened(String),
    PushAccepted(RemotePushAccepted),
    PushClosed(String),
    Error(CoreLinkError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairedRemoteSessionRecord {
    pub baseUrl: String,
    pub sessionId: String,
    pub deviceId: String,
    pub coreDeviceId: String,
    pub remoteDeviceInfo: RemoteDeviceInfo,
    pub pairingServiceVersion: i32,
    pub sessionSecret: String,
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
    pub async fn serve(
        core: impl CoreLinkClient + Send + 'static,
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

    #[allow(non_snake_case)]
    pub async fn serveWithListener(
        core: impl CoreLinkClient + Send + 'static,
        config: RemoteLinkServerConfig,
        listener: TcpListener,
        address: SocketAddr,
    ) -> Result<(), String> {
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
        let core = Arc::new(Mutex::new(Box::new(core) as Box<dyn CoreLinkClient + Send>));
        let transportCore = SharedAccessCoreClient { core: core.clone() };
        let linkDispatcher = operit_link::CoreLinkHttpDispatcher::new(transportCore.clone());
        let state = RemoteLinkState {
            core: Arc::new(Mutex::new(transportCore)),
            linkDispatcher,
            token: config.token.clone(),
            localControlToken: config.localControlToken.clone(),
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
            .route("/link/call", post(call))
            .route("/link/watch/snapshot", post(watch_snapshot))
            .route("/link/watch/channel/events", post(watch_channel_events))
            .route("/link/watch/channel/open", post(watch_channel_open))
            .route("/link/watch/channel/close", post(watch_channel_close))
            .route("/link/push/open", post(push_open))
            .route("/link/push/item", post(push_item))
            .route("/link/push/close", post(push_close))
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
        clientDeviceInfo: RemoteDeviceInfo,
    ) -> Result<PairStartState, String> {
        let clientSecret = StaticSecret::random_from_rng(OsRng);
        let clientPublic = PublicKey::from(&clientSecret);
        let clientDeviceId = format!("client-{}", Uuid::new_v4());
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
            sessionSecret: session_secret(
                &state.sharedSecret,
                &state.clientNonce,
                &state.serverNonce,
            ),
            watchChannel: Arc::new(StdMutex::new(None)),
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
    let response = defaultHttpHost()
        .executeHttpRequest(HttpRequestData {
            url: url.clone(),
            method: method.to_string(),
            headers,
            body,
            formFields: Vec::new(),
            fileParts: Vec::new(),
            connectTimeoutSeconds: 10,
            readTimeoutSeconds: 120,
            followRedirects: false,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        })
        .map_err(|error| error.to_string())?;
    if !(200..300).contains(&response.statusCode) {
        return Err(format!(
            "Link HTTP {method} {url} failed with status {}: {}",
            response.statusCode,
            String::from_utf8_lossy(&response.body)
        ));
    }
    Ok(response.body)
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
    sessionSecret: Vec<u8>,
    watchChannel: Arc<StdMutex<Option<PairedRemoteWatchChannel>>>,
}

struct PairedRemoteWatchChannel {
    channelId: String,
    streamId: String,
    subscriptions: BTreeMap<String, tokio::sync::mpsc::UnboundedSender<CoreEvent>>,
    buffer: Vec<u8>,
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
            sessionSecret: BASE64
                .decode(record.sessionSecret)
                .map_err(|error| error.to_string())?,
            watchChannel: Arc::new(StdMutex::new(None)),
        })
    }

    #[allow(non_snake_case)]
    pub async fn sessionInfo(&self) -> Result<RemoteSessionInfoResponse, String> {
        let body = operit_link::encodeLink(&RemoteSessionInfoEnvelope {
            nonce: Uuid::new_v4().to_string(),
        })
        .map_err(|error| error.to_string())?;
        operit_link::decodeLink(&self.signedRemotePost("session", body)?)
            .map_err(|error| error.to_string())
    }

    pub async fn call(&self, request: CoreCallRequest) -> Result<CoreCallResponse, String> {
        let body = operit_link::encodeLink(&RemoteCallEnvelope { request })
            .map_err(|error| error.to_string())?;
        operit_link::decodeLink(&self.signedRemotePost("call", body)?)
            .map_err(|error| error.to_string())
    }

    /// Opens one HTTP-carried Link push stream.
    pub async fn pushOpen(&self, request: CorePushRequest) -> Result<String, String> {
        let pushId = request.requestId.0.clone();
        let body = operit_link::encodeLink(operit_link::LinkPushOpenEnvelope {
            pushId: pushId.clone(),
            request,
        })
        .map_err(|error| error.to_string())?;
        let response = self.signedPushPost("open", body).await?;
        let opened = operit_link::decodeLink::<operit_link::LinkPushOpenResponse>(&response)
            .map_err(|error| error.to_string())?;
        Ok(opened.pushId)
    }

    /// Sends one ordered item through the HTTP push carrier.
    pub async fn pushItem(&self, item: CorePushItem) -> Result<(), String> {
        let body = operit_link::encodeLink(item).map_err(|error| error.to_string())?;
        self.signedPushPost("item", body).await?;
        Ok(())
    }

    /// Closes one HTTP-carried Link push stream.
    pub async fn pushClose(&self, pushId: String) -> Result<(), String> {
        let body = operit_link::encodeLink(operit_link::LinkPushCloseEnvelope { pushId })
            .map_err(|error| error.to_string())?;
        self.signedPushPost("close", body).await?;
        Ok(())
    }

    /// Posts one signed push lifecycle message and returns its bytes.
    async fn signedPushPost(&self, operation: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
        self.signedRemotePost(&format!("push/{operation}"), body)
    }

    pub async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, String> {
        let body = operit_link::encodeLink(&RemoteWatchEnvelope { request })
            .map_err(|error| error.to_string())?;
        operit_link::decodeLink(&self.signedRemotePost("watch/snapshot", body)?)
            .map_err(|error| error.to_string())
    }

    /// Posts one authenticated Link protocol frame through the runtime HTTP host.
    fn signedRemotePost(&self, path: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
        remoteHttpRequest(
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
        )
    }

    /// Opens one authenticated remote watch through the configured streaming HTTP Host.
    pub async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, String> {
        let channelId = self.ensureWatchChannel().await?;
        let subscriptionId = format!("watch-{}", Uuid::new_v4().simple());
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        {
            let mut guard = self
                .watchChannel
                .lock()
                .map_err(|error| format!("paired watch channel lock poisoned: {error}"))?;
            let channel = guard
                .as_mut()
                .ok_or_else(|| "paired watch channel is not open".to_string())?;
            if channel.channelId != channelId {
                return Err("paired watch channel changed while opening subscription".to_string());
            }
            channel
                .subscriptions
                .insert(subscriptionId.clone(), sender);
        }
        let body = operit_link::encodeLink(&RemoteWatchChannelOpenEnvelope {
            channelId: channelId.clone(),
            subscriptionId: subscriptionId.clone(),
            request,
        })
        .map_err(|error| error.to_string())?;
        let openResult = self.signedRemotePost("watch/channel/open", body).and_then(|bytes| {
            operit_link::decodeLink::<RemoteWatchChannelOpenResponse>(&bytes)
                .map_err(|error| error.to_string())
        });
        if let Err(error) = openResult {
            close_paired_watch_subscription(
                &self.watchChannel,
                &channelId,
                &subscriptionId,
            )?;
            return Err(error);
        }
        let closeSession = self.clone();
        let watchChannel = self.watchChannel.clone();
        Ok(CoreEventStream::new(receiver).withOnClose(move || {
            let body = operit_link::encodeLink(&RemoteWatchChannelCloseEnvelope {
                channelId: channelId.clone(),
                subscriptionId: subscriptionId.clone(),
            })
            .expect("watch close envelope must encode");
            let _ = closeSession.signedRemotePost("watch/channel/close", body);
            close_paired_watch_subscription(
                &watchChannel,
                &channelId,
                &subscriptionId,
            )
            .expect("paired watch subscription must close");
        }))
    }

    /// Opens and authenticates the Host-owned HTTP byte stream for remote watch events.
    #[allow(non_snake_case)]
    async fn ensureWatchChannel(&self) -> Result<String, String> {
        if let Some(channelId) = self
            .watchChannel
            .lock()
            .map_err(|error| format!("paired watch channel lock poisoned: {error}"))?
            .as_ref()
            .map(|channel| channel.channelId.clone())
        {
            return Ok(channelId);
        }
        let channelId = format!("watch-channel-{}", Uuid::new_v4().simple());
        let streamId = format!("link-watch-http-{}", Uuid::new_v4().simple());
        let body = operit_link::encodeLink(&RemoteWatchChannelEnvelope {
            channelId: channelId.clone(),
        })
        .map_err(|error| error.to_string())?;
        let signature = sign(&self.sessionSecret, &body);
        let (openedSender, openedReceiver) = tokio::sync::oneshot::channel();
        let openedSender = Arc::new(StdMutex::new(Some(openedSender)));
        {
            let mut guard = self
                .watchChannel
                .lock()
                .map_err(|error| format!("paired watch channel lock poisoned: {error}"))?;
            *guard = Some(PairedRemoteWatchChannel {
                channelId: channelId.clone(),
                streamId: streamId.clone(),
                subscriptions: BTreeMap::new(),
                buffer: Vec::new(),
            });
        }
        let openedSignal = openedSender.clone();
        let chunkChannel = self.watchChannel.clone();
        let chunkChannelId = channelId.clone();
        let closedSignal = openedSender.clone();
        let closedChannel = self.watchChannel.clone();
        let closedChannelId = channelId.clone();
        let openResult = defaultHttpHost().openHttpByteStream(
            streamId.clone(),
            HttpRequestData {
                url: format!("{}/link/watch/channel/events", self.baseUrl),
                method: "POST".to_string(),
                headers: vec![
                    ("x-operit-link-version".to_string(), "3".to_string()),
                    ("x-operit-session".to_string(), self.sessionId.clone()),
                    ("x-operit-device".to_string(), self.deviceId.clone()),
                    ("x-operit-signature".to_string(), signature),
                ],
                body,
                formFields: Vec::new(),
                fileParts: Vec::new(),
                connectTimeoutSeconds: 10,
                readTimeoutSeconds: 0,
                followRedirects: false,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            },
            Arc::new(move || {
                if let Some(sender) = openedSignal
                    .lock()
                    .expect("paired watch open signal lock poisoned")
                    .take()
                {
                    let _ = sender.send(Ok(()));
                }
            }),
            Arc::new(move |chunk| {
                dispatch_paired_watch_chunk(&chunkChannel, &chunkChannelId, chunk)
                    .expect("paired watch chunk must decode");
            }),
            Arc::new(move |result| {
                if let Some(sender) = closedSignal
                    .lock()
                    .expect("paired watch close signal lock poisoned")
                    .take()
                {
                    let _ = sender.send(result.clone());
                }
                let mut guard = closedChannel
                    .lock()
                    .expect("paired watch channel lock poisoned");
                if guard.as_ref().map(|channel| channel.channelId.as_str())
                    == Some(closedChannelId.as_str())
                {
                    let _ = guard.take();
                }
            }),
        );
        if let Err(error) = openResult {
            let mut guard = self
                .watchChannel
                .lock()
                .map_err(|lockError| format!("paired watch channel lock poisoned: {lockError}"))?;
            if guard.as_ref().map(|channel| channel.channelId.as_str())
                == Some(channelId.as_str())
            {
                let _ = guard.take();
            }
            return Err(error.to_string());
        }
        openedReceiver
            .await
            .map_err(|error| format!("paired watch open signal closed: {error}"))??;
        Ok(channelId)
    }
}

/// Decodes complete length-prefixed remote watch frames from one Host HTTP chunk.
fn dispatch_paired_watch_chunk(
    watchChannel: &Arc<StdMutex<Option<PairedRemoteWatchChannel>>>,
    channelId: &str,
    chunk: Vec<u8>,
) -> Result<(), String> {
    let mut guard = watchChannel
        .lock()
        .map_err(|error| format!("paired watch channel lock poisoned: {error}"))?;
    let channel = guard
        .as_mut()
        .ok_or_else(|| "paired watch channel is not open".to_string())?;
    if channel.channelId != channelId {
        return Err("paired watch chunk targets a stale channel".to_string());
    }
    channel.buffer.extend_from_slice(&chunk);
    while channel.buffer.len() >= 4 {
        let frameLength = u32::from_be_bytes(
            channel.buffer[..4]
                .try_into()
                .expect("watch frame length prefix must be four bytes"),
        ) as usize;
        if channel.buffer.len() < 4 + frameLength {
            break;
        }
        let frame = channel.buffer.drain(..4 + frameLength).collect::<Vec<_>>();
        let event = operit_link::decodeLink::<RemoteWatchChannelEvent>(&frame[4..])
            .map_err(|error| error.to_string())?;
        if let Some(sender) = channel.subscriptions.get(&event.subscriptionId) {
            let _ = sender.send(event.event);
        }
    }
    Ok(())
}

/// Removes one watch subscription and closes its Host byte stream when it becomes empty.
fn close_paired_watch_subscription(
    watchChannel: &Arc<StdMutex<Option<PairedRemoteWatchChannel>>>,
    channelId: &str,
    subscriptionId: &str,
) -> Result<(), String> {
    let streamId = {
        let mut guard = watchChannel
            .lock()
            .map_err(|error| format!("paired watch channel lock poisoned: {error}"))?;
        let Some(channel) = guard.as_mut() else {
            return Ok(());
        };
        if channel.channelId != channelId {
            return Err("paired watch subscription targets a stale channel".to_string());
        }
        channel.subscriptions.remove(subscriptionId);
        if channel.subscriptions.is_empty() {
            guard.take().map(|channel| channel.streamId)
        } else {
            None
        }
    };
    if let Some(streamId) = streamId {
        defaultHttpHost()
            .closeHttpByteStream(&streamId)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[async_trait(?Send)]
impl CoreLinkClient for PairedRemoteSession {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        match PairedRemoteSession::call(self, request).await {
            Ok(response) => response,
            Err(error) => CoreCallResponse::err(requestId, CoreLinkError::internal(error)),
        }
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        PairedRemoteSession::watchSnapshot(self, request)
            .await
            .map_err(CoreLinkError::internal)
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        PairedRemoteSession::watch(self, request)
            .await
            .map_err(CoreLinkError::internal)
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn hello(State(state): State<RemoteLinkState>, headers: HeaderMap) -> Response {
    if !token_matches(&state, &headers) {
        return unauthorized("invalid token");
    }
    Json(HelloResponse {
        protocolVersion: 3,
        pairingServiceVersion: REMOTE_PAIRING_SERVICE_VERSION,
        coreDeviceId: state.deviceId,
        coreDeviceInfo: state.deviceInfo,
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
    if let Err(error) = state.accessStore.savePendingPairing(pairingRecord) {
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
    let Some(pairing) = state.pairings.lock().await.remove(&request.pairingId) else {
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
    let sessionId = Uuid::new_v4().to_string();
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
            nonce: envelope.nonce,
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
async fn call(State(state): State<RemoteLinkState>, headers: HeaderMap, body: Bytes) -> Response {
    let verified = match verify_session(&state, &headers, &body).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    withRuntimeHostInteractionOrigin(verified.origin(), state.linkDispatcher.call(body)).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn watch_snapshot(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let verified = match verify_session(&state, &headers, &body).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    withRuntimeHostInteractionOrigin(verified.origin(), state.linkDispatcher.watchSnapshot(body))
        .await
}

#[cfg(not(target_arch = "wasm32"))]
async fn watch_channel_events(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    state.linkDispatcher.watchChannelEvents(body).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn watch_channel_open(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let verified = match verify_session(&state, &headers, &body).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    withRuntimeHostInteractionOrigin(
        verified.origin(),
        state.linkDispatcher.watchChannelOpen(body),
    )
    .await
}

#[cfg(not(target_arch = "wasm32"))]
async fn watch_channel_close(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    state.linkDispatcher.watchChannelClose(body).await
}

/// Opens an authenticated client-owned Link input stream.
#[cfg(not(target_arch = "wasm32"))]
async fn push_open(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let verified = match verify_session(&state, &headers, &body).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    withRuntimeHostInteractionOrigin(verified.origin(), state.linkDispatcher.pushOpen(body)).await
}

/// Accepts one authenticated item for a Link input stream.
#[cfg(not(target_arch = "wasm32"))]
async fn push_item(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let verified = match verify_session(&state, &headers, &body).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    withRuntimeHostInteractionOrigin(verified.origin(), state.linkDispatcher.pushItem(body)).await
}

/// Closes an authenticated client-owned Link input stream.
#[cfg(not(target_arch = "wasm32"))]
async fn push_close(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    state.linkDispatcher.pushClose(body).await
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

#[cfg(not(target_arch = "wasm32"))]
async fn ws(State(state): State<RemoteLinkState>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| handle_ws(socket, state))
        .into_response()
}

#[cfg(not(target_arch = "wasm32"))]
async fn handle_ws(mut socket: WebSocket, state: RemoteLinkState) {
    let mut pushes = BTreeMap::<String, RemotePushState>::new();
    while let Some(Ok(message)) = socket.recv().await {
        match message {
            Message::Binary(bytes) => {
                let response = handle_ws_binary(&state, &mut pushes, &bytes).await;
                let _ = socket.send(Message::Binary(response)).await;
            }
            Message::Close(frame) => {
                let _ = socket.send(Message::Close(frame)).await;
                break;
            }
            _ => {}
        }
    }
}

/// Decodes one signed websocket envelope and encodes its response.
#[cfg(not(target_arch = "wasm32"))]
async fn handle_ws_binary(
    state: &RemoteLinkState,
    pushes: &mut BTreeMap<String, RemotePushState>,
    bytes: &[u8],
) -> Vec<u8> {
    let response = match operit_link::decodeLink::<RemoteWsEnvelope>(bytes) {
        Ok(envelope) => handle_ws_envelope(state, pushes, envelope).await,
        Err(error) => RemoteWsResponse::Error(CoreLinkError::new("BAD_REQUEST", error.to_string())),
    };
    operit_link::encodeLink(&response).expect("RemoteWsResponse must serialize")
}

/// Verifies the raw websocket payload bytes and dispatches the decoded payload.
#[cfg(not(target_arch = "wasm32"))]
async fn handle_ws_envelope(
    state: &RemoteLinkState,
    pushes: &mut BTreeMap<String, RemotePushState>,
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
                nonce: request.nonce,
            })
        }
        RemoteWsPayload::Call(request) => {
            withRuntimeHostInteractionOrigin(verified.origin(), async {
                let mut core = state.core.lock().await;
                RemoteWsResponse::Call(core.call(request.request).await)
            })
            .await
        }
        RemoteWsPayload::WatchSnapshot(request) => {
            withRuntimeHostInteractionOrigin(verified.origin(), async {
                let mut core = state.core.lock().await;
                match core.watchSnapshot(request.request).await {
                    Ok(event) => RemoteWsResponse::WatchSnapshot(event),
                    Err(error) => RemoteWsResponse::Error(error),
                }
            })
            .await
        }
        RemoteWsPayload::PushOpen(request) => {
            let pushId = request.requestId.0.clone();
            if pushes.contains_key(&pushId) {
                return RemoteWsResponse::Error(CoreLinkError::new(
                    "PUSH_ALREADY_EXISTS",
                    "Link push stream already exists",
                ));
            }
            pushes.insert(
                pushId.clone(),
                RemotePushState {
                    request,
                    nextSequence: 0,
                },
            );
            RemoteWsResponse::PushOpened(pushId)
        }
        RemoteWsPayload::PushItem(item) => {
            let Some(push) = pushes.get_mut(&item.pushId) else {
                return RemoteWsResponse::Error(CoreLinkError::new(
                    "PUSH_NOT_FOUND",
                    "Link push stream not found",
                ));
            };
            if item.sequence != push.nextSequence {
                return RemoteWsResponse::Error(CoreLinkError::new(
                    "PUSH_SEQUENCE_MISMATCH",
                    format!(
                        "Link push sequence is {}, expected {}",
                        item.sequence, push.nextSequence
                    ),
                ));
            }
            push.nextSequence += 1;
            let request = push.request.clone();
            let response = withRuntimeHostInteractionOrigin(verified.origin(), async {
                let mut core = state.core.lock().await;
                core.call(request.itemCall(item.sequence, item.args.clone()))
                    .await
            })
            .await;
            match response.result {
                Ok(_) => RemoteWsResponse::PushAccepted(RemotePushAccepted {
                    pushId: item.pushId,
                    sequence: item.sequence,
                }),
                Err(error) => RemoteWsResponse::Error(error),
            }
        }
        RemoteWsPayload::PushClose(pushId) => {
            if pushes.remove(&pushId).is_none() {
                return RemoteWsResponse::Error(CoreLinkError::new(
                    "PUSH_NOT_FOUND",
                    "Link push stream not found",
                ));
            }
            RemoteWsResponse::PushClosed(pushId)
        }
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

#[cfg(not(target_arch = "wasm32"))]
impl VerifiedRemoteSession {
    fn origin(&self) -> RuntimeHostInteractionRequestOrigin {
        RuntimeHostInteractionRequestOrigin::RemoteSession {
            sessionId: self.sessionId.clone(),
            deviceId: self.deviceId.clone(),
        }
    }
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

#[cfg(not(target_arch = "wasm32"))]
fn verify_client_control(state: &RemoteLinkState, headers: &HeaderMap) -> Result<(), Response> {
    let Some(expected) = state.localControlToken.as_deref() else {
        return Err(unauthorized("client control token is not configured"));
    };
    let Some(provided) = header_string(headers, "x-operit-client-control-token") else {
        return Err(unauthorized("missing client control token"));
    };
    if provided != expected {
        return Err(unauthorized("invalid client control token"));
    }
    Ok(())
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
    let mut bytes = match (webAccess.readAsset)(&fullPath) {
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
    if relativePath == Path::new("index.html") {
        let html = match String::from_utf8(bytes) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        bytes = inject_web_access_runtime_config(&html).into_bytes();
    }
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
fn inject_web_access_runtime_config(html: &str) -> String {
    let config = serde_json::json!({
        "mode": "pair",
        "baseUrl": "",
        "pairingServiceVersion": REMOTE_PAIRING_SERVICE_VERSION,
    });
    let script = format!(
        "<script>window.__OPERIT_WEB_ACCESS__ = {};</script>",
        serde_json::to_string(&config).expect("web access config must serialize")
    );
    html.replace(
        "<script src=\"operit_runtime_bridge.js\"></script>",
        &format!("{script}\n  <script src=\"operit_runtime_bridge.js\"></script>"),
    )
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
