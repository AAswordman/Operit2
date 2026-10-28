# Edge 验收记录（2026-10-29）

验收基线为 `upstream/main` 的 `3c5fcc81653999613328209bd0beafb415949c82`。Edge 分支已完成 rebase，再进行目录迁移、SDK 拆分和回归。

本次通过 macOS 上实际运行的 CLI 与 ESP32 产品模拟器验收。模拟器编译产品共用的 Rust 业务模块，使用固定 C UI 的 Wasm 构建，两个独立进程通过真实 TCP、配对、加密、空间审批和业务路由通信。

## 统一验收入口

在仓库根目录执行：

```sh
EMSDK=/path/to/emsdk npm run test:device-budget --prefix edge/boards/esp32/editor
```

需要编辑器文档列出的 Node、Python、Rust 和 Emscripten 环境。该入口重建预览、模拟器、插件资产和当前 CLI，清理可能跳过压力场景的环境变量，再执行全部场景。任意阶段失败都会终止并返回非零退出码。

本次完整执行通过：

| 项目 | 结果 |
| --- | --- |
| C/Wasm 预览构建、TypeScript 检查 | 通过 |
| 产品共用 Rust 模块 | 62 项通过 |
| 编辑器与实际堆/栈边界探针 | 116 项通过 |
| 配对、加入空间、持久化与重启 | 通过 |
| CLI 退出后的独立流式聊天、连续额外 12 轮、设备重启后聊天 | 通过 |
| 空间聊天与两边消息互通、完整 UI 操作与恢复 | 通过 |

完整互通场景同时验证聊天列表、创建和切换会话、历史分页、长 UTF-8 消息跨帧传输、取消、离线取消、退出与重新加入、审批期间存储失败后的恢复，以及重启后的状态恢复。语音场景验证 PCM/WAV、较长识别结果、多段 TTS、背压、断开重连；插件场景经过真实 Core JS 处理器和设备服务路由。

## 实际资源约束

模拟器启用固定 **262,144 字节 Rust 堆**。主线程、异步任务与阻塞任务分别使用 **16/24/20 KiB** 可用栈，并使用保护页检测越界。macOS 的页对齐不会增加可用栈预算；边界测试同时验证预算内成功与预算外失败。

| 真实进程场景 | 跨重启最大 Rust 堆峰值 | 采样状态中的分配失败 |
| --- | ---: | ---: |
| 配对、空间接入与重启 | 175,584 / 262,144 字节 | 0 |
| 独立聊天、12 轮压力与重启 | 231,224 / 262,144 字节 | 0 |
| 双向互通、语音与恢复 | 226,952 / 262,144 字节 | 0 |

这些测量覆盖受约束的 Rust 产品运行时。桌面操作系统、C/Wasm UI 和网络系统库拥有各自的内存，不计入这块 Rust 堆。

## SDK 与厂商产品

```sh
cargo test --locked --manifest-path edge/sdk/Cargo.toml --workspace --all-features
python3 edge/tools/check-sdk-dependencies.py
cargo run --locked --manifest-path edge/boards/headless_sensor/Cargo.toml -- --once
```

SDK workspace 的 23 项测试通过，覆盖设备协议、服务路由、场景渲染、本地 agent 工具闭环、未知工具拒绝、上下文与步数预算，以及 HTTP 模型响应上限和工具结果回传。

依赖审计统计正常依赖与构建依赖的不同包，排除 SDK 根包：

| SDK 功能 | 依赖包数 |
| --- | ---: |
| 默认本地 agent | 0 |
| `space` | 108 |
| `http-agent` | 25 |
| `scene` | 41 |

各功能分别检查；组合启用时不能直接相加。审计拒绝引入完整 Core 应用运行时、SQL、JavaScript、本地模型引擎和 reqwest。空间接入复用共享认证、权限、存储和 Peer runtime，仍需要对应资源；默认零依赖不能代表联网成本。

无屏产品通过 `--once` 本地工具调用，以及普通启动的真实监听端口、持久身份、状态查询、本地 agent 与正常退出检查。它仅选择 SDK 的 `space`，不引入 ESP32 UI 或 scene。温度读数是明确标记的桌面演示值，厂商可用自己的 HAL 替换工具。

## 共享实现回归

```sh
cargo test --locked --manifest-path core/Cargo.toml -p operit-node-runtime -p operit-peer-link
cargo test --locked --manifest-path core/Cargo.toml -p operit-store --no-default-features --features node-state
cargo test --locked --manifest-path hosts/common/operit-host-native-scheduler/Cargo.toml --no-default-features -- --test-threads=1
cargo test --locked --manifest-path hosts/common/operit-host-native-scheduler/Cargo.toml --no-default-features --features esp32-compat -- --test-threads=1
cargo test --locked --manifest-path edge/boards/esp32/editor/simulator/Cargo.toml --features core-integration-tests --test space_admission
```

节点运行时与 Peer Link 共 323 项通过；node-state 存储 181 项通过、1 项忽略；调度器普通模式 25 项、ESP32 兼容模式 26 项通过；跨设备类型的协议与空间接入 42 项通过。各 Cargo workspace 使用自己的 target 目录，避免不同锁文件下的动态库构建产物互相覆盖。

## 验收边界

模型和 STT/TTS 服务使用本地确定性提供方，便于复现长消息、取消与断开恢复。实际执行的是生产传输、配对、审批、权限、持久化、聊天流水线、插件处理器和 UI；本次没有调用外部付费模型，也不据此评价外部模型质量。

本次没有执行 ESP-IDF 目标固件构建、真实 ESP32 烧录或整板 Wi-Fi/TLS/C 内存和闪存测量。硬件交付仍需要按具体板型执行这些检查。此记录的通过范围是实际运行的 macOS CLI 与受资源约束的产品模拟器，以及上述 SDK 和共享模块回归。
