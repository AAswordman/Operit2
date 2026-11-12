//! Transparent logical-message delivery over bounded authenticated session frames.
use super::*;
use operit_peer_link::{PeerEndpoint, PeerTransport};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize};

/// Enforces the production encoded-frame budget and preserves pending reads across cancellation.
struct BoundedPeer {
    endpoint: PeerEndpoint,
    frames: StdMutex<VecDeque<PeerMessage>>,
    changed: tokio::sync::Notify,
    closed: AtomicBool,
    limit: usize,
    largest: AtomicUsize,
    count: AtomicUsize,
}

#[async_trait::async_trait]
impl PeerConnection for BoundedPeer {
    /// Supplies the local test endpoint.
    fn source(&self) -> &PeerEndpoint { &self.endpoint }
    /// Supplies the remote test endpoint.
    fn target(&self) -> &PeerEndpoint { &self.endpoint }
    /// Uses the common session layer above the TCP carrier.
    fn transport(&self) -> PeerTransport { PeerTransport::Tcp }
    /// Applies the real wire codec and rejects any oversized physical frame.
    async fn send(&self, message: PeerMessage) -> Result<(), String> {
        let bytes = encodeLink(message).map_err(|failure| failure.to_string())?;
        if bytes.len() > self.limit { return Err(operit_peer_link::MESSAGE_TOO_LARGE_ERROR.into()); }
        self.largest.fetch_max(bytes.len(), Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.frames.lock().unwrap().push_back(PeerMessage::decodeOwned(bytes.into()).unwrap());
        self.changed.notify_one();
        Ok(())
    }
    /// Waits for whole physical frames without consuming partial logical messages.
    async fn receive(&self) -> Result<Option<PeerMessage>, String> {
        loop {
            let changed = self.changed.notified();
            if let Some(message) = self.frames.lock().unwrap().pop_front() { return Ok(Some(message)); }
            if self.closed.load(Ordering::Relaxed) { return Ok(None); }
            changed.await;
        }
    }
    /// Wakes a pending receive so incomplete reassembly can be discarded.
    async fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
        self.changed.notify_one();
    }
}

/// Creates matching authenticated endpoints over a size-enforcing replay carrier.
fn channels(limit: usize, clientSends: bool) -> (Arc<BoundedPeer>, Arc<Channel>, Arc<Channel>) {
    let raw = Arc::new(BoundedPeer {
        endpoint: PeerEndpoint { nodeId: "peer".into(), address: "memory".into() },
        frames: StdMutex::default(), changed: tokio::sync::Notify::new(), closed: AtomicBool::new(false),
        limit, largest: AtomicUsize::new(0), count: AtomicUsize::new(0),
    });
    let sender = Channel::new(raw.clone(), &[7; 32], b"fragments", clientSends, limit).unwrap();
    let receiver = Channel::new(raw.clone(), &[7; 32], b"fragments", !clientSends, limit).unwrap();
    (raw, sender, receiver)
}

/// Makes a binary payload whose equality detects lost, reordered or repeated bytes.
fn message(size: usize, response: bool) -> PeerMessage {
    let payload = CoreValue::Bytes((0..size).map(|index| (index % 251) as u8).collect::<Vec<_>>().into());
    if response {
        PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse::ok(CoreRequestId::new("large"), payload)))
    } else {
        PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new("large", "test", "upload", payload)))
    }
}

/// Large requests and replies in either key direction arrive once, unchanged, with bounded frames.
#[tokio::test]
async fn messages_above_four_mib_are_transparent_in_both_directions() {
    for limit in [8 * 1024, operit_peer_link::DEFAULT_MAX_PEER_MESSAGE_BYTES] {
        for clientSends in [true, false] {
            for response in [false, true] {
                let (raw, sender, receiver) = channels(limit, clientSends);
                let expected = encodeLink(message(5 * 1024 * 1024 + 19, response)).unwrap();
                sender.sendEncodedBatch(vec![expected.clone()]).await.unwrap();
                assert!(raw.count.load(Ordering::Relaxed) > 1);
                assert!(raw.largest.load(Ordering::Relaxed) <= limit);
                let received = receiver.receiveEncoded().await.unwrap().unwrap().intoBytes();
                assert_eq!(received, expected);
                assert_eq!(PeerMessage::decode(&received).unwrap(), message(5 * 1024 * 1024 + 19, response));
                sender.send(message(3, false)).await.unwrap();
                assert_eq!(receiver.receive().await.unwrap().unwrap(), message(3, false));
                raw.close().await;
                assert!(receiver.receive().await.unwrap().is_none());
            }
        }
    }
}

