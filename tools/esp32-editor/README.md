# 共用设备 UI 开发调试台

## 自绘 UI（唯一后端）

设备与 Wasm 预览直接编译 `apps/esp32/ui_port/operit_mini_ui.c`；
Rust 适配器为 `apps/esp32/src/ui.rs`，C ABI 为 `operit_ui_*`。
已删除 LVGL 依赖、旧页面/软键盘/emoji 实现及后端切换标记，不再提供回退构建。

```powershell
# 仓库根目录
npm run build --prefix tools/esp32-editor             # 同源 Wasm 预览
npm run check --prefix tools/esp32-editor             # TypeScript 检查
npm test --prefix tools/esp32-editor                  # 自绘 UI、协议、工具与存储测试
npm run build:firmware --prefix tools/esp32-editor    # 构建固件/预览；不烧录
npm run dev --prefix tools/esp32-editor -- --port COMxx --no-monitor
npm start --prefix tools/esp32-editor                # 预览仍在 8766
```

构建和调试使用上述唯一的一组 npm 命令；旧 `:mini` 别名已删除。

- 固定 320×240 页面：配对、状态、空间审批、简要聊天及解除配对确认。
- 无动态 UI 分配池；两行 RGB565 缓冲为 1280 字节。
- 编译断言约束渲染器静态 RAM（含调试 JSON）≤10 KiB，不包括调用栈、
  驱动、Rust/网络服务或浏览器 framebuffer。debug snapshot 返回实际 `staticBytes`。
- 中文和数字使用仓库独立的 `ui_port/mini_font.h`，保留 OFL 授权，不需下载或提取第三方 UI 字体。
- 审批仍调用共享节点服务；busy 禁止重复提交，配对码优先；解除配对需要确认。
- 暂不支持图片/emoji、动画、机内软键盘、聊天翻页、会话选择和动态布局。
  电脑输入仍可发送文字；网页布局编辑开关停用。布局文档仅作为 Editor 草稿保留；旧分区地址保持不变，但固件不再加载或写入布局包。
- Rust 模拟器 memory 只报告应用注入的资源限制，不再使用退役 UI 的历史堆预算；
  `simulator_heap_*` 为 0 只表示无 UI 动态分配池，不代表整机剩余堆为 0。
- 预览构建无需 ESP-IDF 缓存，仅需 Node.js/Emscripten；固件构建另需 Rust/ESP-IDF/espflash。

如果本机有多份 ESP-IDF 缓存，可一次性配置完整 SDK，之后 npm 命令无需额外环境变量：
`npm run device:setup --prefix tools/esp32-editor -- --idf <ESP-IDF目录>`。
这只保存本机 SDK 路径，不关闭 SDK 的 Git 子模块检查。

## ESP32 模拟设备：真实配对与聊天

运行 `npm start --prefix tools/esp32-editor`，打开 http://127.0.0.1:8766，点击“启动模拟设备”。
首次启动需要可用的桌面 Rust/Cargo 工具链，工具会编译并启动本机 Rust 模拟器。
界面展示实际 TCP 地址、Edge Token、配对码、连接状态和编译日志；停止按钮会终止模拟设备，编辑器退出也会回收进程。

1. 在运行本次 Edge 功能版本的 Core 应用中打开 Space 设备面板，选择 Edge 配对。
2. 填写模拟器显示的局域网 TCP 地址（默认监听 `0.0.0.0:18765`，面板会显示可连接的局域网 IP）和“复制 Token”得到的令牌。
3. Core 发起配对后，模拟 ESP32 屏幕的 Network 页面显示六位配对码；将其填回 Core 完成配对。
4. 在模拟 ESP32 屏幕上进入 Network / Chat 页面。搜索、配对和聊天入口都由共享 自绘 UI 设备 UI 处理；编辑器面板只负责模拟器生命周期、Token 和开发日志。
5. 检查流式回复、Core 端历史记录和错误提示。停止模拟器后 Core 应显示离线；重启保留设备身份与配对凭据，Core 重新连接后恢复固定聊天。

