//! Native HTTP serving and WebSocket sockets. Routing is supplied by the application.
use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade as AxumUpgrade},
        FromRequestParts, State,
    },
    response::IntoResponse,
    Extension, Router,
};
use http_body_util::BodyExt;
use hyper::service::service_fn;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto,
};
use operit_host_api::{HostError, HostResult, HttpServer::*};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
pub struct NativeHttpServerHost;
struct Listener(Mutex<Option<TcpListener>>, std::net::SocketAddr);
/// Converts transport failures into the shared Host error type.
fn error(e: impl std::fmt::Display) -> HostError {
    HostError::new(e.to_string())
}
#[async_trait]
impl HttpServerHost for NativeHttpServerHost {
    /// Declares the WebSocket upgrades supplied by the native HTTP server.
    fn supportsWebSocketUpgrade(&self) -> bool {
        true
    }

    /// Accepts streams from the shared TCP listener.
    fn supportsAcceptedConnections(&self) -> bool {
        true
    }

    /// Uses the same HTTP connection driver as directly bound listeners.
    async fn serveConnection(
        &self,
        connection: Arc<dyn operit_host_api::TcpConnection>,
        handler: HttpServerHandler,
        shutdown: ServerFuture<()>,
    ) -> HostResult<()> {
        let result = serveHttpConnection(
            super::stream::HostStream::new(connection.clone()),
            connection.remote_address(),
            handler,
            shutdown,
        )
        .await;
        connection.close().await;
        result
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
    /// Returns the socket address assigned when binding the listener.
    fn localAddress(&self) -> HostResult<std::net::SocketAddr> {
        Ok(self.1)
    }
    /// Accepts sockets and delegates every connection to the shared driver.
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
        let (stop, _) = tokio::sync::watch::channel(());
        let mut connections = tokio::task::JoinSet::new();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                accepted = listener.accept() => {
                    let (stream, remote) = accepted.map_err(error)?;
                    let handler = handler.clone();
                    let mut stopped = stop.subscribe();
                    connections.spawn(async move {
                        let _ = serveHttpConnection(stream, Some(remote), handler, Box::pin(async move {
                            let _ = stopped.changed().await;
                        })).await;
                    });
                }
                Some(_) = connections.join_next(), if !connections.is_empty() => {}
            }
        }
        drop(listener);
        drop(stop);
        while connections.join_next().await.is_some() {}
        Ok(())
    }
}

