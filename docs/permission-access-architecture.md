# Permission / Access / Workspace Architecture

本文档定义 Operit2 的权限链路、工作区访问模型与设置页呈现方式。后续实现、评审与排查都以这里的边界为准。

这个设计替换现有以 `ALLOW / ASK / FORBID` 为核心的工具权限模型。新模型不向前兼容旧配置，不迁移旧的单工具授权数据。

## 1. Terms

```text
Boundary
  系统外层边界。
  包括真实虚拟机、容器、系统账号、移动系统应用沙盒、服务器部署边界。
  Operit 可以读取边界信息并展示，但不能把 UI 开关描述成真实 VM 隔离。

Host Capability
  Host 被操作系统或部署环境授予的能力。
  典型例子：
    Windows 普通用户 / 管理员
    Android 普通应用 / Shizuku / root
    Linux 普通用户 / root / service account
  Host Capability 是真实系统执行边界，不是产品策略层。
  软件只能在 Host Capability 之下继续收紧权限，不能绕过它。
  例如 Android 非 root 进程不能访问 /data，Operit 不能通过 UI 或工具授权绕过这个限制。
  Host Capability 由 host 注册，不由 Flutter UI 猜测。

Access User
  app 或远程连接进入 runtime 时绑定的用户身份。
  个人模式创建内置本机用户。
  企业部署由 host/server 侧创建用户并绑定 session。

Workspace Grant
  Access User 被授予的工作区范围。
  它描述用户能看到哪些工作区、能以什么模式访问这些工作区。
  企业场景下，目录授权必须在 host/server 侧执行，客户端隐藏列表不算权限控制。

AI Work Mode
  面向普通用户展示的 AI 工作模式。
  只定义三个模式：
    readOnly
    sandboxWrite
    fullAccess

Tool Capability
  工具自身声明的能力类型。
  它不是“某个 toolName 是否允许”的用户配置表，而是执行器用于判定工具行为的结构化能力。

Audit
  权限链路的执行记录。
  至少记录 user、session、workspace、mode、host capability、tool capability、decision。
```

核心关系：

```text
effective access
  = Host Capability
  ∩ Boundary
  ∩ Access User
  ∩ Workspace Grant
  ∩ AI Work Mode
  ∩ Tool Capability
```

Host Capability 是硬上限。任意一层不允许，请求就不能执行。权限判断必须落在执行链路上，不能只落在 UI 上。

## 2. Product Surface

普通用户只看到两类信息：

```text
当前模式
  只读
  沙盒读写
  完整权限

当前 Host 能力
  当前设备类型
  当前 host 权限等级
  当前可访问的工作区
  当前连接来源：本机 / 已配对客户端 / Web Access / CLI
```

高级用户可以展开完整链路：

```text
Boundary
  当前是否运行在系统级隔离边界内。

Host Capability
  host 注册的系统能力与权限等级。

Access User
  当前 session 绑定的用户。

Workspace Grant
  当前用户被授予的工作区列表与访问模式。

AI Work Mode
  当前 AI 对文件、命令、网络、外部工具的工作模式。

Tool Capability
  工具声明能力与本次执行判定。
```

设置分类不新增“权限总表”。现有 `工具与扩展` 分类保留工具、插件、MCP 与 AI 工作模式入口。现有 `工作区` 分类负责工作区路径与授权展示。现有 `访问入口` 分类负责配对设备、远程 session 与 Web Access。

欢迎页的权限授予也按同一原则处理：

```text
当前实现
  Flutter 欢迎页写死 Android 权限项。
  AndroidPlatformChannel 返回 location、bluetoothConnect、bluetoothScan、overlay、batteryOptimization。

目标实现
  欢迎页只渲染 Host 注册的 onboarding capability requirements。
  每一项是否需要展示、是否已满足、如何申请、申请后如何验证，都由 Host 提供。
  Flutter UI 不承诺权限本身，只展示 Host 的声明与校验结果。
```

Host 是每个权限项的保证方。Android 的蓝牙、定位、悬浮窗、电池优化由 Android host 检查；Windows、Linux、服务器部署也由各自 host 注册自己的能力项。UI 不能把某个权限项写死为所有平台都存在。

## 3. AI Work Modes

### readOnly

```text
目标
  让普通用户知道 AI 只能读取上下文，不能修改文件、执行写入命令或调用会改变外部状态的工具。

允许
  读取已授权工作区内文件
  列目录
  搜索代码
  读取 host capability 描述
  读取已授权连接状态

禁止
  写文件
  删除文件
  移动文件
  执行会修改系统或网络状态的命令
  写数据库
  调用外部服务的变更接口
```

