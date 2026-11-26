# ESP32 Edge 构建核对记录（2026-11-26）

当前提交：`7605b2fd1c39b0c4a2c6db544615429a27fd7aa2`。组合为 `remote-display`，即根固件 crate 不启用 `standalone-apps`。本次读取已有产物，并通过 Cargo metadata / unit graph 核对当前构建，不重新编译、刷机或修改产品源码。

核对依据：`Cargo 固件依赖清单`（本机构建产物）、`ESP-IDF 应用编译命令`（本机构建产物）、`Bootloader 编译命令`（本机构建产物）、`当前 Cargo 编译任务图`（本机构建产物）。

Cargo 当前图有 397 个构建单元：342 个编译单元和 55 个 build.rs 执行单元。编译单元中，目标侧为 180 个（178 lib、1 rlib、1 固件 bin）；电脑侧为 162 个（98 lib、16 proc-macro、48 build.rs 编译）。这些是构建单元数量，含标准库，并非 397 个文件或 397 个独立库。

仓库中参与目标编译输入的手写 Rust 源文件为 237 个，分属 16 个 crate；另有 3 个仓库构建脚本/辅助 Rust 文件；该顶层依赖清单还列出了 2 个 Node runtime 的 OUT_DIR 生成文件。ESP-IDF sys 等第三方 crate 另有自己的生成绑定，未计入这 2 个文件。此计数不包括第三方 Rust 库和宏生成的内存代码。`.d` 表示输入依赖；源码中的未使用函数、接口和常量可能在链接时被删除，因此不能按文件数推算 Flash。

| 仓库 crate 目录 | Rust 输入数 | 用途 |
|---|---:|---|
| [core/crates/foundation/host-api](<../../../../core/crates/foundation/host-api>) | 3 | HostManager 和公共 Host API 组装 |
| [core/crates/foundation/host-contract](<../../../../core/crates/foundation/host-contract>) | 21 | 跨平台 Host 接口及数据类型 |
| [core/crates/foundation/link](<../../../../core/crates/foundation/link>) | 11 | 帧/操作/值编解码、会话、流、客户端和路由协议 |
| [core/crates/foundation/util](<../../../../core/crates/foundation/util>) | 74 | JSON、日志、偏好等共享工具；包含未使用的模块定义 |
| [core/crates/foundation/vfs](<../../../../core/crates/foundation/vfs>) | 7 | Host 文件与路径接口 |
| [core/crates/node/peer-link](<../../../../core/crates/node/peer-link>) | 11 | TCP/UART 等承载适配及流消息边界 |
| [core/crates/node/runtime](<../../../../core/crates/node/runtime>) | 29 | 配对、认证加密、设备身份、Space 成员与权限、心跳/重连 |
| [core/crates/persistence/store](<../../../../core/crates/persistence/store>) | 28 | 身份、配对/空间状态、网络控制与偏好存储 |
| [edge/boards/esp32](<..>) | 17 | 固件启动、服务组装、Wi-Fi、设置、空间 UI、本地存储、产品预算 |
| [edge/boards/esp32/board](<../board>) | 14 | SPI 屏幕、触摸、GPIO、TCP、串口及板级 Host |
| [edge/sdk](<../../../sdk>) | 3 | 制造商 SDK 入口、本地 agent 抽象和 SpaceDevice |
| [edge/sdk/contract](<../../../sdk/contract>) | 3 | 设备服务契约；接口定义不等同于功能实现 |
| [edge/sdk/node](<../../../sdk/node>) | 5 | NativeClient、认证后的服务路由和节点入口 |
| [edge/sdk/proxy](<../../../sdk/proxy>) | 1 | Edge 原生 Proxy 接口 |
| [edge/sdk/scene](<../../../sdk/scene>) | 2 | 场景协议、素材上传/缓存、解码、条带渲染 |
| [hosts/common/operit-host-native-scheduler](<../../../../hosts/common/operit-host-native-scheduler>) | 8 | ESP-IDF 上使用的任务/事件调度与 I/O 调度 |

固件入口的 17 个文件：

| 文件 | 用途 |
|---|---|
| [config.rs](<../src/config.rs>) | 固件启动配置 |
| [device_preferences.rs](<../src/device_preferences.rs>) | 设备偏好服务 |
| [device_services.rs](<../src/device_services.rs>) | 原生设备服务与场景请求分发 |
| [device_space_ui.rs](<../src/device_space_ui.rs>) | 设备空间界面状态 |
| [main.rs](<../src/main.rs>) | 启动、硬件/共享运行时组装、任务轮询与故障诊断 |
| [runtime_host.rs](<../src/runtime_host.rs>) | 固件 Host 能力适配和服务发现 |
| [runtime_profile.rs](<../src/runtime_profile.rs>) | ESP32 产品专属 Link 帧预算和 Scene 预算 |
| [runtime_storage_codec.rs](<../src/runtime_storage_codec.rs>) | 持久化数据编解码 |
| [runtime_storage_host.rs](<../src/runtime_storage_host.rs>) | NVS 上的 RuntimeStorageHost 实现 |
| [runtime_storage_pack.rs](<../src/runtime_storage_pack.rs>) | 存储内容打包 |
| [settings.rs](<../src/settings.rs>) | 设置持久化与设置 HTTP 服务 |
| [space_join_ui.rs](<../src/space_join_ui.rs>) | 空间加入提示/交互 |
| [space_work.rs](<../src/space_work.rs>) | 空间工作队列 |
| [status.rs](<../src/status.rs>) | 固件状态 |
| [ui.rs](<../src/ui.rs>) | Rust/C UI 桥接和场景显示 |
| [ui_capabilities.rs](<../src/ui_capabilities.rs>) | 设备 UI 能力暴露 |
| [wifi.rs](<../src/wifi.rs>) | Wi-Fi 连接和网络状态 |

