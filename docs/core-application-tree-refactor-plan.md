# Core 单树收敛重构计划

## 1. 文档范围

本文档重新说明两件事：

1. 当前 Core 由哪些模块组成，谁创建谁，谁持有什么，调用和生命周期如何流动。
2. 目标上如何把这些模块收敛成一个由 Application 管理的 Core 树，并通过依赖注入连接子模块。

本轮只重写架构文档，不修改 Rust、Dart、生成器或协议实现。

本文中的 CoreApplication、FoundationContext、PersistenceContext、NodeRuntime 等名称是目标架构概念，不代表当前仓库已经存在同名类型。现有类型会在“现状”章节中使用真实名称。

## 2. 核心结论

当前系统不是一棵树，而是多个宿主手工组装出来的一张共享引用图：

```text
Flutter Bridge ──┐
CLI CliCore ─────┼──> LocalCoreProxy ──> OperitApplication
Link server ─────┼──> CoreNodeRouter
Global route ────┘
```

同一批 Runtime、Proxy、Router、Space、Link Access 和后台任务分别被 Flutter、CLI、Web server task、全局 route runtime 持有。结果是：

- 初始化顺序由宿主代码决定。
- 关闭顺序由宿主代码决定。
- Router 的 callback 能力从 Proxy 反向注入。
- Proxy、server 和 runtime 之间出现编译期耦合。
- 第三方必须了解大量底层构造函数，才能启动一个 Core。

目标是建立一个唯一的 Application 根：

```text
CoreApplication
├── Foundation
├── Persistence
├── RuntimeApplication
├── ProviderServices
├── ToolServices
├── PluginServices
├── NodeRuntime
├── AccessServices
├── LocalCoreClient
├── ApplicationCommands
└── Lifecycle
```

这棵树表达所有权和生命周期，不表示所有模块必须放进一个 crate，也不表示 Flutter 要调用 Router。

仍然保持两条完全不同的调用链：

```text
本地应用调用：
Flutter / CLI
  -> GeneratedCoreProxy
  -> LocalCoreProxy
  -> generated_core_dispatch
  -> 本地 Rust Core

Rust 内部路由调用：
Rust 调用带 #[operit_core_route] 的函数
  -> 注解生成的 wrapper
  -> CoreNodeRouter
  -> 本地实现或 PeerLink 上的远端 CoreNode
```

## 3. 当前结构：真实模块和职责

### 3.1 基础契约和通用层

| crate | 当前内容 | 当前角色 |
| --- | --- | --- |
| operit-host-api | HostManager、RuntimeStorage、HTTP、WebSocket、任务调度、时间和平台能力契约 | 所有平台能力的抽象接口 |
| operit-util | 文件、网络、日志、序列化、归档、媒体池、文本处理、Stream、ReverseStream、HotStream 和流式操作符 | 通用基础能力 |
| operit-model | Chat、Message、MessagePart、Memory、Prompt、Model、Workflow、STT/TTS、Node 等共享模型 | 纯数据和协议模型 |
| operit-link | CoreValue、call/watch/push/event、stream、Peer frame、route runtime 接口 | Core 调用和节点传输协议 |

这些模块不拥有 OperitApplication，不能反向依赖业务 Runtime。

### 3.2 Store、身份、Binding 和 Space 数据层

operit-store 不是一个单一数据库类，而是完整的数据层：

```text
operit-store
├── db
│   └── AppDatabase
├── dao
│   ├── ChatDao
│   ├── MessageDao
│   ├── MessagePartDao
│   └── MessageVariantDao
├── repository
│   ├── ChatHistoryManager
│   ├── MemoryRepository
│   ├── WorkflowRepository
│   ├── AvatarRepository
│   ├── CustomEmojiRepository
│   ├── UserMarkdownRepository
│   ├── UsageStatisticsStore
│   ├── RuntimeStorageRepository
│   └── UIHierarchyManager
├── PreferencesDataStore
├── PreferencesEncryption
├── SqliteStore / ObjectBoxStore
├── SyncOperationStore
├── sync
│   └── SqlChatSyncStore
├── RuntimeFileSyncStore
├── RuntimeStorageHost
├── RuntimeStorePaths
├── CoreNodeIdentityStore
├── CoreNodeBindingStore
└── CoreSpaceStore
```

