use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use operit_host_api::{
    PluginSdkIpcEndpoint, PluginSdkIpcHost, PluginSdkIpcSessionCallbacks,
    PluginSdkIpcSessionId,
};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CorePushItem, CorePushRequest,
    CoreValue, CoreWatchRequest,
};
use tokio::sync::{mpsc, oneshot};

use crate::frame::{decodePluginSdkIpcMessage, encodePluginSdkIpcMessage, PluginSdkIpcMessage};

enum PendingCall {
    /// Receives one call response.
    Call(oneshot::Sender<CoreCallResponse>),
    /// Receives the first event of one snapshot request.
    Snapshot(oneshot::Sender<Result<CoreEvent, CoreLinkError>>),
}

struct ClientState {
    host: Arc<dyn PluginSdkIpcHost>,
    sessionId: Option<PluginSdkIpcSessionId>,
    pendingCalls: BTreeMap<String, PendingCall>,
    pendingWatches: BTreeMap<String, oneshot::Sender<Result<(), CoreLinkError>>>,
    pendingPushOpens: BTreeMap<String, oneshot::Sender<Result<(), CoreLinkError>>>,
    pendingPushItems: BTreeMap<(String, u64), oneshot::Sender<Result<(), CoreLinkError>>>,
    pendingPushCloses: BTreeMap<String, oneshot::Sender<Result<(), CoreLinkError>>>,
    watches: BTreeMap<String, mpsc::UnboundedSender<CoreEvent>>,
    nextRequest: u64,
}

/// Implements the external SDK side of the canonical Core Link IPC envelope.
#[derive(Clone)]
pub struct PluginSdkClient {
    state: Arc<Mutex<ClientState>>,
}

impl PluginSdkClient {
    /// Activates and connects to Operit through the selected platform host.
    pub fn connect(
        host: Arc<dyn PluginSdkIpcHost>,
        endpoint: PluginSdkIpcEndpoint,
    ) -> Result<Self, CoreLinkError> {
        let state = Arc::new(Mutex::new(ClientState {
            host: host.clone(),
            sessionId: None,
            pendingCalls: BTreeMap::new(),
            pendingWatches: BTreeMap::new(),
            pendingPushOpens: BTreeMap::new(),
            pendingPushItems: BTreeMap::new(),
            pendingPushCloses: BTreeMap::new(),
            watches: BTreeMap::new(),
            nextRequest: 1,
        }));
        let callbacks = PluginSdkIpcSessionCallbacks::new(
            {
                let state = state.clone();
                Arc::new(move |sessionId| {
                    if let Ok(mut guard) = state.lock() {
                        guard.sessionId = Some(sessionId);
                    }
                })
            },
            {
                let state = state.clone();
                Arc::new(move |_sessionId, bytes| dispatchIncoming(&state, bytes))
            },
            {
                let state = state.clone();
                Arc::new(move |_sessionId| failPending(&state, "Plugin SDK IPC session closed"))
            },
        );
        let sessionId = host
            .connect(endpoint, callbacks)
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        state
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .sessionId = Some(sessionId);
        Ok(Self { state })
    }

    /// Closes the shared SDK session and rejects its outstanding operations.
    pub fn close(&self) -> Result<(), CoreLinkError> {
        let (host, session_id) = {
            let mut state = self.state.lock().map_err(|e| CoreLinkError::internal(e.to_string()))?;
            let session_id = state.sessionId.take().ok_or_else(|| CoreLinkError::internal("Plugin SDK session is closed"))?;
            (state.host.clone(), session_id)
        };
        failPending(&self.state, "Plugin SDK IPC session closed");
        host.closeSession(&session_id).map_err(|e| CoreLinkError::internal(e.to_string()))
    }

