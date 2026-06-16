# Operit2 Doc Assembly

本文档是 Operit2 当前工程的总装说明。它描述项目实际目录、核心 crate、App 导航、Flutter 与 Rust 通信、Host 能力、工具调用、插件装配、工作区、发布产物和 Kotlin 到 Rust 的复刻规则。

## 1. Assembly Map

Operit2 当前由五层组成：

```text
UI Layer
  apps/flutter/app
  apps/cli

Bridge Layer
  apps/flutter/app/lib/core/bridge
  apps/flutter/app/lib/core/link
  apps/flutter/native/operit-flutter-bridge
  core/crates/operit-link
  core/crates/operit-core-proxy

Runtime Layer
  core/crates/operit-runtime
  core/crates/operit-command-core
  core/crates/operit-store

Host Layer
  core/crates/operit-host-api
  hosts/android
  hosts/linux
  hosts/web
  hosts/windows

Extension Layer
  plugins/buildin
  plugins/types
  plugins/examples
  plugins/tools
```

运行方向：

```text
Flutter UI / CLI
  -> CoreProxy
  -> operit-link request
  -> LocalCoreProxy / app remote link client
  -> OperitApplication / ChatRuntimeHolder
  -> EnhancedAIService
  -> ToolExecutionManager / AIToolHandler
  -> Host trait / ToolPkg / MCP / Memory / Storage
```

Link / Access / Remote 边界见：

```text
docs/link-access-architecture.md
```

平台能力方向：

```text
operit-runtime
  -> operit-host-api trait
  -> hosts/{platform}
  -> OS / browser / Android runtime / local storage
```

插件方向：

```text
plugins/buildin TypeScript
  -> runtime asset build
  -> BuiltinPluginAssets
  -> PackageManager
  -> ToolPkg bridge
  -> AIToolHandler
```

## 2. Root Structure

```text
apps/
  cli/
    src/main.rs
    src/bootstrap.rs
    src/chat_runtime.rs
    src/core_proxy.rs
    src/cli/
    src/tui/

  flutter/
    app/
      lib/core/
      lib/ui/
      android/
      linux/
      windows/
      web/
    native/operit-flutter-bridge/
    thirdparty/xterm/

  server/

core/
  Cargo.toml
  crates/
    operit-command-core/
    operit-core-proxy/
    operit-host-api/
    operit-link/
    operit-runtime/
    operit-store/

hosts/
  android/
  ios/
  linux/
  server/
  web/
  windows/

plugins/
  buildin/
  docs/
  examples/
  tools/
  types/

tools/
  android-runtime/
  release/
```

关键约束：

```text
core/Cargo.toml 是 Rust core workspace
apps/cli/Cargo.toml 是 CLI 产品入口
apps/flutter/app/pubspec.yaml 是 Flutter App 产品入口
hosts/* 是平台能力实现
plugins/* 是 ToolPkg 开发与打包区域
tools/release/RELEASE_SPEC.md 是发布产物规范
```

## 3. Rust Core Workspace

入口：

```text
core/Cargo.toml
```

成员：

```text
operit-host-api
operit-command-core
operit-core-proxy
operit-link
operit-store
operit-runtime
```

共享依赖：

```text
async-trait
futures-util
reqwest
serde
serde_json
sha2
base64
regex
json5
rquickjs
rusqlite
thiserror
tokio
uuid
chrono
zip
```

### 3.1 operit-runtime

路径：

```text
core/crates/operit-runtime
```

入口：

```text
src/lib.rs
```

导出：

```text
pub mod R
pub mod api
pub mod core
pub mod data
pub mod plugins
pub mod services
pub mod ui
pub mod util

pub use api::chat::EnhancedAIService::EnhancedAIService
pub use core::chat::AIMessageManager::AIMessageManager
```

模块说明：

```text
api/
  chat/
    ChatRuntimeHolder.rs       主聊天 runtime 持有者
    ChatRuntimeSlot.rs         runtime slot
    EnhancedAIService.rs       AI 对话与工具循环核心
    AIForegroundService.rs     前台服务镜像结构
    enhance/                   对话组装、工具执行、引用、文件绑定
    library/                   memory library 与自动保存
    llmprovider/               OpenAI、Gemini、Claude、Ollama 等 provider

core/
  application/                 OperitApplication 与 OperitApplicationContext
  chat/                        AIMessageManager、hooks、plugins
  config/                      system prompt、tool prompt、functional prompt
  tools/                       AIToolHandler、ToolRegistration、ToolPermissionSystem
  avatar/                      avatar model/controller/renderer factory

data/
  model/                       ChatMessage、Memory、AttachmentInfo、ModelConfigData 等
  dao/                         ChatDao、MessageDao、MessageVariantDao
  db/                          AppDatabase
  repository/                  ChatHistoryManager、WorkspaceService、MemoryRepository 等
  preferences/                 ApiPreferences、ModelConfigManager、EnvPreferences 等
  sync/                        SqlChatSyncStore
  backup/                      RawSnapshotBackupManager
  mcp/                         MCPRepository、MCPLocalServer、plugins
  skill/                       SkillRepository

plugins/
  BuiltinPluginAssets.rs       构建期内置插件资产
  PluginRegistry.rs            插件注册表
  toolpkg/                     ToolPkg bridge 与 hook bridge
  toolbox/                     toolbox plugin
  workflow/                    workflow lifecycle plugin

services/
  ChatServiceCore.rs
  core/*Delegate.rs

ui/
  features/chat/webview/workspace/

util/
  stream/
  streamnative/
  AppLogger.rs
  ChatMarkupRegex.rs
  ChatUtils.rs
  DocumentConversionUtil.rs
  FileUtils.rs
  PathMapper.rs
  StructuredAssistantContentParser.rs
```

