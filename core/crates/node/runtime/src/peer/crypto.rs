//! Runtime 的密钥交换和 AEAD；PeerLink 始终只看到标准 Link Call。
use operit_link::*;
use operit_peer_link::{PeerConnection, PeerMessage};
use rand_core::{CryptoRng, RngCore};
use ring::{aead, agreement, digest, hkdf, hmac, rand::{SecureRandom, SystemRandom}};
use serde::{Deserialize, Serialize};
use spake2::{Ed25519Group, Identity, Password, Spake2};
use std::collections::BTreeMap;
use std::sync::{atomic::Ordering, Arc, Mutex as StdMutex};
use tokio::sync::{mpsc, oneshot, Mutex};

#[path = "encoded_message.rs"]
mod encoded_message;
pub(super) use encoded_message::EncodedMessage;
use super::heartbeat::{Heartbeat, HeartbeatAction};
use futures_util::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};

const CONTROL_TARGET: &str = "$peer.channel";
const HEARTBEAT_TICK_MS: u64 = 1_000;

/// Creates a protocol security error at the authenticated channel boundary.
pub(super) fn error(message: impl Into<String>) -> CoreLinkError {
    CoreLinkError::new("PEER_SECURITY", message)
}
pub(super) fn random() -> Result<[u8; 32], CoreLinkError> {
    let mut bytes = [0; 32];
    SystemRandom::new().fill(&mut bytes).map_err(|_| error("Random source failed"))?;
    Ok(bytes)
}
pub(super) fn ephemeral() -> Result<(agreement::EphemeralPrivateKey, Vec<u8>), CoreLinkError> {
    let key = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &SystemRandom::new())
        .map_err(|_| error("X25519 key generation failed"))?;
    let public = key.compute_public_key().map_err(|_| error("X25519 public key failed"))?;
    Ok((key, public.as_ref().to_vec()))
}
pub(super) fn agree(key: agreement::EphemeralPrivateKey, public: &[u8]) -> Result<Vec<u8>, CoreLinkError> {
    agreement::agree_ephemeral(key, &agreement::UnparsedPublicKey::new(&agreement::X25519, public), |bytes| bytes.to_vec())
        .map_err(|_| error("Invalid X25519 public key"))
}
struct KeyLength;
impl hkdf::KeyType for KeyLength { fn len(&self) -> usize { 32 } }
pub(super) fn derive(secret: &[u8], salt: &[u8], label: &[u8]) -> Result<[u8; 32], CoreLinkError> {
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, salt).extract(secret);
    let labels = [label];
    let mut key = [0; 32];
    prk.expand(&labels, KeyLength).map_err(|_| error("HKDF expansion failed"))?
        .fill(&mut key).map_err(|_| error("HKDF failed"))?;
    Ok(key)
}
pub(super) fn proof(key: &[u8], context: &[u8], role: &[u8]) -> Vec<u8> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, key);
    let mut message = context.to_vec(); message.extend_from_slice(role);
    hmac::sign(&key, &message).as_ref().to_vec()
}
pub(super) fn verify(key: &[u8], context: &[u8], role: &[u8], supplied: &[u8]) -> Result<(), CoreLinkError> {
    let mut message = context.to_vec(); message.extend_from_slice(role);
    hmac::verify(&hmac::Key::new(hmac::HMAC_SHA256, key), &message, supplied)
        .map_err(|_| error("Key proof rejected"))
}
/// A receiver-only, fixed-width code; leading zeroes are significant.
pub(super) fn pairingCode() -> Result<String, CoreLinkError> {
    // Reject the non-uniform tail instead of introducing a modulo bias in the
    // only secret an online attacker gets to guess.
    loop {
        let value = u32::from_be_bytes(random()?[..4].try_into().unwrap());
        if value < 4_294_000_000 {
            return Ok(format!("{:06}", value % 1_000_000));
        }
    }
}

