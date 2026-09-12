#![allow(non_snake_case)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use esp_idf_hal::gpio::{Output, OutputPin, PinDriver};
use operit_host_api::{
    DeviceDigitalOutputRequest, DeviceDigitalOutputState, DeviceIoHost, HostError, HostManager,
    HostResult,
};

/// Owns one ESP-IDF digital output and exposes it through the Host API.
pub struct Esp32GpioHost {
    pinNumber: u8,
    output: Mutex<PinDriver<'static, Output>>,
    level: AtomicBool,
}

impl Esp32GpioHost {
    /// Creates a GPIO host around one output-capable ESP-IDF pin.
    pub fn new<T>(pinNumber: u8, pin: T) -> HostResult<Self>
    where
        T: OutputPin + 'static,
    {
        let output = PinDriver::output(pin).map_err(|error| HostError::new(error.to_string()))?;
        Ok(Self {
            pinNumber,
            output: Mutex::new(output),
            level: AtomicBool::new(false),
        })
    }
}

impl DeviceIoHost for Esp32GpioHost {
    /// Writes the configured ESP32 GPIO and records its committed level.
    fn setDigitalOutput(
        &self,
        request: DeviceDigitalOutputRequest,
    ) -> HostResult<DeviceDigitalOutputState> {
        if request.pin != self.pinNumber {
            return Err(HostError::new(format!(
                "ESP32 GPIO {} is not configured",
                request.pin
            )));
        }
        let mut output = self
            .output
            .lock()
            .map_err(|error| HostError::new(format!("ESP32 GPIO lock poisoned: {error}")))?;
        if request.level {
            output
                .set_high()
                .map_err(|error| HostError::new(error.to_string()))?;
        } else {
            output
                .set_low()
                .map_err(|error| HostError::new(error.to_string()))?;
        }
        self.level.store(request.level, Ordering::Release);
        Ok(DeviceDigitalOutputState {
            pin: self.pinNumber,
            level: request.level,
        })
    }

    /// Reads the last committed level of the configured ESP32 GPIO.
    fn getDigitalOutput(&self, pin: u8) -> HostResult<DeviceDigitalOutputState> {
        if pin != self.pinNumber {
            return Err(HostError::new(format!(
                "ESP32 GPIO {} is not configured",
                pin
            )));
        }
        Ok(DeviceDigitalOutputState {
            pin: self.pinNumber,
            level: self.level.load(Ordering::Acquire),
        })
    }
}

/// Creates the HostManager used by an ESP32 Edge Core app.
pub fn createRuntimeHostManager(deviceIoHost: Arc<dyn DeviceIoHost>) -> HostManager {
    HostManager::new()
        .withDeviceIoHost(deviceIoHost)
        .withHostEnvironment(operit_host_api::HostEnvironmentDescriptor::esp32())
}
