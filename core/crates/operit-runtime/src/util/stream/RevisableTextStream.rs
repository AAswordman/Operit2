use crate::util::stream::HotStream::{
    mutable_shared_stream, share, MutableSharedStreamImpl, SharedStream, StreamStart,
};
use crate::util::stream::Stream::Stream;

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

pub trait RevisableTextStream: Stream<Item = String> + TextStreamEventCarrier {}

pub trait RevisableSharedTextStream: SharedStream<String> + RevisableTextStream {}

pub trait RevisableCharStream: Stream<Item = char> + TextStreamEventCarrier {}

pub trait RevisableTextStreamLike: RevisableTextStream + Send {}

impl<T> RevisableTextStreamLike for T where T: RevisableTextStream + Send {}

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
    pub fn new(
        upstream: S,
        event_channel: MutableSharedStreamImpl<TextStreamEvent>,
    ) -> Self {
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

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        self.upstream.collect(collector);
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

impl<S> RevisableTextStream for DelegatingRevisableTextStream<S> where S: Stream<Item = String> {}

#[derive(Clone, Debug)]
pub struct DelegatingRevisableSharedTextStream {
    pub upstream: MutableSharedStreamImpl<String>,
    pub event_channel: MutableSharedStreamImpl<TextStreamEvent>,
}

impl DelegatingRevisableSharedTextStream {
    pub fn new(
        upstream: MutableSharedStreamImpl<String>,
        event_channel: MutableSharedStreamImpl<TextStreamEvent>,
    ) -> Self {
        Self {
            upstream,
            event_channel,
        }
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

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        self.upstream.collect(collector);
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

impl RevisableTextStream for DelegatingRevisableSharedTextStream {}
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
    pub fn new(
        upstream: S,
        event_channel: MutableSharedStreamImpl<TextStreamEvent>,
    ) -> Self {
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

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        self.upstream.collect(collector);
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
