use super::*;
use operit_host_api::TcpHost;

const NETWORK: [PeerTransport; 3] = [
    PeerTransport::Http,
    PeerTransport::WebSocket,
    PeerTransport::Tcp,
];
fn endpoint(address: String) -> PeerEndpoint {
    PeerEndpoint {
        nodeId: "mixed".into(),
        address,
    }
}
fn message() -> PeerMessage {
    PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new(
        "mixed",
        "test",
        "echo",
        CoreValue::String("同端口".into()),
    )))
}
async fn roundTrip(
    host: Arc<HostManager>,
    link: &HostPeerLink,
    listener: &Arc<dyn PeerListener>,
    transport: PeerTransport,
) {
    let address = listener.localAddress().unwrap();
    let target = match transport {
        PeerTransport::Http => format!("http://{address}/link"),
        PeerTransport::WebSocket => format!("ws://{address}/link"),
        _ => format!("tcp://{address}"),
    };
    let client = link
        .connect(host, endpoint(String::new()), endpoint(target), transport)
        .await
        .unwrap();
    client.send(message()).await.unwrap();
    let server = listener.accept().await.unwrap().unwrap();
    assert_eq!(server.transport(), transport);
    assert_eq!(server.source().address, address);
    assert!(server.remoteAddress().unwrap().ip().is_loopback());
    assert_eq!(
        encodeLink(server.receive().await.unwrap().unwrap()).unwrap(),
        encodeLink(message()).unwrap()
    );
    server.send(message()).await.unwrap();
    assert_eq!(
        encodeLink(client.receive().await.unwrap().unwrap()).unwrap(),
        encodeLink(message()).unwrap()
    );
    server.close().await;
    client.close().await;
}

/// All seven selections and all six three-carrier registration orders use one port.
#[tokio::test]
async fn every_network_selection_and_registration_order_uses_one_port() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut orders = Vec::new();
        for mask in 1..8 {
            let selected: Vec<_> = NETWORK
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, t)| *t)
                .collect();
            orders.push(selected.clone());
            orders.push(selected.into_iter().rev().collect());
        }
        for a in NETWORK {
            for b in NETWORK {
                for c in NETWORK {
                    if a != b && b != c && a != c {
                        orders.push(vec![a, b, c]);
                    }
                }
            }
        }
        for order in orders {
            let host = host();
            let link = HostPeerLink::default();
            let mut address = "127.0.0.1:0".to_owned();
            let mut listeners = Vec::new();
            for transport in order {
                let listener = link
                    .listen(host.clone(), endpoint(address.clone()), transport)
                    .await
                    .unwrap();
                let actual = listener.localAddress().unwrap();
                assert_ne!(actual.parse::<std::net::SocketAddr>().unwrap().port(), 0);
                if !listeners.is_empty() {
                    assert_eq!(actual, address);
                }
                address = actual;
                assert!(std::net::TcpListener::bind(&address).is_err());
                listeners.push((transport, listener));
            }
            if listeners
                .iter()
                .any(|(transport, _)| *transport != PeerTransport::Tcp)
            {
                let expected = if listeners
                    .iter()
                    .any(|(transport, _)| *transport == PeerTransport::WebSocket)
                {
                    PeerTransport::WebSocket
                } else {
                    PeerTransport::Http
                };
                assert_eq!(
                    detectTransport(host.clone(), &address).await.unwrap(),
                    expected
                );
                if expected == PeerTransport::WebSocket {
                    // The upgrade probe is a real, deliberately short-lived WS.
                    // Production's accept loop drains it before the pairing channel.
                    let (_, listener) = listeners
                        .iter()
                        .find(|(transport, _)| *transport == expected)
                        .unwrap();
                    listener.accept().await.unwrap().unwrap().close().await;
                }
            }
            for (transport, listener) in &listeners {
                roundTrip(host.clone(), &link, listener, *transport).await;
            }
            // Removing a carrier leaves all remaining carriers usable.
            while let Some((_, listener)) = listeners.pop() {
                listener.close().await;
                for (transport, listener) in &listeners {
                    roundTrip(host.clone(), &link, listener, *transport).await;
                }
            }
            // No polling/retry: final close has released the actual socket.
            let rebound = link
                .listen(host.clone(), endpoint(address.clone()), PeerTransport::Tcp)
                .await
                .unwrap();
            roundTrip(host, &link, &rebound, PeerTransport::Tcp).await;
            rebound.close().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn disabled_carriers_can_be_registered_again_without_rebinding() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let host = host();
        let link = HostPeerLink::default();
        let tcp = link
            .listen(
                host.clone(),
                endpoint("127.0.0.1:0".into()),
                PeerTransport::Tcp,
            )
            .await
            .unwrap();
        let address = tcp.localAddress().unwrap();
        for _ in 0..3 {
            let http = link
                .listen(host.clone(), endpoint(address.clone()), PeerTransport::Http)
                .await
                .unwrap();
            let ws = link
                .listen(
                    host.clone(),
                    endpoint(address.clone()),
                    PeerTransport::WebSocket,
                )
                .await
                .unwrap();
            let ((), (), ()) = tokio::join!(
                roundTrip(host.clone(), &link, &http, PeerTransport::Http),
                roundTrip(host.clone(), &link, &ws, PeerTransport::WebSocket),
                roundTrip(host.clone(), &link, &tcp, PeerTransport::Tcp),
            );
            http.close().await;
            ws.close().await;
            let (detected, ()) = tokio::join!(detectTransport(host.clone(), &address), async {
                let probe = tcp.accept().await.unwrap().unwrap();
                assert!(probe.receive().await.is_err());
                probe.close().await;
            });
            assert_eq!(detected.unwrap(), PeerTransport::Tcp);
            roundTrip(host.clone(), &link, &tcp, PeerTransport::Tcp).await;
        }
        tcp.close().await;
    })
    .await
    .unwrap();
}

