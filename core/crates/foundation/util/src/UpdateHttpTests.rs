use super::*;
use operit_host_api::*;
use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::task::{Context, Waker};

#[derive(Clone)]
pub(crate) struct Callbacks {
    pub head: HttpStreamResponseCallback,
    pub bytes: HttpStreamChunkCallback,
    pub closed: HttpStreamClosedCallback,
}

#[derive(Default)]
pub(crate) struct ControlledHttpHost {
    callbacks: Mutex<Option<Callbacks>>,
    pub closed: AtomicBool,
}

impl ControlledHttpHost {
    /// Borrows the open request callbacks without retaining the fixture lock.
    pub fn callbacks(&self) -> Callbacks {
        self.callbacks.lock().unwrap().as_ref().unwrap().clone()
    }
}

impl HttpStreamHost for ControlledHttpHost {
    /// Rejects the legacy body-only transport in this response fixture.
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("response metadata required")
    }

    /// Retains callbacks until the test supplies headers, body and completion.
    fn openHttpResponseStream(
        &self,
        _: String,
        _: HttpRequestData,
        head: HttpStreamResponseCallback,
        bytes: HttpStreamChunkCallback,
        closed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        self.closed.store(false, Ordering::SeqCst);
        *self.callbacks.lock().unwrap() = Some(Callbacks {
            head,
            bytes,
            closed,
        });
        Ok(())
    }

    /// Cancels an open request independently of response completion.
    fn closeHttpByteStream(&self, _: &str) -> HostResult<()> {
        self.closed.store(true, Ordering::SeqCst);
        self.callbacks.lock().unwrap().take();
        Ok(())
    }
}

impl HttpHost for ControlledHttpHost {
    /// Selects byte delivery for the fixture.
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    /// Fails any accidental synchronous request in the update pipeline.
    fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> {
        panic!("synchronous HTTP must not be used")
    }
    /// Fails any accidental synchronous file download in the update pipeline.
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        panic!("synchronous download must not be used")
    }
}

/// Supplies a successful response head with a declared payload length.
pub(crate) fn head(length: usize) -> HttpResponseHead {
    HttpResponseHead {
        finalUrl: "https://fixture.invalid/package".into(),
        statusCode: 200,
        statusMessage: "OK".into(),
        headers: vec![("Content-Length".into(), length.to_string())],
    }
}

/// Polls once without threads, timers, or an ambient Tokio executor.
pub(crate) fn assertPending(future: std::pin::Pin<&mut impl Future>) {
    assert!(future
        .poll(&mut Context::from_waker(Waker::noop()))
        .is_pending());
}

/// Builds a request with the same network policy as release discovery.
fn fixtureRequest() -> HttpRequestData {
    super::super::GithubReleaseUtil::updateHttpRequest("https://fixture.invalid/package", "GET")
}

/// Headers and chunks may arrive later; only a clean terminal callback finishes the future.
#[tokio::test]
async fn waits_for_host_callbacks_without_blocking_or_buffering() {
    let host = Arc::new(ControlledHttpHost::default());
    let body = Arc::new(Mutex::new(Vec::new()));
    let sink = body.clone();
    let mut future = Box::pin(request(
        host.clone(),
        fixtureRequest(),
        HttpDownloadControl::new(),
        move |event| {
            if let UpdateHttpEvent::Bytes(bytes) = event {
                sink.lock().unwrap().extend(bytes);
            }
            Ok(())
        },
    ));
    assertPending(future.as_mut());
    let callbacks = host.callbacks();
    (callbacks.head)(head(4));
    (callbacks.bytes)(b"ab".to_vec());
    assert_eq!(*body.lock().unwrap(), b"ab");
    assertPending(future.as_mut());
    (callbacks.bytes)(b"cd".to_vec());
    (callbacks.closed)(Ok(()));
    future.await.unwrap();
    assert!(host.closed.load(Ordering::SeqCst));
    assert_eq!(*body.lock().unwrap(), b"abcd");
}

/// Cancelling before response headers promptly closes the request through its owning Host.
#[tokio::test]
async fn cancellation_and_future_drop_close_pending_requests() {
    let host = Arc::new(ControlledHttpHost::default());
    let control = HttpDownloadControl::new();
    let mut future = Box::pin(request(
        host.clone(),
        fixtureRequest(),
        control.clone(),
        |_| Ok(()),
    ));
    assertPending(future.as_mut());
    control.cancel();
    assert_eq!(future.await.unwrap_err(), "UPDATE_CANCELLED");
    assert!(host.closed.load(Ordering::SeqCst));
    let mut future = Box::pin(request(
        host.clone(),
        fixtureRequest(),
        HttpDownloadControl::new(),
        |_| Ok(()),
    ));
    assertPending(future.as_mut());
    drop(future);
    assert!(host.closed.load(Ordering::SeqCst));
}

/// Progress consumers may cancel reentrantly without acquiring HTTP state again.
#[tokio::test]
async fn consumer_cancellation_does_not_deadlock() {
    let host = Arc::new(ControlledHttpHost::default());
    let control = HttpDownloadControl::new();
    let cancel = control.clone();
    let mut future = Box::pin(request(
        host.clone(),
        fixtureRequest(),
        control,
        move |_| {
            cancel.cancel();
            Ok(())
        },
    ));
    assertPending(future.as_mut());
    (host.callbacks().head)(head(4));
    assert_eq!(future.await.unwrap_err(), "UPDATE_CANCELLED");
}

/// Rejects malformed callback order, failed statuses, disconnects and failed closes.
#[tokio::test]
async fn rejects_incomplete_and_invalid_host_responses() {
    for scenario in 0..6 {
        let host = Arc::new(ControlledHttpHost::default());
        let mut future = Box::pin(request(
            host.clone(),
            fixtureRequest(),
            HttpDownloadControl::new(),
            |_| Ok(()),
        ));
        assertPending(future.as_mut());
        let callbacks = host.callbacks();
        match scenario {
            0 => (callbacks.bytes)(vec![1]),
            1 => {
                (callbacks.head)(head(4));
                (callbacks.head)(head(4));
            }
            2 => (callbacks.closed)(Ok(())),
            3 => {
                let mut response = head(4);
                response.statusCode = 403;
                (callbacks.head)(response);
            }
            4 => {
                (callbacks.head)(head(4));
                (callbacks.closed)(Err("connection lost".into()));
            }
            _ => {
                host.callbacks.lock().unwrap().take();
            }
        }
        drop(callbacks);
        assert!(future.await.is_err(), "scenario {scenario}");
        assert!(host.closed.load(Ordering::SeqCst));
    }
}
