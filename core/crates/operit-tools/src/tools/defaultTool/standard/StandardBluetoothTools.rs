use std::sync::Arc;

use operit_host_api::{
    BluetoothBleCharacteristicAddress, BluetoothBleConnectRequest, BluetoothBleSubscribeRequest,
    BluetoothBleWriteAndReadRequest, BluetoothBleWriteRequest, BluetoothClassicAcceptRequest,
    BluetoothClassicConnectRequest, BluetoothClassicListenRequest, BluetoothHost, BluetoothPayload,
    BluetoothReadRequest, BluetoothScanRequest, HostError,
};

use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{
    AITool, ToolAccessSpec, ToolBoundary, ToolEffect, ToolExecutor, ToolValidationResult,
};
use operit_tools::tools::ToolResultDataClasses::{
    BluetoothBleNotificationData, BluetoothBleServicesData, BluetoothBondedDevicesData,
    BluetoothReadData, BluetoothScanResultData, BluetoothSessionData, BluetoothStateData,
    BluetoothTransferData, ToolResultData, stringResultData,
};

const DEFAULT_CLASSIC_UUID: &str = "00001101-0000-1000-8000-00805f9b34fb";

#[derive(Clone)]
pub struct StandardBluetoothTools {
    bluetoothHost: Option<Arc<dyn BluetoothHost>>,
}

#[derive(Clone, Copy)]
pub enum BluetoothToolOperation {
    RequestPermission,
    GetState,
    RequestEnable,
    ListBondedDevices,
    ScanDevices,
    Connect,
    Listen,
    Accept,
    Send,
    Read,
    SendAndRead,
    Close,
    BleConnect,
    BleDiscoverServices,
    BleReadCharacteristic,
    BleWriteCharacteristic,
    BleWriteAndReadCharacteristic,
    BleSubscribeCharacteristic,
    BleReadNotifications,
}

#[derive(Clone)]
pub struct BluetoothToolExecutor {
    pub tools: StandardBluetoothTools,
    pub operation: BluetoothToolOperation,
}

impl StandardBluetoothTools {
    pub fn new(bluetoothHost: Option<Arc<dyn BluetoothHost>>) -> Self {
        Self { bluetoothHost }
    }

    #[allow(non_snake_case)]
    fn host(&self) -> Result<&dyn BluetoothHost, HostError> {
        self.bluetoothHost
            .as_deref()
            .ok_or_else(|| HostError::new("BluetoothHost is not registered for this runtime."))
    }

