use crate::util::stream::HotStream::{MutableSharedStreamImpl, SharedStream};
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
    fn event_channel(&self) -> &dyn SharedStream<TextStreamEvent>;
}

pub trait RevisableTextStream: Stream<Item = String> + TextStreamEventCarrier {}

pub trait RevisableSharedTextStream: SharedStream<String> + RevisableTextStream {}

pub trait RevisableCharStream: Stream<Item = char> + TextStreamEventCarrier {}

pub struct DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    pub upstream: S,
    pub event_channel: MutableSharedStreamImpl<TextStreamEvent>,
}

impl<S> Stream for DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    type Item = String;

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        self.upstream.collect(collector);
    }
}

impl<S> TextStreamEventCarrier for DelegatingRevisableTextStream<S>
where
    S: Stream<Item = String>,
{
    fn event_channel(&self) -> &dyn SharedStream<TextStreamEvent> {
        &self.event_channel
    }
}

impl<S> RevisableTextStream for DelegatingRevisableTextStream<S> where S: Stream<Item = String> {}
