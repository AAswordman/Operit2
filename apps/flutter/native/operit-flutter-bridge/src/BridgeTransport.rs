use std::collections::{hash_map::Entry, HashMap};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use js_sys::Function;
#[cfg(not(target_arch = "wasm32"))]
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
#[cfg(not(target_arch = "wasm32"))]
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_link::{
    CoreEventKind, CoreLinkClient, CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient,
    CorePushItem, CorePushRequest, CoreWatchRequest,
};
use operit_link_access::{LinkAccessHostConfig, LinkAccessStore, RemoteDeviceInfo, LinkTransportPreference};
use operit_core_server::RuntimeRemoteLinkService::RuntimeRemoteLinkService;
use serde::de::DeserializeOwned;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

use crate::{native_watch_event_vec, OperitFlutterBridge};

/// Stores the route and sequence state selected when a client opens one push stream.
#[derive(Clone)]
pub(crate) enum NativePushState {
    Local {
        session: Arc<tokio::sync::Mutex<Option<Box<dyn CoreLinkPushSession>>>>,
        nextSequence: u64,
    },
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
    fn send(&self, frame: Vec<u8>) {
        if !self.closed.load(Ordering::SeqCst) {
            let _ = self.sender.send(NativeWatchChannelMessage::Event(frame));
        }
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

impl Drop for OperitFlutterBridge {
    /// Closes all bridge-owned watch resources before the bridge is released.
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.watchChannel.close();
            if let Ok(mut subscriptions) = self.watchSubscriptions.lock() {
                for (_, cancelSender) in subscriptions.drain() {
                    let _ = cancelSender.send(());
                }
            }
        }
    }
}

