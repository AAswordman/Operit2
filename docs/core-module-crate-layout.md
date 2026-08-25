# Core 目标嵌套 Crate 结构

## 1. 说明

这份文档只描述目标目录和 crate 边界，不修改当前代码。

目标目录格式固定为：

~~~text
core/crates/
└── 一级目录/
    └── 二级目录/
        ├── Cargo.toml
        └── src/lib.rs
~~~

每个二级目录都是一个独立 crate。一级目录只做归类，不承担运行时对象所有权。真正的所有权由 CoreApplication 负责。

目标拆分原则：

- 二级 crate 只保留一个主要职责。
- 每个二级 crate 有明确创建者和关闭者。
- 构造参数通过 Context 或 trait 注入。
- 子 crate 不通过全局变量寻找父 crate。
- 子 crate 不创建兄弟 crate。
- 依赖只能向下或横向通过 contract，不允许形成反向环。

## 2. 完整目标目录

~~~text
core/crates/
├── foundation/
│   ├── host-api/Cargo.toml
│   ├── model/Cargo.toml
│   ├── util/Cargo.toml
│   ├── link/Cargo.toml
│   └── contracts/Cargo.toml
├── persistence/
│   ├── core/Cargo.toml
│   ├── preferences/Cargo.toml
│   ├── chat/Cargo.toml
│   ├── memory/Cargo.toml
│   ├── workspace/Cargo.toml
│   ├── node/Cargo.toml
│   └── sync/Cargo.toml
├── provider/
│   ├── contracts/Cargo.toml
│   ├── llm/Cargo.toml
│   ├── chat/Cargo.toml
│   ├── media/Cargo.toml
│   ├── memory/Cargo.toml
│   ├── market/Cargo.toml
│   └── local-model/Cargo.toml
├── tool/
│   ├── contracts/Cargo.toml
│   ├── runtime/Cargo.toml
│   ├── builtin/Cargo.toml
│   ├── skill/Cargo.toml
│   ├── package/Cargo.toml
│   ├── mcp/Cargo.toml
│   └── javascript/Cargo.toml
├── plugin/
│   ├── sdk/Cargo.toml
│   ├── codegen/Cargo.toml
│   ├── runtime/Cargo.toml
│   └── javascript-bridge/Cargo.toml
├── runtime/
│   ├── contracts/Cargo.toml
│   ├── application/Cargo.toml
│   ├── chat/Cargo.toml
│   ├── preferences/Cargo.toml
│   ├── workspace/Cargo.toml
│   ├── host/Cargo.toml
│   ├── transfer/Cargo.toml
│   ├── plugin-host/Cargo.toml
│   └── events/Cargo.toml
├── node/
│   ├── route-macros/Cargo.toml
│   ├── contracts/Cargo.toml
│   ├── router/Cargo.toml
│   ├── local-runtime/Cargo.toml
│   ├── space-runtime/Cargo.toml
│   └── space-sync/Cargo.toml
├── access/
│   ├── identity/Cargo.toml
│   ├── auth/Cargo.toml
│   ├── pairing/Cargo.toml
│   ├── discovery/Cargo.toml
│   ├── peer-link/Cargo.toml
│   ├── server/Cargo.toml
│   └── web/Cargo.toml
├── proxy/
│   ├── codegen/Cargo.toml
│   └── local/Cargo.toml
├── command/
│   └── core/Cargo.toml
└── application/
    └── core/Cargo.toml
~~~

目标 package 名称统一使用路径含义，例如：

~~~text
foundation/contracts       -> operit-core-contracts
persistence/chat           -> operit-chat-store
provider/contracts         -> operit-provider-contracts
runtime/application        -> operit-runtime-application
node/router                -> operit-core-node
access/peer-link           -> operit-peer-link
proxy/codegen              -> operit-proxy-scan
proxy/local                -> operit-proxy-local
application/core           -> operit-core-application
~~~

## 3. Foundation

### 3.1 foundation/host-api/Cargo.toml

对应当前 operit-host-api。

职责：

- HostManager。
- RuntimeStorageHost。
- HTTP、WebSocket、任务调度、时间和平台能力契约。

创建输入：

~~~text
无业务输入。
由各平台 Host 实现提供具体实现。
~~~

直接依赖：

~~~text
serde
serde_json
~~~

禁止依赖：

~~~text
operit-runtime
operit-core-node
operit-core-application
operit-proxy-local
~~~

### 3.2 foundation/model/Cargo.toml

对应当前 operit-model。

职责：

