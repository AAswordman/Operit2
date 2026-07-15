#![allow(non_snake_case)]

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use operit_plugin_sdk::js_sdk::toolpkg::{
    ToolPkgBroadcastAdapterData, ToolPkgBroadcastAirplaneModeData, ToolPkgBroadcastBatteryData,
    ToolPkgBroadcastBluetoothDeviceData, ToolPkgBroadcastBootData, ToolPkgBroadcastHeadsetData,
    ToolPkgBroadcastLifecycleData, ToolPkgBroadcastNetworkChangedData,
    ToolPkgBroadcastPowerConnectionData, ToolPkgBroadcastPowerSleepData,
    ToolPkgBroadcastScreenData, ToolPkgBroadcastSessionData, ToolPkgBroadcastTimeData,
    ToolPkgBroadcastUserPresenceData,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEventDomain {
    #[serde(rename = "app")]
    App,
    #[serde(rename = "host")]
    Host,
    #[serde(rename = "runtime")]
    Runtime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEventSource {
    #[serde(rename = "android.broadcast")]
    AndroidBroadcast,
    #[serde(rename = "linux.dbus")]
    LinuxDbus,
    #[serde(rename = "windows.system")]
    WindowsSystem,
    #[serde(rename = "macos.system")]
    MacosSystem,
    #[serde(rename = "ios.system")]
    IosSystem,
    #[serde(rename = "ohos.system")]
    OhosSystem,
    #[serde(rename = "web.event")]
    WebEvent,
    #[serde(rename = "flutter.lifecycle")]
    FlutterLifecycle,
    #[serde(rename = "runtime.timer")]
    RuntimeTimer,
    #[serde(rename = "runtime.interval")]
    RuntimeInterval,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEventPlatform {
    #[serde(rename = "android")]
    Android,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "macos")]
    Macos,
    #[serde(rename = "ios")]
    Ios,
    #[serde(rename = "ohos")]
    Ohos,
    #[serde(rename = "web")]
    Web,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEventTopic {
    #[serde(rename = "app.lifecycle.resumed")]
    AppLifecycleResumed,
    #[serde(rename = "app.lifecycle.inactive")]
    AppLifecycleInactive,
    #[serde(rename = "app.lifecycle.paused")]
    AppLifecyclePaused,
    #[serde(rename = "app.lifecycle.detached")]
    AppLifecycleDetached,
    #[serde(rename = "app.lifecycle.hidden")]
    AppLifecycleHidden,
    #[serde(rename = "system.boot.completed")]
    SystemBootCompleted,
    #[serde(rename = "system.power.connected")]
    SystemPowerConnected,
    #[serde(rename = "system.power.disconnected")]
    SystemPowerDisconnected,
    #[serde(rename = "system.power.sleep")]
    SystemPowerSleep,
    #[serde(rename = "system.power.wake")]
    SystemPowerWake,
    #[serde(rename = "system.battery.low")]
    SystemBatteryLow,
    #[serde(rename = "system.battery.okay")]
    SystemBatteryOkay,
    #[serde(rename = "system.screen.on")]
    SystemScreenOn,
    #[serde(rename = "system.screen.off")]
    SystemScreenOff,
    #[serde(rename = "system.user.present")]
    SystemUserPresent,
    #[serde(rename = "system.time.tick")]
    SystemTimeTick,
    #[serde(rename = "system.date.changed")]
    SystemDateChanged,
    #[serde(rename = "system.timezone.changed")]
    SystemTimezoneChanged,
    #[serde(rename = "system.airplane_mode.changed")]
    SystemAirplaneModeChanged,
    #[serde(rename = "system.headset.plug")]
    SystemHeadsetPlug,
    #[serde(rename = "system.network.changed")]
    SystemNetworkChanged,
    #[serde(rename = "system.session.lock")]
    SystemSessionLock,
    #[serde(rename = "system.session.unlock")]
    SystemSessionUnlock,
    #[serde(rename = "bluetooth.device.found")]
    BluetoothDeviceFound,
    #[serde(rename = "bluetooth.device.name_changed")]
    BluetoothDeviceNameChanged,
    #[serde(rename = "bluetooth.device.connected")]
    BluetoothDeviceConnected,
    #[serde(rename = "bluetooth.device.disconnected")]
    BluetoothDeviceDisconnected,
    #[serde(rename = "bluetooth.device.bond_state_changed")]
    BluetoothDeviceBondStateChanged,
    #[serde(rename = "bluetooth.adapter.connection_state_changed")]
    BluetoothAdapterConnectionStateChanged,
    #[serde(rename = "bluetooth.adapter.powered_changed")]
    BluetoothAdapterPoweredChanged,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeEvent {
    pub domain: RuntimeEventDomain,
    pub source: RuntimeEventSource,
    pub topic: RuntimeEventTopic,
    pub platform: RuntimeEventPlatform,
    pub payload: Value,
    pub occurredAtMillis: u64,
}

impl RuntimeEvent {
    /// Converts the normalized runtime event into the payload delivered to ToolPkg hooks.
    pub fn hostEventPayload(&self) -> Result<Value, String> {
        let data = self.canonicalData()?;
        Ok(serde_json::json!({
            "domain": &self.domain,
            "source": &self.source,
            "topic": &self.topic,
            "platform": &self.platform,
            "data": data,
            "occurredAtMillis": self.occurredAtMillis,
        }))
    }

    /// Validates and canonicalizes data according to the selected standard topic.
    #[allow(non_snake_case)]
    fn canonicalData(&self) -> Result<Value, String> {
        match self.topic {
            RuntimeEventTopic::AppLifecycleResumed
            | RuntimeEventTopic::AppLifecycleInactive
            | RuntimeEventTopic::AppLifecyclePaused
            | RuntimeEventTopic::AppLifecycleDetached
            | RuntimeEventTopic::AppLifecycleHidden => {
                canonicalData::<ToolPkgBroadcastLifecycleData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemBootCompleted => {
                canonicalData::<ToolPkgBroadcastBootData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemPowerConnected
            | RuntimeEventTopic::SystemPowerDisconnected => {
                canonicalData::<ToolPkgBroadcastPowerConnectionData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemPowerSleep | RuntimeEventTopic::SystemPowerWake => {
                canonicalData::<ToolPkgBroadcastPowerSleepData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemBatteryLow | RuntimeEventTopic::SystemBatteryOkay => {
                canonicalData::<ToolPkgBroadcastBatteryData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemScreenOn | RuntimeEventTopic::SystemScreenOff => {
                canonicalData::<ToolPkgBroadcastScreenData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemUserPresent => {
                canonicalData::<ToolPkgBroadcastUserPresenceData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemTimeTick
            | RuntimeEventTopic::SystemDateChanged
            | RuntimeEventTopic::SystemTimezoneChanged => {
                canonicalData::<ToolPkgBroadcastTimeData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemAirplaneModeChanged => {
                canonicalData::<ToolPkgBroadcastAirplaneModeData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemHeadsetPlug => {
                canonicalData::<ToolPkgBroadcastHeadsetData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemNetworkChanged => {
                canonicalData::<ToolPkgBroadcastNetworkChangedData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::SystemSessionLock | RuntimeEventTopic::SystemSessionUnlock => {
                canonicalData::<ToolPkgBroadcastSessionData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::BluetoothDeviceFound
            | RuntimeEventTopic::BluetoothDeviceNameChanged
            | RuntimeEventTopic::BluetoothDeviceConnected
            | RuntimeEventTopic::BluetoothDeviceDisconnected
            | RuntimeEventTopic::BluetoothDeviceBondStateChanged => {
                canonicalData::<ToolPkgBroadcastBluetoothDeviceData>(&self.topic, &self.payload)
            }
            RuntimeEventTopic::BluetoothAdapterConnectionStateChanged
            | RuntimeEventTopic::BluetoothAdapterPoweredChanged => {
                canonicalData::<ToolPkgBroadcastAdapterData>(&self.topic, &self.payload)
            }
        }
    }
}

/// Round-trips one topic payload through its canonical SDK data structure.
#[allow(non_snake_case)]
fn canonicalData<T>(topic: &RuntimeEventTopic, payload: &Value) -> Result<Value, String>
where
    T: DeserializeOwned + Serialize,
{
    let data = serde_json::from_value::<T>(payload.clone())
        .map_err(|error| format!("invalid {topic:?} event data: {error}"))?;
    serde_json::to_value(data)
        .map_err(|error| format!("encode canonical {topic:?} event data failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies every supported host platform can enter the normalized runtime event model.
    #[test]
    fn deserializes_every_supported_host_platform() {
        let cases = [
            ("android", RuntimeEventPlatform::Android),
            ("windows", RuntimeEventPlatform::Windows),
            ("linux", RuntimeEventPlatform::Linux),
            ("macos", RuntimeEventPlatform::Macos),
            ("ios", RuntimeEventPlatform::Ios),
            ("ohos", RuntimeEventPlatform::Ohos),
            ("web", RuntimeEventPlatform::Web),
        ];
        for (wire, expected) in cases {
            let event: RuntimeEvent = serde_json::from_value(serde_json::json!({
                "domain": "host",
                "source": "flutter.lifecycle",
                "topic": "app.lifecycle.resumed",
                "platform": wire,
                "payload": {"state": "resumed"},
                "occurredAtMillis": 1,
            }))
            .expect("supported platform event must deserialize");
            assert_eq!(event.platform, expected);
            assert_eq!(event.topic, RuntimeEventTopic::AppLifecycleResumed);
        }
    }

    /// Verifies the ToolPkg-facing broadcast payload preserves normalized event fields.
    #[test]
    fn builds_toolpkg_broadcast_payload() {
        let event = RuntimeEvent {
            domain: RuntimeEventDomain::Host,
            source: RuntimeEventSource::FlutterLifecycle,
            topic: RuntimeEventTopic::AppLifecycleResumed,
            platform: RuntimeEventPlatform::Ios,
            payload: serde_json::json!({"state": "resumed"}),
            occurredAtMillis: 42,
        };

        assert_eq!(
            event
                .hostEventPayload()
                .expect("valid payload must canonicalize"),
            serde_json::json!({
                "domain": "host",
                "source": "flutter.lifecycle",
                "topic": "app.lifecycle.resumed",
                "platform": "ios",
                "data": {"state": "resumed"},
                "occurredAtMillis": 42,
            })
        );
    }

    /// Verifies one topic produces an identical plugin data shape on every platform.
    #[test]
    fn canonicalizes_network_data_identically_across_platforms() {
        let platforms = [
            RuntimeEventPlatform::Android,
            RuntimeEventPlatform::Windows,
            RuntimeEventPlatform::Linux,
            RuntimeEventPlatform::Macos,
            RuntimeEventPlatform::Ios,
            RuntimeEventPlatform::Ohos,
            RuntimeEventPlatform::Web,
        ];
        for platform in platforms {
            let event = RuntimeEvent {
                domain: RuntimeEventDomain::Host,
                source: RuntimeEventSource::FlutterLifecycle,
                topic: RuntimeEventTopic::SystemNetworkChanged,
                platform,
                payload: serde_json::json!({
                    "connected": true,
                    "networkType": "wifi",
                }),
                occurredAtMillis: 10,
            };
            let payload = event
                .hostEventPayload()
                .expect("canonical network data must validate");
            assert_eq!(
                payload["data"],
                serde_json::json!({
                    "connected": true,
                    "networkType": "wifi"
                })
            );
        }
    }

    /// Verifies malformed platform data cannot reach a standard topic hook.
    #[test]
    fn rejects_noncanonical_standard_topic_data() {
        let event = RuntimeEvent {
            domain: RuntimeEventDomain::Host,
            source: RuntimeEventSource::WindowsSystem,
            topic: RuntimeEventTopic::SystemNetworkChanged,
            platform: RuntimeEventPlatform::Windows,
            payload: serde_json::json!({"notificationType": 1}),
            occurredAtMillis: 10,
        };
        assert!(event.hostEventPayload().is_err());
    }
}
