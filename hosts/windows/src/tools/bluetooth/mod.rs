use std::collections::{BTreeMap, HashMap};
use std::mem::{size_of, zeroed};
use std::collections::VecDeque;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use operit_host_api::{
    BluetoothBleCharacteristicAddress, BluetoothBleCharacteristicData, BluetoothBleConnectRequest,
    BluetoothBleNotificationData, BluetoothBleNotificationEntry, BluetoothBleServiceData,
    BluetoothBleServicesData, BluetoothBleSubscribeRequest, BluetoothBleWriteAndReadRequest,
    BluetoothBleWriteRequest, BluetoothBondedDevicesData, BluetoothClassicAcceptRequest,
    BluetoothClassicConnectRequest, BluetoothClassicListenRequest, BluetoothDeviceData,
    BluetoothHost, BluetoothPayload, BluetoothReadData, BluetoothReadRequest, BluetoothScanRequest,
    BluetoothScanResultData, BluetoothScannedDeviceData, BluetoothSessionData, BluetoothStateData,
    BluetoothTransferData, HostError, HostResult,
};
use windows_sys::Win32::Devices::Bluetooth::{
    BluetoothFindDeviceClose, BluetoothFindFirstDevice, BluetoothFindFirstRadio,
    BluetoothFindNextDevice, BluetoothFindNextRadio, BluetoothFindRadioClose,
    BluetoothGetRadioInfo, AF_BTH, BTHPROTO_RFCOMM, BLUETOOTH_DEVICE_INFO,
    BLUETOOTH_DEVICE_SEARCH_PARAMS, BLUETOOTH_FIND_RADIO_PARAMS, BLUETOOTH_RADIO_INFO,
    SOCKADDR_BTH,
};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Networking::WinSock::{
    accept, bind, closesocket, connect, listen, recv, send, shutdown, socket, WSAGetLastError,
    WSAStartup, INVALID_SOCKET, SD_BOTH, SOCKADDR, SOCK_STREAM, SOCKET, WSADATA,
};
use uuid::Uuid;
use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothLEDevice};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattCharacteristicProperties,
    GattClientCharacteristicConfigurationDescriptorValue, GattCommunicationStatus,
    GattValueChangedEventArgs, GattWriteOption,
};
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::{DataReader, DataWriter, IBuffer};

#[derive(Clone, Debug)]
pub struct WindowsBluetoothHost {
    sessions: Arc<Mutex<HashMap<String, WindowsBluetoothSession>>>,
}

#[derive(Debug)]
enum WindowsBluetoothSession {
    Classic(WindowsClassicSession),
    Ble(WindowsBleSession),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowsClassicSession {
    socket: SOCKET,
    listener: bool,
}

#[derive(Debug)]
struct WindowsBleSession {
    address: String,
    device: BluetoothLEDevice,
    notifications: Arc<Mutex<VecDeque<BluetoothBleNotificationEntry>>>,
    subscriptions: HashMap<String, (GattCharacteristic, i64)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WindowsBluetoothDevice {
    address: String,
    name: Option<String>,
    connected: bool,
    remembered: bool,
    authenticated: bool,
}

impl WindowsBluetoothHost {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn lockSessions(
        &self,
    ) -> HostResult<std::sync::MutexGuard<'_, HashMap<String, WindowsBluetoothSession>>> {
        self.sessions
            .lock()
            .map_err(|error| HostError::new(format!("Windows Bluetooth session lock failed: {error}")))
    }
}

impl Default for WindowsBluetoothHost {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothHost for WindowsBluetoothHost {
    fn requestBluetoothPermission(&self) -> HostResult<String> {
        Ok("windows_bluetooth_permission_not_required".to_string())
    }

    fn bluetoothState(&self) -> HostResult<BluetoothStateData> {
        let radios = enumerate_radios();
        Ok(BluetoothStateData {
            supported: !radios.is_empty(),
            enabled: !radios.is_empty(),
            state: if radios.is_empty() { "unavailable" } else { "enabled" }.to_string(),
        })
    }

