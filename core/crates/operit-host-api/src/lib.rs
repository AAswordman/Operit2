#![allow(non_snake_case)]

pub mod TimeUtils;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

pub type HostResult<T> = Result<T, HostError>;

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
    pub chatId: String,
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

pub trait SystemOperationHost: Send + Sync {
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
