# Permission / AI Capability / Sandbox Architecture

本文档定义 Operit2 的工具执行权限受限层级、AI 能力限制模型、欢迎页权限授予模型与设置页呈现方式。后续实现、评审与排查都以这里的边界为准。

这个设计替换现有单工具授权模型。新模型不向前兼容旧配置，不迁移旧的单工具授权数据。

## 1. Core Stack

工具执行权限只按下面 0-3 层自上而下收紧：

```text
0. App Runtime Sandbox
   整个 app 所在的外层运行边界。
   例如真实虚拟机、容器、系统账号、Android 应用沙盒、服务器部署边界。
   这是最高层隔离，和 Operit 软件内部策略无关。

1. Host Authorization
   Host 被操作系统或部署环境授予的真实能力。
   例如 Android 普通 / Shizuku / root，Windows 普通 / 管理员，Linux 普通 / root。
   这是软件权限的源头，也是不能绕过的真实系统边界。

2. AI Capability Limit
   用户给 AI 选择的能力范围。
   runtime 保存 AiPermissionMode：ReadOnly / WorkspaceWrite / Full。
   ReadOnly = 只读 + 应用内沙盒 on。
   WorkspaceWrite = 读写 + 应用内沙盒 on。
   Full = 读写 + 应用内沙盒 off。
   Full 不是新的底层文件能力，也不是 Host 提权。
   这一层只表达 AI 对工具和文件的当前能力限制。

3. User Tool Approval
   用户是否批准 AI 发起的具体工具调用。
   ToolPkg 是 Host 上的插件/扩展资源，不是 User Tool Approval 的批准对象。
   插件安装、删除、更新、启停、配置不走工具调用批准。
   ToolPkg 内部每次 Tools.* 调用不逐次审批；触达具体 Host bridge 工具时，按该工具的能力声明和 AiPermissionMode 检查。
```

上层没有的能力，下层不能变出来；下层只能继续限制，不能绕过上层。

## 2. Product Surface

普通用户只需要看到三类信息：

```text
当前模式
  只读
  工作区读写
  完整权限
    只读 = readOnly + sandbox on
    工作区读写 = workspaceWrite + sandbox on
    完整权限 = workspaceWrite + sandbox off

应用内沙盒
  未启用
  已启用
  当前阶段显示为未实现

当前 Host
  当前设备类型
  当前 host 权限等级
  当前运行隔离状态
  当前工作区
  当前连接来源：本机 / 已配对客户端 / Web Access / CLI
```

普通视图只回答三个问题：

```text
AI 现在能读吗？
AI 现在能写吗？
AI 现在是否运行在应用内沙盒里？
```

高级用户可以展开完整链路：

```text
0. App Runtime Sandbox
1. Host Authorization
2. AI Capability Limit
   当前模式
   当前工作区
   应用内沙盒开关
3. User Tool Approval
```

设置分类不新增“权限总表”。现有 `工具与权限` 分类保留 AI 能力模式、Host 授权状态与扩展入口。现有 `工作区` 分类负责工作区路径展示。现有 `访问入口` 分类负责配对设备、远程 session、Web Access 与 CLI 连接。

插件、Skill、MCP 是 Host 上的扩展资源。Operit 内部不做用户权限管理；能打开本机设置的人就是本机 owner。企业部署由 Operit Server Manager 在外层控制谁能连接到哪个 Host / workspace。

## 3. AI Capability Mapping

这一节只定义用户选择的三档模式。三档不是三个独立权限层，也不是三套执行系统。

```text
AiPermissionMode
  ReadOnly
    文件能力：读
    应用内沙盒：开启
    工具批准：READ 不询问，WRITE 不允许

  WorkspaceWrite
    文件能力：读写
    应用内沙盒：开启
    工具批准：READ / 普通 workspace WRITE 不询问，沙盒逃逸 WRITE 询问

  Full
    文件能力：读写
    应用内沙盒：关闭
    工具批准：内置工具不询问，PackageTool 仍询问
```

runtime 只保存用户选择的 `AiPermissionMode`。文件能力、应用内沙盒状态、工具批准策略都由这个模式派生，不单独保存成一个混合状态对象。

```text
readOnly
  文件工具只能读当前 workspace。
  ToolEffect.READ 不询问。
  ToolEffect.WRITE 不允许启动。

workspaceWrite
  文件工具可以读写当前 workspace。
  仍然运行在应用内沙盒里。
  ToolEffect.READ 不询问。
  普通 workspace 内 ToolEffect.WRITE 不询问。
  如果 ToolEffect.WRITE 需要逃逸应用内沙盒，则询问用户。

full
  文件工具可以读写当前 workspace。
  不启用应用内沙盒。
  内置工具调用不询问。
  PackageTool 调用仍询问。
  READ / WRITE 工具可以在 Host Authorization 允许的范围内启动。
```

`full` 不能被解释成 root/admin 提权，也不能绕过 Host Authorization。Host 进程没有的系统能力，AI 模式和工具批准都不能变出来。