- Chat、Message、MessagePart。
- Memory、Prompt、Model。
- Workflow、STT/TTS、Node 等共享数据模型。

创建输入：

~~~text
纯数据类型。
不接收 Runtime、Store 或 Host 实例。
~~~

直接依赖：

~~~text
operit-host-api
operit-link
operit-util
serde
chrono
uuid
~~~

禁止依赖：

~~~text
operit-runtime
operit-providers
operit-tools
operit-core-node
~~~

### 3.3 foundation/util/Cargo.toml

对应当前 operit-util。

职责：

- 文件、网络、日志、归档、媒体池。
- 文本、Markdown、JSON、序列化。
- Stream、ReverseStream、HotStream 和通用流操作符。

创建输入：

~~~text
HostServices 中需要的基础 host contract。
~~~

直接依赖：

~~~text
operit-host-api
serde
serde_json
chrono
~~~

禁止依赖：

~~~text
operit-runtime
operit-store
operit-providers
operit-core-node
~~~

### 3.4 foundation/link/Cargo.toml

对应当前 operit-link。

职责：

- CoreValue。
- call、watch、push、event、stream 协议类型。
- Peer frame 的通用协议语义。
- CoreRouteRuntime contract。

创建输入：

~~~text
协议常量和编解码配置。
不接收 Runtime、Router 或 Store。
~~~

直接依赖：

~~~text
serde
serde_json
serde_bytes
ciborium
~~~

禁止依赖：

~~~text
operit-runtime
operit-core-node
operit-access-runtime
operit-proxy-local
~~~

### 3.5 foundation/contracts/Cargo.toml

新建的跨域 contract crate。

职责：

- Runtime 对 NodeRuntime 暴露的本地执行 contract。
- Provider、Tool、Plugin、Store 之间的最小 trait。
- Host、Storage、Stream、Event 和生命周期句柄的抽象。

创建输入：

~~~text
FoundationContext
PersistenceContext
RuntimeContext
NodeContext
AccessContext
~~~

直接依赖：

~~~text
foundation/host-api
foundation/model
foundation/link
foundation/util
~~~

禁止依赖：

~~~text
operit-runtime-application
operit-core-node
operit-proxy-local
operit-access-runtime
~~~

## 4. Persistence

### 4.1 persistence/core/Cargo.toml

从当前 operit-store 中抽出通用数据库和存储核心。

职责：

- AppDatabase。
- SqliteStore、ObjectBoxStore。
- RuntimeStorageHost 适配。
- RuntimeStorePaths。
- 通用事务和 storage handle。

创建输入：

~~~text
HostManager.runtimeStorageHost
StorageConfig
~~~

直接依赖：

~~~text
foundation/host-api
foundation/model
foundation/util
rusqlite
~~~

### 4.2 persistence/preferences/Cargo.toml

职责：

- PreferencesDataStore。
- PreferencesEncryption。
- 偏好数据 Flow。

创建输入：

~~~text
PersistenceCore
StoragePath
EncryptionKeyProvider
~~~

直接依赖：

~~~text
persistence/core
foundation/model
foundation/util
~~~

### 4.3 persistence/chat/Cargo.toml

职责：

- ChatDao。
- MessageDao。
- MessagePartDao。
- MessageVariantDao。
- ChatHistoryRepository。

创建输入：

~~~text
PersistenceCore
ChatStoreConfig
~~~

直接依赖：

~~~text
persistence/core
foundation/model
~~~

### 4.4 persistence/memory/Cargo.toml

职责：

- MemoryRepository。
- MemoryAutoSaveCandidateRepository。

创建输入：

~~~text
PersistenceCore
~~~

直接依赖：

~~~text
persistence/core
foundation/model
~~~

### 4.5 persistence/workspace/Cargo.toml

职责：

- WorkflowRepository。
- UserMarkdownRepository。
- UIHierarchyManager。
- Workspace 数据仓储。

创建输入：

~~~text
PersistenceCore
WorkspaceStorageConfig
~~~

直接依赖：

~~~text
persistence/core
foundation/model
~~~

### 4.6 persistence/node/Cargo.toml

职责：

- CoreNodeIdentityStore。
- CoreNodeBindingStore。
- CoreSpaceStore。

创建输入：

~~~text
PersistenceCore
NodeStorageConfig
~~~

直接依赖：

~~~text
persistence/core
foundation/model
~~~

### 4.7 persistence/sync/Cargo.toml

职责：

- SyncOperationStore。
- SqlChatSyncStore。
- RuntimeFileSyncStore。
- operation log 和 vector clock 的持久化。

创建输入：

