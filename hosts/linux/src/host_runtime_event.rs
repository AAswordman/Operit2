#![allow(non_snake_case)]

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::{
    logHostError, HostError, HostResult, HostRuntimeEventHost, HostRuntimeEventRegistration,
    HostRuntimeEventSink,
};
use serde_json::Value;
use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use zbus::MatchRule;

const TAG: &str = "LinuxHostRuntimeEvent";
const DBUS_PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
const DBUS_OBJECT_MANAGER_INTERFACE: &str = "org.freedesktop.DBus.ObjectManager";
const LOGIND_MANAGER_INTERFACE: &str = "org.freedesktop.login1.Manager";
const LOGIND_SESSION_INTERFACE: &str = "org.freedesktop.login1.Session";
const NETWORK_MANAGER_INTERFACE: &str = "org.freedesktop.NetworkManager";
const TIMEDATE_INTERFACE: &str = "org.freedesktop.timedate1";
const UPOWER_DAEMON_INTERFACE: &str = "org.freedesktop.UPower";
const UPOWER_DEVICE_INTERFACE: &str = "org.freedesktop.UPower.Device";
const BLUEZ_ADAPTER_INTERFACE: &str = "org.bluez.Adapter1";
const BLUEZ_DEVICE_INTERFACE: &str = "org.bluez.Device1";

#[derive(Clone, Debug, Default)]
pub struct LinuxHostRuntimeEventHost;

impl LinuxHostRuntimeEventHost {
    pub fn new() -> Self {
        Self
    }
}

