#![allow(non_snake_case)]

use super::LinkChannel;
use async_trait::async_trait;
use operit_host_api::{SerialPortConnection, SerialPortHost};
use operit_link::LinkFrame;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::serial_codec::{encodeSerialFrame, SerialFrameDecoder};

/// Length/checksum framed serial carrier for standard Operit Link frames.
///
/// The magic prefix makes it possible to recover after ordinary UART logs or
/// a partial frame have appeared on the same physical line. Production ESP32
/// firmware should still use a dedicated UART for this carrier when possible.
pub struct SerialLinkChannel {
    connection: Arc<dyn SerialPortConnection>,
    decoder: Mutex<SerialFrameDecoder>,
}

impl SerialLinkChannel {
    /// Opens a host serial port such as `COM27`.
    pub async fn open(
        host: &dyn SerialPortHost,
        port: &str,
        baudRate: u32,
    ) -> Result<Arc<Self>, String> {
        let connection = host
            .open(port, baudRate)
            .await
            .map_err(|error| format!("open Edge serial port {port}: {error}"))?;
        Ok(Self::fromConnection(connection))
    }

    pub fn fromConnection(connection: Arc<dyn SerialPortConnection>) -> Arc<Self> {
        Arc::new(Self {
            connection,
            decoder: Mutex::new(SerialFrameDecoder::new()),
        })
    }
}

#[async_trait]
impl LinkChannel for SerialLinkChannel {
    async fn send(&self, frame: LinkFrame) -> Result<(), String> {
        let encoded = encodeSerialFrame(&frame)?;
        self.connection
            .write(&encoded)
            .await
            .map_err(|error| error.to_string())
    }

    async fn receive(&self) -> Result<Option<LinkFrame>, String> {
        let mut decoder = self.decoder.lock().await;
        // One read can contain multiple frames. Drain any buffered frame first.
        if let Some(frame) = decoder.push(&[])? {
            return Ok(Some(frame));
        }
        loop {
            let Some(bytes) = self
                .connection
                .read()
                .await
                .map_err(|error| error.to_string())?
            else {
                return Ok(None);
            };
            if let Some(frame) = decoder.push(&bytes)? {
                return Ok(Some(frame));
            }
        }
    }

    async fn close(&self) {
        self.connection.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::super::serial_codec::crc32;
    use super::*;
    use operit_host_api::HostResult;
    use operit_link::LinkFramePayload;
    use tokio::sync::mpsc;

    struct TestPort {
        input: Mutex<mpsc::UnboundedReceiver<Vec<u8>>>,
        output: mpsc::UnboundedSender<Vec<u8>>,
    }

    #[async_trait]
    impl SerialPortConnection for TestPort {
        async fn read(&self) -> HostResult<Option<Vec<u8>>> {
            Ok(self.input.lock().await.recv().await)
        }
        async fn write(&self, bytes: &[u8]) -> HostResult<()> {
            self.output.send(bytes.to_vec()).unwrap();
            Ok(())
        }
        async fn close(&self) {}
    }

    #[tokio::test]
    async fn framesRoundTripOverHostBytesIncludingCoalescedFrames() {
        let (input, receiver) = mpsc::unbounded_channel();
        let (sender, mut output) = mpsc::unbounded_channel();
        let channel = SerialLinkChannel::fromConnection(Arc::new(TestPort {
            input: Mutex::new(receiver),
            output: sender,
        }));
        let frame = LinkFrame {
            messageId: "host-frame".into(),
            payload: LinkFramePayload::Heartbeat { sequence: 12 },
        };
        channel.send(frame.clone()).await.unwrap();
        assert_eq!(
            output.recv().await.unwrap(),
            encodeSerialFrame(&frame).unwrap()
        );
        let mut bytes = b"boot log\n".to_vec();
        bytes.extend(encodeSerialFrame(&frame).unwrap());
        bytes.extend(encodeSerialFrame(&frame).unwrap());
        input.send(bytes[..9].to_vec()).unwrap();
        input.send(bytes[9..].to_vec()).unwrap();
        drop(input);
        assert_eq!(channel.receive().await.unwrap(), Some(frame.clone()));
        assert_eq!(channel.receive().await.unwrap(), Some(frame));
        assert_eq!(channel.receive().await.unwrap(), None);
    }

    #[test]
    fn crcMatchesKnownVector() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }
}