~~~text
PersistenceCore
SyncStorageConfig
~~~

直接依赖：

~~~text
persistence/core
persistence/chat
persistence/node
foundation/model
foundation/link
~~~

禁止依赖：

~~~text
operit-core-node
operit-runtime-chat
operit-provider-llm
~~~

## 5. Provider

### 5.1 provider/contracts/Cargo.toml

职责：

- AIService。
- AIServiceFactory。
- Provider 请求、响应、模型配置 contract。
- ProviderRuntimeContext。

创建输入：

~~~text
ProviderConfig
Host HTTP contract
LocalModel contract
~~~

直接依赖：

~~~text
foundation/host-api
foundation/model
foundation/util
~~~

禁止依赖：

~~~text
operit-runtime-application
operit-core-node
operit-proxy-local
~~~

### 5.2 provider/llm/Cargo.toml

职责：

- OpenAI、Claude、Gemini、Deepseek。
- Qwen、Kimi、Mistral、Ollama。
- OpenRouter、Nvidia、Doubao、Mimo、NousPortal。
- Rate limit、request concurrency、model list 和 connection test。

创建输入：

~~~text
ProviderContracts
Host HTTP client
ProviderConfig
~~~

直接依赖：

~~~text
provider/contracts
foundation/host-api
foundation/model
foundation/util
~~~

### 5.3 provider/chat/Cargo.toml

职责：

- EnhancedAIService。
- ConversationService。
- ConversationRoundManager。
- InputProcessor。
- FileBindingService。
- ReferenceManager。
- PromptHookRegistry。
- SummaryHookRegistry。

创建输入：

~~~text
ProviderContracts
LLMProviderRegistry
ChatStore
MemoryStore
ToolContracts
~~~

直接依赖：

~~~text
provider/contracts
provider/llm
persistence/chat
persistence/memory
tool/contracts
foundation/model
foundation/util
~~~

### 5.4 provider/media/Cargo.toml

职责：

- SpeechToTextService。
- VoiceService。
- STT/TTS provider。

创建输入：

~~~text
ProviderContracts
Host HTTP contract
Audio/TTS configuration
~~~

直接依赖：

~~~text
provider/contracts
foundation/host-api
foundation/model
foundation/util
~~~

### 5.5 provider/memory/Cargo.toml

职责：

- MemoryLibrary。
- MemoryAutoSaveScheduler。

创建输入：

~~~text
MemoryStore
ProviderContracts
Scheduler contract
~~~

直接依赖：

~~~text
provider/contracts
persistence/memory
foundation/model
foundation/host-api
~~~

### 5.6 provider/market/Cargo.toml

职责：

- MarketStatsApiService。
- ArtifactAuthorValidation。

创建输入：

~~~text
ProviderContracts
Host HTTP contract
~~~

直接依赖：

~~~text
provider/contracts
foundation/host-api
foundation/model
~~~

### 5.7 provider/local-model/Cargo.toml

职责：

- LocalModelCatalog。
- LocalModelRegistry。
- LocalModelStorage。
- LocalModelDownload。
- LocalEngineCatalog。
- LocalEngineDownload。
- LocalInference。

创建输入：

~~~text
Host filesystem/storage/process contract
LocalModelConfig
~~~

直接依赖：

~~~text
foundation/host-api
foundation/model
foundation/util
~~~

## 6. Tool

### 6.1 tool/contracts/Cargo.toml

职责：

- AITool。
- ToolResult。
- ToolRegistration。
- ToolExecutionContext。
- ToolPermission contract。

创建输入：

~~~text
ToolDefinition
Host capability contract
Model types
~~~

直接依赖：

~~~text
foundation/host-api
foundation/model
foundation/link
~~~

禁止依赖：

~~~text
operit-runtime-application
operit-core-node
operit-proxy-local
~~~

### 6.2 tool/runtime/Cargo.toml

职责：

- ToolExecutionManager。
- AIToolHandler。
- ToolProgressBus。
- ToolExecutionLimits。
- ToolPermissionSystem。

创建输入：

~~~text
ToolContracts
HostServices
Persistence handles
Provider handles
~~~

直接依赖：

~~~text
tool/contracts
foundation/host-api
foundation/model
foundation/link
persistence/core
~~~

### 6.3 tool/builtin/Cargo.toml

职责：

- filesystem、terminal、HTTP、browser。
- memory、chat、system、music、Bluetooth 工具。

创建输入：

~~~text
ToolRuntime
HostServices
Runtime local contracts
~~~

直接依赖：

