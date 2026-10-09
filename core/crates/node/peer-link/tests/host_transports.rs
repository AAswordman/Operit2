//! Real Host sockets, exercising transport rather than a recording substitute.
use operit_host_api::HttpServer::HttpServerHost;
use operit_host_api::{HostManager::HostManager, *};
use operit_host_native_http::{NativeHttpHost, NativeHttpServerHost};
use operit_link::*;
use operit_peer_link::*;
use std::{sync::Arc, time::Duration};

struct Scheduler;
impl HostRuntimeTaskSchedulerHost for Scheduler {
    /// Reads the test Host's process-local monotonic timer clock.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        Ok(START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64)
    }


    fn scheduleHostRuntimeTask(&self, _: &str, task: HostRuntimeTask) -> HostResult<()> {
        std::thread::spawn(task);
        Ok(())
    }
    fn scheduleHostRuntimeAsyncTask(&self, _: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(task());
        });
        Ok(())
    }

    /// Executes cooperative work on the host-owned asynchronous executor.
    fn scheduleHostRuntimeCooperativeAsyncTask(&self, _: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(task());
        });
        Ok(())
    }
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _: &str,
        delay: u64,
        task: HostRuntimeTask,
    ) -> HostResult<()> {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(delay));
            task();
        });
        Ok(())
    }
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        Box::pin(async {
            tokio::task::yield_now().await;
            Ok(())
        })
    }
    fn waitForHostRuntimeDelay(&self, ms: u64) -> HostRuntimeTurnFuture {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(ms)).await;
            Ok(())
        })
    }
}
fn host() -> Arc<HostManager> {
    let network = Arc::new(NativeHttpHost::new());
    Arc::new(HostManager {
        httpHost: Some(network.clone()),
        webSocketHost: Some(network),
        httpServerHost: Some(Arc::new(NativeHttpServerHost)),
        hostRuntimeTaskSchedulerHost: Some(Arc::new(Scheduler)),
        ..HostManager::default()
    })
}
async fn address() -> String {
    let temporary = NativeHttpServerHost.bind("127.0.0.1:0").await.unwrap();
    temporary.localAddress().unwrap().to_string()
}
async fn pair(
    transport: PeerTransport,
) -> (
    Arc<dyn PeerConnection>,
    Arc<dyn PeerConnection>,
    Arc<dyn PeerListener>,
    String,
) {
    let host = host();
    let bind = address().await;
    let server = HostPeerLink::default()
        .listen(
            host.clone(),
            PeerEndpoint {
                nodeId: "server".into(),
                address: bind.clone(),
            },
            transport,
        )
        .await
        .unwrap();
    let scheme = if transport == PeerTransport::Http {
        "http"
    } else {
        "ws"
    };
    let outgoing = HostPeerLink::default()
        .connect(
            host,
            PeerEndpoint {
                nodeId: "client".into(),
                address: String::new(),
            },
            PeerEndpoint {
                nodeId: "server".into(),
                address: format!("{scheme}://{bind}/link"),
            },
            transport,
        )
        .await
        .unwrap();
    let incoming = server.accept().await.unwrap().unwrap();
    (outgoing, incoming, server, bind)
}
async fn exchange(transport: PeerTransport) {
    let (a, b, server, bind) = pair(transport).await;
    assert_eq!(a.transport(), transport);
    assert_eq!(b.transport(), transport);
    // Advertised node IDs are not authentication evidence; inbound metadata stays untrusted.
    assert!(b.target().nodeId.is_empty());
    let calls = vec![
        CoreLinkRequest::Call(CoreCallRequest::new(
            "c",
            "test",
            "echo",
            CoreValue::String("hello".into()),
        )),
        CoreLinkRequest::Watch(CoreLinkWatchRequest::Open(CoreWatchRequest::new(
            "w",
            "test",
            "events",
            CoreValue::Null,
        ))),
        CoreLinkRequest::Watch(CoreLinkWatchRequest::Close {
            requestId: CoreRequestId::new("w"),
        }),
        CoreLinkRequest::Push(CoreLinkPushRequestMessage::Open(CorePushRequest::new(
            "p", "test", "input",
        ))),
        CoreLinkRequest::Push(CoreLinkPushRequestMessage::Item(CorePushItem {
            pushId: "p".into(),
            sequence: 0,
            args: CoreValue::Bytes(vec![7; 70000]),
        })),
        CoreLinkRequest::Push(CoreLinkPushRequestMessage::Close { pushId: "p".into() }),
    ];
    for request in calls {
        a.send(PeerMessage::Request(request.clone())).await.unwrap();
        match b.receive().await.unwrap().unwrap() {
            PeerMessage::Request(value) => assert_eq!(value, request),
            _ => panic!("wrong operation"),
        }
    }
    let responses = vec![
        CoreLinkResponse::Call(CoreCallResponse::ok(
            CoreRequestId::new("c"),
            CoreValue::String("world".into()),
        )),
        CoreLinkResponse::Watch {
            requestId: CoreRequestId::new("w"),
            result: Ok(CoreLinkWatchResponse::Opened),
        },
        CoreLinkResponse::Watch {
            requestId: CoreRequestId::new("w"),
            result: Ok(CoreLinkWatchResponse::Closed),
        },
        CoreLinkResponse::Push {
            pushId: "p".into(),
            result: Ok(CoreLinkPushResponse::ItemAccepted { sequence: 0 }),
        },
    ];
    for response in responses {
        b.send(PeerMessage::Response(response.clone()))
            .await
            .unwrap();
        match a.receive().await.unwrap().unwrap() {
            PeerMessage::Response(value) => assert_eq!(value, response),
            _ => panic!("wrong response"),
        }
    }
    // Dropping a receive future does not consume subsequent messages.
    assert!(tokio::time::timeout(Duration::from_millis(30), a.receive())
        .await
        .is_err());
    b.send(PeerMessage::Response(CoreLinkResponse::Call(
        CoreCallResponse::ok(CoreRequestId::new("last"), CoreValue::Null),
    )))
    .await
    .unwrap();
    assert!(a.receive().await.unwrap().is_some());
    // Close wakes a pending read and is idempotent.
    let (read, _) = tokio::join!(b.receive(), async {
        tokio::task::yield_now().await;
        b.close().await;
    });
    assert!(read.unwrap().is_none());
    b.close().await;
    a.close().await;
    let (accept, _) = tokio::join!(server.accept(), async {
        tokio::task::yield_now().await;
        server.close().await;
    });
    assert!(accept.unwrap().is_none());
    // Graceful Host listener shutdown must eventually release the socket.
    for _ in 0..50 {
        if NativeHttpServerHost.bind(&bind).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("listener did not release {bind}");
}
#[tokio::test]
async fn http_call_watch_push_round_trip_and_close() {
    tokio::time::timeout(Duration::from_secs(15), exchange(PeerTransport::Http))
        .await
        .unwrap();
}
#[tokio::test]
async fn websocket_call_watch_push_round_trip_and_close() {
    tokio::time::timeout(Duration::from_secs(15), exchange(PeerTransport::WebSocket))
        .await
        .unwrap();
}
#[test]
fn wire_is_standard_link_not_an_extra_peer_envelope() {
    let request =
        CoreLinkRequest::Call(CoreCallRequest::new("id", "test", "echo", CoreValue::Null));
    let bytes = encodeLink(PeerMessage::Request(request.clone())).unwrap();
    assert_eq!(decodeLink::<CoreLinkRequest>(&bytes).unwrap(), request);
    let response = CoreLinkResponse::Call(CoreCallResponse::ok(
        CoreRequestId::new("id"),
        CoreValue::Null,
    ));
    assert_eq!(
        decodeLink::<CoreLinkResponse>(
            &encodeLink(PeerMessage::Response(response.clone())).unwrap()
        )
        .unwrap(),
        response
    );
}

#[tokio::test]
async fn http_and_websocket_share_one_host_listener() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let host = host(); let address = address().await; let link = HostPeerLink::default();
        let source = PeerEndpoint { nodeId: "server".into(), address: address.clone() };
        let http = link.listen(host.clone(), source.clone(), PeerTransport::Http).await.unwrap();
        let ws = link.listen(host.clone(), source.clone(), PeerTransport::WebSocket).await.unwrap();
        assert!(link.listen(host.clone(), source, PeerTransport::Http).await.is_err());
        for (transport, listener, scheme) in [(PeerTransport::Http, http.clone(), "http"), (PeerTransport::WebSocket, ws.clone(), "ws")] {
            let client = link.connect(host.clone(), PeerEndpoint { nodeId: "client".into(), address: String::new() }, PeerEndpoint { nodeId: "server".into(), address: format!("{scheme}://{address}/link") }, transport).await.unwrap();
            let server = listener.accept().await.unwrap().unwrap();
            client.send(PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new("shared", "test", "ping", CoreValue::Null)))).await.unwrap();
            assert!(matches!(server.receive().await.unwrap(), Some(PeerMessage::Request(_))));
            server.close().await; client.close().await;
        }
        http.close().await;
        // Removing HTTP must not remove the WS handler or close the shared socket.
        let client = link.connect(host.clone(), PeerEndpoint { nodeId: "client".into(), address: String::new() }, PeerEndpoint { nodeId: "server".into(), address: format!("ws://{address}/link") }, PeerTransport::WebSocket).await.unwrap();
        ws.accept().await.unwrap().unwrap().close().await; client.close().await;
        ws.close().await;
    }).await.unwrap();
}

#[tokio::test]
async fn websocket_final_response_survives_immediate_server_close() {
    tokio::time::timeout(Duration::from_secs(15), async {
        for _ in 0..20 {
            let (client, server, listener, _) = pair(PeerTransport::WebSocket).await;
            let response = CoreLinkResponse::Call(CoreCallResponse::ok(
                CoreRequestId::new("last-response"), CoreValue::String("confirmed".into()),
            ));
            server.send(PeerMessage::Response(response.clone())).await.unwrap();
            server.close().await;
            // Reproduce pairing's send-and-close: let the Host observe socket
            // closure before the application consumes its buffered response.
            tokio::time::sleep(Duration::from_millis(100)).await;
            match client.receive().await.unwrap().unwrap() {
                PeerMessage::Response(value) => assert_eq!(value, response),
                _ => panic!("final response was not preserved"),
            }
            assert!(client.receive().await.unwrap().is_none());
            client.close().await;
            listener.close().await;
        }
    }).await.unwrap();
}
