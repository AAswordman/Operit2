use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex as StdMutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Json, Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use futures_util::StreamExt;
use hmac::{Hmac, Mac};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};

use operit_link::CoreLinkClient;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError, CoreWatchRequest,
};

type HmacSha256 = Hmac<Sha256>;
pub const REMOTE_PAIRING_SERVICE_VERSION: i32 = 1;

#[derive(Clone)]
pub struct RemoteLinkServerConfig {
    pub bindAddress: String,
    pub token: String,
    pub deviceInfo: RemoteDeviceInfo,
    pub hostInteractionBroker: Option<RemoteHostInteractionBroker>,
    pub webAccess: Option<RemoteWebAccessConfig>,
    pub printStartupInfo: bool,
    pub acceptedSessions: BTreeMap<String, AcceptedRemoteSessionRecord>,
    pub acceptedSessionLoader: Option<AcceptedRemoteSessionLoader>,
    pub acceptedSessionStore: Option<AcceptedRemoteSessionStore>,
    pub pairingCodeSink: Option<RemotePairingCodeSink>,
}

impl Default for RemoteLinkServerConfig {
    fn default() -> Self {
        Self {
            bindAddress: "0.0.0.0:37192".to_string(),
            token: "operit-link-dev".to_string(),
            deviceInfo: RemoteDeviceInfo::native(),
            hostInteractionBroker: None,
            webAccess: None,
            printStartupInfo: true,
            acceptedSessions: BTreeMap::new(),
            acceptedSessionLoader: None,
            acceptedSessionStore: None,
            pairingCodeSink: None,
        }
    }
}

pub struct RemoteLinkServer;

pub type AcceptedRemoteSessionStore =
    Arc<dyn Fn(String, AcceptedRemoteSessionRecord) -> Result<(), String> + Send + Sync>;
pub type AcceptedRemoteSessionLoader =
    Arc<dyn Fn() -> Result<BTreeMap<String, AcceptedRemoteSessionRecord>, String> + Send + Sync>;
pub type RemotePairingCodeSink =
    Arc<dyn Fn(RemotePairingCodeRecord) -> Result<(), String> + Send + Sync>;

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

#[derive(Clone)]
pub struct RemoteWebAccessConfig {
    pub token: String,
    pub shutdownToken: String,
    pub webRoot: PathBuf,
}

#[derive(Clone)]
struct RemoteLinkState {
    core: Arc<Mutex<Box<dyn CoreLinkClient + Send>>>,
    linkDispatcher: operit_link::CoreLinkHttpDispatcher,
    token: String,
    keySecret: Arc<StaticSecret>,
    keyPublic: String,
    deviceId: String,
    deviceInfo: RemoteDeviceInfo,
    pairings: Arc<Mutex<BTreeMap<String, PendingPairing>>>,
    sessions: Arc<Mutex<BTreeMap<String, RemoteSession>>>,
    acceptedSessionIds: Arc<Mutex<BTreeSet<String>>>,
    acceptedSessionLoader: Option<AcceptedRemoteSessionLoader>,
    acceptedSessionStore: Option<AcceptedRemoteSessionStore>,
    pairingCodeSink: Option<RemotePairingCodeSink>,
    hostInteractionBroker: Option<RemoteHostInteractionBroker>,
    webAccess: Option<RemoteWebAccessState>,
}

#[derive(Clone)]
struct SharedAccessCoreClient {
    core: Arc<Mutex<Box<dyn CoreLinkClient + Send>>>,
}

#[async_trait]
impl CoreLinkClient for SharedAccessCoreClient {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        self.core.lock().await.call(request).await
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        self.core.lock().await.watchSnapshot(request).await
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.core.lock().await.watch(request).await
    }
}

#[derive(Clone)]
struct RemoteWebAccessState {
    shutdownToken: String,
    shutdownSender: Arc<StdMutex<Option<oneshot::Sender<()>>>>,
    webRoot: PathBuf,
}

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

