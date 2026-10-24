use futures_util::{stream::FuturesUnordered, StreamExt};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use operit_host_api::{
    HostRuntimeTaskSchedulerHost, PluginSdkIpcEndpoint, PluginSdkIpcHost,
    PluginSdkIpcSessionCallbacks, PluginSdkIpcSessionId,
};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreLinkError, CoreLinkPushSession, CorePushItem,
    CorePushRequest, CoreWatchRequest,
};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};

use crate::frame::{decodePluginSdkIpcMessage, encodePluginSdkIpcMessage, PluginSdkIpcMessage};
use crate::handler::{PluginSdkLinkTarget, SharedPluginSdkLinkTarget};
use crate::surface::PluginSdkSurface;

struct InboundMessage {
    sessionId: PluginSdkIpcSessionId,
    message: PluginSdkIpcMessage,
}

struct ServerState {
    activeRequests: HashMap<(String, String), Arc<AtomicBool>>,
    pushSessions: HashMap<(String, String), Arc<AsyncMutex<Option<Box<dyn CoreLinkPushSession>>>>>,
}

/// Owns the Operit-side forwarding of Core Link traffic for external SDKs.
pub struct PluginSdkIpcServer {
    host: Arc<dyn PluginSdkIpcHost>,
    endpoint: PluginSdkIpcEndpoint,
    target: SharedPluginSdkLinkTarget,
    surface: PluginSdkSurface,
    state: Arc<Mutex<ServerState>>,
    ingress: Arc<Mutex<Option<mpsc::UnboundedSender<InboundMessage>>>>,
    shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
}

impl PluginSdkIpcServer {
    /// Creates a server that forwards every SDK message to one existing Core Link target.
    pub fn new(
        host: Arc<dyn PluginSdkIpcHost>,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
        endpoint: PluginSdkIpcEndpoint,
        target: SharedPluginSdkLinkTarget,
        surface: PluginSdkSurface,
    ) -> Self {
        Self {
            host,
            endpoint,
            target,
            surface,
            state: Arc::new(Mutex::new(ServerState {
                activeRequests: HashMap::new(),
                pushSessions: HashMap::new(),
            })),
            ingress: Arc::new(Mutex::new(None)),
            shutdown: Arc::new(Mutex::new(None)),
            scheduler,
        }
    }

    /// Starts the host listener and one Host-owned task for non-Send Core Link futures.
    pub fn start(&self) -> Result<(), CoreLinkError> {
        let (sender, mut receiver) = mpsc::unbounded_channel::<InboundMessage>();
        {
            let mut ingress = self
                .ingress
                .lock()
                .map_err(|error| CoreLinkError::internal(error.to_string()))?;
            if ingress.is_some() {
                return Err(CoreLinkError::new(
                    "PLUGIN_SDK_IPC_ALREADY_STARTED",
                    "Plugin SDK IPC listener is already running",
                ));
            }
            *ingress = Some(sender.clone());
        }
        let (shutdownSender, shutdownReceiver) = oneshot::channel();
        self.shutdown
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .replace(shutdownSender);
        let server = Arc::new(self.cloneForCallbacks());
        let callbacks = PluginSdkIpcSessionCallbacks::new(
            Arc::new(|_sessionId| {}),
            {
                let server = server.clone();
                Arc::new(move |sessionId, bytes| {
                    server.enqueue(sessionId, bytes);
                })
            },
            {
                let server = server.clone();
                Arc::new(move |sessionId| server.closeSession(&sessionId))
            },
        );
        if let Err(error) = self.host.startListener(self.endpoint.clone(), callbacks) {
            let _ = self.shutdown.lock().map(|mut value| value.take());
            let _ = self.ingress.lock().map(|mut value| value.take());
            return Err(CoreLinkError::internal(error.to_string()));
        }
        let workerServer = server.clone();
        if let Err(error) = self.scheduler.scheduleHostRuntimeAsyncTask(
            "plugin-sdk-ipc",
            Box::new(move || {
                Box::pin(async move {
                    let mut active = FuturesUnordered::new();
                    tokio::pin!(shutdownReceiver);
                    loop {
                        tokio::select! {
                            _ = &mut shutdownReceiver => break,
                            _ = active.next(), if !active.is_empty() => {},
                            inbound = receiver.recv() => {
                                let Some(inbound) = inbound else { break; };
                                let server = workerServer.clone();
                                active.push(async move { server.handleMessage(inbound).await; });
                            }
                        }
                    }
                })
            }),
        ) {
            let _ = self.shutdown.lock().map(|mut value| value.take());
            let _ = self.ingress.lock().map(|mut value| value.take());
            self.host
                .stopListener()
                .map_err(|error| CoreLinkError::internal(error.to_string()))?;
            return Err(CoreLinkError::internal(error.to_string()));
        }
        Ok(())
    }

