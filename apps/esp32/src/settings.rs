#![allow(non_snake_case)]

use std::sync::{Arc, Mutex};

use esp_idf_hal::delay::FreeRtos;
use esp_idf_svc::http::server::{Configuration as HttpConfig, EspHttpServer};
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition, EspNvs};
use operit_host_api::{HostError, HostResult};

use crate::status::FirmwareStatus;

const SETTINGS_NAMESPACE: &str = "operit_setup";
const WIFI_SSID_KEY: &str = "wifi_ssid";
const WIFI_PASSWORD_KEY: &str = "wifi_password";
const EDGE_TOKEN_KEY: &str = "edge_token";

/// Runtime configuration loaded from device NVS.
#[derive(Clone, Debug, Default)]
pub struct Esp32FirmwareSettings {
    pub wifiSsid: String,
    pub wifiPassword: String,
    pub edgeToken: String,
}

impl Esp32FirmwareSettings {
    fn load(nvs: &Mutex<EspDefaultNvs>) -> HostResult<Self> {
        Ok(Self {
            wifiSsid: getString(nvs, WIFI_SSID_KEY)?,
            wifiPassword: getString(nvs, WIFI_PASSWORD_KEY)?,
            edgeToken: getString(nvs, EDGE_TOKEN_KEY)?,
        })
    }

    /// Writes all fields, including empty values that intentionally clear them.
    fn save(&self, nvs: &Mutex<EspDefaultNvs>) -> HostResult<()> {
        setString(nvs, WIFI_SSID_KEY, &self.wifiSsid)?;
        setString(nvs, WIFI_PASSWORD_KEY, &self.wifiPassword)?;
        setString(nvs, EDGE_TOKEN_KEY, &self.edgeToken)
    }
}

/// NVS-backed runtime settings for Wi-Fi and Edge startup.
pub struct Esp32SettingsStore {
    nvs: Mutex<EspDefaultNvs>,
}

impl Esp32SettingsStore {
    /// Opens the dedicated setup namespace in the default NVS partition.
    pub fn new(partition: EspDefaultNvsPartition) -> HostResult<Arc<Self>> {
        let nvs = EspNvs::new(partition, SETTINGS_NAMESPACE, true)
            .map_err(|error| HostError::new(format!("setup NVS: {error}")))?;
        Ok(Arc::new(Self {
            nvs: Mutex::new(nvs),
        }))
    }

    /// Loads settings from device flash.
    pub fn load(&self) -> HostResult<Esp32FirmwareSettings> {
        Esp32FirmwareSettings::load(&self.nvs)
    }

    /// Saves settings and commits them to flash.
    pub fn save(&self, settings: &Esp32FirmwareSettings) -> HostResult<()> {
        settings.save(&self.nvs)
    }
}

/// HTTP setup page shown while the device is an access point.
pub struct Esp32SetupServer {
    _server: EspHttpServer<'static>,
}

impl Esp32SetupServer {
    /// Serves the captive setup form and persists submitted settings.
    pub fn start(
        status: Arc<FirmwareStatus>,
        settingsStore: Arc<Esp32SettingsStore>,
        httpPort: u16,
    ) -> HostResult<Self> {
        let mut server = EspHttpServer::new(&HttpConfig {
            http_port: httpPort,
            ..Default::default()
        })
        .map_err(|error| HostError::new(format!("setup server: {error}")))?;
        crate::ui_capabilities::register(&mut server)?;
        let pageStatus = Arc::clone(&status);
        server
            .fn_handler("/", Method::Get, move |request| {
                request
                    .into_ok_response()?
                    .write_all(renderSetupPage(&pageStatus.snapshot()).as_bytes())?;
                Ok::<(), esp_idf_svc::io::EspIOError>(())
            })
            .map_err(|error| HostError::new(format!("setup /: {error}")))?;
        let savedStatus = Arc::clone(&status);
        let savedStore = Arc::clone(&settingsStore);
        server
            .fn_handler("/settings", Method::Post, move |mut request| {
                let mut body = vec![0u8; 1024];
                let length = request.read(&mut body)?;
                let form = parseUrlEncoded(&body[..length]);
                let settings = Esp32FirmwareSettings {
                    wifiSsid: form
                        .get("ssid")
                        .cloned()
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                    wifiPassword: form.get("password").cloned().unwrap_or_default(),
                    edgeToken: form
                        .get("token")
                        .cloned()
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                };
                if settings.wifiSsid.is_empty() {
                    request
                        .into_response(400, Some("Bad Request"), &[])?
                        .write_all(b"SSID is required")?;
                    return Ok::<(), esp_idf_svc::io::EspIOError>(());
                }
                if let Err(error) = savedStore.save(&settings) {
                    log::error!("operit-esp32 setup save: {}", error.message);
                    request
                        .into_response(500, Some("Internal Error"), &[])?
                        .write_all(b"Saving settings failed")?;
                    return Ok::<(), esp_idf_svc::io::EspIOError>(());
                }
                savedStatus.setExpression("restarting");
                let _ = std::thread::Builder::new()
                    .name("operit-setup-restart".to_string())
                    .spawn(move || {
                        FreeRtos::delay_ms(700);
                        esp_idf_svc::hal::reset::restart();
                    })
                    .map_err(|error| log::error!("operit-esp32 setup restart: {error}"));
                request
                    .into_ok_response()?
                    .write_all(renderSavedPage(&settings.wifiSsid).as_bytes())?;
                Ok::<(), esp_idf_svc::io::EspIOError>(())
            })
            .map_err(|error| HostError::new(format!("setup /settings: {error}")))?;
        let jsonStatus = Arc::clone(&status);
        server
            .fn_handler("/status.json", Method::Get, move |request| {
                request
                    .into_response(
                        200,
                        Some("OK"),
                        &[("Content-Type", "application/json; charset=utf-8")],
                    )?
                    .write_all(
                        crate::status::renderStatusJson(&jsonStatus.snapshot()).as_bytes(),
                    )?;
                Ok::<(), esp_idf_svc::io::EspIOError>(())
            })
            .map_err(|error| HostError::new(format!("setup /status.json: {error}")))?;
        Ok(Self { _server: server })
    }
}

