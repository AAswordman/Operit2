# Screen Studio 前端与软件内嵌约定

> 当前能力说明（2026-10-10）：唯一系统 UI 渲染器为
> `apps/esp32/ui_port/operit_mini_ui.c`，C ABI 为 `operit_ui_*`。
> 固定 UI 不应用布局/OUI2 草稿，不提供拖拽控件或软键盘。侧栏、设置、
> Core 插件列表与详情及状态表情由共享 C 实现。系统页面修改需重建。
> 聊天图片与插件场景是独立能力：图片原件送至 Core，设备显示有界预览；
> 可选 `display.scene` 提供像素素材、本地动画与事件。草稿 API 不能代替场景 API。
> 网页只模拟当前 ESP32 板型；Core 的通用 Edge 路由与事件不依赖板型。

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

修改网页 CSS 只改变编辑器，不改变设备画面。布局 API 或 JSON 修改只保存草稿；修改当前设备系统页面需编辑共用 C 实现并重建。插件场景由 Core JS/TS 通过 `Tools.Edge` 更新。
不要手改 `generated/` 产物；当前固件不生成或应用布局描述头文件。新增前端模块必须同时注册 `src/server.mts` 的静态路由。

## 布局草稿工具（当前固件不应用）

1. AI 调用 `GET /api/components` 和 `GET /api/layout`，读取能力、文档与 revision。
2. AI 使用 `PATCH /api/layout` 提交结构化操作及 revision，后端校验布局、资源预算和版本冲突。
3. 这些接口用于草稿存储与校验；当前固定 UI 页面未启用布局编辑器，也不会把 JSON 更新渲染到设备画布。
4. 保存只写 JSON；OUI2 仅供草稿导出，当前 Wi-Fi/USB 布局部署在联系设备前拒绝。系统 C/Rust 修改由开发者显式构建运行时。
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

## 草稿格式与当前固件安装

`src/layout/project-model.mts` 与 `package-layout.mts` 保留 v2 草稿模型、校验及
OUI2 导出。`pages.js`、`editor.js` 等模块是历史草稿工具，当前固定 UI 不启用布局编辑。
导出 JSON/OUI2 不表示已改变设备画面。`POST /api/deploy/layout` 和
`POST /api/deploy/usb-layout` 当前会拒绝，不存在无需编译的系统页面部署流程。

当前相关接口：

- `GET /api/deploy/ports`、`GET /api/deploy/flash` 查询串口与烧录状态。
- `POST /api/deploy/flash {port}` 安装已构建固件，不启动编译。默认擦除并重写程序分区，保留 NVS。
- `GET /api/deploy/device?address=http://...` 查询设备能力；`dynamicLayout:false` 表示不能应用布局包。
- `POST /api/deploy/package {document}` 仅导出历史 `.oui` 草稿。

显式清空设备使用已有 `resetData` / `confirmReset` 流程，删除配对、空间状态和
历史设备数据，需要重新配置；Core 端存档不会因此被清空。操作细节见 README。

## 模拟器与插件场景

`simulator.js` 自动请求启动本机 Rust 模拟器，提供生命周期、TCP/Token 与开发日志。
固定 C/Wasm UI 承接触摸并发送现有 action；`device-state.js` 映射真实状态，
分别显示配对、等待空间审批和 Core 对话就绪，不能把对话连接当成数据同步完成。
刷新网页不是重启模拟器；停止/启动按钮才控制 Rust 进程。

`scene-renderer.js` 通过开发镜像 `/api/simulator/scene-view` 显示模拟器场景；
这是网页开发功能，不是市场插件接口。插件仍使用 Core 上的 JS/TS、
`Tools.Edge.execute` 和 `on_edge_event`，通过现有 Space/Binding 与端侧通信。
显示协议与渲染位于端侧可选库 `hosts/common/operit-edge-scene`，Core 不依赖该库。

固定 UI 的 C 静态 RAM、动态分配与 Wasm C 栈读数只描述相应渲染器，
不包含 Rust 网络、场景素材、驱动与整板堆。模拟器资源限制不是实机遥测。
跨设备插件开发和真实联调见 [Edge 插件作者指南](../../plugins/docs/edge-plugin-guide.md)。
