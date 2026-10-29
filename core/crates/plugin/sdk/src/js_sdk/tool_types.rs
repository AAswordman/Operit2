//! Canonical tool-name to result-type mapping for generated TypeScript declarations.

use super::results::*;

/// Maps every built-in tool name to its concrete public result type.
#[operit_plugin_sdk_macros::tool_display]
pub struct ToolResultMap {
    pub edge_execute: EdgePortResultData,
    pub io_execute: EdgePortResultData,
    #[display(action = "list", icon = "folder", target = "path", result = "directory")]
    pub list_files: DirectoryListingData,
    #[display(action = "read", icon = "file", target = "path")]
    pub read_file: FileContentData,
    #[display(action = "read", icon = "file", target = "path")]
    pub read_file_part: FilePartContentData,
    #[display(action = "read", icon = "file", target = "path")]
    pub read_file_full: FileContentData,
    #[display(action = "read", icon = "file", target = "path")]
    pub read_file_binary: BinaryFileContentData,
    #[display(action = "write", icon = "edit", target = "path", result = "fileDiff")]
    pub write_file: FileApplyResultData,
    #[display(action = "write", icon = "edit", target = "path")]
    pub write_file_binary: FileOperationData,
    #[display(action = "delete", icon = "delete", target = "path")]
    pub delete_file: FileOperationData,
    #[display(action = "info", icon = "file", target = "path")]
    pub file_exists: FileExistsData,
    #[display(action = "move", icon = "file", target = "source")]
    pub move_file: FileOperationData,
    #[display(action = "copy", icon = "file", target = "source")]
    pub copy_file: FileOperationData,
    #[display(action = "create", icon = "folder", target = "path")]
    pub make_directory: FileOperationData,
    #[display(action = "search", icon = "search", target = "pattern")]
    pub find_files: FindFilesResultData,
    #[display(action = "search", icon = "search", target = "pattern")]
    pub grep_code: GrepResultData,
    #[display(action = "search", icon = "search", target = "pattern")]
    pub grep_context: GrepResultData,
    #[display(action = "info", icon = "file", target = "path")]
    pub file_info: FileInfoData,
    #[display(action = "archive", icon = "folder", target = "destination")]
    pub zip_files: FileOperationData,
    #[display(action = "extract", icon = "folder", target = "source")]
    pub unzip_files: FileOperationData,
    #[display(action = "open", icon = "file", target = "path")]
    pub open_file: FileOperationData,
    #[display(action = "share", icon = "file", target = "path")]
    pub share_file: FileOperationData,
    #[display(action = "download", icon = "download", target = "url")]
    pub download_file: FileOperationData,
    #[display(action = "edit", icon = "edit", target = "path", result = "fileDiff")]
    pub apply_file: FileApplyResultData,
    #[display(action = "create", icon = "file", target = "path", result = "fileDiff")]
    pub create_file: FileApplyResultData,
    #[display(action = "edit", icon = "edit", target = "path", result = "fileDiff")]
    pub edit_file: FileApplyResultData,
    pub storage_request: StorageRequestResultData,
    pub web_socket: WebSocketResultData,
    #[display(action = "request", icon = "web", target = "url")]
    pub http_request: HttpResponseData,
    #[display(action = "visit", icon = "web", target = "url")]
    pub visit_web: VisitWebResultData,
    pub browser_click: String,
    pub browser_close: String,
    pub browser_close_all: String,
    pub browser_console_messages: String,
    pub browser_drag: String,
    pub browser_evaluate: String,
    pub browser_file_upload: String,
    pub browser_fill_form: String,
    pub browser_handle_dialog: String,
    pub browser_hover: String,
    #[display(action = "visit", icon = "web", target = "url")]
    pub browser_navigate: String,
    pub browser_navigate_back: String,
    pub browser_network_requests: String,
    pub browser_press_key: String,
    pub browser_resize: String,
    pub browser_run_code: String,
    pub browser_select_option: String,
    pub browser_wait_for: String,
    pub browser_snapshot: String,
    pub browser_take_screenshot: String,
    pub browser_type: String,
    pub browser_tabs: String,
    #[display(action = "request", icon = "web", target = "url")]
    pub multipart_request: HttpResponseData,
    pub manage_cookies: HttpResponseData,
    #[display(action = "wait", icon = "time", target = "duration_ms")]
    pub sleep: SleepResultData,
    pub get_system_setting: SystemSettingData,
    pub modify_system_setting: SystemSettingData,
    #[display(action = "notify", icon = "notification", target = "message")]
    pub toast: String,
    #[display(action = "write", icon = "clipboard", target = "text")]
    pub set_clipboard_text: String,
    #[display(action = "notify", icon = "notification", target = "title")]
    pub send_notification: String,
    pub install_app: AppOperationData,
    pub uninstall_app: AppOperationData,
    pub list_installed_apps: AppListData,
    pub start_app: AppOperationData,
    pub stop_app: AppOperationData,
    pub device_info: DeviceInfoResultData,
    pub get_notifications: NotificationData,
    pub get_app_usage_time: AppUsageTimeResultData,
    pub get_device_location: LocationData,
    pub capture_screenshot: String,
    pub request_bluetooth_permission: String,
    pub get_bluetooth_state: BluetoothStateData,
    pub request_enable_bluetooth: String,
    pub list_bluetooth_bonded_devices: BluetoothBondedDevicesData,
    pub scan_bluetooth_devices: BluetoothScanResultData,
    pub bluetooth_connect: BluetoothSessionData,
    pub bluetooth_listen: BluetoothSessionData,
    pub bluetooth_accept: BluetoothSessionData,
    pub bluetooth_send: BluetoothTransferData,
    pub bluetooth_read: BluetoothReadData,
    pub bluetooth_send_and_read: BluetoothReadData,
    pub bluetooth_close: String,
    pub bluetooth_ble_connect: BluetoothSessionData,
    pub bluetooth_ble_discover_services: BluetoothBleServicesData,
    pub bluetooth_ble_read_characteristic: BluetoothReadData,
    pub bluetooth_ble_write_characteristic: BluetoothTransferData,
    pub bluetooth_ble_write_and_read_characteristic: BluetoothReadData,
    pub bluetooth_ble_subscribe_characteristic: BluetoothTransferData,
    pub bluetooth_ble_read_notifications: BluetoothBleNotificationData,
    pub read_environment_variable: EnvironmentVariableReadResultData,
    pub write_environment_variable: EnvironmentVariableWriteResultData,
    #[display(action = "execute", icon = "terminal", target = "command")]
    pub execute_cli_command: serde_json::Value,
    pub models_list_summaries: ModelsListSummariesResultData,
    #[display(action = "speak", icon = "audio", target = "text")]
    pub tts_synthesize: TtsSynthesizeResultData,
    pub tts_speak: TtsSpeakResultData,
    #[display(action = "transcribe", icon = "audio", target = "path")]
    pub stt_transcribe: SttTranscribeResultData,
    pub tts_list_configs: TtsListConfigsResultData,
    pub tts_get_current_config_id: TtsGetCurrentConfigIdResultData,
    pub tts_set_current_config_id: TtsSetCurrentConfigIdResultData,
    pub media_register: MediaRegistrationResultData,
    pub media_register_from_path: MediaRegistrationResultData,
    pub theme_list_configs: ThemeListConfigsResultData,
    pub theme_apply_config: ThemeApplyConfigResultData,
    #[display(action = "activate", icon = "package", target = "package_name")]
    pub use_package: String,
    pub catalog_read_tool_sources: CatalogReadToolSourcesResultData,
    pub list_core_nodes: String,
    pub switch_core: String,
    pub package_proxy: ToolResultData,
    pub get_terminal_info: TerminalInfoResultData,
    #[display(action = "execute", icon = "terminal", target = "command")]
    pub execute_in_terminal_session: TerminalCommandResultData,
    #[display(action = "execute", icon = "terminal", target = "command")]
    pub execute_in_terminal_session_streaming: TerminalCommandResultData,
    pub terminal_observe: TerminalObservationResultData,
    pub execute_hidden_terminal_command: HiddenTerminalCommandResultData,
    #[display(action = "create", icon = "terminal", target = "session_name")]
    pub create_terminal_session: TerminalSessionCreationResultData,
    #[display(action = "close", icon = "terminal", target = "session_id")]
    pub close_terminal_session: TerminalSessionCloseResultData,
    #[display(action = "input", icon = "terminal", target = "input")]
    pub input_in_terminal_session: String,
    #[display(action = "read", icon = "terminal", target = "session_id")]
    pub get_terminal_session_screen: TerminalSessionScreenResultData,
    #[display(action = "play", icon = "audio", target = "path")]
    pub music_play: MusicPlaybackResultData,
    #[display(action = "pause", icon = "audio")]
    pub music_pause: MusicPlaybackResultData,
    pub music_resume: MusicPlaybackResultData,
    #[display(action = "stop", icon = "audio")]
    pub music_stop: MusicPlaybackResultData,
    pub music_seek: MusicPlaybackResultData,
    pub music_set_volume: MusicPlaybackResultData,
    pub music_status: MusicPlaybackResultData,
    pub start_chat_service: ChatServiceStartResultData,
    pub stop_chat_service: ChatServiceStartResultData,
    pub create_new_chat: ChatCreationResultData,
    pub list_chats: ChatListResultData,
    pub find_chat: ChatFindResultData,
    pub agent_status: AgentStatusResultData,
    pub switch_chat: ChatSwitchResultData,
    pub update_chat_title: ChatTitleUpdateResultData,
    pub update_chat_group: String,
    pub update_chat_pinned: String,
    pub update_chat_locked: String,
    pub reorder_chats: String,
    pub delete_chat: ChatDeleteResultData,
    pub send_message_to_ai: MessageSendResultData,
    pub send_message_to_ai_streaming: MessageSendResultData,
    pub chat_send_message: ChatSendMessageResultData,
    pub chat_observe: ChatObservationResultData,
    pub chat_cancel: ChatCancelResultData,
    pub chat_read_extension: ChatReadExtensionResultData,
    pub chat_write_extension: ChatWriteExtensionResultData,
    pub chat_delete_extension: ChatDeleteExtensionResultData,
    pub call_chat_model: ChatCallResultData,
    pub get_chat_messages: ChatMessagesResultData,
    pub get_chat_messages_range: ChatMessagesResultData,
}