/// Decodes a small HTML form without pulling a URL parser into flash.
fn parseUrlEncoded(body: &[u8]) -> std::collections::BTreeMap<String, String> {
    let mut output = std::collections::BTreeMap::new();
    let text = std::str::from_utf8(body).unwrap_or("");
    for pair in text.split('&') {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        if name.is_empty() {
            continue;
        }
        output.insert(decodeFormValue(name), decodeFormValue(value));
    }
    output
}

fn decodeFormValue(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let high = decodeHexDigit(bytes[index + 1]);
                let low = decodeHexDigit(bytes[index + 2]);
                if let (Some(high), Some(low)) = (high, low) {
                    output.push(high * 16 + low);
                    index += 3;
                } else {
                    output.push(b'%');
                    index += 1;
                }
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

fn decodeHexDigit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn getString(nvs: &Mutex<EspDefaultNvs>, key: &str) -> HostResult<String> {
    let nvs = nvs
        .lock()
        .map_err(|error| HostError::new(error.to_string()))?;
    let mut buffer = vec![0u8; 512];
    match nvs.get_str(key, &mut buffer) {
        Ok(Some(value)) => Ok(value.to_string()),
        Ok(None) => Ok(String::new()),
        Err(error) => Err(HostError::new(format!("setup NVS {key}: {error}"))),
    }
}

fn setString(nvs: &Mutex<EspDefaultNvs>, key: &str, value: &str) -> HostResult<()> {
    let nvs = nvs
        .lock()
        .map_err(|error| HostError::new(error.to_string()))?;
    nvs.set_str(key, value)
        .map_err(|error| HostError::new(format!("setup NVS {key}: {error}")))
}

fn renderSetupPage(_snapshot: &crate::status::FirmwareStatusSnapshot) -> String {
    "<!DOCTYPE html>\
<html lang=\"zh-CN\">\
<head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Operit2 设置</title></head>\
<body><h1>Operit2 设置</h1><p>连接家庭 Wi-Fi 后，ESP32 会启动 Edge 服务。</p>\
<form method=\"post\" action=\"/settings\">\
<label>Wi-Fi SSID<br><input name=\"ssid\" required></label><br>\
<label>Wi-Fi 密码<br><input name=\"password\" type=\"password\"></label><br>\
<label>Edge token<br><input name=\"token\"></label><br>\
<button type=\"submit\">保存并重启</button>\
</form>\
<p><a href=\"/status.json\">状态</a></p>\
</body></html>"
        .to_string()
}

fn renderSavedPage(ssid: &str) -> String {
    format!(
        "<!DOCTYPE html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><title>已保存</title></head>\
<body><h1>已保存</h1><p>Wi-Fi：{ssid}</p><p>设备正在重启。</p></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsesSpacesAndPercentEncoding() {
        let mut form = parseUrlEncoded(b"ssid=Home%20Net&password=a%2Bb%20c&token=Operit%20123");
        assert_eq!(form.remove("ssid").as_deref(), Some("Home Net"));
        assert_eq!(form.remove("password").as_deref(), Some("a+b c"));
        assert_eq!(form.remove("token").as_deref(), Some("Operit 123"));
    }
}
