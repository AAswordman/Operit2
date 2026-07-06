#![allow(non_snake_case)]

pub mod TimeUtils;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type HostResult<T> = Result<T, HostError>;

pub type HostRuntimeEventSink = Arc<dyn Fn(Value) + Send + Sync + 'static>;

type HostLogSink = Arc<dyn Fn(&str, &str) + Send + Sync + 'static>;
static HOST_LOG_SINK: OnceLock<RwLock<Option<HostLogSink>>> = OnceLock::new();

pub fn setHostLogSink(sink: HostLogSink) {
    let holder = HOST_LOG_SINK.get_or_init(|| RwLock::new(None));
    *holder.write().expect("host log sink lock poisoned") = Some(sink);
}

pub fn logHostError(tag: &str, message: &str) {
    let sink = HOST_LOG_SINK
        .get_or_init(|| RwLock::new(None))
        .read()
        .expect("host log sink lock poisoned")
        .clone()
        .expect("host log sink must be installed before host errors are logged");
    sink(tag, message);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostEnvironmentDescriptor {
    pub id: String,
    pub displayName: String,
    pub pathStyleDescriptionEn: String,
    pub pathStyleDescriptionCn: String,
    pub examplePaths: Vec<String>,
    pub usesEnvironmentParameter: bool,
    pub environmentParameterDescriptionEn: String,
    pub environmentParameterDescriptionCn: String,
    pub capabilities: Vec<String>,
}

impl HostEnvironmentDescriptor {
    pub fn android() -> Self {
        Self {
            id: "android".to_string(),
            displayName: "Android".to_string(),
            pathStyleDescriptionEn: "Use Android absolute paths such as /sdcard/Download or an attached repository path.".to_string(),
            pathStyleDescriptionCn: "使用 Android 绝对路径，例如 /sdcard/Download，或使用已附加的仓库路径。".to_string(),
            examplePaths: vec![
                "/sdcard/Download".to_string(),
                "/sdcard/Documents".to_string(),
            ],
            usesEnvironmentParameter: true,
            environmentParameterDescriptionEn: "optional, execution environment. Values: \"android\" (Android file system) | \"linux\" (local terminal environment) | \"repo:<repositoryName>\" (attached local storage repository)".to_string(),
            environmentParameterDescriptionCn: "可选，执行环境。取值：\"android\"（Android 文件系统）| \"linux\"（本地终端环境）| \"repo:<仓库名>\"（附加本地储存仓库）".to_string(),
            capabilities: vec![
                "fs.read".to_string(),
                "fs.write".to_string(),
                "fs.search".to_string(),
                "fs.archive".to_string(),
                "os.open".to_string(),
                "os.share".to_string(),
                "audio.playback".to_string(),
                "music.playback".to_string(),
                "bluetooth.classic".to_string(),
                "bluetooth.ble".to_string(),
                "tts.synthesis".to_string(),
                "tts.playback".to_string(),
                "system.location".to_string(),
                "system.notifications.read".to_string(),
                "system.app_usage".to_string(),
                "system.app.install".to_string(),
                "system.app.uninstall".to_string(),
                "system.settings".to_string(),
                "runtime.process".to_string(),
                "runtime.storage".to_string(),
                "runtime.sqlite".to_string(),
            ],
        }
    }

    pub fn windows() -> Self {
        Self {
            id: "windows".to_string(),
            displayName: "Windows".to_string(),
            pathStyleDescriptionEn:
                "Use absolute Windows paths such as C:/Users/Name/Documents or D:/Code/project."
                    .to_string(),
            pathStyleDescriptionCn:
                "使用 Windows 绝对路径，例如 C:/Users/Name/Documents 或 D:/Code/project。"
                    .to_string(),
            examplePaths: vec![
                "C:/Users/Name/Documents".to_string(),
                "D:/Code/project".to_string(),
            ],
            usesEnvironmentParameter: false,
            environmentParameterDescriptionEn: String::new(),
            environmentParameterDescriptionCn: String::new(),
            capabilities: vec![
                "fs.read".to_string(),
                "fs.write".to_string(),
                "fs.search".to_string(),
                "fs.archive".to_string(),
                "os.open".to_string(),
                "os.share".to_string(),
                "audio.playback".to_string(),
                "music.playback".to_string(),
                "bluetooth.classic".to_string(),
                "bluetooth.ble".to_string(),
                "tts.synthesis".to_string(),
                "tts.playback".to_string(),
                "system.location".to_string(),
                "system.notifications.read".to_string(),
                "system.app_usage".to_string(),
                "system.app.install".to_string(),
                "system.app.uninstall".to_string(),
                "system.settings".to_string(),
            ],
        }
    }

    pub fn linux() -> Self {
        Self {
            id: "linux".to_string(),
            displayName: "Linux".to_string(),
            pathStyleDescriptionEn:
                "Use absolute Linux paths such as /home/user/project or /tmp/work.".to_string(),
            pathStyleDescriptionCn: "使用 Linux 绝对路径，例如 /home/user/project 或 /tmp/work。"
                .to_string(),
            examplePaths: vec!["/home/user/project".to_string(), "/tmp/work".to_string()],
            usesEnvironmentParameter: false,
            environmentParameterDescriptionEn: String::new(),
            environmentParameterDescriptionCn: String::new(),
            capabilities: vec![
                "fs.read".to_string(),
                "fs.write".to_string(),
                "fs.search".to_string(),
                "fs.archive".to_string(),
                "os.open".to_string(),
                "os.share".to_string(),
                "audio.playback".to_string(),
                "music.playback".to_string(),
                "bluetooth.classic".to_string(),
                "bluetooth.ble".to_string(),
                "tts.synthesis".to_string(),
                "tts.playback".to_string(),
                "system.location".to_string(),
                "system.notifications.read".to_string(),
                "system.app_usage".to_string(),
                "system.app.install".to_string(),
                "system.app.uninstall".to_string(),
                "system.settings".to_string(),
            ],
        }
    }

    pub fn web() -> Self {
        Self {
            id: "web".to_string(),
            displayName: "Web".to_string(),
            pathStyleDescriptionEn: "Use paths exposed by the browser host bridge.".to_string(),
            pathStyleDescriptionCn: "使用浏览器 host bridge 暴露的路径。".to_string(),
            examplePaths: vec![
                "operit.db".to_string(),
                "preferences/models.json".to_string(),
                "workspace/project".to_string(),
            ],
            usesEnvironmentParameter: false,
            environmentParameterDescriptionEn: String::new(),
            environmentParameterDescriptionCn: String::new(),
            capabilities: vec![
                "fs.read".to_string(),
                "fs.write".to_string(),
                "fs.search".to_string(),
                "fs.archive".to_string(),
                "web.visit".to_string(),
                "runtime.process".to_string(),
                "runtime.storage".to_string(),
                "runtime.sqlite".to_string(),
                "audio.playback".to_string(),
                "music.playback".to_string(),
                "bluetooth.classic".to_string(),
                "bluetooth.ble".to_string(),
                "tts.playback".to_string(),
                "os.open".to_string(),
                "os.share".to_string(),
                "system.location".to_string(),
                "system.notifications.read".to_string(),
                "system.app_usage".to_string(),
                "system.app.install".to_string(),
                "system.app.uninstall".to_string(),
                "system.settings".to_string(),
            ],
        }
    }
}

impl Default for HostEnvironmentDescriptor {
    fn default() -> Self {
        Self::android()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostError {
    pub message: String,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for HostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for HostError {}

impl From<std::io::Error> for HostError {
    fn from(value: std::io::Error) -> Self {
        Self::new(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub isDirectory: bool,
    pub size: i64,
    pub permissions: String,
    pub lastModified: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileExistence {
    pub exists: bool,
    pub isDirectory: bool,
    pub size: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileInfo {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindFilesRequest {
    pub path: String,
    pub pattern: String,
    pub maxDepth: i32,
    pub usePathPattern: bool,
    pub caseInsensitive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrepCodeRequest {
    pub path: String,
    pub pattern: String,
    pub filePattern: String,
    pub caseInsensitive: bool,
    pub contextLines: usize,
    pub maxResults: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrepLineMatch {
    pub lineNumber: usize,
    pub lineContent: String,
    pub matchContext: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrepFileMatch {
    pub filePath: String,
    pub lineMatches: Vec<GrepLineMatch>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrepCodeResult {
    pub matches: Vec<GrepFileMatch>,
    pub totalMatches: usize,
    pub filesSearched: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebVisitRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub userAgent: String,
    pub includeImageLinks: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebVisitLinkData {
    pub url: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebVisitResult {
    pub url: String,
    pub title: String,
    pub content: String,
    pub metadata: Vec<(String, String)>,
    pub links: Vec<WebVisitLinkData>,
    pub imageLinks: Vec<String>,
}

pub trait WebVisitHost: Send + Sync {
    fn visitWeb(&self, request: WebVisitRequest) -> HostResult<WebVisitResult>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserAutomationRequest {
    pub requestId: String,
    pub toolName: String,
    pub parametersJson: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserAutomationResponse {
    pub output: String,
}

pub trait BrowserAutomationHost: Send + Sync {
    fn executeBrowserTool(
        &self,
        request: BrowserAutomationRequest,
    ) -> HostResult<BrowserAutomationResponse>;
}

pub trait ComposeDslWebViewHost: Send + Sync {
    fn handleControllerCommand(&self, payloadJson: &str) -> HostResult<String>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpFilePart {
    pub fieldName: String,
    pub fileName: String,
    pub contentType: String,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequestData {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub formFields: Vec<(String, String)>,
    pub fileParts: Vec<HttpFilePart>,
    pub connectTimeoutSeconds: u64,
    pub readTimeoutSeconds: u64,
    pub followRedirects: bool,
    pub ignoreSsl: bool,
    pub proxyHost: String,
    pub proxyPort: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponseData {
    pub finalUrl: String,
    pub statusCode: i32,
    pub statusMessage: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub trait HttpHost: Send + Sync {
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManagedRuntimeProgram {
    Node,
    Python,
    Uv,
    Pnpm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeProcessRequest {
    pub program: ManagedRuntimeProgram,
    pub executablePath: Option<String>,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeCommandOutput {
    pub exitCode: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait ManagedRuntimeProcess: Send {
    fn writeLine(&self, line: &str) -> HostResult<()>;
    fn readStdoutLine(&self, timeoutMs: u64) -> HostResult<Option<String>>;
    fn drainStderr(&self) -> HostResult<String>;
    fn isRunning(&self) -> HostResult<bool>;
    fn kill(&self) -> HostResult<()>;
}

pub trait ManagedRuntimeHost: Send + Sync {
    fn runtimeWorkspaceDir(&self) -> HostResult<String>;
    fn resolveRuntimeExecutable(
        &self,
        program: ManagedRuntimeProgram,
        executablePath: Option<&str>,
    ) -> HostResult<String>;
    fn startRuntimeProcess(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<Box<dyn ManagedRuntimeProcess>>;
    fn runRuntimeCommand(&self, request: RuntimeProcessRequest)
        -> HostResult<RuntimeCommandOutput>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalSessionInfo {
    pub sessionId: String,
    pub sessionName: String,
    pub terminalType: String,
    pub isNewSession: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalCommandOutput {
    pub command: String,
    pub output: String,
    pub exitCode: i32,
    pub sessionId: String,
    pub terminalType: String,
    pub timedOut: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenTerminalCommandOutput {
    pub command: String,
    pub output: String,
    pub exitCode: i32,
    pub executorKey: String,
    pub terminalType: String,
    pub timedOut: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalInputOutput {
    pub sessionId: String,
    pub acceptedChars: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalCloseOutput {
    pub sessionId: String,
    pub success: bool,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalScreenOutput {
    pub sessionId: String,
    pub terminalType: String,
    pub rows: usize,
    pub cols: usize,
    pub content: String,
    pub commandRunning: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalSessionListEntry {
    pub sessionId: String,
    pub sessionName: String,
    pub terminalType: String,
    pub sessionKind: String,
    pub workingDir: String,
    pub commandRunning: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalTypeInfo {
    pub terminalType: String,
    pub available: bool,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalInfo {
    pub platform: String,
    pub defaultType: String,
    pub types: Vec<TerminalTypeInfo>,
}

pub trait TerminalHost: Send + Sync {
    fn terminalInfo(&self) -> HostResult<TerminalInfo>;
    fn startPtySession(
        &self,
        sessionName: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String>;
    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>>;
    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize>;
    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()>;
    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>>;
    fn closePtySession(&self, sessionId: &str) -> HostResult<()>;
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>>;
    fn createOrGetSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo>;
    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput>;
    fn executeHiddenCommand(
        &self,
        command: &str,
        terminalType: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput>;
    fn inputInSession(
        &self,
        sessionId: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> HostResult<TerminalInputOutput>;
    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput>;
    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeStorageEntry {
    pub path: String,
    pub isDirectory: bool,
    pub size: i64,
}

pub trait RuntimeStorageHost: Send + Sync {
    fn rootDir(&self) -> Option<PathBuf>;
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>>;
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()>;
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()>;
    fn exists(&self, path: &str) -> HostResult<bool>;
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum SqliteValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl SqliteValue {
    pub fn asI64(&self) -> HostResult<i64> {
        match self {
            SqliteValue::Integer(value) => Ok(*value),
            other => Err(HostError::new(format!(
                "expected sqlite integer, got {other:?}"
            ))),
        }
    }

    pub fn asF64(&self) -> HostResult<f64> {
        match self {
            SqliteValue::Real(value) => Ok(*value),
            SqliteValue::Integer(value) => Ok(*value as f64),
            other => Err(HostError::new(format!(
                "expected sqlite real, got {other:?}"
            ))),
        }
    }

    pub fn asString(&self) -> HostResult<String> {
        match self {
            SqliteValue::Text(value) => Ok(value.clone()),
            other => Err(HostError::new(format!(
                "expected sqlite text, got {other:?}"
            ))),
        }
    }

    pub fn isNull(&self) -> bool {
        matches!(self, SqliteValue::Null)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SqliteRow {
    pub columns: Vec<String>,
    pub values: Vec<SqliteValue>,
}

impl SqliteRow {
    pub fn valueAt(&self, index: usize) -> HostResult<&SqliteValue> {
        self.values
            .get(index)
            .ok_or_else(|| HostError::new(format!("sqlite column index out of bounds: {index}")))
    }

    pub fn valueNamed(&self, name: &str) -> HostResult<&SqliteValue> {
        let index = self
            .columns
            .iter()
            .position(|column| column == name)
            .ok_or_else(|| HostError::new(format!("sqlite column not found: {name}")))?;
        self.valueAt(index)
    }
}

pub trait RuntimeSqliteConnection: Send {
    fn executeBatch(&mut self, sql: &str) -> HostResult<()>;
    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize>;
    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>>;
    fn lastInsertRowId(&self) -> HostResult<i64>;
    fn beginTransaction(&mut self) -> HostResult<Box<dyn RuntimeSqliteTransaction + '_>>;
}

pub trait RuntimeSqliteTransaction {
    fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize>;
    fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>>;
    fn lastInsertRowId(&self) -> HostResult<i64>;
    fn commit(self: Box<Self>) -> HostResult<()>;
}

pub trait RuntimeSqliteHost: Send + Sync {
    fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>>;
}

pub trait HostRuntimeEventRegistration: Send {}

pub trait HostRuntimeEventHost: Send + Sync {
    fn startHostRuntimeEventStream(
        &self,
        sink: HostRuntimeEventSink,
    ) -> HostResult<Box<dyn HostRuntimeEventRegistration>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemSettingData {
    pub namespace: String,
    pub setting: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppOperationData {
    pub operationType: String,
    pub packageName: String,
    pub success: bool,
    pub details: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppListData {
    pub includesSystemApps: bool,
    pub packages: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationEntry {
    pub packageName: String,
    pub text: String,
    pub timestamp: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationData {
    pub notifications: Vec<NotificationEntry>,
    pub timestamp: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppUsageTimeEntry {
    pub packageName: String,
    pub appName: String,
    pub totalForegroundTimeMs: i64,
    pub lastTimeUsed: i64,
    pub isSystemApp: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppUsageTimeResultData {
    pub startTime: i64,
    pub endTime: i64,
    pub sinceHours: i32,
    pub requestedPackageName: Option<String>,
    pub includesSystemApps: bool,
    pub totalEntries: i32,
    pub entries: Vec<AppUsageTimeEntry>,
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceInfoData {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OCRLanguage {
    Latin,
    Chinese,
    Japanese,
    Korean,
}

impl OCRLanguage {
    pub fn asHostValue(self) -> &'static str {
        match self {
            OCRLanguage::Latin => "LATIN",
            OCRLanguage::Chinese => "CHINESE",
            OCRLanguage::Japanese => "JAPANESE",
            OCRLanguage::Korean => "KOREAN",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OCRQuality {
    Low,
    High,
}

impl OCRQuality {
    pub fn asHostValue(self) -> &'static str {
        match self {
            OCRQuality::Low => "LOW",
            OCRQuality::High => "HIGH",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioPlaybackStatus {
    pub path: String,
    pub started: bool,
    pub details: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MusicPlaybackRequest {
    pub source: String,
    pub sourceType: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub loopPlayback: bool,
    pub volume: f64,
    pub startPositionMs: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MusicPlaybackStatus {
    pub state: String,
    pub source: Option<String>,
    pub sourceType: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub durationMs: Option<i64>,
    pub positionMs: i64,
    pub bufferedPositionMs: i64,
    pub volume: f64,
    pub loopPlayback: bool,
    pub message: String,
}

pub trait AudioPlaybackHost: Send + Sync {
    fn playAudio(&self, path: &str) -> HostResult<AudioPlaybackStatus>;
    fn playMusic(&self, request: MusicPlaybackRequest) -> HostResult<MusicPlaybackStatus>;
    fn pauseMusic(&self) -> HostResult<MusicPlaybackStatus>;
    fn resumeMusic(&self) -> HostResult<MusicPlaybackStatus>;
    fn stopMusic(&self) -> HostResult<MusicPlaybackStatus>;
    fn seekMusic(&self, positionMs: i64) -> HostResult<MusicPlaybackStatus>;
    fn setMusicVolume(&self, volume: f64) -> HostResult<MusicPlaybackStatus>;
    fn musicStatus(&self) -> HostResult<MusicPlaybackStatus>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothStateData {
    pub supported: bool,
    pub enabled: bool,
    pub state: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothDeviceData {
    pub name: Option<String>,
    pub address: String,
    pub r#type: String,
    pub bondState: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBondedDevicesData {
    pub devices: Vec<BluetoothDeviceData>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothScannedDeviceData {
    pub name: Option<String>,
    pub address: String,
    pub r#type: String,
    pub bondState: String,
    pub source: String,
    pub rssi: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothScanRequest {
    pub durationMs: i64,
    pub includeBle: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothScanResultData {
    pub devices: Vec<BluetoothScannedDeviceData>,
    pub durationMs: i64,
    pub includesBle: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothSessionData {
    pub sessionId: String,
    pub address: String,
    pub mode: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothClassicConnectRequest {
    pub address: String,
    pub uuid: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothClassicListenRequest {
    pub name: String,
    pub uuid: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothClassicAcceptRequest {
    pub listenerSessionId: String,
    pub timeoutMs: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothPayload {
    pub text: Option<String>,
    pub dataBase64: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothTransferData {
    pub sessionId: String,
    pub bytesWritten: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothReadRequest {
    pub sessionId: String,
    pub maxBytes: i64,
    pub timeoutMs: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothReadData {
    pub sessionId: String,
    pub bytesRead: i64,
    pub text: Option<String>,
    pub dataBase64: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBleConnectRequest {
    pub address: String,
    pub autoConnect: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBleCharacteristicData {
    pub uuid: String,
    pub properties: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBleServiceData {
    pub uuid: String,
    pub characteristics: Vec<BluetoothBleCharacteristicData>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBleServicesData {
    pub sessionId: String,
    pub services: Vec<BluetoothBleServiceData>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBleCharacteristicAddress {
    pub sessionId: String,
    pub serviceUuid: String,
    pub characteristicUuid: String,
    pub timeoutMs: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBleWriteRequest {
    pub sessionId: String,
    pub serviceUuid: String,
    pub characteristicUuid: String,
    pub text: Option<String>,
    pub dataBase64: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBleWriteAndReadRequest {
    pub sessionId: String,
    pub writeServiceUuid: String,
    pub writeCharacteristicUuid: String,
    pub readServiceUuid: String,
    pub readCharacteristicUuid: String,
    pub text: Option<String>,
    pub dataBase64: Option<String>,
    pub timeoutMs: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBleSubscribeRequest {
    pub sessionId: String,
    pub serviceUuid: String,
    pub characteristicUuid: String,
    pub enable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBleNotificationEntry {
    pub characteristicUuid: String,
    pub bytesRead: i64,
    pub text: Option<String>,
    pub dataBase64: Option<String>,
    pub timestamp: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BluetoothBleNotificationData {
    pub sessionId: String,
    pub notifications: Vec<BluetoothBleNotificationEntry>,
}

pub trait BluetoothHost: Send + Sync {
    fn requestBluetoothPermission(&self) -> HostResult<String>;
    fn bluetoothState(&self) -> HostResult<BluetoothStateData>;
    fn requestEnableBluetooth(&self) -> HostResult<String>;
    fn listBluetoothBondedDevices(&self) -> HostResult<BluetoothBondedDevicesData>;
    fn scanBluetoothDevices(
        &self,
        request: BluetoothScanRequest,
    ) -> HostResult<BluetoothScanResultData>;
    fn bluetoothConnect(
        &self,
        request: BluetoothClassicConnectRequest,
    ) -> HostResult<BluetoothSessionData>;
    fn bluetoothListen(
        &self,
        request: BluetoothClassicListenRequest,
    ) -> HostResult<BluetoothSessionData>;
    fn bluetoothAccept(
        &self,
        request: BluetoothClassicAcceptRequest,
    ) -> HostResult<BluetoothSessionData>;
    fn bluetoothSend(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
    ) -> HostResult<BluetoothTransferData>;
    fn bluetoothRead(&self, request: BluetoothReadRequest) -> HostResult<BluetoothReadData>;
    fn bluetoothSendAndRead(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
        read: BluetoothReadRequest,
    ) -> HostResult<BluetoothReadData>;
    fn bluetoothClose(&self, sessionId: &str) -> HostResult<String>;
    fn bluetoothBleConnect(
        &self,
        request: BluetoothBleConnectRequest,
    ) -> HostResult<BluetoothSessionData>;
    fn bluetoothBleDiscoverServices(
        &self,
        sessionId: &str,
        timeoutMs: i64,
    ) -> HostResult<BluetoothBleServicesData>;
    fn bluetoothBleReadCharacteristic(
        &self,
        address: BluetoothBleCharacteristicAddress,
    ) -> HostResult<BluetoothReadData>;
    fn bluetoothBleWriteCharacteristic(
        &self,
        request: BluetoothBleWriteRequest,
    ) -> HostResult<BluetoothTransferData>;
    fn bluetoothBleWriteAndReadCharacteristic(
        &self,
        request: BluetoothBleWriteAndReadRequest,
    ) -> HostResult<BluetoothReadData>;
    fn bluetoothBleSubscribeCharacteristic(
        &self,
        request: BluetoothBleSubscribeRequest,
    ) -> HostResult<BluetoothTransferData>;
    fn bluetoothBleReadNotifications(
        &self,
        sessionId: &str,
        limit: i64,
    ) -> HostResult<BluetoothBleNotificationData>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TtsSynthesisRequest {
    pub text: String,
    pub voice: String,
    pub locale: String,
    pub speed: f64,
    pub pitch: f64,
    pub outputFormat: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TtsSynthesisResponse {
    pub audioPath: String,
    pub details: String,
}

pub trait TtsSynthesisHost: Send + Sync {
    fn synthesizeSpeech(&self, request: TtsSynthesisRequest) -> HostResult<TtsSynthesisResponse>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TtsPlaybackRequest {
    pub text: String,
    pub voice: String,
    pub locale: String,
    pub speed: f64,
    pub pitch: f64,
    pub interrupt: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TtsPlaybackStatus {
    pub path: String,
    pub active: bool,
    pub paused: bool,
    pub details: String,
}

pub trait TtsPlaybackHost: Send + Sync {
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus>;
    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus>;
    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus>;
    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus>;
    fn speechState(&self) -> HostResult<TtsPlaybackStatus>;
}

pub trait SystemOperationHost: Send + Sync {
    fn getSystemLanguageCode(&self) -> HostResult<String>;
    fn toast(&self, message: &str) -> HostResult<()>;
    fn sendNotification(&self, title: &str, message: &str) -> HostResult<()>;
    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> HostResult<SystemSettingData>;
    fn getSystemSetting(&self, namespace: &str, setting: &str) -> HostResult<SystemSettingData>;
    fn installApp(&self, path: &str) -> HostResult<AppOperationData>;
    fn uninstallApp(&self, packageName: &str) -> HostResult<AppOperationData>;
    fn listInstalledApps(&self, includeSystemApps: bool) -> HostResult<AppListData>;
    fn startApp(&self, packageName: &str) -> HostResult<AppOperationData>;
    fn stopApp(&self, packageName: &str) -> HostResult<AppOperationData>;
    fn getNotifications(&self, limit: i32, includeOngoing: bool) -> HostResult<NotificationData>;
    fn getAppUsageTime(
        &self,
        packageName: &str,
        sinceHours: i32,
        limit: i32,
        includeSystemApps: bool,
    ) -> HostResult<AppUsageTimeResultData>;
    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> HostResult<LocationData>;
    fn getDeviceInfo(&self) -> HostResult<DeviceInfoData>;
    fn captureScreenshot(&self) -> HostResult<String>;
    fn recognizeText(
        &self,
        imagePath: &str,
        language: OCRLanguage,
        quality: OCRQuality,
    ) -> HostResult<String>;
}

pub trait FileSystemHost: Send + Sync {
    fn envLabel(&self) -> &str;
    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor;
    fn validatePath(&self, path: &str, paramName: &str) -> HostResult<()>;
    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>>;
    fn readFile(&self, path: &str) -> HostResult<String>;
    fn readFileWithLimit(&self, path: &str, maxBytes: usize) -> HostResult<String>;
    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>>;
    fn writeFile(&self, path: &str, content: &str, append: bool) -> HostResult<()>;
    fn writeFileBytes(&self, path: &str, content: &[u8]) -> HostResult<()>;
    fn deleteFile(&self, path: &str, recursive: bool) -> HostResult<()>;
    fn fileExists(&self, path: &str) -> HostResult<FileExistence>;
    fn moveFile(&self, source: &str, destination: &str) -> HostResult<()>;
    fn copyFile(&self, source: &str, destination: &str, recursive: bool) -> HostResult<()>;
    fn makeDirectory(&self, path: &str, createParents: bool) -> HostResult<()>;
    fn findFiles(&self, request: FindFilesRequest) -> HostResult<Vec<String>>;
    fn fileInfo(&self, path: &str) -> HostResult<FileInfo>;
    fn grepCode(&self, request: GrepCodeRequest) -> HostResult<GrepCodeResult>;
    fn zipFiles(&self, source: &str, destination: &str) -> HostResult<()>;
    fn unzipFiles(&self, source: &str, destination: &str) -> HostResult<()>;
    fn openFile(&self, path: &str) -> HostResult<()>;
    fn shareFile(&self, path: &str, title: &str) -> HostResult<()>;
}
