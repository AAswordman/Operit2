//! Real sockets: the probe classifier must separate all three carriers.
use operit_host_api::HttpServer::HttpServerHost;
use operit_host_api::{HostManager::HostManager, *};
use operit_host_native_common::NativeTcpHost;
use operit_host_native_http::{NativeHttpHost, NativeHttpServerHost};
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
    fn scheduleDelayedHostRuntimeTask(&self, _: &str, delay: u64, task: HostRuntimeTask) -> HostResult<()> {
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
        tcpHost: Some(Arc::new(NativeTcpHost)),
        hostRuntimeTaskSchedulerHost: Some(Arc::new(Scheduler)),
        ..HostManager::default()
    })
}

/// Reserves one ephemeral port through the real HTTP server Host.
async fn freeAddress() -> String {
    let temporary = NativeHttpServerHost.bind("127.0.0.1:0").await.unwrap();
    temporary.localAddress().unwrap().to_string()
}

fn endpoint(nodeId: &str, address: &str) -> PeerEndpoint {
    PeerEndpoint { nodeId: nodeId.into(), address: address.into() }
}

/// A WebSocket listener answers the upgrade probe with 101.
#[tokio::test]
async fn websocket_listener_is_detected() {
    let host = host();
    let address = freeAddress().await;
    let server = HostPeerLink::default()
        .listen(host.clone(), endpoint("server", &address), PeerTransport::WebSocket)
        .await
        .unwrap();
    let detected = detectTransport(host.clone(), &address).await.unwrap();
    assert_eq!(detected, PeerTransport::WebSocket);
    server.close().await;
}

/// Without a WebSocket handler the same server answers 404, which is still HTTP.
#[tokio::test]
async fn http_listener_without_ws_falls_back_to_http() {
    let host = host();
    let address = freeAddress().await;
    let server = HostPeerLink::default()
        .listen(host.clone(), endpoint("server", &address), PeerTransport::Http)
        .await
        .unwrap();
    let detected = detectTransport(host.clone(), &address).await.unwrap();
    assert_eq!(detected, PeerTransport::Http);
    server.close().await;
}

/// A framed TCP receiver drops the bogus length header without writing a byte.
#[tokio::test]
async fn framed_tcp_listener_is_detected() {
    let host = host();
    let temporary = NativeTcpHost.bind("127.0.0.1:0").await.unwrap();
    let address = temporary.local_address().unwrap();
    temporary.close().await;
    let server = HostPeerLink::default()
        .listen(host.clone(), endpoint("server", &address), PeerTransport::Tcp)
        .await
        .unwrap();
    // Mirror the production accept loop: consume the probe, let the unreadable
    // frame fail receive, and close so the probe observes end-of-stream.
    tokio::spawn(async move {
        while let Ok(Some(connection)) = server.accept().await {
            let connection = connection.clone();
            tokio::spawn(async move {
                let _ = connection.receive().await;
                connection.close().await;
            });
        }
    });
    let detected = tokio::time::timeout(Duration::from_secs(10), detectTransport(host.clone(), &address))
        .await
        .expect("detection must finish")
        .unwrap();
    assert_eq!(detected, PeerTransport::Tcp);
}

/// Nothing listening is an error, not a carrier guess.
#[tokio::test]
async fn refused_endpoint_is_an_error() {
    let host = host();
    let temporary = NativeTcpHost.bind("127.0.0.1:0").await.unwrap();
    let address = temporary.local_address().unwrap();
    temporary.close().await;
    assert!(detectTransport(host, &address).await.is_err());
}
