ObjC.import('Foundation');
ObjC.import('ApplicationServices');
ObjC.import('AppKit');
var se = Application('System Events');
var origin = [0, 0];
var hostedProcessId = null;
var hostedWindowId = null;
var previewProcessId = null;
var previewSourceId = null;
var previewBounds = null;

/** Reads the desktop union in the same top-left point coordinates as AX and Quartz. */
function desktopFrame() {
    var screens = $.NSScreen.screens;
    if (screens.count === 0) throw Error('macOS has no attached screen');
    var top = screens.objectAtIndex(0).frame.size.height;
    var left = 0, upper = 0, right = 0, bottom = 0;
    for (var index = 0; index < screens.count; index++) {
        var frame = screens.objectAtIndex(index).frame;
        var y = top - frame.origin.y - frame.size.height;
        left = Math.min(left, frame.origin.x); upper = Math.min(upper, y);
        right = Math.max(right, frame.origin.x + frame.size.width); bottom = Math.max(bottom, y + frame.size.height);
    }
    return [left, upper, right - left, bottom - upper];
}

/** Runs one fixed native utility and reports its actual exit status. */
function nativeTask(program, args) {
    var task = $.NSTask.alloc.init;
    task.launchPath = program; task.arguments = args;
    task.standardOutput = $.NSFileHandle.fileHandleWithNullDevice;
    task.launch; task.waitUntilExit;
    if (task.terminationStatus !== 0) throw Error(program + ' failed: ' + task.terminationStatus);
}

/** Resolves the WindowServer identity of the exact AX process and frame. */
function captureWindowId(entry, frame) {
    var nativeWindows = ObjC.deepUnwrap($.CGWindowListCopyWindowInfo(0, 0));
    var matches = nativeWindows.filter(function (item) {
        var bounds = item.kCGWindowBounds;
        return item.kCGWindowOwnerPID === entry.process.unixId() && item.kCGWindowLayer === 0 && bounds &&
            Math.round(bounds.X) === frame[0] && Math.round(bounds.Y) === frame[1] &&
            Math.round(bounds.Width) === frame[2] && Math.round(bounds.Height) === frame[3];
    });
    if (matches.length !== 1) throw Error('macOS could not resolve the exact selected capture window; refresh list_windows');
    return matches[0].kCGWindowNumber;
}

/** Captures one native window or desktop rectangle and normalizes Retina pixels to AX points. */
function captureSurface(frame, windowId) {
    if (frame[2] <= 0 || frame[3] <= 0) throw Error('The selected surface has no drawable bounds');
    var path = ObjC.unwrap($.NSTemporaryDirectory()) + 'operit-ui-' + ObjC.unwrap($.NSUUID.UUID.UUIDString) + '.png';
    try {
        var target = windowId === undefined ? ['-R' + frame.join(',')] : ['-l' + windowId, '-o'];
        nativeTask('/usr/sbin/screencapture', ['-x'].concat(target, [path]));
        nativeTask('/usr/bin/sips', ['-z', String(frame[3]), String(frame[2]), path]);
        var data = $.NSData.dataWithContentsOfFile($(path));
        if (!data || data.length === 0) throw Error('macOS did not return a screenshot; check Screen Recording permission');
        return JSON.stringify({pngBase64: ObjC.unwrap(data.base64EncodedStringWithOptions(0))});
    } finally { $.NSFileManager.defaultManager.removeItemAtPathError($(path), null); }
}

/** Reads an optional AX attribute without fabricating an absent value. */
function attribute(element, name) {
    var item = element.attributes.byName(name);
    return item.exists() ? item.value() : null;
}

/** Collects every descendant exposed by one native accessibility window. */
function readNode(element) {
    var role = attribute(element, 'AXRole');
    var position = attribute(element, 'AXPosition');
    var size = attribute(element, 'AXSize');
    var children = [];
    var nativeChildren = element.uiElements();
    for (var index = 0; index < nativeChildren.length; index++) {
        children.push(readNode(nativeChildren[index]));
    }
    var text = role === 'AXStaticText' || role === 'AXTextField' || role === 'AXTextArea' ? attribute(element, 'AXValue') : attribute(element, 'AXTitle');
    if (attribute(element, 'AXSubrole') === 'AXSecureTextField') text = null;
    return {
        className: role, text: typeof text === 'string' ? text : null,
        contentDesc: attribute(element, 'AXDescription'), resourceId: attribute(element, 'AXIdentifier'),
        bounds: position && size ? '[' + Math.round(position[0] - origin[0]) + ',' + Math.round(position[1] - origin[1]) + '][' + Math.round(position[0] + size[0] - origin[0]) + ',' + Math.round(position[1] + size[1] - origin[1]) + ']' : null,
        isClickable: element.actions.byName('AXPress').exists(), children: children,
        properties: {role: role, subrole: attribute(element, 'AXSubrole'), isEnabled: attribute(element, 'AXEnabled'),
            isFocused: attribute(element, 'AXFocused'), value: attribute(element, 'AXSubrole') === 'AXSecureTextField' ? null : attribute(element, 'AXValue')}
    };
}