    fn requestEnableBluetooth(&self) -> HostResult<String> {
        Err(HostError::new("Windows Bluetooth enable must be changed in system settings"))
    }

    fn listBluetoothBondedDevices(&self) -> HostResult<BluetoothBondedDevicesData> {
        let devices = enumerate_devices(false)
            .into_values()
            .filter(|device| device.authenticated || device.remembered)
            .map(device_data)
            .collect();
        Ok(BluetoothBondedDevicesData { devices })
    }

    fn scanBluetoothDevices(
        &self,
        request: BluetoothScanRequest,
    ) -> HostResult<BluetoothScanResultData> {
        let durationMs = request.durationMs.max(0);
        let inquiry = durationMs > 0;
        let devices = enumerate_devices(inquiry)
            .into_values()
            .map(|device| BluetoothScannedDeviceData {
                name: device.name,
                address: device.address,
                r#type: "classic".to_string(),
                bondState: bond_state(device.authenticated, device.remembered),
                source: "windows.bluetooth".to_string(),
                rssi: None,
            })
            .collect();
        if durationMs > 0 {
            thread::sleep(Duration::from_millis(durationMs as u64));
        }
        Ok(BluetoothScanResultData {
            devices,
            durationMs,
            includesBle: false,
        })
    }

    fn bluetoothConnect(
        &self,
        request: BluetoothClassicConnectRequest,
    ) -> HostResult<BluetoothSessionData> {
        ensure_winsock()?;
        let address = bluetooth_address_value(&request.address)?;
        let serviceClassId = parse_uuid(&request.uuid)?;
        let socket = bluetooth_socket()?;
        let sockaddr = SOCKADDR_BTH {
            addressFamily: AF_BTH,
            btAddr: address,
            serviceClassId,
            port: 0,
        };
        let result = unsafe {
            connect(
                socket,
                &sockaddr as *const SOCKADDR_BTH as *const SOCKADDR,
                size_of::<SOCKADDR_BTH>() as i32,
            )
        };
        if result != 0 {
            let error = winsock_error("Windows Bluetooth classic connect failed");
            unsafe {
                closesocket(socket);
            }
            return Err(error);
        }
        let sessionId = format!("windows-bt-classic-{}", Uuid::new_v4());
        self.lockSessions()?.insert(
            sessionId.clone(),
            WindowsBluetoothSession::Classic(WindowsClassicSession {
                socket,
                listener: false,
            }),
        );
        Ok(BluetoothSessionData {
            sessionId,
            address: request.address,
            mode: "classic".to_string(),
        })
    }

    fn bluetoothListen(
        &self,
        request: BluetoothClassicListenRequest,
    ) -> HostResult<BluetoothSessionData> {
        ensure_winsock()?;
        let serviceClassId = parse_uuid(&request.uuid)?;
        let socket = bluetooth_socket()?;
        let sockaddr = SOCKADDR_BTH {
            addressFamily: AF_BTH,
            btAddr: 0,
            serviceClassId,
            port: 0,
        };
        let bindResult = unsafe {
            bind(
                socket,
                &sockaddr as *const SOCKADDR_BTH as *const SOCKADDR,
                size_of::<SOCKADDR_BTH>() as i32,
            )
        };
        if bindResult != 0 {
            let error = winsock_error("Windows Bluetooth classic bind failed");
            unsafe {
                closesocket(socket);
            }
            return Err(error);
        }
        if unsafe { listen(socket, 1) } != 0 {
            let error = winsock_error("Windows Bluetooth classic listen failed");
            unsafe {
                closesocket(socket);
            }
            return Err(error);
        }
        let sessionId = format!("windows-bt-listener-{}", Uuid::new_v4());
        self.lockSessions()?.insert(
            sessionId.clone(),
            WindowsBluetoothSession::Classic(WindowsClassicSession {
                socket,
                listener: true,
            }),
        );
        Ok(BluetoothSessionData {
            sessionId,
            address: request.name,
            mode: "classic_listener".to_string(),
        })
    }