~~~text
tool/runtime
tool/contracts
foundation/host-api
foundation/model
foundation/util
~~~

### 6.4 tool/skill/Cargo.toml

职责：

- SkillManager。
- SkillPackage。
- SkillRepository。

创建输入：

~~~text
ToolRuntime
Storage handles
Plugin SDK contracts
~~~

直接依赖：

~~~text
tool/runtime
tool/contracts
persistence/core
plugin/sdk
foundation/model
~~~

### 6.5 tool/package/Cargo.toml

职责：

- RuntimePackageManager。
- Package installation、refresh 和 execution。

创建输入：

~~~text
ToolRuntime
Plugin SDK
PackageStorage
~~~

直接依赖：

~~~text
tool/runtime
plugin/sdk
persistence/core
foundation/host-api
~~~

### 6.6 tool/mcp/Cargo.toml

职责：

- MCPManager。
- MCPTool。
- MCPPackage。
- MCPServerConfig。
- MCPToolExecutor。
- MCPLocalServer。
- MCPRepository。

创建输入：

~~~text
ToolRuntime
Host process/network contract
MCP configuration store
~~~

直接依赖：

~~~text
tool/runtime
tool/contracts
persistence/core
foundation/host-api
foundation/model
~~~

### 6.7 tool/javascript/Cargo.toml

职责：

- Tool JavaScript runtime。
- Tool JS execution context。

创建输入：

~~~text
ToolRuntime
Plugin JavaScript contract
Host scheduler
~~~

直接依赖：

~~~text
tool/runtime
plugin/javascript-bridge
foundation/host-api
foundation/model
~~~

## 7. Plugin

### 7.1 plugin/sdk/Cargo.toml

对应当前 operit-plugin-sdk。

职责：

- ToolPkg models。
- Package protection。
- Hook models。
- Compose DSL。
- Wasm runtime。
- JavaScript SDK。

创建输入：

~~~text
Package data
Host crypto/storage contract
~~~

直接依赖：

~~~text
foundation/host-api
plugin/codegen
serde
serde_json
~~~

### 7.2 plugin/codegen/Cargo.toml

对应当前 operit-plugin-sdk-codegen。

职责：

- 从 Rust 声明生成 TypeScript/runtime bindings。

创建输入：

~~~text
Rust source AST
Codegen configuration
~~~

直接依赖：

~~~text
syn
quote
proc-macro2
~~~

### 7.3 plugin/runtime/Cargo.toml

职责：

- PluginRegistry。
- BuiltinPluginAssets。
- BundledExternalSkillAssets。
- ToolPkg lifecycle bridge。

创建输入：

~~~text
Plugin SDK
Tool services
Runtime context
Storage handles
~~~

直接依赖：

~~~text
plugin/sdk
tool/contracts
tool/runtime
persistence/core
foundation/host-api
~~~

### 7.4 plugin/javascript-bridge/Cargo.toml

对应当前 operit-js-bridge。

职责：

- JavaScript engine。
- Script loader。
- Java bridge。
- External Java code loader。
- JS tool manager。

创建输入：

~~~text
Host scheduler
Host storage
Plugin SDK
~~~

直接依赖：

~~~text
foundation/host-api
plugin/sdk
foundation/util
~~~

## 8. Runtime

### 8.1 runtime/contracts/Cargo.toml

职责：

- Runtime 对 Provider、Tool、Node、Access 暴露的最小 contract。
- Chat execution contract。
- Runtime storage、event、stream 和 handoff contract。

创建输入：

~~~text
无运行时实例。
只定义 trait、request、response 和 capability handle。
~~~

直接依赖：

~~~text
foundation/contracts
foundation/model
foundation/link
~~~

### 8.2 runtime/application/Cargo.toml

职责：

- OperitApplication。
- ActivityLifecycleManager。
- ForegroundServiceCompat。
- Runtime 级启动和关闭。

创建输入：

~~~text
HostServices
Persistence handles
ProviderServices
ToolServices
PluginServices
RuntimeConfig
~~~

直接依赖：

~~~text
runtime/contracts
runtime/chat
runtime/preferences
runtime/workspace
runtime/host
runtime/transfer
runtime/plugin-host
runtime/events
provider/chat
provider/media
provider/local-model
tool/builtin
tool/skill
tool/package
tool/mcp
plugin/runtime
~~~

### 8.3 runtime/chat/Cargo.toml

职责：

- ChatRuntimeHolder。
- ChatRuntimeSlot。
- ChatServiceCore。
- MessageProcessingDelegate。
- MessageCoordinationDelegate。
- ChatHistoryDelegate。
- TokenStatisticsDelegate。
- AIMessageManager。

