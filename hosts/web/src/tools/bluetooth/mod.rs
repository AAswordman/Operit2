use operit_host_api::{
    BluetoothBleCharacteristicAddress, BluetoothBleConnectRequest, BluetoothBleNotificationData,
    BluetoothBleServicesData, BluetoothBleSubscribeRequest, BluetoothBleWriteAndReadRequest,
    BluetoothBleWriteRequest, BluetoothBondedDevicesData, BluetoothClassicAcceptRequest,
    BluetoothClassicConnectRequest, BluetoothClassicListenRequest, BluetoothHost,
    BluetoothPayload, BluetoothReadData, BluetoothReadRequest, BluetoothScanRequest,
    BluetoothScanResultData, BluetoothSessionData, BluetoothStateData, BluetoothTransferData,
    HostResult,
};
use wasm_bindgen::prelude::*;

use crate::common::{
    bluetooth_ble_characteristic_address_to_js, bluetooth_ble_connect_request_to_js,
    bluetooth_ble_notification_data, bluetooth_ble_services_data,
    bluetooth_ble_subscribe_request_to_js, bluetooth_ble_write_and_read_request_to_js,
    bluetooth_ble_write_request_to_js, bluetooth_bonded_devices_data,
    bluetooth_classic_accept_request_to_js, bluetooth_classic_connect_request_to_js,
    bluetooth_classic_listen_request_to_js, bluetooth_payload_to_js, bluetooth_read_data,
    bluetooth_read_request_to_js, bluetooth_scan_request_to_js, bluetooth_scan_result_data,
    bluetooth_session_data, bluetooth_state_data, bluetooth_transfer_data, call_bluetooth,
    js_string,
};

#[derive(Clone, Debug, Default)]
pub struct WebBluetoothHost;

unsafe impl Send for WebBluetoothHost {}
unsafe impl Sync for WebBluetoothHost {}

impl WebBluetoothHost {
    pub fn new() -> Self {
        Self
    }
}

impl BluetoothHost for WebBluetoothHost {
    fn requestBluetoothPermission(&self) -> HostResult<String> {
        js_string(call_bluetooth("requestBluetoothPermission", &[])?, "requestBluetoothPermission")
    }

    fn bluetoothState(&self) -> HostResult<BluetoothStateData> {
        bluetooth_state_data(call_bluetooth("bluetoothState", &[])?)
    }

    fn requestEnableBluetooth(&self) -> HostResult<String> {
        js_string(call_bluetooth("requestEnableBluetooth", &[])?, "requestEnableBluetooth")
    }

    fn listBluetoothBondedDevices(&self) -> HostResult<BluetoothBondedDevicesData> {
        bluetooth_bonded_devices_data(call_bluetooth("listBluetoothBondedDevices", &[])?)
    }

    fn scanBluetoothDevices(
        &self,
        request: BluetoothScanRequest,
    ) -> HostResult<BluetoothScanResultData> {
        bluetooth_scan_result_data(call_bluetooth(
            "scanBluetoothDevices",
            &[bluetooth_scan_request_to_js(request)],
        )?)
    }

    fn bluetoothConnect(
        &self,
        request: BluetoothClassicConnectRequest,
    ) -> HostResult<BluetoothSessionData> {
        bluetooth_session_data(call_bluetooth(
            "bluetoothConnect",
            &[bluetooth_classic_connect_request_to_js(request)],
        )?)
    }

    fn bluetoothListen(
        &self,
        request: BluetoothClassicListenRequest,
    ) -> HostResult<BluetoothSessionData> {
        bluetooth_session_data(call_bluetooth(
            "bluetoothListen",
            &[bluetooth_classic_listen_request_to_js(request)],
        )?)
    }

    fn bluetoothAccept(
        &self,
        request: BluetoothClassicAcceptRequest,
    ) -> HostResult<BluetoothSessionData> {
        bluetooth_session_data(call_bluetooth(
            "bluetoothAccept",
            &[bluetooth_classic_accept_request_to_js(request)],
        )?)
    }

