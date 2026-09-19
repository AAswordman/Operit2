use async_trait::async_trait;
use operit_host_api::{HostError, HostResult, SerialPortConnection, SerialPortHost};
use std::sync::Arc;

/// iOS owns accessory admission. A native accessory provider can implement the
/// same SerialPortHost contract without changing Core's serial Link protocol.
pub struct IosSerialPortHost;

#[async_trait]
impl SerialPortHost for IosSerialPortHost {
    async fn open(&self, port: &str, _baud_rate: u32) -> HostResult<Arc<dyn SerialPortConnection>> {
        Err(HostError::new(format!(
            "The iOS Host has no serial accessory provider for {port}; a native accessory transport is required"
        )))
    }
}
