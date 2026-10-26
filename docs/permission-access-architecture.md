# 权限、AI 能力与执行边界

工具执行按以下层级逐层收紧。所有入口共用 runtime 的执行检查，Host 决定实际可用的系统能力。

| 层级 | 负责什么 | 边界 |
| --- | --- | --- |
| App Runtime Sandbox | 进程所在的系统账号、应用沙盒、容器或虚拟机 | 由操作系统与部署环境提供 |
| Host Authorization | 文件、终端、网络及平台能力的真实授权 | runtime 与用户批准都不能提升 Host 权限 |
| AI Capability Limit | 用户选择的 `AiPermissionMode` | 决定工具读写限制和工作区写入范围 |
| User Tool Approval | 对具体工具调用的批准 | 批准只放行对应调用或会话范围 |

## AI 模式

runtime 保存 `ReadOnly`、`WorkspaceWrite` 或 `Full`，工作区路径由执行上下文单独提供。

| 模式 | READ | 工作区内文件 WRITE | 工作区外文件与非文件 WRITE |
| --- | --- | --- | --- |
| `ReadOnly` | 允许 | 请求批准 | 请求批准 |
| `WorkspaceWrite` | 允许 | 允许 | 请求批准 |
| `Full` | 允许 | 允许 | 允许，仍受 Host 授权限制 |

读取交给 VFS 和 Host Authorization，不要求读取路径位于当前工作区。`Full` 不赋予 root、管理员权限或远端节点没有的能力。

产品设计中的应用内沙盒开关由模式派生：`ReadOnly`、`WorkspaceWrite` 开启，`Full` 关闭。当前没有真实的应用内沙盒执行边界，界面必须明确显示这一状态。工作区文件检查不能作为终端已经隔离的证明。

## 工具执行链路

```text
Flutter / CLI / 已认证远端请求
  -> runtime 执行上下文（AiPermissionMode、workspacePath）
  -> executor 解析并声明本次访问效果
  -> AIToolHandler 检查模式、路径与所需批准
  -> 具体工具 / VFS
  -> Host
```

工具声明 `ToolAccessSpec`，包含本次调用的 `ToolEffect.READ` / `WRITE` 和 `ToolBoundary`。声明阶段只能解析参数，不能执行动作。同一工具的不同参数可以产生不同效果；不能通过工具名或命令文本猜测读写权限。

- `None`：终端、网络、系统操作等非文件调用，按实际效果检查。
- `FilePath`：解析 `path`，按该路径的访问效果检查。
- `FilePair`：分别检查 `source`、`destination` 的访问效果。

文件路径经 `PathMapper.resolve` 和 `PathMapper.relativePath` 判断归属。受限模式下，写入当前工作区之外的路径需要批准；非法路径和参数直接报错。`move` / `copy` 按各自的源、目标声明分别判定，不通过反复匹配路径片段决定归属。

终端信息和屏幕读取属于 READ；创建会话、执行命令、输入和关闭会话属于 WRITE。shell 直接受 Host 的系统边界限制，命令字符串检查不能构成沙盒。

## ToolPkg 与批准

ToolPkg 是安装和配置资源；`packageName:toolName` 是 AI 可调用的具体包工具。插件、Skill、MCP 的安装、删除、启停与配置由本机 owner 管理。

PackageTool 的外层声明不代表脚本内部全部读写效果。脚本中的 `Tools.*` 进入具体 Host bridge 工具后，执行同一套模式、效果和路径检查。不能把批准整个包当作后续操作的权限证明，也不建立独立的包内审批系统。

```text
PackageToolExecutor
  -> 插件脚本 Tools.Files.write
  -> toolCall("write_file")
  -> AIToolHandler
  -> VFS / Host 文件接口
```

## 远端访问

Peer session 验证配对凭证、认证会话和当前通道授权，Router 继续检查业务授权。Web Access 与远程 CLI 也必须进入同一 runtime 执行链路。

Space 成员资格、连接认证和本机系统权限分别判定。加入空间不会授予本地文件、终端、root 或管理员能力；客户端自报的节点身份不能成为授权依据。空间路由见 [Link 架构](link-access-architecture.md)。

## Host 与界面职责

Host descriptor 提供平台、权限等级、隔离状态、能力、工作区根目录以及 `onboardingRequirements`。每个权限项包含标题、说明、能力 ID、状态和申请动作。

欢迎页读取 Host 的要求，调用 `request(hostId, requirementId)`，再读取实际状态。`Satisfied`、`Missing`、`Unavailable` 由 Host 校验；按钮点击成功不能证明权限已获得。系统授权申请与平台实现集中在 Host 兼容层，Flutter 按数据渲染。

设置页按现有分类呈现：

- 工具与权限：AI 模式、Host 授权状态与扩展管理入口。
- 工作区：工作区列表及 Host 允许展示的路径。
- 访问入口：配对设备、远端 session、连接来源与工作区摘要。

外部部署器可以通过系统账号、挂载目录、容器或虚拟机限制整个 Host。内部权限设置只描述并收紧现有能力。

## 维护入口

| 位置 | 职责 |
| --- | --- |
| [Host API](../core/crates/foundation/host-api/README.md) | Host 描述、能力和授权接口 |
| [AIToolHandler](../core/crates/tool/services/src/tools/AIToolHandler.rs) | 模式、访问效果、工作区和批准检查 |
| [ToolExecutionManager](../core/crates/tool/services/src/ToolExecutionManager.rs) | invocation 流转与执行上下文 |
| [PathMapper](../core/crates/tool/services/src/files/PathMapper.rs) | VFS 解析和结构化路径归属 |
| [Peer dispatch](../core/crates/node/runtime/src/peer/dispatch.rs) | 认证通道准入 |
| [平台 Host](../hosts/README.md) | 系统授权与具体能力执行 |

权限执行检查必须位于 Core 与 Host 的实际调用路径。Flutter、CLI 和 Web Access 负责展示与发起请求；Link 协议层负责传输契约。
