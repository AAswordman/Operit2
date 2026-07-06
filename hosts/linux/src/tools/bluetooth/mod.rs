use std::collections::{HashMap, VecDeque};
use std::process::Command;
use std::sync::{Arc, Mutex};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(target_os = "linux")]
use std::io::{Read, Write};
#[cfg(target_os = "linux")]
use std::mem::{size_of, zeroed};
#[cfg(target_os = "linux")]
use std::net::Shutdown;
#[cfg(target_os = "linux")]
use std::os::fd::{FromRawFd, RawFd};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use operit_host_api::{
    BluetoothBleCharacteristicAddress, BluetoothBleConnectRequest, BluetoothBleNotificationData,
    BluetoothBleNotificationEntry, BluetoothBleServicesData, BluetoothBleSubscribeRequest,
    BluetoothBleWriteAndReadRequest, BluetoothBleWriteRequest, BluetoothBondedDevicesData,
    BluetoothClassicAcceptRequest, BluetoothClassicConnectRequest, BluetoothClassicListenRequest,
    BluetoothDeviceData, BluetoothHost, BluetoothPayload, BluetoothReadData, BluetoothReadRequest,
    BluetoothScanRequest, BluetoothScanResultData, BluetoothScannedDeviceData,
    BluetoothSessionData, BluetoothStateData, BluetoothTransferData, HostError, HostResult,
};
use uuid::Uuid;
#[cfg(target_os = "linux")]
use zbus::blocking::{Connection, Proxy};
#[cfg(target_os = "linux")]
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

#[cfg(target_os = "linux")]
const BTPROTO_RFCOMM: libc::c_int = 3;
#[cfg(target_os = "linux")]
const BLUEZ_DESTINATION: &str = "org.bluez";
#[cfg(target_os = "linux")]
const DBUS_OBJECT_MANAGER_INTERFACE: &str = "org.freedesktop.DBus.ObjectManager";
#[cfg(target_os = "linux")]
const BLUEZ_DEVICE_INTERFACE: &str = "org.bluez.Device1";
#[cfg(target_os = "linux")]
const BLUEZ_GATT_SERVICE_INTERFACE: &str = "org.bluez.GattService1";
#[cfg(target_os = "linux")]
const BLUEZ_GATT_CHARACTERISTIC_INTERFACE: &str = "org.bluez.GattCharacteristic1";

#[cfg(target_os = "linux")]
#[repr(C)]
struct BdAddr {
    bytes: [u8; 6],
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct SockAddrRc {
    rc_family: libc::sa_family_t,
    rc_bdaddr: BdAddr,
    rc_channel: u8,
}

#[derive(Clone, Debug)]
pub struct LinuxBluetoothHost {
    sessions: Arc<Mutex<HashMap<String, LinuxBluetoothSession>>>,
}

#[derive(Debug)]
enum LinuxBluetoothSession {
    #[cfg(target_os = "linux")]
    Classic(std::net::TcpStream),
    #[cfg(target_os = "linux")]
    Listener(RawFd),
    #[cfg(target_os = "linux")]
    Ble(LinuxBleSession),
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct LinuxBleSession {
    address: String,
    devicePath: String,
    notifications: Arc<Mutex<VecDeque<BluetoothBleNotificationEntry>>>,
    subscriptions: Arc<Mutex<HashMap<String, LinuxBleSubscription>>>,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct LinuxBleSubscription {
    characteristicPath: String,
    stop: Arc<AtomicBool>,
}

impl LinuxBluetoothHost {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn lockSessions(
        &self,
    ) -> HostResult<std::sync::MutexGuard<'_, HashMap<String, LinuxBluetoothSession>>> {
        self.sessions
            .lock()
            .map_err(|error| HostError::new(format!("Linux Bluetooth session lock failed: {error}")))
    }
}

impl Default for LinuxBluetoothHost {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothHost for LinuxBluetoothHost {
    fn requestBluetoothPermission(&self) -> HostResult<String> {
        Ok("linux_bluetooth_permission_not_required".to_string())
    }

    fn bluetoothState(&self) -> HostResult<BluetoothStateData> {
        let output = bluetoothctl(&["show"])?;
        let supported = !output.contains("No default controller available");
        let enabled = output.lines().any(|line| line.trim() == "Powered: yes");
        Ok(BluetoothStateData {
            supported,
            enabled,
            state: if enabled {
                "enabled"
            } else if supported {
                "disabled"
            } else {
                "unavailable"
            }
            .to_string(),
        })
    }

