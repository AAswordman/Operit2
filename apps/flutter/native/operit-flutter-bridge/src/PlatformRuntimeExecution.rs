//! Blocking and browser execution adapters for the shared Flutter bridge.
//! This module is the only place where the ABI execution style is selected.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreLinkError, CorePushItem, CoreWatchRequest,
};

use crate::BridgeCodec::native_watch_event_vec;
use crate::OperitFlutterBridge;

#[cfg(target_arch = "wasm32")]
use js_sys::Function;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

/// Owns transport resources selected at the host boundary.
pub(crate) struct PlatformBridgeState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) watchChannel: NativeWatchChannel,
}

impl PlatformBridgeState {
    /// Creates only the transport resources used by the selected host.
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            watchChannel: NativeWatchChannel::new(),
        }
    }

    /// Stops native peer services and closes the native event queue before Core destruction.
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
        self.watchChannel.close();
    }

    /// Leaves browser task cancellation to the shared subscription and Core owners.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn beforeShutdown(&self, _bridge: &OperitFlutterBridge) {}

    /// Releases registrations owned by the selected host boundary.
    pub(crate) fn releaseHost(&self) {
        crate::PlatformRuntimeFactory::release_host();
    }
}

/// Carries native watch events from the async runtime to the platform channel reader.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub(crate) struct NativeWatchChannel {
    sender: mpsc::Sender<NativeWatchChannelMessage>,
    receiver: Arc<Mutex<mpsc::Receiver<NativeWatchChannelMessage>>>,
    closed: Arc<AtomicBool>,
}

/// Represents one queued native watch-channel message.
#[cfg(not(target_arch = "wasm32"))]
enum NativeWatchChannelMessage {
    Event(Vec<u8>),
    Closed,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeWatchChannel {
    /// Creates the native watch-channel queue.
    pub(crate) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Queues one encoded watch event while the channel remains open.
    fn send(&self, frame: Vec<u8>) -> Result<(), CoreLinkError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(CoreLinkError::new(
                "WATCH_CHANNEL_CLOSED",
                "Watch channel is closed",
            ));
        }
        self.sender
            .send(NativeWatchChannelMessage::Event(frame))
            .map_err(|error| CoreLinkError::internal(format!("Watch delivery failed: {error}")))
    }

    /// Closes the native watch-channel queue exactly once.
    pub(crate) fn close(&self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            let _ = self.sender.send(NativeWatchChannelMessage::Closed);
        }
    }

    /// Waits for the next encoded watch event from the queue.
    pub(crate) fn nextEvent(&self) -> Result<Vec<u8>, CoreLinkError> {
        let receiver = self.receiver.lock().map_err(|error| {
            CoreLinkError::internal(format!("watch channel lock poisoned: {error}"))
        })?;
        match receiver.recv() {
            Ok(NativeWatchChannelMessage::Event(frame)) => Ok(frame),
            Ok(NativeWatchChannelMessage::Closed) | Err(_) => Err(CoreLinkError::new(
                "WATCH_CHANNEL_CLOSED",
                "watch channel closed",
            )),
        }
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
                        let _ = sender.send(task().await);
                    })
                }),
            )
            .map_err(|error| error.to_string())?;
        receiver
            .recv()
            .map_err(|error| format!("Runtime task result channel closed: {error}"))
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

    /// Adapts the local Core async API to the blocking C and JNI ABI.
    pub(crate) fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let task = self.prepareCall(request);
        match self.runHostRuntimeAsyncTask("operit-flutter-call", move || task.execute()) {
            Ok(response) => response,
            Err(error) => CoreCallResponse::err(requestId, CoreLinkError::internal(error)),
        }
    }

    /// Executes the shared validated push task for a blocking ABI caller.
    pub(crate) fn pushItem(&self, item: CorePushItem) -> Result<(), CoreLinkError> {
        let task = self.preparePushItem(item)?;
        self.runHostRuntimeAsyncTask("operit-flutter-push-item", move || task.execute())
            .map_err(CoreLinkError::internal)?
    }

    /// Executes the shared push close task for a blocking ABI caller.
    pub(crate) fn pushClose(&self, id: &str) -> Result<(), CoreLinkError> {
        let task = self.preparePushClose(id)?;
        self.runHostRuntimeAsyncTask("operit-flutter-push-close", move || task.execute())
            .map_err(CoreLinkError::internal)?
    }

    /// Executes the shared snapshot task for a blocking ABI caller.
    pub(crate) fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, CoreLinkError> {
        let task = self.prepareWatchSnapshot(request);
        self.runHostRuntimeAsyncTask("operit-flutter-snapshot", move || task.execute())
            .map_err(CoreLinkError::internal)?
    }

    /// Opens the shared source before acknowledging and schedules its ordered frame delivery.
    pub(crate) fn watchStream(
        &self,
        id: String,
        request: CoreWatchRequest,
    ) -> Result<String, CoreLinkError> {
        let task = self.prepareWatchStream(id.clone(), request)?;
        let watch = self
            .runHostRuntimeAsyncTask("operit-flutter-watch-open", move || task.open())
            .map_err(CoreLinkError::internal)??;
        let channel = self.platform.watchChannel.clone();
        self.taskScheduler
            .scheduleHostRuntimeAsyncTask(
                "operit-flutter-watch",
                Box::new(move || {
                    Box::pin(async move {
                        if let Err(error) = watch
                            .forward(move |id, event| {
                                channel.send(native_watch_event_vec(id, event))
                            })
                            .await
                        {
                            operit_util::AppLogger::AppLogger::e(
                                "OperitFlutterBridge",
                                &error.to_string(),
                            );
                        }
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok(id)
    }

    /// Reads one native queue frame without changing the shared watch lifecycle.
    pub(crate) fn nextWatchChannelEvent(&self) -> Result<Vec<u8>, CoreLinkError> {
        self.platform.watchChannel.nextEvent()
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
    /// Executes the same validated push task inside the browser runtime.
    pub(crate) async fn pushItem(&self, item: CorePushItem) -> Result<(), CoreLinkError> {
        self.preparePushItem(item)?.execute().await
    }
    /// Executes the same push close task inside the browser runtime.
    pub(crate) async fn pushClose(&self, id: &str) -> Result<(), CoreLinkError> {
        self.preparePushClose(id)?.execute().await
    }
    /// Executes the same snapshot task inside the browser runtime.
    pub(crate) async fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, CoreLinkError> {
        self.prepareWatchSnapshot(request).execute().await
    }
    /// Delivers the shared ordered watch stream to its explicit JavaScript callback.
    pub(crate) async fn watchStream(
        &self,
        id: String,
        request: CoreWatchRequest,
        callback: Function,
    ) -> Result<String, CoreLinkError> {
        let watch = self.prepareWatchStream(id.clone(), request)?.open().await?;
        wasm_bindgen_futures::spawn_local(async move {
            let result = watch
                .forward(move |id, event| {
                    let bytes = native_watch_event_vec(id, event);
                    callback
                        .call1(
                            &JsValue::NULL,
                            &js_sys::Uint8Array::from(bytes.as_slice()).into(),
                        )
                        .map(|_| ())
                        .map_err(|error| {
                            CoreLinkError::internal(format!(
                                "Browser watch callback failed: {error:?}"
                            ))
                        })
                })
                .await;
            if let Err(error) = result {
                operit_util::AppLogger::AppLogger::e("OperitFlutterBridge", &error.to_string());
            }
        });
        Ok(id)
    }
}
