/* METADATA
{
    "name": "system_tools",
    "display_name": { "zh": "系统工具", "en": "System Tools" },
    "description": {
        "zh": "提供系统设置、应用管理、通知、位置、设备信息和蓝牙操作。",
        "en": "System settings, app management, notifications, location, device information, and Bluetooth operations."
    },
    "enabledByDefault": true,
    "category": "System",
    "tools": [
        { "name": "get_system_setting", "display": {"label": {"zh": "读取系统设置", "en": "Read system setting"}, "targetParameter": "setting"}, "description": { "zh": "获取系统设置。", "en": "Get a system setting." }, "parameters": [
            { "name": "setting", "description": { "zh": "设置名称。", "en": "Setting name." }, "type": "string", "required": true },
            { "name": "namespace", "description": { "zh": "命名空间：system、secure 或 global。", "en": "Namespace: system, secure, or global." }, "type": "string", "required": false }
        ] },
        { "name": "modify_system_setting", "display": {"label": {"zh": "修改系统设置", "en": "Change system setting"}, "icon": "edit", "targetParameter": "setting"}, "description": { "zh": "修改系统设置。", "en": "Modify a system setting." }, "parameters": [
            { "name": "setting", "description": { "zh": "设置名称。", "en": "Setting name." }, "type": "string", "required": true },
            { "name": "value", "description": { "zh": "设置值。", "en": "Setting value." }, "type": "string", "required": true },
            { "name": "namespace", "description": { "zh": "命名空间。", "en": "Setting namespace." }, "type": "string", "required": false }
        ] },
        { "name": "install_app", "display": {"label": {"zh": "安装应用", "en": "Install app"}, "icon": "package", "targetParameter": "path"}, "description": { "zh": "安装 APK。", "en": "Install an APK." }, "parameters": [
            { "name": "path", "description": { "zh": "APK 文件路径。", "en": "APK file path." }, "type": "string", "required": true }
        ] },
        { "name": "uninstall_app", "display": {"label": {"zh": "卸载应用", "en": "Uninstall app"}, "icon": "delete", "targetParameter": "package_name"}, "description": { "zh": "卸载应用。", "en": "Uninstall an app." }, "parameters": [
            { "name": "package_name", "description": { "zh": "应用包名。", "en": "Application package name." }, "type": "string", "required": true },
            { "name": "keep_data", "description": { "zh": "保留应用数据。", "en": "Keep application data." }, "type": "boolean", "required": false }
        ] },
        { "name": "list_installed_apps", "display": {"label": {"zh": "查看已安装应用", "en": "List installed apps"}, "icon": "package"}, "description": { "zh": "列出已安装应用。", "en": "List installed apps." }, "parameters": [
            { "name": "include_system_apps", "description": { "zh": "包含系统应用。", "en": "Include system apps." }, "type": "boolean", "required": false }
        ] },
        { "name": "start_app", "display": {"label": {"zh": "启动应用", "en": "Start app"}, "icon": "package", "targetParameter": "package_name"}, "description": { "zh": "启动应用。", "en": "Start an app." }, "parameters": [
            { "name": "package_name", "description": { "zh": "应用包名。", "en": "Application package name." }, "type": "string", "required": true },
            { "name": "activity", "description": { "zh": "可选 Activity。", "en": "Optional activity." }, "type": "string", "required": false }
        ] },
        { "name": "stop_app", "display": {"label": {"zh": "停止应用", "en": "Stop app"}, "icon": "package", "targetParameter": "package_name"}, "description": { "zh": "停止应用。", "en": "Stop an app." }, "parameters": [
            { "name": "package_name", "description": { "zh": "应用包名。", "en": "Application package name." }, "type": "string", "required": true }
        ] },
        { "name": "get_notifications", "display": {"label": {"zh": "读取通知", "en": "Read notifications"}, "icon": "notification"}, "description": { "zh": "获取设备通知。", "en": "Get device notifications." }, "parameters": [
            { "name": "limit", "description": { "zh": "最大条数。", "en": "Maximum number of entries." }, "type": "number", "required": false },
            { "name": "include_ongoing", "description": { "zh": "包含常驻通知。", "en": "Include ongoing notifications." }, "type": "boolean", "required": false }
        ] },
        { "name": "get_app_usage_time", "display": {"label": {"zh": "查看应用使用时长", "en": "Read app usage"}, "icon": "time", "targetParameter": "package_name"}, "description": { "zh": "获取应用前台使用时长。", "en": "Get application foreground usage time." }, "parameters": [
            { "name": "package_name", "description": { "zh": "应用包名。", "en": "Application package name." }, "type": "string", "required": false },
            { "name": "since_hours", "description": { "zh": "统计时间范围，单位小时。", "en": "Lookback period in hours." }, "type": "number", "required": false },
            { "name": "limit", "description": { "zh": "最大返回条数。", "en": "Maximum number of entries." }, "type": "number", "required": false },
            { "name": "include_system_apps", "description": { "zh": "包含系统应用。", "en": "Include system apps." }, "type": "boolean", "required": false }
        ] },
        { "name": "get_device_location", "display": {"label": {"zh": "获取设备位置", "en": "Get device location"}}, "description": { "zh": "获取设备位置。", "en": "Get device location." }, "parameters": [
            { "name": "high_accuracy", "description": { "zh": "使用高精度模式。", "en": "Use high accuracy mode." }, "type": "boolean", "required": false },
            { "name": "timeout", "description": { "zh": "超时时间，单位秒。", "en": "Timeout in seconds." }, "type": "number", "required": false }
        ] },
        { "name": "request_bluetooth_permission", "display": {"label": {"zh": "申请蓝牙权限", "en": "Request Bluetooth permission"}}, "description": { "zh": "请求蓝牙权限。", "en": "Request Bluetooth permission." }, "parameters": [] },
        { "name": "get_bluetooth_state", "display": {"label": {"zh": "读取蓝牙状态", "en": "Read Bluetooth state"}}, "description": { "zh": "获取蓝牙状态。", "en": "Get Bluetooth state." }, "parameters": [] },
        { "name": "request_enable_bluetooth", "display": {"label": {"zh": "开启蓝牙", "en": "Enable Bluetooth"}}, "description": { "zh": "请求开启蓝牙。", "en": "Request Bluetooth to be enabled." }, "parameters": [] },
        { "name": "list_bluetooth_bonded_devices", "display": {"label": {"zh": "查看已配对蓝牙设备", "en": "List paired Bluetooth devices"}}, "description": { "zh": "列出已配对蓝牙设备。", "en": "List bonded Bluetooth devices." }, "parameters": [] },
        { "name": "scan_bluetooth_devices", "display": {"label": {"zh": "扫描蓝牙设备", "en": "Scan Bluetooth devices"}, "icon": "search"}, "description": { "zh": "扫描蓝牙设备。", "en": "Scan Bluetooth devices." }, "parameters": [
            { "name": "duration_ms", "description": { "zh": "扫描时长毫秒。", "en": "Scan duration in milliseconds." }, "type": "number", "required": false },
            { "name": "include_ble", "description": { "zh": "包含 BLE。", "en": "Include BLE." }, "type": "boolean", "required": false }
        ] },
        { "name": "bluetooth_connect", "display": {"label": {"zh": "连接蓝牙设备", "en": "Connect Bluetooth device"}, "targetParameter": "address"}, "description": { "zh": "连接经典蓝牙设备。", "en": "Connect to a classic Bluetooth device." }, "parameters": [
            { "name": "address", "description": { "zh": "蓝牙地址。", "en": "Bluetooth address." }, "type": "string", "required": true },
            { "name": "uuid", "description": { "zh": "RFCOMM UUID。", "en": "RFCOMM UUID." }, "type": "string", "required": false }
        ] },
        { "name": "bluetooth_listen", "display": {"label": {"zh": "监听蓝牙连接", "en": "Listen for Bluetooth connections"}, "targetParameter": "name"}, "description": { "zh": "监听经典蓝牙连接。", "en": "Listen for a classic Bluetooth connection." }, "parameters": [
            { "name": "name", "description": { "zh": "服务名。", "en": "Service name." }, "type": "string", "required": false },
            { "name": "uuid", "description": { "zh": "RFCOMM UUID。", "en": "RFCOMM UUID." }, "type": "string", "required": false }
        ] },
        { "name": "bluetooth_accept", "display": {"label": {"zh": "接受蓝牙连接", "en": "Accept Bluetooth connection"}, "targetParameter": "listener_session_id"}, "description": { "zh": "接受蓝牙连接。", "en": "Accept a Bluetooth connection." }, "parameters": [
            { "name": "listener_session_id", "description": { "zh": "监听会话 ID。", "en": "Listener session ID." }, "type": "string", "required": true },
            { "name": "timeout_ms", "description": { "zh": "等待毫秒数。", "en": "Wait time in milliseconds." }, "type": "number", "required": false }
        ] },
        { "name": "bluetooth_send", "display": {"label": {"zh": "发送蓝牙数据", "en": "Send Bluetooth data"}, "targetParameter": "session_id"}, "description": { "zh": "发送蓝牙数据。", "en": "Send Bluetooth data." }, "parameters": [
            { "name": "session_id", "description": { "zh": "会话 ID。", "en": "Session ID." }, "type": "string", "required": true },
            { "name": "text", "description": { "zh": "UTF-8 文本。", "en": "UTF-8 text." }, "type": "string", "required": false },
            { "name": "data_base64", "description": { "zh": "Base64 字节。", "en": "Base64 bytes." }, "type": "string", "required": false }
        ] },
        { "name": "bluetooth_read", "display": {"label": {"zh": "读取蓝牙数据", "en": "Read Bluetooth data"}, "targetParameter": "session_id"}, "description": { "zh": "读取蓝牙数据。", "en": "Read Bluetooth data." }, "parameters": [
            { "name": "session_id", "description": { "zh": "会话 ID。", "en": "Session ID." }, "type": "string", "required": true },
            { "name": "max_bytes", "description": { "zh": "最大字节数。", "en": "Maximum bytes." }, "type": "number", "required": false },
            { "name": "timeout_ms", "description": { "zh": "等待毫秒数。", "en": "Wait time in milliseconds." }, "type": "number", "required": false }
        ] },
        { "name": "bluetooth_send_and_read", "display": {"label": {"zh": "收发蓝牙数据", "en": "Exchange Bluetooth data"}, "targetParameter": "session_id"}, "description": { "zh": "发送并读取蓝牙数据。", "en": "Send and read Bluetooth data." }, "parameters": [
            { "name": "session_id", "description": { "zh": "会话 ID。", "en": "Session ID." }, "type": "string", "required": true },
            { "name": "text", "description": { "zh": "UTF-8 文本。", "en": "UTF-8 text." }, "type": "string", "required": false },
            { "name": "data_base64", "description": { "zh": "Base64 字节。", "en": "Base64 bytes." }, "type": "string", "required": false },
            { "name": "max_bytes", "description": { "zh": "最大字节数。", "en": "Maximum bytes." }, "type": "number", "required": false },
            { "name": "timeout_ms", "description": { "zh": "等待毫秒数。", "en": "Wait time in milliseconds." }, "type": "number", "required": false }
        ] },
        { "name": "bluetooth_close", "display": {"label": {"zh": "关闭蓝牙连接", "en": "Close Bluetooth connection"}, "targetParameter": "session_id"}, "description": { "zh": "关闭蓝牙会话。", "en": "Close a Bluetooth session." }, "parameters": [
            { "name": "session_id", "description": { "zh": "会话 ID。", "en": "Session ID." }, "type": "string", "required": true }
        ] },
        { "name": "bluetooth_ble_connect", "display": {"label": {"zh": "连接 BLE 设备", "en": "Connect BLE device"}, "targetParameter": "address"}, "description": { "zh": "连接 BLE 设备。", "en": "Connect to a BLE device." }, "parameters": [
            { "name": "address", "description": { "zh": "蓝牙地址。", "en": "Bluetooth address." }, "type": "string", "required": true },
            { "name": "auto_connect", "description": { "zh": "使用自动连接。", "en": "Use auto-connect." }, "type": "boolean", "required": false }
        ] },
        { "name": "bluetooth_ble_discover_services", "display": {"label": {"zh": "发现 BLE 服务", "en": "Discover BLE services"}, "icon": "search", "targetParameter": "session_id"}, "description": { "zh": "发现 BLE 服务。", "en": "Discover BLE services." }, "parameters": [
            { "name": "session_id", "description": { "zh": "BLE 会话 ID。", "en": "BLE session ID." }, "type": "string", "required": true },
            { "name": "timeout_ms", "description": { "zh": "等待毫秒数。", "en": "Wait time in milliseconds." }, "type": "number", "required": false }
        ] },
        { "name": "bluetooth_ble_read_characteristic", "display": {"label": {"zh": "读取 BLE 特征", "en": "Read BLE characteristic"}, "targetParameter": "characteristic_uuid"}, "description": { "zh": "读取 BLE characteristic。", "en": "Read a BLE characteristic." }, "parameters": [
            { "name": "session_id", "description": { "zh": "BLE 会话 ID。", "en": "BLE session ID." }, "type": "string", "required": true },
            { "name": "service_uuid", "description": { "zh": "Service UUID。", "en": "Service UUID." }, "type": "string", "required": true },
            { "name": "characteristic_uuid", "description": { "zh": "Characteristic UUID。", "en": "Characteristic UUID." }, "type": "string", "required": true },
            { "name": "timeout_ms", "description": { "zh": "等待毫秒数。", "en": "Wait time in milliseconds." }, "type": "number", "required": false }
        ] },
        { "name": "bluetooth_ble_write_characteristic", "display": {"label": {"zh": "写入 BLE 特征", "en": "Write BLE characteristic"}, "icon": "edit", "targetParameter": "characteristic_uuid"}, "description": { "zh": "写入 BLE characteristic。", "en": "Write a BLE characteristic." }, "parameters": [
            { "name": "session_id", "description": { "zh": "BLE 会话 ID。", "en": "BLE session ID." }, "type": "string", "required": true },
            { "name": "service_uuid", "description": { "zh": "Service UUID。", "en": "Service UUID." }, "type": "string", "required": true },
            { "name": "characteristic_uuid", "description": { "zh": "Characteristic UUID。", "en": "Characteristic UUID." }, "type": "string", "required": true },
            { "name": "text", "description": { "zh": "UTF-8 文本。", "en": "UTF-8 text." }, "type": "string", "required": false },
            { "name": "data_base64", "description": { "zh": "Base64 字节。", "en": "Base64 bytes." }, "type": "string", "required": false }
        ] },
        { "name": "bluetooth_ble_write_and_read_characteristic", "display": {"label": {"zh": "收发 BLE 特征数据", "en": "Exchange BLE characteristic data"}, "targetParameter": "session_id"}, "description": { "zh": "写入并读取 BLE characteristic。", "en": "Write and read BLE characteristics." }, "parameters": [
            { "name": "session_id", "description": { "zh": "BLE 会话 ID。", "en": "BLE session ID." }, "type": "string", "required": true },
            { "name": "write_service_uuid", "description": { "zh": "写入 Service UUID。", "en": "Write service UUID." }, "type": "string", "required": true },
            { "name": "write_characteristic_uuid", "description": { "zh": "写入 Characteristic UUID。", "en": "Write characteristic UUID." }, "type": "string", "required": true },
            { "name": "read_service_uuid", "description": { "zh": "读取 Service UUID。", "en": "Read service UUID." }, "type": "string", "required": true },
            { "name": "read_characteristic_uuid", "description": { "zh": "读取 Characteristic UUID。", "en": "Read characteristic UUID." }, "type": "string", "required": true },
            { "name": "text", "description": { "zh": "UTF-8 文本。", "en": "UTF-8 text." }, "type": "string", "required": false },
            { "name": "data_base64", "description": { "zh": "Base64 字节。", "en": "Base64 bytes." }, "type": "string", "required": false },
            { "name": "timeout_ms", "description": { "zh": "等待毫秒数。", "en": "Wait time in milliseconds." }, "type": "number", "required": false }
        ] },
        { "name": "bluetooth_ble_subscribe_characteristic", "display": {"label": {"zh": "设置 BLE 订阅", "en": "Set BLE subscription"}, "icon": "notification", "targetParameter": "characteristic_uuid"}, "description": { "zh": "订阅或取消订阅 BLE characteristic。", "en": "Subscribe or unsubscribe a BLE characteristic." }, "parameters": [
            { "name": "session_id", "description": { "zh": "BLE 会话 ID。", "en": "BLE session ID." }, "type": "string", "required": true },
            { "name": "service_uuid", "description": { "zh": "Service UUID。", "en": "Service UUID." }, "type": "string", "required": true },
            { "name": "characteristic_uuid", "description": { "zh": "Characteristic UUID。", "en": "Characteristic UUID." }, "type": "string", "required": true },
            { "name": "enable", "description": { "zh": "是否订阅。", "en": "Whether to subscribe." }, "type": "boolean", "required": false }
        ] },
        { "name": "bluetooth_ble_read_notifications", "display": {"label": {"zh": "读取 BLE 通知", "en": "Read BLE notifications"}, "icon": "notification", "targetParameter": "session_id"}, "description": { "zh": "读取 BLE 通知。", "en": "Read BLE notifications." }, "parameters": [
            { "name": "session_id", "description": { "zh": "BLE 会话 ID。", "en": "BLE session ID." }, "type": "string", "required": true },
            { "name": "limit", "description": { "zh": "读取条数。", "en": "Number of notifications." }, "type": "number", "required": false }
        ] },
        { "name": "get_device_info", "display": {"label": {"zh": "读取设备信息", "en": "Read device info"}}, "description": { "zh": "获取详细设备信息。", "en": "Get detailed device information." }, "parameters": [] }
    ]
}
*/

