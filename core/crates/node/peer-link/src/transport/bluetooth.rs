//! Bluetooth Classic 字节流适配；连接、收发和关闭全部调用 Host。
//! BLE GATT 不是 Classic 字节流，不在这里假装成同一种传输。
use super::{
    inbox::{hostTask, Inbox},
    stream::{ByteConnection, FramedPeerConnection},
};
use crate::{PeerConnection, PeerEndpoint, PeerListener, PeerTransport};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use operit_host_api::{HostManager::HostManager, *};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{mpsc, watch, Mutex};

// Identifies the transport service, not a peer identity or authorization.
const LINK_SERVICE_UUID: &str = "959f4b7a-b6b8-491f-aabd-9c54c931a7d3";
struct Session {
    host: Arc<dyn BluetoothHost>,
    id: String,
    closed: AtomicBool,
}
impl Session {
    fn close(&self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            let _ = self.host.bluetoothClose(&self.id);
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}
struct Connection {
    session: Arc<Session>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    inbox: Arc<Inbox>,
}
impl Drop for Connection {
    fn drop(&mut self) {
        self.session.close();
        self.inbox.finish(Ok(()));
    }
}
#[async_trait]
impl ByteConnection for Connection {
    async fn write(&self, bytes: &[u8]) -> Result<(), String> {
        self.inbox.status()?;
        let session = self.session.clone();
        let bytes = bytes.to_vec();
        hostTask(&self.scheduler, move || {
            if session.closed.load(Ordering::SeqCst) {
                return Err(HostError::new("Bluetooth Link closed"));
            }
            let result = session.host.bluetoothSend(
                &session.id,
                BluetoothPayload {
                    text: None,
                    dataBase64: Some(STANDARD.encode(&bytes)),
                },
            )?;
            if result.bytesWritten != bytes.len() as i64 {
                return Err(HostError::new("Incomplete Bluetooth Link write"));
            }
            Ok(())
        })
        .await
    }
    async fn read(&self) -> Result<Option<Vec<u8>>, String> {
        self.inbox.read().await
    }
    async fn close(&self) {
        self.session.close();
        self.inbox.finish(Ok(()));
    }
}
fn wrap(
    host: Arc<dyn BluetoothHost>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    data: BluetoothSessionData,
    source: PeerEndpoint,
    target: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerConnection>, String> {
    let session = Arc::new(Session {
        host,
        id: data.sessionId,
        closed: AtomicBool::new(false),
    });
    let inbox = Inbox::new(maxMessageBytes);
    let connection = Arc::new(Connection {
        session: session.clone(),
        scheduler: scheduler.clone(),
        inbox: inbox.clone(),
    });
    // A single Host-owned pump preserves bytes if runtime cancels receive().
    // close() releases the Host session and wakes blocking read; no runtime timer loop.
    scheduler
        .scheduleHostRuntimeTask(
            "peer-bluetooth-read",
            Box::new(move || {
                let result = (|| -> Result<(), String> {
                    while !session.closed.load(Ordering::SeqCst) {
                        let data = session
                            .host
                            .bluetoothRead(BluetoothReadRequest {
                                sessionId: session.id.clone(),
                                maxBytes: 4096,
                                timeoutMs: -1,
                            })
                            .map_err(|e| e.to_string())?;
                        if session.closed.load(Ordering::SeqCst) {
                            break;
                        }
                        if data.bytesRead == 0 {
                            break;
                        }
                        let bytes = STANDARD
                            .decode(
                                data.dataBase64
                                    .ok_or("Bluetooth Host returned non-binary data")?,
                            )
                            .map_err(|e| e.to_string())?;
                        if data.bytesRead != bytes.len() as i64 || bytes.len() > 4096 {
                            return Err("Invalid Bluetooth Host read length".into());
                        }
                        inbox.put(bytes);
                        inbox.status()?;
                    }
                    Ok(())
                })();
                inbox.finish(result);
                session.close();
            }),
        )
        .map_err(|e| e.to_string())?;
    Ok(FramedPeerConnection::new(
        source,
        target,
        PeerTransport::Bluetooth,
        connection,
        maxMessageBytes,
    ))
}
pub(super) async fn connect(
    host: &HostManager,
    source: PeerEndpoint,
    target: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerConnection>, String> {
    let provider = host
        .bluetoothHost
        .clone()
        .ok_or("Bluetooth Host is not installed")?;
    let scheduler = host
        .hostRuntimeTaskSchedulerHost
        .clone()
        .ok_or("Host task scheduler is not installed")?;
    let io = provider.clone();
    let address = target.address.clone();
    let data = hostTask(&scheduler, move || {
        io.bluetoothConnect(BluetoothClassicConnectRequest {
            address,
            uuid: LINK_SERVICE_UUID.into(),
        })
    })
    .await?;
    wrap(provider, scheduler, data, source, target, maxMessageBytes)
}
struct Listener {
    session: Arc<Session>,
    incoming: Mutex<mpsc::Receiver<Result<Arc<dyn PeerConnection>, String>>>,
    stopped: watch::Sender<bool>,
}
impl Drop for Listener {
    fn drop(&mut self) {
        self.stopped.send_replace(true);
        self.session.close();
    }
}
#[async_trait]
impl PeerListener for Listener {
    async fn accept(&self) -> Result<Option<Arc<dyn PeerConnection>>, String> {
        let mut stop = self.stopped.subscribe();
        if *stop.borrow() {
            return Ok(None);
        }
        let mut rx = self.incoming.lock().await;
        tokio::select! {
            biased;
            _ = stop.wait_for(|closed| *closed) => Ok(None),
            connection = rx.recv() => connection.transpose(),
        }
    }
    async fn close(&self) {
        self.stopped.send_replace(true);
        self.session.close();
        self.incoming.lock().await.close();
    }
}
pub(super) async fn listen(
    host: &HostManager,
    source: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerListener>, String> {
    let provider = host
        .bluetoothHost
        .clone()
        .ok_or("Bluetooth Host is not installed")?;
    let scheduler = host
        .hostRuntimeTaskSchedulerHost
        .clone()
        .ok_or("Host task scheduler is not installed")?;
    let io = provider.clone();
    let name = source.nodeId.clone();
    let data = hostTask(&scheduler, move || {
        io.bluetoothListen(BluetoothClassicListenRequest {
            name,
            uuid: LINK_SERVICE_UUID.into(),
        })
    })
    .await?;
    let session = Arc::new(Session {
        host: provider.clone(),
        id: data.sessionId,
        closed: AtomicBool::new(false),
    });
    let (tx, rx) = mpsc::channel(32);
    let (stopped, _) = watch::channel(false);
    let listener = Arc::new(Listener {
        session: session.clone(),
        incoming: Mutex::new(rx),
        stopped,
    });
    let schedulerForConnections = scheduler.clone();
    scheduler
        .scheduleHostRuntimeTask(
            "peer-bluetooth-accept",
            Box::new(move || {
                while !session.closed.load(Ordering::SeqCst) {
                    let result = session.host.bluetoothAccept(BluetoothClassicAcceptRequest {
                        listenerSessionId: session.id.clone(),
                        timeoutMs: -1,
                    });
                    if session.closed.load(Ordering::SeqCst) {
                        if let Ok(data) = result {
                            let _ = session.host.bluetoothClose(&data.sessionId);
                        }
                        break;
                    }
                    let result = result.map_err(|e| e.to_string()).and_then(|data| {
                        let target = PeerEndpoint {
                            nodeId: String::new(),
                            address: data.address.clone(),
                        };
                        wrap(
                            provider.clone(),
                            schedulerForConnections.clone(),
                            data,
                            source.clone(),
                            target,
                            maxMessageBytes,
                        )
                    });
                    let failed = result.is_err();
                    // No Host resource survives a full or closed accept queue.
                    if tx.try_send(result).is_err() || failed {
                        break;
                    }
                }
                session.close();
            }),
        )
        .map_err(|e| e.to_string())?;
    Ok(listener)
}