### 3.2 operit-host-api

路径：

```text
core/crates/operit-host-api
```

核心文件：

```text
src/lib.rs
src/TimeUtils.rs
```

职责：

```text
定义 HostError / HostResult
定义 HostEnvironmentDescriptor
定义文件、HTTP、浏览器访问、runtime process、storage、sqlite、system、terminal 相关 trait 与数据结构
让 runtime 不直接依赖平台代码
```

环境描述：

```text
HostEnvironmentDescriptor::android()
HostEnvironmentDescriptor::windows()
HostEnvironmentDescriptor::linux()
HostEnvironmentDescriptor::web()
```

能力标签：

```text
fs.read
fs.write
fs.search
fs.archive
web.visit
runtime.process
runtime.storage
runtime.sqlite
os.open
os.share
system.location
system.notifications.read
system.app_usage
system.app.install
system.app.uninstall
system.settings
```

### 3.3 operit-link

路径：

```text
core/crates/operit-link
```

主要文件：

```text
src/protocol.rs
src/client.rs
src/http.rs
src/lib.rs
```

职责：

```text
CoreObjectPath
CoreCallRequest
CoreCallResponse
CoreWatchRequest
CoreEvent
CoreEventKind
CoreLinkClient
CoreLinkError
```

职责边界：

```text
operit-link 只描述 core call/watch/event/error/stream 的穿透语义
Proxy 只表示 core 能力在 app 侧的代理投影
Remote 只表示跨 app 使用 link 的连接场景
配对、session、签名、设备信任和 server 生命周期属于 app access
```

通信语义：

```text
call          一次性方法调用
watchSnapshot 单次属性快照
watchStream   持续事件流
```

### 3.4 operit-core-proxy

路径：

```text
core/crates/operit-core-proxy
```

构建脚本：

```text
build.rs
build_rust_codegen.rs
build_dart_codegen.rs
```

职责：

```text
读取 core 对象与方法定义
生成 Dart proxy models/clients
生成 Rust LocalCoreProxy 分发表达
把 Flutter/CLI 的 typed client 调用转换成 operit-link request
```

生成链路：

```text
Rust runtime object/method
  -> build script
  -> CoreProxyModels.g.dart
  -> CoreProxyClients.g.dart
  -> Dart ViewModel 调用
```

### 3.5 operit-command-core

路径：

```text
core/crates/operit-command-core
```

命令模块：

```text
commands/approval.rs
commands/chat.rs
commands/host.rs
commands/market.rs
commands/mcp.rs
commands/memory.rs
commands/model.rs
commands/package.rs
commands/people.rs
commands/plugin.rs
commands/prefs.rs
commands/skill.rs
commands/tag.rs
commands/tool.rs
commands/update.rs
commands/workspace.rs
```

CLI 通过该 crate 复用 command 行为。

### 3.6 operit-store

路径：

```text
core/crates/operit-store
```

主要文件：

```text
RuntimeStorePaths.rs
RuntimeStorageHost.rs
PreferencesDataStore.rs
SqliteStore.rs
ObjectBoxStore.rs
SyncOperationStore.rs
```

职责：

```text
runtime 存储路径
偏好状态流
SQLite store
ObjectBox 风格 store
同步操作记录
```

## 4. Platform Hosts

Host 实现目录：

```text
hosts/windows
hosts/linux
hosts/web
hosts/android
hosts/ios
hosts/server
```

Windows/Linux/Web host 采用相似布局：

```text
src/lib.rs
src/bridge/mod.rs
src/registry/mod.rs
src/tools/mod.rs
src/tools/browser/mod.rs
src/tools/fs/mod.rs
src/tools/http/mod.rs
src/tools/runtime/mod.rs
src/tools/storage/mod.rs
src/tools/system/mod.rs
src/tools/terminal/mod.rs
```

### 4.1 Native Host Assembly

Native host crate 的职责：

```text
实现 operit-host-api trait
把路径、进程、HTTP、浏览器访问、SQLite、文件系统映射到本机能力
向 operit-runtime 提供统一能力对象
```

Flutter native bridge 通过条件编译选择 host：

```text
target_os = "android" -> operit_host_android_native
target_os = "linux"   -> operit_host_linux_native
windows               -> operit_host_windows_native
target_arch = "wasm32" -> operit_host_web
```

### 4.2 Web Host Assembly

路径：

```text
hosts/web
apps/flutter/app/web/operit_runtime_bridge.js
```

Web host 由浏览器 JS 安装：

```text
globalThis.__operitHost
```

Bridge 能力对象：

```text
fileSystem
webVisit
managedRuntime
managedRuntimeProcess
runtimeStorage
sqlite
systemOperation
```

Web storage 设计：