Store 负责：

- 持久化聊天、消息、消息部件、记忆、工作流和配置。
- 保存 CoreNode identity、Binding、Space 成员和设备 profile。
- 保存同步 operation、clock 和文件同步记录。
- 提供 Preferences/Flow 数据存储。

Store 不负责：

- 选择远端执行节点。
- 产生实时 Chat Stream。
- 替代 Flow、Watch 或 PeerLink。
- 通过 SQL 查询驱动实时消息状态。

### 3.3 Provider 和本地模型层

operit-providers 当前包含多个能力域：

```text
operit-providers
├── chat
│   ├── EnhancedAIService
│   ├── enhance
│   │   ├── ConversationService
│   │   ├── ConversationRoundManager
│   │   ├── InputProcessor
│   │   ├── FileBindingService
│   │   ├── ReferenceManager
│   │   └── MultiServiceManager
│   ├── hooks
│   │   ├── PromptHookRegistry
│   │   └── SummaryHookRegistry
│   ├── library
│   │   ├── MemoryLibrary
│   │   └── MemoryAutoSaveScheduler
│   ├── config
│   │   ├── SystemPromptConfig
│   │   ├── FunctionalPrompts
│   │   └── SystemToolPrompts
│   └── llmprovider
│       ├── AIService / AIServiceFactory
│       ├── OpenAI / Claude / Gemini / Deepseek
│       ├── Qwen / Kimi / Mistral / Ollama
│       ├── OpenRouter / Nvidia / Doubao / Mimo
│       ├── NousPortal
│       ├── rate limiting and request concurrency
│       ├── model list and connection testing
│       └── media and structured tool-call bridge
├── stt
│   └── SpeechToTextService and HTTP providers
├── tts
│   ├── VoiceService
│   ├── SystemVoiceProvider
│   ├── OpenAIVoiceProvider
│   └── response pipeline
├── market
│   └── MarketStatsApiService and artifact validation
└── runtime_support
```

operit-local-models 是独立的本地推理能力：

```text
operit-local-models
├── LocalModelCatalog
├── LocalModelRegistry
├── LocalModelRegistryStore
├── LocalModelStorage
├── LocalModelDownload
├── LocalModelManifest
├── LocalEngineCatalog
├── LocalEngineManifest
├── LocalEngineDownload
├── LocalModelProvider
└── LocalInference
```

Provider 和 LocalModel 都是能力服务，不知道 Flutter、CLI、RemoteLinkServer 或 Space 路由。跨节点执行必须由上层 Runtime 的 route wrapper 表达。

### 3.4 Tool、Skill、Package 和 MCP 层

operit-tools 当前是完整的工具执行树：

```text
operit-tools
├── ToolExecutionManager
├── ToolRegistration / ToolGetter
├── AIToolHandler / AIToolHook
├── ToolProgressBus
├── ToolPermissionSystem
├── ToolExecutionLimits
├── ToolJsRuntime
├── files
│   ├── VisualFileSystem
│   └── PathMapper
├── defaultTool
│   ├── filesystem
│   ├── terminal
│   ├── HTTP
│   ├── web visit
│   ├── browser automation
│   ├── system operation
│   ├── memory
│   ├── chat manager
│   ├── music
│   └── Bluetooth
├── skill / skill_runtime
│   ├── SkillManager
│   ├── SkillPackage
│   └── SkillRepository
├── packTool
│   ├── RuntimePackageManager
│   ├── PackageDebugInstallReceiver
│   └── PackageDebugRefreshReceiver
├── mcp
│   ├── MCPManager
│   ├── MCPTool
│   ├── MCPPackage
│   ├── MCPServerConfig
│   └── MCPToolExecutor
├── mcp_runtime
│   ├── MCPLocalServer
│   ├── MCPRepository
│   └── MCP bridge plugins
├── condition
│   └── ConditionEvaluator
└── climode
    └── CliToolModeSupport
```