/// Every edge around a payload boundary preserves exact length and the following message.
#[tokio::test]
async fn exact_chunk_boundaries_and_batches_preserve_message_order() {
    let (raw, sender, receiver) = channels(8192, true);
    let boundary = sender.chunkBytes;
    let messages = [1, boundary - 1, boundary, boundary + 1, boundary * 2, boundary * 2 + 1]
        .map(|size| vec![42; size]);
    sender.sendEncodedBatch(messages.to_vec()).await.unwrap();
    for expected in messages {
        assert_eq!(receiver.receiveEncoded().await.unwrap().unwrap().intoBytes(), expected);
    }
    assert!(raw.frames.lock().unwrap().is_empty());
}

/// Cancellation between heartbeat ticks retains authenticated bytes without exposing a partial message.
#[tokio::test]
async fn cancelled_receive_resumes_and_fragments_count_as_liveness_progress() {
    let (raw, sender, receiver) = channels(8192, true);
    let expected = encodeLink(message(24 * 1024, false)).unwrap();
    sender.sendEncodedBatch(vec![expected.clone()]).await.unwrap();
    let rest = raw.frames.lock().unwrap().split_off(1);
    {
        let receive = receiver.receiveEncoded();
        tokio::pin!(receive);
        assert!(futures_util::poll!(&mut receive).is_pending());
    }
    assert_eq!(receiver.activityRevision.load(Ordering::Relaxed), 1);
    assert!(receiver.receive.lock().await.partial.is_some());
    raw.frames.lock().unwrap().extend(rest);
    raw.changed.notify_one();
    assert_eq!(receiver.receiveEncoded().await.unwrap().unwrap().intoBytes(), expected);
    assert!(receiver.receive.lock().await.partial.is_none());
}

/// An incomplete EOF fails the logical message and releases its retained bytes.
#[tokio::test]
async fn disconnect_discards_partial_message_and_never_delivers_it() {
    let (raw, sender, receiver) = channels(8192, true);
    sender.send(message(24 * 1024, false)).await.unwrap();
    raw.frames.lock().unwrap().truncate(1);
    raw.close().await;
    let failure = receiver.receiveEncoded().await.err().unwrap();
    assert!(failure.message.contains("during a fragmented message"));
    assert!(receiver.receive.lock().await.partial.is_none());
}

/// Makes an individually authenticated fragment with intentionally supplied boundary metadata.
fn fragment(cipher: &mut Cipher, total: u64, offset: u64, payload: &[u8]) -> PeerMessage {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&total.to_be_bytes());
    bytes.extend_from_slice(&offset.to_be_bytes());
    bytes.extend_from_slice(payload);
    let sequence = cipher.next;
    cipher.seal(&mut bytes).unwrap();
    sessionFrame(sequence, bytes)
}

/// Authenticated peers cannot splice messages, skip offsets or exceed the declared total.
#[tokio::test]
async fn invalid_fragment_boundaries_fail_without_partial_delivery() {
    for (total, offset, payload) in [(12, 4, vec![1]), (2, 0, vec![1; 3]), (0, 0, vec![]), (12, 0, vec![1; 8192])] {
        let (raw, _, receiver) = channels(8192, true);
        let key = derive(&[7; 32], b"fragments", b"operit-link-client-to-server-v1").unwrap();
        let mut cipher = Cipher::new(key, b"fragments").unwrap();
        // Bypass the carrier only for deliberately invalid oversized inbound fragment tests.
        raw.frames.lock().unwrap().push_back(fragment(&mut cipher, total, offset, &payload));
        assert!(receiver.receiveEncoded().await.is_err());
        assert!(receiver.receive.lock().await.partial.is_none());
    }
    for (total, offset) in [(13, 4), (12, 3), (12, 0)] {
        let (raw, _, receiver) = channels(8192, true);
        let key = derive(&[7; 32], b"fragments", b"operit-link-client-to-server-v1").unwrap();
        let mut cipher = Cipher::new(key, b"fragments").unwrap();
        raw.send(fragment(&mut cipher, 12, 0, &[1; 4])).await.unwrap();
        raw.send(fragment(&mut cipher, total, offset, &[2; 4])).await.unwrap();
        assert!(receiver.receiveEncoded().await.is_err());
        assert!(receiver.receive.lock().await.partial.is_none());
    }
}