    fn requestEnableBluetooth(&self) -> HostResult<String> {
        bluetoothctl(&["power", "on"])?;
        Ok("linux_bluetooth_power_on_requested".to_string())
    }

    fn listBluetoothBondedDevices(&self) -> HostResult<BluetoothBondedDevicesData> {
        let devices = bluetoothctl(&["paired-devices"])?
            .lines()
            .filter_map(parse_device_line)
            .map(|(address, name)| BluetoothDeviceData {
                name,
                address,
                r#type: "unknown".to_string(),
                bondState: "bonded".to_string(),
            })
            .collect();
        Ok(BluetoothBondedDevicesData { devices })
    }

    fn scanBluetoothDevices(
        &self,
        request: BluetoothScanRequest,
    ) -> HostResult<BluetoothScanResultData> {
        let durationMs = request.durationMs.max(0);
        bluetoothctl(&["scan", "on"])?;
        if durationMs > 0 {
            thread::sleep(Duration::from_millis(durationMs as u64));
        }
        bluetoothctl(&["scan", "off"])?;
        let devices = bluetoothctl(&["devices"])?
            .lines()
            .filter_map(parse_device_line)
            .map(|(address, name)| BluetoothScannedDeviceData {
                name,
                address,
                r#type: "unknown".to_string(),
                bondState: "unknown".to_string(),
                source: "linux.bluez".to_string(),
                rssi: None,
            })
            .collect();
        Ok(BluetoothScanResultData {
            devices,
            durationMs,
            includesBle: request.includeBle,
        })
    }

    fn bluetoothConnect(
        &self,
        request: BluetoothClassicConnectRequest,
    ) -> HostResult<BluetoothSessionData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = request;
            return Err(HostError::new("Linux Bluetooth classic connect requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let channel = rfcomm_channel(&request.uuid)?;
        let fd = rfcomm_socket()?;
        let sockaddr = rfcomm_sockaddr(&request.address, channel)?;
        let result = unsafe {
            libc::connect(
                fd,
                &sockaddr as *const SockAddrRc as *const libc::sockaddr,
                size_of::<SockAddrRc>() as libc::socklen_t,
            )
        };
        if result != 0 {
            let error = os_error("Linux Bluetooth classic connect failed");
            unsafe {
                libc::close(fd);
            }
            return Err(error);
        }
        let stream = unsafe { std::net::TcpStream::from_raw_fd(fd) };
        let sessionId = format!("linux-bt-classic-{}", Uuid::new_v4());
        self.lockSessions()?
            .insert(sessionId.clone(), LinuxBluetoothSession::Classic(stream));
        Ok(BluetoothSessionData {
            sessionId,
            address: request.address,
            mode: "classic".to_string(),
        })
        }
    }

    fn bluetoothListen(
        &self,
        request: BluetoothClassicListenRequest,
    ) -> HostResult<BluetoothSessionData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = request;
            return Err(HostError::new("Linux Bluetooth classic listen requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let channel = rfcomm_channel(&request.uuid)?;
        let fd = rfcomm_socket()?;
        let sockaddr = rfcomm_any_sockaddr(channel);
        let bind_result = unsafe {
            libc::bind(
                fd,
                &sockaddr as *const SockAddrRc as *const libc::sockaddr,
                size_of::<SockAddrRc>() as libc::socklen_t,
            )
        };
        if bind_result != 0 {
            let error = os_error("Linux Bluetooth classic bind failed");
            unsafe {
                libc::close(fd);
            }
            return Err(error);
        }
        if unsafe { libc::listen(fd, 1) } != 0 {
            let error = os_error("Linux Bluetooth classic listen failed");
            unsafe {
                libc::close(fd);
            }
            return Err(error);
        }
        let sessionId = format!("linux-bt-listener-{}", Uuid::new_v4());
        self.lockSessions()?
            .insert(sessionId.clone(), LinuxBluetoothSession::Listener(fd));
        Ok(BluetoothSessionData {
            sessionId,
            address: request.name,
            mode: "classic_listener".to_string(),
        })
        }
    }