    fn bluetoothAccept(
        &self,
        request: BluetoothClassicAcceptRequest,
    ) -> HostResult<BluetoothSessionData> {
        let listener = {
            let sessions = self.lockSessions()?;
            match sessions
                .get(&request.listenerSessionId)
                .ok_or_else(|| HostError::new(format!(
                    "Windows Bluetooth listener session is not available: {}",
                    request.listenerSessionId
                )))? {
                    WindowsBluetoothSession::Classic(session) => *session,
                    WindowsBluetoothSession::Ble(_) => {
                        return Err(HostError::new(format!(
                            "Windows Bluetooth session is not a listener: {}",
                            request.listenerSessionId
                        )))
                    }
                }
        };
        if !listener.listener {
            return Err(HostError::new(format!(
                "Windows Bluetooth session is not a listener: {}",
                request.listenerSessionId
            )));
        }
        let mut sockaddr: SOCKADDR_BTH = unsafe { zeroed() };
        let mut sockaddr_len = size_of::<SOCKADDR_BTH>() as i32;
        let socket = unsafe {
            accept(
                listener.socket,
                &mut sockaddr as *mut SOCKADDR_BTH as *mut SOCKADDR,
                &mut sockaddr_len,
            )
        };
        if socket == INVALID_SOCKET {
            return Err(winsock_error("Windows Bluetooth classic accept failed"));
        }
        let sessionId = format!("windows-bt-classic-{}", Uuid::new_v4());
        let address = bluetooth_address_string(sockaddr.btAddr);
        self.lockSessions()?.insert(
            sessionId.clone(),
            WindowsBluetoothSession::Classic(WindowsClassicSession {
                socket,
                listener: false,
            }),
        );
        Ok(BluetoothSessionData {
            sessionId,
            address,
            mode: "classic".to_string(),
        })
    }

    fn bluetoothSend(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
    ) -> HostResult<BluetoothTransferData> {
        let bytes = bluetooth_payload_bytes(payload)?;
        let session = self.classicSession(sessionId)?;
        let written = unsafe {
            send(
                session.socket,
                bytes.as_ptr(),
                i32::try_from(bytes.len()).map_err(|error| HostError::new(error.to_string()))?,
                0,
            )
        };
        if written < 0 {
            return Err(winsock_error("Windows Bluetooth classic send failed"));
        }
        Ok(BluetoothTransferData {
            sessionId: sessionId.to_string(),
            bytesWritten: i64::from(written),
        })
    }

    fn bluetoothRead(&self, request: BluetoothReadRequest) -> HostResult<BluetoothReadData> {
        let session = self.classicSession(&request.sessionId)?;
        let mut buffer = vec![
            0u8;
            usize::try_from(request.maxBytes.max(1))
                .map_err(|error| HostError::new(error.to_string()))?
        ];
        let read = unsafe {
            recv(
                session.socket,
                buffer.as_mut_ptr(),
                i32::try_from(buffer.len()).map_err(|error| HostError::new(error.to_string()))?,
                0,
            )
        };
        if read < 0 {
            return Err(winsock_error("Windows Bluetooth classic read failed"));
        }
        buffer.truncate(read as usize);
        Ok(bluetooth_read_data(request.sessionId, buffer))
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
        if let Some(session) = self.lockSessions()?.remove(sessionId) {
            match session {
                WindowsBluetoothSession::Classic(session) => unsafe {
                    shutdown(session.socket, SD_BOTH);
                    closesocket(session.socket);
                },
                WindowsBluetoothSession::Ble(mut session) => {
                    for (_, (characteristic, token)) in session.subscriptions.drain() {
                        let _ = characteristic.RemoveValueChanged(token);
                    }
                    let _ = session.device.Close();
                }
            }
        }
        Ok(format!("windows_bluetooth_session_closed:{sessionId}"))
    }