/// Supplies SPAKE2 with entropy derived from the Host-backed ring source.
///
/// SPAKE2 asks its RNG for one 64-byte scalar seed and cannot report an entropy
/// failure from `fill_bytes`. Seeding an HMAC-SHA256 DRBG here keeps every
/// platform (including ESP32 and WASM) on the same random source, turns an
/// entropy failure into an ordinary error, and removes any length ceiling a
/// fixed seed would impose on the caller.
struct SeededRng {
    key: [u8; 32],
    counter: u64,
    block: [u8; 32],
    offset: usize,
}
impl SeededRng {
    fn new() -> Result<Self, CoreLinkError> {
        Ok(Self { key: random()?, counter: 0, block: [0; 32], offset: 32 })
    }
    fn refill(&mut self) {
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.key);
        let message = self.counter.to_be_bytes();
        self.block.copy_from_slice(hmac::sign(&key, &message).as_ref());
        self.counter = self.counter.wrapping_add(1);
        self.offset = 0;
    }
}
impl RngCore for SeededRng {
    fn next_u32(&mut self) -> u32 {
        let mut bytes = [0; 4];
        self.fill_bytes(&mut bytes);
        u32::from_le_bytes(bytes)
    }
    fn next_u64(&mut self) -> u64 {
        let mut bytes = [0; 8];
        self.fill_bytes(&mut bytes);
        u64::from_le_bytes(bytes)
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for byte in dest {
            if self.offset == self.block.len() { self.refill(); }
            *byte = self.block[self.offset];
            self.offset += 1;
        }
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}
impl CryptoRng for SeededRng {}

/// Binds one SPAKE2 exchange to the exact X25519 transcript.
///
/// Both identities include the transcript hash, so messages cannot be replayed
/// into a different session or spliced between pairing attempts.
pub(super) fn pairingIdentities(context: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut client = b"operit-pairing-client-v2".to_vec();
    client.extend_from_slice(context);
    let mut server = b"operit-pairing-server-v2".to_vec();
    server.extend_from_slice(context);
    (client, server)
}

/// Starts one SPAKE2 exchange for the pairing confirmation code.
///
/// The code never appears on the wire and is never used as a MAC message. An
/// active man in the middle that saw the earlier X25519 exchange can therefore
/// only test one guessed code by completing a full online exchange, which the
/// per-pairing attempt budget bounds.
pub(super) fn spakeStart(
    client: bool,
    code: &[u8],
    clientId: &[u8],
    serverId: &[u8],
) -> Result<(Spake2<Ed25519Group>, Vec<u8>), CoreLinkError> {
    let password = Password::new(code);
    let idA = Identity::new(clientId);
    let idB = Identity::new(serverId);
    let rng = SeededRng::new()?;
    Ok(if client {
        Spake2::start_a_with_rng(&password, &idA, &idB, rng)
    } else {
        Spake2::start_b_with_rng(&password, &idA, &idB, rng)
    })
}

/// Completes SPAKE2 and returns the raw password-authenticated key.
///
/// A wrong code still produces a key, so callers must prove possession of the
/// derived session secret before committing any credential.
pub(super) fn spakeFinish(
    state: Spake2<Ed25519Group>,
    peerMessage: &[u8],
) -> Result<Vec<u8>, CoreLinkError> {
    state
        .finish(peerMessage)
        .map_err(|_| error("Pairing confirmation message rejected"))
}

/// Derives the long-lived session secret from the PAKE output.
///
/// The start-phase root and the finish transcript are both included so the
/// secret is bound to the exact pairing transaction. The X25519 root alone is
/// not a credential: an active man in the middle knows it, but does not know the
/// SPAKE2 output without guessing the code online.
pub(super) fn pairingSessionSecret(
    spakeKey: &[u8],
    startRoot: &[u8],
    context: &[u8],
) -> Result<[u8; 32], CoreLinkError> {
    let mut salt = Vec::with_capacity(startRoot.len() + context.len());
    salt.extend_from_slice(startRoot);
    salt.extend_from_slice(context);
    derive(spakeKey, &salt, b"operit-pairing-session-v2")
}

#[cfg(test)]
mod pairing_code_tests {
    #[test]
    fn receiver_code_is_six_ascii_digits() {
        for _ in 0..100 {
            let code = super::pairingCode().unwrap();
            assert_eq!(code.len(), 6);
            assert!(code.bytes().all(|b| b.is_ascii_digit()));
        }
    }
}

#[cfg(test)]
mod spake_tests {
    use super::*;

    /// Both sides of a matching code derive the same key even though the code
    /// never appears on the wire.
    #[test]
    fn matching_codes_agree() {
        let (client, clientMessage) = spakeStart(true, b"123456", b"client", b"server").unwrap();
        let (server, serverMessage) = spakeStart(false, b"123456", b"client", b"server").unwrap();
        let clientKey = spakeFinish(client, &serverMessage).unwrap();
        let serverKey = spakeFinish(server, &clientMessage).unwrap();
        assert_eq!(clientKey, serverKey);
    }

