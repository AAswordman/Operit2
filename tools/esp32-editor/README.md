# ESP32 编辑器与调试工具

本目录提供共享 C/Wasm UI 预览、Rust Edge 模拟器、固件构建和真机调试。
[Edge 职责](../../docs/edge-architecture.md)定义设备空间边界；
[固件说明](../../apps/esp32/README.md)维护板卡实现和限制；
[前端源码说明](FRONTEND.md)与[调试接口](AGENT_API.md)维护开发工具接口。

## 启动与源码生效

运行需要支持 `--experimental-strip-types` 的 Node.js 22.6+（推荐 24）、桌面 Rust/Cargo。
预览构建还需要 Emscripten，完整固件构建需要 ESP Rust / ESP-IDF 工具链。

```powershell
npm install --prefix tools/esp32-editor
npm run build --prefix tools/esp32-editor
npm start --prefix tools/esp32-editor
```

打开 `http://127.0.0.1:8766`。`generated/` 被 Git 忽略，首次启动前需要构建；
预览独立编译仓库 C 源码，不需要 ESP-IDF 构建缓存。
通过 `EMSDK` 或 `npm run device:setup --prefix tools/esp32-editor -- --emsdk <目录>`
指定 SDK。工具默认仅监听本机，手机访问需要由宿主提供受控转发。

| 修改内容 | 生效方式 |
| --- | --- |
| Web TypeScript / CSS | 刷新网页 |
| 共享 C UI / 表情 SVG | `npm run build` 后刷新网页；真机另需构建和更新固件 |
| Rust 模拟器 / 共享固件模块 | 停止、启动模拟设备，Cargo 重新检查并编译 |
| 设备配置和场景素材 | 使用设备原生接口保存，无需修改固件 |

刷新网页不等于重启 Rust 进程。系统页面直接修改 `apps/esp32/ui_port/operit_mini_ui.c`，
状态与动作使用固件 Rust 模块；不要另写一套 Web 页面来模拟设备控件。

## Rust 模拟器

网页启动真实 Rust Edge 进程，默认 TCP 监听 `0.0.0.0:18765`，身份和配对保存到
忽略提交的 `generated/simulator/`，与真实板卡分开。面板显示实际地址和编译日志，
配对码由共享 C/Wasm 屏幕显示。停止设备或退出编辑器会回收模拟器进程。

模拟器复用固件的设备空间控制器、独立聊天、远端聊天和原生服务模块，并使用共享
Peer runtime 完成验证码配对、加密连接和空间审批。可从 Core 发起配对并在设备上
批准加入，也可使用「设置 → 设备空间」从设备发起。完整空间生命周期见 Edge 职责文档。

mDNS 使用共享 Host 服务；隔离测试可设置 `OPERIT_SIM_DISCOVERY=false`。
`OPERIT_SIM_BIND` 指定监听地址，`OPERIT_SIM_ADVERTISE` 指定对外展示的 IP。
仅绑定 `127.0.0.1` 时只能进行本机直连测试；局域网访问需要相应端口可达。

网页与 Rust 是两个运行时：C/Wasm 拥有导航、绘制、滚动和输入草稿，Rust 拥有设备
会话和数据状态。网页轮询 `/api/simulator/state`，操作后主动刷新；后台标签页可能
被浏览器节流。IPC 只服务本机调试，不替代节点间 Link。

每次 Rust 启动有独立 sessionId，发送回执通过本地 sendId 去重。这些 ID 不进入
公共 Link 协议。多个网页保留各自的导航和草稿，共享同一模拟设备的业务会话。

模拟器不执行 ESP32 指令集，不模拟无线电、SPI 时序或整板 RAM。屏幕来自实际 C
渲染器，资源限制来自应用注入，实际堆和 NVS 必须读取真机遥测。

## 检查与场景验证

先构建当前源码的预览，再运行依赖 Wasm 的测试，避免使用旧 `generated/` 产物。

