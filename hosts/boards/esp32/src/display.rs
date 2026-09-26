#![allow(non_snake_case)]

use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{InputPin, Output, OutputPin, PinDriver};
use esp_idf_hal::spi::{
    config::Config as SpiConfig, Dma, SpiAnyPins, SpiDeviceDriver, SpiDriver, SpiDriverConfig,
};
use esp_idf_hal::units::FromValueType;
use operit_host_api::{HostError, HostResult};

use crate::face::FaceRect;
use crate::mirror_color::pack_rgb332;
use crate::pins::{logicalDisplaySize, madctlForRotation, DISPLAY_ROTATION_DEGREES};
use crate::robot_face::FaceCanvas;

const ILI9341_SWRESET: u8 = 0x01;
const ILI9341_SLPOUT: u8 = 0x11;
const ILI9341_NORON: u8 = 0x13;
const ILI9341_DISPON: u8 = 0x29;
const ILI9341_CASET: u8 = 0x2A;
const ILI9341_PASET: u8 = 0x2B;
const ILI9341_RAMWR: u8 = 0x2C;
const ILI9341_MADCTL: u8 = 0x36;
const ILI9341_PIXFMT: u8 = 0x3A;
const ILI9341_FRMCTR1: u8 = 0xB1;
const ILI9341_DISCTRL: u8 = 0xB6;
const ILI9341_PWCTRL1: u8 = 0xC0;
const ILI9341_PWCTRL2: u8 = 0xC1;
const ILI9341_VMCTRL1: u8 = 0xC5;
const ILI9341_VMCTRL2: u8 = 0xC7;
const ILI9341_PWCTRLA: u8 = 0xCB;
const ILI9341_PWCTRLB: u8 = 0xCF;
const ILI9341_ENABLE3G: u8 = 0xF2;
const ILI9341_GAMMASET: u8 = 0x26;
const ILI9341_POSGAMMA: u8 = 0xE0;
const ILI9341_NEGGAMMA: u8 = 0xE1;

/// Thread-safe copy of the logical display framebuffer for diagnostics and mirroring.
///
/// The face renderer writes to the ILI9341, so keeping this copy lets the firmware
/// expose the current screen over Wi-Fi without reading the panel back over SPI.
pub struct Esp32ScreenMirror {
    width: u16,
    height: u16,
    lvgl_active: AtomicBool,
    state: Mutex<Esp32ScreenMirrorState>,
}

/// One ordered rectangle in the compact screen mirror.
#[derive(Clone, Copy, Debug)]
pub struct Esp32ScreenMirrorRect {
    pub rect: FaceRect,
    pub color: u16,
}

struct Esp32ScreenMirrorState {
    background: u16,
    rects: Vec<Esp32ScreenMirrorRect>,
    pixels: Vec<u8>,
}

impl Esp32ScreenMirror {
    /// Creates a compact RGB332 mirror with fallible allocation.
    pub fn new(width: u16, height: u16) -> HostResult<Self> {
        let length = usize::from(width) * usize::from(height);
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(length)
            .map_err(|error| HostError::new(format!("screen mirror allocation: {error}")))?;
        pixels.resize(length, 0);
        Ok(Self {
            width,
            height,
            lvgl_active: AtomicBool::new(false),
            state: Mutex::new(Esp32ScreenMirrorState {
                background: 0,
                rects: Vec::new(),
                pixels,
            }),
        })
    }

