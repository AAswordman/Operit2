use crate::{CoreEventStream, CoreLinkError, CoreValue, CoreWatchRequest};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

thread_local! {
    static SYNC_CORE_STREAM_CAPTURE: RefCell<Option<Vec<CoreStreamAttachment>>> = const { RefCell::new(None) };
    static SYNC_CORE_STREAM_SOURCE_RESOLVER: RefCell<Option<Arc<dyn Fn(&CoreStreamDescriptor) -> Option<Arc<CoreStreamSource>> + Send + Sync>>> = const { RefCell::new(None) };
}

tokio::task_local! {
    static ASYNC_CORE_STREAM_CAPTURE: Arc<Mutex<Vec<CoreStreamAttachment>>>;
}

static NEXT_CORE_STREAM_ID: AtomicU64 = AtomicU64::new(0);

/// Carries one in-process stream source from a serialized Core value into its owning proxy.
#[derive(Clone)]
pub struct CoreStreamAttachment {
    /// Identifies the logical stream represented by the attachment.
    pub streamId: String,
    /// Holds the source without making the protocol crate depend on a concrete stream type.
    pub source: Arc<CoreStreamSource>,
}

/// Opens one stable logical Core stream source for a concrete Link watch request.
#[derive(Clone)]
pub struct CoreStreamSource {
    opener: Arc<dyn Fn(CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> + Send + Sync>,
}

impl CoreStreamSource {
    /// Creates a source backed by one local stream opener.
    pub fn new(
        opener: impl Fn(CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            opener: Arc::new(opener),
        }
    }

    /// Opens one client-facing watch over the stable logical source.
    pub fn open(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        (self.opener)(request)
    }
}

/// Captures in-process stream attachments across one asynchronous local dispatch.
#[allow(non_snake_case)]
pub async fn withCoreStreamCapture<F>(future: F) -> (F::Output, Vec<CoreStreamAttachment>)
where
    F: Future,
{
    let storage = Arc::new(Mutex::new(Vec::new()));
    let result = ASYNC_CORE_STREAM_CAPTURE
        .scope(storage.clone(), future)
        .await;
    let attachments = storage
        .lock()
        .expect("core stream capture mutex poisoned")
        .drain(..)
        .collect();
    (result, attachments)
}

/// Captures in-process stream attachments across one synchronous local dispatch.
#[allow(non_snake_case)]
pub fn withCoreStreamCaptureSync<R>(
    operation: impl FnOnce() -> R,
) -> (R, Vec<CoreStreamAttachment>) {
    SYNC_CORE_STREAM_CAPTURE.with(|capture| {
        let previous = capture.replace(Some(Vec::new()));
        let result = operation();
        let attachments = capture.replace(previous).unwrap_or_default();
        (result, attachments)
    })
}

/// Resolves embedded stream descriptors while decoding one synchronous Core value.
#[allow(non_snake_case)]
pub fn withCoreStreamSourceResolverSync<R>(
    resolver: Arc<dyn Fn(&CoreStreamDescriptor) -> Option<Arc<CoreStreamSource>> + Send + Sync>,
    operation: impl FnOnce() -> R,
) -> R {
    SYNC_CORE_STREAM_SOURCE_RESOLVER.with(|storage| {
        let previous = storage.replace(Some(resolver));
        let result = operation();
        storage.replace(previous);
        result
    })
}

/// Records one source into every active local dispatch capture.
fn recordCoreStreamAttachment(attachment: CoreStreamAttachment) {
    let _ = ASYNC_CORE_STREAM_CAPTURE.try_with(|capture| {
        capture
            .lock()
            .expect("core stream capture mutex poisoned")
            .push(attachment.clone());
    });
    SYNC_CORE_STREAM_CAPTURE.with(|capture| {
        if let Some(attachments) = capture.borrow_mut().as_mut() {
            attachments.push(attachment);
        }
    });
}

/// Resolves a source for one decoded stream descriptor when a resolver is active.
#[allow(non_snake_case)]
fn resolveCoreStreamSource(descriptor: &CoreStreamDescriptor) -> Option<Arc<CoreStreamSource>> {
    SYNC_CORE_STREAM_SOURCE_RESOLVER.with(|storage| {
        storage
            .borrow()
            .as_ref()
            .and_then(|resolver| resolver(descriptor))
    })
}

/// Describes one stream property that the generic Link bridge can subscribe to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreStreamDescriptor {
    /// Identifies one logical stream independently from its current source.
    pub streamId: String,
    /// Identifies the generated object that owns the stream property.
    pub targetObjectId: u32,
    pub propertyName: String,
    pub args: CoreValue,
}
/// Carries a wire descriptor and an opaque local source attachment.
pub struct CoreStream<T> {
    pub descriptor: CoreStreamDescriptor,
    marker: PhantomData<T>,
    source: Option<Arc<CoreStreamSource>>,
}