    fn bluetoothBleConnect(
        &self,
        request: BluetoothBleConnectRequest,
    ) -> HostResult<BluetoothSessionData> {
        let address = bluetooth_address_value(&request.address)?;
        let device = BluetoothLEDevice::FromBluetoothAddressAsync(address)
            .map_err(winrt_error("create Windows BLE connect operation"))?
            .join()
            .map_err(winrt_error("connect Windows BLE device"))?;
        let sessionId = format!("windows-ble-{}", Uuid::new_v4());
        self.lockSessions()?.insert(
            sessionId.clone(),
            WindowsBluetoothSession::Ble(WindowsBleSession {
                address: request.address.clone(),
                device,
                notifications: Arc::new(Mutex::new(VecDeque::new())),
                subscriptions: HashMap::new(),
            }),
        );
        Ok(BluetoothSessionData {
            sessionId,
            address: request.address,
            mode: "ble".to_string(),
        })
    }

    fn bluetoothBleDiscoverServices(
        &self,
        sessionId: &str,
        timeoutMs: i64,
    ) -> HostResult<BluetoothBleServicesData> {
        let session = self.bleSession(sessionId)?;
        let result = session
            .device
            .GetGattServicesWithCacheModeAsync(BluetoothCacheMode::Uncached)
            .map_err(winrt_error("create Windows BLE service discovery operation"))?
            .join()
            .map_err(winrt_error("discover Windows BLE services"))?;
        let status = result
            .Status()
            .map_err(winrt_error("read Windows BLE service discovery status"))?;
        ensure_gatt_success(status, "Windows BLE service discovery")?;
        let services = result
            .Services()
            .map_err(winrt_error("read Windows BLE service collection"))?;
        let mut output = Vec::new();
        for service in vector_view_items(&services)? {
            let serviceUuid = guid_string(service.Uuid().map_err(winrt_error("read Windows BLE service UUID"))?);
            let characteristics = service
                .GetCharacteristicsWithCacheModeAsync(BluetoothCacheMode::Uncached)
                .map_err(winrt_error("create Windows BLE characteristic discovery operation"))?
                .join()
                .map_err(winrt_error("discover Windows BLE characteristics"))?;
            ensure_gatt_success(
                characteristics
                    .Status()
                    .map_err(winrt_error("read Windows BLE characteristic discovery status"))?,
                "Windows BLE characteristic discovery",
            )?;
            let mut characteristicData = Vec::new();
            for characteristic in vector_view_items(
                &characteristics
                    .Characteristics()
                    .map_err(winrt_error("read Windows BLE characteristic collection"))?,
            )? {
                characteristicData.push(BluetoothBleCharacteristicData {
                    uuid: guid_string(
                        characteristic
                            .Uuid()
                            .map_err(winrt_error("read Windows BLE characteristic UUID"))?,
                    ),
                    properties: gatt_properties(
                        characteristic
                            .CharacteristicProperties()
                            .map_err(winrt_error("read Windows BLE characteristic properties"))?,
                    ),
                });
            }
            output.push(BluetoothBleServiceData {
                uuid: serviceUuid,
                characteristics: characteristicData,
            });
        }
        let _ = timeoutMs;
        Ok(BluetoothBleServicesData {
            sessionId: sessionId.to_string(),
            services: output,
        })
    }

    fn bluetoothBleReadCharacteristic(
        &self,
        address: BluetoothBleCharacteristicAddress,
    ) -> HostResult<BluetoothReadData> {
        let characteristic = self.bleCharacteristic(
            &address.sessionId,
            &address.serviceUuid,
            &address.characteristicUuid,
        )?;
        let result = characteristic
            .ReadValueWithCacheModeAsync(BluetoothCacheMode::Uncached)
            .map_err(winrt_error("create Windows BLE characteristic read operation"))?
            .join()
            .map_err(winrt_error("read Windows BLE characteristic"))?;
        ensure_gatt_success(
            result
                .Status()
                .map_err(winrt_error("read Windows BLE characteristic read status"))?,
            "Windows BLE characteristic read",
        )?;
        let buffer = result
            .Value()
            .map_err(winrt_error("read Windows BLE characteristic value"))?;
        Ok(bluetooth_read_data(address.sessionId, buffer_bytes(&buffer)?))
    }