    /// Returns the logical framebuffer dimensions.
    pub fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// Releases the optional 320x240 pixel copy when no preview server uses it.
    /// Physical TFT writes continue independently of this diagnostic mirror.
    pub fn disablePixelMirror(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.pixels = Vec::new();
        }
    }

    /// Makes LVGL the sole owner of physical display updates.
    pub fn activateLvgl(&self) {
        self.lvgl_active.store(true, Ordering::Release);
    }

    fn isLvglActive(&self) -> bool {
        self.lvgl_active.load(Ordering::Acquire)
    }

    /// Reads the compact drawing state while holding its lock for the duration of the read.
    pub fn withState<F, T>(&self, reader: F) -> T
    where
        F: FnOnce(u16, &[Esp32ScreenMirrorRect]) -> T,
    {
        match self.state.lock() {
            Ok(state) => reader(state.background, &state.rects),
            Err(_) => reader(0, &[]),
        }
    }

    /// Reads compact RGB332 pixels; callers expand only the output row or packet.
    pub fn withRgb332<F, T>(&self, reader: F) -> T
    where
        F: FnOnce(&[u8]) -> T,
    {
        match self.state.lock() {
            Ok(state) => reader(&state.pixels),
            Err(_) => reader(&[]),
        }
    }

    /// Copies one row, releasing the screen lock before callers perform network I/O.
    pub fn copyRgb332Row(&self, y: u16, output: &mut [u8]) {
        output.fill(0);
        if y >= self.height { return; }
        if let Ok(state) = self.state.lock() {
            if state.pixels.is_empty() { return; }
            let width = usize::from(self.width);
            let count = output.len().min(width);
            let start = usize::from(y) * width;
            output[..count].copy_from_slice(&state.pixels[start..start + count]);
        }
    }

    fn fill(&self, color: u16) {
        if let Ok(mut state) = self.state.lock() {
            state.background = color;
            state.rects.clear();
            state.pixels.fill(pack_rgb332(color));
        }
    }

    fn fillRect(&self, rect: FaceRect, color: u16) {
        if let Ok(mut state) = self.state.lock() {
            state.rects.push(Esp32ScreenMirrorRect { rect, color });
            if state.pixels.is_empty() { return; }
            let right = rect.x.saturating_add(rect.width).min(self.width);
            let bottom = rect.y.saturating_add(rect.height).min(self.height);
            for y in rect.y.min(self.height)..bottom {
                for x in rect.x.min(self.width)..right {
                    let index = usize::from(y) * usize::from(self.width) + usize::from(x);
                    state.pixels[index] = pack_rgb332(color);
                }
            }
        }
    }

    /// Copies one little-endian RGB565 LVGL region into the framebuffer mirror.
    fn writeRgb565(&self, rect: FaceRect, bytes: &[u8]) {
        if let Ok(mut state) = self.state.lock() {
            if state.pixels.is_empty() { return; }
            let right = rect.x.saturating_add(rect.width).min(self.width);
            let bottom = rect.y.saturating_add(rect.height).min(self.height);
            for y in rect.y.min(self.height)..bottom {
                for x in rect.x.min(self.width)..right {
                    let offset = (usize::from(y - rect.y) * usize::from(rect.width) + usize::from(x - rect.x)) * 2;
                    if offset + 1 >= bytes.len() {
                        return;
                    }
                    let index = usize::from(y) * usize::from(self.width) + usize::from(x);
                    state.pixels[index] = pack_rgb332(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]));
                }
            }
        }
    }
}

/// Drives the ESP32-2432S028 ILI9341 panel over HSPI.
pub struct Esp32Ili9341 {
    spi: SpiDeviceDriver<'static, SpiDriver<'static>>,
    dc: PinDriver<'static, Output>,
    _backlight: PinDriver<'static, Output>,
    width: u16,
    height: u16,
    mirror: Arc<Esp32ScreenMirror>,
}

impl Esp32Ili9341 {
    /// Initializes the ILI9341 at the board's configured landscape rotation.
    pub fn new<SPI, SCLK, MOSI, MISO, CS, DC, BL>(
        spi: SPI,
        sclk: SCLK,
        mosi: MOSI,
        miso: MISO,
        cs: CS,
        dc: DC,
        backlight: BL,
    ) -> HostResult<Self>
    where
        SPI: SpiAnyPins + 'static,
        SCLK: OutputPin + 'static,
        MOSI: OutputPin + 'static,
        MISO: InputPin + 'static,
        CS: OutputPin + 'static,
        DC: OutputPin + 'static,
        BL: OutputPin + 'static,
    {
        let driver = SpiDriver::new(
            spi,
            sclk,
            mosi,
            Some(miso),
            &SpiDriverConfig::new().dma(Dma::Auto(4096)),
        )
        .map_err(|error| HostError::new(error.to_string()))?;
        let spi = SpiDeviceDriver::new(
            driver,
            Some(cs),
            &SpiConfig::new().baudrate(10.MHz().into()),
        )
        .map_err(|error| HostError::new(error.to_string()))?;
        let dc = PinDriver::output(dc).map_err(|error| HostError::new(error.to_string()))?;
        let mut backlight =
            PinDriver::output(backlight).map_err(|error| HostError::new(error.to_string()))?;
        backlight
            .set_low()
            .map_err(|error| HostError::new(error.to_string()))?;
        let (width, height) = logicalDisplaySize(DISPLAY_ROTATION_DEGREES);
        let mut display = Self {
            spi,
            dc,
            _backlight: backlight,
            width,
            height,
            mirror: Arc::new(Esp32ScreenMirror::new(width, height)?),
        };
        display.initialize()?;
        display
            ._backlight
            .set_high()
            .map_err(|error| HostError::new(error.to_string()))?;
        Ok(display)
    }