远程组合明确没有选中的板级模块：`edge_chat.rs`、`standalone_chat.rs`、`device_speech.rs`、`device_http.rs`、`audio.rs`、`wav.rs`，见 [main.rs](<../src/main.rs>)。其中最后的 `audio.rs` 是固件应用模块；`board/src/audio.rs` 是另一份板级模块，它仍被读取。IDF 的 `operit_board_audio/audio.c` 也仍进入编译清单，是否保留其函数由最终引用决定。

当前 feature：SDK=`space,scene`；Node runtime=`peer-runtime,peer-state,route-catalog`；Store=`node-state`；Util=`compact-json,no-app-logging`。未启用 Node runtime 的 `full` 和 Store 的 `full`。Host 契约源码里出现 JavaScript/终端接口，并不表示固件编入了对应引擎。

本地 C 输入有 [operit_mini_ui.c](<../ui_port/operit_mini_ui.c>) 和 [audio.c](<../board/operit_board_audio/audio.c>)。UI C 文件还通过 include 使用 `operit_mini_ui.h`、`mini_font.h`、`mini_face.h`；头文件内容并入使用它的翻译单元。应用 IDF 清单有 783 个 C/C++/汇编翻译单元，Bootloader 独立清单有 91 个。Ring 自己的 build.rs 还会编译密码算法后端，不能把 IDF 的 783 个单元当作全部第三方原生代码。完整原生命令见 JSON/TSV。

猫插件的源代码在 [service.ts](<../../../../plugins/packages/external/edge_esp32_pet/src/service.ts>)、[driver.ts](<../../../../plugins/packages/external/edge_esp32_pet/src/driver.ts>) 等文件，资源为该插件 `resources/cat-*.esp`。它们打包成 Core 安装的 ToolPkg，不参与 Xtensa 固件链接。Core 订阅任务状态并同步场景/素材，Edge 执行通用场景服务。设备可能将收到的素材写入持久化存储，那也不等于素材预先嵌入 app.bin。

编译前代码及生成代码的实际例子：

① 产品预算是普通 Rust 数据，定义在 [runtime_profile.rs](<../src/runtime_profile.rs>)，不是共享协议的全局写死值。

```rust
pub const SCENE_LIMITS: SceneLimits = SceneLimits {
    pack_bytes: 8192,
    cache_bytes: 16384,
};

pub fn peer_limits() -> PeerRuntimeLimits {
    PeerRuntimeLimits {
        maxFrameBytes: 2 * 1024,
        ..PeerRuntimeLimits::constrained()
    }
}
```