创建输入：

~~~text
RuntimeContracts
ChatStore
Provider chat services
Tool runtime
RuntimeConfig
~~~

直接依赖：

~~~text
runtime/contracts
persistence/chat
persistence/preferences
provider/chat
tool/runtime
foundation/model
foundation/util
~~~

### 8.4 runtime/preferences/Cargo.toml

职责：

- ActivePromptManager。
- ModelConfigManager。
- UserPreferencesManager。
- Tts/Stt configuration managers。
- Functional and environment preferences。

创建输入：

~~~text
PreferencesStore
ModelCatalog
RuntimeConfig
~~~

直接依赖：

~~~text
runtime/contracts
persistence/preferences
foundation/model
~~~

### 8.5 runtime/workspace/Cargo.toml

职责：

- WorkspaceService。
- Workspace webview。
- Workspace templates。
- Workspace backup。
- Attachment processing。

创建输入：

~~~text
WorkspaceStore
Host filesystem/browser contract
Runtime events
~~~

直接依赖：

~~~text
runtime/contracts
persistence/workspace
foundation/host-api
foundation/model
foundation/util
~~~

### 8.6 runtime/host/Cargo.toml

职责：

- RuntimeHostInfoService。
- RuntimeHostInteractionService。
- RuntimeBrowserService。
- RuntimeTerminalService。
- RuntimeEventIngressService。

创建输入：

~~~text
HostServices
Runtime event bus
Host interaction callbacks
~~~

直接依赖：

~~~text
runtime/contracts
foundation/host-api
foundation/model
foundation/link
~~~

### 8.7 runtime/transfer/Cargo.toml

职责：

- ArchiveTransferManager。
- SnapshotImportManager。
- SyncBlobTransferManager。

创建输入：

~~~text
Storage handles
Host filesystem/storage contract
Runtime event bus
~~~

直接依赖：

~~~text
runtime/contracts
persistence/core
persistence/sync
foundation/host-api
foundation/model
~~~

### 8.8 runtime/plugin-host/Cargo.toml

职责：

- Runtime 内的 PluginRegistry 持有。
- ToolPkg hook bridge。
- Plugin lifecycle。

创建输入：

~~~text
Plugin runtime
Tool runtime
Chat runtime event contract
~~~

直接依赖：

~~~text
runtime/contracts
plugin/runtime
tool/runtime
foundation/model
~~~

### 8.9 runtime/events/Cargo.toml

职责：

- RuntimeEvent。
- Runtime event bus contract。
- Event ingress。

创建输入：

~~~text
EventConfig
Scheduler handle
~~~

直接依赖：

~~~text
runtime/contracts
foundation/model
foundation/link
~~~

## 9. Node

### 9.1 node/route-macros/Cargo.toml

对应当前 operit-route-macros。

职责：

- operit_core_route attribute macro。
- wrapper 展开。
- route metadata 展开。

创建输入：

~~~text
Rust source AST
Route binding configuration
~~~

直接依赖：

~~~text
syn
quote
proc-macro2
~~~

禁止依赖：

~~~text
operit-proxy-local
operit-runtime-application
operit-access-runtime
~~~

### 9.2 node/contracts/Cargo.toml

职责：

- Node route contract。
- Binding resolver contract。
- Local runtime capability contract。
- Peer transport contract。

创建输入：

~~~text
RouteSchema
BindingKey
LocalRuntimeCapabilities
PeerTransportHandle
~~~

直接依赖：

~~~text
foundation/contracts
foundation/model
foundation/link
~~~

### 9.3 node/router/Cargo.toml

对应当前 CoreNodeRouter 的主体。

职责：

- CoreNodeRouter。
- Binding 解析。
- local/remote target selection。
- call/watch/push/handoff dispatch。
- route catalog。

创建输入：

~~~text
NodeContracts
NodeStore
LocalRuntimeHandle
PeerTransportHandle
SpaceRuntimeHandle
~~~

直接依赖：

~~~text
node/contracts
persistence/node
foundation/link
foundation/model
~~~

禁止依赖：

~~~text
Flutter bridge
CLI TUI
proxy/local
~~~

### 9.4 node/local-runtime/Cargo.toml

对应当前 CoreNodeLocalRuntime 的能力容器。

职责：

- Runtime 本地能力适配。
- RuntimeStorageHost。
- Tool/runtime callback contract。
- Handoff、push 和 local execution capability。

创建输入：

~~~text
RuntimeContracts
Runtime local capability handle
Storage handle
SpaceRuntimeHandle
~~~

