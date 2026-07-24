use std::collections::{hash_map::Entry, HashMap};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use js_sys::Function;
use operit_core_proxy::RuntimeCoreRouter::RuntimeCorePushTarget;
use operit_link::{
    CoreEventKind, CoreLinkClient, CoreLinkError, CoreLinkSharedClient, CorePushItem,
    CorePushRequest, CoreWatchRequest,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

use crate::{native_watch_event_vec, OperitFlutterBridge};

/// Stores the route and sequence state selected when a client opens one push stream.
#[derive(Clone)]
pub(crate) enum NativePushState {
    Routed {
        target: RuntimeCorePushTarget,
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
                for (_, task) in subscriptions.drain() {
                    task.abort();
                }
            }
        }
    }
}

impl OperitFlutterBridge {
    /// Opens one native client-owned input stream through the runtime-selected Link route.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn pushOpen(&self, request: CorePushRequest) -> Result<String, CoreLinkError> {
        let pushId = request.requestId.0.clone();
        let state = NativePushState::Routed {
            target: self.runtime.block_on(self.proxyCore.openPush(request))?,
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

    /// Opens one wasm client-owned input stream through the runtime-selected Link route.
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn pushOpen(&self, request: CorePushRequest) -> Result<String, CoreLinkError> {
        let pushId = request.requestId.0.clone();
        let state = NativePushState::Routed {
            target: self.proxyCore.openPush(request).await?,
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
        match state {
            NativePushState::Routed { target, .. } => self
                .runtime
                .block_on(self.proxyCore.pushItem(&target, item)),
        }
    }

    /// Dispatches one wasm push item in stream order.
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn pushItem(&self, item: CorePushItem) -> Result<(), CoreLinkError> {
        let state = self.takePushItemState(&item)?;
        let NativePushState::Routed { target, .. } = state;
        self.proxyCore.pushItem(&target, item).await
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
        let NativePushState::Routed { target, .. } = state;
        self.runtime.block_on(self.proxyCore.closePush(target))
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
        let NativePushState::Routed { target, .. } = state;
        self.proxyCore.closePush(target).await
    }

    /// Validates one item sequence and returns its registered transport state.
    fn takePushItemState(&self, item: &CorePushItem) -> Result<NativePushState, CoreLinkError> {
        let mut pushes = self.pushStreams.lock().map_err(|error| {
            CoreLinkError::internal(format!("push stream lock poisoned: {error}"))
        })?;
        let state = pushes
            .get_mut(&item.pushId)
            .ok_or_else(|| CoreLinkError::new("PUSH_NOT_FOUND", "Link push stream not found"))?;
        let NativePushState::Routed { nextSequence, .. } = state;
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
        self.runtime.block_on(CoreLinkSharedClient::watchSnapshot(
            self.proxyCore.as_ref(),
            request,
        ))
    }

    /// Reads one wasm watch snapshot through the runtime-selected route.
    #[cfg(target_arch = "wasm32")]
    #[allow(non_snake_case)]
    pub(crate) async fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self.proxyCore.as_ref(), request).await
    }

    /// Opens one native watch stream and forwards events to the platform channel.
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
        let receiver = self.runtime.block_on(CoreLinkSharedClient::watch(
            self.proxyCore.as_ref(),
            request,
        ))?;
        let channel = self.watchChannel.clone();
        let taskSubscriptionId = subscriptionId.clone();
        let task = self.runtime.spawn(async move {
            let mut receiver = receiver;
            while let Some(event) = receiver.recv().await {
                let completed = event.kind == CoreEventKind::Completed;
                channel.send(native_watch_event_vec(&taskSubscriptionId, event));
                if completed {
                    break;
                }
            }
        });
        let mut subscriptions = self.watchSubscriptions.lock().map_err(|error| {
            CoreLinkError::internal(format!("watch subscription lock poisoned: {error}"))
        })?;
        match subscriptions.entry(subscriptionId.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(task);
                Ok(subscriptionId)
            }
            Entry::Occupied(_) => {
                task.abort();
                Err(CoreLinkError::new(
                    "WATCH_ALREADY_EXISTS",
                    "watch subscription already exists",
                ))
            }
        }
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
                    match CoreLinkSharedClient::watch(self.proxyCore.as_ref(), request).await {
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
            if let Some(task) = subscriptions.remove(subscriptionId) {
                task.abort();
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