如需指定绑定地址，在启动编辑器的终端设置 `$env:OPERIT_SIM_BIND = "0.0.0.0:18765"`；
也可设置 `$env:OPERIT_SIM_ADVERTISE = "192.168.1.20"` 指定面板展示给 Core 的局域网 IP。防火墙需允许该 TCP 端口。
模拟器的身份与真实板卡分开；令牌及配对凭据保存在忽略提交的 `generated/simulator/`，聊天数据仍存于 Core。

模拟器直接编译固件的 `apps/esp32/src/edge_chat.rs`，复用 `operit-node-runtime`
的配对、加密会话、PeerLink，以及设备空间审批和 SpaceBinding 路由。
编辑器到本机进程的 IPC 只承接 UI 和生命周期，不替代节点之间的 Link 协议。
自绘 UI 画布继续运行共享 C/Wasm，连接指示跟随真实会话。聊天文字、懒加载翻页与触摸由同一份自绘 C UI 执行；电脑键盘/串口草稿用于测试输入，当前无设备软键盘。
此环境模拟设备运行能力，不执行 ESP32 指令集，不模拟 Wi-Fi 无线电、SPI 时序或板上 RAM 限制。

验证命令（仓库根目录执行）：

```powershell
cargo test --manifest-path tools/esp32-editor/simulator/Cargo.toml
npm run check --prefix tools/esp32-editor
npm test --prefix tools/esp32-editor
```

Rust 测试通过真实 TCP 连接检查密码学配对、固件聊天的 routed call/watch、回复显示、权限错误回传和持久身份重连。
其中相邻 Core 使用协议测试端；真实 Core 的 Binding 与权限执行仍由 `operit-node-runtime` 集成测试及上述联调流程验证。

浏览器与 ESP32 使用同一个 `apps/esp32/ui_port/operit_mini_ui.c`，不再用 JavaScript 重写界面。
自绘 UI 的文字绘制、触摸与页面逻辑在 WebAssembly 中运行；HTML 仅提供开发面板，Canvas 显示 自绘 UI 输出的 RGB565 像素。

## 使用与开发构建

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

网页的“清空设备后重新安装”选项默认关闭，执行前二次确认；网页和 npm 复用同一烧录计划。
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
整板堆和 NVS 应通过真实设备 `device:health` 获取；历史 UI 内存预算不再限制图片发送。


普通使用只需 Node.js 22.6+ 和已有的 `generated/ui.mjs`、`ui.wasm`、`manifest.json`：

```powershell
npm install --prefix ./tools/esp32-editor
npm start --prefix ./tools/esp32-editor
```

如果首次启动提示构建清单 404，说明忽略的 `generated/` 产物尚未生成。安装并激活 Emscripten 4.0.14 后执行：

```powershell
npm run build --prefix ./tools/esp32-editor
```

然后重新打开编辑器。底层 C/Rust 运行时变更则执行 `npm run build:firmware --prefix ./tools/esp32-editor`。

访问 http://127.0.0.1:8766 。当前页面预览固定自绘屏幕；布局 JSON 可作为草稿保存，但不会应用到设备屏幕。固件部署使用构建产物。USB 操作需要电脑已安装 espflash、Python/pyserial；当前固定 UI 不支持 Wi-Fi/USB 布局下发；两个部署入口都会在访问设备或串口前明确拒绝。手机可通过端口转发访问此服务，默认不开放局域网监听。

开发者修改共享 C/Rust 或新增底层能力后，在开发环境执行：

```powershell
npm run build:firmware --prefix ./tools/esp32-editor
```