    /// Allocates a request id for a generated SDK method.
    pub fn nextRequestId(&self) -> Result<String, CoreLinkError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        let requestId = format!("plugin-sdk-{}", guard.nextRequest);
        guard.nextRequest += 1;
        Ok(requestId)
    }

    /// Sends one Link envelope through the host-owned carrier.
    fn sendMessage(&self, message: PluginSdkIpcMessage) -> Result<(), CoreLinkError> {
        let (host, sessionId) = {
            let guard = self
                .state
                .lock()
                .map_err(|error| CoreLinkError::internal(error.to_string()))?;
            let sessionId = guard
                .sessionId
                .clone()
                .ok_or_else(|| CoreLinkError::internal("Plugin SDK IPC session is missing"))?;
            (guard.host.clone(), sessionId)
        };
        let bytes = encodePluginSdkIpcMessage(&message)?;
        host.send(&sessionId, bytes)
            .map_err(|error| CoreLinkError::internal(error.to_string()))
    }

    /// Removes one watch subscription from the local pending table.
    fn removeWatch(&self, subscriptionId: &str) {
        if let Ok(mut guard) = self.state.lock() {
            guard.watches.remove(subscriptionId);
        }
    }

    /// Executes one call request through the external process connection.
    pub async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.0.clone();
        let (sender, receiver) = oneshot::channel();
        if let Ok(mut guard) = self.state.lock() {
            guard
                .pendingCalls
                .insert(requestId.clone(), PendingCall::Call(sender));
        } else {
            return CoreCallResponse::err(
                request.requestId,
                CoreLinkError::internal("Plugin SDK IPC client state is poisoned"),
            );
        }
        if let Err(error) = self.sendMessage(PluginSdkIpcMessage::Call(request.clone())) {
            self.removePendingCall(&requestId);
            return CoreCallResponse::err(request.requestId, error);
        }
        receiver.await.unwrap_or_else(|_| {
            CoreCallResponse::err(
                request.requestId,
                CoreLinkError::internal("Plugin SDK call response channel closed"),
            )
        })
    }

    /// Reads one watch snapshot through the external process connection.
    #[allow(non_snake_case)]
    pub async fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let requestId = request.requestId.0.clone();
        let (sender, receiver) = oneshot::channel();
        self.state
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .pendingCalls
            .insert(requestId.clone(), PendingCall::Snapshot(sender));
        if let Err(error) = self.sendMessage(PluginSdkIpcMessage::WatchSnapshot { request }) {
            self.removePendingCall(&requestId);
            return Err(error);
        }
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }

    /// Opens one watch stream and waits for the server-side Core target to open.
    pub async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let subscriptionId = request.requestId.0.clone();
        let (openSender, openReceiver) = oneshot::channel();
        let (eventSender, stream) = CoreEventStream::channel();
        self.state
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .pendingWatches
            .insert(subscriptionId.clone(), openSender);
        self.state
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .watches
            .insert(subscriptionId.clone(), eventSender);
        if let Err(error) = self.sendMessage(PluginSdkIpcMessage::WatchOpen {
            subscriptionId: subscriptionId.clone(),
            request,
        }) {
            self.removePendingWatch(&subscriptionId);
            self.removeWatch(&subscriptionId);
            return Err(error);
        }
        if let Err(error) = openReceiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
        {
            self.removeWatch(&subscriptionId);
            return Err(error);
        }
        let client = self.clone();
        Ok(stream.withOnClose(move || {
            client.removeWatch(&subscriptionId);
            let _ = client.sendMessage(PluginSdkIpcMessage::WatchClose {
                subscriptionId,
                error: None,
            });
        }))
    }

    /// Opens one caller-owned push stream and waits for its server-side handle.
    #[allow(non_snake_case)]
    pub async fn openPush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let pushId = request.requestId.0.clone();
        let (sender, receiver) = oneshot::channel();
        self.state
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .pendingPushOpens
            .insert(pushId.clone(), sender);
        if let Err(error) = self.sendMessage(PluginSdkIpcMessage::PushOpen(request)) {
            self.removePendingPushOpen(&pushId);
            return Err(error);
        }
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))??;
        Ok(Box::new(PluginSdkPushSession {
            client: self.clone(),
            pushId,
            sequence: 0,
        }))
    }

    /// Removes one pending call response after a carrier send failure.
    fn removePendingCall(&self, requestId: &str) {
        if let Ok(mut guard) = self.state.lock() {
            guard.pendingCalls.remove(requestId);
        }
    }

    /// Removes one pending watch-open response after a carrier send failure.
    fn removePendingWatch(&self, subscriptionId: &str) {
        if let Ok(mut guard) = self.state.lock() {
            guard.pendingWatches.remove(subscriptionId);
        }
    }

    /// Removes one pending push-open response after a carrier send failure.
    fn removePendingPushOpen(&self, pushId: &str) {
        if let Ok(mut guard) = self.state.lock() {
            guard.pendingPushOpens.remove(pushId);
        }
    }
}