### sandboxWrite

```text
目标
  让 AI 可以在工作区边界内修改内容。

含义
  这里的 sandbox 是 workspace boundary，不是 VM、容器或进程级隔离。
  UI 文案必须避免暗示系统级沙盒。

允许
  读取已授权工作区
  写入已授权工作区
  在已授权工作区内创建文件
  在已授权工作区内执行声明为 workspace-scoped 的工具

禁止
  访问未授权工作区
  访问 host 暴露但当前 user grant 未包含的路径
  跨出 workspace boundary 写系统路径
  使用 host root/admin 能力修改工作区外资源
```

### fullAccess

```text
目标
  面向开发者、设备所有者或明确授权的企业运维场景。

含义
  不再逐个工具弹窗确认。
  但 fullAccess 仍然受 Boundary、Host Capability、Access User、Workspace Grant 限制。

允许
  使用当前 host 注册并对当前 user 开放的全部能力。

禁止
  把 fullAccess 解释为绕过 host/server 授权。
  把远程客户端 UI 的选择解释为服务器权限。
```

## 4. Data Model

Host 注册自身能力：

```rust
pub struct HostEnvironmentDescriptor {
    pub id: String,
    pub displayName: String,
    pub platform: HostPlatform,
    pub privilege: HostPrivilege,
    pub isolation: HostIsolation,
    pub capabilities: Vec<HostCapability>,
    pub onboardingRequirements: Vec<HostOnboardingRequirement>,
    pub workspaceRoots: Vec<WorkspaceRootDescriptor>,
}

pub enum HostPlatform {
    Android,
    Windows,
    Linux,
    Macos,
    Web,
}

pub enum HostPrivilege {
    Normal,
    AndroidShizuku,
    AndroidRoot,
    Administrator,
    Root,
    ServiceAccount,
}

pub enum HostIsolation {
    None,
    OsAppSandbox,
    Container,
    VirtualMachine,
}

pub struct HostCapability {
    pub id: String,
    pub scope: CapabilityScope,
    pub operations: Vec<CapabilityOperation>,
}

pub struct HostOnboardingRequirement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub capabilityIds: Vec<String>,
    pub status: HostRequirementStatus,
    pub action: HostRequirementAction,
}

pub enum HostRequirementStatus {
    Satisfied,
    Missing,
    Unavailable,
}

pub enum HostRequirementAction {
    RuntimePermission,
    OpenSystemSettings,
    HostManaged,
    None,
}
```

Access session 绑定用户和工作区授权：

```rust
pub struct AccessContext {
    pub sessionId: String,
    pub origin: AccessOrigin,
    pub userId: String,
    pub workspaceGrantIds: Vec<String>,
    pub aiMode: AiWorkMode,
}

pub enum AccessOrigin {
    LocalOwner,
    PairedDevice,
    Cli,
    WebAccess,
}

pub enum AiWorkMode {
    ReadOnly,
    SandboxWrite,
    FullAccess,
}
```

用户和工作区授权由 host/server 侧保存：

```rust
pub struct AccessUser {
    pub id: String,
    pub displayName: String,
    pub kind: AccessUserKind,
    pub enabled: bool,
}

pub enum AccessUserKind {
    LocalOwner,
    EnterpriseUser,
}

pub struct WorkspaceGrant {
    pub id: String,
    pub userId: String,
    pub workspaceId: String,
    pub mode: WorkspaceGrantMode,
}

pub enum WorkspaceGrantMode {
    ReadOnly,
    ReadWrite,
}

pub struct WorkspaceDescriptor {
    pub id: String,
    pub displayName: String,
    pub vfsRoot: String,
    pub physicalRoot: String,
}
```

工具声明自身能力，不再保存用户对每个 toolName 的 allow/ask/forbid：

```rust
pub struct ToolCapabilityDescriptor {
    pub toolId: String,
    pub operations: Vec<ToolOperation>,
    pub pathPolicy: ToolPathPolicy,
    pub networkPolicy: ToolNetworkPolicy,
    pub statePolicy: ToolStatePolicy,
}

pub enum ToolOperation {
    ReadFile,
    WriteFile,
    DeleteFile,
    ExecuteCommand,
    NetworkRead,
    NetworkWrite,
    ExternalStateMutation,
}
```

## 5. Request Flow

Local personal mode：