    #[allow(non_snake_case)]
    fn requestPermission(&self, tool: &AITool) -> ToolResult {
        match self.host().and_then(|host| host.requestBluetoothPermission()) {
            Ok(value) => toolSuccessString(tool, value),
            Err(error) => toolError(tool, format!("Error requesting Bluetooth permission: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn getState(&self, tool: &AITool) -> ToolResult {
        match self.host().and_then(|host| host.bluetoothState()) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothStateData(BluetoothStateData::from(data))),
            Err(error) => toolError(tool, format!("Error getting Bluetooth state: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn requestEnable(&self, tool: &AITool) -> ToolResult {
        match self.host().and_then(|host| host.requestEnableBluetooth()) {
            Ok(value) => toolSuccessString(tool, value),
            Err(error) => toolError(tool, format!("Error requesting Bluetooth enable: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn listBondedDevices(&self, tool: &AITool) -> ToolResult {
        match self.host().and_then(|host| host.listBluetoothBondedDevices()) {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::BluetoothBondedDevicesData(BluetoothBondedDevicesData {
                    devices: data.devices.into_iter().map(Into::into).collect(),
                }),
            ),
            Err(error) => toolError(tool, format!("Error listing bonded Bluetooth devices: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn scanDevices(&self, tool: &AITool) -> ToolResult {
        let request = BluetoothScanRequest {
            durationMs: integerParameterValue(tool, "duration_ms", 10000),
            includeBle: booleanParameterValue(tool, "include_ble", true),
        };
        match self.host().and_then(|host| host.scanBluetoothDevices(request)) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothScanResultData(BluetoothScanResultData::from(data))),
            Err(error) => toolError(tool, format!("Error scanning Bluetooth devices: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn connect(&self, tool: &AITool) -> ToolResult {
        let request = BluetoothClassicConnectRequest {
            address: parameterValue(tool, "address"),
            uuid: nonEmptyParameterValue(tool, "uuid", DEFAULT_CLASSIC_UUID),
        };
        match self.host().and_then(|host| host.bluetoothConnect(request)) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothSessionData(BluetoothSessionData::from(data))),
            Err(error) => toolError(tool, format!("Error connecting Bluetooth session: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn listen(&self, tool: &AITool) -> ToolResult {
        let request = BluetoothClassicListenRequest {
            name: nonEmptyParameterValue(tool, "name", "OperitBluetooth"),
            uuid: nonEmptyParameterValue(tool, "uuid", DEFAULT_CLASSIC_UUID),
        };
        match self.host().and_then(|host| host.bluetoothListen(request)) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothSessionData(BluetoothSessionData::from(data))),
            Err(error) => toolError(tool, format!("Error creating Bluetooth listener: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn accept(&self, tool: &AITool) -> ToolResult {
        let request = BluetoothClassicAcceptRequest {
            listenerSessionId: parameterValue(tool, "listener_session_id"),
            timeoutMs: integerParameterValue(tool, "timeout_ms", 30000),
        };
        match self.host().and_then(|host| host.bluetoothAccept(request)) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothSessionData(BluetoothSessionData::from(data))),
            Err(error) => toolError(tool, format!("Error accepting Bluetooth session: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn send(&self, tool: &AITool) -> ToolResult {
        let sessionId = parameterValue(tool, "session_id");
        let payload = bluetoothPayload(tool);
        match self.host().and_then(|host| host.bluetoothSend(&sessionId, payload)) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothTransferData(BluetoothTransferData::from(data))),
            Err(error) => toolError(tool, format!("Error sending Bluetooth data: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn read(&self, tool: &AITool) -> ToolResult {
        let request = bluetoothReadRequest(tool);
        match self.host().and_then(|host| host.bluetoothRead(request)) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothReadData(BluetoothReadData::from(data))),
            Err(error) => toolError(tool, format!("Error reading Bluetooth data: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn sendAndRead(&self, tool: &AITool) -> ToolResult {
        let sessionId = parameterValue(tool, "session_id");
        let payload = bluetoothPayload(tool);
        let read = bluetoothReadRequest(tool);
        match self
            .host()
            .and_then(|host| host.bluetoothSendAndRead(&sessionId, payload, read))
        {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothReadData(BluetoothReadData::from(data))),
            Err(error) => toolError(tool, format!("Error sending and reading Bluetooth data: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn close(&self, tool: &AITool) -> ToolResult {
        let sessionId = parameterValue(tool, "session_id");
        match self.host().and_then(|host| host.bluetoothClose(&sessionId)) {
            Ok(value) => toolSuccessString(tool, value),
            Err(error) => toolError(tool, format!("Error closing Bluetooth session: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn bleConnect(&self, tool: &AITool) -> ToolResult {
        let request = BluetoothBleConnectRequest {
            address: parameterValue(tool, "address"),
            autoConnect: booleanParameterValue(tool, "auto_connect", false),
        };
        match self.host().and_then(|host| host.bluetoothBleConnect(request)) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothSessionData(BluetoothSessionData::from(data))),
            Err(error) => toolError(tool, format!("Error connecting BLE session: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn bleDiscoverServices(&self, tool: &AITool) -> ToolResult {
        let sessionId = parameterValue(tool, "session_id");
        let timeoutMs = integerParameterValue(tool, "timeout_ms", 30000);
        match self
            .host()
            .and_then(|host| host.bluetoothBleDiscoverServices(&sessionId, timeoutMs))
        {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothBleServicesData(BluetoothBleServicesData::from(data))),
            Err(error) => toolError(tool, format!("Error discovering BLE services: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn bleReadCharacteristic(&self, tool: &AITool) -> ToolResult {
        let address = bleCharacteristicAddress(tool);
        match self.host().and_then(|host| host.bluetoothBleReadCharacteristic(address)) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothReadData(BluetoothReadData::from(data))),
            Err(error) => toolError(tool, format!("Error reading BLE characteristic: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn bleWriteCharacteristic(&self, tool: &AITool) -> ToolResult {
        let request = BluetoothBleWriteRequest {
            sessionId: parameterValue(tool, "session_id"),
            serviceUuid: parameterValue(tool, "service_uuid"),
            characteristicUuid: parameterValue(tool, "characteristic_uuid"),
            text: optionalNonEmptyParameterValue(tool, "text"),
            dataBase64: optionalNonEmptyParameterValue(tool, "data_base64"),
        };
        match self.host().and_then(|host| host.bluetoothBleWriteCharacteristic(request)) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothTransferData(BluetoothTransferData::from(data))),
            Err(error) => toolError(tool, format!("Error writing BLE characteristic: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn bleWriteAndReadCharacteristic(&self, tool: &AITool) -> ToolResult {
        let request = BluetoothBleWriteAndReadRequest {
            sessionId: parameterValue(tool, "session_id"),
            writeServiceUuid: parameterValue(tool, "write_service_uuid"),
            writeCharacteristicUuid: parameterValue(tool, "write_characteristic_uuid"),
            readServiceUuid: parameterValue(tool, "read_service_uuid"),
            readCharacteristicUuid: parameterValue(tool, "read_characteristic_uuid"),
            text: optionalNonEmptyParameterValue(tool, "text"),
            dataBase64: optionalNonEmptyParameterValue(tool, "data_base64"),
            timeoutMs: integerParameterValue(tool, "timeout_ms", 30000),
        };
        match self
            .host()
            .and_then(|host| host.bluetoothBleWriteAndReadCharacteristic(request))
        {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothReadData(BluetoothReadData::from(data))),
            Err(error) => toolError(tool, format!("Error writing and reading BLE characteristic: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn bleSubscribeCharacteristic(&self, tool: &AITool) -> ToolResult {
        let request = BluetoothBleSubscribeRequest {
            sessionId: parameterValue(tool, "session_id"),
            serviceUuid: parameterValue(tool, "service_uuid"),
            characteristicUuid: parameterValue(tool, "characteristic_uuid"),
            enable: booleanParameterValue(tool, "enable", true),
        };
        match self.host().and_then(|host| host.bluetoothBleSubscribeCharacteristic(request)) {
            Ok(data) => toolSuccessData(tool, ToolResultData::BluetoothTransferData(BluetoothTransferData::from(data))),
            Err(error) => toolError(tool, format!("Error subscribing BLE characteristic: {}", error.message)),
        }
    }

    #[allow(non_snake_case)]
    fn bleReadNotifications(&self, tool: &AITool) -> ToolResult {
        let sessionId = parameterValue(tool, "session_id");
        let limit = integerParameterValue(tool, "limit", 50);
        match self
            .host()
            .and_then(|host| host.bluetoothBleReadNotifications(&sessionId, limit))
        {
            Ok(data) => toolSuccessData(
                tool,
                ToolResultData::BluetoothBleNotificationData(BluetoothBleNotificationData::from(data)),
            ),
            Err(error) => toolError(tool, format!("Error reading BLE notifications: {}", error.message)),
        }
    }
}

impl ToolExecutor for BluetoothToolExecutor {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult {
        validateBluetoothTool(self.operation, tool)
    }

    fn accessSpec(&self, _tool: &AITool) -> Result<ToolAccessSpec, String> {
        let effect = match self.operation {
            BluetoothToolOperation::GetState
            | BluetoothToolOperation::ListBondedDevices
            | BluetoothToolOperation::ScanDevices
            | BluetoothToolOperation::Read
            | BluetoothToolOperation::BleDiscoverServices
            | BluetoothToolOperation::BleReadCharacteristic
            | BluetoothToolOperation::BleReadNotifications => ToolEffect::READ,
            BluetoothToolOperation::RequestPermission
            | BluetoothToolOperation::RequestEnable
            | BluetoothToolOperation::Connect
            | BluetoothToolOperation::Listen
            | BluetoothToolOperation::Accept
            | BluetoothToolOperation::Send
            | BluetoothToolOperation::SendAndRead
            | BluetoothToolOperation::Close
            | BluetoothToolOperation::BleConnect
            | BluetoothToolOperation::BleWriteCharacteristic
            | BluetoothToolOperation::BleWriteAndReadCharacteristic
            | BluetoothToolOperation::BleSubscribeCharacteristic => ToolEffect::WRITE,
        };
        Ok(ToolAccessSpec {
            effect,
            boundary: ToolBoundary::None,
        })
    }

    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult> {
        let result = match self.operation {
            BluetoothToolOperation::RequestPermission => self.tools.requestPermission(tool),
            BluetoothToolOperation::GetState => self.tools.getState(tool),
            BluetoothToolOperation::RequestEnable => self.tools.requestEnable(tool),
            BluetoothToolOperation::ListBondedDevices => self.tools.listBondedDevices(tool),
            BluetoothToolOperation::ScanDevices => self.tools.scanDevices(tool),
            BluetoothToolOperation::Connect => self.tools.connect(tool),
            BluetoothToolOperation::Listen => self.tools.listen(tool),
            BluetoothToolOperation::Accept => self.tools.accept(tool),
            BluetoothToolOperation::Send => self.tools.send(tool),
            BluetoothToolOperation::Read => self.tools.read(tool),
            BluetoothToolOperation::SendAndRead => self.tools.sendAndRead(tool),
            BluetoothToolOperation::Close => self.tools.close(tool),
            BluetoothToolOperation::BleConnect => self.tools.bleConnect(tool),
            BluetoothToolOperation::BleDiscoverServices => self.tools.bleDiscoverServices(tool),
            BluetoothToolOperation::BleReadCharacteristic => self.tools.bleReadCharacteristic(tool),
            BluetoothToolOperation::BleWriteCharacteristic => self.tools.bleWriteCharacteristic(tool),
            BluetoothToolOperation::BleWriteAndReadCharacteristic => {
                self.tools.bleWriteAndReadCharacteristic(tool)
            }
            BluetoothToolOperation::BleSubscribeCharacteristic => {
                self.tools.bleSubscribeCharacteristic(tool)
            }
            BluetoothToolOperation::BleReadNotifications => self.tools.bleReadNotifications(tool),
        };
        vec![result]
    }
}

#[allow(non_snake_case)]
fn validateBluetoothTool(operation: BluetoothToolOperation, tool: &AITool) -> ToolValidationResult {
    let invalid = |message: &str| ToolValidationResult {
        valid: false,
        errorMessage: message.to_string(),
    };
    for name in requiredParameterNames(operation) {
        if parameterValue(tool, name).is_empty() {
            return invalid(&format!("{name} is required."));
        }
    }
    for name in integerParameterNames(operation) {
        if invalidIntegerParameter(tool, name) {
            return invalid(&format!("{name} must be an integer."));
        }
    }
    match operation {
        BluetoothToolOperation::Send
        | BluetoothToolOperation::SendAndRead
        | BluetoothToolOperation::BleWriteCharacteristic
        | BluetoothToolOperation::BleWriteAndReadCharacteristic => {
            let hasText = optionalNonEmptyParameterValue(tool, "text").is_some();
            let hasData = optionalNonEmptyParameterValue(tool, "data_base64").is_some();
            if hasText == hasData {
                return invalid("Provide exactly one of text or data_base64.");
            }
        }
        _ => {}
    }
    ToolValidationResult {
        valid: true,
        errorMessage: String::new(),
    }
}

#[allow(non_snake_case)]
fn requiredParameterNames(operation: BluetoothToolOperation) -> &'static [&'static str] {
    match operation {
        BluetoothToolOperation::Connect | BluetoothToolOperation::BleConnect => &["address"],
        BluetoothToolOperation::Accept => &["listener_session_id"],
        BluetoothToolOperation::Send
        | BluetoothToolOperation::Read
        | BluetoothToolOperation::SendAndRead
        | BluetoothToolOperation::Close
        | BluetoothToolOperation::BleDiscoverServices
        | BluetoothToolOperation::BleReadNotifications => &["session_id"],
        BluetoothToolOperation::BleReadCharacteristic
        | BluetoothToolOperation::BleWriteCharacteristic
        | BluetoothToolOperation::BleSubscribeCharacteristic => {
            &["session_id", "service_uuid", "characteristic_uuid"]
        }
        BluetoothToolOperation::BleWriteAndReadCharacteristic => &[
            "session_id",
            "write_service_uuid",
            "write_characteristic_uuid",
            "read_service_uuid",
            "read_characteristic_uuid",
        ],
        BluetoothToolOperation::RequestPermission
        | BluetoothToolOperation::GetState
        | BluetoothToolOperation::RequestEnable
        | BluetoothToolOperation::ListBondedDevices
        | BluetoothToolOperation::ScanDevices
        | BluetoothToolOperation::Listen => &[],
    }
}

#[allow(non_snake_case)]
fn integerParameterNames(operation: BluetoothToolOperation) -> &'static [&'static str] {
    match operation {
        BluetoothToolOperation::ScanDevices => &["duration_ms"],
        BluetoothToolOperation::Accept
        | BluetoothToolOperation::Read
        | BluetoothToolOperation::SendAndRead
        | BluetoothToolOperation::BleDiscoverServices
        | BluetoothToolOperation::BleReadCharacteristic
        | BluetoothToolOperation::BleWriteAndReadCharacteristic => &["timeout_ms"],
        BluetoothToolOperation::BleReadNotifications => &["limit"],
        _ => &[],
    }
}

#[allow(non_snake_case)]
fn bluetoothPayload(tool: &AITool) -> BluetoothPayload {
    BluetoothPayload {
        text: optionalNonEmptyParameterValue(tool, "text"),
        dataBase64: optionalNonEmptyParameterValue(tool, "data_base64"),
    }
}

#[allow(non_snake_case)]
fn bluetoothReadRequest(tool: &AITool) -> BluetoothReadRequest {
    BluetoothReadRequest {
        sessionId: parameterValue(tool, "session_id"),
        maxBytes: integerParameterValue(tool, "max_bytes", 4096),
        timeoutMs: integerParameterValue(tool, "timeout_ms", 30000),
    }
}

#[allow(non_snake_case)]
fn bleCharacteristicAddress(tool: &AITool) -> BluetoothBleCharacteristicAddress {
    BluetoothBleCharacteristicAddress {
        sessionId: parameterValue(tool, "session_id"),
        serviceUuid: parameterValue(tool, "service_uuid"),
        characteristicUuid: parameterValue(tool, "characteristic_uuid"),
        timeoutMs: integerParameterValue(tool, "timeout_ms", 30000),
    }
}

#[allow(non_snake_case)]
fn optionalParameterValue(tool: &AITool, name: &str) -> Option<String> {
    tool.parameters
        .iter()
        .find(|parameter| parameter.name == name)
        .map(|parameter| parameter.value.trim().to_string())
}

#[allow(non_snake_case)]
fn parameterValue(tool: &AITool, name: &str) -> String {
    optionalParameterValue(tool, name).unwrap_or_default()
}

#[allow(non_snake_case)]
fn optionalNonEmptyParameterValue(tool: &AITool, name: &str) -> Option<String> {
    optionalParameterValue(tool, name).filter(|value| !value.is_empty())
}

#[allow(non_snake_case)]
fn nonEmptyParameterValue(tool: &AITool, name: &str, value: &str) -> String {
    optionalNonEmptyParameterValue(tool, name).unwrap_or_else(|| value.to_string())
}

#[allow(non_snake_case)]
fn booleanParameterValue(tool: &AITool, name: &str, value: bool) -> bool {
    match optionalParameterValue(tool, name) {
        Some(raw) if matches!(raw.as_str(), "true" | "1" | "yes" | "y" | "on") => true,
        Some(raw) if matches!(raw.as_str(), "false" | "0" | "no" | "n" | "off") => false,
        Some(_) => value,
        None => value,
    }
}

#[allow(non_snake_case)]
fn integerParameterValue(tool: &AITool, name: &str, value: i64) -> i64 {
    optionalParameterValue(tool, name)
        .filter(|raw| !raw.is_empty())
        .map(|raw| raw.parse::<i64>().expect("integer parameter must be validated"))
        .unwrap_or(value)
}

#[allow(non_snake_case)]
fn invalidIntegerParameter(tool: &AITool, name: &str) -> bool {
    optionalParameterValue(tool, name)
        .filter(|raw| !raw.is_empty())
        .is_some_and(|raw| raw.parse::<i64>().is_err())
}

#[allow(non_snake_case)]
fn toolSuccessData(tool: &AITool, data: ToolResultData) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: data,
        error: None,
    }
}

#[allow(non_snake_case)]
fn toolSuccessString(tool: &AITool, value: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: true,
        result: stringResultData(value),
        error: None,
    }
}

#[allow(non_snake_case)]
fn toolError(tool: &AITool, error: String) -> ToolResult {
    ToolResult {
        toolName: tool.name.clone(),
        success: false,
        result: stringResultData(""),
        error: Some(error),
    }
}