```text
operit2.runtime.*  runtime storage
operit2.files.*    文件内容
operit2.sqlite.*   sqlite database export
```

SQLite JS bridge：

```text
sqliteConnections
sqliteTransactions
sql-wasm.js
initSqlJs()
Uint8Array blob
integer string preservation
```

## 5. Flutter App Assembly

路径：

```text
apps/flutter/app
```

主要区域：

```text
lib/core/
  bridge/
  browser/
  host/
  link/
  proxy/generated/

lib/ui/
  common/
  features/
  i18n/
  main/
  permissions/
  theme/

android/
linux/
windows/
web/
```

### 5.1 UI Feature Structure

```text
ui/features/chat/
  screens/AIChatScreen.dart
  viewmodel/ChatViewModel.dart
  components/ChatScreenContent.dart
  components/WorkspaceShell.dart
  components/workspace/
  components/workspace/browser/
  components/workspace/file_preview/
  components/workspace/html_preview/
  components/part/
  components/share/
  components/style/

ui/features/packages/
  screens/PackageManagerScreen.dart
  screens/UnifiedMarketScreen.dart
  screens/ArtifactPublishScreen.dart
  components/
  dialogs/
  model/
  utils/

ui/features/settings/
  screens/SettingsScreen.dart
  components/
  models/

ui/main/
  screens/
  navigation/
  layout/
  components/
  TopBarController.dart
```

### 5.2 Navigation Model

导航核心文件：

```text
ui/main/screens/OperitMainScreen.dart
ui/main/screens/OperitScreens.dart
ui/main/screens/ScreenRouteRegistry.dart
ui/main/navigation/AppNavigationModels.dart
ui/main/navigation/AppRouteCatalog.dart
ui/main/components/AppContent.dart
ui/main/layout/PhoneLayout.dart
ui/main/layout/TabletLayout.dart
```

类型层次：

```text
RouteSpec
  routeId
  runtime
  title
  keepAlive
  reuseOnTop

RouteEntry
  instanceId
  routeId
  args
  source

NavigationEntrySpec
  entryId
  routeId
  surface
  title
  icon
  order

OperitScreen
  routeTypeName
  title
  participatesInCrossfadeTransition
  keepAlive
  routeArgs()
  stableScreenKey()
  build()
```

Route runtime：

```text
RouteRuntime.native
```

Navigation surface：

```text
NavigationSurface.mainSidebarAi
```

Route source：

```text
RouteEntrySource.defaultSource
RouteEntrySource.drawer
RouteEntrySource.script
```

Transition source：

```text
NavigationTransitionSource.defaultSource
NavigationTransitionSource.drawer
```

### 5.3 Registered Screens

`ScreenRouteRegistry` 注册四个主屏幕：

```text
AiChatScreenRoute
  routeTypeName: AiChat
  routeId: native.ai_chat
  sidebar entry: main.ai_chat
  title: Operit
  icon: Icons.chat_bubble_outline
  order: 10

PackageManagerScreenRoute
  routeTypeName: PackageManager
  routeId: native.package_manager
  sidebar entry: main.package_manager
  title: 包管理
  icon: Icons.extension_outlined
  order: 20
  keepAlive: true

MarketScreenRoute
  routeTypeName: Market
  routeId: native.market
  sidebar entry: main.market
  title: 市场
  icon: Icons.store_outlined
  order: 30
  keepAlive: true
  args: initialTab

SettingsScreenRoute
  routeTypeName: Settings
  routeId: native.settings
  sidebar entry: main.settings
  title: 设置
  icon: Icons.settings_outlined
  order: 40
  keepAlive: true
```

Route id 生成：

```text
native.${camelToSnakeCase(routeTypeName)}
```

示例：

```text
AiChat -> native.ai_chat
PackageManager -> native.package_manager
```

### 5.4 Main Screen State

`OperitMainScreen` 持有：

```text
AppNavigationModel _navigationModel
AppRouterState _routerState
TopBarController _topBarController
bool _drawerOpen
bool _isTabletSidebarExpanded
bool _isNavigatingBack
NavigationTransitionSource _navigationTransitionSource
```

初始化：

```text
TopBarController()
AppRouterState(AppRouteCatalog.initialEntry())
AppRouterGateway.install(handler: _navigateToRoute, reset: _resetToRoute)
```

依赖变化：

```text
AppRouteCatalog.build(context)
AppRouteDiscoveryGateway.install(() => _navigationModel.routes)
```

导航入口：

```text
_navigateToRoute(routeId, args, source)
_resetToRoute(routeId, args, source)
_navigateToNavigationEntry(entry)
_activateConversationRoute()
```

返回键行为：

```text
drawer open        -> close drawer
router canPop      -> pop route stack
not AiChat screen  -> reset to main.ai_chat
AiChat root        -> double press exit
```

### 5.5 AppContent Screen Rendering

`AppContent` 负责：

```text
顶部栏
页面缓存
keepAlive screen 保留
跨屏 transition
drawer transition
screen builder 调用
```

缓存 key：

```text
currentScreen.stableScreenKey() 或 RouteEntry.instanceId
```

保留条件：

```text
OperitScreen.keepAlive
```

顶部栏图标：

