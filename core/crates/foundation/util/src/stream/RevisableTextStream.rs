use crate::stream::HotStream::{
    mutable_shared_stream, share, MutableSharedStreamImpl, SharedStream, StreamStart,
};
use crate::stream::Stream::{CollectFuture, Stream};
use std::sync::{Arc, Mutex};

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

pub trait TextStreamEventCarrier {
    fn event_channel(&self) -> &MutableSharedStreamImpl<TextStreamEvent>;
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
}

impl<S> DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    pub fn new(upstream: S, event_channel: MutableSharedStreamImpl<TextStreamEvent>) -> Self {
        Self {
            upstream,
            event_channel,
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
        }
    }

    /// Emits one response chunk to both legacy and ordered subscribers.
    pub fn emit_chunk(&self, chunk: String) {
        if let ResponseItemStream::Ordered(orderedItems) = &self.item_stream {
            orderedItems.emit(ResponseStreamItem::Chunk(chunk.clone()));
        }
        self.upstream.emit(chunk);
    }

    /// Emits one response revision to both legacy and ordered subscribers.
    pub fn emit_revision(&self, event: TextStreamEvent) {
        let ResponseItemStream::Ordered(orderedItems) = &self.item_stream else {
            panic!("revisable response stream must preserve item order");
        };
        orderedItems.emit(ResponseStreamItem::Revision(event.clone()));
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
    fn collect_ordered<'a>(
        &'a mut self,
        collector: &'a mut dyn FnMut(ResponseStreamItem),
    ) -> CollectFuture<'a> {
        match &self.item_stream {
            ResponseItemStream::Ordered(orderedItems) => {
                let mut orderedItems = orderedItems.clone();
                Box::pin(async move {
                    orderedItems.collect(collector).await;
                })
            }
            ResponseItemStream::Chunks => Box::pin(async move {
                self.upstream
                    .collect(&mut |chunk| collector(ResponseStreamItem::Chunk(chunk)))
                    .await;
            }),
        }
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

}
