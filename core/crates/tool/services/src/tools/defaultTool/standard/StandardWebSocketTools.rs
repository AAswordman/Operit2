//! Engine-owned WebSocket sessions using the cross-platform networking Host.
#![allow(non_snake_case)]

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use super::ToolArguments::{decodeArguments, RequestArguments};
use crate::tools::ToolInvocation::prepareInvocation;
use crate::tools::ToolResultDataClasses::ToolResultData;
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext,
};
use operit_host_api::{WebSocketHost, WebSocketRequestData};
use operit_plugin_sdk::js_sdk::network::{NetWebSocketEvent, NetWebSocketRequest};
use operit_plugin_sdk::js_sdk::results::WebSocketResultData;

const MAX_EVENTS: usize = 256;
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_SOCKETS: usize = 16;

/// Holds sockets for exactly one authenticated JavaScript engine.
#[derive(Clone, Default)]
pub(crate) struct WebSocketSession(Arc<Mutex<BTreeMap<String, Socket>>>);

/// Owns one Host connection and the bounded events awaiting its consumer.
struct Socket {
    host: Arc<dyn WebSocketHost>,
    id: String,
    state: Arc<Mutex<SocketState>>,
}

/// Retains ordered events and a terminal result without dropping received messages silently.
#[derive(Default)]
struct SocketState {
    events: VecDeque<NetWebSocketEvent>,
    bytes: usize,
    terminal: bool,
    closed: bool,
}

impl SocketState {
    /// Enqueues one event or terminates an overfull receive queue explicitly.
    fn push(&mut self, event: NetWebSocketEvent) {
        if self.terminal {
            return;
        }
        let size = match &event {
            NetWebSocketEvent::Message { text } => text.len(),
            _ => 0,
        };
        if self.events.len() >= MAX_EVENTS || self.bytes + size > MAX_BYTES {
            self.events.push_back(NetWebSocketEvent::Error {
                message: "WebSocket receive queue capacity exceeded".into(),
            });
            self.terminal = true;
            return;
        }
        self.terminal = matches!(
            event,
            NetWebSocketEvent::Closed | NetWebSocketEvent::Error { .. }
        );
        self.bytes += size;
        self.events.push_back(event);
    }
}

impl Drop for Socket {
    /// Releases the Host connection when its engine or explicit handle is disposed.
    fn drop(&mut self) {
        let closed = self.state.lock().expect("WebSocket state poisoned").closed;
        if !closed {
            if let Err(error) = self.host.closeWebSocket(&self.id) {
                operit_util::AppLogger::AppLogger::e("WebSocket", &error.to_string());
            }
        }
    }
}

/// Runs typed WebSocket requests through ordinary tool admission and the captured engine owner.
pub(crate) struct WebSocketToolExecutor;

impl SharedToolExecutor for WebSocketToolExecutor {
    /// Validates the exact SDK request before admitting the networking operation.
    fn prepare<'a>(
        &'a self,
        request: &'a ToolCall,
        context: &'a ToolInvocationContext,
        checkAccess: bool,
    ) -> Result<PreparedToolInvocation<'a>, String> {
        let args: RequestArguments<NetWebSocketRequest> = decodeArguments(request)?;
        Ok(prepareInvocation(
            request,
            checkAccess,
            ToolEffect::WRITE,
            async move {
                context.requirePluginOwner()?;
                let result = context
                    .host()
                    .webSocketSession
                    .request(args.request, context)?;
                Ok(ToolResultData::WebSocketResultData(result))
            },
        ))
    }
}