应用内沙盒当前只定义字段和 UI 位置，不实现真实执行边界。没有真实执行边界前，不能用 VFS 路径限制、命令字符串审查或工具名称分类冒充沙盒。

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
    pub displayName: String,
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

AI 模式：

```rust
pub enum AiPermissionMode {
    ReadOnly,
    WorkspaceWrite,
    Full,
}
```

`AiPermissionMode` 是用户选择，也是 runtime 执行判定的输入。workspace 选择属于工作区上下文，不属于权限模式字段。

远程连接只携带 session 和连接来源；AI 能力限制仍由当前会话的 AiPermissionMode 表达。

包内部的 `Tools.Files.write` 类似 Node 程序里的 `fs.writeFile`。runtime 不应在包内部每一次 write 调用上做人为审批。当前实现里它直接调用默认 `write_file`，这是 Host bridge 行为；它受 Host 真实系统边界、AI 当前模式和具体工具能力边界约束。

```text
Tools.Files.write
  -> toolCall("write_file")
  -> AIToolHandler.executeTool
  -> StandardFileSystemTools.writeFile
  -> VisualFileSystem.resolvePath
  -> Host FileSystemHost.writeFile
```

这条链路本身可以保留。它不能被描述为沙盒；它应该被描述为插件内部 API 触达 Host bridge 后，由具体 Host bridge 工具执行边界做检查。

## 5. Request Flow

Local access：

```text
Flutter UI / CLI
  -> read AiPermissionMode
  -> CoreProxy / local bridge
  -> runtime checks AiPermissionMode
  -> tool execution follows 0-3 permission stack
  -> host executes
```

Remote paired client：

```text
client app
  -> signed app access request
  -> host app access verifies session
  -> app access attaches current AiPermissionMode
  -> operit-link dispatcher
  -> LocalCoreProxy
  -> runtime checks AiPermissionMode
  -> host executes allowed operation
```

Web Access：

```text
browser
  -> WebCrypto session signing
  -> host app access verifies session
  -> request enters same runtime/tool enforcement path as remote paired client
```

Remote CLI connection：

```text
remote CLI
  -> connect to Operit Host
  -> host returns current workspace and host descriptor
  -> every call carries signed session
  -> runtime/tool execution checks AiPermissionMode
```

## 6. Enforcement

权限判断放在三处：

```text
app access / host access
  认证 session
  读取当前 AI 模式
  拒绝无效 session

runtime / tool execution
  检查 AiPermissionMode
  检查 Host Authorization
  检查 User Tool Approval

host bridge boundary
  文件工具检查 workspace read/write
  终端、网络、浏览器、系统等工具按本次 accessSpec(tool) 声明检查 READ / WRITE
  终端是否受应用内沙盒限制只取决于实际执行边界，不取决于命令字符串审查
  具体 Host 执行仍受系统真实权限限制
```

文件路径必须使用 VFS 和 workspace descriptor 做结构化解析。不能用反复字符串包含判断来猜测路径是否属于工作区。

### Enforcement Points

权限拦截分成四个具体位置：

```text
1. app session
   位置：
     apps/flutter/native/operit-flutter-bridge/src/access.rs
     apps/flutter/app/lib/core/link_host/LinkHostConfig.dart
   作用：
     验证本机、配对设备、CLI、Web Access 的 session。
     把请求交给当前 Host 的 runtime。
     不在这里扩展 AI 能力语义。

2. tool invocation approval
   位置：
     core/crates/operit-runtime/src/api/chat/enhance/ToolExecutionManager.rs
   已落地：
     executeInvocations 负责解析后的 invocation 流转、CLI 公开工具、角色卡工具暴露检查。
     User Tool Approval 不再由 ToolExecutionManager 承载，执行前能力判定在 AIToolHandler。
     这里不硬编码终端专用能力名。
     这里不审批 ToolPkg / ToolPackage 容器。
     PackageToolExecutor 只是 packageName:toolName 的执行器，不是权限闸口。
     PackageTool 脚本内部每一次 Tools.* 调用不逐次走 User Tool Approval。

3. direct host bridge execution
   位置：
     core/crates/operit-runtime/src/core/tools/AIToolHandler.rs
   已落地：
     executeToolSafelyWithResolvedExecutor 会移出 executor，validateParameters，然后 executor.invokeAndStream。
     JS toolCall、PackageToolExecutor、MCPToolExecutor 最终都会走 AIToolHandler 的执行链路。
     这里不是 ToolPkg 拦截点。
     这里保证直接 toolCall 触达默认工具、MCP、终端、文件等 Host bridge 前能拿到当前 AiPermissionMode。
     这里调用 executor.accessSpec(tool)，用 ToolEffect.READ / ToolEffect.WRITE 检查 AiPermissionMode。
     这里对 packageName:toolName 触发 User Tool Approval。
     WorkspaceWrite 下非文件 WRITE 作为沙盒逃逸请求用户确认。

4. file operation boundary
   位置：
     core/crates/operit-runtime/src/core/tools/defaultTool/standard/StandardFileSystemTools.rs
     core/crates/operit-runtime/src/core/files/VisualFileSystem.rs
   已落地：
     StandardFileSystemTools 从 AITool 参数取 path。
     VisualFileSystem.resolvePath 使用 PathMapper 解析 VFS path。
     AIToolHandler 在执行前读取当前 workspacePath，用 PathMapper.resolve 解析 workspace 和目标 path。
     文件读写在调用 Host FileSystemHost 前完成当前 workspace 边界检查。
     list/read/fileExists/fileInfo/find/grep 属于 ToolEffect.READ。
     write/writeBinary/delete/move/copy/mkdir/download/apply/create 属于 ToolEffect.WRITE。
     读写检查基于 PathMapper.relativePath 的结构化 VFS 归属，不基于字符串包含。
```

