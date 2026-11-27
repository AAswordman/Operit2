/* METADATA
{
    "api_version": "2.0.0",
    "name": "browser",
    "display_name": {
        "zh": "Browser 自动化操作",
        "en": "Browser Automation"
    },
    "description": {
        "zh": "严格对齐 Playwright MCP 默认 browser 工具面的浏览器自动化工具集。",
        "en": "Browser automation tools aligned to the default Playwright MCP browser surface."
    },
    "enabledByDefault": true,
    "category": "Automatic",
    "tools": [
        {
            "name": "click",
            "display": {"label": {"zh": "点击网页元素", "en": "Click element"}, "icon": "web", "targetParameter": "element"},
            "description": { "zh": "按快照 ref 点击页面元素，包括同源 iframe 内的元素。", "en": "Click an element by snapshot ref, including refs inside same-origin iframes." },
            "parameters": [
                { "name": "ref", "description": { "zh": "快照中的目标元素引用；和 selector 至少提供一个。", "en": "Target element ref from the snapshot; provide ref or selector." }, "type": "string", "required": false },
                { "name": "selector", "description": { "zh": "可选，元素选择器。", "en": "Optional element selector." }, "type": "string", "required": false },
                { "name": "element", "description": { "zh": "可选，人类可读元素描述。", "en": "Optional human-readable element description." }, "type": "string", "required": false },
                { "name": "doubleClick", "description": { "zh": "可选，是否双击。", "en": "Optional double click." }, "type": "boolean", "required": false },
                { "name": "button", "description": { "zh": "可选，left/right/middle。", "en": "Optional mouse button: left/right/middle." }, "type": "string", "required": false },
                { "name": "modifiers", "description": { "zh": "可选，修饰键数组。", "en": "Optional modifier keys array." }, "type": "array", "required": false }
            ]
        },
        {
            "name": "close",
            "display": {"label": {"zh": "关闭标签页", "en": "Close tab"}, "icon": "web"},
            "description": { "zh": "关闭当前 tab。", "en": "Close the current tab." },
            "parameters": []
        },
        {
            "name": "close_all",
            "display": {"label": {"zh": "关闭所有标签页", "en": "Close all tabs"}, "icon": "web"},
            "description": { "zh": "关闭全部 tab。", "en": "Close all tabs." },
            "parameters": []
        },
        {
            "name": "console_messages",
            "display": {"label": {"zh": "读取控制台消息", "en": "Read console messages"}, "icon": "web", "targetParameter": "level"},
            "description": { "zh": "读取控制台消息。", "en": "Read console messages." },
            "parameters": [
                { "name": "level", "description": { "zh": "可选，日志级别：error/warning/info/debug，默认 info。", "en": "Optional log level: error/warning/info/debug. Defaults to info." }, "type": "string", "required": false },
                { "name": "filename", "description": { "zh": "可选，保存输出的文件名。", "en": "Optional output file name." }, "type": "string", "required": false }
            ]
        },
        {
            "name": "drag",
            "display": {"label": {"zh": "拖拽网页元素", "en": "Drag element"}, "icon": "web", "targetParameter": "startElement"},
            "description": { "zh": "在两个元素之间拖拽。", "en": "Drag between two elements." },
            "parameters": [
                { "name": "startElement", "description": { "zh": "源元素的人类可读描述。", "en": "Human-readable source element description." }, "type": "string", "required": true },
                { "name": "startRef", "description": { "zh": "源元素 ref。", "en": "Source element ref." }, "type": "string", "required": true },
                { "name": "endElement", "description": { "zh": "目标元素的人类可读描述。", "en": "Human-readable target element description." }, "type": "string", "required": true },
                { "name": "endRef", "description": { "zh": "目标元素 ref。", "en": "Target element ref." }, "type": "string", "required": true }
            ]
        },
        {
            "name": "evaluate",
            "display": {"label": {"zh": "执行网页脚本", "en": "Evaluate page script"}, "icon": "web", "targetParameter": "element"},
            "description": { "zh": "在页面或元素上执行 JavaScript 函数。", "en": "Evaluate a JavaScript function on the page or an element." },
            "parameters": [
                { "name": "function", "description": { "zh": "要执行的函数源码。", "en": "Function source to execute." }, "type": "string", "required": true },
                { "name": "element", "description": { "zh": "可选，人类可读元素描述。", "en": "Optional human-readable element description." }, "type": "string", "required": false },
                { "name": "ref", "description": { "zh": "可选，目标元素 ref。", "en": "Optional target element ref." }, "type": "string", "required": false }
            ]
        },
        {
            "name": "upload",
            "display": {"label": {"zh": "上传文件", "en": "Upload files"}, "icon": "file"},
            "description": { "zh": "向点击或执行脚本打开的当前文件选择器上传文件；支持单文件、多文件及取消，上传失败保留选择器。", "en": "Upload files to the chooser opened by a click or page script. Supports single/multiple files and cancellation; failed uploads preserve the chooser." },
            "parameters": [
                { "name": "paths", "description": { "zh": "可选，文件路径数组；Flutter 浏览器支持 VFS 路径。省略表示取消并保留已选文件，空数组表示清空。多个文件要求选择器支持 multiple。", "en": "Optional file paths; the Flutter browser supports VFS paths. Omission cancels without changing selected files; an empty array clears the selection. Multiple files require a multiple chooser." }, "type": "array", "required": false }
            ]
        },
        {
            "name": "fill_form",
            "display": {"label": {"zh": "填写网页表单", "en": "Fill form"}, "icon": "web"},
            "description": { "zh": "批量填写表单字段。", "en": "Fill multiple form fields." },
            "parameters": [
                { "name": "fields", "description": { "zh": "字段数组。", "en": "Array of form fields." }, "type": "array", "required": true }
            ]
        },
        {
            "name": "dialog",
            "display": {"label": {"zh": "处理网页弹窗", "en": "Handle dialog"}, "icon": "web"},
            "description": { "zh": "查看、接受或取消当前标签页的 alert、confirm、prompt。get 在没有弹窗时返回 dialog: null。accept 可为 prompt 提供文本，dismiss 取消输入。", "en": "Inspect, accept, or dismiss an alert, confirm, or prompt in the current tab. get returns dialog: null when no dialog is open. accept can supply prompt text; dismiss cancels input." },
            "parameters": [
                { "name": "action", "description": { "zh": "get 查看当前弹窗；accept 确认；dismiss 取消。", "en": "get inspects the current dialog; accept confirms; dismiss cancels." }, "type": "string", "required": true },
                { "name": "promptText", "description": { "zh": "可选，prompt 的输入文本。", "en": "Optional prompt text." }, "type": "string", "required": false }
            ]
        },
        {
            "name": "hover",
            "display": {"label": {"zh": "悬停网页元素", "en": "Hover element"}, "icon": "web", "targetParameter": "element"},
            "description": { "zh": "悬停到页面元素上。", "en": "Hover over an element." },
            "parameters": [
                { "name": "ref", "description": { "zh": "目标元素 ref。", "en": "Target element ref." }, "type": "string", "required": true },
                { "name": "element", "description": { "zh": "可选，人类可读元素描述。", "en": "Optional human-readable element description." }, "type": "string", "required": false }
            ]
        },
        {
            "name": "goto",
            "display": {"label": {"zh": "打开网页", "en": "Open page"}, "icon": "web", "targetParameter": "url"},
            "description": { "zh": "导航到指定 URL。", "en": "Navigate to a URL." },
            "parameters": [
                { "name": "url", "description": { "zh": "目标 URL。", "en": "Target URL." }, "type": "string", "required": true }
            ]
        },
        {
            "name": "back",
            "display": {"label": {"zh": "返回上一页", "en": "Go back"}, "icon": "web"},
            "description": { "zh": "后退到上一页。", "en": "Go back to the previous page." },
            "parameters": []
        },
        {
            "name": "network_requests",
            "display": {"label": {"zh": "读取网络请求", "en": "Read network requests"}, "icon": "web"},
            "description": { "zh": "读取当前页面的网络请求。", "en": "Read network requests for the current page." },
            "parameters": [
                { "name": "includeStatic", "description": { "zh": "可选，是否包含静态资源请求，默认 false。", "en": "Optional include static resource requests. Defaults to false." }, "type": "boolean", "required": false },
                { "name": "filename", "description": { "zh": "可选，保存输出的文件名。", "en": "Optional output file name." }, "type": "string", "required": false }
            ]
        },
        {
            "name": "press_key",
            "display": {"label": {"zh": "发送浏览器按键", "en": "Press key"}, "icon": "web", "targetParameter": "key"},
            "description": { "zh": "按下键盘按键。", "en": "Press a keyboard key." },
            "parameters": [
                { "name": "key", "description": { "zh": "按键名。", "en": "Key name." }, "type": "string", "required": true }
            ]
        },
        {
            "name": "resize",
            "display": {"label": {"zh": "调整浏览器窗口", "en": "Resize browser"}, "icon": "web"},
            "description": { "zh": "调整浏览器视口大小。", "en": "Resize the browser viewport." },
            "parameters": [
                { "name": "width", "description": { "zh": "宽度。", "en": "Width." }, "type": "number", "required": true },
                { "name": "height", "description": { "zh": "高度。", "en": "Height." }, "type": "number", "required": true }
            ]
        },
        {
            "name": "run_code",
            "display": {"label": {"zh": "运行浏览器代码", "en": "Run browser code"}, "icon": "web"},
            "description": { "zh": "在当前页面执行异步 JavaScript 函数体；使用 return 返回结果，支持 await。", "en": "Run an async JavaScript function body in the current page; use return for a result and await for asynchronous work." },
            "parameters": [
                { "name": "code", "description": { "zh": "异步 JavaScript 函数体。", "en": "Async JavaScript function body." }, "type": "string", "required": true }
            ]
        },
        {
            "name": "select_option",
            "display": {"label": {"zh": "选择网页选项", "en": "Select option"}, "icon": "web", "targetParameter": "element"},
            "description": { "zh": "在下拉框中选择选项。", "en": "Select options in a dropdown." },
            "parameters": [
                { "name": "ref", "description": { "zh": "目标元素 ref。", "en": "Target element ref." }, "type": "string", "required": true },
                { "name": "values", "description": { "zh": "要选择的值数组。", "en": "Values to select." }, "type": "array", "required": true },
                { "name": "element", "description": { "zh": "可选，人类可读元素描述。", "en": "Optional human-readable element description." }, "type": "string", "required": false }
            ]
        },
        {
            "name": "snapshot",
            "display": {"label": {"zh": "读取网页快照", "en": "Read page snapshot"}, "icon": "web", "targetParameter": "selector"},
            "description": { "zh": "获取当前页面结构化快照，包括同源 iframe 内容。", "en": "Get a structured page snapshot, including same-origin iframe content." },
            "parameters": [
                { "name": "filename", "description": { "zh": "可选，保存快照的文件名。", "en": "Optional snapshot output file name." }, "type": "string", "required": false },
                { "name": "selector", "description": { "zh": "可选，作为快照根节点的元素选择器。", "en": "Optional root element selector for a partial snapshot." }, "type": "string", "required": false },
                { "name": "depth", "description": { "zh": "可选，限制快照树深度。", "en": "Optional snapshot tree depth limit." }, "type": "number", "required": false }
            ]
        },
        {
            "name": "take_screenshot",
            "display": {"label": {"zh": "截取网页图片", "en": "Take screenshot"}, "icon": "web"},
            "description": {"zh": "截取当前页面或指定元素。Windows 工作区浏览器支持 PNG 视口截图；其他工作区浏览器暂不支持。", "en": "Capture the current page or an element. The Windows workspace browser supports PNG viewport screenshots; other workspace browsers do not yet support this operation."},
            "parameters": [
                {"name": "type", "description": {"zh": "图片格式：png 或 jpeg。", "en": "Image format: png or jpeg."}, "type": "string", "required": false},
                {"name": "filename", "description": {"zh": "可选输出文件名。", "en": "Optional output file name."}, "type": "string", "required": false},
                {"name": "ref", "description": {"zh": "可选元素引用。", "en": "Optional element reference."}, "type": "string", "required": false},
                {"name": "element", "description": {"zh": "可选元素描述。", "en": "Optional element description."}, "type": "string", "required": false},
                {"name": "fullPage", "description": {"zh": "是否截取完整页面。", "en": "Capture the full page."}, "type": "boolean", "required": false}
            ]
        },
        {
            "name": "type",
            "display": {"label": {"zh": "输入网页文本", "en": "Type text"}, "icon": "web", "targetParameter": "element"},
            "description": { "zh": "向可编辑元素输入文本。", "en": "Type text into an editable element." },
            "parameters": [
                { "name": "ref", "description": { "zh": "目标元素 ref。", "en": "Target element ref." }, "type": "string", "required": true },
                { "name": "text", "description": { "zh": "输入文本。", "en": "Text to type." }, "type": "string", "required": true },
                { "name": "element", "description": { "zh": "可选，人类可读元素描述。", "en": "Optional human-readable element description." }, "type": "string", "required": false },
                { "name": "submit", "description": { "zh": "可选，输入后是否提交。", "en": "Optional submit after typing." }, "type": "boolean", "required": false },
                { "name": "slowly", "description": { "zh": "可选，是否逐字输入。", "en": "Optional type slowly." }, "type": "boolean", "required": false }
            ]
        },
        {
            "name": "wait_for",
            "display": {"label": {"zh": "等待网页状态", "en": "Wait for page"}, "icon": "web", "targetParameter": "text"},
            "description": { "zh": "等待文本出现、消失或等待指定时间。", "en": "Wait for text to appear, disappear, or for a duration." },
            "parameters": [
                { "name": "time", "description": { "zh": "可选，等待秒数。", "en": "Optional number of seconds to wait." }, "type": "number", "required": false },
                { "name": "text", "description": { "zh": "可选，等待出现的文本。", "en": "Optional text to wait for." }, "type": "string", "required": false },
                { "name": "textGone", "description": { "zh": "可选，等待消失的文本。", "en": "Optional text to wait to disappear." }, "type": "string", "required": false }
            ]
        },
        {
            "name": "tabs",
            "display": {"label": {"zh": "管理标签页", "en": "Manage tabs"}, "icon": "web", "targetParameter": "action"},
            "description": { "zh": "列出、创建、切换或关闭 tab。", "en": "List, create, select, or close tabs." },
            "parameters": [
                { "name": "action", "description": { "zh": "操作：list/create/select/close。", "en": "Action: list/create/select/close." }, "type": "string", "required": true },
                { "name": "index", "description": { "zh": "可选，0-based tab 索引。", "en": "Optional 0-based tab index." }, "type": "number", "required": false }
            ]
        }
    ]
}*/

