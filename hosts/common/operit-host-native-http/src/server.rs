//! Native HTTP serving and WebSocket sockets. Routing is supplied by the application.
use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade as AxumUpgrade},
        ConnectInfo, FromRequestParts, State,
    },
    response::IntoResponse,
    Router,
};
use http_body_util::BodyExt;
use operit_host_api::{HostError, HostResult, HttpServer::*};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
pub struct NativeHttpServerHost;
struct Listener(Mutex<Option<TcpListener>>, std::net::SocketAddr);
fn error(e: impl std::fmt::Display) -> HostError {
    HostError::new(e.to_string())
}
#[async_trait]
impl HttpServerHost for NativeHttpServerHost {
    /// Declares the WebSocket upgrades supplied by the native HTTP server.
    fn supportsWebSocketUpgrade(&self) -> bool {
        true
    }

    /// Binds the Host-owned HTTP and WebSocket server socket.
    async fn bind(&self, address: &str) -> HostResult<Arc<dyn HttpServerListener>> {
        let listener = TcpListener::bind(address).await.map_err(error)?;
        let address = listener.local_addr().map_err(error)?;
        Ok(Arc::new(Listener(Mutex::new(Some(listener)), address)))
    }
}
#[async_trait]
impl HttpServerListener for Listener {
    fn localAddress(&self) -> HostResult<std::net::SocketAddr> {
        Ok(self.1)
    }
    async fn serve(
        &self,
        handler: HttpServerHandler,
        shutdown: ServerFuture<()>,
    ) -> HostResult<()> {
        let listener = self
            .0
            .lock()
            .map_err(error)?
            .take()
            .ok_or_else(|| error("Listener already served"))?;
        axum::serve(
            listener,
            Router::new()
                .fallback(dispatch)
                .with_state(handler)
                .into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(error)
    }
}
async fn dispatch(
    State(handler): State<HttpServerHandler>,
    ConnectInfo(remote): ConnectInfo<std::net::SocketAddr>,
    request: axum::extract::Request,
) -> axum::response::Response {
    let (mut parts, body) = request.into_parts();
    parts.extensions.insert(RemoteAddress(remote));
    if let Ok(upgrade) = AxumUpgrade::from_request_parts(&mut parts, &()).await {
        parts
            .extensions
            .insert(WebSocketUpgrade(Arc::new(Mutex::new(Some(Box::new(
                move |handler| {
                    upgrade
                        .on_upgrade(move |socket| handler(Box::new(Socket(socket))))
                        .into_response()
                        .map(|body| body.map_err(error).boxed_unsync())
                },
            ))))));
    }
    handler(ServerRequest::from_parts(
        parts,
        body.map_err(error).boxed_unsync(),
    ))
    .await
    .map(Body::new)
}
struct Socket(WebSocket);
#[async_trait]
impl ServerWebSocket for Socket {
    async fn send(&mut self, message: WebSocketMessage) -> HostResult<()> {
        self.0
            .send(match message {
                WebSocketMessage::Binary(v) => Message::Binary(v),
                WebSocketMessage::Text(v) => Message::Text(v),
                WebSocketMessage::Ping(v) => Message::Ping(v),
                WebSocketMessage::Pong(v) => Message::Pong(v),
                WebSocketMessage::Close(v) => Message::Close(v.map(|(code, reason)| CloseFrame {
                    code,
                    reason: reason.into(),
                })),
            })
            .await
            .map_err(error)
    }
    async fn recv(&mut self) -> Option<HostResult<WebSocketMessage>> {
        self.0.recv().await.map(|result| {
            result
                .map(|message| match message {
                    Message::Binary(v) => WebSocketMessage::Binary(v),
                    Message::Text(v) => WebSocketMessage::Text(v),
                    Message::Ping(v) => WebSocketMessage::Ping(v),
                    Message::Pong(v) => WebSocketMessage::Pong(v),
                    Message::Close(v) => {
                        WebSocketMessage::Close(v.map(|v| (v.code, v.reason.into_owned())))
                    }
                })
                .map_err(error)
        })
    }
}

#[cfg(test)]
mod tests {
    use tokio_tungstenite::tungstenite;
    use super::*;
    use axum::body::Bytes;
    use http_body_util::Full;
    fn response(bytes: Vec<u8>) -> ServerResponse {
        ServerResponse::new(
            Full::new(Bytes::from(bytes))
                .map_err(|never| match never {})
                .boxed_unsync(),
        )
    }
    #[tokio::test]
    async fn serves_http_body_headers_and_releases_listener() {
        let listener = NativeHttpServerHost.bind("127.0.0.1:0").await.unwrap();
        let address = listener.localAddress().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            listener
                .serve(
                    Arc::new(|request| {
                        Box::pin(async move {
                            assert_eq!(request.method(), "POST");
                            assert_eq!(request.uri().path(), "/test");
                            assert_eq!(request.headers()["x-test"], "value");
                            response(
                                request
                                    .into_body()
                                    .collect()
                                    .await
                                    .unwrap()
                                    .to_bytes()
                                    .to_vec(),
                            )
                        })
                    }),
                    Box::pin(async {
                        let _ = stopped.await;
                    }),
                )
                .await
                .unwrap();
        });
        let result = reqwest::Client::new()
            .post(format!("http://{address}/test"))
            .header("x-test", "value")
            .body("test bytes")
            .send()
            .await
            .unwrap();
        assert_eq!(result.text().await.unwrap(), "test bytes");
        stop.send(()).unwrap();
        server.await.unwrap();
        assert!(NativeHttpServerHost
            .bind(&address.to_string())
            .await
            .is_ok());
    }
    #[tokio::test]
    async fn websocket_upgrade_and_binary_round_trip_are_host_owned() {
        let listener = NativeHttpServerHost.bind("127.0.0.1:0").await.unwrap();
        let address = listener.localAddress().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            listener
                .serve(
                    Arc::new(|mut request| {
                        Box::pin(async move {
                            request
                                .extensions_mut()
                                .remove::<WebSocketUpgrade>()
                                .unwrap()
                                .accept(Box::new(|mut socket| {
                                    Box::pin(async move {
                                        let message = socket.recv().await.unwrap().unwrap();
                                        assert!(matches!(message, WebSocketMessage::Binary(_)));
                                        socket.send(message).await.unwrap();
                                        socket.send(WebSocketMessage::Close(None)).await.unwrap();
                                    })
                                }))
                                .unwrap()
                        })
                    }),
                    Box::pin(async {
                        let _ = stopped.await;
                    }),
                )
                .await
                .unwrap();
        });
        tokio::task::spawn_blocking(move || {
            let (mut socket, _) = tungstenite::connect(format!("ws://{address}/ws")).unwrap();
            socket
                .send(tungstenite::Message::Binary(vec![1, 2, 3]))
                .unwrap();
            assert_eq!(
                socket.read().unwrap(),
                tungstenite::Message::Binary(vec![1, 2, 3])
            );
            assert!(matches!(
                socket.read().unwrap(),
                tungstenite::Message::Close(_)
            ));
        })
        .await
        .unwrap();
        stop.send(()).unwrap();
        server.await.unwrap();
    }
    #[tokio::test]
    async fn binding_conflict_and_unserved_drop_release_port() {
        let listener = NativeHttpServerHost.bind("127.0.0.1:0").await.unwrap();
        let address = listener.localAddress().unwrap().to_string();
        assert!(NativeHttpServerHost.bind(&address).await.is_err());
        drop(listener);
        assert!(NativeHttpServerHost.bind(&address).await.is_ok());
    }
}