拦截顺序：

```text
request/session
  -> read AiPermissionMode
  -> ToolExecutionManager passes parsed tool invocation
  -> AIToolHandler resolves executor and reads accessSpec(tool)
  -> AI capability guard checks READ / WRITE mode
  -> file boundary resolves workspace root and target path through PathMapper
  -> PathMapper.relativePath confirms the target stays inside the active workspace
  -> Host FileSystemHost executes
```

`AiPermissionMode` 不能只存在 UI 层。它必须进入 runtime 执行路径，并且能被 AIToolHandler 读取。ToolExecutionManager 只负责把当前 workspacePath 继续传入执行链路。

### Workspace Read / Write Checks

文件边界判定直接使用 `ToolEffect.READ / ToolEffect.WRITE`，不再定义第二套 `WorkspaceOperation`。

当前实现由 `AIToolHandler` 在执行前读取 `workspacePath`，分别用 `PathMapper.resolve` 解析当前 workspace root 和目标 path，再用 `PathMapper.relativePath` 判定目标是否落在当前 workspace 内：

```text
ToolEffect.READ
  要求 path 位于当前 workspacePath 内。
  AiPermissionMode 可以是 ReadOnly、WorkspaceWrite 或 Full。

ToolEffect.WRITE
  要求 path 位于当前 workspacePath 内。
  AiPermissionMode 必须是 WorkspaceWrite 或 Full。
```

虚拟目录按当前 workspace 展示：

```text
/app
  可以展示固定虚拟入口。

/app/workspaces
  展示当前 Host 注册的 workspace。

具体 workspace 下的路径
  先 resolve 成物理路径，再通过 PathMapper.relativePath 判定是否属于当前 workspace。
```

跨 workspace 操作按两端分别判定：

```text
move / copy
  source 需要 Read。
  destination 需要 Write。
  source 和 destination 都必须位于当前 workspacePath 内。
```

不做字符串命令审查，不通过反复匹配路径片段判断归属。路径归属只来自 `PathMapper.resolve` 和 `PathMapper.relativePath` 的结构化路径结果。

### Tool Invocation Checks

用户批准对象是 AI 发起的具体工具调用，不是 Host bridge，也不是 ToolPkg 容器：

```text
代码里的对象关系
  ToolPkg / ToolPackage
    插件包 / 工具包容器。
    由 PackageManager 安装、启用、停用、解析元数据。
    不是一次 AI tool invocation。

  PackageTool
    ToolPackage.tools 里的一个可执行工具声明。
    字段包含 name、description、parameters、script、advice。
    advice = true 的 PackageTool 不注册成可执行工具。

  packageName:toolName
    AIToolHandler.registerPackageTools 注册出来的实际工具名。
    AI 直接调用包工具时，调用的是这个 AITool.name。
    这个才是 User Tool Approval 面对的工具调用对象。

  PackageToolExecutor
    packageName:toolName 的 executor。
    执行时解析 packageName 和 toolName，找到 PackageTool，然后运行该 PackageTool.script。

  Tools.* / toolCall(...)
    插件脚本内部 API。
    例如 Tools.Files.write 最终调用 toolCall("write_file", params)。
    这属于插件程序内部行为，不再逐次走 ToolExecutionManager.checkToolPermission。
```

因此：

```text
工具调用批准
  ToolExecutionManager.checkToolPermission
  用于用户批准 AI 发起的某一次工具调用。
  批准粒度是 tool invocation，也就是工具名和本次参数。
  内置工具是否询问由 AiPermissionMode、ToolEffect 和沙盒逃逸情况共同决定。
  PackageTool invocation 永远询问。

AI 直接调用内置工具
  AITool.name = "read_file_full" / "write_file" / ...
  ReadOnly 下 ToolEffect.READ 不询问，ToolEffect.WRITE 不允许。
  WorkspaceWrite 下 ToolEffect.READ 不询问，普通 workspace WRITE 不询问，沙盒逃逸 WRITE 询问。
  Full 下内置工具不询问。

AI 直接调用包工具
  AITool.name = "packageName:toolName"
  User Tool Approval 对这个 packageName:toolName 生效。
  PackageTool invocation 永远询问。
  不是批准整个 ToolPkg 容器。

Host bridge 上下文传递
  AIToolHandler.executeToolSafelyWithResolvedExecutor
  不作为 ToolPkg 容器拦截点。
  用于把 AiPermissionMode 带到默认工具、MCP、终端、文件等具体执行边界。

插件内部 toolCall
  JsNativeInterfaceDelegates.callToolSync
  会把 toolCall(...) 解析成 AITool 后直接调用 AIToolHandler.executeTool。
  AIToolHandler.executeTool 不调用 ToolExecutionManager.checkToolPermission。
  所以插件内部 Tools.* 调用不应该被描述成用户逐次批准。
  这条链路必须在具体工具能力边界检查 AiPermissionMode 和 Host Authorization。
```

