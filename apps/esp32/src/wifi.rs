#![allow(non_snake_case)]

use std::net::Ipv4Addr;

use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::netif::{EspNetif, NetifConfiguration, NetifStack};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sntp::EspSntp;
use esp_idf_svc::wifi::{
    AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi, WifiDriver,
};
use operit_host_api::{HostError, HostResult};

use crate::config::Esp32FirmwareConfig;

/// Holds the ESP-IDF station client for the firmware lifetime.
pub struct Esp32Wifi {
    wifi: BlockingWifi<EspWifi<'static>>,
}

/// Reports whether the device ended in station mode or setup AP mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Esp32WifiMode {
    Station,
    SetupAccessPoint,
}

impl Esp32Wifi {
    /// Starts SNTP after station networking is ready and configures local time.
    pub fn startTimeSync() -> HostResult<EspSntp<'static>> {
        let timezone = option_env!("OPERIT_TIMEZONE").unwrap_or("CST-8");
        std::env::set_var("TZ", timezone);
        unsafe {
            tzset();
        }
        EspSntp::new_default()
            .map_err(|error| HostError::new(format!("SNTP init: {error}")))
    }

    /// Connects to the compile-time station network and waits for IPv4.
    #[allow(dead_code)]
    pub fn connect(
        modem: Modem<'static>,
        config: &Esp32FirmwareConfig,
        nvs: EspDefaultNvsPartition,
    ) -> HostResult<Self> {
        let sysLoop = EspSystemEventLoop::take()
            .map_err(|error| HostError::new(format!("event loop: {error}")))?;
        let mut wifi = BlockingWifi::wrap(
            EspWifi::new(modem, sysLoop.clone(), Some(nvs))
                .map_err(|error| HostError::new(format!("wifi driver: {error}")))?,
            sysLoop,
        )
        .map_err(|error| HostError::new(format!("wifi wrap: {error}")))?;
        let mut client = ClientConfiguration::default();
        client.ssid.clear();
        client
            .ssid
            .push_str(&config.wifiSsid)
            .map_err(|_| HostError::new("Wi-Fi SSID exceeds the ESP-IDF station field"))?;
        client.password.clear();
        client
            .password
            .push_str(&config.wifiPassword)
            .map_err(|_| HostError::new("Wi-Fi password exceeds the ESP-IDF station field"))?;
        client.auth_method = if config.wifiPassword.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        };
        wifi.set_configuration(&Configuration::Client(client))
            .map_err(|error| HostError::new(format!("wifi config: {error}")))?;
        wifi.start()
            .map_err(|error| HostError::new(format!("wifi start: {error}")))?;
        wifi.connect()
            .map_err(|error| HostError::new(format!("wifi connect: {error}")))?;
        wifi.wait_netif_up()
            .map_err(|error| HostError::new(format!("wifi netif: {error}")))?;
        Ok(Self { wifi })
    }

    /// Returns the station IPv4 address after association.
    pub fn ipv4(&self) -> HostResult<Ipv4Addr> {
        let info = self
            .wifi
            .wifi()
            .sta_netif()
            .get_ip_info()
            .map_err(|error| HostError::new(format!("wifi ip: {error}")))?;
        Ok(info.ip)
    }

    /// Connects to station Wi-Fi or falls back to the setup access point.
    pub fn connectOrSetup(
        modem: Modem<'static>,
        config: &Esp32FirmwareConfig,
        nvs: EspDefaultNvsPartition,
    ) -> HostResult<(Self, Esp32WifiMode)> {
        let sysLoop = EspSystemEventLoop::take()
            .map_err(|error| HostError::new(format!("event loop: {error}")))?;
        let mut wifi = BlockingWifi::wrap(
            EspWifi::wrap_all(
                WifiDriver::new(modem, sysLoop.clone(), Some(nvs))
                    .map_err(|error| HostError::new(format!("wifi driver: {error}")))?,
                EspNetif::new(NetifStack::Sta)
                    .map_err(|error| HostError::new(format!("wifi STA netif: {error}")))?,
                setupAccessPointNetif()?,
            )
            .map_err(|error| HostError::new(format!("wifi wrap: {error}")))?,
            sysLoop,
        )
        .map_err(|error| HostError::new(format!("wifi wrap: {error}")))?;
        if config.hasWifi() {
            if let Ok(()) = tryStation(&mut wifi, config) {
                return Ok((Self { wifi }, Esp32WifiMode::Station));
            }
            log::warn!("operit-esp32 station connect failed; switching to setup AP");
            let _ = wifi.stop();
            configureAccessPoint(&mut wifi)?;
            wifi.start()
                .map_err(|error| HostError::new(format!("setup AP start: {error}")))?;
            return Ok((Self { wifi }, Esp32WifiMode::SetupAccessPoint));
        }
        configureAccessPoint(&mut wifi)?;
        wifi.start()
            .map_err(|error| HostError::new(format!("setup AP start: {error}")))?;
        Ok((Self { wifi }, Esp32WifiMode::SetupAccessPoint))
    }

    /// Starts the open setup access point when station credentials are absent or failed.
    #[allow(dead_code)]
    pub fn startSetupAccessPoint(
        modem: Modem<'static>,
        nvs: EspDefaultNvsPartition,
    ) -> HostResult<Self> {
        let sysLoop = EspSystemEventLoop::take()
            .map_err(|error| HostError::new(format!("event loop: {error}")))?;
        let mut wifi = BlockingWifi::wrap(
            EspWifi::new(modem, sysLoop.clone(), Some(nvs))
                .map_err(|error| HostError::new(format!("wifi driver: {error}")))?,
            sysLoop,
        )
        .map_err(|error| HostError::new(format!("wifi wrap: {error}")))?;
        let mut accessPoint = esp_idf_svc::wifi::AccessPointConfiguration::default();
        accessPoint.ssid.clear();
        accessPoint
            .ssid
            .push_str("Operit-ESP32-Setup")
            .map_err(|_| HostError::new("setup AP SSID exceeds the Wi-Fi field"))?;
        accessPoint.auth_method = AuthMethod::None;
        accessPoint.max_connections = 4;
        wifi.set_configuration(&Configuration::AccessPoint(accessPoint))
            .map_err(|error| HostError::new(format!("setup AP config: {error}")))?;
        wifi.start()
            .map_err(|error| HostError::new(format!("setup AP start: {error}")))?;
        Ok(Self { wifi })
    }
}