Tool 层可以使用 Host、Store、Model、Plugin SDK 和 Provider 能力，但不能自己组装 Application，也不能直接选择远端 CoreNode。

### 3.5 Plugin SDK、ToolPkg 和 JavaScript 层

```text
PluginServices
├── operit-plugin-sdk
│   ├── ToolPkg parser/loader/manager
│   ├── package protection and models
│   ├── Hook models
│   ├── Compose DSL
│   ├── Wasm runtime
│   └── JavaScript SDK
├── operit-plugin-sdk-codegen
│   └── TypeScript/runtime binding generation
├── operit-js-bridge
│   ├── JavaScript engine
│   ├── script loader
│   ├── Java bridge
│   ├── external Java code loader
│   ├── JS tool manager
│   └── execution trace
└── operit-runtime/plugins
    ├── PluginRegistry
    ├── toolbox
    └── ToolPkg hook bridges
```

SDK 是公共契约；PluginRegistry、ToolPkg hook bridge 和 JS runtime 实例才属于某一个 CoreApplication。

### 3.6 Runtime 业务层

operit-runtime 是当前业务 Core 的主体：

```text
operit-runtime
├── core
│   ├── application
│   │   ├── OperitApplication
│   │   ├── ActivityLifecycleManager
│   │   └── ForegroundServiceCompat
│   ├── chat
│   │   ├── ChatRuntimeHolder
│   │   ├── ChatRuntimeSlot
│   │   ├── AIMessageManager
│   │   └── MessageProcessingPluginRegistry
│   └── events
│       └── RuntimeEvent
├── services
│   ├── ChatServiceCore
│   │   ├── MessageProcessingDelegate
│   │   ├── MessageCoordinationDelegate
│   │   ├── ChatHistoryDelegate
│   │   └── TokenStatisticsDelegate
│   ├── ProviderRuntimeSupportService
│   ├── ToolRuntimeSupportService
│   ├── LocalProviderService
│   ├── LocalModelService
│   ├── RuntimeHostInfoService
│   ├── RuntimeHostInteractionService
│   ├── RuntimeBrowserService
│   ├── RuntimeTerminalService
│   ├── RuntimeEventIngressService
│   ├── WorkspaceService
│   ├── ArchiveTransferManager
│   ├── SnapshotImportManager
│   ├── SyncBlobTransferManager
│   ├── GitHubOAuthBrokerService
│   ├── SttRecognitionService
│   ├── TtsSynthesisService
│   └── TtsPlaybackService
├── data
│   ├── preferences
│   ├── archive
│   └── backup/import
├── plugins
├── ui
│   └── chat webview/workspace support
└── R.rs
```

OperitApplication 当前负责创建和初始化大量业务运行时，但不统一持有 Router、Link Access、PeerLink 和 server 生命周期。

### 3.7 Route、Node、Space 和 Link Access

operit-route-macros 是过程宏 crate。它将带注解的公开函数展开为 wrapper，保留本地实现，并在 wrapper 中进入 route runtime。

operit-node-runtime 当前包含：

```text
operit-node-runtime
├── CoreNodeRouter
├── CoreNodeLocalRuntime
├── SpaceRuntime
├── RuntimeRemoteLinkService
├── RuntimeRemoteLinkDiscovery
├── SpacePersistenceSyncService
└── generated route catalog
```

CoreNodeRouter 当前负责：

- Binding 解析。
- 本地目标和远程目标选择。
- call/watch/push/handoff 路由。
- PeerLink 转发。
- Space 路由。
- 安装全局 operit-link::CORE_ROUTE_RUNTIME。

CoreNodeLocalRuntime 当前是 Router 的能力容器，包含 shared client、runtime storage、tool/runtime callback、handoff callback、push callback 和 SpaceRuntime。