内置工具不能靠工具名判断读写，也不能靠字符串包含规则猜测。当前代码里 `AIToolHandler.registerTool` 只保存 executor 和 visibility；`ToolRegistration.rs` 注册内置工具时虽然能看到 `FileSystemToolOperation`、`TerminalToolOperation`、`HttpToolOperation` 等 operation enum，但这些信息没有被统一暴露成权限判定输入。

`ToolEffect` 也不应该是注册期静态元数据。同一个工具在不同参数下可能是 READ，也可能是 WRITE。典型例子是 terminal、browser、http、蓝牙、系统设置、以及未来的复合工具。effect 必须是“本次调用”的结果。

正确位置是 `ToolExecutor` 的动态 preflight 方法。executor 了解自己的 operation 和参数语义，应当在真正执行前，带着本次 `AITool` 参数返回访问声明。这个方法只做解析和声明，不产生副作用。

```text
ToolEffect
  READ
    读取状态或内容。

  WRITE
    修改状态或内容。
```

Rust 形状：

```rust
pub enum ToolEffect {
    READ,
    WRITE,
}

pub enum ToolBoundary {
    None,
    FilePath {
        effect: ToolEffect,
    },
    FilePair {
        source: ToolEffect,
        destination: ToolEffect,
    },
}

pub struct ToolAccessSpec {
    pub effect: ToolEffect,
    pub boundary: ToolBoundary,
}

pub trait ToolExecutor: Send {
    fn validateParameters(&self, tool: &AITool) -> ToolValidationResult;
    fn accessSpec(&self, tool: &AITool) -> Result<ToolAccessSpec, String>;
    fn invokeAndStream(&mut self, tool: &AITool) -> Vec<ToolResult>;
}
```

含义：

```text
accessSpec(tool)
  输入是本次 AITool，包括工具名和参数。
  输出是本次调用的 ToolEffect 和边界声明。
  只能做参数解析和访问声明，不能执行工具动作。

READ
  只检查 AiPermissionMode 是否允许启动 READ 工具。
  不做 workspace path 判定。

WRITE
  检查 AiPermissionMode 是否允许启动 WRITE 工具。

FilePath boundary
  再从参数中解析 path，交给 PathMapper 解析当前 workspace 和目标路径。
  最后按 ToolEffect.READ / ToolEffect.WRITE 检查 workspace 边界。

FilePair boundary
  move/copy 这类双路径工具。
  source 按 READ 判定，destination 按 WRITE 判定。
```

文件工具的 `accessSpec(tool)` 可以由 `FileSystemToolOperation` 和本次参数共同决定：

```text
READ
  list_files
  read_file
  read_file_part
  read_file_full
  read_file_binary
  file_exists
  find_files
  file_info
  grep_code
  grep_context

WRITE
  write_file
  write_file_binary
  delete_file
  move_file
  copy_file
  make_directory
  create_file
  edit_file
  zip_files
  unzip_files
  download_file
  apply_file

WRITE
  open_file
  share_file
```

非文件工具也必须由 executor 动态返回 `ToolAccessSpec`：

```text
Terminal tools
  根据本次命令/动作返回 READ 或 WRITE。
  get_terminal_info、get_terminal_session_screen 可以是 READ。
  create_terminal_session、execute_in_terminal_session、execute_hidden_terminal_command、input_in_terminal_session、close_terminal_session 是 WRITE。
  命令字符串不用于猜文件边界；它只影响 terminal 工具自己声明的 effect。

Network / external mutation tools
  根据 method / action / 参数返回 READ 或 WRITE。
  GET 类请求可以是 READ。
  POST / PUT / PATCH / DELETE 等变更类请求是 WRITE。

Browser automation tools
  snapshot / console / network 读取类是 READ。
  navigate / click / type / upload / evaluate / run_code 等状态变更类是 WRITE。

System operation tools
  get / list / info 类是 READ。
  set / modify / start / stop / install / uninstall 类是 WRITE。

Bluetooth / device control tools
  get / list / read / scan 可以是 READ。
  connect / listen / accept / send / write / close / request_enable 是 WRITE。

Memory / chat / app-data tools
  query / get / list 是 READ。
  create / update / delete / move / link 是 WRITE。
  这些不是 workspace 文件读写，不能混入 workspace boundary。
```

执行判定：

