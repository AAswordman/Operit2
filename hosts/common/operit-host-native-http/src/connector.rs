//! Executes connection setup independently of the HTTP request poll stack.
use std::{
    error::Error,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Layer, Service};

type ConnectionError = Box<dyn Error + Send + Sync>;

#[derive(Clone)]
pub(super) struct ConnectionTaskLayer;

#[derive(Clone)]
pub(super) struct ConnectionTask<S>(S);

impl<S> Layer<S> for ConnectionTaskLayer {
    type Service = ConnectionTask<S>;

    /// Wraps the Host connector without changing transport selection or policy.
    fn layer(&self, service: S) -> Self::Service {
        ConnectionTask(service)
    }
}

impl<S, R> Service<R> for ConnectionTask<S>
where
    S: Service<R, Error = ConnectionError> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    R: Send + 'static,
{
    type Response = S::Response;
    type Error = ConnectionError;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, ConnectionError>> + Send>>;

    /// Preserves the underlying connector's readiness contract.
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    /// Polls connection setup as a cancellable task on the same Host executor.
    fn call(&mut self, request: R) -> Self::Future {
        let replacement = self.0.clone();
        let mut service = std::mem::replace(&mut self.0, replacement);
        Box::pin(async move { runTask(async move { service.call(request).await }).await? })
    }
}

/// Polls one network operation at a fresh task root on the same bounded executor.
/// Dropping the caller aborts the operation instead of leaving detached I/O.
pub(super) async fn runTask<F>(operation: F) -> Result<F::Output, ConnectionError>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let mut task = tokio::task::JoinSet::new();
    task.spawn(operation);
    task.join_next().await
        .ok_or_else(|| std::io::Error::other("HTTP network task is missing"))?
        .map_err(|error| -> ConnectionError { Box::new(error) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::sync::oneshot;

    #[derive(Clone)]
    struct PendingConnector {
        started: Arc<Mutex<Option<oneshot::Sender<std::thread::ThreadId>>>>,
        released: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    }

    struct ConnectionLease(Option<oneshot::Sender<()>>);

    impl Drop for ConnectionLease {
        /// Reports release even when connection setup is cancelled before completion.
        fn drop(&mut self) {
            if let Some(released) = self.0.take() {
                let _ = released.send(());
            }
        }
    }

    impl Service<()> for PendingConnector {
        type Response = ();
        type Error = ConnectionError;
        type Future = Pin<Box<dyn Future<Output = Result<(), ConnectionError>> + Send>>;

        /// Makes the controlled connector immediately ready.
        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        /// Holds a cancellable connection open until its task is dropped.
        fn call(&mut self, _: ()) -> Self::Future {
            let lease = ConnectionLease(self.released.lock().unwrap().take());
            self.started
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(std::thread::current().id())
                .unwrap();
            Box::pin(async move {
                let _lease = lease;
                std::future::pending().await
            })
        }
    }

    /// Setup yields to the same runtime and dropping its caller releases in-flight work.
    #[tokio::test]
    async fn connection_setup_yields_and_cancellation_releases_its_task() {
        let (started, entered) = oneshot::channel();
        let (released, finished) = oneshot::channel();
        let inner = PendingConnector {
            started: Arc::new(Mutex::new(Some(started))),
            released: Arc::new(Mutex::new(Some(released))),
        };
        let mut service = ConnectionTaskLayer.layer(inner.clone());
        let mut connection = service.call(());
        assert!(futures_util::poll!(&mut connection).is_pending());
        assert!(
            inner.started.lock().unwrap().is_some(),
            "connector must leave the request poll stack"
        );
        assert_eq!(entered.await.unwrap(), std::thread::current().id());
        drop(connection);
        finished.await.unwrap();
    }
}
