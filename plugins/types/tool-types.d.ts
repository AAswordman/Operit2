/**
 * Tool name type definitions for Assistance Package Tools
 * 
 * This file defines the available tool names and maps them to their result types.
 */

import {
    DirectoryListingData, FileContentData, BinaryFileContentData, FileOperationData, FileExistsData,
    FindFilesResultData, FileInfoData,
    HttpResponseData, VisitWebResultData,
    SleepResultData, StringResultData, SystemSettingData, AppOperationData, AppListData,
    DeviceInfoResultData, NotificationData, LocationData, BluetoothStateData, BluetoothBondedDevicesData,
    BluetoothScanResultData, BluetoothSessionData, BluetoothTransferData, BluetoothReadData,
    BluetoothBleServicesData, BluetoothBleNotificationData,
    UIPageResultData, UIActionResultData, CombinedOperationResultData, AutomationExecutionResultData,
    IntentResultData, TerminalCommandResultData, HiddenTerminalCommandResultData,
    TerminalSessionCreationResultData, TerminalSessionCloseResultData, TerminalSessionScreenResultData, MusicPlaybackResultData,
    FilePartContentData, FileApplyResultData,
    ChatServiceStartResultData, ChatCreationResultData, ChatListResultData, ChatFindResultData, AgentStatusResultData,
    ChatSwitchResultData, ChatTitleUpdateResultData, ChatDeleteResultData, MessageSendResultData, MemoryQueryResultData, MemoryLinkResultData, MemoryLinkQueryResultData, GrepResultData,
    ChatMessagesResultData, CharacterCardListResultData,
    EnvironmentVariableReadResultData, EnvironmentVariableWriteResultData,
    SandboxPackageResultItem, SandboxPackagesResultData, SandboxPackageUpdateResultData,
    McpRestartLogPluginResultItem, McpRestartWithLogsResultData,
    SpeechServicesConfigResultData, SpeechServicesUpdateResultData,
    SandboxScriptExecutionResultData,
    ModelConfigsResultData, ModelConfigCreateResultData, ModelConfigUpdateResultData, ModelConfigDeleteResultData,
    FunctionModelConfigsResultData, FunctionModelConfigResultData, FunctionModelBindingResultData, ModelConfigConnectionTestResultData
} from './results';

/**
 * Maps tool names to their result data types
 */
export interface ToolResultMap {
    // File operations
    'list_files': DirectoryListingData;
    'read_file': FileContentData;
    'read_file_part': FilePartContentData;
    'read_file_full': FileContentData;
    'read_file_binary': BinaryFileContentData;

    'write_file': FileOperationData;
    'delete_file': FileOperationData;
    'file_exists': FileExistsData;
    'move_file': FileOperationData;
    'copy_file': FileOperationData;
    'make_directory': FileOperationData;
    'find_files': FindFilesResultData;
    'grep_code': GrepResultData;
    'grep_context': GrepResultData;
    'file_info': FileInfoData;
    'zip_files': FileOperationData;
    'unzip_files': FileOperationData;
    'open_file': FileOperationData;
    'share_file': FileOperationData;
    'download_file': FileOperationData;
    'apply_file': FileApplyResultData;
    'create_file': FileApplyResultData;
    'edit_file': FileApplyResultData;

    // Network operations
    'http_request': HttpResponseData;
    'visit_web': VisitWebResultData;
    'browser_click': StringResultData;
    'browser_close': StringResultData;
    'browser_close_all': StringResultData;
    'browser_console_messages': StringResultData;
    'browser_drag': StringResultData;
    'browser_evaluate': StringResultData;
    'browser_file_upload': StringResultData;
    'browser_fill_form': StringResultData;
    'browser_handle_dialog': StringResultData;
    'browser_hover': StringResultData;
    'browser_navigate': StringResultData;
    'browser_navigate_back': StringResultData;
    'browser_network_requests': StringResultData;
    'browser_press_key': StringResultData;
    'browser_resize': StringResultData;
    'browser_run_code': StringResultData;
    'browser_select_option': StringResultData;
    'browser_wait_for': StringResultData;
    'browser_snapshot': StringResultData;
    'browser_take_screenshot': StringResultData;
    'browser_type': StringResultData;
    'browser_tabs': StringResultData;
    'multipart_request': HttpResponseData;
    'manage_cookies': HttpResponseData;