```text
AIToolHandler / ToolExecutionManager
  validateParameters(tool)
  accessSpec(tool)
  check AiPermissionMode against ToolEffect
  check workspace boundary if accessSpec declares FilePath / FilePair
  check Host Authorization
  invokeAndStream(tool)

ToolEffect.READ
  ReadOnly / WorkspaceWrite / Full 都可启动。
  不询问用户。

ToolEffect.WRITE
  WorkspaceWrite / Full 可启动。
  带 FilePath / FilePair boundary 的工具必须继续做结构化 workspace 检查。
  WorkspaceWrite + sandbox on 下，如果本次调用需要逃逸应用内沙盒，则询问用户。
  Full 下内置工具不询问。

PackageTool invocation
  无论 ReadOnly / WorkspaceWrite / Full，都先询问用户是否允许 AI 调用 packageName:toolName。
  PackageTool invocation 自身的 accessSpec 不代表脚本内部的 Host 读写效果。
  允许后再由 PackageTool 脚本内部具体 Host bridge 工具的 accessSpec(tool)、AiPermissionMode 和 workspace boundary 继续判定。

Sandbox
  不属于 ToolEffect。
  它是独立的被动执行边界。
  WorkspaceWrite 模式下 sandbox on；Full 模式下 sandbox off。
  当前没有真实应用内沙盒执行边界，不能把当前任何工具执行路径描述成软件沙盒。
```

PackageTool 内部 `Tools.Files.write` 不做逐调用审批，也不在 ToolPkg 容器层拦截；它调用默认 `write_file` 后进入文件操作边界：

```text
AI calls packageName:toolName
  -> ToolExecutionManager.checkRoleCardToolAccess(packageName:toolName)
  -> AIToolHandler.executeToolSafelyWithResolvedExecutor
  -> AIToolHandler.executeAccessPreflight
  -> ToolPermissionSystem.checkPackageToolApproval(packageName:toolName)
  -> PackageToolExecutor
  -> JsToolManager.executeScript(PackageTool.script)
  -> JS Tools.Files.write
  -> toolCall("write_file")
  -> JsNativeInterfaceDelegates.callToolSync
  -> AIToolHandler.executeTool(write_file)
  -> AIToolHandler carries AiPermissionMode
  -> StandardFileSystemTools.writeFile
  -> VisualFileSystem.resolvePath
  -> workspace Write check
  -> Host FileSystemHost.writeFile
```

所以 ToolPkg 容器不拦截，PackageTool 被 AI 直接调用时才是工具批准对象；包内部 API 不弹窗。文件读写仍在文件 Host bridge 边界受当前 `AiPermissionMode` 限制。

工具执行受限层级：

```text
0. App Runtime Sandbox
   整个 app 所处运行环境先限制一切工具执行。

1. Host Authorization
   Host 真实拥有什么系统权限，工具最多只能用到这里。

2. AI Capability Limit
  当前检查 AiPermissionMode、ToolEffect.READ / ToolEffect.WRITE 和 workspace 边界。
  应用内沙盒只保留模式位置，不作为已实现执行边界描述。

3. User Tool Approval
   用户批准的是工具调用，不是 Host bridge。
   ToolPkg 容器是插件/扩展资源，不是用户批准对象。
   PackageTool 注册成 packageName:toolName 后，是 AI 可直接调用的工具。
   AI 调用 packageName:toolName 时，User Tool Approval 对这个 PackageTool invocation 生效。
   PackageTool 脚本内部 Tools.* 调用属于程序内部行为，不逐次审批。
```

`workspaceWrite` 只能写入当前 workspace。它不允许借助 host 的管理员/root 能力写 workspace 外部路径。

`full` 不是提权。`full` 表示 workspaceWrite + sandbox off。Host 进程没有的系统能力，runtime、工具、远程客户端都拿不到。

不要把 ToolPkg 容器、PackageTool 脚本内部 API 或 Host bridge 当成用户批准对象。用户批准的是 AI 发起的具体工具调用：内置工具名，或包工具名 `packageName:toolName`。`Tools.Files.write`、`Tools.Net.*`、未来的进程能力，都应该表现为插件程序运行时拿到的普通 API。文件、终端、网络等 Host bridge 边界只负责 AI 能力限制、工具能力声明和系统边界检查。

## 7. Terminal

终端必须单独处理。`execute_in_terminal_session`、`execute_hidden_terminal_command`、PTY 输入都不是 VFS 操作；它们把字符串交给 Host shell 执行。shell 可以自己解析路径、启动解释器、访问环境变量、调用系统命令，因此会绕开 Files API 的 VFS 限制。

```text
当前终端链路
  Tools.System.terminal.*
    -> toolCall("execute_in_terminal_session" / "execute_hidden_terminal_command")
    -> StandardTerminalTools
    -> TerminalHost.executeInSession / TerminalHost.executeHiddenCommand
    -> host shell
```

终端工具不再定义单独能力名。终端和其他工具一样，由 `TerminalToolExecutor.accessSpec(tool)` 根据本次调用返回 `ToolEffect.READ` 或 `ToolEffect.WRITE`：

