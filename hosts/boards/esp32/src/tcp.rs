//! ESP-IDF socket I/O only. No Link framing, pairing or routing belongs here.
use async_trait::async_trait;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostError, HostResult, TcpConnection, TcpHost, TcpListener};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener as SocketListener, TcpStream, ToSocketAddrs};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex as StdMutex,
};
use tokio::sync::Mutex;

/// BSD sockets are a board Host capability, not an Edge protocol implementation.
#[derive(Default)]
pub struct Esp32TcpHost;

#[async_trait]
impl TcpHost for Esp32TcpHost {
    async fn connect(&self, address: &str) -> HostResult<Arc<dyn TcpConnection>> {
        // DNS and connect must not block the firmware's async executor. If the
        // caller cancels, the result (and its socket) is dropped by the worker.
        let address = address.to_owned();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        std::thread::Builder::new()
            .name("host-tcp-connect".into())
            .spawn(move || {
                let result = (|| {
                    let addresses = address
                        .to_socket_addrs()
                        .map_err(|error| HostError::new(error.to_string()))?;
                    let mut last_error = HostError::new("TCP address resolved to no endpoints");
                    for address in addresses {
                        match TcpStream::connect_timeout(
                            &address,
                            std::time::Duration::from_secs(10),
                        ) {
                            Ok(stream) => {
                                return Esp32TcpConnection::from_stream(stream)
                                    .map(|connection| connection as Arc<dyn TcpConnection>)
                            }
                            Err(error) => last_error = HostError::new(error.to_string()),
                        }
                    }
                    Err(last_error)
                })();
                let _ = sender.send(result);
            })
            .map_err(|error| HostError::new(error.to_string()))?;
        receiver
            .await
            .map_err(|_| HostError::new("TCP connect worker ended"))?
    }

    async fn bind(&self, address: &str) -> HostResult<Arc<dyn TcpListener>> {
        let listener =
            SocketListener::bind(address).map_err(|error| HostError::new(error.to_string()))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| HostError::new(error.to_string()))?;
        let address = listener
            .local_addr()
            .map_err(|error| HostError::new(error.to_string()))?
            .to_string();
        Ok(Arc::new(Esp32TcpListener {
            listener: Mutex::new(Some(listener)),
            address,
        }))
    }
}

struct Esp32TcpListener {
    listener: Mutex<Option<SocketListener>>,
    address: String,
}
#[async_trait]
impl TcpListener for Esp32TcpListener {
    fn local_address(&self) -> HostResult<String> {
        Ok(self.address.clone())
    }

    async fn accept(&self) -> HostResult<Arc<dyn TcpConnection>> {
        loop {
            // Release the lock before waiting so close() can release the port
            // even while an accept is pending.
            {
                let guard = self.listener.lock().await;
                let listener = guard
                    .as_ref()
                    .ok_or_else(|| HostError::new("TCP listener is closed"))?;
                match listener.accept() {
                    Ok((stream, _)) => {
                        return Esp32TcpConnection::from_stream(stream)
                            .map(|connection| connection as Arc<dyn TcpConnection>)
                    }
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            || error.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(HostError::new(error.to_string())),
                }
            }
            defaultHostRuntimeTaskSchedulerHost()
                .waitForHostRuntimeDelay(2)
                .await?;
        }
    }

    async fn close(&self) {
        self.listener.lock().await.take();
    }
}

struct Esp32TcpConnection {
    stream: StdMutex<Option<TcpStream>>,
    remote: Option<std::net::SocketAddr>,
    reader: Mutex<()>,
    writer: Mutex<()>,
    closed: AtomicBool,
}
impl Esp32TcpConnection {
    fn from_stream(stream: TcpStream) -> HostResult<Arc<Self>> {
        stream
            .set_nonblocking(true)
            .map_err(|error| HostError::new(error.to_string()))?;
        let remote = stream.peer_addr().ok();
        Ok(Arc::new(Self {
            stream: StdMutex::new(Some(stream)),
            remote,
            reader: Mutex::new(()),
            writer: Mutex::new(()),
            closed: AtomicBool::new(false),
        }))
    }
}
#[async_trait]
impl TcpConnection for Esp32TcpConnection {
    fn remote_address(&self) -> Option<std::net::SocketAddr> {
        self.remote
    }