SpaceRuntime 是 Router 到达本机后的 Space 执行子系统，维护 Space 侧的嵌套 stream 能力。

operit-access-runtime 当前包含：

```text
operit-access-runtime
├── LinkAccessStore
├── CoreNodeIdentityStore integration
├── pairing records
├── inbound/outbound sessions
├── RemoteLinkServer
├── CoreNodePeerLink
├── HTTP/WebSocket authentication
├── device discovery
├── PeerLink carrier
└── static Web/control server
```

Access 负责身份、配对、认证、session 和传输载体；Router 负责路由决策；SpacePersistenceSyncService 负责持久化同步。三者目前被宿主分散启动。

### 3.8 Proxy、Command 和宿主层

operit-proxy-local 当前负责：

- Rust AST scanner。
- Rust schema、dispatch 和 proxy codegen。
- Dart model/proxy 生成。
- LocalCoreProxy。
- 本地 object id。
- CoreStreamPool。
- 本地 call/watch/push。

它当前直接依赖 operit-node-runtime，并且生成器曾经尝试推断 server runtime constructor。这说明 Proxy 仍然知道过多 server 组装细节。

operit-command-core 当前包含：

```text
chat / model / memory / tool / plugin / package / skill
mcp / storage / workspace / host / approval
market / people / prefs / stt / local_models / update / log / tag
```

它是命令编排层，不是另一套 Runtime。

Flutter、CLI 和 Web Access 属于宿主层：

```text
Flutter
├── core/application
├── core/bridge
├── core/host
├── core/link
├── core/link_access
├── core/runtime
├── core/proxy/generated
├── core/space/generated
├── data/preferences
└── ui

CLI
├── bootstrap / main
├── core_proxy
├── chat_runtime
├── cli commands / host_ops
├── link / web_access / transfer
├── mdns / browser_callback
└── tui

Web Access
├── runtime bridge
├── runtime worker
├── model install worker
└── v86 worker
```

这些宿主可以拥有 UI、输入、渲染和传输适配状态，但不能各自拥有一套 Core 业务树。

### 3.9 Host 平台实现层

hosts/ 是 HostManager 的平台实现，不是业务 Core：

```text
hosts
├── common
│   ├── filesystem
│   ├── http
│   ├── scheduler
│   ├── storage
│   ├── terminal
│   └── browser support
├── windows
├── linux
├── macos
├── android
├── ios
├── apple
├── ohos
└── web
    ├── JavaScript runtime
    ├── event/task scheduler
    ├── browser session
    ├── filesystem/storage
    ├── HTTP/WebSocket
    └── local inference
```

Host 实现通过 operit-host-api 注入 Core，不能传播到 Runtime、Proxy 或 route 的业务接口。

## 4. 当前依赖和生命周期问题

### 4.1 当前编译依赖方向

```text
host implementations
        ↓
operit-host-api
        ↓
operit-util ───────────────┐
        ↓                  │
operit-model ──────────┐   │
        ↓              │   │
operit-store           │   │
        ↓              │   │
operit-providers       │   │
operit-tools ──────────┘   │
        ↓                  │
operit-js-bridge ──────────┘
        ↓
operit-runtime
        ├── operit-route-macros
        └── operit-link

operit-access-runtime
        └── link + runtime + store + host

operit-node-runtime
        └── runtime + link-access + link + store

operit-proxy-local
        └── runtime + core-server + link-access + generated inputs

operit-command-core
        └── runtime + providers + tools + store + host
```

这张图是编译依赖图，不是所有权树。当前 operit-runtime 没有反向依赖 operit-node-runtime，但 operit-proxy-local 直接依赖 server，造成 Proxy/server 组装耦合。

### 4.2 当前实际生命周期根

当前至少有四类根：

1. Flutter Bridge 持有本地 LocalCoreProxy 和 OperitApplication。
2. CLI CliCore 持有本地 Proxy/Runtime。
3. Flutter 或 CLI 的 Web/Link server task 持有 Router 和 Access 状态。
4. operit-link::CORE_ROUTE_RUNTIME 全局槽位持有 Router 能力。