直接依赖：

~~~text
node/contracts
runtime/contracts
foundation/host-api
foundation/link
~~~

禁止依赖：

~~~text
Flutter bridge
CLI
GeneratedCoreProxy
~~~

### 9.5 node/space-runtime/Cargo.toml

对应当前 SpaceRuntime。

职责：

- Space 注解函数的本地执行。
- Space stream pool。
- Space route local target。

创建输入：

~~~text
Runtime local capability contract
Space route catalog
Space stream pool config
~~~

直接依赖：

~~~text
node/contracts
runtime/contracts
foundation/link
foundation/model
~~~

### 9.6 node/space-sync/Cargo.toml

对应当前 SpacePersistenceSyncService 的节点侧调度部分。

职责：

- Space persistence sync orchestration。
- Peer operation exchange。
- Remote store apply scheduling。

创建输入：

~~~text
SyncStore
SpaceStore
PeerTransportHandle
SyncConfig
~~~

直接依赖：

~~~text
persistence/sync
persistence/node
access/peer-link
foundation/link
foundation/model
~~~

## 10. Access

### 10.1 access/identity/Cargo.toml

职责：

- CoreNode identity。
- Device profile。
- Identity persistence adapter。

创建输入：

~~~text
NodeStore
DeviceInfo
~~~

直接依赖：

~~~text
persistence/node
foundation/model
~~~

### 10.2 access/auth/Cargo.toml

职责：

- Token。
- HMAC。
- Session signature。
- Remote authorization contract。

创建输入：

~~~text
AuthConfig
Identity handle
Session secret
~~~

直接依赖：

~~~text
foundation/link
foundation/model
sha2
hmac
~~~

### 10.3 access/pairing/Cargo.toml

职责：

- Pairing records。
- Pairing handshake。
- Inbound/outbound session record。

创建输入：

~~~text
Identity handle
NodeStore
Auth service
PairingConfig
~~~

直接依赖：

~~~text
access/identity
access/auth
persistence/node
foundation/model
foundation/link
~~~

### 10.4 access/discovery/Cargo.toml

职责：

- Device discovery。
- Peer announcement。
- Discovery state。

创建输入：

~~~text
Identity handle
Host network contract
DiscoveryConfig
~~~

直接依赖：

~~~text
access/identity
foundation/host-api
foundation/model
~~~

### 10.5 access/peer-link/Cargo.toml

对应当前 CoreNodePeerLink 和 carrier 部分。

职责：

- PeerFrame。
- PeerConnection。
- PeerLink registration。
- HTTP/WebSocket carrier。

创建输入：

~~~text
Auth service
Identity handle
TransportConfig
NodeTransportReceiver
~~~

直接依赖：

~~~text
foundation/link
foundation/host-api
access/auth
access/identity
~~~

禁止依赖：

~~~text
runtime/chat
proxy/local
provider/llm
~~~

### 10.6 access/server/Cargo.toml

对应 RemoteLinkServer 的网络入口部分。

职责：

- RemoteLinkServer。
- HTTP/WS request acceptance。
- Session authentication。
- PeerLink channel opening。

创建输入：

~~~text
PairingService
AuthService
PeerLinkCarrier
NodeTransportHandle
ServerConfig
~~~

直接依赖：

~~~text
access/pairing
access/auth
access/peer-link
foundation/host-api
foundation/link
~~~

### 10.7 access/web/Cargo.toml

职责：

- Static Web server。
- Static Web pairing/control endpoints。
- Web shutdown handle。

创建输入：

~~~text
WebRoot
PairingControlHandle
WebAccessConfig
~~~

直接依赖：

~~~text
access/pairing
access/auth
foundation/host-api
~~~

禁止依赖：

~~~text
node/router
runtime/chat
proxy/local
~~~

## 11. Proxy

### 11.1 proxy/codegen/Cargo.toml

对应当前 operit-proxy-local 的 build scanner 和 generator。

职责：

- Rust source scanner。
- Rust schema generator。
- Rust dispatch generator。
- Dart model/proxy generator。

创建输入：

~~~text
Source roots
Schema configuration
Generated object metadata
~~~

直接依赖：

~~~text
syn
quote
proc-macro2
foundation/model source metadata
foundation/link protocol metadata
~~~

编译期允许扫描 server/runtime 源码，但运行时依赖禁止指向 node/router 或 access/server。

### 11.2 proxy/local/Cargo.toml

对应当前 LocalCoreProxy 和 CoreStreamPool。

职责：