#[async_trait(?Send)]
impl CoreLinkClient for PluginSdkClient {
    /// Executes one Core call through IPC.
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        PluginSdkClient::call(self, request).await
    }

    /// Reads one Core watch snapshot through IPC.
    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        PluginSdkClient::watchSnapshot(self, request).await
    }

    /// Opens one Core watch stream through IPC.
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        PluginSdkClient::watch(self, request).await
    }

    /// Opens one Core push stream through IPC.
    #[allow(non_snake_case)]
    async fn openPush(
        &mut self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        PluginSdkClient::openPush(self, request).await
    }
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for PluginSdkClient {
    /// Executes one Core call through IPC without requiring mutable ownership.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        PluginSdkClient::call(self, request).await
    }

    /// Reads one Core watch snapshot through IPC without requiring mutable ownership.
    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        PluginSdkClient::watchSnapshot(self, request).await
    }

    /// Opens one Core watch stream through IPC without requiring mutable ownership.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        PluginSdkClient::watch(self, request).await
    }

    /// Opens one IPC input stream without requiring mutable client ownership.
    async fn openPush(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn operit_link::CoreLinkPushSession>, operit_link::CoreLinkError> {
        PluginSdkClient::openPush(self, request).await
    }
}

struct PluginSdkPushSession {
    client: PluginSdkClient,
    pushId: String,
    sequence: u64,
}

#[async_trait]
impl CoreLinkPushSession for PluginSdkPushSession {
    /// Sends one typed value and waits for the Core push acknowledgement.
    async fn send(&mut self, value: CoreValue) -> Result<(), CoreLinkError> {
        let sequence = self.sequence;
        self.sequence += 1;
        let (sender, receiver) = oneshot::channel();
        self.client
            .state
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .pendingPushItems
            .insert((self.pushId.clone(), sequence), sender);
        if let Err(error) = self.client.sendMessage(PluginSdkIpcMessage::PushItem(CorePushItem {
            pushId: self.pushId.clone(),
            sequence,
            args: value,
        })) {
            self.client
                .state
                .lock()
                .map_err(|error| CoreLinkError::internal(error.to_string()))?
                .pendingPushItems
                .remove(&(self.pushId.clone(), sequence));
            return Err(error);
        }
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }

    /// Closes the push stream and waits for the Core close acknowledgement.
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError> {
        let (sender, receiver) = oneshot::channel();
        self.client
            .state
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .pendingPushCloses
            .insert(self.pushId.clone(), sender);
        if let Err(error) = self.client.sendMessage(PluginSdkIpcMessage::PushClose {
            pushId: self.pushId.clone(),
        }) {
            self.client
                .state
                .lock()
                .map_err(|lockError| CoreLinkError::internal(lockError.to_string()))?
                .pendingPushCloses
                .remove(&self.pushId);
            return Err(error);
        }
        receiver
            .await
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
    }
}