    /// A wrong guess still completes the PAKE but cannot produce the peer's key,
    /// so every guess costs a full online exchange.
    #[test]
    fn mismatched_codes_diverge() {
        let (client, clientMessage) = spakeStart(true, b"123456", b"client", b"server").unwrap();
        let (server, serverMessage) = spakeStart(false, b"654321", b"client", b"server").unwrap();
        let clientKey = spakeFinish(client, &serverMessage).unwrap();
        let serverKey = spakeFinish(server, &clientMessage).unwrap();
        assert_ne!(clientKey, serverKey);
        let context = b"context";
        assert_ne!(
            pairingSessionSecret(&clientKey, b"root", context).unwrap(),
            pairingSessionSecret(&serverKey, b"root", context).unwrap(),
        );
    }

    /// The stored credential is bound to the pairing transcript and start root,
    /// so a recorded PAKE message cannot be replayed into another session.
    #[test]
    fn session_secret_is_bound_to_transcript_and_root() {
        let key = [7u8; 32];
        let first = pairingSessionSecret(&key, b"root-a", b"context-a").unwrap();
        let second = pairingSessionSecret(&key, b"root-b", b"context-a").unwrap();
        let third = pairingSessionSecret(&key, b"root-a", b"context-b").unwrap();
        assert_ne!(first, second);
        assert_ne!(first, third);
    }
}
/// 整个 transcript 编码绑定版本、双方身份、两份公钥和服务端随机挑战。
#[derive(Serialize, Deserialize)]
pub(super) struct Transcript {
    pub version: u32, pub sessionId: String,
    pub clientNodeId: String, pub serverNodeId: String,
    pub clientPublic: Vec<u8>, pub serverPublic: Vec<u8>, pub challenge: Vec<u8>,
}
pub(super) fn transcript(value: &Transcript) -> Result<Vec<u8>, CoreLinkError> {
    let bytes = encodeLink(value).map_err(|e| error(e.to_string()))?;
    Ok(digest::digest(&digest::SHA256, &bytes).as_ref().to_vec())
}
struct Cipher { key: aead::LessSafeKey, next: u64, context: Vec<u8> }
impl Cipher {
    fn new(key: [u8; 32], context: &[u8]) -> Result<Self, CoreLinkError> {
        Ok(Self { key: aead::LessSafeKey::new(aead::UnboundKey::new(&aead::CHACHA20_POLY1305, &key)
            .map_err(|_| error("AEAD initialization failed"))?), next: 0, context: context.to_vec() })
    }
    // Keep the nonce and AEAD arguments in one synchronous frame. In the
    // Xtensa async send frame, optimized nonce temporaries produced invalid
    // ciphertext; diagnostic spills masked it. This boundary is also smaller
    // than retaining the crypto call's temporaries in the connection future.
    #[inline(never)]
    fn seal(&mut self, bytes: &mut Vec<u8>) -> Result<(), CoreLinkError> {
        let nonce = self.nonce()?;
        self.key.seal_in_place_append_tag(nonce, aead::Aad::from(&self.context), bytes)
            .map_err(|_| error("Encryption failed"))
    }
    fn nonce(&mut self) -> Result<aead::Nonce, CoreLinkError> {
        let next = self.next; self.next = next.checked_add(1).ok_or_else(|| error("Session nonce exhausted"))?;
        let mut nonce = [0; 12]; nonce[4..].copy_from_slice(&next.to_be_bytes());
        Ok(aead::Nonce::assume_unique_for_key(nonce))
    }
}
/// 两方向独立密钥、单调 nonce。收到明文、乱序、重放或错误 tag 一律断开。
static CHANNEL_GENERATION: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

pub(super) struct Channel {
    pub generation: u32,
    pub raw: Arc<dyn PeerConnection>,
    send: Mutex<Cipher>, receive: Mutex<Cipher>,
}
impl Channel {
    pub fn new(raw: Arc<dyn PeerConnection>, key: &[u8], context: &[u8], client: bool) -> Result<Arc<Self>, CoreLinkError> {
        let c2s = derive(key, context, b"operit-link-client-to-server-v1")?;
        let s2c = derive(key, context, b"operit-link-server-to-client-v1")?;
        Ok(Arc::new(Self { generation: CHANNEL_GENERATION.fetch_add(1, Ordering::Relaxed), raw, send: Mutex::new(Cipher::new(if client { c2s } else { s2c }, context)?),
            receive: Mutex::new(Cipher::new(if client { s2c } else { c2s }, context)?) }))
    }
    /// Sends a single frame through the shared ordered writer.
    pub async fn send(&self, message: PeerMessage) -> Result<(), CoreLinkError> { self.sendBatch(vec![message]).await }