    fn bluetoothAccept(
        &self,
        request: BluetoothClassicAcceptRequest,
    ) -> HostResult<BluetoothSessionData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = request;
            return Err(HostError::new("Linux Bluetooth classic accept requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let listener_fd = {
            let sessions = self.lockSessions()?;
            match sessions.get(&request.listenerSessionId) {
                Some(LinuxBluetoothSession::Listener(fd)) => *fd,
                Some(LinuxBluetoothSession::Classic(_)) => {
                    return Err(HostError::new(format!(
                        "Linux Bluetooth session is not a listener: {}",
                        request.listenerSessionId
                    )))
                }
                Some(LinuxBluetoothSession::Ble(_)) => {
                    return Err(HostError::new(format!(
                        "Linux Bluetooth session is not a listener: {}",
                        request.listenerSessionId
                    )))
                }
                None => {
                    return Err(HostError::new(format!(
                        "Linux Bluetooth listener session is not available: {}",
                        request.listenerSessionId
                    )))
                }
            }
        };
        let mut sockaddr: SockAddrRc = unsafe { zeroed() };
        let mut sockaddr_len = size_of::<SockAddrRc>() as libc::socklen_t;
        let fd = unsafe {
            libc::accept(
                listener_fd,
                &mut sockaddr as *mut SockAddrRc as *mut libc::sockaddr,
                &mut sockaddr_len,
            )
        };
        if fd < 0 {
            return Err(os_error("Linux Bluetooth classic accept failed"));
        }
        let stream = unsafe { std::net::TcpStream::from_raw_fd(fd) };
        let sessionId = format!("linux-bt-classic-{}", Uuid::new_v4());
        let address = bluetooth_address_string(sockaddr.rc_bdaddr.bytes);
        self.lockSessions()?
            .insert(sessionId.clone(), LinuxBluetoothSession::Classic(stream));
        Ok(BluetoothSessionData {
            sessionId,
            address,
            mode: "classic".to_string(),
        })
        }
    }

    fn bluetoothSend(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
    ) -> HostResult<BluetoothTransferData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (sessionId, payload);
            return Err(HostError::new("Linux Bluetooth classic send requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let bytes = bluetooth_payload_bytes(payload)?;
        let mut sessions = self.lockSessions()?;
        let stream = classic_stream(&mut sessions, sessionId)?;
        stream.write_all(&bytes)?;
        Ok(BluetoothTransferData {
            sessionId: sessionId.to_string(),
            bytesWritten: bytes.len() as i64,
        })
        }
    }

    fn bluetoothRead(&self, request: BluetoothReadRequest) -> HostResult<BluetoothReadData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = request;
            return Err(HostError::new("Linux Bluetooth classic read requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let mut buffer = vec![
            0u8;
            usize::try_from(request.maxBytes.max(1))
                .map_err(|error| HostError::new(error.to_string()))?
        ];
        let mut sessions = self.lockSessions()?;
        let stream = classic_stream(&mut sessions, &request.sessionId)?;
        let read = stream.read(&mut buffer)?;
        buffer.truncate(read);
        Ok(bluetooth_read_data(request.sessionId, buffer))
        }
    }

    fn bluetoothSendAndRead(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
        read: BluetoothReadRequest,
    ) -> HostResult<BluetoothReadData> {
        self.bluetoothSend(sessionId, payload)?;
        self.bluetoothRead(read)
    }