    fn bluetoothSend(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
    ) -> HostResult<BluetoothTransferData> {
        bluetooth_transfer_data(call_bluetooth(
            "bluetoothSend",
            &[JsValue::from_str(sessionId), bluetooth_payload_to_js(payload)],
        )?)
    }

    fn bluetoothRead(&self, request: BluetoothReadRequest) -> HostResult<BluetoothReadData> {
        bluetooth_read_data(call_bluetooth(
            "bluetoothRead",
            &[bluetooth_read_request_to_js(request)],
        )?)
    }

    fn bluetoothSendAndRead(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
        read: BluetoothReadRequest,
    ) -> HostResult<BluetoothReadData> {
        bluetooth_read_data(call_bluetooth(
            "bluetoothSendAndRead",
            &[
                JsValue::from_str(sessionId),
                bluetooth_payload_to_js(payload),
                bluetooth_read_request_to_js(read),
            ],
        )?)
    }

    fn bluetoothClose(&self, sessionId: &str) -> HostResult<String> {
        js_string(
            call_bluetooth("bluetoothClose", &[JsValue::from_str(sessionId)])?,
            "bluetoothClose",
        )
    }

    fn bluetoothBleConnect(
        &self,
        request: BluetoothBleConnectRequest,
    ) -> HostResult<BluetoothSessionData> {
        bluetooth_session_data(call_bluetooth(
            "bluetoothBleConnect",
            &[bluetooth_ble_connect_request_to_js(request)],
        )?)
    }

    fn bluetoothBleDiscoverServices(
        &self,
        sessionId: &str,
        timeoutMs: i64,
    ) -> HostResult<BluetoothBleServicesData> {
        bluetooth_ble_services_data(call_bluetooth(
            "bluetoothBleDiscoverServices",
            &[JsValue::from_str(sessionId), JsValue::from_f64(timeoutMs as f64)],
        )?)
    }

    fn bluetoothBleReadCharacteristic(
        &self,
        address: BluetoothBleCharacteristicAddress,
    ) -> HostResult<BluetoothReadData> {
        bluetooth_read_data(call_bluetooth(
            "bluetoothBleReadCharacteristic",
            &[bluetooth_ble_characteristic_address_to_js(address)],
        )?)
    }

    fn bluetoothBleWriteCharacteristic(
        &self,
        request: BluetoothBleWriteRequest,
    ) -> HostResult<BluetoothTransferData> {
        bluetooth_transfer_data(call_bluetooth(
            "bluetoothBleWriteCharacteristic",
            &[bluetooth_ble_write_request_to_js(request)],
        )?)
    }

    fn bluetoothBleWriteAndReadCharacteristic(
        &self,
        request: BluetoothBleWriteAndReadRequest,
    ) -> HostResult<BluetoothReadData> {
        bluetooth_read_data(call_bluetooth(
            "bluetoothBleWriteAndReadCharacteristic",
            &[bluetooth_ble_write_and_read_request_to_js(request)],
        )?)
    }

    fn bluetoothBleSubscribeCharacteristic(
        &self,
        request: BluetoothBleSubscribeRequest,
    ) -> HostResult<BluetoothTransferData> {
        bluetooth_transfer_data(call_bluetooth(
            "bluetoothBleSubscribeCharacteristic",
            &[bluetooth_ble_subscribe_request_to_js(request)],
        )?)
    }

    fn bluetoothBleReadNotifications(
        &self,
        sessionId: &str,
        limit: i64,
    ) -> HostResult<BluetoothBleNotificationData> {
        bluetooth_ble_notification_data(call_bluetooth(
            "bluetoothBleReadNotifications",
            &[JsValue::from_str(sessionId), JsValue::from_f64(limit as f64)],
        )?)
    }
}