/// Routes one incoming IPC frame to its matching SDK operation.
fn dispatchIncoming(state: &Arc<Mutex<ClientState>>, bytes: Vec<u8>) {
    let Ok(message) = decodePluginSdkIpcMessage(&bytes) else {
        return;
    };
    let Ok(mut guard) = state.lock() else {
        return;
    };
    match message {
        PluginSdkIpcMessage::CallResponse(response) => {
            if let Some(PendingCall::Call(sender)) = guard.pendingCalls.remove(&response.requestId.0)
            {
                let _ = sender.send(response);
            }
        }
        PluginSdkIpcMessage::WatchOpened {
            subscriptionId,
            result,
        } => {
            if let Some(sender) = guard.pendingWatches.remove(&subscriptionId) {
                let _ = sender.send(result);
            }
        }
        PluginSdkIpcMessage::WatchEvent {
            subscriptionId,
            event,
        } => {
            if let Some(PendingCall::Snapshot(sender)) = guard.pendingCalls.remove(&subscriptionId) {
                let _ = sender.send(Ok(event));
            } else if let Some(watch) = guard.watches.get(&subscriptionId) {
                let _ = watch.send(event);
            }
        }
        PluginSdkIpcMessage::WatchClose {
            subscriptionId,
            error,
        } => {
            guard.watches.remove(&subscriptionId);
            let closeError = error.unwrap_or_else(|| {
                CoreLinkError::new("WATCH_CLOSED", "Core watch closed")
            });
            if let Some(sender) = guard.pendingWatches.remove(&subscriptionId) {
                let _ = sender.send(Err(closeError.clone()));
            }
            if let Some(PendingCall::Snapshot(sender)) = guard.pendingCalls.remove(&subscriptionId) {
                let _ = sender.send(Err(closeError));
            }
        }
        PluginSdkIpcMessage::PushOpened { pushId, result } => {
            if let Some(sender) = guard.pendingPushOpens.remove(&pushId) {
                let _ = sender.send(result);
            }
        }
        PluginSdkIpcMessage::PushItemResult {
            pushId,
            sequence,
            result,
        } => {
            if let Some(sender) = guard.pendingPushItems.remove(&(pushId, sequence)) {
                let _ = sender.send(result);
            }
        }
        PluginSdkIpcMessage::PushClosed { pushId, result } => {
            if let Some(sender) = guard.pendingPushCloses.remove(&pushId) {
                let _ = sender.send(result);
            }
        }
        PluginSdkIpcMessage::ProtocolError { .. }
        | PluginSdkIpcMessage::Call(_)
        | PluginSdkIpcMessage::WatchOpen { .. }
        | PluginSdkIpcMessage::WatchSnapshot { .. }
        | PluginSdkIpcMessage::PushOpen(_)
        | PluginSdkIpcMessage::PushItem(_)
        | PluginSdkIpcMessage::PushClose { .. } => {}
    }
}

/// Fails all pending operations when the host closes the IPC session.
fn failPending(state: &Arc<Mutex<ClientState>>, message: &str) {
    let Ok(mut guard) = state.lock() else {
        return;
    };
    guard.watches.clear();
    let error = || CoreLinkError::internal(message.to_string());
    let pendingCalls = std::mem::take(&mut guard.pendingCalls);
    for (requestId, pending) in pendingCalls {
        match pending {
            PendingCall::Call(sender) => {
                let _ = sender.send(CoreCallResponse::err(
                    operit_link::CoreRequestId::new(requestId),
                    error(),
                ));
            }
            PendingCall::Snapshot(sender) => {
                let _ = sender.send(Err(error()));
            }
        }
    }
    for (_, sender) in std::mem::take(&mut guard.pendingWatches) {
        let _ = sender.send(Err(error()));
    }
    for (_, sender) in std::mem::take(&mut guard.pendingPushOpens) {
        let _ = sender.send(Err(error()));
    }
    for (_, sender) in std::mem::take(&mut guard.pendingPushItems) {
        let _ = sender.send(Err(error()));
    }
    for (_, sender) in std::mem::take(&mut guard.pendingPushCloses) {
        let _ = sender.send(Err(error()));
    }
}
