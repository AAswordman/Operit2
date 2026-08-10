// Generated from canonical Rust Tools traits and bindings. Do not edit.
var Tools = {};

function __operitToolsSnakeCase(name) {
    return String(name).replace(/[A-Z]/g, function(character) {
        return "_" + character.toLowerCase();
    });
}

function __operitToolsFlattens(name) {
    return name === "options" || name === "params" || name === "updates" ||
        name.endsWith("OrOptions") || name.endsWith("OrParams");
}

function __operitToolsScalarField(name) {
    return __operitToolsSnakeCase(name.replace(/OrOptions$|OrParams$/, ""));
}

function __operitToolsWireName(namespace, method, name) {
    if (namespace === "Files" && method === "writeBinary" && name === "base64Content") return "base64Content";
    if (namespace === "Files" && (method === "apply" || method === "create" || method === "edit") && name === "newContent") return "new";
    if (namespace === "Files" && method === "edit" && name === "oldContent") return "old";
    if (namespace === "System" && method === "sleep" && name === "milliseconds") return "duration_ms";
    if (namespace === "System" && method === "listApps" && name === "includeSystem") return "include_system_apps";
    return __operitToolsSnakeCase(name);
}

function __operitToolsSelectOverload(overloads, args) {
    if (overloads.length === 1) return overloads[0];
    if (args.length > 0 && args[0] !== null && typeof args[0] === "object" && !Array.isArray(args[0])) {
        for (var index = 0; index < overloads.length; index += 1) {
            if (overloads[index].length === 1 && __operitToolsFlattens(overloads[index][0])) {
                return overloads[index];
            }
        }
    }
    var selected = overloads[0];
    for (var overloadIndex = 1; overloadIndex < overloads.length; overloadIndex += 1) {
        if (overloads[overloadIndex].length > selected.length) selected = overloads[overloadIndex];
    }
    return selected;
}

function __operitToolsBuildParameters(namespace, method, overloads, args) {
    var names = __operitToolsSelectOverload(overloads, args);
    var parameters = {};
    for (var index = 0; index < names.length; index += 1) {
        var name = names[index];
        var value = args[index];
        if (__operitToolsFlattens(name)) {
            if (value === undefined || value === null) continue;
            if (typeof value === "object" && !Array.isArray(value)) {
                Object.keys(value).forEach(function(field) {
                    var wireField = namespace === "Net" ? field : __operitToolsSnakeCase(field);
                    parameters[wireField] = value[field];
                });
            } else {
                parameters[__operitToolsScalarField(name)] = value;
            }
        } else if (value !== undefined) {
            parameters[__operitToolsWireName(namespace, method, name)] = value;
        }
    }
    if (namespace === "Net" && method === "httpGet") parameters.method = "GET";
    if (namespace === "Net" && method === "httpPost") parameters.method = "POST";
    if (namespace === "Net.cookies" && method === "get") parameters.action = "get";
    if (namespace === "Net.cookies" && method === "set") parameters.action = "set";
    if (namespace === "Net.cookies" && method === "clear") parameters.action = "clear";
    if (namespace === "Memory" && Array.isArray(parameters.titles)) parameters.titles = parameters.titles.join(",");
    return parameters;
}

function __operitInvokeToolsBinding(namespace, method, toolName, overloads, args) {
    return toolCall(toolName, __operitToolsBuildParameters(namespace, method, overloads, args));
}