const MAX_INLINE_BROWSER_TEXT_CHARS = 24000;
type BrowserTabAction = "list" | "create" | "select" | "close";
type BrowserMouseButton = "left" | "right" | "middle";
type BrowserModifierKey = "Alt" | "Control" | "ControlOrMeta" | "Meta" | "Shift";
type ToolParamValue = string | number | boolean | object;
type BrowserToolResult = string | { value: string; toString(): string };

interface FilenamePayload {
    filename?: string;
}

interface SnapshotPayload extends FilenamePayload {
    selector?: string;
    depth?: number;
}

interface ScreenshotPayload extends FilenamePayload {
    type?: "png" | "jpeg";
    element?: string;
    ref?: string;
    fullPage?: boolean;
}

interface ClickPayload {
    ref?: string;
    selector?: string;
    element?: string;
    button?: BrowserMouseButton;
    modifiers?: BrowserModifierKey[];
    doubleClick?: boolean;
}

interface ConsoleMessagesPayload extends FilenamePayload {
    level: string;
}

interface EvaluatePayload {
    function: string;
    ref?: string;
    element?: string;
}

interface UploadPayload {
    paths?: string[];
}

interface FillFormFieldPayload {
    name: string;
    type: string;
    value: ToolParamValue;
    ref?: string;
    selector?: string;
}