#[derive(Clone, Debug)]
struct RemoteSession {
    deviceId: String,
    deviceInfo: RemoteDeviceInfo,
    pairingServiceVersion: i32,
    sessionSecret: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteDeviceInfo {
    pub platform: String,
    pub model: String,
}

impl RemoteDeviceInfo {
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
    pub token: String,
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
pub struct RemoteHostInteractionPollEnvelope {
    pub timeoutMs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteHostInteractionPollResponse {
    pub request: Option<RemoteHostInteractionRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteHostInteractionRequest {
    pub requestId: String,
    pub kind: String,
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteHostInteractionRespondEnvelope {
    pub requestId: String,
    pub response: serde_json::Value,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteWsEnvelope {
    pub sessionId: String,
    pub deviceId: String,
    pub signature: String,
    pub payload: RemoteWsPayload,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum RemoteWsPayload {
    SessionInfo(RemoteSessionInfoEnvelope),
    Call(RemoteCallEnvelope),
    WatchSnapshot(RemoteWatchEnvelope),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum RemoteWsResponse {
    SessionInfo(RemoteSessionInfoResponse),
    Call(CoreCallResponse),
    WatchSnapshot(CoreEvent),
    Error(CoreLinkError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairedRemoteSessionRecord {
    pub baseUrl: String,
    pub sessionId: String,
    pub deviceId: String,
    pub remoteDeviceInfo: RemoteDeviceInfo,
    pub pairingServiceVersion: i32,
    pub sessionSecret: String,
}

#[derive(Clone, Debug)]
pub struct PairStartState {
    pub pairingId: String,
    pub pairingServiceVersion: i32,
    pub clientDeviceId: String,
    pub clientDeviceInfo: RemoteDeviceInfo,
    pub clientPublicKey: String,
    pub coreDeviceInfo: RemoteDeviceInfo,
    pub clientNonce: String,
    pub serverNonce: String,
    pub sharedSecret: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct RemoteLinkClient {
    baseUrl: String,
    http: reqwest::Client,
}

#[derive(Clone, Debug)]
pub struct RemoteHostInteractionBroker {
    inner: Arc<RemoteHostInteractionBrokerInner>,
}

#[derive(Debug)]
struct RemoteHostInteractionBrokerInner {
    state: StdMutex<RemoteHostInteractionBrokerState>,
    changed: Condvar,
}

#[derive(Debug)]
struct RemoteHostInteractionBrokerState {
    pending: BTreeMap<String, RemoteHostInteractionRequest>,
    responses: BTreeMap<String, serde_json::Value>,
}

impl RemoteHostInteractionBroker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RemoteHostInteractionBrokerInner {
                state: StdMutex::new(RemoteHostInteractionBrokerState {
                    pending: BTreeMap::new(),
                    responses: BTreeMap::new(),
                }),
                changed: Condvar::new(),
            }),
        }
    }

    #[allow(non_snake_case)]
    pub fn requestInteraction(
        &self,
        kind: impl Into<String>,
        payload: serde_json::Value,
        timeout: Duration,
    ) -> Option<serde_json::Value> {
        let requestId = Uuid::new_v4().to_string();
        let request = RemoteHostInteractionRequest {
            requestId: requestId.clone(),
            kind: kind.into(),
            payload,
        };
        let startedAt = Instant::now();
        let mut state = self
            .inner
            .state
            .lock()
            .expect("remote host interaction mutex poisoned");
        state.pending.insert(requestId.clone(), request);
        self.inner.changed.notify_all();
        loop {
            if let Some(result) = state.responses.remove(&requestId) {
                state.pending.remove(&requestId);
                self.inner.changed.notify_all();
                return Some(result);
            }
            let elapsed = startedAt.elapsed();
            if elapsed >= timeout {
                state.pending.remove(&requestId);
                self.inner.changed.notify_all();
                return None;
            }
            let wait = timeout.saturating_sub(elapsed);
            let (nextState, _) = self
                .inner
                .changed
                .wait_timeout(state, wait)
                .expect("remote host interaction mutex poisoned");
            state = nextState;
        }
    }

    pub fn poll(&self, timeout: Duration) -> Option<RemoteHostInteractionRequest> {
        let startedAt = Instant::now();
        let mut state = self
            .inner
            .state
            .lock()
            .expect("remote host interaction mutex poisoned");
        loop {
            if let Some(request) = state.pending.values().next().cloned() {
                return Some(request);
            }
            let elapsed = startedAt.elapsed();
            if elapsed >= timeout {
                return None;
            }
            let wait = timeout.saturating_sub(elapsed);
            let (nextState, result) = self
                .inner
                .changed
                .wait_timeout(state, wait)
                .expect("remote host interaction mutex poisoned");
            state = nextState;
            if result.timed_out() && state.pending.is_empty() {
                return None;
            }
        }
    }

    pub fn respond(&self, requestId: &str, response: serde_json::Value) -> bool {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("remote host interaction mutex poisoned");
        if !state.pending.contains_key(requestId) {
            return false;
        }
        state.responses.insert(requestId.to_string(), response);
        self.inner.changed.notify_all();
        true
    }
}

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
        let acceptedSessionIds = Arc::new(Mutex::new(BTreeSet::new()));
        for (sessionId, session) in config.acceptedSessions.iter() {
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
            acceptedSessionIds.lock().await.insert(sessionId.clone());
        }
        let webAccess = webAccessConfig.clone().map(|value| RemoteWebAccessState {
            shutdownToken: value.shutdownToken,
            shutdownSender: Arc::new(StdMutex::new(Some(shutdownSender))),
            webRoot: value.webRoot,
        });
        let core = Arc::new(Mutex::new(Box::new(core) as Box<dyn CoreLinkClient + Send>));
        let linkDispatcher =
            operit_link::CoreLinkHttpDispatcher::new(SharedAccessCoreClient { core: core.clone() });
        let state = RemoteLinkState {
            core,
            linkDispatcher,
            token: config.token.clone(),
            keySecret,
            keyPublic,
            deviceId: format!("core-{}", Uuid::new_v4()),
            deviceInfo: config.deviceInfo.clone(),
            pairings: Arc::new(Mutex::new(BTreeMap::new())),
            sessions,
            acceptedSessionIds,
            acceptedSessionLoader: config.acceptedSessionLoader.clone(),
            acceptedSessionStore: config.acceptedSessionStore.clone(),
            pairingCodeSink: config.pairingCodeSink.clone(),
            hostInteractionBroker: config.hostInteractionBroker.clone(),
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
            .route("/host/interaction/poll", post(host_interaction_poll))
            .route("/host/interaction/respond", post(host_interaction_respond))
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
            http: reqwest::Client::new(),
        }
    }

    pub async fn hello(&self, token: &str) -> Result<HelloResponse, String> {
        self.http
            .get(format!("{}/link/hello", self.baseUrl))
            .header("x-operit-link-token", token)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<HelloResponse>()
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn pairStart(
        &self,
        token: &str,
        clientDeviceInfo: RemoteDeviceInfo,
    ) -> Result<PairStartState, String> {
        let clientSecret = StaticSecret::random_from_rng(OsRng);
        let clientPublic = PublicKey::from(&clientSecret);
        let clientDeviceId = format!("client-{}", Uuid::new_v4());
        let clientNonce = Uuid::new_v4().to_string();
        let request = PairStartRequest {
            pairingServiceVersion: REMOTE_PAIRING_SERVICE_VERSION,
            token: token.to_string(),
            clientDeviceId: clientDeviceId.clone(),
            clientDeviceInfo: clientDeviceInfo.clone(),
            clientPublicKey: public_key_to_string(&clientPublic),
            clientNonce: clientNonce.clone(),
        };
        let response = self
            .http
            .post(format!("{}/link/pair/start", self.baseUrl))
            .json(&request)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<PairStartResponse>()
            .await
            .map_err(|error| error.to_string())?;
        let corePublic = parse_public_key(&response.corePublicKey)?;
        let sharedSecret = clientSecret.diffie_hellman(&corePublic).as_bytes().to_vec();
        Ok(PairStartState {
            pairingId: response.pairingId,
            pairingServiceVersion: response.pairingServiceVersion,
            clientDeviceId,
            clientDeviceInfo,
            clientPublicKey: public_key_to_string(&clientPublic),
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
        let response = self
            .http
            .post(format!("{}/link/pair/finish", self.baseUrl))
            .json(&request)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<PairFinishResponse>()
            .await
            .map_err(|error| error.to_string())?;
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
            http: self.http.clone(),
            sessionId: response.sessionId,
            deviceId: state.clientDeviceId.clone(),
            remoteDeviceInfo: state.coreDeviceInfo.clone(),
            pairingServiceVersion: response.pairingServiceVersion,
            sessionSecret: session_secret(
                &state.sharedSecret,
                &state.clientNonce,
                &state.serverNonce,
            ),
            watchChannel: Arc::new(Mutex::new(None)),
        })
    }
}

#[derive(Clone)]
pub struct PairedRemoteSession {
    baseUrl: String,
    http: reqwest::Client,
    pub sessionId: String,
    pub deviceId: String,
    pub remoteDeviceInfo: RemoteDeviceInfo,
    pub pairingServiceVersion: i32,
    sessionSecret: Vec<u8>,
    watchChannel: Arc<Mutex<Option<PairedRemoteWatchChannel>>>,
}

struct PairedRemoteWatchChannel {
    channelId: String,
    subscriptions: BTreeMap<String, mpsc::UnboundedSender<CoreEvent>>,
    task: JoinHandle<()>,
}

impl PairedRemoteSession {
    #[allow(non_snake_case)]
    pub fn exportRecord(&self) -> PairedRemoteSessionRecord {
        PairedRemoteSessionRecord {
            baseUrl: self.baseUrl.clone(),
            sessionId: self.sessionId.clone(),
            deviceId: self.deviceId.clone(),
            remoteDeviceInfo: self.remoteDeviceInfo.clone(),
            pairingServiceVersion: self.pairingServiceVersion,
            sessionSecret: BASE64.encode(&self.sessionSecret),
        }
    }

    #[allow(non_snake_case)]
    pub fn fromRecord(record: PairedRemoteSessionRecord) -> Result<Self, String> {
        Ok(Self {
            baseUrl: record.baseUrl.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
            sessionId: record.sessionId,
            deviceId: record.deviceId,
            remoteDeviceInfo: record.remoteDeviceInfo,
            pairingServiceVersion: record.pairingServiceVersion,
            sessionSecret: BASE64
                .decode(record.sessionSecret)
                .map_err(|error| error.to_string())?,
            watchChannel: Arc::new(Mutex::new(None)),
        })
    }

    #[allow(non_snake_case)]
    pub async fn sessionInfo(&self) -> Result<RemoteSessionInfoResponse, String> {
        let body = serde_json::to_vec(&RemoteSessionInfoEnvelope {
            nonce: Uuid::new_v4().to_string(),
        })
        .map_err(|error| error.to_string())?;
        let signature = sign(&self.sessionSecret, &body);
        self.http
            .post(format!("{}/link/session", self.baseUrl))
            .header("x-operit-session", &self.sessionId)
            .header("x-operit-device", &self.deviceId)
            .header("x-operit-signature", signature)
            .body(body)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<RemoteSessionInfoResponse>()
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn call(&self, request: CoreCallRequest) -> Result<CoreCallResponse, String> {
        let body = serde_json::to_vec(&RemoteCallEnvelope { request })
            .map_err(|error| error.to_string())?;
        let signature = sign(&self.sessionSecret, &body);
        self.http
            .post(format!("{}/link/call", self.baseUrl))
            .header("x-operit-session", &self.sessionId)
            .header("x-operit-device", &self.deviceId)
            .header("x-operit-signature", signature)
            .body(body)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<CoreCallResponse>()
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, String> {
        let body = serde_json::to_vec(&RemoteWatchEnvelope { request })
            .map_err(|error| error.to_string())?;
        let signature = sign(&self.sessionSecret, &body);
        self.http
            .post(format!("{}/link/watch/snapshot", self.baseUrl))
            .header("x-operit-session", &self.sessionId)
            .header("x-operit-device", &self.deviceId)
            .header("x-operit-signature", signature)
            .body(body)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<CoreEvent>()
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, String> {
        let channelId = self.ensureWatchChannel().await?;
        let subscriptionId = format!("watch-{}", Uuid::new_v4().simple());
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        {
            let mut guard = self.watchChannel.lock().await;
            let Some(channel) = guard.as_mut() else {
                return Err("watch channel is not open".to_string());
            };
            if channel.channelId != channelId {
                return Err("watch channel changed while opening subscription".to_string());
            }
            channel.subscriptions.insert(subscriptionId.clone(), sender);
        }
        let body = serde_json::to_vec(&RemoteWatchChannelOpenEnvelope {
            channelId: channelId.clone(),
            subscriptionId: subscriptionId.clone(),
            request,
        })
        .map_err(|error| error.to_string())?;
        let signature = sign(&self.sessionSecret, &body);
        let openResult = self
            .http
            .post(format!("{}/link/watch/channel/open", self.baseUrl))
            .header("x-operit-session", &self.sessionId)
            .header("x-operit-device", &self.deviceId)
            .header("x-operit-signature", signature)
            .body(body)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<RemoteWatchChannelOpenResponse>()
            .await;
        if let Err(error) = openResult {
            self.removeLocalWatchSubscription(&channelId, &subscriptionId)
                .await;
            return Err(error.to_string());
        }
        let http = self.http.clone();
        let baseUrl = self.baseUrl.clone();
        let sessionId = self.sessionId.clone();
        let deviceId = self.deviceId.clone();
        let sessionSecret = self.sessionSecret.clone();
        let watchChannel = self.watchChannel.clone();
        Ok(CoreEventStream::new(receiver).withOnClose(move || {
            tokio::spawn(async move {
                remove_paired_watch_subscription(&watchChannel, &channelId, &subscriptionId).await;
                let body = match serde_json::to_vec(&RemoteWatchChannelCloseEnvelope {
                    channelId,
                    subscriptionId,
                }) {
                    Ok(value) => value,
                    Err(_) => return,
                };
                let signature = sign(&sessionSecret, &body);
                let _ = http
                    .post(format!("{}/link/watch/channel/close", baseUrl))
                    .header("x-operit-session", sessionId)
                    .header("x-operit-device", deviceId)
                    .header("x-operit-signature", signature)
                    .body(body)
                    .send()
                    .await;
            });
        }))
    }

    #[allow(non_snake_case)]
    async fn ensureWatchChannel(&self) -> Result<String, String> {
        if let Some(channelId) = self
            .watchChannel
            .lock()
            .await
            .as_ref()
            .map(|channel| channel.channelId.clone())
        {
            return Ok(channelId);
        }
        let channelId = format!("watch-channel-{}", Uuid::new_v4().simple());
        let body = serde_json::to_vec(&RemoteWatchChannelEnvelope {
            channelId: channelId.clone(),
        })
        .map_err(|error| error.to_string())?;
        let signature = sign(&self.sessionSecret, &body);
        let response = self
            .http
            .post(format!("{}/link/watch/channel/events", self.baseUrl))
            .header("x-operit-session", &self.sessionId)
            .header("x-operit-device", &self.deviceId)
            .header("x-operit-signature", signature)
            .body(body)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        let watchChannel = self.watchChannel.clone();
        let taskChannelId = channelId.clone();
        let task = tokio::spawn(async move {
            let mut bytes = response.bytes_stream();
            let mut buffer = Vec::<u8>::new();
            while let Some(item) = bytes.next().await {
                let Ok(chunk) = item else {
                    break;
                };
                buffer.extend_from_slice(&chunk);
                while let Some(index) = buffer.iter().position(|byte| *byte == b'\n') {
                    let line = buffer.drain(..=index).collect::<Vec<_>>();
                    let line = &line[..line.len().saturating_sub(1)];
                    if line.is_empty() {
                        continue;
                    }
                    let Ok(event) = serde_json::from_slice::<RemoteWatchChannelEvent>(line) else {
                        continue;
                    };
                    let sender = {
                        let guard = watchChannel.lock().await;
                        guard.as_ref().and_then(|channel| {
                            if channel.channelId == taskChannelId {
                                channel.subscriptions.get(&event.subscriptionId).cloned()
                            } else {
                                None
                            }
                        })
                    };
                    if let Some(sender) = sender {
                        let _ = sender.send(event.event);
                    }
                }
            }
            let mut guard = watchChannel.lock().await;
            if guard.as_ref().map(|channel| channel.channelId.as_str())
                == Some(taskChannelId.as_str())
            {
                let _ = guard.take();
            }
        });
        let mut guard = self.watchChannel.lock().await;
        if let Some(previous) = guard.replace(PairedRemoteWatchChannel {
            channelId: channelId.clone(),
            subscriptions: BTreeMap::new(),
            task,
        }) {
            previous.task.abort();
        }
        Ok(channelId)
    }

    #[allow(non_snake_case)]
    async fn removeLocalWatchSubscription(&self, channelId: &str, subscriptionId: &str) {
        remove_paired_watch_subscription(&self.watchChannel, channelId, subscriptionId).await;
    }

    #[allow(non_snake_case)]
    pub async fn pollHostInteraction(
        &self,
        timeoutMs: u64,
    ) -> Result<Option<RemoteHostInteractionRequest>, String> {
        let body = serde_json::to_vec(&RemoteHostInteractionPollEnvelope { timeoutMs })
            .map_err(|error| error.to_string())?;
        let signature = sign(&self.sessionSecret, &body);
        let response = self
            .http
            .post(format!("{}/host/interaction/poll", self.baseUrl))
            .header("x-operit-session", &self.sessionId)
            .header("x-operit-device", &self.deviceId)
            .header("x-operit-signature", signature)
            .body(body)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json::<RemoteHostInteractionPollResponse>()
            .await
            .map_err(|error| error.to_string())?;
        Ok(response.request)
    }

    #[allow(non_snake_case)]
    pub async fn respondHostInteraction(
        &self,
        requestId: &str,
        response: serde_json::Value,
    ) -> Result<(), String> {
        let body = serde_json::to_vec(&RemoteHostInteractionRespondEnvelope {
            requestId: requestId.to_string(),
            response,
        })
        .map_err(|error| error.to_string())?;
        let signature = sign(&self.sessionSecret, &body);
        self.http
            .post(format!("{}/host/interaction/respond", self.baseUrl))
            .header("x-operit-session", &self.sessionId)
            .header("x-operit-device", &self.deviceId)
            .header("x-operit-signature", signature)
            .body(body)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

async fn remove_paired_watch_subscription(
    watchChannel: &Arc<Mutex<Option<PairedRemoteWatchChannel>>>,
    channelId: &str,
    subscriptionId: &str,
) {
    let mut guard = watchChannel.lock().await;
    if let Some(channel) = guard.as_mut() {
        if channel.channelId == channelId {
            channel.subscriptions.remove(subscriptionId);
        }
    }
}

#[async_trait]
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

async fn hello(State(state): State<RemoteLinkState>, headers: HeaderMap) -> Response {
    if !token_matches(&state, &headers) {
        return unauthorized("invalid token");
    }
    Json(HelloResponse {
        protocolVersion: 1,
        pairingServiceVersion: REMOTE_PAIRING_SERVICE_VERSION,
        coreDeviceId: state.deviceId,
        coreDeviceInfo: state.deviceInfo,
        corePublicKey: state.keyPublic,
        transports: vec!["http".to_string(), "ws".to_string()],
        pairingRequired: true,
    })
    .into_response()
}

async fn pair_start(
    State(state): State<RemoteLinkState>,
    Json(request): Json<PairStartRequest>,
) -> Response {
    if request.token != state.token {
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
    if let Some(sink) = state.pairingCodeSink.as_ref() {
        if let Err(error) = sink(RemotePairingCodeRecord {
            pairingId: pairingId.clone(),
            pairingServiceVersion: request.pairingServiceVersion,
            clientDeviceId: request.clientDeviceId.clone(),
            clientDeviceInfo: request.clientDeviceInfo.clone(),
            pairingCode: pairingCode.clone(),
            createdAt: unix_millis(),
        }) {
            return internal_server_error(error);
        }
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
    if let Some(store) = state.acceptedSessionStore.as_ref() {
        if let Err(error) = store(
            sessionId.clone(),
            AcceptedRemoteSessionRecord {
                deviceId: pairing.clientDeviceId.clone(),
                deviceInfo: pairing.clientDeviceInfo.clone(),
                pairingServiceVersion: pairing.pairingServiceVersion,
                sessionSecret: BASE64.encode(sessionSecret.as_slice()),
            },
        ) {
            return internal_server_error(error);
        }
    }
    state
        .acceptedSessionIds
        .lock()
        .await
        .insert(sessionId.clone());
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

async fn session_info(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    let envelope = match serde_json::from_slice::<RemoteSessionInfoEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => return bad_request(error.to_string()),
    };
    let Some(sessionId) = header_string(&headers, "x-operit-session") else {
        return unauthorized("missing session");
    };
    let sessions = state.sessions.lock().await;
    let Some(session) = sessions.get(&sessionId) else {
        return unauthorized("invalid session");
    };
    Json(RemoteSessionInfoResponse {
        protocolVersion: 1,
        pairingServiceVersion: session.pairingServiceVersion,
        coreDeviceId: state.deviceId,
        coreDeviceInfo: state.deviceInfo,
        clientDeviceId: session.deviceId.clone(),
        clientDeviceInfo: session.deviceInfo.clone(),
        transports: vec!["http".to_string(), "ws".to_string()],
        nonce: envelope.nonce,
    })
    .into_response()
}

async fn call(State(state): State<RemoteLinkState>, headers: HeaderMap, body: Bytes) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    state.linkDispatcher.call(body).await
}

async fn watch_snapshot(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    state.linkDispatcher.watchSnapshot(body).await
}

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

async fn watch_channel_open(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    state.linkDispatcher.watchChannelOpen(body).await
}

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

async fn host_interaction_poll(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    let envelope = match serde_json::from_slice::<RemoteHostInteractionPollEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => return bad_request(error.to_string()),
    };
    let Some(broker) = state.hostInteractionBroker.clone() else {
        return Json(RemoteHostInteractionPollResponse { request: None }).into_response();
    };
    let request = match tokio::task::spawn_blocking(move || {
        broker.poll(Duration::from_millis(envelope.timeoutMs))
    })
    .await
    {
        Ok(request) => request,
        Err(error) => return bad_request(error.to_string()),
    };
    Json(RemoteHostInteractionPollResponse { request }).into_response()
}

async fn host_interaction_respond(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    let envelope = match serde_json::from_slice::<RemoteHostInteractionRespondEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => return bad_request(error.to_string()),
    };
    let Some(broker) = state.hostInteractionBroker.clone() else {
        return bad_request("host interaction broker is not registered");
    };
    if broker.respond(&envelope.requestId, envelope.response) {
        Json(serde_json::json!({"ok": true})).into_response()
    } else {
        bad_request("host interaction request not found")
    }
}

async fn web_access_index(State(state): State<RemoteLinkState>) -> Response {
    let Some(webAccess) = state.webAccess.as_ref() else {
        return bad_request("web access is not enabled");
    };
    serve_web_access_file(webAccess, "index.html")
}

async fn web_access_asset(
    State(state): State<RemoteLinkState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let Some(webAccess) = state.webAccess.as_ref() else {
        return bad_request("web access is not enabled");
    };
    serve_web_access_file(webAccess, &path)
}

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

async fn ws(State(state): State<RemoteLinkState>, upgrade: WebSocketUpgrade) -> Response {
    upgrade
        .on_upgrade(move |socket| handle_ws(socket, state))
        .into_response()
}

async fn handle_ws(mut socket: WebSocket, state: RemoteLinkState) {
    while let Some(Ok(message)) = socket.recv().await {
        match message {
            Message::Text(text) => {
                let response = handle_ws_text(&state, text).await;
                let _ = socket.send(Message::Text(response)).await;
            }
            Message::Close(frame) => {
                let _ = socket.send(Message::Close(frame)).await;
                break;
            }
            _ => {}
        }
    }
}

async fn handle_ws_text(state: &RemoteLinkState, text: String) -> String {
    let response = match serde_json::from_str::<RemoteWsEnvelope>(&text) {
        Ok(envelope) => handle_ws_envelope(state, envelope).await,
        Err(error) => RemoteWsResponse::Error(CoreLinkError::new("BAD_REQUEST", error.to_string())),
    };
    serde_json::to_string(&response).expect("RemoteWsResponse must serialize")
}

async fn handle_ws_envelope(
    state: &RemoteLinkState,
    envelope: RemoteWsEnvelope,
) -> RemoteWsResponse {
    let body = match serde_json::to_vec(&envelope.payload) {
        Ok(value) => value,
        Err(error) => {
            return RemoteWsResponse::Error(CoreLinkError::internal(error.to_string()));
        }
    };
    if let Err(error) = verify_session_parts(
        state,
        &envelope.sessionId,
        &envelope.deviceId,
        &envelope.signature,
        &body,
    )
    .await
    {
        return RemoteWsResponse::Error(error);
    }
    match envelope.payload {
        RemoteWsPayload::SessionInfo(request) => {
            let sessions = state.sessions.lock().await;
            let Some(session) = sessions.get(&envelope.sessionId) else {
                return RemoteWsResponse::Error(CoreLinkError::new(
                    "UNAUTHORIZED",
                    "invalid session",
                ));
            };
            RemoteWsResponse::SessionInfo(RemoteSessionInfoResponse {
                protocolVersion: 1,
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
            let mut core = state.core.lock().await;
            RemoteWsResponse::Call(core.call(request.request).await)
        }
        RemoteWsPayload::WatchSnapshot(request) => {
            let mut core = state.core.lock().await;
            match core.watchSnapshot(request.request).await {
                Ok(event) => RemoteWsResponse::WatchSnapshot(event),
                Err(error) => RemoteWsResponse::Error(error),
            }
        }
    }
}

async fn verify_session(
    state: &RemoteLinkState,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), Response> {
    let Some(sessionId) = header_string(headers, "x-operit-session") else {
        return Err(unauthorized("missing session"));
    };
    let Some(deviceId) = header_string(headers, "x-operit-device") else {
        return Err(unauthorized("missing device"));
    };
    let Some(signature) = header_string(headers, "x-operit-signature") else {
        return Err(unauthorized("missing signature"));
    };
    verify_session_parts(state, &sessionId, &deviceId, &signature, body)
        .await
        .map_err(|error| (StatusCode::UNAUTHORIZED, Json(error)).into_response())
}

async fn verify_session_parts(
    state: &RemoteLinkState,
    sessionId: &str,
    deviceId: &str,
    signature: &str,
    body: &[u8],
) -> Result<(), CoreLinkError> {
    refresh_accepted_session(state, sessionId).await?;
    let sessions = state.sessions.lock().await;
    let Some(session) = sessions.get(sessionId) else {
        return Err(CoreLinkError::new("UNAUTHORIZED", "invalid session"));
    };
    if session.deviceId != deviceId {
        return Err(CoreLinkError::new("UNAUTHORIZED", "device mismatch"));
    }
    if sign(&session.sessionSecret, body) != signature {
        return Err(CoreLinkError::new("UNAUTHORIZED", "signature mismatch"));
    }
    Ok(())
}

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

async fn refresh_accepted_session(
    state: &RemoteLinkState,
    sessionId: &str,
) -> Result<(), CoreLinkError> {
    let known_accepted_session = state.acceptedSessionIds.lock().await.contains(sessionId);
    if !known_accepted_session {
        return Ok(());
    }
    let Some(loader) = state.acceptedSessionLoader.as_ref() else {
        return Ok(());
    };
    let records = loader().map_err(CoreLinkError::internal)?;
    if let Some(record) = records.get(sessionId) {
        state
            .sessions
            .lock()
            .await
            .insert(sessionId.to_string(), accepted_session_from_record(record)?);
    } else {
        state.acceptedSessionIds.lock().await.remove(sessionId);
        state.sessions.lock().await.remove(sessionId);
    }
    Ok(())
}

fn token_matches(state: &RemoteLinkState, headers: &HeaderMap) -> bool {
    header_string(headers, "x-operit-link-token")
        .map(|value| value == state.token)
        .unwrap_or(false)
}

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

fn pairing_code() -> String {
    let bytes = Uuid::new_v4().as_u128();
    format!("{:06}", (bytes % 1_000_000) as u32)
}

fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
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

fn unauthorized(message: impl Into<String>) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(CoreLinkError::new("UNAUTHORIZED", message.into())),
    )
        .into_response()
}

fn bad_request(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(CoreLinkError::new("BAD_REQUEST", message.into())),
    )
        .into_response()
}

fn internal_server_error(message: impl Into<String>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(CoreLinkError::new("INTERNAL_SERVER_ERROR", message.into())),
    )
        .into_response()
}

fn serve_web_access_file(webAccess: &RemoteWebAccessState, path: &str) -> Response {
    let relativePath = match sanitize_web_asset_path(path) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let fullPath = webAccess.webRoot.join(&relativePath);
    if !fullPath.starts_with(&webAccess.webRoot) {
        return bad_request("web asset path escapes web root");
    }
    let mut bytes = match fs::read(&fullPath) {
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
        .body(Body::from(bytes))
        .expect("web asset response must build")
}

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
