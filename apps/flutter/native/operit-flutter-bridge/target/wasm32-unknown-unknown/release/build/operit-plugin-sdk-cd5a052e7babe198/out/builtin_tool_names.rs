// Generated from ToolResultMap. Do not edit.

/// Identifies one statically declared built-in tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BuiltinToolName {
    /// Selects the `list_files` built-in tool.
    ListFiles,
    /// Selects the `read_file` built-in tool.
    ReadFile,
    /// Selects the `read_file_part` built-in tool.
    ReadFilePart,
    /// Selects the `read_file_full` built-in tool.
    ReadFileFull,
    /// Selects the `read_file_binary` built-in tool.
    ReadFileBinary,
    /// Selects the `write_file` built-in tool.
    WriteFile,
    /// Selects the `write_file_binary` built-in tool.
    WriteFileBinary,
    /// Selects the `delete_file` built-in tool.
    DeleteFile,
    /// Selects the `file_exists` built-in tool.
    FileExists,
    /// Selects the `move_file` built-in tool.
    MoveFile,
    /// Selects the `copy_file` built-in tool.
    CopyFile,
    /// Selects the `make_directory` built-in tool.
    MakeDirectory,
    /// Selects the `find_files` built-in tool.
    FindFiles,
    /// Selects the `grep_code` built-in tool.
    GrepCode,
    /// Selects the `grep_context` built-in tool.
    GrepContext,
    /// Selects the `file_info` built-in tool.
    FileInfo,
    /// Selects the `zip_files` built-in tool.
    ZipFiles,
    /// Selects the `unzip_files` built-in tool.
    UnzipFiles,
    /// Selects the `open_file` built-in tool.
    OpenFile,
    /// Selects the `share_file` built-in tool.
    ShareFile,
    /// Selects the `download_file` built-in tool.
    DownloadFile,
    /// Selects the `apply_file` built-in tool.
    ApplyFile,
    /// Selects the `create_file` built-in tool.
    CreateFile,
    /// Selects the `edit_file` built-in tool.
    EditFile,
    /// Selects the `http_request` built-in tool.
    HttpRequest,
    /// Selects the `visit_web` built-in tool.
    VisitWeb,
    /// Selects the `browser_click` built-in tool.
    BrowserClick,
    /// Selects the `browser_close` built-in tool.
    BrowserClose,
    /// Selects the `browser_close_all` built-in tool.
    BrowserCloseAll,
    /// Selects the `browser_console_messages` built-in tool.
    BrowserConsoleMessages,
    /// Selects the `browser_drag` built-in tool.
    BrowserDrag,
    /// Selects the `browser_evaluate` built-in tool.
    BrowserEvaluate,
    /// Selects the `browser_file_upload` built-in tool.
    BrowserFileUpload,
    /// Selects the `browser_fill_form` built-in tool.
    BrowserFillForm,
    /// Selects the `browser_handle_dialog` built-in tool.
    BrowserHandleDialog,
    /// Selects the `browser_hover` built-in tool.
    BrowserHover,
    /// Selects the `browser_navigate` built-in tool.
    BrowserNavigate,
    /// Selects the `browser_navigate_back` built-in tool.
    BrowserNavigateBack,
    /// Selects the `browser_network_requests` built-in tool.
    BrowserNetworkRequests,
    /// Selects the `browser_press_key` built-in tool.
    BrowserPressKey,
    /// Selects the `browser_resize` built-in tool.
    BrowserResize,
    /// Selects the `browser_run_code` built-in tool.
    BrowserRunCode,
    /// Selects the `browser_select_option` built-in tool.
    BrowserSelectOption,
    /// Selects the `browser_wait_for` built-in tool.
    BrowserWaitFor,
    /// Selects the `browser_snapshot` built-in tool.
    BrowserSnapshot,
    /// Selects the `browser_take_screenshot` built-in tool.
    BrowserTakeScreenshot,
    /// Selects the `browser_type` built-in tool.
    BrowserType,
    /// Selects the `browser_tabs` built-in tool.
    BrowserTabs,
    /// Selects the `multipart_request` built-in tool.
    MultipartRequest,
    /// Selects the `manage_cookies` built-in tool.
    ManageCookies,
    /// Selects the `sleep` built-in tool.
    Sleep,
    /// Selects the `get_system_setting` built-in tool.
    GetSystemSetting,
    /// Selects the `modify_system_setting` built-in tool.
    ModifySystemSetting,
    /// Selects the `toast` built-in tool.
    Toast,
    /// Selects the `send_notification` built-in tool.
    SendNotification,
    /// Selects the `install_app` built-in tool.
    InstallApp,
    /// Selects the `uninstall_app` built-in tool.
    UninstallApp,
    /// Selects the `list_installed_apps` built-in tool.
    ListInstalledApps,
    /// Selects the `start_app` built-in tool.
    StartApp,
    /// Selects the `stop_app` built-in tool.
    StopApp,
    /// Selects the `device_info` built-in tool.
    DeviceInfo,
    /// Selects the `get_notifications` built-in tool.
    GetNotifications,
    /// Selects the `get_app_usage_time` built-in tool.
    GetAppUsageTime,
    /// Selects the `get_device_location` built-in tool.
    GetDeviceLocation,
    /// Selects the `capture_screenshot` built-in tool.
    CaptureScreenshot,
    /// Selects the `request_bluetooth_permission` built-in tool.
    RequestBluetoothPermission,
    /// Selects the `get_bluetooth_state` built-in tool.
    GetBluetoothState,
    /// Selects the `request_enable_bluetooth` built-in tool.
    RequestEnableBluetooth,
    /// Selects the `list_bluetooth_bonded_devices` built-in tool.
    ListBluetoothBondedDevices,
    /// Selects the `scan_bluetooth_devices` built-in tool.
    ScanBluetoothDevices,
    /// Selects the `bluetooth_connect` built-in tool.
    BluetoothConnect,
    /// Selects the `bluetooth_listen` built-in tool.
    BluetoothListen,
    /// Selects the `bluetooth_accept` built-in tool.
    BluetoothAccept,
    /// Selects the `bluetooth_send` built-in tool.
    BluetoothSend,
    /// Selects the `bluetooth_read` built-in tool.
    BluetoothRead,
    /// Selects the `bluetooth_send_and_read` built-in tool.
    BluetoothSendAndRead,
    /// Selects the `bluetooth_close` built-in tool.
    BluetoothClose,
    /// Selects the `bluetooth_ble_connect` built-in tool.
    BluetoothBleConnect,
    /// Selects the `bluetooth_ble_discover_services` built-in tool.
    BluetoothBleDiscoverServices,
    /// Selects the `bluetooth_ble_read_characteristic` built-in tool.
    BluetoothBleReadCharacteristic,
    /// Selects the `bluetooth_ble_write_characteristic` built-in tool.
    BluetoothBleWriteCharacteristic,
    /// Selects the `bluetooth_ble_write_and_read_characteristic` built-in tool.
    BluetoothBleWriteAndReadCharacteristic,
    /// Selects the `bluetooth_ble_subscribe_characteristic` built-in tool.
    BluetoothBleSubscribeCharacteristic,
    /// Selects the `bluetooth_ble_read_notifications` built-in tool.
    BluetoothBleReadNotifications,
    /// Selects the `read_environment_variable` built-in tool.
    ReadEnvironmentVariable,
    /// Selects the `write_environment_variable` built-in tool.
    WriteEnvironmentVariable,
    /// Selects the `execute_cli_command` built-in tool.
    ExecuteCliCommand,
    /// Selects the `use_package` built-in tool.
    UsePackage,
    /// Selects the `list_core_nodes` built-in tool.
    ListCoreNodes,
    /// Selects the `switch_core` built-in tool.
    SwitchCore,
    /// Selects the `package_proxy` built-in tool.
    PackageProxy,
    /// Selects the `get_terminal_info` built-in tool.
    GetTerminalInfo,
    /// Selects the `execute_in_terminal_session` built-in tool.
    ExecuteInTerminalSession,
    /// Selects the `execute_in_terminal_session_streaming` built-in tool.
    ExecuteInTerminalSessionStreaming,
    /// Selects the `execute_hidden_terminal_command` built-in tool.
    ExecuteHiddenTerminalCommand,
    /// Selects the `create_terminal_session` built-in tool.
    CreateTerminalSession,
    /// Selects the `close_terminal_session` built-in tool.
    CloseTerminalSession,
    /// Selects the `input_in_terminal_session` built-in tool.
    InputInTerminalSession,
    /// Selects the `get_terminal_session_screen` built-in tool.
    GetTerminalSessionScreen,
    /// Selects the `music_play` built-in tool.
    MusicPlay,
    /// Selects the `music_pause` built-in tool.
    MusicPause,
    /// Selects the `music_resume` built-in tool.
    MusicResume,
    /// Selects the `music_stop` built-in tool.
    MusicStop,
    /// Selects the `music_seek` built-in tool.
    MusicSeek,
    /// Selects the `music_set_volume` built-in tool.
    MusicSetVolume,
    /// Selects the `music_status` built-in tool.
    MusicStatus,
    /// Selects the `start_chat_service` built-in tool.
    StartChatService,
    /// Selects the `stop_chat_service` built-in tool.
    StopChatService,
    /// Selects the `create_new_chat` built-in tool.
    CreateNewChat,
    /// Selects the `list_chats` built-in tool.
    ListChats,
    /// Selects the `find_chat` built-in tool.
    FindChat,
    /// Selects the `agent_status` built-in tool.
    AgentStatus,
    /// Selects the `switch_chat` built-in tool.
    SwitchChat,
    /// Selects the `update_chat_title` built-in tool.
    UpdateChatTitle,
    /// Selects the `delete_chat` built-in tool.
    DeleteChat,
    /// Selects the `send_message_to_ai` built-in tool.
    SendMessageToAi,
    /// Selects the `send_message_to_ai_streaming` built-in tool.
    SendMessageToAiStreaming,
    /// Selects the `list_character_cards` built-in tool.
    ListCharacterCards,
    /// Selects the `get_chat_messages` built-in tool.
    GetChatMessages,
    /// Selects the `query_memory` built-in tool.
    QueryMemory,
    /// Selects the `get_memory_by_title` built-in tool.
    GetMemoryByTitle,
    /// Selects the `create_memory` built-in tool.
    CreateMemory,
    /// Selects the `update_memory` built-in tool.
    UpdateMemory,
    /// Selects the `delete_memory` built-in tool.
    DeleteMemory,
    /// Selects the `move_memory` built-in tool.
    MoveMemory,
    /// Selects the `link_memories` built-in tool.
    LinkMemories,
    /// Selects the `query_memory_links` built-in tool.
    QueryMemoryLinks,
    /// Selects the `update_memory_link` built-in tool.
    UpdateMemoryLink,
    /// Selects the `delete_memory_link` built-in tool.
    DeleteMemoryLink,
    /// Selects the `update_user_preferences` built-in tool.
    UpdateUserPreferences,
}