    /// Returns the framebuffer mirror shared with the firmware web server.
    pub fn screenMirror(&self) -> Arc<Esp32ScreenMirror> {
        Arc::clone(&self.mirror)
    }

    /// Sends the ILI9341 power-up sequence and selects the firmware rotation.
    fn initialize(&mut self) -> HostResult<()> {
        // ST7789 variant of ESP32-2432S028: configure power after sleep-out.
        self.writeCommand(ILI9341_SWRESET, &[])?;
        FreeRtos::delay_ms(150);
        self.writeCommand(ILI9341_SLPOUT, &[])?;
        FreeRtos::delay_ms(120);
        self.writeCommand(ILI9341_PIXFMT, &[0x55])?;
        self.writeCommand(ILI9341_MADCTL, &[0x60])?;
        self.writeCommand(0xB2, &[0x0C, 0x0C, 0x00, 0x33, 0x33])?;
        self.writeCommand(0xB7, &[0x35])?;
        self.writeCommand(0xBB, &[0x19])?;
        self.writeCommand(0xC0, &[0x2C])?;
        self.writeCommand(0xC2, &[0x01])?;
        self.writeCommand(0xC3, &[0x12])?;
        self.writeCommand(0xC4, &[0x20])?;
        self.writeCommand(0xC6, &[0x0F])?;
        self.writeCommand(0xD0, &[0xA4, 0xA1])?;
        self.writeCommand(0x21, &[])?;
        self.writeCommand(ILI9341_NORON, &[])?;
        self.writeCommand(ILI9341_DISPON, &[])?;
        FreeRtos::delay_ms(120);
        log::info!("TFT: ST7789 initialization, landscape MADCTL=0x60");
        Ok(())
    }

    /// Writes one ILI9341 command with optional data bytes.
    fn writeCommand(&mut self, command: u8, data: &[u8]) -> HostResult<()> {
        self.dc
            .set_low()
            .map_err(|error| HostError::new(error.to_string()))?;
        self.spi
            .write(&[command])
            .map_err(|error| HostError::new(error.to_string()))?;
        if !data.is_empty() {
            self.dc
                .set_high()
                .map_err(|error| HostError::new(error.to_string()))?;
            self.spi
                .write(data)
                .map_err(|error| HostError::new(error.to_string()))?;
        }
        Ok(())
    }

