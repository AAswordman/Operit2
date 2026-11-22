//! Explicit native synchronous adapters for the shared event-driven terminal protocol.

use operit_host_api::TerminalCommand::{TerminalCommandBuffer, TerminalCommandExecution};
use operit_host_api::{HostError, HostResult, TerminalCommandOutput};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

/// Wakes a synchronous native caller when its session publishes output or closes.
struct TerminalWake(std::thread::Thread);

impl Wake for TerminalWake {
    /// Retains notifications that arrive before the observing caller parks.
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Parks a native caller until producer progress or one absolute command deadline.
fn waitForTerminalEvent<T>(
    timeoutMs: u64,
    mut poll: impl FnMut(&mut Context<'_>, bool) -> Poll<HostResult<T>>,
) -> HostResult<T> {
    let deadline = (timeoutMs != 0).then(|| Instant::now() + Duration::from_millis(timeoutMs));
    let waker = Waker::from(Arc::new(TerminalWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    loop {
        let expired = deadline.is_some_and(|deadline| Instant::now() >= deadline);
        match poll(&mut context, expired) {
            Poll::Ready(result) => return result,
            Poll::Pending => match deadline {
                Some(deadline) => {
                    std::thread::park_timeout(deadline.saturating_duration_since(Instant::now()))
                }
                None => std::thread::park(),
            },
        }
    }
}

/// Uses the backend's sole command state machine for explicit synchronous native APIs.
pub fn runTerminalCommandSynchronously(
    mut execution: Box<dyn TerminalCommandExecution>,
    onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
) -> HostResult<TerminalCommandOutput> {
    let result = waitForTerminalEvent(execution.timeoutMs(), |context, expired| {
        execution.pollOutput(context, expired, onOutput)
    });
    let settled = if result.as_ref().map_or(true, |receipt| receipt.timedOut) {
        execution.beginInterrupt().and_then(|()| {
            waitForTerminalEvent(3000, |context, expired| {
                execution.pollInterrupt(context, expired)
            })
        })
    } else {
        Ok(())
    };
    execution.finish(result, settled)
}

/// Waits for prompt bytes through the same producer wakeups used by asynchronous commands.
pub fn waitForTerminalPrompt(
    output: &TerminalCommandBuffer,
    timeoutMs: u64,
    mut ready: impl FnMut(&[u8]) -> HostResult<bool>,
    timeoutError: impl Fn(&[u8]) -> HostError,
) -> HostResult<()> {
    let mut collected = Vec::new();
    waitForTerminalEvent(timeoutMs, |context, expired| {
        let failure = output.pollDrain(context, &mut collected)?;
        if ready(&collected)? {
            Poll::Ready(Ok(()))
        } else if let Some(error) = failure {
            Poll::Ready(Err(error))
        } else if expired {
            Poll::Ready(Err(timeoutError(&collected)))
        } else {
            Poll::Pending
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// Fragmented output and silent reader failure both wake a parked startup observer.
    #[test]
    fn startup_wait_uses_producer_events_and_preserves_close_errors() {
        for closes in [false, true] {
            let output = Arc::new(TerminalCommandBuffer::new(1024));
            let waiting = output.clone();
            let (polled, observed) = mpsc::channel();
            let waiter = std::thread::spawn(move || {
                let mut previous = usize::MAX;
                waitForTerminalPrompt(
                    &waiting,
                    1000,
                    |bytes| {
                        if previous != bytes.len() {
                            previous = bytes.len();
                            polled.send(previous).unwrap();
                        }
                        Ok(bytes == b"ready")
                    },
                    |_| HostError::new("startup deadline expired"),
                )
            });
            assert_eq!(observed.recv_timeout(Duration::from_secs(1)).unwrap(), 0);
            output.append(b"rea");
            assert_eq!(observed.recv_timeout(Duration::from_secs(1)).unwrap(), 3);
            if closes {
                output.fail(HostError::new("silent close"));
                assert_eq!(waiter.join().unwrap().unwrap_err().message, "silent close");
            } else {
                output.append(b"dy");
                waiter.join().unwrap().unwrap();
            }
        }
    }
}