    /// Stops the listener and terminates the forwarding worker.
    pub fn stop(&self) -> Result<(), CoreLinkError> {
        self.cancelSessionState(None);
        if let Some(shutdown) = self
            .shutdown
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .take()
        {
            let _ = shutdown.send(());
        }
        self.ingress
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .take();
        self.host
            .stopListener()
            .map_err(|error| CoreLinkError::internal(error.to_string()))
    }

    /// Clones the callback-visible state without cloning lifecycle controls.
    fn cloneForCallbacks(&self) -> Self {
        Self {
            host: self.host.clone(),
            endpoint: self.endpoint.clone(),
            target: self.target.clone(),
            surface: self.surface.clone(),
            state: self.state.clone(),
            ingress: self.ingress.clone(),
            shutdown: self.shutdown.clone(),
            scheduler: self.scheduler.clone(),
        }
    }

    /// Decodes and queues one host-carried frame for the worker runtime.
    fn enqueue(&self, sessionId: PluginSdkIpcSessionId, bytes: Vec<u8>) {
        let message = match decodePluginSdkIpcMessage(&bytes) {
            Ok(message) => message,
            Err(error) => {
                self.sendMessage(&sessionId, PluginSdkIpcMessage::ProtocolError { error });
                return;
            }
        };
        let sender = self.ingress.lock().ok().and_then(|value| value.clone());
        if let Some(sender) = sender {
            let _ = sender.send(InboundMessage { sessionId, message });
        }
    }

    /// Handles one queued message in the worker's local async runtime.
    async fn handleMessage(self: Arc<Self>, inbound: InboundMessage) {
        let InboundMessage { sessionId, message } = inbound;
        match message {
            PluginSdkIpcMessage::Call(request) => self.handleCall(sessionId, request).await,
            PluginSdkIpcMessage::WatchOpen {
                subscriptionId,
                request,
            } => {
                self.handleWatchOpen(sessionId, subscriptionId, request)
                    .await
            }
            PluginSdkIpcMessage::WatchSnapshot { request } => {
                self.handleWatchSnapshot(sessionId, request).await
            }
            PluginSdkIpcMessage::WatchClose { subscriptionId, .. } => {
                self.cancelRequest(&sessionId, &subscriptionId);
            }
            PluginSdkIpcMessage::PushOpen(request) => self.handlePushOpen(sessionId, request).await,
            PluginSdkIpcMessage::PushItem(item) => self.handlePushItem(sessionId, item).await,
            PluginSdkIpcMessage::PushClose { pushId } => {
                self.handlePushClose(sessionId, pushId).await
            }
            PluginSdkIpcMessage::CallResponse(_)
            | PluginSdkIpcMessage::WatchOpened { .. }
            | PluginSdkIpcMessage::WatchEvent { .. }
            | PluginSdkIpcMessage::PushOpened { .. }
            | PluginSdkIpcMessage::PushItemResult { .. }
            | PluginSdkIpcMessage::PushClosed { .. }
            | PluginSdkIpcMessage::ProtocolError { .. } => {
                self.sendProtocolError(
                    &sessionId,
                    "CLIENT_MESSAGE_NOT_ALLOWED",
                    "client sent a server-owned Link message",
                );
            }
        }
    }

    /// Forwards one call without interpreting its target object or method.
    async fn handleCall(
        self: Arc<Self>,
        sessionId: PluginSdkIpcSessionId,
        request: CoreCallRequest,
    ) {
        if let Err(error) = self.surface.check(&request.target, &request.methodName) {
            self.sendMessage(
                &sessionId,
                PluginSdkIpcMessage::CallResponse(CoreCallResponse::err(request.requestId, error)),
            );
            return;
        }
        let response = self.target.call(request).await;
        self.sendMessage(&sessionId, PluginSdkIpcMessage::CallResponse(response));
    }

    /// Forwards one watch snapshot and returns its event through the watch envelope.
    async fn handleWatchSnapshot(
        self: Arc<Self>,
        sessionId: PluginSdkIpcSessionId,
        request: CoreWatchRequest,
    ) {
        let subscriptionId = request.requestId.0.clone();
        if let Err(error) = self.surface.check(&request.target, &request.propertyName) {
            self.sendMessage(
                &sessionId,
                PluginSdkIpcMessage::WatchClose {
                    subscriptionId,
                    error: Some(error),
                },
            );
            return;
        }
        match self.target.watchSnapshot(request.clone()).await {
            Ok(mut event) => {
                event.requestId = Some(request.requestId.clone());
                self.sendMessage(
                    &sessionId,
                    PluginSdkIpcMessage::WatchEvent {
                        subscriptionId: subscriptionId.clone(),
                        event,
                    },
                );
                self.sendMessage(
                    &sessionId,
                    PluginSdkIpcMessage::WatchClose {
                        subscriptionId,
                        error: None,
                    },
                );
            }
            Err(error) => self.sendMessage(
                &sessionId,
                PluginSdkIpcMessage::WatchClose {
                    subscriptionId,
                    error: Some(error),
                },
            ),
        }
    }