    fn bluetoothClose(&self, sessionId: &str) -> HostResult<String> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = sessionId;
            return Ok(format!("linux_bluetooth_session_closed:{sessionId}"));
        }
        #[cfg(target_os = "linux")]
        {
        if let Some(session) = self.lockSessions()?.remove(sessionId) {
            match session {
                LinuxBluetoothSession::Classic(stream) => {
                    let _ = stream.shutdown(Shutdown::Both);
                }
                LinuxBluetoothSession::Listener(fd) => unsafe {
                    libc::close(fd);
                },
                LinuxBluetoothSession::Ble(session) => {
                    stop_linux_ble_session(&session)?;
                }
            }
        }
        Ok(format!("linux_bluetooth_session_closed:{sessionId}"))
        }
    }

    fn bluetoothBleConnect(
        &self,
        request: BluetoothBleConnectRequest,
    ) -> HostResult<BluetoothSessionData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = request;
            return Err(HostError::new("Linux BLE connect requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let connection = bluez_connection()?;
        let devicePath = bluez_device_path(&connection, &request.address)?;
        let device = bluez_proxy(&connection, &devicePath, BLUEZ_DEVICE_INTERFACE)?;
        let _: () = device.call("Connect", &())?;
        let sessionId = format!("linux-ble-{}", Uuid::new_v4());
        self.lockSessions()?.insert(
            sessionId.clone(),
            LinuxBluetoothSession::Ble(LinuxBleSession {
                address: request.address.clone(),
                devicePath,
                notifications: Arc::new(Mutex::new(VecDeque::new())),
                subscriptions: Arc::new(Mutex::new(HashMap::new())),
            }),
        );
        Ok(BluetoothSessionData {
            sessionId,
            address: request.address,
            mode: "ble".to_string(),
        })
        }
    }

    fn bluetoothBleDiscoverServices(
        &self,
        sessionId: &str,
        timeoutMs: i64,
    ) -> HostResult<BluetoothBleServicesData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (sessionId, timeoutMs);
            return Err(HostError::new("Linux BLE service discovery requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let session = self.bleSession(sessionId)?;
        let connection = bluez_connection()?;
        wait_for_bluez_services(&connection, &session.devicePath, timeoutMs)?;
        let services = bluez_ble_services(&connection, &session.devicePath)?;
        Ok(BluetoothBleServicesData {
            sessionId: sessionId.to_string(),
            services,
        })
        }
    }

    fn bluetoothBleReadCharacteristic(
        &self,
        address: BluetoothBleCharacteristicAddress,
    ) -> HostResult<BluetoothReadData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = address;
            return Err(HostError::new("Linux BLE characteristic read requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let session = self.bleSession(&address.sessionId)?;
        let connection = bluez_connection()?;
        let path = bluez_characteristic_path(
            &connection,
            &session.devicePath,
            &address.serviceUuid,
            &address.characteristicUuid,
        )?;
        let characteristic = bluez_proxy(&connection, &path, BLUEZ_GATT_CHARACTERISTIC_INTERFACE)?;
        let bytes: Vec<u8> = characteristic.call("ReadValue", &(HashMap::<String, Value<'_>>::new(),))?;
        Ok(bluetooth_read_data(address.sessionId, bytes))
        }
    }

    fn bluetoothBleWriteCharacteristic(
        &self,
        request: BluetoothBleWriteRequest,
    ) -> HostResult<BluetoothTransferData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = request;
            return Err(HostError::new("Linux BLE characteristic write requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let session = self.bleSession(&request.sessionId)?;
        let connection = bluez_connection()?;
        let path = bluez_characteristic_path(
            &connection,
            &session.devicePath,
            &request.serviceUuid,
            &request.characteristicUuid,
        )?;
        let bytes = bluetooth_payload_bytes(BluetoothPayload {
            text: request.text,
            dataBase64: request.dataBase64,
        })?;
        let characteristic = bluez_proxy(&connection, &path, BLUEZ_GATT_CHARACTERISTIC_INTERFACE)?;
        let _: () = characteristic.call(
            "WriteValue",
            &(bytes.clone(), HashMap::<String, Value<'_>>::new()),
        )?;
        Ok(BluetoothTransferData {
            sessionId: request.sessionId,
            bytesWritten: bytes.len() as i64,
        })
        }
    }

    fn bluetoothBleWriteAndReadCharacteristic(
        &self,
        request: BluetoothBleWriteAndReadRequest,
    ) -> HostResult<BluetoothReadData> {
        self.bluetoothBleWriteCharacteristic(BluetoothBleWriteRequest {
            sessionId: request.sessionId.clone(),
            serviceUuid: request.writeServiceUuid,
            characteristicUuid: request.writeCharacteristicUuid,
            text: request.text,
            dataBase64: request.dataBase64,
        })?;
        self.bluetoothBleReadCharacteristic(BluetoothBleCharacteristicAddress {
            sessionId: request.sessionId,
            serviceUuid: request.readServiceUuid,
            characteristicUuid: request.readCharacteristicUuid,
            timeoutMs: request.timeoutMs,
        })
    }

    fn bluetoothBleSubscribeCharacteristic(
        &self,
        request: BluetoothBleSubscribeRequest,
    ) -> HostResult<BluetoothTransferData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = request;
            return Err(HostError::new("Linux BLE characteristic subscribe requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let session = self.bleSession(&request.sessionId)?;
        let connection = bluez_connection()?;
        let path = bluez_characteristic_path(
            &connection,
            &session.devicePath,
            &request.serviceUuid,
            &request.characteristicUuid,
        )?;
        let characteristic = bluez_proxy(&connection, &path, BLUEZ_GATT_CHARACTERISTIC_INTERFACE)?;
        let subscriptionKey =
            linux_ble_subscription_key(&request.serviceUuid, &request.characteristicUuid);
        if request.enable {
            {
                let subscriptions = session.subscriptions.lock().map_err(|error| {
                    HostError::new(format!("Linux BLE subscription lock failed: {error}"))
                })?;
                if subscriptions.get(&subscriptionKey).is_some() {
                    return Err(HostError::new(format!(
                        "Linux BLE subscription is already active: {subscriptionKey}"
                    )));
                }
            }
            let _: () = characteristic.call("StartNotify", &())?;
            let stop = Arc::new(AtomicBool::new(false));
            let notifications = session.notifications.clone();
            let characteristicUuid = request.characteristicUuid.clone();
            let workerPath = path.clone();
            let workerStop = stop.clone();
            thread::spawn(move || {
                if let Err(error) = linux_ble_notification_worker(
                    workerPath,
                    characteristicUuid,
                    notifications,
                    workerStop,
                ) {
                    eprintln!("Linux BLE notification worker stopped: {error}");
                }
            });
            session
                .subscriptions
                .lock()
                .map_err(|error| {
                    HostError::new(format!("Linux BLE subscription lock failed: {error}"))
                })?
                .insert(
                    subscriptionKey,
                    LinuxBleSubscription {
                        characteristicPath: path,
                        stop,
                    },
                );
        } else {
            let subscription = session
                .subscriptions
                .lock()
                .map_err(|error| {
                    HostError::new(format!("Linux BLE subscription lock failed: {error}"))
                })?
                .remove(&subscriptionKey)
                .ok_or_else(|| {
                    HostError::new(format!(
                        "Linux BLE subscription is not active: {subscriptionKey}"
                    ))
                })?;
            subscription.stop.store(true, Ordering::SeqCst);
            let _: () = characteristic.call("StopNotify", &())?;
        }
        Ok(BluetoothTransferData {
            sessionId: request.sessionId,
            bytesWritten: 0,
        })
        }
    }

    fn bluetoothBleReadNotifications(
        &self,
        sessionId: &str,
        limit: i64,
    ) -> HostResult<BluetoothBleNotificationData> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (sessionId, limit);
            return Err(HostError::new("Linux BLE notification read requires a Linux target"));
        }
        #[cfg(target_os = "linux")]
        {
        let session = self.bleSession(sessionId)?;
        let count = usize::try_from(limit.max(0))
            .map_err(|error| HostError::new(error.to_string()))?;
        let mut queue = session.notifications.lock().map_err(|error| {
            HostError::new(format!("Linux BLE notification queue lock failed: {error}"))
        })?;
        let mut notifications = Vec::new();
        for _ in 0..count {
            let Some(entry) = queue.pop_front() else {
                break;
            };
            notifications.push(entry);
        }
        Ok(BluetoothBleNotificationData {
            sessionId: sessionId.to_string(),
            notifications,
        })
        }
    }
}

