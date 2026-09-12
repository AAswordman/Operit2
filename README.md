<h1 align="center"><img src="https://raw.githubusercontent.com/luojiaping/Operit2/bf09634a6b1bc39dc0a0655d1ef9f3c6f2fc75fb/docs/assets/operit-logo.svg" width="64" alt="Operit logo" valign="middle"> Operit2</h1>
<p align="center"><sub>一个 Agent 核心，连接你的设备空间。</sub></p>
<p align="center"><strong>预览版本</strong> · <a href="https://github.com/luojiaping/Operit2/blob/bf09634a6b1bc39dc0a0655d1ef9f3c6f2fc75fb/docs/Operit2%E6%8A%80%E6%9C%AF%E7%99%BD%E7%9A%AE%E4%B9%A6.pdf">阅读技术白皮书</a></p>

<p align="center">
  <img src="https://raw.githubusercontent.com/luojiaping/Operit2/bf09634a6b1bc39dc0a0655d1ef9f3c6f2fc75fb/docs/assets/operit-device-space-concept-v7.png" alt="Operit2 设备空间概念图" width="100%">
</p>

Operit2 是面向个人用户的开源跨设备 Agent 项目。它希望让手机、桌面和云端设备各展所长，让对话、任务与上下文在个人设备空间中延续。

