use operit_host_api::DeviceDigitalOutputState;
use operit_proxy_edge::{EdgeDeviceIoClient, EdgeProxyError};

/// Owns the small device behavior running on top of the Edge Proxy contract.
pub struct Esp32App<C> {
    edgeClient: C,
    outputPin: u8,
    level: bool,
}

impl<C> Esp32App<C> {
    /// Creates the ESP32 sample app with a low initial output level.
    pub fn new(edgeClient: C, outputPin: u8) -> Self {
        Self {
            edgeClient,
            outputPin,
            level: false,
        }
    }
}

impl<C> Esp32App<C>
where
    C: EdgeDeviceIoClient,
{
    /// Advances the sample app by toggling and committing its output level.
    pub async fn tick(&mut self) -> Result<DeviceDigitalOutputState, EdgeProxyError> {
        let nextLevel = !self.level;
        let state = self
            .edgeClient
            .setDigitalOutput(self.outputPin, nextLevel)
            .await?;
        self.level = state.level;
        Ok(state)
    }
}
