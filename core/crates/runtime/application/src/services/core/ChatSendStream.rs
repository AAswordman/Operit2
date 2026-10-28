//! Bounded, latest-state semantic observation of one owner-authenticated native send.
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use operit_model::ChatTurnOptions::ChatOutputObserver;
use operit_model::MessagePart::{MessagePart, MessagePartKind};
use operit_plugin_sdk::js_sdk::chat::{ChatMessagePart, ChatMessagePartKind, ChatSendEvent};
use operit_plugin_sdk::js_sdk::results::MessageSendResultData;
use operit_plugin_sdk::js_sdk::{JsAsyncIterable, JsAsyncIterator, JsFuture, JsHostError, JsNullable};
use tokio::sync::{watch, Mutex};

/// Caps all retained semantic snapshots for this send; an overflow rejects observation explicitly.
const MAX_CHAT_SNAPSHOT_BYTES: usize = 8 * 1024 * 1024;

/// Retains only the latest snapshot of each real assistant message and one terminal outcome.
#[derive(Clone, Debug, Default)]
struct ObservationState {
    revision: u64,
    snapshots: BTreeMap<i64, (ChatSendEvent, usize)>,
    terminal: Option<Result<MessageSendResultData, String>>,
    error: Option<String>,
    closed: bool,
}

/// Receives native semantic snapshots without coupling generation to plugin consumption speed.
#[derive(Debug)]
pub(crate) struct ChatSendObservation {
    chatId: String,
    detached: AtomicBool,
    state: watch::Sender<ObservationState>,
}

impl ChatSendObservation {
    /// Creates one bounded latest-state mailbox and its single-consumer iterator.
    pub fn create(chatId: String) -> (Arc<Self>, JsAsyncIterable<ChatSendEvent>) {
        let (state, receiver) = watch::channel(ObservationState::default());
        let observation = Arc::new(Self { chatId, detached: AtomicBool::new(false), state });
        let iterator = Arc::new(ChatSendIterator {
            observation: observation.clone(),
            cursor: Arc::new(Mutex::new(ObservationCursor { receiver, seenRevision: 0, finished: false })),
        });
        (observation, iterator)
    }

    /// Publishes the real terminal result only after native finalization and lease release.
    pub fn finish(&self, result: Result<MessageSendResultData, String>) {
        self.state.send_modify(|state| { state.terminal = Some(result); });
    }

    /// Disposes only observation, releases retained snapshots, and wakes a pending pull.
    fn detach(&self) {
        self.detached.store(true, Ordering::Release);
        self.state.send_modify(|state| { state.closed = true; state.snapshots.clear(); });
    }
}

impl ChatOutputObserver for ChatSendObservation {
    /// Publishes an atomic replacement, including rollback removals, from the existing native parser.
    fn snapshot(&self, messageTimestamp: i64, parts: &[MessagePart]) {
        if self.detached.load(Ordering::Acquire) { return; }
        let size = parts.iter().fold(256usize, |size, part| {
            let attributes = part.attributes.iter().fold(0usize, |bytes, (key, value)| bytes.saturating_add(128).saturating_add(key.len()).saturating_add(value.len()));
            size.saturating_add(256).saturating_add(part.partId.len()).saturating_add(part.content.len())
                .saturating_add(part.toolCallId.as_ref().map_or(0, String::len))
                .saturating_add(part.toolName.as_ref().map_or(0, String::len)).saturating_add(attributes)
        });
        // Bound memory before copying user-controlled model output into the observation mailbox.
        if size > MAX_CHAT_SNAPSHOT_BYTES { self.failed("Chat semantic observation exceeded its 8 MiB snapshot limit".to_string()); return; }
        let parts = parts.iter().map(|part| ChatMessagePart {
            partId: part.partId.clone(), sequence: part.sequence,
            kind: match &part.kind {
                MessagePartKind::Markdown => ChatMessagePartKind::Markdown,
                MessagePartKind::Thinking => ChatMessagePartKind::Thinking,
                MessagePartKind::ToolCall => ChatMessagePartKind::ToolCall,
                MessagePartKind::ToolResult => ChatMessagePartKind::ToolResult,
                MessagePartKind::Status => ChatMessagePartKind::Status,
            },
            content: part.content.clone(), toolCallId: JsNullable::from_option(part.toolCallId.clone()), toolName: JsNullable::from_option(part.toolName.clone()), attributes: part.attributes.clone(),
        }).collect::<Vec<_>>();
        self.state.send_modify(|state| {
            if state.closed || state.error.is_some() { return; }
            let total = state.snapshots.iter().filter(|(timestamp, _)| **timestamp != messageTimestamp)
                .fold(size, |bytes, (_, (_, retained))| bytes.saturating_add(*retained));
            if total > MAX_CHAT_SNAPSHOT_BYTES { state.error = Some("Chat semantic observation exceeded its 8 MiB snapshot limit".to_string()); return; }
            state.revision += 1;
            state.snapshots.insert(messageTimestamp, (ChatSendEvent::Part {
                chatId: self.chatId.clone(), messageTimestamp, revision: state.revision, parts,
            }, size));
        });
    }

    /// Preserves an observation failure without cancelling or changing the accepted native execution.
    fn failed(&self, error: String) {
        self.state.send_modify(|state| { state.error = Some(error); });
    }
}

/// Serializes pulls and remembers the exact delivered semantic revision and terminal boundary.
struct ObservationCursor {
    receiver: watch::Receiver<ObservationState>,
    seenRevision: u64,
    finished: bool,
}

