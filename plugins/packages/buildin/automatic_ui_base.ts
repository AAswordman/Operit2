/* METADATA
{
    "name": "Automatic_ui_base",
    "display_name": {
        "zh": "自动化基础工具",
        "en": "Automation Base Tools"
    },
    "description": {
        "zh": "操作设备系统界面和其他应用的基础 UI 自动化工具。默认关闭；需要平台提供整机自动化权限。",
        "en": "Basic UI automation for the device system interface and other apps. Disabled by default; requires a whole-device platform provider."
    },
    "category": "Automatic",
    "enabledByDefault": false,
    "tools": [
        {
            "name": "usage_advice",
            "description": {
                "zh": "先用 list_windows 发现系统窗口及其 windowId，再用 focus_window 聚焦目标窗口，用 get_page_info(window_id) 获取基于完整原生树生成的精简文本摘要。窗口 ID 只对当前系统会话有效，窗口关闭后须重新列出。根据文本、资源标识、内容描述或边界定位元素；用 index 选择同名元素，partialMatch 显式启用部分匹配。坐标使用树中的屏幕坐标系。操作后重新取摘要确认结果。未授权或平台不支持时报告实际错误。",
                "en": "Discover system windows with list_windows, focus the target windowId with focus_window, then read a compact text summary of its complete native tree with get_page_info(window_id). Window IDs belong to the current session; refresh discovery after a window closes. Target elements by text, resource ID, description or bounds; use index for duplicates and partialMatch for explicit substring matching. Use the tree's screen coordinates and read a fresh summary after input. Report permission or platform errors."
            },
            "parameters": [],
            "advice": true,
            "display": {
                "label": {
                    "zh": "查看自动化使用建议",
                    "en": "Read automation guidance"
                },
                "icon": "info"
            }
        },
        {
            "name": "app_launch",
            "description": {
                "zh": "根据设备应用标识启动应用，失败时返回实际错误。",
                "en": "Launch an app by its device application identifier and report the actual failure."
            },
            "parameters": [
                {
                    "name": "package_name",
                    "description": {
                        "zh": "系统应用列表返回的应用标识，例如安卓包名。",
                        "en": "Application identifier from the system app list, such as an Android package name."
                    },
                    "type": "string",
                    "required": true
                }
            ],
            "display": {
                "label": {
                    "zh": "启动应用",
                    "en": "Launch app"
                },
                "icon": "package",
                "targetParameter": "package_name"
            }
        },
        {
            "name": "list_windows",
            "description": {
                "zh": "列出系统原生窗口，包含 windowId、标题、应用、边界、活动、焦点和最小化状态。",
                "en": "List native system windows with windowId, title, application, bounds, active, focused and minimized state."
            },
            "parameters": [],
            "display": { "label": { "zh": "列出系统窗口", "en": "List system windows" }, "icon": "screen" }
        },
        {
            "name": "focus_window",
            "description": {
                "zh": "按 list_windows 返回的 windowId 聚焦系统窗口；窗口不存在或系统拒绝聚焦时返回错误。",
                "en": "Focus the exact windowId returned by list_windows; report stale windows or denied activation."
            },
            "parameters": [{ "name": "window_id", "type": "string", "required": true,
                "description": { "zh": "系统窗口 ID。", "en": "Native system window ID." } }],
            "display": { "label": { "zh": "聚焦系统窗口", "en": "Focus system window" }, "icon": "screen", "targetParameter": "window_id" }
        },
        {
            "name": "get_page_info",
            "description": {
                "zh": "底层读取指定窗口的完整无障碍 UI 树，给 AI 返回有长度预算的精简文本摘要，包含可操作元素、定位信息和关键状态；摘要标注省略数量。不传 window_id 时读取当前活动窗口。",
                "en": "Collect the selected window's full exposed accessibility tree and return a bounded compact text summary with actionable elements, selectors, key states and omission counts. Omit window_id to read the active window."
            },
            "parameters": [{ "name": "window_id", "type": "string", "required": false,
                "description": { "zh": "list_windows 返回的系统窗口 ID；省略则取当前活动窗口。", "en": "System window ID from list_windows; omit for the active window." } }],
            "display": {
                "label": {
                    "zh": "读取窗口 UI 树",
                    "en": "Read window UI tree"
                },
                "icon": "search"
            }
        },
        {
            "name": "get_page_screenshot_image",
            "description": {
                "zh": "通过系统 Host 截取设备屏幕，返回 VFS 文件路径和已注册图片链接。",
                "en": "Capture the device screen through the system Host and return a VFS file path and registered image link."
            },
            "parameters": [],
            "display": {
                "label": {
                    "zh": "截取设备屏幕",
                    "en": "Capture device screen"
                },
                "icon": "screen"
            }
        },
        {
            "name": "tap",
            "description": {
                "zh": "在特定坐标模拟点击。",
                "en": "Simulate a tap at the specified coordinates."
            },
            "parameters": [
                {
                    "name": "x",
                    "description": {
                        "zh": "X坐标",
                        "en": "X coordinate."
                    },
                    "type": "number",
                    "required": true
                },
                {
                    "name": "y",
                    "description": {
                        "zh": "Y坐标",
                        "en": "Y coordinate."
                    },
                    "type": "number",
                    "required": true
                }
            ],
            "display": {
                "label": {
                    "zh": "点击屏幕",
                    "en": "Tap screen"
                },
                "icon": "tap"
            }
        },
        {
            "name": "double_tap",
            "description": {
                "zh": "在特定坐标模拟双击（快速连续点击两次）。",
                "en": "Simulate a double tap at the specified coordinates (two quick taps)."
            },
            "parameters": [
                {
                    "name": "x",
                    "description": {
                        "zh": "X坐标",
                        "en": "X coordinate."
                    },
                    "type": "number",
                    "required": true
                },
                {
                    "name": "y",
                    "description": {
                        "zh": "Y坐标",
                        "en": "Y coordinate."
                    },
                    "type": "number",
                    "required": true
                }
            ],
            "display": {
                "label": {
                    "zh": "双击屏幕",
                    "en": "Double tap screen"
                },
                "icon": "double_tap"
            }
        },
        {
            "name": "long_press",
            "description": {
                "zh": "在特定坐标模拟长按操作。适用于呼出上下文菜单、拖拽前的按住等场景。",
                "en": "Simulate a long press at the specified coordinates. Useful for context menus or starting a drag."
            },
            "parameters": [
                {
                    "name": "x",
                    "description": {
                        "zh": "X坐标",
                        "en": "X coordinate."
                    },
                    "type": "number",
                    "required": true
                },
                {
                    "name": "y",
                    "description": {
                        "zh": "Y坐标",
                        "en": "Y coordinate."
                    },
                    "type": "number",
                    "required": true
                }
            ],
            "display": {
                "label": {
                    "zh": "长按屏幕",
                    "en": "Long press screen"
                },
                "icon": "long_press"
            }
        },
        {
            "name": "click_element",
            "description": {
                "zh": "按文本、资源标识、类名、内容描述或边界点击系统界面元素，至少提供一个定位条件。",
                "en": "Click a system UI element by text, resource ID, class, content description or bounds; supply at least one criterion."
            },
            "parameters": [
                {
                    "name": "resourceId",
                    "description": {
                        "zh": "元素资源ID",
                        "en": "Element resourceId."
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "text",
                    "description": {
                        "zh": "元素文本。",
                        "en": "Element text."
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "className",
                    "description": {
                        "zh": "元素类名",
                        "en": "Element class name."
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "contentDesc",
                    "description": {
                        "zh": "无障碍内容描述。",
                        "en": "Accessibility content description."
                    },
                    "type": "string",
                    "required": false
                },
                {
                    "name": "index",
                    "description": {
                        "zh": "要点击的匹配元素，从0开始计数，默认0",
                        "en": "Index of the matched element to click (0-based, default: 0)."
                    },
                    "type": "number",
                    "required": false
                },
                {
                    "name": "partialMatch",
                    "description": {
                        "zh": "是否启用部分匹配，默认false",
                        "en": "Enable partial match (default: false)."
                    },
                    "type": "boolean",
                    "required": false
                },
                {
                    "name": "bounds",
                    "description": {
                        "zh": "元素边界，格式为'[left,top][right,bottom]'",
                        "en": "Element bounds in format '[left,top][right,bottom]'."
                    },
                    "type": "string",
                    "required": false
                }
            ],
            "display": {
                "label": {
                    "zh": "点击界面元素",
                    "en": "Click UI element"
                },
                "icon": "tap"
            }
        },
        {
            "name": "set_input_text",
            "description": {
                "zh": "设置当前焦点输入框或指定唯一资源标识的输入框文本；空字符串用于清空。",
                "en": "Set the focused field or a field with a unique resource ID; an empty string clears it."
            },
            "parameters": [
                {
                    "name": "text",
                    "description": {
                        "zh": "要输入的文本",
                        "en": "Text to input."
                    },
                    "type": "string",
                    "required": true
                },
                {
                    "name": "resource_id",
                    "description": {
                        "zh": "可选，唯一输入框资源标识。",
                        "en": "Optional unique input resource ID."
                    },
                    "type": "string",
                    "required": false
                }
            ],
            "display": {
                "label": {
                    "zh": "输入文本",
                    "en": "Set input text"
                },
                "icon": "edit"
            }
        },
        {
            "name": "press_key",
            "description": {
                "zh": "模拟按键。",
                "en": "Simulate a key press."
            },
            "parameters": [
                {
                    "name": "key_code",
                    "description": {
                        "zh": "平台支持的命名按键。桌面如 ENTER、TAB、ESCAPE；安卓如 BACK、HOME、RECENTS。",
                        "en": "Provider-supported named key: e.g. ENTER, TAB, ESCAPE on desktop or BACK, HOME, RECENTS on Android."
                    },
                    "type": "string",
                    "required": true
                }
            ],
            "display": {
                "label": {
                    "zh": "按下按键",
                    "en": "Press key"
                },
                "targetParameter": "key_code",
                "icon": "keyboard"
            }
        },
        {
            "name": "swipe",
            "description": {
                "zh": "模拟滑动手势。",
                "en": "Simulate a swipe gesture."
            },
            "parameters": [
                {
                    "name": "start_x",
                    "description": {
                        "zh": "起始X坐标",
                        "en": "Start X coordinate."
                    },
                    "type": "number",
                    "required": true
                },
                {
                    "name": "start_y",
                    "description": {
                        "zh": "起始Y坐标",
                        "en": "Start Y coordinate."
                    },
                    "type": "number",
                    "required": true
                },
                {
                    "name": "end_x",
                    "description": {
                        "zh": "结束X坐标",
                        "en": "End X coordinate."
                    },
                    "type": "number",
                    "required": true
                },
                {
                    "name": "end_y",
                    "description": {
                        "zh": "结束Y坐标",
                        "en": "End Y coordinate."
                    },
                    "type": "number",
                    "required": true
                },
                {
                    "name": "duration",
                    "description": {
                        "zh": "持续时间，1–60000 毫秒，默认 300。",
                        "en": "Duration from 1 to 60000 milliseconds; defaults to 300."
                    },
                    "type": "number",
                    "required": false
                }
            ],
            "display": {
                "label": {
                    "zh": "滑动屏幕",
                    "en": "Swipe screen"
                },
                "icon": "swipe"
            }
        }
    ]
}
*/

