use std::collections::BTreeMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Condvar, Mutex as StdMutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::body::Body;
use axum::body::Bytes;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Json, State};
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
use tokio::sync::Mutex;
use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::client::CoreLinkClient;
use crate::protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError, CoreWatchRequest,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct RemoteLinkServerConfig {
    pub bindAddress: String,
    pub token: String,
    pub hostInteractionBroker: Option<RemoteHostInteractionBroker>,
}

impl Default for RemoteLinkServerConfig {
    fn default() -> Self {
        Self {
            bindAddress: "0.0.0.0:37192".to_string(),
            token: "operit-link-dev".to_string(),
            hostInteractionBroker: None,
        }
    }
}

pub struct RemoteLinkServer;

#[derive(Clone)]
struct RemoteLinkState {
    core: Arc<Mutex<Box<dyn CoreLinkClient + Send>>>,
    token: String,
    keySecret: Arc<StaticSecret>,
    keyPublic: String,
    deviceId: String,
    pairings: Arc<Mutex<BTreeMap<String, PendingPairing>>>,
    sessions: Arc<Mutex<BTreeMap<String, RemoteSession>>>,
    hostInteractionBroker: Option<RemoteHostInteractionBroker>,
}

#[derive(Clone, Debug)]
struct PendingPairing {
    clientDeviceId: String,
    clientPublicKey: String,
    pairingCode: String,
    serverNonce: String,
    clientNonce: String,
    sharedSecret: Vec<u8>,
}

