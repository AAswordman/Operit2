#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{addr_of_mut, null, null_mut};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::{
    HostError, HostResult, HostRuntimeEventHost, HostRuntimeEventRegistration, HostRuntimeEventSink,
};
use serde_json::Value;
use windows_sys::core::GUID;
use windows_sys::Win32::Devices::Bluetooth::{
    BluetoothFindDeviceClose, BluetoothFindFirstDevice, BluetoothFindFirstRadio,
    BluetoothFindNextDevice, BluetoothFindNextRadio, BluetoothFindRadioClose,
    BluetoothGetRadioInfo, BluetoothIsConnectable, BluetoothIsDiscoverable,
    BLUETOOTH_DEVICE_INFO, BLUETOOTH_DEVICE_SEARCH_PARAMS, BLUETOOTH_FIND_RADIO_PARAMS,
    BLUETOOTH_RADIO_INFO,
    GUID_BLUETOOTHLE_DEVICE_INTERFACE, GUID_BLUETOOTH_GATT_SERVICE_DEVICE_INTERFACE,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    CancelMibChangeNotify2, NotifyIpInterfaceChange, MIB_IPINTERFACE_ROW,
    MIB_NOTIFICATION_TYPE,
};
use windows_sys::Win32::Networking::WinSock::AF_UNSPEC;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Power::{
    RegisterPowerSettingNotification, UnregisterPowerSettingNotification, HPOWERNOTIFY,
    POWERBROADCAST_SETTING,
};
use windows_sys::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, PostMessageW, RegisterClassW, RegisterDeviceNotificationW,
    SetWindowLongPtrW, UnregisterDeviceNotification,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, DBT_DEVICEARRIVAL, DBT_DEVICEREMOVECOMPLETE,
    DBT_DEVTYP_DEVICEINTERFACE, DEV_BROADCAST_DEVICEINTERFACE_W, DEV_BROADCAST_HDR,
    DEVICE_NOTIFY_WINDOW_HANDLE,
    GWLP_USERDATA, HWND_MESSAGE, MSG, WM_CLOSE, WM_DEVICECHANGE, WM_POWERBROADCAST,
    WM_TIMECHANGE, WM_WTSSESSION_CHANGE, WNDCLASSW, WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
};

const CLASS_NAME: &[u16] = &[
    'O' as u16, 'p' as u16, 'e' as u16, 'r' as u16, 'i' as u16, 't' as u16, 'H' as u16,
    'o' as u16, 's' as u16, 't' as u16, 'R' as u16, 'u' as u16, 'n' as u16, 't' as u16,
    'i' as u16, 'm' as u16, 'e' as u16, 'E' as u16, 'v' as u16, 'e' as u16, 'n' as u16,
    't' as u16, 0,
];

const PBT_APMSUSPEND: usize = 0x0004;
const PBT_APMRESUMESUSPEND: usize = 0x0007;
const PBT_APMRESUMEAUTOMATIC: usize = 0x0012;
const PBT_POWERSETTINGCHANGE: usize = 0x8013;
const GUID_ACDC_POWER_SOURCE: GUID = GUID {
    data1: 0x5d3e9a59,
    data2: 0xe9d5,
    data3: 0x4b00,
    data4: [0xa6, 0xbd, 0xff, 0x34, 0xff, 0x51, 0x65, 0x48],
};
const GUID_BATTERY_PERCENTAGE_REMAINING: GUID = GUID {
    data1: 0xa7ad8041,
    data2: 0xb45a,
    data3: 0x4cae,
    data4: [0x87, 0xa3, 0xee, 0xcb, 0xb4, 0x68, 0xa9, 0xe1],
};
const GUID_CONSOLE_DISPLAY_STATE: GUID = GUID {
    data1: 0x6fe69556,
    data2: 0x704a,
    data3: 0x47a0,
    data4: [0x8f, 0x24, 0xc2, 0x8d, 0x93, 0x6f, 0xda, 0x47],
};
const GUID_BTHPORT_DEVICE_INTERFACE: GUID = GUID {
    data1: 0x0850302a,
    data2: 0xb344,
    data3: 0x4fda,
    data4: [0x9b, 0xe9, 0x90, 0x57, 0x6b, 0x8d, 0x46, 0xf0],
};
const GUID_AUDIO_RENDER_INTERFACE: GUID = GUID {
    data1: 0xe6327cad,
    data2: 0xdcec,
    data3: 0x4949,
    data4: [0xae, 0x8a, 0x99, 0x1e, 0x97, 0x6a, 0x79, 0xd2],
};
const GUID_AUDIO_CAPTURE_INTERFACE: GUID = GUID {
    data1: 0x2eef81be,
    data2: 0x33fa,
    data3: 0x4800,
    data4: [0x96, 0x70, 0x1c, 0xd4, 0x74, 0x97, 0x2c, 0x3f],
};

