# Screen Studio 前端与软件内嵌约定

> 当前后端说明（2026-10-06）：UI 已删除，唯一渲染器是
> `apps/esp32/ui_port/operit_mini_ui.c`，C ABI 为 `operit_ui_*`。
> 以下布局/组件协议保留为草稿与存储工具；当前固定自绘 UI 不应用布局包，
> 不提供拖拽控件、软键盘或图片预览。左侧抽屉、设置、插件占位页和 SVG 表情屏由共用 C 实现；SVG 构建为固定绘制指令。实现新屏幕需修改自绘 C 源码并重建。
> 历史控件布局能力不能视为当前设备能力；以 `/api/board` capabilities 为准。


目标是让用户在 Operit 对话中要求 AI 修改硬件界面，并在同一个编辑器里看到结果。
当前提供独立 Web 编辑器、HTTP/MCP 布局接口、独立模型 API，以及 Flutter 工作区浏览器到当前对话的桥接源码。
不需要把编译工具链复制到手机，也不需要额外前端框架。

## 文件边界

| 修改内容 | 入口 |
| --- | --- |
| 编辑器结构、工具栏、可访问性 | `index.html` |
| 配色、间距、桌面三栏、手机样式 | `style.css` 的变量与媒体查询 |
| 面板切换、自适应缩放 | `shell.js` |
| 拖拽、组件属性、撤销/重做、草稿状态 | `editor.js` |
| 数据访问及软件宿主适配 | `transport.js` |
| UI 加载、触摸、像素输出、构建状态 | `app.js` |
| 可用组件和内存预算校验 | `src/layout/layout-model.mts` |
| 用户硬件界面的持久化设计 | `../../apps/esp32/ui/layout.json` |
| 控件实际绘制、字体、动作 | `../../apps/esp32/ui_port/operit_mini_ui.c` |

修改网页 CSS 只改变编辑器，不改变设备画面。修改设备布局请通过布局 API 或编辑 JSON；扩展控件行为则修改共用 C 实现。
不要手改 `generated/` 产物；当前固件不生成或应用布局描述头文件。新增前端模块必须同时注册 `src/server.mts` 的静态路由。

## 对话 → 布局 → 固件

1. AI 调用 `GET /api/components` 和 `GET /api/layout`，读取能力、文档与 revision。
2. AI 使用 `PATCH /api/layout` 提交结构化操作及 revision，后端校验布局、资源预算和版本冲突。
3. 编辑器每两秒读取最新版本；页面在后台时暂停读取。无草稿时更新画布，有草稿时保留本地内容并提示冲突。
4. 保存只写 JSON；部署打包为 OUI2 数据，下发设备，不编译。底层 C/Rust 修改才由开发者显式构建运行时。
5. 烧录保持独立操作；构建成功不等于已烧入设备。

现有 Agent/MCP 工具及请求示例见 [AGENT_API.md](AGENT_API.md)。以后软件中的不同模型共用这些工具和校验，不需要各写一套布局解释器。

## 内嵌宿主入口

默认前端通过相对地址访问同源服务。软件可在加载 `app.js` 前注入：

```js
window.operitHost = {
  async request({path, method, body}) {
    // 由软件宿主桥接到已授权的硬件项目后端。
    // 返回已解析的 JSON；校验失败、409 冲突等必须 reject / throw。
    return hardwareProjectBackend.request({path, method, body});
  }
};
```

`hardwareProjectBackend` 是待实现的软件宿主接口，上面是接口示意，并非现成插件。
路径包括 `/api/board`、`/api/layout`、`/api/build`。布局请求和构建状态都走 `transport.js`。
WebView 还需用 HTTP/应用资源服务提供前端文件及 `generated/manifest.json`、`ui.mjs`、`ui.wasm`；Wasm 使用 `application/wasm`。
手机上的 localhost 指手机自身，不能直接连接电脑回环地址。宿主应负责转发到构建后端或通过受控桥接调用。
不要为嵌入直接放开本地构建接口的跨域限制。这里没有通配来源的 postMessage 接收器。

## 编辑约定与验证

- 保持 320×240 逻辑坐标，缩放只改变显示尺寸；父容器坐标相对父级。
- 手机优先轻点添加，再拖动定位；设计面板提供 1 px 微调。桌面支持拖入与键盘。
- 保留 HTML 的控件 ID，或同步更新使用者。所有动态用户内容用 `textContent`。
- Ctrl/Cmd+S 保存；Ctrl/Cmd+Z 撤销；Ctrl/Cmd+Shift+Z 或 Ctrl/Cmd+Y 重做；Ctrl/Cmd+D 复制；方向键移动 1 px，Shift 为 8 px。文本输入框保留原生编辑快捷键。
- 撤销/重做合计最多 20 步，存于内存。没有 localStorage、IndexedDB、Service Worker 或历史产物缓存。
- 运行 `node --test --experimental-strip-types ./tools/esp32-editor/tests/*.test.mjs`。
- 启动 `node --experimental-strip-types ./tools/esp32-editor/src/server.mts`，检查 1440、390、320 px 宽度；添加、拖动、缩放、复制、删除、撤销/重做、搜索、手机面板切换。
- 检查保存期间继续修改仍显示未保存、AI 外部修改不覆盖草稿、无草稿时更新 AI 修改。
- 修改编辑器/JSON 不构建；运行时 C/Rust 修改才由开发环境执行 `npm run build` / `npm run build:firmware`。

