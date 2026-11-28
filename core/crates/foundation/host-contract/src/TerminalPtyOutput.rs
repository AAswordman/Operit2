//! Bounded binary PTY output with producer wakeups and explicit end-of-stream.

use crate::{HostError, HostResult};
use std::collections::VecDeque;
use std::future::poll_fn;
use std::sync::Mutex;
use std::task::{Poll, Waker};

/// Owns the unread PTY bytes independently of command capture and screen history.
pub struct TerminalPtyOutputBuffer {
    state: Mutex<OutputState>,
    limit: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Wake};

    #[derive(Default)]
    struct WakeCount(AtomicUsize);

    impl Wake for WakeCount {
        /// Counts producer notifications without any timer or executor thread.
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Proves idle reads sleep and only actual producer events wake them.
    #[test]
    fn output_wakes_a_pending_binary_read() {
        let output = TerminalPtyOutputBuffer::new(64);
        let counter = Arc::new(WakeCount::default());
        let waker = counter.clone().into();
        let mut context = Context::from_waker(&waker);
        let mut read = Box::pin(output.next());
        assert!(read.as_mut().poll(&mut context).is_pending());
        assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        output.append(&[0, 0xff, 0xc3]);
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        match read.as_mut().poll(&mut context) {
            Poll::Ready(Ok(Some(bytes))) => assert_eq!(bytes, [0, 0xff, 0xc3]),
            _ => panic!("binary producer event was lost"),
        }
    }

    /// Preserves final bytes before EOF and wakes a completely idle close.
    #[test]
    fn final_output_precedes_eof() {
        let output = TerminalPtyOutputBuffer::new(64);
        output.append(b"tail");
        output.close();
        let counter = Arc::new(WakeCount::default());
        let waker = counter.clone().into();
        let mut context = Context::from_waker(&waker);
        assert!(matches!(Box::pin(output.next()).as_mut().poll(&mut context), Poll::Ready(Ok(Some(bytes))) if bytes == b"tail"));
        assert!(matches!(Box::pin(output.next()).as_mut().poll(&mut context), Poll::Ready(Ok(None))));
        let idle = TerminalPtyOutputBuffer::new(64);
        let mut read = Box::pin(idle.next());
        assert!(read.as_mut().poll(&mut context).is_pending());
        idle.close();
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        assert!(matches!(read.as_mut().poll(&mut context), Poll::Ready(Ok(None))));
    }

    /// Keeps unattached output bounded and preserves data before a durable failure.
    #[test]
    fn bounded_tail_drains_before_error() {
        let output = TerminalPtyOutputBuffer::new(4);
        output.append(b"123456");
        output.fail(HostError::new("reader failed"));
        assert_eq!(output.read().unwrap(), b"3456");
        let counter = Arc::new(WakeCount::default());
        let waker = counter.into();
        let mut context = Context::from_waker(&waker);
        assert!(matches!(Box::pin(output.next()).as_mut().poll(&mut context), Poll::Ready(Err(error)) if error.message == "reader failed"));
    }
}

#[derive(Default)]
struct OutputState {
    bytes: VecDeque<u8>,
    closed: bool,
    error: Option<String>,
    waker: Option<Waker>,
}

impl TerminalPtyOutputBuffer {
    /// Creates a bounded output tail for an unattached or paused UI consumer.
    pub fn new(limit: usize) -> Self {
        assert!(limit > 0);
        Self { state: Mutex::new(OutputState::default()), limit }
    }

    /// Publishes a complete byte fragment before waking its asynchronous reader.
    pub fn append(&self, bytes: &[u8]) {
        if bytes.is_empty() { return; }
        let waker = {
            let mut state = self.state.lock().expect("PTY output state poisoned");
            if state.closed { return; }
            state.bytes.extend(bytes.iter().copied());
            let excess = state.bytes.len().saturating_sub(self.limit);
            state.bytes.drain(..excess);
            state.waker.take()
        };
        if let Some(waker) = waker { waker.wake(); }
    }

    /// Marks EOF after all producer bytes have been queued.
    pub fn close(&self) {
        let waker = {
            let mut state = self.state.lock().expect("PTY output state poisoned");
            state.closed = true;
            state.waker.take()
        };
        if let Some(waker) = waker { waker.wake(); }
    }

    /// Publishes a terminal failure after allowing already queued bytes to drain.
    pub fn fail(&self, error: HostError) {
        {
            let mut state = self.state.lock().expect("PTY output state poisoned");
            state.error = Some(error.message);
        }
        self.close();
    }

    /// Drains the currently available bytes for explicit synchronous callers.
    pub fn read(&self) -> HostResult<Vec<u8>> {
        let mut state = self.state.lock().map_err(|_| HostError::new("PTY output state poisoned"))?;
        Ok(state.bytes.drain(..).collect())
    }

    /// Clears startup output without altering the producer's terminal state.
    pub fn clear(&self) -> HostResult<()> {
        self.read().map(|_| ())
    }

    /// Sleeps until bytes, EOF, or failure; registration and draining are atomic.
    pub async fn next(&self) -> HostResult<Option<Vec<u8>>> {
        poll_fn(|context| {
            let mut state = self.state.lock().expect("PTY output state poisoned");
            if !state.bytes.is_empty() {
                return Poll::Ready(Ok(Some(state.bytes.drain(..).collect())));
            }
            if let Some(error) = &state.error {
                return Poll::Ready(Err(HostError::new(error.clone())));
            }
            if state.closed { return Poll::Ready(Ok(None)); }
            state.waker = Some(context.waker().clone());
            Poll::Pending
        }).await
    }
}