type ToolResponse = {
    success: boolean;
    message: string;
    data?: unknown;
};

type EmptyParams = Record<string, never>;
type UsageParams = {
    package_name?: string;
    since_hours?: number;
    limit?: number;
    include_system_apps?: boolean;
};
type ScanParams = { duration_ms?: number | string; include_ble?: boolean };
type ConnectParams = { address: string; uuid?: string };
type ListenParams = { name?: string; uuid?: string };
type AcceptParams = { listener_session_id: string; timeout_ms?: number | string };
type PayloadParams = { session_id: string; text?: string; data_base64?: string };
type ReadParams = { session_id: string; max_bytes?: number | string; timeout_ms?: number | string };
type SendAndReadParams = PayloadParams & { max_bytes?: number | string; timeout_ms?: number | string };
type BleConnectParams = { address: string; auto_connect?: boolean };
type BleDiscoverParams = { session_id: string; timeout_ms?: number | string };
type BleCharacteristicParams = {
    session_id: string;
    service_uuid: string;
    characteristic_uuid: string;
    timeout_ms?: number | string;
};
type BleWriteParams = Omit<BleCharacteristicParams, "timeout_ms"> & { text?: string; data_base64?: string };
type BleWriteAndReadParams = {
    session_id: string;
    write_service_uuid: string;
    write_characteristic_uuid: string;
    read_service_uuid: string;
    read_characteristic_uuid: string;
    text?: string;
    data_base64?: string;
    timeout_ms?: number | string;
};
type BleSubscribeParams = Omit<BleCharacteristicParams, "timeout_ms"> & { enable?: boolean };
type BleReadNotificationsParams = { session_id: string; limit?: number | string };

