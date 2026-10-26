use crate::AppLogger::AppLogger;
use crate::stream::HotStream::{
    mutable_shared_stream, share, MutableSharedStreamImpl, SharedStream, SharedStreamTail,
    StreamStart,
};
use crate::stream::Stream::{CollectFuture, Stream};
use crate::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;
use std::sync::{Arc, Mutex};

const REVISABLE_STREAM_LOG_TAG: &str = "RevisableTextStream";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TextStreamEvent {
    pub event_type: TextStreamEventType,
    pub id: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TextStreamEventType {
    Savepoint,
    Rollback,
}

/// Preserves the source order between response text and revision instructions.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ResponseStreamItem {
    Chunk(String),
    Revision(TextStreamEvent),
}

#[derive(Clone, Debug)]
enum ResponseItemStream {
    Chunks,
    Ordered(MutableSharedStreamImpl<ResponseStreamItem>),
}

enum ResponseTail {
    Chunks(SharedStreamTail<String>),
    Ordered(SharedStreamTail<ResponseStreamItem>),
}

pub trait TextStreamEventCarrier {
    fn event_channel(&self) -> &MutableSharedStreamImpl<TextStreamEvent>;
}

/// Shares a terminal producer failure with a cold stream wrapper.
#[derive(Clone, Debug, Default)]
pub struct TerminalFailureState {
    value: Arc<Mutex<Option<String>>>,
}

impl TerminalFailureState {
    /// Creates an empty terminal failure state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the error that ended the producer.
    pub fn set(&self, error: String) {
        *self
            .value
            .lock()
            .expect("terminal failure state mutex poisoned") = Some(error);
    }

    /// Returns the producer error retained by the wrapper.
    pub fn get(&self) -> Option<String> {
        self.value
            .lock()
            .expect("terminal failure state mutex poisoned")
            .clone()
    }
}

impl<T> TextStreamEventCarrier for Box<T>
where
    T: ?Sized + TextStreamEventCarrier,
{
    fn event_channel(&self) -> &MutableSharedStreamImpl<TextStreamEvent> {
        (**self).event_channel()
    }
}

pub trait RevisableTextStream: Stream<Item = String> + TextStreamEventCarrier {
    /// Returns the producer failure after collection, separately from normal channel closure.
    fn terminal_failure(&self) -> Option<String>;

    /// Collects response text and revision instructions in source order.
    fn collect_ordered<'a>(
        &'a mut self,
        collector: &'a mut dyn FnMut(ResponseStreamItem),
    ) -> CollectFuture<'a>;
}

/// Exposes the retained prefix required to render a revisable text stream.
pub trait RenderableTextStream: RevisableTextStream {
    /// Returns the content that precedes the next streamed text segment.
    fn initial_render_content(&self) -> String;
}

impl<T> RevisableTextStream for Box<T>
where
    T: ?Sized + RevisableTextStream,
{
    /// Preserves the wrapped producer's terminal failure through trait-object erasure.
    fn terminal_failure(&self) -> Option<String> {
        (**self).terminal_failure()
    }

    /// Delegates ordered collection to the wrapped stream.
    fn collect_ordered<'a>(
        &'a mut self,
        collector: &'a mut dyn FnMut(ResponseStreamItem),
    ) -> CollectFuture<'a> {
        (**self).collect_ordered(collector)
    }
}

pub trait RevisableSharedTextStream: SharedStream<String> + RevisableTextStream {}

pub trait RevisableCharStream: Stream<Item = char> + TextStreamEventCarrier {}

pub trait RevisableTextStreamLike: RevisableTextStream {}

impl<T> RevisableTextStreamLike for T where T: RevisableTextStream {}

#[derive(Clone, Debug)]
pub struct DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    upstream: S,
    event_channel: MutableSharedStreamImpl<TextStreamEvent>,
    terminalFailure: TerminalFailureState,
}

impl<S> DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    pub fn new(upstream: S, event_channel: MutableSharedStreamImpl<TextStreamEvent>) -> Self {
        Self::new_with_failure(upstream, event_channel, TerminalFailureState::new())
    }

    /// Creates a text stream wrapper with an externally writable terminal failure state.
    pub fn new_with_failure(
        upstream: S,
        event_channel: MutableSharedStreamImpl<TextStreamEvent>,
        terminalFailure: TerminalFailureState,
    ) -> Self {
        Self {
            upstream,
            event_channel,
            terminalFailure,
        }
    }
}

