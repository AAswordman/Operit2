//! Real HTTP batching under an explicitly stalled POST response.
use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Mutex,
};
use tokio::sync::Notify;

struct GatedHttpHost {
    inner: NativeHttpHost,
    bodies: Mutex<Vec<Vec<u8>>>,
    posted: Notify,
    release: Mutex<mpsc::Receiver<()>>,
    active: AtomicUsize,
    peak: AtomicUsize,
    reject: bool,
}

impl GatedHttpHost {
    /// Blocks POST completion until the test releases each individual batch.
    fn new(reject: bool) -> (Arc<Self>, mpsc::Sender<()>) {
        let (release, receiver) = mpsc::channel();
        (
            Arc::new(Self {
                inner: NativeHttpHost::new(),
                bodies: Mutex::new(Vec::new()),
                posted: Notify::new(),
                release: Mutex::new(receiver),
                active: AtomicUsize::new(0),
                peak: AtomicUsize::new(0),
                reject,
            }),
            release,
        )
    }

    /// Waits for a real sender attempt without relying on scheduling sleeps.
    async fn waitForPosts(&self, count: usize) {
        loop {
            let posted = self.posted.notified();
            if self.bodies.lock().unwrap().len() >= count {
                return;
            }
            posted.await;
        }
    }
}

impl HttpHost for GatedHttpHost {
    /// Retains the real Host's image-delivery capability.
    fn imageDelivery(&self) -> HttpImageDelivery {
        self.inner.imageDelivery()
    }

    /// Records immutable POST bodies and gates the completion observed by the transport.
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
        assert_eq!(request.method, "POST");
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        self.bodies.lock().unwrap().push(request.body.clone());
        self.posted.notify_one();
        let released = self
            .release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5));
        let result = match released {
            Err(error) => Err(HostError::new(error.to_string())),
            Ok(()) if self.reject => Ok(HttpResponseData {
                finalUrl: request.url,
                statusCode: 503,
                statusMessage: "Busy".into(),
                headers: vec![],
                body: vec![],
            }),
            Ok(()) => self.inner.executeHttpRequest(request),
        };
        self.active.fetch_sub(1, Ordering::SeqCst);
        result
    }

    /// Leaves download behavior with the real Host rather than the POST probe.
    fn downloadFiles(
        &self,
        request: HttpDownloadRequest,
        control: HttpDownloadControl,
        progress: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        self.inner.downloadFiles(request, control, progress)
    }
}

impl HttpStreamHost for GatedHttpHost {
    /// Uses the production streaming GET client and callbacks.
    fn openHttpByteStream(
        &self,
        id: String,
        request: HttpRequestData,
        opened: HttpStreamOpenedCallback,
        chunk: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        self.inner
            .openHttpByteStream(id, request, opened, chunk, closed)
    }

    /// Cancels the production streaming GET when the connection ends.
    fn closeHttpByteStream(&self, id: &str) -> HostResult<()> {
        self.inner.closeHttpByteStream(id)
    }
}

/// Creates a real listener and one dialing client using the controlled POST Host.
async fn connectGated(
    probe: Arc<GatedHttpHost>,
) -> (
    Arc<dyn PeerConnection>,
    Arc<dyn PeerConnection>,
    Arc<dyn PeerListener>,
) {
    let bind = address().await;
    let host = Arc::new(HostManager {
        httpHost: Some(probe),
        httpServerHost: Some(Arc::new(NativeHttpServerHost)),
        hostRuntimeTaskSchedulerHost: Some(Arc::new(Scheduler)),
        ..HostManager::default()
    });
    let link = HostPeerLink::default();
    let listener = link
        .listen(
            host.clone(),
            PeerEndpoint {
                nodeId: "server".into(),
                address: bind.clone(),
            },
            PeerTransport::Http,
        )
        .await
        .unwrap();
    let client = link
        .connect(
            host,
            PeerEndpoint {
                nodeId: "client".into(),
                address: String::new(),
            },
            PeerEndpoint {
                nodeId: "server".into(),
                address: format!("http://{bind}/link"),
            },
            PeerTransport::Http,
        )
        .await
        .unwrap();
    let server = listener.accept().await.unwrap().unwrap();
    (client, server, listener)
}