impl LinuxBluetoothHost {
    #[cfg(target_os = "linux")]
    fn bleSession(&self, sessionId: &str) -> HostResult<LinuxBleSession> {
        match self.lockSessions()?.get(sessionId) {
            Some(LinuxBluetoothSession::Ble(session)) => Ok(LinuxBleSession {
                address: session.address.clone(),
                devicePath: session.devicePath.clone(),
                notifications: session.notifications.clone(),
                subscriptions: session.subscriptions.clone(),
            }),
            Some(LinuxBluetoothSession::Classic(_)) | Some(LinuxBluetoothSession::Listener(_)) => {
                Err(HostError::new(format!(
                    "Linux Bluetooth session is not a BLE session: {sessionId}"
                )))
            }
            None => Err(HostError::new(format!(
                "Linux BLE session is not available: {sessionId}"
            ))),
        }
    }
}

#[cfg(target_os = "linux")]
fn rfcomm_socket() -> HostResult<RawFd> {
    let fd = unsafe { libc::socket(libc::AF_BLUETOOTH, libc::SOCK_STREAM, BTPROTO_RFCOMM) };
    if fd < 0 {
        Err(os_error("Linux Bluetooth socket creation failed"))
    } else {
        Ok(fd)
    }
}

#[cfg(target_os = "linux")]
fn rfcomm_channel(uuid: &str) -> HostResult<u8> {
    if uuid == "00001101-0000-1000-8000-00805f9b34fb" {
        return Ok(1);
    }
    let value = uuid
        .parse::<u8>()
        .map_err(|error| HostError::new(format!("Linux RFCOMM channel parse failed: {error}")))?;
    if value == 0 {
        return Err(HostError::new("Linux RFCOMM channel must be greater than 0"));
    }
    Ok(value)
}

