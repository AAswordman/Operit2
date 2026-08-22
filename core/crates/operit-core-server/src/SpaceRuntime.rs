use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError,
    CoreStreamAttachment, CoreStreamSource, CoreValue, CoreWatchRequest, CORE_STREAM_POOL_OBJECT_ID,
};
use operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

/// Owns the Space-side runtime object registry and embedded stream pool.
#[derive(Clone)]
pub struct SpaceRuntime {
    chatRuntimeHolder: Arc<AsyncMutex<ChatRuntimeHolder>>,
    streamPool: Arc<SpaceStreamPool>,
}

/// Stores sources referenced by embedded stream descriptors returned over Link.
struct SpaceStreamPool {
    sources: Mutex<BTreeMap<String, Arc<CoreStreamSource>>>,
}

impl SpaceStreamPool {
    /// Creates an empty Space-owned stream pool.
    fn new() -> Self {
        Self {
            sources: Mutex::new(BTreeMap::new()),
        }
    }

    /// Adopts one captured embedded stream source.
    fn adopt(&self, attachment: CoreStreamAttachment) {
        let mut sources = self
            .sources
            .lock()
            .expect("Space stream pool mutex poisoned");
        if let Some(existing) = sources.get(&attachment.streamId) {
            if !Arc::ptr_eq(existing, &attachment.source) {
                existing.attachNextSegment(attachment.source);
            }
        } else {
            sources.insert(attachment.streamId, attachment.source);
        }
    }

    /// Removes one source after the corresponding Link watch closes.
    fn remove(&self, streamId: &str) {
        self.sources
            .lock()
            .expect("Space stream pool mutex poisoned")
            .remove(streamId);
    }
}

impl SpaceRuntime {
    /// Creates a Space runtime over the process ChatRuntimeHolder.
    pub fn new(chatRuntimeHolder: Arc<AsyncMutex<ChatRuntimeHolder>>) -> Self {
        Self {
            chatRuntimeHolder,
            streamPool: Arc::new(SpaceStreamPool::new()),
        }
    }

    /// Executes one annotation-addressed Space call on the main runtime slot.
    pub async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let Some(route) = crate::generated_space_call_route(&request) else {
            return CoreCallResponse::err(
                requestId,
                CoreLinkError::new("SPACE_ROUTE_NOT_FOUND", "Space call route is not registered"),
            );
        };
        let (result, attachments) = operit_link::withCoreStreamCapture({
            let holder = self.chatRuntimeHolder.clone();
            async move {
                let mut holder = holder.lock().await;
                let core = holder.getCore(ChatRuntimeSlot::MAIN);
                crate::generated_space_call_on_chat_core(core, request).await
            }
        })
        .await;
        self.adoptAttachments(attachments);
        match result {
            Ok(value) => CoreCallResponse::ok(requestId, value),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    /// Reads one annotation-addressed Space watch snapshot on the main slot.
    pub async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        let Some(_route) = crate::generated_space_watch_route(&request) else {
            return Err(CoreLinkError::new(
                "SPACE_ROUTE_NOT_FOUND",
                "Space watch route is not registered",
            ));
        };
        let requestId = request.requestId.clone();
        let targetObjectId = request.targetObjectId;
        let propertyName = request.propertyName.clone();
        let (result, attachments) = operit_link::withCoreStreamCapture({
            let holder = self.chatRuntimeHolder.clone();
            async move {
                let mut holder = holder.lock().await;
                let core = holder.getCore(ChatRuntimeSlot::MAIN);
                crate::generated_space_watch_snapshot_on_chat_core(core, &request)
            }
        })
        .await;
        self.adoptAttachments(attachments);
        Ok(CoreEvent {
            requestId: Some(requestId),
            targetObjectId,
            propertyName,
            kind: CoreEventKind::Snapshot,
            value: result?,
        })
    }

    /// Opens one annotation-addressed Space watch on the main slot.
    pub async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.targetObjectId == CORE_STREAM_POOL_OBJECT_ID {
            return self.openEmbeddedStream(request);
        }
        let Some(_route) = crate::generated_space_watch_route(&request) else {
            return Err(CoreLinkError::new(
                "SPACE_ROUTE_NOT_FOUND",
                "Space watch route is not registered",
            ));
        };
        let mut holder = self.chatRuntimeHolder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        crate::generated_space_watch_on_chat_core(core, request)
    }

    /// Adopts captured stream sources into the Space-owned pool.
    fn adoptAttachments(&self, attachments: Vec<CoreStreamAttachment>) {
        for attachment in attachments {
            self.streamPool.adopt(attachment);
        }
    }

    /// Opens an embedded response stream referenced by the fixed Link pool object id.
    fn openEmbeddedStream(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.propertyName != "openCoreStream" {
            return Err(CoreLinkError::watchNotFound(&request.registryKey()));
        }
        let mut args = match request.args.clone() {
            CoreValue::Map(value) => value,
            CoreValue::Null => BTreeMap::new(),
            _ => return Err(CoreLinkError::new("INVALID_ARGS", "stream pool arguments must be a map")),
        };
        let streamId: String = decodeArgument(&mut args, "streamId")?;
        let source = self
            .streamPool
            .sources
            .lock()
            .expect("Space stream pool mutex poisoned")
            .get(&streamId)
            .cloned()
            .ok_or_else(|| CoreLinkError::watchNotFound(&request.registryKey()))?;
        source.open(request)
    }
}

/// Decodes one named argument from a Link argument map.
fn decodeArgument<T: DeserializeOwned>(
    args: &mut BTreeMap<String, CoreValue>,
    name: &str,
) -> Result<T, CoreLinkError> {
    operit_link::fromCoreValue(args.remove(name).unwrap_or(CoreValue::Null))
        .map_err(|error| CoreLinkError::new("INVALID_ARGS", format!("{name}: {error}")))
}