Tools["Chat"] = Tools["Chat"] || {};
Tools["Chat"]["agentStatus"] = function(chatId) {
    return __operitInvokeToolsBinding("Chat", "agentStatus", "agent_status", [["chatId"]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["createNew"] = function(group, setAsCurrentChat, characterCardId) {
    return __operitInvokeToolsBinding("Chat", "createNew", "create_new_chat", [["group", "setAsCurrentChat", "characterCardId"]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["deleteChat"] = function(chatId) {
    return __operitInvokeToolsBinding("Chat", "deleteChat", "delete_chat", [["chatId"]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["findChat"] = function(params) {
    return __operitInvokeToolsBinding("Chat", "findChat", "find_chat", [["params"]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["getMessages"] = function(chatId, options) {
    return __operitInvokeToolsBinding("Chat", "getMessages", "get_chat_messages", [["chatId", "options"]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["listAll"] = function() {
    return __operitInvokeToolsBinding("Chat", "listAll", "list_chats", [[]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["listCharacterCards"] = function() {
    return __operitInvokeToolsBinding("Chat", "listCharacterCards", "list_character_cards", [[]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["listChats"] = function(params) {
    return __operitInvokeToolsBinding("Chat", "listChats", "list_chats", [["params"]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["sendMessage"] = function(message, chatId, roleCardId, senderName, options) {
    return __operitInvokeToolsBinding("Chat", "sendMessage", "send_message_to_ai", [["message", "chatId", "roleCardId", "senderName", "options"]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["sendMessageStreaming"] = function(message, chatId, roleCardId, senderName, options) {
    return __operitInvokeToolsBinding("Chat", "sendMessageStreaming", "send_message_to_ai_streaming", [["message", "chatId", "roleCardId", "senderName", "options"]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["startService"] = function(options) {
    return __operitInvokeToolsBinding("Chat", "startService", "start_chat_service", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["stopService"] = function() {
    return __operitInvokeToolsBinding("Chat", "stopService", "stop_chat_service", [[]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["switchTo"] = function(chatId) {
    return __operitInvokeToolsBinding("Chat", "switchTo", "switch_chat", [["chatId"]], Array.prototype.slice.call(arguments));
};
Tools["Chat"]["updateTitle"] = function(chatId, title) {
    return __operitInvokeToolsBinding("Chat", "updateTitle", "update_chat_title", [["chatId", "title"]], Array.prototype.slice.call(arguments));
};
Tools["Files"] = Tools["Files"] || {};
Tools["Files"]["apply"] = function(path, type, old, newContent) {
    return __operitInvokeToolsBinding("Files", "apply", "apply_file", [["path", "type", "old", "newContent"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["copy"] = function(source, destination, recursive) {
    return __operitInvokeToolsBinding("Files", "copy", "copy_file", [["source", "destination", "recursive"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["create"] = function(path, newContent) {
    return __operitInvokeToolsBinding("Files", "create", "create_file", [["path", "newContent"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["deleteFile"] = function(path, recursive) {
    return __operitInvokeToolsBinding("Files", "deleteFile", "delete_file", [["path", "recursive"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["download"] = function(url, destination, headers) {
    return __operitInvokeToolsBinding("Files", "download", "download_file", [["url", "destination", "headers"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["edit"] = function(path, oldContent, newContent) {
    return __operitInvokeToolsBinding("Files", "edit", "edit_file", [["path", "oldContent", "newContent"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["exists"] = function(path) {
    return __operitInvokeToolsBinding("Files", "exists", "file_exists", [["path"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["find"] = function(path, pattern, options) {
    return __operitInvokeToolsBinding("Files", "find", "find_files", [["path", "pattern", "options"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["grep"] = function(path, pattern, options) {
    return __operitInvokeToolsBinding("Files", "grep", "grep_code", [["path", "pattern", "options"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["grepContext"] = function(path, intent, options) {
    return __operitInvokeToolsBinding("Files", "grepContext", "grep_context", [["path", "intent", "options"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["info"] = function(path) {
    return __operitInvokeToolsBinding("Files", "info", "file_info", [["path"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["list"] = function(path) {
    return __operitInvokeToolsBinding("Files", "list", "list_files", [["path"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["mkdir"] = function(path, create_parents) {
    return __operitInvokeToolsBinding("Files", "mkdir", "make_directory", [["path", "create_parents"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["move"] = function(source, destination) {
    return __operitInvokeToolsBinding("Files", "move", "move_file", [["source", "destination"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["open"] = function(path) {
    return __operitInvokeToolsBinding("Files", "open", "open_file", [["path"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["read"] = function(options) {
    return __operitInvokeToolsBinding("Files", "read", "read_file_full", [["path"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["readBinary"] = function(path) {
    return __operitInvokeToolsBinding("Files", "readBinary", "read_file_binary", [["path"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["readPart"] = function(path, startLine, endLine) {
    return __operitInvokeToolsBinding("Files", "readPart", "read_file_part", [["path", "startLine", "endLine"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["share"] = function(path, title) {
    return __operitInvokeToolsBinding("Files", "share", "share_file", [["path", "title"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["unzip"] = function(source, destination) {
    return __operitInvokeToolsBinding("Files", "unzip", "unzip_files", [["source", "destination"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["write"] = function(path, content, append) {
    return __operitInvokeToolsBinding("Files", "write", "write_file", [["path", "content", "append"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["writeBinary"] = function(path, base64Content) {
    return __operitInvokeToolsBinding("Files", "writeBinary", "write_file_binary", [["path", "base64Content"]], Array.prototype.slice.call(arguments));
};
Tools["Files"]["zip"] = function(source, destination, include_root_directory) {
    return __operitInvokeToolsBinding("Files", "zip", "zip_files", [["source", "destination", "include_root_directory"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"] = Tools["Memory"] || {};
Tools["Memory"]["create"] = function(title, content, targetOwnerKey, contentType, source, folderPath, tags) {
    return __operitInvokeToolsBinding("Memory", "create", "create_memory", [["title", "content", "targetOwnerKey", "contentType", "source", "folderPath", "tags"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"]["deleteLink"] = function(targetOwnerKey, linkId, sourceTitle, targetTitle, linkType) {
    return __operitInvokeToolsBinding("Memory", "deleteLink", "delete_memory_link", [["targetOwnerKey", "linkId", "sourceTitle", "targetTitle", "linkType"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"]["deleteMemory"] = function(title, targetOwnerKey) {
    return __operitInvokeToolsBinding("Memory", "deleteMemory", "delete_memory", [["title", "targetOwnerKey"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"]["getByTitle"] = function(title, targetOwnerKey, chunkIndex, chunkRange, query, limit) {
    return __operitInvokeToolsBinding("Memory", "getByTitle", "get_memory_by_title", [["title", "targetOwnerKey", "chunkIndex", "chunkRange", "query", "limit"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"]["link"] = function(sourceTitle, targetTitle, targetOwnerKey, linkType, weight, description) {
    return __operitInvokeToolsBinding("Memory", "link", "link_memories", [["sourceTitle", "targetTitle", "targetOwnerKey", "linkType", "weight", "description"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"]["move"] = function(targetFolderPath, targetOwnerKey, titles, sourceFolderPath) {
    return __operitInvokeToolsBinding("Memory", "move", "move_memory", [["targetFolderPath", "targetOwnerKey", "titles", "sourceFolderPath"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"]["query"] = function(query, folderPath, limit, startTime, endTime, snapshotId, threshold, targetOwnerKey) {
    return __operitInvokeToolsBinding("Memory", "query", "query_memory", [["query", "folderPath", "limit", "startTime", "endTime", "snapshotId", "threshold", "targetOwnerKey"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"]["queryLinks"] = function(targetOwnerKey, linkId, sourceTitle, targetTitle, linkType, limit) {
    return __operitInvokeToolsBinding("Memory", "queryLinks", "query_memory_links", [["targetOwnerKey", "linkId", "sourceTitle", "targetTitle", "linkType", "limit"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"]["update"] = function(oldTitle, targetOwnerKey, updates) {
    return __operitInvokeToolsBinding("Memory", "update", "update_memory", [["oldTitle", "targetOwnerKey", "updates"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"]["updateLink"] = function(targetOwnerKey, linkId, sourceTitle, targetTitle, linkType, newLinkType, weight, description) {
    return __operitInvokeToolsBinding("Memory", "updateLink", "update_memory_link", [["targetOwnerKey", "linkId", "sourceTitle", "targetTitle", "linkType", "newLinkType", "weight", "description"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Memory"]["updateUserPreferences"] = function(content, targetOwnerKey) {
    return __operitInvokeToolsBinding("Memory", "updateUserPreferences", "update_user_preferences", [["content", "targetOwnerKey"], ["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"] = Tools["Net"] || {};
Tools["Net"]["browserClick"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserClick", "browser_click", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserClose"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserClose", "browser_close", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserCloseAll"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserCloseAll", "browser_close_all", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserConsoleMessages"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserConsoleMessages", "browser_console_messages", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserDrag"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserDrag", "browser_drag", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserEvaluate"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserEvaluate", "browser_evaluate", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserFileUpload"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserFileUpload", "browser_file_upload", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserFillForm"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserFillForm", "browser_fill_form", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserHandleDialog"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserHandleDialog", "browser_handle_dialog", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserHover"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserHover", "browser_hover", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserNavigate"] = function(urlOrOptions) {
    return __operitInvokeToolsBinding("Net", "browserNavigate", "browser_navigate", [["urlOrOptions"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserNavigateBack"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserNavigateBack", "browser_navigate_back", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserNetworkRequests"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserNetworkRequests", "browser_network_requests", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserPressKey"] = function(keyOrOptions) {
    return __operitInvokeToolsBinding("Net", "browserPressKey", "browser_press_key", [["keyOrOptions"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserResize"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserResize", "browser_resize", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserRunCode"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserRunCode", "browser_run_code", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserSelectOption"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserSelectOption", "browser_select_option", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserSnapshot"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserSnapshot", "browser_snapshot", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserTabs"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserTabs", "browser_tabs", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserTakeScreenshot"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserTakeScreenshot", "browser_take_screenshot", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserType"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserType", "browser_type", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["browserWaitFor"] = function(options) {
    return __operitInvokeToolsBinding("Net", "browserWaitFor", "browser_wait_for", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["http"] = function(options) {
    return __operitInvokeToolsBinding("Net", "http", "http_request", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["httpGet"] = function(url, ignore_ssl) {
    return __operitInvokeToolsBinding("Net", "httpGet", "http_request", [["url", "ignore_ssl"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["httpPost"] = function(url, body, ignore_ssl) {
    return __operitInvokeToolsBinding("Net", "httpPost", "http_request", [["url", "body", "ignore_ssl"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["uploadFile"] = function(options) {
    return __operitInvokeToolsBinding("Net", "uploadFile", "multipart_request", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["visit"] = function(urlOrParams) {
    return __operitInvokeToolsBinding("Net", "visit", "visit_web", [["urlOrParams"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["cookies"] = Tools["Net"]["cookies"] || {};
Tools["Net"]["cookies"]["clear"] = function(domain) {
    return __operitInvokeToolsBinding("Net.cookies", "clear", "manage_cookies", [["domain"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["cookies"]["get"] = function(domain) {
    return __operitInvokeToolsBinding("Net.cookies", "get", "manage_cookies", [["domain"]], Array.prototype.slice.call(arguments));
};
Tools["Net"]["cookies"]["set"] = function(domain, cookies) {
    return __operitInvokeToolsBinding("Net.cookies", "set", "manage_cookies", [["domain", "cookies"]], Array.prototype.slice.call(arguments));
};
Tools["SoftwareSettings"] = Tools["SoftwareSettings"] || {};
Tools["SoftwareSettings"]["exec"] = function(args) {
    return __operitInvokeToolsBinding("SoftwareSettings", "exec", "execute_cli_command", [["args"]], Array.prototype.slice.call(arguments));
};
Tools["SoftwareSettings"]["readEnvironmentVariable"] = function(key) {
    return __operitInvokeToolsBinding("SoftwareSettings", "readEnvironmentVariable", "read_environment_variable", [["key"]], Array.prototype.slice.call(arguments));
};
Tools["SoftwareSettings"]["writeEnvironmentVariable"] = function(key, value) {
    return __operitInvokeToolsBinding("SoftwareSettings", "writeEnvironmentVariable", "write_environment_variable", [["key", "value"]], Array.prototype.slice.call(arguments));
};
Tools["System"] = Tools["System"] || {};
Tools["System"]["getAppUsageTime"] = function(options) {
    return __operitInvokeToolsBinding("System", "getAppUsageTime", "get_app_usage_time", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["getDeviceInfo"] = function() {
    return __operitInvokeToolsBinding("System", "getDeviceInfo", "device_info", [[]], Array.prototype.slice.call(arguments));
};
Tools["System"]["getLocation"] = function(highAccuracy, timeout, includeAddress) {
    return __operitInvokeToolsBinding("System", "getLocation", "get_device_location", [["highAccuracy", "timeout", "includeAddress"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["getNotifications"] = function(limit, includeOngoing) {
    return __operitInvokeToolsBinding("System", "getNotifications", "get_notifications", [["limit", "includeOngoing"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["getSetting"] = function(setting, namespace) {
    return __operitInvokeToolsBinding("System", "getSetting", "get_system_setting", [["setting", "namespace"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["installApp"] = function(path) {
    return __operitInvokeToolsBinding("System", "installApp", "install_app", [["path"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["listApps"] = function(includeSystem) {
    return __operitInvokeToolsBinding("System", "listApps", "list_installed_apps", [["includeSystem"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["sendNotification"] = function(message, title) {
    return __operitInvokeToolsBinding("System", "sendNotification", "send_notification", [["message", "title"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["setSetting"] = function(setting, value, namespace) {
    return __operitInvokeToolsBinding("System", "setSetting", "modify_system_setting", [["setting", "value", "namespace"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["sleep"] = function(milliseconds) {
    return __operitInvokeToolsBinding("System", "sleep", "sleep", [["milliseconds"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["startApp"] = function(packageName, activity) {
    return __operitInvokeToolsBinding("System", "startApp", "start_app", [["packageName", "activity"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["stopApp"] = function(packageName) {
    return __operitInvokeToolsBinding("System", "stopApp", "stop_app", [["packageName"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["toast"] = function(message) {
    return __operitInvokeToolsBinding("System", "toast", "toast", [["message"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["uninstallApp"] = function(packageName) {
    return __operitInvokeToolsBinding("System", "uninstallApp", "uninstall_app", [["packageName"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["usePackage"] = function(packageName) {
    return __operitInvokeToolsBinding("System", "usePackage", "use_package", [["packageName"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"] = Tools["System"]["bluetooth"] || {};
Tools["System"]["bluetooth"]["accept"] = function(listenerSessionId, timeoutMs) {
    return __operitInvokeToolsBinding("System.bluetooth", "accept", "bluetooth_accept", [["listenerSessionId", "timeoutMs"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["close"] = function(sessionId) {
    return __operitInvokeToolsBinding("System.bluetooth", "close", "bluetooth_close", [["sessionId"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["connect"] = function(options) {
    return __operitInvokeToolsBinding("System.bluetooth", "connect", "bluetooth_connect", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["getState"] = function() {
    return __operitInvokeToolsBinding("System.bluetooth", "getState", "get_bluetooth_state", [[]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["listBondedDevices"] = function() {
    return __operitInvokeToolsBinding("System.bluetooth", "listBondedDevices", "list_bluetooth_bonded_devices", [[]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["listen"] = function(options) {
    return __operitInvokeToolsBinding("System.bluetooth", "listen", "bluetooth_listen", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["read"] = function(sessionId, options) {
    return __operitInvokeToolsBinding("System.bluetooth", "read", "bluetooth_read", [["sessionId", "options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["requestEnable"] = function() {
    return __operitInvokeToolsBinding("System.bluetooth", "requestEnable", "request_enable_bluetooth", [[]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["requestPermission"] = function() {
    return __operitInvokeToolsBinding("System.bluetooth", "requestPermission", "request_bluetooth_permission", [[]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["scan"] = function(options) {
    return __operitInvokeToolsBinding("System.bluetooth", "scan", "scan_bluetooth_devices", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["send"] = function(sessionId, options) {
    return __operitInvokeToolsBinding("System.bluetooth", "send", "bluetooth_send", [["sessionId", "options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["sendAndRead"] = function(sessionId, options) {
    return __operitInvokeToolsBinding("System.bluetooth", "sendAndRead", "bluetooth_send_and_read", [["sessionId", "options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["ble"] = Tools["System"]["bluetooth"]["ble"] || {};
Tools["System"]["bluetooth"]["ble"]["connect"] = function(options) {
    return __operitInvokeToolsBinding("System.bluetooth.ble", "connect", "bluetooth_ble_connect", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["ble"]["discoverServices"] = function(sessionId, timeoutMs) {
    return __operitInvokeToolsBinding("System.bluetooth.ble", "discoverServices", "bluetooth_ble_discover_services", [["sessionId", "timeoutMs"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["ble"]["readCharacteristic"] = function(sessionId, options) {
    return __operitInvokeToolsBinding("System.bluetooth.ble", "readCharacteristic", "bluetooth_ble_read_characteristic", [["sessionId", "options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["ble"]["readNotifications"] = function(sessionId, limit) {
    return __operitInvokeToolsBinding("System.bluetooth.ble", "readNotifications", "bluetooth_ble_read_notifications", [["sessionId", "limit"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["ble"]["subscribe"] = function(sessionId, options) {
    return __operitInvokeToolsBinding("System.bluetooth.ble", "subscribe", "bluetooth_ble_subscribe_characteristic", [["sessionId", "options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["ble"]["writeAndReadCharacteristic"] = function(sessionId, options) {
    return __operitInvokeToolsBinding("System.bluetooth.ble", "writeAndReadCharacteristic", "bluetooth_ble_write_and_read_characteristic", [["sessionId", "options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["bluetooth"]["ble"]["writeCharacteristic"] = function(sessionId, options) {
    return __operitInvokeToolsBinding("System.bluetooth.ble", "writeCharacteristic", "bluetooth_ble_write_characteristic", [["sessionId", "options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["music"] = Tools["System"]["music"] || {};
Tools["System"]["music"]["pause"] = function() {
    return __operitInvokeToolsBinding("System.music", "pause", "music_pause", [[]], Array.prototype.slice.call(arguments));
};
Tools["System"]["music"]["play"] = function(options) {
    return __operitInvokeToolsBinding("System.music", "play", "music_play", [["options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["music"]["resume"] = function() {
    return __operitInvokeToolsBinding("System.music", "resume", "music_resume", [[]], Array.prototype.slice.call(arguments));
};
Tools["System"]["music"]["seek"] = function(positionMs) {
    return __operitInvokeToolsBinding("System.music", "seek", "music_seek", [["positionMs"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["music"]["setVolume"] = function(volume) {
    return __operitInvokeToolsBinding("System.music", "setVolume", "music_set_volume", [["volume"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["music"]["status"] = function() {
    return __operitInvokeToolsBinding("System.music", "status", "music_status", [[]], Array.prototype.slice.call(arguments));
};
Tools["System"]["music"]["stop"] = function() {
    return __operitInvokeToolsBinding("System.music", "stop", "music_stop", [[]], Array.prototype.slice.call(arguments));
};
Tools["System"]["terminal"] = Tools["System"]["terminal"] || {};
Tools["System"]["terminal"]["close"] = function(sessionId) {
    return __operitInvokeToolsBinding("System.terminal", "close", "close_terminal_session", [["sessionId"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["terminal"]["create"] = function(sessionName) {
    return __operitInvokeToolsBinding("System.terminal", "create", "create_terminal_session", [["sessionName"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["terminal"]["exec"] = function(sessionId, command, timeoutMs) {
    return __operitInvokeToolsBinding("System.terminal", "exec", "execute_in_terminal_session", [["sessionId", "command", "timeoutMs"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["terminal"]["execStreaming"] = function(sessionId, command, options) {
    return __operitInvokeToolsBinding("System.terminal", "execStreaming", "execute_in_terminal_session_streaming", [["sessionId", "command", "options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["terminal"]["hiddenExec"] = function(command, options) {
    return __operitInvokeToolsBinding("System.terminal", "hiddenExec", "execute_hidden_terminal_command", [["command", "options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["terminal"]["info"] = function() {
    return __operitInvokeToolsBinding("System.terminal", "info", "get_terminal_info", [[]], Array.prototype.slice.call(arguments));
};
Tools["System"]["terminal"]["input"] = function(sessionId, options) {
    return __operitInvokeToolsBinding("System.terminal", "input", "input_in_terminal_session", [["sessionId", "options"]], Array.prototype.slice.call(arguments));
};
Tools["System"]["terminal"]["screen"] = function(sessionId) {
    return __operitInvokeToolsBinding("System.terminal", "screen", "get_terminal_session_screen", [["sessionId"]], Array.prototype.slice.call(arguments));
};