#[cfg(target_os = "linux")]
fn rfcomm_sockaddr(address: &str, channel: u8) -> HostResult<SockAddrRc> {
    Ok(SockAddrRc {
        rc_family: libc::AF_BLUETOOTH as libc::sa_family_t,
        rc_bdaddr: BdAddr {
            bytes: bluetooth_address_bytes(address)?,
        },
        rc_channel: channel,
    })
}

#[cfg(target_os = "linux")]
fn rfcomm_any_sockaddr(channel: u8) -> SockAddrRc {
    SockAddrRc {
        rc_family: libc::AF_BLUETOOTH as libc::sa_family_t,
        rc_bdaddr: BdAddr { bytes: [0; 6] },
        rc_channel: channel,
    }
}

#[cfg(target_os = "linux")]
fn bluetooth_address_bytes(address: &str) -> HostResult<[u8; 6]> {
    let mut parsed = [0u8; 6];
    let mut count = 0usize;
    for part in address.split(':') {
        if count >= parsed.len() {
            return Err(HostError::new("invalid Bluetooth address segment count"));
        }
        parsed[5 - count] = u8::from_str_radix(part, 16)
            .map_err(|error| HostError::new(format!("invalid Bluetooth address: {error}")))?;
        count += 1;
    }
    if count != parsed.len() {
        return Err(HostError::new("invalid Bluetooth address segment count"));
    }
    Ok(parsed)
}

#[cfg(target_os = "linux")]
fn bluetooth_address_string(bytes: [u8; 6]) -> String {
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0]
    )
}

#[cfg(target_os = "linux")]
fn classic_stream<'a>(
    sessions: &'a mut HashMap<String, LinuxBluetoothSession>,
    sessionId: &str,
) -> HostResult<&'a mut std::net::TcpStream> {
    match sessions.get_mut(sessionId) {
        Some(LinuxBluetoothSession::Classic(stream)) => Ok(stream),
        Some(LinuxBluetoothSession::Listener(_)) => Err(HostError::new(format!(
            "Linux Bluetooth session is a listener: {sessionId}"
        ))),
        Some(LinuxBluetoothSession::Ble(_)) => Err(HostError::new(format!(
            "Linux Bluetooth session is not a classic session: {sessionId}"
        ))),
        None => Err(HostError::new(format!(
            "Linux Bluetooth session is not available: {sessionId}"
        ))),
    }
}

fn bluetooth_payload_bytes(payload: BluetoothPayload) -> HostResult<Vec<u8>> {
    match (payload.text, payload.dataBase64) {
        (Some(text), None) => Ok(text.into_bytes()),
        (None, Some(data)) => BASE64_STANDARD
            .decode(data)
            .map_err(|error| HostError::new(format!("Bluetooth base64 payload decode failed: {error}"))),
        (Some(_), Some(_)) => Err(HostError::new("Provide exactly one of text or dataBase64")),
        (None, None) => Err(HostError::new("Provide exactly one of text or dataBase64")),
    }
}

#[cfg(target_os = "linux")]
type BluezObjects =
    HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

#[cfg(target_os = "linux")]
fn bluez_connection() -> HostResult<Connection> {
    Connection::system()
        .map_err(|error| HostError::new(format!("connect Linux BlueZ D-Bus failed: {error}")))
}

#[cfg(target_os = "linux")]
fn bluez_proxy<'a>(
    connection: &'a Connection,
    path: &'a str,
    interface: &'a str,
) -> HostResult<Proxy<'a>> {
    Proxy::new(connection, BLUEZ_DESTINATION, path, interface)
        .map_err(|error| HostError::new(format!("create Linux BlueZ proxy failed: {error}")))
}

#[cfg(target_os = "linux")]
fn bluez_objects(connection: &Connection) -> HostResult<BluezObjects> {
    let manager = bluez_proxy(connection, "/", DBUS_OBJECT_MANAGER_INTERFACE)?;
    manager
        .call("GetManagedObjects", &())
        .map_err(|error| HostError::new(format!("Linux BlueZ GetManagedObjects failed: {error}")))
}

