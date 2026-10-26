//! Native host lifecycle execution and browser adapters for the shared Flutter bridge.
//! This module is the only place where the ABI execution style is selected.

use std::future::Future;
use std::sync::mpsc;

use operit_link::CoreLinkError;
#[cfg(target_arch = "wasm32")]
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreLinkSharedClient, CorePushItem, CorePushRequest,
    CoreWatchRequest,
};

#[cfg(target_arch = "wasm32")]
use crate::BridgeCodec::{native_result_vec, native_watch_event_vec, NativeFrame};
#[cfg(target_arch = "wasm32")]
use crate::BridgeTransport::{BridgeStreams, StreamAction, WatchDelivery, WATCH_BATCH_CAPACITY};
use crate::OperitFlutterBridge;
#[cfg(target_arch = "wasm32")]
use std::{
    cell::{Cell, OnceCell, RefCell},
    collections::HashMap,
    rc::Rc,
};

#[cfg(target_arch = "wasm32")]
use js_sys::Function;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

/// Owns lifecycle hooks selected at the host boundary.
pub(crate) struct PlatformBridgeState {
    #[cfg(target_arch = "wasm32")]
    streams: OnceCell<BrowserStreamSession>,
}

#[cfg(target_arch = "wasm32")]
struct BrowserWatch {
    tag: i64,
    callback: Function,
}

#[cfg(target_arch = "wasm32")]
struct BrowserStreamSession {
    streams: Rc<BridgeStreams>,
    callbacks: Rc<RefCell<HashMap<String, BrowserWatch>>>,
    next_tag: Cell<i64>,
}

impl PlatformBridgeState {
    /// Creates the lifecycle state used by the selected host.
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            streams: OnceCell::new(),
        }
    }

    /// Stops native peer services before Core destruction.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn beforeShutdown(&self, bridge: &OperitFlutterBridge) {
        if let Ok(application) = bridge.coreApplication.lock() {
            if let Some(application) = application.as_ref() {
                if let Ok(services) = application.nodeServices() {
                    if let Err(error) = bridge
                        .runHostRuntimeAsyncTask("operit-flutter-node-stop", move || async move {
                            services.peers().stop().await
                        })
                    {
                        operit_util::AppLogger::AppLogger::e("OperitFlutterBridge", &error);
                    }
                }
            }
        }
    }

    /// Stops the browser connection and releases all registered callbacks.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn beforeShutdown(&self, _bridge: &OperitFlutterBridge) {
        if let Some(session) = self.streams.get() {
            session.streams.close();
            session.callbacks.borrow_mut().clear();
        }
    }

    /// Releases registrations owned by the selected host boundary.
    pub(crate) fn releaseHost(&self) {
        crate::PlatformRuntimeFactory::release_host();
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl OperitFlutterBridge {
    /// Executes a prepared shared task on this runtime's required scheduler host.
    pub(crate) fn runHostRuntimeAsyncTask<T, F>(
        &self,
        name: &'static str,
        task: impl FnOnce() -> F + Send + 'static,
    ) -> Result<T, String>
    where
        T: Send + 'static,
        F: Future<Output = T> + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        self.taskScheduler
            .scheduleHostRuntimeAsyncTask(
                name,
                Box::new(move || {
                    Box::pin(async move {
                        use futures_util::FutureExt as _;
                        let outcome = std::panic::AssertUnwindSafe(task()).catch_unwind().await;
                        let result = outcome.map_err(|payload| {
                            format!(
                                "FATAL_CORE_PANIC: {}",
                                crate::BridgeExports::panic_payload_message(payload.as_ref())
                            )
                        });
                        let _ = sender.send(result);
                    })
                }),
            )
            .map_err(|error| error.to_string())?;
        receiver
            .recv()
            .map_err(|error| format!("Runtime task result channel closed: {error}"))?
    }

    /// Executes owner event ingress on the runtime's scheduler for C/JNI callers.
    pub(crate) fn emitRuntimeEvent(&self, encoded: &str) -> String {
        let result =
            crate::PlatformRuntimeFactory::runtimeEventRequest(encoded).and_then(|request| {
                let task = self.prepareCall(request);
                self.runHostRuntimeAsyncTask("operit-flutter-runtime-event", move || task.execute())
                    .map_err(CoreLinkError::internal)?
                    .result
            });
        crate::PlatformRuntimeFactory::runtimeEventResponse(result)
    }
}

#[cfg(target_arch = "wasm32")]
impl OperitFlutterBridge {
    /// Preserves owner event ingress without blocking the browser event loop.
    pub(crate) async fn emitRuntimeEvent(&self, encoded: &str) -> String {
        let result = match crate::PlatformRuntimeFactory::runtimeEventRequest(encoded) {
            Ok(request) => self.prepareCall(request).execute().await.result,
            Err(error) => Err(error),
        };
        crate::PlatformRuntimeFactory::runtimeEventResponse(result)
    }