impl OperitFlutterBridge {
    /// Executes one Space control call without entering the local Core proxy dispatcher.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn controlCall(
        &self,
        request: operit_link::CoreCallRequest,
    ) -> operit_link::CoreCallResponse {
        self.runtime.block_on(self.controlCallAsync(request))
    }

    /// Executes one Space control call on the wasm runtime scheduler.
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn controlCall(
        &self,
        request: operit_link::CoreCallRequest,
    ) -> operit_link::CoreCallResponse {
        self.controlCallAsync(request).await
    }

    /// Dispatches one Space control request to the server-owned service facade.
    async fn controlCallAsync(
        &self,
        request: operit_link::CoreCallRequest,
    ) -> operit_link::CoreCallResponse {
        let requestId = request.requestId.clone();
        let result = dispatchSpaceControlCall(
            self.spaceService.clone(),
            self.runtimeStorageHost.clone(),
            request,
        )
        .await;
        match result {
            Ok(value) => operit_link::CoreCallResponse::ok(requestId, value),
            Err(error) => operit_link::CoreCallResponse::err(requestId, error),
        }
    }

    /// Opens one native client-owned input stream directly on the local Core proxy.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn pushOpen(&self, request: CorePushRequest) -> Result<String, CoreLinkError> {
        let pushId = request.requestId.0.clone();
        let session = self.localCore.openPushLocal(request)?;
        let state = NativePushState::Local {
            session: Arc::new(tokio::sync::Mutex::new(Some(session))),
            nextSequence: 0,
        };
        let mut pushes = self.pushStreams.lock().map_err(|error| {
            CoreLinkError::internal(format!("push stream lock poisoned: {error}"))
        })?;
        if pushes.insert(pushId.clone(), state).is_some() {
            return Err(CoreLinkError::new(
                "PUSH_ALREADY_EXISTS",
                "Link push stream already exists",
            ));
        }
        Ok(pushId)
    }

    /// Opens one wasm client-owned input stream directly on the local Core proxy.
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn pushOpen(&self, request: CorePushRequest) -> Result<String, CoreLinkError> {
        let pushId = request.requestId.0.clone();
        let session = self.localCore.openPushLocal(request)?;
        let state = NativePushState::Local {
            session: Arc::new(tokio::sync::Mutex::new(Some(session))),
            nextSequence: 0,
        };
        let mut pushes = self.pushStreams.lock().map_err(|error| {
            CoreLinkError::internal(format!("push stream lock poisoned: {error}"))
        })?;
        if pushes.insert(pushId.clone(), state).is_some() {
            return Err(CoreLinkError::new(
                "PUSH_ALREADY_EXISTS",
                "Link push stream already exists",
            ));
        }
        Ok(pushId)
    }

    /// Dispatches one native push item in stream order.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn pushItem(&self, item: CorePushItem) -> Result<(), CoreLinkError> {
        let state = self.takePushItemState(&item)?;
        let NativePushState::Local { session, .. } = state;
        self.runtime.block_on(async move {
            let mut session = session.lock().await;
            session
                .as_mut()
                .ok_or_else(|| CoreLinkError::new("PUSH_CLOSED", "Link push stream is closed"))?
                .send(item.args)
                .await
        })
    }

    /// Dispatches one wasm push item in stream order.
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn pushItem(&self, item: CorePushItem) -> Result<(), CoreLinkError> {
        let state = self.takePushItemState(&item)?;
        let NativePushState::Local { session, .. } = state;
        let mut session = session.lock().await;
        session
            .as_mut()
            .ok_or_else(|| CoreLinkError::new("PUSH_CLOSED", "Link push stream is closed"))?
            .send(item.args)
            .await
    }

    /// Closes one native client-owned input stream.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn pushClose(&self, pushId: &str) -> Result<(), CoreLinkError> {
        let removed = self
            .pushStreams
            .lock()
            .map_err(|error| {
                CoreLinkError::internal(format!("push stream lock poisoned: {error}"))
            })?
            .remove(pushId);
        let state = removed
            .ok_or_else(|| CoreLinkError::new("PUSH_NOT_FOUND", "Link push stream not found"))?;
        let NativePushState::Local { session, .. } = state;
        self.runtime.block_on(async move {
            let session = session
                .lock()
                .await
                .take()
                .ok_or_else(|| CoreLinkError::new("PUSH_CLOSED", "Link push stream is closed"))?;
            session.close().await
        })
    }

    /// Closes one wasm client-owned input stream.
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn pushClose(&self, pushId: &str) -> Result<(), CoreLinkError> {
        let removed = self
            .pushStreams
            .lock()
            .map_err(|error| {
                CoreLinkError::internal(format!("push stream lock poisoned: {error}"))
            })?
            .remove(pushId);
        let state = removed
            .ok_or_else(|| CoreLinkError::new("PUSH_NOT_FOUND", "Link push stream not found"))?;
        let NativePushState::Local { session, .. } = state;
        let session = session
            .lock()
            .await
            .take()
            .ok_or_else(|| CoreLinkError::new("PUSH_CLOSED", "Link push stream is closed"))?;
        session.close().await
    }

    /// Validates one item sequence and returns its registered transport state.
    fn takePushItemState(&self, item: &CorePushItem) -> Result<NativePushState, CoreLinkError> {
        let mut pushes = self.pushStreams.lock().map_err(|error| {
            CoreLinkError::internal(format!("push stream lock poisoned: {error}"))
        })?;
        let state = pushes
            .get_mut(&item.pushId)
            .ok_or_else(|| CoreLinkError::new("PUSH_NOT_FOUND", "Link push stream not found"))?;
        let NativePushState::Local { nextSequence, .. } = state;
        if item.sequence != *nextSequence {
            return Err(CoreLinkError::new(
                "PUSH_SEQUENCE_MISMATCH",
                format!(
                    "Link push sequence is {}, expected {}",
                    item.sequence, nextSequence
                ),
            ));
        }
        *nextSequence += 1;
        Ok(state.clone())
    }

    /// Reads one native watch snapshot through the runtime-selected route.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(non_snake_case)]
    pub(crate) fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, CoreLinkError> {
        self.runtime
            .block_on(CoreLinkSharedClient::watchSnapshot(self.localCore.as_ref(), request))
    }

    /// Reads one wasm watch snapshot through the runtime-selected route.
    #[cfg(target_arch = "wasm32")]
    #[allow(non_snake_case)]
    pub(crate) async fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self.localCore.as_ref(), request).await
    }

    /// Registers one native watch stream and opens its routed source on the host scheduler.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn watchStream(
        &self,
        subscriptionId: String,
        request: CoreWatchRequest,
    ) -> Result<String, CoreLinkError> {
        {
            let subscriptions = self.watchSubscriptions.lock().map_err(|error| {
                CoreLinkError::internal(format!("watch subscription lock poisoned: {error}"))
            })?;
            if subscriptions.contains_key(&subscriptionId) {
                return Err(CoreLinkError::new(
                    "WATCH_ALREADY_EXISTS",
                    "watch subscription already exists",
                ));
            }
        }
        let (cancelSender, mut cancelReceiver) = tokio::sync::oneshot::channel();
        let mut subscriptions = self.watchSubscriptions.lock().map_err(|error| {
            CoreLinkError::internal(format!("watch subscription lock poisoned: {error}"))
        })?;
        match subscriptions.entry(subscriptionId.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(cancelSender);
            }
            Entry::Occupied(_) => {
                return Err(CoreLinkError::new(
                    "WATCH_ALREADY_EXISTS",
                    "watch subscription already exists",
                ));
            }
        }
        drop(subscriptions);

        let channel = self.watchChannel.clone();
        let taskSubscriptionId = subscriptionId.clone();
        let taskSubscriptions = self.watchSubscriptions.clone();
        let localCore = self.localCore.clone();
        let scheduleResult = HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(
            defaultHostRuntimeTaskSchedulerHost().as_ref(),
            "operit-flutter-watch",
            Box::new(move || {
                Box::pin(async move {
                    let openedReceiver = tokio::select! {
                        _ = &mut cancelReceiver => None,
                        opened = CoreLinkSharedClient::watch(localCore.as_ref(), request) => Some(opened),
                    };
                    let Some(openedReceiver) = openedReceiver else {
                        return;
                    };
                    let mut receiver = match openedReceiver {
                        Ok(receiver) => receiver,
                        Err(error) => {
                            eprintln!(
                                "[FlutterBridgeWatch] source open failed subscription={} error={}",
                                taskSubscriptionId, error
                            );
                            if let Ok(mut subscriptions) = taskSubscriptions.lock() {
                                subscriptions.remove(&taskSubscriptionId);
                            }
                            return;
                        }
                    };
                    loop {
                        let event = tokio::select! {
                            _ = &mut cancelReceiver => None,
                            event = receiver.recv() => event,
                        };
                        let Some(event) = event else {
                            break;
                        };
                        let completed = event.kind == CoreEventKind::Completed;
                        channel.send(native_watch_event_vec(&taskSubscriptionId, event));
                        if completed {
                            break;
                        }
                    }
                    if let Ok(mut subscriptions) = taskSubscriptions.lock() {
                        subscriptions.remove(&taskSubscriptionId);
                    }
                })
            }),
        );
        if let Err(error) = scheduleResult {
            if let Ok(mut subscriptions) = self.watchSubscriptions.lock() {
                subscriptions.remove(&subscriptionId);
            }
            return Err(CoreLinkError::internal(error.to_string()));
        }
        Ok(subscriptionId)
    }

    /// Opens one wasm watch stream and forwards events to the JavaScript callback.
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn watchStream(
        &self,
        subscriptionId: String,
        request: CoreWatchRequest,
        onEvent: Function,
    ) -> Result<String, CoreLinkError> {
        let mut subscriptions = self.watchSubscriptions.lock().map_err(|error| {
            CoreLinkError::internal(format!("watch subscription lock poisoned: {error}"))
        })?;
        match subscriptions.entry(subscriptionId.clone()) {
            Entry::Vacant(entry) => {
                let (cancelSender, mut cancelReceiver) = tokio::sync::oneshot::channel();
                entry.insert(cancelSender);
                drop(subscriptions);
                let receiver =
                    match CoreLinkSharedClient::watch(self.localCore.as_ref(), request).await {
                        Ok(receiver) => receiver,
                        Err(error) => {
                            if let Ok(mut subscriptions) = self.watchSubscriptions.lock() {
                                subscriptions.remove(&subscriptionId);
                            }
                            return Err(error);
                        }
                    };
                let taskSubscriptionId = subscriptionId.clone();
                let taskSubscriptions = self.watchSubscriptions.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut receiver = receiver;
                    loop {
                        let event = tokio::select! {
                            _ = &mut cancelReceiver => None,
                            event = receiver.recv() => event,
                        };
                        let Some(event) = event else {
                            break;
                        };
                        let completed = event.kind == CoreEventKind::Completed;
                        let frame = native_watch_event_vec(&taskSubscriptionId, event);
                        let frame = js_sys::Uint8Array::from(frame.as_slice());
                        let _ = onEvent.call1(&JsValue::NULL, &frame.into());
                        if completed {
                            break;
                        }
                    }
                    if let Ok(mut subscriptions) = taskSubscriptions.lock() {
                        subscriptions.remove(&taskSubscriptionId);
                    }
                });
                Ok(subscriptionId)
            }
            Entry::Occupied(_) => Err(CoreLinkError::new(
                "WATCH_ALREADY_EXISTS",
                "watch subscription already exists",
            )),
        }
    }

    /// Closes one active watch stream on the current platform transport.
    pub(crate) fn closeWatchStream(&self, subscriptionId: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(mut subscriptions) = self.watchSubscriptions.lock() {
            if let Some(cancelSender) = subscriptions.remove(subscriptionId) {
                let _ = cancelSender.send(());
            }
        }
        #[cfg(target_arch = "wasm32")]
        if let Ok(mut subscriptions) = self.watchSubscriptions.lock() {
            if let Some(cancelSender) = subscriptions.remove(subscriptionId) {
                let _ = cancelSender.send(());
            }
        }
    }

    /// Reads the next native watch-channel frame for an FFI caller.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn nextWatchChannelEvent(&self) -> Result<Vec<u8>, CoreLinkError> {
        self.watchChannel.nextEvent()
    }
}