    /// Holds the authenticated writer for an entire burst so other operations cannot interleave it.
    pub async fn sendBatch(&self, messages: Vec<PeerMessage>) -> Result<(), CoreLinkError> {
        let mut cipher = self.send.lock().await;
        for message in messages {
            let bytes = encodeLink(message).map_err(|e| error(e.to_string()))?;
            self.sendEncoded(&mut cipher, bytes).await?;
        }
        Ok(())
    }
    /// Preserves the same nonce order and indivisible burst for prepared replies.
    pub(super) async fn sendEncodedBatch(&self, messages: Vec<Vec<u8>>) -> Result<(), CoreLinkError> {
        let mut cipher = self.send.lock().await;
        for bytes in messages { self.sendEncoded(&mut cipher, bytes).await?; }
        Ok(())
    }
    async fn sendEncoded(&self, cipher: &mut Cipher, mut bytes: Vec<u8>) -> Result<(), CoreLinkError> {
        let sequence = cipher.next;
        cipher.seal(&mut bytes)?;
        self.raw.send(PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new(
            sequence.to_string(), "$peer.session", "data", CoreValue::Bytes(bytes.into()))))).await.map_err(|message| {
            if message == operit_peer_link::MESSAGE_TOO_LARGE_ERROR {
                CoreLinkError::new("PEER_MESSAGE_TOO_LARGE", message)
            } else { error(message) }
        })
    }
    /// Closes the authenticated carrier without retaining another session implementation.
    pub async fn close(&self) { self.raw.close().await; }

    /// Receives the next authenticated frame in nonce order.
    pub async fn receive(&self) -> Result<Option<PeerMessage>, CoreLinkError> {
        self.receiveEncoded().await?
            .map(|message| message.decodeWith(PeerMessage::decode)).transpose()
    }

    /// Retains each decrypted frame until its dispatcher or consumer needs the typed payload.
    pub(super) async fn receiveEncoded(&self) -> Result<Option<EncodedMessage<PeerMessage>>, CoreLinkError> {
        let mut cipher = self.receive.lock().await;
        let Some(message) = self.raw.receive().await.map_err(error)? else { return Ok(None); };
        let PeerMessage::Request(CoreLinkRequest::Call(request)) = message else { return Err(error("Encrypted Call required")); };
        if request.target != "$peer.session" || request.methodName != "data" || request.requestId.0 != cipher.next.to_string() {
            return Err(error(format!("Unexpected/replayed session message: expected={}, received={:?}", cipher.next, request.requestId.0.parse::<u64>().ok())));
        }
        let CoreValue::Bytes(bytes) = request.args else { return Err(error("Encrypted bytes required")); };
        // Reuse uniquely owned ciphertext. Shared buffers from Host adapters
        // still copy before mutation, preserving every other reader's bytes.
        let mut bytes = Vec::from(bytes);
        let nonce = cipher.nonce()?;
        let plaintext = cipher.key.open_in_place(nonce, aead::Aad::from(&cipher.context), &mut bytes)
            .map_err(|_| error("Session authentication tag rejected"))?;
        let length = plaintext.len();
        bytes.truncate(length);
        Ok(Some(EncodedMessage::fromBytes(bytes)))
    }
}