    /// Opens a Core watch and forwards every event until completion or close.
    async fn handleWatchOpen(
        self: Arc<Self>,
        sessionId: PluginSdkIpcSessionId,
        subscriptionId: String,
        request: CoreWatchRequest,
    ) {
        if let Err(error) = self.surface.check(&request.target, &request.propertyName) {
            self.sendMessage(
                &sessionId,
                PluginSdkIpcMessage::WatchOpened {
                    subscriptionId,
                    result: Err(error),
                },
            );
            return;
        }
        let active = match self.registerRequest(&sessionId, &subscriptionId) {
            Ok(active) => active,
            Err(error) => {
                self.sendMessage(
                    &sessionId,
                    PluginSdkIpcMessage::WatchOpened {
                        subscriptionId,
                        result: Err(error),
                    },
                );
                return;
            }
        };
        let stream = match self.target.watch(request.clone()).await {
            Ok(stream) => stream,
            Err(error) => {
                self.finishRequest(&sessionId, &subscriptionId, &active);
                self.sendMessage(
                    &sessionId,
                    PluginSdkIpcMessage::WatchOpened {
                        subscriptionId,
                        result: Err(error),
                    },
                );
                return;
            }
        };
        self.sendMessage(
            &sessionId,
            PluginSdkIpcMessage::WatchOpened {
                subscriptionId: subscriptionId.clone(),
                result: Ok(()),
            },
        );
        self.forwardStream(sessionId, subscriptionId, request, active, stream)
            .await;
    }

    /// Forwards one opened Core event stream through ordered IPC events.
    async fn forwardStream(
        &self,
        sessionId: PluginSdkIpcSessionId,
        subscriptionId: String,
        request: CoreWatchRequest,
        active: Arc<AtomicBool>,
        mut stream: operit_link::CoreEventStream,
    ) {
        while let Some(mut event) = stream.recv().await {
            if active.load(Ordering::SeqCst) {
                break;
            }
            event.requestId = Some(request.requestId.clone());
            self.sendMessage(
                &sessionId,
                PluginSdkIpcMessage::WatchEvent {
                    subscriptionId: subscriptionId.clone(),
                    event,
                },
            );
        }
        if self.finishRequest(&sessionId, &subscriptionId, &active) {
            self.sendMessage(
                &sessionId,
                PluginSdkIpcMessage::WatchClose {
                    subscriptionId,
                    error: None,
                },
            );
        }
    }

    /// Opens one Core push stream and acknowledges the server-side session.
    async fn handlePushOpen(
        self: Arc<Self>,
        sessionId: PluginSdkIpcSessionId,
        request: CorePushRequest,
    ) {
        let pushId = request.requestId.0.clone();
        if let Err(error) = self.surface.check(&request.target, &request.methodName) {
            self.sendMessage(
                &sessionId,
                PluginSdkIpcMessage::PushOpened {
                    pushId,
                    result: Err(error),
                },
            );
            return;
        }
        match self.target.openPush(request).await {
            Ok(push) => {
                self.state
                    .lock()
                    .expect("Plugin SDK server state mutex poisoned")
                    .pushSessions
                    .insert(
                        (sessionId.0.clone(), pushId.clone()),
                        Arc::new(AsyncMutex::new(Some(push))),
                    );
                self.sendMessage(
                    &sessionId,
                    PluginSdkIpcMessage::PushOpened {
                        pushId,
                        result: Ok(()),
                    },
                );
            }
            Err(error) => self.sendMessage(
                &sessionId,
                PluginSdkIpcMessage::PushOpened {
                    pushId,
                    result: Err(error),
                },
            ),
        }
    }

    /// Delivers one ordered value to a stored Core push session.
    async fn handlePushItem(&self, sessionId: PluginSdkIpcSessionId, item: CorePushItem) {
        let push = self
            .state
            .lock()
            .expect("Plugin SDK server state mutex poisoned")
            .pushSessions
            .get(&(sessionId.0.clone(), item.pushId.clone()))
            .cloned();
        let Some(push) = push else {
            self.sendMessage(
                &sessionId,
                PluginSdkIpcMessage::PushItemResult {
                    pushId: item.pushId,
                    sequence: item.sequence,
                    result: Err(CoreLinkError::new(
                        "PUSH_NOT_FOUND",
                        "Core push is not active",
                    )),
                },
            );
            return;
        };
        let result = {
            let mut guard = push.lock().await;
            match guard.as_mut() {
                Some(session) => session.send(item.args).await,
                None => Err(CoreLinkError::new(
                    "PUSH_CLOSED",
                    "Core push is already closed",
                )),
            }
        };
        self.sendMessage(
            &sessionId,
            PluginSdkIpcMessage::PushItemResult {
                pushId: item.pushId,
                sequence: item.sequence,
                result,
            },
        );
    }