impl<S> Stream for DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    type Item = String;

    fn is_locked(&self) -> bool {
        self.upstream.is_locked()
    }

    fn buffered_count(&self) -> usize {
        self.upstream.buffered_count()
    }

    fn lock(&mut self) {
        self.upstream.lock();
    }

    fn unlock(&mut self) {
        self.upstream.unlock();
    }

    fn clear_buffer(&mut self) {
        self.upstream.clear_buffer();
    }

    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        self.upstream.collect(collector)
    }
}

impl<S> TextStreamEventCarrier for DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    fn event_channel(&self) -> &MutableSharedStreamImpl<TextStreamEvent> {
        &self.event_channel
    }
}

impl<S> RevisableTextStream for DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    /// Reports no producer failure for an infallible text-only upstream.
    fn terminal_failure(&self) -> Option<String> {
        self.terminalFailure.get()
    }

    /// Collects the text-only source without synthesizing revision events.
    fn collect_ordered<'a>(
        &'a mut self,
        collector: &'a mut dyn FnMut(ResponseStreamItem),
    ) -> CollectFuture<'a> {
        Box::pin(async move {
            self.upstream
                .collect(&mut |chunk| collector(ResponseStreamItem::Chunk(chunk)))
                .await;
        })
    }
}

#[derive(Clone, Debug)]
pub struct DelegatingRevisableSharedTextStream {
    pub upstream: MutableSharedStreamImpl<String>,
    pub event_channel: MutableSharedStreamImpl<TextStreamEvent>,
    item_stream: ResponseItemStream,
    terminalFailure: Arc<Mutex<Option<String>>>,
    initialContent: Arc<Mutex<String>>,
    renderedSegment: Arc<Mutex<TextStreamRevisionTracker>>,
    emitOrder: Arc<Mutex<()>>,
}

impl DelegatingRevisableSharedTextStream {
    /// Creates a shared revisable text stream over independent text and revision channels.
    pub fn new(
        upstream: MutableSharedStreamImpl<String>,
        event_channel: MutableSharedStreamImpl<TextStreamEvent>,
    ) -> Self {
        Self::with_item_stream(upstream, event_channel, ResponseItemStream::Chunks)
    }

    /// Creates a shared response stream whose text and revisions have one order.
    pub fn new_ordered(
        upstream: MutableSharedStreamImpl<String>,
        event_channel: MutableSharedStreamImpl<TextStreamEvent>,
    ) -> Self {
        Self::with_item_stream(
            upstream,
            event_channel,
            ResponseItemStream::Ordered(mutable_shared_stream(usize::MAX)),
        )
    }

    /// Creates a shared stream with one concrete ordered-item strategy.
    fn with_item_stream(
        upstream: MutableSharedStreamImpl<String>,
        event_channel: MutableSharedStreamImpl<TextStreamEvent>,
        item_stream: ResponseItemStream,
    ) -> Self {
        Self {
            upstream,
            event_channel,
            item_stream,
            terminalFailure: Arc::new(Mutex::new(None)),
            initialContent: Arc::new(Mutex::new(String::new())),
            renderedSegment: Arc::new(Mutex::new(TextStreamRevisionTracker::new(""))),
            emitOrder: Arc::new(Mutex::new(())),
        }
    }

    /// Emits one response chunk to both legacy and ordered subscribers.
    ///
    /// Holds the emit-order lock so a subscriber attaching concurrently captures the
    /// chunk in exactly one of the replay snapshot and the live tail.
    pub fn emit_chunk(&self, chunk: String) {
        let _emitOrder = self
            .emitOrder
            .lock()
            .expect("shared text stream emit order mutex poisoned");
        self.renderedSegment
            .lock()
            .expect("shared text stream rendered segment mutex poisoned")
            .append(&chunk);
        if let ResponseItemStream::Ordered(orderedItems) = &self.item_stream {
            orderedItems.emit_live(ResponseStreamItem::Chunk(chunk.clone()));
        }
        self.upstream.emit_live(chunk);
    }

