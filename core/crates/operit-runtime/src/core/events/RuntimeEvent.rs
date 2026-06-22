#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    #[serde(rename = "web")]
    Web,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEventTopic {
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
    pub fn hostEventPayload(&self) -> Value {
        serde_json::json!({
            "domain": &self.domain,
            "source": &self.source,
            "topic": &self.topic,
            "platform": &self.platform,
            "data": &self.payload,
            "occurredAtMillis": self.occurredAtMillis,
        })
    }
}