因此同一个节点的 Core 实际上由多个对象和任务共同保活。

### 4.3 当前最重要的耦合

1. 宿主直接创建 OperitApplication、LocalCoreProxy、CoreNodeLocalRuntime、CoreNodeRouter、SpaceRuntime 和 RemoteLinkServer。
2. CoreNodeLocalRuntime 通过 callback 反向拿到 Proxy 的 shared client、handoff、tool 和 push 能力。
3. Proxy 依赖 server crate，生成器还尝试把 server runtime constructor 当成 Proxy 构造契约。
4. 全局 route runtime 不属于明确的 Application 生命周期。
5. Store、Provider、Tool、Plugin、LocalModel 和 JS bridge 的参数由不同宿主分别拼装。
6. CLI command、Flutter bridge 和 Web Access 可能直接绕过统一入口访问底层服务。
7. 传输、路由、Space 同步和业务 Runtime 的启动停止没有一个统一 owner。
8. 业务 Stream/Flow、持久化 Store 和跨节点 PeerLink 的边界容易被宿主代码混在一起。

## 5. 目标结构：完整 Core Application 树

### 5.1 目标所有权树

```text
CoreApplication
├── Foundation
│   ├── HostServices
│   ├── ModelServices
│   ├── LinkProtocol
│   └── UtilServices
├── Persistence
│   ├── Database
│   ├── DAOs
│   ├── Repositories
│   ├── Preferences
│   ├── IdentityStore
│   ├── BindingStore
│   ├── SpaceStore
│   ├── RuntimeFileSyncStore
│   └── SyncOperationStore
├── RuntimeApplication
│   ├── OperitApplication
│   │   ├── ActivityLifecycle
│   │   └── ForegroundService
│   ├── ChatRuntimeHolder
│   │   ├── ChatRuntimeSlot::MAIN
│   │   ├── ChatRuntimeSlot::FLOATING
│   │   └── ChatRuntimeSlot::DETACHED(id)
│   ├── ChatServiceCore
│   │   ├── MessageProcessingDelegate
│   │   ├── MessageCoordinationDelegate
│   │   ├── ChatHistoryDelegate
│   │   └── TokenStatisticsDelegate
│   ├── Runtime Services
│   ├── Preferences / Archive / Backup
│   ├── Runtime Events
│   ├── Workspace/WebView
│   └── Plugin Runtime
├── ProviderServices
│   ├── RemoteLLMProviders
│   ├── LocalModelServices
│   ├── STT
│   ├── TTS
│   ├── MemoryLibrary
│   ├── PromptHooks
│   └── MarketServices
├── ToolServices
│   ├── ToolExecution
│   ├── ToolRegistration
│   ├── ToolPermissions
│   ├── BuiltinTools
│   ├── SkillServices
│   ├── PackageServices
│   ├── MCPServices
│   └── ToolJavaScriptRuntime
├── PluginServices
│   ├── PluginRegistry
│   ├── ToolPkgRuntime
│   ├── PluginHookBridges
│   ├── PluginCodegenContracts
│   └── JavaScriptBridge
├── NodeRuntime
│   ├── CoreNodeRouter
│   ├── CoreNodeLocalRuntime
│   ├── Binding resolution
│   ├── SpaceRuntime
│   ├── route catalog
│   └── PeerLink route transport
├── AccessServices
│   ├── LinkAccessStore
│   ├── Identity / Pairing / Session
│   ├── RuntimeRemoteLinkService
│   ├── RuntimeRemoteLinkDiscovery
│   ├── SpacePersistenceSyncService
│   ├── RemoteLinkServer
│   ├── PeerLink carrier
│   └── static Web/control server
├── LocalCoreClient
│   ├── LocalCoreProxy
│   ├── generated_core_dispatch
│   └── CoreStreamPool
├── ApplicationCommands
│   └── operit-command-core
└── Lifecycle
    ├── start
    ├── running
    ├── stop services
    └── shutdown
```