    /// Emits one response revision to both legacy and ordered subscribers.
    pub fn emit_revision(&self, event: TextStreamEvent) {
        let _emitOrder = self
            .emitOrder
            .lock()
            .expect("shared text stream emit order mutex poisoned");
        {
            let mut renderedSegment = self
                .renderedSegment
                .lock()
                .expect("shared text stream rendered segment mutex poisoned");
            match event.event_type {
                TextStreamEventType::Savepoint => renderedSegment.savepoint(&event.id),
                TextStreamEventType::Rollback => {
                    if renderedSegment.rollback(&event.id).is_none() {
                        AppLogger::w(
                            REVISABLE_STREAM_LOG_TAG,
                            &format!(
                                "shared text stream rollback skipped; savepoint {} is not active",
                                event.id
                            ),
                        );
                    }
                }
            }
        }
        let ResponseItemStream::Ordered(orderedItems) = &self.item_stream else {
            panic!("revisable response stream must preserve item order");
        };
        orderedItems.emit_live(ResponseStreamItem::Revision(event.clone()));
        self.event_channel.emit(event);
    }

    /// Closes every response channel after the producer has finished.
    pub fn close(&self) {
        self.upstream.close();
        self.event_channel.close();
        if let ResponseItemStream::Ordered(orderedItems) = &self.item_stream {
            orderedItems.close();
        }
    }

    /// Records the terminal failure that stopped this shared text stream.
    pub fn set_terminal_failure(&self, error: String) {
        *self
            .terminalFailure
            .lock()
            .expect("shared text stream terminal failure mutex poisoned") = Some(error);
    }

    /// Returns the terminal failure recorded for this shared text stream.
    pub fn terminal_failure(&self) -> Option<String> {
        self.terminalFailure
            .lock()
            .expect("shared text stream terminal failure mutex poisoned")
            .clone()
    }

    /// Returns a cloneable text-only stream for lifecycle subscribers.
    pub fn chunk_stream(&self) -> MutableSharedStreamImpl<String> {
        self.upstream.clone()
    }

    /// Stores the retained content that precedes newly emitted text chunks.
    pub fn set_initial_content(&self, content: String) {
        *self
            .initialContent
            .lock()
            .expect("shared text stream initial content mutex poisoned") = content;
    }

    /// Returns the retained content that precedes newly emitted text chunks.
    pub fn initial_content(&self) -> String {
        self.initialContent
            .lock()
            .expect("shared text stream initial content mutex poisoned")
            .clone()
    }

    /// Returns the currently renderable text represented by the retained prefix and emitted segment.
    pub fn current_render_content(&self) -> String {
        let initialContent = self.initial_content();
        let segmentContent = self
            .renderedSegment
            .lock()
            .expect("shared text stream rendered segment mutex poisoned")
            .current_content()
            .to_string();
        format!("{initialContent}{segmentContent}")
    }
}

impl Stream for DelegatingRevisableSharedTextStream {
    type Item = String;

    fn is_locked(&self) -> bool {
        self.upstream.is_locked()
    }

    fn buffered_count(&self) -> usize {
        self.upstream.buffered_count()
    }

    fn lock(&mut self) {
        self.upstream.lock();
    }

    fn unlock(&mut self) {
        self.upstream.unlock();
    }

    fn clear_buffer(&mut self) {
        self.upstream.clear_buffer();
    }

    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        self.upstream.collect(collector)
    }
}

impl SharedStream<String> for DelegatingRevisableSharedTextStream {
    fn subscription_count(&self) -> usize {
        self.upstream.subscription_count()
    }

    fn replay_cache(&self) -> Vec<String> {
        self.upstream.replay_cache()
    }
}

impl TextStreamEventCarrier for DelegatingRevisableSharedTextStream {
    fn event_channel(&self) -> &MutableSharedStreamImpl<TextStreamEvent> {
        &self.event_channel
    }
}

impl RevisableTextStream for DelegatingRevisableSharedTextStream {
    /// Exposes the recorded producer failure to provider-stream consumers.
    fn terminal_failure(&self) -> Option<String> {
        DelegatingRevisableSharedTextStream::terminal_failure(self)
    }