    fn bluetoothBleWriteCharacteristic(
        &self,
        request: BluetoothBleWriteRequest,
    ) -> HostResult<BluetoothTransferData> {
        let characteristic = self.bleCharacteristic(
            &request.sessionId,
            &request.serviceUuid,
            &request.characteristicUuid,
        )?;
        let bytes = bluetooth_payload_bytes(BluetoothPayload {
            text: request.text,
            dataBase64: request.dataBase64,
        })?;
        let buffer = bytes_buffer(&bytes)?;
        let status = characteristic
            .WriteValueWithOptionAsync(&buffer, GattWriteOption::WriteWithResponse)
            .map_err(winrt_error("create Windows BLE characteristic write operation"))?
            .join()
            .map_err(winrt_error("write Windows BLE characteristic"))?;
        ensure_gatt_success(status, "Windows BLE characteristic write")?;
        Ok(BluetoothTransferData {
            sessionId: request.sessionId,
            bytesWritten: bytes.len() as i64,
        })
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
        let characteristic = self.bleCharacteristic(
            &request.sessionId,
            &request.serviceUuid,
            &request.characteristicUuid,
        )?;
        let key = ble_subscription_key(&request.serviceUuid, &request.characteristicUuid);
        let mut sessions = self.lockSessions()?;
        let session = match sessions.get_mut(&request.sessionId) {
            Some(WindowsBluetoothSession::Ble(session)) => session,
            Some(WindowsBluetoothSession::Classic(_)) => {
                return Err(HostError::new(format!(
                    "Windows Bluetooth session is not a BLE session: {}",
                    request.sessionId
                )))
            }
            None => {
                return Err(HostError::new(format!(
                    "Windows BLE session is not available: {}",
                    request.sessionId
                )))
            }
        };
        if request.enable {
            let queue = session.notifications.clone();
            let characteristicUuid = request.characteristicUuid.clone();
            let token = characteristic
                .ValueChanged(&TypedEventHandler::<GattCharacteristic, GattValueChangedEventArgs>::new(
                    move |_sender, args| {
                        if let Some(args) = args.as_ref() {
                            if let Ok(buffer) = args.CharacteristicValue() {
                                if let Ok(bytes) = buffer_bytes(&buffer) {
                                    if let Ok(mut notifications) = queue.lock() {
                                        notifications.push_back(ble_notification_entry(
                                            characteristicUuid.clone(),
                                            bytes,
                                        ));
                                    }
                                }
                            }
                        }
                        Ok(())
                    },
                ))
                .map_err(winrt_error("subscribe Windows BLE characteristic value event"))?;
            let status = characteristic
                .WriteClientCharacteristicConfigurationDescriptorAsync(
                    GattClientCharacteristicConfigurationDescriptorValue::Notify,
                )
                .map_err(winrt_error("create Windows BLE notification enable operation"))?
                .join()
                .map_err(winrt_error("enable Windows BLE notifications"))?;
            ensure_gatt_success(status, "Windows BLE notification enable")?;
            if let Some((oldCharacteristic, oldToken)) = session.subscriptions.insert(key, (characteristic, token)) {
                let _ = oldCharacteristic.RemoveValueChanged(oldToken);
            }
        } else if let Some((oldCharacteristic, oldToken)) = session.subscriptions.remove(&key) {
            let _ = oldCharacteristic.RemoveValueChanged(oldToken);
            let status = oldCharacteristic
                .WriteClientCharacteristicConfigurationDescriptorAsync(
                    GattClientCharacteristicConfigurationDescriptorValue::None,
                )
                .map_err(winrt_error("create Windows BLE notification disable operation"))?
                .join()
                .map_err(winrt_error("disable Windows BLE notifications"))?;
            ensure_gatt_success(status, "Windows BLE notification disable")?;
        }
        Ok(BluetoothTransferData {
            sessionId: request.sessionId,
            bytesWritten: 0,
        })
    }

