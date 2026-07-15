# Link / Access / Remote Architecture

本文档定义 Operit2 中 Proxy、Link、Access、Remote 的边界。后续实现、评审与排查都以这里的职责划分为准。

## 1. Terms

```text
Proxy
  core 能力在 app 侧的代理投影。
  典型对象是 CoreProxy、生成的 typed proxy client、LocalCoreProxy。

Link
  Proxy 调用语义的穿透协议。
  它描述 call、watch、push、event、error、stream 的请求与返回形状。

Access
  app 之间建立信任关系的控制面。
  它负责配对、session、签名、设备信任、权限 UI、server 组合与 Web Access UX。

Remote
  跨 app 使用 link 的连接场景。
  Remote 是产品语义和运行模式，不是 operit-link 的子模块。
```

核心关系：

```text
local
  app -> Proxy -> Link -> core

remote
  app -> Access -> Link -> host app -> Link -> core

web access
  web app -> Access -> Link -> host app -> Link -> core
```

## 2. Responsibility Matrix

| Part | Owns | Must Not Own |
| --- | --- | --- |
| runtime/core | 核心执行、core 对象状态、进程内 core 能力 | 配对、session、签名、server 生命周期、Web Access UX |
| CoreProxy | app 侧 typed 调用投影、把 typed 调用转换成 link request | app 间信任、密钥、远程连接生命周期 |
| operit-link | call/watch/push/event/error/stream 协议，link envelope 与承载工具 | 配对、session store、签名算法、设备信任、listener 启动、静态文件服务 |
| app access | 配对、session、签名、设备信任、权限 UI、server 组合 | core 业务执行、runtime 内部状态 |
| Flutter Dart | 配对 UI、session 持久化、remote link client、本地/远程选择 | core 内部对象、host server 监听 |
| Flutter native Rust | host server、accepted session、access endpoint、link dispatcher 接入 LocalCoreProxy | Dart UI 状态、普通 Web wasm runtime 路径 |
| CLI app | CLI session、配对命令、serve/connect/sync/watch | operit-link 内部 access 状态 |
| Web Access JS | 浏览器 app 的配对、session、签名、link 调用 | wasm local runtime 替换、host app server 生命周期 |

## 3. Module Ownership

```text
core/crates/operit-link
  src/protocol.rs
  src/client.rs
  src/http.rs

apps/flutter/app/lib/core/link
  Dart link protocol models
  remote runtime link client

apps/flutter/app/lib/core/path
  app access storage paths
  runtime connection config: client/access/runtime_connection.json

apps/flutter/native/operit-flutter-bridge
  Flutter host access server
  accepted session store
  pairing endpoints
  link dispatcher wiring

apps/cli/src
  CLI app access session
  link serve/connect command behavior
  CLI remote link client
  CLI access storage paths
  client/access/link_sessions.json
  client/access/link_server_sessions.json
```

`operit-link` 可以提供 HTTP/WebSocket 的 link 承载工具，但这些工具只处理已经被 app access 接受的 link 请求。请求是谁发来的、是否可信、用什么 session、签名如何验证，都由调用它的 app access 层决定。

远程请求的 `sessionId` 与 `deviceId` 保留在 app access 层；host access 验签后把二者写入 `RuntimeHostInteractionRequestOrigin::RemoteSession`，再交给 `operit-link` dispatcher。`operit-link` 协议 envelope 不增加 target device 字段。

## 4. Request Flow

Flutter local：

```text
Flutter UI
  -> CoreProxy
  -> MethodChannel / wasm bridge
  -> LocalCoreProxy
  -> core
```

Flutter remote：

```text
Flutter UI
  -> CoreProxy
  -> RemoteRuntimeLinkClient
  -> app access signed HTTP
  -> host app access
  -> operit-link HTTP dispatcher
  -> LocalCoreProxy
  -> core
```

CLI remote：

```text
CLI command
  -> CLI remote link client
  -> app access signed HTTP
  -> host app access
  -> operit-link HTTP dispatcher
  -> LocalCoreProxy
  -> core
```

Web Access：

```text
browser web app
  -> WebCrypto pairing/session/signing
  -> app access signed HTTP
  -> host app access
  -> operit-link HTTP dispatcher
  -> LocalCoreProxy
  -> core
```

普通 Flutter Web：

```text
Flutter Web
  -> operit_runtime_bridge.js
  -> operit_flutter_bridge_bg.wasm
  -> wasm LocalCoreProxy
  -> core
```

普通 Flutter Web 不经过 Web Access remote client。

Host runtime event ingress：

```text
platform host source
  -> operit-host-api HostRuntimeEventHost
  -> RuntimeEventIngressService
  -> ToolPkg host event hook dispatch
```

Android app-hosted runtime event ingress：

```text
Android BroadcastReceiver
  -> app RuntimeEvents topic mapping
  -> JNI emitRuntimeEvent
  -> RuntimeEventIngressService
  -> ToolPkg host event hook dispatch
```

