ObjC.import('Foundation');
ObjC.import('ApplicationServices');
var se = Application('System Events');
var count = 0;
var truncated = false;

/** Reads an optional AX attribute without fabricating an absent value. */
function attribute(element, name) {
    var item = element.attributes.byName(name);
    return item.exists() ? item.value() : null;
}

/** Collects one bounded native accessibility subtree. */
function readNode(element, depth) {
    count++;
    var role = attribute(element, 'AXRole');
    var position = attribute(element, 'AXPosition');
    var size = attribute(element, 'AXSize');
    var children = [];
    var nativeChildren = element.uiElements();
    if (depth >= 40 || count >= 4000) truncated = truncated || nativeChildren.length > 0;
    else for (var index = 0; index < nativeChildren.length; index++) {
        if (count >= 4000) { truncated = true; break; }
        children.push(readNode(nativeChildren[index], depth + 1));
    }
    var text = role === 'AXStaticText' || role === 'AXTextField' || role === 'AXTextArea' ? attribute(element, 'AXValue') : attribute(element, 'AXTitle');
    if (attribute(element, 'AXSubrole') === 'AXSecureTextField') text = null;
    return {
        className: role, text: typeof text === 'string' ? text : null,
        contentDesc: attribute(element, 'AXDescription'), resourceId: attribute(element, 'AXIdentifier'),
        bounds: position && size ? '[' + Math.round(position[0]) + ',' + Math.round(position[1]) + '][' + Math.round(position[0] + size[0]) + ',' + Math.round(position[1] + size[1]) + ']' : null,
        isClickable: element.actions.byName('AXPress').exists(), children: children
    };
}

/** Posts one Quartz pointer transition in desktop point coordinates. */
function mouse(kind, x, y) {
    var event = $.CGEventCreateMouseEvent(null, kind, {x: Number(x), y: Number(y)}, 0);
    if (!event) throw Error('Quartz could not create an input event');
    $.CGEventPost(0, event);
    $.CFRelease(event);
}

/** Collects matching AXIdentifiers and rejects an incomplete search. */
function findElements(element, identifier, budget, matches, depth) {
    if (--budget.left < 0 || depth > 40) throw Error('Element search exceeded its traversal budget');
    if (attribute(element, 'AXIdentifier') === identifier) matches.push(element);
    var children = element.uiElements();
    for (var index = 0; index < children.length; index++) {
        findElements(children[index], identifier, budget, matches, depth + 1);
    }
}

/** Executes a structured request against the active macOS process. */
function run() {
    if (!$.AXIsProcessTrusted()) throw Error('Grant Accessibility permission to Operit in System Settings');
    var data = $.NSFileHandle.fileHandleWithStandardInput.readDataToEndOfFile;
    var request = JSON.parse(ObjC.unwrap($.NSString.alloc.initWithDataEncoding(data, $.NSUTF8StringEncoding)));
    var processes = se.applicationProcesses.whose({frontmost: true})();
    if (!processes.length) throw Error('There is no accessible foreground application');
    var process = processes[0];
    var params = request.parameters;
    switch (request.operation) {
        case 'get_page_info': {
            var tree = readNode(process, 0);
            return JSON.stringify({packageName: process.bundleIdentifier(), activityName: process.name(), uiElements: tree, scope: 'desktop', truncated: truncated});
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
                findElements(process, params.resource_id, {left: 4000}, matches, 0);
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