```text
canGoBack                         -> Icons.arrow_back
tablet sidebar expanded           -> Icons.chevron_left
normal navigation button           -> Icons.segment
```

### 5.6 Phone Layout

`PhoneLayout` 采用抽屉式导航。

关键参数：

```text
drawerWidth
drawerOpen
enableNavigationAnimation
navigationEntries
selectedRouteId
```

动画状态：

```text
AnimationController.unbounded
SpringSimulation
drawerProgress 0.0..1.0
```

内容变换：

```text
translationX = drawerWidth * 0.82 * progress
translationY = 12 * progress
scale = 1.0 - 0.08 * progress
rotationY = -7 degree * progress
cornerRadius = 24 * progress
shadowElevation = 18 * progress
```

手势：

```text
horizontal drag right  -> open drawer
horizontal drag left   -> close drawer
tap outside drawer     -> close drawer
```

### 5.7 Tablet Layout

`TabletLayout` 采用侧栏式导航。

状态：

```text
isTabletSidebarExpanded
_isSidebarWidthExpanded
_isSidebarContentExpanded
```

布局：

```text
Row
  AnimatedContainer sidebar
    AnimatedSwitcher
      DrawerContent
      CollapsedDrawerContent
  Expanded content
```

动画：

```text
sidebar width duration: 280ms
sidebar content fade: 160ms
```

## 6. Chat Screen Assembly

入口：

```text
apps/flutter/app/lib/ui/features/chat/screens/AIChatScreen.dart
```

ViewModel：

```text
apps/flutter/app/lib/ui/features/chat/viewmodel/ChatViewModel.dart
```

AIChatScreen 状态：

```text
TextEditingController _messageController
FocusNode _inputFocusNode
ScrollController _scrollController
List<ChatUiMessage> _messages
ValueNotifier<_ChatContentData> _chatContentDataNotifier
ValueNotifier<bool> _autoScrollToBottomNotifier
ValueNotifier<String> _modelLabelNotifier
ValueNotifier<String?> _toastMessageNotifier
StreamSubscription<ChatViewModelSnapshot> _mainStateSubscription
StreamSubscription<String?> _toastEventSubscription
TopBarController _topBarController
```

聊天快照数据：

```text
messages
loading
errorMessage
inputProcessingState
currentChatId
hasOlderDisplayHistory
hasNewerDisplayHistory
isLoadingDisplayWindow
isMultiSelectMode
selectedMessageIndices
```

ChatViewModel 连接：

```text
ProxyCoreRuntimeBridge
GeneratedCoreProxyClients
GeneratedChatRuntimeHolderMainCoreProxy
```

状态监听：

```text
chatHistoryFlowChanges
currentChatIdFlowChanges
chatHistoriesFlowChanges
activeStreamingChatIdsFlowChanges
inputProcessingStateByChatIdFlowChanges
responseStreamCompletedRefresh
```

发送消息：

```text
ChatViewModel.sendUserMessage(text)
  -> _chat.updateUserMessage(message: text)
  -> preferencesFunctionalConfigManager.getConfigMappingForFunction(CHAT)
  -> _chat.sendUserMessage(...)
```

消息操作：

```text
cancelCurrentMessage
setMessageFavorite
deleteMessage
deleteMessages
updateMessage
deleteMessagesFrom
deleteMessageVariant
rollbackToMessage
rewindAndResendMessage
regenerateSingleAiMessage
createBranch
insertSummary
loadOlderMessagesForCurrentChat
loadNewerMessagesForCurrentChat
showLatestMessagesForCurrentChat
```

Workspace 操作：

```text
createAndBindDefaultWorkspace
bindChatToWorkspace
listWorkspaceFiles
readWorkspaceTextFile
readWorkspaceFileBytes
writeWorkspaceFileBytes
openWorkspaceFile
previewWorkspaceChangesForMessage
```

## 7. Workspace Assembly

Flutter UI：

```text
WorkspaceShell.dart
workspace/WorkspacePanel.dart
workspace/WorkspaceTabStrip.dart
workspace/WorkspaceTabContent.dart
workspace/WorkspaceFileBrowserContent.dart
workspace/WorkspaceFilePreviewContent.dart
workspace/WorkspaceTerminalContent.dart
workspace/WorkspaceSetupContent.dart
workspace/browser/WorkspaceBrowserContent.dart
workspace/file_preview/*
workspace/html_preview/*
```

Runtime side：

```text
core/crates/operit-runtime/src/data/repository/WorkspaceService.rs
core/crates/operit-runtime/src/ui/features/chat/webview/workspace/
core/crates/operit-runtime/assets/workspace_templates/
```

WorkspaceShell 输入：

```text
workspaceOpen
currentChatId
hasBoundWorkspace
workspacePath
onListWorkspaceFiles
onReadWorkspaceTextFile
onReadWorkspaceFileBytes
onWriteWorkspaceFileBytes
onOpenWorkspaceFile
onCreateDefaultWorkspace
onBindWorkspace
```

WorkspaceShell 布局：

```text
width >= workspaceTabletBreakpoint
  -> chat + resizable side workspace

width < workspaceTabletBreakpoint
  -> full overlay workspace panel
```

平板 workspace 约束：

```text
workspaceMinTabletChatWidth
workspaceMinWidth
workspaceDefaultTabletWidth
workspaceResizeHandleHitWidth
```