/** Gets a system setting through the shared host API. */
async function get_system_setting(params: { setting: string; namespace?: string }): Promise<ToolResponse> {
    const result = await Tools.System.getSetting(params.setting, params.namespace ?? "system");
    return { success: true, message: "成功获取系统设置", data: result };
}

/** Modifies a system setting through the shared host API. */
async function modify_system_setting(params: { setting: string; value: string; namespace?: string }): Promise<ToolResponse> {
    const result = await Tools.System.setSetting(params.setting, params.value, params.namespace ?? "system");
    const success = result.value === params.value;
    return { success, message: success ? "成功修改系统设置" : "修改系统设置失败", data: result };
}

/** Installs an application from an APK path. */
async function install_app(params: { path: string }): Promise<ToolResponse> {
    const result = await Tools.System.installApp(params.path);
    return { success: result.success, message: result.success ? "应用安装成功" : "应用安装失败", data: result };
}

/** Uninstalls an application by package name. */
async function uninstall_app(params: { package_name: string; keep_data?: boolean }): Promise<ToolResponse> {
    const result = await Tools.System.uninstallApp(params.package_name);
    return { success: result.success, message: result.success ? "应用卸载成功" : "应用卸载失败", data: result };
}

/** Lists installed applications. */
async function list_installed_apps(params: { include_system_apps?: boolean }): Promise<ToolResponse> {
    const result = await Tools.System.listApps(params.include_system_apps ?? false);
    return { success: true, message: "成功获取应用列表", data: result };
}