## 组件上下文与功能路由

编辑模式下右键或触摸长按组件（550 ms）打开“组件功能”；键盘可用 Shift+F10，或点工具栏“功能 / AI”。移动超过 8 px 会取消长按，继续拖动。运行模式的长按由 UI 处理。

`src/layout/routes.mts` 描述可执行路由，`/api/components` 同时提供 `routes` 和 `eventBindings`。节点的 `action` 绑定点击，兼容原有文档；可选 `longAction` 绑定长按。长按有动作时使用 UI SHORT_CLICKED / LONG_PRESSED 分流，长按释放不会重复执行点击。当前支持内置页面跳转与既有设备命令。新增页面使用 v2 pages 和 go:ID，无需扩展 C；新增底层命令仍需实现运行时能力。

`interactions.js` 管理组件源码引用面板；`src/source/component-context.mts` 构造上下文。主要流程是发送组件文件位置和需求，让 AI 直接修改布局、事件与路由源码。手动配置和粘贴结构化路由提案保留在折叠的辅助面板中，不是 AI 必须遵循的回复格式。

软件可在原有 `window.operitHost` 上提供以下入口：

```js
window.operitHost.sendToChat = async payload => {
  // 将结构化组件引用与 requirement 交给当前软件对话。
  // 接收成功必须显式确认；失败 throw。宿主负责聊天会话和模型调用。
  await chatWorkspace.attachHardwareComponent(payload);
  return {accepted: true};
  // 也可返回 {accepted:true, proposal:{componentId,operations:[...]}}。
  // 编辑器展示提案，用户点击应用后才改变草稿。
};
```

上面的 `chatWorkspace` 为接口示意。软件宿主需要把结构化任务送入 Core route；WebView 只承担页面输入源职责，不注册聊天 UI 回调。普通浏览器没有软件宿主时，可复制引用或切换独立 API。密钥和模型配置只保留当前页内存。


### 源码引用协议 v2

`kind: operit.hardware.component` 保持不变，`version: 2` 增加 `codeReference`：

- `location`：布局相对文件路径、真实 `startLine/endLine`（1 起始）、组件 JSON pointer、原始片段、字段行号与源码 revision。
- `implementation`：事件分发、事件绑定、页面函数、Rust 设备动作处理、路由目录的位置与各自文件版本。
- `status`：`saved-component`、`modified-component` 或 `unsaved-component`。新组件没有磁盘行号，`location` 为 null；`draftPointer` 指向随消息附带的草稿文档。

`GET /api/component-source?id=...&revision=...` 每次从磁盘读取，不保存索引缓存。布局版本不一致返回 409。`source-reference.mjs` 解析实际 JSON 偏移，不能用格式化后的草稿估算行号。`component-source.mjs` 只定位固定项目文件，不接受任意文件路径。

宿主的 `sendToChat(payload)` 应把代码引用显示为对话附件或引用卡片，需求作为用户输入；无需在宿主中解释或执行模型生成的代码。拥有工作区工具的 AI 根据引用阅读/编辑源码，再通过现有编译流程验证。行号可能随着编辑改变，必须用组件 ID 和版本复核。接入 `request` 的宿主也需转发新的 `/api/component-source` 路径。

## 多页面和部署

`src/layout/project-model.mts` 管理页面、动态路由和矩阵展开；`pages.js` 提供页面关系图；`src/layout/geometry.mts` 提供四角缩放。`ai.js` / `src/api/ai-api.mts` 支持默认软件对话及独立 Chat Completions 接口，可读取固定项目源文件，提出布局操作/精确源码替换，审阅后应用。后端上限、revision 和唯一替换校验防止过期修改。

`src/layout/package-layout.mts` 将 v2 JSON 打包成 OUI2（16字节头：magic/数据长度/CRC32/协议号），不使用编译器。`deploy.js` / `src/api/deploy-api.mts` 提供设备检查、下发、项目/布局包导出和现成固件串口烧录。软件宿主若替换 transport，应代理 `/api/deploy/*` 和 `/api/ai/*`。手机连接电脑可用安全端口转发；默认服务仍只监听127.0.0.1。

- `GET /api/deploy/ports`、`GET /api/deploy/flash` 查询串口/烧录状态。
- `POST /api/deploy/flash {port}` 使用现有 dist 二进制，不构建，不擦除整个Flash。
- `GET /api/deploy/device?address=http://...` 检查协议与设备布局版本。
- `POST /api/deploy/layout {address,token,document}` 直接部署当前草稿，返回 accepted/bytes/previousRevision；前端轮询确认新版本才显示部署完成。
- `POST /api/deploy/package {document}` 下载 `.oui`。

整个系统不缓存聊天记录/历史布局；撤销20步在内存。设备双槽用于掉电恢复，不是无限历史。

USB 离线部署：在“部署到设备”刷新串口，选择 COM 口，点击“USB 下发布局，无需编译”。后端先读取设备分区表和两个布局槽位，核对地址/尺寸/CRC/版本，只覆盖非当前槽位并递增版本；不改程序分区或 NVS。临时读取文件和布局文件结束后删除。接口为 `POST /api/deploy/usb-layout {port,document}`，状态仍用 `GET /api/deploy/flash`。与 Wi-Fi 热更新不同，USB 使用 ROM 引导器，会重启设备。