// Migrated from Operit 1 examples/automatic_ui_base.ts; all device access uses v2 Host APIs.

const UIAutomationTools = (() => {

    type ToolResponse = {
        success: boolean;
        message: string;
        data?: unknown;
    };

    type PointParams = { x: number; y: number };
    type ElementParams = {
        resourceId?: string;
        text?: string;
        className?: string;
        contentDesc?: string;
        index?: number;
        partialMatch?: boolean;
        bounds?: string;
    };
    type SwipeParams = {
        start_x: number;
        start_y: number;
        end_x: number;
        end_y: number;
        duration?: number;
    };

    /** Discovers native windows before selecting a target for automation. */
    async function list_windows(): Promise<ToolResponse> {
        return { success: true, message: '成功获取系统窗口列表', data: await Tools.UI.listWindows() };
    }

    /** Activates exactly the native window chosen by the caller. */
    async function focus_window(params: { window_id: string }): Promise<ToolResponse> {
        return { success: true, message: '成功聚焦系统窗口', data: await Tools.UI.focusWindow(params.window_id) };
    }

    /** Returns the complete accessibility tree of the selected native window. */
    async function get_page_info(params: { window_id?: string }): Promise<ToolResponse> {
        const page = await Tools.UI.getPageInfo(params.window_id);
        return {
            success: true, message: '成功获取窗口 UI 摘要',
            data: {
                windowId: page.windowId, packageName: page.packageName, activityName: page.activityName,
                scope: page.scope, truncated: page.truncated, summary: page.summary,
            },
        };
    }

    /** Captures and registers a screenshot through platform-neutral Host and VFS APIs. */
    async function get_page_screenshot_image(): Promise<ToolResponse> {
        const path = await toolCall('capture_screenshot', {});
        const image = await Tools.Media.registerFromPath({ type: 'image', path });
        return {
            success: true,
            message: '成功截取设备屏幕',
            data: { file_path: path, image_link: image.link },
        };
    }

    /** Taps the requested position in the provider's screen coordinate space. */
    async function tap(params: PointParams): Promise<ToolResponse> {
        const result = await Tools.UI.tap(params.x, params.y);
        return { success: true, message: '点击操作成功', data: result };
    }

    /** Performs the original two-tap sequence with a short explicit interval. */
    async function double_tap(params: PointParams): Promise<ToolResponse> {
        const first = await Tools.UI.tap(params.x, params.y);
        await Tools.System.sleep(120);
        const second = await Tools.UI.tap(params.x, params.y);
        return { success: true, message: '双击操作成功', data: { first, second } };
    }

    /** Long-presses the requested system screen position. */
    async function long_press(params: PointParams): Promise<ToolResponse> {
        const result = await Tools.UI.longPress(params.x, params.y);
        return { success: true, message: '长按操作成功', data: result };
    }

    /** Passes explicit element criteria to the common system UI selector. */
    async function click_element(params: ElementParams): Promise<ToolResponse> {
        const result = await Tools.UI.clickElement(params);
        return { success: true, message: '点击元素操作成功', data: result };
    }

    /** Sets or clears the focused field or an explicitly addressed input element. */
    async function set_input_text(params: { text: string; resource_id?: string }): Promise<ToolResponse> {
        const result = await Tools.UI.setText(params.text, params.resource_id);
        return { success: true, message: '输入文本操作成功', data: result };
    }

    /** Sends a named key supported by the device's system provider. */
    async function press_key(params: { key_code: string }): Promise<ToolResponse> {
        const result = await Tools.UI.pressKey(params.key_code);
        return { success: true, message: '按键操作成功', data: result };
    }

    /** Preserves the caller's swipe duration when forwarding the gesture. */
    async function swipe(params: SwipeParams): Promise<ToolResponse> {
        const result = await Tools.UI.swipe(
            params.start_x, params.start_y, params.end_x, params.end_y, params.duration,
        );
        return { success: true, message: '滑动操作成功', data: result };
    }

    /** Launches exactly the requested app and preserves the platform operation outcome. */
    async function app_launch(params: { package_name: string }): Promise<ToolResponse> {
        if (!params.package_name?.trim()) throw new Error('必须提供 package_name 参数');
        const operation = await Tools.System.startApp(params.package_name);
        return {
            success: operation.success,
            message: operation.details,
            data: { operation },
        };
    }

    /** Adapts a typed handler to the package completion boundary without alternate actions. */
    function asTool<P>(handler: (params: P) => Promise<ToolResponse>): (params: P) => Promise<void> {
        /** Reports the actual operation result or failure to the package caller. */
        return async function (params: P): Promise<void> {
            try {
                complete(await handler(params));
            } catch (error: unknown) {
                complete({ success: false, message: error instanceof Error ? error.message : String(error) });
            }
        };
    }

    /** Reports package readiness without performing any device input. */
    async function main(): Promise<void> {
        complete({ success: true, message: '自动化基础工具包已加载' });
    }

    return {
        list_windows: asTool(list_windows),
        focus_window: asTool(focus_window),
        get_page_info: asTool(get_page_info),
        app_launch: asTool(app_launch),
        get_page_screenshot_image: asTool(get_page_screenshot_image),
        tap: asTool(tap),
        double_tap: asTool(double_tap),
        long_press: asTool(long_press),
        click_element: asTool(click_element),
        set_input_text: asTool(set_input_text),
        press_key: asTool(press_key),
        swipe: asTool(swipe),
        main,
    };
})();

exports.get_page_info = UIAutomationTools.get_page_info;
exports.list_windows = UIAutomationTools.list_windows;
exports.focus_window = UIAutomationTools.focus_window;
exports.app_launch = UIAutomationTools.app_launch;
exports.get_page_screenshot_image = UIAutomationTools.get_page_screenshot_image;
exports.tap = UIAutomationTools.tap;
exports.double_tap = UIAutomationTools.double_tap;
exports.long_press = UIAutomationTools.long_press;
exports.click_element = UIAutomationTools.click_element;
exports.set_input_text = UIAutomationTools.set_input_text;
exports.press_key = UIAutomationTools.press_key;
exports.swipe = UIAutomationTools.swipe;
exports.main = UIAutomationTools.main;
