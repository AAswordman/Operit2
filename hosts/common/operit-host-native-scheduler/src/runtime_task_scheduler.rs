use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use std::sync::OnceLock;

static ASYNC_RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

/// Owns I/O drivers, timers and spawned tasks for the native process lifetime.
/// Connections returned by one request may be reused by later requests.
fn asyncRuntime() -> HostResult<&'static tokio::runtime::Runtime> {
    let runtime = ASYNC_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("operit-runtime-worker")
            .build()
            .map_err(|error| error.to_string())
    });
    runtime
        .as_ref()
        .map_err(|error| HostError::new(format!("create runtime async executor failed: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{rc::Rc, sync::mpsc, time::Duration};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Joining the native thread ensures its request executor has returned before
    // a subsequent request touches the saved connection or spawned receiver.
    fn runRequest(task: HostRuntimeAsyncTask) {
        std::thread::spawn(move || runAsyncRuntimeTask(task))
            .join()
            .expect("request thread panicked");
    }

    #[test]
    fn connectionSurvivesBetweenRequests() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        runRequest(Box::new(move || {
            Box::pin(async move {
                let stream = tokio::net::TcpStream::connect(address).await.unwrap();
                sender.send(stream).unwrap();
            })
        }));
        let mut stream = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        let (mut peer, _) = listener.accept().unwrap();
        runRequest(Box::new(move || {
            Box::pin(async move {
                // Start/finish pairing must be able to use the very same socket.
                std::io::Write::write_all(&mut peer, b"code").unwrap();
                let mut received = [0; 4];
                tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut received))
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(&received, b"code");
                stream.write_all(b"done").await.unwrap();
            })
        }));
    }

    #[test]
    fn spawnedReceiverSurvivesRequestCompletion() {
        let (release, released) = tokio::sync::oneshot::channel();
        let (done, completion) = mpsc::channel();
        runRequest(Box::new(move || {
            Box::pin(async move {
                tokio::spawn(async move {
                    released.await.unwrap();
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    done.send(()).unwrap();
                });
            })
        }));
        release
            .send(())
            .expect("receiver was aborted when request ended");
        completion.recv_timeout(Duration::from_secs(5)).unwrap();
    }

    #[test]
    fn requestFutureCanRemainNonSend() {
        runRequest(Box::new(|| {
            Box::pin(async {
                let local = Rc::new(42);
                let thread = std::thread::current().id();
                tokio::time::sleep(Duration::from_millis(1)).await;
                assert_eq!(*local, 42);
                assert_eq!(std::thread::current().id(), thread);
            })
        }));
    }
}

/// Runs one asynchronous runtime task on its dedicated named native thread.
fn runAsyncRuntimeTask(task: HostRuntimeAsyncTask) {
    // block_on keeps the possibly !Send request future on its named thread;
    // the shared runtime keeps its sockets and tokio::spawn children alive.
    asyncRuntime()
        .expect("create runtime task executor failed")
        .block_on(async move { task().await });
}

/// Schedules one-shot runtime tasks on named native threads.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeHostRuntimeTaskSchedulerHost;

impl NativeHostRuntimeTaskSchedulerHost {
    /// Creates the native runtime task scheduler host.
    pub fn new() -> Self {
        Self
    }
}

impl HostRuntimeTaskSchedulerHost for NativeHostRuntimeTaskSchedulerHost {
    /// Starts the task on a named native thread.
    fn scheduleHostRuntimeTask(&self, taskName: &str, task: HostRuntimeTask) -> HostResult<()> {
        std::thread::Builder::new()
            .name(taskName.to_string())
            .spawn(task)
            .map(|_| ())
            .map_err(|error| {
                HostError::new(format!(
                    "create runtime task thread {taskName} failed: {error}"
                ))
            })
    }

    /// Starts an asynchronous task on its own named native thread.
    fn scheduleHostRuntimeAsyncTask(
        &self,
        taskName: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        // Report initialization errors to the caller, not just a detached thread.
        asyncRuntime()?;
        std::thread::Builder::new()
            .name(taskName.to_string())
            .spawn(move || runAsyncRuntimeTask(task))
            .map(|_| ())
            .map_err(|error| {
                HostError::new(format!(
                    "create runtime async task thread {taskName} failed: {error}"
                ))
            })
    }

    /// Starts a named native task after the requested delay.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        taskName: &str,
        delayMs: u64,
        task: HostRuntimeTask,
    ) -> HostResult<()> {
        let taskName = taskName.to_string();
        asyncRuntime()?.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delayMs)).await;
            std::thread::Builder::new()
                .name(taskName)
                .spawn(task)
                .expect("delayed runtime task thread must start");
        });
        Ok(())
    }

    /// Waits for a later native executor turn without occupying the caller thread.
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        Box::pin(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            Ok(())
        })
    }

    /// Waits through the native timer executor for one platform-owned delay.
    fn waitForHostRuntimeDelay(&self, delayMs: u64) -> HostRuntimeTurnFuture {
        Box::pin(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delayMs)).await;
            Ok(())
        })
    }
}