/** Starts an application by package name. */
async function start_app(params: { package_name: string; activity?: string }): Promise<ToolResponse> {
    const result = await Tools.System.startApp(params.package_name, params.activity);
    return { success: result.success, message: result.success ? "应用启动成功" : "应用启动失败", data: result };
}

/** Stops an application by package name. */
async function stop_app(params: { package_name: string }): Promise<ToolResponse> {
    const result = await Tools.System.stopApp(params.package_name);
    return { success: result.success, message: result.success ? "应用停止成功" : "应用停止失败", data: result };
}

/** Gets device notifications. */
async function get_notifications(params: { limit?: number; include_ongoing?: boolean }): Promise<ToolResponse> {
    const result = await Tools.System.getNotifications(params.limit ?? 10, params.include_ongoing ?? false);
    return { success: true, message: "成功获取通知", data: result };
}

/** Gets application foreground usage time. */
async function get_app_usage_time(params: UsageParams): Promise<ToolResponse> {
    const result = await Tools.System.getAppUsageTime({
        packageName: params.package_name,
        sinceHours: params.since_hours ?? 24,
        limit: params.limit ?? 10,
        includeSystemApps: params.include_system_apps ?? false,
    });
    return { success: true, message: "成功获取应用使用时长", data: result };
}

/** Gets the current device location. */
async function get_device_location(params: { high_accuracy?: boolean; timeout?: number }): Promise<ToolResponse> {
    const result = await Tools.System.getLocation(params.high_accuracy ?? false, params.timeout ?? 10);
    return { success: true, message: "成功获取位置信息", data: result };
}

