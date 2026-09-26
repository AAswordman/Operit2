#![allow(non_snake_case)]

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use hmac::{Hmac, Mac};
use operit_link::{
    LinkDeviceInfo, LinkFrame, LinkFramePayload, LinkPairFinishRequest, LinkPairFinishResponse,
    LinkPairStartRequest, LinkPairStartResponse,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::LinkChannel;

type HmacSha256 = Hmac<Sha256>;

pub const EDGE_PAIRING_SERVICE_VERSION: u16 = 1;

/// A completed Edge pairing session that can authenticate future Link frames.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeSession {
    pub sessionId: String,
    pub deviceId: String,
    pub peerDeviceId: String,
    pub sessionSecret: Vec<u8>,
}

/// Small persisted state owned by a lightweight Edge device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgePairingPersistentState {
    /// The device's long-lived X25519 private key bytes.
    pub keySecret: Vec<u8>,
    /// Authenticated Core sessions accepted after a device restart.
    pub sessions: Vec<EdgeSession>,
}

/// Storage boundary for device-specific NVS/flash implementations.
pub trait EdgePairingStore: Send + Sync {
    fn load(&self) -> Result<Option<EdgePairingPersistentState>, String>;
    fn save(&self, state: &EdgePairingPersistentState) -> Result<(), String>;
}

/// Client-side state kept between pairing start and the user-entered code.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgePairStartState {
    pub pairingId: String,
    pub clientDeviceId: String,
    pub edgeDeviceId: String,
    pub edgeDeviceInfo: LinkDeviceInfo,
    clientNonce: String,
    serverNonce: String,
    sharedSecret: Vec<u8>,
}

/// Returns the same token hash format used by Link Access pairing.
pub fn linkTokenHash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    BASE64.encode(hasher.finalize())
}

/// Signs one encoded Link payload with a completed pairing session.
pub fn signSession(sessionSecret: &[u8], payload: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(sessionSecret).expect("HMAC accepts any session secret length");
    mac.update(payload);
    BASE64.encode(mac.finalize().into_bytes())
}

fn publicKeyString(key: &PublicKey) -> String {
    BASE64.encode(key.as_bytes())
}

fn parsePublicKey(value: &str) -> Result<PublicKey, String> {
    let bytes = BASE64.decode(value).map_err(|error| error.to_string())?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "X25519 public key must contain 32 bytes".to_string())?;
    Ok(PublicKey::from(bytes))
}

fn proof(sharedSecret: &[u8], clientNonce: &str, serverNonce: &str, role: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sharedSecret);
    hasher.update(clientNonce.as_bytes());
    hasher.update(serverNonce.as_bytes());
    hasher.update(role.as_bytes());
    BASE64.encode(hasher.finalize())
}

fn sessionSecret(sharedSecret: &[u8], clientNonce: &str, serverNonce: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(sharedSecret);
    hasher.update(clientNonce.as_bytes());
    hasher.update(serverNonce.as_bytes());
    hasher.update(b"session");
    hasher.finalize().to_vec()
}

fn pairingCode() -> String {
    let mut bytes = [0u8; 4];
    OsRng.fill_bytes(&mut bytes);
    format!("{:06}", u32::from_be_bytes(bytes) % 1_000_000)
}

#[derive(Clone)]
struct PendingPairing {
    clientDeviceId: String,
    clientDeviceInfo: LinkDeviceInfo,
    clientNonce: String,
    serverNonce: String,
    sharedSecret: Vec<u8>,
    pairingCode: String,
}

/// Owns the small pairing state required by an Edge device.
pub struct EdgePairingAuthority {
    tokenHash: String,
    deviceId: String,
    deviceInfo: LinkDeviceInfo,
    keySecret: StaticSecret,
    pending: Mutex<BTreeMap<String, PendingPairing>>,
    sessions: Mutex<BTreeMap<String, EdgeSession>>,
    store: Option<Arc<dyn EdgePairingStore>>,
    onPairingCode: Arc<dyn Fn(String) + Send + Sync>,
}