include!(concat!(env!("OUT_DIR"), "/builtin_tool_names.rs"));

#[cfg(test)]
mod tests {
    use super::super::results::{
        stringResultData, FileApplyResultData, FileOperationData, ToolResultData,
    };
    use super::BuiltinToolName;

    /// Builds a representative file-apply result payload.
    fn file_apply_result() -> ToolResultData {
        ToolResultData::FileApplyResultData(FileApplyResultData {
            operation: FileOperationData {
                operation: "create".to_string(),
                path: "/workspace/example.txt".to_string(),
                successful: true,
                details: "created".to_string(),
            },
            aiDiffInstructions: String::new(),
            diffContent: Some("--- old\n+++ new".to_string()),
        })
    }

    #[test]
    /// Ensures file wrapper tools accept the delegated apply-file payload.
    fn create_and_edit_file_accept_file_apply_results() {
        let result = file_apply_result();

        assert!(BuiltinToolName::ApplyFile.accepts_runtime_result(&result));
        assert!(BuiltinToolName::CreateFile.accepts_runtime_result(&result));
        assert!(BuiltinToolName::EditFile.accepts_runtime_result(&result));
        assert!(BuiltinToolName::WriteFile.accepts_runtime_result(&result));
    }

    #[test]
    /// Keeps plugin-owned role and memory tools out of the Core built-in catalog.
    fn plugin_owned_tools_are_not_builtin_catalog_entries() {
        for name in [
            "list_character_cards", "get_memory_owner_key", "query_memory",
            "get_memory_by_title", "create_memory", "update_memory", "delete_memory",
            "move_memory", "update_user_preferences", "link_memories",
            "query_memory_links", "update_memory_link", "delete_memory_link",
        ] {
            assert_eq!(BuiltinToolName::from_name(name), None, "{name} belongs to a plugin");
        }
    }

