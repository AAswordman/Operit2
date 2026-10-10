# ESP32 界面开发与调试接口

系统页面由 `apps/esp32/ui_port/operit_mini_ui.c` 绘制，
Rust 的 `apps/esp32/src/ui.rs` 和 `main.rs` 负责状态与动作适配。
浏览器预览和真机使用同一份 C 实现。修改系统页面时直接编辑这些源码，
重新构建 C/Wasm 预览与固件，再分别验证模拟器和物理设备。

Core 上的 JS/TS 插件使用 `Tools.Edge` 调用设备能力；
可选 `display.scene` 服务支持像素素材、本地动画和事件。
插件流程见 [Edge 插件指南](../../plugins/docs/edge-plugin-guide.md)，
网页宿主与源码边界见 [FRONTEND.md](FRONTEND.md)。

## HTTP 调试

本地服务默认地址为 `http://127.0.0.1:8766`。

| 方法 | 路径 | 用途 |
| --- | --- | --- |
| GET | `/api/board` | 查询预览板型、尺寸与系统 UI 能力 |
| GET | `/api/build` | 查询构建状态、清单与源码是否变化 |
| POST | `/api/build` | 构建浏览器预览和固件，不烧录 |
| GET | `/api/simulator/state` | 查询 Rust 模拟设备状态、TCP 地址与日志 |
| POST | `/api/simulator/start` | 启动 Rust 模拟设备 |
| POST | `/api/simulator/stop` | 停止 Rust 模拟设备 |
| GET | `/api/simulator/memory` | 查询模拟设备的资源限制 |
| GET | `/api/simulator/debug/tree` | 读取浏览器中当前 C/Wasm UI 的节点树 |
| GET | `/api/simulator/debug/snapshot` | 读取浏览器中当前 C/Wasm UI 的结构化屏幕状态 |
| POST | `/api/simulator/debug/tap` | 使用 `{id}` 点击当前可用控件 |
| POST | `/api/simulator/debug/swipe` | 使用 `{direction}` 向当前 UI 提交滑动 |
| POST | `/api/simulator/send` | 使用 `{text}` 向模拟设备的当前 Core 对话发送文字 |
| GET | `/api/deploy/ports` | 查询真机串口 |
| GET | `/api/deploy/flash` | 查询固件安装状态 |
| POST | `/api/deploy/flash` | 使用 `{port}` 安装已构建固件 |

`/api/simulator/debug/*` 需要浏览器预览页面保持打开，命令由实际 C/Wasm 实例执行。
它们不读取物理设备，也不由 Rust 模拟器构造另一套 UI 树。
点击前先读取当前节点 ID；隐藏、禁用或不存在的节点会返回错误。

## 构建与验证

在仓库根目录执行：

```powershell
npm run build --prefix tools/esp32-editor
npm run build:firmware --prefix tools/esp32-editor
npm run check --prefix tools/esp32-editor
npm test --prefix tools/esp32-editor
```

仅网页或预览修改可使用预览构建；系统页面和设备原生能力变更需要固件构建。
配对、空间审批和远程聊天变更另运行
`npm run test:space --prefix tools/esp32-editor`。
模拟器测试使用独立配置和数据目录；真机性能与内存需要实际设备验证。

## 真机 USB 读屏与点击

```powershell
# 在仓库根目录运行；不需要常驻 HTTP server
npm run device:screen --prefix tools/esp32-editor -- --port COM24
npm run device:tree --prefix tools/esp32-editor -- --port COM24
npm run device:tap --prefix tools/esp32-editor -- --port COM24 --id <从读屏结果取得的节点ID>
python -X utf8 tools/esp32-editor/device-debug.py screen --port COM24
```

按实际设备选择串口。设备需要安装包含串口调试入口的固件；更新命令为
`npm run dev --prefix tools/esp32-editor -- --port COM24 --no-monitor`。
普通更新保留 NVS 中的 Wi-Fi、配对身份和空间状态。

返回的 `source: "device-uart"` 表示内容来自物理设备当前 C 渲染器。
`result` 包含页面、节点 ID、文本、坐标、可点击与可用状态，以及配对码和 UI 静态内存。
这是结构化屏幕内容，不是像素截图或整板 RAM 测量。

点击通过真实触摸/事件入口执行。滑动命令按当前页面的支持情况返回结果。
调试命令与 Link 复用 UART0，在 UI 主线程执行；队列单飞，帧有长度限制和 CRC。
USB 调试不代替配对或空间审批。

串口一次只能由一个程序占用：先结束 monitor 或 CLI 常驻串口 session 再读屏。
串口配对可依次执行 pair-start、读屏取码、pair-finish，每一步结束后释放端口。
读屏和点击不主动复位设备；点击超时后先读屏确认状态，避免重复执行动作。

USB 调试检查：`npm run test:device-debug --prefix tools/esp32-editor`。
Rust UART/帧边界测试：`cargo test --manifest-path hosts/boards/esp32/Cargo.toml serial`。