impl EdgePairingAuthority {
    pub fn new(
        token: impl Into<String>,
        deviceId: impl Into<String>,
        deviceInfo: LinkDeviceInfo,
        onPairingCode: impl Fn(String) + Send + Sync + 'static,
    ) -> Self {
        let token = token.into();
        Self {
            tokenHash: linkTokenHash(&token),
            deviceId: deviceId.into(),
            deviceInfo,
            keySecret: StaticSecret::random_from_rng(OsRng),
            pending: Mutex::new(BTreeMap::new()),
            sessions: Mutex::new(BTreeMap::new()),
            store: None,
            onPairingCode: Arc::new(onPairingCode),
        }
    }

    /// Creates an authority backed by device flash/NVS. The private key and
    /// completed sessions are restored before accepting clients.
    pub fn newWithStore(
        token: impl Into<String>,
        deviceId: impl Into<String>,
        deviceInfo: LinkDeviceInfo,
        store: Arc<dyn EdgePairingStore>,
        onPairingCode: impl Fn(String) + Send + Sync + 'static,
    ) -> Result<Self, String> {
        let token = token.into();
        let deviceId = deviceId.into();
        let (keySecret, sessions) = match store.load()? {
            Some(state) => {
                let keyBytes: [u8; 32] = state
                    .keySecret
                    .try_into()
                    .map_err(|_| "persisted Edge X25519 key must contain 32 bytes".to_string())?;
                let sessions = state
                    .sessions
                    .into_iter()
                    .map(|session| {
                        if session.deviceId != deviceId {
                            return Err(format!(
                                "persisted Edge session {} belongs to a different device",
                                session.sessionId
                            ));
                        }
                        Ok((session.sessionId.clone(), session))
                    })
                    .collect::<Result<BTreeMap<_, _>, String>>()?;
                (StaticSecret::from(keyBytes), sessions)
            }
            None => {
                let keySecret = StaticSecret::random_from_rng(OsRng);
                store.save(&EdgePairingPersistentState {
                    keySecret: keySecret.to_bytes().to_vec(),
                    sessions: Vec::new(),
                })?;
                (keySecret, BTreeMap::new())
            }
        };
        Ok(Self {
            tokenHash: linkTokenHash(&token),
            deviceId,
            deviceInfo,
            keySecret,
            pending: Mutex::new(BTreeMap::new()),
            sessions: Mutex::new(sessions),
            store: Some(store),
            onPairingCode: Arc::new(onPairingCode),
        })
    }

    fn persistSessions(&self) -> Result<(), String> {
        let Some(store) = self.store.as_ref() else {
            return Ok(());
        };
        let sessions = self
            .sessions
            .lock()
            .map_err(|error| error.to_string())?
            .values()
            .cloned()
            .collect();
        store.save(&EdgePairingPersistentState {
            keySecret: self.keySecret.to_bytes().to_vec(),
            sessions,
        })
    }

    /// Completes the exact two-step Link pairing flow over a raw carrier.
    pub async fn pair(&self, channel: Arc<dyn LinkChannel>) -> Result<EdgeSession, String> {
        let start = match channel
            .receive()
            .await?
            .ok_or_else(|| "Edge pairing channel closed".to_string())?
            .payload
        {
            LinkFramePayload::PairStart(request) => request,
            _ => return Err("expected Link pairing start".to_string()),
        };
        self.pairFromStart(channel, start).await
    }

