#![allow(non_snake_case)]

use std::sync::Arc;

use esp_idf_hal::gpio::{
    Gpio12, Gpio13, Gpio14, Gpio15, Gpio16, Gpio17, Gpio2, Gpio21, Gpio25, Gpio32, Gpio33, Gpio36,
    Gpio39, Gpio4,
};
use esp_idf_hal::spi::{SPI2, SPI3};
use operit_host_api::HostManager::HostManager;
use operit_host_api::{DeviceIoHost, HostResult, RobotFaceHost};

use crate::display::{Esp32Ili9341, Esp32ScreenMirror};
use crate::face::FaceRect;
use crate::gpio::Esp32GpioHost;
use crate::pins::{LED_BLUE_PIN, LED_GREEN_PIN, LED_RED_PIN};
use crate::robot_face::Esp32RobotFaceHost;
use crate::touch::{Esp32Touch, TouchPoint};
use crate::{createRuntimeHostManager, esp32_2432s028HostEnvironment};

/// Owns ESP32-2432S028 Host implementations constructed from board peripherals.
pub struct Esp32Board {
    gpioHost: Arc<Esp32GpioHost>,
    robotFaceHost: Arc<Esp32RobotFaceHost<Esp32Ili9341>>,
    screenMirror: Arc<Esp32ScreenMirror>,
    touch: Esp32Touch,
}

impl Esp32Board {
    /// Takes the ESP32-2432S028 display, touch, and status-LED pins out of ESP-IDF peripherals.
    pub fn new(
        spi2: SPI2<'static>,
        tftDc: Gpio2<'static>,
        tftMiso: Gpio12<'static>,
        tftMosi: Gpio13<'static>,
        tftSclk: Gpio14<'static>,
        tftCs: Gpio15<'static>,
        tftBacklight: Gpio21<'static>,
        ledRed: Gpio4<'static>,
        ledGreen: Gpio16<'static>,
        ledBlue: Gpio17<'static>,
        spi3: SPI3<'static>,
        touchSclk: Gpio25<'static>,
        touchMosi: Gpio32<'static>,
        touchMiso: Gpio39<'static>,
        touchCs: Gpio33<'static>,
        touchIrq: Gpio36<'static>,
    ) -> HostResult<Self> {
        log::info!("esp32 board: configuring status LEDs");
        let gpioHost = Arc::new(Esp32GpioHost::empty());
        gpioHost.addOutput(LED_RED_PIN, ledRed, true)?;
        gpioHost.addOutput(LED_GREEN_PIN, ledGreen, true)?;
        gpioHost.addOutput(LED_BLUE_PIN, ledBlue, true)?;
        log::info!("esp32 board: status LEDs ready");
        log::info!("esp32 board: initializing TFT SPI");
        let display =
            Esp32Ili9341::new(spi2, tftSclk, tftMosi, tftMiso, tftCs, tftDc, tftBacklight)?;
        log::info!("esp32 board: TFT initialized");
        let screenMirror = display.screenMirror();
        log::info!("esp32 board: creating face host");
        let robotFaceHost = Arc::new(Esp32RobotFaceHost::new(display)?);
        log::info!("esp32 board: face host ready");
        log::info!("esp32 board: initializing touch SPI");
        let touch = Esp32Touch::new(spi3, touchSclk, touchMosi, touchMiso, touchCs, touchIrq)?;
        log::info!("esp32 board: touch ready");
        Ok(Self {
            gpioHost,
            robotFaceHost,
            screenMirror,
            touch,
        })
    }

    /// Returns the board-owned framebuffer mirror used by the Wi-Fi preview.
    pub fn screenMirror(&self) -> Arc<Esp32ScreenMirror> {
        Arc::clone(&self.screenMirror)
    }

    /// Returns the board-owned digital I/O host.
    pub fn gpioHost(&self) -> Arc<dyn DeviceIoHost> {
        self.gpioHost.clone()
    }

    /// Returns the board-owned robot face host.
    pub fn robotFaceHost(&self) -> Arc<dyn RobotFaceHost> {
        self.robotFaceHost.clone()
    }

    /// Paints the empty plugin shelf on the board display.
    pub fn paintPluginShelf(&self) -> HostResult<()> {
        self.robotFaceHost.paintPluginShelf()
    }

    /// Paints the launcher home on the board display.
    pub fn paintHome(&self) -> HostResult<()> {
        self.robotFaceHost.paintHome()
    }

    /// Paints the settings app on the board display.
    pub fn paintSettings(&self) -> HostResult<()> {
        self.robotFaceHost.paintSettings()
    }

    /// Paints the terminal app on the board display.
    pub fn paintTerminal(&self) -> HostResult<()> {
        self.robotFaceHost.paintTerminal()
    }

    /// Flushes a self-drawn UI RGB565 region to the physical display.
    pub fn flushUi(&self, rect: FaceRect, pixels: &[u8]) -> HostResult<()> {
        self.robotFaceHost.flushRgb565(rect, pixels)
    }

    /// Switches the physical panel to self-drawn UI ownership after board startup.
    pub fn activateUi(&self) {
        self.screenMirror.activateUi();
        self.robotFaceHost.activateUi();
    }

    /// Reads one calibrated touch sample, if the panel is pressed.
    pub fn pollTouch(&mut self) -> HostResult<Option<TouchPoint>> {
        self.touch.poll()
    }

    /// Adds ESP32-2432S028 board capabilities to a HostManager.
    pub fn installIntoHostManager(&self) -> HostManager {
        createRuntimeHostManager(self.gpioHost())
            .withRobotFaceHost(self.robotFaceHost())
            .withHostEnvironment(esp32_2432s028HostEnvironment())
    }
}