### 5.2 每个节点的所有权

- CoreApplication 是唯一公开的 composition root。
- Foundation 只提供契约和基础能力，不拥有业务服务。
- Persistence 由 CoreApplication 创建，向 Runtime、NodeRuntime 和 AccessServices 提供存储句柄。
- ProviderServices、ToolServices 和 PluginServices 由 CoreApplication 创建，作为 Runtime 的能力子树。
- RuntimeApplication 拥有 Chat、Workspace、Preference、Event、Plugin runtime 和业务服务实例。
- NodeRuntime 拥有 Router、Binding route 和 Space 本地执行能力。
- AccessServices 拥有配对、认证、session、PeerLink 和 server 任务。
- LocalCoreClient 只是应用拿到的本地调用句柄；当前实现仍是 LocalCoreProxy + generated_core_dispatch + CoreStreamPool，不新增第二套 Proxy。
- ApplicationCommands 只做命令编排和输出适配，不拥有业务状态。
- Flutter、CLI、Web Access 和 TUI 都是 HostSurface，不是 CoreApplication 的平行根。

### 5.3 依赖注入规则

依赖注入的目标不是把整个根对象传给每个模块，而是让父节点创建实例并向下传递窄能力接口：

```text
CoreApplication
  creates Foundation
  creates Persistence
  creates ProviderServices from Foundation + Persistence
  creates ToolServices from Foundation + Persistence + Providers
  creates RuntimeApplication from Foundation + Persistence + Providers + Tools + Plugins
  creates NodeRuntime from Foundation + Persistence + Runtime local contracts
  creates AccessServices from Foundation + Persistence + Node transport
  exposes LocalCoreClient and ApplicationCommands
```

规则：

1. 只有 composition root 创建长期实例。
2. 父节点向子节点传依赖，子节点不反向查找父节点。
3. 传递 trait/context/handle，不传完整 CoreApplication。
4. 子模块不自行创建 Host、Store、Router、SpaceRuntime 或 LinkAccessStore。
5. 不使用隐藏全局 registry 或 Service Locator。
6. 全局 route runtime 由 Application 安装和清理。
7. 共享 Store、Host、Flow bus、Stream pool 通过显式共享句柄注入。
8. Provider、Tool、ChatServiceCore 不直接持有网络 server。
9. NodeRuntime 只依赖 Runtime 暴露的本地执行能力，不依赖 Flutter。
10. 测试也从同一 composition root 装配，只替换 Host/Store/Transport contract。

所有权图必须是一棵树，但依赖引用可以是无环有向图：

```text
所有权：
CoreApplication -> RuntimeApplication -> ChatServiceCore

引用：
ProviderServices -> Persistence
ToolServices -> ProviderServices
RuntimeApplication -> ProviderServices + ToolServices
NodeRuntime -> Runtime local contracts
AccessServices -> Node transport
```

不能为了字面树结构复制 Store、Host 或 Stream bus，也不能让子模块互相拥有。

## 6. 目标调用和数据链路

### 6.1 Flutter/CLI 本地链路

```text
Flutter / CLI
  -> CoreApplication.localClient()
  -> GeneratedCoreProxy
  -> LocalCoreProxy
  -> generated_core_dispatch
  -> RuntimeApplication / ordinary Core function
```

这条链路不经过 CoreNodeRouter、SpaceRuntime、PeerLink、RemoteLinkServer 或远端 Proxy。

### 6.2 Rust 内部 route 链路

```text
Runtime / Provider / Tool 内部调用带 #[operit_core_route] 的函数
  -> attribute macro generated wrapper
  -> operit-link CoreRouteRuntime
  -> NodeRuntime / CoreNodeRouter
  -> local target or PeerLink
  -> remote CoreNodeRouter
  -> remote local implementation
```

Proxy 不得绕过 wrapper 直接调用 local implementation。

### 6.3 Space 持久化同步链路