```text
READ
  get_terminal_info
  get_terminal_session_screen

WRITE
  create_terminal_session
  execute_in_terminal_session
  execute_in_terminal_session_streaming
  execute_hidden_terminal_command
  input_in_terminal_session
  close_terminal_session
```

终端命令字符串不用于判断 workspace 文件边界，也不用于模拟沙盒。shell 语法、解释器、脚本文件、环境变量和平台差异都会产生逃逸路径。

应用内沙盒是独立的被动执行边界：

```text
ReadOnly
  不允许启动 ToolEffect.WRITE 的终端工具。

WorkspaceWrite
  允许启动 ToolEffect.WRITE 的终端工具。
  如果应用内沙盒已启用，shell 必须运行在真实沙盒边界里。
  如果真实沙盒不可用，则不能把终端描述成“已沙盒化”。

Full
  允许启动 ToolEffect.WRITE 的终端工具。
  应用内沙盒关闭，终端直接受 Host Authorization 限制。
```

不要用命令字符串审查来模拟终端沙盒。终端沙盒需要真实执行边界；这个边界不存在时，终端只能被描述为直接 Host shell 执行。

## 8. Enterprise Deployment Boundary

企业部署不进入 Operit 内部权限模型。

未来企业版本可以做独立的 Operit Server Manager：

```text
Operit Server Manager
  创建外层沙盒、容器、VM 或系统账号。
  在沙盒中启动 Operit CLI / Operit Host。
  挂载指定 workspace。
  管理员工连接入口。
  控制谁能进入哪个 Host 实例。
```

Operit Host 只需要读取自己所处环境：

```text
当前进程拥有什么系统权限。
当前进程能看到哪些目录。
当前 Host 注册了哪些 workspace。
当前是否运行在外部沙盒里。
```

这符合真实边界：

```text
Android 普通权限不能访问 /data。
Windows 普通进程不能执行管理员权限动作。
Linux 普通用户不能访问 root-only 资源。
容器没挂载的目录，容器内进程看不到。
```

企业目录隔离由部署器和系统边界完成，不由 Operit 内部用户权限模拟。

## 9. Onboarding Permission Page

欢迎页权限授予页不是独立权限模型，它只是 Host Authorization 的首次配置入口。

当前实现：

```text
OnboardingStartupRoute.dart
  读取 RuntimeHostDescriptor.onboardingRequirements。
  权限页按 HostOnboardingRequirement 列表渲染。
  _PermissionTile 按 Host 返回的 title、description、status、action 渲染。

AndroidPlatformChannel.kt
  hostOnboardingPermissionSnapshot(hostId) 返回 requirement id 到 status 的 map。
  hostOnboardingRequestPermission(hostId, requirementId) 根据 requirementId 触发 Host request action。
```

页面模型：

```text
Host
  注册 onboarding requirements。
  检查每一项当前是否满足。
  提供每一项申请动作。

Flutter
  读取 requirements。
  渲染 requirements。
  调用 request(hostId, requirementId)。
  重新读取 requirements。
```

当前调用形状：

```text
servicesRuntimeHostInfoService.runtimeHostDescriptor()
  -> Vec<HostOnboardingRequirement>

hostOnboardingRequestPermission(hostId, requirementId)
  -> ()

hostOnboardingPermissionSnapshot(hostId)
  -> requirement id -> status
```

Android host 示例：

```text
android.location
  capabilityIds: system.location
  status: ACCESS_FINE_LOCATION 已满足
  action: RuntimePermission

android.bluetooth
  capabilityIds: bluetooth.classic, bluetooth.ble
  status: BLUETOOTH_CONNECT 与 BLUETOOTH_SCAN 已满足
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

Windows / Linux / Remote Host 可以注册完全不同的欢迎页项：

```text
windows.admin
  表示当前 host 是否以管理员身份运行。
  不能通过 Flutter 按钮提升到管理员。

linux.root
  表示当前 host 是否以 root 或指定 service account 运行。
  不能通过 Flutter 按钮提升到 root。

remote.workspace
  表示当前 session 是否连接到一个 Host workspace。
  申请动作由 host 决定。
```

欢迎页权限项也不例外。它只能触发 Host 暴露的 request action，然后重新读取 Host 的 snapshot。最终状态以 Host 校验结果为准，不以 Flutter 按钮点击结果为准。

## 10. Module Ownership

| Part | Owns | Must Not Own |
| --- | --- | --- |
| host-api | host descriptor、host privilege、host capability、workspace root descriptor、onboarding requirement | UI 展示、远程 session 存储、声明 host 实际没有的系统能力 |
| app access / native access | session 验签、配对、当前 Host 请求入口 | core 业务逻辑、工具内部执行、AI 能力判定 |
| runtime/core | AI capability 判定、工具调用批准、workspace 执行检查 | 配对 UI、设备信任 UI、server 生命周期 |
| PathMapper / file tools | VFS 解析、workspace 路径归属判定、物理路径规范化 | 远程配对、UI 模式文案 |
| PackageManager / MCPManager | Host 插件、Skill、MCP 的安装、删除、更新、启停、配置执行 | 用户批准工具调用、工作区路径判定 |
| Flutter UI | 普通模式卡片、高级权限链路展示、工作区/访问入口页面、扩展管理入口状态 | 作为权限执行点 |
| CLI / Web Access | 连接、展示当前 workspace、发起 signed call | 作为目录访问源头 |

## 11. Code Placement

现有代码落点：

```text
apps/flutter/app/lib/ui/features/settings/models/SettingsModels.dart
  SettingsCategory.tools 继续表示“工具与权限”。
  不把左侧分类改成“工具与权限”。

