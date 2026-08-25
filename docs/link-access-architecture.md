# Link / Access / Space Architecture

本文档只描述当前保留的两条链路。应用本地调用和 Space 节点间调用必须分开，
不能把远程节点能力重新挂回 Flutter/CLI Proxy。

## 1. 两条链路

### 本地应用链路

```text
Flutter / CLI
  -> GeneratedCoreProxy
  -> generated_core_dispatch
  -> ChatRuntimeHolder
  -> ChatRuntimeSlot
  -> ChatServiceCore
```

这一条链路只负责当前进程内的 Core 调用。Proxy 生成的对象直接进入本地 dispatch；
这里不经过 CoreNodeRouter、PeerLink、Space 或远程 session。

### Space 节点链路

```text
CoreNodeRouter
  -> PeerLink
  -> 远端 CoreNodeRouter
  -> 目标 Core 对象
```

Space 的调用、同步和跨端流全部从 CoreNodeRouter 接出。PeerLink 是节点之间的
双向数据载体，Flutter/CLI Proxy 不参与这条链路。

## 2. Link Access 的职责

Link Access 只提供控制面和 PeerLink carrier：

```text
/link/hello
/link/pair/start
/link/pair/finish
/link/session
/link/space/adopt
/link/peer/channel/events
/link/peer/channel/frame
/link/ws
```

配对、设备发现、session 建立、Space adopt 和服务器启动/关闭仍属于 Access 控制面。
控制面完成认证并建立 PeerLink 后，业务请求只进入 PeerFrame，不再创建应用层的
独立 HTTP RPC 入口。

## 3. 模块边界

| 模块 | 负责 | 不负责 |
| --- | --- | --- |
| `operit-proxy-local` | 生成本地 Proxy、对象 id 和本地 dispatch | Space 路由、设备信任、远程 session |
| `operit-node-runtime` | `CoreNodeRouter`、Space 路由、Binding 同步和本地 Core 目标解析 | Flutter/CLI 入口协议 |
| `operit-access-runtime` | 配对控制面、session、签名验证、PeerLink HTTP/WebSocket carrier | 本地 Proxy 调用投影 |
| `operit-link` | Core 值、调用/观察/反向流类型和通用错误模型 | 设备配对、session 存储和业务路由 |
| `operit-runtime` | Holder、Slot、ChatServiceCore 和运行时状态 | 节点间传输和配对 |
| Flutter / CLI | 调用生成的本地 Proxy、展示 Access 状态 | 自行拼装远程路由或 PeerFrame |

## 4. Space 请求模型

节点间请求统一包装为 `RoutedCoreRequest<T>`：

```text
RoutedCoreRequest<T> {
  spaceId
  targetNodeId
  ttl
  routeKind: ObjectId | SpaceRoute
  payload: T
}
```

`payload` 保持 `operit-link` 的标准类型：

```text
PeerRequest::Call(RoutedCoreRequest<CoreCallRequest>)
PeerRequest::Handoff(RoutedCoreRequest<CoreHandoffRequest>)
PeerRequest::BindingApply(RoutedCoreRequest<CoreNodeBindingApplyRequest>)
PeerRequest::WatchSnapshot(RoutedCoreRequest<CoreWatchRequest>)
PeerRequest::WatchOpen(PeerWatchOpenRequest)
PeerRequest::WatchClose(PeerWatchCloseRequest)
PeerRequest::PushOpen(PeerPushOpenRequest)
PeerRequest::PushItem(CorePushItem)
PeerRequest::PushClose(PeerPushCloseRequest)
```

`targetObjectId` 是生成器分配的对象地址。SpaceRoute 由 route 注解生成的注册表
解析；本地 Proxy 的对象 id 不会被当成远程链路入口。不存在分段对象路径或手写对象路径。

## 5. PeerFrame 载体

PeerLink 在认证的直接节点连接上交换以下帧：

```text
PeerFrame {
  messageId
  payload: Request | Response | WatchEvent | WatchClosed | Heartbeat
}
```

HTTP 只承载已经建立的 PeerLink channel：

* `events` 是远端主动发送 WatchEvent 的长连接方向。
* `frame` 发送请求、响应、Push item、关闭通知和心跳。

WebSocket 使用同一套 `PeerFrame` 语义。carrier 的选择不改变 CoreNodeRouter 的
调用模型，也不会生成第二条应用调用链。

## 6. 流和同步

Watch 和反向 Push 都拥有稳定的 `subscriptionId` 或 `pushId`，生命周期由 PeerLink
维护。相同 id 内保持顺序，不同 id 之间不定义全局顺序。断开 PeerLink 时，相关
订阅和输入流一起结束。

BindingApply 和 Space 持久化同步也使用 `PeerRequest::BindingApply`，由目标
`CoreNodeRouter` 执行或继续转发；它不经过应用 Proxy。

## 7. 明确禁止

以下内容不得重新加入应用入口或 `operit-proxy-local`：

```text
应用层远程 call/watch/push HTTP 路由
Flutter/CLI 到远端 CoreNode 的 Proxy 转发
分段对象路径、手写 CoreObjectPath 或 Holder 路径协议地址
把 PeerLink 请求转换成普通本地 Proxy 请求
```

配对、发现、静态 Web 服务器和控制面 session 仍由 Link Access 保留；它们与本地
Core 链路和 Space 数据链路相互独立。