#[derive(Clone, Debug)]
struct RemoteSession {
    deviceId: String,
    sessionSecret: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HelloResponse {
    pub protocolVersion: i32,
    pub coreDeviceId: String,
    pub corePublicKey: String,
    pub transports: Vec<String>,
    pub pairingRequired: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairStartRequest {
    pub token: String,
    pub clientDeviceId: String,
    pub clientPublicKey: String,
    pub clientNonce: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairStartResponse {
    pub pairingId: String,
    pub coreDeviceId: String,
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
    pub coreDeviceId: String,
    pub clientDeviceId: String,
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
    pub sessionSecret: String,
}

#[derive(Clone, Debug)]
pub struct PairStartState {
    pub pairingId: String,
    pub clientDeviceId: String,
    pub clientPublicKey: String,
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
        let keySecret = Arc::new(StaticSecret::random_from_rng(OsRng));
        let keyPublic = public_key_to_string(&PublicKey::from(keySecret.as_ref()));
        let state = RemoteLinkState {
            core: Arc::new(Mutex::new(Box::new(core))),
            token: config.token.clone(),
            keySecret,
            keyPublic,
            deviceId: format!("core-{}", Uuid::new_v4()),
            pairings: Arc::new(Mutex::new(BTreeMap::new())),
            sessions: Arc::new(Mutex::new(BTreeMap::new())),
            hostInteractionBroker: config.hostInteractionBroker.clone(),
        };
        let app = Router::new()
            .route("/link/hello", get(hello))
            .route("/link/pair/start", post(pair_start))
            .route("/link/pair/finish", post(pair_finish))
            .route("/link/session", post(session_info))
            .route("/link/call", post(call))
            .route("/link/watch/snapshot", post(watch_snapshot))
            .route("/link/watch/stream", post(watch_stream))
            .route("/host/interaction/poll", post(host_interaction_poll))
            .route("/host/interaction/respond", post(host_interaction_respond))
            .route("/link/ws", get(ws))
            .with_state(state);
        let address: SocketAddr = config
            .bindAddress
            .parse()
            .map_err(|error| format!("invalid bind address: {error}"))?;
        let listener = TcpListener::bind(address)
            .await
            .map_err(|error| error.to_string())?;
        println!("operit link server listening on http://{address}");
        println!("link token: {}", config.token);
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

    pub async fn pairStart(&self, token: &str) -> Result<PairStartState, String> {
        let clientSecret = StaticSecret::random_from_rng(OsRng);
        let clientPublic = PublicKey::from(&clientSecret);
        let clientDeviceId = format!("client-{}", Uuid::new_v4());
        let clientNonce = Uuid::new_v4().to_string();
        let request = PairStartRequest {
            token: token.to_string(),
            clientDeviceId: clientDeviceId.clone(),
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
            clientDeviceId,
            clientPublicKey: public_key_to_string(&clientPublic),
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
            sessionSecret: session_secret(
                &state.sharedSecret,
                &state.clientNonce,
                &state.serverNonce,
            ),
        })
    }
}

#[derive(Clone, Debug)]
pub struct PairedRemoteSession {
    baseUrl: String,
    http: reqwest::Client,
    pub sessionId: String,
    pub deviceId: String,
    sessionSecret: Vec<u8>,
}

impl PairedRemoteSession {
    #[allow(non_snake_case)]
    pub fn exportRecord(&self) -> PairedRemoteSessionRecord {
        PairedRemoteSessionRecord {
            baseUrl: self.baseUrl.clone(),
            sessionId: self.sessionId.clone(),
            deviceId: self.deviceId.clone(),
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
            sessionSecret: BASE64
                .decode(record.sessionSecret)
                .map_err(|error| error.to_string())?,
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
        let body = serde_json::to_vec(&RemoteWatchEnvelope { request })
            .map_err(|error| error.to_string())?;
        let signature = sign(&self.sessionSecret, &body);
        let response = self
            .http
            .post(format!("{}/link/watch/stream", self.baseUrl))
            .header("x-operit-session", &self.sessionId)
            .header("x-operit-device", &self.deviceId)
            .header("x-operit-signature", signature)
            .body(body)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
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
                    if let Ok(event) = serde_json::from_slice::<CoreEvent>(line) {
                        let _ = sender.send(event);
                    }
                }
            }
        });
        Ok(CoreEventStream::new(receiver).withOnClose(move || task.abort()))
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
        coreDeviceId: state.deviceId,
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
    state.pairings.lock().await.insert(
        pairingId.clone(),
        PendingPairing {
            clientDeviceId: request.clientDeviceId,
            clientPublicKey: request.clientPublicKey,
            pairingCode,
            serverNonce: serverNonce.clone(),
            clientNonce: request.clientNonce,
            sharedSecret,
        },
    );
    Json(PairStartResponse {
        pairingId,
        coreDeviceId: state.deviceId,
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
    state.sessions.lock().await.insert(
        sessionId.clone(),
        RemoteSession {
            deviceId: pairing.clientDeviceId,
            sessionSecret,
        },
    );
    Json(PairFinishResponse {
        sessionId,
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
        coreDeviceId: state.deviceId,
        clientDeviceId: session.deviceId.clone(),
        transports: vec!["http".to_string(), "ws".to_string()],
        nonce: envelope.nonce,
    })
    .into_response()
}

async fn call(State(state): State<RemoteLinkState>, headers: HeaderMap, body: Bytes) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    let envelope = match serde_json::from_slice::<RemoteCallEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => return bad_request(error.to_string()),
    };
    let mut core = state.core.lock().await;
    Json(core.call(envelope.request).await).into_response()
}

async fn watch_snapshot(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    let envelope = match serde_json::from_slice::<RemoteWatchEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => return bad_request(error.to_string()),
    };
    let mut core = state.core.lock().await;
    match core.watchSnapshot(envelope.request).await {
        Ok(event) => Json(event).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, Json(error)).into_response(),
    }
}

async fn watch_stream(
    State(state): State<RemoteLinkState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = verify_session(&state, &headers, &body).await {
        return response;
    }
    let envelope = match serde_json::from_slice::<RemoteWatchEnvelope>(&body) {
        Ok(value) => value,
        Err(error) => return bad_request(error.to_string()),
    };
    let mut core = state.core.lock().await;
    let receiver = match core.watch(envelope.request).await {
        Ok(receiver) => receiver,
        Err(error) => return (StatusCode::BAD_REQUEST, Json(error)).into_response(),
    };
    drop(core);
    let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|event| {
            let mut line = serde_json::to_vec(&event).expect("CoreEvent must serialize");
            line.push(b'\n');
            (Ok::<Bytes, Infallible>(Bytes::from(line)), receiver)
        })
    });
    Response::builder()
        .header("content-type", "application/x-ndjson")
        .body(Body::from_stream(stream))
        .expect("watch stream response must build")
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
                coreDeviceId: state.deviceId.clone(),
                clientDeviceId: session.deviceId.clone(),
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