模板：

```text
go/
java/
office/
python/
typescript/
web/
```

模板资产生成：

```text
core/crates/operit-runtime/build.rs
  -> workspace_template_assets.rs
```

## 8. Flutter Bridge Assembly

Dart bridge：

```text
lib/core/bridge/OperitRuntimeBridge.dart
lib/core/bridge/ProxyCoreRuntimeBridge.dart
lib/core/bridge/CoreProxy.dart
lib/core/bridge/PlatformCoreProxy.dart
lib/core/link/CoreLinkProtocol.dart
lib/core/link/RemoteRuntimeLinkClient.dart
lib/core/proxy/generated/CoreProxyClients.g.dart
lib/core/proxy/generated/CoreProxyModels.g.dart
```

Bridge 抽象：

```text
OperitRuntimeBridge.call(CoreCallRequest)
OperitRuntimeBridge.watchSnapshot(CoreWatchRequest)
OperitRuntimeBridge.watchStream(CoreWatchRequest)
OperitRuntimeBridge.hostDescriptor()
```

便利方法：

```text
callApplication(methodName, args)
watch(targetPath, propertyName, args)
watchChanges(targetPath, propertyName, args)
```

Remote runtime link client endpoints：

```text
POST /link/call
POST /link/watch/snapshot
POST /link/watch/channel/events
POST /link/watch/channel/open
POST /link/watch/channel/close
POST /link/session
```

Remote signed headers：

```text
content-type: application/json
x-operit-session
x-operit-device
x-operit-signature
```

签名：

```text
Hmac(sha256, base64Decode(sessionSecret), utf8(body))
```

签名、session 和配对属于 Flutter app access；`operit-link` 不持有这些状态。

Native bridge：

```text
apps/flutter/native/operit-flutter-bridge/src/lib.rs
apps/flutter/native/operit-flutter-bridge/src/access.rs
```

Native bridge 持有：

```text
tokio::runtime::Runtime
Mutex<LocalCoreProxy>
Mutex<HashMap<String, CoreEventStream>>
FlutterApprovalBridge
FlutterBrowserAutomationBridge
NativeTerminalHost
```

Native app access 持有：

```text
pairing endpoints
accepted session store
request signature verification
Web Access host server
operit-link HTTP dispatcher wiring
```

权限桥：

```text
PermissionRequestResult
PendingApproval
PERMISSION_REQUEST_TIMEOUT_MS = 60000
```

浏览器自动化桥：

```text
PendingBrowserAutomationRequest
BrowserAutomationToolResponse
BROWSER_AUTOMATION_REQUEST_TIMEOUT_MS = 180000
```

## 9. EnhancedAIService Assembly

路径：

```text
core/crates/operit-runtime/src/api/chat/EnhancedAIService.rs
```

核心依赖：

```text
MultiServiceManager
ConversationService
AIToolHandler
ToolExecutionManager
MemoryLibrary
PromptHookRegistry
SystemPromptConfig
SystemToolPrompts
CliToolModeSupport
CharacterCardManager
SkillRepository
```

共享状态：

```text
is_service_manager_initialized
per_request_token_counts
request_window_estimate
active_execution_contexts
next_execution_context_id
tool_execution_jobs
accumulated_input_token_count
accumulated_output_token_count
accumulated_cached_input_token_count
current_request_input_token_count
current_request_output_token_count
current_request_cached_input_token_count
current_response_callback_registered
current_complete_callback_registered
last_reply_content
last_provider_model
last_turn_token_snapshot
```

发送选项：

```text
SendMessageOptions
  message
  maxTokens
  tokenUsageThreshold
  chatId
  chatHistory
  workspacePath
  workspaceEnv
  functionType
  promptFunctionType
  enableThinking
  enableMemoryAutoUpdate
  customSystemPromptTemplate
  isSubTask
  characterName
  avatarUri
  roleCardId
  enableGroupOrchestrationHint
  groupParticipantNamesText
  proxySenderName
  notifyReplyOverride
  chatModelConfigIdOverride
  chatModelIndexOverride
  preferenceProfileIdOverride
  stream
  disableWarning
```

生命周期枚举：

```text
EnsureInitialized
StartAiService
SetProcessingState
PrepareConversationHistory
SyncPreparedHistoryToExecutionContext
SetConnectingState
GetModelParametersForFunction
GetAIServiceForFunction
ClearPerRequestTokenCounts
GetAvailableToolsForFunction
BeforeFinalizePromptHook
BeforeSendToModelHook
StripGeminiThoughtSignatureMeta
ApplyFinalizedCurrentUserTurn
SyncRequestHistoryToExecutionContext
EstimatePreparedRequestWindow
SendMessageRequest
StartAssistantResponseRound
CollectResponseStream
ExtractToolInvocations
ExecuteToolInvocations
ProcessToolResults
PersistTokenUsage
ProcessStreamCompletion
UnregisterExecutionContext
StopAiService
```

对话主链路：