impl WebSocketSession {
    /// Opens, reads, writes or disposes one engine-local socket handle.
    fn request(
        &self,
        request: NetWebSocketRequest,
        context: &ToolInvocationContext,
    ) -> Result<WebSocketResultData, String> {
        let mut sockets = self.0.lock().map_err(|error| error.to_string())?;
        match request {
            NetWebSocketRequest::Open {
                url,
                connect_timeout_seconds,
            } => {
                let parsed = url::Url::parse(&url).map_err(|error| error.to_string())?;
                if !matches!(parsed.scheme(), "ws" | "wss") || parsed.host_str().is_none() {
                    return Err("WebSocket URL must use ws or wss and contain a host".into());
                }
                if connect_timeout_seconds == 0 || connect_timeout_seconds > 120 {
                    return Err(
                        "WebSocket connect timeout must be between 1 and 120 seconds".into(),
                    );
                }
                if sockets.len() >= MAX_SOCKETS {
                    return Err("WebSocket engine connection limit exceeded".into());
                }
                let host = context
                    .host()
                    .getContext()
                    .webSocketHost
                    .ok_or("WebSocketHost is not registered")?;
                let id = uuid::Uuid::new_v4().to_string();
                let state = Arc::new(Mutex::new(SocketState::default()));
                let opened = state.clone();
                let messages = state.clone();
                let closed = state.clone();
                let messageHost = Arc::downgrade(&host);
                let messageId = id.clone();
                host.openWebSocket(
                    id.clone(),
                    WebSocketRequestData {
                        url,
                        headers: Vec::new(),
                        connectTimeoutSeconds: connect_timeout_seconds,
                        ignoreSsl: false,
                    },
                    Arc::new(move || {
                        opened
                            .lock()
                            .expect("WebSocket state poisoned")
                            .push(NetWebSocketEvent::Opened);
                    }),
                    Arc::new(move |bytes| {
                        let event = match String::from_utf8(bytes) {
                            Ok(text) => NetWebSocketEvent::Message { text },
                            Err(error) => NetWebSocketEvent::Error {
                                message: format!("WebSocket text is not UTF-8: {error}"),
                            },
                        };
                        let mustClose = {
                            let mut state = messages.lock().expect("WebSocket state poisoned");
                            state.push(event);
                            state.terminal
                        };
                        if mustClose {
                            if let Some(host) = messageHost.upgrade() {
                                if let Err(error) = host.closeWebSocket(&messageId) {
                                    operit_util::AppLogger::AppLogger::e(
                                        "WebSocket",
                                        &error.to_string(),
                                    );
                                }
                            }
                        }
                    }),
                    Arc::new(move |result| {
                        let event = match result {
                            Ok(()) => NetWebSocketEvent::Closed,
                            Err(message) => NetWebSocketEvent::Error { message },
                        };
                        let mut state = closed.lock().expect("WebSocket state poisoned");
                        state.closed = true;
                        state.push(event);
                    }),
                )
                .map_err(|error| error.to_string())?;
                sockets.insert(
                    id.clone(),
                    Socket {
                        host,
                        id: id.clone(),
                        state,
                    },
                );
                Ok(WebSocketResultData::Open { handle: id })
            }
            NetWebSocketRequest::Read { handle } => {
                let socket = sockets
                    .get(&handle)
                    .ok_or("Unknown WebSocket handle for this engine")?;
                let mut state = socket.state.lock().map_err(|error| error.to_string())?;
                let events = state.events.drain(..).collect();
                state.bytes = 0;
                Ok(WebSocketResultData::Read { events })
            }
            NetWebSocketRequest::Send { handle, text } => {
                if text.len() > MAX_BYTES {
                    return Err("WebSocket message exceeds 4 MiB".into());
                }
                let socket = sockets
                    .get(&handle)
                    .ok_or("Unknown WebSocket handle for this engine")?;
                if socket
                    .state
                    .lock()
                    .map_err(|error| error.to_string())?
                    .terminal
                {
                    return Err("WebSocket is closed".into());
                }
                socket
                    .host
                    .sendWebSocketText(&socket.id, text)
                    .map_err(|error| error.to_string())?;
                Ok(WebSocketResultData::Send)
            }
            NetWebSocketRequest::Close { handle } => {
                let socket = sockets
                    .get(&handle)
                    .ok_or("Unknown WebSocket handle for this engine")?;
                let closed = socket
                    .state
                    .lock()
                    .map_err(|error| error.to_string())?
                    .closed;
                if !closed {
                    socket
                        .host
                        .closeWebSocket(&socket.id)
                        .map_err(|error| error.to_string())?;
                    socket
                        .state
                        .lock()
                        .map_err(|error| error.to_string())?
                        .closed = true;
                }
                sockets.remove(&handle);
                Ok(WebSocketResultData::Close)
            }
        }
    }
}
