# Edge V1 收尾记录（更新于 2026-10-30）

首次验收基线：`3d6238a4`（main）。本轮在 Windows 上收尾 SDK 接入和验收入口。
PR 已 rebase 到 `06784b03`；最新基线复验见下节，旧基线结果仅作历史记录。
历史 macOS 模拟器验收见 [验收记录](acceptance.md)；历史固件和硬件数据见
[ESP32 验证范围](../boards/esp32/README.md#verification-and-current-limitations)。
历史结果不能替代本轮结果，也不能代表当前固件已经烧录。
最新基线的固件容量阻塞已修复，Edge 软件验收通过；真机交付尚未完成。
CLI 同步栈溢出的修复及代理配置复验见下文。

## 交付范围

Edge V1 提供原生设备服务、共享身份与验证码配对、加密通信、Space 申请与审批、
权限校验、撤销、节点状态持久化，以及按产品选择的聊天显示、语音传输和本地 agent。
厂商通过 `operit-edge-sdk` 接入；板级驱动、产品 UI、模型配置和物理动作校验由产品负责。
SDK 的默认、`space`、`http-agent` 和 `scene` 功能按需选择。

ESP32 产品的独立文本聊天、远端 Core 聊天和语音编排继续使用已有产品实现。
SDK 新增的通用 agent 并未因此自动装进 ESP32；设备不执行完整 Core 或 ToolPkg。
屏幕图片占位、没有完整本机键盘、没有主动 mDNS 浏览均为当前产品边界。

## 本轮变更

- 统一设备预算验收先执行 SDK workspace 全功能测试及默认和可选功能全部八种组合的依赖审计。
- ESP32 Host 从活动 station/AP 网络接口读取当前 IPv4 地址，连接坐标查询不依赖 mDNS 浏览或缓存的 UI 地址。
- 集成测试按新 CLI 的设备名、空间名与待审批设备名选择目标，并继续检查原请求 ID、审批版本冲突和取消恢复。
- Cargo 的烧录 runner 移除机器专属 COM27；编辑器仍只自动选择唯一 USB 串口。
- 监听文档与现有自动端口、实际绑定地址、地址占用后重试一次的实现对齐。
- 记录 SDK 装配边界：`SpaceDevice::assemble` 同步初始化；ESP32 保留后台初始化与失败重试，模拟器保留分阶段启动。
- 拆开模拟器 UI action 与语音/IPC 的异步执行帧；将审批的 claim、outcome 和回复跨阶段保存在 Box 中，避免固定任务栈同时承载多段大型解码与审批流程。
- 共享身份初始化优先读取 typed record，避免每次鉴权展开无关的设备展示数据；节点存储的迁移、外层解码与回调保留独立栈边界。
- 权限命令的 JSON Value 解码统一复用已有文本解码器，减少固件重复代码。增加每条命令的临时序列化缓冲，保留原有数据形状、旧格式兼容及歧义拒绝检查，内存影响纳入固定 arena 集成验收。
- CLI `space sync` 独立装箱同步 future，避免复制进命令和会话的异步执行帧，修复 Windows 调试版主线程栈溢出。
- 同步进度集成断言与最新上游“小批量等待后仍隐藏”的行为对齐，大批量及阈值转换继续由现有进度测试覆盖。
- 本地统一集成场景清理继承的全局 HTTP 代理，Cargo 下载代理单独保留；另行复验带全局代理且绕过回环地址的独立聊天场景。

现有设备服务 trait、聊天显示契约、Link 编码和权限规则沿用共享实现。
没有另建配对、审批或业务 HTTP 通道。

## 最新基线复验（`06784b03`，2026-10-30）

未修改前在最新 `main` 上复现 `.flash.text` 超限 3,226 字节。
三个小型映射改成逐项插入：旧版 `nodeIds` 的默认身份映射、四个内置角色和
审批路由 BFS 的起始距离。避免为这些构造保留批量收集器的临时排序缓冲和
独立排序代码，最终代码段减少 8,524 字节；映射内容、节点顺序、权限和协议保持一致。
4 MB 分区、TLS 校验、panic 隔离、256 KiB arena 和任务栈预算保持原值。

- SDK 全功能 23 项测试及全部 8 种功能组合依赖审计通过。
- 存储 `node-state` 单元测试 192 项通过，1 项原有压力测试忽略；包含历史审批格式升级回归。
- ESP32 release 编译、链接、Tokio/ring 最终 ELF Xtensa ABI 检查与 4 MB 镜像导出通过。
- 模拟器固件模块 62 项测试和 TypeScript 检查通过。
- Preferences 布局与升级 19 项测试通过；Core `device_space` 108 项及同步进度 9 项测试通过。
- 编辑器和实际内存/受保护栈边界探针 116 项通过。
- 带固件的统一验收完整通过：当前 CLI 和 release 模拟器重建后，配对审批与重启恢复、
  独立聊天额外 12 轮及 Core/设备退出重启、远端聊天与存储故障及二进制语音恢复均通过。
- Windows 调试版 CLI `space sync` 在未继承代理时也复现过主线程栈溢出；
  将同步 future 独立装箱后，上述场景全部通过。
- 另行保留 `HTTP_PROXY` / `HTTPS_PROXY=http://127.0.0.1:7890`，
  设置 `NO_PROXY=127.0.0.1,localhost,::1`，独立聊天 12 轮与重启场景通过。
  这验证了配置代理且回环地址直连的本地链路，不代表外部代理服务或远端模型已验收。

最后一轮统一验收的 arena 高水位：

| 场景 | 峰值 / 262,144 字节 |
| --- | --- |
| 配对、审批、重启恢复 | 168,952 |
| 独立聊天、额外 12 轮、Core 退出、设备重启 | 225,824 |
| 远端聊天、存储故障、二进制语音恢复与双向审批 | 215,608 |

单独复验独立聊天时最高采样为 234,920 字节；带代理且回环直连的复验为 226,104 字节。
以上所有场景的采样分配失败均为 0。任务栈继续按 16/24/20 KiB 受保护预算验收。

| 资源 | 占用 / 上限 | 余量 |
| --- | --- | --- |
| factory 应用分区 | 3,959,312 / 3,997,696 字节 | 38,384 字节 |
| `.flash.text` 映射区 | 3,337,006 / 3,342,304 字节 | 5,298 字节 |

以上数据来自最新基线加本次修复后的实际固件；构建产物不纳入 Git。
代码段余量仍然有限，后续更新必须重新构建并检查两种容量限制。

| 产物 | SHA-256 |
| --- | --- |
| ELF `operit-esp32` | `3ec09572bac486a2cf17a171361f7380d6106f7b11c2b83b87684bcc0bdd92da` |
| 应用 `operit-esp32.bin` | `456fa22357e3d4138db79159e252cc7dc57ccdb3b080906e9c8236b25417a049` |
| 合并镜像 `operit-esp32-4mb-full.bin` | `1c14d8e31172969f7cad9d4459141bde2afa5cc1acfa7194872170003e1daa46` |

本机日志位于 `$env:TEMP`：`operit-edge-20261030-firmware-maps.log`、
`operit-edge-20261030-gate-final.log`、`operit-edge-20261030-store.log`、
`operit-edge-20261030-layout.log`、`operit-edge-20261030-space-verified.log`、
`operit-edge-20261030-progress.log` 和 `operit-edge-20261030-proxy.log`。
复现命令沿用下文；统一验收使用 `npm run test:device-budget --prefix edge/boards/esp32/editor -- --firmware`。
本轮未连接交付板，未执行烧录或真机验收。

## 历史 Rebase 后复验（`1ced7f40`）

当时 PR 直接 rebase 到上游 `main` 的 `ec9702ae`，没有保留 merge commit。
下述 Rust 复验针对 `1ced7f40`；后续 `ec9702ae` 仅合入 Android ABI 文件修改，
没有更改本次 Rust/ESP32 验证路径。
本节覆盖下方首次基线的软件完成结论；首次基线的数据和镜像哈希仅作为历史记录。

- SDK 全功能 23 项测试和 8 种功能组合依赖审计通过。
- `node-state` 存储单元测试 192 项通过、1 项原有压力测试忽略；布局与升级测试 19 项通过。
- Core `device_space` 行为测试 108 项通过。
- ESP32 固件在链接时失败：`.flash.text` 超出原有映射区 **3,226 字节**。
  不能沿用首次基线“固件构建通过”的结论；需要修复该容量回归后再次完整验收。
- 带固件的统一验收在构建阶段停止，未完成后续端到端场景。

新基线复验日志位于本机 `$env:TEMP`：`operit-edge-main-update-gate.log`、
`operit-edge-main-update-store.log` 和 `operit-edge-main-update-space.log`。
没有扩大 4 MB 分区或提交本地构建产物。

## 首次基线验证（`3d6238a4`）

固定预算保持为 256 KiB Rust arena，主任务、Space worker、网络任务栈分别为
16/24/20 KiB；没有扩大预算、移除功能、关闭 panic 隔离或取消 TLS 证书校验。

| 项目 | 本轮结果 |
| --- | --- |
| SDK workspace `--all-features` | 23 项通过 |
| SDK 依赖边界 | 默认及 `space` / `http-agent` / `scene` 全部 8 种组合通过 |
| 存储单元测试 `node-state` | 190 通过、0 失败；原有 ultra compaction 压力测试 1 项忽略 |
| Preferences 布局及升级 | 19 项通过 |
| ESP32 release 固件 | 编译、链接、最终 ELF Xtensa ABI 检查及 4 MB 镜像导出通过 |
| C/Wasm UI | 构建通过；共享 UI 源哈希 `d24f1dd9ff94` |
| TypeScript 检查 | 通过 |
| 共享固件模块的模拟器测试 | 62 项通过 |
| 编辑器及真实堆/栈边界探针 | 116 项通过 |
| 最后一轮统一预算验收 | 全部通过；重建当前 CLI 后完成 3 个独立集成场景 |
| Core `device_space` 行为测试 | 107 项通过，单线程执行 |
| 真机烧录、音频与持续运行 | 未执行；无 USB 交付板 |

最后一轮统一验收的实测 arena 高水位：

| 场景 | 峰值 / 262,144 字节 |
| --- | --- |
| 配对、审批、重启恢复 | 167,216 |
| 独立聊天、额外 12 轮、Core 退出、设备重启 | 235,016 |
| 远端聊天、存储故障、二进制语音恢复与双向审批 | 215,384 |

所有采样状态的分配失败计数均为 0；受保护的任务栈通过越界探针和实际流程验证。
远端场景覆盖过期审批不得作用于新申请、离线撤回跨两端重启后恢复、配对凭证保留、
长 UTF-8 消息、90 行分页历史、Core 插件/场景调用、STT/TTS 多轮恢复和音频背压。
65 秒 PCM 指字节量，语音和模型提供者使用本地确定性 fixture，不代表真实服务识别质量
或实际麦克风/扬声器已经验收。

本机完整日志位于 `$env:TEMP`：`operit-edge-closeout-gate-final.log`、
`operit-edge-core-space-closeout-tests.log`、`operit-edge-store-unit-closeout.log`、
`operit-edge-store-layout-closeout.log` 和 `operit-edge-firmware-final.log`。

复现入口（从仓库根目录运行）：

```powershell
npm run test:device-budget --prefix edge/boards/esp32/editor
npm run build:firmware --prefix edge/boards/esp32/editor
cargo test --locked --manifest-path core/crates/persistence/store/Cargo.toml --no-default-features --features node-state --lib
cargo test --locked --manifest-path core/crates/persistence/store/Cargo.toml --no-default-features --features node-state --test preferences_upgrade --test preference_layout
cargo test --locked --manifest-path core/crates/node/runtime/Cargo.toml --lib device_space -- --test-threads=1
```

也可给统一验收命令加 `-- --firmware`，把固件构建并入该次验收。
本机配置了 `EMSDK`，使用仓库 `.venv` 中的 Python，以及 ESP-IDF 5.3.2 和
已准备的 Xtensa Tokio/ring overlay。Core 行为测试的原生模型依赖使用已缓存的源码，
由 `OPERIT_MNN_SOURCE` 和 `OPERIT_LLAMA_SOURCE` 指定；这些本机路径不写入项目配置。

首次基线中，调试版 CLI 继承全局 HTTP/HTTPS 代理时，`space sync` 曾出现主线程
栈溢出，清除代理后当时通过。2026-10-30 在最新基线无代理环境中也复现了此栈溢出，
已通过独立装箱同步 future 修复；完整统一验收及带代理/回环直连的补充复验见最新章节。
本地模型和语音仍为确定性 fixture，不能扩大为真实远端模型或语音服务已验收。

### 固件资源与可追溯性

本轮产物来自 `fix/edge-sdk-v1-closeout` 分支相对 `3d6238a4` 的收尾源码，
在提交前完成构建，以下 SHA-256 标识实际验收产物。ELF 位于本机构建目录，
镜像位于 `edge/boards/esp32/dist/`，构建产物不纳入 Git。

| 资源 | 占用 / 上限 | 余量 |
| --- | --- | --- |
| factory 应用分区 | 3,961,680 / 3,997,696 字节 | 36,016 字节（约 35.2 KiB） |
| `.flash.text` 映射区 | 3,339,390 / 3,342,304 字节 | 2,914 字节（约 2.85 KiB） |

代码段余量尤其紧，后续增加功能必须重新验证链接、镜像大小和固定内存预算。
镜像大小不能代替代码段映射区检查；4 MB 分区布局保持不变。

| 产物 | SHA-256 |
| --- | --- |
| ELF `operit-esp32` | `ec4fccfcba8fb9716c07a620eb9b32aa0a590203a31a088f378a5cbbc454f86d` |
| 应用 `operit-esp32.bin` | `481d12dd14478f9d136702b8f48af92ec9046f7e8a4f53a8c106811b2e25ca28` |
| 合并镜像 `operit-esp32-4mb-full.bin` | `f5775ee85c061da092c8e5caf9e03694dfaf5686b0dfe4b8b905a83a3896afa8` |

## 真机交付检查

本机只发现蓝牙串口，没有检测到 USB ESP32，因此没有执行烧录或真机操作。
需要在交付板上记录固件提交号、ELF/镜像校验值、板型、实际分区及资源数据，再完成：

1. 首次配对、验证码错误与取消、Space 审批、授权后原生能力调用。
2. 设备和 Core 分别重启，确认身份、配对、成员权限与配置恢复。
3. 断网重连、请求取消、撤销后拒绝访问、本地退出再申请加入。
4. 独立聊天、空间聊天、历史窗口和连续长消息，观察 UI 响应与内存。
5. 实际麦克风采集、Core STT、回复、TTS 和扬声器播放，记录真实服务及音频格式。
6. 连续运行及重复语音、重连和重启，记录运行时长、空闲堆、最大连续块、栈高水位、
   NVS 占用和异常计数；确认没有持续增长、分配失败、队列溢出或意外重启。

构建结果必须同时记录镜像分区和 `.flash.text` 的余量。模拟器 Rust arena 不包含
ESP-IDF、Wi-Fi、TLS、C UI 和原生驱动，不能作为整板内存结论。

当前版本已通过上述新基线软件复验；仍须通过真机检查，才能标记
“ESP32 Edge V1 硬件交付完成”。