    /// Adapts the shared local Core API to a browser Promise.
    pub(crate) async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.callShared(request).await
    }
    /// Returns the one browser connection backed by the shared Host stream owner.
    fn streamSession(&self) -> Result<&BrowserStreamSession, CoreLinkError> {
        if self.platform.streams.get().is_none() {
            let (streams, mut events) =
                BridgeStreams::start(self.localCore.clone(), self.taskScheduler.as_ref())?;
            let streams = Rc::new(streams);
            let callbacks = Rc::new(RefCell::new(HashMap::<String, BrowserWatch>::new()));
            let session = BrowserStreamSession {
                streams: streams.clone(),
                callbacks: callbacks.clone(),
                next_tag: Cell::new(1),
            };
            self.platform.streams.set(session).map_err(|_| {
                CoreLinkError::internal("Browser stream session already initialized")
            })?;
            wasm_bindgen_futures::spawn_local(async move {
                let mut batch = Vec::with_capacity(WATCH_BATCH_CAPACITY);
                while events.recv_many(&mut batch, WATCH_BATCH_CAPACITY).await != 0 {
                    for delivery in batch.drain(..) {
                        let callback = callbacks
                            .borrow()
                            .get(&delivery.id)
                            .filter(|entry| entry.tag == delivery.tag)
                            .map(|entry| entry.callback.clone());
                        let Some(callback) = callback else {
                            continue;
                        };
                        let terminal = !matches!(&delivery.event, Ok(Some(_)));
                        if terminal {
                            callbacks.borrow_mut().remove(&delivery.id);
                        }
                        let id = delivery.id.clone();
                        let tag = delivery.tag;
                        for bytes in browser_watch_frames(delivery) {
                            if let Err(error) = callback.call1(
                                &JsValue::NULL,
                                &js_sys::Uint8Array::from(bytes.as_slice()).into(),
                            ) {
                                operit_host_api::logHostError(
                                    "BridgeStreams",
                                    &format!("Browser watch callback failed: {error:?}"),
                                );
                                callbacks.borrow_mut().remove(&id);
                                if let Err(error) = streams
                                    .request(StreamAction::CloseWatch {
                                        id: id.clone(),
                                        tag,
                                    })
                                    .await
                                {
                                    operit_host_api::logHostError(
                                        "BridgeStreams",
                                        &error.to_string(),
                                    );
                                }
                                break;
                            }
                        }
                    }
                }
                callbacks.borrow_mut().clear();
            });
        }
        Ok(self
            .platform
            .streams
            .get()
            .expect("Browser stream session initialized"))
    }

    /// Opens a push in the browser connection's shared stream owner.
    pub(crate) async fn pushOpen(
        &self,
        request: CorePushRequest,
    ) -> Result<Option<String>, CoreLinkError> {
        self.streamSession()?
            .streams
            .request(StreamAction::OpenPush(request))
            .await
    }

    /// Sends one item through its push's ordered mailbox.
    pub(crate) async fn pushItem(&self, item: CorePushItem) -> Result<(), CoreLinkError> {
        self.streamSession()?
            .streams
            .request(StreamAction::PushItem(item))
            .await
            .map(|_| ())
    }

    /// Closes a push after its accepted input items have completed.
    pub(crate) async fn pushClose(&self, id: &str) -> Result<(), CoreLinkError> {
        self.streamSession()?
            .streams
            .request(StreamAction::ClosePush(id.to_string()))
            .await
            .map(|_| ())
    }

    /// Reads a snapshot independently of persistent stream operations.
    pub(crate) async fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self.localCore.as_ref(), request).await
    }

    /// Registers one callback while the shared owner opens and polls its source.
    pub(crate) async fn watchStream(
        &self,
        id: String,
        request: CoreWatchRequest,
        callback: Function,
    ) -> Result<String, CoreLinkError> {
        let session = self.streamSession()?;
        let tag = session.next_tag.get();
        session.next_tag.set(
            tag.checked_add(1)
                .ok_or_else(|| CoreLinkError::internal("Browser stream generation overflowed"))?,
        );
        {
            let mut callbacks = session.callbacks.borrow_mut();
            if callbacks.contains_key(&id) {
                return Err(CoreLinkError::new(
                    "WATCH_ALREADY_EXISTS",
                    "Watch subscription already exists",
                ));
            }
            callbacks.insert(id.clone(), BrowserWatch { tag, callback });
        }
        let result = session
            .streams
            .request(StreamAction::OpenWatch {
                id: id.clone(),
                request,
                tag,
            })
            .await;
        if result.is_err() {
            let mut callbacks = session.callbacks.borrow_mut();
            if callbacks.get(&id).is_some_and(|entry| entry.tag == tag) {
                callbacks.remove(&id);
            }
        }
        result.map(|_| id)
    }

    /// Enqueues cancellation immediately, including while the source is still opening.
    pub(crate) fn closeWatchStream(&self, id: &str) -> Result<(), CoreLinkError> {
        let session = self.streamSession()?;
        let tag = session.callbacks.borrow().get(id).map(|entry| entry.tag);
        let Some(tag) = tag else {
            return Ok(());
        };
        session.streams.submit(StreamAction::CloseWatch {
            id: id.to_string(),
            tag,
        })?;
        session.callbacks.borrow_mut().remove(id);
        Ok(())
    }
}

/// Encodes browser deliveries with the same explicit value, error, and close frame kinds.
#[cfg(target_arch = "wasm32")]
fn browser_watch_frames(delivery: WatchDelivery) -> Vec<Vec<u8>> {
    let frame = |kind, payload: Vec<u8>| {
        let mut bytes = NativeFrame::empty(kind, delivery.tag).into_parts().0;
        bytes.extend(payload);
        bytes
    };
    match delivery.event {
        Ok(Some(event)) => vec![frame(1, native_watch_event_vec(&delivery.id, event))],
        Ok(None) => vec![frame(3, Vec::new())],
        Err(error) => vec![
            frame(2, native_result_vec(Err::<(), _>(error))),
            frame(3, Vec::new()),
        ],
    }
}