    /// Completes pairing after the caller has already consumed PairStart.
    pub async fn pairFromStart(
        &self,
        channel: Arc<dyn LinkChannel>,
        start: LinkPairStartRequest,
    ) -> Result<EdgeSession, String> {
        if start.pairingServiceVersion != EDGE_PAIRING_SERVICE_VERSION {
            return Err("unsupported Edge pairing service version".to_string());
        }
        if start.tokenHash != self.tokenHash {
            return Err("invalid Edge pairing token".to_string());
        }
        let clientPublic = parsePublicKey(&start.clientPublicKey)?;
        let serverPublic = PublicKey::from(&self.keySecret);
        let sharedSecret = self
            .keySecret
            .diffie_hellman(&clientPublic)
            .as_bytes()
            .to_vec();
        let pairingId = Uuid::new_v4().to_string();
        let serverNonce = Uuid::new_v4().to_string();
        let code = pairingCode();
        self.pending
            .lock()
            .map_err(|error| error.to_string())?
            .insert(
                pairingId.clone(),
                PendingPairing {
                    clientDeviceId: start.clientDeviceId.clone(),
                    clientDeviceInfo: start.clientDeviceInfo.clone(),
                    clientNonce: start.clientNonce,
                    serverNonce: serverNonce.clone(),
                    sharedSecret,
                    pairingCode: code.clone(),
                },
            );
        (self.onPairingCode)(code);
        channel
            .send(LinkFrame {
                messageId: "pair-start-response".to_string(),
                payload: LinkFramePayload::PairStartResponse(LinkPairStartResponse {
                    pairingId,
                    pairingServiceVersion: EDGE_PAIRING_SERVICE_VERSION,
                    edgeDeviceId: self.deviceId.clone(),
                    edgeDeviceInfo: self.deviceInfo.clone(),
                    edgePublicKey: publicKeyString(&serverPublic),
                    serverNonce,
                }),
            })
            .await?;
        let finish = match channel
            .receive()
            .await?
            .ok_or_else(|| "Edge pairing channel closed before finish".to_string())?
            .payload
        {
            LinkFramePayload::PairFinish(request) => request,
            _ => return Err("expected Link pairing finish".to_string()),
        };
        self.pairFinishFromRequest(channel, finish).await
    }

    /// Completes a pending pairing from a PairFinish request.
    ///
    /// PairStart and PairFinish normally share one carrier, but the CLI is
    /// intentionally stateless between commands. Accepting PairFinish on a
    /// fresh carrier lets `pair-start` and `pair-finish` run in separate
    /// processes while retaining the same pending transaction on the Edge.
    pub async fn pairFinishFromRequest(
        &self,
        channel: Arc<dyn LinkChannel>,
        finish: LinkPairFinishRequest,
    ) -> Result<EdgeSession, String> {
        let pending = self
            .pending
            .lock()
            .map_err(|error| error.to_string())?
            .remove(&finish.pairingId)
            .ok_or_else(|| "Edge pairing transaction not found".to_string())?;
        if pending.pairingCode != finish.pairingCode.trim() {
            return Err("invalid Edge pairing code".to_string());
        }
        let expectedProof = proof(
            &pending.sharedSecret,
            &pending.clientNonce,
            &pending.serverNonce,
            "client",
        );
        if expectedProof != finish.clientProof {
            return Err("invalid Edge client proof".to_string());
        }
        let session = EdgeSession {
            sessionId: finish.pairingId.clone(),
            deviceId: self.deviceId.clone(),
            peerDeviceId: pending.clientDeviceId,
            sessionSecret: sessionSecret(
                &pending.sharedSecret,
                &pending.clientNonce,
                &pending.serverNonce,
            ),
        };
        self.sessions
            .lock()
            .map_err(|error| error.to_string())?
            .insert(session.sessionId.clone(), session.clone());
        if let Err(error) = self.persistSessions() {
            self.sessions
                .lock()
                .map_err(|lockError| lockError.to_string())?
                .remove(&session.sessionId);
            return Err(format!("persist Edge session: {error}"));
        }
        // The one-time code is no longer valid once the session is persisted.
        (self.onPairingCode)(String::new());
        channel
            .send(LinkFrame {
                messageId: "pair-finish-response".to_string(),
                payload: LinkFramePayload::PairFinishResponse(LinkPairFinishResponse {
                    sessionId: session.sessionId.clone(),
                    pairingServiceVersion: EDGE_PAIRING_SERVICE_VERSION,
                    coreProof: proof(
                        &pending.sharedSecret,
                        &pending.clientNonce,
                        &pending.serverNonce,
                        "core",
                    ),
                }),
            })
            .await?;
        Ok(session)
    }
    /// Removes all persisted pairing sessions and pending transactions.
    ///
    /// This does not erase the device token, Wi-Fi settings, or identity key.
    pub fn clearPairings(&self) -> Result<(), String> {
        self.pending
            .lock()
            .map_err(|error| error.to_string())?
            .clear();
        self.sessions
            .lock()
            .map_err(|error| error.to_string())?
            .clear();
        self.persistSessions()
    }

