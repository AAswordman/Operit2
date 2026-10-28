#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use esp_idf_hal::gpio::{Output, OutputPin, PinDriver};
use operit_host_api::{
    DeviceDigitalOutputRequest, DeviceDigitalOutputState, DeviceIoHost, HostError, HostResult,
};

/// One configured digital output on the ESP32-2432S028.
struct DigitalOutput {
    driver: PinDriver<'static, Output>,
    activeLow: bool,
    level: AtomicBool,
}

/// Owns the ESP32-2432S028 status LED outputs and exposes them through Host API.
pub struct Esp32GpioHost {
    outputs: Mutex<BTreeMap<u8, DigitalOutput>>,
}

impl Esp32GpioHost {
    /// Creates an empty GPIO host. Call `addOutput` before Edge services start.
    pub fn empty() -> Self {
        Self {
            outputs: Mutex::new(BTreeMap::new()),
        }
    }

    /// Creates a GPIO host around one output-capable ESP-IDF pin.
    pub fn new<T>(pinNumber: u8, pin: T) -> HostResult<Self>
    where
        T: OutputPin + 'static,
    {
        let host = Self::empty();
        host.addOutput(pinNumber, pin, false)?;
        Ok(host)
    }

    /// Adds one output pin. `activeLow` is true for the onboard RGB status LEDs.
    pub fn addOutput<T>(&self, pinNumber: u8, pin: T, activeLow: bool) -> HostResult<()>
    where
        T: OutputPin + 'static,
    {
        let mut driver =
            PinDriver::output(pin).map_err(|error| HostError::new(error.to_string()))?;
        if activeLow {
            driver
                .set_high()
                .map_err(|error| HostError::new(error.to_string()))?;
        } else {
            driver
                .set_low()
                .map_err(|error| HostError::new(error.to_string()))?;
        }
        let mut outputs = self
            .outputs
            .lock()
            .map_err(|error| HostError::new(format!("ESP32 GPIO lock poisoned: {error}")))?;
        outputs.insert(
            pinNumber,
            DigitalOutput {
                driver,
                activeLow,
                level: AtomicBool::new(false),
            },
        );
        Ok(())
    }
}

impl DeviceIoHost for Esp32GpioHost {
    /// Writes one configured ESP32 GPIO and records its logical level.
    fn setDigitalOutput(
        &self,
        request: DeviceDigitalOutputRequest,
    ) -> HostResult<DeviceDigitalOutputState> {
        let mut outputs = self
            .outputs
            .lock()
            .map_err(|error| HostError::new(format!("ESP32 GPIO lock poisoned: {error}")))?;
        let output = outputs.get_mut(&request.pin).ok_or_else(|| {
            HostError::new(format!("ESP32 GPIO {} is not configured", request.pin))
        })?;
        let physicalHigh = if output.activeLow {
            !request.level
        } else {
            request.level
        };
        if physicalHigh {
            output
                .driver
                .set_high()
                .map_err(|error| HostError::new(error.to_string()))?;
        } else {
            output
                .driver
                .set_low()
                .map_err(|error| HostError::new(error.to_string()))?;
        }
        output.level.store(request.level, Ordering::Release);
        Ok(DeviceDigitalOutputState {
            pin: request.pin,
            level: request.level,
        })
    }

    /// Reads the last committed logical level of one configured ESP32 GPIO.
    fn getDigitalOutput(&self, pin: u8) -> HostResult<DeviceDigitalOutputState> {
        let outputs = self
            .outputs
            .lock()
            .map_err(|error| HostError::new(format!("ESP32 GPIO lock poisoned: {error}")))?;
        let output = outputs
            .get(&pin)
            .ok_or_else(|| HostError::new(format!("ESP32 GPIO {pin} is not configured")))?;
        Ok(DeviceDigitalOutputState {
            pin,
            level: output.level.load(Ordering::Acquire),
        })
    }
}