```text
ChatRuntimeHolder.sendUserMessage
  -> EnhancedAIService.sendMessage
  -> PrepareConversationHistory
  -> PromptHookRegistry
  -> SystemPromptComposer
  -> MultiServiceManager.getAIServiceForFunction
  -> AIService.sendMessage
  -> collect response stream
  -> ToolExecutionManager.extractToolInvocations
  -> ToolExecutionManager.executeInvocations
  -> ConversationMarkupManager.formatToolResultForMessage
  -> continue conversation round
  -> persist message / token usage / stream completion
```

## 10. Tool Assembly

工具注册入口：

```text
core/crates/operit-runtime/src/core/tools/ToolRegistration.rs
core/crates/operit-runtime/src/core/tools/defaultTool/ToolGetter.rs
```

ToolGetter：

```text
getFileSystemTools(context)
getHttpTools(context)
getWebVisitTool(context)
getBrowserAutomationTools(context)
getSystemOperationTools(context)
getTerminalTools(context)
```

标准工具：

```text
StandardFileSystemTools
StandardHttpTools
StandardWebVisitTool
StandardBrowserAutomationTools
StandardSystemOperationTools
StandardTerminalTools
StandardMemoryTools
StandardChatManagerTool
```

公开工具注册：

```text
sleep
filesystem tools
visit_web
system operation tools
memory public tools
chat tools
use_package
cli search tool
cli proxy tool
```

内部工具注册：

```text
MCP tools
package tools
host-backed tools
runtime storage tools
terminal tools
browser automation tools
```

工具调用 XML：

```text
<tool name="...">
  <param name="...">...</param>
</tool>
```

工具结果 XML：

```text
<tool_result name="..." status="...">
  <content>...</content>
</tool_result>
```

解析：

```text
ChatMarkupRegex.tool_call_matches
ToolExecutionManager.extractToolInvocations
tag_ranges(body, "param")
attr_value(raw, "name")
```

执行：

```text
validateParameters
checkToolPermission
invokeAndStream
notifyToolExecutionResult
formatToolResultForMessage
```

权限参与者：

```text
ToolPermissionSystem
AIToolHandler
CharacterCardToolAccessResolver
FlutterApprovalBridge
```

CLI exposure：

```text
ToolExposureMode::FULL
ToolExposureMode::CLI
CliToolModeSupport::SEARCH_TOOL_NAME
CliToolModeSupport::PROXY_TOOL_NAME
```

包调用上下文：

```text
__operit_package_caller_name
__operit_package_chat_id
__operit_package_caller_card_id
```

## 11. Plugin And ToolPkg Assembly

插件目录：

```text
plugins/buildin
plugins/examples
plugins/types
plugins/docs
plugins/tools
```

runtime 模块：

```text
core/crates/operit-runtime/src/plugins
core/crates/operit-runtime/src/core/tools/packTool
core/crates/operit-runtime/src/plugins/toolpkg
core/crates/operit-runtime/src/core/tools/javascript
```

ToolPkg 解析：

```text
ToolPkgParser.rs
ToolPkgLoader.rs
ToolPkgManager.rs
ToolPkgMainRegistrationScriptParser.rs
ToolPkgComposeDslParser.rs
ToolPkgTemplateModels.rs
ToolPkgHookModels.rs
```

ToolPkg bridge：

```text
ToolPkgCommonBridgePlugin
ToolPkgHookBridgeSupport
ToolPkgPromptHookBridge
ToolPkgSummaryHookBridge
ToolPkgMessageProcessingBridge
ToolPkgToolLifecycleBridge
ToolPkgChatInputHookBridge
ToolPkgChatViewHookBridge
ToolPkgInputMenuToggleBridge
ToolPkgAiProviderRegistry
```

JS runtime：

```text
JsEngine
JsToolManager
JsTools
JsToolPkgRegistration
JsNativeInterfaceDelegates
JsInitRuntimeScriptBuilder
JsExecutionScriptBuilder
JsExecutionResultProtocol
```

类型声明：

```text
plugins/types/core.d.ts
plugins/types/toolpkg.d.ts
plugins/types/tool-types.d.ts
plugins/types/results.d.ts
plugins/types/files.d.ts
plugins/types/network.d.ts
plugins/types/memory.d.ts
plugins/types/chat.d.ts
plugins/types/ui.d.ts
plugins/types/compose-dsl.d.ts
plugins/types/compose-dsl.material3.generated.d.ts
plugins/types/android.d.ts
```

内置插件构建：

```text
plugins/buildin/*
  -> core/crates/operit-runtime/build.rs
  -> builtin_plugin_assets.rs
  -> BuiltinPluginAssets
  -> PackageManager
```

## 12. Package Manager And Market UI

Flutter UI：

```text
ui/features/packages/screens/PackageManagerScreen.dart
ui/features/packages/screens/UnifiedMarketScreen.dart
ui/features/packages/screens/PluginTabContent.dart
ui/features/packages/screens/PackageTabContent.dart
ui/features/packages/screens/MCPConfigScreen.dart
ui/features/packages/screens/SkillConfigScreen.dart
ui/features/packages/screens/ArtifactPublishScreen.dart
```

组件：

```text
PackageGrid
PackageListItem
PackageTab
MarketEntryCard
EmptyState
```

Dialogs：

```text
PackageDetailsDialog
PackageToolRunDialog
MCPDetailsDialog
MCPImportDialog
MCPToolRunDialog
SkillImportDialog
```

runtime side：

