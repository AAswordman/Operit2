//! Board UART byte I/O. No pairing, Link message, codec or application dependency.
use async_trait::async_trait;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostError, HostResult, SerialPortConnection, SerialPortHost};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex as StdMutex, Weak,
};
use tokio::sync::Mutex;
use crate::serial_debug::SerialDebugIo;

// Nonblocking operations keep the async executor responsive. This small driver
// boundary also lets native tests exercise exactly the board connection lifecycle.
trait UartIo: Send + Sync {
    fn read(&self, bytes: &mut [u8]) -> HostResult<usize>;
    fn write(&self, bytes: &[u8]) -> HostResult<usize>;
    fn configure(&self, baud: u32) -> HostResult<()>;
    fn tx_idle(&self) -> HostResult<bool> { Ok(true) }
}

/// Owns the UART driver independently of any protocol or admitted session.
/// One open connection leases the port; close/drop permits a fresh connection.
pub struct Esp32SerialPortHost {
    port: String,
    uart: Arc<dyn UartIo>,
    active: StdMutex<Weak<UartConnection>>,
    debug: Arc<SerialDebugIo>,
}
impl Esp32SerialPortHost {
    /// Local USB debug mailbox, serviced by the existing Link UART reader.
    pub fn debug_io(&self) -> Arc<SerialDebugIo> { self.debug.clone() }
    #[cfg(target_os = "espidf")]
    pub fn new(port: impl Into<String>, uart: esp_idf_hal::uart::UartDriver<'static>) -> Self {
        Self {
            port: port.into(),
            uart: Arc::new(uart),
            active: StdMutex::new(Weak::new()),
            debug: Arc::new(SerialDebugIo::default()),
        }
    }
}

#[cfg(target_os = "espidf")]
impl UartIo for esp_idf_hal::uart::UartDriver<'static> {
    fn read(&self, bytes: &mut [u8]) -> HostResult<usize> {
        if self
            .remaining_read()
            .map_err(|error| HostError::new(error.to_string()))?
            == 0
        {
            return Ok(0);
        }
        self.read(bytes, 0)
            .map_err(|error| HostError::new(error.to_string()))
    }
    fn write(&self, bytes: &[u8]) -> HostResult<usize> {
        self.write_nb(bytes)
            .map_err(|error| HostError::new(error.to_string()))
    }
    fn tx_idle(&self) -> HostResult<bool> {
        match self.wait_tx_done(0) {
            Ok(()) => Ok(true),
            Err(error) if error.code() == esp_idf_hal::sys::ESP_ERR_TIMEOUT => Ok(false),
            Err(error) => Err(HostError::new(error.to_string())),
        }
    }
    fn configure(&self, baud: u32) -> HostResult<()> {
        // Reprogramming UART0 after enqueueing a final response can truncate TX.
        if self.baudrate().map_err(|e| HostError::new(e.to_string()))?.0 == baud { return Ok(()); }
        self.change_baudrate(baud)
            .map(|_| ())
            .map_err(|error| HostError::new(error.to_string()))
    }
}

#[async_trait]
impl SerialPortHost for Esp32SerialPortHost {
    fn listenerPort(&self) -> Option<String> { Some(self.port.clone()) }
    async fn open(&self, port: &str, baud_rate: u32) -> HostResult<Arc<dyn SerialPortConnection>> {
        if port != self.port || baud_rate == 0 {
            return Err(HostError::new("Invalid board UART port or baud rate"));
        }
        let mut active = self
            .active
            .lock()
            .map_err(|error| HostError::new(error.to_string()))?;
        if active
            .upgrade()
            .is_some_and(|connection| !connection.closed.load(Ordering::Acquire))
        {
            return Err(HostError::new("Board UART port is already open"));
        }
        self.uart.configure(baud_rate)?;
        let connection = Arc::new(UartConnection {
            uart: self.uart.clone(),
            debug: self.debug.clone(),
            reader: Mutex::new(()),
            writer: Mutex::new(()),
            io: StdMutex::new(()),
            closed: AtomicBool::new(false),
        });
        *active = Arc::downgrade(&connection);
        Ok(connection)
    }
}

