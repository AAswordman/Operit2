#[allow(non_snake_case)]
pub fn getJsToolsDefinition() -> &'static str {
    r#"
        var Tools = {
            Files: {
                list: function(path, environment) {
                    var params = { path: path };
                    if (environment) params.environment = environment;
                    return toolCall("list_files", params);
                },
                read: function(pathOrOptions) {
                    var params = typeof pathOrOptions === 'string' ? { path: pathOrOptions } : (pathOrOptions || {});
                    return toolCall("read_file_full", params);
                },
                readBinary: function(path, environment) {
                    var params = { path: path };
                    if (environment) params.environment = environment;
                    return toolCall("read_file_binary", params);
                },
                readPart: function(path, startLine, endLine, environment) {
                    var params = { path: path };
                    if (startLine !== undefined) params.start_line = String(startLine);
                    if (endLine !== undefined) params.end_line = String(endLine);
                    if (environment) params.environment = environment;
                    return toolCall("read_file_part", params);
                },
                write: function(path, content, append, environment) {
                    var params = { path: path, content: content };
                    if (append !== undefined) params.append = append ? "true" : "false";
                    if (environment) params.environment = environment;
                    return toolCall("write_file", params);
                },
                writeBinary: function(path, base64Content, environment) {
                    var params = { path: path, base64Content: base64Content };
                    if (environment) params.environment = environment;
                    return toolCall("write_file_binary", params);
                },
                deleteFile: function(path, recursive, environment) {
                    var params = { path: path };
                    if (recursive !== undefined) params.recursive = recursive ? "true" : "false";
                    if (environment) params.environment = environment;
                    return toolCall("delete_file", params);
                },
                exists: function(path, environment) {
                    var params = { path: path };
                    if (environment) params.environment = environment;
                    return toolCall("file_exists", params);
                },
                move: function(source, destination, environment) {
                    var params = { source: source, destination: destination };
                    if (environment) params.environment = environment;
                    return toolCall("move_file", params);
                },
                copy: function(source, destination, recursive, sourceEnvironment, destEnvironment) {
                    var params = { source: source, destination: destination };
                    if (recursive !== undefined) params.recursive = recursive ? "true" : "false";
                    if (sourceEnvironment) params.source_environment = sourceEnvironment;
                    if (destEnvironment) params.dest_environment = destEnvironment;
                    return toolCall("copy_file", params);
                },
                mkdir: function(path, createParents, environment) {
                    var params = { path: path };
                    if (createParents !== undefined) params.create_parents = createParents ? "true" : "false";
                    if (environment) params.environment = environment;
                    return toolCall("make_directory", params);
                },
                find: function(path, pattern, options, environment) {
                    var params = Object.assign({ path: path, pattern: pattern }, options || {});
                    if (environment) params.environment = environment;
                    return toolCall("find_files", params);
                },
                grep: function(path, pattern, options) {
                    return toolCall("grep_code", Object.assign({ path: path, pattern: pattern }, options || {}));
                },
                grepContext: function(path, intent, options) {
                    return toolCall("grep_context", Object.assign({ path: path, intent: intent }, options || {}));
                },
                info: function(path, environment) {
                    var params = { path: path };
                    if (environment) params.environment = environment;
                    return toolCall("file_info", params);
                },
                create: function(path, newContent, environment) {
                    var params = { path: path, new: newContent };
                    if (environment) params.environment = environment;
                    return toolCall("create_file", params);
                },
                edit: function(path, oldContent, newContent, environment) {
                    var params = { path: path, old: oldContent, new: newContent };
                    if (environment) params.environment = environment;
                    return toolCall("edit_file", params);
                },
                zip: function(source, destination, environment, includeRootDirectory) {
                    var params = { source: source, destination: destination };
                    if (environment) params.environment = environment;
                    if (includeRootDirectory !== undefined) params.include_root_directory = includeRootDirectory ? "true" : "false";
                    return toolCall("zip_files", params);
                },
                unzip: function(source, destination, environment) {
                    var params = { source: source, destination: destination };
                    if (environment) params.environment = environment;
                    return toolCall("unzip_files", params);
                },
                open: function(path, environment) {
                    var params = { path: path };
                    if (environment) params.environment = environment;
                    return toolCall("open_file", params);
                },
                share: function(path, title, environment) {
                    var params = { path: path };
                    if (title) params.title = title;
                    if (environment) params.environment = environment;
                    return toolCall("share_file", params);
                },
                download: function(urlOrOptions, destination, environment, headers) {
                    var params = typeof urlOrOptions === 'string' ? { url: urlOrOptions } : (urlOrOptions || {});
                    if (destination !== undefined && destination !== null) params.destination = destination;
                    if (environment) params.environment = environment;
                    if (headers !== undefined && headers !== null && typeof headers === 'object') params.headers = JSON.stringify(headers);
                    if (params.headers !== undefined && params.headers !== null && typeof params.headers === 'object') params.headers = JSON.stringify(params.headers);
                    return toolCall("download_file", params);
                },
                apply: function(path, type, oldContent, newContent, environment) {
                    var params = { path: path, type: type };
                    if (oldContent !== undefined && oldContent !== null) params.old = String(oldContent);
                    if (newContent !== undefined && newContent !== null) params.new = String(newContent);
                    if (environment) params.environment = environment;
                    return toolCall("apply_file", params);
                }
            },
            Net: {
                httpGet: function(url, ignoreSsl) {
                    var params = { url: url, method: "GET" };
                    if (ignoreSsl !== undefined) params.ignore_ssl = ignoreSsl ? "true" : "false";
                    return toolCall("http_request", params);
                },
                httpPost: function(url, body, ignoreSsl) {
                    var params = { url: url, method: "POST", body: typeof body === 'object' ? JSON.stringify(body) : body };
                    if (ignoreSsl !== undefined) params.ignore_ssl = ignoreSsl ? "true" : "false";
                    return toolCall("http_request", params);
                },
                visit: function(params) {
                    if (typeof params === 'string') return toolCall("visit_web", { url: params });
                    if (params && typeof params === 'object' && params.headers !== undefined && typeof params.headers === 'object') {
                        params = Object.assign({}, params, { headers: JSON.stringify(params.headers) });
                    }
                    return toolCall("visit_web", params || {});
                },
                http: function(options) {
                    var params = Object.assign({}, options || {});
                    if (params.body !== undefined && typeof params.body === 'object') params.body = JSON.stringify(params.body);
                    if (params.headers !== undefined && typeof params.headers === 'object') params.headers = JSON.stringify(params.headers);
                    if (params.ignore_ssl !== undefined && typeof params.ignore_ssl === 'boolean') params.ignore_ssl = params.ignore_ssl ? "true" : "false";
                    return toolCall("http_request", params);
                },
                uploadFile: function(options) {
                    var optionsValue = options || {};
                    var params = Object.assign({}, optionsValue, {
                        files: JSON.stringify(optionsValue.files || []),
                        form_data: JSON.stringify(optionsValue.form_data || {})
                    });
                    if (optionsValue.headers !== undefined && typeof optionsValue.headers === 'object') params.headers = JSON.stringify(optionsValue.headers);
                    if (params.ignore_ssl !== undefined && typeof params.ignore_ssl === 'boolean') params.ignore_ssl = params.ignore_ssl ? "true" : "false";
                    return toolCall("multipart_request", params);
                },
                cookies: {
                    get: function(domain) { return toolCall("manage_cookies", { action: "get", domain: domain }); },
                    set: function(domain, cookies) { return toolCall("manage_cookies", { action: "set", domain: domain, cookies: cookies }); },
                    clear: function(domain) { return toolCall("manage_cookies", { action: "clear", domain: domain }); }
                }
            },
            System: {
                sleep: function(milliseconds) { return toolCall("sleep", { duration_ms: parseInt(milliseconds) }); },
                getSetting: function(setting, namespace) { return toolCall("get_system_setting", { setting: setting, namespace: namespace }); },
                setSetting: function(setting, value, namespace) { return toolCall("modify_system_setting", { setting: setting, value: value, namespace: namespace }); },
                getDeviceInfo: function() { return toolCall("device_info"); },
                toast: function(message) { return toolCall("toast", { message: String(message === null || message === undefined ? "" : message) }); },
                sendNotification: function(message, title) {
                    var params = { message: String(message === null || message === undefined ? "" : message) };
                    if (title !== undefined && title !== null && String(title).trim() !== "") params.title = String(title);
                    return toolCall("send_notification", params);
                },
                usePackage: function(packageName) {
                    return toolCall("use_package", { package_name: String(packageName === null || packageName === undefined ? "" : packageName) });
                },
                installApp: function(path) { return toolCall("install_app", { path: path }); },
                uninstallApp: function(packageName) { return toolCall("uninstall_app", { package_name: packageName }); },
                startApp: function(packageName, activity) {
                    var params = { package_name: packageName };
                    if (activity) params.activity = activity;
                    return toolCall("start_app", params);
                },
                stopApp: function(packageName) { return toolCall("stop_app", { package_name: packageName }); },
                listApps: function(includeSystem) { return toolCall("list_installed_apps", { include_system_apps: !!includeSystem }); },
                getNotifications: function(limit, includeOngoing) {
                    return toolCall("get_notifications", { limit: parseInt(limit === undefined ? 10 : limit), include_ongoing: !!includeOngoing });
                },
                shell: function(command) { return toolCall("execute_shell", { command: command }); },
                terminal: {
                    create: function(sessionName) { return toolCall("create_terminal_session", { session_name: sessionName }); },
                    exec: function(sessionId, command, timeoutMs) {
                        var params = { session_id: sessionId, command: command };
                        if (timeoutMs !== undefined && timeoutMs !== null) params.timeout_ms = String(timeoutMs);
                        return toolCall("execute_in_terminal_session", params);
                    },
                    screen: function(sessionId) { return toolCall("get_terminal_session_screen", { session_id: sessionId }); },
                    close: function(sessionId) { return toolCall("close_terminal_session", { session_id: sessionId }); },
                    input: function(sessionId, options) {
                        var params = { session_id: sessionId };
                        options = options || {};
                        if (options.input !== undefined && options.input !== null) params.input = String(options.input);
                        if (options.control !== undefined && options.control !== null) params.control = String(options.control);
                        return toolCall("input_in_terminal_session", params);
                    }
                }
            },
            SoftwareSettings: {
                readEnvironmentVariable: function(key) {
                    return toolCall("read_environment_variable", { key: String(key === null || key === undefined ? "" : key) });
                },
                writeEnvironmentVariable: function(key, value) {
                    return toolCall("write_environment_variable", {
                        key: String(key === null || key === undefined ? "" : key),
                        value: value !== undefined && value !== null ? String(value) : ""
                    });
                },
                exec: function(args) {
                    return toolCall("execute_cli_command", {
                        args: JSON.stringify(args)
                    });
                }
            },
            Memory: {
                _normalizeCallerCardId: function(callerCardId) {
                    if (callerCardId === undefined || callerCardId === null) return undefined;
                    var normalized = String(callerCardId).trim();
                    return normalized.length > 0 ? normalized : undefined;
                },
                query: function(query, folderPath, limit, startTime, endTime, snapshotId, threshold, callerCardId) {
                    var options = query && typeof query === 'object' && !Array.isArray(query) ? query : { query: query, folderPath: folderPath, limit: limit, startTime: startTime, endTime: endTime, snapshotId: snapshotId, threshold: threshold, callerCardId: callerCardId };
                    var params = { query: options.query };
                    if (options.folderPath) params.folder_path = options.folderPath;
                    if (options.startTime !== undefined) params.start_time = options.startTime;
                    if (options.endTime !== undefined) params.end_time = options.endTime;
                    if (options.limit !== undefined) params.limit = options.limit;
                    if (options.snapshotId !== undefined && options.snapshotId !== null) params.snapshot_id = String(options.snapshotId);
                    if (options.threshold !== undefined) params.threshold = options.threshold;
                    var normalizedCallerCardId = Tools.Memory._normalizeCallerCardId(options.callerCardId);
                    if (normalizedCallerCardId !== undefined) params.caller_card_id = normalizedCallerCardId;
                    return toolCall("query_memory", params);
                },
                getByTitle: function(title, chunkIndex, chunkRange, query, limit, callerCardId) {
                    var options = title && typeof title === 'object' && !Array.isArray(title) ? title : { title: title, chunkIndex: chunkIndex, chunkRange: chunkRange, query: query, limit: limit, callerCardId: callerCardId };
                    var params = { title: options.title };
                    if (options.chunkIndex !== undefined) params.chunk_index = options.chunkIndex;
                    if (options.chunkRange) params.chunk_range = options.chunkRange;
                    if (options.query) params.query = options.query;
                    if (options.limit !== undefined) params.limit = options.limit;
                    var normalizedCallerCardId = Tools.Memory._normalizeCallerCardId(options.callerCardId);
                    if (normalizedCallerCardId !== undefined) params.caller_card_id = normalizedCallerCardId;
                    return toolCall("get_memory_by_title", params);
                },
                create: function(title, content, contentType, source, folderPath, tags, callerCardId) {
                    var options = title && typeof title === 'object' && !Array.isArray(title) ? title : { title: title, content: content, contentType: contentType, source: source, folderPath: folderPath, tags: tags, callerCardId: callerCardId };
                    var params = { title: options.title, content: options.content };
                    if (options.contentType) params.content_type = options.contentType;
                    if (options.source) params.source = options.source;
                    if (options.folderPath) params.folder_path = options.folderPath;
                    if (options.tags) params.tags = options.tags;
                    var normalizedCallerCardId = Tools.Memory._normalizeCallerCardId(options.callerCardId);
                    if (normalizedCallerCardId !== undefined) params.caller_card_id = normalizedCallerCardId;
                    return toolCall("create_memory", params);
                }
            },
            calc: function(expression) {
                return toolCall("calculate", { expression: expression });
            }
        };
    "#
}
