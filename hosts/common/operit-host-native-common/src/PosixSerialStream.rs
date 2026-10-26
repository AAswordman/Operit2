//! Drives a configured POSIX serial descriptor with Tokio readiness notifications.

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tokio::io::{unix::AsyncFd, AsyncRead, AsyncWrite, ReadBuf};

pub struct SerialStream(AsyncFd<File>);

impl SerialStream {
    /// Configures one raw 8N1 port without asserting modem control lines.
    pub fn open(path: &str, baud: u32) -> io::Result<Self> {
        let port = serial2::SerialPort::open(path, |mut settings: serial2::Settings| {
            settings.set_raw();
            settings.set_baud_rate(baud)?;
            settings.set_char_size(serial2::CharSize::Bits8);
            settings.set_stop_bits(serial2::StopBits::One);
            settings.set_parity(serial2::Parity::None);
            settings.set_flow_control(serial2::FlowControl::None);
            Ok(settings)
        })?;
        Self::fromPort(port)
    }

    /// Transfers descriptor ownership into nonblocking Tokio I/O exactly once.
    fn fromPort(port: serial2::SerialPort) -> io::Result<Self> {
        let descriptor: OwnedFd = port.into();
        let file = File::from(descriptor);
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(AsyncFd::new(file)?))
    }

    /// Creates real connected raw PTYs for the shared serial connection tests.
    #[cfg(test)]
    pub fn pair() -> io::Result<(Self, Self)> {
        let (first, second) = serial2::SerialPort::pair()?;
        Ok((Self::fromPort(first)?, Self::fromPort(second)?))
    }
}

impl AsyncRead for SerialStream {
    /// Reads available bytes and yields while the serial descriptor is idle.
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let mut ready = ready!(self.0.poll_read_ready(cx))?;
            match ready.try_io(|descriptor| descriptor.get_ref().read(buffer.initialize_unfilled()))
            {
                Ok(Ok(count)) => {
                    buffer.advance(count);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(error)) => return Poll::Ready(Err(error)),
                Err(_) => continue,
            }
        }
    }
}

impl AsyncWrite for SerialStream {
    /// Writes without blocking the executor when a serial peer stops reading.
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            let mut ready = ready!(self.0.poll_write_ready(cx))?;
            match ready.try_io(|descriptor| descriptor.get_ref().write(bytes)) {
                Ok(result) => return Poll::Ready(result),
                Err(_) => continue,
            }
        }
    }

    /// Completes immediately because writes go directly to the descriptor.
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    /// Lets the connection owner close its descriptor without a blocking drain.
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