只生成浏览器预览时用 `npm run build --prefix ./tools/esp32-editor`。预览构建需要 Node.js/Emscripten；固件构建还需 Rust/ESP-IDF。通过 `EMSDK` 或 `--emsdk` 指定 Emscripten 安装目录；不在代码中写入机器绝对路径。Windows 上脚本自动把 Cargo target 放在当前工作区所在盘的 `\esp32`，并把 `CARGO_TARGET_DIR` 与 `CARGO_WORKSPACE_DIR` 注入固件构建。预览独立编译仓库自绘 C 源码，不查找第三方 UI 缓存。仅保留当前增量对象与当前产物。

- 保存 JSON / 编辑 C 都不会自动启动编译；调试面板只显示运行时状态。
- 显式构建生成同源 Wasm 与基础固件。构建失败保留旧预览并显示错误。
- `apps/esp32/dist/operit-esp32.bin` 是程序镜像（0x10000），另含 bootloader（0x1000）和 partition-table（0x8000）。网页默认清理程序分区后烧录这些已有文件，保留 NVS 和布局分区；勾选清空设备时删除全部历史设备数据。
- 合并的 `operit-esp32-4mb-full.bin` 用于初始部署，不应当作日常界面修改方式。
- 调试面板的临时状态不写设备设置。屏幕设计与绘制请修改共用 C；Rust 继续负责状态/动作。

## 边界

这是共用自绘 UI 的 WebAssembly 执行环境，不模拟 ESP32 CPU / SPI / Wi-Fi。Web 字体与图标现在来自同一自绘 C 源码和独立字库，不再使用浏览器近似替代。触摸事件交给自绘 C 实现处理。
渲染时间是浏览器指标；内存显示是自绘静态 RAM 与 Wasm C 栈，不是整板 RAM。
当前共用 UI 仅支持 320×240，其他分辨率需要先调整同一份 C 布局，不能只拉伸网页。

## 文件

- `src/build.mts`：编译共用自绘 UI、生成 Wasm、可同时构建 ESP32，记录产物哈希。`npm run build` / `npm run build:firmware`。
- `wasm/bridge.c`：RGB565 帧缓冲、触摸、自绘 UI 内存统计和动作回调。
- `wasm/esp_timer.h`：浏览器时钟适配。
- `web/app.ts`：WebAssembly 宿主和调试面板，不包含按钮布局绘制逻辑。
- `src/server.mts`：本地服务、运行时状态、布局/AI/部署接口。
- `src/api/`：布局、部署、AI 的 HTTP 路由。
- `src/layout/`：项目模型、组件目录、路由和设备布局包。
- `src/source/`：组件源码引用。
- `tests/`：Node 测试与 C store 夹具。
- `generated/`：忽略提交的构建产物和日志。

集成软件本体时可将前端与 `generated/ui.mjs`、`ui.wasm` 放入 WebView；布局与部署由后端管理，基础运行时构建由开发环境完成。

## 磁盘占用

仅保留一份当前预览（约 450 KB）和一份增量编译对象缓存（约 1.2 MB），对象按源文件覆盖，已删除源文件的缓存自动移除。不保存历史固件或预览版本。
临时链接文件与发布暂存文件在构建结束（包括失败）时自动清理；固件日志只保留最新 256 KiB，服务内存中的构建输出限制为 16,000 字符。
Emscripten 编译工具链单独安装，不属于调试器运行包；运行已构建的预览无需携带工具链。安装下载包及分片无需保留。

## 布局草稿与未来 UI 开发

布局模型、HTTP/MCP 文档接口和布局包导出仍保留，但当前固定渲染器不使用布局文档。
拖拽/组件主题/侧栏/软键盘等退役视图不会因保存项目自动恢复。修改屏幕请编辑
`apps/esp32/ui_port/operit_mini_ui.c` 并重新构建；页面设计下一步将采用像素科技风格。
服务返回的 `/api/board` capabilities 表明当前不支持布局编辑、图片或软键盘。
协议和软件工作区嵌入接口见 [AGENT_API.md](AGENT_API.md) 与 [FRONTEND.md](FRONTEND.md)。
