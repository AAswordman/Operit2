//! Build-only reference for WPA station networking with the board SDK config.
//! No Edge protocol, pairing, display, HTTP or application services are included.
//! This is a size probe, not an Edge node. Do not flash it over a paired device.

use esp_idf_hal::{delay::FreeRtos, peripherals::Peripherals};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    netif::{EspNetif, NetifStack},
    nvs::EspDefaultNvsPartition,
    sys::EspError,
    wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi, WifiDriver},
};

fn main() {
    esp_idf_svc::sys::link_patches();
    if let Err(error) = run() {
        unsafe {
            esp_idf_svc::sys::esp_rom_printf(
                b"Wi-Fi size probe failed: %d\n\0".as_ptr().cast(), error.code(),
            );
        }
    }
    loop { FreeRtos::delay_ms(1000); }
}

fn run() -> Result<(), EspError> {
    let peripherals = Peripherals::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let event_loop = EspSystemEventLoop::take()?;
    let mut wifi = EspWifi::wrap_all(
        WifiDriver::new(peripherals.modem, event_loop, Some(nvs))?,
        EspNetif::new(NetifStack::Sta)?,
        EspNetif::new(NetifStack::Ap)?,
    )?;
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: "operit-size-probe".try_into().unwrap(),
        password: "size-probe-only".try_into().unwrap(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))?;
    wifi.start()?;
    wifi.connect()?;
    loop { FreeRtos::delay_ms(1000); }
}