    // System operations
    'sleep': SleepResultData;
    'get_system_setting': SystemSettingData;
    'modify_system_setting': SystemSettingData;
    'toast': StringResultData;
    'send_notification': StringResultData;
    'install_app': AppOperationData;
    'uninstall_app': AppOperationData;
    'list_installed_apps': AppListData;
    'start_app': AppOperationData;
    'stop_app': AppOperationData;
    'device_info': DeviceInfoResultData;
    'get_notifications': NotificationData;
    'get_device_location': LocationData;
    'request_bluetooth_permission': StringResultData;
    'get_bluetooth_state': BluetoothStateData;
    'request_enable_bluetooth': StringResultData;
    'list_bluetooth_bonded_devices': BluetoothBondedDevicesData;
    'scan_bluetooth_devices': BluetoothScanResultData;
    'bluetooth_connect': BluetoothSessionData;
    'bluetooth_listen': BluetoothSessionData;
    'bluetooth_accept': BluetoothSessionData;
    'bluetooth_send': BluetoothTransferData;
    'bluetooth_read': BluetoothReadData;
    'bluetooth_send_and_read': BluetoothReadData;
    'bluetooth_close': StringResultData;
    'bluetooth_ble_connect': BluetoothSessionData;
    'bluetooth_ble_discover_services': BluetoothBleServicesData;
    'bluetooth_ble_read_characteristic': BluetoothReadData;
    'bluetooth_ble_write_characteristic': BluetoothTransferData;
    'bluetooth_ble_write_and_read_characteristic': BluetoothReadData;
    'bluetooth_ble_subscribe_characteristic': BluetoothTransferData;
    'bluetooth_ble_read_notifications': BluetoothBleNotificationData;
    'read_environment_variable': EnvironmentVariableReadResultData;
    'write_environment_variable': EnvironmentVariableWriteResultData;
    'execute_cli_command': unknown;

    // UI operations
    'get_page_info': UIPageResultData;
    'click_element': UIActionResultData;
    'tap': UIActionResultData;
    'set_input_text': UIActionResultData;
    'press_key': UIActionResultData;
    'swipe': UIActionResultData;
    'combined_operation': CombinedOperationResultData;
    'run_ui_subagent': AutomationExecutionResultData;

    // Package operations
    'use_package': string;
    'query_memory': MemoryQueryResultData;

    // Intent operations
    'execute_intent': IntentResultData;
    'send_broadcast': IntentResultData;

    // Terminal operations
    'execute_in_terminal_session': TerminalCommandResultData;
    'execute_in_terminal_session_streaming': TerminalCommandResultData;
    'execute_hidden_terminal_command': HiddenTerminalCommandResultData;
    'create_terminal_session': TerminalSessionCreationResultData;
    'close_terminal_session': TerminalSessionCloseResultData;
    'input_in_terminal_session': StringResultData;
    'get_terminal_session_screen': TerminalSessionScreenResultData;

    // Music playback operations
    'music_play': MusicPlaybackResultData;
    'music_pause': MusicPlaybackResultData;
    'music_resume': MusicPlaybackResultData;
    'music_stop': MusicPlaybackResultData;
    'music_seek': MusicPlaybackResultData;
    'music_set_volume': MusicPlaybackResultData;
    'music_status': MusicPlaybackResultData;

    // Chat Manager operations
    'start_chat_service': ChatServiceStartResultData;
    'stop_chat_service': ChatServiceStartResultData;
    'create_new_chat': ChatCreationResultData;
    'list_chats': ChatListResultData;
    'find_chat': ChatFindResultData;
    'agent_status': AgentStatusResultData;
    'switch_chat': ChatSwitchResultData;
    'update_chat_title': ChatTitleUpdateResultData;
    'delete_chat': ChatDeleteResultData;

    'send_message_to_ai': MessageSendResultData;
    'send_message_to_ai_streaming': MessageSendResultData;
    'list_character_cards': CharacterCardListResultData;
    'get_chat_messages': ChatMessagesResultData;

    // Memory operations
    'link_memories': MemoryLinkResultData;
    'query_memory_links': MemoryLinkQueryResultData;
    'update_user_preferences': StringResultData;
} 