    /// Delivers the current rendered text, then values emitted after subscription.
    ///
    /// Captures the snapshot, its active savepoints, and the live subscription under one
    /// lock so a concurrent emit lands in exactly one of replay and tail. Savepoints
    /// recorded before subscription are replayed at their content positions, so a late
    /// subscriber observes the same savepoint and chunk sequence as an early one and can
    /// still resolve later rollbacks.
    fn collect_ordered<'a>(
        &'a mut self,
        collector: &'a mut dyn FnMut(ResponseStreamItem),
    ) -> CollectFuture<'a> {
        let (snapshot, renderedLen, mut savepoints, tail) = {
            let _emitOrder = self
                .emitOrder
                .lock()
                .expect("shared text stream emit order mutex poisoned");
            let (snapshot, renderedLen, savepoints) = {
                let renderedSegment = self
                    .renderedSegment
                    .lock()
                    .expect("shared text stream rendered segment mutex poisoned");
                let initialContent = self
                    .initialContent
                    .lock()
                    .expect("shared text stream initial content mutex poisoned");
                let state = renderedSegment.state();
                (
                    format!("{initialContent}{}", state.content),
                    state.content.len(),
                    state.savepoints,
                )
            };
            let tail = match &self.item_stream {
                ResponseItemStream::Ordered(orderedItems) => {
                    ResponseTail::Ordered(orderedItems.listen_tail())
                }
                ResponseItemStream::Chunks => ResponseTail::Chunks(self.upstream.listen_tail()),
            };
            let mut savepoints: Vec<(String, usize)> = match &self.item_stream {
                ResponseItemStream::Ordered(_) => savepoints.into_iter().collect(),
                ResponseItemStream::Chunks => Vec::new(),
            };
            savepoints.sort_by_key(|(_, position)| *position);
            (snapshot, renderedLen, savepoints, tail)
        };
        let initialLen = snapshot.len() - renderedLen;
        Box::pin(async move {
            let mut cursor = 0usize;
            for (id, position) in savepoints.drain(..) {
                let split = (initialLen + position).min(snapshot.len());
                if split > cursor {
                    collector(ResponseStreamItem::Chunk(snapshot[cursor..split].to_string()));
                    cursor = split;
                }
                collector(ResponseStreamItem::Revision(TextStreamEvent {
                    event_type: TextStreamEventType::Savepoint,
                    id,
                }));
            }
            if cursor < snapshot.len() {
                collector(ResponseStreamItem::Chunk(snapshot[cursor..].to_string()));
            }
            match tail {
                ResponseTail::Ordered(mut ordered) => {
                    while let Some(item) = ordered.recv().await {
                        collector(item);
                    }
                }
                ResponseTail::Chunks(mut chunks) => {
                    while let Some(chunk) = chunks.recv().await {
                        collector(ResponseStreamItem::Chunk(chunk));
                    }
                }
            }
        })
    }
}

impl RenderableTextStream for DelegatingRevisableSharedTextStream {
    /// Returns the retained prefix required to render this response segment.
    fn initial_render_content(&self) -> String {
        self.initial_content()
    }
}

impl RevisableSharedTextStream for DelegatingRevisableSharedTextStream {}

#[derive(Clone, Debug)]
pub struct DelegatingRevisableCharStream<S>
where
    S: Stream<Item = char>,
{
    upstream: S,
    event_channel: MutableSharedStreamImpl<TextStreamEvent>,
}

impl<S> DelegatingRevisableCharStream<S>
where
    S: Stream<Item = char>,
{
    pub fn new(upstream: S, event_channel: MutableSharedStreamImpl<TextStreamEvent>) -> Self {
        Self {
            upstream,
            event_channel,
        }
    }
}

impl<S> Stream for DelegatingRevisableCharStream<S>
where
    S: Stream<Item = char>,
{
    type Item = char;

    fn is_locked(&self) -> bool {
        self.upstream.is_locked()
    }

    fn buffered_count(&self) -> usize {
        self.upstream.buffered_count()
    }

    fn lock(&mut self) {
        self.upstream.lock();
    }

    fn unlock(&mut self) {
        self.upstream.unlock();
    }

    fn clear_buffer(&mut self) {
        self.upstream.clear_buffer();
    }

    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        self.upstream.collect(collector)
    }
}

impl<S> TextStreamEventCarrier for DelegatingRevisableCharStream<S>
where
    S: Stream<Item = char>,
{
    fn event_channel(&self) -> &MutableSharedStreamImpl<TextStreamEvent> {
        &self.event_channel
    }
}

impl<S> RevisableCharStream for DelegatingRevisableCharStream<S> where S: Stream<Item = char> {}

pub fn with_event_channel<S>(
    stream: S,
    event_channel: MutableSharedStreamImpl<TextStreamEvent>,
) -> DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    DelegatingRevisableTextStream::new(stream, event_channel)
}

/// Wraps a cold stream while exposing a producer failure state to consumers.
pub fn with_event_channel_and_failure<S>(
    stream: S,
    event_channel: MutableSharedStreamImpl<TextStreamEvent>,
    terminalFailure: TerminalFailureState,
) -> DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    DelegatingRevisableTextStream::new_with_failure(stream, event_channel, terminalFailure)
}