impl BuiltinToolName {
    /// Contains every statically declared built-in tool.
    pub const ALL: &'static [Self] = &[
        Self::ListFiles,
        Self::ReadFile,
        Self::ReadFilePart,
        Self::ReadFileFull,
        Self::ReadFileBinary,
        Self::WriteFile,
        Self::WriteFileBinary,
        Self::DeleteFile,
        Self::FileExists,
        Self::MoveFile,
        Self::CopyFile,
        Self::MakeDirectory,
        Self::FindFiles,
        Self::GrepCode,
        Self::GrepContext,
        Self::FileInfo,
        Self::ZipFiles,
        Self::UnzipFiles,
        Self::OpenFile,
        Self::ShareFile,
        Self::DownloadFile,
        Self::ApplyFile,
        Self::CreateFile,
        Self::EditFile,
        Self::HttpRequest,
        Self::VisitWeb,
        Self::BrowserClick,
        Self::BrowserClose,
        Self::BrowserCloseAll,
        Self::BrowserConsoleMessages,
        Self::BrowserDrag,
        Self::BrowserEvaluate,
        Self::BrowserFileUpload,
        Self::BrowserFillForm,
        Self::BrowserHandleDialog,
        Self::BrowserHover,
        Self::BrowserNavigate,
        Self::BrowserNavigateBack,
        Self::BrowserNetworkRequests,
        Self::BrowserPressKey,
        Self::BrowserResize,
        Self::BrowserRunCode,
        Self::BrowserSelectOption,
        Self::BrowserWaitFor,
        Self::BrowserSnapshot,
        Self::BrowserTakeScreenshot,
        Self::BrowserType,
        Self::BrowserTabs,
        Self::MultipartRequest,
        Self::ManageCookies,
        Self::Sleep,
        Self::GetSystemSetting,
        Self::ModifySystemSetting,
        Self::Toast,
        Self::SendNotification,
        Self::InstallApp,
        Self::UninstallApp,
        Self::ListInstalledApps,
        Self::StartApp,
        Self::StopApp,
        Self::DeviceInfo,
        Self::GetNotifications,
        Self::GetAppUsageTime,
        Self::GetDeviceLocation,
        Self::CaptureScreenshot,
        Self::RequestBluetoothPermission,
        Self::GetBluetoothState,
        Self::RequestEnableBluetooth,
        Self::ListBluetoothBondedDevices,
        Self::ScanBluetoothDevices,
        Self::BluetoothConnect,
        Self::BluetoothListen,
        Self::BluetoothAccept,
        Self::BluetoothSend,
        Self::BluetoothRead,
        Self::BluetoothSendAndRead,
        Self::BluetoothClose,
        Self::BluetoothBleConnect,
        Self::BluetoothBleDiscoverServices,
        Self::BluetoothBleReadCharacteristic,
        Self::BluetoothBleWriteCharacteristic,
        Self::BluetoothBleWriteAndReadCharacteristic,
        Self::BluetoothBleSubscribeCharacteristic,
        Self::BluetoothBleReadNotifications,
        Self::ReadEnvironmentVariable,
        Self::WriteEnvironmentVariable,
        Self::ExecuteCliCommand,
        Self::UsePackage,
        Self::ListCoreNodes,
        Self::SwitchCore,
        Self::PackageProxy,
        Self::GetTerminalInfo,
        Self::ExecuteInTerminalSession,
        Self::ExecuteInTerminalSessionStreaming,
        Self::ExecuteHiddenTerminalCommand,
        Self::CreateTerminalSession,
        Self::CloseTerminalSession,
        Self::InputInTerminalSession,
        Self::GetTerminalSessionScreen,
        Self::MusicPlay,
        Self::MusicPause,
        Self::MusicResume,
        Self::MusicStop,
        Self::MusicSeek,
        Self::MusicSetVolume,
        Self::MusicStatus,
        Self::StartChatService,
        Self::StopChatService,
        Self::CreateNewChat,
        Self::ListChats,
        Self::FindChat,
        Self::AgentStatus,
        Self::SwitchChat,
        Self::UpdateChatTitle,
        Self::DeleteChat,
        Self::SendMessageToAi,
        Self::SendMessageToAiStreaming,
        Self::ListCharacterCards,
        Self::GetChatMessages,
        Self::QueryMemory,
        Self::GetMemoryByTitle,
        Self::CreateMemory,
        Self::UpdateMemory,
        Self::DeleteMemory,
        Self::MoveMemory,
        Self::LinkMemories,
        Self::QueryMemoryLinks,
        Self::UpdateMemoryLink,
        Self::DeleteMemoryLink,
        Self::UpdateUserPreferences,
    ];

    /// Returns the stable tool name used by JavaScript and the runtime registry.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ListFiles => "list_files",
            Self::ReadFile => "read_file",
            Self::ReadFilePart => "read_file_part",
            Self::ReadFileFull => "read_file_full",
            Self::ReadFileBinary => "read_file_binary",
            Self::WriteFile => "write_file",
            Self::WriteFileBinary => "write_file_binary",
            Self::DeleteFile => "delete_file",
            Self::FileExists => "file_exists",
            Self::MoveFile => "move_file",
            Self::CopyFile => "copy_file",
            Self::MakeDirectory => "make_directory",
            Self::FindFiles => "find_files",
            Self::GrepCode => "grep_code",
            Self::GrepContext => "grep_context",
            Self::FileInfo => "file_info",
            Self::ZipFiles => "zip_files",
            Self::UnzipFiles => "unzip_files",
            Self::OpenFile => "open_file",
            Self::ShareFile => "share_file",
            Self::DownloadFile => "download_file",
            Self::ApplyFile => "apply_file",
            Self::CreateFile => "create_file",
            Self::EditFile => "edit_file",
            Self::HttpRequest => "http_request",
            Self::VisitWeb => "visit_web",
            Self::BrowserClick => "browser_click",
            Self::BrowserClose => "browser_close",
            Self::BrowserCloseAll => "browser_close_all",
            Self::BrowserConsoleMessages => "browser_console_messages",
            Self::BrowserDrag => "browser_drag",
            Self::BrowserEvaluate => "browser_evaluate",
            Self::BrowserFileUpload => "browser_file_upload",
            Self::BrowserFillForm => "browser_fill_form",
            Self::BrowserHandleDialog => "browser_handle_dialog",
            Self::BrowserHover => "browser_hover",
            Self::BrowserNavigate => "browser_navigate",
            Self::BrowserNavigateBack => "browser_navigate_back",
            Self::BrowserNetworkRequests => "browser_network_requests",
            Self::BrowserPressKey => "browser_press_key",
            Self::BrowserResize => "browser_resize",
            Self::BrowserRunCode => "browser_run_code",
            Self::BrowserSelectOption => "browser_select_option",
            Self::BrowserWaitFor => "browser_wait_for",
            Self::BrowserSnapshot => "browser_snapshot",
            Self::BrowserTakeScreenshot => "browser_take_screenshot",
            Self::BrowserType => "browser_type",
            Self::BrowserTabs => "browser_tabs",
            Self::MultipartRequest => "multipart_request",
            Self::ManageCookies => "manage_cookies",
            Self::Sleep => "sleep",
            Self::GetSystemSetting => "get_system_setting",
            Self::ModifySystemSetting => "modify_system_setting",
            Self::Toast => "toast",
            Self::SendNotification => "send_notification",
            Self::InstallApp => "install_app",
            Self::UninstallApp => "uninstall_app",
            Self::ListInstalledApps => "list_installed_apps",
            Self::StartApp => "start_app",
            Self::StopApp => "stop_app",
            Self::DeviceInfo => "device_info",
            Self::GetNotifications => "get_notifications",
            Self::GetAppUsageTime => "get_app_usage_time",
            Self::GetDeviceLocation => "get_device_location",
            Self::CaptureScreenshot => "capture_screenshot",
            Self::RequestBluetoothPermission => "request_bluetooth_permission",
            Self::GetBluetoothState => "get_bluetooth_state",
            Self::RequestEnableBluetooth => "request_enable_bluetooth",
            Self::ListBluetoothBondedDevices => "list_bluetooth_bonded_devices",
            Self::ScanBluetoothDevices => "scan_bluetooth_devices",
            Self::BluetoothConnect => "bluetooth_connect",
            Self::BluetoothListen => "bluetooth_listen",
            Self::BluetoothAccept => "bluetooth_accept",
            Self::BluetoothSend => "bluetooth_send",
            Self::BluetoothRead => "bluetooth_read",
            Self::BluetoothSendAndRead => "bluetooth_send_and_read",
            Self::BluetoothClose => "bluetooth_close",
            Self::BluetoothBleConnect => "bluetooth_ble_connect",
            Self::BluetoothBleDiscoverServices => "bluetooth_ble_discover_services",
            Self::BluetoothBleReadCharacteristic => "bluetooth_ble_read_characteristic",
            Self::BluetoothBleWriteCharacteristic => "bluetooth_ble_write_characteristic",
            Self::BluetoothBleWriteAndReadCharacteristic => "bluetooth_ble_write_and_read_characteristic",
            Self::BluetoothBleSubscribeCharacteristic => "bluetooth_ble_subscribe_characteristic",
            Self::BluetoothBleReadNotifications => "bluetooth_ble_read_notifications",
            Self::ReadEnvironmentVariable => "read_environment_variable",
            Self::WriteEnvironmentVariable => "write_environment_variable",
            Self::ExecuteCliCommand => "execute_cli_command",
            Self::UsePackage => "use_package",
            Self::ListCoreNodes => "list_core_nodes",
            Self::SwitchCore => "switch_core",
            Self::PackageProxy => "package_proxy",
            Self::GetTerminalInfo => "get_terminal_info",
            Self::ExecuteInTerminalSession => "execute_in_terminal_session",
            Self::ExecuteInTerminalSessionStreaming => "execute_in_terminal_session_streaming",
            Self::ExecuteHiddenTerminalCommand => "execute_hidden_terminal_command",
            Self::CreateTerminalSession => "create_terminal_session",
            Self::CloseTerminalSession => "close_terminal_session",
            Self::InputInTerminalSession => "input_in_terminal_session",
            Self::GetTerminalSessionScreen => "get_terminal_session_screen",
            Self::MusicPlay => "music_play",
            Self::MusicPause => "music_pause",
            Self::MusicResume => "music_resume",
            Self::MusicStop => "music_stop",
            Self::MusicSeek => "music_seek",
            Self::MusicSetVolume => "music_set_volume",
            Self::MusicStatus => "music_status",
            Self::StartChatService => "start_chat_service",
            Self::StopChatService => "stop_chat_service",
            Self::CreateNewChat => "create_new_chat",
            Self::ListChats => "list_chats",
            Self::FindChat => "find_chat",
            Self::AgentStatus => "agent_status",
            Self::SwitchChat => "switch_chat",
            Self::UpdateChatTitle => "update_chat_title",
            Self::DeleteChat => "delete_chat",
            Self::SendMessageToAi => "send_message_to_ai",
            Self::SendMessageToAiStreaming => "send_message_to_ai_streaming",
            Self::ListCharacterCards => "list_character_cards",
            Self::GetChatMessages => "get_chat_messages",
            Self::QueryMemory => "query_memory",
            Self::GetMemoryByTitle => "get_memory_by_title",
            Self::CreateMemory => "create_memory",
            Self::UpdateMemory => "update_memory",
            Self::DeleteMemory => "delete_memory",
            Self::MoveMemory => "move_memory",
            Self::LinkMemories => "link_memories",
            Self::QueryMemoryLinks => "query_memory_links",
            Self::UpdateMemoryLink => "update_memory_link",
            Self::DeleteMemoryLink => "delete_memory_link",
            Self::UpdateUserPreferences => "update_user_preferences",
        }
    }

    /// Resolves an exact runtime name to its statically declared built-in tool.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "list_files" => Some(Self::ListFiles),
            "read_file" => Some(Self::ReadFile),
            "read_file_part" => Some(Self::ReadFilePart),
            "read_file_full" => Some(Self::ReadFileFull),
            "read_file_binary" => Some(Self::ReadFileBinary),
            "write_file" => Some(Self::WriteFile),
            "write_file_binary" => Some(Self::WriteFileBinary),
            "delete_file" => Some(Self::DeleteFile),
            "file_exists" => Some(Self::FileExists),
            "move_file" => Some(Self::MoveFile),
            "copy_file" => Some(Self::CopyFile),
            "make_directory" => Some(Self::MakeDirectory),
            "find_files" => Some(Self::FindFiles),
            "grep_code" => Some(Self::GrepCode),
            "grep_context" => Some(Self::GrepContext),
            "file_info" => Some(Self::FileInfo),
            "zip_files" => Some(Self::ZipFiles),
            "unzip_files" => Some(Self::UnzipFiles),
            "open_file" => Some(Self::OpenFile),
            "share_file" => Some(Self::ShareFile),
            "download_file" => Some(Self::DownloadFile),
            "apply_file" => Some(Self::ApplyFile),
            "create_file" => Some(Self::CreateFile),
            "edit_file" => Some(Self::EditFile),
            "http_request" => Some(Self::HttpRequest),
            "visit_web" => Some(Self::VisitWeb),
            "browser_click" => Some(Self::BrowserClick),
            "browser_close" => Some(Self::BrowserClose),
            "browser_close_all" => Some(Self::BrowserCloseAll),
            "browser_console_messages" => Some(Self::BrowserConsoleMessages),
            "browser_drag" => Some(Self::BrowserDrag),
            "browser_evaluate" => Some(Self::BrowserEvaluate),
            "browser_file_upload" => Some(Self::BrowserFileUpload),
            "browser_fill_form" => Some(Self::BrowserFillForm),
            "browser_handle_dialog" => Some(Self::BrowserHandleDialog),
            "browser_hover" => Some(Self::BrowserHover),
            "browser_navigate" => Some(Self::BrowserNavigate),
            "browser_navigate_back" => Some(Self::BrowserNavigateBack),
            "browser_network_requests" => Some(Self::BrowserNetworkRequests),
            "browser_press_key" => Some(Self::BrowserPressKey),
            "browser_resize" => Some(Self::BrowserResize),
            "browser_run_code" => Some(Self::BrowserRunCode),
            "browser_select_option" => Some(Self::BrowserSelectOption),
            "browser_wait_for" => Some(Self::BrowserWaitFor),
            "browser_snapshot" => Some(Self::BrowserSnapshot),
            "browser_take_screenshot" => Some(Self::BrowserTakeScreenshot),
            "browser_type" => Some(Self::BrowserType),
            "browser_tabs" => Some(Self::BrowserTabs),
            "multipart_request" => Some(Self::MultipartRequest),
            "manage_cookies" => Some(Self::ManageCookies),
            "sleep" => Some(Self::Sleep),
            "get_system_setting" => Some(Self::GetSystemSetting),
            "modify_system_setting" => Some(Self::ModifySystemSetting),
            "toast" => Some(Self::Toast),
            "send_notification" => Some(Self::SendNotification),
            "install_app" => Some(Self::InstallApp),
            "uninstall_app" => Some(Self::UninstallApp),
            "list_installed_apps" => Some(Self::ListInstalledApps),
            "start_app" => Some(Self::StartApp),
            "stop_app" => Some(Self::StopApp),
            "device_info" => Some(Self::DeviceInfo),
            "get_notifications" => Some(Self::GetNotifications),
            "get_app_usage_time" => Some(Self::GetAppUsageTime),
            "get_device_location" => Some(Self::GetDeviceLocation),
            "capture_screenshot" => Some(Self::CaptureScreenshot),
            "request_bluetooth_permission" => Some(Self::RequestBluetoothPermission),
            "get_bluetooth_state" => Some(Self::GetBluetoothState),
            "request_enable_bluetooth" => Some(Self::RequestEnableBluetooth),
            "list_bluetooth_bonded_devices" => Some(Self::ListBluetoothBondedDevices),
            "scan_bluetooth_devices" => Some(Self::ScanBluetoothDevices),
            "bluetooth_connect" => Some(Self::BluetoothConnect),
            "bluetooth_listen" => Some(Self::BluetoothListen),
            "bluetooth_accept" => Some(Self::BluetoothAccept),
            "bluetooth_send" => Some(Self::BluetoothSend),
            "bluetooth_read" => Some(Self::BluetoothRead),
            "bluetooth_send_and_read" => Some(Self::BluetoothSendAndRead),
            "bluetooth_close" => Some(Self::BluetoothClose),
            "bluetooth_ble_connect" => Some(Self::BluetoothBleConnect),
            "bluetooth_ble_discover_services" => Some(Self::BluetoothBleDiscoverServices),
            "bluetooth_ble_read_characteristic" => Some(Self::BluetoothBleReadCharacteristic),
            "bluetooth_ble_write_characteristic" => Some(Self::BluetoothBleWriteCharacteristic),
            "bluetooth_ble_write_and_read_characteristic" => Some(Self::BluetoothBleWriteAndReadCharacteristic),
            "bluetooth_ble_subscribe_characteristic" => Some(Self::BluetoothBleSubscribeCharacteristic),
            "bluetooth_ble_read_notifications" => Some(Self::BluetoothBleReadNotifications),
            "read_environment_variable" => Some(Self::ReadEnvironmentVariable),
            "write_environment_variable" => Some(Self::WriteEnvironmentVariable),
            "execute_cli_command" => Some(Self::ExecuteCliCommand),
            "use_package" => Some(Self::UsePackage),
            "list_core_nodes" => Some(Self::ListCoreNodes),
            "switch_core" => Some(Self::SwitchCore),
            "package_proxy" => Some(Self::PackageProxy),
            "get_terminal_info" => Some(Self::GetTerminalInfo),
            "execute_in_terminal_session" => Some(Self::ExecuteInTerminalSession),
            "execute_in_terminal_session_streaming" => Some(Self::ExecuteInTerminalSessionStreaming),
            "execute_hidden_terminal_command" => Some(Self::ExecuteHiddenTerminalCommand),
            "create_terminal_session" => Some(Self::CreateTerminalSession),
            "close_terminal_session" => Some(Self::CloseTerminalSession),
            "input_in_terminal_session" => Some(Self::InputInTerminalSession),
            "get_terminal_session_screen" => Some(Self::GetTerminalSessionScreen),
            "music_play" => Some(Self::MusicPlay),
            "music_pause" => Some(Self::MusicPause),
            "music_resume" => Some(Self::MusicResume),
            "music_stop" => Some(Self::MusicStop),
            "music_seek" => Some(Self::MusicSeek),
            "music_set_volume" => Some(Self::MusicSetVolume),
            "music_status" => Some(Self::MusicStatus),
            "start_chat_service" => Some(Self::StartChatService),
            "stop_chat_service" => Some(Self::StopChatService),
            "create_new_chat" => Some(Self::CreateNewChat),
            "list_chats" => Some(Self::ListChats),
            "find_chat" => Some(Self::FindChat),
            "agent_status" => Some(Self::AgentStatus),
            "switch_chat" => Some(Self::SwitchChat),
            "update_chat_title" => Some(Self::UpdateChatTitle),
            "delete_chat" => Some(Self::DeleteChat),
            "send_message_to_ai" => Some(Self::SendMessageToAi),
            "send_message_to_ai_streaming" => Some(Self::SendMessageToAiStreaming),
            "list_character_cards" => Some(Self::ListCharacterCards),
            "get_chat_messages" => Some(Self::GetChatMessages),
            "query_memory" => Some(Self::QueryMemory),
            "get_memory_by_title" => Some(Self::GetMemoryByTitle),
            "create_memory" => Some(Self::CreateMemory),
            "update_memory" => Some(Self::UpdateMemory),
            "delete_memory" => Some(Self::DeleteMemory),
            "move_memory" => Some(Self::MoveMemory),
            "link_memories" => Some(Self::LinkMemories),
            "query_memory_links" => Some(Self::QueryMemoryLinks),
            "update_memory_link" => Some(Self::UpdateMemoryLink),
            "delete_memory_link" => Some(Self::DeleteMemoryLink),
            "update_user_preferences" => Some(Self::UpdateUserPreferences),
            _ => None,
        }
    }

    /// Reports whether a successful runtime payload matches this tool's public contract.
    pub fn accepts_runtime_result(self, result: &ToolResultData) -> bool {
        match self {
            Self::ListFiles => matches!(result, ToolResultData::DirectoryListingData(_)),
            Self::ReadFile => matches!(result, ToolResultData::FileContentData(_)),
            Self::ReadFilePart => matches!(result, ToolResultData::FilePartContentData(_)),
            Self::ReadFileFull => matches!(result, ToolResultData::FileContentData(_)),
            Self::ReadFileBinary => matches!(result, ToolResultData::BinaryFileContentData(_)),
            Self::WriteFile => matches!(result, ToolResultData::FileOperationData(_)),
            Self::WriteFileBinary => matches!(result, ToolResultData::FileOperationData(_)),
            Self::DeleteFile => matches!(result, ToolResultData::FileOperationData(_)),
            Self::FileExists => matches!(result, ToolResultData::FileExistsData(_)),
            Self::MoveFile => matches!(result, ToolResultData::FileOperationData(_)),
            Self::CopyFile => matches!(result, ToolResultData::FileOperationData(_)),
            Self::MakeDirectory => matches!(result, ToolResultData::FileOperationData(_)),
            Self::FindFiles => matches!(result, ToolResultData::FindFilesResultData(_)),
            Self::GrepCode => matches!(result, ToolResultData::GrepResultData(_)),
            Self::GrepContext => matches!(result, ToolResultData::GrepResultData(_)),
            Self::FileInfo => matches!(result, ToolResultData::FileInfoData(_)),
            Self::ZipFiles => matches!(result, ToolResultData::FileOperationData(_)),
            Self::UnzipFiles => matches!(result, ToolResultData::FileOperationData(_)),
            Self::OpenFile => matches!(result, ToolResultData::FileOperationData(_)),
            Self::ShareFile => matches!(result, ToolResultData::FileOperationData(_)),
            Self::DownloadFile => matches!(result, ToolResultData::FileOperationData(_)),
            Self::ApplyFile => matches!(result, ToolResultData::FileApplyResultData(_)),
            Self::CreateFile => matches!(result, ToolResultData::FileApplyResultData(_)),
            Self::EditFile => matches!(result, ToolResultData::FileApplyResultData(_)),
            Self::HttpRequest => matches!(result, ToolResultData::HttpResponseData(_)),
            Self::VisitWeb => matches!(result, ToolResultData::VisitWebResultData(_)),
            Self::BrowserClick => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserClose => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserCloseAll => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserConsoleMessages => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserDrag => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserEvaluate => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserFileUpload => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserFillForm => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserHandleDialog => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserHover => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserNavigate => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserNavigateBack => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserNetworkRequests => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserPressKey => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserResize => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserRunCode => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserSelectOption => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserWaitFor => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserSnapshot => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserTakeScreenshot => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserType => matches!(result, ToolResultData::StringResultData(_)),
            Self::BrowserTabs => matches!(result, ToolResultData::StringResultData(_)),
            Self::MultipartRequest => matches!(result, ToolResultData::HttpResponseData(_)),
            Self::ManageCookies => matches!(result, ToolResultData::HttpResponseData(_)),
            Self::Sleep => matches!(result, ToolResultData::SleepResultData(_)),
            Self::GetSystemSetting => matches!(result, ToolResultData::SystemSettingData(_)),
            Self::ModifySystemSetting => matches!(result, ToolResultData::SystemSettingData(_)),
            Self::Toast => matches!(result, ToolResultData::StringResultData(_)),
            Self::SendNotification => matches!(result, ToolResultData::StringResultData(_)),
            Self::InstallApp => matches!(result, ToolResultData::AppOperationData(_)),
            Self::UninstallApp => matches!(result, ToolResultData::AppOperationData(_)),
            Self::ListInstalledApps => matches!(result, ToolResultData::AppListData(_)),
            Self::StartApp => matches!(result, ToolResultData::AppOperationData(_)),
            Self::StopApp => matches!(result, ToolResultData::AppOperationData(_)),
            Self::DeviceInfo => matches!(result, ToolResultData::DeviceInfoResultData(_)),
            Self::GetNotifications => matches!(result, ToolResultData::NotificationData(_)),
            Self::GetAppUsageTime => matches!(result, ToolResultData::AppUsageTimeResultData(_)),
            Self::GetDeviceLocation => matches!(result, ToolResultData::LocationData(_)),
            Self::CaptureScreenshot => matches!(result, ToolResultData::StringResultData(_)),
            Self::RequestBluetoothPermission => matches!(result, ToolResultData::StringResultData(_)),
            Self::GetBluetoothState => matches!(result, ToolResultData::BluetoothStateData(_)),
            Self::RequestEnableBluetooth => matches!(result, ToolResultData::StringResultData(_)),
            Self::ListBluetoothBondedDevices => matches!(result, ToolResultData::BluetoothBondedDevicesData(_)),
            Self::ScanBluetoothDevices => matches!(result, ToolResultData::BluetoothScanResultData(_)),
            Self::BluetoothConnect => matches!(result, ToolResultData::BluetoothSessionData(_)),
            Self::BluetoothListen => matches!(result, ToolResultData::BluetoothSessionData(_)),
            Self::BluetoothAccept => matches!(result, ToolResultData::BluetoothSessionData(_)),
            Self::BluetoothSend => matches!(result, ToolResultData::BluetoothTransferData(_)),
            Self::BluetoothRead => matches!(result, ToolResultData::BluetoothReadData(_)),
            Self::BluetoothSendAndRead => matches!(result, ToolResultData::BluetoothReadData(_)),
            Self::BluetoothClose => matches!(result, ToolResultData::StringResultData(_)),
            Self::BluetoothBleConnect => matches!(result, ToolResultData::BluetoothSessionData(_)),
            Self::BluetoothBleDiscoverServices => matches!(result, ToolResultData::BluetoothBleServicesData(_)),
            Self::BluetoothBleReadCharacteristic => matches!(result, ToolResultData::BluetoothReadData(_)),
            Self::BluetoothBleWriteCharacteristic => matches!(result, ToolResultData::BluetoothTransferData(_)),
            Self::BluetoothBleWriteAndReadCharacteristic => matches!(result, ToolResultData::BluetoothReadData(_)),
            Self::BluetoothBleSubscribeCharacteristic => matches!(result, ToolResultData::BluetoothTransferData(_)),
            Self::BluetoothBleReadNotifications => matches!(result, ToolResultData::BluetoothBleNotificationData(_)),
            Self::ReadEnvironmentVariable => matches!(result, ToolResultData::EnvironmentVariableReadResultData(_)),
            Self::WriteEnvironmentVariable => matches!(result, ToolResultData::EnvironmentVariableWriteResultData(_)),
            Self::ExecuteCliCommand => matches!(result, _),
            Self::UsePackage => matches!(result, ToolResultData::StringResultData(_)),
            Self::ListCoreNodes => matches!(result, ToolResultData::StringResultData(_)),
            Self::SwitchCore => matches!(result, ToolResultData::StringResultData(_)),
            Self::PackageProxy => matches!(result, _),
            Self::GetTerminalInfo => matches!(result, ToolResultData::TerminalInfoResultData(_)),
            Self::ExecuteInTerminalSession => matches!(result, ToolResultData::TerminalCommandResultData(_)),
            Self::ExecuteInTerminalSessionStreaming => matches!(result, ToolResultData::TerminalStreamEventData(_) | ToolResultData::TerminalCommandResultData(_)),
            Self::ExecuteHiddenTerminalCommand => matches!(result, ToolResultData::HiddenTerminalCommandResultData(_)),
            Self::CreateTerminalSession => matches!(result, ToolResultData::TerminalSessionCreationResultData(_)),
            Self::CloseTerminalSession => matches!(result, ToolResultData::TerminalSessionCloseResultData(_)),
            Self::InputInTerminalSession => matches!(result, ToolResultData::StringResultData(_)),
            Self::GetTerminalSessionScreen => matches!(result, ToolResultData::TerminalSessionScreenResultData(_)),
            Self::MusicPlay => matches!(result, ToolResultData::MusicPlaybackResultData(_)),
            Self::MusicPause => matches!(result, ToolResultData::MusicPlaybackResultData(_)),
            Self::MusicResume => matches!(result, ToolResultData::MusicPlaybackResultData(_)),
            Self::MusicStop => matches!(result, ToolResultData::MusicPlaybackResultData(_)),
            Self::MusicSeek => matches!(result, ToolResultData::MusicPlaybackResultData(_)),
            Self::MusicSetVolume => matches!(result, ToolResultData::MusicPlaybackResultData(_)),
            Self::MusicStatus => matches!(result, ToolResultData::MusicPlaybackResultData(_)),
            Self::StartChatService => matches!(result, ToolResultData::ChatServiceStartResultData(_)),
            Self::StopChatService => matches!(result, ToolResultData::ChatServiceStartResultData(_)),
            Self::CreateNewChat => matches!(result, ToolResultData::ChatCreationResultData(_)),
            Self::ListChats => matches!(result, ToolResultData::ChatListResultData(_)),
            Self::FindChat => matches!(result, ToolResultData::ChatFindResultData(_)),
            Self::AgentStatus => matches!(result, ToolResultData::AgentStatusResultData(_)),
            Self::SwitchChat => matches!(result, ToolResultData::ChatSwitchResultData(_)),
            Self::UpdateChatTitle => matches!(result, ToolResultData::ChatTitleUpdateResultData(_)),
            Self::DeleteChat => matches!(result, ToolResultData::ChatDeleteResultData(_)),
            Self::SendMessageToAi => matches!(result, ToolResultData::MessageSendResultData(_)),
            Self::SendMessageToAiStreaming => matches!(result, ToolResultData::MessageSendResultData(_)),
            Self::ListCharacterCards => matches!(result, ToolResultData::CharacterCardListResultData(_)),
            Self::GetChatMessages => matches!(result, ToolResultData::ChatMessagesResultData(_)),
            Self::QueryMemory => matches!(result, ToolResultData::MemoryQueryResultData(_)),
            Self::GetMemoryByTitle => matches!(result, ToolResultData::MemoryQueryResultData(_)),
            Self::CreateMemory => matches!(result, ToolResultData::StringResultData(_)),
            Self::UpdateMemory => matches!(result, ToolResultData::StringResultData(_)),
            Self::DeleteMemory => matches!(result, ToolResultData::StringResultData(_)),
            Self::MoveMemory => matches!(result, ToolResultData::StringResultData(_)),
            Self::LinkMemories => matches!(result, ToolResultData::MemoryLinkResultData(_)),
            Self::QueryMemoryLinks => matches!(result, ToolResultData::MemoryLinkQueryResultData(_)),
            Self::UpdateMemoryLink => matches!(result, ToolResultData::MemoryLinkQueryResultData(_)),
            Self::DeleteMemoryLink => matches!(result, ToolResultData::StringResultData(_)),
            Self::UpdateUserPreferences => matches!(result, ToolResultData::StringResultData(_)),
        }
    }
}

impl std::fmt::Display for BuiltinToolName {
    /// Formats the stable runtime tool name.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
formatter.write_str(self.as_str())
}
}
