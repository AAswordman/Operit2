ObjC.import('Foundation');
ObjC.import('ApplicationServices');
var se = Application('System Events');

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
        bounds: position && size ? '[' + Math.round(position[0]) + ',' + Math.round(position[1]) + '][' + Math.round(position[0] + size[0]) + ',' + Math.round(position[1] + size[1]) + ']' : null,
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
        bounds: position && size ? '[' + Math.round(position[0]) + ',' + Math.round(position[1]) + '][' + Math.round(position[0] + size[0]) + ',' + Math.round(position[1] + size[1]) + ']' : null,
        isActive: entry.process.frontmost() && focused, isFocused: entry.process.frontmost() && focused,
        isMinimized: attribute(entry.window, 'AXMinimized') === true};
}

/** Posts one Quartz pointer transition in desktop point coordinates. */
function mouse(kind, x, y) {
    var event = $.CGEventCreateMouseEvent(null, kind, {x: Number(x), y: Number(y)}, 0);
    if (!event) throw Error('Quartz could not create an input event');
    $.CGEventPost(0, event);
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
    if (!$.AXIsProcessTrusted()) throw Error('Grant Accessibility permission to Operit in System Settings');
    var data = $.NSFileHandle.fileHandleWithStandardInput.readDataToEndOfFile;
    var request = JSON.parse(ObjC.unwrap($.NSString.alloc.initWithDataEncoding(data, $.NSUTF8StringEncoding)));
    var params = request.parameters;
    var entries = windows();
    if (request.operation === 'list_windows') return JSON.stringify(entries.map(describeWindow));
    var selected = entries.filter(function (entry) {
        return params.window_id === undefined ? describeWindow(entry).isActive : entry.id === params.window_id;
    });
    if (selected.length !== 1) throw Error('The selected native window is unavailable; refresh list_windows');
    var entry = selected[0];
    var process = entry.process;
    var window = entry.window;
    switch (request.operation) {
        case 'focus_window': {
            if (attribute(window, 'AXMinimized') === true) window.attributes.byName('AXMinimized').value = false;
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
            return JSON.stringify({windowId: entry.id, packageName: process.bundleIdentifier(), activityName: window.name(), uiElements: tree, scope: 'desktop', truncated: false});
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
            se.keyCode(key); break;
        }
        default: throw Error('Unknown UI operation: ' + request.operation);
    }
    return '{}';
}
