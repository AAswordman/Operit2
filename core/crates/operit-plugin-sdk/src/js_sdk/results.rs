//! Runtime-backed result contracts with their serialization and formatting behavior.

#![allow(non_snake_case)]
pub use crate::js_sdk::{JsNullable, JsOptional};
use chrono::{Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Contains every concrete payload returned by the built-in tool runtime.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "__type")]
pub enum ToolResultData {
    BooleanResultData(BooleanResultData),
    StringResultData(StringResultData),
    SleepResultData(SleepResultData),
    EnvironmentVariableReadResultData(EnvironmentVariableReadResultData),
    EnvironmentVariableWriteResultData(EnvironmentVariableWriteResultData),
    IntResultData(IntResultData),
    BinaryResultData(BinaryResultData),
    FilePartContentData(FilePartContentData),
    DirectoryListingData(DirectoryListingData),
    FileContentData(FileContentData),
    BinaryFileContentData(BinaryFileContentData),
    FileExistsData(FileExistsData),
    FileInfoData(FileInfoData),
    FileOperationData(FileOperationData),
    FileApplyResultData(FileApplyResultData),
    HttpResponseData(HttpResponseData),
    HttpStreamEventData(HttpStreamEventData),
    SystemSettingData(SystemSettingData),
    AppOperationData(AppOperationData),
    AppListData(AppListData),
    AppUsageTimeResultData(AppUsageTimeResultData),
    NotificationData(NotificationData),
    LocationData(LocationData),
    DeviceInfoResultData(DeviceInfoResultData),
    MemoryQueryResultData(MemoryQueryResultData),
    ChatServiceStartResultData(ChatServiceStartResultData),
    ChatCreationResultData(ChatCreationResultData),
    ChatListResultData(ChatListResultData),
    ChatFindResultData(ChatFindResultData),
    AgentStatusResultData(AgentStatusResultData),
    ChatSwitchResultData(ChatSwitchResultData),
    ChatTitleUpdateResultData(ChatTitleUpdateResultData),
    ChatDeleteResultData(ChatDeleteResultData),
    MessageSendResultData(MessageSendResultData),
    ChatMessagesResultData(ChatMessagesResultData),
    CharacterCardListResultData(CharacterCardListResultData),
    VisitWebResultData(VisitWebResultData),
    TerminalInfoResultData(TerminalInfoResultData),
    TerminalCommandResultData(TerminalCommandResultData),
    TerminalStreamEventData(TerminalStreamEventData),
    HiddenTerminalCommandResultData(HiddenTerminalCommandResultData),
    TerminalSessionCreationResultData(TerminalSessionCreationResultData),
    TerminalSessionCloseResultData(TerminalSessionCloseResultData),
    TerminalSessionScreenResultData(TerminalSessionScreenResultData),
    MusicPlaybackResultData(MusicPlaybackResultData),
    BluetoothStateData(BluetoothStateData),
    BluetoothBondedDevicesData(BluetoothBondedDevicesData),
    BluetoothScanResultData(BluetoothScanResultData),
    BluetoothSessionData(BluetoothSessionData),
    BluetoothTransferData(BluetoothTransferData),
    BluetoothReadData(BluetoothReadData),
    BluetoothBleServicesData(BluetoothBleServicesData),
    BluetoothBleNotificationData(BluetoothBleNotificationData),
    FindFilesResultData(FindFilesResultData),
    GrepResultData(GrepResultData),
    MemoryLinkResultData(MemoryLinkResultData),
    MemoryLinkQueryResultData(MemoryLinkQueryResultData),
}

impl ToolResultData {
    /// Serializes the tagged tool result into JSON.
    #[allow(non_snake_case)]
    pub fn toJson(&self) -> String {
        serde_json::to_string(self).expect("ToolResultData serialization failed")
    }

    /// Formats the concrete tool payload for text output.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        match self {
            Self::BooleanResultData(data) => data.value.to_string(),
            Self::StringResultData(data) => data.value.clone(),
            Self::SleepResultData(data) => data.toString(),
            Self::EnvironmentVariableReadResultData(data) => data.toString(),
            Self::EnvironmentVariableWriteResultData(data) => data.toString(),
            Self::IntResultData(data) => data.value.to_string(),
            Self::BinaryResultData(data) => format!("Binary data ({} bytes)", data.value.len()),
            Self::FilePartContentData(data) => data.toString(),
            Self::DirectoryListingData(data) => data.toString(),
            Self::FileContentData(data) => data.toString(),
            Self::BinaryFileContentData(data) => data.toString(),
            Self::FileExistsData(data) => data.toString(),
            Self::FileInfoData(data) => data.toString(),
            Self::FileOperationData(data) => data.toString(),
            Self::FileApplyResultData(data) => data.toString(),
            Self::HttpResponseData(data) => data.toString(),
            Self::HttpStreamEventData(data) => data.toString(),
            Self::SystemSettingData(data) => data.toString(),
            Self::AppOperationData(data) => data.toString(),
            Self::AppListData(data) => data.toString(),
            Self::AppUsageTimeResultData(data) => data.toString(),
            Self::NotificationData(data) => data.toString(),
            Self::LocationData(data) => data.toString(),
            Self::DeviceInfoResultData(data) => data.toString(),
            Self::MemoryQueryResultData(data) => data.toString(),
            Self::ChatServiceStartResultData(data) => {
                if data.isConnected {
                    "Chat service started and connected successfully".to_string()
                } else {
                    "Chat service connection failed".to_string()
                }
            }
            Self::ChatCreationResultData(data) => {
                format!("Created new chat\nChat ID: {}", data.chatId)
            }
            Self::ChatListResultData(data) => data.toString(),
            Self::ChatFindResultData(data) => data.toString(),
            Self::AgentStatusResultData(data) => data.toString(),
            Self::ChatSwitchResultData(data) => data.toString(),
            Self::ChatTitleUpdateResultData(data) => {
                format!("Updated chat title: {} -> {}", data.chatId, data.title)
            }
            Self::ChatDeleteResultData(data) => format!("Deleted chat: {}", data.chatId),
            Self::MessageSendResultData(data) => data.toString(),
            Self::ChatMessagesResultData(data) => format!(
                "Chat messages: {} (order={}, limit={})\nTotal: {}",
                data.chatId,
                data.order,
                data.limit,
                data.messages.len()
            ),
            Self::CharacterCardListResultData(data) => data.toString(),
            Self::VisitWebResultData(data) => data.toString(),
            Self::TerminalInfoResultData(data) => data.toString(),
            Self::TerminalCommandResultData(data) => data.toString(),
            Self::TerminalStreamEventData(data) => data.toString(),
            Self::HiddenTerminalCommandResultData(data) => data.toString(),
            Self::TerminalSessionCreationResultData(data) => data.toString(),
            Self::TerminalSessionCloseResultData(data) => data.message.clone(),
            Self::TerminalSessionScreenResultData(data) => data.toString(),
            Self::MusicPlaybackResultData(data) => data.toString(),
            Self::BluetoothStateData(data) => data.toString(),
            Self::BluetoothBondedDevicesData(data) => data.toString(),
            Self::BluetoothScanResultData(data) => data.toString(),
            Self::BluetoothSessionData(data) => data.toString(),
            Self::BluetoothTransferData(data) => data.toString(),
            Self::BluetoothReadData(data) => data.toString(),
            Self::BluetoothBleServicesData(data) => data.toString(),
            Self::BluetoothBleNotificationData(data) => data.toString(),
            Self::FindFilesResultData(data) => data.toString(),
            Self::GrepResultData(data) => data.toString(),
            Self::MemoryLinkResultData(data) => data.toString(),
            Self::MemoryLinkQueryResultData(data) => data.toString(),
        }
    }
}

