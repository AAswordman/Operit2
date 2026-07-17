#![allow(non_snake_case)]

pub mod HostManager;
pub mod TimeUtils;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type HostResult<T> = Result<T, HostError>;

pub type HostRuntimeEventSink = Arc<dyn Fn(Value) + Send + Sync + 'static>;

pub type HostRuntimeEventScheduleSink =
    Arc<dyn Fn(HostRuntimeEventScheduleFire) + Send + Sync + 'static>;

type HostLogSink = Arc<dyn Fn(&str, &str) + Send + Sync + 'static>;
static HOST_LOG_SINK: OnceLock<RwLock<Option<HostLogSink>>> = OnceLock::new();

/// Installs the process-wide host log sink used for host error reporting.
pub fn setHostLogSink(sink: HostLogSink) {
    let holder = HOST_LOG_SINK.get_or_init(|| RwLock::new(None));
    *holder.write().expect("host log sink lock poisoned") = Some(sink);
}

/// Writes a host error message through the installed host log sink.
pub fn logHostError(tag: &str, message: &str) {
    let sink = HOST_LOG_SINK
        .get_or_init(|| RwLock::new(None))
        .read()
        .expect("host log sink lock poisoned")
        .clone()
        .expect("host log sink must be installed before host errors are logged");
    sink(tag, message);
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostEnvironmentDescriptor {
    pub id: String,
    pub displayName: String,
    pub platform: HostPlatform,
    pub privilege: HostPrivilege,
    pub isolation: HostIsolation,
    pub pathStyleDescriptionEn: String,
    pub pathStyleDescriptionCn: String,
    pub examplePaths: Vec<String>,
    pub usesEnvironmentParameter: bool,
    pub environmentParameterDescriptionEn: String,
    pub environmentParameterDescriptionCn: String,
    pub capabilities: Vec<String>,
    pub structuredCapabilities: Vec<HostCapability>,
    pub onboardingRequirements: Vec<HostOnboardingRequirement>,
    pub workspaceRoots: Vec<WorkspaceRootDescriptor>,
}

impl HostEnvironmentDescriptor {
    /// Builds the Android host descriptor used by mobile runtime prompts.
    pub fn android() -> Self {
        Self {
            id: "android".to_string(),
            displayName: "Android".to_string(),
            platform: HostPlatform::Android,
            privilege: HostPrivilege::Normal,
            isolation: HostIsolation::OsAppSandbox,
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
            structuredCapabilities: defaultHostCapabilities(),
            onboardingRequirements: androidOnboardingRequirements(),
            workspaceRoots: Vec::new(),
        }
    }

    /// Builds the OpenHarmony host descriptor used by mobile runtime prompts.
    pub fn ohos() -> Self {
        Self {
            id: "ohos".to_string(),
            displayName: "OpenHarmony".to_string(),
            platform: HostPlatform::Ohos,
            privilege: HostPrivilege::Normal,
            isolation: HostIsolation::OsAppSandbox,
            pathStyleDescriptionEn:
                "Use absolute OpenHarmony application paths supplied by the host.".to_string(),
            pathStyleDescriptionCn: "使用鸿蒙 Host 提供的绝对应用路径。".to_string(),
            examplePaths: Vec::new(),
            usesEnvironmentParameter: false,
            environmentParameterDescriptionEn: String::new(),
            environmentParameterDescriptionCn: String::new(),
            capabilities: vec![
                "fs.read".to_string(),
                "fs.write".to_string(),
                "fs.search".to_string(),
                "fs.archive".to_string(),
                "http.request".to_string(),
                "web.visit".to_string(),
                "os.open".to_string(),
                "os.share".to_string(),
                "terminal.pty".to_string(),
                "audio.playback".to_string(),
                "music.playback".to_string(),
                "tts.synthesis".to_string(),
                "tts.playback".to_string(),
                "bluetooth.classic".to_string(),
                "bluetooth.ble".to_string(),
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
            structuredCapabilities: hostCapabilities(&[
                "fs.read",
                "fs.write",
                "fs.search",
                "fs.archive",
                "http.request",
                "web.visit",
                "os.open",
                "os.share",
                "terminal.pty",
                "audio.playback",
                "music.playback",
                "tts.synthesis",
                "tts.playback",
                "bluetooth.classic",
                "bluetooth.ble",
                "system.location",
                "system.notifications.read",
                "system.app_usage",
                "system.app.install",
                "system.app.uninstall",
                "system.settings",
                "runtime.process",
                "runtime.storage",
                "runtime.sqlite",
            ]),
            onboardingRequirements: ohosOnboardingRequirements(),
            workspaceRoots: Vec::new(),
        }
    }

    /// Builds the Windows host descriptor used by desktop runtime prompts.
    pub fn windows() -> Self {
        Self {
            id: "windows".to_string(),
            displayName: "Windows".to_string(),
            platform: HostPlatform::Windows,
            privilege: HostPrivilege::Normal,
            isolation: HostIsolation::None,
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
            structuredCapabilities: defaultHostCapabilities(),
            onboardingRequirements: vec![HostOnboardingRequirement {
                id: "windows.admin".to_string(),
                title: "管理员权限".to_string(),
                description: "显示当前 Host 是否以管理员身份运行；提升权限必须由系统启动边界决定。"
                    .to_string(),
                capabilityIds: vec!["host.privilege".to_string()],
                status: HostRequirementStatus::Missing,
                action: HostRequirementAction::HostManaged,
            }],
            workspaceRoots: Vec::new(),
        }
    }

    /// Builds the Linux host descriptor used by desktop runtime prompts.
    pub fn linux() -> Self {
        Self {
            id: "linux".to_string(),
            displayName: "Linux".to_string(),
            platform: HostPlatform::Linux,
            privilege: HostPrivilege::Normal,
            isolation: HostIsolation::None,
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
            structuredCapabilities: defaultHostCapabilities(),
            onboardingRequirements: vec![HostOnboardingRequirement {
                id: "linux.root".to_string(),
                title: "root / service account".to_string(),
                description: "显示当前 Host 的系统账号权限；提权必须由系统或部署器完成。"
                    .to_string(),
                capabilityIds: vec!["host.privilege".to_string()],
                status: HostRequirementStatus::Missing,
                action: HostRequirementAction::HostManaged,
            }],
            workspaceRoots: Vec::new(),
        }
    }

    /// Builds the browser host descriptor used by WebAssembly runtime prompts.
    pub fn web() -> Self {
        Self {
            id: "web".to_string(),
            displayName: "Web".to_string(),
            platform: HostPlatform::Web,
            privilege: HostPrivilege::Normal,
            isolation: HostIsolation::OsAppSandbox,
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
            structuredCapabilities: defaultHostCapabilities(),
            onboardingRequirements: Vec::new(),
            workspaceRoots: Vec::new(),
        }
    }
}

impl Default for HostEnvironmentDescriptor {
    fn default() -> Self {
        Self::android()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostPlatform {
    Android,
    Ohos,
    Windows,
    Linux,
    Macos,
    Ios,
    Web,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostPrivilege {
    Normal,
    AndroidShizuku,
    AndroidRoot,
    Administrator,
    Root,
    ServiceAccount,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostIsolation {
    None,
    OsAppSandbox,
    Container,
    VirtualMachine,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityScope {
    FileSystem,
    System,
    Network,
    Runtime,
    Device,
    Media,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityOperation {
    Read,
    Write,
    Execute,
    Connect,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCapability {
    pub id: String,
    pub displayName: String,
    pub scope: CapabilityScope,
    pub operations: Vec<CapabilityOperation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostOnboardingRequirement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub capabilityIds: Vec<String>,
    pub status: HostRequirementStatus,
    pub action: HostRequirementAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostRequirementStatus {
    Satisfied,
    Missing,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostRequirementAction {
    RuntimePermission,
    OpenSystemSettings,
    HostManaged,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRootDescriptor {
    pub id: String,
    pub displayName: String,
    pub vfsRoot: String,
    pub physicalRoot: String,
}

fn defaultHostCapabilities() -> Vec<HostCapability> {
    vec![
        HostCapability {
            id: "fs.read".to_string(),
            displayName: "文件读取".to_string(),
            scope: CapabilityScope::FileSystem,
            operations: vec![CapabilityOperation::Read],
        },
        HostCapability {
            id: "fs.write".to_string(),
            displayName: "文件写入".to_string(),
            scope: CapabilityScope::FileSystem,
            operations: vec![CapabilityOperation::Write],
        },
        HostCapability {
            id: "fs.search".to_string(),
            displayName: "文件搜索".to_string(),
            scope: CapabilityScope::FileSystem,
            operations: vec![CapabilityOperation::Read],
        },
        HostCapability {
            id: "fs.archive".to_string(),
            displayName: "归档文件处理".to_string(),
            scope: CapabilityScope::FileSystem,
            operations: vec![CapabilityOperation::Read, CapabilityOperation::Write],
        },
        HostCapability {
            id: "http.request".to_string(),
            displayName: "HTTP 请求".to_string(),
            scope: CapabilityScope::Network,
            operations: vec![
                CapabilityOperation::Read,
                CapabilityOperation::Write,
                CapabilityOperation::Connect,
            ],
        },
        HostCapability {
            id: "web.visit".to_string(),
            displayName: "网页访问".to_string(),
            scope: CapabilityScope::Network,
            operations: vec![CapabilityOperation::Read, CapabilityOperation::Connect],
        },
        HostCapability {
            id: "os.open".to_string(),
            displayName: "打开文件".to_string(),
            scope: CapabilityScope::System,
            operations: vec![CapabilityOperation::Execute],
        },
        HostCapability {
            id: "os.share".to_string(),
            displayName: "系统分享".to_string(),
            scope: CapabilityScope::System,
            operations: vec![CapabilityOperation::Execute],
        },
        HostCapability {
            id: "terminal.pty".to_string(),
            displayName: "交互终端".to_string(),
            scope: CapabilityScope::Runtime,
            operations: vec![CapabilityOperation::Read, CapabilityOperation::Write],
        },
        HostCapability {
            id: "runtime.process".to_string(),
            displayName: "进程执行".to_string(),
            scope: CapabilityScope::Runtime,
            operations: vec![CapabilityOperation::Execute],
        },
        HostCapability {
            id: "runtime.storage".to_string(),
            displayName: "运行时存储".to_string(),
            scope: CapabilityScope::Runtime,
            operations: vec![CapabilityOperation::Read, CapabilityOperation::Write],
        },
        HostCapability {
            id: "runtime.sqlite".to_string(),
            displayName: "SQLite 存储".to_string(),
            scope: CapabilityScope::Runtime,
            operations: vec![CapabilityOperation::Read, CapabilityOperation::Write],
        },
        HostCapability {
            id: "audio.playback".to_string(),
            displayName: "音频播放".to_string(),
            scope: CapabilityScope::Media,
            operations: vec![CapabilityOperation::Execute],
        },
        HostCapability {
            id: "music.playback".to_string(),
            displayName: "音乐播放控制".to_string(),
            scope: CapabilityScope::Media,
            operations: vec![CapabilityOperation::Read, CapabilityOperation::Execute],
        },
        HostCapability {
            id: "tts.synthesis".to_string(),
            displayName: "语音合成".to_string(),
            scope: CapabilityScope::Media,
            operations: vec![CapabilityOperation::Execute],
        },
        HostCapability {
            id: "tts.playback".to_string(),
            displayName: "语音播放".to_string(),
            scope: CapabilityScope::Media,
            operations: vec![CapabilityOperation::Read, CapabilityOperation::Execute],
        },
        HostCapability {
            id: "system.location".to_string(),
            displayName: "定位".to_string(),
            scope: CapabilityScope::System,
            operations: vec![CapabilityOperation::Read],
        },
        HostCapability {
            id: "system.notifications.read".to_string(),
            displayName: "通知读取".to_string(),
            scope: CapabilityScope::System,
            operations: vec![CapabilityOperation::Read],
        },
        HostCapability {
            id: "system.app_usage".to_string(),
            displayName: "应用使用统计".to_string(),
            scope: CapabilityScope::System,
            operations: vec![CapabilityOperation::Read],
        },
        HostCapability {
            id: "system.app.install".to_string(),
            displayName: "应用安装".to_string(),
            scope: CapabilityScope::System,
            operations: vec![CapabilityOperation::Execute],
        },
        HostCapability {
            id: "system.app.uninstall".to_string(),
            displayName: "应用卸载".to_string(),
            scope: CapabilityScope::System,
            operations: vec![CapabilityOperation::Execute],
        },
        HostCapability {
            id: "system.settings".to_string(),
            displayName: "系统设置".to_string(),
            scope: CapabilityScope::System,
            operations: vec![CapabilityOperation::Read, CapabilityOperation::Write],
        },
        HostCapability {
            id: "bluetooth.classic".to_string(),
            displayName: "经典蓝牙".to_string(),
            scope: CapabilityScope::Device,
            operations: vec![CapabilityOperation::Read, CapabilityOperation::Connect],
        },
        HostCapability {
            id: "bluetooth.ble".to_string(),
            displayName: "低功耗蓝牙".to_string(),
            scope: CapabilityScope::Device,
            operations: vec![CapabilityOperation::Read, CapabilityOperation::Connect],
        },
    ]
}

/// Builds structured capabilities for the provided capability id order.
#[allow(non_snake_case)]
fn hostCapabilities(ids: &[&str]) -> Vec<HostCapability> {
    let capabilities = defaultHostCapabilities();
    ids.iter()
        .map(|id| {
            capabilities
                .iter()
                .find(|capability| capability.id == *id)
                .unwrap_or_else(|| panic!("host capability is not defined: {id}"))
                .clone()
        })
        .collect()
}

fn androidOnboardingRequirements() -> Vec<HostOnboardingRequirement> {
    vec![
        HostOnboardingRequirement {
            id: "android.fileManagement".to_string(),
            title: "文件管理".to_string(),
            description: "Host 需要文件管理授权来读取和写入用户选择的 Android 共享存储目录。"
                .to_string(),
            capabilityIds: vec![
                "fs.read".to_string(),
                "fs.write".to_string(),
                "fs.search".to_string(),
                "fs.archive".to_string(),
            ],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::OpenSystemSettings,
        },
        HostOnboardingRequirement {
            id: "android.notifications".to_string(),
            title: "通知".to_string(),
            description: "Host 需要通知授权来显示前台服务、任务进度和工具执行结果。".to_string(),
            capabilityIds: vec!["system.notifications.send".to_string()],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::RuntimePermission,
        },
        HostOnboardingRequirement {
            id: "android.appList".to_string(),
            title: "应用列表".to_string(),
            description: "Host 需要包可见性声明来列出、启动和停止 Android 应用。".to_string(),
            capabilityIds: vec!["system.app.list".to_string()],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::None,
        },
        HostOnboardingRequirement {
            id: "android.usageStats".to_string(),
            title: "应用使用统计".to_string(),
            description: "Host 需要使用情况访问权限来读取应用前台使用时长。".to_string(),
            capabilityIds: vec!["system.app_usage".to_string()],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::OpenSystemSettings,
        },
        HostOnboardingRequirement {
            id: "android.writeSettings".to_string(),
            title: "系统设置修改".to_string(),
            description: "Host 需要修改系统设置权限来写入允许的 Android 系统设置项。".to_string(),
            capabilityIds: vec!["system.settings".to_string()],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::OpenSystemSettings,
        },
        HostOnboardingRequirement {
            id: "android.location".to_string(),
            title: "附近设备定位".to_string(),
            description: "Host 需要系统定位授权来完成部分附近设备发现能力。".to_string(),
            capabilityIds: vec!["system.location".to_string()],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::RuntimePermission,
        },
        HostOnboardingRequirement {
            id: "android.bluetooth".to_string(),
            title: "蓝牙连接".to_string(),
            description: "Host 需要蓝牙扫描与连接授权来发现和连接设备。".to_string(),
            capabilityIds: vec!["bluetooth.classic".to_string(), "bluetooth.ble".to_string()],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::RuntimePermission,
        },
        HostOnboardingRequirement {
            id: "android.overlay".to_string(),
            title: "悬浮入口".to_string(),
            description: "Host 需要系统悬浮窗授权来在其他应用中显示入口。".to_string(),
            capabilityIds: vec!["android.overlay".to_string()],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::OpenSystemSettings,
        },
        HostOnboardingRequirement {
            id: "android.batteryOptimization".to_string(),
            title: "持续任务".to_string(),
            description: "Host 需要电池优化例外来保持同步、协作和长任务连续。".to_string(),
            capabilityIds: vec!["runtime.background".to_string()],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::OpenSystemSettings,
        },
    ]
}

/// Builds OpenHarmony runtime permission requirements for host onboarding.
#[allow(non_snake_case)]
fn ohosOnboardingRequirements() -> Vec<HostOnboardingRequirement> {
    vec![
        HostOnboardingRequirement {
            id: "ohos.location".to_string(),
            title: "定位授权".to_string(),
            description: "Host 需要系统定位授权来读取设备位置，并支持依赖位置权限的附近设备发现。"
                .to_string(),
            capabilityIds: vec!["system.location".to_string()],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::RuntimePermission,
        },
        HostOnboardingRequirement {
            id: "ohos.bluetooth".to_string(),
            title: "蓝牙授权".to_string(),
            description: "Host 需要蓝牙使用与发现授权来扫描、连接和读写经典蓝牙或 BLE 设备。"
                .to_string(),
            capabilityIds: vec!["bluetooth.classic".to_string(), "bluetooth.ble".to_string()],
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::RuntimePermission,
        },
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostError {
    pub message: String,
}

impl HostError {
    /// Creates a host boundary error from a displayable message.
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSessionInfo {
    pub sessionId: String,
    pub currentUrl: String,
    pub title: String,
    pub userAgent: Option<String>,
    pub active: bool,
    pub canGoBack: bool,
    pub canGoForward: bool,
    pub isLoading: bool,
    pub progress: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSessionCommand {
    pub action: String,
    pub sessionId: Option<String>,
    pub url: Option<String>,
    pub script: Option<String>,
    pub payloadJson: String,
    pub userAgent: Option<String>,
    pub headers: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSessionCommandResult {
    pub success: bool,
    pub session: Option<BrowserSessionInfo>,
    pub sessions: Vec<BrowserSessionInfo>,
    pub resultJson: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSessionSnapshot {
    pub session: BrowserSessionInfo,
    pub resultJson: String,
}

pub trait BrowserSessionHost: Send + Sync {
    /// Lists interactive browser sessions owned by the host.
    fn listBrowserSessions(&self) -> HostResult<Vec<BrowserSessionInfo>>;

    /// Creates an interactive browser session on the host.
    fn createBrowserSession(
        &self,
        initialUrl: &str,
        userAgent: Option<&str>,
        headers: BTreeMap<String, String>,
    ) -> HostResult<BrowserSessionInfo>;

    /// Updates host-owned browser session request metadata.
    fn updateBrowserSession(
        &self,
        sessionId: &str,
        userAgent: Option<&str>,
        headers: BTreeMap<String, String>,
    ) -> HostResult<BrowserSessionInfo>;

    /// Submits a semantic browser command to the host session.
    fn submitBrowserCommand(
        &self,
        command: BrowserSessionCommand,
    ) -> HostResult<BrowserSessionCommandResult>;

    /// Reads the latest host-owned browser session snapshot.
    fn getBrowserSessionSnapshot(&self, sessionId: &str) -> HostResult<BrowserSessionSnapshot>;

    /// Closes a host-owned browser session.
    fn closeBrowserSession(&self, sessionId: &str) -> HostResult<BrowserSessionCommandResult>;
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpDownloadFileRequest {
    pub fileId: String,
    pub url: String,
    pub targetPath: String,
    pub headers: Vec<(String, String)>,
    pub expectedBytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpDownloadRequest {
    pub downloadId: String,
    pub files: Vec<HttpDownloadFileRequest>,
    pub maxConcurrency: usize,
    pub connectTimeoutSeconds: u64,
    pub readTimeoutSeconds: u64,
    pub followRedirects: bool,
    pub ignoreSsl: bool,
    pub proxyHost: String,
    pub proxyPort: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpDownloadProgressState {
    Started,
    Downloading,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpDownloadProgress {
    pub downloadId: String,
    pub fileId: String,
    pub state: HttpDownloadProgressState,
    pub fileDownloadedBytes: u64,
    pub fileTotalBytes: u64,
    pub downloadedBytes: u64,
    pub totalBytes: u64,
    pub completedFiles: usize,
    pub totalFiles: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpDownloadFileResult {
    pub fileId: String,
    pub finalUrl: String,
    pub targetPath: String,
    pub downloadedBytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpDownloadResult {
    pub downloadId: String,
    pub files: Vec<HttpDownloadFileResult>,
    pub downloadedBytes: u64,
}

#[derive(Clone, Debug, Default)]
pub struct HttpDownloadControl {
    cancelled: Arc<AtomicBool>,
}

impl HttpDownloadControl {
    /// Creates an active download control token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation for every file in the associated download operation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Returns whether cancellation has been requested.
    pub fn isCancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub type HttpDownloadProgressCallback = Arc<dyn Fn(HttpDownloadProgress) + Send + Sync + 'static>;

pub trait HttpHost: Send + Sync {
    /// Executes one buffered HTTP request.
    fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData>;

    /// Downloads files with bounded worker concurrency and progress reporting.
    fn downloadFiles(
        &self,
        request: HttpDownloadRequest,
        control: HttpDownloadControl,
        onProgress: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult>;
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
    /// Writes one protocol line to the managed runtime process.
    fn writeLine(&self, line: &str) -> HostResult<()>;
    /// Writes multiple protocol lines to the managed runtime process in one call.
    fn writeLines(&self, lines: &[String]) -> HostResult<()>;
    /// Reads one protocol line from the managed runtime process.
    fn readStdoutLine(&self, timeoutMs: u64) -> HostResult<Option<String>>;
    /// Drains stderr text collected from the managed runtime process.
    fn drainStderr(&self) -> HostResult<String>;
    /// Returns whether the managed runtime process is still running.
    fn isRunning(&self) -> HostResult<bool>;
    /// Terminates the managed runtime process.
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
        terminalType: &str,
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
    /// Returns the physical root used for runtime storage entries.
    fn runtimeRootDir(&self) -> Option<PathBuf>;
    /// Returns the physical root used for workspace storage entries.
    fn workspaceRootDir(&self) -> Option<PathBuf>;
    /// Reads bytes from a virtual runtime storage path.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>>;
    /// Writes bytes to a virtual runtime storage path.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()>;
    /// Deletes a virtual runtime storage entry.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()>;
    /// Checks whether a virtual runtime storage entry exists.
    fn exists(&self, path: &str) -> HostResult<bool>;
    /// Lists entries under a virtual runtime storage prefix.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>>;
}

pub trait HostSecretStore: Send + Sync {
    fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>>;
    fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()>;
    fn deleteSecret(&self, key: &str) -> HostResult<()>;
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostRuntimeEventScheduleKind {
    #[serde(rename = "timer")]
    Timer,
    #[serde(rename = "interval")]
    Interval,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRuntimeEventSchedule {
    pub scheduleId: String,
    pub containerPackageName: String,
    pub hookId: String,
    pub kind: HostRuntimeEventScheduleKind,
    pub delayMs: u64,
    pub intervalMs: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostRuntimeEventScheduleFire {
    pub scheduleId: String,
    pub scheduledAtMillis: u64,
    pub firedAtMillis: u64,
}

pub trait HostRuntimeEventSchedulerHost: Send + Sync {
    /// Replaces all active ToolPkg schedules and installs their firing callback.
    fn replaceHostRuntimeEventSchedules(
        &self,
        schedules: Vec<HostRuntimeEventSchedule>,
        sink: HostRuntimeEventScheduleSink,
    ) -> HostResult<()>;
}

/// Owns a one-shot runtime task that must execute outside Core's synchronous startup path.
pub type HostRuntimeTask = Box<dyn FnOnce() + Send + 'static>;

/// Owns an asynchronous runtime task scheduled by the platform event executor.
///
/// Native schedulers move tasks to named threads, while browser futures remain
/// local to the JavaScript event loop and therefore cannot require `Send`.
#[cfg(not(target_arch = "wasm32"))]
pub type HostRuntimeAsyncTask = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// Owns a browser-local asynchronous runtime task.
#[cfg(target_arch = "wasm32")]
pub type HostRuntimeAsyncTask = Pin<Box<dyn Future<Output = ()> + 'static>>;

pub trait HostRuntimeTaskSchedulerHost: Send + Sync {
    /// Schedules a named one-shot runtime task through the platform execution mechanism.
    fn scheduleHostRuntimeTask(&self, taskName: &str, task: HostRuntimeTask) -> HostResult<()>;

    /// Schedules a named asynchronous runtime task through the platform executor.
    fn scheduleHostRuntimeAsyncTask(
        &self,
        taskName: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()>;

    /// Schedules a named runtime task after a platform-owned delay.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        taskName: &str,
        delayMs: u64,
        task: HostRuntimeTask,
    ) -> HostResult<()>;
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
    /// Returns whether this host can speak text through a platform system voice.
    fn supportsSystemSpeech(&self) -> bool;

    /// Starts playback for one generated speech audio file.
    fn playAudio(&self, path: &str) -> HostResult<TtsPlaybackStatus>;

    /// Starts one platform-provided system speech request.
    fn speakText(&self, request: TtsPlaybackRequest) -> HostResult<TtsPlaybackStatus>;

    /// Pauses the active speech session.
    fn pauseSpeech(&self) -> HostResult<TtsPlaybackStatus>;

    /// Resumes the active speech session.
    fn resumeSpeech(&self) -> HostResult<TtsPlaybackStatus>;

    /// Stops the active speech session.
    fn stopSpeech(&self) -> HostResult<TtsPlaybackStatus>;

    /// Returns the current speech session state.
    fn speechState(&self) -> HostResult<TtsPlaybackStatus>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalSttInferenceHostRequest {
    pub engineLibraryDirectory: String,
    pub modelDirectory: String,
    pub driverJson: String,
    pub audioPath: String,
    pub language: Option<String>,
    pub optionsJson: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalSttInferenceHostResponse {
    pub text: String,
    pub resultJson: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalTtsInferenceHostRequest {
    pub engineLibraryDirectory: String,
    pub modelDirectory: String,
    pub driverJson: String,
    pub text: String,
    pub voice: String,
    pub speed: f64,
    pub outputPath: String,
    pub optionsJson: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LocalTtsInferenceHostResponse {
    pub audioPath: String,
    pub outputFormat: String,
}

pub trait LocalInferenceHost: Send + Sync {
    /// Transcribes one local audio request through a platform inference engine.
    fn transcribeLocalSpeech(
        &self,
        request: LocalSttInferenceHostRequest,
    ) -> HostResult<LocalSttInferenceHostResponse>;

    /// Synthesizes one local speech request through a platform inference engine.
    fn synthesizeLocalSpeech(
        &self,
        request: LocalTtsInferenceHostRequest,
    ) -> HostResult<LocalTtsInferenceHostResponse>;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Builds a sorted id set from borrowed string ids.
    fn idSet(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    /// Returns a sorted id set for descriptor capability ids.
    fn descriptorCapabilitySet(descriptor: &HostEnvironmentDescriptor) -> BTreeSet<String> {
        descriptor.capabilities.iter().cloned().collect()
    }

    /// Returns a sorted id set for descriptor structured capability ids.
    fn structuredCapabilitySet(descriptor: &HostEnvironmentDescriptor) -> BTreeSet<String> {
        descriptor
            .structuredCapabilities
            .iter()
            .map(|capability| capability.id.clone())
            .collect()
    }

    /// Verifies that OpenHarmony advertises the implemented non-terminal host capabilities.
    #[test]
    fn ohosDescriptorExposesImplementedHostCapabilities() {
        let descriptor = HostEnvironmentDescriptor::ohos();
        let actual = descriptorCapabilitySet(&descriptor);
        let expected = idSet(&[
            "fs.read",
            "fs.write",
            "fs.search",
            "fs.archive",
            "http.request",
            "web.visit",
            "os.open",
            "os.share",
            "terminal.pty",
            "audio.playback",
            "music.playback",
            "tts.synthesis",
            "tts.playback",
            "bluetooth.classic",
            "bluetooth.ble",
            "system.location",
            "system.notifications.read",
            "system.app_usage",
            "system.app.install",
            "system.app.uninstall",
            "system.settings",
            "runtime.process",
            "runtime.storage",
            "runtime.sqlite",
        ]);
        let missing: Vec<_> = expected.difference(&actual).cloned().collect();

        assert!(
            missing.is_empty(),
            "OpenHarmony host descriptor is missing capabilities: {missing:?}"
        );
    }

    /// Verifies that raw and structured OpenHarmony capabilities stay in sync.
    #[test]
    fn ohosStructuredCapabilitiesMatchDescriptorCapabilities() {
        let descriptor = HostEnvironmentDescriptor::ohos();
        let raw = descriptorCapabilitySet(&descriptor);
        let structured = structuredCapabilitySet(&descriptor);

        assert_eq!(raw, structured);
    }

    /// Verifies that OpenHarmony does not advertise unimplemented OCR capability.
    #[test]
    fn ohosDescriptorExcludesUnimplementedCapabilities() {
        let descriptor = HostEnvironmentDescriptor::ohos();
        let actual = descriptorCapabilitySet(&descriptor);
        let unimplemented = idSet(&["ocr.recognition"]);
        let advertised: Vec<_> = actual.intersection(&unimplemented).cloned().collect();

        assert!(
            advertised.is_empty(),
            "OpenHarmony host descriptor advertises unimplemented capabilities: {advertised:?}"
        );
    }

    /// Verifies that OpenHarmony onboarding exposes required runtime permissions.
    #[test]
    fn ohosOnboardingRequirementsExposeRuntimePermissions() {
        let descriptor = HostEnvironmentDescriptor::ohos();
        let actual: BTreeSet<_> = descriptor
            .onboardingRequirements
            .iter()
            .map(|requirement| requirement.id.clone())
            .collect();
        let expected = idSet(&["ohos.location", "ohos.bluetooth"]);
        let missing: Vec<_> = expected.difference(&actual).cloned().collect();

        assert!(
            missing.is_empty(),
            "OpenHarmony onboarding requirements are missing ids: {missing:?}"
        );
    }
}