/// A partial binary prefix and an idle connection must not stall other protocols.
#[tokio::test]
async fn fragmented_tcp_header_is_replayed_and_other_connections_keep_working() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let host = host();
        let link = HostPeerLink::default();
        let http = link
            .listen(
                host.clone(),
                endpoint("127.0.0.1:0".into()),
                PeerTransport::Http,
            )
            .await
            .unwrap();
        let address = http.localAddress().unwrap();
        let tcp = link
            .listen(host.clone(), endpoint(address.clone()), PeerTransport::Tcp)
            .await
            .unwrap();
        let idle = operit_host_native_common::NativeTcpHost
            .connect(&address)
            .await
            .unwrap();
        let raw = operit_host_native_common::NativeTcpHost
            .connect(&address)
            .await
            .unwrap();
        let payload = encodeLink(message()).unwrap();
        let header = (payload.len() as u32).to_be_bytes();
        raw.write(&header[..2]).await.unwrap();
        roundTrip(host.clone(), &link, &http, PeerTransport::Http).await;
        raw.write(&header[2..]).await.unwrap();
        raw.write(&payload).await.unwrap();
        let peer = tcp.accept().await.unwrap().unwrap();
        assert_eq!(
            encodeLink(peer.receive().await.unwrap().unwrap()).unwrap(),
            payload
        );
        peer.close().await;
        raw.close().await;
        http.close().await;
        tcp.close().await;
        // Shutdown cancels an incomplete protocol classification immediately.
        assert!(tokio::time::timeout(Duration::from_secs(1), idle.read())
            .await
            .unwrap()
            .unwrap()
            .is_none());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn fragmented_http_method_is_replayed_and_disabled_tcp_is_rejected() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let host = host(); let link = HostPeerLink::default();
        let http = link.listen(host.clone(), endpoint("127.0.0.1:0".into()), PeerTransport::Http).await.unwrap();
        let address = http.localAddress().unwrap();
        let raw = operit_host_native_common::NativeTcpHost.connect(&address).await.unwrap();
        raw.write(b"OP").await.unwrap();
        roundTrip(host.clone(), &link, &http, PeerTransport::Http).await;
        raw.write(b"TIONS /link HTTP/1.1\r\nHost: localhost\r\nOrigin: https://example.test\r\nAccess-Control-Request-Method: POST\r\nConnection: close\r\n\r\n").await.unwrap();
        let response = raw.read().await.unwrap().unwrap();
        assert!(response.starts_with(b"HTTP/1.1 204"), "{}", String::from_utf8_lossy(&response));
        raw.close().await;
        let raw = operit_host_native_common::NativeTcpHost.connect(&address).await.unwrap();
        raw.write(&[0, 0, 0, 1, 0]).await.unwrap();
        assert!(raw.read().await.unwrap().is_none());
        raw.close().await; http.close().await;
    }).await.unwrap();
}

/// Legacy HTTP Hosts may drain established streams after the listener is removed.
#[tokio::test]
async fn http_only_host_can_stop_listening_before_closing_established_channels() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut host = host();
        Arc::get_mut(&mut host).unwrap().tcpHost = None;
        let link = HostPeerLink::default();
        let listener = link
            .listen(
                host.clone(),
                endpoint("127.0.0.1:0".into()),
                PeerTransport::Http,
            )
            .await
            .unwrap();
        let address = listener.localAddress().unwrap();
        let client = link
            .connect(
                host,
                endpoint(String::new()),
                endpoint(format!("http://{address}/link")),
                PeerTransport::Http,
            )
            .await
            .unwrap();
        let server = listener.accept().await.unwrap().unwrap();
        listener.close().await;
        server.close().await;
        client.close().await;
    })
    .await
    .unwrap();
}