Host 反向触发 runtime 只走 host runtime ingress。它不是 CoreProxy 调用，不进入 `operit-link` 协议，也不通过 Flutter UI bridge 增加事件入口。桌面平台由 host crate 实现 `HostRuntimeEventHost`；Android 因系统广播只能由 app 进程注册，所以由 Android app 侧收集广播后送入同一个 `RuntimeEventIngressService`。

Runtime owner host interaction：

```text
runtime core
  -> RuntimeHostInteractionService
  -> owner app subscriber / owner host bridge
  -> platform capability
```

交互式浏览器会话：

```text
controller app
  -> CoreProxy / Link push input
  -> RuntimeBrowserService
  -> BrowserSessionHost
  -> RuntimeHostInteractionService(browserSession)
  -> runtime owner app subscriber
  -> runtime owner app real WebView
  -> RuntimeBrowserService.browserSessionEvents
  -> Link watch stream
  -> controller app projection view
```

控制端 view 不加载外部页面 URL，也不持有 owner app 的 WebView controller。
外部页面只在 runtime owner app 的真实 WebView 中运行；view 通过 Link 接收语义
投影并把用户交互送回同一会话。

TTS owner interaction：

```text
Android SYSTEM_TTS playback
  -> TtsPlaybackService
  -> AndroidTtsPlaybackHost callback
  -> RuntimeHostInteractionService(ttsPlayback)
  -> OwnerSystemCapabilityChannel
  -> Android TextToSpeech

Generated TTS audio playback
  -> TtsSynthesisService writes runtime storage audio
  -> TtsPlaybackService.playAudio
  -> runtime owner's TtsPlaybackHost
  -> owner platform speech audio player
```

远程连接只转发 core call/watch/push 请求到 runtime owner。`TtsPlaybackHost`、`TtsSynthesisHost`、Android owner interaction 都归 runtime owner 所在 app/host 处理；remote client 不接管远端 runtime 的 host 能力。

## 5. Push Stream Protocol

`push` 是 `watch` 的方向对偶。`watch` 由 Core 持续向客户端发送事件；
`push` 由客户端打开输入流，持续向 Core 方法发送参数值。它不是 call batching，
每个 item 不创建独立的 Link request/response 操作。

协议消息：

```text
CorePushRequest {
  requestId: "browser-input-0",
  targetPath: CoreObjectPath { segments: ["services", "runtimeBrowserService"] },
  methodName: "submitBrowserCommand"
}

CorePushItem {
  pushId: "browser-input-0",
  sequence: 42,
  args: CoreValue::Map(...)
}
```

`pushId` 标识逻辑输入流，`sequence` 在该流内单调递增。carrier 必须保持同一
`pushId` 的 item 顺序。打开和关闭定义流生命周期；向不存在或已经关闭的流发送
item 必须返回 `PUSH_NOT_FOUND`，不得转成普通 call 重试。

HTTP carrier 提供：

```text
POST /link/push/open
POST /link/push/item
POST /link/push/close
```

WebSocket carrier 使用签名信封承载原始 MessagePack payload bytes，payload bytes
内部仍是 tagged frame：`PushOpen`、`PushItem`、`PushClose`。签名对象必须是
`payloadBytes` 原始字节，服务端验证同一段字节后再解码，不得通过重编码
结构化 payload 参与验签。高频交互应使用独立 WebSocket push carrier，不能反复请求
`/link/call`。push 输入与 watch 输出使用不同物理通道，避免大体积 watch frame
阻塞鼠标、滚轮和键盘输入。

Flutter MethodChannel、WASM 和 CLI 必须暴露同一组 push 生命周期语义。平台载体
可以按自身消息机制传输 item，但 Dart API 必须表现为 `CorePushSink.add/close`，
而不是让业务层循环调用 `call`。

Link v3 首次加入 push；v2 与 v3 不协商混用，握手、HTTP header 和 WebSocket
envelope 必须声明版本 3。WebSocket envelope 字段固定为
`protocolVersion/sessionId/deviceId/signature/payloadBytes`。

## 6. Watch Stream Protocol

`watchStream` 表示一个逻辑订阅。协议必须把多个逻辑订阅复用到少量物理 watch channel 上，以解决浏览器和移动端连接数量、系统资源和调度开销问题。

watch channel 是主动事件通道。实现应等待 core 侧产生事件，并在事件出现时写入物理通道；固定间隔轮询不是 Core Link watch 协议语义，不得作为 watch stream 传输接口。

HTTP watch channel 使用以下端点：

```text
POST /link/watch/channel/events
POST /link/watch/channel/open
POST /link/watch/channel/close
```

`/link/watch/channel/events` 打开物理事件通道，请求体只包含 channel：

```text
LinkWatchChannelEnvelope {
  channelId: "watch-channel-0"
}
```

请求体使用 MessagePack 编码。HTTP 响应是持续二进制事件流：

```text
content-type: application/msgpack-seq
```

流中每个 frame 的线格式为：

```text
[u32 big-endian payload length][MessagePack LinkWatchChannelEvent payload]

LinkWatchChannelEvent {
  subscriptionId: "watch-0",
  event: CoreEvent { ... }
}
```

