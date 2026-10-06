#![allow(non_snake_case)]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use operit_board_esp32::ESP32_2432S028_BOARD_ID;

/// Snapshot published by the firmware status endpoint.
#[derive(Clone, Debug)]
pub struct FirmwareStatusSnapshot {
    pub boardId: String,
    pub expression: String,
    pub ipv4: String,
    pub wifiSsid: String,
    pub wifiConnected: bool,
    pub pairingCode: String,
}

/// Shared firmware status consumed by the setup and status endpoints.
pub struct FirmwareStatus {
    boardId: String,
    revision: AtomicU32,
    expression: Mutex<String>,
    ipv4: Mutex<String>,
    wifiSsid: Mutex<String>,
    wifiConnected: AtomicBool,
    pairingCode: Mutex<String>,
}

impl FirmwareStatus {
    /// Creates firmware status with no network address yet.
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            boardId: ESP32_2432S028_BOARD_ID.to_string(),
            revision: AtomicU32::new(0),
            expression: Mutex::new(expression.into()),
            ipv4: Mutex::new(String::new()),
            wifiSsid: Mutex::new(String::new()),
            wifiConnected: AtomicBool::new(false),
            pairingCode: Mutex::new(String::new()),
        }
    }

    /// True only for a live station connection; an AP address is not enough.
    pub fn setWifiConnected(&self, connected: bool) {
        if self.wifiConnected.swap(connected, Ordering::Relaxed) != connected {
            self.revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records the current robot face expression.
    pub fn setExpression(&self, expression: impl Into<String>) {
        if let Ok(mut current) = self.expression.lock() {
            *current = expression.into();
            self.revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records the current station IPv4 address.
    pub fn setIpv4(&self, ipv4: impl Into<String>) {
        if let Ok(mut current) = self.ipv4.lock() {
            *current = ipv4.into();
            self.revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Records the associated Wi-Fi SSID without storing the password.
    pub fn setWifiSsid(&self, ssid: impl Into<String>) {
        if let Ok(mut current) = self.wifiSsid.lock() {
            *current = ssid.into();
            self.revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Publishes the one-time Edge pairing code while a pairing is pending.
    pub fn setPairingCode(&self, code: impl Into<String>) {
        if let Ok(mut current) = self.pairingCode.lock() {
            let code = code.into();
            if *current != code {
                *current = code;
                self.revision.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Returns a monotonic version for consumers that only need to repaint on change.
    pub fn revision(&self) -> u32 {
        self.revision.load(Ordering::Relaxed)
    }

    /// Returns a copy of the values shown on the firmware home page.
    pub fn snapshot(&self) -> FirmwareStatusSnapshot {
        FirmwareStatusSnapshot {
            boardId: self.boardId.clone(),
            wifiConnected: self.wifiConnected.load(Ordering::Relaxed),
            expression: self
                .expression
                .lock()
                .map(|value| value.clone())
                .unwrap_or_default(),
            ipv4: self
                .ipv4
                .lock()
                .map(|value| value.clone())
                .unwrap_or_default(),
            wifiSsid: self
                .wifiSsid
                .lock()
                .map(|value| value.clone())
                .unwrap_or_default(),
            pairingCode: self
                .pairingCode
                .lock()
                .map(|value| value.clone())
                .unwrap_or_default(),
        }
    }
}

/// Renders firmware status as JSON for machine clients.
pub fn renderStatusJson(snapshot: &FirmwareStatusSnapshot) -> String {
    serde_json::json!({
        "boardId": snapshot.boardId, "expression": snapshot.expression,
        "wifiSsid": snapshot.wifiSsid, "ipv4": snapshot.ipv4,
        "wifiConnected": snapshot.wifiConnected, "pairingCode": snapshot.pairingCode,
    }).to_string()
}

#[cfg(test)]
mod pairing_tests {
    use super::*;
    #[test]
    fn setup_ap_address_is_not_a_station_connection() {
        let status = FirmwareStatus::new("test");
        status.setIpv4("192.168.4.1");
        status.setWifiSsid("Operit-ESP32-Setup");
        assert!(!status.snapshot().wifiConnected);
        status.setWifiConnected(true);
        let revision = status.revision();
        status.setWifiConnected(true);
        assert_eq!(status.revision(), revision);
        status.setWifiConnected(false);
        assert!(!status.snapshot().wifiConnected);
        assert!(status.revision() > revision);
        let json: serde_json::Value = serde_json::from_str(&renderStatusJson(&status.snapshot())).unwrap();
        assert_eq!(json["wifiConnected"], false);
    }
    #[test]
    fn unchanged_pairing_code_does_not_trigger_repaint() {
        let status = FirmwareStatus::new("test");
        status.setPairingCode("001234");
        let revision = status.revision();
        status.setPairingCode("001234");
        assert_eq!(status.revision(), revision);
        status.setPairingCode("");
        assert_ne!(status.revision(), revision);
    }
}

/// The six-digit card must follow a NEW request, not the first UUID in storage.
/// Keeps only the current service snapshot, never an accumulating history.
#[derive(Default)]
pub struct PairingCodeSelection {
    knownIds: Vec<String>,
    selectedId: String,
}
impl PairingCodeSelection {
    pub fn select(
        &mut self,
        prompts: &[operit_node_runtime::NodeServices::PairingPrompt],
    ) -> String {
        let valid = |p: &&operit_node_runtime::NodeServices::PairingPrompt| {
            p.confirmationCode.len() == 6 && p.confirmationCode.bytes().all(|b| b.is_ascii_digit())
        };
        let selected = prompts
            .iter()
            .filter(valid)
            .find(|p| !self.knownIds.contains(&p.pairingId))
            .or_else(|| {
                prompts
                    .iter()
                    .filter(valid)
                    .find(|p| p.pairingId == self.selectedId)
            })
            .or_else(|| prompts.iter().filter(valid).next());
        self.selectedId = selected.map(|p| p.pairingId.clone()).unwrap_or_default();
        self.knownIds = prompts.iter().map(|p| p.pairingId.clone()).collect();
        selected
            .map(|p| p.confirmationCode.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod pairing_selection_tests {
    use super::*;
    use operit_node_runtime::NodeServices::PairingPrompt;
    fn prompt(id: &str, code: &str) -> PairingPrompt {
        PairingPrompt {
            pairingId: id.into(),
            peerNodeId: "peer".into(),
            displayName: "device".into(),
            confirmationCode: code.into(),
        }
    }
    #[test]
    fn newly_received_code_replaces_restored_prompt_and_stays_selected() {
        let mut selection = PairingCodeSelection::default();
        assert_eq!(selection.select(&[prompt("old", "054216")]), "054216");
        let both = [prompt("old", "054216"), prompt("new", "123456")];
        assert_eq!(selection.select(&both), "123456");
        assert_eq!(selection.select(&both), "123456");
        assert_eq!(selection.select(&[prompt("old", "054216")]), "054216");
        assert_eq!(selection.select(&[]), "");
        assert!(selection.knownIds.is_empty());
    }
    #[test]
    fn invalid_codes_never_displace_a_valid_current_prompt() {
        let mut selection = PairingCodeSelection::default();
        assert_eq!(selection.select(&[prompt("old", "123456")]), "123456");
        assert_eq!(
            selection.select(&[prompt("old", "123456"), prompt("bad", "你好")]),
            "123456"
        );
        assert_eq!(selection.select(&[prompt("bad", "12A456")]), "");
    }
}