    /// Selects the ILI9341 column and row window for a later pixel burst.
    fn setWindow(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> HostResult<()> {
        self.writeCommand(
            ILI9341_CASET,
            &[
                (x0 >> 8) as u8,
                (x0 & 0xff) as u8,
                (x1 >> 8) as u8,
                (x1 & 0xff) as u8,
            ],
        )?;
        self.writeCommand(
            ILI9341_PASET,
            &[
                (y0 >> 8) as u8,
                (y0 & 0xff) as u8,
                (y1 >> 8) as u8,
                (y1 & 0xff) as u8,
            ],
        )
    }

    /// Fills the current window with one RGB565 color.
    fn fillWindow(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: u16) -> HostResult<()> {
        if x0 > x1 || y0 > y1 {
            return Ok(());
        }
        self.setWindow(x0, y0, x1, y1)?;
        self.dc
            .set_low()
            .map_err(|error| HostError::new(error.to_string()))?;
        self.spi
            .write(&[ILI9341_RAMWR])
            .map_err(|error| HostError::new(error.to_string()))?;
        self.dc
            .set_high()
            .map_err(|error| HostError::new(error.to_string()))?;
        let pixel = [(color >> 8) as u8, (color & 0xff) as u8];
        let count = u32::from(x1 - x0 + 1) * u32::from(y1 - y0 + 1);
        let mut burst = [0u8; 128];
        for chunk in burst.chunks_mut(2) {
            chunk.copy_from_slice(&pixel);
        }
        let mut remaining = count;
        let mut burstsSinceYield = 0u8;
        while remaining > 0 {
            let pixels = remaining.min(64);
            let bytes = (pixels * 2) as usize;
            self.spi
                .write(&burst[..bytes])
                .map_err(|error| HostError::new(error.to_string()))?;
            remaining -= pixels;
            burstsSinceYield = burstsSinceYield.saturating_add(1);
            if burstsSinceYield >= 8 && remaining > 0 {
                FreeRtos::delay_ms(1);
                burstsSinceYield = 0;
            }
        }
        Ok(())
    }
}

impl FaceCanvas for Esp32Ili9341 {
    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn fill(&mut self, color: u16) -> HostResult<()> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }
        if self.mirror.isLvglActive() {
            return Ok(());
        }
        self.fillWindow(0, 0, self.width - 1, self.height - 1, color)?;
        self.mirror.fill(color);
        Ok(())
    }

    fn fillRect(&mut self, rect: FaceRect, color: u16) -> HostResult<()> {
        if rect.width == 0 || rect.height == 0 {
            return Ok(());
        }
        if self.mirror.isLvglActive() {
            return Ok(());
        }
        let x1 = rect
            .x
            .saturating_add(rect.width)
            .saturating_sub(1)
            .min(self.width.saturating_sub(1));
        let y1 = rect
            .y
            .saturating_add(rect.height)
            .saturating_sub(1)
            .min(self.height.saturating_sub(1));
        if rect.x > x1 || rect.y > y1 {
            return Ok(());
        }
        self.fillWindow(rect.x, rect.y, x1, y1, color)?;
        self.mirror.fillRect(rect, color);
        Ok(())
    }

    fn flushRgb565(&mut self, rect: FaceRect, pixels: &[u8]) -> HostResult<()> {
        if rect.width == 0 || rect.height == 0 || pixels.len() < usize::from(rect.width) * usize::from(rect.height) * 2 {
            return Ok(());
        }
        let x1 = rect
            .x
            .saturating_add(rect.width)
            .saturating_sub(1)
            .min(self.width.saturating_sub(1));
        let y1 = rect
            .y
            .saturating_add(rect.height)
            .saturating_sub(1)
            .min(self.height.saturating_sub(1));
        if rect.x > x1 || rect.y > y1 {
            return Ok(());
        }
        // DMA transfers one complete RGB565 row at a time.
        self.setWindow(rect.x, rect.y, x1, y1)?;
        self.dc.set_low().map_err(|error| HostError::new(error.to_string()))?;
        self.spi.write(&[ILI9341_RAMWR]).map_err(|error| HostError::new(error.to_string()))?;
        self.dc.set_high().map_err(|error| HostError::new(error.to_string()))?;
        let mut swapped = [0u8; 640];
        for y in rect.y..=y1 {
            let offset = usize::from(y - rect.y) * usize::from(rect.width) * 2;
            let length = usize::from(x1 - rect.x + 1) * 2;
            for index in (0..length).step_by(2) {
                swapped[index] = pixels[offset + index + 1];
                swapped[index + 1] = pixels[offset + index];
            }
            self.spi.write(&swapped[..length]).map_err(|error| HostError::new(error.to_string()))?;
        }
        self.mirror.writeRgb565(rect, pixels);
        Ok(())
    }
}

impl operit_ui::Canvas for Esp32Ili9341 {
    fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn fill(&mut self, color: operit_ui::Color) {
        FaceCanvas::fill(self, color.0).unwrap_or(());
    }

    fn fill_rect(&mut self, rect: operit_ui::Rect, color: operit_ui::Color) {
        let _ = FaceCanvas::fillRect(
            self,
            FaceRect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            },
            color.0,
        );
    }
}