interface FillFormPayload {
    fields: FillFormFieldPayload[];
}

interface DialogPayload {
    action: "get" | "accept" | "dismiss";
    promptText?: string;
}

interface HoverPayload {
    ref: string;
    element?: string;
}

interface DragPayload {
    startElement: string;
    startRef: string;
    endElement: string;
    endRef: string;
}

interface GotoPayload {
    url: string;
}

interface NetworkRequestsPayload extends FilenamePayload {
    includeStatic?: boolean;
}

interface PressKeyPayload {
    key: string;
}

interface ResizePayload {
    width: number;
    height: number;
}

interface RunCodePayload {
    code: string;
}

interface SelectOptionPayload {
    ref: string;
    values: string[];
    element?: string;
}

interface TypePayload {
    ref: string;
    text: string;
    element?: string;
    submit?: boolean;
    slowly?: boolean;
}

interface WaitForPayload {
    time?: number;
    text?: string;
    textGone?: string;
}

interface TabsPayload {
    action: BrowserTabAction;
    index?: number;
}

const TOOL_NAMES = [
    "click",
    "close",
    "close_all",
    "console_messages",
    "drag",
    "evaluate",
    "upload",
    "fill_form",
    "dialog",
    "hover",
    "goto",
    "back",
    "network_requests",
    "press_key",
    "resize",
    "run_code",
    "select_option",
    "snapshot",
    "take_screenshot",
    "type",
    "wait_for",
    "tabs"
];