/** Enumerates native windows using process and window identities, never title matching. */
function windows() {
    var result = [];
    var processes = se.applicationProcesses();
    for (var p = 0; p < processes.length; p++) {
        var nativeWindows = processes[p].windows();
        if (processes[p].unixId() === previewProcessId) {
            if (nativeWindows.length !== 1) throw Error('The native window preview is unavailable');
            var previewPosition = attribute(nativeWindows[0], 'AXPosition');
            var previewSize = attribute(nativeWindows[0], 'AXSize');
            if (!previewPosition || !previewSize) throw Error('The window preview has no accessible bounds');
            previewBounds = '[' + Math.round(previewPosition[0]) + ',' + Math.round(previewPosition[1]) + '][' +
                Math.round(previewPosition[0] + previewSize[0]) + ',' + Math.round(previewPosition[1] + previewSize[1]) + ']';
            continue;
        }
        var axWindows = Ref();
        var appElement = $.AXUIElementCreateApplication(processes[p].unixId());
        var status = $.AXUIElementCopyAttributeValue(appElement, $('AXWindows'), axWindows);
        if (status !== 0) {
            if (nativeWindows.length === 0) continue;
            throw Error('macOS could not read the native window identities: ' + status);
        }
        var references = ObjC.unwrap(axWindows[0]);
        if (references.length !== nativeWindows.length) throw Error('macOS window list changed during discovery; retry list_windows');
        for (var w = 0; w < nativeWindows.length; w++) {
            result.push({id: processes[p].unixId() + ':' + $.CFHash(references[w]), process: processes[p], window: nativeWindows[w]});
        }
    }
    return result;
}

/** Describes a window without reading any of its descendants. */
function describeWindow(entry) {
    var position = attribute(entry.window, 'AXPosition');
    var size = attribute(entry.window, 'AXSize');
    var focused = entry.window.focused() === true;
    return {windowId: entry.id, title: entry.window.name(), packageName: entry.process.bundleIdentifier(),
        bounds: entry.id === previewSourceId ? previewBounds : position && size ? '[' + Math.round(position[0]) + ',' + Math.round(position[1]) + '][' + Math.round(position[0] + size[0]) + ',' + Math.round(position[1] + size[1]) + ']' : null,
        isActive: entry.process.frontmost() && focused, isFocused: entry.process.frontmost() && focused,
        isMinimized: attribute(entry.window, 'AXMinimized') === true};
}

/** Posts one Quartz pointer transition in desktop point coordinates. */
function mouse(kind, x, y) {
    var event = $.CGEventCreateMouseEvent(null, kind, {x: Number(x) + origin[0], y: Number(y) + origin[1]}, 0);
    if (!event) throw Error('Quartz could not create an input event');
    if (hostedProcessId !== null) {
        $.CGEventSetIntegerValueField(event, 91, hostedWindowId);
        $.CGEventSetIntegerValueField(event, 92, hostedWindowId);
        $.CGEventPostToPid(hostedProcessId, event);
    } else $.CGEventPost(0, event);
    $.CFRelease(event);
}

/** Collects matching AXIdentifiers and rejects an incomplete search. */
function findElements(element, identifier, matches) {
    if (attribute(element, 'AXIdentifier') === identifier) matches.push(element);
    var children = element.uiElements();
    for (var index = 0; index < children.length; index++) {
        findElements(children[index], identifier, matches);
    }
}