/// Drives HTTP/1, HTTP/2 and WebSocket lifetimes for every native server entry.
async fn serveHttpConnection<I>(
    stream: I,
    remote: Option<std::net::SocketAddr>,
    handler: HttpServerHandler,
    shutdown: ServerFuture<()>,
) -> HostResult<()>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    use tower::ServiceExt;
    let (upgrade, mut upgraded) = tokio::sync::watch::channel(UpgradeState::Pending);
    let (_cancel, cancelled) = tokio::sync::watch::channel(());
    let router = Router::new().fallback(dispatch).with_state(handler);
    let service = service_fn(move |request: hyper::Request<hyper::body::Incoming>| {
        let router = router.clone();
        let upgrade = UpgradeCompletion {
            state: upgrade.clone(),
            cancelled: cancelled.clone(),
        };
        async move {
            let mut request = request.map(Body::new);
            request.extensions_mut().insert(upgrade);
            if let Some(remote) = remote {
                request.extensions_mut().insert(RemoteAddress(remote));
            }
            router.oneshot(request).await
        }
    });
    let builder = auto::Builder::new(TokioExecutor::new());
    let serving = builder.serve_connection_with_upgrades(TokioIo::new(stream), service);
    tokio::pin!(serving, shutdown);
    tokio::select! {
        result = &mut serving => result.map_err(error)?,
        _ = &mut shutdown => return Ok(()),
    }
    // Keep the Host task alive through a WebSocket upgrade, including on executors
    // whose runtime ends when this future returns. Dropping _cancel stops the socket.
    if *upgraded.borrow() == UpgradeState::Running {
        tokio::select! {
            _ = async { let _ = upgraded.wait_for(|state| *state == UpgradeState::Finished).await; } => {},
            _ = &mut shutdown => {},
        }
    }
    Ok(())
}
#[derive(Clone, Copy, PartialEq)]
enum UpgradeState {
    Pending,
    Running,
    Finished,
}
#[derive(Clone)]
struct UpgradeCompletion {
    state: tokio::sync::watch::Sender<UpgradeState>,
    cancelled: tokio::sync::watch::Receiver<()>,
}
struct UpgradeFinished(UpgradeCompletion);
impl Drop for UpgradeFinished {
    /// Releases the connection task when the upgraded session ends.
    fn drop(&mut self) {
        self.0.state.send_replace(UpgradeState::Finished);
    }
}
/// Adapts requests and WebSocket upgrades to the platform-independent handler.
async fn dispatch(
    State(handler): State<HttpServerHandler>,
    Extension(completion): Extension<UpgradeCompletion>,
    request: axum::extract::Request,
) -> axum::response::Response {
    let (mut parts, body) = request.into_parts();
    if let Ok(upgrade) = AxumUpgrade::from_request_parts(&mut parts, &()).await {
        parts
            .extensions
            .insert(WebSocketUpgrade(Arc::new(Mutex::new(Some(Box::new(
                move |handler| {
                    completion.state.send_replace(UpgradeState::Running);
                    let failed = completion.clone();
                    upgrade
                        .on_failed_upgrade(move |_| {
                            let _finished = UpgradeFinished(failed);
                        })
                        .on_upgrade(move |socket| async move {
                            let mut cancelled = completion.cancelled.clone();
                            let _finished = UpgradeFinished(completion);
                            tokio::select! {
                                _ = handler(Box::new(Socket(socket))) => {},
                                _ = cancelled.changed() => {},
                            }
                        })
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
    /// Sends an application WebSocket frame through the native socket.
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
    /// Receives one application WebSocket frame from the native socket.
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
    use super::*;
    use axum::body::Bytes;
    use http_body_util::Full;
    use std::time::Duration;
    use tokio_tungstenite::tungstenite;
    /// Builds a complete response body for transport integration checks.
    fn response(bytes: Vec<u8>) -> ServerResponse {
        ServerResponse::new(
            Full::new(Bytes::from(bytes))
                .map_err(|never| match never {})
                .boxed_unsync(),
        )
    }

    struct AcceptedConnection {
        reader: tokio::sync::Mutex<tokio::net::tcp::OwnedReadHalf>,
        writer: tokio::sync::Mutex<tokio::net::tcp::OwnedWriteHalf>,
        remote: std::net::SocketAddr,
    }

    #[async_trait]
    impl operit_host_api::TcpConnection for AcceptedConnection {
        /// Preserves the peer address across the Host stream adapter.
        fn remote_address(&self) -> Option<std::net::SocketAddr> {
            Some(self.remote)
        }

        /// Reads independent chunks without coupling HTTP frames to socket reads.
        async fn read(&self) -> HostResult<Option<Vec<u8>>> {
            use tokio::io::AsyncReadExt;
            let mut bytes = vec![0; 16384];
            let count = self
                .reader
                .lock()
                .await
                .read(&mut bytes)
                .await
                .map_err(error)?;
            bytes.truncate(count);
            Ok((count > 0).then_some(bytes))
        }

        /// Writes all bytes accepted by the Host stream adapter.
        async fn write(&self, bytes: &[u8]) -> HostResult<()> {
            use tokio::io::AsyncWriteExt;
            self.writer
                .lock()
                .await
                .write_all(bytes)
                .await
                .map_err(error)
        }

        /// Releases the accepted socket's write side.
        async fn close(&self) {
            use tokio::io::AsyncWriteExt;
            let _ = self.writer.lock().await.shutdown().await;
        }
    }

    /// Both server entry points multiplex simultaneous HTTP/2 streams on one socket.
    #[tokio::test]
    async fn http2_multiplexes_bound_and_accepted_connections() {
        for accepted in [false, true] {
            let barrier = Arc::new(tokio::sync::Barrier::new(2));
            let handler: HttpServerHandler = Arc::new(move |request| {
                let barrier = barrier.clone();
                Box::pin(async move {
                    assert_eq!(request.version(), hyper::Version::HTTP_2);
                    let remote = request.extensions().get::<RemoteAddress>().unwrap().0;
                    barrier.wait().await;
                    response(remote.to_string().into_bytes())
                })
            });
            let (stop, stopped) = tokio::sync::oneshot::channel();
            let shutdown: ServerFuture<()> = Box::pin(async {
                let _ = stopped.await;
            });
            let (address, server) = if accepted {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let address = listener.local_addr().unwrap();
                let server = tokio::spawn(async move {
                    let (stream, remote) = listener.accept().await.unwrap();
                    let (reader, writer) = stream.into_split();
                    NativeHttpServerHost
                        .serveConnection(
                            Arc::new(AcceptedConnection {
                                reader: tokio::sync::Mutex::new(reader),
                                writer: tokio::sync::Mutex::new(writer),
                                remote,
                            }),
                            handler,
                            shutdown,
                        )
                        .await
                        .unwrap();
                });
                (address, server)
            } else {
                let listener = NativeHttpServerHost.bind("127.0.0.1:0").await.unwrap();
                let address = listener.localAddress().unwrap();
                let server = tokio::spawn(async move {
                    listener.serve(handler, shutdown).await.unwrap();
                });
                (address, server)
            };
            let client = reqwest::Client::builder()
                .http2_prior_knowledge()
                .build()
                .unwrap();
            let url = format!("http://{address}/multiplex");
            // Both streams must reach the handler together on the same pooled connection.
            let first = client.get(&url).send();
            let second = async {
                tokio::task::yield_now().await;
                client.get(&url).send().await
            };
            let (first, second) = tokio::time::timeout(Duration::from_secs(10), async {
                tokio::join!(first, second)
            })
            .await
            .unwrap();
            let first = first.unwrap();
            let second = second.unwrap();
            assert_eq!(first.version(), hyper::Version::HTTP_2);
            assert_eq!(second.version(), hyper::Version::HTTP_2);
            assert_eq!(first.text().await.unwrap(), second.text().await.unwrap());
            stop.send(()).unwrap();
            tokio::time::timeout(Duration::from_secs(10), server)
                .await
                .unwrap()
                .unwrap();
        }
    }

    /// The production client negotiates h2 over TLS without forcing a protocol version.
    #[tokio::test]
    async fn native_client_negotiates_http2_with_tls_alpn() {
        use tokio_rustls::{rustls, TlsAcceptor};
        let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let key = rustls::pki_types::PrivatePkcs8KeyDer::from(certificate.key_pair.serialize_der());
        let mut config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certificate.cert.der().clone()], key.into())
            .unwrap();
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (stream, remote) = listener.accept().await.unwrap();
            let stream = acceptor.accept(stream).await.unwrap();
            assert_eq!(stream.get_ref().1.alpn_protocol(), Some(b"h2".as_slice()));
            serveHttpConnection(
                stream,
                Some(remote),
                Arc::new(|request| {
                    Box::pin(async move {
                        assert_eq!(request.version(), hyper::Version::HTTP_2);
                        response(b"negotiated h2".to_vec())
                    })
                }),
                Box::pin(async {
                    let _ = stopped.await;
                }),
            )
            .await
            .unwrap();
        });
        let client = super::super::buildHttpStreamClient(5, 5, true, true, "", 0).unwrap();
        let response = client
            .get(format!("https://localhost:{}/alpn", address.port()))
            .send()
            .await
            .unwrap();
        assert_eq!(response.version(), hyper::Version::HTTP_2);
        assert_eq!(response.text().await.unwrap(), "negotiated h2");
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .unwrap()
            .unwrap();
    }

    /// Buffered HTTP requests retain headers and bodies and release their port on shutdown.
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
    /// Native WebSocket upgrades preserve binary frames and close cleanly.
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

    struct UpgradeLease(Option<tokio::sync::oneshot::Sender<()>>);

    impl Drop for UpgradeLease {
        /// Reports cancellation when the upgraded handler is dropped.
        fn drop(&mut self) {
            if let Some(finished) = self.0.take() {
                let _ = finished.send(());
            }
        }
    }

    /// Stopping a listener cancels an open upgrade and releases its handler.
    #[tokio::test]
    async fn shutdown_cancels_open_websocket_handler() {
        let listener = NativeHttpServerHost.bind("127.0.0.1:0").await.unwrap();
        let address = listener.localAddress().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let (entered, started) = tokio::sync::oneshot::channel();
        let (released, finished) = tokio::sync::oneshot::channel();
        let session = Arc::new(Mutex::new(Some((entered, released))));
        let server = tokio::spawn(async move {
            listener
                .serve(
                    Arc::new(move |mut request| {
                        let (entered, released) = session.lock().unwrap().take().unwrap();
                        Box::pin(async move {
                            request
                                .extensions_mut()
                                .remove::<WebSocketUpgrade>()
                                .unwrap()
                                .accept(Box::new(move |socket| {
                                    Box::pin(async move {
                                        let _socket = socket;
                                        let _lease = UpgradeLease(Some(released));
                                        let _ = entered.send(());
                                        std::future::pending::<()>().await;
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
        let (_socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}/open"))
            .await
            .unwrap();
        started.await.unwrap();
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            server.await.unwrap();
            finished.await.unwrap();
        })
        .await
        .unwrap();
        assert!(NativeHttpServerHost
            .bind(&address.to_string())
            .await
            .is_ok());
    }

    /// Binding conflicts fail and dropping an unused listener releases its port.
    #[tokio::test]
    async fn binding_conflict_and_unserved_drop_release_port() {
        let listener = NativeHttpServerHost.bind("127.0.0.1:0").await.unwrap();
        let address = listener.localAddress().unwrap().to_string();
        assert!(NativeHttpServerHost.bind(&address).await.is_err());
        drop(listener);
        assert!(NativeHttpServerHost.bind(&address).await.is_ok());
    }
}