function normalizeOptionalString(value?: string): string | undefined {
    if (value === undefined) {
        return undefined;
    }
    const normalized = value.trim();
    return normalized ? normalized : undefined;
}

function buildLargeOutputFilename(prefix: string, extension: string): string {
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
    const rand = Math.floor(Math.random() * 1000000);
    return OPERIT_CLEAN_ON_EXIT_DIR + "/browser_" + prefix + "_" + timestamp + "_" + rand + "." + extension;
}

function browserResultText(result: BrowserToolResult): string {
    return typeof result === "string" ? result : result.value;
}

async function maybePersistLargeBrowserResponse(
    result: BrowserToolResult,
    prefix: string,
    extension: string = "md"
) {
    const text = browserResultText(result);
    if (text.length <= MAX_INLINE_BROWSER_TEXT_CHARS) {
        return result;
    }
    await Tools.Files.mkdir(OPERIT_CLEAN_ON_EXIT_DIR, true);
    const filename = buildLargeOutputFilename(prefix, extension);
    await Tools.Files.write(filename, text, false);
    const normalizedPath = filename.replace(/\\/g, "/");
    return "Large browser response saved to:\n- [Browser Output](" + normalizedPath + ")";
}

async function click(params: ClickPayload) {
    const result = await Tools.Net.browserClick(params);
    return maybePersistLargeBrowserResponse(result, "click");
}