```text
SpacePersistenceSyncService
  -> PeerLink
  -> operation log / vector clock
  -> remote store apply
```

SQL 负责保存和恢复数据及同步操作，不是实时消息流的驱动源。

### 6.4 Space 实时链路

```text
CoreNodeRouter
  -> PeerLink
  -> remote CoreNodeRouter
  -> call / watch / push / stream event
```

Space 实时传输和持久化同步使用不同服务，但由同一个 CoreApplication 管理生命周期。

### 6.5 Link Access 控制链路

```text
pairing / discovery / session / authentication
  -> PeerLink carrier
  -> NodeRuntime
```

静态 Web server、配对 API、设备发现和 session 不能重新变成 Flutter 到远端 Core 的 Proxy 链路。

## 7. 目标生命周期

### 7.1 启动顺序

```text
1. 创建 HostServices
2. 初始化 Foundation contracts
3. 创建 Persistence
4. 初始化 CoreNode identity / Space / Binding stores
5. 创建 ProviderServices
6. 创建 ToolServices
7. 创建 PluginServices / JS bridge
8. 创建 RuntimeApplication
9. 创建 NodeRuntime 和 CoreNodeRouter
10. 创建 AccessServices 和 PeerLink carrier
11. 安装 Application-owned route runtime
12. 创建 LocalCoreClient
13. 启动后台同步、发现、server 和控制面
14. 对外返回 CoreApplication handle
```

### 7.2 关闭顺序

```text
1. 停止接受新的应用调用
2. 停止 Web/control server 和配对入口
3. 关闭新的 PeerLink route 请求
4. 结束 watch/push/stream transport
5. 停止 Space persistence sync
6. 停止 Runtime 后台任务和 Provider/Tool 执行
7. 清理全局 route runtime
8. 关闭 Router、Access、Runtime、Plugin、Store 和 Host resources
```

所有后台任务都必须由 CoreApplication 保存的 service handle 管理，不能依靠宿主字段或任务泄漏来保活。

## 8. 迁移计划

### Phase 0：建立完整基线

- 记录每个宿主当前创建的 Runtime、Proxy、Router、Space、Access 和 server 对象。
- 记录每个后台 task 的创建者和关闭者。
- 为本地调用、Rust route、Space 实时传输、持久化同步、配对控制面分别保留链路图。
- 冻结当前行为，不通过改名掩盖所有权问题。

### Phase 1：定义 Context 和 Service contracts

- 定义 Foundation、Persistence、Provider、Tool、Plugin、Runtime、Node、Access 的注入接口。
- 定义每个子树的 owner、启动状态、关闭状态和共享句柄。
- 定义 localClient、access 和 shutdown 的公共 facade 形态。
- 将 route runtime 的 install/clear 纳入生命周期合同。

### Phase 2：建立 composition root

- 在高层 facade/composition crate 组装完整 CoreApplication。
- 同时装配 Store、LocalModel、Provider、Tool、Plugin、JS、Runtime、Node 和 Access。
- 保持 operit-runtime 不依赖 operit-node-runtime。
- 将 CoreNodeLocalRuntime 的 callback 组装收进 NodeRuntime，不由 Flutter/CLI 拼接。

### Phase 3：迁移 Flutter、CLI 和 Web Access

- Flutter bridge 只持有 CoreApplication facade、local client 和 Access handles。
- CLI CliCore 只持有同一个 facade，不再以 _coreNodeRouter 单独保活。
- Command Core、TUI、Flutter UI 和 Web Access 通过 facade 获取能力。
- 保留静态 Web server、配对、设备发现和控制面，但交给 AccessServices 生命周期管理。

### Phase 4：收紧 Proxy 生成器

- 继续扫描普通 server Core 函数，使其成为正常本地 Core API。
- 删除根据类型名推断 route 特例的逻辑。
- 删除把任意 server new(...) 推断为 Proxy 构造契约的逻辑。
- Proxy 只生成本地调用面，不生成 Router/Space/RemoteLink 组装。
- 将 Proxy 对 server crate 的依赖收敛到生成输入或明确的公共 contract。

