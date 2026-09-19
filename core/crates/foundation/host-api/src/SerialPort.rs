use crate::HostResult;
use async_trait::async_trait;
use std::sync::Arc;

/// An ordered, full-duplex byte stream. Framing and pairing belong to Core.
#[async_trait]
pub trait SerialPortConnection: Send + Sync {
    /// Sends all bytes in order, or reports a transport error.
    async fn write(&self, bytes: &[u8]) -> HostResult<()>;
    /// Returns up to 4096 bytes; None indicates a closed connection.
    /// Cancelling this future must not lose bytes already read from the port.
    async fn read(&self) -> HostResult<Option<Vec<u8>>>;
    /// Releases the port and wakes pending reads and writes. Must be idempotent.
    async fn close(&self);
}

/// Opens a platform serial device or accessory. Port identifiers are host-owned.
#[async_trait]
pub trait SerialPortHost: Send + Sync {
    async fn open(&self, port: &str, baud_rate: u32) -> HostResult<Arc<dyn SerialPortConnection>>;
}