async function close() {
    const result = await Tools.Net.browserClose({});
    return maybePersistLargeBrowserResponse(result, "close");
}

async function close_all() {
    const result = await Tools.Net.browserCloseAll({});
    return maybePersistLargeBrowserResponse(result, "close_all");
}

async function console_messages(params: Partial<ConsoleMessagesPayload> = {}) {
    const result = await Tools.Net.browserConsoleMessages(params);
    return maybePersistLargeBrowserResponse(result, "console_messages");
}

async function drag(params: DragPayload) {
    const result = await Tools.Net.browserDrag(params);
    return maybePersistLargeBrowserResponse(result, "drag");
}

async function evaluate(params: EvaluatePayload) {
    const result = await Tools.Net.browserEvaluate(params);
    return maybePersistLargeBrowserResponse(result, "evaluate");
}

/** Uploads host-backed files into the active chooser or explicitly cancels it. */
async function upload(params: UploadPayload = {}) {
    const result = await Tools.Net.browserFileUpload(params);
    return maybePersistLargeBrowserResponse(result, "upload");
}

function normalizeFormFields(fields: FillFormFieldPayload[]): FillFormFieldPayload[] {
    if (fields.length === 0) {
        throw new Error("fields must be a non-empty array");
    }
    return fields.map((field, index) => {
        const normalized: FillFormFieldPayload = {
            name: field.name.trim(),
            type: field.type.trim(),
            value: field.value as ToolParamValue
        };
        if (!normalized.name) {
            throw new Error("fields[" + index + "].name is required");
        }
        if (!normalized.type) {
            throw new Error("fields[" + index + "].type is required");
        }
        const ref = normalizeOptionalString(field.ref);
        const selector = normalizeOptionalString(field.selector);
        if (!ref && !selector) {
            throw new Error("fields[" + index + "] requires ref or selector");
        }
        if (ref) {
            normalized.ref = ref;
        }
        if (selector) {
            normalized.selector = selector;
        }
        return normalized;
    });
}