```powershell
npm run build --prefix tools/esp32-editor
npm run check --prefix tools/esp32-editor
npm test --prefix tools/esp32-editor
cargo test --locked --manifest-path tools/esp32-editor/simulator/Cargo.toml
npm run test:device-debug --prefix tools/esp32-editor
npm run test:xtensa --prefix tools/esp32-editor
```

Rust 测试覆盖实际固件模块中的会话恢复、迟到结果隔离、独立聊天空间切换和协议行为。
测试 fixture 在 32 KiB Host worker 下优化编译嵌套 future；这个设置不扩大设备栈，
也不意味着桌面测试可以测出真机资源余量。

以下集成入口使用独立 Core CLI、Rust Edge 和共享 C/Wasm UI，使用临时身份目录：

```powershell
npm run test:device-space --prefix tools/esp32-editor
npm run test:space --prefix tools/esp32-editor
npm run test:standalone --prefix tools/esp32-editor
```

`test:device-space` 覆盖发现、配对输入、保留凭据替换、取消、审批和重启恢复；
`test:space` 覆盖远端 Core / Binding 使用；`test:standalone` 同步设备聊天配置后
关闭 CLI，再验证设备继续请求模型及重启后恢复配置。模型服务使用确定性 HTTP fixture。
可通过 `OPERIT_SIM_TEST_CLI` 指定已构建的兼容 CLI，避免每次重建 Core。

测试不能替代实机验收。`npm run test:hardware` 要求设备已经完成配对和入空间，
检查加密调用、UI、堆/NVS 等声明的场景；它不负责首次配对和完整聊天验收。

## 固件构建、安装与资源

### 实机开发快捷命令

在 `tools/esp32-editor` 目录执行，或在仓库根目录给 npm 加 `--prefix tools/esp32-editor`：

```powershell
cd tools/esp32-editor
npm run dev          # 构建固件/预览 → 烧录 → 串口日志，一条命令
npm run flash        # 只更新已有固件，不重新构建
npm run monitor      # 只看串口日志，不主动重启，不自动写日志文件
npm run ports        # 列出串口
```

默认自动选择唯一 USB 串口，忽略蓝牙串口。多个 USB 串口时拒绝猜测：追加 `-- --port COMxx`。
`dev` 可以追加 `-- --no-monitor`；构建或 ABI 检查失败会中止，绝不会继续烧旧产物。
`dev` 使用刚构建且刚烧录的 ELF 解析崩溃地址；独立 `monitor` 不猜测设备对应的 ELF。
烧录前应退出其他串口监视器。

构建需要 Rust/ESP-IDF、espflash、Python/pyserial 和 Emscripten 4.0.14；运行 npm 入口需要支持
`--experimental-strip-types` 的 Node.js（22.6+，推荐 24）。SDK 路径只需配置一次：

```powershell
npm run device:setup -- --emsdk <已安装的emsdk目录>
# 可选：固定串口，不设置时自动发现；断开指定端口时不会擅自改烧另一块板
npm run device:setup -- --port COMxx
```

本机配置存入被 Git 忽略的 `device.local.json`，只包含 SDK 路径和可选串口，不保存令牌。
`EMSDK` / `ESPFLASH_PORT` 环境变量或命令行参数可以覆盖配置。首次固件构建自动生成
ESP-IDF 前置文件，无需先手动执行一次 Cargo。

### 更新与清空设备的区别

- `npm run flash` / `npm run dev`：先擦除完整 `factory` 程序分区，再写入现成
  bootloader、分区表和程序。清掉旧程序尾部，但保留 NVS（Wi-Fi、配对身份、空间状态）和 UI 布局。