frame 中的 `event` 必须是 `CoreEvent`。接收方必须先按长度边界解码完整 MessagePack frame，再按 `subscriptionId` 分发到对应的 `watchStream`。

`/link/watch/channel/open` 在既有 channel 上创建逻辑订阅：

```text
LinkWatchChannelOpenEnvelope {
  channelId: "watch-channel-0",
  subscriptionId: "watch-0",
  request: CoreWatchRequest {
    requestId: "flutter-0",
    targetPath: CoreObjectPath { segments: ["chatRuntimeHolder", "main"] },
    propertyName: "chatHistoryFlow",
    args: CoreValue::Map(...)
  }
}
```

请求和响应都使用 MessagePack 编码。服务端必须原样返回 `subscriptionId`：

```text
LinkWatchChannelOpenResponse {
  subscriptionId: "watch-0"
}
```

`subscriptionId` 由客户端生成，作用域限定在 `channelId` 内。同一个 channel 内重复 `subscriptionId` 是协议错误。一个 channel 可以承载多个 subscription；当前客户端约定一个 channel 最多承载 16 个 subscription。

`/link/watch/channel/close` 关闭一个逻辑订阅：

```text
LinkWatchChannelCloseEnvelope {
  channelId: "watch-channel-0",
  subscriptionId: "watch-0"
}
```

请求和 `LinkWatchChannelCloseResponse` 响应都使用 MessagePack 编码。

事件顺序规则：

```text
同一个 subscriptionId 内，CoreEvent 顺序必须保持。
不同 subscriptionId 之间，不定义全局顺序。
CoreEvent.kind == Completed 表示该 subscription 自然结束。
发送 Completed 后，服务端必须释放该 subscription。
客户端主动 close 后，不要求服务端再发送 Completed。
物理 channel 断开时，该 channel 上全部 subscription 都失效。
```

各运行环境必须实现同一套 watch channel 语义：

```text
HTTP remote
  控制请求和普通响应使用 application/msgpack。
  /link/watch/channel/events 使用 application/msgpack-seq，按 u32 大端长度分帧。

WebSocket
  使用二进制 MessagePack 消息承载相同 Link 语义。

Android / desktop native bridge
  call、watchSnapshot、watch channel open/close 可以走请求响应通道。
  push open/item/close 必须保持逻辑流身份和 item 顺序；高频 push 使用独立载体。
  watch channel event 必须由 native 主动推送到 Dart，再按 subscriptionId 分发。
  Dart UI isolate 不应通过固定周期 poll 来驱动 watchStream。

Web / wasm bridge
  JS/wasm 侧应提供主动事件回调或等价事件通道，再按 subscriptionId 分发。
```

Native / WASM local CoreProxy carrier 的全部请求、响应和 watch event 使用紧凑
MessagePack tuple，避免在 Dart 与 Rust 之间构造 Link envelope 的字段名 map：

```text
callRequest          = [requestId, targetPathSegments, methodName, args]
pushOpenRequest      = [requestId, targetPathSegments, methodName]
pushItem             = [pushId, sequence, args]
watchSnapshotRequest = [requestId, targetPathSegments, propertyName, args]
watchStreamRequest   = [subscriptionId, requestId, targetPathSegments, propertyName, args]

resultSuccess = [0, payload]
resultFailure = [1, code, message, details, location, backtrace]
event         = [requestId, targetPathSegments, propertyName, kind, value]
watchEvent    = [subscriptionId, event]
watchError    = [1, subscriptionId, code, message]
location      = null | [file, line, column]
```

`pushClose` 和 `closeWatchStream` 直接携带其 string ID，响应仍使用 `resultSuccess` /
`resultFailure`。`args`、`value` 和 `details` 是 Link `CoreValue` 的原生 MessagePack 值，
业务对象可继续使用 map。该 tuple 只属于同进程 native 和 WASM local carrier；HTTP、WebSocket
和 CLI 仍使用 Link 对外协议的标准 MessagePack envelope。

核心约束：

```text
多个 watchStream 合并到少量 watch channel。
watch channel 主动传输事件。
轮询不是 watch stream 协议。
```

## 7. Prohibited Placement

以下内容不得放进 `operit-link`：

```text
host runtime event ingress
host event topic mapping
HostRuntimeEventHost implementation
PairStart / PairFinish
PairedRemoteSession
RemoteLinkServer
RemoteLinkClient
AcceptedRemoteSession
RemoteWebAccessConfig
sessionSecret
HMAC signing
pairing code
token validation
listener startup
static web access assets
```

以下内容不得放进 runtime/core：

```text
platform event collection
Android BroadcastReceiver registration
DBus / Windows message-loop details
app-to-app pairing
device trust
access session storage
request signature verification
HTTP server composition
Web Access UX
```

以下命名规则必须保持：

```text
Proxy 只用于 core 能力代理投影。
Link 用于 core call/watch/push/event 穿透协议。
Remote 只用于跨 app 连接场景与产品模式。
```
