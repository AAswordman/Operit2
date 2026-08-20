use std::cell::RefCell;
use std::collections::HashMap;

use operit_host_api::{
    HostError, HostResult, WebSocketClosedCallback, WebSocketHost, WebSocketMessageCallback,
    WebSocketOpenedCallback, WebSocketRequestData,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{CloseEvent, Event, MessageEvent, WebSocket};

/// Retains one browser WebSocket and all callbacks until the connection closes.
struct WebWebSocketState {
    socket: WebSocket,
    _opened: Closure<dyn FnMut(Event)>,
    _message: Closure<dyn FnMut(MessageEvent)>,
    _closed: Closure<dyn FnMut(CloseEvent)>,
    _error: Closure<dyn FnMut(Event)>,
}

thread_local! {
    static WEB_SOCKETS: RefCell<HashMap<String, WebWebSocketState>> = RefCell::new(HashMap::new());
}

#[derive(Clone, Debug, Default)]
pub struct WebWebSocketHost;

impl WebWebSocketHost {
    /// Creates the browser WebSocket host.
    pub fn new() -> Self {
        Self
    }
}

impl WebSocketHost for WebWebSocketHost {
    /// Opens one browser WebSocket and forwards its lifecycle through callbacks.
    #[allow(non_snake_case)]
    fn openWebSocket(
        &self,
        streamId: String,
        request: WebSocketRequestData,
        onOpened: WebSocketOpenedCallback,
        onMessage: WebSocketMessageCallback,
        onClosed: WebSocketClosedCallback,
    ) -> HostResult<()> {
        WEB_SOCKETS.with(|sockets| {
            let mut sockets = sockets.borrow_mut();
            if sockets.contains_key(&streamId) {
                return Err(HostError::new(format!(
                    "WebSocket is already open: {streamId}"
                )));
            }
            let socket = WebSocket::new(&request.url)
                .map_err(|error| HostError::new(format!("WebSocket connect failed: {error:?}")))?;
            socket.set_binary_type(web_sys::BinaryType::Arraybuffer);
            let opened = Closure::wrap(Box::new(move |_event: Event| {
                onOpened();
            }) as Box<dyn FnMut(Event)>);
            let message = Closure::wrap(Box::new(move |event: MessageEvent| {
                let data = event.data();
                if data.is_instance_of::<js_sys::ArrayBuffer>() {
                    onMessage(js_sys::Uint8Array::new(&data).to_vec());
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            let closedStreamId = streamId.clone();
            let closed = Closure::wrap(Box::new(move |event: CloseEvent| {
                let result = if event.code() == 1000 || event.code() == 1005 {
                    Ok(())
                } else {
                    Err(format!(
                        "WebSocket closed with code {}: {}",
                        event.code(),
                        event.reason()
                    ))
                };
                onClosed(result);
                WEB_SOCKETS.with(|sockets| {
                    sockets.borrow_mut().remove(&closedStreamId);
                });
            }) as Box<dyn FnMut(CloseEvent)>);
            let error = Closure::wrap(Box::new(|_event: Event| {}) as Box<dyn FnMut(Event)>);
            socket.set_onopen(Some(opened.as_ref().unchecked_ref()));
            socket.set_onmessage(Some(message.as_ref().unchecked_ref()));
            socket.set_onclose(Some(closed.as_ref().unchecked_ref()));
            socket.set_onerror(Some(error.as_ref().unchecked_ref()));
            sockets.insert(
                streamId,
                WebWebSocketState {
                    socket,
                    _opened: opened,
                    _message: message,
                    _closed: closed,
                    _error: error,
                },
            );
            Ok(())
        })
    }

    /// Sends one binary message through a browser WebSocket.
    #[allow(non_snake_case)]
    fn sendWebSocketMessage(&self, streamId: &str, message: Vec<u8>) -> HostResult<()> {
        WEB_SOCKETS.with(|sockets| {
            let sockets = sockets.borrow();
            let socket = sockets
                .get(streamId)
                .ok_or_else(|| HostError::new(format!("WebSocket is not open: {streamId}")))?;
            socket
                .socket
                .send_with_u8_array(&message)
                .map_err(|error| HostError::new(format!("WebSocket send failed: {error:?}")))
        })
    }

    /// Closes one browser WebSocket.
    #[allow(non_snake_case)]
    fn closeWebSocket(&self, streamId: &str) -> HostResult<()> {
        WEB_SOCKETS.with(|sockets| {
            let sockets = sockets.borrow();
            let socket = sockets
                .get(streamId)
                .ok_or_else(|| HostError::new(format!("WebSocket is not open: {streamId}")))?;
            socket
                .socket
                .close()
                .map_err(|error| HostError::new(format!("WebSocket close failed: {error:?}")))
        })
    }
}
