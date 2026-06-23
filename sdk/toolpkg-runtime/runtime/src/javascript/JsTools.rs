#[allow(non_snake_case)]
pub fn getJsToolsDefinition() -> &'static str {
    r#"
        var Tools = {
            Files: {
                list: function(path) {
                    var params = { path: path };
                    return toolCall("list_files", params);
                },
                read: function(pathOrOptions) {
                    var params = typeof pathOrOptions === 'string' ? { path: pathOrOptions } : {
                        path: (pathOrOptions || {}).path,
                        intent: (pathOrOptions || {}).intent,
                        direct_image: (pathOrOptions || {}).direct_image
                    };
                    return toolCall("read_file_full", params);
                },
                readBinary: function(path) {
                    var params = { path: path };
                    return toolCall("read_file_binary", params);
                },
                readPart: function(path, startLine, endLine) {
                    var params = { path: path };
                    if (startLine !== undefined) params.start_line = String(startLine);
                    if (endLine !== undefined) params.end_line = String(endLine);
                    return toolCall("read_file_part", params);
                },
                write: function(path, content, append) {
                    var params = { path: path, content: content };
                    if (append !== undefined) params.append = append ? "true" : "false";
                    return toolCall("write_file", params);
                },
                writeBinary: function(path, base64Content) {
                    var params = { path: path, base64Content: base64Content };
                    return toolCall("write_file_binary", params);
                },
                deleteFile: function(path, recursive) {
                    var params = { path: path };
                    if (recursive !== undefined) params.recursive = recursive ? "true" : "false";
                    return toolCall("delete_file", params);
                },
                exists: function(path) {
                    var params = { path: path };
                    return toolCall("file_exists", params);
                },
                move: function(source, destination) {
                    var params = { source: source, destination: destination };
                    return toolCall("move_file", params);
                },
                copy: function(source, destination, recursive) {
                    var params = { source: source, destination: destination };
                    if (recursive !== undefined) params.recursive = recursive ? "true" : "false";
                    return toolCall("copy_file", params);
                },
                mkdir: function(path, createParents) {
                    var params = { path: path };
                    if (createParents !== undefined) params.create_parents = createParents ? "true" : "false";
                    return toolCall("make_directory", params);
                },
                find: function(path, pattern, options) {
                    options = options || {};
                    var params = { path: path, pattern: pattern };
                    if (options.max_depth !== undefined) params.max_depth = options.max_depth;
                    if (options.use_path_pattern !== undefined) params.use_path_pattern = options.use_path_pattern;
                    if (options.case_insensitive !== undefined) params.case_insensitive = options.case_insensitive;
                    return toolCall("find_files", params);
                },
                grep: function(path, pattern, options) {
                    options = options || {};
                    var params = { path: path, pattern: pattern };
                    if (options.file_pattern !== undefined) params.file_pattern = options.file_pattern;
                    if (options.case_insensitive !== undefined) params.case_insensitive = options.case_insensitive;
                    if (options.context_lines !== undefined) params.context_lines = options.context_lines;
                    if (options.max_results !== undefined) params.max_results = options.max_results;
                    return toolCall("grep_code", params);
                },
                grepContext: function(path, intent, options) {
                    options = options || {};
                    var params = { path: path, intent: intent };
                    if (options.file_pattern !== undefined) params.file_pattern = options.file_pattern;
                    if (options.max_results !== undefined) params.max_results = options.max_results;
                    return toolCall("grep_context", params);
                },
                info: function(path) {
                    var params = { path: path };
                    return toolCall("file_info", params);
                },
                create: function(path, newContent) {
                    var params = { path: path, new: newContent };
                    return toolCall("create_file", params);
                },
                edit: function(path, oldContent, newContent) {
                    var params = { path: path, old: oldContent, new: newContent };
                    return toolCall("edit_file", params);
                },
                zip: function(source, destination, includeRootDirectory) {
                    var params = { source: source, destination: destination };
                    if (includeRootDirectory !== undefined) params.include_root_directory = includeRootDirectory ? "true" : "false";
                    return toolCall("zip_files", params);
                },
                unzip: function(source, destination) {
                    var params = { source: source, destination: destination };
                    return toolCall("unzip_files", params);
                },
                open: function(path) {
                    var params = { path: path };
                    return toolCall("open_file", params);
                },
                share: function(path, title) {
                    var params = { path: path };
                    if (title) params.title = title;
                    return toolCall("share_file", params);
                },
                download: function(urlOrOptions, destination, headers) {
                    var params = {};
                    if (typeof urlOrOptions === 'string') {
                        params.url = urlOrOptions;
                    } else {
                        var options = urlOrOptions || {};
                        if (options.url !== undefined) params.url = options.url;
                        if (options.visit_key !== undefined) params.visit_key = options.visit_key;
                        if (options.link_number !== undefined) params.link_number = options.link_number;
                        if (options.image_number !== undefined) params.image_number = options.image_number;
                        if (options.destination !== undefined) params.destination = options.destination;
                        if (options.headers !== undefined) params.headers = options.headers;
                    }
                    if (destination !== undefined && destination !== null) params.destination = destination;
                    if (headers !== undefined && headers !== null && typeof headers === 'object') params.headers = JSON.stringify(headers);
                    if (params.headers !== undefined && params.headers !== null && typeof params.headers === 'object') params.headers = JSON.stringify(params.headers);
                    return toolCall("download_file", params);
                },
                apply: function(path, type, oldContent, newContent) {
                    var params = { path: path, type: type };
                    if (oldContent !== undefined && oldContent !== null) params.old = String(oldContent);
                    if (newContent !== undefined && newContent !== null) params.new = String(newContent);
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
                browserNavigate: function(urlOrOptions) {
                    var params = typeof urlOrOptions === 'string' ? { url: urlOrOptions } : Object.assign({}, urlOrOptions || {});
                    if (!params.url) throw new Error("browserNavigate requires url");
                    if (params.headers !== undefined && typeof params.headers === 'object') params.headers = JSON.stringify(params.headers);
                    return toolCall("browser_navigate", params);
                },
                browserNavigateBack: function(options) {
                    if (options !== undefined && (typeof options !== 'object' || Array.isArray(options))) throw new Error("browserNavigateBack only accepts one options object");
                    return toolCall("browser_navigate_back", options || {});
                },
                browserClick: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserClick only accepts one options object");
                    var params = Object.assign({}, options);
                    if (params.ref !== undefined && params.ref !== null) params.ref = String(params.ref).trim();
                    if (params.selector !== undefined && params.selector !== null) {
                        params.selector = String(params.selector).trim();
                        if (!params.selector) delete params.selector;
                    }
                    if (!params.ref && !params.selector) throw new Error("browserClick requires ref or selector");
                    if (params.element !== undefined && params.element !== null) {
                        params.element = String(params.element).trim();
                        if (!params.element) delete params.element;
                    }
                    if (params.button !== undefined && params.button !== null) {
                        var button = String(params.button).trim();
                        if (button !== 'left' && button !== 'right' && button !== 'middle') throw new Error("button must be one of: left, right, middle");
                        params.button = button;
                    }
                    if (params.modifiers !== undefined) {
                        if (!Array.isArray(params.modifiers)) throw new Error("modifiers must be an array");
                        var allowedModifiers = ['Alt', 'Control', 'ControlOrMeta', 'Meta', 'Shift'];
                        var normalizedModifiers = params.modifiers.map(function(modifier) { return String(modifier).trim(); });
                        var invalidModifiers = normalizedModifiers.filter(function(modifier) { return allowedModifiers.indexOf(modifier) < 0; });
                        if (invalidModifiers.length > 0) throw new Error("Invalid modifiers: " + invalidModifiers.join(', '));
                        params.modifiers = normalizedModifiers;
                    }
                    if (params.doubleClick !== undefined) params.doubleClick = !!params.doubleClick;
                    return toolCall("browser_click", params);
                },
                browserClose: function(options) {
                    if (options !== undefined && (typeof options !== 'object' || Array.isArray(options))) throw new Error("browserClose only accepts one options object");
                    return toolCall("browser_close", options || {});
                },
                browserCloseAll: function(options) {
                    if (options !== undefined && (typeof options !== 'object' || Array.isArray(options))) throw new Error("browserCloseAll only accepts one options object");
                    return toolCall("browser_close_all", options || {});
                },
                browserConsoleMessages: function(options) {
                    if (options !== undefined && (typeof options !== 'object' || Array.isArray(options))) throw new Error("browserConsoleMessages only accepts one options object");
                    var params = Object.assign({}, options || {});
                    if (params.level !== undefined && params.level !== null) params.level = String(params.level).trim();
                    if (!params.level) params.level = "info";
                    if (params.filename !== undefined && params.filename !== null) params.filename = String(params.filename);
                    return toolCall("browser_console_messages", params);
                },
                browserDrag: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserDrag only accepts one options object");
                    var params = Object.assign({}, options);
                    ['startElement', 'startRef', 'endElement', 'endRef'].forEach(function(key) {
                        if (params[key] !== undefined && params[key] !== null) params[key] = String(params[key]).trim();
                        if (!params[key]) throw new Error("browserDrag requires " + key);
                    });
                    return toolCall("browser_drag", params);
                },
                browserEvaluate: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserEvaluate only accepts one options object");
                    var params = Object.assign({}, options);
                    if (params.function !== undefined && params.function !== null) params.function = String(params.function);
                    if (!params.function) throw new Error("browserEvaluate requires function");
                    if (params.element !== undefined && params.element !== null) params.element = String(params.element);
                    if (params.ref !== undefined && params.ref !== null) params.ref = String(params.ref).trim();
                    if (params.element && !params.ref) throw new Error("ref is required when element is provided");
                    return toolCall("browser_evaluate", params);
                },
                browserFileUpload: function(options) {
                    if (options !== undefined && (typeof options !== 'object' || Array.isArray(options))) throw new Error("browserFileUpload only accepts one options object");
                    var params = Object.assign({}, options || {});
                    if (params.paths !== undefined) {
                        if (!Array.isArray(params.paths)) throw new Error("paths must be an array");
                        params.paths = params.paths.map(function(path) { return String(path); });
                    }
                    return toolCall("browser_file_upload", params);
                },
                browserFillForm: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserFillForm only accepts one options object");
                    var params = Object.assign({}, options);
                    if (!Array.isArray(params.fields) || params.fields.length === 0) throw new Error("browserFillForm requires a non-empty fields array");
                    return toolCall("browser_fill_form", params);
                },
                browserHandleDialog: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserHandleDialog only accepts one options object");
                    var params = Object.assign({}, options);
                    if (typeof params.accept !== 'boolean') throw new Error("accept must be a boolean");
                    if (params.promptText !== undefined && params.promptText !== null) params.promptText = String(params.promptText);
                    return toolCall("browser_handle_dialog", params);
                },
                browserHover: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserHover only accepts one options object");
                    var params = Object.assign({}, options);
                    if (params.ref !== undefined && params.ref !== null) params.ref = String(params.ref).trim();
                    if (!params.ref) throw new Error("browserHover requires ref");
                    if (params.element !== undefined && params.element !== null) params.element = String(params.element);
                    return toolCall("browser_hover", params);
                },
                browserNetworkRequests: function(options) {
                    if (options !== undefined && (typeof options !== 'object' || Array.isArray(options))) throw new Error("browserNetworkRequests only accepts one options object");
                    var params = Object.assign({}, options || {});
                    if (params.includeStatic !== undefined) params.includeStatic = !!params.includeStatic;
                    if (params.filename !== undefined && params.filename !== null) params.filename = String(params.filename);
                    return toolCall("browser_network_requests", params);
                },
                browserPressKey: function(keyOrOptions) {
                    var params = typeof keyOrOptions === 'string' ? { key: keyOrOptions } : Object.assign({}, keyOrOptions || {});
                    if (params.key !== undefined && params.key !== null) params.key = String(params.key).trim();
                    if (!params.key) throw new Error("browserPressKey requires key");
                    return toolCall("browser_press_key", params);
                },
                browserResize: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserResize only accepts one options object");
                    var params = Object.assign({}, options);
                    if (params.width === undefined || params.height === undefined) throw new Error("browserResize requires width and height");
                    return toolCall("browser_resize", params);
                },
                browserRunCode: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserRunCode only accepts one options object");
                    var params = Object.assign({}, options);
                    if (params.code !== undefined && params.code !== null) params.code = String(params.code);
                    if (!params.code) throw new Error("browserRunCode requires code");
                    return toolCall("browser_run_code", params);
                },
                browserSelectOption: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserSelectOption only accepts one options object");
                    var params = Object.assign({}, options);
                    if (params.ref !== undefined && params.ref !== null) params.ref = String(params.ref).trim();
                    if (!params.ref) throw new Error("browserSelectOption requires ref");
                    if (!Array.isArray(params.values) || params.values.length === 0) throw new Error("browserSelectOption requires a non-empty values array");
                    params.values = params.values.map(function(value) { return String(value); });
                    if (params.element !== undefined && params.element !== null) params.element = String(params.element);
                    return toolCall("browser_select_option", params);
                },
                browserSnapshot: function(options) {
                    if (options !== undefined && (typeof options !== 'object' || Array.isArray(options))) throw new Error("browserSnapshot only accepts one options object");
                    var params = Object.assign({}, options || {});
                    if (params.filename !== undefined && params.filename !== null) {
                        params.filename = String(params.filename).trim();
                        if (!params.filename) delete params.filename;
                    }
                    if (params.selector !== undefined && params.selector !== null) {
                        params.selector = String(params.selector).trim();
                        if (!params.selector) delete params.selector;
                    }
                    if (params.depth !== undefined && params.depth !== null) {
                        var depth = Number(params.depth);
                        if (!Number.isInteger(depth) || depth < 0) throw new Error("browserSnapshot depth must be a non-negative integer");
                        params.depth = depth;
                    }
                    return toolCall("browser_snapshot", params);
                },
                browserTabs: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserTabs only accepts one options object");
                    var params = Object.assign({}, options);
                    if (params.action !== undefined && params.action !== null) params.action = String(params.action).trim();
                    if (!params.action) throw new Error("browserTabs requires action");
                    return toolCall("browser_tabs", params);
                },
                browserTakeScreenshot: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserTakeScreenshot only accepts one options object");
                    var params = Object.assign({}, options);
                    if (params.type !== undefined && params.type !== null) params.type = String(params.type).trim();
                    if (!params.type) params.type = "png";
                    if (params.element !== undefined && params.element !== null) params.element = String(params.element);
                    if (params.ref !== undefined && params.ref !== null) params.ref = String(params.ref).trim();
                    if (params.ref && !params.element) throw new Error("element is required when ref is provided");
                    if (params.element && !params.ref) throw new Error("ref is required when element is provided");
                    if (params.fullPage !== undefined) params.fullPage = !!params.fullPage;
                    return toolCall("browser_take_screenshot", params);
                },
                browserType: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserType only accepts one options object");
                    var params = Object.assign({}, options);
                    if (params.ref !== undefined && params.ref !== null) params.ref = String(params.ref).trim();
                    if (!params.ref) throw new Error("browserType requires ref");
                    if (params.text === undefined || params.text === null) throw new Error("browserType requires text");
                    params.text = String(params.text);
                    if (params.element !== undefined && params.element !== null) params.element = String(params.element);
                    if (params.submit !== undefined) params.submit = !!params.submit;
                    if (params.slowly !== undefined) params.slowly = !!params.slowly;
                    return toolCall("browser_type", params);
                },
                browserWaitFor: function(options) {
                    if (!options || typeof options !== 'object' || Array.isArray(options)) throw new Error("browserWaitFor only accepts one options object");
                    var params = Object.assign({}, options);
                    if (params.time === undefined && params.text === undefined && params.textGone === undefined) throw new Error("browserWaitFor requires one of: time, text, textGone");
                    if (params.text !== undefined && params.text !== null) params.text = String(params.text);
                    if (params.textGone !== undefined && params.textGone !== null) params.textGone = String(params.textGone);
                    return toolCall("browser_wait_for", params);
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
                getAppUsageTime: function(options) {
                    var params = Object.assign({}, options || {});
                    if (params.packageName !== undefined && params.packageName !== null) {
                        params.package_name = String(params.packageName);
                        delete params.packageName;
                    }
                    if (params.sinceHours !== undefined && params.sinceHours !== null) {
                        params.since_hours = parseInt(params.sinceHours);
                        delete params.sinceHours;
                    }
                    if (params.includeSystemApps !== undefined) {
                        params.include_system_apps = !!params.includeSystemApps;
                        delete params.includeSystemApps;
                    }
                    if (params.limit !== undefined && params.limit !== null) {
                        params.limit = parseInt(params.limit);
                    }
                    return toolCall("get_app_usage_time", params);
                },
                getLocation: function(highAccuracy, timeout) {
                    return toolCall("get_device_location", { high_accuracy: !!highAccuracy, timeout: parseInt(timeout === undefined ? 10 : timeout) });
                },
                intent: function(options) {
                    var source = options || {};
                    var params = {};
                    if (source.action !== undefined && source.action !== null) params.action = String(source.action);
                    if (source.uri !== undefined && source.uri !== null) params.uri = String(source.uri);
                    if (source.package !== undefined && source.package !== null) params.package = String(source.package);
                    if (source.component !== undefined && source.component !== null) params.component = String(source.component);
                    if (source.flags !== undefined && source.flags !== null) params.flags = String(source.flags);
                    if (source.extras !== undefined && source.extras !== null) params.extras = typeof source.extras === 'string' ? source.extras : JSON.stringify(source.extras);
                    if (source.type !== undefined && source.type !== null) params.type = String(source.type);
                    return toolCall("execute_intent", params);
                },
                sendBroadcast: function(options) {
                    var source = options || {};
                    var params = {};
                    if (source.action !== undefined && source.action !== null) params.action = String(source.action);
                    if (source.uri !== undefined && source.uri !== null) params.uri = String(source.uri);
                    if (source.package !== undefined && source.package !== null) params.package = String(source.package);
                    if (source.component !== undefined && source.component !== null) params.component = String(source.component);
                    if (source.extras !== undefined && source.extras !== null) params.extras = typeof source.extras === 'string' ? source.extras : JSON.stringify(source.extras);
                    if (source.extra_key !== undefined && source.extra_key !== null) params.extra_key = String(source.extra_key);
                    if (source.extra_value !== undefined && source.extra_value !== null) params.extra_value = String(source.extra_value);
                    if (source.extra_key2 !== undefined && source.extra_key2 !== null) params.extra_key2 = String(source.extra_key2);
                    if (source.extra_value2 !== undefined && source.extra_value2 !== null) params.extra_value2 = String(source.extra_value2);
                    return toolCall("send_broadcast", params);
                },
                terminal: {
                    info: function() { return toolCall("get_terminal_info", {}); },
                    create: function(sessionName, type) {
                        var params = { session_name: sessionName };
                        if (type !== undefined && type !== null) params.type = String(type);
                        return toolCall("create_terminal_session", params);
                    },
                    exec: function(sessionId, command, timeoutMs) {
                        var params = { session_id: sessionId, command: command };
                        if (timeoutMs !== undefined && timeoutMs !== null) params.timeout_ms = String(timeoutMs);
                        return toolCall("execute_in_terminal_session", params);
                    },
                    execStreaming: function(sessionId, command, options) {
                        var params = { session_id: sessionId, command: command };
                        var toolOptions = {};
                        options = options || {};
                        if (options.timeoutMs !== undefined && options.timeoutMs !== null) params.timeout_ms = String(options.timeoutMs);
                        if (typeof options.onIntermediateResult === "function") toolOptions.onIntermediateResult = options.onIntermediateResult;
                        return toolCall("execute_in_terminal_session_streaming", params, toolOptions);
                    },
                    hiddenExec: function(command, options) {
                        var params = { command: command };
                        options = options || {};
                        if (options.executorKey !== undefined && options.executorKey !== null) params.executor_key = String(options.executorKey);
                        if (options.timeoutMs !== undefined && options.timeoutMs !== null) params.timeout_ms = String(options.timeoutMs);
                        return toolCall("execute_hidden_terminal_command", params);
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
            Chat: {
                _json: function(promise) {
                    return promise.then(function(value) {
                        return typeof value === "string" ? JSON.parse(value) : value;
                    });
                },
                startService: function(options) {
                    var params = {};
                    options = options || {};
                    if (options.initial_mode !== undefined && options.initial_mode !== null && String(options.initial_mode).trim() !== "") params.initial_mode = String(options.initial_mode);
                    if (options.auto_enter_voice_chat !== undefined && options.auto_enter_voice_chat !== null) params.auto_enter_voice_chat = options.auto_enter_voice_chat;
                    if (options.wake_launched !== undefined && options.wake_launched !== null) params.wake_launched = options.wake_launched;
                    if (options.timeout_ms !== undefined && options.timeout_ms !== null) params.timeout_ms = String(options.timeout_ms);
                    if (options.keep_if_exists !== undefined && options.keep_if_exists !== null) params.keep_if_exists = options.keep_if_exists;
                    return Tools.Chat._json(toolCall("start_chat_service", params));
                },
                stopService: function() {
                    return Tools.Chat._json(toolCall("stop_chat_service", {}));
                },
                createNew: function(group, setAsCurrentChat, characterCardId) {
                    var params = {};
                    if (group !== undefined && group !== null && String(group).trim() !== "") params.group = String(group);
                    if (setAsCurrentChat !== undefined && setAsCurrentChat !== null) params.set_as_current_chat = String(setAsCurrentChat);
                    if (characterCardId !== undefined && characterCardId !== null && String(characterCardId).trim() !== "") params.character_card_id = String(characterCardId);
                    return Tools.Chat._json(toolCall("create_new_chat", params));
                },
                listAll: function() {
                    return Tools.Chat._json(toolCall("list_chats", {}));
                },
                listChats: function(params) {
                    return Tools.Chat._json(toolCall("list_chats", params || {}));
                },
                findChat: function(params) {
                    return Tools.Chat._json(toolCall("find_chat", params || {}));
                },
                agentStatus: function(chatId) {
                    return Tools.Chat._json(toolCall("agent_status", { chat_id: chatId }));
                },
                switchTo: function(chatId) {
                    return Tools.Chat._json(toolCall("switch_chat", { chat_id: chatId }));
                },
                updateTitle: function(chatId, title) {
                    return Tools.Chat._json(toolCall("update_chat_title", { chat_id: String(chatId || ""), title: String(title || "") }));
                },
                deleteChat: function(chatId) {
                    return Tools.Chat._json(toolCall("delete_chat", { chat_id: String(chatId || "") }));
                },
                getMessages: function(chatId, options) {
                    var params = { chat_id: String(chatId || "") };
                    options = options || {};
                    if (options.order !== undefined && options.order !== null && String(options.order).trim() !== "") params.order = String(options.order);
                    if (options.limit !== undefined && options.limit !== null) params.limit = String(options.limit);
                    return Tools.Chat._json(toolCall("get_chat_messages", params));
                },
                sendMessage: function(message, chatId, roleCardId, senderName, options) {
                    var params = { message: message };
                    options = options || {};
                    if (chatId) params.chat_id = chatId;
                    if (roleCardId) params.role_card_id = roleCardId;
                    if (senderName) params.sender_name = senderName;
                    if (options.runtime) params.runtime = String(options.runtime);
                    if (options.persist_turn !== undefined) params.persist_turn = options.persist_turn;
                    if (options.notify_reply !== undefined) params.notify_reply = options.notify_reply;
                    if (options.hide_user_message !== undefined) params.hide_user_message = options.hide_user_message;
                    if (options.disable_warning !== undefined) params.disable_warning = options.disable_warning;
                    if (options.timeout_ms !== undefined && options.timeout_ms !== null) params.timeout_ms = String(options.timeout_ms);
                    return Tools.Chat._json(toolCall("send_message_to_ai", params));
                },
                sendMessageStreaming: function(message, chatId, roleCardId, senderName, options) {
                    var params = { message: message };
                    var toolOptions = {};
                    options = options || {};
                    if (chatId) params.chat_id = chatId;
                    if (roleCardId) params.role_card_id = roleCardId;
                    if (senderName) params.sender_name = senderName;
                    if (options.runtime) params.runtime = String(options.runtime);
                    if (options.persist_turn !== undefined) params.persist_turn = options.persist_turn;
                    if (options.notify_reply !== undefined) params.notify_reply = options.notify_reply;
                    if (options.hide_user_message !== undefined) params.hide_user_message = options.hide_user_message;
                    if (options.disable_warning !== undefined) params.disable_warning = options.disable_warning;
                    if (options.timeout_ms !== undefined && options.timeout_ms !== null) params.timeout_ms = String(options.timeout_ms);
                    if (options.waifu !== undefined) params.waifu = options.waifu;
                    if (typeof options.onIntermediateResult === "function") toolOptions.onIntermediateResult = options.onIntermediateResult;
                    return Tools.Chat._json(toolCall("send_message_to_ai_streaming", params, toolOptions));
                },
                listCharacterCards: function() {
                    return Tools.Chat._json(toolCall("list_character_cards", {}));
                }
            },
            Memory: {
                _normalizeOwnerKey: function(targetOwnerKey) {
                    if (targetOwnerKey === undefined || targetOwnerKey === null) return undefined;
                    var normalized = String(targetOwnerKey).trim();
                    return normalized.length > 0 ? normalized : undefined;
                },
                _assignTargetOwnerKey: function(params, options) {
                    var normalizedOwnerKey = Tools.Memory._normalizeOwnerKey(options.targetOwnerKey);
                    if (normalizedOwnerKey !== undefined) params.target_owner_key = normalizedOwnerKey;
                    return params;
                },
                query: function(query, folderPath, limit, startTime, endTime, snapshotId, threshold, targetOwnerKey) {
                    var options = query && typeof query === 'object' && !Array.isArray(query) ? query : { query: query, folderPath: folderPath, limit: limit, startTime: startTime, endTime: endTime, snapshotId: snapshotId, threshold: threshold, targetOwnerKey: targetOwnerKey };
                    var params = { query: options.query };
                    if (options.folderPath) params.folder_path = options.folderPath;
                    if (options.startTime !== undefined) params.start_time = options.startTime;
                    if (options.endTime !== undefined) params.end_time = options.endTime;
                    if (options.limit !== undefined) params.limit = options.limit;
                    if (options.snapshotId !== undefined && options.snapshotId !== null) params.snapshot_id = String(options.snapshotId);
                    if (options.threshold !== undefined) params.threshold = options.threshold;
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("query_memory", params);
                },
                getByTitle: function(title, targetOwnerKey, chunkIndex, chunkRange, query, limit) {
                    var options = title && typeof title === 'object' && !Array.isArray(title) ? title : { title: title, targetOwnerKey: targetOwnerKey, chunkIndex: chunkIndex, chunkRange: chunkRange, query: query, limit: limit };
                    var params = { title: options.title };
                    if (options.chunkIndex !== undefined) params.chunk_index = options.chunkIndex;
                    if (options.chunkRange) params.chunk_range = options.chunkRange;
                    if (options.query) params.query = options.query;
                    if (options.limit !== undefined) params.limit = options.limit;
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("get_memory_by_title", params);
                },
                create: function(title, content, targetOwnerKey, contentType, source, folderPath, tags) {
                    var options = title && typeof title === 'object' && !Array.isArray(title) ? title : { title: title, content: content, targetOwnerKey: targetOwnerKey, contentType: contentType, source: source, folderPath: folderPath, tags: tags };
                    var params = { title: options.title, content: options.content };
                    if (options.contentType) params.content_type = options.contentType;
                    if (options.source) params.source = options.source;
                    if (options.folderPath) params.folder_path = options.folderPath;
                    if (options.tags) params.tags = options.tags;
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("create_memory", params);
                },
                update: function(oldTitle, targetOwnerKey, updates) {
                    var options = oldTitle && typeof oldTitle === 'object' && !Array.isArray(oldTitle) ? oldTitle : Object.assign({ oldTitle: oldTitle, targetOwnerKey: targetOwnerKey }, updates || {});
                    var params = { old_title: options.oldTitle };
                    if (options.newTitle) params.new_title = options.newTitle;
                    if (options.content) params.content = options.content;
                    if (options.contentType) params.content_type = options.contentType;
                    if (options.source) params.source = options.source;
                    if (options.credibility !== undefined) params.credibility = options.credibility;
                    if (options.importance !== undefined) params.importance = options.importance;
                    if (options.folderPath) params.folder_path = options.folderPath;
                    if (options.tags) params.tags = options.tags;
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("update_memory", params);
                },
                updateUserPreferences: function(content, targetOwnerKey) {
                    var options = content && typeof content === 'object' && !Array.isArray(content) ? content : { content: content, targetOwnerKey: targetOwnerKey };
                    var params = { content: String(options.content || '') };
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("update_user_preferences", params);
                },
                deleteMemory: function(title, targetOwnerKey) {
                    var options = title && typeof title === 'object' && !Array.isArray(title) ? title : { title: title, targetOwnerKey: targetOwnerKey };
                    var params = { title: options.title };
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("delete_memory", params);
                },
                move: function(targetFolderPath, targetOwnerKey, titles, sourceFolderPath) {
                    var options = targetFolderPath && typeof targetFolderPath === 'object' && !Array.isArray(targetFolderPath) ? targetFolderPath : { targetFolderPath: targetFolderPath, targetOwnerKey: targetOwnerKey, titles: titles, sourceFolderPath: sourceFolderPath };
                    var params = { target_folder_path: options.targetFolderPath };
                    if (options.titles) params.titles = Array.isArray(options.titles) ? options.titles.join(",") : String(options.titles);
                    if (options.sourceFolderPath !== undefined && options.sourceFolderPath !== null) params.source_folder_path = String(options.sourceFolderPath);
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("move_memory", params);
                },
                link: function(sourceTitle, targetTitle, targetOwnerKey, linkType, weight, description) {
                    var options = sourceTitle && typeof sourceTitle === 'object' && !Array.isArray(sourceTitle) ? sourceTitle : { sourceTitle: sourceTitle, targetTitle: targetTitle, targetOwnerKey: targetOwnerKey, linkType: linkType, weight: weight, description: description };
                    var params = { source_title: options.sourceTitle, target_title: options.targetTitle };
                    if (options.linkType) params.link_type = options.linkType;
                    if (options.weight !== undefined) params.weight = options.weight;
                    if (options.description) params.description = options.description;
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("link_memories", params);
                },
                queryLinks: function(targetOwnerKey, linkId, sourceTitle, targetTitle, linkType, limit) {
                    var options = targetOwnerKey && typeof targetOwnerKey === 'object' && !Array.isArray(targetOwnerKey) ? targetOwnerKey : { targetOwnerKey: targetOwnerKey, linkId: linkId, sourceTitle: sourceTitle, targetTitle: targetTitle, linkType: linkType, limit: limit };
                    var params = {};
                    if (options.linkId !== undefined && options.linkId !== null) params.link_id = options.linkId;
                    if (options.sourceTitle) params.source_title = options.sourceTitle;
                    if (options.targetTitle) params.target_title = options.targetTitle;
                    if (options.linkType) params.link_type = options.linkType;
                    if (options.limit !== undefined) params.limit = options.limit;
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("query_memory_links", params);
                },
                updateLink: function(targetOwnerKey, linkId, sourceTitle, targetTitle, linkType, newLinkType, weight, description) {
                    var options = targetOwnerKey && typeof targetOwnerKey === 'object' && !Array.isArray(targetOwnerKey) ? targetOwnerKey : { targetOwnerKey: targetOwnerKey, linkId: linkId, sourceTitle: sourceTitle, targetTitle: targetTitle, linkType: linkType, newLinkType: newLinkType, weight: weight, description: description };
                    var params = {};
                    if (options.linkId !== undefined && options.linkId !== null) params.link_id = options.linkId;
                    if (options.sourceTitle) params.source_title = options.sourceTitle;
                    if (options.targetTitle) params.target_title = options.targetTitle;
                    if (options.linkType) params.link_type = options.linkType;
                    if (options.newLinkType) params.new_link_type = options.newLinkType;
                    if (options.weight !== undefined) params.weight = options.weight;
                    if (options.description !== undefined) params.description = options.description;
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("update_memory_link", params);
                },
                deleteLink: function(targetOwnerKey, linkId, sourceTitle, targetTitle, linkType) {
                    var options = targetOwnerKey && typeof targetOwnerKey === 'object' && !Array.isArray(targetOwnerKey) ? targetOwnerKey : { targetOwnerKey: targetOwnerKey, linkId: linkId, sourceTitle: sourceTitle, targetTitle: targetTitle, linkType: linkType };
                    var params = {};
                    if (options.linkId !== undefined && options.linkId !== null) params.link_id = options.linkId;
                    if (options.sourceTitle) params.source_title = options.sourceTitle;
                    if (options.targetTitle) params.target_title = options.targetTitle;
                    if (options.linkType) params.link_type = options.linkType;
                    Tools.Memory._assignTargetOwnerKey(params, options);
                    return toolCall("delete_memory_link", params);
                }
            },
        };
    "#
}