/// Multiplexes correlated Call, Watch, and Push responses over one authenticated channel.
pub(super) struct MultiplexedChannel {
    scheduler: Arc<dyn operit_host_api::HostRuntimeTaskSchedulerHost>,
    channel: Arc<Channel>,
    pending: StdMutex<
        BTreeMap<String, oneshot::Sender<Result<EncodedMessage<CoreLinkResponse>, CoreLinkError>>>,
    >,
    watches: StdMutex<BTreeMap<String, mpsc::UnboundedSender<EncodedMessage<CoreEvent>>>>,
    failure: StdMutex<Option<CoreLinkError>>,
    changed: Box<dyn Fn() + Send + Sync>,
    shutdown: tokio::sync::Notify,
    /// Routes peer requests independently while the shared reader correlates responses.
    inboundRequests: StdMutex<Option<mpsc::UnboundedSender<EncodedMessage<PeerMessage>>>>,
    /// Signals that this channel's reader has stopped, for the accept path that
    /// closes the connection as soon as its serve call returns.
    stopped: tokio::sync::Notify,
    /// Cancels inbound sessions when the authenticated carrier ends.
    dispatchStopped: tokio::sync::Notify,
}

static LAST_ERROR: StdMutex<Option<String>> = StdMutex::new(None);

/// Reports the first terminal failure from the shared channel implementation.
pub(super) fn diagnostic() -> Option<String> { LAST_ERROR.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone() }

impl MultiplexedChannel {
    /// Starts the common response reader and inbound dispatcher for both directions.
    pub(super) fn start(
        channel: Arc<Channel>,
        scheduler: Arc<dyn operit_host_api::HostRuntimeTaskSchedulerHost>,
        inbound: super::dispatch::InboundSide,
        changed: impl Fn() + Send + Sync + 'static,
    ) -> Result<Arc<Self>, CoreLinkError> {
        let now = scheduler
            .monotonicTimeMillis()
            .map_err(|failure| error(failure.to_string()))?;
        let (sender, requests) = mpsc::unbounded_channel();
        let multiplexed = Arc::new(Self {
            scheduler: scheduler.clone(),
            channel,
            pending: StdMutex::new(BTreeMap::new()),
            watches: StdMutex::new(BTreeMap::new()),
            failure: StdMutex::new(None),
            changed: Box::new(changed),
            shutdown: tokio::sync::Notify::new(),
            inboundRequests: StdMutex::new(Some(sender)),
            stopped: tokio::sync::Notify::new(),
            dispatchStopped: tokio::sync::Notify::new(),
        });
        let reader = multiplexed.clone();
        scheduler.scheduleHostRuntimeAsyncTask("peer-multiplexed-reader", Box::new(move || Box::pin(async move {
            if let Err(error) = crate::fault_isolation::link("peer channel reader", async {
                reader.clone().readLoop(now).await;
                Ok(())
            }).await {
                reader.fail(error).await;
                reader.stopped.notify_one();
                reader.channel.raw.close().await;
            }
        }))).map_err(|hostError| error(hostError.to_string()))?;
        let dispatch = multiplexed.clone();
        scheduler.scheduleHostRuntimeAsyncTask("peer-multiplexed-dispatch", Box::new(move || Box::pin(async move {
            if let Err(error) = crate::fault_isolation::link("peer channel dispatcher", async {
                dispatch.clone().dispatchLoop(inbound, requests).await;
                Ok(())
            }).await {
                dispatch.fail(error).await;
                dispatch.stopped.notify_one();
                dispatch.channel.raw.close().await;
            }
        }))).map_err(|hostError| error(hostError.to_string()))?;
        Ok(multiplexed)
    }