    #[test]
    /// Ensures CoreNode discovery keeps its public string result contract.
    fn list_core_nodes_accepts_string_results() {
        let result =
            stringResultData(r#"{"currentNodeId":"node-a","nodeIds":["node-a","node-b"]}"#);

        assert!(BuiltinToolName::ListCoreNodes.accepts_runtime_result(&result));
    }
    /// Every new tool uses its own enum variant and serializes the actual tool fields.
    #[test]
    fn domain_results_use_dedicated_tool_result_variants() {
        use super::*;
        use serde_json::json;
        let results = [
            (BuiltinToolName::ModelsListSummaries, ToolResultData::ModelsListSummariesResultData(ModelsListSummariesResultData { models: vec![] }), "models"),
            (BuiltinToolName::TtsListConfigs, ToolResultData::TtsListConfigsResultData(TtsListConfigsResultData { configs: vec![] }), "configs"),
            (BuiltinToolName::TtsGetCurrentConfigId, ToolResultData::TtsGetCurrentConfigIdResultData(TtsGetCurrentConfigIdResultData { configId: "tts".into() }), "configId"),
            (BuiltinToolName::TtsSetCurrentConfigId, ToolResultData::TtsSetCurrentConfigIdResultData(TtsSetCurrentConfigIdResultData { configId: "tts".into() }), "configId"),
            (BuiltinToolName::ThemeListConfigs, ToolResultData::ThemeListConfigsResultData(ThemeListConfigsResultData { configs: vec![] }), "configs"),
            (BuiltinToolName::ThemeApplyConfig, ToolResultData::ThemeApplyConfigResultData(ThemeApplyConfigResultData { id: "theme".into(), name: "Theme".into(), snapshot: Default::default(), createdAt: 1, updatedAt: 2 }), "id"),
            (BuiltinToolName::ChatCancel, ToolResultData::ChatCancelResultData(ChatCancelResultData { chatId: "chat".into(), cancelRequested: true }), "chatId"),
            (BuiltinToolName::ChatReadExtension, ToolResultData::ChatReadExtensionResultData(ChatReadExtensionResultData { extension: JsNullable::Null }), "extension"),
            (BuiltinToolName::ChatWriteExtension, ToolResultData::ChatWriteExtensionResultData(ChatWriteExtensionResultData { extension: Default::default() }), "extension"),
            (BuiltinToolName::ChatDeleteExtension, ToolResultData::ChatDeleteExtensionResultData(ChatDeleteExtensionResultData { deleted: false }), "deleted"),
            (BuiltinToolName::CatalogReadToolSources, ToolResultData::CatalogReadToolSourcesResultData(CatalogReadToolSourcesResultData { builtinTools: vec![], packages: vec![], skills: vec![], mcpServers: vec![] }), "builtinTools"),
            (BuiltinToolName::ChatObserve, ToolResultData::ChatObservationResultData(ChatObservationResultData::Open { handle: "observation".into() }), "handle"),
            (BuiltinToolName::StorageRequest, ToolResultData::StorageRequestResultData(StorageRequestResultData::Open { handle: "database".into(), path: "runtime/data.db".into() }), "handle"),
        ];
        for (name, result, field) in results {
            assert!(name.accepts_runtime_result(&result), "{name}");
            let value = serde_json::to_value(&result).unwrap();
            assert!(value[field] != json!(null) || field == "extension");
            assert!(value["__type"].as_str().unwrap().ends_with("ResultData"));
            let decoded: ToolResultData = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(decoded).unwrap(), value);
            assert!(!result.toString().is_empty());
        }
        let wrong = ToolResultData::StringResultData(StringResultData { value: "tts".into() });
        assert!(!BuiltinToolName::TtsGetCurrentConfigId.accepts_runtime_result(&wrong));
    }

