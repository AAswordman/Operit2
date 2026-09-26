#![allow(non_snake_case)]

use async_trait::async_trait;
use operit_edge_transport::LinkChannel;
use operit_link::{decodeLink, encodeLink, LinkFrame};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream as StdTcpStream};
use std::sync::Arc;
use tokio::sync::Mutex;

const MAX_LINK_FRAME_BYTES: usize = 1024 * 1024;

/// ESP-IDF's mio reactor is unavailable, so the Wi-Fi carrier uses native
/// nonblocking BSD sockets while the current-thread Tokio runtime drives Link.
pub struct Esp32TcpLinkChannel {
    stream: Arc<StdTcpStream>,
    readLock: Mutex<()>,
    writeLock: Mutex<()>,
}

impl Esp32TcpLinkChannel {
    /// Wraps a nonblocking ESP-IDF socket as a Link carrier.
    pub fn fromStream(stream: StdTcpStream) -> Result<Arc<Self>, String> {
        // ESP-IDF's socket wrapper does not implement TcpStream::try_clone().
        // Keep one socket and serialize reads/writes independently; Read/Write
        // are implemented for &TcpStream, so full-duplex operation remains safe.
        stream
            .set_nonblocking(true)
            .map_err(|error| format!("configure Edge TCP socket: {error}"))?;
        Ok(Arc::new(Self {
            stream: Arc::new(stream),
            readLock: Mutex::new(()),
            writeLock: Mutex::new(()),
        }))
    }
}
#[async_trait]
impl LinkChannel for Esp32TcpLinkChannel {
    /// Writes one length-prefixed Link frame.
    async fn send(&self, frame: LinkFrame) -> Result<(), String> {
        let payload = encodeLink(&frame).map_err(|error| error.to_string())?;
        if payload.is_empty() || payload.len() > MAX_LINK_FRAME_BYTES {
            return Err("invalid Edge Link frame size".to_string());
        }
        let mut bytes = Vec::with_capacity(4 + payload.len());
        bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&payload);

        let _writeLock = self.writeLock.lock().await;
        let mut offset = 0;
        while offset < bytes.len() {
            match (&*self.stream).write(&bytes[offset..]) {
                Ok(0) => return Err("Edge TCP peer closed while writing".to_string()),
                Ok(count) => offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(format!("Edge TCP write: {error}")),
            }
        }
        Ok(())
    }

    /// Reads one complete length-prefixed Link frame.
    async fn receive(&self) -> Result<Option<LinkFrame>, String> {
        let _readLock = self.readLock.lock().await;
        let mut header = [0u8; 4];
        let mut offset = 0;
        while offset < header.len() {
            match (&*self.stream).read(&mut header[offset..]) {
                Ok(0) if offset == 0 => return Ok(None),
                Ok(0) => return Err("Edge TCP peer closed during frame header".to_string()),
                Ok(count) => offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(format!("Edge TCP read: {error}")),
            }
        }
        let length = u32::from_be_bytes(header) as usize;
        if length == 0 || length > MAX_LINK_FRAME_BYTES {
            return Err(format!("invalid Edge Link frame length: {length}"));
        }
        let mut payload = vec![0u8; length];
        offset = 0;
        while offset < payload.len() {
            match (&*self.stream).read(&mut payload[offset..]) {
                Ok(0) => return Err("Edge TCP peer closed during frame payload".to_string()),
                Ok(count) => offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(format!("Edge TCP read: {error}")),
            }
        }
        decodeLink(&payload)
            .map(Some)
            .map_err(|error| format!("decode Edge Link frame: {error}"))
    }

    /// Shuts down both directions of the carrier.
    async fn close(&self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}
