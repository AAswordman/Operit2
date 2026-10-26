use async_trait::async_trait;
use operit_host_api::{HostError, HostResult, TcpConnection, TcpHost, TcpListener};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{watch, Mutex};

#[derive(Default)]
pub struct NativeTcpHost;

#[async_trait]
impl TcpHost for NativeTcpHost {
    async fn connect(&self, address: &str) -> HostResult<Arc<dyn TcpConnection>> {
        let stream = TcpStream::connect(address)
            .await
            .map_err(|error| HostError::new(format!("TCP connect {address}: {error}")))?;
        Ok(Arc::new(NativeTcpConnection::from_stream(stream)))
    }

    async fn bind(&self, address: &str) -> HostResult<Arc<dyn TcpListener>> {
        let listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(|error| HostError::new(error.to_string()))?;
        let address = listener
            .local_addr()
            .map_err(|error| HostError::new(error.to_string()))?
            .to_string();
        let (closed, _) = watch::channel(false);
        Ok(Arc::new(NativeTcpListener {
            listener: Mutex::new(Some(listener)),
            address,
            closed,
        }))
    }
}

pub struct NativeTcpConnection {
    remote: Option<std::net::SocketAddr>,
    reader: Mutex<Option<ReadHalf<TcpStream>>>,
    writer: Mutex<Option<WriteHalf<TcpStream>>>,
    closed: watch::Sender<bool>,
}

impl NativeTcpConnection {
    pub fn from_stream(stream: TcpStream) -> Self {
        let remote = stream.peer_addr().ok();
        let (reader, writer) = tokio::io::split(stream);
        let (closed, _) = watch::channel(false);
        Self {
            remote,
            reader: Mutex::new(Some(reader)),
            writer: Mutex::new(Some(writer)),
            closed,
        }
    }
}

#[async_trait]
impl TcpConnection for NativeTcpConnection {
    fn remote_address(&self) -> Option<std::net::SocketAddr> {
        self.remote
    }
    async fn write(&self, bytes: &[u8]) -> HostResult<()> {
        let mut closed = self.closed.subscribe();
        tokio::select! {
            biased;
            _ = closed.wait_for(|value| *value) => Err(HostError::new("TCP connection is closed")),
            result = async {
                let mut writer = self.writer.lock().await;
                let writer = writer.as_mut().ok_or_else(|| HostError::new("TCP connection is closed"))?;
                writer.write_all(bytes).await.map_err(|error| HostError::new(error.to_string()))
            } => result,
        }
    }

    async fn read(&self) -> HostResult<Option<Vec<u8>>> {
        let mut closed = self.closed.subscribe();
        tokio::select! {
            biased;
            _ = closed.wait_for(|value| *value) => Ok(None),
            result = async {
                let mut reader = self.reader.lock().await;
                let Some(reader) = reader.as_mut() else { return Ok(None); };
                let mut bytes = vec![0; 4096];
                let count = reader.read(&mut bytes).await.map_err(|error| HostError::new(error.to_string()))?;
                bytes.truncate(count);
                Ok((count > 0).then_some(bytes))
            } => result,
        }
    }

    async fn close(&self) {
        self.closed.send_replace(true);
        // Closing wakes operations before acquiring locks, including blocked reads.
        self.reader.lock().await.take();
        self.writer.lock().await.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn host_listener_accepts_and_close_wakes_accept() {
        let listener = NativeTcpHost.bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_address().unwrap();
        let (client, server) = tokio::join!(NativeTcpHost.connect(&address), listener.accept());
        let client = client.unwrap();
        let server = server.unwrap();
        client.write(b"host").await.unwrap();
        assert_eq!(server.read().await.unwrap().unwrap(), b"host");
        client.close().await;
        assert!(server.read().await.unwrap().is_none());
        server.close().await;
        let (accept, _) = tokio::join!(listener.accept(), async {
            tokio::task::yield_now().await;
            listener.close().await;
        });
        assert!(accept.is_err());
        listener.close().await;
    }

    #[tokio::test]
    async fn host_transfers_bytes_and_close_wakes_read() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let (connection, accepted) =
            tokio::join!(NativeTcpHost.connect(&address), listener.accept());
        let connection = connection.unwrap();
        let (mut remote, _) = accepted.unwrap();
        remote.write_all(b"incoming").await.unwrap();
        assert_eq!(connection.read().await.unwrap().unwrap(), b"incoming");
        connection.write(b"outgoing").await.unwrap();
        let mut bytes = [0; 8];
        remote.read_exact(&mut bytes).await.unwrap();
        assert_eq!(&bytes, b"outgoing");
        let (read, _) = tokio::join!(connection.read(), async {
            tokio::task::yield_now().await;
            connection.close().await;
        });
        assert_eq!(read.unwrap(), None);
        assert!(connection.write(b"closed").await.is_err());
        connection.close().await;
    }
}

struct NativeTcpListener {
    listener: Mutex<Option<tokio::net::TcpListener>>,
    address: String,
    closed: watch::Sender<bool>,
}
#[async_trait]
impl TcpListener for NativeTcpListener {
    fn local_address(&self) -> HostResult<String> {
        Ok(self.address.clone())
    }
    async fn accept(&self) -> HostResult<Arc<dyn TcpConnection>> {
        let mut closed = self.closed.subscribe();
        tokio::select! {
            biased;
            _ = closed.wait_for(|value| *value) => Err(HostError::new("TCP listener is closed")),
            result = async {
                let listener = self.listener.lock().await;
                let listener = listener.as_ref().ok_or_else(|| HostError::new("TCP listener is closed"))?;
                let (stream, _) = listener.accept().await.map_err(|error| HostError::new(error.to_string()))?;
                Ok(Arc::new(NativeTcpConnection::from_stream(stream)) as Arc<dyn TcpConnection>)
            } => result,
        }
    }
    async fn close(&self) {
        self.closed.send_replace(true);
        self.listener.lock().await.take();
    }
}