apps/flutter/app/lib/ui/features/settings/tools/ToolSettingsPanel.dart
  删除旧工具总开关和逐工具长期授权的主界面。
  改成普通视图：当前模式 + 应用内沙盒状态 + Host Authorization 摘要 + 扩展/MCP 状态。
  高级视图展示完整 0-3 权限链路。

apps/flutter/app/lib/ui/features/onboarding/OnboardingStartupRoute.dart
  已删除旧 Android 固定字段快照对象。
  权限页读取 HostOnboardingRequirement 列表。
  _PermissionTile 按 Host 返回的 title、description、status、action 渲染。
  用户点击授权时只调用 requirement.id 对应的 Host request action。

apps/flutter/app/lib/ui/features/chat/components/style/input/agent/AgentInputMenuPopup.dart
  删除 forbid / ask / allow。
  改成只读 / 工作区读写 / 完整权限三档显示。
  三档显示直接写入 AiPermissionMode。

core/crates/operit-runtime/src/core/tools/ToolPermissionSystem.rs
  删除旧单工具授权与 per-tool override 模型。
  新模型应围绕 AiPermissionMode 和 User Tool Approval。

core/crates/operit-runtime/src/api/chat/enhance/ToolExecutionManager.rs
  只保留解析、CLI 公开工具、角色卡工具暴露检查。
  不再承载逐工具授权或长期批准模型。

core/crates/operit-runtime/src/core/tools/AIToolHandler.rs
  作为工具执行前的能力判定点。
  调用 executor.accessSpec(tool)，用 ToolEffect 检查 AiPermissionMode。
  对 packageName:toolName 触发用户批准；“本次会话中始终允许”只进入内存会话批准集合。

core/crates/operit-runtime/src/core/tools/javascript/JsTools.rs
  当前 Tools.Files.write 只是 toolCall("write_file", params)。
  新模型不在这里塞逐调用审批。
  这里应继续表现为包运行环境里的文件 API。

core/crates/operit-runtime/src/core/tools/javascript/JsNativeInterfaceDelegates.rs
  当前 callToolSync 把 JS toolCall 转成 AITool 后直接 executeTool。
  这条直接执行链路可以保留。
  它代表 PackageTool 脚本内部 toolCall 触达具体工具执行链路，不代表沙盒，也不作为 ToolPkg 容器拦截点。

core/crates/operit-runtime/src/core/files/VisualFileSystem.rs
  当前只做 PathMapper.resolve 后调用 Host FileSystemHost。
  文件读写边界由 AIToolHandler 在调用工具前完成。
  VisualFileSystem 只保留 VFS 到 Host FileSystemHost 的路径解析与执行职责。
  应用内沙盒需要真实运行环境边界；不能把 VFS 路径检查描述成沙盒。

core/crates/operit-runtime/src/core/tools/defaultTool/standard/StandardTerminalTools.rs
  当前终端工具直接调用 TerminalHost。
  execute_in_terminal_session、execute_hidden_terminal_command、input_in_terminal_session 都会绕开 VFS。
  终端 executor 必须实现 accessSpec(tool)，按本次调用返回 ToolEffect.READ / ToolEffect.WRITE。
  命令字符串不用于判断 workspace 文件边界；应用内沙盒限制只能来自真实执行边界。

core/crates/operit-runtime/src/services/RuntimeTerminalService.rs
  startTerminalPty 只把 /app 路径映射成物理 workingDir。
  命令执行本身仍由 Host shell 负责，不受 VFS 文件 API 约束。

core/crates/operit-host-api/src/lib.rs
  扩展 HostEnvironmentDescriptor。
  Host 注册 platform、privilege、isolation、capabilities、workspace roots。

core/crates/operit-runtime/src/core/application/OperitApplicationContext.rs
  持有 hostEnvironment 与 host 能力入口。
  runtime 从这里读取 host authorization，不从 UI 推断。

core/crates/operit-runtime/src/core/tools/packTool/PackageManager.rs
  作为插件、Skill、ToolPkg 安装、删除、更新、启停的执行点。
  由本机 owner 直接管理；企业部署交给外层 Operit Server Manager。

core/crates/operit-runtime/src/core/tools/mcp/MCPManager.rs
  作为 MCP 服务启停、配置和连接管理的执行点。
  由本机 owner 直接管理；企业部署交给外层 Operit Server Manager。

