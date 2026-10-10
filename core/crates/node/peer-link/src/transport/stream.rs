//! 字节流传输共用的消息边界处理；不含 socket、配对或路由。
use crate::{PeerConnection, PeerEndpoint, PeerMessage, PeerTransport};
use async_trait::async_trait;
use operit_link::{decodeLink, encodeLink};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use tokio::sync::Mutex;
// Diagnostic counters only; never used to authorize or route a request.
// Ordering is kept compatible with the existing device health snapshot:
// valid frames, CRC errors, discarded noise bytes, expired partial frames.
static SERIAL_COUNTERS: [std::sync::atomic::AtomicU32; 4] =
    [const { std::sync::atomic::AtomicU32::new(0) }; 4];
pub(crate) fn serialFrameCounters() -> [u32; 4] {
    std::array::from_fn(|i| SERIAL_COUNTERS[i].load(Ordering::Relaxed))
}

#[cfg(test)]
const MAX_PEER_MESSAGE_BYTES: usize = crate::DEFAULT_MAX_PEER_MESSAGE_BYTES;

#[async_trait]
pub(super) trait ByteConnection: Send + Sync {
    fn remoteAddress(&self) -> Option<std::net::SocketAddr> {
        None
    }
    async fn write(&self, bytes: &[u8]) -> Result<(), String>;
    async fn read(&self) -> Result<Option<Vec<u8>>, String>;
    async fn close(&self);
}

pub(super) struct FramedPeerConnection {
    source: PeerEndpoint,
    target: PeerEndpoint,
    transport: PeerTransport,
    io: Arc<dyn ByteConnection>,
    writeLock: Mutex<()>,
    readLock: Mutex<()>,
    pending: Mutex<ReadBuffer>,
    framing: FrameMode,
    maxMessageBytes: usize,
    closeLock: Mutex<()>,
    closeRequested: AtomicBool,
    remoteClosed: AtomicBool,
}
/// Retain a Host chunk's unconsumed suffix without treating it as one frame.
/// Both pieces live across receive cancellation; only `pending` is assembled.
#[derive(Default)]
struct ReadBuffer {
    pending: Vec<u8>,
    chunk: Vec<u8>,
    offset: usize,
}

