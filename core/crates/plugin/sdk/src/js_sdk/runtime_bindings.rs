//! Canonical bindings from every JavaScript Tools method to the registered executor contract.

use super::tool_types::BuiltinToolName;

/// Binds one public Tools method to its registered executor and result presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsToolBinding {
    /// Contains the dotted JavaScript namespace path.
    pub namespace: &'static str,
    /// Contains the public method name inside the namespace.
    pub method: &'static str,
    /// Selects the exact executor contract invoked through the common tool pipeline.
    pub target: BuiltinToolName,
    /// Validates exact SDK arguments before submitting the ordinary tool call.
    pub strict_arguments: bool,
    /// Selects only how the public result is presented, without changing invocation transport.
    pub facade: JsToolFacade,
    /// Declares fixed executor parameters that are absent from the public SDK arguments.
    pub fixed_parameters: &'static [(&'static str, &'static str)],
}

/// Selects JavaScript presentation without declaring a second native transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsToolFacade {
    Promise,
    AsyncIterator,
    TerminalCallback,
    Storage,
}

/// Declares parameter conventions shared by every generated built-in tool binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsToolParameterRules {
    /// Lists arguments whose object fields become executor parameters.
    pub flatten_arguments: &'static [&'static str],
    /// Lists union argument suffixes whose scalar branch uses the name before the suffix.
    pub scalar_union_suffixes: &'static [&'static str],
    /// Lists namespaces whose flattened object fields retain their public spelling.
    pub preserve_object_namespaces: &'static [&'static str],
    /// Maps exact namespace, method and positional argument names to executor field names.
    pub argument_names: &'static [(&'static str, &'static str, &'static str, &'static str)],
}

/// Contains the sole declaration of executor argument naming and flattening conventions.
pub const JS_TOOL_PARAMETER_RULES: JsToolParameterRules = JsToolParameterRules {
    flatten_arguments: &["options", "params", "updates"],
    scalar_union_suffixes: &["OrOptions", "OrParams"],
    preserve_object_namespaces: &["Net", "Media"],
    argument_names: &[
        ("Chat", "cancel", "chatId", "chatId"),
        ("Files", "writeBinary", "base64Content", "base64Content"),
        ("Files", "apply", "newContent", "new"),
        ("Files", "create", "newContent", "new"),
        ("Files", "edit", "newContent", "new"),
        ("Files", "edit", "oldContent", "old"),
        ("System", "sleep", "milliseconds", "duration_ms"),
        ("System", "listApps", "includeSystem", "include_system_apps"),
    ],
};

/// Declares one versioned JavaScript Tools method implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsToolApiVariant {
    /// Contains the JavaScript namespace containing the gated method.
    pub namespace: &'static str,
    /// Contains the public method name controlled by the gate.
    pub method: &'static str,
    /// Contains the first version selecting this implementation within its API family.
    pub since: &'static str,
    /// Contains an exclusive upper bound; None ends at the next major API family.
    pub until: Option<&'static str>,
    /// Contains an optional variant-specific JavaScript parameter list.
    pub arguments: Option<&'static [&'static str]>,
}