```text
PackageManager.rs
PackageManagerToolPkgFacade.rs
PackageDebugRefreshReceiver.rs
PackageDebugInstallReceiver.rs
PackageToolExecutor
MCPManager
SkillManager
SkillRepository
```

## 13. CLI/TUI Assembly

路径：

```text
apps/cli
```

二进制：

```text
operit2
```

依赖：

```text
operit-core-proxy
operit-command-core
operit-link
operit-runtime
operit-store
operit-host-windows-native
operit-host-linux-native
```

入口：

```text
src/main.rs
src/bootstrap.rs
src/core_proxy.rs
src/chat_runtime.rs
```

CLI command：

```text
src/cli/link.rs
src/cli/transfer.rs
src/cli/mod.rs
src/access.rs
```

TUI：

```text
src/tui/app.rs
src/tui/approval.rs
src/tui/commands.rs
src/tui/empty_state.rs
src/tui/helpers.rs
src/tui/input.rs
src/tui/link_proxy_rs.rs
src/tui/markdown.rs
src/tui/render.rs
src/tui/theme.rs
src/tui/typewriter.rs
```

CLI 调用链：

```text
main
  -> bootstrap
  -> host init
  -> LocalCoreProxy
  -> operit-command-core
  -> runtime / tool / workspace / package / update command
```

CLI app access：

```text
link serve/connect/sync/watch 由 CLI app access 管理配对、session、签名和 accepted sessions
operit-link 只承载 core call/watch/event 穿透协议
CLI app access session storage: client/access/link_sessions.json
CLI app accepted session storage: client/access/link_server_sessions.json
```

## 14. Data Assembly

数据模型目录：

```text
core/crates/operit-runtime/src/data/model
```

主要模型：

```text
ActivePrompt
AITool
AiReference
ApiKeyInfo
AttachmentInfo
BillingMode
CharacterCard
CharacterGroupCard
ChatHistory
ChatMessage
ChatMessageDisplayMode
ChatMessageLocatorPreview
ChatMessageTimestampAllocator
ChatTurnOptions
CloudEmbeddingConfig
CustomEmoji
DocumentChunk
DragonBones
Embedding
EmbeddingConverter
EmbeddingDimensionUsage
FunctionType
InputProcessingState
Memory
MemoryAutoSaveCandidate
MemoryExportModel
MemorySearchConfig
MemorySearchDebugInfo
MessageEntity
MessageVariantEntity
ModelConfigData
ModelParameter
OpenAIModels
OperitChatArchive
OperitNodeInfo
PreferenceProfile
PromptFunctionType
PromptTag
SerializableColorScheme
SerializableTypography
StandardModelParameters
ToolPrompt
Workflow
WorkflowExecutionLog
WorkspaceRenameResult
```

Repository：

```text
AvatarRepository
ChatHistoryManager
CustomEmojiRepository
MemoryAutoSaveCandidateRepository
MemoryRepository
RuntimeStorageRepository
UIHierarchyManager
WorkflowRepository
WorkspaceService
```

Preferences：

```text
ActivePromptManager
AgreementPreferences
AndroidPermissionPreferences
ApiPreferences
CharacterCardManager
CharacterCardToolAccessResolver
CharacterGroupCardManager
CustomEmojiPreferences
DisplayPreferencesManager
EnvPreferences
ExternalHttpApiPreferences
FreeUsagePreferences
FunctionalConfigManager
GitHubAuthBus
GitHubAuthPreferences
MemorySearchSettingsPreferences
ModelConfigManager
PersonaCardChatHistoryManager
PromptTagManager
PromptVersionManager
RemoteAnnouncementPreferences
SkillVisibilityPreferences
SpeechServicesPreferences
ThemePreferenceSnapshot
ToolCollapseMode
UserPreferencesManager
WaifuPreferences
WakeWordPreferences
```

## 15. Release Assembly

规范：

```text
tools/release/RELEASE_SPEC.md
```

脚本：

```text
tools/release/release.py
```

Tag：

```text
v{major}.{minor}.{patch}+{build}
```

Asset：

```text
operit2-{product}-{platform}-{arch}.{ext}
```

Product：

```text
app
cli
```

Platform：

```text
windows
linux
macos
android
```

Desktop arch：

```text
x86_64
aarch64
```

Android ABI：

```text
arm64-v8a
armeabi-v7a
x86_64
```

App assets：

```text
operit2-app-windows-x86_64.zip
operit2-app-windows-aarch64.zip
operit2-app-linux-x86_64.tar.gz
operit2-app-linux-aarch64.tar.gz
operit2-app-macos-x86_64.tar.gz
operit2-app-macos-aarch64.tar.gz
operit2-app-android-arm64-v8a.apk
operit2-app-android-armeabi-v7a.apk
operit2-app-android-x86_64.apk
```

CLI assets：

```text
operit2-cli-windows-x86_64.zip
operit2-cli-windows-aarch64.zip
operit2-cli-linux-x86_64.tar.gz
operit2-cli-linux-aarch64.tar.gz
operit2-cli-macos-x86_64.tar.gz
operit2-cli-macos-aarch64.tar.gz
```

下载要求：

```text
Content-Length > 0
HTTP Range request returns 206
6-thread Range download
```