#[derive(Clone, Copy)]
enum FrameMode {
    Length,
    Serial { initiator: bool },
}
impl FramedPeerConnection {
    pub(super) fn new(
        source: PeerEndpoint,
        target: PeerEndpoint,
        transport: PeerTransport,
        io: Arc<dyn ByteConnection>,
        maxMessageBytes: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            source,
            target,
            transport,
            io,
            writeLock: Mutex::new(()),
            readLock: Mutex::new(()),
            pending: Mutex::new(ReadBuffer::default()),
            framing: FrameMode::Length,
            maxMessageBytes,
            closeLock: Mutex::new(()),
            closeRequested: AtomicBool::new(false),
            remoteClosed: AtomicBool::new(false),
        })
    }
    pub(super) fn newSerial(
        source: PeerEndpoint,
        target: PeerEndpoint,
        io: Arc<dyn ByteConnection>,
        initiator: bool,
        maxMessageBytes: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            source,
            target,
            transport: PeerTransport::Serial,
            io,
            writeLock: Mutex::new(()),
            readLock: Mutex::new(()),
            pending: Mutex::new(ReadBuffer::default()),
            framing: FrameMode::Serial { initiator },
            maxMessageBytes,
            closeLock: Mutex::new(()),
            closeRequested: AtomicBool::new(false),
            remoteClosed: AtomicBool::new(false),
        })
    }
    async fn awaitSerialEnd(&self) -> Result<(), String> {
        while !self.remoteClosed.load(Ordering::Acquire) {
            if self.readFrame().await?.is_none() { break; }
        }
        if !self.remoteClosed.load(Ordering::Acquire) {
            return Err("Serial connection ended before boundary acknowledgement".into());
        }
        Ok(())
    }
    /// A UART has no physical EOF. Reset an abandoned session and wait for its
    /// ACK before sending a fresh hello. This also recovers cancelled callers.
    pub(super) async fn initializeSerial(&self) -> Result<(), String> {
        {
            let _writer = self.writeLock.lock().await;
            self.io.write(&serialEndFrame()).await?;
        }
        tokio::time::timeout(Duration::from_secs(2), self.awaitSerialEnd()).await
            .map_err(|_| "Serial session boundary was not acknowledged; update device firmware".to_string())??;
        self.remoteClosed.store(false, Ordering::Release);
        Ok(())
    }
    async fn readFrame(&self) -> Result<Option<Vec<u8>>, String> {
        let _guard = self.readLock.lock().await;
        let mut buffer = self.pending.lock().await;
        let ReadBuffer { pending, chunk, offset } = &mut *buffer;
        loop {
            if self.remoteClosed.load(Ordering::Acquire) { return Ok(None); }
            if let FrameMode::Serial { initiator } = self.framing {
                if let Some(frame) = readSerialFrame(pending, self.maxMessageBytes)? {
                    // Canonical zero-length, CRC-protected OPS1 is transport EOF.
                    // The listener acknowledges before releasing its UART lease.
                    // An initiator consumes ACK, never echoes it into a new session.
                    if frame.is_empty() {
                        self.remoteClosed.store(true, Ordering::Release);
                        if !initiator {
                            let _writer = self.writeLock.lock().await;
                            let result = self.io.write(&serialEndFrame()).await;
                            self.io.close().await;
                            result?;
                        }
                        return Ok(None);
                    }
                    return Ok(Some(frame));
                }
            }
            if matches!(self.framing, FrameMode::Length) && pending.len() >= 4 {
                let length = u32::from_be_bytes(pending[..4].try_into().unwrap()) as usize;
                if length == 0 || length > self.maxMessageBytes {
                    return Err(format!("invalid peer message length: {length}"));
                }
                if pending.len() >= length + 4 {
                    if pending.len() == length + 4 {
                        // Transfer a complete single frame instead of copying it.
                        let mut frame = std::mem::take(pending);
                        frame.drain(..4);
                        return Ok(Some(frame));
                    }
                    let mut frame = Vec::new();
                    frame
                        .try_reserve_exact(length)
                        .map_err(|_| "Insufficient memory for peer frame".to_string())?;
                    frame.extend_from_slice(&pending[4..length + 4]);
                    pending.drain(..length + 4);
                    return Ok(Some(frame));
                }
            }
            // The size bound applies to each frame, not to an arbitrary Host
            // read. Consume at most one frame-buffer's worth, then parse before
            // copying more. Keep the remaining chunk for the next receive.
            if *offset < chunk.len() {
                let limit = self.maxMessageBytes + match self.framing {
                    FrameMode::Length => 4,
                    FrameMode::Serial { .. } => SERIAL_FRAME_OVERHEAD,
                };
                if pending.is_empty() && *offset == 0 && chunk.len() <= limit {
                    *pending = std::mem::take(chunk);
                } else {
                    let count = (limit - pending.len()).min(chunk.len() - *offset);
                    if count == 0 { return Err("peer receive buffer exceeded limit".into()); }
                    pending.try_reserve_exact(count)
                        .map_err(|_| "Insufficient memory for peer receive buffer".to_string())?;
                    pending.extend_from_slice(&chunk[*offset..*offset + count]);
                    *offset += count;
                    if *offset == chunk.len() {
                        // Do not retain a second payload-sized allocation on ESP32.
                        *chunk = Vec::new();
                        *offset = 0;
                    }
                }
                continue;
            }
            // UART/log corruption or an interrupted writer can leave a plausible
            // header without its full body. After an idle second discard that
            // partial frame so a fresh END/hello can resynchronize. Never scan
            // for control frames inside payload bytes of an in-progress frame.
            let read = if matches!(self.framing, FrameMode::Serial { .. }) && !pending.is_empty() {
                match tokio::time::timeout(Duration::from_secs(1), self.io.read()).await {
                    Ok(result) => result,
                    Err(_) => { SERIAL_COUNTERS[3].fetch_add(1, Ordering::Relaxed); pending.clear(); continue; }
                }
            } else { self.io.read().await };
            match read? {
                Some(bytes) if !bytes.is_empty() => {
                    *chunk = bytes;
                    *offset = 0;
                }
                Some(_) => continue,
                None if pending.is_empty() => return Ok(None),
                None => return Err("peer stream ended inside a message".into()),
            }
        }
    }
}
#[async_trait]
impl PeerConnection for FramedPeerConnection {
    fn requiresSessionReuse(&self) -> bool { matches!(self.framing, FrameMode::Serial { .. }) }