/// Decodes a JSON-shaped argument from the standard CoreValue map.
fn decodeSpaceArgument<T: DeserializeOwned>(
    args: &operit_link::CoreValue,
    name: &str,
) -> Result<T, operit_link::CoreLinkError> {
    let value = operit_link::fromCoreValue::<serde_json::Value>(args.clone())
        .map_err(|error| operit_link::CoreLinkError::new("SPACE_INVALID_ARGS", error.to_string()))?;
    let object = value.as_object().ok_or_else(|| {
        operit_link::CoreLinkError::new("SPACE_INVALID_ARGS", "Space control arguments must be an object")
    })?;
    let argument = object.get(name).cloned().unwrap_or(serde_json::Value::Null);
    serde_json::from_value(argument)
        .map_err(|error| operit_link::CoreLinkError::new("SPACE_INVALID_ARGS", format!("{name}: {error}")))
}

/// Converts one service result into the shared Link value representation.
fn encodeSpaceResult<T: serde::Serialize>(value: T) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {
    operit_link::toCoreValue(value)
        .map_err(|error| operit_link::CoreLinkError::new("SPACE_ENCODE_ERROR", error.to_string()))
}

/// Dispatches the independent Space control object IDs and method names.
async fn dispatchSpaceControlCall(
    service: Arc<RuntimeRemoteLinkService>,
    storage: Arc<dyn operit_host_api::RuntimeStorageHost>,
    request: operit_link::CoreCallRequest,
) -> Result<operit_link::CoreValue, operit_link::CoreLinkError> {
    const RUNTIME_OBJECT_ID: u32 = 0;
    const LINK_ACCESS_OBJECT_ID: u32 = 1;
    let method = request.methodName.as_str();
    if request.targetObjectId == RUNTIME_OBJECT_ID {
        let result = match method {
            "deviceSpace" => encodeSpaceResult(service.deviceSpace()),
            "deviceSpaceTopology" => encodeSpaceResult(service.deviceSpaceTopology()),
            "updateCurrentDeviceUserName" => encodeSpaceResult(service.updateCurrentDeviceUserName(decodeSpaceArgument(&request.args, "userName")?)),
            "renameDeviceSpace" => encodeSpaceResult(service.renameDeviceSpace(decodeSpaceArgument(&request.args, "spaceName")?)),
            "leaveDeviceSpace" => encodeSpaceResult(service.leaveDeviceSpace()),
            "pairedDevicesSnapshot" => encodeSpaceResult(service.pairedDevicesSnapshot()),
            "pairedDeviceOnline" => encodeSpaceResult(service.pairedDeviceOnline(decodeSpaceArgument(&request.args, "deviceId")?)),
            "disconnectDeviceSpaceConnection" => encodeSpaceResult(service.disconnectDeviceSpaceConnection(decodeSpaceArgument(&request.args, "deviceId")?)),
            "removePairedDevice" => encodeSpaceResult(service.removePairedDevice(decodeSpaceArgument(&request.args, "deviceId")?)),
            "startSpaceSync" => encodeSpaceResult(service.startSpaceSync()),
            "joinPairedDeviceSpace" => encodeSpaceResult(service.joinPairedDeviceSpace(decodeSpaceArgument(&request.args, "name")?).await),
            "startPairedRemote" => encodeSpaceResult(service.startPairedRemote(
                decodeSpaceArgument(&request.args, "baseUrl")?,
                decodeSpaceArgument(&request.args, "tokenHash")?,
                decodeSpaceArgument(&request.args, "clientDeviceInfo")?,
            ).await),
            "finishPairedRemote" => encodeSpaceResult(service.finishPairedRemote(
                decodeSpaceArgument(&request.args, "pairingId")?,
                decodeSpaceArgument(&request.args, "pairingCode")?,
                decodeSpaceArgument(&request.args, "name")?,
            ).await),
            "setPairedRemoteTransport" => encodeSpaceResult(service.setPairedRemoteTransport(
                decodeSpaceArgument(&request.args, "name")?,
                decodeSpaceArgument::<LinkTransportPreference>(&request.args, "transport")?,
            )),
            #[cfg(not(target_arch = "wasm32"))]
            "discoverSpaces" => encodeSpaceResult(service.discoverSpaces(decodeSpaceArgument(&request.args, "timeoutMs")?).await),
            _ => Err(operit_link::CoreLinkError::new("SPACE_METHOD_NOT_FOUND", format!("Unknown Space method: {method}"))),
        }?;
        return Ok(result);
    }
    if request.targetObjectId == LINK_ACCESS_OBJECT_ID {
        let store = LinkAccessStore::new(storage);
        return match method {
            "initializeIdentity" => encodeSpaceResult(store.initializeIdentity(decodeSpaceArgument::<RemoteDeviceInfo>(&request.args, "deviceInfo")?)),
            "initializeHostConfig" => encodeSpaceResult(store.initializeHostConfig()),
            "saveHostConfig" => encodeSpaceResult(store.saveHostConfig(decodeSpaceArgument::<LinkAccessHostConfig>(&request.args, "config")?)),
            _ => Err(operit_link::CoreLinkError::new("SPACE_METHOD_NOT_FOUND", format!("Unknown Link Access method: {method}"))),
        };
    }
    Err(operit_link::CoreLinkError::new("SPACE_OBJECT_NOT_FOUND", format!("Unknown Space object id: {}", request.targetObjectId)))
}