/// Exposes one pull-based view whose disposal has no model-cancellation authority.
struct ChatSendIterator {
    observation: Arc<ChatSendObservation>,
    cursor: Arc<Mutex<ObservationCursor>>,
}

impl JsAsyncIterator<ChatSendEvent> for ChatSendIterator {
    /// Delivers pending authoritative snapshots before the finalized receipt, then ends exactly once.
    fn next(&self) -> JsFuture<Option<ChatSendEvent>> {
        let cursor = self.cursor.clone();
        Box::pin(async move {
            let mut cursor = cursor.lock().await;
            loop {
                if cursor.finished { return Ok(None); }
                let state = cursor.receiver.borrow_and_update().clone();
                if state.closed { cursor.finished = true; return Ok(None); }
                if let Some(error) = state.error { cursor.finished = true; return Err(JsHostError::new(error)); }
                let pending = state.snapshots.values().filter_map(|(event, _)| match event {
                    ChatSendEvent::Part { revision, .. } if *revision > cursor.seenRevision => Some((*revision, event)),
                    _ => None,
                }).min_by_key(|(revision, _)| *revision);
                if let Some((revision, event)) = pending {
                    cursor.seenRevision = revision;
                    return Ok(Some(event.clone()));
                }
                if let Some(result) = state.terminal {
                    cursor.finished = true;
                    return result.map(|result| Some(ChatSendEvent::Completed { result })).map_err(JsHostError::new);
                }
                cursor.receiver.changed().await.map_err(|error| JsHostError::new(format!("Chat observation closed without a terminal receipt: {error}")))?;
            }
        })
    }

    /// Wakes pending pulls and releases observation without requesting cancellation.
    fn close(&self) -> JsFuture<()> {
        self.observation.detach();
        Box::pin(async { Ok(()) })
    }
}

impl Drop for ChatSendIterator {
    /// Releases observations abandoned by engine teardown while native cleanup remains host-owned.
    fn drop(&mut self) { self.observation.detach(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_plugin_sdk::js_sdk::results::{JsOptional, MessageSendOutcome, MessageSendStatus};

    /// Builds a terminal receipt solely for deterministic mailbox tests, not a live host execution.
    fn receipt() -> MessageSendResultData {
        MessageSendResultData { chatId: "chat".to_string(), message: "input".to_string(), aiResponse: JsOptional::Null,
            receivedAt: JsOptional::Null, sentAt: 1, outcome: MessageSendOutcome::NotPersisted { status: MessageSendStatus::Completed } }
    }

    /// Coalesces unconsumed revisions and preserves an explicit rollback's complete semantic replacement.
    #[tokio::test]
    async fn delivers_latest_semantic_replacement_before_terminal_receipt() {
        let (observation, stream) = ChatSendObservation::create("chat".to_string());
        observation.snapshot(10, &[MessagePart::markdown("part".to_string(), 0, "discarded".to_string())]);
        observation.snapshot(10, &[MessagePart::thinking("part".to_string(), 0, "retained".to_string())]);
        observation.finish(Ok(receipt()));
        match stream.next().await.unwrap().unwrap() {
            ChatSendEvent::Part { revision, parts, .. } => {
                assert_eq!(revision, 2); assert_eq!(parts.len(), 1);
                assert_eq!(parts[0].kind, ChatMessagePartKind::Thinking); assert_eq!(parts[0].content, "retained");
            }
            _ => panic!("Expected the latest parsed semantic snapshot"),
        }
        assert!(matches!(stream.next().await.unwrap(), Some(ChatSendEvent::Completed { .. })));
        assert!(stream.next().await.unwrap().is_none());
    }

    /// Retains the final snapshot of each physical assistant message without an append-only chunk queue.
    #[tokio::test]
    async fn preserves_each_real_assistant_message() {
        let (observation, stream) = ChatSendObservation::create("chat".to_string());
        observation.snapshot(10, &[MessagePart::markdown("part".to_string(), 0, "first".to_string())]);
        observation.snapshot(20, &[MessagePart::markdown("part".to_string(), 0, "second".to_string())]);
        observation.finish(Ok(receipt()));
        assert!(matches!(stream.next().await.unwrap(), Some(ChatSendEvent::Part { messageTimestamp: 10, .. })));
        assert!(matches!(stream.next().await.unwrap(), Some(ChatSendEvent::Part { messageTimestamp: 20, .. })));
        assert!(matches!(stream.next().await.unwrap(), Some(ChatSendEvent::Completed { .. })));
    }

    /// Detaches and wakes a pending pull without holding any cancellation authority.
    #[tokio::test]
    async fn disposal_ends_pending_observation() {
        let (observation, stream) = ChatSendObservation::create("chat".to_string());
        let pending = tokio::spawn(stream.next());
        stream.close().await.unwrap();
        assert!(pending.await.unwrap().unwrap().is_none());
        observation.finish(Ok(receipt()));
        assert!(stream.next().await.unwrap().is_none());
    }

    /// Rejects oversized semantic state explicitly while retaining the separate actual terminal outcome.
    #[tokio::test]
    async fn rejects_snapshot_overflow_without_fabricating_a_part() {
        let (observation, stream) = ChatSendObservation::create("chat".to_string());
        observation.snapshot(10, &[MessagePart::markdown("part".to_string(), 0, "x".repeat(MAX_CHAT_SNAPSHOT_BYTES))]);
        observation.finish(Ok(receipt()));
        assert!(stream.next().await.unwrap_err().message.ends_with("8 MiB snapshot limit"));
        assert!(observation.state.borrow().snapshots.is_empty());
        assert!(observation.state.borrow().terminal.is_some());
    }
}