/// Builds a tagged string tool result.
#[allow(non_snake_case)]
pub fn stringResultData(value: impl Into<String>) -> ToolResultData {
    ToolResultData::StringResultData(StringResultData {
        value: value.into(),
    })
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Captures the UI node and Android surface reached when an automation run finishes.
pub struct AutomationExecutionFinalState {
    /// Identifies the final automation node.
    #[serde(rename = "nodeId")]
    pub node_id: String,
    /// Identifies the package shown in the final state.
    #[serde(rename = "packageName")]
    pub package_name: String,
    /// Identifies the activity shown in the final state.
    #[serde(rename = "activityName")]
    pub activity_name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Records an automation or UI subagent invocation, its session, outcome, logs, and final state.
pub struct AutomationExecutionResultData {
    #[serde(rename = "functionName")]
    ///Function name of the automation or subagent
    pub function_name: String,
    #[serde(rename = "providedParameters")]
    ///Parameters provided to the automation
    pub provided_parameters: BTreeMap<String, String>,
    #[serde(rename = "agentId")]
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Optional agent id used for this run (can be reused to keep operating on the same virtual screen session)
    pub agent_id: JsOptional<String>,
    #[serde(rename = "displayId")]
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Optional virtual display id associated with the agent session
    pub display_id: JsOptional<f64>,
    #[serde(rename = "executionSuccess")]
    ///Whether the execution succeeded
    pub execution_success: bool,
    #[serde(rename = "executionMessage")]
    ///Detailed execution message and action logs
    pub execution_message: String,
    #[serde(rename = "executionError")]
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Optional error message when execution fails
    pub execution_error: JsOptional<String>,
    #[serde(rename = "finalState")]
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Final UI state information, if available
    pub final_state: JsOptional<AutomationExecutionFinalState>,
    #[serde(rename = "executionSteps")]
    ///Number of steps executed
    pub execution_steps: f64,
}
impl AutomationExecutionResultData {
    /// Formats the automation result for tool output.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        self.execution_message.clone()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Represents one serializable UI hierarchy node with accessibility metadata and child nodes.
pub struct SimplifiedUINode {
    #[serde(rename = "className")]
    pub class_name: Option<String>,
    #[serde(rename = "text")]
    pub text: Option<String>,
    #[serde(rename = "contentDesc")]
    pub content_desc: Option<String>,
    #[serde(rename = "resourceId")]
    pub resource_id: Option<String>,
    #[serde(rename = "bounds")]
    pub bounds: Option<String>,
    #[serde(rename = "isClickable")]
    pub is_clickable: bool,
    #[serde(rename = "children")]
    pub children: Vec<SimplifiedUINode>,
}
impl SimplifiedUINode {
    /// Formats the node as a compact JSON value.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        serde_json::to_string(self).expect("SimplifiedUINode serialization failed")
    }
    /// Formats this node and its descendants as an indented tree.
    #[allow(non_snake_case)]
    pub fn toTreeString(&self, indent: Option<String>) -> String {
        let indent = indent.unwrap_or_default();
        let mut output = format!("{indent}{}", self.toString());
        let child_indent = format!("{indent}  ");
        for child in &self.children {
            output.push('\n');
            output.push_str(&child.toTreeString(Some(child_indent.clone())));
        }
        output
    }
    /// Reports whether the node contains information relevant to UI automation.
    #[allow(non_snake_case)]
    pub fn shouldKeepNode(&self) -> bool {
        self.is_clickable || self.text.as_ref().is_some_and(|value| !value.is_empty())
            || self.content_desc.as_ref().is_some_and(|value| !value.is_empty())
            || self.resource_id.as_ref().is_some_and(|value| !value.is_empty())
            || !self.children.is_empty()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Captures the active Android package, activity, and simplified UI hierarchy.
pub struct UIPageResultData {
    #[serde(rename = "packageName")]
    pub package_name: String,
    #[serde(rename = "activityName")]
    pub activity_name: String,
    #[serde(rename = "uiElements")]
    pub ui_elements: SimplifiedUINode,
}
impl UIPageResultData {
    /// Formats the current UI page for tool output.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        format!(
            "Package: {}\nActivity: {}\n{}", self.package_name, self.activity_name, self
            .ui_elements.toTreeString(None)
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Describes an executed UI action and its optional target coordinates or element identifier.
pub struct UIActionResultData {
    #[serde(rename = "actionType")]
    pub action_type: String,
    #[serde(rename = "actionDescription")]
    pub action_description: String,
    #[serde(rename = "coordinates")]
    pub coordinates: Option<(f64, f64)>,
    #[serde(rename = "elementId")]
    pub element_id: Option<String>,
}
impl UIActionResultData {
    /// Formats the UI action for tool output.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        self.action_description.clone()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies the command interpreter backing a terminal session.
pub enum TerminalType {
    #[serde(rename = "powershell")]
    Powershell,
    #[serde(rename = "bash")]
    Bash,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "shell")]
    Shell,
}
impl TerminalType {
    /// Returns the JavaScript terminal type literal.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Powershell => "powershell",
            Self::Bash => "bash",
            Self::Linux => "linux",
            Self::Shell => "shell",
        }
    }
}
impl std::fmt::Display for TerminalType {
    /// Writes the JavaScript terminal type literal.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
impl TryFrom<&str> for TerminalType {
    type Error = String;
    /// Parses a JavaScript terminal type literal.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "powershell" => Ok(Self::Powershell),
            "bash" => Ok(Self::Bash),
            "linux" => Ok(Self::Linux),
            "shell" => Ok(Self::Shell),
            _ => Err(format!("invalid terminal type: {value}")),
        }
    }
}
/// Selects the command interpreter requested when creating a terminal session.
pub type TerminalCreateType = TerminalType;
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Reports the start or an incremental chunk of a streamed chat-message response.
pub struct MessageSendStreamEventData {
    ///Event type, currently "start" or "chunk"
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "chatId")]
    ///The ID of the chat receiving the streamed reply
    pub chat_id: String,
    ///The original message content that was sent
    #[serde(rename = "message")]
    pub message: String,
    ///Whether waifu-style chunk aggregation is enabled for this stream
    #[serde(rename = "waifu")]
    pub waifu: bool,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Incremental chunk content for "chunk" events
    #[serde(rename = "chunk")]
    pub chunk: JsOptional<String>,
    #[serde(rename = "chunkIndex")]
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Zero-based chunk index
    pub chunk_index: JsOptional<f64>,
    #[serde(rename = "receivedChars")]
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Total received character count so far
    pub received_chars: JsOptional<f64>,
}
impl MessageSendStreamEventData {
    #[allow(non_snake_case)]
    /// Returns the original message associated with this stream event.
    pub fn toString(&self) -> String {
        self.message.clone()
    }
}
#[derive(Clone, Serialize, Deserialize)]
/// Wraps a boolean value returned by a built-in tool.
pub struct BooleanResultData {
    pub value: bool,
}
#[derive(Clone, Serialize, Deserialize)]
/// Wraps a string value returned by a built-in tool.
pub struct StringResultData {
    /// Contains the returned string value.
    #[serde(rename = "value")]
    pub value: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Records the requested and actual duration of a completed sleep operation.
pub struct SleepResultData {
    #[serde(rename = "requestedMs")]
    pub requestedMs: i32,
    #[serde(rename = "sleptMs")]
    pub sleptMs: i32,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports the current value and existence state of an environment variable.
pub struct EnvironmentVariableReadResultData {
    #[serde(rename = "key")]
    pub key: String,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    #[serde(rename = "value")]
    pub value: JsOptional<String>,
    #[serde(rename = "exists")]
    pub exists: bool,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports the requested and resulting state of an environment-variable update.
pub struct EnvironmentVariableWriteResultData {
    #[serde(rename = "key")]
    pub key: String,
    #[serde(rename = "requestedValue")]
    pub requestedValue: String,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    #[serde(rename = "value")]
    pub value: JsOptional<String>,
    #[serde(rename = "exists")]
    pub exists: bool,
    #[serde(rename = "cleared")]
    pub cleared: bool,
}
#[derive(Clone, Serialize, Deserialize)]
/// Wraps an integer value returned by a built-in tool.
pub struct IntResultData {
    pub value: i32,
}
#[derive(Clone, Serialize, Deserialize)]
/// Carries raw bytes returned by a built-in tool.
pub struct BinaryResultData {
    #[serde(with = "serde_bytes")]
    pub value: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains one numbered line segment read from a text file.
pub struct FilePartContentData {
    #[serde(rename = "path")]
    pub path: String,
    #[serde(rename = "content")]
    pub content: String,
    #[serde(rename = "partIndex")]
    pub partIndex: i32,
    #[serde(rename = "totalParts")]
    pub totalParts: i32,
    #[serde(rename = "startLine")]
    pub startLine: i32,
    #[serde(rename = "endLine")]
    pub endLine: i32,
    #[serde(rename = "totalLines")]
    pub totalLines: i32,
}
#[derive(Clone, Serialize, Deserialize)]
/// Describes one file-system entry returned in a directory listing.
pub struct FileEntry {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "isDirectory")]
    pub isDirectory: bool,
    #[serde(rename = "size")]
    pub size: i64,
    #[serde(rename = "permissions")]
    pub permissions: String,
    #[serde(rename = "lastModified")]
    pub lastModified: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains the entries found at a virtual file-system directory path.
pub struct DirectoryListingData {
    #[serde(rename = "path")]
    pub path: String,
    #[serde(rename = "entries")]
    pub entries: Vec<FileEntry>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains the complete text and byte size read from a file.
pub struct FileContentData {
    #[serde(rename = "path")]
    pub path: String,
    #[serde(rename = "content")]
    pub content: String,
    #[serde(rename = "size")]
    pub size: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains file bytes encoded as Base64 together with the original byte size.
pub struct BinaryFileContentData {
    #[serde(rename = "path")]
    pub path: String,
    ///Base64 encoded content of the file
    #[serde(rename = "contentBase64")]
    pub contentBase64: String,
    ///File size in bytes
    #[serde(rename = "size")]
    pub size: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports whether a virtual file-system path exists and what it contains.
pub struct FileExistsData {
    #[serde(rename = "path")]
    pub path: String,
    #[serde(rename = "exists")]
    pub exists: bool,
    #[serde(rename = "isDirectory")]
    pub isDirectory: bool,
    #[serde(rename = "size")]
    pub size: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports virtual file-system stat metadata, including ownership, permissions, and raw output.
pub struct FileInfoData {
    #[serde(rename = "path")]
    pub path: String,
    #[serde(rename = "exists")]
    pub exists: bool,
    #[serde(rename = "fileType")]
    pub fileType: String,
    #[serde(rename = "size")]
    pub size: i64,
    #[serde(rename = "permissions")]
    pub permissions: String,
    #[serde(rename = "owner")]
    pub owner: String,
    #[serde(rename = "group")]
    pub group: String,
    #[serde(rename = "lastModified")]
    pub lastModified: String,
    #[serde(rename = "rawStatOutput")]
    pub rawStatOutput: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports the target, success state, and details of a virtual file-system mutation.
pub struct FileOperationData {
    #[serde(rename = "operation")]
    pub operation: String,
    #[serde(rename = "path")]
    pub path: String,
    #[serde(rename = "successful")]
    pub successful: bool,
    #[serde(rename = "details")]
    pub details: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Combines a file operation outcome with its generated diff and patch instructions.
pub struct FileApplyResultData {
    #[serde(rename = "operation")]
    pub operation: FileOperationData,
    #[serde(rename = "aiDiffInstructions")]
    pub aiDiffInstructions: String,
    pub diffContent: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains an HTTP response status, headers, cookies, media type, and decoded or Base64 body.
pub struct HttpResponseData {
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "statusCode")]
    pub statusCode: i32,
    #[serde(rename = "statusMessage")]
    pub statusMessage: String,
    #[serde(rename = "headers")]
    pub headers: HashMap<String, String>,
    #[serde(rename = "contentType")]
    pub contentType: String,
    #[serde(rename = "content")]
    pub content: String,
    pub contentBase64: Option<String>,
    #[serde(rename = "size")]
    pub size: i32,
    pub cookies: HashMap<String, String>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Describes one metadata or body-chunk event from a streaming HTTP response.
pub struct HttpStreamEventData {
    pub r#type: String,
    pub url: String,
    pub statusCode: Option<i32>,
    pub statusMessage: Option<String>,
    pub headers: HashMap<String, String>,
    pub contentType: Option<String>,
    pub chunk: Option<String>,
    pub chunkIndex: Option<i32>,
    pub receivedBytes: Option<i64>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Identifies a system-setting namespace, key, and current value.
pub struct SystemSettingData {
    #[serde(rename = "namespace")]
    pub namespace: String,
    #[serde(rename = "setting")]
    pub setting: String,
    #[serde(rename = "value")]
    pub value: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports the outcome of installing, uninstalling, starting, or stopping an application package.
pub struct AppOperationData {
    #[serde(rename = "operationType")]
    pub operationType: String,
    #[serde(rename = "packageName")]
    pub packageName: String,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "details")]
    pub details: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Lists installed application packages and whether system applications were included.
pub struct AppListData {
    #[serde(rename = "includesSystemApps")]
    pub includesSystemApps: bool,
    #[serde(rename = "packages")]
    pub packages: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Records one application's foreground duration, last use, and system-app classification.
pub struct AppUsageTimeEntry {
    #[serde(rename = "packageName")]
    pub packageName: String,
    #[serde(rename = "appName")]
    pub appName: String,
    #[serde(rename = "totalForegroundTimeMs")]
    pub totalForegroundTimeMs: i64,
    #[serde(rename = "lastTimeUsed")]
    pub lastTimeUsed: i64,
    #[serde(rename = "isSystemApp")]
    pub isSystemApp: bool,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains application foreground usage over a requested time window and package filter.
pub struct AppUsageTimeResultData {
    #[serde(rename = "startTime")]
    pub startTime: i64,
    #[serde(rename = "endTime")]
    pub endTime: i64,
    #[serde(rename = "sinceHours")]
    pub sinceHours: i32,
    #[serde(rename = "requestedPackageName")]
    pub requestedPackageName: Option<String>,
    #[serde(rename = "includesSystemApps")]
    pub includesSystemApps: bool,
    #[serde(rename = "totalEntries")]
    pub totalEntries: i32,
    #[serde(rename = "entries")]
    pub entries: Vec<AppUsageTimeEntry>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Captures the source, text, and arrival time of one system notification.
pub struct Notification {
    pub packageName: String,
    pub text: String,
    pub timestamp: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains the system notifications visible at a specific retrieval time.
pub struct NotificationData {
    ///List of notification objects
    #[serde(rename = "notifications")]
    pub notifications: Vec<Notification>,
    ///Timestamp when the notifications were retrieved
    #[serde(rename = "timestamp")]
    pub timestamp: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports a device location with provider accuracy, raw data, and reverse-geocoded address fields.
pub struct LocationData {
    ///Latitude coordinate in decimal degrees
    #[serde(rename = "latitude")]
    pub latitude: f64,
    ///Longitude coordinate in decimal degrees
    #[serde(rename = "longitude")]
    pub longitude: f64,
    ///Accuracy of the location in meters
    #[serde(rename = "accuracy")]
    pub accuracy: f32,
    ///Location provider (e.g., "gps", "network", etc.)
    #[serde(rename = "provider")]
    pub provider: String,
    ///Timestamp when the location was retrieved
    #[serde(rename = "timestamp")]
    pub timestamp: i64,
    ///Raw location data from the system
    #[serde(rename = "rawData")]
    pub rawData: String,
    ///Street address determined from coordinates
    #[serde(rename = "address")]
    pub address: String,
    ///City name determined from coordinates
    #[serde(rename = "city")]
    pub city: String,
    ///Province/state name determined from coordinates
    #[serde(rename = "province")]
    pub province: String,
    ///Country name determined from coordinates
    #[serde(rename = "country")]
    pub country: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Summarizes device identity, Android version, display, memory, storage, power, CPU, and network state.
pub struct DeviceInfoResultData {
    #[serde(rename = "deviceId")]
    pub deviceId: String,
    #[serde(rename = "model")]
    pub model: String,
    #[serde(rename = "manufacturer")]
    pub manufacturer: String,
    #[serde(rename = "androidVersion")]
    pub androidVersion: String,
    #[serde(rename = "sdkVersion")]
    pub sdkVersion: i32,
    #[serde(rename = "screenResolution")]
    pub screenResolution: String,
    #[serde(rename = "screenDensity")]
    pub screenDensity: f32,
    #[serde(rename = "totalMemory")]
    pub totalMemory: String,
    #[serde(rename = "availableMemory")]
    pub availableMemory: String,
    #[serde(rename = "totalStorage")]
    pub totalStorage: String,
    #[serde(rename = "availableStorage")]
    pub availableStorage: String,
    #[serde(rename = "batteryLevel")]
    pub batteryLevel: i32,
    #[serde(rename = "batteryCharging")]
    pub batteryCharging: bool,
    #[serde(rename = "cpuInfo")]
    pub cpuInfo: String,
    #[serde(rename = "networkType")]
    pub networkType: String,
    #[serde(rename = "additionalInfo")]
    pub additionalInfo: BTreeMap<String, String>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Describes one stored memory together with ownership, provenance, tags, and chunk metadata.
pub struct MemoryInfo {
    pub ownerKey: String,
    pub title: String,
    pub content: String,
    pub source: String,
    pub tags: Vec<String>,
    pub createdAt: String,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    pub chunkInfo: JsOptional<String>,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    pub chunkIndices: JsOptional<Vec<i32>>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains matched memories and snapshot metadata used to suppress previously returned matches.
pub struct MemoryQueryResultData {
    ///Queried memories
    #[serde(rename = "memories")]
    pub memories: Vec<MemoryInfo>,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Snapshot id for de-duplicated follow-up or parallel queries; may be auto-generated or caller-specified
    #[serde(rename = "snapshotId")]
    pub snapshotId: JsOptional<String>,
    ///Whether this call created a new snapshot, including when a caller-specified id was created on first use
    #[serde(rename = "snapshotCreated")]
    pub snapshotCreated: Option<bool>,
    ///Number of matched memories excluded because they were already seen in the snapshot
    #[serde(rename = "excludedBySnapshotCount")]
    pub excludedBySnapshotCount: Option<i32>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports whether the chat service connected and when the connection was established.
pub struct ChatServiceStartResultData {
    ///Whether the service is connected
    #[serde(rename = "isConnected")]
    pub isConnected: bool,
    ///Connection timestamp
    #[serde(rename = "connectionTime")]
    pub connectionTime: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Identifies a newly created chat and its creation time.
pub struct ChatCreationResultData {
    ///The ID of the newly created chat
    #[serde(rename = "chatId")]
    pub chatId: String,
    ///Creation timestamp
    #[serde(rename = "createdAt")]
    pub createdAt: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Summarizes a chat, its activity, token use, current status, and bound character card.
pub struct ChatInfo {
    ///Chat ID
    #[serde(rename = "id")]
    pub id: String,
    ///Chat title
    #[serde(rename = "title")]
    pub title: String,
    ///Number of messages in the chat
    #[serde(rename = "messageCount")]
    pub messageCount: i32,
    ///Creation timestamp
    #[serde(rename = "createdAt")]
    pub createdAt: String,
    ///Last updated timestamp
    #[serde(rename = "updatedAt")]
    pub updatedAt: String,
    ///Whether this is the current active chat
    #[serde(rename = "isCurrent")]
    pub isCurrent: bool,
    ///Total input tokens used
    #[serde(rename = "inputTokens")]
    pub inputTokens: i32,
    ///Total output tokens used
    #[serde(rename = "outputTokens")]
    pub outputTokens: i32,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Bound character card name (if any)
    #[serde(rename = "characterCardName")]
    pub characterCardName: JsOptional<String>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains the available chats and identifies the currently active chat.
pub struct ChatListResultData {
    ///Total number of chats
    #[serde(rename = "totalCount")]
    pub totalCount: usize,
    ///The ID of the current active chat
    #[serde(rename = "currentChatId")]
    pub currentChatId: JsNullable<String>,
    ///List of chat information
    #[serde(rename = "chats")]
    pub chats: Vec<ChatInfo>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports the number of matching chats and the selected match, when one exists.
pub struct ChatFindResultData {
    ///Total matched chats
    #[serde(rename = "matchedCount")]
    pub matchedCount: usize,
    ///The selected chat (if any)
    #[serde(rename = "chat")]
    pub chat: JsNullable<ChatInfo>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports a chat agent's current state and whether it is idle or processing.
pub struct AgentStatusResultData {
    ///Target chat id
    #[serde(rename = "chatId")]
    pub chatId: String,
    ///Current state key
    #[serde(rename = "state")]
    pub state: String,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Optional detail message
    #[serde(rename = "message")]
    pub message: JsOptional<String>,
    ///Whether the chat is idle
    #[serde(rename = "isIdle")]
    pub isIdle: bool,
    ///Whether the chat is processing
    #[serde(rename = "isProcessing")]
    pub isProcessing: bool,
}
#[derive(Clone, Serialize, Deserialize)]
/// Identifies the chat selected as active and the time of the switch.
pub struct ChatSwitchResultData {
    ///The ID of the chat switched to
    #[serde(rename = "chatId")]
    pub chatId: String,
    ///The title of the chat
    #[serde(rename = "chatTitle")]
    pub chatTitle: String,
    ///Switch timestamp
    #[serde(rename = "switchedAt")]
    pub switchedAt: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports a chat's updated title and update time.
pub struct ChatTitleUpdateResultData {
    ///Target chat ID
    #[serde(rename = "chatId")]
    pub chatId: String,
    ///Updated title
    #[serde(rename = "title")]
    pub title: String,
    ///Update timestamp
    #[serde(rename = "updatedAt")]
    pub updatedAt: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Identifies a deleted chat and the time it was deleted.
pub struct ChatDeleteResultData {
    ///Deleted chat ID
    #[serde(rename = "chatId")]
    pub chatId: String,
    ///Delete timestamp
    #[serde(rename = "deletedAt")]
    pub deletedAt: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Records a sent chat message and its optional final AI reply.
pub struct MessageSendResultData {
    ///The ID of the chat the message was sent to
    #[serde(rename = "chatId")]
    pub chatId: String,
    ///The message content that was sent
    #[serde(rename = "message")]
    pub message: String,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Final AI response content when available
    #[serde(rename = "aiResponse")]
    pub aiResponse: JsOptional<String>,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Reply receive timestamp when available
    #[serde(rename = "receivedAt")]
    pub receivedAt: JsOptional<i64>,
    ///Sent timestamp
    #[serde(rename = "sentAt")]
    pub sentAt: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Describes one chat message together with its role, provider, model, and timestamp.
pub struct ChatMessageInfo {
    #[serde(rename = "sender")]
    pub sender: String,
    #[serde(rename = "content")]
    pub content: String,
    #[serde(rename = "timestamp")]
    pub timestamp: i64,
    #[serde(rename = "roleName")]
    pub roleName: String,
    #[serde(rename = "provider")]
    pub provider: String,
    #[serde(rename = "modelName")]
    pub modelName: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains an ordered, limited message history for a chat.
pub struct ChatMessagesResultData {
    #[serde(rename = "chatId")]
    pub chatId: String,
    #[serde(rename = "order")]
    pub order: String,
    #[serde(rename = "limit")]
    pub limit: i32,
    #[serde(rename = "messages")]
    pub messages: Vec<ChatMessageInfo>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Describes a character card, including its default status and lifecycle timestamps.
pub struct CharacterCardInfo {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "isDefault")]
    pub isDefault: bool,
    #[serde(rename = "createdAt")]
    pub createdAt: i64,
    #[serde(rename = "updatedAt")]
    pub updatedAt: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains the character cards available to the chat service.
pub struct CharacterCardListResultData {
    #[serde(rename = "totalCount")]
    pub totalCount: usize,
    #[serde(rename = "cards")]
    pub cards: Vec<CharacterCardInfo>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports the runtime platform, default terminal type, and available interpreters.
pub struct TerminalInfoResultData {
    ///Current runtime platform, such as windows, linux, or android
    #[serde(rename = "platform")]
    pub platform: String,
    ///Default terminal type for this platform
    #[serde(rename = "defaultType")]
    pub defaultType: TerminalType,
    ///Terminal types known to this host
    #[serde(rename = "types")]
    pub types: Vec<TerminalTypeInfoData>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports whether one terminal interpreter is available and describes its purpose.
pub struct TerminalTypeInfoData {
    ///Terminal type id supported by a terminal host
    #[serde(rename = "terminalType")]
    pub terminalType: TerminalType,
    ///Whether this terminal type is available on the current platform
    #[serde(rename = "available")]
    pub available: bool,
    ///Human-readable terminal type description
    #[serde(rename = "description")]
    pub description: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Records a terminal command's session, interpreter, output, exit code, and timeout state.
pub struct TerminalCommandResultData {
    ///The command that was executed
    #[serde(rename = "command")]
    pub command: String,
    ///The output from the command execution
    #[serde(rename = "output")]
    pub output: String,
    ///Exit code from the command (0 typically means success)
    #[serde(rename = "exitCode")]
    pub exitCode: i32,
    ///ID of the terminal session used for execution
    #[serde(rename = "sessionId")]
    pub sessionId: String,
    ///Actual terminal type used for execution
    #[serde(rename = "terminalType")]
    pub terminalType: TerminalType,
    ///Whether this execution ended due to timeout. On timeout, the current command is cancelled and the terminal session is kept.
    #[serde(rename = "timedOut")]
    pub timedOut: bool,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports the start or an incremental output chunk of a terminal command stream.
pub struct TerminalStreamEventData {
    ///Event type, currently "start" or "chunk"
    #[serde(rename = "type")]
    pub r#type: String,
    ///The command being executed
    #[serde(rename = "command")]
    pub command: String,
    ///ID of the terminal session used for execution
    #[serde(rename = "sessionId")]
    pub sessionId: String,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Incremental output chunk for "chunk" events
    #[serde(rename = "chunk")]
    pub chunk: JsOptional<String>,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Zero-based chunk index
    #[serde(rename = "chunkIndex")]
    pub chunkIndex: JsOptional<i32>,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Total received character count so far
    #[serde(rename = "receivedChars")]
    pub receivedChars: JsOptional<i32>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Records a hidden executor command's interpreter, output, exit code, and timeout state.
pub struct HiddenTerminalCommandResultData {
    ///The command that was executed
    #[serde(rename = "command")]
    pub command: String,
    ///The output from the command execution
    #[serde(rename = "output")]
    pub output: String,
    ///Exit code from the command (0 typically means success)
    #[serde(rename = "exitCode")]
    pub exitCode: i32,
    ///Hidden executor key used for execution
    #[serde(rename = "executorKey")]
    pub executorKey: String,
    ///Actual terminal type used for execution
    #[serde(rename = "terminalType")]
    pub terminalType: TerminalType,
    ///Whether this execution ended due to timeout. On timeout, the current command is cancelled and the hidden executor session is kept.
    #[serde(rename = "timedOut")]
    pub timedOut: bool,
}
#[derive(Clone, Serialize, Deserialize)]
/// Identifies a created or reused named terminal session and its interpreter.
pub struct TerminalSessionCreationResultData {
    ///ID of the created or retrieved session
    #[serde(rename = "sessionId")]
    pub sessionId: String,
    ///Name of the session
    #[serde(rename = "sessionName")]
    pub sessionName: String,
    ///Actual terminal type for the session
    #[serde(rename = "terminalType")]
    pub terminalType: TerminalType,
    ///Whether a new session was created
    #[serde(rename = "isNewSession")]
    pub isNewSession: bool,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports whether a terminal session closed successfully and provides the host message.
pub struct TerminalSessionCloseResultData {
    ///ID of the closed session
    #[serde(rename = "sessionId")]
    pub sessionId: String,
    ///Whether the session was closed successfully
    #[serde(rename = "success")]
    pub success: bool,
    ///A message describing the result
    #[serde(rename = "message")]
    pub message: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Captures one terminal session's visible screen, dimensions, interpreter, and running state.
pub struct TerminalSessionScreenResultData {
    ///ID of the session
    #[serde(rename = "sessionId")]
    pub sessionId: String,
    ///Actual terminal type for the session
    #[serde(rename = "terminalType")]
    pub terminalType: TerminalType,
    ///Screen row count
    #[serde(rename = "rows")]
    pub rows: usize,
    ///Screen column count
    #[serde(rename = "cols")]
    pub cols: usize,
    ///Current visible screen text content
    #[serde(rename = "content")]
    pub content: String,
    ///Whether a command is currently running in this session
    #[serde(rename = "commandRunning")]
    pub commandRunning: bool,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports the active music source, transport state, position, buffering, and volume.
pub struct MusicPlaybackResultData {
    ///Playback state
    #[serde(rename = "state")]
    pub state: String,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Current audio source
    #[serde(rename = "source")]
    pub source: JsOptional<String>,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Current audio source type
    #[serde(rename = "sourceType")]
    pub sourceType: JsOptional<String>,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Display title
    #[serde(rename = "title")]
    pub title: JsOptional<String>,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Display artist
    #[serde(rename = "artist")]
    pub artist: JsOptional<String>,
    #[serde(default, skip_serializing_if = "JsOptional::is_undefined")]
    ///Duration in milliseconds, when known
    #[serde(rename = "durationMs")]
    pub durationMs: JsOptional<i64>,
    ///Current playback position in milliseconds
    #[serde(rename = "positionMs")]
    pub positionMs: i64,
    ///Buffered playback position in milliseconds
    #[serde(rename = "bufferedPositionMs")]
    pub bufferedPositionMs: i64,
    ///Playback volume from 0 to 1
    #[serde(rename = "volume")]
    pub volume: f64,
    ///Whether current track loops
    #[serde(rename = "loop")]
    pub r#loop: bool,
    ///Operation message
    #[serde(rename = "message")]
    pub message: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports whether Bluetooth is supported, enabled, and its current adapter state.
pub struct BluetoothStateData {
    #[serde(rename = "supported")]
    pub supported: bool,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "state")]
    pub state: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Identifies a bonded Bluetooth device and its device and bond classifications.
pub struct BluetoothDeviceData {
    #[serde(rename = "name")]
    pub name: Option<String>,
    #[serde(rename = "address")]
    pub address: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "bondState")]
    pub bondState: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains the devices currently bonded with the Bluetooth adapter.
pub struct BluetoothBondedDevicesData {
    #[serde(rename = "devices")]
    pub devices: Vec<BluetoothDeviceData>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Describes a discovered Bluetooth device, including discovery source and optional signal strength.
pub struct BluetoothScannedDeviceData {
    pub name: Option<String>,
    pub address: String,
    pub r#type: String,
    pub bondState: String,
    #[serde(rename = "source")]
    pub source: String,
    #[serde(rename = "rssi")]
    pub rssi: Option<i32>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains devices discovered during a timed Bluetooth scan and indicates BLE coverage.
pub struct BluetoothScanResultData {
    #[serde(rename = "devices")]
    pub devices: Vec<BluetoothScannedDeviceData>,
    #[serde(rename = "durationMs")]
    pub durationMs: i64,
    #[serde(rename = "includesBle")]
    pub includesBle: bool,
}
#[derive(Clone, Serialize, Deserialize)]
/// Identifies an open Bluetooth session, its remote address, and connection mode.
pub struct BluetoothSessionData {
    #[serde(rename = "sessionId")]
    pub sessionId: String,
    #[serde(rename = "address")]
    pub address: String,
    #[serde(rename = "mode")]
    pub mode: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Reports the number of bytes written through a Bluetooth session.
pub struct BluetoothTransferData {
    #[serde(rename = "sessionId")]
    pub sessionId: String,
    #[serde(rename = "bytesWritten")]
    pub bytesWritten: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains bytes read from a Bluetooth session as optional text and Base64 data.
pub struct BluetoothReadData {
    #[serde(rename = "sessionId")]
    pub sessionId: String,
    #[serde(rename = "bytesRead")]
    pub bytesRead: i64,
    #[serde(rename = "text")]
    pub text: Option<String>,
    #[serde(rename = "dataBase64")]
    pub dataBase64: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Describes a BLE characteristic UUID and the operations it supports.
pub struct BluetoothBleCharacteristicData {
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "properties")]
    pub properties: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Describes a discovered BLE service and its characteristics.
pub struct BluetoothBleServiceData {
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "characteristics")]
    pub characteristics: Vec<BluetoothBleCharacteristicData>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains the BLE services and characteristics discovered for a session.
pub struct BluetoothBleServicesData {
    #[serde(rename = "sessionId")]
    pub sessionId: String,
    #[serde(rename = "services")]
    pub services: Vec<BluetoothBleServiceData>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Carries one timestamped value received from a subscribed BLE characteristic.
pub struct BluetoothBleNotificationEntry {
    #[serde(rename = "characteristicUuid")]
    pub characteristicUuid: String,
    #[serde(rename = "bytesRead")]
    pub bytesRead: i64,
    #[serde(rename = "text")]
    pub text: Option<String>,
    #[serde(rename = "dataBase64")]
    pub dataBase64: Option<String>,
    #[serde(rename = "timestamp")]
    pub timestamp: i64,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains timestamped characteristic notifications received by a BLE session.
pub struct BluetoothBleNotificationData {
    #[serde(rename = "sessionId")]
    pub sessionId: String,
    #[serde(rename = "notifications")]
    pub notifications: Vec<BluetoothBleNotificationEntry>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Contains extracted web-page content, links, images, metadata, and truncation storage details.
pub struct VisitWebResultData {
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "content")]
    pub content: String,
    #[serde(rename = "metadata")]
    pub metadata: HashMap<String, String>,
    #[serde(rename = "links")]
    pub links: Vec<LinkData>,
    #[serde(rename = "imageLinks")]
    pub imageLinks: Vec<String>,
    #[serde(rename = "visitKey")]
    pub visitKey: Option<String>,
    #[serde(rename = "contentSavedTo")]
    pub contentSavedTo: Option<String>,
    #[serde(rename = "contentTruncated")]
    pub contentTruncated: bool,
    #[serde(rename = "originalContentLength")]
    pub originalContentLength: Option<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Describes a URL and the human-readable text associated with it.
pub struct LinkData {
    pub url: String,
    pub text: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains file paths matching a pattern beneath a requested search path.
pub struct FindFilesResultData {
    #[serde(rename = "path")]
    pub path: String,
    #[serde(rename = "pattern")]
    pub pattern: String,
    #[serde(rename = "files")]
    pub files: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Identifies one matching line and its optional surrounding grep context.
pub struct GrepLineMatch {
    #[serde(rename = "lineNumber")]
    pub lineNumber: i32,
    #[serde(rename = "lineContent")]
    pub lineContent: String,
    #[serde(rename = "matchContext")]
    pub matchContext: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Groups grep line matches by source file.
pub struct GrepFileMatch {
    #[serde(rename = "filePath")]
    pub filePath: String,
    #[serde(rename = "lineMatches")]
    pub lineMatches: Vec<GrepLineMatch>,
}
#[derive(Clone, Serialize, Deserialize)]
/// Summarizes a text search across files and groups all reported line matches by file.
pub struct GrepResultData {
    #[serde(rename = "searchPath")]
    pub searchPath: String,
    #[serde(rename = "pattern")]
    pub pattern: String,
    #[serde(rename = "matches")]
    pub matches: Vec<GrepFileMatch>,
    #[serde(rename = "totalMatches")]
    pub totalMatches: i32,
    #[serde(rename = "filesSearched")]
    pub filesSearched: i32,
}
#[derive(Clone, Serialize, Deserialize)]
/// Describes a newly created weighted relationship between two memories.
pub struct MemoryLinkResultData {
    ///The title of the source memory
    #[serde(rename = "sourceTitle")]
    pub sourceTitle: String,
    ///The title of the target memory
    #[serde(rename = "targetTitle")]
    pub targetTitle: String,
    ///The type of link (e.g., "related", "causes", "explains", "part_of")
    #[serde(rename = "linkType")]
    pub linkType: String,
    ///The strength of the link (0.0-1.0)
    #[serde(rename = "weight")]
    pub weight: f32,
    ///Optional description of the link
    #[serde(rename = "description")]
    pub description: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Describes a weighted relationship between two stored memories.
pub struct LinkInfo {
    pub linkId: i64,
    pub sourceTitle: String,
    pub targetTitle: String,
    pub linkType: String,
    pub weight: f32,
    pub description: String,
}
#[derive(Clone, Serialize, Deserialize)]
/// Contains the weighted memory relationships returned by a link query.
pub struct MemoryLinkQueryResultData {
    ///Number of links returned
    #[serde(rename = "totalCount")]
    pub totalCount: i32,
    ///Queried links
    #[serde(rename = "links")]
    pub links: Vec<LinkInfo>,
}
impl SleepResultData {
    /// Formats the actual sleep duration in milliseconds.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        format!("Slept for {}ms", self.sleptMs)
    }
}
impl EnvironmentVariableReadResultData {
    /// Formats the variable as a key-value pair or reports that it is unset.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        if self.exists {
            match &self.value {
                JsOptional::Value(value) => format!("{}={value}", self.key),
                JsOptional::Null => format!("{}=null", self.key),
                JsOptional::Undefined => format!("{} is undefined", self.key),
            }
        } else {
            format!("{} is not set", self.key)
        }
    }
}
impl EnvironmentVariableWriteResultData {
    /// Formats the variable's resulting value or reports that it was cleared.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        if self.cleared {
            format!("{} cleared", self.key)
        } else {
            match &self.value {
                JsOptional::Value(value) => format!("{}={value}", self.key),
                JsOptional::Null => format!("{}=null", self.key),
                JsOptional::Undefined => format!("{} is undefined", self.key),
            }
        }
    }
}
impl FilePartContentData {
    /// Formats the segment number and line range followed by the segment content.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let partInfo = format!(
            "Part {} of {} (Lines {}-{} of {})", self.partIndex + 1, self.totalParts,
            self.startLine + 1, self.endLine, self.totalLines
        );
        format!("{partInfo}\n\n{}", self.content)
    }
}
impl DirectoryListingData {
    /// Formats directory entries with type, permissions, size, modification time, and name.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(&format!("Directory listing for {}:\n", self.path));
        for entry in &self.entries {
            let typeIndicator = if entry.isDirectory { "d" } else { "-" };
            sb.push_str(
                &format!(
                    "{typeIndicator}{} {:>8} {} {}\n", entry.permissions, entry.size,
                    entry.lastModified, entry.name
                ),
            );
        }
        sb
    }
}
impl FileContentData {
    /// Formats the file path followed by its text content.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        format!("Content of {}:\n{}", self.path, self.content)
    }
}
impl BinaryFileContentData {
    /// Summarizes the binary file path, byte size, and Base64 character count.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        format!(
            "Binary content of {} ({} bytes, base64 length={})", self.path, self.size,
            self.contentBase64.chars().count()
        )
    }
}
impl FileExistsData {
    /// Reports whether the path exists, its kind, and its byte size.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        if self.exists {
            let fileType = if self.isDirectory { "Directory" } else { "File" };
            format!(
                "{fileType} exists at path: {} (size: {} bytes)", self.path, self.size
            )
        } else {
            format!("No file or directory exists at path: {}", self.path)
        }
    }
}
impl FileInfoData {
    /// Formats the path's type, size, permissions, ownership, and modification time.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        if !self.exists {
            return format!("File or directory does not exist at path: {}", self.path);
        }
        let mut sb = String::new();
        sb.push_str(&format!("File information for {}:\n", self.path));
        sb.push_str(&format!("Type: {}\n", self.fileType));
        sb.push_str(&format!("Size: {} bytes\n", self.size));
        sb.push_str(&format!("Permissions: {}\n", self.permissions));
        sb.push_str(&format!("Owner: {}\n", self.owner));
        sb.push_str(&format!("Group: {}\n", self.group));
        sb.push_str(&format!("Last modified: {}\n", self.lastModified));
        sb
    }
}
impl FileOperationData {
    /// Returns the operation details supplied by the file-system host.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        self.details.clone()
    }
}
impl FileApplyResultData {
    /// Formats the file operation with diff markup, request context, and generated instructions.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(&self.operation.toString());
        sb.push('\n');
        if let Some(diffContent) = &self.diffContent {
            sb.push_str(
                &format!(
                    "<file-diff path=\"{}\" details=\"{}\"><![CDATA[{}]]></file-diff>",
                    self.operation.path, self.operation.details, diffContent
                ),
            );
        }
        let requestContent = self.buildRequestContent();
        if !requestContent.trim().is_empty() {
            sb.push_str(
                &format!(
                    "<file-request-content><![CDATA[{requestContent}]]></file-request-content>"
                ),
            );
        }
        if !self.aiDiffInstructions.is_empty()
            && !self.aiDiffInstructions.starts_with("Error")
        {
            sb.push_str("\n--- AI-Generated Diff ---\n");
            sb.push_str(&self.aiDiffInstructions);
            sb.push('\n');
        }
        sb
    }
    /// Builds request context from the operation details and detected change summary.
    #[allow(non_snake_case)]
    fn buildRequestContent(&self) -> String {
        let mut sections = vec![self.operation.toString()];
        if let Some(summary) = self.extractDiffSummaryLine() {
            sections.push(summary);
        }
        sections.join("\n")
    }
    /// Extracts the first recognized change-summary line from the diff or generated instructions.
    #[allow(non_snake_case)]
    fn extractDiffSummaryLine(&self) -> Option<String> {
        for candidate in self
            .diffContent
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(self.aiDiffInstructions.as_str()))
        {
            for line in candidate.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Changes: +")
                    || trimmed
                        .eq_ignore_ascii_case(
                            "No changes detected (files are identical)",
                        )
                {
                    return Some(trimmed.to_string());
                }
            }
        }
        None
    }
}
impl HttpResponseData {
    /// Formats response metadata, a bounded cookie preview, and the decoded body content.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("HTTP Response:\n");
        sb.push_str(&format!("URL: {}\n", self.url));
        sb.push_str(&format!("Status: {} {}\n", self.statusCode, self.statusMessage));
        sb.push_str(&format!("Content-Type: {}\n", self.contentType));
        sb.push_str(&format!("Size: {} bytes\n", self.size));
        if !self.cookies.is_empty() {
            sb.push_str(&format!("Cookies: {}\n", self.cookies.len()));
            let mut entries = self.cookies.iter().collect::<Vec<_>>();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (name, value) in entries.into_iter().take(5) {
                let preview = value.chars().take(30).collect::<String>();
                let suffix = if value.chars().count() > 30 { "..." } else { "" };
                sb.push_str(&format!("  {name}: {preview}{suffix}\n"));
            }
            if self.cookies.len() > 5 {
                sb.push_str(
                    &format!("  ... and {} more cookies\n", self.cookies.len() - 5),
                );
            }
        }
        sb.push('\n');
        sb.push_str("Content Summary:\n");
        sb.push_str(&self.content);
        sb
    }
}
impl HttpStreamEventData {
    /// Returns chunk content directly or formats the stream-start status and other event kinds.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        match self.r#type.as_str() {
            "chunk" => self.chunk.clone().unwrap_or_default(),
            "response_started" => {
                let statusCode = self
                    .statusCode
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let statusMessage = self.statusMessage.clone().unwrap_or_default();
                format!("HTTP stream started: {statusCode} {statusMessage}")
                    .trim()
                    .to_string()
            }
            value => format!("HTTP stream event: {value}"),
        }
    }
}
impl SystemSettingData {
    /// Formats the namespaced setting name and its current value.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        format!("Current value of {}.{}: {}", self.namespace, self.setting, self.value)
    }
}
impl AppOperationData {
    /// Formats a successful package operation or returns the host-provided operation details.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        match self.operationType.as_str() {
            "install" => {
                format!(
                    "Successfully installed app: {} {}", self.packageName, self.details
                )
            }
            "uninstall" => {
                format!(
                    "Successfully uninstalled app: {} {}", self.packageName, self.details
                )
            }
            "start" => {
                format!(
                    "Successfully started app: {} {}", self.packageName, self.details
                )
            }
            "stop" => {
                format!(
                    "Successfully stopped app: {} {}", self.packageName, self.details
                )
            }
            _ => self.details.clone(),
        }
    }
}
impl AppListData {
    /// Formats installed package names and indicates whether the list includes system apps.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let appType = if self.includesSystemApps {
            "All Apps"
        } else {
            "Third-Party Apps"
        };
        format!("Installed {appType} List:\n{}", self.packages.join("\n"))
    }
}
impl AppUsageTimeResultData {
    /// Formats the query window and each application's human-readable foreground duration.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut header = format!("App usage time (last {}h)", self.sinceHours);
        if let Some(packageName) = self
            .requestedPackageName
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            header.push_str(&format!(" for {packageName}"));
        }
        if self.entries.is_empty() {
            return format!("{header}\nNo app usage found in the selected time window.");
        }
        let lines = self
            .entries
            .iter()
            .map(|entry| {
                format!(
                    "- {} ({}): {}", entry.appName, entry.packageName,
                    formatDuration(entry.totalForegroundTimeMs)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("{header}\n{lines}")
    }
}
impl NotificationData {
    #[allow(non_snake_case)]
    /// Formats a numbered list of notification package names and text content.
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(
            &format!("Device Notifications ({} total):\n", self.notifications.len()),
        );
        for (index, notification) in self.notifications.iter().enumerate() {
            sb.push_str(
                &format!("{}. Package: {}\n", index + 1, notification.packageName),
            );
            sb.push_str(&format!("   Content: {}\n\n", notification.text));
        }
        if self.notifications.is_empty() {
            sb.push_str("No notifications\n");
        }
        sb
    }
}
impl LocationData {
    #[allow(non_snake_case)]
    /// Formats coordinates, accuracy, provider, local timestamp, and available address fields.
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Device Location Information:\n");
        sb.push_str(&format!("Longitude: {}\n", self.longitude));
        sb.push_str(&format!("Latitude: {}\n", self.latitude));
        sb.push_str(&format!("Accuracy: {} meters\n", self.accuracy));
        sb.push_str(&format!("Provider: {}\n", self.provider));
        sb.push_str(&format!("Timestamp: {}\n", formatTimestamp(self.timestamp)));
        if !self.address.is_empty() {
            sb.push_str(&format!("Address: {}\n", self.address));
        }
        if !self.city.is_empty() {
            sb.push_str(&format!("City: {}\n", self.city));
        }
        if !self.province.is_empty() {
            sb.push_str(&format!("Province/State: {}\n", self.province));
        }
        if !self.country.is_empty() {
            sb.push_str(&format!("Country: {}\n", self.country));
        }
        sb
    }
}
impl DeviceInfoResultData {
    /// Formats a labeled report of device hardware, Android, storage, power, and network details.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Device Information:\n");
        sb.push_str(&format!("Device ID: {}\n", self.deviceId));
        sb.push_str(&format!("Model: {}\n", self.model));
        sb.push_str(&format!("Manufacturer: {}\n", self.manufacturer));
        sb.push_str(&format!("Android Version: {}\n", self.androidVersion));
        sb.push_str(&format!("SDK Version: {}\n", self.sdkVersion));
        sb.push_str(&format!("Screen Resolution: {}\n", self.screenResolution));
        sb.push_str(&format!("Screen Density: {}\n", self.screenDensity));
        sb.push_str(&format!("Total Memory: {}\n", self.totalMemory));
        sb.push_str(&format!("Available Memory: {}\n", self.availableMemory));
        sb.push_str(&format!("Total Storage: {}\n", self.totalStorage));
        sb.push_str(&format!("Available Storage: {}\n", self.availableStorage));
        sb.push_str(&format!("Battery Level: {}%\n", self.batteryLevel));
        sb.push_str(&format!("Battery Charging: {}\n", self.batteryCharging));
        sb.push_str(&format!("CPU Info: {}\n", self.cpuInfo));
        sb.push_str(&format!("Network Type: {}\n", self.networkType));
        if !self.additionalInfo.is_empty() {
            sb.push_str("Additional Info:\n");
            for (key, value) in &self.additionalInfo {
                sb.push_str(&format!("- {key}: {value}\n"));
            }
        }
        sb
    }
}
impl MemoryQueryResultData {
    #[allow(non_snake_case)]
    /// Formats snapshot de-duplication metadata followed by the matched memory records.
    pub fn toString(&self) -> String {
        let mut snapshotSummary = Vec::new();
        if let Some(snapshotId) = self
            .snapshotId
            .as_value()
            .filter(|value| !value.trim().is_empty())
        {
            snapshotSummary.push(format!("Snapshot ID: {snapshotId}"));
        }
        if self.snapshotCreated.is_some_and(|created| created) {
            snapshotSummary.push("Snapshot created: true".to_string());
        }
        if let Some(excluded_count) = self
            .excludedBySnapshotCount
            .filter(|count| *count > 0)
        {
            snapshotSummary.push(format!("Excluded by snapshot: {excluded_count}"));
        }
        let snapshotSummary = snapshotSummary.join("\n");
        if self.memories.is_empty() {
            return if snapshotSummary.trim().is_empty() {
                "No relevant memories found.".to_string()
            } else {
                format!("{snapshotSummary}\nNo relevant memories found.")
            };
        }
        let memoryText = self
            .memories
            .iter()
            .map(|memory| {
                format!(
                    "Owner: {}\nTitle: {}\nContent: {}\nSource: {}\nTags: {}\nCreated: {}",
                    memory.ownerKey, memory.title, memory.content, memory.source, memory
                    .tags.join(", "), memory.createdAt
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n");
        if snapshotSummary.trim().is_empty() {
            memoryText
        } else {
            format!("{snapshotSummary}\n---\n{memoryText}")
        }
    }
}
impl ChatListResultData {
    #[allow(non_snake_case)]
    /// Formats chat summaries, marks the current chat, and includes token and card metadata.
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(&format!("Chat List ({} total):\n", self.totalCount));
        if let Some(currentChatId) = self.currentChatId.as_value() {
            sb.push_str(&format!("Current Chat ID: {currentChatId}\n"));
        }
        sb.push('\n');
        if self.chats.is_empty() {
            sb.push_str("No chats\n");
        } else {
            for chat in &self.chats {
                let currentMarker = if chat.isCurrent { " [Current]" } else { "" };
                sb.push_str(&format!("ID: {}{}\n", chat.id, currentMarker));
                sb.push_str(&format!("Title: {}\n", chat.title));
                sb.push_str(&format!("Message Count: {}\n", chat.messageCount));
                if let Some(characterCardName) = chat.characterCardName.as_value() {
                    if !characterCardName.trim().is_empty() {
                        sb.push_str(&format!("Character Card: {characterCardName}\n"));
                    }
                }
                sb.push_str(
                    &format!(
                        "Token Statistics: Input {} / Output {}\n", chat.inputTokens,
                        chat.outputTokens
                    ),
                );
                sb.push_str(&format!("Created: {}\n", chat.createdAt));
                sb.push_str(&format!("Updated: {}\n", chat.updatedAt));
                sb.push_str("---\n");
            }
        }
        sb.trim().to_string()
    }
}
impl ChatFindResultData {
    #[allow(non_snake_case)]
    /// Reports the selected chat identifier and total match count, or that no chat was found.
    pub fn toString(&self) -> String {
        match &self.chat {
            JsNullable::Value(chat) => {
                format!("Found chat ({}) (matched={})", chat.id, self.matchedCount)
            }
            JsNullable::Null => format!("No chat found (matched={})", self.matchedCount),
        }
    }
}
impl AgentStatusResultData {
    #[allow(non_snake_case)]
    /// Formats the chat agent's state with its optional detail message.
    pub fn toString(&self) -> String {
        let detail = match &self.message {
            JsOptional::Value(message) if !message.trim().is_empty() => {
                format!(" ({message})")
            }
            JsOptional::Value(_) | JsOptional::Null | JsOptional::Undefined => {
                String::new()
            }
        };
        format!("Chat {} status: {}{}", self.chatId, self.state, detail)
    }
}
impl ChatSwitchResultData {
    #[allow(non_snake_case)]
    /// Formats the selected chat title and identifier.
    pub fn toString(&self) -> String {
        if !self.chatTitle.trim().is_empty() {
            format!("Switched to chat: {}\nChat ID: {}", self.chatTitle, self.chatId)
        } else {
            format!("Switched to chat: {}", self.chatId)
        }
    }
}
impl MessageSendResultData {
    #[allow(non_snake_case)]
    /// Formats bounded previews of the sent message and optional AI reply.
    pub fn toString(&self) -> String {
        let messagePreview = if self.message.chars().count() > 50 {
            format!("{}...", self.message.chars().take(50).collect::< String > ())
        } else {
            self.message.clone()
        };
        match &self.aiResponse {
            JsOptional::Value(response) if !response.trim().is_empty() => {
                let responsePreview = if response.chars().count() > 200 {
                    format!("{}...", response.chars().take(200).collect::< String > ())
                } else {
                    response.clone()
                };
                format!(
                    "Message sent to chat: {}\nMessage content: {}\nAI Reply: {}", self
                    .chatId, messagePreview, responsePreview
                )
            }
            JsOptional::Value(_) | JsOptional::Null | JsOptional::Undefined => {
                format!(
                    "Message sent to chat: {}\nMessage content: {}", self.chatId,
                    messagePreview
                )
            }
        }
    }
}
impl CharacterCardListResultData {
    /// Formats character card metadata and marks the default card.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(&format!("Character Cards ({} total):\n", self.totalCount));
        if self.cards.is_empty() {
            sb.push_str("No cards\n");
        } else {
            for card in &self.cards {
                let defaultMarker = if card.isDefault { " [Default]" } else { "" };
                sb.push_str(&format!("ID: {}{}\n", card.id, defaultMarker));
                sb.push_str(&format!("Name: {}\n", card.name));
                if !card.description.trim().is_empty() {
                    sb.push_str(&format!("Description: {}\n", card.description));
                }
                sb.push_str(&format!("Created: {}\n", card.createdAt));
                sb.push_str(&format!("Updated: {}\n", card.updatedAt));
                sb.push_str("---\n");
            }
        }
        sb.trim().to_string()
    }
}
impl TerminalInfoResultData {
    #[allow(non_snake_case)]
    /// Formats the platform, default terminal, and availability of each known terminal type.
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Terminal Info:\n");
        sb.push_str(&format!("Platform: {}\n", self.platform));
        sb.push_str(&format!("Default Type: {}\n", self.defaultType));
        if !self.types.is_empty() {
            sb.push_str("Types:\n");
            for terminalType in &self.types {
                sb.push_str(
                    &format!(
                        "- {}: available={} ({})\n", terminalType.terminalType,
                        terminalType.available, terminalType.description
                    ),
                );
            }
        }
        sb.trim_end().to_string()
    }
}
impl TerminalCommandResultData {
    #[allow(non_snake_case)]
    /// Formats command, session, interpreter, exit, timeout, and captured output details.
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Terminal Command Execution Result:\n");
        sb.push_str(&format!("Command: {}\n", self.command));
        sb.push_str(&format!("Session: {}\n", self.sessionId));
        sb.push_str(&format!("Terminal Type: {}\n", self.terminalType));
        sb.push_str(&format!("Exit Code: {}\n", self.exitCode));
        if self.timedOut {
            sb.push_str("Timed Out: true\n");
        }
        sb.push_str("\nOutput:\n");
        sb.push_str(&self.output);
        sb.push('\n');
        sb
    }
}
impl TerminalStreamEventData {
    #[allow(non_snake_case)]
    /// Returns an output chunk directly or labels the stream lifecycle event.
    pub fn toString(&self) -> String {
        match self.r#type.as_str() {
            "chunk" => {
                match &self.chunk {
                    JsOptional::Value(chunk) => chunk.clone(),
                    JsOptional::Null => "null".to_string(),
                    JsOptional::Undefined => "undefined".to_string(),
                }
            }
            "start" => "Terminal stream started".to_string(),
            value => format!("Terminal stream event: {value}"),
        }
    }
}
impl HiddenTerminalCommandResultData {
    #[allow(non_snake_case)]
    /// Formats command, executor, interpreter, exit, timeout, and captured output details.
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Hidden Terminal Command Execution Result:\n");
        sb.push_str(&format!("Command: {}\n", self.command));
        sb.push_str(&format!("Executor Key: {}\n", self.executorKey));
        sb.push_str(&format!("Terminal Type: {}\n", self.terminalType));
        sb.push_str(&format!("Exit Code: {}\n", self.exitCode));
        if self.timedOut {
            sb.push_str("Timed Out: true\n");
        }
        sb.push_str("\nOutput:\n");
        sb.push_str(&self.output);
        sb.push('\n');
        sb
    }
}
impl TerminalSessionCreationResultData {
    /// Reports whether the named terminal session was created or reused and identifies it.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        if self.isNewSession {
            format!(
                "Successfully created new terminal session. Session Name: '{}', Session ID: {}",
                self.sessionName, self.sessionId
            )
        } else {
            format!(
                "Successfully retrieved existing terminal session. Session Name: '{}', Session ID: {}",
                self.sessionName, self.sessionId
            )
        }
    }
}
impl TerminalSessionScreenResultData {
    #[allow(non_snake_case)]
    /// Formats screen dimensions and session state followed by the visible terminal content.
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Terminal Session Screen Snapshot:\n");
        sb.push_str(&format!("Session: {}\n", self.sessionId));
        sb.push_str(&format!("Terminal Type: {}\n", self.terminalType));
        sb.push_str(&format!("Size: {}x{}\n", self.cols, self.rows));
        sb.push_str(&format!("Command Running: {}\n", self.commandRunning));
        sb.push('\n');
        sb.push_str(&self.content);
        sb
    }
}
impl MusicPlaybackResultData {
    #[allow(non_snake_case)]
    /// Formats playback state, track metadata, timing, buffering, volume, looping, and message.
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Music Playback:\n");
        sb.push_str(&format!("State: {}\n", self.state));
        if let Some(title) = self.title.as_value() {
            if !title.trim().is_empty() {
                sb.push_str(&format!("Title: {title}\n"));
            }
        }
        if let Some(artist) = self.artist.as_value() {
            if !artist.trim().is_empty() {
                sb.push_str(&format!("Artist: {artist}\n"));
            }
        }
        if let Some(source) = self.source.as_value() {
            if !source.trim().is_empty() {
                sb.push_str(&format!("Source: {source}\n"));
            }
        }
        if let Some(sourceType) = self.sourceType.as_value() {
            if !sourceType.trim().is_empty() {
                sb.push_str(&format!("Source Type: {sourceType}\n"));
            }
        }
        sb.push_str(&format!("Position: {}ms\n", self.positionMs));
        if let Some(durationMs) = self.durationMs.as_value() {
            sb.push_str(&format!("Duration: {durationMs}ms\n"));
        }
        sb.push_str(&format!("Buffered Position: {}ms\n", self.bufferedPositionMs));
        sb.push_str(&format!("Volume: {:.2}\n", self.volume));
        sb.push_str(&format!("Loop: {}\n", self.r#loop));
        if !self.message.trim().is_empty() {
            sb.push_str(&format!("Message: {}\n", self.message));
        }
        sb.trim_end().to_string()
    }
}
impl BluetoothStateData {
    /// Formats Bluetooth support, enablement, and adapter state.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        format!(
            "Bluetooth State:\nSupported: {}\nEnabled: {}\nState: {}", self.supported,
            self.enabled, self.state
        )
    }
}
impl BluetoothBondedDevicesData {
    /// Formats each bonded device's name, address, type, and bond state.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(&format!("Bluetooth Bonded Devices ({}):\n", self.devices.len()));
        for device in &self.devices {
            sb.push_str(
                &format!(
                    "- {} [{}] type={} bond={}\n", device.name.as_deref()
                    .unwrap_or("Unnamed"), device.address, device.r#type, device
                    .bondState
                ),
            );
        }
        sb.trim_end().to_string()
    }
}
impl BluetoothScanResultData {
    /// Formats scan duration and BLE coverage followed by every discovered device.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(
            &format!(
                "Bluetooth Scan Result ({} devices, {}ms, includes BLE={}):\n", self
                .devices.len(), self.durationMs, self.includesBle
            ),
        );
        for device in &self.devices {
            let rssi = device
                .rssi
                .map(|value| format!(" rssi={value}"))
                .unwrap_or_default();
            sb.push_str(
                &format!(
                    "- {} [{}] type={} bond={} source={}{}\n", device.name.as_deref()
                    .unwrap_or("Unnamed"), device.address, device.r#type, device
                    .bondState, device.source, rssi
                ),
            );
        }
        sb.trim_end().to_string()
    }
}
impl BluetoothSessionData {
    /// Formats the Bluetooth session identifier, remote address, and connection mode.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        format!(
            "Bluetooth Session:\nSession ID: {}\nAddress: {}\nMode: {}", self.sessionId,
            self.address, self.mode
        )
    }
}
impl BluetoothTransferData {
    /// Formats the Bluetooth session identifier and number of bytes written.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        format!(
            "Bluetooth Transfer:\nSession ID: {}\nBytes Written: {}", self.sessionId,
            self.bytesWritten
        )
    }
}
impl BluetoothReadData {
    /// Formats the byte count and available text or Base64 payload read from the session.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Bluetooth Read:\n");
        sb.push_str(&format!("Session ID: {}\n", self.sessionId));
        sb.push_str(&format!("Bytes Read: {}\n", self.bytesRead));
        if let Some(text) = &self.text {
            if !text.is_empty() {
                sb.push_str("Text:\n");
                sb.push_str(text);
                sb.push('\n');
            }
        }
        if let Some(dataBase64) = &self.dataBase64 {
            if !dataBase64.is_empty() {
                sb.push_str(&format!("Data Base64: {dataBase64}\n"));
            }
        }
        sb.trim_end().to_string()
    }
}
impl BluetoothBleServicesData {
    /// Formats discovered BLE services and the properties of each characteristic.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(
            &format!(
                "BLE Services for session {} ({} services):\n", self.sessionId, self
                .services.len()
            ),
        );
        for service in &self.services {
            sb.push_str(&format!("- Service {}\n", service.uuid));
            for characteristic in &service.characteristics {
                sb.push_str(
                    &format!(
                        "  - Characteristic {} [{}]\n", characteristic.uuid,
                        characteristic.properties.join(", ")
                    ),
                );
            }
        }
        sb.trim_end().to_string()
    }
}
impl BluetoothBleNotificationData {
    /// Formats each received BLE notification with its source, timestamp, and available payload.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(
            &format!(
                "BLE Notifications for session {} ({}):\n", self.sessionId, self
                .notifications.len()
            ),
        );
        for item in &self.notifications {
            sb.push_str(
                &format!(
                    "- {} bytes from {} at {}\n", item.bytesRead, item
                    .characteristicUuid, item.timestamp
                ),
            );
            if let Some(text) = &item.text {
                if !text.is_empty() {
                    sb.push_str(&format!("  Text: {text}\n"));
                }
            }
            if let Some(dataBase64) = &item.dataBase64 {
                if !dataBase64.is_empty() {
                    sb.push_str(&format!("  Data Base64: {dataBase64}\n"));
                }
            }
        }
        sb.trim_end().to_string()
    }
}
impl FindFilesResultData {
    /// Formats search parameters and a bounded preview of matched file paths.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("File Search Result:\n");
        sb.push_str(&format!("Search Path: {}\n", self.path));
        sb.push_str(&format!("Pattern: {}\n", self.pattern));
        sb.push_str(&format!("Found {} files:\n", self.files.len()));
        for (index, file) in self.files.iter().enumerate() {
            if index < 10 || self.files.len() <= 20 {
                sb.push_str(&format!("- {file}\n"));
            } else if index == 10 && self.files.len() > 20 {
                sb.push_str(&format!("... and {} other files\n", self.files.len() - 10));
            }
        }
        sb
    }
}
impl GrepResultData {
    /// Formats search statistics and bounded, line-aware match context grouped by file.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Grep Search Result:\n");
        sb.push_str(&format!("Search Path: {}\n", self.searchPath));
        sb.push_str(&format!("Pattern: {}\n", self.pattern));
        sb.push_str(
            &format!(
                "Total Matches: {} (in {} files)\n", self.totalMatches, self.matches
                .len()
            ),
        );
        sb.push_str(&format!("Files Searched: {}\n\n", self.filesSearched));
        if self.matches.is_empty() {
            sb.push_str("No matches found\n");
        } else {
            let maxDisplayMatches = 30usize;
            let mut displayedMatches = 0usize;
            let mut collapsedMatches = 0usize;
            for fileMatch in &self.matches {
                let remainingSlots = maxDisplayMatches.saturating_sub(displayedMatches);
                if remainingSlots == 0 {
                    collapsedMatches += fileMatch.lineMatches.len();
                    continue;
                }
                sb.push_str(&format!("File: {}\n", fileMatch.filePath));
                let matchesToShow = fileMatch
                    .lineMatches
                    .iter()
                    .take(remainingSlots)
                    .collect::<Vec<_>>();
                let matchesCollapsedInFile = fileMatch
                    .lineMatches
                    .len()
                    .saturating_sub(matchesToShow.len());
                for lineMatch in matchesToShow {
                    match &lineMatch.matchContext {
                        Some(context) if !context.trim().is_empty() => {
                            let contextLines = context.lines().collect::<Vec<_>>();
                            let isPreNumberedContext = contextLines
                                .iter()
                                .any(|line| !line.trim().is_empty())
                                && contextLines
                                    .iter()
                                    .all(|line| {
                                        line.trim().is_empty()
                                            || parsePreNumberedLineNumber(line).is_some()
                                    });
                            if isPreNumberedContext {
                                for contextLine in contextLines {
                                    let renderedLine = if parsePreNumberedLineNumber(
                                        contextLine,
                                    ) == Some(lineMatch.lineNumber)
                                    {
                                        markPreNumberedContextLine(contextLine)
                                    } else {
                                        contextLine.to_string()
                                    };
                                    sb.push_str(&renderedLine);
                                    sb.push('\n');
                                }
                            } else {
                                let centerIndex = (contextLines.len() / 2) as i32;
                                for (index, contextLine) in contextLines.iter().enumerate()
                                {
                                    let actualLineNumber = lineMatch.lineNumber - centerIndex
                                        + index as i32;
                                    if index as i32 == centerIndex {
                                        sb.push_str(
                                            &format!("{actualLineNumber:>6}|>{contextLine}\n"),
                                        );
                                    } else {
                                        sb.push_str(
                                            &format!("{actualLineNumber:>6}| {contextLine}\n"),
                                        );
                                    }
                                }
                            }
                            sb.push('\n');
                        }
                        _ => {
                            sb.push_str(
                                &format!(
                                    "{:>6}| {}\n", lineMatch.lineNumber, lineMatch.lineContent
                                ),
                            );
                        }
                    }
                    displayedMatches += 1;
                }
                if matchesCollapsedInFile > 0 {
                    sb.push_str(
                        &format!(
                            "  ... ({matchesCollapsedInFile} more match groups collapsed in this file)\n"
                        ),
                    );
                    collapsedMatches += matchesCollapsedInFile;
                }
                sb.push('\n');
            }
            if collapsedMatches > 0 {
                sb.push_str(&format!("{}\n", "=".repeat(60)));
                sb.push_str(
                    &format!(
                        "To save space, {collapsedMatches} match groups were collapsed\n"
                    ),
                );
                sb.push_str(
                    &format!(
                        "Displayed {displayedMatches} match groups, total {} matches\n",
                        self.totalMatches
                    ),
                );
            }
        }
        sb
    }
}
impl MemoryLinkResultData {
    #[allow(non_snake_case)]
    /// Formats the source, target, relationship type, and strength of the created memory link.
    pub fn toString(&self) -> String {
        format!(
            "Successfully linked memory: '{}' -> '{}' (Type: {}, Strength: {})", self
            .sourceTitle, self.targetTitle, self.linkType, self.weight
        )
    }
}
impl MemoryLinkQueryResultData {
    #[allow(non_snake_case)]
    /// Formats each queried memory link with identifiers, relationship metadata, and description.
    pub fn toString(&self) -> String {
        if self.links.is_empty() {
            return "No memory links found.".to_string();
        }
        let mut sb = String::new();
        sb.push_str(&format!("Memory Links ({}):\n", self.totalCount));
        for link in &self.links {
            sb.push_str(
                &format!(
                    "- #{}: '{}' -> '{}' (Type: {}, Weight: {})\n", link.linkId, link
                    .sourceTitle, link.targetTitle, link.linkType, link.weight
                ),
            );
            if !link.description.trim().is_empty() {
                sb.push_str(&format!("  Description: {}\n", link.description));
            }
        }
        sb.trim().to_string()
    }
}
impl VisitWebResultData {
    /// Formats bounded link and image previews, persisted-content metadata, and page content.
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        const MAX_INLINE_LINKS: usize = 120;
        const MAX_INLINE_IMAGES: usize = 120;
        let mut sb = String::new();
        if let Some(visitKey) = &self.visitKey {
            sb.push_str(&format!("Visit key: {visitKey}\n\n"));
        }
        if !self.links.is_empty() {
            sb.push_str("Results:\n");
            for (index, link) in self.links.iter().take(MAX_INLINE_LINKS).enumerate() {
                sb.push_str(&format!("[{}] {}\n", index + 1, link.text));
            }
            let omittedCount = self.links.len().saturating_sub(MAX_INLINE_LINKS);
            if omittedCount > 0 {
                sb.push_str(
                    &format!(
                        "... ({omittedCount} more links omitted from inline preview)\n"
                    ),
                );
            }
            sb.push('\n');
        }
        if !self.imageLinks.is_empty() {
            sb.push_str("Images:\n");
            for (index, link) in self
                .imageLinks
                .iter()
                .take(MAX_INLINE_IMAGES)
                .enumerate()
            {
                let name = link
                    .rsplit('/')
                    .next()
                    .and_then(|part| part.split('?').next())
                    .filter(|part| !part.is_empty())
                    .unwrap_or("image");
                sb.push_str(&format!("[{}] {}\n", index + 1, name));
            }
            let omittedCount = self.imageLinks.len().saturating_sub(MAX_INLINE_IMAGES);
            if omittedCount > 0 {
                sb.push_str(
                    &format!(
                        "... ({omittedCount} more images omitted from inline preview)\n"
                    ),
                );
            }
            sb.push('\n');
        }
        if let Some(savedTo) = &self.contentSavedTo {
            sb.push_str(&format!("Full content saved to file: {savedTo}\n"));
            if let Some(totalChars) = self.originalContentLength {
                sb.push_str(&format!("Original content length: {totalChars} chars\n"));
            }
            if self.contentTruncated {
                sb.push_str(
                    "Use read_file_part or grep_code to inspect the saved file.\n",
                );
            }
            sb.push('\n');
        }
        if self.contentTruncated {
            sb.push_str("Content Preview:\n");
        } else {
            sb.push_str("Content:\n");
        }
        sb.push_str(&self.content);
        sb
    }
}
/// Parses the line number before the first pipe separator in pre-numbered context.
#[allow(non_snake_case)]
fn parsePreNumberedLineNumber(line: &str) -> Option<i32> {
    let trimmed = line.trim_start();
    let separatorIndex = trimmed.find('|')?;
    if separatorIndex == 0 {
        return None;
    }
    trimmed[..separatorIndex].trim().parse::<i32>().ok()
}
/// Marks a pre-numbered context line as the active match after its pipe separator.
#[allow(non_snake_case)]
fn markPreNumberedContextLine(line: &str) -> String {
    let Some(separatorIndex) = line.find('|') else {
        return line.to_string();
    };
    if line.as_bytes().get(separatorIndex + 1).is_some_and(|value| *value == b'>') {
        return line.to_string();
    }
    let mut output = String::with_capacity(line.len() + 1);
    output.push_str(&line[..separatorIndex + 1]);
    output.push('>');
    output.push_str(&line[separatorIndex + 1..]);
    output
}
/// Formats a millisecond duration as compact hours, minutes, and seconds.
#[allow(non_snake_case)]
fn formatDuration(durationMs: i64) -> String {
    if durationMs <= 0 {
        return "0s".to_string();
    }
    let totalSeconds = durationMs / 1000;
    let hours = totalSeconds / 3600;
    let minutes = (totalSeconds % 3600) / 60;
    let seconds = totalSeconds % 60;
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{seconds}s"));
    }
    parts.join(" ")
}
/// Formats a Unix millisecond timestamp in local `YYYY-MM-DD HH:MM:SS` time.
#[allow(non_snake_case)]
fn formatTimestamp(timestamp: i64) -> String {
    Local
        .timestamp_millis_opt(timestamp)
        .single()
        .expect("valid timestamp millis")
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}