    /// Validates the first authenticated frame of a reconnecting Core and
    /// returns the stored session plus its decoded inner Link frame.
    pub fn authenticateFrame(&self, frame: &LinkFrame) -> Result<(EdgeSession, LinkFrame), String> {
        let LinkFramePayload::Authenticated {
            sessionId,
            deviceId,
            signature,
            payloadBytes,
        } = &frame.payload
        else {
            return Err("expected authenticated Edge Link frame".to_string());
        };
        let session = self
            .sessions
            .lock()
            .map_err(|error| error.to_string())?
            .get(sessionId)
            .cloned()
            .ok_or_else(|| "Edge Link session is not known".to_string())?;
        if deviceId != &session.peerDeviceId {
            return Err("Edge Link device id mismatch".to_string());
        }
        if signSession(&session.sessionSecret, payloadBytes) != *signature {
            return Err("Edge Link signature mismatch".to_string());
        }
        let inner = operit_link::decodeLink(payloadBytes).map_err(|error| error.to_string())?;
        Ok((session, inner))
    }
}

/// Starts the client side of the existing Link pairing exchange.
pub async fn startPairAsClient(
    channel: Arc<dyn LinkChannel>,
    tokenHash: String,
    deviceId: String,
    deviceInfo: LinkDeviceInfo,
) -> Result<EdgePairStartState, String> {
    let clientSecret = StaticSecret::random_from_rng(OsRng);
    let clientNonce = Uuid::new_v4().to_string();
    channel
        .send(LinkFrame {
            messageId: "pair-start".to_string(),
            payload: LinkFramePayload::PairStart(LinkPairStartRequest {
                pairingServiceVersion: EDGE_PAIRING_SERVICE_VERSION,
                tokenHash,
                clientDeviceId: deviceId.clone(),
                clientDeviceInfo: deviceInfo,
                clientPublicKey: publicKeyString(&PublicKey::from(&clientSecret)),
                clientNonce: clientNonce.clone(),
            }),
        })
        .await?;
    let response = match channel
        .receive()
        .await?
        .ok_or_else(|| "Edge pairing channel closed after start".to_string())?
        .payload
    {
        LinkFramePayload::PairStartResponse(response) => response,
        _ => return Err("expected Edge pairing start response".to_string()),
    };
    let edgeDeviceId = response.edgeDeviceId.clone();
    let edgeDeviceInfo = response.edgeDeviceInfo.clone();
    let edgePublic = parsePublicKey(&response.edgePublicKey)?;
    let serverNonce = response.serverNonce.clone();
    let sharedSecret = clientSecret.diffie_hellman(&edgePublic).as_bytes().to_vec();
    Ok(EdgePairStartState {
        pairingId: response.pairingId,
        clientDeviceId: deviceId,
        edgeDeviceId,
        edgeDeviceInfo,
        clientNonce,
        serverNonce,
        sharedSecret,
    })
}