    fn source(&self) -> &PeerEndpoint {
        &self.source
    }
    fn target(&self) -> &PeerEndpoint {
        &self.target
    }
    fn transport(&self) -> PeerTransport {
        self.transport
    }
    fn remoteAddress(&self) -> Option<std::net::SocketAddr> {
        self.io.remoteAddress()
    }
    async fn send(&self, message: PeerMessage) -> Result<(), String> {
        let mut payload = encodeLink(message).map_err(|e| e.to_string())?;
        if payload.is_empty() || payload.len() > self.maxMessageBytes {
            return Err("peer message exceeds limit".into());
        }
        let _guard = self.writeLock.lock().await;
        if self.closeRequested.load(Ordering::Acquire) || self.remoteClosed.load(Ordering::Acquire) {
            return Err("Peer connection is closed".into());
        }
        match self.framing {
            FrameMode::Length => {
                let length = payload.len();
                payload
                    .try_reserve_exact(4)
                    .map_err(|_| "Insufficient memory for peer send buffer".to_string())?;
                payload.resize(length + 4, 0);
                payload.copy_within(..length, 4);
                payload[..4].copy_from_slice(&(length as u32).to_be_bytes());
                self.io.write(&payload).await
            }
            FrameMode::Serial { .. } => {
                // Keep one encoded payload allocation, as the length sender does.
                let length = payload.len();
                let crc = serialCrc32(&payload);
                payload.try_reserve_exact(SERIAL_FRAME_OVERHEAD)
                    .map_err(|_| "Insufficient memory for serial peer frame".to_string())?;
                payload.resize(length + SERIAL_FRAME_OVERHEAD, 0);
                payload.copy_within(..length, 8);
                payload[..4].copy_from_slice(&SERIAL_MAGIC);
                payload[4..8].copy_from_slice(&(length as u32).to_be_bytes());
                payload[8 + length..].copy_from_slice(&crc.to_be_bytes());
                self.io.write(&payload).await
            }
        }
    }
    async fn receive(&self) -> Result<Option<PeerMessage>, String> {
        self.readFrame()
            .await?
            .map(|bytes| decodeLink(&bytes).map_err(|e| e.to_string()))
            .transpose()
    }
    async fn close(&self) {
        let _closing = self.closeLock.lock().await;
        let first = !self.closeRequested.swap(true, Ordering::AcqRel);
        if first && !self.remoteClosed.load(Ordering::Acquire)
            && matches!(self.framing, FrameMode::Serial { initiator: true }) {
            // Bound the WHOLE graceful exchange, including a busy writer or
            // stalled write. A timeout cancels the END write before port release.
            let _ = tokio::time::timeout(Duration::from_millis(500), async {
                {
                    let _writer = self.writeLock.lock().await;
                    self.io.write(&serialEndFrame()).await?;
                }
                // The existing reader can consume ACK; no extra UART task.
                self.awaitSerialEnd().await
            }).await;
        }
        self.io.close().await;
    }
}

const SERIAL_MAGIC: [u8; 4] = *b"OPS1";
const SERIAL_FRAME_OVERHEAD: usize = 4 + 4 + 4;

fn serialEndFrame() -> [u8; SERIAL_FRAME_OVERHEAD] {
    // CRC32(empty) == 0. This is not an empty application message.
    [b'O', b'P', b'S', b'1', 0, 0, 0, 0, 0, 0, 0, 0]
}