pub fn with_event_channel_shared(
    stream: MutableSharedStreamImpl<String>,
    event_channel: MutableSharedStreamImpl<TextStreamEvent>,
) -> DelegatingRevisableSharedTextStream {
    DelegatingRevisableSharedTextStream::new(stream, event_channel)
}

/// Creates a shared response stream that preserves text and revision order.
pub fn with_ordered_event_channel_shared(
    stream: MutableSharedStreamImpl<String>,
    event_channel: MutableSharedStreamImpl<TextStreamEvent>,
) -> DelegatingRevisableSharedTextStream {
    DelegatingRevisableSharedTextStream::new_ordered(stream, event_channel)
}

pub fn with_text_event_channel<S>(
    stream: S,
    event_channel: MutableSharedStreamImpl<TextStreamEvent>,
) -> DelegatingRevisableCharStream<S>
where
    S: Stream<Item = char>,
{
    DelegatingRevisableCharStream::new(stream, event_channel)
}

pub fn share_revisable<S>(
    stream: S,
    replay: usize,
    started: StreamStart,
) -> DelegatingRevisableSharedTextStream
where
    S: Stream<Item = String> + TextStreamEventCarrier + Send + 'static,
{
    let event_channel = stream.event_channel().clone();
    let shared_text_stream = share(stream, replay, started);
    let shared_event_stream = share(event_channel, usize::MAX, started);
    DelegatingRevisableSharedTextStream::new(shared_text_stream, shared_event_stream)
}