- LocalCoreProxy。
- generated_core_dispatch 调用面。
- 本地 object registry。
- CoreStreamPool。

创建输入：

~~~text
Runtime local client contract
HostServices
Storage handle
Generated dispatch table
~~~

直接依赖：

~~~text
foundation/contracts
foundation/model
foundation/link
foundation/host-api
runtime/contracts
~~~

禁止依赖：

~~~text
node/router
node/space-runtime
access/server
access/peer-link
~~~

## 12. Command

### 12.1 command/core/Cargo.toml

对应当前 operit-command-core。

职责：

- chat、model、memory、tool、plugin、package、skill。
- MCP、storage、workspace、host、approval。
- market、people、prefs、stt、local_models、update、log、tag。

创建输入：

~~~text
CoreApplication command handle
LocalCoreClient
Access handle
Command output sink
~~~

直接依赖：

~~~text
application/core
foundation/model
foundation/host-api
~~~

Command crate 是 Application 的消费方，不是 CoreApplication 的子节点。它只拿 facade、local client 和 access handle，不直接创建 Runtime、Provider、Tool 或 Router。

## 13. Application

### 13.1 application/core/Cargo.toml

新建的唯一 composition root。

职责：

- CoreApplication。
- CoreApplicationConfig。
- CoreApplicationLifecycle。
- Context 创建。
- 子 crate 创建顺序。
- route runtime install/clear。
- service handle 统一暴露。

创建输入：

~~~text
HostManager
CoreApplicationConfig
RuntimeConfig
PersistenceConfig
ProviderConfig
ToolConfig
NodeConfig
AccessConfig
~~~

直接依赖：

