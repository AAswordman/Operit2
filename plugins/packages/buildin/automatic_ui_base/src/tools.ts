/* METADATA
{
    "api_version": "2.0.0",
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
    "enabledByDefault": true,
    "tools": [
        {
            "name": "usage_advice",
            "description": {
                "zh": "先用 list_windows 发现系统窗口，再用 focus_window(window_id) 聚焦并选择目标。选择后 UI 树、截图和输入统一使用窗口左上角 (0,0) 坐标；Android 小窗使用其原始虚拟屏幕，窗口列表 bounds 只表示物理预览位置。不传 ID 的 get_page_info 保持当前选择，传入 ID 会选择该窗口但不激活它。focus_window 不传 window_id 可取消选择，恢复主屏幕截图和屏幕坐标。app_launch 按设备配置打开原生小窗属于正常工作模式。树保留屏外节点；不可见或只露出边缘的元素须先滚动进入视口并重新读取，disabled 元素不能点击。根据文本、资源标识、内容描述或边界定位元素；用 index 选择同名元素，partialMatch 显式启用部分匹配。操作后读取页面确认实际变化。失效窗口、未授权或平台不支持时报告实际错误。UI 悬浮提示会穿透点击。完成或放弃操作后，调用 complete_ui_operation 关闭宿主小窗、释放屏幕和提示后再回复用户。",
                "en": "Discover windows with list_windows and select one with focus_window(window_id). Trees, screenshots and input then share window-local coordinates with (0,0) at the top left; Android floating windows use their original virtual display. Window-list bounds only locate the physical preview. get_page_info without an ID retains selection; an explicit ID selects that window without activating it. Omit window_id from focus_window to clear selection and restore desktop capture and screen coordinates. Native floating app launches are normal. Scroll invisible or edge-clipped nodes into view and refresh the tree; disabled controls cannot be clicked. Select by text, ID, description or bounds; use index for duplicates and partialMatch for substrings. Inspect the page after input to confirm its effect. Report stale windows, permission and unsupported-platform errors. The indicator passes through input. Call complete_ui_operation after completing or abandoning UI operations, before replying."
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
            "name": "complete_ui_operation",
            "description": {
                "zh": "结束当前对话的 UI 自动化操作；最后一个操作结束时关闭宿主小窗、释放屏幕、取消窗口选择并关闭提示。完成或放弃 UI 操作后、回复用户前调用，等待关闭完成；重复调用安全。其他对话仍在操作时延后释放。",
                "en": "Finish this chat's UI automation. After the last active operation, close hosted windows, release screens, clear selection and dismiss the overlay. Await completion before replying to the user. Safe to repeat. Release is deferred while another chat is operating."
            },
            "parameters": [],
            "display": { "label": { "zh": "完成 UI 操作", "en": "Complete UI operation" }, "icon": "check" }
        },
        {
            "name": "app_launch",
            "description": {
                "zh": "根据设备应用标识启动应用，按设备配置使用原生小窗或主屏；小窗是正常工作模式。启动后用 list_windows、focus_window 和 get_page_info 选择并检查新窗口。失败时返回实际错误。",
                "en": "Launch an app by its device identifier using the configured native window or primary screen. A floating window is a normal mode. Discover, focus and inspect its window after launch. Report the actual failure."
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
                "zh": "传入 list_windows 返回的 windowId 选择操作目标。Android 始终操作对应的原始虚拟屏幕，预览小窗最小化或缩放不影响读取、截图和输入，选择目标不会恢复或放大预览。Windows/macOS/Linux X11 在宿主预览小窗中显示原窗口，不抢占用户的前台工作。后续 UI 树、截图和输入共用窗口左上角 (0,0) 坐标。省略 window_id 可取消选择，恢复主屏幕截图和屏幕坐标。窗口失效时返回错误。",
                "en": "Select a native windowId. Android operates on its original virtual display: minimizing or resizing the preview does not affect trees, screenshots or input, and selection preserves the preview size. Windows/macOS/Linux X11 show a host-owned preview without bringing the source in front of the user. Trees, screenshots and input then share window-local coordinates with (0,0) at the top left. Omit window_id to clear selection and return to desktop capture and screen coordinates. Stale targets fail."
            },
            "parameters": [{ "name": "window_id", "type": "string", "required": false,
                "description": { "zh": "系统窗口 ID；省略可取消窗口选择。", "en": "Native window ID; omit to clear selection." } }],
            "display": { "label": { "zh": "聚焦系统窗口", "en": "Focus system window" }, "icon": "screen", "targetParameter": "window_id" }
        },
        {
            "name": "get_page_info",
            "description": {
                "zh": "读取完整无障碍树并返回有长度预算的精简摘要。传入 window_id 选择窗口，后续 UI 树、截图和输入共用窗口坐标；省略则保持选择，未选择时读取主屏活动界面并使用屏幕坐标。",
                "en": "Read the complete accessibility tree and return a bounded summary. An explicit window_id selects that window for local tree, screenshot and input coordinates. Omit it to retain selection, or inspect the active main surface in screen coordinates when no window is selected."
            },
            "parameters": [{ "name": "window_id", "type": "string", "required": false,
                "description": { "zh": "系统窗口 ID；省略则保持当前选择。", "en": "Native window ID; omit to retain the current selection." } }],
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
                "zh": "截取当前选中窗口，截图像素与 UI 树、输入共用窗口左上角 (0,0) 坐标。未选择窗口时截取主屏幕。返回 VFS 文件路径和已注册图片链接。",
                "en": "Capture the selected window with screenshot pixels matching its tree and input coordinates from (0,0). Capture the main surface when selection is cleared. Return a VFS path and registered image link."
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

    /** Selects the caller's native target or clears it through the owning host. */
    async function focus_window(params: { window_id?: string }): Promise<ToolResponse> {
        const data = await Tools.UI.focusWindow(params.window_id);
        await markTarget(params.window_id);
        return { success: true, message: params.window_id ? '成功选择系统窗口' : '已取消窗口选择，回到主屏幕', data };
    }

    /** Returns the complete accessibility tree of the selected native window. */
    async function get_page_info(params: { window_id?: string }): Promise<ToolResponse> {
        const page = await Tools.UI.getPageInfo(params.window_id);
        if (page.scope === 'window' && page.windowId) await markTarget(page.windowId);
        return {
            success: true, message: '成功获取窗口 UI 摘要',
            data: {
                windowId: page.windowId, packageName: page.packageName, activityName: page.activityName,
                scope: page.scope, truncated: page.truncated, summary: page.summary,
                coordinateNote: page.scope === 'window'
                    ? 'Tree bounds, screenshot pixels and input share window-local coordinates with (0,0) at the top left. Window-list bounds only locate the preview on the desktop. Desktop previews show the selected source without requiring foreground activation. Scroll clipped or offscreen elements into view and read again. Call focus_window without window_id to return to the main screen.'
                    : 'No window is selected. Tree bounds, input and screenshot use desktop coordinates. Supply window_id to select a window and use its local coordinates and screenshot. Scroll clipped or offscreen elements into view and read again.',
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
    function asTool<P>(handler: (params: P) => Promise<ToolResponse>, startsOperation = true, target?: (params: P) => string | undefined): (params: P) => Promise<void> {
        /** Reports the actual operation result or failure to the package caller. */
        return async function (params: P): Promise<void> {
            try {
                const chatId = getChatId();
                if (startsOperation && chatId) {
                    const windowId = target?.(params);
                    await ToolPkg.ipc.call("ui_automation.begin", { chatId, ...(windowId ? { windowId } : {}) }, { targetRuntime: "main" });
                }
                complete(await handler(params));
            } catch (error: unknown) {
                complete({ success: false, message: error instanceof Error ? error.message : String(error) });
            }
        };
    }

    /** Reconciles a focused/restored window or the actual window returned by a snapshot. */
    async function markTarget(windowId?: string): Promise<void> {
        const chatId = getChatId();
        if (chatId) await ToolPkg.ipc.call("ui_automation.begin", { chatId, windowId: windowId ?? null }, { targetRuntime: "main" });
    }

    /** Awaits the owning main runtime instead of changing isolated tool-runtime state. */
    async function complete_ui_operation(): Promise<ToolResponse> {
        const chatId = getChatId();
        if (!chatId) throw new Error('当前调用没有对话上下文，无法结束 UI 操作');
        await ToolPkg.ipc.call("ui_automation.complete", { chatId, closeWindows: true }, { targetRuntime: "main" });
        return { success: true, message: '当前对话的 UI 操作已完成；最后一个操作结束时已关闭宿主小窗、释放屏幕并取消选择，其他对话仍在操作时延后释放' };
    }

    /** Reports package readiness without performing any device input. */
    async function main(): Promise<void> {
        complete({ success: true, message: '自动化基础工具包已加载' });
    }

    return {
        complete_ui_operation: asTool(complete_ui_operation, false),
        list_windows: asTool(list_windows),
        focus_window: asTool(focus_window, true, params => params.window_id),
        get_page_info: asTool(get_page_info, true, params => params.window_id),
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
exports.complete_ui_operation = UIAutomationTools.complete_ui_operation;
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
