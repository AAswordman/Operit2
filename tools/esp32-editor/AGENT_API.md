# Agent UI editing contract

> 当前能力说明（2026-10-10）：唯一系统 UI 渲染器是
> `apps/esp32/ui_port/operit_mini_ui.c`，C ABI 为 `operit_ui_*`。
> 以下布局/组件协议保留为草稿与存储工具；当前固定自绘 UI 不应用布局包，
> 固定 UI 不应用布局草稿，不提供拖拽控件或软键盘。聊天图片预览和插件场景是独立能力；系统页面修改需重建，插件场景使用 Core 上的 Tools.Edge。
> 历史控件布局能力不能视为当前设备能力；以 `/api/board` capabilities 为准。


All clients edit the same `apps/esp32/ui/layout.json`; there is no Agent-specific copy.
The GUI, HTTP clients and MCP adapter share validation and optimistic revision checks.

## HTTP (any language / host)

Base URL: `http://127.0.0.1:8766`.

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/components` | Entire catalog with editable/resource requirements, limits and actions |
| GET | `/api/component-source?id=ID&revision=REV` | Actual layout lines, excerpt, event/router implementation entrypoints; 409 on stale revision |
| GET | `/api/layout` | `{document, revision}` |
| PUT | `/api/layout` | Replace with `{revision, document}` |
| PATCH | `/api/layout` | `{revision, operations}` |
| POST | `/api/layout/validate` | `{document}` → `{errors}`; no write |
| GET | `/api/build` | Build progress, failure, source/firmware manifest |
| POST | `/api/build` | Build preview and firmware; never flash |

PATCH operations:

```json
{"revision":"COPY_FROM_GET","operations":[
  {"op":"update","id":"home_brand","changes":{"text":"MY DEVICE","x":20}},
  {"op":"document","changes":{"enabled":true}}
]}
```

Other operations: `{op:"add",node:{...}}` and `{op:"remove",id:"..."}`. Removing a panel removes descendants.
A stale revision returns HTTP 409. Read again, compare the new document, and deliberately merge changes; never retry by blindly replacing another editor's work.

## CLI

```powershell
npm --prefix tools/esp32-editor run agent -- ui_components
npm --prefix tools/esp32-editor run agent -- ui_read
npm --prefix tools/esp32-editor run agent -- ui_patch @patch.json
npm --prefix tools/esp32-editor run agent -- ui_build_status
```

Use a JSON file for arguments to avoid shell quoting. `OPERIT_UI_URL` overrides the service URL.

## MCP stdio

Register this command in an Agent's MCP configuration:

```json
{
  "command":"node",
  "args":["--experimental-strip-types","./tools/esp32-editor/src/agent.mts","--mcp"],
  "env":{"OPERIT_UI_URL":"http://127.0.0.1:8766"}
}
```

Tools: `ui_component_source`, `ui_read`, `ui_components`, `ui_patch`, `ui_validate`, `ui_build_status`, `ui_build`.
The adapter implements JSON-RPC initialization, tools/list, tools/call and ping; no dependencies or model-provider coupling.

Windows/macOS/Linux Agents can use HTTP or MCP. Android/iOS/Web/OHOS Agents use their host's HTTP capability or existing MCP/remote-tool bridge to this development service. Their `localhost` is not the development PC: use the software's existing forwarding/bridge, not the device's loopback address. This change provides a shared protocol and adapter; it does not install MCP configuration or network forwarding into every host automatically. The service remains bound to the PC's loopback and is not a public unauthenticated editor.

## Data and firmware

Version 1 supports the current board's 320×240 layout, up to 24 nodes and a conservative complexity budget of 40. Nested children reference a preceding `panel`; coordinates are relative to their parent. Bounds, unique IDs, supported widget types, actions, and strings are validated before writes. The draft validator's text constraints do not describe the fixed UI's built-in Chinese font; arbitrary C code and file paths cannot be submitted through layout operations.

Layouts are independent Editor drafts, not firmware UI descriptors. The fixed renderer does not apply layout documents; layout deployment is rejected before contacting a device. Screen code changes require rebuilding the shared C renderer. Build failures remain visible, and the previous successful preview is retained.

Read `/api/build` after editing. An accepted JSON write is not proof of successful compilation or adequate real-device memory/performance. Board flashing remains a separate operation.

## Component conversations and interaction routes

Right-click / touch-hold an editor component to compose a function request. The context includes `kind: "operit.hardware.component"`, `componentId`, component, complete document, revision, dirty flag, request and route capabilities. Never treat an unsaved draft as the persisted document. When dirty, preserve the attached draft and coordinate it before changing persisted layout; do not silently overwrite it with `ui_patch`.

Both GUI and API expose click (`action`) and optional long-press (`longAction`). `GET /api/components` / `ui_components` provides route IDs, labels, targets and event fields. Known routes include `home`, `apps`, `page:theme`, `page:settings`, `page:network`, `page:face`, `page:terminal`, `face_online`, `run_node`, and the empty string to unbind. Unknown routes are rejected. Navigation runs in the shared UI C implementation; device commands use the existing firmware callback. Registering a name alone does not implement a new feature.

Example reply to a component conversation (paste into the editor's AI reply field):

```json
{"componentId":"openApps","operations":[
  {"op":"update","id":"openApps","changes":{"action":"page:theme","longAction":"home"}}
]}
```

Only the referenced component's action fields are accepted in this reply flow; no automatic execution or saving. For direct MCP/API updates, wrap operations with a freshly read `revision`. In run mode, short tap navigates to Theme; long press returns Home and does not also trigger the click. In edit mode, long press opens component context instead. To add new pages or capabilities, update the shared C/Rust implementation, route catalog and tests, then build both targets.


## Source-reference workflow (v2, primary)

The component attachment now includes `codeReference.location` (path, 1-based start/end lines, JSON pointer, excerpt, per-field lines, revision) and `codeReference.implementation` (C event/router functions, Rust command handling and route catalog locations). Use `ui_component_source` with `{id,revision?}` to refresh these references. Use IDs as stable identity; re-read before editing because line numbers move.

The user's request is to implement behavior in the project, not merely generate route JSON. Read the referenced files, modify the selected component's binding and implement any needed page/command code and registered routes, then validate and build both targets. The JSON proposal example above is an optional compatibility path for small binding-only edits. New unsaved components have no file line number (`location:null`); their ID, draft pointer and full draft document identify them without inventing a saved position.

## v2 草稿格式与导出（当前固件不应用）

当前默认项目为 v2：根 nodes/background 是 home，pages 是附加页面（总数最多12），entryPage 是启动页；每页24组件、复杂度40。组件 ID 全项目唯一，parent 同页且指向前面的 panel。节点可设 binding clock/connection/expression，fontSize14/48。草稿路由 `go:页面ID` 仅用于草稿描述，不改变当前设备页面；swipeLeft/swipeRight 是目标页面 ID。

补丁新增 `addPage {page}`、`updatePage {id,changes}`、`removePage {id}`；`add` 可带 pageId。删除页面清除所有入向路由，首页/当前启动页不能直接删除。矩阵推荐 panel+独立 child button；不要把用户交互做成不可选中的内部控件。

保存只修改 `apps/esp32/ui/layout.json` 草稿，不自动改写 C，不启动编译，也不应用到固定 UI。`package-layout.mts` 仅导出 OUI2 草稿；`/api/deploy/layout` 与 `/api/deploy/usb-layout` 当前在联系设备前拒绝。改变系统页面需修改 C/Rust 并显式构建；插件素材/业务更新则走 Core ToolPkg 和 `Tools.Edge`。详见 FRONTEND.md 与 `plugins/docs/edge-plugin-guide.md`。

AI 开发任务 kind `operit.hardware.task` 带 task/context；context 包含当前完整草稿、页面/组件ID、JSON pointer、实际源码行号与 revision。默认发到软件当前对话；独立API通过 `/api/ai/develop` 返回审阅提案。密钥不得写入项目。

USB 当前仅用于安装已构建固件。历史双槽布局部署不可作为当前设备能力；草稿导出不会改变固件。


## 真机 USB 读屏与点击（与模拟器分开）

```powershell
# 在仓库根目录运行；没有常驻 server 也可使用
npm run device:screen --prefix tools/esp32-editor -- --port COM24
npm run device:tree --prefix tools/esp32-editor -- --port COM24
npm run device:tap --prefix tools/esp32-editor -- --port COM24 --id <从读屏结果取得的节点ID>
# 也可直接调用 Python
python -X utf8 tools/esp32-editor/device-debug.py screen --port COM24
```

需要安装包含串口调试入口的固件。首次更新极简固件：
`npm run dev --prefix tools/esp32-editor -- --port COM24 --no-monitor`。
普通更新保留 NVS 中的 Wi-Fi、配对身份、空间状态；不要为此使用 `flash:fresh`。

返回的 `source: "device-uart"` 表示数据来自**物理设备当前 C 渲染器**，不是浏览器 Wasm 或模拟器。
`result` 包含页面、屏幕节点 ID/文本/坐标/可点击与可用状态；极简版还包含屏幕配对码和 UI 静态内存。
这是结构化屏幕内容/对象树，不是像素截图，也不是整板 RAM 测量；不在设备上分配全屏 framebuffer。
原来的 `/api/simulator/debug/*` 仍只操作模拟器，不能当真机读屏使用。

点击使用当前真实 UI 的触摸/事件入口，隐藏/禁用/不存在的目标会报错。
`device:swipe -- --port COM24 --direction left` 仅在当前渲染器支持时成功；极简试验版暂不支持滑动。
调试命令通过 UART0 单一读通道与 Link 复用，在 UI 主线程执行，队列单飞、请求/响应有长度上限和 CRC。
只开放本机 USB，不开放未认证的网络读码接口，也不代替配对/空间审批协议。

PC 侧串口一次只能被一个程序占用：先结束 `monitor` 或 CLI 常驻串口 session 再读屏。
串口配对可依次运行 `pair-start` → 真机 `device:screen` 读取码 → `pair-finish`，每一步结束后释放端口。
读屏/点击命令不主动复位设备；超时不要盲目重试点击（可能已执行而确认丢失），应先读屏确认状态。
日志及 Link 帧会被响应扫描器跳过；不落盘配对码、屏幕历史或临时截图。

检查：`npm run test:device-debug --prefix tools/esp32-editor`；
Rust UART/帧边界测试：`cargo test --manifest-path hosts/boards/esp32/Cargo.toml serial`。