/// Completes client pairing after the user has entered the displayed code.
pub async fn finishPairAsClient(
    channel: Arc<dyn LinkChannel>,
    state: EdgePairStartState,
    pairingCode: String,
) -> Result<EdgeSession, String> {
    let clientProof = proof(
        &state.sharedSecret,
        &state.clientNonce,
        &state.serverNonce,
        "client",
    );
    channel
        .send(LinkFrame {
            messageId: "pair-finish".to_string(),
            payload: LinkFramePayload::PairFinish(LinkPairFinishRequest {
                pairingId: state.pairingId.clone(),
                pairingCode,
                clientProof,
            }),
        })
        .await?;
    let finish = match channel
        .receive()
        .await?
        .ok_or_else(|| "Edge pairing channel closed after finish".to_string())?
        .payload
    {
        LinkFramePayload::PairFinishResponse(response) => response,
        _ => return Err("expected Edge pairing finish response".to_string()),
    };
    let expectedCoreProof = proof(
        &state.sharedSecret,
        &state.clientNonce,
        &state.serverNonce,
        "core",
    );
    let expectedCoreProof = expectedCoreProof;
    // The current wire response has no separate nonce field; proof validation
    // remains tied to the nonce received in PairStartResponse.
    if finish.coreProof != expectedCoreProof {
        return Err("invalid Edge core proof".to_string());
    }
    Ok(EdgeSession {
        sessionId: finish.sessionId,
        deviceId: state.clientDeviceId,
        peerDeviceId: state.edgeDeviceId,
        sessionSecret: sessionSecret(&state.sharedSecret, &state.clientNonce, &state.serverNonce),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use operit_link::LinkFrame;
    use tokio::sync::{mpsc, Mutex as AsyncMutex};

    struct MemoryChannel {
        sender: mpsc::UnboundedSender<LinkFrame>,
        receiver: AsyncMutex<mpsc::UnboundedReceiver<LinkFrame>>,
    }

    #[async_trait]
    impl LinkChannel for MemoryChannel {
        async fn send(&self, frame: LinkFrame) -> Result<(), String> {
            self.sender.send(frame).map_err(|error| error.to_string())
        }

        async fn receive(&self) -> Result<Option<LinkFrame>, String> {
            Ok(self.receiver.lock().await.recv().await)
        }

        async fn close(&self) {}
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn completesExistingPairingExchange() {
        let (clientSender, authorityReceiver) = mpsc::unbounded_channel();
        let (authoritySender, clientReceiver) = mpsc::unbounded_channel();
        let clientChannel = Arc::new(MemoryChannel {
            sender: clientSender,
            receiver: AsyncMutex::new(clientReceiver),
        });
        let authorityChannel = Arc::new(MemoryChannel {
            sender: authoritySender,
            receiver: AsyncMutex::new(authorityReceiver),
        });
        let displayedCode = Arc::new(Mutex::new(None));
        let displayedCodeForCallback = Arc::clone(&displayedCode);
        let authority = Arc::new(EdgePairingAuthority::new(
            "edge-token",
            "edge-1",
            LinkDeviceInfo {
                platform: "esp32".to_string(),
                model: "test".to_string(),
            },
            move |code| {
                *displayedCodeForCallback.lock().unwrap() = Some(code);
            },
        ));
        let authorityTask = {
            let authority = Arc::clone(&authority);
            let authorityChannel = Arc::clone(&authorityChannel);
            tokio::spawn(async move { authority.pair(authorityChannel).await.unwrap() })
        };
        let start = startPairAsClient(
            clientChannel.clone(),
            linkTokenHash("edge-token"),
            "core-1".to_string(),
            LinkDeviceInfo {
                platform: "windows".to_string(),
                model: "test".to_string(),
            },
        )
        .await
        .unwrap();
        let code = displayedCode.lock().unwrap().clone().unwrap();
        let clientSession = finishPairAsClient(clientChannel, start, code)
            .await
            .unwrap();
        let authoritySession = authorityTask.await.unwrap();
        assert_eq!(clientSession.sessionId, authoritySession.sessionId);
        assert_eq!(clientSession.sessionSecret, authoritySession.sessionSecret);
    }
}
