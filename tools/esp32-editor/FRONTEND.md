# ESP32 调试台前端与软件内嵌约定

设备系统 UI 和浏览器预览共同编译 `apps/esp32/ui_port/operit_mini_ui.c`，
通过 `operit_ui_*` C ABI 更新状态、接收触摸并输出 RGB565 像素。
逻辑屏幕为 320×240；页面、控件位置、文字和动作由 C 实现，状态与设备命令由 Rust 适配。
修改系统页面后需要重新构建预览和固件。

聊天图片原件发送至 Core，设备显示有界预览；Core 插件可通过可选
`display.scene` 服务上传像素素材、播放本地动画并接收事件。
插件开发见 [Edge 插件指南](../../plugins/docs/edge-plugin-guide.md)。

## 文件边界

| 修改内容 | 入口 |
| --- | --- |
| 网页结构、工具栏、可访问性 | `web/index.html` |
| 网页配色、间距、响应式样式 | `web/style.css` |
| 面板切换与自适应缩放 | `web/shell.ts` |
| WebAssembly 加载、触摸、像素输出与电脑输入 | `web/app.ts` |
| JSON 请求及软件宿主适配 | `web/transport.ts` |
| 模拟设备的生命周期与连接状态 | `web/simulator.ts` |
| 插件场景预览 | `web/scene-renderer.ts` |
| 本地 HTTP 服务与静态路由 | `src/server.mts`、`src/api/` |
| Wasm 像素缓冲与 C 回调 | `wasm/bridge.c` |
| 设备控件绘制、文字、页面与触摸动作 | `../../apps/esp32/ui_port/operit_mini_ui.c` |
| 状态更新与设备动作处理 | `../../apps/esp32/src/ui.rs`、`../../apps/esp32/src/main.rs` |
| 表情图形 | `../../apps/esp32/ui_port/face.svg` |

网页 CSS 只改变调试台外观。Canvas 显示 C/Wasm 的输出，不能通过网页样式改变设备控件。
新增前端模块时同步注册 `src/server.mts` 的静态路由。
`generated/` 中的 Wasm、清单和日志由构建产生。

## 内嵌宿主入口

前端默认通过相对地址访问同源服务。软件可在加载 `app.js` 前注入请求桥接：

```js
window.operitHost = {
  async request({path, method, body, signal}) {
    return hardwareProjectBackend.request({path, method, body, signal});
  }
};
```

`hardwareProjectBackend` 是宿主需要实现的接口示意。
桥接返回已解析的 JSON，并在请求失败时抛出错误。
`web/transport.ts` 使用这个入口处理 `/api/board`、`/api/build` 等 JSON 请求；
模拟器面板和图片发送还使用直接 `fetch`，内嵌时需提供对应的同源 HTTP 服务。

WebView 需要能加载前端文件、`generated/manifest.json`、`generated/ui.mjs`
和 `generated/ui.wasm`；Wasm 的 MIME 类型为 `application/wasm`。
手机上的 localhost 指手机自身，宿主需负责连接开发电脑上的后端或提供受控转发。

## 开发与验证

1. 修改共用 C 页面、Rust 状态/动作适配或表情源码。
2. 执行 `npm run build --prefix tools/esp32-editor`，生成同源 Wasm 预览。
3. 执行 `npm start --prefix tools/esp32-editor`，检查触摸、页面、连接状态和电脑输入。
4. 系统 UI 或设备能力变更执行 `npm run build:firmware --prefix tools/esp32-editor`。
5. 安装固件后通过真机读屏、点击和实际屏幕检查结果。

前端变更运行 `npm run check --prefix tools/esp32-editor`；
渲染器或调试台行为变更运行 `npm test --prefix tools/esp32-editor`。
涉及配对、空间审批或远程聊天时运行 `npm run test:space --prefix tools/esp32-editor`。
HTTP 与 USB 调试入口见 [AGENT_API.md](AGENT_API.md)。

保持 320×240 逻辑坐标；网页缩放只改变显示尺寸。
保留 HTML 控件 ID，或同步修改其使用者；动态用户内容使用 `textContent`。
检查桌面与手机宽度下的面板、缩放、模拟设备控制及错误提示。
浏览器的渲染时间、Wasm C 栈与静态内存指标不能代替真机 RAM 和 SPI 性能测量。

## 固件安装

`GET /api/deploy/ports` 查询串口，`GET /api/deploy/flash` 查询烧录状态。
`POST /api/deploy/flash {port}` 安装已构建固件，不启动编译。
默认擦除并重写程序分区，保留 NVS；构建成功后需要单独执行安装。
Core 插件业务和符合设备能力的场景素材通过 Core 导入或上传更新。