```text
Flutter UI / CLI
  -> create local AccessContext(userId = local-owner)
  -> CoreProxy / local bridge
  -> runtime checks AccessContext
  -> tool execution checks Host Capability + Workspace Grant + AI Work Mode + Tool Capability
  -> host executes
```

Remote paired client：

```text
client app
  -> signed app access request
  -> host app access verifies session
  -> session maps to AccessUser + WorkspaceGrant
  -> app access writes AccessContext into request origin
  -> operit-link dispatcher
  -> LocalCoreProxy
  -> runtime checks AccessContext
  -> host executes allowed operation
```

Web Access：

```text
browser
  -> WebCrypto session signing
  -> host app access verifies session
  -> session maps to AccessUser + WorkspaceGrant
  -> same runtime/tool enforcement path as remote paired client
```

CLI enterprise connection：

```text
employee CLI
  -> login/pair/connect to enterprise Operit Host
  -> session maps to enterprise AccessUser
  -> host returns only granted workspaces
  -> every call carries signed session
  -> host enforces workspace grant on execution
```

## 6. Enforcement

权限判断放在两处：

```text
app access / host access
  认证 session
  绑定 AccessUser
  绑定 WorkspaceGrant
  构造 AccessContext
  拒绝无效 session

runtime / tool execution
  检查 AccessContext
  检查 AI Work Mode
  检查 Tool Capability
  检查 Workspace Grant
  检查 Host Capability
  写 audit
```

文件路径必须使用 VFS 和 workspace descriptor 做结构化解析。不能用反复字符串包含判断来猜测路径是否属于工作区。

工作区边界判断规则：

```text
1. 将工具请求里的 path 解析成 VFS path。
2. 通过 PathMapper 解析到 workspace id 与规范化路径。
3. 查询 AccessContext.workspaceGrantIds。
4. 检查 grant 是否包含该 workspace id。
5. 检查 grant.mode 是否允许本次 ToolOperation。
6. 检查 AI Work Mode 是否允许本次 ToolOperation。
7. 检查 Host Capability 是否提供本次能力。
```

`sandboxWrite` 只能写入 grant 覆盖的 workspace。它不允许借助 host 的管理员/root 能力写 workspace 外部路径。

`fullAccess` 关闭单工具确认，但不能越过 Host Capability。Host 进程没有的系统能力，runtime、工具、远程客户端都拿不到。

欢迎页权限项也不例外。它只能触发 Host 暴露的 request action，然后重新读取 Host 的 snapshot。最终状态以 Host 校验结果为准，不以 Flutter 按钮点击结果为准。

## 7. Personal Mode

个人模式启动时创建内置本机用户：

```text
userId
  local-owner

kind
  LocalOwner

session
  LocalOwner origin

workspace grant
  用户在本机创建或绑定的工作区
```

个人模式不需要展示“用户管理”。普通用户只需要看到：

```text
当前模式
当前设备能力
当前工作区
当前是否有远程客户端连接
```

高级视图可以显示 `local-owner`，用于解释权限链路。

## 8. Enterprise / Remote Host

企业部署时，服务器上的 Operit Host 是权限源头。客户端只负责连接、展示与发起请求。

服务器侧必须保存：

```text
users
  enterprise user id
  display name
  enabled state

workspaces
  workspace id
  display name
  VFS root
  physical root

grants
  user id
  workspace id
  readOnly / readWrite

sessions
  session id
  device id
  user id
  created time
  expires time
```

员工连接后：

```text
1. 员工登录或配对到企业 Operit Host。
2. Host 将 session 绑定到 AccessUser。
3. Host 只返回该用户被授予的 WorkspaceDescriptor。
4. 员工选择工作区后发起 link call。
5. Host 根据 session 重新加载 AccessContext。
6. ToolExecutionManager 执行前检查 workspace grant。
7. Host 执行工具并写 audit。
```

目录授权必须在服务器侧生效：

```text
员工客户端看不到未授权目录。
员工手写 VFS path 也不能访问未授权目录。
员工调用 CLI 也不能访问未授权目录。
员工使用 Web Access 也不能访问未授权目录。
```

企业最小模型只做“用户 + 工作区”。暂不引入团队、组织、角色、策略继承。

## 9. Module Ownership