async function fill_form(params: FillFormPayload) {
    const payload: FillFormPayload = {
        fields: normalizeFormFields(params.fields)
    };
    const result = await Tools.Net.browserFillForm(payload);
    return maybePersistLargeBrowserResponse(result, "fill_form");
}

/** Inspects or resolves the active page dialog through the owning browser host. */
async function dialog(params: DialogPayload) {
    const result = await Tools.Net.browserDialog(params);
    return maybePersistLargeBrowserResponse(result, "dialog");
}

async function hover(params: HoverPayload) {
    const result = await Tools.Net.browserHover(params);
    return maybePersistLargeBrowserResponse(result, "hover");
}

async function goto(params: GotoPayload) {
    const result = await Tools.Net.browserNavigate(params);
    const text = browserResultText(result);
    if (text.length > MAX_INLINE_BROWSER_TEXT_CHARS) {
        return maybePersistLargeBrowserResponse(result, "goto");
    }
    return JSON.parse(text);
}

async function back() {
    const result = await Tools.Net.browserNavigateBack({});
    return maybePersistLargeBrowserResponse(result, "back");
}

async function network_requests(params: NetworkRequestsPayload = {}) {
    const result = await Tools.Net.browserNetworkRequests(params);
    return maybePersistLargeBrowserResponse(result, "network_requests");
}

async function press_key(params: PressKeyPayload) {
    const result = await Tools.Net.browserPressKey(params);
    return maybePersistLargeBrowserResponse(result, "press_key");
}

async function resize(params: ResizePayload) {
    const result = await Tools.Net.browserResize(params);
    return maybePersistLargeBrowserResponse(result, "resize");
}

async function run_code(params: RunCodePayload) {
    const result = await Tools.Net.browserRunCode(params);
    return maybePersistLargeBrowserResponse(result, "run_code");
}

async function select_option(params: SelectOptionPayload) {
    const result = await Tools.Net.browserSelectOption(params);
    return maybePersistLargeBrowserResponse(result, "select_option");
}

async function snapshot(params: SnapshotPayload = {}) {
    const result = await Tools.Net.browserSnapshot(params);
    return maybePersistLargeBrowserResponse(result, "snapshot");
}

/** Captures the current page or a referenced element through the browser host. */
async function take_screenshot(params: ScreenshotPayload = {}) {
    const { filename, ...options } = params;
    const result = browserResultText(await Tools.Net.browserTakeScreenshot(options));
    const match = /^data:image\/(png|jpeg);base64,([A-Za-z0-9+/=]+)$/.exec(result);
    if (!match) {
        throw new Error("Browser screenshot did not return image data");
    }
    const path = normalizeOptionalString(filename) || buildLargeOutputFilename("screenshot", match[1] === "jpeg" ? "jpg" : "png");
    if (!normalizeOptionalString(filename)) {
        await Tools.Files.mkdir(OPERIT_CLEAN_ON_EXIT_DIR, true);
    }
    await Tools.Files.writeBinary(path, match[2]);
    const image = await Tools.Media.registerFromPath({ type: "image", path });
    return { file_path: path, image_link: image.link };
}

async function type(params: TypePayload) {
    const result = await Tools.Net.browserType(params);
    return maybePersistLargeBrowserResponse(result, "type");
}

async function wait_for(params: WaitForPayload = {}) {
    const result = await Tools.Net.browserWaitFor(params);
    return maybePersistLargeBrowserResponse(result, "wait_for");
}

async function tabs(params: TabsPayload) {
    const result = await Tools.Net.browserTabs(params);
    return maybePersistLargeBrowserResponse(result, "tabs");
}

async function browserMain() {
    return "Browser package ready: " + TOOL_NAMES.join(", ");
}

exports.click = click;
exports.close = close;
exports.close_all = close_all;
exports.console_messages = console_messages;
exports.drag = drag;
exports.evaluate = evaluate;
exports.upload = upload;
exports.fill_form = fill_form;
exports.dialog = dialog;
exports.hover = hover;
exports.goto = goto;
exports.back = back;
exports.network_requests = network_requests;
exports.press_key = press_key;
exports.resize = resize;
exports.run_code = run_code;
exports.select_option = select_option;
exports.snapshot = snapshot;
exports.take_screenshot = take_screenshot;
exports.type = type;
exports.wait_for = wait_for;
exports.tabs = tabs;
exports.main = browserMain;