#[derive(Clone, Debug, Default)]
pub struct WindowsHostRuntimeEventHost;

impl WindowsHostRuntimeEventHost {
    pub fn new() -> Self {
        Self
    }
}

pub struct WindowsHostRuntimeEventRegistration {
    hwnd: isize,
    worker: Option<JoinHandle<()>>,
}

impl HostRuntimeEventRegistration for WindowsHostRuntimeEventRegistration {}

impl Drop for WindowsHostRuntimeEventRegistration {
    fn drop(&mut self) {
        unsafe {
            PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl HostRuntimeEventHost for WindowsHostRuntimeEventHost {
    fn startHostRuntimeEventStream(
        &self,
        sink: HostRuntimeEventSink,
    ) -> HostResult<Box<dyn HostRuntimeEventRegistration>> {
        let (sender, receiver) = mpsc::channel::<Result<isize, String>>();
        let worker = thread::Builder::new()
            .name("operit-windows-host-runtime-event".to_string())
            .spawn(move || run_windows_event_loop(sink, sender))
            .map_err(|error| HostError::new(format!("spawn windows host event worker failed: {error}")))?;
        let hwnd = receiver
            .recv()
            .map_err(|error| HostError::new(format!("receive windows host event hwnd failed: {error}")))?
            .map_err(HostError::new)? as HWND;
        Ok(Box::new(WindowsHostRuntimeEventRegistration {
            hwnd: hwnd as isize,
            worker: Some(worker),
        }))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BluetoothDeviceSnapshot {
    address: String,
    name: String,
    connected: bool,
    remembered: bool,
    authenticated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BluetoothRadioSnapshot {
    address: String,
    name: String,
    connectable: bool,
    discoverable: bool,
    classOfDevice: u32,
    lmpSubversion: u16,
    manufacturer: u16,
}

struct WindowsEventState {
    sink: HostRuntimeEventSink,
    batteryLow: Option<bool>,
    bluetoothDevices: BTreeMap<String, BluetoothDeviceSnapshot>,
    bluetoothRadios: BTreeMap<String, BluetoothRadioSnapshot>,
    bluetoothAdapterConnected: bool,
    powerNotifications: Vec<HPOWERNOTIFY>,
    deviceNotifications: Vec<*mut c_void>,
    networkNotification: HANDLE,
}

fn run_windows_event_loop(sink: HostRuntimeEventSink, init: mpsc::Sender<Result<isize, String>>) {
    unsafe {
        let instance = GetModuleHandleW(null()) as HINSTANCE;
        let wndClass = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: CLASS_NAME.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&wndClass);
        let hwnd = CreateWindowExW(
            0,
            CLASS_NAME.as_ptr(),
            CLASS_NAME.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            instance,
            null(),
        );
        if hwnd.is_null() {
            let _ = init.send(Err(format!("create windows host event window failed: {}", GetLastError())));
            return;
        }
        let bluetoothDevices = enumerate_bluetooth_devices();
        let state = Box::new(WindowsEventState {
            sink,
            batteryLow: None,
            bluetoothAdapterConnected: bluetooth_devices_connected(&bluetoothDevices),
            bluetoothDevices,
            bluetoothRadios: enumerate_bluetooth_radios(),
            powerNotifications: Vec::new(),
            deviceNotifications: Vec::new(),
            networkNotification: null_mut(),
        });
        let statePtr = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, statePtr as isize);
        register_power_notifications(hwnd, &mut *statePtr);
        register_session_notification(hwnd);
        register_bluetooth_notification(hwnd, &mut *statePtr);
        register_audio_notifications(hwnd, &mut *statePtr);
        register_network_notification(&mut *statePtr);
        let _ = init.send(Ok(hwnd as isize));
        let mut message: MSG = zeroed();
        while GetMessageW(addr_of_mut!(message), null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        cleanup_windows_event_state(hwnd, statePtr);
    }
}

unsafe fn register_power_notifications(hwnd: HWND, state: &mut WindowsEventState) {
    for guid in [
        GUID_ACDC_POWER_SOURCE,
        GUID_BATTERY_PERCENTAGE_REMAINING,
        GUID_CONSOLE_DISPLAY_STATE,
    ] {
        let handle = RegisterPowerSettingNotification(hwnd as *mut c_void, &guid, DEVICE_NOTIFY_WINDOW_HANDLE);
        if handle != 0 {
            state.powerNotifications.push(handle);
        }
    }
}

unsafe fn register_session_notification(hwnd: HWND) {
    WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
}

unsafe fn register_network_notification(state: &mut WindowsEventState) {
    let status = NotifyIpInterfaceChange(
        AF_UNSPEC as u16,
        Some(network_change_callback),
        state as *mut WindowsEventState as *const c_void,
        0,
        &mut state.networkNotification,
    );
    if status != 0 {
        state.networkNotification = null_mut();
    }
}

unsafe extern "system" fn network_change_callback(
    callerContext: *const c_void,
    row: *const MIB_IPINTERFACE_ROW,
    notificationType: MIB_NOTIFICATION_TYPE,
) {
    if callerContext.is_null() {
        return;
    }
    let state = &*(callerContext as *const WindowsEventState);
    let (interfaceIndex, family) = if row.is_null() {
        (0u32, 0u16)
    } else {
        ((*row).InterfaceIndex, (*row).Family)
    };
    emit(
        &state.sink,
        "system.network.changed",
        serde_json::json!({
            "notificationType": notificationType,
            "interfaceIndex": interfaceIndex,
            "family": family,
        }),
    );
}

unsafe fn register_bluetooth_notification(hwnd: HWND, state: &mut WindowsEventState) {
    register_device_notification(hwnd, state, GUID_BTHPORT_DEVICE_INTERFACE);
    register_device_notification(hwnd, state, GUID_BLUETOOTHLE_DEVICE_INTERFACE);
    register_device_notification(hwnd, state, GUID_BLUETOOTH_GATT_SERVICE_DEVICE_INTERFACE);
}

unsafe fn register_audio_notifications(hwnd: HWND, state: &mut WindowsEventState) {
    register_device_notification(hwnd, state, GUID_AUDIO_RENDER_INTERFACE);
    register_device_notification(hwnd, state, GUID_AUDIO_CAPTURE_INTERFACE);
}

unsafe fn register_device_notification(hwnd: HWND, state: &mut WindowsEventState, classGuid: GUID) {
    let mut filter = DEV_BROADCAST_DEVICEINTERFACE_W {
        dbcc_size: size_of::<DEV_BROADCAST_DEVICEINTERFACE_W>() as u32,
        dbcc_devicetype: DBT_DEVTYP_DEVICEINTERFACE,
        dbcc_reserved: 0,
        dbcc_classguid: classGuid,
        dbcc_name: [0; 1],
    };
    let notification = RegisterDeviceNotificationW(
        hwnd as *mut c_void,
        &mut filter as *mut _ as *mut c_void,
        DEVICE_NOTIFY_WINDOW_HANDLE,
    );
    if !notification.is_null() {
        state.deviceNotifications.push(notification);
    }
}

unsafe fn cleanup_windows_event_state(hwnd: HWND, statePtr: *mut WindowsEventState) {
    if !statePtr.is_null() {
        let state = Box::from_raw(statePtr);
        for handle in state.powerNotifications {
            UnregisterPowerSettingNotification(handle);
        }
        for notification in state.deviceNotifications {
            if !notification.is_null() {
                UnregisterDeviceNotification(notification);
            }
        }
        if !state.networkNotification.is_null() {
            CancelMibChangeNotify2(state.networkNotification);
        }
        WTSUnRegisterSessionNotification(hwnd);
    }
    DestroyWindow(hwnd);
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_CLOSE {
        windows_sys::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
        return 0;
    }
    let statePtr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowsEventState;
    if !statePtr.is_null() {
        let state = &mut *statePtr;
        match message {
            WM_POWERBROADCAST => {
                handle_power_broadcast(state, wparam, lparam);
                return 1;
            }
            WM_WTSSESSION_CHANGE => {
                handle_session_change(state, wparam);
                return 0;
            }
            WM_TIMECHANGE => {
                handle_time_change(state);
                return 0;
            }
            WM_DEVICECHANGE => {
                handle_device_change(state, wparam, lparam);
                return 0;
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

unsafe fn handle_power_broadcast(state: &mut WindowsEventState, wparam: WPARAM, lparam: LPARAM) {
    match wparam {
        PBT_APMSUSPEND => {
            emit(&state.sink, "system.power.sleep", serde_json::json!({}));
            return;
        }
        PBT_APMRESUMESUSPEND | PBT_APMRESUMEAUTOMATIC => {
            emit(&state.sink, "system.power.wake", serde_json::json!({}));
            return;
        }
        PBT_POWERSETTINGCHANGE => {}
        _ => return,
    }
    if lparam == 0 {
        return;
    }
    let setting = &*(lparam as *const POWERBROADCAST_SETTING);
    let data = power_setting_u32(setting);
    if guid_eq(setting.PowerSetting, GUID_ACDC_POWER_SOURCE) {
        match data {
            Some(0) => emit(&state.sink, "system.power.connected", serde_json::json!({"source": "ac"})),
            Some(1) | Some(2) => emit(&state.sink, "system.power.disconnected", serde_json::json!({"source": "battery"})),
            _ => {}
        }
    }
    if guid_eq(setting.PowerSetting, GUID_BATTERY_PERCENTAGE_REMAINING) {
        if let Some(percentage) = data {
            let isLow = percentage <= 20;
            if state.batteryLow != Some(isLow) {
                state.batteryLow = Some(isLow);
                let topic = match isLow {
                    true => "system.battery.low",
                    false => "system.battery.okay",
                };
                emit(&state.sink, topic, serde_json::json!({"percentage": percentage}));
            }
        }
    }
    if guid_eq(setting.PowerSetting, GUID_CONSOLE_DISPLAY_STATE) {
        match data {
            Some(0) => emit(&state.sink, "system.screen.off", serde_json::json!({"state": "off"})),
            Some(1) => emit(&state.sink, "system.screen.on", serde_json::json!({"state": "on"})),
            _ => {}
        }
    }
}

fn guid_eq(left: GUID, right: GUID) -> bool {
    left.data1 == right.data1
        && left.data2 == right.data2
        && left.data3 == right.data3
        && left.data4 == right.data4
}

unsafe fn power_setting_u32(setting: &POWERBROADCAST_SETTING) -> Option<u32> {
    if setting.DataLength < size_of::<u32>() as u32 {
        return None;
    }
    let ptr = setting.Data.as_ptr() as *const u32;
    Some(ptr.read_unaligned())
}

fn handle_session_change(state: &mut WindowsEventState, wparam: WPARAM) {
    match wparam as u32 {
        WTS_SESSION_LOCK => emit(&state.sink, "system.session.lock", serde_json::json!({})),
        WTS_SESSION_UNLOCK => {
            emit(&state.sink, "system.session.unlock", serde_json::json!({}));
            emit(&state.sink, "system.user.present", serde_json::json!({}));
        }
        _ => {}
    }
}

fn handle_time_change(state: &mut WindowsEventState) {
    emit(&state.sink, "system.date.changed", serde_json::json!({}));
    emit(&state.sink, "system.timezone.changed", serde_json::json!({}));
}

fn handle_device_change(state: &mut WindowsEventState, wparam: WPARAM, lparam: LPARAM) {
    let (deviceInterface, devicePath) = match unsafe { device_interface_path(lparam) } {
        Some(device) => device,
        None => return,
    };
    match wparam as u32 {
        DBT_DEVICEARRIVAL => match deviceInterface {
            "bluetoothle" | "bluetooth_gatt" => sync_bluetooth_device_snapshots(state),
            "bthport" => {
                sync_bluetooth_radio_snapshots(state);
                sync_bluetooth_device_snapshots(state);
            }
            "audio" => emit(
                &state.sink,
                "system.headset.plug",
                serde_json::json!({
                    "deviceInterface": deviceInterface,
                    "devicePath": devicePath,
                    "arrived": true,
                }),
            ),
            _ => {}
        },
        DBT_DEVICEREMOVECOMPLETE => match deviceInterface {
            "bluetoothle" | "bluetooth_gatt" => sync_bluetooth_device_snapshots(state),
            "bthport" => {
                sync_bluetooth_radio_snapshots(state);
                sync_bluetooth_device_snapshots(state);
            }
            "audio" => emit(
                &state.sink,
                "system.headset.plug",
                serde_json::json!({
                    "deviceInterface": deviceInterface,
                    "devicePath": devicePath,
                    "arrived": false,
                }),
            ),
            _ => {}
        },
        _ => {}
    }
}

unsafe fn device_interface_path(lparam: LPARAM) -> Option<(&'static str, String)> {
    if lparam == 0 {
        return None;
    }
    let header = &*(lparam as *const DEV_BROADCAST_HDR);
    if header.dbch_devicetype != DBT_DEVTYP_DEVICEINTERFACE {
        return None;
    }
    let device = &*(lparam as *const DEV_BROADCAST_DEVICEINTERFACE_W);
    let deviceInterface = if guid_eq(device.dbcc_classguid, GUID_BTHPORT_DEVICE_INTERFACE) {
        "bthport"
    } else if guid_eq(device.dbcc_classguid, GUID_BLUETOOTHLE_DEVICE_INTERFACE) {
        "bluetoothle"
    } else if guid_eq(device.dbcc_classguid, GUID_BLUETOOTH_GATT_SERVICE_DEVICE_INTERFACE) {
        "bluetooth_gatt"
    } else if guid_eq(device.dbcc_classguid, GUID_AUDIO_RENDER_INTERFACE)
        || guid_eq(device.dbcc_classguid, GUID_AUDIO_CAPTURE_INTERFACE)
    {
        "audio"
    } else {
        return None;
    };
    let namePtr = device.dbcc_name.as_ptr();
    let maxNameUnits = ((device.dbcc_size as usize).saturating_sub(size_of::<DEV_BROADCAST_DEVICEINTERFACE_W>()) / 2) + 1;
    let mut nameUnits = 0usize;
    while nameUnits < maxNameUnits && *namePtr.add(nameUnits) != 0 {
        nameUnits += 1;
    }
    Some((deviceInterface, String::from_utf16_lossy(std::slice::from_raw_parts(namePtr, nameUnits))))
}

fn sync_bluetooth_device_snapshots(state: &mut WindowsEventState) {
    let current = enumerate_bluetooth_devices();
    for (address, currentDevice) in current.iter() {
        match state.bluetoothDevices.get(address) {
            Some(previousDevice) => {
                if previousDevice.name != currentDevice.name {
                    emit(
                        &state.sink,
                        "bluetooth.device.name_changed",
                        bluetooth_device_payload(currentDevice),
                    );
                }
                if previousDevice.authenticated != currentDevice.authenticated {
                    emit(
                        &state.sink,
                        "bluetooth.device.bond_state_changed",
                        bluetooth_device_payload(currentDevice),
                    );
                }
                if previousDevice.connected != currentDevice.connected {
                    let topic = match currentDevice.connected {
                        true => "bluetooth.device.connected",
                        false => "bluetooth.device.disconnected",
                    };
                    emit(&state.sink, topic, bluetooth_device_payload(currentDevice));
                }
            }
            None => emit(
                &state.sink,
                "bluetooth.device.found",
                bluetooth_device_payload(currentDevice),
            ),
        }
    }
    for (address, previousDevice) in state.bluetoothDevices.iter() {
        if !current.contains_key(address) {
            emit(
                &state.sink,
                "bluetooth.device.disconnected",
                bluetooth_device_payload(previousDevice),
            );
        }
    }
    sync_bluetooth_adapter_connection_state(state, &current);
    state.bluetoothDevices = current;
}

fn sync_bluetooth_radio_snapshots(state: &mut WindowsEventState) {
    let current = enumerate_bluetooth_radios();
    for (address, currentRadio) in current.iter() {
        match state.bluetoothRadios.get(address) {
            Some(previousRadio) => {
                if previousRadio != currentRadio {
                    emit(
                        &state.sink,
                        "bluetooth.adapter.powered_changed",
                        bluetooth_radio_payload(currentRadio),
                    );
                }
            }
            None => emit(
                &state.sink,
                "bluetooth.adapter.powered_changed",
                bluetooth_radio_payload(currentRadio),
            ),
        }
    }
    for (address, previousRadio) in state.bluetoothRadios.iter() {
        if !current.contains_key(address) {
            let mut payload = bluetooth_radio_payload(previousRadio);
            if let Some(object) = payload.as_object_mut() {
                object.insert("connectable".to_string(), Value::Bool(false));
                object.insert("powered".to_string(), Value::Bool(false));
            }
            emit(&state.sink, "bluetooth.adapter.powered_changed", payload);
        }
    }
    state.bluetoothRadios = current;
}

fn sync_bluetooth_adapter_connection_state(
    state: &mut WindowsEventState,
    current: &BTreeMap<String, BluetoothDeviceSnapshot>,
) {
    let connected = bluetooth_devices_connected(current);
    if state.bluetoothAdapterConnected != connected {
        state.bluetoothAdapterConnected = connected;
        emit(
            &state.sink,
            "bluetooth.adapter.connection_state_changed",
            serde_json::json!({ "connected": connected }),
        );
    }
}

fn bluetooth_devices_connected(devices: &BTreeMap<String, BluetoothDeviceSnapshot>) -> bool {
    devices.values().any(|device| device.connected)
}

fn bluetooth_radio_payload(radio: &BluetoothRadioSnapshot) -> Value {
    serde_json::json!({
        "adapterAddress": radio.address,
        "adapterName": radio.name,
        "connectable": radio.connectable,
        "discoverable": radio.discoverable,
        "powered": true,
        "classOfDevice": radio.classOfDevice,
        "lmpSubversion": radio.lmpSubversion,
        "manufacturer": radio.manufacturer,
    })
}

fn bluetooth_device_payload(device: &BluetoothDeviceSnapshot) -> Value {
    serde_json::json!({
        "deviceAddress": device.address,
        "deviceName": device.name,
        "connected": device.connected,
        "remembered": device.remembered,
        "bonded": device.authenticated,
    })
}

fn enumerate_bluetooth_radios() -> BTreeMap<String, BluetoothRadioSnapshot> {
    unsafe {
        let mut params = BLUETOOTH_FIND_RADIO_PARAMS {
            dwSize: size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
        };
        let mut radioHandle: HANDLE = null_mut();
        let findHandle = BluetoothFindFirstRadio(&mut params, &mut radioHandle);
        let mut radios = BTreeMap::new();
        if findHandle.is_null() {
            return radios;
        }
        loop {
            let mut info: BLUETOOTH_RADIO_INFO = zeroed();
            info.dwSize = size_of::<BLUETOOTH_RADIO_INFO>() as u32;
            if BluetoothGetRadioInfo(radioHandle, &mut info) == 0 {
                let snapshot = bluetooth_radio_snapshot(radioHandle, &info);
                radios.insert(snapshot.address.clone(), snapshot);
            }
            CloseHandle(radioHandle);
            radioHandle = null_mut();
            if BluetoothFindNextRadio(findHandle, &mut radioHandle) == 0 {
                break;
            }
        }
        BluetoothFindRadioClose(findHandle);
        radios
    }
}

unsafe fn bluetooth_radio_snapshot(
    radioHandle: HANDLE,
    info: &BLUETOOTH_RADIO_INFO,
) -> BluetoothRadioSnapshot {
    BluetoothRadioSnapshot {
        address: bluetooth_address_string(info.address.Anonymous.ullLong),
        name: wide_string(&info.szName),
        connectable: BluetoothIsConnectable(radioHandle) != 0,
        discoverable: BluetoothIsDiscoverable(radioHandle) != 0,
        classOfDevice: info.ulClassofDevice,
        lmpSubversion: info.lmpSubversion,
        manufacturer: info.manufacturer,
    }
}

fn enumerate_bluetooth_devices() -> BTreeMap<String, BluetoothDeviceSnapshot> {
    unsafe {
        let mut params: BLUETOOTH_DEVICE_SEARCH_PARAMS = zeroed();
        params.dwSize = size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32;
        params.fReturnAuthenticated = 1;
        params.fReturnRemembered = 1;
        params.fReturnUnknown = 1;
        params.fReturnConnected = 1;
        params.fIssueInquiry = 0;
        params.cTimeoutMultiplier = 0;
        params.hRadio = null_mut();

        let mut info: BLUETOOTH_DEVICE_INFO = zeroed();
        info.dwSize = size_of::<BLUETOOTH_DEVICE_INFO>() as u32;
        let handle = BluetoothFindFirstDevice(&params, &mut info);
        let mut devices = BTreeMap::new();
        if handle.is_null() {
            return devices;
        }
        loop {
            let snapshot = bluetooth_device_snapshot(&info);
            devices.insert(snapshot.address.clone(), snapshot);
            info = zeroed();
            info.dwSize = size_of::<BLUETOOTH_DEVICE_INFO>() as u32;
            if BluetoothFindNextDevice(handle, &mut info) == 0 {
                break;
            }
        }
        BluetoothFindDeviceClose(handle);
        devices
    }
}

unsafe fn bluetooth_device_snapshot(info: &BLUETOOTH_DEVICE_INFO) -> BluetoothDeviceSnapshot {
    let address = bluetooth_address_string(info.Address.Anonymous.ullLong);
    BluetoothDeviceSnapshot {
        address,
        name: wide_string(&info.szName),
        connected: info.fConnected != 0,
        remembered: info.fRemembered != 0,
        authenticated: info.fAuthenticated != 0,
    }
}

fn bluetooth_address_string(address: u64) -> String {
    let bytes = address.to_be_bytes();
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
    )
}

fn wide_string(value: &[u16]) -> String {
    let end = value.iter().position(|unit| *unit == 0).unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end])
}

fn emit(sink: &HostRuntimeEventSink, topic: &str, payload: Value) {
    sink(serde_json::json!({
        "domain": "host",
        "source": "windows.system",
        "topic": topic,
        "platform": "windows",
        "payload": payload,
        "occurredAtMillis": unix_millis(),
    }));
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after unix epoch")
        .as_millis() as u64
}
