//! Bounded, runtime-owned Edge audio receivers. Authentication is supplied by
//! CoreNodeRouter; an ingress can only attach once to a preregistered device.
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use operit_edge_contract::audio::AudioIngressOpen;
use operit_host_api::{AudioCapture::AUDIO_MAX_CHUNK_BYTES, AudioInputFormat};
use operit_link::{CoreLinkError, CoreLinkPushSession, CoreValue};
use operit_plugin_sdk::js_sdk::results::EdgeAudioReadResultData;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch, Mutex as AsyncMutex};

pub const AUDIO_QUEUE_BLOCKS: usize = 8;
pub const AUDIO_MAX_STREAMS: usize = 16;
const READ_WAIT: Duration = Duration::from_secs(5);
const SEND_WAIT: Duration = Duration::from_secs(10);
struct Cursor {
    receiver: mpsc::Receiver<Vec<u8>>,
    sequence: u32,
    samples: u64,
}
struct Entry {
    node: String,
    format: AudioInputFormat,
    sender: Mutex<Option<mpsc::Sender<Vec<u8>>>>,
    cursor: AsyncMutex<Cursor>,
    terminal: Mutex<Option<Option<String>>>,
    cancelled: watch::Sender<bool>,
    expires: Instant,
}
impl Entry {
    fn finish(&self, error: Option<String>) {
        let mut terminal = self.terminal.lock().unwrap();
        if terminal.is_none() {
            *terminal = Some(error);
        }
    }
    fn cancel(&self) {
        self.finish(Some("Audio input was closed".into()));
        self.cancelled.send_replace(true);
        self.sender.lock().unwrap().take();
    }
}
#[derive(Default)]
pub struct EdgeAudioRegistry {
    entries: Mutex<HashMap<String, Arc<Entry>>>,
    stopped: std::sync::atomic::AtomicBool,
}
impl EdgeAudioRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(
        &self,
        node: String,
        format: AudioInputFormat,
        durationMs: u32,
    ) -> Result<String, String> {
        format.validate()?;
        if node.is_empty() || node.len() > 128 || durationMs == 0 || durationMs > 300_000 {
            return Err("Invalid audio input node or duration".into());
        }
        let mut entries = self.entries.lock().unwrap();
        if self.stopped.load(std::sync::atomic::Ordering::Acquire) {
            return Err("Core audio runtime stopped".into());
        }
        entries.retain(|_, entry| {
            if Instant::now() >= entry.expires {
                entry.cancel();
                false
            } else {
                true
            }
        });
        if entries.len() >= AUDIO_MAX_STREAMS {
            return Err("Audio receiver capacity exceeded".into());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let (sender, receiver) = mpsc::channel(AUDIO_QUEUE_BLOCKS);
        let (cancelled, _) = watch::channel(false);
        entries.insert(
            id.clone(),
            Arc::new(Entry {
                node,
                format,
                sender: Mutex::new(Some(sender)),
                cursor: AsyncMutex::new(Cursor {
                    receiver,
                    sequence: 0,
                    samples: 0,
                }),
                terminal: Mutex::new(None),
                cancelled,
                expires: Instant::now()
                    + Duration::from_millis(durationMs as u64)
                    + Duration::from_secs(30),
            }),
        );
        Ok(id)
    }
    fn entry(&self, id: &str) -> Result<Arc<Entry>, String> {
        let entry = self
            .entries
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or("Unknown or closed audio stream")?;
        if Instant::now() >= entry.expires {
            entry.cancel();
            return Err("Audio stream expired".into());
        }
        Ok(entry)
    }
    pub fn openIngress(
        &self,
        args: CoreValue,
        origin: &str,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let open: AudioIngressOpen =
            operit_link::fromCoreValue(args).map_err(|e| invalid(e.to_string()))?;
        let entry = self.entry(&open.streamId).map_err(invalid)?;
        if entry.node != origin || entry.format != open.format || *entry.cancelled.borrow() {
            return Err(CoreLinkError::new(
                "AUDIO_ORIGIN_DENIED",
                "Audio ingress does not match its registered device and format",
            ));
        }
        let sender = entry
            .sender
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| invalid("Audio ingress already opened"))?;
        Ok(Box::new(Ingress {
            entry,
            sender: Some(sender),
            ended: false,
        }))
    }
    pub async fn read(&self, id: &str) -> Result<EdgeAudioReadResultData, String> {
        let entry = self.entry(id)?;
        let mut cancelled = entry.cancelled.subscribe();
        let mut cursor = entry
            .cursor
            .try_lock()
            .map_err(|_| "Concurrent reads of one audio stream are not allowed")?;
        let mut result = EdgeAudioReadResultData {
            nodeId: entry.node.clone(),
            streamId: id.into(),
            sequence: cursor.sequence,
            sampleOffset: cursor.samples,
            dataBase64: String::new(),
            byteLength: 0,
            pending: false,
            done: false,
            error: None,
        };
        if *cancelled.borrow() {
            result.done = true;
            result.error = Some("Audio input was closed".into());
            return Ok(result);
        }
        let next = tokio::select! {
            _ = cancelled.changed() => None,
            next = tokio::time::timeout(READ_WAIT, cursor.receiver.recv()) => match next {
                Ok(next) => next,
                Err(_) => { result.pending = true; return Ok(result); }
            }
        };
        match next {
            Some(bytes) => {
                result.byteLength = bytes.len() as u32;
                result.dataBase64 = STANDARD.encode(&bytes);
                cursor.sequence += 1;
                cursor.samples += (bytes.len() / (entry.format.channels as usize * 2)) as u64;
            }
            None => {
                result.done = true;
                result.error = entry
                    .terminal
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap_or(Some("Audio stream disconnected".into()));
            }
        }
        Ok(result)
    }
    /// Explicit Core shutdown also covers external handles that outlive the tree.
    pub fn shutdown(&self) {
        self.stopped
            .store(true, std::sync::atomic::Ordering::Release);
        for (_, entry) in self.entries.lock().unwrap().drain() {
            entry.cancel();
        }
    }
    /// Cancel before calling Edge stop: this releases a producer blocked by backpressure.
    pub fn close(&self, id: &str) -> Option<String> {
        let entry = self.entries.lock().unwrap().remove(id)?;
        entry.cancel();
        Some(entry.node.clone())
    }
}
impl Drop for EdgeAudioRegistry {
    fn drop(&mut self) {
        for entry in self.entries.get_mut().unwrap().values() {
            entry.cancel();
        }
    }
}
fn invalid(message: impl Into<String>) -> CoreLinkError {
    CoreLinkError::new("AUDIO_STREAM_ERROR", message)
}
struct Ingress {
    entry: Arc<Entry>,
    sender: Option<mpsc::Sender<Vec<u8>>>,
    ended: bool,
}
#[async_trait]
impl CoreLinkPushSession for Ingress {
    async fn send(&mut self, value: CoreValue) -> Result<(), CoreLinkError> {
        if self.ended {
            return Err(invalid("Audio stream already ended"));
        }
        match value {
            CoreValue::Bytes(bytes) => {
                self.entry.format.validateChunk(&bytes).map_err(invalid)?;
                debug_assert!(bytes.len() <= AUDIO_MAX_CHUNK_BYTES);
                let mut cancelled = self.entry.cancelled.subscribe();
                if *cancelled.borrow() {
                    return Err(invalid("Audio receiver closed"));
                }
                tokio::select! {
                    _ = cancelled.changed() => Err(invalid("Audio receiver closed")),
                    result = tokio::time::timeout(SEND_WAIT, self.sender.as_ref().unwrap().send(bytes)) =>
                        result.map_err(|_| invalid("Audio consumer stalled for ten seconds"))?
                            .map_err(|_| invalid("Audio receiver disconnected"))
                }
            }
            CoreValue::Map(mut end) if end.get("end") == Some(&CoreValue::Bool(true)) => {
                end.remove("end");
                let error = match end.remove("error") {
                    None | Some(CoreValue::Null) => None,
                    Some(CoreValue::String(error)) if error.len() <= 512 => Some(error),
                    _ => return Err(invalid("Invalid audio terminal error")),
                };
                if !end.is_empty() {
                    return Err(invalid("Unknown audio terminal fields"));
                }
                self.entry.finish(error);
                self.sender.take();
                self.ended = true;
                Ok(())
            }
            _ => Err(invalid(
                "Audio items must be binary PCM or an explicit end marker",
            )),
        }
    }
    async fn close(mut self: Box<Self>) -> Result<(), CoreLinkError> {
        if !self.ended {
            self.entry
                .finish(Some("Audio Push closed without an end marker".into()));
        }
        self.sender.take();
        Ok(())
    }
}
impl Drop for Ingress {
    fn drop(&mut self) {
        if !self.ended {
            self.entry
                .finish(Some("Audio stream disconnected before EOF".into()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn open(
        registry: &EdgeAudioRegistry,
        id: &str,
        node: &str,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        registry.openIngress(
            operit_link::toCoreValue(AudioIngressOpen {
                streamId: id.into(),
                format: AudioInputFormat::default(),
            })
            .unwrap(),
            node,
        )
    }
    fn eof(error: Option<&str>) -> CoreValue {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("end".into(), CoreValue::Bool(true));
        if let Some(e) = error {
            fields.insert("error".into(), CoreValue::String(e.into()));
        }
        CoreValue::Map(fields)
    }
    #[tokio::test]
    async fn binary_blocks_are_ordered_and_eof_drains_buffer() {
        let registry = EdgeAudioRegistry::new();
        let id = registry
            .register("edge".into(), AudioInputFormat::default(), 1000)
            .unwrap();
        let mut push = open(&registry, &id, "edge").unwrap();
        push.send(CoreValue::Bytes(vec![0, 128, 255, 127]))
            .await
            .unwrap();
        push.send(CoreValue::Bytes(vec![7, 0])).await.unwrap();
        push.send(eof(None)).await.unwrap();
        assert!(push.send(CoreValue::Bytes(vec![0, 0])).await.is_err());
        push.close().await.unwrap();
        let first = registry.read(&id).await.unwrap();
        assert_eq!(
            STANDARD.decode(first.dataBase64).unwrap(),
            vec![0, 128, 255, 127]
        );
        assert_eq!(
            (first.sequence, first.sampleOffset, first.byteLength),
            (0, 0, 4)
        );
        let second = registry.read(&id).await.unwrap();
        assert_eq!((second.sequence, second.sampleOffset), (1, 2));
        let end = registry.read(&id).await.unwrap();
        assert!(end.done && end.error.is_none() && !end.pending);
        assert_eq!((end.sequence, end.sampleOffset), (2, 3));
        registry.close(&id);
        assert!(registry.read(&id).await.is_err());
    }
    #[tokio::test]
    async fn provenance_format_and_replay_are_rejected() {
        let registry = EdgeAudioRegistry::new();
        assert!(open(&registry, &uuid::Uuid::new_v4().to_string(), "edge").is_err());
        let id = registry
            .register("edge".into(), AudioInputFormat::default(), 1000)
            .unwrap();
        assert!(open(&registry, &id, "other-edge").is_err());
        assert!(registry
            .openIngress(
                operit_link::toCoreValue(AudioIngressOpen {
                    streamId: id.clone(),
                    format: AudioInputFormat {
                        sampleRateHz: 8000,
                        ..Default::default()
                    }
                })
                .unwrap(),
                "edge"
            )
            .is_err());
        let mut push = open(&registry, &id, "edge").unwrap();
        assert!(open(&registry, &id, "edge").is_err());
        for bytes in [vec![], vec![1], vec![0; AUDIO_MAX_CHUNK_BYTES + 2]] {
            assert!(push.send(CoreValue::Bytes(bytes)).await.is_err());
        }
        drop(push);
        let end = registry.read(&id).await.unwrap();
        assert!(end.done && end.error.unwrap().contains("disconnected"));
    }
    #[tokio::test]
    async fn bounded_queue_backpressures_and_close_unblocks_sender() {
        let registry = EdgeAudioRegistry::new();
        let id = registry
            .register("edge".into(), AudioInputFormat::default(), 1000)
            .unwrap();
        let mut push = open(&registry, &id, "edge").unwrap();
        for _ in 0..AUDIO_QUEUE_BLOCKS {
            push.send(CoreValue::Bytes(vec![0, 0])).await.unwrap();
        }
        {
            let blocked = push.send(CoreValue::Bytes(vec![1, 0]));
            tokio::pin!(blocked);
            assert!(
                tokio::time::timeout(Duration::from_millis(20), &mut blocked)
                    .await
                    .is_err()
            );
            registry.read(&id).await.unwrap();
            blocked.await.unwrap();
        }
        let blocked = push.send(CoreValue::Bytes(vec![2, 0]));
        tokio::pin!(blocked);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut blocked)
                .await
                .is_err()
        );
        registry.close(&id);
        assert!(blocked.await.is_err());
    }
    #[tokio::test]
    async fn close_without_eof_and_capture_failure_are_terminal_errors() {
        let registry = EdgeAudioRegistry::new();
        let id = registry
            .register("edge".into(), AudioInputFormat::default(), 1000)
            .unwrap();
        open(&registry, &id, "edge").unwrap().close().await.unwrap();
        assert!(registry
            .read(&id)
            .await
            .unwrap()
            .error
            .unwrap()
            .contains("without an end marker"));
        let id = registry
            .register("edge".into(), AudioInputFormat::default(), 1000)
            .unwrap();
        let mut push = open(&registry, &id, "edge").unwrap();
        push.send(CoreValue::Bytes(vec![1, 0])).await.unwrap();
        push.send(eof(Some("microphone overrun"))).await.unwrap();
        assert!(!registry.read(&id).await.unwrap().done);
        assert_eq!(
            registry.read(&id).await.unwrap().error.as_deref(),
            Some("microphone overrun")
        );
    }
    #[tokio::test]
    async fn cancellation_of_read_keeps_next_block_and_runtime_drop_stops_sender() {
        let registry = EdgeAudioRegistry::new();
        let id = registry
            .register("edge".into(), AudioInputFormat::default(), 1000)
            .unwrap();
        let mut push = open(&registry, &id, "edge").unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), registry.read(&id))
                .await
                .is_err()
        );
        push.send(CoreValue::Bytes(vec![4, 0])).await.unwrap();
        assert_eq!(registry.read(&id).await.unwrap().sequence, 0);
        drop(registry);
        assert!(push.send(CoreValue::Bytes(vec![5, 0])).await.is_err());
    }
    #[tokio::test]
    async fn explicit_runtime_shutdown_cancels_external_push_handles() {
        let registry = EdgeAudioRegistry::new();
        let id = registry
            .register("edge".into(), AudioInputFormat::default(), 1000)
            .unwrap();
        let mut push = open(&registry, &id, "edge").unwrap();
        registry.shutdown();
        registry.shutdown();
        assert!(push.send(CoreValue::Bytes(vec![0, 0])).await.is_err());
        assert!(registry
            .register("edge".into(), AudioInputFormat::default(), 1000)
            .is_err());
        assert!(registry.read(&id).await.is_err());
    }
}