    /// Closes one stored Core push session and acknowledges its result.
    async fn handlePushClose(&self, sessionId: PluginSdkIpcSessionId, pushId: String) {
        let push = self
            .state
            .lock()
            .expect("Plugin SDK server state mutex poisoned")
            .pushSessions
            .remove(&(sessionId.0.clone(), pushId.clone()));
        let result = match push {
            Some(push) => match push.lock().await.take() {
                Some(session) => session.close().await,
                None => Err(CoreLinkError::new(
                    "PUSH_CLOSED",
                    "Core push is already closed",
                )),
            },
            None => Err(CoreLinkError::new(
                "PUSH_NOT_FOUND",
                "Core push is not active",
            )),
        };
        self.sendMessage(
            &sessionId,
            PluginSdkIpcMessage::PushClosed { pushId, result },
        );
    }

    /// Registers one session-local request id.
    fn registerRequest(
        &self,
        sessionId: &PluginSdkIpcSessionId,
        requestId: &str,
    ) -> Result<Arc<AtomicBool>, CoreLinkError> {
        let mut state = self
            .state
            .lock()
            .expect("Plugin SDK server state mutex poisoned");
        let key = (sessionId.0.clone(), requestId.to_string());
        if state.activeRequests.contains_key(&key) {
            return Err(CoreLinkError::new(
                "REQUEST_ID_IN_USE",
                format!("Core request id is active: {requestId}"),
            ));
        }
        let active = Arc::new(AtomicBool::new(false));
        state.activeRequests.insert(key, active.clone());
        Ok(active)
    }

    /// Removes one completed request and reports whether it can publish.
    fn finishRequest(
        &self,
        sessionId: &PluginSdkIpcSessionId,
        requestId: &str,
        active: &Arc<AtomicBool>,
    ) -> bool {
        let mut state = self
            .state
            .lock()
            .expect("Plugin SDK server state mutex poisoned");
        let key = (sessionId.0.clone(), requestId.to_string());
        if state
            .activeRequests
            .get(&key)
            .is_some_and(|value| Arc::ptr_eq(value, active))
        {
            state.activeRequests.remove(&key);
        }
        !active.load(Ordering::SeqCst)
    }

    /// Marks one active watch as closed by its SDK owner.
    fn cancelRequest(&self, sessionId: &PluginSdkIpcSessionId, requestId: &str) {
        if let Some(active) = self
            .state
            .lock()
            .expect("Plugin SDK server state mutex poisoned")
            .activeRequests
            .get(&(sessionId.0.clone(), requestId.to_string()))
        {
            active.store(true, Ordering::SeqCst);
        }
    }

    /// Cancels requests and removes push handles owned by one session.
    fn cancelSessionState(&self, sessionId: Option<&PluginSdkIpcSessionId>) {
        let mut state = self
            .state
            .lock()
            .expect("Plugin SDK server state mutex poisoned");
        for ((owner, _), active) in state.activeRequests.iter() {
            if sessionId.is_none_or(|session| session.0 == *owner) {
                active.store(true, Ordering::SeqCst);
            }
        }
        if let Some(sessionId) = sessionId {
            state
                .pushSessions
                .retain(|(owner, _), _| owner != &sessionId.0);
        } else {
            state.pushSessions.clear();
        }
    }

    /// Handles one SDK session closing at the host layer.
    fn closeSession(&self, sessionId: &PluginSdkIpcSessionId) {
        self.cancelSessionState(Some(sessionId));
    }

    /// Sends one encoded response/event through the host carrier.
    fn sendMessage(&self, sessionId: &PluginSdkIpcSessionId, message: PluginSdkIpcMessage) {
        if let Ok(bytes) = encodePluginSdkIpcMessage(&message) {
            let _ = self.host.send(sessionId, bytes);
        }
    }

    /// Reports a protocol violation without inventing a business response.
    fn sendProtocolError(&self, sessionId: &PluginSdkIpcSessionId, code: &str, message: &str) {
        self.sendMessage(
            sessionId,
            PluginSdkIpcMessage::ProtocolError {
                error: CoreLinkError::new(code, message),
            },
        );
    }
}

impl Drop for PluginSdkIpcServer {
    /// Stops the host listener when the owning Core application is released.
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Keeps the target trait in this module's public API documentation.
#[allow(dead_code)]
fn _target_is_object_safe(_: &dyn PluginSdkLinkTarget) {}