    async fn write(&self, bytes: &[u8]) -> HostResult<()> {
        let _guard = self.writer.lock().await;
        let mut offset = 0;
        while offset < bytes.len() {
            if self.closed.load(Ordering::Acquire) {
                return Err(HostError::new("TCP connection is closed"));
            }
            let result = {
                let stream = self
                    .stream
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let mut stream = stream
                    .as_ref()
                    .ok_or_else(|| HostError::new("TCP connection is closed"))?;
                stream.write(&bytes[offset..])
            };
            match result {
                Ok(0) => return Err(HostError::new("TCP peer closed while writing")),
                Ok(count) => offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    defaultHostRuntimeTaskSchedulerHost()
                        .waitForHostRuntimeDelay(2)
                        .await?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(HostError::new(error.to_string())),
            }
        }
        Ok(())
    }
    async fn read(&self) -> HostResult<Option<Vec<u8>>> {
        let _guard = self.reader.lock().await;
        // Keep idle reads small; preserve the buffer across WouldBlock without
        // allocating a desktop-sized 4 KiB block for every connection.
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(512)
            .map_err(|_| HostError::new("Insufficient memory for TCP receive buffer"))?;
        bytes.resize(512, 0);
        loop {
            if self.closed.load(Ordering::Acquire) {
                return Ok(None);
            }
            let result = {
                let stream = self
                    .stream
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let Some(mut stream) = stream.as_ref() else {
                    return Ok(None);
                };
                stream.read(&mut bytes)
            };
            match result {
                Ok(0) => return Ok(None),
                Ok(count) => {
                    bytes.truncate(count);
                    return Ok(Some(bytes));
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    defaultHostRuntimeTaskSchedulerHost()
                        .waitForHostRuntimeDelay(2)
                        .await?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(HostError::new(error.to_string())),
            }
        }
    }
    async fn close(&self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            if let Some(stream) = self
                .stream
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .take()
            {
                let _ = stream.shutdown(Shutdown::Both);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::{poll_fn, Future};
    use std::task::Poll;
    use std::time::Duration;

    fn scheduler() {
        operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
            operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost,
        ));
    }

    async fn pending_once<F: Future>(future: std::pin::Pin<&mut F>) {
        let mut future = future;
        poll_fn(|cx| {
            assert!(future.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
    }

    #[tokio::test]
    async fn host_connect_accept_read_write_and_eof() {
        scheduler();
        tokio::time::timeout(Duration::from_secs(5), async {
            let listener = Esp32TcpHost.bind("127.0.0.1:0").await.unwrap();
            let client = Esp32TcpHost
                .connect(&listener.local_address().unwrap())
                .await
                .unwrap();
            let server = listener.accept().await.unwrap();
            client.write(b"call/watch/push bytes").await.unwrap();
            let mut received = Vec::new();
            while received.len() < 21 {
                received.extend(server.read().await.unwrap().unwrap());
            }
            assert_eq!(received, b"call/watch/push bytes");
            assert!(server.remote_address().unwrap().ip().is_loopback());
            let payload = vec![0x5a; 2049];
            client.write(&payload).await.unwrap();
            let mut received = Vec::new();
            while received.len() < payload.len() {
                let chunk = server.read().await.unwrap().unwrap();
                assert!(chunk.len() <= 512);
                received.extend(chunk);
            }
            assert_eq!(received, payload);
            server.write(b"response").await.unwrap();
            assert_eq!(client.read().await.unwrap().unwrap(), b"response");
            client.close().await;
            client.close().await;
            assert!(server.read().await.unwrap().is_none());
            server.close().await;
            listener.close().await;
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn close_releases_listener_with_pending_accept() {
        scheduler();
        tokio::time::timeout(Duration::from_secs(5), async {
            let listener = Esp32TcpHost.bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_address().unwrap();
            let mut accept = Box::pin(listener.accept());
            pending_once(accept.as_mut()).await;
            listener.close().await;
            assert!(accept.await.is_err());
            listener.close().await;
            let rebound = Esp32TcpHost.bind(&address).await.unwrap();
            rebound.close().await;
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn cancelled_read_loses_no_bytes_and_close_ends_pending_read() {
        scheduler();
        tokio::time::timeout(Duration::from_secs(5), async {
            let listener = Esp32TcpHost.bind("127.0.0.1:0").await.unwrap();
            let client = Esp32TcpHost
                .connect(&listener.local_address().unwrap())
                .await
                .unwrap();
            let server = listener.accept().await.unwrap();
            let mut read = Box::pin(server.read());
            pending_once(read.as_mut()).await;
            drop(read);
            client.write(b"preserved").await.unwrap();
            assert_eq!(server.read().await.unwrap().unwrap(), b"preserved");
            let mut read = Box::pin(server.read());
            pending_once(read.as_mut()).await;
            server.close().await;
            assert!(read.await.unwrap().is_none());
            assert!(server.write(b"closed").await.is_err());
            client.close().await;
            listener.close().await;
        })
        .await
        .unwrap();
    }
}