    /// Returns whether the underlying authenticated channel has failed.
    pub(super) fn isFailed(&self) -> bool { self.failure.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).is_some() }

    /// Identifies the authenticated carrier used by a Space client.
    pub(super) fn generation(&self) -> u32 {
        self.channel.generation
    }

    /// Reports the Host transport of this peer's sole carrier.
    pub(super) fn transport(&self) -> operit_peer_link::PeerTransport {
        self.channel.raw.transport()
    }

    /// Terminates the carrier and all of its logical streams together.
    pub(super) async fn close(&self) {
        self.fail(error("Authenticated Link channel closed")).await;
    }

    /// Sends one correlated Link request and waits for its matching response.
    pub(super) async fn exchange(
        &self,
        request: CoreLinkRequest,
    ) -> Result<CoreLinkResponse, CoreLinkError> {
        let key = requestKey(&request)?;
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if pending.contains_key(&key) {
                return Err(error("Link request correlation is already in flight"));
            }
            pending.insert(key.clone(), sender);
        }
        if let Err(error) = self.send(PeerMessage::Request(request)).await {
            self.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&key);
            return Err(error);
        }
        receiver
            .await
            .map_err(|_| error("Multiplexed response sender closed"))??
            .decodeWith(decodeLinkResponse)
    }

    /// Opens one Watch and routes its future events through the shared reader.
    pub(super) async fn openWatch(
        self: &Arc<Self>,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let requestId = request.requestId.0.clone();
        let (sender, receiver) = mpsc::unbounded_channel();
        self.watches
            .lock()
            .unwrap()
            .insert(requestId.clone(), sender);
        let response = self
            .exchange(CoreLinkRequest::Watch(CoreLinkWatchRequest::Open(request)))
            .await;
        match response {
            Ok(CoreLinkResponse::Watch {
                result: Ok(CoreLinkWatchResponse::Opened),
                ..
            }) => {
                let stream = CoreEventStream::fromMappedReceiver(
                    receiver,
                    |event: EncodedMessage<CoreEvent>| {
                        Some(
                            event
                                .decodeWith(decodeLink)
                                .expect("Locally encoded Watch event must decode"),
                        )
                    },
                )
                .withOnClose(self.watchCloseCallback(requestId.clone()));
                Ok(stream)
            }
            Ok(CoreLinkResponse::Watch {
                result: Err(error), ..
            }) => {
                self.watches.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&requestId);
                Err(error)
            }
            Ok(_) => {
                self.watches.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&requestId);
                Err(error("Watch open response mismatch"))
            }
            Err(error) => {
                self.watches.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&requestId);
                Err(error)
            }
        }
    }

    /// A stream can be dropped outside any Tokio executor, including a browser turn.
    fn watchCloseCallback(self: &Arc<Self>, requestId: String) -> impl FnOnce() + Send + 'static {
        let channel = self.clone();
        let scheduler = self.scheduler.clone();
        move || {
            let closing = channel.clone();
            if let Err(hostError) = scheduler.scheduleHostRuntimeAsyncTask(
                "peer-multiplexed-watch-close",
                Box::new(move || {
                    Box::pin(async move {
                        closing.closeWatch(requestId).await;
                    })
                }),
            ) {
                channel.terminate(error(hostError.to_string()));
            }
        }
    }

    /// Closes one Watch without closing the shared authenticated channel.
    async fn closeWatch(&self, requestId: String) {
        self.watches.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&requestId);
        let _ = self
            .exchange(CoreLinkRequest::Watch(CoreLinkWatchRequest::Close {
                requestId: CoreRequestId::new(requestId),
            }))
            .await;
    }

    /// Publishes terminal state and releases every logical operation without suspension.
    fn terminate(&self, error: CoreLinkError) {
        {
            let mut failure = self.failure.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if failure.is_some() {
                return;
            }
            *failure = Some(error.clone());
        }
        *LAST_ERROR.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(error.message.clone());
        let drained = std::mem::take(&mut *self.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
        for (_, sender) in drained {
            let _ = sender.send(Err(error.clone()));
        }
        self.watches.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
        self.inboundRequests.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
        self.dispatchStopped.notify_one();
        self.shutdown.notify_one();
        (self.changed)();
    }

    /// Returns the terminal channel error or a new local error.
    fn failureOr(&self, message: &str) -> CoreLinkError {
        self.failure.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone().unwrap_or_else(|| error(message))
    }

    /// Releases local state before awaiting transport shutdown.
    async fn fail(&self, failure: CoreLinkError) {
        self.terminate(failure);
        self.channel.raw.close().await;
    }

    /// Sends one authenticated frame and terminates all logical operations on write failure.
    async fn send(&self, message: PeerMessage) -> Result<(), CoreLinkError> {
        if let Some(failure) = self.failure.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone() {
            return Err(failure);
        }
        let result = self.channel.send(message).await;
        if let Err(failure) = &result {
            self.fail(failure.clone()).await;
        }
        result
    }

    /// Sends an ordered encoded reply batch through the same authenticated writer.
    async fn sendEncodedBatch(&self, messages: Vec<Vec<u8>>) -> Result<(), CoreLinkError> {
        if let Some(failure) = self.failure.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone() {
            return Err(failure);
        }
        let result = self.channel.sendEncodedBatch(messages).await;
        if let Err(failure) = &result {
            self.fail(failure.clone()).await;
        }
        result
    }

    /// Builds heartbeat controls in the unchanged authenticated Link envelope.
    fn control(id: String, method: &str) -> PeerMessage {
        PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new(
            id,
            CONTROL_TARGET,
            method,
            CoreValue::Null,
        )))
    }

    /// Handles heartbeats and response correlation in the sole authenticated reader.
    async fn readLoop(self: Arc<Self>, now: u64) {
        let mut heartbeat = Heartbeat::new(now);
        let mut writes =
            FuturesUnordered::<LocalBoxFuture<'static, Result<(), CoreLinkError>>>::new();
        let mut tick = self.scheduler.waitForHostRuntimeDelay(HEARTBEAT_TICK_MS);
        let result: Result<(), CoreLinkError> = async {
            loop {
                tokio::select! {
                    _ = self.shutdown.notified() => return Ok(()),
                    message = self.channel.receiveEncoded() => {
                        let message = message?.ok_or_else(|| error("Authenticated channel closed"))?;
                        let now = self.scheduler.monotonicTimeMillis().map_err(|failure| error(failure.to_string()))?;
                        heartbeat.received(now);
                        let header = message.callHeader()?;
                        if header.as_ref().is_some_and(|header| header.control) {
                            let PeerMessage::Request(CoreLinkRequest::Call(request)) = message.decodeWith(PeerMessage::decode)? else {
                                return Err(error("Invalid control envelope"));
                            };
                            if request.args != CoreValue::Null { return Err(error("Invalid channel control payload")); }
                            match request.methodName.as_str() {
                                "ping" => {
                                    let channel = self.clone();
                                    writes.push(Box::pin(async move { channel.send(Self::control(request.requestId.0, "pong")).await }));
                                },
                                "pong" => heartbeat.pong(&request.requestId.0, now),
                                _ => return Err(error("Unknown channel control operation")),
                            }
                        } else {
                            self.routeIncoming(message, header).await?;
                        }
                    },
                    result = &mut tick => {
                        result.map_err(|failure| error(failure.to_string()))?;
                        tick = self.scheduler.waitForHostRuntimeDelay(HEARTBEAT_TICK_MS);
                        let now = self.scheduler.monotonicTimeMillis().map_err(|failure| error(failure.to_string()))?;
                        match heartbeat.tick(now) {
                            HeartbeatAction::Ping(id) => {
                                let channel = self.clone();
                                writes.push(Box::pin(async move { channel.send(Self::control(id, "ping")).await }));
                            },
                            HeartbeatAction::Expired => return Err(CoreLinkError::new("PEER_HEARTBEAT_TIMEOUT", "Authenticated channel remained silent through both heartbeat deadlines")),
                            HeartbeatAction::Suspect | HeartbeatAction::None => {},
                        }
                    },
                    Some(result) = writes.next(), if !writes.is_empty() => result?,
                }
            }
        }.await;
        match result {
            Err(failure) => self.fail(failure).await,
            Ok(()) => self.channel.raw.close().await,
        }
        self.stopped.notify_one();
    }

    /// Resolves once this channel's reader has stopped, so the side that accepted
    /// the connection keeps it open for as long as the channel serves it.
    pub(super) async fn waitStopped(&self) {
        self.stopped.notified().await;
    }

    /// Hands one request from the peer to this channel's dispatcher, keeping this
    /// reader free to correlate the calls made here.
    fn forwardInbound(&self, request: EncodedMessage<PeerMessage>) -> Result<(), CoreLinkError> {
        let sender = self
            .inboundRequests
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| error("Multiplexed channel received an unexpected request"))?;
        sender
            .send(request)
            .map_err(|_| error("Multiplexed channel dispatcher stopped"))
    }

    /// Routes the already inspected frame without a second queue or header decode.
    async fn routeIncoming(
        &self,
        message: EncodedMessage<PeerMessage>,
        header: Option<encoded_message::CallHeader>,
    ) -> Result<(), CoreLinkError> {
        if let Some(header) = header {
            if header.request {
                return self.forwardInbound(message);
            }
            if let Some(sender) = self
                .pending
                .lock()
                .unwrap()
                .remove(&format!("call:{}", header.requestId.0))
            {
                let _ = sender.send(Ok(EncodedMessage::fromBytes(message.intoBytes())));
            }
            return Ok(());
        }
        if !message.isResponse()? {
            return self.forwardInbound(message);
        }
        let PeerMessage::Response(response) = message.decodeWith(PeerMessage::decode)? else {
            return Err(error("Expected Link response"));
        };
        self.dispatchResponse(response).await
    }

    /// Answers the peer's calls while this reader keeps the other direction
    /// flowing, and flushes the watch events this side owes it.
    async fn dispatchLoop(
        self: Arc<Self>,
        side: super::dispatch::InboundSide,
        requests: mpsc::UnboundedReceiver<EncodedMessage<PeerMessage>>,
    ) {
        // Dropping InboundSide also terminates its open Watch/Push sessions.
        // A dispatcher may be awaiting a backpressured Push item when the reader
        // ends, so closing only the request queue is not sufficient.
        tokio::select! {
            _ = self.dispatchStopped.notified() => {},
            result = super::dispatch::serveIncoming(
                side,
                futures_util::stream::unfold(requests, |mut requests| async move {
                    requests.recv().await.map(|request| (Ok(request), requests))
                }),
                |messages| self.sendEncodedBatch(messages),
            ) => if let Err(error) = result {
                self.fail(error).await;
            },
        }
    }

    /// Moves ordered Watch events into compact storage and completes pending operations.
    async fn dispatchResponse(&self, response: CoreLinkResponse) -> Result<(), CoreLinkError> {
        let response = match response {
            CoreLinkResponse::Watch {
                requestId,
                result: Ok(CoreLinkWatchResponse::Event(event)),
            } => {
                if let Some(sender) = self.watches.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get(&requestId.0) {
                    let _ = sender.send(EncodedMessage::new(event)?);
                    return Ok(());
                }
                CoreLinkResponse::Watch {
                    requestId,
                    result: Ok(CoreLinkWatchResponse::Event(event)),
                }
            }
            response => response,
        };
        if let CoreLinkResponse::Watch {
            requestId,
            result: Ok(CoreLinkWatchResponse::Closed),
        } = &response
        {
            self.watches.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&requestId.0);
        }
        if let Some(sender) = self.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&responseKey(&response)) {
            let _ = sender.send(EncodedMessage::new(response));
        }
        Ok(())
    }
}

