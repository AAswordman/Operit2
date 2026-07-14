use std::sync::Arc;

use operit_host_api::{
    BluetoothBleCharacteristicAddress, BluetoothBleConnectRequest, BluetoothBleNotificationData,
    BluetoothBleServicesData, BluetoothBleSubscribeRequest, BluetoothBleWriteAndReadRequest,
    BluetoothBleWriteRequest, BluetoothBondedDevicesData, BluetoothClassicAcceptRequest,
    BluetoothClassicConnectRequest, BluetoothClassicListenRequest, BluetoothHost, BluetoothPayload,
    BluetoothReadData, BluetoothReadRequest, BluetoothScanRequest, BluetoothScanResultData,
    BluetoothSessionData, BluetoothStateData, BluetoothTransferData, HostError, HostResult,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

pub type OhosBluetoothController = Arc<dyn Fn(&str, Value) -> HostResult<Value> + Send + Sync>;

#[derive(Clone)]
pub struct OhosBluetoothHost {
    controller: OhosBluetoothController,
}

impl OhosBluetoothHost {
    /// Creates an OpenHarmony Bluetooth host from a platform owner callback.
    pub fn fromController(controller: OhosBluetoothController) -> Self {
        Self { controller }
    }

    /// Executes one OpenHarmony Bluetooth command and decodes the typed result.
    fn execute<T>(&self, command: &str, params: Value) -> HostResult<T>
    where
        T: DeserializeOwned,
    {
        let value = (self.controller)(command, params)?;
        serde_json::from_value(value).map_err(|error| {
            HostError::new(format!(
                "OpenHarmony Bluetooth response decode failed: {error}"
            ))
        })
    }

    /// Encodes one Bluetooth request into JSON params.
    fn requestParams<T: serde::Serialize>(request: T) -> HostResult<Value> {
        serde_json::to_value(request).map_err(|error| {
            HostError::new(format!(
                "OpenHarmony Bluetooth request encode failed: {error}"
            ))
        })
    }
}

impl BluetoothHost for OhosBluetoothHost {
    /// Requests Bluetooth permission through the OpenHarmony owner app.
    fn requestBluetoothPermission(&self) -> HostResult<String> {
        self.execute("request_permission", json!({}))
    }

    /// Reads Bluetooth adapter state through the OpenHarmony owner app.
    fn bluetoothState(&self) -> HostResult<BluetoothStateData> {
        self.execute("state", json!({}))
    }

    /// Requests enabling Bluetooth through the OpenHarmony owner app.
    fn requestEnableBluetooth(&self) -> HostResult<String> {
        self.execute("request_enable", json!({}))
    }

    /// Lists bonded Bluetooth devices through the OpenHarmony owner app.
    fn listBluetoothBondedDevices(&self) -> HostResult<BluetoothBondedDevicesData> {
        self.execute("bonded_devices", json!({}))
    }

    /// Scans Bluetooth devices through the OpenHarmony owner app.
    fn scanBluetoothDevices(
        &self,
        request: BluetoothScanRequest,
    ) -> HostResult<BluetoothScanResultData> {
        self.execute("scan", Self::requestParams(request)?)
    }

    /// Connects to a classic Bluetooth device through the OpenHarmony owner app.
    fn bluetoothConnect(
        &self,
        request: BluetoothClassicConnectRequest,
    ) -> HostResult<BluetoothSessionData> {
        self.execute("classic_connect", Self::requestParams(request)?)
    }

    /// Starts a classic Bluetooth listener through the OpenHarmony owner app.
    fn bluetoothListen(
        &self,
        request: BluetoothClassicListenRequest,
    ) -> HostResult<BluetoothSessionData> {
        self.execute("classic_listen", Self::requestParams(request)?)
    }

    /// Accepts a classic Bluetooth connection through the OpenHarmony owner app.
    fn bluetoothAccept(
        &self,
        request: BluetoothClassicAcceptRequest,
    ) -> HostResult<BluetoothSessionData> {
        self.execute("classic_accept", Self::requestParams(request)?)
    }

    /// Sends classic Bluetooth payload through the OpenHarmony owner app.
    fn bluetoothSend(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
    ) -> HostResult<BluetoothTransferData> {
        self.execute(
            "classic_send",
            json!({
                "sessionId": sessionId,
                "text": payload.text,
                "dataBase64": payload.dataBase64,
            }),
        )
    }

    /// Reads classic Bluetooth payload through the OpenHarmony owner app.
    fn bluetoothRead(&self, request: BluetoothReadRequest) -> HostResult<BluetoothReadData> {
        self.execute("classic_read", Self::requestParams(request)?)
    }

    /// Sends and reads classic Bluetooth payload through the OpenHarmony owner app.
    fn bluetoothSendAndRead(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
        read: BluetoothReadRequest,
    ) -> HostResult<BluetoothReadData> {
        self.execute(
            "classic_send_and_read",
            json!({
                "sessionId": sessionId,
                "text": payload.text,
                "dataBase64": payload.dataBase64,
                "maxBytes": read.maxBytes,
                "timeoutMs": read.timeoutMs,
            }),
        )
    }

    /// Closes a Bluetooth session through the OpenHarmony owner app.
    fn bluetoothClose(&self, sessionId: &str) -> HostResult<String> {
        self.execute("close", json!({ "sessionId": sessionId }))
    }

    /// Connects to a BLE device through the OpenHarmony owner app.
    fn bluetoothBleConnect(
        &self,
        request: BluetoothBleConnectRequest,
    ) -> HostResult<BluetoothSessionData> {
        self.execute("ble_connect", Self::requestParams(request)?)
    }

    /// Discovers BLE services through the OpenHarmony owner app.
    fn bluetoothBleDiscoverServices(
        &self,
        sessionId: &str,
        timeoutMs: i64,
    ) -> HostResult<BluetoothBleServicesData> {
        self.execute(
            "ble_discover_services",
            json!({
                "sessionId": sessionId,
                "timeoutMs": timeoutMs,
            }),
        )
    }

    /// Reads a BLE characteristic through the OpenHarmony owner app.
    fn bluetoothBleReadCharacteristic(
        &self,
        address: BluetoothBleCharacteristicAddress,
    ) -> HostResult<BluetoothReadData> {
        self.execute("ble_read_characteristic", Self::requestParams(address)?)
    }

    /// Writes a BLE characteristic through the OpenHarmony owner app.
    fn bluetoothBleWriteCharacteristic(
        &self,
        request: BluetoothBleWriteRequest,
    ) -> HostResult<BluetoothTransferData> {
        self.execute("ble_write_characteristic", Self::requestParams(request)?)
    }

    /// Writes and reads BLE characteristics through the OpenHarmony owner app.
    fn bluetoothBleWriteAndReadCharacteristic(
        &self,
        request: BluetoothBleWriteAndReadRequest,
    ) -> HostResult<BluetoothReadData> {
        self.execute(
            "ble_write_and_read_characteristic",
            Self::requestParams(request)?,
        )
    }

    /// Subscribes to BLE characteristic notifications through the OpenHarmony owner app.
    fn bluetoothBleSubscribeCharacteristic(
        &self,
        request: BluetoothBleSubscribeRequest,
    ) -> HostResult<BluetoothTransferData> {
        self.execute(
            "ble_subscribe_characteristic",
            Self::requestParams(request)?,
        )
    }

    /// Reads BLE characteristic notifications through the OpenHarmony owner app.
    fn bluetoothBleReadNotifications(
        &self,
        sessionId: &str,
        limit: i64,
    ) -> HostResult<BluetoothBleNotificationData> {
        self.execute(
            "ble_read_notifications",
            json!({
                "sessionId": sessionId,
                "limit": limit,
            }),
        )
    }
}