/** Executes a structured request against the active macOS process. */
function run() {
    var data = $.NSFileHandle.fileHandleWithStandardInput.readDataToEndOfFile;
    var request = JSON.parse(ObjC.unwrap($.NSString.alloc.initWithDataEncoding(data, $.NSUTF8StringEncoding)));
    var params = request.parameters;
    previewProcessId = params.preview_process_id === undefined ? null : params.preview_process_id;
    previewSourceId = params.preview_window_id === undefined ? null : params.preview_window_id;
    var desktop = desktopFrame();
    if (request.operation === 'capture_screenshot' && params.window_id === undefined) return captureSurface(desktop);
    if (!$.AXIsProcessTrusted()) throw Error('Grant Accessibility permission to Operit in System Settings');
    var entries = windows();
    if (request.operation === 'list_windows') return JSON.stringify(entries.map(describeWindow));
    var selected = entries.filter(function (entry) {
        return params.window_id === undefined ? describeWindow(entry).isActive : entry.id === params.window_id;
    });
    if (selected.length !== 1) throw Error('The selected native window is unavailable; refresh list_windows');
    var entry = selected[0];
    var process = entry.process;
    var window = entry.window;
    var position = attribute(window, 'AXPosition'), size = attribute(window, 'AXSize');
    var selectedWindow = params.window_id !== undefined;
    if (selectedWindow && (!position || !size)) throw Error('The selected window has no accessible bounds');
    var frame = selectedWindow ? [Math.round(position[0]), Math.round(position[1]), Math.round(size[0]), Math.round(size[1])] : desktop;
    origin = [frame[0], frame[1]];
    if (request.operation !== 'focus_window' && selectedWindow && attribute(window, 'AXMinimized') === true) throw Error('Focus the minimized window before inspecting or operating it');
    if (selectedWindow && !params.hosted_window && request.operation !== 'focus_window' && request.operation !== 'get_page_info' && !describeWindow(entry).isActive) throw Error('Focus the selected window before capturing or sending input');
    if (params.hosted_window && request.operation !== 'focus_window') {
        hostedProcessId = process.unixId();
        hostedWindowId = captureWindowId(entry, frame);
    }
    [['x', 'y'], ['start_x', 'start_y'], ['end_x', 'end_y']].forEach(function (keys) {
        if (selectedWindow && params[keys[0]] !== undefined && (params[keys[0]] < 0 || params[keys[1]] < 0 || params[keys[0]] >= frame[2] || params[keys[1]] >= frame[3])) throw Error('Input coordinate is outside the selected window');
    });
    switch (request.operation) {
        case 'capture_screenshot': return captureSurface(frame, captureWindowId(entry, frame));
        case 'focus_window': {
            if (attribute(window, 'AXMinimized') === true) window.attributes.byName('AXMinimized').value = false;
            if (params.hosted_window) break;
            process.frontmost = true;
            window.actions.byName('AXRaise').perform();
            var deadline = Date.now() + 2000;
            while (!describeWindow(entry).isActive) {
                if (Date.now() >= deadline) throw Error('macOS did not activate the selected window');
                delay(0.02);
            }
            break;
        }
        case 'get_page_info': {
            var tree = readNode(window);
            return JSON.stringify({windowId: entry.id, packageName: process.bundleIdentifier(), activityName: window.name(), uiElements: tree, scope: selectedWindow ? 'window' : 'desktop', truncated: false});
        }
        case 'tap': mouse(1, params.x, params.y); mouse(2, params.x, params.y); break;
        case 'long_press':
            mouse(1, params.x, params.y);
            try { delay(0.6); } finally { mouse(2, params.x, params.y); }
            break;
        case 'swipe':
            mouse(1, params.start_x, params.start_y);
            try {
                for (var step = 1; step <= 30; step++) {
                    mouse(6, params.start_x + (params.end_x - params.start_x) * step / 30, params.start_y + (params.end_y - params.start_y) * step / 30);
                    delay(params.duration / 30000);
                }
            } finally { mouse(2, params.end_x, params.end_y); }
            break;
        case 'set_input_text': {
            var element;
            if (params.resource_id === undefined) element = attribute(process, 'AXFocusedUIElement');
            else {
                var matches = [];
                findElements(window, params.resource_id, matches);
                if (matches.length !== 1) throw Error('Select one unique input resource ID');
                element = matches[0];
            }
            if (!element) throw Error('No matching editable accessible element');
            var value = element.attributes.byName('AXValue');
            if (!value.exists() || !value.settable()) throw Error('The selected accessibility value is not editable');
            value.value = params.text;
            break;
        }
        case 'press_key': {
            var keys = {ENTER: 36, TAB: 48, ESCAPE: 53, BACKSPACE: 51, DELETE: 117, HOME: 115, END: 119, UP: 126, DOWN: 125, LEFT: 123, RIGHT: 124};
            var key = keys[String(params.key_code).toUpperCase()];
            if (key === undefined) throw Error('Unsupported UI key');
            if (hostedProcessId !== null) {
                [true, false].forEach(function (down) {
                    var event = $.CGEventCreateKeyboardEvent(null, key, down);
                    if (!event) throw Error('Quartz could not create a hosted key event');
                    $.CGEventPostToPid(hostedProcessId, event); $.CFRelease(event);
                });
            } else se.keyCode(key);
            break;
        }
        default: throw Error('Unknown UI operation: ' + request.operation);
    }
    return '{}';
}