fn tryStation(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    config: &Esp32FirmwareConfig,
) -> HostResult<()> {
    let mut client = ClientConfiguration::default();
    client.ssid.clear();
    client
        .ssid
        .push_str(&config.wifiSsid)
        .map_err(|_| HostError::new("Wi-Fi SSID exceeds the ESP-IDF station field"))?;
    client.password.clear();
    client
        .password
        .push_str(&config.wifiPassword)
        .map_err(|_| HostError::new("Wi-Fi password exceeds the ESP-IDF station field"))?;
    client.auth_method = if config.wifiPassword.is_empty() {
        AuthMethod::None
    } else {
        AuthMethod::WPAWPA2Personal
    };
    wifi.set_configuration(&Configuration::Client(client))
        .map_err(|error| HostError::new(format!("wifi config: {error}")))?;
    wifi.start()
        .map_err(|error| HostError::new(format!("wifi start: {error}")))?;
    match wifi.scan() {
        Ok(networks) => {
            for network in networks {
                log::info!(
                    "operit-esp32 Wi-Fi scan: ssid={:?} auth={:?} signal={}",
                    network.ssid,
                    network.auth_method,
                    network.signal_strength
                );
            }
        }
        Err(error) => log::warn!("operit-esp32 Wi-Fi scan failed: {error}"),
    }
    wifi.connect()
        .map_err(|error| HostError::new(format!("wifi connect: {error}")))?;
    wifi.wait_netif_up()
        .map_err(|error| HostError::new(format!("wifi netif: {error}")))
}

fn configureAccessPoint(wifi: &mut BlockingWifi<EspWifi<'static>>) -> HostResult<()> {
    let mut accessPoint = esp_idf_svc::wifi::AccessPointConfiguration::default();
    accessPoint.ssid.clear();
    accessPoint
        .ssid
        .push_str("Operit-ESP32-Setup")
        .map_err(|_| HostError::new("setup AP SSID exceeds the Wi-Fi field"))?;
    accessPoint.auth_method = AuthMethod::None;
    accessPoint.max_connections = 4;
    wifi.set_configuration(&Configuration::AccessPoint(accessPoint))
        .map_err(|error| HostError::new(format!("setup AP config: {error}")))
}

fn setupAccessPointNetif() -> HostResult<EspNetif> {
    let mut configuration = NetifConfiguration::wifi_default_router();
    configuration.key = "WIFI_AP_SETUP".try_into().unwrap();
    if let Some(esp_idf_svc::ipv4::Configuration::Router(router)) =
        &mut configuration.ip_configuration
    {
        router.subnet.gateway = Ipv4Addr::new(192, 168, 4, 1);
    }
    EspNetif::new_with_conf(&configuration)
        .map_err(|error| HostError::new(format!("setup AP netif: {error}")))
}

unsafe extern "C" {
    fn tzset();
}
