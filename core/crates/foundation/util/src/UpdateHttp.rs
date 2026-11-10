//! Async update requests over the Host's existing ordered HTTP callbacks.
use operit_host_api::{HttpDownloadControl, HttpHost, HttpRequestData, HttpResponseHead};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::oneshot;

static NEXT_REQUEST: AtomicUsize = AtomicUsize::new(0);

pub(crate) enum UpdateHttpEvent {
    Head(HttpResponseHead),
    Bytes(Vec<u8>),
}

type Consumer = Box<dyn FnMut(UpdateHttpEvent) -> Result<(), String> + Send>;
struct RequestState {
    consumer: Option<Consumer>,
    completion: Option<oneshot::Sender<Result<(), String>>>,
    receivedHead: bool,
}

impl RequestState {
    /// Releases the sink before notifying the waiting caller, including file handles.
    fn finish(&mut self, result: Result<(), String>) {
        self.consumer = None;
        if let Some(sender) = self.completion.take() {
            let _ = sender.send(result);
        }
    }

    /// Validates response ordering and delivers each chunk directly to its consumer.
    fn deliver(&mut self, event: UpdateHttpEvent) {
        if self.consumer.is_none() {
            return;
        }
        let error = match &event {
            UpdateHttpEvent::Head(_) if self.receivedHead => {
                Some("HTTP Host sent duplicate headers".into())
            }
            UpdateHttpEvent::Head(head) if !(200..300).contains(&head.statusCode) => {
                Some(format!("HTTP {}: {}", head.statusCode, head.statusMessage))
            }
            UpdateHttpEvent::Head(_) => {
                self.receivedHead = true;
                None
            }
            UpdateHttpEvent::Bytes(_) if !self.receivedHead => {
                Some("HTTP Host sent bytes before headers".into())
            }
            _ => None,
        };
        let result = match error {
            Some(error) => Err(error),
            None => self.consumer.as_mut().expect("active HTTP consumer")(event),
        };
        if let Err(error) = result {
            self.finish(Err(error));
        }
    }
}

struct RequestGuard {
    host: Arc<dyn HttpHost>,
    id: String,
    state: Weak<Mutex<RequestState>>,
}

impl Drop for RequestGuard {
    /// Cancels header and body reads when the future completes or is dropped.
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            state
                .lock()
                .expect("update HTTP state")
                .finish(Err("Update request dropped".into()));
        }
        let _ = self.host.closeHttpByteStream(&self.id);
    }
}

/// Awaits Host completion without a worker, timer, blocking request or body queue.
pub(crate) async fn request(
    host: Arc<dyn HttpHost>,
    request: HttpRequestData,
    control: HttpDownloadControl,
    consume: impl FnMut(UpdateHttpEvent) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    if control.isCancelled() {
        return Err("UPDATE_CANCELLED".into());
    }
    let (sender, receiver) = oneshot::channel();
    let state = Arc::new(Mutex::new(RequestState {
        consumer: Some(Box::new(consume)),
        completion: Some(sender),
        receivedHead: false,
    }));
    let guard = RequestGuard {
        host,
        id: format!(
            "update-http-{}",
            NEXT_REQUEST.fetch_add(1, Ordering::Relaxed)
        ),
        state: Arc::downgrade(&state),
    };
    let headState = state.clone();
    let bytesState = state.clone();
    let closeState = state.clone();
    guard
        .host
        .openHttpResponseStream(
            guard.id.clone(),
            request,
            Arc::new(move |head| {
                headState
                    .lock()
                    .expect("update HTTP state")
                    .deliver(UpdateHttpEvent::Head(head))
            }),
            Arc::new(move |bytes| {
                bytesState
                    .lock()
                    .expect("update HTTP state")
                    .deliver(UpdateHttpEvent::Bytes(bytes))
            }),
            Arc::new(move |result| {
                let mut state = closeState.lock().expect("update HTTP state");
                let result = result.and_then(|()| {
                    if state.receivedHead {
                        Ok(())
                    } else {
                        Err("HTTP Host closed without headers".into())
                    }
                });
                state.finish(result);
            }),
        )
        .map_err(|error| error.to_string())?;
    drop(state);
    // Notification does not acquire request state: progress callbacks may cancel.
    let (cancelSender, cancelled) = oneshot::channel();
    let cancelSender = Mutex::new(Some(cancelSender));
    let _subscription = control.onCancel(Arc::new(move || {
        if let Some(sender) = cancelSender.lock().expect("update cancellation").take() {
            let _ = sender.send(());
        }
    }));
    tokio::select! {
        biased;
        _ = cancelled => Err("UPDATE_CANCELLED".into()),
        result = receiver => result.map_err(|_| "HTTP Host disconnected without completion".to_string())?,
    }
}

#[cfg(test)]
#[path = "UpdateHttpTests.rs"]
pub(crate) mod tests;