/// Serial is shared with the ESP-IDF console, so its reader must discard boot/log
/// output and resynchronize instead of treating it as a fatal length prefix error.
fn readSerialFrame(pending: &mut Vec<u8>, maxMessageBytes: usize) -> Result<Option<Vec<u8>>, String> {
    loop {
        let Some(start) = pending
            .windows(SERIAL_MAGIC.len())
            .position(|part| part == SERIAL_MAGIC)
        else {
            let keep = pending.len().min(SERIAL_MAGIC.len() - 1);
            if pending.len() > keep {
                let drop = pending.len() - keep;
                SERIAL_COUNTERS[2].fetch_add(drop as u32, Ordering::Relaxed);
                pending.drain(..drop);
            }
            return Ok(None);
        };
        if start > 0 {
            SERIAL_COUNTERS[2].fetch_add(start as u32, Ordering::Relaxed);
            pending.drain(..start);
        }
        if pending.len() < SERIAL_FRAME_OVERHEAD {
            return Ok(None);
        }
        let length = u32::from_be_bytes(pending[4..8].try_into().unwrap()) as usize;
        if length > maxMessageBytes {
            pending.drain(..1);
            continue;
        }
        let total = SERIAL_FRAME_OVERHEAD + length;
        if pending.len() < total {
            return Ok(None);
        }
        let crcStart = 8 + length;
        let expected = u32::from_be_bytes(pending[crcStart..total].try_into().unwrap());
        if expected != serialCrc32(&pending[8..crcStart]) {
            SERIAL_COUNTERS[1].fetch_add(1, Ordering::Relaxed);
            pending.drain(..1);
            continue;
        }
        SERIAL_COUNTERS[0].fetch_add(1, Ordering::Relaxed);
        if pending.len() == total {
            let mut frame = std::mem::take(pending);
            frame.truncate(crcStart);
            frame.drain(..8);
            return Ok(Some(frame));
        }
        // Coalesced frames must retain their suffix. Allocation failure is a
        // transport error, not an abort caused by an infallible payload clone.
        let mut frame = Vec::new();
        frame.try_reserve_exact(length)
            .map_err(|_| "Insufficient memory for serial peer frame".to_string())?;
        frame.extend_from_slice(&pending[8..crcStart]);
        pending.drain(..total);
        return Ok(Some(frame));
    }
}