impl<T> Clone for CoreStream<T> {
    /// Clones the transport descriptor and the local source attachment.
    fn clone(&self) -> Self {
        Self {
            descriptor: self.descriptor.clone(),
            marker: PhantomData,
            source: self.source.clone(),
        }
    }
}

impl<T> fmt::Debug for CoreStream<T> {
    /// Formats only the wire-visible stream descriptor.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CoreStream")
            .field("descriptor", &self.descriptor)
            .finish()
    }
}

impl<T> Serialize for CoreStream<T> {
    /// Serializes the descriptor and records the local source for the owning proxy.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(source) = self.source.as_ref() {
            recordCoreStreamAttachment(CoreStreamAttachment {
                streamId: self.descriptor.streamId.clone(),
                source: source.clone(),
            });
        }
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("$coreStream", &self.descriptor)?;
        map.end()
    }
}

impl<'de, T> Deserialize<'de> for CoreStream<T> {
    /// Deserializes a wire descriptor without creating a local source attachment.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(rename = "$coreStream")]
            descriptor: CoreStreamDescriptor,
        }
        let wire = Wire::deserialize(deserializer)?;
        let source = resolveCoreStreamSource(&wire.descriptor);
        Ok(Self {
            descriptor: wire.descriptor,
            marker: PhantomData,
            source,
        })
    }
}

impl<T, U> PartialEq<CoreStream<U>> for CoreStream<T> {
    /// Compares embedded stream sources without comparing their item marker types.
    fn eq(&self, other: &CoreStream<U>) -> bool {
        self.descriptor == other.descriptor
    }
}

impl<T> CoreStream<T> {
    /// Creates an anonymous stream handle backed by one stable logical source.
    #[allow(non_snake_case)]
    pub fn fromSource(source: Arc<CoreStreamSource>) -> Self {
        let streamId = format!(
            "core-stream-{}",
            NEXT_CORE_STREAM_ID.fetch_add(1, Ordering::Relaxed)
        );
        Self::fromSourceWithId(streamId, source)
    }

    /// Creates a stream handle for one route-owned stable stream identifier.
    pub fn fromSourceWithId(streamId: String, source: Arc<CoreStreamSource>) -> Self {
        Self {
            descriptor: CoreStreamDescriptor {
                streamId: streamId.clone(),
                targetObjectId: crate::CORE_STREAM_POOL_OBJECT_ID,
                propertyName: "openCoreStream".to_string(),
                args: CoreValue::Map(BTreeMap::from([(
                    "streamId".to_string(),
                    CoreValue::String(streamId),
                )])),
            },
            marker: PhantomData,
            source: Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies a Flow-local sync capture still receives streams inside an async call capture.
    #[tokio::test(flavor = "current_thread")]
    async fn sync_capture_records_stream_inside_async_capture() {
        let source = Arc::new(CoreStreamSource::new(|_request| {
            let (_sender, receiver) = CoreEventStream::channel();
            Ok(receiver)
        }));
        let stream =
            CoreStream::<String>::fromSourceWithId("nested-capture-stream".to_string(), source);

        let (sync_attachment_count, async_attachments) = withCoreStreamCapture(async {
            let (_value, sync_attachments) =
                withCoreStreamCaptureSync(|| crate::toCoreValue(stream).unwrap());
            sync_attachments.len()
        })
        .await;

        assert_eq!(sync_attachment_count, 1);
        assert_eq!(async_attachments.len(), 1);
    }
}
