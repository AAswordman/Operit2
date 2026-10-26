//! Clocked HTTP writes with one immutable batch in flight and one bounded pending buffer.
use super::inbox::Inbox;
use operit_host_api::HostRuntimeTaskSchedulerHost;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

const FLUSH_INTERVAL_MS: u64 = 16;

pub(super) struct HttpOutbox {
    pending: Mutex<Option<Vec<u8>>>,
    limit: usize,
    inbox: Arc<Inbox>,
    available: Notify,
    space: Notify,
    closed: Notify,
}

impl HttpOutbox {
    /// Shares the receiver's failure state and the existing HTTP body byte limit.
    pub fn new(limit: usize, inbox: Arc<Inbox>) -> Arc<Self> {
        Arc::new(Self {
            pending: Mutex::new(Some(Vec::new())),
            limit,
            inbox,
            available: Notify::new(),
            space: Notify::new(),
            closed: Notify::new(),
        })
    }

    /// Accepts complete frames in order, waiting for byte capacity instead of growing a task queue.
    pub async fn write(&self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > self.limit {
            return Err(crate::MESSAGE_TOO_LARGE_ERROR.into());
        }
        loop {
            let space = self.space.notified();
            {
                let mut pending = self.pending.lock().unwrap();
                self.inbox.status()?;
                let buffer = pending.as_mut().ok_or("HTTP Link closed")?;
                if bytes.len() <= self.limit - buffer.len() {
                    buffer.extend_from_slice(bytes);
                    self.available.notify_one();
                    return Ok(());
                }
            }
            space.await;
        }
    }

    /// Coalesces idle bursts once and drains frames queued during I/O without another delay.
    pub async fn nextBatch(
        &self,
        scheduler: &dyn HostRuntimeTaskSchedulerHost,
    ) -> Result<Option<Vec<u8>>, String> {
        let mut wasIdle = false;
        loop {
            let available = self.available.notified();
            {
                let pending = self.pending.lock().unwrap();
                match pending.as_ref() {
                    None => return Ok(None),
                    Some(bytes) if !bytes.is_empty() => break,
                    Some(_) => {}
                }
            }
            wasIdle = true;
            available.await;
        }
        if wasIdle {
            tokio::select! {
                result = scheduler.waitForHostRuntimeDelay(FLUSH_INTERVAL_MS) => {
                    result.map_err(|error| error.to_string())?;
                }
                _ = self.closed.notified() => return Ok(None),
            }
        }
        let batch = self.pending.lock().unwrap().as_mut().map(std::mem::take);
        self.space.notify_one();
        Ok(batch)
    }

    /// Preserves the first transport result and releases all blocked send and receive operations.
    pub fn finish(&self, result: Result<(), String>) {
        let mut pending = self.pending.lock().unwrap();
        self.inbox.finish(result);
        *pending = None;
        self.available.notify_one();
        self.space.notify_one();
        self.closed.notify_one();
    }
}

#[cfg(test)]
#[path = "http_outbox_tests.rs"]
mod tests;
