use async_trait::async_trait;
use operit_host_api::{HostError, HostResult, SerialPortConnection, SerialPortHost};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::{watch, Mutex};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

#[derive(Default)]
pub struct NativeSerialPortHost;

#[async_trait]
impl SerialPortHost for NativeSerialPortHost {
    async fn open(&self, port: &str, baud_rate: u32) -> HostResult<Arc<dyn SerialPortConnection>> {
        if port.trim().is_empty() || baud_rate == 0 {
            return Err(HostError::new(
                "Serial port and a nonzero baud rate are required",
            ));
        }
        let stream = tokio_serial::new(port, baud_rate)
            .open_native_async()
            .map_err(|error| HostError::new(format!("Open serial port {port}: {error}")))?;
        Ok(Arc::new(NativeSerialConnection::from_stream(stream)))
    }
}

struct NativeSerialConnection {
    reader: Mutex<Option<ReadHalf<SerialStream>>>,
    writer: Mutex<Option<WriteHalf<SerialStream>>>,
    closed: watch::Sender<bool>,
}

impl NativeSerialConnection {
    fn from_stream(stream: SerialStream) -> Self {
        let (reader, writer) = tokio::io::split(stream);
        let (closed, _) = watch::channel(false);
        Self {
            reader: Mutex::new(Some(reader)),
            writer: Mutex::new(Some(writer)),
            closed,
        }
    }
}

#[async_trait]
impl SerialPortConnection for NativeSerialConnection {
    async fn write(&self, bytes: &[u8]) -> HostResult<()> {
        let mut closed = self.closed.subscribe();
        tokio::select! {
            biased;
            _ = closed.wait_for(|value| *value) => Err(HostError::new("Serial port is closed")),
            result = async {
                let mut writer = self.writer.lock().await;
                let writer = writer.as_mut().ok_or_else(|| HostError::new("Serial port is closed"))?;
                // SerialStream is unbuffered. flush() calls blocking tcdrain on
                // Darwin, preventing cancellation when a peer stops reading.
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hostStreamTransfersBytesAndCloseWakesPendingRead() {
        let (mut master, slave) = SerialStream::pair().unwrap();
        // Darwin pseudo terminals do not implement the serial baud-rate ioctl.
        // Exercise the connection using the driver's preconfigured PTY stream.
        let port = NativeSerialConnection::from_stream(slave);
        master.write_all(b"host-read").await.unwrap();
        assert_eq!(port.read().await.unwrap().unwrap(), b"host-read");
        port.write(b"host-write").await.unwrap();
        let mut bytes = [0; 10];
        master.read_exact(&mut bytes).await.unwrap();
        assert_eq!(&bytes, b"host-write");
        let pending = port.read();
        let close = async {
            tokio::task::yield_now().await;
            port.close().await;
        };
        let (read, _) = tokio::join!(pending, close);
        assert_eq!(read.unwrap(), None);
        assert!(port.write(b"closed").await.is_err());
        port.close().await;
    }
}