struct UartConnection {
    uart: Arc<dyn UartIo>,
    debug: Arc<SerialDebugIo>,
    reader: Mutex<()>,
    writer: Mutex<()>,
    // close serializes against short nonblocking driver calls. An old connection
    // can never touch the UART after a new connection has acquired the lease.
    io: StdMutex<()>,
    closed: AtomicBool,
}
#[async_trait]
impl SerialPortConnection for UartConnection {
    async fn read(&self) -> HostResult<Option<Vec<u8>>> {
        let _reader = self.reader.lock().await;
        // Keep the long-lived UART read buffer small: the ESP32 shares this
        // UART with the console and has only a small heap after Wi-Fi starts.
        // Framing already handles split messages, so 512 bytes is sufficient.
        let mut bytes = vec![0; 512];
        loop {
            if let Some(response) = self.debug.take_response() {
                // Shares the same write lock as Link; frames cannot interleave.
                self.write(&response).await?;
            }
            let count = {
                let _io = self
                    .io
                    .lock()
                    .map_err(|error| HostError::new(error.to_string()))?;
                if self.closed.load(Ordering::Acquire) {
                    return Ok(None);
                }
                self.uart.read(&mut bytes)?
            };
            if count > 0 {
                self.debug.observe(&bytes[..count]);
                bytes.truncate(count);
                return Ok(Some(bytes));
            }
            defaultHostRuntimeTaskSchedulerHost()
                .waitForHostRuntimeDelay(2)
                .await?;
        }
    }
    async fn write(&self, bytes: &[u8]) -> HostResult<()> {
        let _writer = self.writer.lock().await;
        let mut offset = 0;
        loop {
            let count = {
                let _io = self
                    .io
                    .lock()
                    .map_err(|error| HostError::new(error.to_string()))?;
                if self.closed.load(Ordering::Acquire) {
                    return Err(HostError::new("Board UART connection is closed"));
                }
                if offset == bytes.len() {
                    return Ok(());
                }
                self.uart.write(&bytes[offset..])?
            };
            offset += count;
            if count == 0 {
                defaultHostRuntimeTaskSchedulerHost()
                    .waitForHostRuntimeDelay(2)
                    .await?;
            }
        }
    }
    async fn close(&self) {
        let _writer = self.writer.lock().await;
        // write_nb means accepted by FIFO, not delivered to USB. Keep the lease
        // until the FINAL acknowledgement has actually left the UART wire.
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(250);
        loop {
            let ready = {
                let _io = self.io.lock().unwrap_or_else(|e| e.into_inner());
                if self.closed.load(Ordering::Acquire) { return; }
                self.uart.tx_idle().unwrap_or(true)
            };
            if ready || std::time::Instant::now() >= deadline { break; }
            if defaultHostRuntimeTaskSchedulerHost().waitForHostRuntimeDelay(2).await.is_err() { break; }
        }
        let _io = self.io.lock().unwrap_or_else(|error| error.into_inner());
        self.closed.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::future::{poll_fn, Future};
    use std::task::Poll;

    #[derive(Default)]
    struct Uart {
        input: StdMutex<VecDeque<u8>>,
        output: StdMutex<Vec<u8>>,
        tx_pending: std::sync::atomic::AtomicUsize,
    }
    impl UartIo for Uart {
        fn read(&self, bytes: &mut [u8]) -> HostResult<usize> {
            let mut input = self.input.lock().unwrap();
            let count = bytes.len().min(input.len());
            for byte in &mut bytes[..count] {
                *byte = input.pop_front().unwrap();
            }
            Ok(count)
        }
        fn write(&self, bytes: &[u8]) -> HostResult<usize> {
            let count = bytes.len().min(3);
            self.output
                .lock()
                .unwrap()
                .extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn configure(&self, _: u32) -> HostResult<()> {
            Ok(())
        }
        fn tx_idle(&self) -> HostResult<bool> {
            Ok(self.tx_pending.fetch_update(Ordering::AcqRel, Ordering::Acquire,
                |n| n.checked_sub(1)).is_err())
        }
    }
    fn host() -> (Esp32SerialPortHost, Arc<Uart>) {
        operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
            operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost,
        ));
        let uart = Arc::new(Uart::default());
        (
            Esp32SerialPortHost {
                port: "uart0".into(),
                uart: uart.clone(),
                active: StdMutex::new(Weak::new()),
            debug: Arc::new(SerialDebugIo::default()),
            },
            uart,
        )
    }
    #[tokio::test]
    async fn host_transfers_opaque_bytes_and_releases_port() {
        let (host, uart) = host();
        assert!(host.open("missing", 115200).await.is_err());
        assert!(host.open("uart0", 0).await.is_err());
        let connection = host.open("uart0", 115200).await.unwrap();
        assert!(host.open("uart0", 115200).await.is_err());
        connection.write(b"not a Link frame").await.unwrap();
        assert_eq!(*uart.output.lock().unwrap(), b"not a Link frame");
        uart.input.lock().unwrap().extend(b"raw input");
        assert_eq!(connection.read().await.unwrap().unwrap(), b"raw input");
        connection.close().await;
        connection.close().await;
        assert!(connection.read().await.unwrap().is_none());
        assert!(connection.write(b"stale").await.is_err());
        let replacement = host.open("uart0", 115200).await.unwrap();
        connection.close().await;
        replacement.write(b"replacement").await.unwrap();
        drop(replacement);
        assert!(host.open("uart0", 115200).await.is_ok());
    }
    #[tokio::test]
    async fn debug_uses_the_existing_reader_and_preserves_raw_link_bytes() {
        let (host, uart) = host();
        let debug = host.debug_io();
        let connection = host.open("uart0", 115200).await.unwrap();
        // Captured OPD1 snapshot request: id 1, opcode 1, CRC32(payload).
        let request = [79, 80, 68, 49, 0, 0, 0, 5, 0, 0, 0, 1, 1, 168, 62, 246, 202];
        uart.input.lock().unwrap().extend(request);
        assert_eq!(connection.read().await.unwrap().unwrap(), request);
        let request = debug.take_request().expect("existing reader observes debug request");
        assert_eq!(request.id(), 1);
        debug.respond(request.id(), b"{\"page\":\"pairing\"}");
        // Reading resumes and writes the UI-thread response via the same UART.
        uart.input.lock().unwrap().extend(b"raw link bytes");
        assert_eq!(connection.read().await.unwrap().unwrap(), b"raw link bytes");
        assert!(uart.output.lock().unwrap().starts_with(b"OPD2"));
        assert!(host.open("uart0", 115200).await.is_err());
    }

    #[tokio::test]
    async fn close_retains_lease_until_final_acknowledgement_leaves_fifo() {
        let (host, uart) = host();
        let connection = host.open("uart0", 115200).await.unwrap();
        connection.write(b"final acknowledgement").await.unwrap();
        uart.tx_pending.store(2, Ordering::Release);
        let mut closing = Box::pin(connection.close());
        poll_fn(|cx| {
            assert!(closing.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        }).await;
        assert!(host.open("uart0", 115200).await.is_err());
        closing.await;
        assert_eq!(uart.tx_pending.load(Ordering::Acquire), 0);
        assert!(host.open("uart0", 115200).await.is_ok());
    }

    #[tokio::test]
    async fn cancelled_read_keeps_bytes_in_driver() {
        let (host, uart) = host();
        let connection = host.open("uart0", 115200).await.unwrap();
        let mut pending = Box::pin(connection.read());
        poll_fn(|cx| {
            assert!(pending.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        drop(pending);
        uart.input.lock().unwrap().extend(b"kept");
        assert_eq!(connection.read().await.unwrap().unwrap(), b"kept");
    }
}