    /// A Chat send keeps committed identities and optional values without a receipt wrapper.
    #[test]
    fn chat_send_tool_result_preserves_the_originating_receipt() {
        use super::*;
        use serde_json::json;
        let receipt = MessageSendResultData {
            chatId: "chat".into(), message: "message".into(), aiResponse: JsOptional::Undefined,
            receivedAt: JsOptional::Undefined, sentAt: 123,
            outcome: MessageSendOutcome::Committed { status: MessageSendStatus::Completed, userMessageTimestamp: Some(100), assistant: Some(ChatCommittedMessage { messageTimestamp: 101, variantIndex: 2 }) },
        };
        let expected = serde_json::to_value(&receipt).unwrap();
        let result = ToolResultData::ChatSendMessageResultData(receipt.into());
        assert!(BuiltinToolName::ChatSendMessage.accepts_runtime_result(&result));
        let mut value = serde_json::to_value(&result).unwrap();
        assert_eq!(value.as_object_mut().unwrap().remove("__type"), Some(json!("ChatSendMessageResultData")));
        assert_eq!(value, expected);
        assert!(value.get("receipt").is_none());
        assert!(value.get("aiResponse").is_none());
    }

    /// Operation-specific results keep null distinct from missing and never use arbitrary JSON payloads.
    #[test]
    fn observation_and_storage_results_preserve_operation_fields() {
        use super::*;
        use super::super::storage::StorageResponse;
        use serde_json::json;
        let next = ToolResultData::ChatObservationResultData(ChatObservationResultData::Next { event: JsNullable::Null });
        assert_eq!(serde_json::to_value(next).unwrap(), json!({"__type":"ChatObservationResultData","op":"next","event":null}));
        let close = ToolResultData::ChatObservationResultData(ChatObservationResultData::Close);
        assert_eq!(serde_json::to_value(close).unwrap(), json!({"__type":"ChatObservationResultData","op":"close"}));
        let result = StorageRequestResultData::from(StorageResponse::Get(None));
        assert_eq!(serde_json::to_value(result).unwrap(), json!({"op":"get","entry":null}));
    }

}