pub fn empty_revisable_event_channel() -> MutableSharedStreamImpl<TextStreamEvent> {
    mutable_shared_stream(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies retained render content is shared by every stream clone.
    #[test]
    fn retained_render_content_is_shared() {
        let stream = DelegatingRevisableSharedTextStream::new_ordered(
            mutable_shared_stream(usize::MAX),
            mutable_shared_stream(usize::MAX),
        );
        let clone = stream.clone();

        stream.set_initial_content("persisted prefix".to_string());

        assert_eq!(clone.initial_render_content(), "persisted prefix");
    }

    /// Verifies the render snapshot tracks emitted chunks and ordered revisions.
    #[test]
    fn current_render_content_tracks_chunks_and_revisions() {
        let stream = DelegatingRevisableSharedTextStream::new_ordered(
            mutable_shared_stream(usize::MAX),
            mutable_shared_stream(usize::MAX),
        );

        stream.set_initial_content("prefix ".to_string());
        stream.emit_chunk("first ".to_string());
        stream.emit_revision(TextStreamEvent {
            event_type: TextStreamEventType::Savepoint,
            id: "retry".to_string(),
        });
        stream.emit_chunk("discarded".to_string());
        stream.emit_revision(TextStreamEvent {
            event_type: TextStreamEventType::Rollback,
            id: "retry".to_string(),
        });
        stream.emit_chunk("second".to_string());

        assert_eq!(stream.current_render_content(), "prefix first second");
    }

    /// Preserves the failure and partial text through shared clones and trait-object erasure.
    #[tokio::test]
    async fn boxed_stream_retains_terminal_failure_after_collection() {
        let stream = DelegatingRevisableSharedTextStream::new_ordered(
            mutable_shared_stream(usize::MAX), mutable_shared_stream(usize::MAX),
        );
        let producer = stream.clone();
        let mut response: Box<dyn RevisableTextStreamLike> = Box::new(stream);
        producer.emit_chunk("partial response".to_string());
        producer.set_terminal_failure("original transport error".to_string());
        producer.close();
        let mut items = Vec::new();
        response.collect_ordered(&mut |item| items.push(item)).await;
        assert_eq!(items, vec![ResponseStreamItem::Chunk("partial response".to_string())]);
        assert_eq!(RevisableTextStream::terminal_failure(&response), Some("original transport error".to_string()));
    }

    /// Distinguishes successful empty completion from a producer failure.
    #[tokio::test]
    async fn successful_empty_stream_has_no_terminal_failure() {
        let mut stream = DelegatingRevisableSharedTextStream::new_ordered(
            mutable_shared_stream(usize::MAX), mutable_shared_stream(usize::MAX),
        );
        stream.close();
        let mut items = Vec::new();
        stream.collect_ordered(&mut |item| items.push(item)).await;
        assert!(items.is_empty());
        assert_eq!(RevisableTextStream::terminal_failure(&stream), None);
    }

    /// Replays savepoints recorded before subscription together with the rolled-back snapshot.
    #[tokio::test]
    async fn late_subscriber_replays_active_savepoints() {
        let stream = DelegatingRevisableSharedTextStream::new_ordered(
            mutable_shared_stream(usize::MAX), mutable_shared_stream(usize::MAX),
        );
        stream.emit_revision(TextStreamEvent {
            event_type: TextStreamEventType::Savepoint,
            id: "retry".to_string(),
        });
        stream.emit_chunk("attempt one".to_string());
        stream.emit_revision(TextStreamEvent {
            event_type: TextStreamEventType::Rollback,
            id: "retry".to_string(),
        });
        stream.emit_chunk("attempt two".to_string());
        stream.close();

        let mut consumer = stream.clone();
        let mut items = Vec::new();
        consumer.collect_ordered(&mut |item| items.push(item)).await;

        assert_eq!(
            items,
            vec![
                ResponseStreamItem::Revision(TextStreamEvent {
                    event_type: TextStreamEventType::Savepoint,
                    id: "retry".to_string(),
                }),
                ResponseStreamItem::Chunk("attempt two".to_string()),
            ]
        );
    }

    /// Reproduces the retry rollback race: a savepoint emitted before subscription is
    /// replayed, so a later rollback truncates the partial attempt instead of panicking.
    #[tokio::test]
    async fn late_subscriber_resolves_rollback_from_replayed_savepoint() {
        let stream = DelegatingRevisableSharedTextStream::new_ordered(
            mutable_shared_stream(usize::MAX), mutable_shared_stream(usize::MAX),
        );
        stream.emit_revision(TextStreamEvent {
            event_type: TextStreamEventType::Savepoint,
            id: "retry".to_string(),
        });
        let producer = stream.clone();
        let mut consumer = stream.clone();
        let tracker = Arc::new(Mutex::new(TextStreamRevisionTracker::new("")));
        let workerTracker = tracker.clone();
        let collection = async move {
            let mut content = String::new();
            consumer
                .collect_ordered(&mut |item| match item {
                    ResponseStreamItem::Chunk(chunk) => {
                        workerTracker
                            .lock()
                            .expect("tracker mutex poisoned")
                            .append(&chunk);
                        content.push_str(&chunk);
                    }
                    ResponseStreamItem::Revision(event) => match event.event_type {
                        TextStreamEventType::Savepoint => workerTracker
                            .lock()
                            .expect("tracker mutex poisoned")
                            .savepoint(&event.id),
                        TextStreamEventType::Rollback => {
                            if let Some(rolled) = workerTracker
                                .lock()
                                .expect("tracker mutex poisoned")
                                .rollback(&event.id)
                            {
                                content = rolled.to_owned();
                            }
                        }
                    },
                })
                .await;
        };
        let production = async {
            tokio::task::yield_now().await;
            producer.emit_chunk("discarded attempt".to_string());
            producer.emit_revision(TextStreamEvent {
                event_type: TextStreamEventType::Rollback,
                id: "retry".to_string(),
            });
            producer.emit_chunk("final answer".to_string());
            producer.close();
        };
        tokio::join!(collection, production);
        assert_eq!(
            tracker
                .lock()
                .expect("tracker mutex poisoned")
                .current_content(),
            "final answer"
        );
    }

    /// Orders replayed savepoints by content position rather than by savepoint id.
    #[tokio::test]
    async fn replay_orders_savepoints_by_position() {
        let stream = DelegatingRevisableSharedTextStream::new_ordered(
            mutable_shared_stream(usize::MAX), mutable_shared_stream(usize::MAX),
        );
        stream.emit_chunk("abc".to_string());
        stream.emit_revision(TextStreamEvent {
            event_type: TextStreamEventType::Savepoint,
            id: "b".to_string(),
        });
        stream.emit_chunk("def".to_string());
        stream.emit_revision(TextStreamEvent {
            event_type: TextStreamEventType::Savepoint,
            id: "a".to_string(),
        });
        stream.close();

        let mut consumer = stream.clone();
        let mut items = Vec::new();
        consumer.collect_ordered(&mut |item| items.push(item)).await;

        assert_eq!(
            items,
            vec![
                ResponseStreamItem::Chunk("abc".to_string()),
                ResponseStreamItem::Revision(TextStreamEvent {
                    event_type: TextStreamEventType::Savepoint,
                    id: "b".to_string(),
                }),
                ResponseStreamItem::Chunk("def".to_string()),
                ResponseStreamItem::Revision(TextStreamEvent {
                    event_type: TextStreamEventType::Savepoint,
                    id: "a".to_string(),
                }),
            ]
        );
    }
}