② 场景数据的源码使用 Serde derive，见 [lib.rs](<../../../sdk/scene/src/lib.rs>)。

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}
```

这段短声明会展开为 Serialize/Deserialize 实现、字段识别器、map/seq visitor、错误路径等。不同的具体类型和反序列化器还可能生成不同的机器码实例。derive 生成器作为电脑上的 proc-macro 编译，其生成的实现才进入 ESP32 固件。后续整理已删除 Serde 和 MessagePack 的板级体积补丁，当前使用原版库；下方编译清单是整理前的历史快照。

ESP-IDF 的 Rust FFI 绑定也由构建时的 bindgen 生成，使 Rust 能调用 IDF C API；这是另一类生成源码。

③ Node runtime 的 [build.rs](<../../../../core/crates/node/runtime/build.rs>) 会扫描 Core 服务源码中的路由声明，在电脑上生成下面两份 Rust 文件：

- `generated_route_catalog.rs`（本机构建产物）
- `peer_protocol_identity.rs`（本机构建产物）

协议身份生成文件的实际内容：

```rust
pub const VERSION: u32 = 29;
```

路由目录生成文件包含路由 ID、方法、绑定与权限元数据，再通过 `include!` 进入 runtime。扫描桌面服务源码生成元数据，不等于把桌面服务实现一并编译。该目录的最终体积仍需根据链接结果核对。

④ [face.svg](<../ui_port/face.svg>) 经 [face-svg.mts](<../editor/src/face-svg.mts>) 生成 [mini_face.h](<../ui_port/mini_face.h>)，内容如下。这是设备内置表情 UI 的图元，不是 Core 插件下发的任务猫素材。

```c
/* Generated from face.svg by face-svg.mts. Do not edit. */
static const face_primitive_t face_primitives[] = {
    {0, 48, 38, 10, 17, 0, 0xb1f2d8},
    {0, 112, 38, 10, 17, 0, 0xb1f2d8},
    {1, 56, 68, 64, 75, 4, 0xb1f2d8},
    {1, 64, 75, 80, 79, 4, 0xb1f2d8},
    {1, 80, 79, 96, 75, 4, 0xb1f2d8},
    {1, 96, 75, 104, 68, 4, 0xb1f2d8}
};
```

⑤ C 源码经过编译/链接后实际变为 Xtensa 指令。下面使用最终固件中同一个函数对照，汇编直接来自已保存 ELF，未经示意改写。

[operit_mini_ui.c](<../ui_port/operit_mini_ui.c>)

```c
void operit_ui_set_paired(bool paired) { if(ui.paired!=paired){ui.paired=paired;invalidate();} }
```


```asm
402c4af0 <operit_ui_set_paired>:
402c4af0:	004136        	entry	a1, 32
402c4af3:	5a1381        	l32r	a8, 4029b340 <esp_reset_reason_init+0xeb8> (3ffb6ed8 <ui>)
402c4af6:	742020        	extui	a2, a2, 0, 8
402c4af9:	220892        	l8ui	a9, a8, 34
402c4afc:	051927        	beq	a9, a2, 402c4b05 <operit_ui_set_paired+0x15>
402c4aff:	224822        	s8i	a2, a8, 34
402c4b02:	fd72e5        	call8	402c2230 <invalidate>
402c4b05:	f01d      	retw.n
```

字节码列是最终机器指令，右侧是反汇编后的助记符；比较配对状态，变化时写入并调用 invalidate。Rust 也最终变成这样的目标指令和数据，不会以 `.rs` 文本形式在设备上执行。

完整构建流程：


```
Core 插件 TypeScript + 猫素材 → ToolPkg → 在 Core 上运行/下发

PC 构建工具：prepare-{tokio,ring,msgpack,serde}.py
    → 板级第三方库 overlay
face.svg → face-svg.mts → mini_face.h
路由声明 + peer-protocol.json → build.rs → OUT_DIR 生成 Rust

手写 Rust + 宏展开/生成 Rust + std/第三方库 → rustc/LLVM → Xtensa 目标对象/rlib
UI C + ESP-IDF C/C++/汇编 + 密码后端 → Xtensa GCC/Assembler → 对象/静态库
    → ldproxy/链接器 + Rust fat LTO + 未使用节删除 → operit-esp32 (ELF)
    → espflash save-image → 应用 .bin → 检查实际 .bin 与 app 分区
    → 合并 bootloader、分区表与应用 → 完整刷写镜像
```

当前构建入口 [build.mts](<../editor/src/build.mts>) 会准备 overlay、设置 Xtensa C 编译环境、调用 Cargo、检查 ABI，再实际生成 bin 和核对分区。仅执行 cargo build 不会完成后两项。

## UI 文件职责与体积

`ui_port` 是板级屏幕、交互和绘图适配的合理位置。目前 `operit_mini_ui.c` 混合了基础绘图、字形排版、页面节点构造、交互和调试 JSON，1,077 行、65,280 B 源码。这些职责可以拆分，但拆文件本身不会自动减少 Flash。

基线 ELF 中与 UI 对象交叉匹配的 49 个函数指令共 13,025 B，不含常量、字体、向量数据和链接填充，不能视作整个 UI 的占用。

| 函数或资源 | 字节 |
|---|---:|
| `scene` 页面布局 | 4,384 |
| `paint_nodes` 节点绘制 | 2,165 |
| `operit_ui_set_touch` 交互 | 1,853 |
| `text` 文字绘制 | 439 |
| `rect` 矩形填充 | 71 |
| `vector_line` 直线 | 114 |
| 全部字体位图和字形表 | 121,145 |

字体保留 4,158 个文字字形和 10 个大数字字形。屏幕为 320×240 RGB565，绘制缓冲为两行、1,280 B。目标 UI 静态符号合计 10,181 B，含状态、调试缓冲和条带，不含对齐；代码使用 10 KiB 静态 RAM 断言约束。

远程组合已用 Rust feature 排除独立应用模块，但 C 页面仍按运行时 `remote_display` 分支选择。聊天和插件列表等页面是否能进一步排除，需要检查编译可达性；不能用当前页面是否显示来判断。

后续 UI 库试验应分别量化实际应用镜像、目标 RAM、栈和绘图开销，保持字符、画面和交互覆盖。库是否适合维护，与它在该板卡上是否减少资源占用，须分别评估。

## 复现完整固件检查

在仓库根目录，按板卡 README 准备工具链后运行：

```powershell
node --experimental-strip-types edge/boards/esp32/editor/src/build.mts --firmware-check --remote-display
```

基线应用镜像为 **2,889,056 B**，固定应用分区为 **3,997,696 B**。ELF SHA256 为 `8cc51ee07e8261a3ed8ca158c5036ef4c0b0fdd8fc1d3c17e39caa016679dfa5`。此处记录的是该提交和工具链下的构建快照。
