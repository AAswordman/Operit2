# Link / Access / Remote Architecture

本文档定义 Operit2 中 Proxy、Link、Access、Remote 的边界。后续实现、评审与排查都以这里的职责划分为准。

## 1. Terms

```text
Proxy
  core 能力在 app 侧的代理投影。
  典型对象是 CoreProxy、生成的 typed proxy client、LocalCoreProxy。

Link
  Proxy 调用语义的穿透协议。
  它描述 call、watch、event、error、stream 的请求与返回形状。

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
| operit-link | call/watch/event/error/stream 协议，link envelope 与承载工具 | 配对、session store、签名算法、设备信任、listener 启动、静态文件服务 |
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

TTS owner interaction：

```text
Android SYSTEM_TTS playback
  -> TtsPlaybackService
  -> AndroidTtsPlaybackHost callback
  -> RuntimeHostInteractionService(ttsPlayback)
  -> OwnerSystemCapabilityChannel
  -> Android TextToSpeech
```

远程连接只转发 core call/watch 请求到 runtime owner。`TtsPlaybackHost`、`TtsSynthesisHost`、Android owner interaction 都归 runtime owner 所在 app/host 处理；remote client 不接管远端 runtime 的 host 能力。

## 5. Watch Stream Protocol

`watchStream` 表示一个逻辑订阅。协议必须把多个逻辑订阅复用到少量物理 watch channel 上，以解决浏览器和移动端连接数量、系统资源和调度开销问题。

watch channel 是主动事件通道。实现应等待 core 侧产生事件，并在事件出现时写入物理通道；固定间隔轮询不是 Core Link watch 协议语义，不得作为 watch stream 传输接口。

HTTP watch channel 使用以下端点：

```text
POST /link/watch/channel/events
POST /link/watch/channel/open
POST /link/watch/channel/close
```

`/link/watch/channel/events` 打开物理事件通道，请求体只包含 channel：

```json
{
  "channelId": "watch-channel-0"
}
```

HTTP 响应必须是持续事件流：

```text
content-type: application/x-ndjson
```

每一行是一个 watch channel frame。当前 frame 形状为：

```json
{
  "subscriptionId": "watch-0",
  "event": {
    "kind": "Changed",
    "value": {}
  }
}
```

frame 中的 `event` 必须是 `CoreEvent`。接收方按 `subscriptionId` 分发到对应的 `watchStream`。读端必须忽略未知字段，以便后续增加 `channelId`、`seq`、`type` 等诊断或排序字段。

`/link/watch/channel/open` 在既有 channel 上创建逻辑订阅：

```json
{
  "channelId": "watch-channel-0",
  "subscriptionId": "watch-0",
  "request": {
    "requestId": "flutter-0",
    "targetPath": { "segments": ["chatRuntimeHolder", "main"] },
    "propertyName": "chatHistoryFlow",
    "args": {}
  }
}
```

服务端必须原样返回 `subscriptionId`：

```json
{
  "subscriptionId": "watch-0"
}
```

`subscriptionId` 由客户端生成，作用域限定在 `channelId` 内。同一个 channel 内重复 `subscriptionId` 是协议错误。一个 channel 可以承载多个 subscription；当前客户端约定一个 channel 最多承载 16 个 subscription。

`/link/watch/channel/close` 关闭一个逻辑订阅：

```json
{
  "channelId": "watch-channel-0",
  "subscriptionId": "watch-0"
}
```

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
  使用 /link/watch/channel/events 的 NDJSON 长响应。

WebSocket
  可承载相同 watch channel frame，但 frame 语义必须与 HTTP watch channel 一致。

Android / desktop native bridge
  call、watchSnapshot、watch channel open/close 可以走请求响应通道。
  watch channel event 必须由 native 主动推送到 Dart，再按 subscriptionId 分发。
  Dart UI isolate 不应通过固定周期 poll 来驱动 watchStream。

Web / wasm bridge
  JS/wasm 侧应提供主动事件回调或等价事件通道，再按 subscriptionId 分发。
```

核心约束：

```text
多个 watchStream 合并到少量 watch channel。
watch channel 主动传输事件。
轮询不是 watch stream 协议。
```

## 6. Prohibited Placement

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
Link 用于 core call/watch/event 穿透协议。
Remote 只用于跨 app 连接场景与产品模式。
```