### Phase 5：收敛 Route、Space 和 Access

- Router 只由 NodeRuntime 创建和持有。
- CoreNodeLocalRuntime 只作为 NodeRuntime 内部能力容器。
- SpaceRuntime 作为 Router 的本地 Space 执行子系统。
- RuntimeRemoteLinkService、RuntimeRemoteLinkDiscovery、SpacePersistenceSyncService 和 PeerLink carrier 由 Access/Node service handles 管理。
- route wrapper、PeerLink 和本地执行之间保持单一标准 Link 协议。

### Phase 6：收缩公开底层构造

- 限制 Router、BindingStore、SpaceRuntime、RemoteLinkServer 和 LocalRuntime 的直接构造。
- 删除宿主中的重复保活字段、裸 Router clone 和匿名 callback 组装。
- 将 Store、Provider、Tool、Plugin、LocalModel 和 JS bridge 参数纳入 Application 子配置。
- 对第三方只暴露 facade、local client、Access 状态和生命周期句柄。

### Phase 7：验证

必须验证：

1. Flutter 本地 call/watch/push。
2. CLI 本地 call/watch/push。
3. Rust 内部注解函数必经 wrapper 和 Router。
4. Router 本地目标和远端目标。
5. Space 加入、退出、成员失效、配对取消。
6. 两端同时订阅同一 Flow/Stream。
7. 持久化同步断线、重连和时钟收敛。
8. 本地模型下载和推理生命周期。
9. Tool、Skill、Package、MCP、Plugin、JS 执行生命周期。
10. Web/control server 启停。
11. Application shutdown 后所有后台 task、Router、PeerLink、sync、server 和 global route runtime 释放。

## 9. 验收标准

- Core 只有一个 Application 生命周期根。
- Flutter、CLI、Web Access 不再直接组装 Runtime、Router、SpaceRuntime、Proxy 和 RemoteLinkServer。
- LocalCoreClient 只是 facade handle，真实本地调用仍是 LocalCoreProxy -> generated_core_dispatch。
- 普通 server Core 函数仍能作为普通本地函数生成到 Proxy。
- 带 route 注解的 Rust 函数调用始终经过 wrapper。
- Proxy 不包含 Router、Space、PeerLink 或远端 Proxy 链路。
- operit-runtime 不依赖 operit-node-runtime。
- Router 只由 NodeRuntime 管理。
- Access 只负责配对、认证、session、传输和控制面。
- Space 持久化同步不替代实时 Flow/Watch/Stream。
- Store 不成为实时消息驱动源。
- Provider、Tool、Plugin、LocalModel 和 JS runtime 都属于同一个 CoreApplication 生命周期。
- 第三方只需要统一 facade，不需要理解底层 callback 和构造顺序。

## 10. 明确不做

- 不把 route 接回 Flutter/CLI 外层协议。
- 不新增第二套远程 Proxy。
- 不恢复应用层 /link/call 作为远程业务入口。
- 不通过 Proxy 类型黑名单掩盖所有权设计。
- 不删除配对、设备发现、静态 Web server 或控制面。
- 不把 ChatMessage、SQL 或数据库查询变成实时 Stream 的驱动源。
- 不通过复制 Store、Host、Flow bus 或 Stream pool 来伪造树结构。
- 不让 operit-runtime 反向依赖 operit-node-runtime。

## 11. 最终目标

最终对第三方暴露的是一个完整 Core，而不是一组散装构造函数：

```rust
let core = CoreApplication::start(config).await?;
let local = core.local_client();
let access = core.access();

// local: current-process Core calls
// access: pairing, discovery, Space and Link state

core.shutdown().await?;
```

调用者只面对 CoreApplication 的生命周期和明确的 service handle；Core 内部才负责 Runtime、Provider、Tool、Plugin、Store、Route、Space、Link Access、Host 和本地 Proxy 的完整组装。




