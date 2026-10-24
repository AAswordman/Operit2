//! Defines the platform-independent byte stream used by host daemon transports.

use tokio::io::{AsyncRead, AsyncWrite};

/// Provides the same byte-stream contract for every local daemon transport.
pub trait ControlIo: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> ControlIo for T {}

/// Carries one local daemon request and response independently of its OS transport.
pub type ControlStream = Box<dyn ControlIo>;