/** Requests Bluetooth nearby-device permission. */
async function request_bluetooth_permission(_params: EmptyParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.requestPermission();
    return { success: true, message: "成功请求蓝牙权限", data: result };
}

/** Gets the Bluetooth adapter state. */
async function get_bluetooth_state(_params: EmptyParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.getState();
    return { success: true, message: "成功获取蓝牙状态", data: result };
}

/** Requests the system Bluetooth enable dialog. */
async function request_enable_bluetooth(_params: EmptyParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.requestEnable();
    return { success: true, message: "已打开蓝牙开启请求", data: result };
}

/** Lists bonded Bluetooth devices. */
async function list_bluetooth_bonded_devices(_params: EmptyParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.listBondedDevices();
    return { success: true, message: "成功获取已配对蓝牙设备", data: result };
}

/** Scans nearby classic Bluetooth and BLE devices. */
async function scan_bluetooth_devices(params: ScanParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.scan({ durationMs: params.duration_ms, includeBle: params.include_ble });
    return { success: true, message: "成功扫描蓝牙设备", data: result };
}

/** Connects to a classic Bluetooth device. */
async function bluetooth_connect(params: ConnectParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.connect({ address: params.address, uuid: params.uuid });
    return { success: true, message: "成功连接蓝牙设备", data: result };
}