apps/flutter/app/lib/core/link_host/LinkHostConfig.dart
  InboundLinkSessionRecord 只保留 session、device、origin、过期时间和连接状态。
  不扩展内部多用户权限字段。

apps/flutter/native/operit-flutter-bridge/src/access.rs
  /link/session、/link/call、/link/watch/channel/* 验证 session。
  不构造内部用户权限系统。

apps/flutter/app/android/app/src/main/kotlin/app/operit/AndroidPlatformChannel.kt
  hostOnboardingPermissionSnapshot(hostId: android) 返回 Android requirement id 到 status 的 map：
    android.location
    android.bluetooth
    android.overlay
    android.batteryOptimization
  每个 requirement 的 status 由 Android API 检查。
  hostOnboardingRequestPermission(hostId: android, requirementId) 由 Android host 执行。
```

## 12. Settings UI

`工具与权限`：

```text
普通视图
  AI 能力模式：只读 / 读写 / 完整权限
    只读 = mode ReadOnly
    读写 = mode WorkspaceWrite
    完整权限 = mode Full
  Host 授权：
    显示当前 Host 注册的授权项
    status/action 全部由 Host 检查和执行
    用户可直接点击 Host 提供的授予动作

高级视图
  App Runtime Sandbox
  Host Authorization
  AI Capability Limit
    AiPermissionMode
    Workspace
    Mode Mapping
  User Tool Approval
```

`工作区`：

```text
本机工作区列表
远程 host 暴露的工作区列表
当前 AI 模式
工作区物理路径只在 host 允许展示时显示
```

`访问入口`：

```text
已配对设备
Web Access session
CLI session
远程 host 连接状态
session 绑定的 device / origin
session 当前 workspace 摘要
```

## 13. Prohibited Placement

不得把以下内容放进 `operit-link`：

```text
Host Authorization 判定
AI Capability Limit 判定
工具执行批准
权限 UI
目录访问策略
```

不得把以下内容只放在 Flutter UI：

```text
目录访问判定
full 显示态判定
workspace boundary 执行检查
工具能力判定
欢迎页权限项的满足状态
欢迎页权限项的能力保证
```

不得把以下概念混在一起：

```text
Host Authorization != 可绕过的产品策略
Host Authorization != AI Capability Limit
AI Capability Limit == AiPermissionMode
完整权限 == workspaceWrite + sandbox off
workspaceWrite != VM sandbox
full != root/admin bypass
remote pairing != user management
ToolPkg 内部 API 调用 != 产品权限审批点
ToolPkg / ToolPackage 容器 != User Tool Approval 对象
PackageToolExecutor != 权限闸口
packageName:toolName == AI 可直接调用的包工具名
插件 / Skill / MCP 安装管理 != User Tool Approval
Tools.Files.write != User Tool Approval 对象
Tools.Files.write == 插件程序里的 host bridge 文件写 API
TerminalHost != VFS 受限文件 API
终端工具 effect != 沙盒能力
终端 READ / WRITE 由 accessSpec(tool) 动态声明
沙盒是独立被动边界
```

工具执行受限层级必须保持：

```text
0. App Runtime Sandbox
1. Host Authorization
2. AI Capability Limit
3. User Tool Approval
```

## 14. Implementation Order

```text
1. 定义 AiPermissionMode，并由 HostEnvironmentDescriptor 暴露 workspaceRoots。
2. 扩展 HostEnvironmentDescriptor，由各 host 注册 platform / privilege / isolation / capabilities。
3. 增加 HostOnboardingRequirement，由各 host 注册欢迎页权限项。
4. 改造 OnboardingStartupRoute，让权限页按 Host requirements 渲染。
5. 删除访问控制模型，把用户可见状态收敛到 AI 能力限制。
6. 为 ToolExecutor 增加 accessSpec(tool)，由 executor 按本次参数返回 READ / WRITE 与边界声明。
7. 改造 ToolExecutionManager，让它传递当前 workspacePath，不承载工具批准。
8. 改造 AIToolHandler，执行前读取 accessSpec(tool)，处理 User Tool Approval，并把 AiPermissionMode 传入默认工具、MCP、终端、文件等具体执行边界。
9. 保留 JsToolManager / JsEngine 包内 toolCall 直连默认工具的能力。
10. 为 TerminalToolExecutor 实现 accessSpec(tool)，按本次调用返回 READ / WRITE；删除终端专用能力名设计。
11. 替换 ToolSettingsPanel 和 AgentInputMenuPopup 的旧三态。
12. 删除旧 ToolPermissionSystem 数据与 UI。
```

这不是 UI 改名任务。核心改动是把“工具权限”从一个 UI 开关，改成按 0-3 层自上而下收紧的工具执行权限链路。其中第 2 层当前收窄为：用户选择的 AiPermissionMode。普通用户看到的“完整权限”是 workspaceWrite + sandbox off 的映射；PackageTool invocation 始终询问。
