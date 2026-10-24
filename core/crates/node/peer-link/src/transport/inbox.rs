//! Bounded byte-stream buffering between synchronous Host callbacks and async readers.
use operit_host_api::{HostResult, HostRuntimeTaskSchedulerHost};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{oneshot, Mutex, Notify};

#[derive(Default)]
struct InboxState {
    bytes: Vec<u8>,
    terminal: Option<Result<(), String>>,
}

pub(super) struct Inbox {
    maxBufferedBytes: usize,
    state: StdMutex<InboxState>,
    reader: Mutex<()>,
    available: Notify,
}

impl Inbox {
    /// Reserves a byte budget for one maximum-sized length-prefixed frame.
    pub fn new(maxMessageBytes: usize) -> Arc<Self> {
        Arc::new(Self {
            maxBufferedBytes: maxMessageBytes + 4,
            state: StdMutex::new(InboxState::default()),
            reader: Mutex::new(()),
            available: Notify::new(),
        })
    }

    /// Preserves the first terminal result and wakes the serialized reader.
    pub fn finish(&self, result: Result<(), String>) {
        let mut state = self.state.lock().unwrap();
        if state.terminal.is_none() {
            state.terminal = Some(result);
            self.available.notify_one();
        }
    }

    /// Buffers Host byte fragments without treating callback boundaries as messages.
    pub fn put(&self, bytes: Vec<u8>) {
        let mut state = self.state.lock().unwrap();
        if state.terminal.is_some() {
            return;
        }
        if bytes.len() > self.maxBufferedBytes - state.bytes.len() {
            state.terminal = Some(Err(format!(
                "Peer receive buffer exceeded byte limit: buffered={} incoming={} limit={}",
                state.bytes.len(),
                bytes.len(),
                self.maxBufferedBytes,
            )));
        } else {
            state.bytes.extend(bytes);
        }
        self.available.notify_one();
    }

    /// Returns buffered bytes in order, preserving errors and draining a clean close.
    pub async fn read(&self) -> Result<Option<Vec<u8>>, String> {
        let _reader = self.reader.lock().await;
        loop {
            {
                let mut state = self.state.lock().unwrap();
                if let Some(Err(error)) = &state.terminal {
                    return Err(error.clone());
                }
                if !state.bytes.is_empty() {
                    return Ok(Some(std::mem::take(&mut state.bytes)));
                }
                if state.terminal.is_some() {
                    return Ok(None);
                }
            }
            self.available.notified().await;
        }
    }

    /// Reports terminal transport status without consuming buffered bytes.
    pub fn status(&self) -> Result<(), String> {
        match &self.state.lock().unwrap().terminal {
            None => Ok(()),
            Some(Err(error)) => Err(error.clone()),
            Some(Ok(())) => Err("Peer connection closed".into()),
        }
    }
}

/// Runs synchronous Host I/O on the installed scheduler without blocking the executor.
pub(super) async fn hostTask<T: Send + 'static>(
    scheduler: &Arc<dyn HostRuntimeTaskSchedulerHost>,
    task: impl FnOnce() -> HostResult<T> + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = oneshot::channel();
    scheduler
        .scheduleHostRuntimeTask(
            "peer-transport-io",
            Box::new(move || {
                let _ = tx.send(task());
            }),
        )
        .map_err(|e| e.to_string())?;
    rx.await
        .map_err(|_| "Host transport task cancelled".to_string())?
        .map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "inbox_tests.rs"]
mod tests;