    fn bluetoothBleReadNotifications(
        &self,
        sessionId: &str,
        limit: i64,
    ) -> HostResult<BluetoothBleNotificationData> {
        let mut sessions = self.lockSessions()?;
        let session = match sessions.get_mut(sessionId) {
            Some(WindowsBluetoothSession::Ble(session)) => session,
            Some(WindowsBluetoothSession::Classic(_)) => {
                return Err(HostError::new(format!(
                    "Windows Bluetooth session is not a BLE session: {sessionId}"
                )))
            }
            None => {
                return Err(HostError::new(format!(
                    "Windows BLE session is not available: {sessionId}"
                )))
            }
        };
        let mut queue = session
            .notifications
            .lock()
            .map_err(|error| HostError::new(format!("Windows BLE notification queue lock failed: {error}")))?;
        let count = usize::try_from(limit.max(0)).map_err(|error| HostError::new(error.to_string()))?;
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

impl WindowsBluetoothHost {
    fn classicSession(&self, sessionId: &str) -> HostResult<WindowsClassicSession> {
        match self.lockSessions()?.get(sessionId) {
            Some(WindowsBluetoothSession::Classic(session)) => {
                if session.listener {
                    Err(HostError::new(format!(
                        "Windows Bluetooth session is a listener: {sessionId}"
                    )))
                } else {
                    Ok(*session)
                }
            }
            Some(WindowsBluetoothSession::Ble(_)) => Err(HostError::new(format!(
                "Windows Bluetooth session is not a classic session: {sessionId}"
            ))),
            None => Err(HostError::new(format!(
                "Windows Bluetooth session is not available: {sessionId}"
            ))),
        }
    }

    fn bleSession(&self, sessionId: &str) -> HostResult<WindowsBleSession> {
        match self.lockSessions()?.get(sessionId) {
            Some(WindowsBluetoothSession::Ble(session)) => Ok(WindowsBleSession {
                address: session.address.clone(),
                device: session.device.clone(),
                notifications: session.notifications.clone(),
                subscriptions: HashMap::new(),
            }),
            Some(WindowsBluetoothSession::Classic(_)) => Err(HostError::new(format!(
                "Windows Bluetooth session is not a BLE session: {sessionId}"
            ))),
            None => Err(HostError::new(format!(
                "Windows BLE session is not available: {sessionId}"
            ))),
        }
    }

    fn bleCharacteristic(
        &self,
        sessionId: &str,
        serviceUuid: &str,
        characteristicUuid: &str,
    ) -> HostResult<GattCharacteristic> {
        let session = self.bleSession(sessionId)?;
        let services = session
            .device
            .GetGattServicesForUuidWithCacheModeAsync(winrt_guid(serviceUuid)?, BluetoothCacheMode::Uncached)
            .map_err(winrt_error("create Windows BLE service lookup operation"))?
            .join()
            .map_err(winrt_error("lookup Windows BLE service"))?;
        ensure_gatt_success(
            services
                .Status()
                .map_err(winrt_error("read Windows BLE service lookup status"))?,
            "Windows BLE service lookup",
        )?;
        let service = vector_view_first(
            &services
                .Services()
                .map_err(winrt_error("read Windows BLE matching services"))?,
            "Windows BLE service",
        )?;
        let characteristics = service
            .GetCharacteristicsForUuidWithCacheModeAsync(
                winrt_guid(characteristicUuid)?,
                BluetoothCacheMode::Uncached,
            )
            .map_err(winrt_error("create Windows BLE characteristic lookup operation"))?
            .join()
            .map_err(winrt_error("lookup Windows BLE characteristic"))?;
        ensure_gatt_success(
            characteristics
                .Status()
                .map_err(winrt_error("read Windows BLE characteristic lookup status"))?,
            "Windows BLE characteristic lookup",
        )?;
        vector_view_first(
            &characteristics
                .Characteristics()
                .map_err(winrt_error("read Windows BLE matching characteristics"))?,
            "Windows BLE characteristic",
        )
    }
}

fn enumerate_radios() -> Vec<String> {
    unsafe {
        let mut params = BLUETOOTH_FIND_RADIO_PARAMS {
            dwSize: size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
        };
        let mut radioHandle: HANDLE = null_mut();
        let findHandle = BluetoothFindFirstRadio(&mut params, &mut radioHandle);
        let mut radios = Vec::new();
        if findHandle.is_null() {
            return radios;
        }
        loop {
            let mut info: BLUETOOTH_RADIO_INFO = zeroed();
            info.dwSize = size_of::<BLUETOOTH_RADIO_INFO>() as u32;
            if BluetoothGetRadioInfo(radioHandle, &mut info) == 0 {
                radios.push(bluetooth_address_string(info.address.Anonymous.ullLong));
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

fn enumerate_devices(issueInquiry: bool) -> BTreeMap<String, WindowsBluetoothDevice> {
    unsafe {
        let mut params: BLUETOOTH_DEVICE_SEARCH_PARAMS = zeroed();
        params.dwSize = size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32;
        params.fReturnAuthenticated = 1;
        params.fReturnRemembered = 1;
        params.fReturnUnknown = 1;
        params.fReturnConnected = 1;
        params.fIssueInquiry = if issueInquiry { 1 } else { 0 };
        params.cTimeoutMultiplier = if issueInquiry { 2 } else { 0 };
        params.hRadio = null_mut();

        let mut info: BLUETOOTH_DEVICE_INFO = zeroed();
        info.dwSize = size_of::<BLUETOOTH_DEVICE_INFO>() as u32;
        let handle = BluetoothFindFirstDevice(&params, &mut info);
        let mut devices = BTreeMap::new();
        if handle.is_null() {
            return devices;
        }
        loop {
            let device = bluetooth_device(&info);
            devices.insert(device.address.clone(), device);
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

unsafe fn bluetooth_device(info: &BLUETOOTH_DEVICE_INFO) -> WindowsBluetoothDevice {
    let name = wide_string(&info.szName);
    WindowsBluetoothDevice {
        address: bluetooth_address_string(info.Address.Anonymous.ullLong),
        name: if name.is_empty() { None } else { Some(name) },
        connected: info.fConnected != 0,
        remembered: info.fRemembered != 0,
        authenticated: info.fAuthenticated != 0,
    }
}

fn ensure_winsock() -> HostResult<()> {
    static START: OnceLock<HostResult<()>> = OnceLock::new();
    START
        .get_or_init(|| {
            let mut data: WSADATA = unsafe { zeroed() };
            let result = unsafe { WSAStartup(0x0202, &mut data) };
            if result == 0 {
                Ok(())
            } else {
                Err(HostError::new(format!("Windows WSAStartup failed: {result}")))
            }
        })
        .clone()
}

fn bluetooth_socket() -> HostResult<SOCKET> {
    let socket = unsafe { socket(i32::from(AF_BTH), SOCK_STREAM, BTHPROTO_RFCOMM as i32) };
    if socket == INVALID_SOCKET {
        Err(winsock_error("Windows Bluetooth socket creation failed"))
    } else {
        Ok(socket)
    }
}

fn winsock_error(context: &str) -> HostError {
    let code = unsafe { WSAGetLastError() };
    HostError::new(format!("{context}: WSA error {code}"))
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

fn bluetooth_read_data(sessionId: String, bytes: Vec<u8>) -> BluetoothReadData {
    let text = String::from_utf8(bytes.clone()).ok();
    BluetoothReadData {
        sessionId,
        bytesRead: bytes.len() as i64,
        text,
        dataBase64: Some(BASE64_STANDARD.encode(bytes)),
    }
}

fn winrt_error(
    operation: &'static str,
) -> impl FnOnce(windows::core::Error) -> HostError {
    move |error| HostError::new(format!("{operation} failed: {error}"))
}

fn ensure_gatt_success(status: GattCommunicationStatus, operation: &str) -> HostResult<()> {
    if status == GattCommunicationStatus::Success {
        Ok(())
    } else {
        Err(HostError::new(format!("{operation} failed with status: {status:?}")))
    }
}

fn winrt_guid(value: &str) -> HostResult<windows::core::GUID> {
    let uuid = Uuid::parse_str(value)
        .map_err(|error| HostError::new(format!("invalid BLE UUID: {error}")))?;
    Ok(windows::core::GUID::from_u128(uuid.as_u128()))
}

fn guid_string(value: windows::core::GUID) -> String {
    Uuid::from_u128(value.to_u128()).to_string()
}

fn vector_view_items<T>(view: &windows_collections::IVectorView<T>) -> HostResult<Vec<T>>
where
    T: windows::core::RuntimeType + Clone + 'static,
{
    let size = view
        .Size()
        .map_err(|error| HostError::new(format!("read Windows collection size failed: {error}")))?;
    let mut items = Vec::with_capacity(size as usize);
    for index in 0..size {
        items.push(
            view.GetAt(index)
                .map_err(|error| HostError::new(format!("read Windows collection item failed: {error}")))?,
        );
    }
    Ok(items)
}

fn vector_view_first<T>(
    view: &windows_collections::IVectorView<T>,
    name: &str,
) -> HostResult<T>
where
    T: windows::core::RuntimeType + Clone + 'static,
{
    if view
        .Size()
        .map_err(|error| HostError::new(format!("read {name} collection size failed: {error}")))?
        == 0
    {
        return Err(HostError::new(format!("{name} is not available")));
    }
    view.GetAt(0)
        .map_err(|error| HostError::new(format!("read {name} failed: {error}")))
}

fn gatt_properties(value: GattCharacteristicProperties) -> Vec<String> {
    let mut properties = Vec::new();
    if value.contains(GattCharacteristicProperties::Read) {
        properties.push("read".to_string());
    }
    if value.contains(GattCharacteristicProperties::Write) {
        properties.push("write".to_string());
    }
    if value.contains(GattCharacteristicProperties::WriteWithoutResponse) {
        properties.push("write_without_response".to_string());
    }
    if value.contains(GattCharacteristicProperties::Notify) {
        properties.push("notify".to_string());
    }
    if value.contains(GattCharacteristicProperties::Indicate) {
        properties.push("indicate".to_string());
    }
    properties
}

fn bytes_buffer(bytes: &[u8]) -> HostResult<IBuffer> {
    let writer = DataWriter::new()
        .map_err(winrt_error("create Windows BLE data writer"))?;
    writer
        .WriteBytes(bytes)
        .map_err(winrt_error("write Windows BLE data bytes"))?;
    writer
        .DetachBuffer()
        .map_err(winrt_error("detach Windows BLE data buffer"))
}

fn buffer_bytes(buffer: &IBuffer) -> HostResult<Vec<u8>> {
    let reader = DataReader::FromBuffer(buffer)
        .map_err(winrt_error("create Windows BLE data reader"))?;
    let length = buffer
        .Length()
        .map_err(winrt_error("read Windows BLE buffer length"))?;
    let mut bytes = vec![0u8; length as usize];
    reader
        .ReadBytes(&mut bytes)
        .map_err(winrt_error("read Windows BLE buffer bytes"))?;
    Ok(bytes)
}

fn ble_subscription_key(serviceUuid: &str, characteristicUuid: &str) -> String {
    format!(
        "{}:{}",
        serviceUuid.trim().to_ascii_lowercase(),
        characteristicUuid.trim().to_ascii_lowercase()
    )
}

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

fn bluetooth_address_value(address: &str) -> HostResult<u64> {
    let mut value = 0u64;
    for part in address.split(':') {
        let byte = u8::from_str_radix(part, 16)
            .map_err(|error| HostError::new(format!("invalid Bluetooth address: {error}")))?;
        value = (value << 8) | u64::from(byte);
    }
    Ok(value)
}

fn parse_uuid(value: &str) -> HostResult<windows_sys::core::GUID> {
    let uuid = Uuid::parse_str(value)
        .map_err(|error| HostError::new(format!("invalid Bluetooth UUID: {error}")))?;
    Ok(windows_sys::core::GUID::from_u128(uuid.as_u128()))
}

fn device_data(device: WindowsBluetoothDevice) -> BluetoothDeviceData {
    BluetoothDeviceData {
        name: device.name,
        address: device.address,
        r#type: "classic".to_string(),
        bondState: bond_state(device.authenticated, device.remembered),
    }
}

fn bond_state(authenticated: bool, remembered: bool) -> String {
    if authenticated {
        "bonded".to_string()
    } else if remembered {
        "remembered".to_string()
    } else {
        "none".to_string()
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
