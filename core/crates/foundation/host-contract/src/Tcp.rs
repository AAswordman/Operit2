use crate::HostResult;
use async_trait::async_trait;
use std::sync::Arc;

/// Host-owned ordered byte stream. Link framing and authentication stay in Core.
#[async_trait]
pub trait TcpConnection: Send + Sync {
    /// 实际 socket 来源，仅由 Host 提供；缺失时不得推断局域网免 token。
    fn remote_address(&self) -> Option<std::net::SocketAddr> { None }
    async fn write(&self, bytes: &[u8]) -> HostResult<()>;
    /// Returns at most 4096 bytes, or None at EOF. Cancelling a read loses no bytes.
    async fn read(&self) -> HostResult<Option<Vec<u8>>>;
    /// Idempotently releases the socket and wakes pending I/O.
    async fn close(&self);
}

/// Host-owned listener; accepted connections use the same byte-stream contract.
#[async_trait]
pub trait TcpListener: Send + Sync {
    fn local_address(&self) -> HostResult<String>;
    async fn accept(&self) -> HostResult<Arc<dyn TcpConnection>>;
    async fn close(&self);
}

#[async_trait]
pub trait TcpHost: Send + Sync {
    async fn connect(&self, address: &str) -> HostResult<Arc<dyn TcpConnection>>;
    async fn bind(&self, address: &str) -> HostResult<Arc<dyn TcpListener>>;
}