- `npm run flash:fresh`：明确确认后擦除整片 Flash 再安装，删除包括旧 NVS/旧布局在内的历史设备数据。
  交互终端要求输入 `ERASE`；自动执行必须显式使用 `npm run flash:fresh -- --confirm-reset`。
  需要同时重新构建、烧录并看日志时用 `npm run dev:fresh`（同样要求输入 `ERASE`）。
  **之后必须重新配置 Wi-Fi、重新配对；Core 中旧身份的配对记录不会自动删除。**
- `npm run flash -- --dry-run`：只校验现有产物并输出烧录计划，不连接或修改设备。

当前固定预览未启用固件安装面板，请使用上述 npm 命令。保留的部署模块与 npm 复用同一烧录计划；其清空设备选项默认关闭，执行前需要确认。
两种模式都会在擦除前检查产物存在、程序大小和完整的固定板型分区布局。
中间步骤保持引导器状态，仅最后一次写入成功后重启；任何步骤失败立即停止。

固件始终覆盖固定地址，不会每次烧录追加一份程序。保留 NVS 不等于 NVS 自动无限增长；
它保存的是运行时的真实状态，快照切换会清理旧/未发布的块；不再运行退役布局的双槽写入流程。
有意累积的有效身份/成员/记录仍受 NVS 容量上限约束。不能用每次全擦掩盖运行时泄漏。

运行时 NVS 只支持当前格式：`snapshot_v2` 提交标记、ONV3 路径压缩封装、2 KiB 压缩块。
旧 JSON/base64、ONV2、1 KiB 块和未压缩快照的读取/迁移已删除；遇到不支持的格式会报错，
不会自动迁移、当成空节点重建或擦除数据。普通烧录保留分区不代表支持旧存储格式。
健康日志会定期输出 `nvs_used` / `nvs_free` / `nvs_available` / `nvs_total`，与
`heap_free` / `heap_min` / `largest_8bit` / `main_stack_free` 分开观察，区别持久储存不足与 RAM 不足。
重复更新/删除/模拟重启的存储测试检查不积累失效快照，但不替代实机长期验证。

构建产物复用固定路径：`generated/`、`apps/esp32/dist/` 和当前盘根目录的 `esp32/`，
不是每次新建一份固件历史。Rust 的增量缓存是电脑磁盘数据，不在设备 NVS 中。

### 模拟器的资源限制与真实遥测边界

模拟器与真机都使用 `PeerRuntimeLimits::constrained()`：消息帧上限 8 KiB，
入站会话/待配对各 2 个、并发探测 1 个。限制由应用注入，不按桌面/ESP-IDF 编译目标分叉。
Host worker 使用 32 KiB 栈；Wasm C 栈采用固件的主任务栈配置并开启溢出检查。
UI 静态 RAM、绘制缓冲和 C 栈由渲染器独立报告，不代表 ESP32 整板堆。
`/api/simulator/memory` 仅报告 `configured-limits`，不伪造 `freeHeap` 或最大连续块遥测。

```powershell
Invoke-RestMethod http://127.0.0.1:8766/api/simulator/memory
```

桌面进程仍由操作系统分配内存，无法用本机 RSS 证明真机不会 OOM。
整板堆和 NVS 应通过真实设备 `device:health` 获取。

## 插件与音频联调

宿主插件使用现有 Call / Watch / Push 访问设备。配置编辑、设备本地交互和需要持续
运行的宿主处理分别遵守[硬件 ToolPkg 接入](../../plugins/docs/edge-plugin-guide.md)。
[桌宠配置器](../../plugins/packages/external/edge_pixel_pet/README.md)保存原生 `edge.scene`
配置；关闭配置器不影响设备继续显示和处理点击。

模拟器注册应用自有的 `operit.device.speech` Watch / Push 示例协议，可连接
[语音处理插件](../../plugins/packages/examples/device_speech_processor/README.md)。
通用音频通道可用不表示真实板卡已提供麦克风或扬声器，也不表示聊天麦克风按钮已经
接入完整识别流程；请按具体入口和实际驱动分别验证。