/** Opens a classic Bluetooth listener. */
async function bluetooth_listen(params: ListenParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.listen({ name: params.name, uuid: params.uuid });
    return { success: true, message: "成功创建蓝牙监听", data: result };
}

/** Accepts an incoming classic Bluetooth connection. */
async function bluetooth_accept(params: AcceptParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.accept(params.listener_session_id, params.timeout_ms);
    return { success: true, message: "成功接受蓝牙连接", data: result };
}

/** Sends text or Base64 bytes over a classic Bluetooth session. */
async function bluetooth_send(params: PayloadParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.send(params.session_id, { text: params.text, dataBase64: params.data_base64 });
    return { success: true, message: "成功发送蓝牙数据", data: result };
}

/** Reads data from a classic Bluetooth session. */
async function bluetooth_read(params: ReadParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.read(params.session_id, { maxBytes: params.max_bytes, timeoutMs: params.timeout_ms });
    return { success: true, message: "成功读取蓝牙数据", data: result };
}

/** Sends data and reads the response from a classic Bluetooth session. */
async function bluetooth_send_and_read(params: SendAndReadParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.sendAndRead(params.session_id, {
        text: params.text,
        dataBase64: params.data_base64,
        maxBytes: params.max_bytes,
        timeoutMs: params.timeout_ms,
    });
    return { success: true, message: "成功发送并读取蓝牙数据", data: result };
}