~~~text
foundation/*
persistence/*
provider/*
tool/*
plugin/*
runtime/*
node/*
access/*
proxy/local
~~~

application/core 是唯一允许同时依赖 Runtime、Node、Access 和 Proxy 的 crate。command/core 反向依赖 application/core facade，不能被 application/core 依赖。

## 14. 目标构造顺序

~~~text
CoreApplication::start
  -> foundation/host-api
  -> foundation/contracts
  -> persistence/core
  -> persistence/preferences
  -> persistence/chat
  -> persistence/memory
  -> persistence/workspace
  -> persistence/node
  -> persistence/sync
  -> provider/*
  -> tool/*
  -> plugin/*
  -> runtime/*
  -> access/identity
  -> access/auth
  -> access/pairing
  -> access/peer-link
  -> node/local-runtime
  -> node/space-runtime
  -> node/router
  -> node/space-sync
  -> access/server
  -> access/web
  -> proxy/local
  -> install route runtime
  -> start background services
~~~

## 15. 依赖禁止事项

~~~text
proxy/local          -X-> node/router
proxy/local          -X-> access/server
runtime/chat         -X-> access/server
provider/llm         -X-> node/router
tool/builtin         -X-> node/router
persistence/*        -X-> runtime/*
foundation/*         -X-> runtime/*
access/peer-link     -X-> runtime/chat
node/router          -X-> Flutter
node/router          -X-> CLI
~~~

所有跨边界调用都必须经过 contract/context 或 application facade。

## 16.1 Route 注解的编译期依赖

带 operit_core_route 注解的业务 crate 可以在编译期依赖 node/route-macros：

~~~text
runtime/chat       -> node/route-macros 仅用于宏展开
runtime/workspace  -> node/route-macros 仅用于宏展开
provider/chat      -> node/route-macros 仅用于宏展开
tool/builtin       -> node/route-macros 仅用于宏展开
~~~

这不是运行时依赖。业务 crate 不因此直接依赖 node/router、access/peer-link 或 proxy/local。宏展开后的 wrapper 通过 foundation/link 的 CoreRouteRuntime contract 进入 NodeRuntime。

## 16. 迁移顺序

1. 先建立目标 workspace 目录和 Cargo.toml，不改变调用行为。
2. 抽出 foundation/contracts 和 runtime/contracts。
3. 把 proxy/codegen 从 proxy/local 分离。
4. 把 node/router、node/space-runtime、node/space-sync 从 core-server 分离。
5. 把 access/peer-link、access/server、access/pairing 从 link-access 分离。
6. 建立 application/core，统一创建和关闭所有服务。
7. 迁移 Flutter 和 CLI，使其只依赖 application/core。
8. 再拆 Provider、Tool、Plugin、Runtime、Persistence 的二级 crate。
9. 删除旧的宿主手工构造和旧 crate 直接依赖。

## 17. 完成标准

- 每个二级目录都有独立 Cargo.toml 和 src/lib.rs。
- 每个二级 crate 只有一个明确 owner。
- 每个长期实例由 application/core 创建。
- 每个构造函数的输入来自明确 Context 或 trait。
- proxy/local 不依赖 node 或 access。
- runtime/chat 不依赖 Flutter、CLI 或 Link server。
- node/router 不依赖 Proxy。
- access/server 不负责 route 决策。
- persistence 不负责实时流。
- Flutter、CLI 和第三方只依赖 application/core 暴露的 handle。

## 18. 现有 crate 到目标路径的迁移表

| 当前 crate | 目标二级 crate | 迁移方式 |
| --- | --- | --- |
| operit-host-api | foundation/host-api | 保留职责，移动 workspace 路径 |
| operit-model | foundation/model | 保留共享模型，去掉业务反向依赖 |
| operit-util | foundation/util | 保留通用工具和流原语 |
| operit-link | foundation/link | 保留协议类型，route runtime contract 收紧 |
| 无对应 crate | foundation/contracts | 新建跨域 contract |
| operit-store | persistence/core、preferences、chat、memory、workspace、node、sync | 按数据 ownership 拆分 |
| operit-providers | provider/contracts、llm、chat、media、memory、market、local-model | 按能力域拆分 |
| operit-tools | tool/contracts、runtime、builtin、skill、package、mcp、javascript | 按执行责任拆分 |
| operit-plugin-sdk | plugin/sdk | 保留公共插件契约 |
| operit-plugin-sdk-codegen | plugin/codegen | 保留代码生成职责 |
| operit-js-bridge | plugin/javascript-bridge | 保留脚本执行桥接 |
| operit-runtime | runtime/contracts、application、chat、preferences、workspace、host、transfer、plugin-host、events | 先抽 contract，再按生命周期拆分 |
| operit-route-macros | node/route-macros | 保留过程宏，改 workspace 路径 |
| operit-node-runtime | node/contracts、router、local-runtime、space-runtime、space-sync | 删除混合 server crate |
| operit-access-runtime | access/identity、auth、pairing、discovery、peer-link、server、web | 按控制面和载体拆分 |
| operit-proxy-local | proxy/codegen、proxy/local | 编译期 codegen 与运行时 Proxy 分离 |
| operit-command-core | command/core | 变成 application/core 的消费方 |
| 无对应 crate | application/core | 新建唯一 composition root |

## 19. Workspace 组织方式

目标根 workspace 的成员应直接指向每个二级 crate 的 Cargo.toml：

~~~toml
[workspace]
members = [
    "crates/foundation/host-api",
    "crates/foundation/model",
    "crates/foundation/util",
    "crates/foundation/link",
    "crates/foundation/contracts",
    "crates/persistence/core",
    "crates/persistence/preferences",
    "crates/persistence/chat",
    "crates/persistence/memory",
    "crates/persistence/workspace",
    "crates/persistence/node",
    "crates/persistence/sync",
    "crates/provider/contracts",
    "crates/provider/llm",
    "crates/provider/chat",
    "crates/provider/media",
    "crates/provider/memory",
    "crates/provider/market",
    "crates/provider/local-model",
    "crates/tool/contracts",
    "crates/tool/runtime",
    "crates/tool/builtin",
    "crates/tool/skill",
    "crates/tool/package",
    "crates/tool/mcp",
    "crates/tool/javascript",
    "crates/plugin/sdk",
    "crates/plugin/codegen",
    "crates/plugin/runtime",
    "crates/plugin/javascript-bridge",
    "crates/runtime/contracts",
    "crates/runtime/application",
    "crates/runtime/chat",
    "crates/runtime/preferences",
    "crates/runtime/workspace",
    "crates/runtime/host",
    "crates/runtime/transfer",
    "crates/runtime/plugin-host",
    "crates/runtime/events",
    "crates/node/route-macros",
    "crates/node/contracts",
    "crates/node/router",
    "crates/node/local-runtime",
    "crates/node/space-runtime",
    "crates/node/space-sync",
    "crates/access/identity",
    "crates/access/auth",
    "crates/access/pairing",
    "crates/access/discovery",
    "crates/access/peer-link",
    "crates/access/server",
    "crates/access/web",
    "crates/proxy/codegen",
    "crates/proxy/local",
    "crates/command/core",
    "crates/application/core",
]
~~~

这里的一级目录只是物理归类；Cargo workspace 的真正成员是二级目录。任何二级 crate 都必须拥有自己的 Cargo.toml、src/lib.rs、依赖声明和测试边界。
