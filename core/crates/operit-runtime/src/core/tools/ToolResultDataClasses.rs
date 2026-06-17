use std::collections::{BTreeMap, HashMap};

use chrono::{Local, TimeZone};
use operit_host_api::WebVisitLinkData;
use serde::{Deserialize, Serialize};

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
    FindFilesResultData(FindFilesResultData),
    GrepResultData(GrepResultData),
    MemoryLinkResultData(MemoryLinkResultData),
    MemoryLinkQueryResultData(MemoryLinkQueryResultData),
}

impl ToolResultData {
    #[allow(non_snake_case)]
    pub fn toJson(&self) -> String {
        serde_json::to_string(self).expect("ToolResultData serialization failed")
    }

    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        match self {
            ToolResultData::BooleanResultData(data) => data.value.to_string(),
            ToolResultData::StringResultData(data) => data.value.clone(),
            ToolResultData::SleepResultData(data) => data.toString(),
            ToolResultData::EnvironmentVariableReadResultData(data) => data.toString(),
            ToolResultData::EnvironmentVariableWriteResultData(data) => data.toString(),
            ToolResultData::IntResultData(data) => data.value.to_string(),
            ToolResultData::BinaryResultData(data) => {
                format!("Binary data ({} bytes)", data.value.len())
            }
            ToolResultData::FilePartContentData(data) => data.toString(),
            ToolResultData::DirectoryListingData(data) => data.toString(),
            ToolResultData::FileContentData(data) => data.toString(),
            ToolResultData::BinaryFileContentData(data) => data.toString(),
            ToolResultData::FileExistsData(data) => data.toString(),
            ToolResultData::FileInfoData(data) => data.toString(),
            ToolResultData::FileOperationData(data) => data.toString(),
            ToolResultData::FileApplyResultData(data) => data.toString(),
            ToolResultData::HttpResponseData(data) => data.toString(),
            ToolResultData::HttpStreamEventData(data) => data.toString(),
            ToolResultData::SystemSettingData(data) => data.toString(),
            ToolResultData::AppOperationData(data) => data.toString(),
            ToolResultData::AppListData(data) => data.toString(),
            ToolResultData::AppUsageTimeResultData(data) => data.toString(),
            ToolResultData::NotificationData(data) => data.toString(),
            ToolResultData::LocationData(data) => data.toString(),
            ToolResultData::DeviceInfoResultData(data) => data.toString(),
            ToolResultData::MemoryQueryResultData(data) => data.toString(),
            ToolResultData::ChatServiceStartResultData(data) => {
                if data.isConnected {
                    "Chat service started and connected successfully".to_string()
                } else {
                    "Chat service connection failed".to_string()
                }
            }
            ToolResultData::ChatCreationResultData(data) => {
                format!("Created new chat\nChat ID: {}", data.chatId)
            }
            ToolResultData::ChatListResultData(data) => data.toString(),
            ToolResultData::ChatFindResultData(data) => data.toString(),
            ToolResultData::AgentStatusResultData(data) => data.toString(),
            ToolResultData::ChatSwitchResultData(data) => data.toString(),
            ToolResultData::ChatTitleUpdateResultData(data) => {
                format!("Updated chat title: {} -> {}", data.chatId, data.title)
            }
            ToolResultData::ChatDeleteResultData(data) => {
                format!("Deleted chat: {}", data.chatId)
            }
            ToolResultData::MessageSendResultData(data) => data.toString(),
            ToolResultData::ChatMessagesResultData(data) => {
                format!(
                    "Chat messages: {} (order={}, limit={})\nTotal: {}",
                    data.chatId,
                    data.order,
                    data.limit,
                    data.messages.len()
                )
            }
            ToolResultData::CharacterCardListResultData(data) => data.toString(),
            ToolResultData::VisitWebResultData(data) => data.toString(),
            ToolResultData::TerminalInfoResultData(data) => data.toString(),
            ToolResultData::TerminalCommandResultData(data) => data.toString(),
            ToolResultData::TerminalStreamEventData(data) => data.toString(),
            ToolResultData::HiddenTerminalCommandResultData(data) => data.toString(),
            ToolResultData::TerminalSessionCreationResultData(data) => data.toString(),
            ToolResultData::TerminalSessionCloseResultData(data) => data.message.clone(),
            ToolResultData::TerminalSessionScreenResultData(data) => data.toString(),
            ToolResultData::FindFilesResultData(data) => data.toString(),
            ToolResultData::GrepResultData(data) => data.toString(),
            ToolResultData::MemoryLinkResultData(data) => data.toString(),
            ToolResultData::MemoryLinkQueryResultData(data) => data.toString(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BooleanResultData {
    pub value: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StringResultData {
    pub value: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SleepResultData {
    pub requestedMs: i32,
    pub sleptMs: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EnvironmentVariableReadResultData {
    pub key: String,
    pub value: Option<String>,
    pub exists: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EnvironmentVariableWriteResultData {
    pub key: String,
    pub requestedValue: String,
    pub value: Option<String>,
    pub exists: bool,
    pub cleared: bool,
}

#[allow(non_snake_case)]
pub fn stringResultData(value: impl Into<String>) -> ToolResultData {
    ToolResultData::StringResultData(StringResultData {
        value: value.into(),
    })
}

#[derive(Clone, Serialize, Deserialize)]
pub struct IntResultData {
    pub value: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BinaryResultData {
    pub value: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FilePartContentData {
    pub path: String,
    pub content: String,
    pub partIndex: i32,
    pub totalParts: i32,
    pub startLine: i32,
    pub endLine: i32,
    pub totalLines: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub isDirectory: bool,
    pub size: i64,
    pub permissions: String,
    pub lastModified: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DirectoryListingData {
    pub path: String,
    pub entries: Vec<FileEntry>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileContentData {
    pub path: String,
    pub content: String,
    pub size: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BinaryFileContentData {
    pub path: String,
    pub contentBase64: String,
    pub size: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileExistsData {
    pub path: String,
    pub exists: bool,
    pub isDirectory: bool,
    pub size: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileInfoData {
    pub path: String,
    pub exists: bool,
    pub fileType: String,
    pub size: i64,
    pub permissions: String,
    pub owner: String,
    pub group: String,
    pub lastModified: String,
    pub rawStatOutput: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileOperationData {
    pub operation: String,
    pub path: String,
    pub successful: bool,
    pub details: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileApplyResultData {
    pub operation: FileOperationData,
    pub aiDiffInstructions: String,
    pub diffContent: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HttpResponseData {
    pub url: String,
    pub statusCode: i32,
    pub statusMessage: String,
    pub headers: HashMap<String, String>,
    pub contentType: String,
    pub content: String,
    pub contentBase64: Option<String>,
    pub size: i32,
    pub cookies: HashMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize)]
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
pub struct SystemSettingData {
    pub namespace: String,
    pub setting: String,
    pub value: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppOperationData {
    pub operationType: String,
    pub packageName: String,
    pub success: bool,
    pub details: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppListData {
    pub includesSystemApps: bool,
    pub packages: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppUsageTimeEntry {
    pub packageName: String,
    pub appName: String,
    pub totalForegroundTimeMs: i64,
    pub lastTimeUsed: i64,
    pub isSystemApp: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppUsageTimeResultData {
    pub startTime: i64,
    pub endTime: i64,
    pub sinceHours: i32,
    pub requestedPackageName: Option<String>,
    pub includesSystemApps: bool,
    pub totalEntries: i32,
    pub entries: Vec<AppUsageTimeEntry>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Notification {
    pub packageName: String,
    pub text: String,
    pub timestamp: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub notifications: Vec<Notification>,
    pub timestamp: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LocationData {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f32,
    pub provider: String,
    pub timestamp: i64,
    pub rawData: String,
    pub address: String,
    pub city: String,
    pub province: String,
    pub country: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceInfoResultData {
    pub deviceId: String,
    pub model: String,
    pub manufacturer: String,
    pub androidVersion: String,
    pub sdkVersion: i32,
    pub screenResolution: String,
    pub screenDensity: f32,
    pub totalMemory: String,
    pub availableMemory: String,
    pub totalStorage: String,
    pub availableStorage: String,
    pub batteryLevel: i32,
    pub batteryCharging: bool,
    pub cpuInfo: String,
    pub networkType: String,
    pub additionalInfo: BTreeMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub ownerKey: String,
    pub title: String,
    pub content: String,
    pub source: String,
    pub tags: Vec<String>,
    pub createdAt: String,
    pub chunkInfo: Option<String>,
    pub chunkIndices: Option<Vec<i32>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MemoryQueryResultData {
    pub memories: Vec<MemoryInfo>,
    pub snapshotId: Option<String>,
    pub snapshotCreated: bool,
    pub excludedBySnapshotCount: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatServiceStartResultData {
    pub isConnected: bool,
    pub connectionTime: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatCreationResultData {
    pub chatId: String,
    pub createdAt: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatInfo {
    pub id: String,
    pub title: String,
    pub messageCount: i32,
    pub createdAt: String,
    pub updatedAt: String,
    pub isCurrent: bool,
    pub inputTokens: i32,
    pub outputTokens: i32,
    pub characterCardName: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatListResultData {
    pub totalCount: usize,
    pub currentChatId: Option<String>,
    pub chats: Vec<ChatInfo>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatFindResultData {
    pub matchedCount: usize,
    pub chat: Option<ChatInfo>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AgentStatusResultData {
    pub chatId: String,
    pub state: String,
    pub message: Option<String>,
    pub isIdle: bool,
    pub isProcessing: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatSwitchResultData {
    pub chatId: String,
    pub chatTitle: String,
    pub switchedAt: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatTitleUpdateResultData {
    pub chatId: String,
    pub title: String,
    pub updatedAt: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatDeleteResultData {
    pub chatId: String,
    pub deletedAt: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MessageSendResultData {
    pub chatId: String,
    pub message: String,
    pub aiResponse: Option<String>,
    pub receivedAt: Option<i64>,
    pub sentAt: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessageInfo {
    pub sender: String,
    pub content: String,
    pub timestamp: i64,
    pub roleName: String,
    pub provider: String,
    pub modelName: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessagesResultData {
    pub chatId: String,
    pub order: String,
    pub limit: i32,
    pub messages: Vec<ChatMessageInfo>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CharacterCardInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub isDefault: bool,
    pub createdAt: i64,
    pub updatedAt: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CharacterCardListResultData {
    pub totalCount: usize,
    pub cards: Vec<CharacterCardInfo>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TerminalInfoResultData {
    pub platform: String,
    pub defaultType: String,
    pub types: Vec<TerminalTypeInfoData>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TerminalTypeInfoData {
    pub terminalType: String,
    pub available: bool,
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TerminalCommandResultData {
    pub command: String,
    pub output: String,
    pub exitCode: i32,
    pub sessionId: String,
    pub timedOut: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TerminalStreamEventData {
    pub r#type: String,
    pub command: String,
    pub sessionId: String,
    pub chunk: Option<String>,
    pub chunkIndex: Option<i32>,
    pub receivedChars: Option<i32>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HiddenTerminalCommandResultData {
    pub command: String,
    pub output: String,
    pub exitCode: i32,
    pub executorKey: String,
    pub timedOut: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TerminalSessionCreationResultData {
    pub sessionId: String,
    pub sessionName: String,
    pub isNewSession: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TerminalSessionCloseResultData {
    pub sessionId: String,
    pub success: bool,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TerminalSessionScreenResultData {
    pub sessionId: String,
    pub rows: usize,
    pub cols: usize,
    pub content: String,
    pub commandRunning: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisitWebResultData {
    pub url: String,
    pub title: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub links: Vec<LinkData>,
    pub imageLinks: Vec<String>,
    pub visitKey: Option<String>,
    pub contentSavedTo: Option<String>,
    pub contentTruncated: bool,
    pub originalContentLength: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkData {
    pub url: String,
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FindFilesResultData {
    pub path: String,
    pub pattern: String,
    pub files: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GrepLineMatch {
    pub lineNumber: i32,
    pub lineContent: String,
    pub matchContext: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GrepFileMatch {
    pub filePath: String,
    pub lineMatches: Vec<GrepLineMatch>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GrepResultData {
    pub searchPath: String,
    pub pattern: String,
    pub matches: Vec<GrepFileMatch>,
    pub totalMatches: i32,
    pub filesSearched: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MemoryLinkResultData {
    pub sourceTitle: String,
    pub targetTitle: String,
    pub linkType: String,
    pub weight: f32,
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    pub linkId: i64,
    pub sourceTitle: String,
    pub targetTitle: String,
    pub linkType: String,
    pub weight: f32,
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MemoryLinkQueryResultData {
    pub totalCount: i32,
    pub links: Vec<LinkInfo>,
}

impl From<WebVisitLinkData> for LinkData {
    fn from(value: WebVisitLinkData) -> Self {
        Self {
            url: value.url,
            text: value.text,
        }
    }
}

impl SleepResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        format!("Slept for {}ms", self.sleptMs)
    }
}

impl EnvironmentVariableReadResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        if self.exists {
            format!("{}={}", self.key, self.value.clone().unwrap_or_default())
        } else {
            format!("{} is not set", self.key)
        }
    }
}

impl EnvironmentVariableWriteResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        if self.cleared {
            format!("{} cleared", self.key)
        } else {
            format!("{}={}", self.key, self.value.clone().unwrap_or_default())
        }
    }
}

impl FilePartContentData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        let partInfo = format!(
            "Part {} of {} (Lines {}-{} of {})",
            self.partIndex + 1,
            self.totalParts,
            self.startLine + 1,
            self.endLine,
            self.totalLines
        );
        format!("{partInfo}\n\n{}", self.content)
    }
}

impl DirectoryListingData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(&format!("Directory listing for {}:\n", self.path));
        for entry in &self.entries {
            let typeIndicator = if entry.isDirectory { "d" } else { "-" };
            sb.push_str(&format!(
                "{typeIndicator}{} {:>8} {} {}\n",
                entry.permissions, entry.size, entry.lastModified, entry.name
            ));
        }
        sb
    }
}

impl FileContentData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        format!("Content of {}:\n{}", self.path, self.content)
    }
}

impl BinaryFileContentData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        format!(
            "Binary content of {} ({} bytes, base64 length={})",
            self.path,
            self.size,
            self.contentBase64.chars().count()
        )
    }
}

impl FileExistsData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        if self.exists {
            let fileType = if self.isDirectory {
                "Directory"
            } else {
                "File"
            };
            format!(
                "{fileType} exists at path: {} (size: {} bytes)",
                self.path, self.size
            )
        } else {
            format!("No file or directory exists at path: {}", self.path)
        }
    }
}

impl FileInfoData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
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
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        self.details.clone()
    }
}

impl FileApplyResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(&self.operation.toString());
        sb.push('\n');
        if let Some(diffContent) = &self.diffContent {
            sb.push_str(&format!(
                "<file-diff path=\"{}\" details=\"{}\"><![CDATA[{}]]></file-diff>",
                self.operation.path, self.operation.details, diffContent
            ));
        }
        let requestContent = self.buildRequestContent();
        if !requestContent.trim().is_empty() {
            sb.push_str(&format!(
                "<file-request-content><![CDATA[{requestContent}]]></file-request-content>"
            ));
        }
        if !self.aiDiffInstructions.is_empty() && !self.aiDiffInstructions.starts_with("Error") {
            sb.push_str("\n--- AI-Generated Diff ---\n");
            sb.push_str(&self.aiDiffInstructions);
            sb.push('\n');
        }
        sb
    }

    #[allow(non_snake_case)]
    fn buildRequestContent(&self) -> String {
        let mut sections = vec![self.operation.toString()];
        if let Some(summary) = self.extractDiffSummaryLine() {
            sections.push(summary);
        }
        sections.join("\n")
    }

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
                    || trimmed.eq_ignore_ascii_case("No changes detected (files are identical)")
                {
                    return Some(trimmed.to_string());
                }
            }
        }
        None
    }
}

impl HttpResponseData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("HTTP Response:\n");
        sb.push_str(&format!("URL: {}\n", self.url));
        sb.push_str(&format!(
            "Status: {} {}\n",
            self.statusCode, self.statusMessage
        ));
        sb.push_str(&format!("Content-Type: {}\n", self.contentType));
        sb.push_str(&format!("Size: {} bytes\n", self.size));
        if !self.cookies.is_empty() {
            sb.push_str(&format!("Cookies: {}\n", self.cookies.len()));
            let mut entries = self.cookies.iter().collect::<Vec<_>>();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (name, value) in entries.into_iter().take(5) {
                let preview = value.chars().take(30).collect::<String>();
                let suffix = if value.chars().count() > 30 {
                    "..."
                } else {
                    ""
                };
                sb.push_str(&format!("  {name}: {preview}{suffix}\n"));
            }
            if self.cookies.len() > 5 {
                sb.push_str(&format!(
                    "  ... and {} more cookies\n",
                    self.cookies.len() - 5
                ));
            }
        }
        sb.push('\n');
        sb.push_str("Content Summary:\n");
        sb.push_str(&self.content);
        sb
    }
}

impl HttpStreamEventData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
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
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        format!(
            "Current value of {}.{}: {}",
            self.namespace, self.setting, self.value
        )
    }
}

impl AppOperationData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        match self.operationType.as_str() {
            "install" => format!(
                "Successfully installed app: {} {}",
                self.packageName, self.details
            ),
            "uninstall" => format!(
                "Successfully uninstalled app: {} {}",
                self.packageName, self.details
            ),
            "start" => format!(
                "Successfully started app: {} {}",
                self.packageName, self.details
            ),
            "stop" => format!(
                "Successfully stopped app: {} {}",
                self.packageName, self.details
            ),
            _ => self.details.clone(),
        }
    }
}

impl AppListData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        let appType = if self.includesSystemApps {
            "All Apps"
        } else {
            "Third-Party Apps"
        };
        format!("Installed {appType} List:\n{}", self.packages.join("\n"))
    }
}

impl AppUsageTimeResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
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
                    "- {} ({}): {}",
                    entry.appName,
                    entry.packageName,
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
    fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(&format!(
            "Device Notifications ({} total):\n",
            self.notifications.len()
        ));
        for (index, notification) in self.notifications.iter().enumerate() {
            sb.push_str(&format!(
                "{}. Package: {}\n",
                index + 1,
                notification.packageName
            ));
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
    fn toString(&self) -> String {
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
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
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
    fn toString(&self) -> String {
        let mut snapshotSummary = Vec::new();
        if let Some(snapshotId) = self
            .snapshotId
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            snapshotSummary.push(format!("Snapshot ID: {snapshotId}"));
        }
        if self.snapshotCreated {
            snapshotSummary.push("Snapshot created: true".to_string());
        }
        if self.excludedBySnapshotCount > 0 {
            snapshotSummary.push(format!(
                "Excluded by snapshot: {}",
                self.excludedBySnapshotCount
            ));
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
                    memory.ownerKey,
                    memory.title,
                    memory.content,
                    memory.source,
                    memory.tags.join(", "),
                    memory.createdAt
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
    fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str(&format!("Chat List ({} total):\n", self.totalCount));
        if let Some(currentChatId) = &self.currentChatId {
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
                if let Some(characterCardName) = &chat.characterCardName {
                    if !characterCardName.trim().is_empty() {
                        sb.push_str(&format!("Character Card: {characterCardName}\n"));
                    }
                }
                sb.push_str(&format!(
                    "Token Statistics: Input {} / Output {}\n",
                    chat.inputTokens, chat.outputTokens
                ));
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
    fn toString(&self) -> String {
        match &self.chat {
            Some(chat) => format!("Found chat ({}) (matched={})", chat.id, self.matchedCount),
            None => format!("No chat found (matched={})", self.matchedCount),
        }
    }
}

impl AgentStatusResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        let detail = self
            .message
            .as_ref()
            .filter(|message| !message.trim().is_empty())
            .map(|message| format!(" ({message})"))
            .unwrap_or_default();
        format!("Chat {} status: {}{}", self.chatId, self.state, detail)
    }
}

impl ChatSwitchResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        if !self.chatTitle.trim().is_empty() {
            format!(
                "Switched to chat: {}\nChat ID: {}",
                self.chatTitle, self.chatId
            )
        } else {
            format!("Switched to chat: {}", self.chatId)
        }
    }
}

impl MessageSendResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        let messagePreview = if self.message.chars().count() > 50 {
            format!("{}...", self.message.chars().take(50).collect::<String>())
        } else {
            self.message.clone()
        };
        match &self.aiResponse {
            Some(response) if !response.trim().is_empty() => {
                let responsePreview = if response.chars().count() > 200 {
                    format!("{}...", response.chars().take(200).collect::<String>())
                } else {
                    response.clone()
                };
                format!(
                    "Message sent to chat: {}\nMessage content: {}\nAI Reply: {}",
                    self.chatId, messagePreview, responsePreview
                )
            }
            _ => format!(
                "Message sent to chat: {}\nMessage content: {}",
                self.chatId, messagePreview
            ),
        }
    }
}

impl CharacterCardListResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
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
    fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Terminal Info:\n");
        sb.push_str(&format!("Platform: {}\n", self.platform));
        sb.push_str(&format!("Default Type: {}\n", self.defaultType));
        if !self.types.is_empty() {
            sb.push_str("Types:\n");
            for terminalType in &self.types {
                sb.push_str(&format!(
                    "- {}: available={} ({})\n",
                    terminalType.terminalType, terminalType.available, terminalType.description
                ));
            }
        }
        sb.trim_end().to_string()
    }
}

impl TerminalCommandResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Terminal Command Execution Result:\n");
        sb.push_str(&format!("Command: {}\n", self.command));
        sb.push_str(&format!("Session: {}\n", self.sessionId));
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
    fn toString(&self) -> String {
        match self.r#type.as_str() {
            "chunk" => self.chunk.clone().unwrap_or_default(),
            "start" => "Terminal stream started".to_string(),
            value => format!("Terminal stream event: {value}"),
        }
    }
}

impl HiddenTerminalCommandResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Hidden Terminal Command Execution Result:\n");
        sb.push_str(&format!("Command: {}\n", self.command));
        sb.push_str(&format!("Executor Key: {}\n", self.executorKey));
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
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
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
    fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Terminal Session Screen Snapshot:\n");
        sb.push_str(&format!("Session: {}\n", self.sessionId));
        sb.push_str(&format!("Size: {}x{}\n", self.cols, self.rows));
        sb.push_str(&format!("Command Running: {}\n", self.commandRunning));
        sb.push('\n');
        sb.push_str(&self.content);
        sb
    }
}

impl FindFilesResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
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
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        let mut sb = String::new();
        sb.push_str("Grep Search Result:\n");
        sb.push_str(&format!("Search Path: {}\n", self.searchPath));
        sb.push_str(&format!("Pattern: {}\n", self.pattern));
        sb.push_str(&format!(
            "Total Matches: {} (in {} files)\n",
            self.totalMatches,
            self.matches.len()
        ));
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
                            let isPreNumberedContext =
                                contextLines.iter().any(|line| !line.trim().is_empty())
                                    && contextLines.iter().all(|line| {
                                        line.trim().is_empty()
                                            || parsePreNumberedLineNumber(line).is_some()
                                    });
                            if isPreNumberedContext {
                                for contextLine in contextLines {
                                    let renderedLine = if parsePreNumberedLineNumber(contextLine)
                                        == Some(lineMatch.lineNumber)
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
                                for (index, contextLine) in contextLines.iter().enumerate() {
                                    let actualLineNumber =
                                        lineMatch.lineNumber - centerIndex + index as i32;
                                    if index as i32 == centerIndex {
                                        sb.push_str(&format!(
                                            "{actualLineNumber:>6}|>{contextLine}\n"
                                        ));
                                    } else {
                                        sb.push_str(&format!(
                                            "{actualLineNumber:>6}| {contextLine}\n"
                                        ));
                                    }
                                }
                            }
                            sb.push('\n');
                        }
                        _ => {
                            sb.push_str(&format!(
                                "{:>6}| {}\n",
                                lineMatch.lineNumber, lineMatch.lineContent
                            ));
                        }
                    }
                    displayedMatches += 1;
                }
                if matchesCollapsedInFile > 0 {
                    sb.push_str(&format!(
                        "  ... ({matchesCollapsedInFile} more match groups collapsed in this file)\n"
                    ));
                    collapsedMatches += matchesCollapsedInFile;
                }
                sb.push('\n');
            }
            if collapsedMatches > 0 {
                sb.push_str(&format!("{}\n", "=".repeat(60)));
                sb.push_str(&format!(
                    "To save space, {collapsedMatches} match groups were collapsed\n"
                ));
                sb.push_str(&format!(
                    "Displayed {displayedMatches} match groups, total {} matches\n",
                    self.totalMatches
                ));
            }
        }
        sb
    }
}

impl MemoryLinkResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        format!(
            "Successfully linked memory: '{}' -> '{}' (Type: {}, Strength: {})",
            self.sourceTitle, self.targetTitle, self.linkType, self.weight
        )
    }
}

impl MemoryLinkQueryResultData {
    #[allow(non_snake_case)]
    fn toString(&self) -> String {
        if self.links.is_empty() {
            return "No memory links found.".to_string();
        }
        let mut sb = String::new();
        sb.push_str(&format!("Memory Links ({}):\n", self.totalCount));
        for link in &self.links {
            sb.push_str(&format!(
                "- #{}: '{}' -> '{}' (Type: {}, Weight: {})\n",
                link.linkId, link.sourceTitle, link.targetTitle, link.linkType, link.weight
            ));
            if !link.description.trim().is_empty() {
                sb.push_str(&format!("  Description: {}\n", link.description));
            }
        }
        sb.trim().to_string()
    }
}

impl VisitWebResultData {
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
                sb.push_str(&format!(
                    "... ({omittedCount} more links omitted from inline preview)\n"
                ));
            }
            sb.push('\n');
        }
        if !self.imageLinks.is_empty() {
            sb.push_str("Images:\n");
            for (index, link) in self.imageLinks.iter().take(MAX_INLINE_IMAGES).enumerate() {
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
                sb.push_str(&format!(
                    "... ({omittedCount} more images omitted from inline preview)\n"
                ));
            }
            sb.push('\n');
        }
        if let Some(savedTo) = &self.contentSavedTo {
            sb.push_str(&format!("Full content saved to file: {savedTo}\n"));
            if let Some(totalChars) = self.originalContentLength {
                sb.push_str(&format!("Original content length: {totalChars} chars\n"));
            }
            if self.contentTruncated {
                sb.push_str("Use read_file_part or grep_code to inspect the saved file.\n");
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

#[allow(non_snake_case)]
fn parsePreNumberedLineNumber(line: &str) -> Option<i32> {
    let trimmed = line.trim_start();
    let separatorIndex = trimmed.find('|')?;
    if separatorIndex == 0 {
        return None;
    }
    trimmed[..separatorIndex].trim().parse::<i32>().ok()
}

#[allow(non_snake_case)]
fn markPreNumberedContextLine(line: &str) -> String {
    let Some(separatorIndex) = line.find('|') else {
        return line.to_string();
    };
    if line
        .as_bytes()
        .get(separatorIndex + 1)
        .is_some_and(|value| *value == b'>')
    {
        return line.to_string();
    }
    let mut output = String::with_capacity(line.len() + 1);
    output.push_str(&line[..separatorIndex + 1]);
    output.push('>');
    output.push_str(&line[separatorIndex + 1..]);
    output
}

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

#[allow(non_snake_case)]
fn formatTimestamp(timestamp: i64) -> String {
    Local
        .timestamp_millis_opt(timestamp)
        .single()
        .expect("valid timestamp millis")
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}