/// Contains every active JavaScript Tools method binding.
#[rustfmt::skip]
pub const JS_TOOL_BINDINGS: &[JsToolBinding] = &[
    JsToolBinding { namespace: "Net", method: "webSocket", target: BuiltinToolName::WebSocket, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "agentStatus", target: BuiltinToolName::AgentStatus, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "createNew", target: BuiltinToolName::CreateNewChat, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "deleteChat", target: BuiltinToolName::DeleteChat, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "findChat", target: BuiltinToolName::FindChat, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "getMessages", target: BuiltinToolName::GetChatMessages, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "getMessagesRange", target: BuiltinToolName::GetChatMessagesRange, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "call", target: BuiltinToolName::CallChatModel, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "listAll", target: BuiltinToolName::ListChats, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[("list_all", "true")] },
    JsToolBinding { namespace: "Chat", method: "listChats", target: BuiltinToolName::ListChats, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "startService", target: BuiltinToolName::StartChatService, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "stopService", target: BuiltinToolName::StopChatService, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "switchTo", target: BuiltinToolName::SwitchChat, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "updateTitle", target: BuiltinToolName::UpdateChatTitle, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "updateGroup", target: BuiltinToolName::UpdateChatGroup, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "updatePinned", target: BuiltinToolName::UpdateChatPinned, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "updateLocked", target: BuiltinToolName::UpdateChatLocked, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "reorder", target: BuiltinToolName::ReorderChats, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "apply", target: BuiltinToolName::ApplyFile, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "copy", target: BuiltinToolName::CopyFile, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "create", target: BuiltinToolName::CreateFile, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "deleteFile", target: BuiltinToolName::DeleteFile, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "download", target: BuiltinToolName::DownloadFile, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "edit", target: BuiltinToolName::EditFile, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "exists", target: BuiltinToolName::FileExists, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "find", target: BuiltinToolName::FindFiles, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "grep", target: BuiltinToolName::GrepCode, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "grepContext", target: BuiltinToolName::GrepContext, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "info", target: BuiltinToolName::FileInfo, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "list", target: BuiltinToolName::ListFiles, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "mkdir", target: BuiltinToolName::MakeDirectory, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "move", target: BuiltinToolName::MoveFile, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "open", target: BuiltinToolName::OpenFile, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "read", target: BuiltinToolName::ReadFileFull, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "readBinary", target: BuiltinToolName::ReadFileBinary, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "readPart", target: BuiltinToolName::ReadFilePart, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "share", target: BuiltinToolName::ShareFile, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "unzip", target: BuiltinToolName::UnzipFiles, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "write", target: BuiltinToolName::WriteFile, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "writeBinary", target: BuiltinToolName::WriteFileBinary, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Files", method: "zip", target: BuiltinToolName::ZipFiles, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserClick", target: BuiltinToolName::BrowserClick, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserClose", target: BuiltinToolName::BrowserClose, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserCloseAll", target: BuiltinToolName::BrowserCloseAll, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserConsoleMessages", target: BuiltinToolName::BrowserConsoleMessages, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserDrag", target: BuiltinToolName::BrowserDrag, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserEvaluate", target: BuiltinToolName::BrowserEvaluate, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserFileUpload", target: BuiltinToolName::BrowserFileUpload, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserFillForm", target: BuiltinToolName::BrowserFillForm, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserHandleDialog", target: BuiltinToolName::BrowserHandleDialog, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserHover", target: BuiltinToolName::BrowserHover, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserNavigate", target: BuiltinToolName::BrowserNavigate, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserNavigateBack", target: BuiltinToolName::BrowserNavigateBack, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserNetworkRequests", target: BuiltinToolName::BrowserNetworkRequests, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserPressKey", target: BuiltinToolName::BrowserPressKey, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserResize", target: BuiltinToolName::BrowserResize, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserRunCode", target: BuiltinToolName::BrowserRunCode, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserSelectOption", target: BuiltinToolName::BrowserSelectOption, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserSnapshot", target: BuiltinToolName::BrowserSnapshot, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserTabs", target: BuiltinToolName::BrowserTabs, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserTakeScreenshot", target: BuiltinToolName::BrowserTakeScreenshot, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserType", target: BuiltinToolName::BrowserType, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "browserWaitFor", target: BuiltinToolName::BrowserWaitFor, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "http", target: BuiltinToolName::HttpRequest, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "httpGet", target: BuiltinToolName::HttpRequest, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[("method", "GET")] },
    JsToolBinding { namespace: "Net", method: "httpPost", target: BuiltinToolName::HttpRequest, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[("method", "POST")] },
    JsToolBinding { namespace: "Net", method: "uploadFile", target: BuiltinToolName::MultipartRequest, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net", method: "visit", target: BuiltinToolName::VisitWeb, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Net.cookies", method: "clear", target: BuiltinToolName::ManageCookies, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[("action", "clear")] },
    JsToolBinding { namespace: "Net.cookies", method: "get", target: BuiltinToolName::ManageCookies, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[("action", "get")] },
    JsToolBinding { namespace: "Net.cookies", method: "set", target: BuiltinToolName::ManageCookies, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[("action", "set")] },
    JsToolBinding { namespace: "SoftwareSettings", method: "exec", target: BuiltinToolName::ExecuteCliCommand, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "SoftwareSettings", method: "readEnvironmentVariable", target: BuiltinToolName::ReadEnvironmentVariable, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "SoftwareSettings", method: "writeEnvironmentVariable", target: BuiltinToolName::WriteEnvironmentVariable, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "getAppUsageTime", target: BuiltinToolName::GetAppUsageTime, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "getDeviceInfo", target: BuiltinToolName::DeviceInfo, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "getLocation", target: BuiltinToolName::GetDeviceLocation, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "getNotifications", target: BuiltinToolName::GetNotifications, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "getSetting", target: BuiltinToolName::GetSystemSetting, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "installApp", target: BuiltinToolName::InstallApp, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "listApps", target: BuiltinToolName::ListInstalledApps, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "sendNotification", target: BuiltinToolName::SendNotification, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "setSetting", target: BuiltinToolName::ModifySystemSetting, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "sleep", target: BuiltinToolName::Sleep, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "startApp", target: BuiltinToolName::StartApp, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "stopApp", target: BuiltinToolName::StopApp, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "toast", target: BuiltinToolName::Toast, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "uninstallApp", target: BuiltinToolName::UninstallApp, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System", method: "usePackage", target: BuiltinToolName::UsePackage, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "accept", target: BuiltinToolName::BluetoothAccept, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "close", target: BuiltinToolName::BluetoothClose, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "connect", target: BuiltinToolName::BluetoothConnect, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "getState", target: BuiltinToolName::GetBluetoothState, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "listBondedDevices", target: BuiltinToolName::ListBluetoothBondedDevices, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "listen", target: BuiltinToolName::BluetoothListen, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "read", target: BuiltinToolName::BluetoothRead, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "requestEnable", target: BuiltinToolName::RequestEnableBluetooth, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "requestPermission", target: BuiltinToolName::RequestBluetoothPermission, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "scan", target: BuiltinToolName::ScanBluetoothDevices, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "send", target: BuiltinToolName::BluetoothSend, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth", method: "sendAndRead", target: BuiltinToolName::BluetoothSendAndRead, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth.ble", method: "connect", target: BuiltinToolName::BluetoothBleConnect, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth.ble", method: "discoverServices", target: BuiltinToolName::BluetoothBleDiscoverServices, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth.ble", method: "readCharacteristic", target: BuiltinToolName::BluetoothBleReadCharacteristic, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth.ble", method: "readNotifications", target: BuiltinToolName::BluetoothBleReadNotifications, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth.ble", method: "subscribe", target: BuiltinToolName::BluetoothBleSubscribeCharacteristic, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth.ble", method: "writeAndReadCharacteristic", target: BuiltinToolName::BluetoothBleWriteAndReadCharacteristic, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.bluetooth.ble", method: "writeCharacteristic", target: BuiltinToolName::BluetoothBleWriteCharacteristic, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.music", method: "pause", target: BuiltinToolName::MusicPause, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.music", method: "play", target: BuiltinToolName::MusicPlay, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.music", method: "resume", target: BuiltinToolName::MusicResume, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.music", method: "seek", target: BuiltinToolName::MusicSeek, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.music", method: "setVolume", target: BuiltinToolName::MusicSetVolume, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.music", method: "status", target: BuiltinToolName::MusicStatus, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.music", method: "stop", target: BuiltinToolName::MusicStop, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.terminal", method: "close", target: BuiltinToolName::CloseTerminalSession, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.terminal", method: "create", target: BuiltinToolName::CreateTerminalSession, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.terminal", method: "exec", target: BuiltinToolName::ExecuteInTerminalSession, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.terminal", method: "execStreaming", target: BuiltinToolName::TerminalObserve, strict_arguments: false, facade: JsToolFacade::TerminalCallback, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.terminal", method: "execStream", target: BuiltinToolName::TerminalObserve, strict_arguments: false, facade: JsToolFacade::AsyncIterator, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.terminal", method: "hiddenExec", target: BuiltinToolName::ExecuteHiddenTerminalCommand, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.terminal", method: "info", target: BuiltinToolName::GetTerminalInfo, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.terminal", method: "input", target: BuiltinToolName::InputInTerminalSession, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "System.terminal", method: "screen", target: BuiltinToolName::GetTerminalSessionScreen, strict_arguments: false, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Storage", method: "request", target: BuiltinToolName::StorageRequest, strict_arguments: true, facade: JsToolFacade::Storage, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "sendMessage", target: BuiltinToolName::ChatSendMessage, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "sendMessageStreaming", target: BuiltinToolName::ChatObserve, strict_arguments: true, facade: JsToolFacade::AsyncIterator, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "cancel", target: BuiltinToolName::ChatCancel, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "readExtension", target: BuiltinToolName::ChatReadExtension, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "writeExtension", target: BuiltinToolName::ChatWriteExtension, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Chat", method: "deleteExtension", target: BuiltinToolName::ChatDeleteExtension, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Models", method: "listSummaries", target: BuiltinToolName::ModelsListSummaries, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Tts", method: "synthesize", target: BuiltinToolName::TtsSynthesize, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Tts", method: "speak", target: BuiltinToolName::TtsSpeak, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Stt", method: "transcribe", target: BuiltinToolName::SttTranscribe, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Tts", method: "listConfigs", target: BuiltinToolName::TtsListConfigs, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Tts", method: "getCurrentConfigId", target: BuiltinToolName::TtsGetCurrentConfigId, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Tts", method: "setCurrentConfigId", target: BuiltinToolName::TtsSetCurrentConfigId, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Theme", method: "listConfigs", target: BuiltinToolName::ThemeListConfigs, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Theme", method: "applyConfig", target: BuiltinToolName::ThemeApplyConfig, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Catalog", method: "readToolSources", target: BuiltinToolName::CatalogReadToolSources, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Media", method: "register", target: BuiltinToolName::MediaRegister, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
    JsToolBinding { namespace: "Media", method: "registerFromPath", target: BuiltinToolName::MediaRegisterFromPath, strict_arguments: true, facade: JsToolFacade::Promise, fixed_parameters: &[] },
];

/// Contains every versioned implementation applied to generated Tools methods.
pub const JS_TOOL_API_VARIANTS: &[JsToolApiVariant] = &[
    JsToolApiVariant { namespace: "Net", method: "webSocket", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Storage", method: "request", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Chat", method: "sendMessage", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Chat", method: "sendMessageStreaming", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Chat", method: "cancel", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Chat", method: "readExtension", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Chat", method: "writeExtension", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Chat", method: "deleteExtension", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Models", method: "listSummaries", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Tts", method: "synthesize", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Tts", method: "speak", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Stt", method: "transcribe", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Tts", method: "listConfigs", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Tts", method: "getCurrentConfigId", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Tts", method: "setCurrentConfigId", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Theme", method: "listConfigs", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Theme", method: "applyConfig", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Catalog", method: "readToolSources", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Media", method: "register", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant { namespace: "Media", method: "registerFromPath", since: "2.0.0", until: None, arguments: None },
    JsToolApiVariant {
        namespace: "Chat",
        method: "call",
        since: "1.0.1",
        until: Some("2.0.0"),
        arguments: None,
    },
    JsToolApiVariant {
        namespace: "Chat",
        method: "call",
        since: "2.0.0",
        until: None,
        arguments: None,
    },
];

