use std::sync::Arc;

use operit_host_api::*;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

pub type AppleBluetoothController = Arc<dyn Fn(&str, Value) -> HostResult<Value> + Send + Sync>;

#[derive(Clone)]
pub struct AppleBluetoothHost {
    controller: AppleBluetoothController,
}

impl AppleBluetoothHost {
    pub fn fromController(controller: AppleBluetoothController) -> Self {
        Self { controller }
    }

    fn execute<T>(&self, command: &str, params: Value) -> HostResult<T>
    where
        T: DeserializeOwned,
    {
        let value = (self.controller)(command, params)?;
        serde_json::from_value(value).map_err(|error| {
            HostError::new(format!("Apple Bluetooth response decode failed: {error}"))
        })
    }

    fn requestParams<T: serde::Serialize>(request: T) -> HostResult<Value> {
        serde_json::to_value(request).map_err(|error| {
            HostError::new(format!("Apple Bluetooth request encode failed: {error}"))
        })
    }
}

impl BluetoothHost for AppleBluetoothHost {
    fn requestBluetoothPermission(&self) -> HostResult<String> {
        self.execute("request_permission", json!({}))
    }

    fn bluetoothState(&self) -> HostResult<BluetoothStateData> {
        self.execute("state", json!({}))
    }

    fn requestEnableBluetooth(&self) -> HostResult<String> {
        self.execute("request_enable", json!({}))
    }

    fn listBluetoothBondedDevices(&self) -> HostResult<BluetoothBondedDevicesData> {
        self.execute("bonded_devices", json!({}))
    }

    fn scanBluetoothDevices(
        &self,
        request: BluetoothScanRequest,
    ) -> HostResult<BluetoothScanResultData> {
        self.execute("scan", Self::requestParams(request)?)
    }

    fn bluetoothConnect(
        &self,
        request: BluetoothClassicConnectRequest,
    ) -> HostResult<BluetoothSessionData> {
        self.execute("classic_connect", Self::requestParams(request)?)
    }

    fn bluetoothListen(
        &self,
        request: BluetoothClassicListenRequest,
    ) -> HostResult<BluetoothSessionData> {
        self.execute("classic_listen", Self::requestParams(request)?)
    }

    fn bluetoothAccept(
        &self,
        request: BluetoothClassicAcceptRequest,
    ) -> HostResult<BluetoothSessionData> {
        self.execute("classic_accept", Self::requestParams(request)?)
    }

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

    fn bluetoothRead(&self, request: BluetoothReadRequest) -> HostResult<BluetoothReadData> {
        self.execute("classic_read", Self::requestParams(request)?)
    }

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

    fn bluetoothClose(&self, sessionId: &str) -> HostResult<String> {
        self.execute("close", json!({ "sessionId": sessionId }))
    }

    fn bluetoothBleConnect(
        &self,
        request: BluetoothBleConnectRequest,
    ) -> HostResult<BluetoothSessionData> {
        self.execute("ble_connect", Self::requestParams(request)?)
    }

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

    fn bluetoothBleReadCharacteristic(
        &self,
        address: BluetoothBleCharacteristicAddress,
    ) -> HostResult<BluetoothReadData> {
        self.execute("ble_read_characteristic", Self::requestParams(address)?)
    }

    fn bluetoothBleWriteCharacteristic(
        &self,
        request: BluetoothBleWriteRequest,
    ) -> HostResult<BluetoothTransferData> {
        self.execute("ble_write_characteristic", Self::requestParams(request)?)
    }

    fn bluetoothBleWriteAndReadCharacteristic(
        &self,
        request: BluetoothBleWriteAndReadRequest,
    ) -> HostResult<BluetoothReadData> {
        self.execute(
            "ble_write_and_read_characteristic",
            Self::requestParams(request)?,
        )
    }

    fn bluetoothBleSubscribeCharacteristic(
        &self,
        request: BluetoothBleSubscribeRequest,
    ) -> HostResult<BluetoothTransferData> {
        self.execute(
            "ble_subscribe_characteristic",
            Self::requestParams(request)?,
        )
    }

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