/** Closes a Bluetooth or BLE session. */
async function bluetooth_close(params: { session_id: string }): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.close(params.session_id);
    return { success: true, message: "成功关闭蓝牙会话", data: result };
}

/** Connects to a BLE device. */
async function bluetooth_ble_connect(params: BleConnectParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.ble.connect({ address: params.address, autoConnect: params.auto_connect });
    return { success: true, message: "成功连接 BLE 设备", data: result };
}

/** Discovers services and characteristics on a BLE session. */
async function bluetooth_ble_discover_services(params: BleDiscoverParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.ble.discoverServices(params.session_id, params.timeout_ms);
    return { success: true, message: "成功发现 BLE 服务", data: result };
}

/** Reads a BLE characteristic. */
async function bluetooth_ble_read_characteristic(params: BleCharacteristicParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.ble.readCharacteristic(params.session_id, {
        serviceUuid: params.service_uuid,
        characteristicUuid: params.characteristic_uuid,
        timeoutMs: params.timeout_ms,
    });
    return { success: true, message: "成功读取 BLE characteristic", data: result };
}

/** Writes text or Base64 bytes to a BLE characteristic. */
async function bluetooth_ble_write_characteristic(params: BleWriteParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.ble.writeCharacteristic(params.session_id, {
        serviceUuid: params.service_uuid,
        characteristicUuid: params.characteristic_uuid,
        text: params.text,
        dataBase64: params.data_base64,
    });
    return { success: true, message: "成功写入 BLE characteristic", data: result };
}

/** Writes to one BLE characteristic and reads another characteristic. */
async function bluetooth_ble_write_and_read_characteristic(params: BleWriteAndReadParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.ble.writeAndReadCharacteristic(params.session_id, {
        writeServiceUuid: params.write_service_uuid,
        writeCharacteristicUuid: params.write_characteristic_uuid,
        readServiceUuid: params.read_service_uuid,
        readCharacteristicUuid: params.read_characteristic_uuid,
        text: params.text,
        dataBase64: params.data_base64,
        timeoutMs: params.timeout_ms,
    });
    return { success: true, message: "成功写入并读取 BLE characteristic", data: result };
}

/** Enables or disables BLE characteristic notifications. */
async function bluetooth_ble_subscribe_characteristic(params: BleSubscribeParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.ble.subscribe(params.session_id, {
        serviceUuid: params.service_uuid,
        characteristicUuid: params.characteristic_uuid,
        enable: params.enable,
    });
    return { success: true, message: "成功更新 BLE 订阅", data: result };
}

/** Reads queued BLE notifications. */
async function bluetooth_ble_read_notifications(params: BleReadNotificationsParams): Promise<ToolResponse> {
    const result = await Tools.System.bluetooth.ble.readNotifications(params.session_id, params.limit);
    return { success: true, message: "成功读取 BLE 通知", data: result };
}

