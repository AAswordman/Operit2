//! Readiness-driven Unix streams owned by the native Host I/O runtime.

use crate::NativeHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostError, HostResult, HostRuntimeTaskSchedulerHost};
use std::io::{self, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use tokio::io::unix::AsyncFd;
use tokio::sync::watch;

/// Cancels one reader through a durable notification rather than a polling interval.
pub struct NativeFdReader {
    stop: watch::Sender<bool>,
    finished: Option<std::sync::mpsc::Receiver<()>>,
}

impl NativeFdReader {
    /// Waits for descriptor release only at an explicit synchronous Host teardown boundary.
    pub fn close(mut self) -> HostResult<()> {
        let _ = self.stop.send(true);
        self.finished
            .take()
            .expect("reader completion receiver")
            .recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|error| HostError::new(format!("Terminal reader shutdown failed: {error}")))
    }
}

impl Drop for NativeFdReader {
    /// Wakes the reader immediately, including when its descriptor has no output.
    fn drop(&mut self) {
        let _ = self.stop.send(true);
    }
}

/// Retains the close callback until every reader exit path has notified its consumer.
struct ReaderCompletion<F: FnOnce(HostError)>(Option<F>, std::sync::mpsc::SyncSender<()>);

impl<F: FnOnce(HostError)> Drop for ReaderCompletion<F> {
    /// Propagates runtime cancellation or panic as a terminal reader failure.
    fn drop(&mut self) {
        if let Some(completed) = self.0.take() {
            completed(HostError::new("Terminal reader stopped"));
        }
        let _ = self.1.send(());
    }
}

/// Schedules a nonblocking descriptor on readiness events without reserving a service worker.
pub fn scheduleNativeFdReader(
    name: &str,
    descriptor: OwnedFd,
    onData: impl FnMut(&[u8]) -> HostResult<()> + Send + 'static,
    onClose: impl FnOnce(HostError) + Send + 'static,
) -> HostResult<NativeFdReader> {
    let flags = unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe {
            libc::fcntl(
                descriptor.as_raw_fd(),
                libc::F_SETFL,
                flags | libc::O_NONBLOCK,
            )
        } < 0
    {
        return Err(io::Error::last_os_error().into());
    }
    let (stop, mut stopping) = watch::channel(false);
    let (finished, released) = std::sync::mpsc::sync_channel(1);
    let completion = ReaderCompletion(Some(onClose), finished);
    NativeHostRuntimeTaskSchedulerHost::new().scheduleHostRuntimeCooperativeAsyncTask(name, Box::new(move || Box::pin(async move {
        let mut completion = completion;
        let mut onData = onData;
        let result: HostResult<()> = async {
            let descriptor = AsyncFd::new(descriptor)?;
            let mut bytes = [0u8; 64 * 1024];
            loop {
                tokio::task::consume_budget().await;
                let mut ready = tokio::select! {
                    biased;
                    _ = stopping.changed() => {
                        // Drain the already-written tail once; descendants cannot prolong shutdown.
                        let count = unsafe { libc::read(descriptor.get_ref().as_raw_fd(), bytes.as_mut_ptr().cast(), bytes.len()) };
                        if count > 0 { onData(&bytes[..count as usize])?; }
                        return Err(HostError::new("Terminal reader stopped"));
                    },
                    ready = descriptor.readable() => ready?,
                };
                match ready.try_io(|descriptor| {
                    let count = unsafe { libc::read(descriptor.get_ref().as_raw_fd(), bytes.as_mut_ptr().cast(), bytes.len()) };
                    if count < 0 { Err(io::Error::last_os_error()) } else { Ok(count as usize) }
                }) {
                    Ok(Ok(0)) => return Err(HostError::new("PTY reader reached EOF")),
                    Ok(Ok(count)) => onData(&bytes[..count])?,
                    Ok(Err(error)) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Ok(Err(error)) => return Err(error.into()),
                    Err(_) => continue,
                }
            }
        }.await;
        drop(onData);
        if let Some(completed) = completion.0.take() {
            completed(result.unwrap_err());
        }
    })))?;
    Ok(NativeFdReader {
        stop,
        finished: Some(released),
    })
}

/// Adapts a shared nonblocking PTY descriptor to explicit synchronous write contracts.
pub struct NativeFdWriter(pub std::fs::File);

impl Write for NativeFdWriter {
    /// Waits for writable readiness only at an explicit synchronous OS submission boundary.
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            match self.0.write(bytes) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                    if remaining.is_zero() {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "PTY input submission timed out",
                        ));
                    }
                    let mut descriptor = libc::pollfd {
                        fd: self.0.as_raw_fd(),
                        events: libc::POLLOUT,
                        revents: 0,
                    };
                    let result = unsafe {
                        libc::poll(
                            &mut descriptor,
                            1,
                            remaining.as_millis().min(i32::MAX as u128) as i32,
                        )
                    };
                    if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted
                    {
                        return Err(io::Error::last_os_error());
                    }
                }
                result => return result,
            }
        }
    }
    /// Flushes the descriptor without introducing a timer or a background wait.
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream;
    use std::sync::mpsc;
    use std::time::Duration;

    /// Silent readers beyond service capacity must remain live and cancel through an event.
    #[test]
    fn many_silent_readers_share_io_workers_and_close_without_polling() {
        let (sent, received) = mpsc::channel();
        let mut readers = Vec::new();
        let mut peers = Vec::new();
        for index in 0..32 {
            let (reader, peer) = UnixStream::pair().unwrap();
            let data = sent.clone();
            let closed = sent.clone();
            readers.push(
                scheduleNativeFdReader(
                    "test-terminal-reader",
                    reader.into(),
                    move |bytes| {
                        data.send((index, bytes.to_vec())).unwrap();
                        Ok(())
                    },
                    move |_| {
                        closed.send((index, Vec::new())).unwrap();
                    },
                )
                .unwrap(),
            );
            peers.push(peer);
        }
        for peer in &mut peers {
            peer.write_all(b"output").unwrap();
        }
        let mut outputs = std::collections::BTreeSet::new();
        for _ in 0..32 {
            let (index, bytes) = received.recv_timeout(Duration::from_secs(5)).unwrap();
            assert_eq!(bytes, b"output");
            outputs.insert(index);
        }
        assert_eq!(outputs.len(), 32);
        drop(readers);
        let mut closes = std::collections::BTreeSet::new();
        for _ in 0..32 {
            let (index, bytes) = received.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(bytes.is_empty());
            closes.insert(index);
        }
        assert_eq!(closes.len(), 32);
    }
}