/// Builds a stable pending-response key for one Link request.
fn requestKey(request: &CoreLinkRequest) -> Result<String, CoreLinkError> {
    Ok(match request {
        CoreLinkRequest::Call(request) => format!("call:{}", request.requestId.0),
        CoreLinkRequest::Watch(
            CoreLinkWatchRequest::Snapshot(request) | CoreLinkWatchRequest::Open(request),
        ) => format!("watch:{}", request.requestId.0),
        CoreLinkRequest::Watch(CoreLinkWatchRequest::Close { requestId }) => {
            format!("watch:{}", requestId.0)
        }
        CoreLinkRequest::Push(CoreLinkPushRequestMessage::Open(request)) => {
            format!("push:{}", request.requestId.0)
        }
        CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(item)) => {
            format!("push:{}", item.pushId)
        }
        CoreLinkRequest::Push(CoreLinkPushRequestMessage::Close { pushId }) => {
            format!("push:{}", pushId)
        }
    })
}

/// Builds a stable pending-response key for one Link response.
fn responseKey(response: &CoreLinkResponse) -> String {
    match response {
        CoreLinkResponse::Call(response) => format!("call:{}", response.requestId.0),
        CoreLinkResponse::Watch { requestId, .. } => format!("watch:{}", requestId.0),
        CoreLinkResponse::Push { pushId, .. } => format!("push:{pushId}"),
    }
}

#[cfg(test)]
mod aead_interop_tests {
    use super::*;

    #[test]
    fn sealed_frames_match_external_chacha20_poly1305_vectors() {
        let key = std::array::from_fn(|i| i as u8);
        let mut cipher = Cipher::new(key, b"operit-link-context").unwrap();
        for expected in [
            "77c82743c4928bbd7a0f374cce262f43d5c3969293c8282f8597921c13035707b7e778865f7946bd138c30",
            "062c19ab587e9516441f1502d27176763d835a27bcd48f062c55dbdf63a7d22a45973c8f8f64178815fece",
        ] {
            let mut bytes = b"operit-link-aead-regression".to_vec();
            cipher.seal(&mut bytes).unwrap();
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(hex, expected);
        }
        assert_eq!(cipher.next, 2);
        cipher.next = u64::MAX;
        assert!(cipher.seal(&mut Vec::new()).is_err());
    }
}

#[cfg(test)]
#[path = "crypto_scheduler_tests.rs"]
mod scheduler_tests;