| Part | Owns | Must Not Own |
| --- | --- | --- |
| host-api | host descriptor、host privilege、host capability、workspace root descriptor | UI 展示、远程 session 存储、声明 host 实际没有的系统能力 |
| app access / native access | session 验签、配对、AccessContext 构造、session 到 user 的绑定 | core 业务逻辑、工具内部执行 |
| runtime/core | AI Work Mode 判定、tool capability 判定、workspace grant 执行检查、audit 事件 | 配对 UI、设备信任 UI、server 生命周期 |
| PathMapper / file tools | VFS 解析、workspace id 解析、物理路径规范化 | 用户登录、远程配对、UI 模式文案 |
| Flutter UI | 普通模式卡片、高级权限链路展示、工作区/访问入口页面 | 作为权限执行点 |
| CLI / Web Access | 连接、展示授权工作区、发起 signed call | 作为企业目录授权源头 |

## 10. Code Placement

现有代码落点：

```text
apps/flutter/app/lib/ui/features/settings/models/SettingsModels.dart
  SettingsCategory.tools 继续表示“工具与扩展”。
  不把左侧分类改成“工具与权限”。

apps/flutter/app/lib/ui/features/settings/tools/ToolSettingsPanel.dart
  删除旧 master switch / per-tool overrides 的主界面。
  改成普通视图：当前 AI Work Mode + Host Capability 摘要 + 扩展/MCP设置。
  高级视图展示完整权限链路。

apps/flutter/app/lib/ui/features/onboarding/OnboardingStartupRoute.dart
  删除写死的 _OnboardingPermissionSnapshot 字段。
  权限页改成读取 HostOnboardingRequirement 列表。
  _PermissionTile 按 Host 返回的 title、description、status、action 渲染。
  用户点击授权时只调用 requirement.id 对应的 Host request action。

apps/flutter/app/lib/ui/features/chat/components/style/input/agent/AgentInputMenuPopup.dart
  删除 forbid / ask / allow。
  改成 readOnly / sandboxWrite / fullAccess。

core/crates/operit-runtime/src/core/tools/ToolPermissionSystem.rs
  删除旧 PermissionLevel 与 per-tool override 模型。
  新模型应围绕 AiWorkMode、ToolCapabilityDescriptor、AccessContext。

core/crates/operit-runtime/src/api/chat/enhance/ToolExecutionManager.rs
  作为工具执行前的主判定点。
  从 toolName permission 切到 capability + access context 判定。

core/crates/operit-host-api/src/lib.rs
  扩展 HostEnvironmentDescriptor。
  Host 注册 platform、privilege、isolation、capabilities、workspace roots。

core/crates/operit-runtime/src/core/application/OperitApplicationContext.rs
  持有 hostEnvironment 与 host 能力入口。
  runtime 从这里读取 host capability，不从 UI 推断。

apps/flutter/app/lib/core/link_host/LinkHostConfig.dart
  InboundLinkSessionRecord 扩展 userId、workspaceGrantIds、aiMode、过期时间。

apps/flutter/native/operit-flutter-bridge/src/access.rs
  /link/session、/link/call、/link/watch/channel/* 验证 session 后构造 AccessContext。
  远程 session 到 user/grant 的绑定在这里进入 host access。

apps/flutter/app/android/app/src/main/kotlin/app/operit/AndroidPlatformChannel.kt
  当前 androidOnboardingPermissionSnapshot 写死 Android 权限 map。
  改成 Android host 注册 onboarding requirements：
    android.location
    android.bluetooth
    android.overlay
    android.batteryOptimization
  每个 requirement 的 status 由 Android API 检查。
  每个 requirement 的 request action 由 Android host 执行。
```

## 11. Settings UI

`工具与扩展`：

```text
普通视图
  当前模式：只读 / 沙盒读写 / 完整权限
  当前 Host：平台、权限等级、隔离状态
  工具与扩展：MCP、插件、工具包状态

高级视图
  Boundary
  Host Capability
  Access User
  Workspace Grant
  AI Work Mode
  Tool Capability
  最近 audit
```

`工作区`：

```text
本机工作区列表
远程 host 暴露的工作区列表
当前用户对每个工作区的 readOnly / readWrite 状态
工作区物理路径只在 host 允许展示时显示
```

`访问入口`：

```text
已配对设备
Web Access session
CLI session
远程 host 连接状态
session 绑定的 user
session 可访问的 workspace grant 摘要
```

普通用户不需要理解五层权限链路。普通视图只回答两个问题：

```text
AI 现在能做什么？
这些能力来自哪个 Host？
```

## 12. Onboarding Permission Page

欢迎页权限授予页不是独立权限模型，它只是 Host Capability 的首次配置入口。

当前页面问题：

```text
OnboardingStartupRoute.dart
  _OnboardingPermissionSnapshot 写死 Android 字段。
  _OnboardingPermissionAction 写死 location / bluetooth / overlay / battery。
  _AiSetupPermissionPage 写死四个 PermissionTile。

AndroidPlatformChannel.kt
  androidOnboardingPermissionSnapshot 返回固定 map。
  androidOnboardingRequestPermission 根据固定字符串分发 Android 请求。
```