fn serialCrc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    struct Bytes {
        chunks: Mutex<VecDeque<Vec<u8>>>,
        sent: Mutex<Vec<u8>>,
    }
    #[async_trait]
    impl ByteConnection for Bytes {
        async fn write(&self, bytes: &[u8]) -> Result<(), String> {
            self.sent.lock().await.extend_from_slice(bytes);
            Ok(())
        }
        async fn read(&self) -> Result<Option<Vec<u8>>, String> {
            Ok(self.chunks.lock().await.pop_front())
        }
        async fn close(&self) {}
    }
    fn connection(chunks: Vec<Vec<u8>>) -> (Arc<FramedPeerConnection>, Arc<Bytes>) {
        let io = Arc::new(Bytes {
            chunks: Mutex::new(chunks.into()),
            sent: Mutex::new(Vec::new()),
        });
        let endpoint = PeerEndpoint {
            nodeId: "test".into(),
            address: String::new(),
        };
        (
            FramedPeerConnection::new(endpoint.clone(), endpoint, PeerTransport::Tcp, io.clone(), MAX_PEER_MESSAGE_BYTES),
            io,
        )
    }
    #[tokio::test]
    async fn injected_limit_rejects_oversized_length_with_bounded_buffer() {
        let (peer, _) = connection(vec![frame(&[7; 17])]);
        let mut peer = Arc::try_unwrap(peer).ok().unwrap();
        peer.maxMessageBytes = 16;
        assert!(peer.readFrame().await.is_err());
        assert!(peer.pending.lock().await.pending.len() <= peer.maxMessageBytes + 4);
    }

    #[tokio::test]
    async fn injected_limit_rejects_oversized_sends_without_writing() {
        use operit_link::{CoreCallRequest, CoreLinkRequest, CoreValue};
        let (peer, io) = connection(Vec::new());
        let mut peer = Arc::try_unwrap(peer).ok().unwrap();
        peer.maxMessageBytes = 16;
        let message = PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new(
            "bounded", "object", "method", CoreValue::String("x".repeat(32)))));
        assert!(peer.send(message).await.is_err());
        assert!(io.sent.lock().await.is_empty());
    }

    #[test]
    fn injected_serial_limit_resynchronizes_without_accepting_oversized_frame() {
        let mut bytes = serialFrame(&[7; 17]);
        bytes.extend(serialFrame(b"next"));
        assert_eq!(readSerialFrame(&mut bytes, 16).unwrap().unwrap(), b"next");
    }

    #[test]
    fn invalid_runtime_limits_are_rejected() {
        assert!(crate::HostPeerLink::withMaxMessageBytes(0).is_err());
        assert!(crate::HostPeerLink::withMaxMessageBytes(8192).is_ok());
    }

    fn frame(bytes: &[u8]) -> Vec<u8> {
        let mut result = (bytes.len() as u32).to_be_bytes().to_vec();
        result.extend_from_slice(bytes);
        result
    }
    fn serialFrame(bytes: &[u8]) -> Vec<u8> {
        let mut result = SERIAL_MAGIC.to_vec();
        result.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        result.extend_from_slice(bytes);
        result.extend_from_slice(&serialCrc32(bytes).to_be_bytes());
        result
    }

    #[test]
    fn serial_reader_discards_console_noise_and_resynchronizes() {
        let mut pending = b"[I] booting\r\n".to_vec();
        pending.extend_from_slice(&serialFrame(b"link"));
        assert_eq!(
            readSerialFrame(&mut pending, MAX_PEER_MESSAGE_BYTES).unwrap(),
            Some(b"link".to_vec())
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn serial_reader_rejects_bad_crc_and_keeps_following_frame() {
        let mut pending = serialFrame(b"bad");
        pending[11] ^= 1;
        pending.extend_from_slice(&serialFrame(b"good"));
        assert_eq!(
            readSerialFrame(&mut pending, MAX_PEER_MESSAGE_BYTES).unwrap(),
            Some(b"good".to_vec())
        );
    }

    struct UartPipe {
        tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        rx: Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>,
        written: Mutex<Vec<Vec<u8>>>,
    }
    #[async_trait]
    impl ByteConnection for UartPipe {
        async fn write(&self, bytes: &[u8]) -> Result<(), String> {
            self.written.lock().await.push(bytes.to_vec());
            self.tx.send(bytes.to_vec()).map_err(|e| e.to_string())
        }
        async fn read(&self) -> Result<Option<Vec<u8>>, String> {
            Ok(self.rx.lock().await.recv().await)
        }
        // Like a real UART, releasing a lease does not send EOF to the peer.
        async fn close(&self) {}
    }
    fn uartPipes() -> (Arc<UartPipe>, Arc<UartPipe>) {
        let (a_tx, a_rx) = tokio::sync::mpsc::unbounded_channel();
        let (b_tx, b_rx) = tokio::sync::mpsc::unbounded_channel();
        (Arc::new(UartPipe { tx: a_tx, rx: Mutex::new(b_rx), written: Mutex::new(Vec::new()) }),
         Arc::new(UartPipe { tx: b_tx, rx: Mutex::new(a_rx), written: Mutex::new(Vec::new()) }))
    }
    fn serialPeer(io: Arc<dyn ByteConnection>, initiator: bool) -> Arc<FramedPeerConnection> {
        let endpoint = PeerEndpoint { nodeId: "serial-test".into(), address: "uart0".into() };
        FramedPeerConnection::newSerial(endpoint.clone(), endpoint, io, initiator, MAX_PEER_MESSAGE_BYTES)
    }

    #[test]
    fn serial_end_requires_complete_frame_and_valid_crc() {
        let end = serialEndFrame();
        let mut pending = end[..11].to_vec();
        assert!(readSerialFrame(&mut pending, MAX_PEER_MESSAGE_BYTES).unwrap().is_none());
        pending.push(end[11]);
        assert_eq!(readSerialFrame(&mut pending, MAX_PEER_MESSAGE_BYTES).unwrap(), Some(Vec::new()));
        let mut invalid = end.to_vec();
        invalid[11] = 1;
        invalid.extend(serialFrame(b"next"));
        assert_eq!(readSerialFrame(&mut invalid, MAX_PEER_MESSAGE_BYTES).unwrap(), Some(b"next".to_vec()));
    }

    #[tokio::test]
    async fn serial_sessions_acknowledge_end_before_following_handshake() {
        use operit_link::{CoreCallRequest, CoreLinkRequest, CoreValue};
        let (pc, board) = uartPipes();
        for _ in 0..3 {
            let client = serialPeer(pc.clone(), true);
            let staleListener = serialPeer(board.clone(), false);
            let (initialized, ended) = tokio::join!(client.initializeSerial(), staleListener.readFrame());
            initialized.unwrap();
            assert!(ended.unwrap().is_none());
            let listener = serialPeer(board.clone(), false);
            let request = PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new("hello", "test", "call", CoreValue::Null)));
            let expected = encodeLink(&request).unwrap();
            client.send(request).await.unwrap();
            assert_eq!(encodeLink(listener.receive().await.unwrap().unwrap()).unwrap(), expected);
            let ((), ended) = tokio::join!(client.close(), listener.receive());
            assert!(ended.unwrap().is_none());
            // Duplicate close on either old lease must not echo END into the
            // next session, including after the next UART lease is acquired.
            let count = pc.written.lock().await.len();
            client.close().await;
            listener.close().await;
            assert_eq!(pc.written.lock().await.len(), count);
            assert!(client.send(PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new("stale", "test", "call", CoreValue::Null)))).await.is_err());
        }
        assert_eq!(pc.written.lock().await.iter().filter(|f| f.as_slice() == serialEndFrame()).count(), 6);
        assert_eq!(board.written.lock().await.iter().filter(|f| f.as_slice() == serialEndFrame()).count(), 6);
    }

    #[tokio::test]
    async fn serial_partial_frame_expires_without_interpreting_payload_as_end() {
        let (pc, board) = uartPipes();
        let listener = serialPeer(board, false);
        let mut partial = SERIAL_MAGIC.to_vec();
        partial.extend_from_slice(&1024u32.to_be_bytes());
        // Even a valid END marker embedded in the body is not a boundary.
        partial.extend_from_slice(&serialEndFrame());
        pc.write(&partial).await.unwrap();
        let delayed = async {
            tokio::time::sleep(Duration::from_millis(1100)).await;
            pc.write(&serialEndFrame()).await.unwrap();
        };
        let (received, ()) = tokio::join!(listener.readFrame(), delayed);
        assert!(received.unwrap().is_none());
        let ack = pc.read().await.unwrap().unwrap();
        assert_eq!(ack, serialEndFrame());
    }

    #[tokio::test]
    async fn serial_new_connection_recovers_abandoned_authenticated_stream() {
        let (pc, board) = uartPipes();
        let abandoned = serialPeer(pc.clone(), true);
        let listener = serialPeer(board.clone(), false);
        drop(abandoned); // no implicit physical EOF on UART
        let replacement = serialPeer(pc.clone(), true);
        let (reset, ended) = tokio::join!(replacement.initializeSerial(), listener.receive());
        reset.unwrap();
        assert!(ended.unwrap().is_none());
        assert!(!replacement.remoteClosed.load(Ordering::Acquire));
        assert!(!replacement.closeRequested.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn serial_close_uses_the_existing_blocked_reader_for_ack() {
        let (pc, board) = uartPipes();
        let client = serialPeer(pc, true);
        let listener = serialPeer(board, false);
        let result = tokio::time::timeout(Duration::from_secs(1), async {
            tokio::join!(client.receive(), client.close(), listener.receive())
        }).await.expect("shutdown cannot deadlock behind the existing reader");
        assert!(result.0.unwrap().is_none());
        assert!(result.2.unwrap().is_none());
        assert!(client.remoteClosed.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn serial_close_is_bounded_without_ack_and_does_not_resend() {
        let (pc, _board) = uartPipes();
        let client = serialPeer(pc.clone(), true);
        tokio::time::timeout(Duration::from_secs(1), client.close()).await.unwrap();
        assert!(client.closeRequested.load(Ordering::Acquire));
        client.close().await;
        assert_eq!(pc.written.lock().await.as_slice(), &[serialEndFrame().to_vec()]);
    }

    #[tokio::test]
    async fn serial_initialization_rejects_eof_without_ack() {
        let (_, io) = connection(Vec::new());
        let client = serialPeer(io, true);
        assert!(client.initializeSerial().await.unwrap_err().contains("before boundary acknowledgement"));
    }

    #[tokio::test]
    async fn receives_split_and_coalesced_frames_without_losing_leftovers() {
        let mut bytes = frame(&vec![7; 1300]);
        bytes.extend(frame(b"next"));
        let chunks = bytes.chunks(512).map(|part| part.to_vec()).collect();
        let (peer, _) = connection(chunks);
        assert_eq!(peer.readFrame().await.unwrap().unwrap(), vec![7; 1300]);
        assert_eq!(peer.readFrame().await.unwrap().unwrap(), b"next");
        assert!(peer.readFrame().await.unwrap().is_none());
        let mut bytes = frame(b"one");
        bytes.extend(frame(b"two"));
        let (peer, _) = connection(vec![bytes]);
        assert_eq!(peer.readFrame().await.unwrap().unwrap(), b"one");
        assert_eq!(peer.readFrame().await.unwrap().unwrap(), b"two");
    }
    #[tokio::test]
    async fn rejects_invalid_length_and_truncated_frame() {
        for length in [0, MAX_PEER_MESSAGE_BYTES as u32 + 1] {
            let (peer, _) = connection(vec![length.to_be_bytes().to_vec()]);
            assert!(peer
                .readFrame()
                .await
                .unwrap_err()
                .contains("invalid peer message length"));
        }
        let (peer, _) = connection(vec![vec![0, 0, 0, 2, 42]]);
        assert!(peer.readFrame().await.is_err());
    }
    // A Host read is a chunk of a byte stream, not exactly one Link frame.
    #[tokio::test]
    async fn maximum_length_frame_allows_coalesced_next_frame() {
        let mut wire = frame(&vec![7; MAX_PEER_MESSAGE_BYTES]);
        wire.extend(frame(b"next"));
        let split = MAX_PEER_MESSAGE_BYTES - 32;
        let tail = wire.split_off(split);
        let (peer, _) = connection(vec![wire, tail]);
        assert_eq!(peer.readFrame().await.unwrap().unwrap().len(), MAX_PEER_MESSAGE_BYTES);
        assert_eq!(peer.readFrame().await.unwrap().unwrap(), b"next");
        assert!(peer.readFrame().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn maximum_serial_frame_allows_coalesced_next_frame() {
        let mut wire = serialFrame(&vec![7; MAX_PEER_MESSAGE_BYTES]);
        wire.extend(serialFrame(b"next"));
        let split = MAX_PEER_MESSAGE_BYTES - 32;
        let tail = wire.split_off(split);
        let (_, io) = connection(vec![wire, tail]);
        let peer = serialPeer(io, true);
        assert_eq!(peer.readFrame().await.unwrap().unwrap().len(), MAX_PEER_MESSAGE_BYTES);
        assert_eq!(peer.readFrame().await.unwrap().unwrap(), b"next");
        assert!(peer.readFrame().await.unwrap().is_none());
    }

    #[test]
    fn serial_extract_reuses_complete_frame_allocation() {
        let mut pending = serialFrame(b"do not duplicate the UART payload");
        let allocation = pending.as_ptr();
        let decoded = readSerialFrame(&mut pending, MAX_PEER_MESSAGE_BYTES).unwrap().unwrap();
        assert_eq!(decoded, b"do not duplicate the UART payload");
        assert_eq!(decoded.as_ptr(), allocation);
        assert_eq!(pending.capacity(), 0);
    }

    #[tokio::test]
    async fn serial_close_deadline_includes_busy_writer() {
        let (pc, _board) = uartPipes();
        let client = serialPeer(pc.clone(), true);
        let writer = client.writeLock.lock().await;
        tokio::time::timeout(Duration::from_millis(900), client.close())
            .await.expect("graceful close must not wait forever for a writer");
        drop(writer);
        client.close().await;
        assert!(pc.written.lock().await.is_empty(), "no delayed END after port release");
    }

    struct StalledWriter {
        closed: AtomicBool,
    }
    #[async_trait]
    impl ByteConnection for StalledWriter {
        async fn write(&self, _: &[u8]) -> Result<(), String> { std::future::pending().await }
        async fn read(&self) -> Result<Option<Vec<u8>>, String> { std::future::pending().await }
        async fn close(&self) { self.closed.store(true, Ordering::Release); }
    }

    #[tokio::test]
    async fn serial_close_deadline_includes_stalled_end_write() {
        let io = Arc::new(StalledWriter { closed: AtomicBool::new(false) });
        let peer = serialPeer(io.clone(), true);
        tokio::time::timeout(Duration::from_millis(900), peer.close()).await
            .expect("END write must also obey graceful close deadline");
        assert!(io.closed.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn serial_cancelled_read_retains_partial_frame() {
        let (pc, board) = uartPipes();
        let peer = serialPeer(board, false);
        let wire = serialFrame(b"preserve a half received packet");
        pc.write(&wire[..15]).await.unwrap();
        assert!(tokio::time::timeout(Duration::from_millis(20), peer.readFrame()).await.is_err());
        pc.write(&wire[15..]).await.unwrap();
        assert_eq!(peer.readFrame().await.unwrap().unwrap(), b"preserve a half received packet");
    }

    #[tokio::test]
    async fn serial_arbitrary_chunk_boundaries_preserve_payload_and_end() {
        // Even a canonical END embedded in a valid body is just application data.
        let mut payload = b"prefix".to_vec();
        payload.extend(serialEndFrame());
        payload.extend(b"suffix");
        let mut wire = serialFrame(&payload);
        wire.extend(serialFrame(b"next"));
        wire.extend(serialEndFrame());
        for chunk_size in 1..=wire.len() {
            let (_, io) = connection(wire.chunks(chunk_size).map(<[u8]>::to_vec).collect());
            let peer = serialPeer(io, true);
            assert_eq!(peer.readFrame().await.unwrap().unwrap(), payload);
            assert_eq!(peer.readFrame().await.unwrap().unwrap(), b"next");
            assert!(peer.readFrame().await.unwrap().is_none());
            assert!(peer.remoteClosed.load(Ordering::Acquire));
        }
    }

    #[tokio::test]
    async fn serial_send_preserves_wire_format() {
        use operit_link::{CoreCallRequest, CoreLinkRequest, CoreValue};
        let message = PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new(
            "test", "object", "method", CoreValue::Null,
        )));
        let expected = serialFrame(&encodeLink(&message).unwrap());
        let (_, io) = connection(Vec::new());
        let peer = serialPeer(io.clone(), true);
        peer.send(message).await.unwrap();
        assert_eq!(*io.sent.lock().await, expected);
    }

    #[tokio::test]
    async fn in_place_send_preserves_wire_format() {
        use operit_link::{CoreCallRequest, CoreLinkRequest, CoreValue};
        let message = PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new(
            "test",
            "object",
            "method",
            CoreValue::Null,
        )));
        let expected = frame(&encodeLink(&message).unwrap());
        let (peer, io) = connection(Vec::new());
        peer.send(message).await.unwrap();
        assert_eq!(*io.sent.lock().await, expected);
    }
    #[tokio::test]
    async fn binary_audio_push_roundtrips_existing_serial_frames_and_acks() {
        use operit_link::{CoreLinkPushRequestMessage, CoreLinkPushResponse, CoreLinkRequest,
            CoreLinkResponse, CorePushItem, CoreValue};
        let (pc, board) = uartPipes();
        let sender = serialPeer(pc.clone(), true);
        let stale = serialPeer(board.clone(), false);
        let (initialized, boundary) = tokio::join!(sender.initializeSerial(), stale.readFrame());
        initialized.unwrap();
        assert!(boundary.unwrap().is_none());
        let receiver = serialPeer(board, false);
        for sequence in 0..100 {
            let pcm: Vec<u8> = (0..1280).map(|i| (i * 13 + sequence as usize) as u8).collect();
            let message = PeerMessage::Request(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(CorePushItem {
                pushId: "audio-push".into(), sequence, args: CoreValue::Bytes(pcm.clone()),
            })));
            sender.send(message).await.unwrap();
            match receiver.receive().await.unwrap().unwrap() {
                PeerMessage::Request(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(item))) => {
                    assert_eq!(item.sequence, sequence);
                    assert_eq!(item.args, CoreValue::Bytes(pcm));
                }
                _ => panic!("unexpected PCM message"),
            }
            receiver.send(PeerMessage::Response(CoreLinkResponse::Push { pushId: "audio-push".into(),
                result: Ok(CoreLinkPushResponse::ItemAccepted { sequence }) })).await.unwrap();
            assert!(matches!(sender.receive().await.unwrap(), Some(PeerMessage::Response(CoreLinkResponse::Push { result: Ok(CoreLinkPushResponse::ItemAccepted { sequence: n }), .. })) if n == sequence));
        }
        let ((), closed) = tokio::join!(sender.close(), receiver.receive());
        assert!(closed.unwrap().is_none());
    }
    #[tokio::test]
    async fn binary_audio_push_survives_fragmented_tcp_frames() {
        use operit_link::{CoreLinkPushRequestMessage, CoreLinkRequest, CorePushItem, CoreValue};
        let mut wire = Vec::new();
        let mut expected = Vec::new();
        for sequence in 0..100 {
            let bytes: Vec<u8> = (0..640).map(|i| (i + sequence as usize * 7) as u8).collect();
            let message = PeerMessage::Request(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(CorePushItem {
                pushId: "audio-tcp".into(), sequence, args: CoreValue::Bytes(bytes.clone()),
            })));
            wire.extend(frame(&encodeLink(message).unwrap()));
            expected.extend(bytes);
        }
        let (receiver, _) = connection(wire.chunks(7).map(|chunk| chunk.to_vec()).collect());
        let mut received = Vec::new();
        for sequence in 0..100 {
            match receiver.receive().await.unwrap().unwrap() {
                PeerMessage::Request(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(item))) => {
                    assert_eq!(item.sequence, sequence);
                    match item.args { CoreValue::Bytes(bytes) => received.extend(bytes), _ => panic!("PCM was not binary") }
                }
                _ => panic!("unexpected PCM message"),
            }
        }
        assert_eq!(received, expected);
    }

}