pub struct LinuxHostRuntimeEventRegistration {
    running: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl HostRuntimeEventRegistration for LinuxHostRuntimeEventRegistration {}

impl Drop for LinuxHostRuntimeEventRegistration {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl HostRuntimeEventHost for LinuxHostRuntimeEventHost {
    fn startHostRuntimeEventStream(
        &self,
        sink: HostRuntimeEventSink,
    ) -> HostResult<Box<dyn HostRuntimeEventRegistration>> {
        let running = Arc::new(AtomicBool::new(true));
        let systemRunning = running.clone();
        let sessionRunning = running.clone();
        let systemSink = sink.clone();
        let sessionSink = sink.clone();
        let (systemSender, systemReceiver) = mpsc::channel::<Result<(), String>>();
        let (sessionSender, sessionReceiver) = mpsc::channel::<Result<(), String>>();
        let systemWorker = thread::Builder::new()
            .name("operit-linux-host-runtime-event-system".to_string())
            .spawn(move || run_system_dbus_event_loop(systemSink, systemRunning, systemSender))
            .map_err(|error| HostError::new(format!("spawn linux system event worker failed: {error}")))?;
        let sessionWorker = thread::Builder::new()
            .name("operit-linux-host-runtime-event-session".to_string())
            .spawn(move || run_session_dbus_event_loop(sessionSink, sessionRunning, sessionSender))
            .map_err(|error| HostError::new(format!("spawn linux session event worker failed: {error}")))?;
        systemReceiver
            .recv()
            .map_err(|error| HostError::new(format!("receive linux system event init result failed: {error}")))?
            .map_err(HostError::new)?;
        sessionReceiver
            .recv()
            .map_err(|error| HostError::new(format!("receive linux session event init result failed: {error}")))?
            .map_err(HostError::new)?;
        Ok(Box::new(LinuxHostRuntimeEventRegistration {
            running,
            workers: vec![systemWorker, sessionWorker],
        }))
    }
}

fn run_system_dbus_event_loop(
    sink: HostRuntimeEventSink,
    running: Arc<AtomicBool>,
    init: mpsc::Sender<Result<(), String>>,
) {
    let connection = match Connection::system() {
        Ok(connection) => connection,
        Err(error) => {
            let _ = init.send(Err(format!("connect system dbus failed: {error}")));
            return;
        }
    };
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .build();
    let mut iterator = match MessageIterator::for_match_rule(rule, &connection, Some(64)) {
        Ok(iterator) => iterator,
        Err(error) => {
            let _ = init.send(Err(format!("register system dbus signal match failed: {error}")));
            return;
        }
    };
    let _ = init.send(Ok(()));
    while running.load(Ordering::SeqCst) {
        let Some(message) = iterator.next() else {
            break;
        };
        match message {
            Ok(message) => {
                let path = message
                    .header()
                    .path()
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                let interface = message
                    .header()
                    .interface()
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                let member = message
                    .header()
                    .member()
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                handle_system_dbus_signal(&sink, &path, &interface, &member, &message);
            }
            Err(error) => {
                logHostError(TAG, &format!("process system dbus signal failed: {error}"));
                break;
            }
        }
    }
}

fn run_session_dbus_event_loop(
    sink: HostRuntimeEventSink,
    running: Arc<AtomicBool>,
    init: mpsc::Sender<Result<(), String>>,
) {
    let connection = match Connection::session() {
        Ok(connection) => connection,
        Err(error) => {
            let _ = init.send(Err(format!("connect session dbus failed: {error}")));
            return;
        }
    };
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .build();
    let mut iterator = match MessageIterator::for_match_rule(rule, &connection, Some(64)) {
        Ok(iterator) => iterator,
        Err(error) => {
            let _ = init.send(Err(format!("register session dbus signal match failed: {error}")));
            return;
        }
    };
    let _ = init.send(Ok(()));
    while running.load(Ordering::SeqCst) {
        let Some(message) = iterator.next() else {
            break;
        };
        match message {
            Ok(message) => {
                let path = message
                    .header()
                    .path()
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                let interface = message
                    .header()
                    .interface()
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                let member = message
                    .header()
                    .member()
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                handle_session_dbus_signal(&sink, &path, &interface, &member, &message);
            }
            Err(error) => {
                logHostError(TAG, &format!("process session dbus signal failed: {error}"));
                break;
            }
        }
    }
}

fn handle_system_dbus_signal(
    sink: &HostRuntimeEventSink,
    path: &str,
    signalInterface: &str,
    member: &str,
    message: &zbus::Message,
) {
    match (signalInterface, member) {
        (DBUS_PROPERTIES_INTERFACE, "PropertiesChanged") => {
            if let Ok((interface, changed, _invalidated)) = message.body().deserialize::<(
                String,
                HashMap<String, OwnedValue>,
                Vec<String>,
            )>() {
                handle_properties_changed(sink, path, &interface, &changed);
            }
        }
        (DBUS_OBJECT_MANAGER_INTERFACE, "InterfacesAdded") => {
            if let Ok((objectPath, interfaces)) = message.body().deserialize::<(
                OwnedObjectPath,
                HashMap<String, HashMap<String, OwnedValue>>,
            )>() {
                handle_interfaces_added(sink, &objectPath.to_string(), &interfaces);
            }
        }
        (DBUS_OBJECT_MANAGER_INTERFACE, "InterfacesRemoved") => {
            if let Ok((objectPath, interfaces)) = message
                .body()
                .deserialize::<(OwnedObjectPath, Vec<String>)>()
            {
                handle_interfaces_removed(sink, &objectPath.to_string(), &interfaces);
            }
        }
        (LOGIND_MANAGER_INTERFACE, "PrepareForSleep") => {
            if let Ok((sleeping,)) = message.body().deserialize::<(bool,)>() {
                handle_prepare_for_sleep(sink, path, sleeping);
            }
        }
        _ => {}
    }
}

fn handle_session_dbus_signal(
    sink: &HostRuntimeEventSink,
    path: &str,
    signalInterface: &str,
    member: &str,
    message: &zbus::Message,
) {
    match (signalInterface, member) {
        ("org.freedesktop.ScreenSaver", "ActiveChanged")
        | ("org.gnome.ScreenSaver", "ActiveChanged") => {
            if let Ok((active,)) = message.body().deserialize::<(bool,)>() {
                handle_screensaver_active_changed(sink, path, signalInterface, active);
            }
        }
        _ => {}
    }
}

fn handle_properties_changed(
    sink: &HostRuntimeEventSink,
    path: &str,
    interface: &str,
    changed: &HashMap<String, OwnedValue>,
) {
    match interface {
        UPOWER_DAEMON_INTERFACE => handle_upower_daemon_changed(sink, path, changed),
        UPOWER_DEVICE_INTERFACE => handle_upower_device_changed(sink, path, changed),
        LOGIND_SESSION_INTERFACE => handle_logind_session_changed(sink, path, changed),
        NETWORK_MANAGER_INTERFACE => handle_network_manager_changed(sink, path, changed),
        TIMEDATE_INTERFACE => handle_timedate_changed(sink, path, changed),
        BLUEZ_ADAPTER_INTERFACE => handle_bluez_adapter_changed(sink, path, changed),
        BLUEZ_DEVICE_INTERFACE => handle_bluez_device_changed(sink, path, changed),
        _ => {}
    }
}

fn handle_upower_daemon_changed(
    sink: &HostRuntimeEventSink,
    path: &str,
    changed: &HashMap<String, OwnedValue>,
) {
    if let Some(onBattery) = prop_bool(changed, "OnBattery") {
        let topic = match onBattery {
            true => "system.power.disconnected",
            false => "system.power.connected",
        };
        emit(sink, topic, serde_json::json!({
            "connected": !onBattery,
            "source": if onBattery { "battery" } else { "ac" },
            "batteryLevel": prop_f64(changed, "Percentage"),
        }));
    }
}

fn handle_upower_device_changed(
    sink: &HostRuntimeEventSink,
    path: &str,
    changed: &HashMap<String, OwnedValue>,
) {
    if let Some(warningLevel) = prop_u64(changed, "WarningLevel") {
        let topic = match warningLevel {
            3 | 4 | 5 => "system.battery.low",
            1 | 2 => "system.battery.okay",
            _ => return,
        };
        let low = matches!(warningLevel, 3 | 4 | 5);
        emit(sink, topic, serde_json::json!({
            "low": low,
            "level": prop_f64(changed, "Percentage"),
            "charging": upower_charging(changed),
        }));
    }
}

fn handle_logind_session_changed(
    sink: &HostRuntimeEventSink,
    path: &str,
    changed: &HashMap<String, OwnedValue>,
) {
    if let Some(lockedHint) = prop_bool(changed, "LockedHint") {
        let payload = serde_json::json!({"locked": lockedHint});
        let topic = match lockedHint {
            true => "system.session.lock",
            false => "system.session.unlock",
        };
        emit(sink, topic, payload.clone());
        if !lockedHint {
            emit(sink, "system.user.present", serde_json::json!({"present": true}));
        }
    }
}

fn handle_network_manager_changed(
    sink: &HostRuntimeEventSink,
    path: &str,
    changed: &HashMap<String, OwnedValue>,
) {
    if let Some(state) = prop_u64(changed, "State") {
        emit(
            sink,
            "system.network.changed",
            serde_json::json!({
                "connected": state >= 50,
                "networkType": if state >= 50 { "other" } else { "none" },
                "metered": null,
                "interfaceName": null,
            }),
        );
    }
    let wirelessEnabled = prop_bool(changed, "WirelessEnabled");
    let wwanEnabled = prop_bool(changed, "WwanEnabled");
    if let (Some(wirelessEnabled), Some(wwanEnabled)) = (wirelessEnabled, wwanEnabled) {
        emit(
            sink,
            "system.airplane_mode.changed",
            serde_json::json!({
                "enabled": !wirelessEnabled && !wwanEnabled,
            }),
        );
    }
}

fn handle_screensaver_active_changed(
    sink: &HostRuntimeEventSink,
    path: &str,
    interface: &str,
    active: bool,
) {
    let topic = match active {
        true => "system.screen.off",
        false => "system.screen.on",
    };
    emit(
        sink,
        topic,
        serde_json::json!({
            "screenOn": !active,
        }),
    );
}

fn handle_timedate_changed(
    sink: &HostRuntimeEventSink,
    path: &str,
    changed: &HashMap<String, OwnedValue>,
) {
    if let Some(timezone) = prop_string(changed, "Timezone") {
        emit(
            sink,
            "system.timezone.changed",
            serde_json::json!({
                "timestampMillis": unix_millis(),
                "timezone": timezone,
            }),
        );
    }
    if let Some(ntp) = prop_bool(changed, "NTP") {
        emit(
            sink,
            "system.date.changed",
            serde_json::json!({
                "timestampMillis": unix_millis(),
                "timezone": null,
            }),
        );
    }
}

fn handle_prepare_for_sleep(sink: &HostRuntimeEventSink, path: &str, sleeping: bool) {
    let topic = match sleeping {
        true => "system.power.sleep",
        false => "system.power.wake",
    };
    emit(
        sink,
        topic,
        serde_json::json!({
            "sleeping": sleeping,
        }),
    );
}

fn handle_interfaces_added(
    sink: &HostRuntimeEventSink,
    path: &str,
    interfaces: &HashMap<String, HashMap<String, OwnedValue>>,
) {
    if let Some(properties) = interfaces.get(BLUEZ_DEVICE_INTERFACE) {
        emit(sink, "bluetooth.device.found", bluez_device_payload(path, properties));
    }
}

fn handle_interfaces_removed(sink: &HostRuntimeEventSink, path: &str, interfaces: &[String]) {
    if interfaces.iter().any(|interface| interface == BLUEZ_DEVICE_INTERFACE) {
        emit(
            sink,
            "bluetooth.device.disconnected",
            serde_json::json!({
                "deviceAddress": null,
                "deviceName": null,
                "connected": false,
                "bonded": null,
                "rssi": null,
            }),
        );
    }
}

fn handle_bluez_adapter_changed(
    sink: &HostRuntimeEventSink,
    path: &str,
    changed: &HashMap<String, OwnedValue>,
) {
    if let Some(powered) = prop_bool(changed, "Powered") {
        emit(
            sink,
            "bluetooth.adapter.powered_changed",
            serde_json::json!({
                "powered": powered,
                "connected": null,
            }),
        );
    }
}

fn handle_bluez_device_changed(
    sink: &HostRuntimeEventSink,
    path: &str,
    changed: &HashMap<String, OwnedValue>,
) {
    if let Some(connected) = prop_bool(changed, "Connected") {
        let topic = match connected {
            true => "bluetooth.device.connected",
            false => "bluetooth.device.disconnected",
        };
        emit(sink, topic, bluez_device_payload(path, changed));
    }
    if let Some(name) = prop_string(changed, "Name") {
        emit(
            sink,
            "bluetooth.device.name_changed",
            serde_json::json!({
                "deviceAddress": prop_string(changed, "Address"),
                "deviceName": name,
                "connected": prop_bool(changed, "Connected"),
                "bonded": prop_bool(changed, "Paired"),
                "rssi": null,
            }),
        );
    }
    if let Some(bonded) = prop_bool(changed, "Paired") {
        emit(
            sink,
            "bluetooth.device.bond_state_changed",
            serde_json::json!({
                "deviceAddress": prop_string(changed, "Address"),
                "deviceName": prop_string(changed, "Name"),
                "connected": prop_bool(changed, "Connected"),
                "bonded": bonded,
                "rssi": null,
            }),
        );
    }
}

fn emit(sink: &HostRuntimeEventSink, topic: &str, payload: Value) {
    sink(serde_json::json!({
        "domain": "host",
        "source": "linux.dbus",
        "topic": topic,
        "platform": "linux",
        "payload": payload,
        "occurredAtMillis": unix_millis(),
    }));
}

fn bluez_device_payload(path: &str, changed: &HashMap<String, OwnedValue>) -> Value {
    serde_json::json!({
        "deviceAddress": prop_string(changed, "Address"),
        "deviceName": prop_string(changed, "Name"),
        "connected": prop_bool(changed, "Connected"),
        "bonded": prop_bool(changed, "Paired"),
        "rssi": null,
    })
}

/// Maps UPower's device state into the shared optional charging field.
fn upower_charging(changed: &HashMap<String, OwnedValue>) -> Option<bool> {
    match prop_u64(changed, "State") {
        Some(1) | Some(4) => Some(true),
        Some(2) | Some(3) | Some(5) | Some(6) => Some(false),
        _ => None,
    }
}

fn prop_bool(properties: &HashMap<String, OwnedValue>, key: &str) -> Option<bool> {
    properties
        .get(key)
        .and_then(|value| bool::try_from(value).ok())
}

fn prop_u64(properties: &HashMap<String, OwnedValue>, key: &str) -> Option<u64> {
    let value = properties.get(key)?;
    u64::try_from(value)
        .ok()
        .or_else(|| u32::try_from(value).ok().map(u64::from))
        .or_else(|| i32::try_from(value).ok().map(|number| number as u64))
}

fn prop_f64(properties: &HashMap<String, OwnedValue>, key: &str) -> Option<f64> {
    properties
        .get(key)
        .and_then(|value| f64::try_from(value).ok())
}

fn prop_string(properties: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    properties
        .get(key)
        .and_then(|value| <&str>::try_from(value).ok())
        .map(str::to_string)
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after unix epoch")
        .as_millis() as u64
}