目标页面模型：

```text
Host
  注册 onboarding requirements。
  检查每一项当前是否满足。
  提供每一项申请动作。

Flutter
  读取 requirements。
  渲染 requirements。
  调用 request(requirementId)。
  重新读取 requirements。
```

目标调用形状：

```text
hostOnboardingRequirements()
  -> Vec<HostOnboardingRequirement>

requestHostOnboardingRequirement(id)
  -> ()

hostOnboardingRequirements()
  -> Vec<HostOnboardingRequirement>
```

Android host 示例：

```text
android.location
  capabilityIds: system.location
  status: ACCESS_FINE_LOCATION granted
  action: RuntimePermission

android.bluetooth
  capabilityIds: bluetooth.classic, bluetooth.ble
  status: BLUETOOTH_CONNECT and BLUETOOTH_SCAN granted
  action: RuntimePermission

android.overlay
  capabilityIds: android.overlay
  status: Settings.canDrawOverlays
  action: OpenSystemSettings

android.batteryOptimization
  capabilityIds: runtime.background
  status: PowerManager.isIgnoringBatteryOptimizations
  action: OpenSystemSettings
```

这个页面不能承诺“授权后一定拥有某能力”。它只能表达：

```text
Host 声明这个能力需要一个系统授权项。
Host 当前检查结果是已满足 / 未满足 / 不可用。
用户可以触发 Host 提供的申请动作。
申请后仍以 Host 重新检查结果为准。
```

Windows / Linux / 企业 Host 可以注册完全不同的欢迎页项：

```text
windows.admin
  表示当前 host 是否以管理员身份运行。
  不能通过 Flutter 按钮提升到管理员。

linux.root
  表示当前 host 是否以 root 或指定 service account 运行。
  不能通过 Flutter 按钮提升到 root。

enterprise.workspace
  表示当前 session 是否有至少一个 workspace grant。
  申请动作由企业 host 决定。
```

## 13. Legacy Replacement

删除旧模型：

```text
PermissionLevel.ALLOW
PermissionLevel.ASK
PermissionLevel.FORBID
master switch
per-tool overrides
ALWAYS_ALLOW
工具确认弹窗作为权限主模型
```

旧配置不迁移。升级后重新生成个人模式默认用户与默认工作区授权：

```text
local-owner
  -> local workspaces
  -> selected AI Work Mode
```

旧工具权限 UI 文案不保留为兼容入口。

## 14. Prohibited Placement

不得把以下内容放进 `operit-link`：

```text
用户管理
工作区授权
Host Capability 判定
AI Work Mode 判定
工具执行授权
权限 UI
企业目录策略
```

不得把以下内容只放在 Flutter UI：

```text
目录授权
远程用户授权
fullAccess 判定
workspace boundary 执行检查
工具能力判定
欢迎页权限项的满足状态
欢迎页权限项的能力保证
```

不得把以下概念混在一起：

```text
Host Capability != 可绕过的产品策略
Host Capability != Access User
Workspace Grant != Tool Capability
sandboxWrite != VM sandbox
fullAccess != root/admin bypass
remote pairing != enterprise user grant
```

## 15. Implementation Order

```text
1. 定义 AccessContext、AiWorkMode、WorkspaceGrant、ToolCapabilityDescriptor。
2. 扩展 HostEnvironmentDescriptor，由各 host 注册 platform / privilege / isolation / capabilities。
3. 增加 HostOnboardingRequirement，由各 host 注册欢迎页权限项。
4. 改造 OnboardingStartupRoute，让权限页按 Host requirements 渲染。
5. 创建个人模式 local-owner，并把本机工作区绑定成 WorkspaceGrant。
6. 改造 /link/session 与 /link/call，让远程请求携带并绑定 AccessContext。
7. 改造 ToolExecutionManager，统一执行 capability + context 判定。
8. 改造文件工具与 PathMapper 的 workspace grant 检查。
9. 替换 ToolSettingsPanel 和 AgentInputMenuPopup 的旧三态。
10. 删除旧 ToolPermissionSystem 数据与 UI。
11. 增加 audit 记录。
12. 企业模式只实现用户 + 工作区授权。
```

这不是 UI 改名任务。核心改动是把“工具权限”从一个 UI 开关，改成 host、user、workspace、mode、tool capability 共同参与的执行期权限链路。
