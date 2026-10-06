use async_trait::async_trait;
use operit_host_api::{HostError, HostResult, SerialPortConnection, SerialPortHost};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
#[cfg(windows)]
use tokio::net::windows::named_pipe::NamedPipeClient as SerialStream;
use tokio::sync::{watch, Mutex};
#[cfg(not(windows))]
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
        // Do not assert DTR on open: on ESP32 USB-UART boards the modem
        // control lines are wired to EN/BOOT. Opening a pairing connection
        // must not intentionally reset its receiving device.
        let stream = open_stream(port, baud_rate).await?;
        Ok(Arc::new(NativeSerialConnection::from_stream(stream)))
    }
}

/// Windows IOCP/USB drivers can briefly retain an exclusive handle after close.
/// Retry only that transient open error, bounded; never reset or evict another app.
async fn open_stream(port: &str, baud_rate: u32) -> HostResult<SerialStream> {
    let mut attempt = 0;
    loop {
        match try_open_stream(port, baud_rate) {
            Ok(stream) => return Ok(stream),
            Err(error) => {
                if cfg!(windows)
                    && attempt < 4
                    && error.kind() == std::io::ErrorKind::PermissionDenied
                {
                    attempt += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    continue;
                }
                return Err(HostError::new(format!("Open serial port {port}: {error}")));
            }
        }
    }
}

#[cfg(not(windows))]
fn try_open_stream(port: &str, baud_rate: u32) -> std::io::Result<SerialStream> {
    tokio_serial::new(port, baud_rate)
        .dtr_on_open(false)
        .open_native_async()
        .map_err(std::io::Error::from)
}

#[cfg(windows)]
fn try_open_stream(port: &str, baud_rate: u32) -> std::io::Result<SerialStream> {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::{AsRawHandle, IntoRawHandle};
    use windows_sys::Win32::Devices::Communication::{
        GetCommState, SetCommState, SetCommTimeouts, COMMTIMEOUTS, DCB, NOPARITY, ONESTOPBIT,
    };
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OVERLAPPED;

    // mio-serial's Windows adapter opens synchronously, closes, then reopens
    // overlapped. Some USB drivers deny that immediate exclusive reopen.
    // Use ONE owned overlapped handle, the same IOCP primitive Tokio uses.
    let path = if port.starts_with(r"\\.\") {
        port.to_owned()
    } else {
        format!(r"\\.\{port}")
    };
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OVERLAPPED)
        .open(path)?;
    let handle = file.as_raw_handle();
    let mut dcb: DCB = unsafe { std::mem::zeroed() };
    dcb.DCBlength = std::mem::size_of::<DCB>() as u32;
    if unsafe { GetCommState(handle, &mut dcb) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    dcb.BaudRate = baud_rate;
    dcb.ByteSize = 8;
    dcb.Parity = NOPARITY;
    dcb.StopBits = ONESTOPBIT;
    // Binary 8N1, no software/hardware flow control, DTR/RTS deasserted.
    dcb._bitfield = 1;
    dcb.XonChar = 17;
    dcb.XoffChar = 19;
    if unsafe { SetCommState(handle, &dcb) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let timeouts = COMMTIMEOUTS {
        ReadIntervalTimeout: 1,
        ReadTotalTimeoutMultiplier: 0,
        ReadTotalTimeoutConstant: 0,
        WriteTotalTimeoutMultiplier: 0,
        WriteTotalTimeoutConstant: 0,
    };
    if unsafe { SetCommTimeouts(handle, &timeouts) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // Transfer ownership exactly once; Tokio also closes it if registration fails.
    unsafe { SerialStream::from_raw_handle(file.into_raw_handle()) }
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
                readUartChunk(reader).await.map(Some)
            } => result,
        }
    }

    async fn close(&self) {
        self.closed.send_replace(true);
        // Closing wakes operations before acquiring locks, including blocked reads.
        self.reader.lock().await.take();
        self.writer.lock().await.take();
        // Let Windows IOCP process deregistration before an immediate reconnect.
        #[cfg(windows)]
        tokio::task::yield_now().await;
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

#[cfg(all(test, windows))]
mod hardware_tests {
    use super::*;
    static HARDWARE_LEASE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    #[tokio::test]
    #[ignore = "requires OPERIT_SERIAL_TEST_PORT and firmware with USB screen debug"]
    async fn windows_serial_reads_actual_ui() {
        let _lease = HARDWARE_LEASE.lock().unwrap();
        let port = std::env::var("OPERIT_SERIAL_TEST_PORT").expect("explicit test port");
        let connection = NativeSerialPortHost.open(&port, 115200).await.unwrap();
        // OPD1: request id 1, snapshot opcode 1, CRC32.
        connection
            .write(&[79, 80, 68, 49, 0, 0, 0, 5, 0, 0, 0, 1, 1, 168, 62, 246, 202])
            .await
            .unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let mut bytes = Vec::new();
            loop {
                let part = connection
                    .read()
                    .await
                    .unwrap()
                    .expect("USB port stays open");
                bytes.extend(part);
                assert!(bytes.len() < 32 * 1024, "bounded receive");
                if let Some(at) = bytes.windows(4).position(|p| p == b"OPD2") {
                    if bytes.len() >= at + 8 {
                        let n =
                            u32::from_be_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
                        if bytes.len() >= at + n + 12 {
                            return String::from_utf8(bytes[at + 12..at + 8 + n].to_vec()).unwrap();
                        }
                    }
                }
            }
        })
        .await;
        connection.close().await;
        let screen = result.expect("receive actual hardware snapshot");
        assert!(screen.contains("\"nodes\""));
        assert!(screen.contains("\"page\""));
    }

    /// Opt-in real hardware regression: never chooses a COM port implicitly.
    #[tokio::test]
    #[ignore = "requires OPERIT_SERIAL_TEST_PORT and an unoccupied USB serial device"]
    async fn windows_serial_open_and_release() {
        let _lease = HARDWARE_LEASE.lock().unwrap();
        let port = std::env::var("OPERIT_SERIAL_TEST_PORT").expect("explicit test port");
        let host = NativeSerialPortHost;
        let connection = host
            .open(&port, 115200)
            .await
            .expect("open actual serial device");
        connection.close().await;
        drop(connection);
        for _ in 0..5 {
            let replacement = host
                .open(&port, 115200)
                .await
                .expect("reopen released device");
            replacement.close().await;
        }
    }
}