/// Encodes a distinct stream event, including a terminal event whose order must be preserved.
fn event(index: u64, kind: CoreEventKind) -> CoreLinkResponse {
    CoreLinkResponse::Watch {
        requestId: CoreRequestId::new("watch"),
        result: Ok(CoreLinkWatchResponse::Event(CoreEvent {
            requestId: None,
            target: "chat".into(),
            propertyName: "messages".into(),
            kind,
            value: CoreValue::Unsigned(index),
        })),
    }
}

/// A slow POST freezes A while subsequent events merge into B and then C, without parallel I/O.
#[tokio::test]
async fn slow_post_batches_watch_events_in_order() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (probe, release) = GatedHttpHost::new(false);
        let (client, server, listener) = connectGated(probe.clone()).await;
        client
            .send(PeerMessage::Response(event(0, CoreEventKind::Snapshot)))
            .await
            .unwrap();
        probe.waitForPosts(1).await;
        let first_body = probe.bodies.lock().unwrap()[0].clone();
        for index in 1..=128 {
            client
                .send(PeerMessage::Response(event(index, CoreEventKind::Delta)))
                .await
                .unwrap();
        }
        assert_eq!(probe.bodies.lock().unwrap().len(), 1);
        assert_eq!(probe.bodies.lock().unwrap()[0], first_body);
        release.send(()).unwrap();
        match server.receive().await.unwrap().unwrap() {
            PeerMessage::Response(value) => assert_eq!(value, event(0, CoreEventKind::Snapshot)),
            _ => panic!("expected the initial snapshot"),
        }
        probe.waitForPosts(2).await;
        let second_body = probe.bodies.lock().unwrap()[1].clone();
        client
            .send(PeerMessage::Response(event(129, CoreEventKind::Completed)))
            .await
            .unwrap();
        assert_eq!(probe.bodies.lock().unwrap().len(), 2);
        assert_eq!(probe.bodies.lock().unwrap()[1], second_body);
        release.send(()).unwrap();
        for index in 1..=128 {
            match server.receive().await.unwrap().unwrap() {
                PeerMessage::Response(value) => {
                    assert_eq!(value, event(index, CoreEventKind::Delta))
                }
                _ => panic!("expected an ordered delta"),
            }
        }
        probe.waitForPosts(3).await;
        release.send(()).unwrap();
        match server.receive().await.unwrap().unwrap() {
            PeerMessage::Response(value) => assert_eq!(value, event(129, CoreEventKind::Completed)),
            _ => panic!("expected completion after every delta"),
        }
        assert_eq!(probe.bodies.lock().unwrap().len(), 3);
        assert_eq!(probe.peak.load(Ordering::SeqCst), 1);
        client.close().await;
        server.close().await;
        listener.close().await;
    })
    .await
    .unwrap();
}

/// A failed batch terminates the receiver and later writes without sending the queued next batch.
#[tokio::test]
async fn failed_post_wakes_watch_receiver_and_stops_pending_batch() {
    tokio::time::timeout(Duration::from_secs(10), async {
        let (probe, release) = GatedHttpHost::new(true);
        let (client, server, listener) = connectGated(probe.clone()).await;
        client
            .send(PeerMessage::Response(event(0, CoreEventKind::Snapshot)))
            .await
            .unwrap();
        probe.waitForPosts(1).await;
        client
            .send(PeerMessage::Response(event(1, CoreEventKind::Delta)))
            .await
            .unwrap();
        release.send(()).unwrap();
        assert_eq!(
            client.receive().await.err().unwrap(),
            "HTTP Link send failed: 503"
        );
        assert_eq!(
            client
                .send(PeerMessage::Response(event(2, CoreEventKind::Completed)))
                .await
                .err()
                .unwrap(),
            "HTTP Link send failed: 503"
        );
        assert_eq!(probe.bodies.lock().unwrap().len(), 1);
        client.close().await;
        server.close().await;
        listener.close().await;
    })
    .await
    .unwrap();
}