项目源于 Operit 的 Android Agent 实践，目前正打磨多端同步、跨设备执行和恢复体验。项目初衷、工程架构与长期方向见 [Operit2 技术白皮书](https://github.com/luojiaping/Operit2/blob/bf09634a6b1bc39dc0a0655d1ef9f3c6f2fc75fb/docs/Operit2%E6%8A%80%E6%9C%AF%E7%99%BD%E7%9A%AE%E4%B9%A6.pdf)。

> 预览阶段，底层结构、数据格式、插件契约和跨设备流程仍可能发生破坏性更新。

## 设备如何协作

每个运行 Core 的实例称为 `CoreNode`，通过本机 `Host` 使用文件、终端、浏览器等能力。`Space` 组织节点之间的协作与持久化同步，`Binding` 记录任务下一步由哪个节点继续。

Space 中的节点地位对等。Linux 云端设备可以因长期在线或具备合适能力而承担更多任务，但不会因此成为固定的主节点。加入 Space 也不会自动继承其他设备的系统权限。

任务交接以工具结果已持久化、下一轮模型请求尚未开始为边界。目标节点取得必要的同步记录后，再继续后续工作。这种接续依赖保存的上下文和任务事实，不搬迁正在运行的模型请求、终端进程或浏览器会话。

我们希望逐步实现这样的体验：在手机上发起任务，由合适的设备持续执行，在桌面上查看或批准，最后回到手机接收结果。主对话保留发起设备的交互归属，子任务在执行节点推进，其他设备通过异步同步了解进展。完整体验仍在完善中。

## 当前能力

### 单节点 Agent

每个 CoreNode 都可以独立运行自己的 Agent 工作流，具体能力取决于平台 Host、模型 Provider 和本地配置。当前仓库已经包含：

- 对话、会话分支、消息管理、附件、角色卡、角色群组和提示词配置；
- Provider 配置、模型参数、工具调用、请求队列和本地模型目录；
- 工作区、文件操作、项目模板、命令和备份/导入导出；
- 终端会话、PTY 输入和输出流；
- 网页访问、浏览器自动化、工作区浏览器和运行时 WebView 会话投影；
- 记忆、摘要、语音识别、语音合成以及其他内置工具。

这些能力并不在所有平台完全相同。真正能否执行某项操作，要看目标节点的 Host 描述、系统权限、已安装服务和当前用户批准。

### 多节点连接与 Space

CLI 已经提供配对、发现、连接、session、传输方式和 Space 成员管理入口；节点之间使用 Link Access 建立认证连接，再由 PeerLink 承载 Space 请求和同步流量。适合在本地或受控局域网环境中试验：

```powershell
operit2 cli link serve --bind <address:port> --token <strong-token>
operit2 cli link discover
operit2 cli link connect <url> --token <token> --save <session-name>
operit2 cli link space join <session-name>
operit2 cli link space show
```

命令参数以 `operit2 --help` 和 `operit2 cli` 的实际输出为准。不要使用默认开发 token 对公网监听，也不要把 token、私钥或真实业务数据放入公开日志和截图。

### 插件、Skill、ToolPkg 和 MCP

Operit2 的扩展面已经从“内置工具集合”逐步抽象为可管理的运行时与 SDK：

- 使用 JavaScript/TypeScript 编写 Package 和 ToolPkg；
- 使用 JavaScript bridge、Wasm runtime 和 Compose DSL 构建工具与界面；
- 使用 Skill 提供可导入、可见性可控的工作流和知识资源；
- 连接 MCP server，管理 MCP 配置、工具和本地 MCP 进程；
- 使用插件市场和包管理命令安装、启用、停用、查看和执行扩展；
- 通过 Rust SDK、TypeScript 声明和 codegen 提供插件开发入口，接口仍在演进。

后续将进一步完善运行时、Host 能力、权限、兼容版本和状态范围的声明，让扩展能在兼容节点上执行或接续。当前请按目标平台验证插件及其依赖，跨设备可移植性仍是重点建设方向。

插件作者入口见 [`plugins/docs/README.md`](plugins/docs/README.md)，公共 SDK 说明见 [`core/crates/plugin/sdk/README.md`](core/crates/plugin/sdk/README.md)。

### Web Access

Web Access 让浏览器访问一个已运行的 CoreNode；打开页面不会自动让浏览器成为 Space 中的独立节点。浏览器 Host 与 WebAssembly 运行时是另一条工程路径，两者的能力和部署方式需要分别看待。

本地开发方式见下方“Web Access 开发”，访问与部署说明见 [`apps/web_access/README.md`](apps/web_access/README.md)。

### 数据备份与迁移

Operit2 把迁移视为个人设备连续性的一部分。CLI 已提供身份、存储路径、快照导出/恢复、备份检查以及 Operit 一代快照检查入口：

```powershell
operit2 cli identity list
operit2 cli storage paths
operit2 cli export snapshot <snapshot.zip>
operit2 cli backup inspect <snapshot.zip>
operit2 cli backup restore <snapshot.zip>
```

快照、配置和身份数据的具体范围以当前命令帮助和格式版本为准。活动中的进程和实时会话仍然属于原节点，不能把“有备份”理解成已经完成了所有运行时的无缝迁移。

## 平台与访问面

| 入口或平台 | 在当前架构中的位置 | 当前边界 |
| --- | --- | --- |
| Flutter App | 移动端和桌面端的主要图形访问面，也可以承载一个 CoreNode | Android、Windows、Linux、macOS、iOS、OpenHarmony 和 Web 的 Host/构建条件不同 |
| Rust CLI/TUI | 本地 CoreNode 和运维/开发入口 | 当前 CLI Host 主要覆盖 Windows、Linux 和 macOS |
| PB_SBC01_H3 | 运行完整 Core 的 Linux 硬件主控入口 | 使用 Linux Host 加板卡 Host 能力，应用入口位于 `apps/pb_sbc01_h3` |
| Web Access | 访问某个已运行 CoreNode 的浏览器入口 | 不是自动加入 Space 的浏览器节点，也不是中心化 Agent Server |
| WebAssembly/browser Host | 浏览器运行时的本地能力边界 | 与 Web Access 访问面分开，具体能力取决于浏览器和当前 Web 构建模式 |
| Linux 云端设备 | Space 中的普通 CoreNode | 长期在线可以承担更多任务，但不因此拥有中心身份 |
| ESP32 Edge Node | 设备侧 Edge Service 能力节点 | 不是完整 CoreNode，不持有 OperitApplication、Chat、Store、Identity 或 Space 同步 |
| Server | 未来的部署形态和 Host 方向 | 当前仓库还没有可以直接发布的完整 Server 产品 |

仓库包含多平台 Host 适配路径，但“能够构建”不等于“已经完成跨设备互操作验证”。平台构建、签名和发布条件请以 [`BUILDING.md`](BUILDING.md) 与对应 workflow 为准。

## 用户控制与安全边界

Operit2 采用从外到内逐层收紧的能力模型：

```text
0. App Runtime Sandbox
   虚拟机、容器、系统账号、Android 应用沙盒或服务器部署边界

1. Host Authorization
   操作系统真正授予本机 Host 的文件、终端、网络和系统能力

2. AI Capability Limit
   用户为 AI 选择的 ReadOnly、WorkspaceWrite 或 Full 能力模式

3. User Tool Approval
   用户对具体工具调用的允许、询问或禁止
```

几个必须保持的原则：

- Space 成员资格不会授予其他节点读取本机文件、使用本机终端或获得管理员/root 能力；
- API key、设备私钥、配对密钥、session 和本地平台权限默认属于节点本地，不会因为加入 Space 自动复制；
- `Full` 只表示 AI 能力限制放宽，不表示 Host 提权，也不能创造操作系统没有的能力；
- 应用内部的 sandbox 字段和 UI 不能冒充真实的操作系统级隔离边界；
- 对外监听 Link 或 Web Access 前，应使用强 token、受限监听地址、TLS/防火墙和脱敏日志；
- 用户始终拥有是否同步敏感数据、是否批准具体动作以及是否让节点参与协作的决定权。

完整边界说明见 [`docs/permission-access-architecture.md`](docs/permission-access-architecture.md) 和 [`hosts/README.md`](hosts/README.md)。

## 预览阶段与演进方向

Rust Core、平台 Host、节点连接、持久化同步和插件运行时已经形成工程基础。当前优先修复问题、打磨稳定性，尤其关注跨设备执行的取消、重连、资源释放和失败恢复。

以下方向仍在设计、完善或验证中：

- 结合节点健康状态（Health）、长期执行记录和用户偏好选择合适的设备；
- 在适合的任务上引入备用节点、受控并行和子任务拆分；
- 完善插件 SDK 的兼容声明与跨节点接续能力；
- 改善新设备加入、数据恢复和长期在线部署的使用体验；
- 验证节点数量增加后的发现、路由与同步成本。

当前以个人设备空间为主要场景，尚未验证数千节点调度，也不以企业组织治理为产品目标。长期运行品质仍需持续测试，Rust 本身不构成无资源泄漏的保证。更详细的设计取舍见 [技术白皮书](https://github.com/luojiaping/Operit2/blob/bf09634a6b1bc39dc0a0655d1ef9f3c6f2fc75fb/docs/Operit2%E6%8A%80%E6%9C%AF%E7%99%BD%E7%9A%AE%E4%B9%A6.pdf)。

## 快速开始

### 环境要求

常用开发入口需要：

- Rust stable 和 rustup；
- Flutter SDK 与 FVM，用于 Flutter App；
- Node.js，用于 Web Access 开发代理；
- Python 3，用于构建、发布和辅助脚本；
- 对应平台的原生构建工具。

完整环境、签名、发布和平台差异见 [`BUILDING.md`](BUILDING.md)。不要把签名文件、API key 或发布 token 提交到仓库。

### CLI/TUI

从仓库根目录执行：

```powershell
cargo check --manifest-path apps/cli/Cargo.toml
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- --help
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- cli version
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- tui
```

没有参数时，`operit2` 默认进入 TUI。更多入口可以通过以下命令查看：

```powershell
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- cli
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- cli link
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- cli web
```

### Flutter App

从 `apps/flutter/app` 目录执行：

```powershell
fvm install --skip-pub-get
fvm dart pub get --enforce-lockfile
fvm flutter analyze
fvm flutter run -d windows
```

`windows` 只是示例设备名。Android、Linux、macOS、iOS、OpenHarmony 和浏览器模式需要各自的平台工具链，且 Host 能力并不相同。

### Web Access 开发

本地 Flutter Web 开发需要两个终端：

```powershell
# 终端一：apps/flutter/app
fvm flutter run -d web-server --web-hostname 127.0.0.1 --web-port 4835

# 终端二：仓库根目录
node tools/dev_web_access_proxy.mjs --upstream-port 4835 --listen-port 4836
```

然后打开 `http://127.0.0.1:4836`。如果端口被占用，可以同时修改 Flutter 上游端口和代理的 `--upstream-port`，但要保持两者一致。

## 仓库结构

```text
apps/
├── cli/                 Rust CLI/TUI 入口
├── pb_sbc01_h3/         PB_SBC01_H3 硬件主控入口
├── flutter/app/         Flutter App 入口
├── web_access/          Web Access 前端边界和共享 bundle
└── server/              Server 形态预留目录

core/
├── crates/              Rust Core 各领域 crate
├── CRATE_BOUNDARIES.md  crate 依赖方向和职责边界
└── examples/            Provider 和插件 SDK 示例

hosts/                   Android、Windows、Linux、Apple、Web 与 boards Host 实现
plugins/                 ToolPkg、Skill、SDK 类型和插件开发工具
tools/                   构建、发布、Web 和开发辅助脚本
docs/                    架构、权限、Link、迁移和版本文档
```

## 文档入口

- [构建与发布](BUILDING.md)
- [贡献指南](CONTRIBUTING.md)
- [Core crate 边界](core/CRATE_BOUNDARIES.md)
- [Core 领域结构](core/crates/README.md)
- [CoreNode、Space 与 Binding](docs/core-node-space-binding-architecture.md)
- [Link、Access 与 Space 边界](docs/link-access-architecture.md)
- [权限、AI 能力与 sandbox 边界](docs/permission-access-architecture.md)
- [平台 Host 实现边界](hosts/README.md)
- [Web Access 前端与部署要求](apps/web_access/README.md)
- [插件作者文档](plugins/docs/README.md)
- [版本、tag、渠道与发布资产](docs/release-versioning.md)
- [当前 crate 拆分与迁移计划](docs/core-module-crate-layout.md)

架构文档中标注为“目标”“计划”或“演进方向”的内容，不代表所有平台已经实现。判断当前实际行为时，请以源码、命令帮助输出、构建结果和目标平台运行结果为准。

## 参与贡献

Operit2 仍然是一项长期工程。我们欢迎代码、文档、测试、平台 Host、插件、Skill、MCP 集成和真实使用反馈。

如果你要修改跨设备能力，请尽量守住三条边界：

1. 本地 Host 权限不因 Space 成员关系自动继承；
2. 同步范围和数据归属必须能够被用户理解；
3. 功能可以演进，但不能静默丢失用户的任务连续性和可恢复记录。

开始前请阅读 [`CONTRIBUTING.md`](CONTRIBUTING.md)。涉及协议、持久化、Binding、Host capability 或插件契约的改动，也请同步更新对应架构文档和测试。

## 许可证

仓库根目录的 [`LICENSE`](LICENSE) 当前为 GNU Affero General Public License v3.0（AGPL-3.0）。具体 crate、插件、ToolPkg、Web bundle、vendored 代码和第三方依赖可能附带自己的许可证或元数据；使用、分发或修改具体组件前，请同时核对该组件目录中的声明。