/// UART idle completion is not stream EOF (notably with Windows COMMTIMEOUTS).
async fn readUartChunk<R: tokio::io::AsyncRead + Unpin>(reader: &mut R) -> HostResult<Vec<u8>> {
    let mut bytes = vec![0; 4096];
    loop {
        let count = reader.read(&mut bytes).await.map_err(|e| HostError::new(e.to_string()))?;
        if count > 0 { bytes.truncate(count); return Ok(bytes); }
        // Avoid spinning; the outer explicit-close select remains cancellable.
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
}
#[cfg(test)]
mod idle_completion_tests {
    use super::*;
    use std::{pin::Pin, task::{Context,Poll}};
    struct ZeroThenData(usize);
    impl tokio::io::AsyncRead for ZeroThenData {
        fn poll_read(mut self: Pin<&mut Self>, _: &mut Context<'_>, buf: &mut tokio::io::ReadBuf<'_>) -> Poll<std::io::Result<()>> {
            self.0 += 1;
            if self.0 > 2 { buf.put_slice(b"frame"); }
            Poll::Ready(Ok(()))
        }
    }
    #[tokio::test]
    async fn idle_zero_byte_completions_do_not_end_the_uart_session() {
        let bytes = tokio::time::timeout(std::time::Duration::from_secs(1), readUartChunk(&mut ZeroThenData(0))).await.unwrap().unwrap();
        assert_eq!(bytes,b"frame");
    }
    #[tokio::test]
    async fn repeated_idle_completions_remain_cancellable() {
        let mut empty = tokio::io::empty();
        assert!(tokio::time::timeout(std::time::Duration::from_millis(10), readUartChunk(&mut empty)).await.is_err());
    }
}