/** Gets detailed device information. */
async function get_device_info(_params: EmptyParams): Promise<ToolResponse> {
    const result = await Tools.System.getDeviceInfo();
    return { success: true, message: "成功获取设备信息", data: result };
}

/** Completes a package tool call and reports unexpected errors. */
async function completeTool<P>(func: (params: P) => Promise<ToolResponse>, params: P): Promise<void> {
    try {
        complete(await func(params));
    } catch (error: any) {
        console.error(`Tool ${func.name} failed unexpectedly`, error);
        complete({ success: false, message: `工具执行时发生意外错误: ${error.message}` });
    }
}

/** Reports that the system tools package is loaded. */
async function main(): Promise<void> {
    complete({ success: true, message: "系统工具包已加载" });
}

exports.get_system_setting = (params: { setting: string; namespace?: string }) => completeTool(get_system_setting, params);
exports.modify_system_setting = (params: { setting: string; value: string; namespace?: string }) => completeTool(modify_system_setting, params);
exports.install_app = (params: { path: string }) => completeTool(install_app, params);
exports.uninstall_app = (params: { package_name: string; keep_data?: boolean }) => completeTool(uninstall_app, params);
exports.list_installed_apps = (params: { include_system_apps?: boolean }) => completeTool(list_installed_apps, params);
exports.start_app = (params: { package_name: string; activity?: string }) => completeTool(start_app, params);
exports.stop_app = (params: { package_name: string }) => completeTool(stop_app, params);
exports.get_notifications = (params: { limit?: number; include_ongoing?: boolean }) => completeTool(get_notifications, params);
exports.get_app_usage_time = (params: UsageParams) => completeTool(get_app_usage_time, params);
exports.get_device_location = (params: { high_accuracy?: boolean; timeout?: number }) => completeTool(get_device_location, params);
exports.request_bluetooth_permission = (params: EmptyParams) => completeTool(request_bluetooth_permission, params);
exports.get_bluetooth_state = (params: EmptyParams) => completeTool(get_bluetooth_state, params);
exports.request_enable_bluetooth = (params: EmptyParams) => completeTool(request_enable_bluetooth, params);
exports.list_bluetooth_bonded_devices = (params: EmptyParams) => completeTool(list_bluetooth_bonded_devices, params);
exports.scan_bluetooth_devices = (params: ScanParams) => completeTool(scan_bluetooth_devices, params);
exports.bluetooth_connect = (params: ConnectParams) => completeTool(bluetooth_connect, params);
exports.bluetooth_listen = (params: ListenParams) => completeTool(bluetooth_listen, params);
exports.bluetooth_accept = (params: AcceptParams) => completeTool(bluetooth_accept, params);
exports.bluetooth_send = (params: PayloadParams) => completeTool(bluetooth_send, params);
exports.bluetooth_read = (params: ReadParams) => completeTool(bluetooth_read, params);
exports.bluetooth_send_and_read = (params: SendAndReadParams) => completeTool(bluetooth_send_and_read, params);
exports.bluetooth_close = (params: { session_id: string }) => completeTool(bluetooth_close, params);
exports.bluetooth_ble_connect = (params: BleConnectParams) => completeTool(bluetooth_ble_connect, params);
exports.bluetooth_ble_discover_services = (params: BleDiscoverParams) => completeTool(bluetooth_ble_discover_services, params);
exports.bluetooth_ble_read_characteristic = (params: BleCharacteristicParams) => completeTool(bluetooth_ble_read_characteristic, params);
exports.bluetooth_ble_write_characteristic = (params: BleWriteParams) => completeTool(bluetooth_ble_write_characteristic, params);
exports.bluetooth_ble_write_and_read_characteristic = (params: BleWriteAndReadParams) => completeTool(bluetooth_ble_write_and_read_characteristic, params);
exports.bluetooth_ble_subscribe_characteristic = (params: BleSubscribeParams) => completeTool(bluetooth_ble_subscribe_characteristic, params);
exports.bluetooth_ble_read_notifications = (params: BleReadNotificationsParams) => completeTool(bluetooth_ble_read_notifications, params);
exports.get_device_info = (params: EmptyParams) => completeTool(get_device_info, params);
exports.main = main;