#[cfg(target_os = "linux")]
fn bluez_device_path(connection: &Connection, address: &str) -> HostResult<String> {
    let wanted = normalized_bluetooth_address(address)?;
    for (path, interfaces) in bluez_objects(connection)? {
        let Some(properties) = interfaces.get(BLUEZ_DEVICE_INTERFACE) else {
            continue;
        };
        let Some(deviceAddress) = owned_value_string(properties, "Address") else {
            continue;
        };
        if normalized_bluetooth_address(&deviceAddress)? == wanted {
            return Ok(path.to_string());
        }
    }
    Err(HostError::new(format!(
        "Linux BlueZ device is not available: {address}"
    )))
}

#[cfg(target_os = "linux")]
fn wait_for_bluez_services(
    connection: &Connection,
    devicePath: &str,
    timeoutMs: i64,
) -> HostResult<()> {
    let deadline = std::time::Instant::now()
        + Duration::from_millis(timeoutMs.max(0) as u64);
    loop {
        let device = bluez_proxy(connection, devicePath, BLUEZ_DEVICE_INTERFACE)?;
        let servicesResolved: bool = device
            .get_property("ServicesResolved")
            .map_err(|error| HostError::new(format!(
                "read Linux BlueZ ServicesResolved failed: {error}"
            )))?;
        if servicesResolved {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(HostError::new(format!(
                "Linux BlueZ services were not resolved for {devicePath}"
            )));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "linux")]
fn bluez_ble_services(
    connection: &Connection,
    devicePath: &str,
) -> HostResult<Vec<BluetoothBleServiceData>> {
    let objects = bluez_objects(connection)?;
    let mut services = Vec::new();
    for (servicePath, interfaces) in &objects {
        let servicePathText = servicePath.to_string();
        if !servicePathText.starts_with(devicePath) {
            continue;
        }
        let Some(serviceProperties) = interfaces.get(BLUEZ_GATT_SERVICE_INTERFACE) else {
            continue;
        };
        let Some(serviceUuid) = owned_value_string(serviceProperties, "UUID") else {
            continue;
        };
        let mut characteristics = Vec::new();
        for (characteristicPath, characteristicInterfaces) in &objects {
            let characteristicPathText = characteristicPath.to_string();
            if !characteristicPathText.starts_with(&servicePathText) {
                continue;
            }
            let Some(characteristicProperties) =
                characteristicInterfaces.get(BLUEZ_GATT_CHARACTERISTIC_INTERFACE)
            else {
                continue;
            };
            let Some(uuid) = owned_value_string(characteristicProperties, "UUID") else {
                continue;
            };
            characteristics.push(BluetoothBleCharacteristicData {
                uuid,
                properties: owned_value_string_array(characteristicProperties, "Flags"),
            });
        }
        services.push(BluetoothBleServiceData {
            uuid: serviceUuid,
            characteristics,
        });
    }
    Ok(services)
}

#[cfg(target_os = "linux")]
fn bluez_characteristic_path(
    connection: &Connection,
    devicePath: &str,
    serviceUuid: &str,
    characteristicUuid: &str,
) -> HostResult<String> {
    let wantedServiceUuid = normalized_uuid(serviceUuid);
    let wantedCharacteristicUuid = normalized_uuid(characteristicUuid);
    let objects = bluez_objects(connection)?;
    for (servicePath, interfaces) in &objects {
        let servicePathText = servicePath.to_string();
        if !servicePathText.starts_with(devicePath) {
            continue;
        }
        let Some(serviceProperties) = interfaces.get(BLUEZ_GATT_SERVICE_INTERFACE) else {
            continue;
        };
        let Some(currentServiceUuid) = owned_value_string(serviceProperties, "UUID") else {
            continue;
        };
        if normalized_uuid(&currentServiceUuid) != wantedServiceUuid {
            continue;
        }
        for (characteristicPath, characteristicInterfaces) in &objects {
            let characteristicPathText = characteristicPath.to_string();
            if !characteristicPathText.starts_with(&servicePathText) {
                continue;
            }
            let Some(characteristicProperties) =
                characteristicInterfaces.get(BLUEZ_GATT_CHARACTERISTIC_INTERFACE)
            else {
                continue;
            };
            let Some(currentCharacteristicUuid) =
                owned_value_string(characteristicProperties, "UUID")
            else {
                continue;
            };
            if normalized_uuid(&currentCharacteristicUuid) == wantedCharacteristicUuid {
                return Ok(characteristicPathText);
            }
        }
    }
    Err(HostError::new(format!(
        "Linux BlueZ characteristic is not available: {serviceUuid}/{characteristicUuid}"
    )))
}

#[cfg(target_os = "linux")]
fn owned_value_string(properties: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    properties
        .get(key)
        .and_then(|value| <&str>::try_from(value).ok())
        .map(str::to_string)
}

#[cfg(target_os = "linux")]
fn owned_value_string_array(properties: &HashMap<String, OwnedValue>, key: &str) -> Vec<String> {
    properties
        .get(key)
        .and_then(|value| Vec::<String>::try_from(value).ok())
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn normalized_bluetooth_address(address: &str) -> HostResult<String> {
    let bytes = bluetooth_address_bytes(address)?;
    Ok(bluetooth_address_string(bytes))
}

#[cfg(target_os = "linux")]
fn normalized_uuid(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn bluetooth_read_data(sessionId: String, bytes: Vec<u8>) -> BluetoothReadData {
    let text = String::from_utf8(bytes.clone()).ok();
    BluetoothReadData {
        sessionId,
        bytesRead: bytes.len() as i64,
        text,
        dataBase64: Some(BASE64_STANDARD.encode(bytes)),
    }
}

#[cfg(target_os = "linux")]
fn linux_ble_subscription_key(serviceUuid: &str, characteristicUuid: &str) -> String {
    format!(
        "{}:{}",
        normalized_uuid(serviceUuid),
        normalized_uuid(characteristicUuid)
    )
}

#[cfg(target_os = "linux")]
fn linux_ble_notification_worker(
    characteristicPath: String,
    characteristicUuid: String,
    notifications: Arc<Mutex<VecDeque<BluetoothBleNotificationEntry>>>,
    stop: Arc<AtomicBool>,
) -> HostResult<()> {
    let connection = bluez_connection()?;
    let characteristic =
        bluez_proxy(&connection, &characteristicPath, BLUEZ_GATT_CHARACTERISTIC_INTERFACE)?;
    let mut changes = characteristic.receive_property_changed::<Vec<u8>>("Value");
    while !stop.load(Ordering::SeqCst) {
        let Some(change) = changes.next() else {
            break;
        };
        let bytes = change.get().map_err(|error| {
            HostError::new(format!("read Linux BLE notification value failed: {error}"))
        })?;
        if stop.load(Ordering::SeqCst) {
            break;
        }
        notifications
            .lock()
            .map_err(|error| {
                HostError::new(format!(
                    "Linux BLE notification queue lock failed: {error}"
                ))
            })?
            .push_back(ble_notification_entry(characteristicUuid.clone(), bytes));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn stop_linux_ble_session(session: &LinuxBleSession) -> HostResult<()> {
    let subscriptions = session
        .subscriptions
        .lock()
        .map_err(|error| {
            HostError::new(format!("Linux BLE subscription lock failed: {error}"))
        })?
        .drain()
        .map(|(_, subscription)| subscription)
        .collect::<Vec<_>>();
    let connection = bluez_connection()?;
    for subscription in subscriptions {
        subscription.stop.store(true, Ordering::SeqCst);
        let characteristic = bluez_proxy(
            &connection,
            &subscription.characteristicPath,
            BLUEZ_GATT_CHARACTERISTIC_INTERFACE,
        )?;
        let _: () = characteristic.call("StopNotify", &())?;
    }
    let device = bluez_proxy(&connection, &session.devicePath, BLUEZ_DEVICE_INTERFACE)?;
    let _: () = device.call("Disconnect", &())?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn ble_notification_entry(
    characteristicUuid: String,
    bytes: Vec<u8>,
) -> BluetoothBleNotificationEntry {
    let text = String::from_utf8(bytes.clone()).ok();
    BluetoothBleNotificationEntry {
        characteristicUuid,
        bytesRead: bytes.len() as i64,
        text,
        dataBase64: Some(BASE64_STANDARD.encode(bytes)),
        timestamp: unix_millis() as i64,
    }
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after unix epoch")
        .as_millis() as u64
}

#[cfg(target_os = "linux")]
fn os_error(context: &str) -> HostError {
    HostError::new(format!("{context}: {}", std::io::Error::last_os_error()))
}

fn bluetoothctl(args: &[&str]) -> HostResult<String> {
    let output = Command::new("bluetoothctl")
        .args(args)
        .output()
        .map_err(|error| HostError::new(format!("linux bluetoothctl failed: {error}")))?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "linux bluetoothctl {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_device_line(line: &str) -> Option<(String, Option<String>)> {
    let line = line.trim();
    let rest = line.strip_prefix("Device ")?;
    let mut parts = rest.splitn(2, ' ');
    let address = parts.next()?.to_string();
    let name = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    Some((address, name))
}