/// A modified tag or repeated encrypted frame is rejected before any logical delivery.
#[tokio::test]
async fn tampered_or_replayed_fragments_are_rejected() {
    for tamper in [true, false] {
        let (raw, sender, receiver) = channels(8192, true);
        sender.send(message(24 * 1024, false)).await.unwrap();
        let mut frames = raw.frames.lock().unwrap();
        let first = frames.pop_front().unwrap();
        let mut encoded = encodeLink(first).unwrap();
        if tamper {
            // Flip the final authentication tag byte, preserving the outer Link envelope.
            let end = encoded.len() - 1;
            encoded[end] ^= 1;
            frames.push_front(PeerMessage::decode(&encoded).unwrap());
        } else {
            frames.push_front(PeerMessage::decode(&encoded).unwrap());
            frames.push_front(PeerMessage::decode(&encoded).unwrap());
        }
        drop(frames);
        assert!(receiver.receiveEncoded().await.is_err());
        assert!(receiver.receive.lock().await.partial.is_none());
    }
}

/// A malicious declared total alone never causes an allocation for that total.
#[tokio::test]
async fn declared_total_does_not_preallocate_unreceived_bytes() {
    let (raw, _, receiver) = channels(8192, true);
    let key = derive(&[7; 32], b"fragments", b"operit-link-client-to-server-v1").unwrap();
    let mut cipher = Cipher::new(key, b"fragments").unwrap();
    raw.send(fragment(&mut cipher, usize::MAX as u64, 0, &[1; 8])).await.unwrap();
    {
        let receive = receiver.receiveEncoded();
        tokio::pin!(receive);
        assert!(futures_util::poll!(&mut receive).is_pending());
    }
    assert!(receiver.receive.lock().await.partial.as_ref().unwrap().bytes.capacity() < 1024);
    receiver.close().await;
    assert!(receiver.receive.lock().await.partial.is_none());
}

/// Real carriers deliver a large request and reply over the same connection, including HTTP backpressure.
#[tokio::test]
async fn large_messages_round_trip_over_tcp_http_and_websocket() {
    use operit_host_api::HostManager::HostManager;
    use operit_host_native_common::{NativeHostRuntimeTaskSchedulerHost, NativeHttpHost, NativeHttpServerHost, NativeTcpHost};
    use operit_peer_link::{HostPeerLink, PeerLink};

    for (transport, scheme) in [(PeerTransport::Tcp, "tcp"), (PeerTransport::Http, "http"), (PeerTransport::WebSocket, "ws")] {
        let network = Arc::new(NativeHttpHost::new());
        let host = Arc::new(HostManager {
            tcpHost: Some(Arc::new(NativeTcpHost)), httpHost: Some(network.clone()), webSocketHost: Some(network),
            httpServerHost: Some(Arc::new(NativeHttpServerHost)),
            hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new())),
            ..HostManager::default()
        });
        let limit = operit_peer_link::DEFAULT_MAX_PEER_MESSAGE_BYTES;
        let link = HostPeerLink::withMaxMessageBytes(limit).unwrap();
        let listener = link.listen(host.clone(), PeerEndpoint {
            nodeId: "server".into(), address: "127.0.0.1:0".into(),
        }, transport).await.unwrap();
        let address = listener.localAddress().unwrap();
        let address = match transport {
            PeerTransport::Tcp => format!("tcp://{address}"),
            _ => format!("{scheme}://{address}/operit/link"),
        };
        let outbound = tokio::time::timeout(std::time::Duration::from_secs(10), link.connect(host,
            PeerEndpoint { nodeId: "client".into(), address: String::new() },
            PeerEndpoint { nodeId: "server".into(), address }, transport),
        ).await.unwrap().unwrap();
        let client = Channel::new(outbound, &[7; 32], b"real-carrier", true, limit).unwrap();
        let server = tokio::time::timeout(std::time::Duration::from_secs(20), async {
            let send = async {
                client.send(message(5 * 1024 * 1024 + 7, false)).await.unwrap();
                assert_eq!(client.receive().await.unwrap().unwrap(), message(5 * 1024 * 1024 + 31, true));
            };
            let reply = async {
                // The shared listener classifies TCP only after its first bytes arrive.
                let server = Channel::new(listener.accept().await.unwrap().unwrap(), &[7; 32], b"real-carrier", false, limit).unwrap();
                assert_eq!(server.receive().await.unwrap().unwrap(), message(5 * 1024 * 1024 + 7, false));
                server.send(message(5 * 1024 * 1024 + 31, true)).await.unwrap();
                server
            };
            let ((), server) = tokio::join!(send, reply);
            server
        }).await.unwrap();
        server.close().await;
        client.close().await;
        listener.close().await;
    }
}
