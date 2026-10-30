//! Adapts the Host byte-stream contract to Hyper without exposing native sockets.
use operit_host_api::{HostResult, TcpConnection};
use std::{
    future::Future,
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

type Operation<T> = Pin<Box<dyn Future<Output = HostResult<T>> + Send>>;
pub(super) struct HostStream {
    connection: Arc<dyn TcpConnection>,
    read: Option<Operation<Option<Vec<u8>>>>,
    pending: Vec<u8>,
    offset: usize,
    write: Option<Operation<()>>,
    shutdown: Option<Operation<()>>,
}
impl HostStream {
    pub(super) fn new(connection: Arc<dyn TcpConnection>) -> Self {
        Self {
            connection,
            read: None,
            pending: Vec::new(),
            offset: 0,
            write: None,
            shutdown: None,
        }
    }
}
fn error(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}
impl AsyncRead for HostStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        loop {
            if self.offset < self.pending.len() {
                let count = output.remaining().min(self.pending.len() - self.offset);
                output.put_slice(&self.pending[self.offset..self.offset + count]);
                self.offset += count;
                return Poll::Ready(Ok(()));
            }
            if self.read.is_none() {
                let connection = self.connection.clone();
                self.read = Some(Box::pin(async move { connection.read().await }));
            }
            let result = std::task::ready!(self.read.as_mut().unwrap().as_mut().poll(cx));
            self.read = None;
            match result {
                Ok(Some(bytes)) if bytes.is_empty() => {
                    return Poll::Ready(Err(error("Host returned an empty TCP chunk")))
                }
                Ok(Some(bytes)) => {
                    self.pending = bytes;
                    self.offset = 0;
                }
                Ok(None) => return Poll::Ready(Ok(())),
                Err(e) => return Poll::Ready(Err(error(e))),
            }
        }
    }
}
impl AsyncWrite for HostStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        std::task::ready!(self.as_mut().poll_flush(cx))?;
        let connection = self.connection.clone();
        let count = bytes.len();
        if count > 0 {
            let bytes = bytes.to_vec();
            self.write = Some(Box::pin(async move { connection.write(&bytes).await }));
        }
        // Accept the owned buffer immediately; later writes/flush drive Host I/O.
        // A Pending write therefore never acknowledges a different caller buffer.
        Poll::Ready(Ok(count))
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let Some(write) = self.write.as_mut() else {
            return Poll::Ready(Ok(()));
        };
        let result = std::task::ready!(write.as_mut().poll(cx));
        self.write = None;
        Poll::Ready(result.map_err(error))
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        std::task::ready!(self.as_mut().poll_flush(cx))?;
        if self.shutdown.is_none() {
            let connection = self.connection.clone();
            self.shutdown = Some(Box::pin(async move {
                connection.close().await;
                Ok(())
            }));
        }
        self.shutdown
            .as_mut()
            .unwrap()
            .as_mut()
            .poll(cx)
            .map(|r| r.map_err(error))
    }
}