Android ABI 名称包含 `-`，asset 解析不能按普通横线切段理解 ABI。

## 16. Android Runtime Assembly

资源目录：

```text
apps/flutter/app/android/app/src/main/assets/android-runtime/
  arm64-v8a/rootfs.tar.gz.bin
  arm64-v8a/rootfs.tar.gz.bin.sha256
  armeabi-v7a/rootfs.tar.gz.bin
  armeabi-v7a/rootfs.tar.gz.bin.sha256
  x86_64/rootfs.tar.gz.bin
  x86_64/rootfs.tar.gz.bin.sha256
```

JNI libs：

```text
apps/flutter/app/android/app/src/main/jniLibs/arm64-v8a/
apps/flutter/app/android/app/src/main/jniLibs/armeabi-v7a/
apps/flutter/app/android/app/src/main/jniLibs/x86_64/
```

库：

```text
libbash.so
liboperit_busybox.so
liboperit_flutter_bridge.so
liboperit_loader.so
liboperit_proot.so
```

构建脚本：

```text
tools/android-runtime/fetch_sources.ps1
tools/android-runtime/fetch_ndk_wsl.sh
tools/android-runtime/build_android_tools_wsl.sh
tools/android-runtime/build_alpine_rootfs_wsl.sh
```

Android App 侧：

```text
AndroidRuntimeAssets.kt
MainActivity.kt
```

## 17. Kotlin To Rust Runtime Assembly

当前工程保留 Kotlin runtime 的文件命名与结构投影。Rust 文件大量使用 `.rs` 后缀承载 Kotlin 风格对象、字段与方法名。

例：

```text
Kotlin:
  D:/Code/prog/assistance/app/src/main/java/com/ai/assistance/operit/api/chat/EnhancedAIService.kt

Rust:
  core/crates/operit-runtime/src/api/chat/EnhancedAIService.rs
```

复刻规则：

```text
Kotlin 源文件是行为事实来源
Rust 模块名跟随 Kotlin 文件名
字段名保留 Kotlin camelCase
生命周期顺序跟随 Kotlin
工具、插件、memory、workspace、provider 边界跟随 Kotlin
新增模块先定位 Kotlin 对应文件
```

不做的事：

```text
不重排 Kotlin 已有模块顺序
不合并职责不同的 Kotlin 类
不把 host 细节塞进 runtime 业务模型
不让 Flutter UI 直接越过 proxy 访问 runtime 内部对象
不把插件私有协议写死到默认工具里
```

## 18. Development Navigation

常见定位路径：

```text
App 主导航
  apps/flutter/app/lib/ui/main/screens/OperitMainScreen.dart
  apps/flutter/app/lib/ui/main/screens/ScreenRouteRegistry.dart

聊天 UI
  apps/flutter/app/lib/ui/features/chat/screens/AIChatScreen.dart
  apps/flutter/app/lib/ui/features/chat/viewmodel/ChatViewModel.dart

聊天 runtime
  core/crates/operit-runtime/src/api/chat/ChatRuntimeHolder.rs
  core/crates/operit-runtime/src/api/chat/EnhancedAIService.rs

工具注册
  core/crates/operit-runtime/src/core/tools/ToolRegistration.rs
  core/crates/operit-runtime/src/core/tools/defaultTool/ToolGetter.rs

工具执行
  core/crates/operit-runtime/src/api/chat/enhance/ToolExecutionManager.rs
  core/crates/operit-runtime/src/core/tools/AIToolHandler.rs

Host 能力
  core/crates/operit-host-api/src/lib.rs
  hosts/{platform}/src/tools/*

Flutter bridge
  apps/flutter/app/lib/core/bridge/*
  apps/flutter/native/operit-flutter-bridge/src/lib.rs

插件包
  plugins/buildin/*
  core/crates/operit-runtime/src/core/tools/packTool/*
  core/crates/operit-runtime/src/plugins/toolpkg/*

工作区
  apps/flutter/app/lib/ui/features/chat/components/workspace/*
  core/crates/operit-runtime/src/data/repository/WorkspaceService.rs
  core/crates/operit-runtime/assets/workspace_templates/*

发布
  tools/release/RELEASE_SPEC.md
  tools/release/release.py
```

## 19. Assembly Checklist

总装检查：

```text
App 四个主导航入口已在 ScreenRouteRegistry 注册
AppRouterGateway 与 AppRouteDiscoveryGateway 在 OperitMainScreen 安装与清理
PhoneLayout 与 TabletLayout 都由同一 AppContent 驱动
ChatViewModel 仅通过 GeneratedCoreProxyClients 访问 core
CoreProxy request 使用 operit-link 协议
Native bridge 持有 LocalCoreProxy 与平台 host
runtime 通过 operit-host-api trait 调用平台能力
ToolRegistration 注册公开工具、内部工具、包工具、MCP 工具
ToolExecutionManager 完成 XML 解析、权限检查、执行、结果格式化
PackageManager 连接内置插件、安装包、ToolPkg bridge
WorkspaceShell 连接 UI panel 与 WorkspaceService
Release asset 命名符合 RELEASE_SPEC
Android runtime assets 和 JNI libs 按 ABI 放置
Rust runtime 文件持续对齐 Kotlin Operit 原实现
```
