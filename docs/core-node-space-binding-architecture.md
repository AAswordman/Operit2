# Core Node / Space / Binding 最终架构

本文档定义 Operit 多节点网络的最终架构。它不是阶段性方案，也不描述过渡版本。

整个架构只有三个领域概念和一个 Link 外层信封：

```text
CoreNode
Space
Binding
RoutedLink
```

不引入 Source、Placement、Lease、Circuit、Capability、OwnershipEpoch 等模型。

## 1. 当前代码结构

当前 App 内部调用链是：

```text
Flutter App
  -> GeneratedCoreProxyClients
  -> MethodChannel / WASM
  -> OperitFlutterBridge
       -> RuntimeCoreRouter
            -> LocalCoreProxy
            -> PairedRemoteSession
```

每个 App 都创建并持有一个本地 `OperitApplication`：

```text
LocalCoreProxy
  -> OperitApplication
       -> ChatRuntimeHolder
       -> Services
       -> HostManager
```

当前边界如下：

- `HostManager` 是本机文件系统、终端、HTTP、存储、浏览器等平台能力的统一入口。
- `operit-link` 定义 call、watch、push、event、error 和 stream 的穿透协议。
- `operit-link-access` 负责配对、认证、session、监听和远程 Link 承载。
- `RuntimeCoreRouter` 当前在整个 Runtime 范围内选择 `Local` 或一个 `Remote(sessionName)`。
- `RemoteLinkServer` 当前直接把远程请求交给 `LocalCoreProxy`，因此不能继续向其他节点中转。
- `PairedRemoteSession` 当前表达一个被选中的远程 Runtime 连接。

现有的双向 Host Server 和两两配对连接继续保留。最终架构改变的是寻址和中转方式，不重做底层连接。

## 2. 最终模型

基础标识只有：

```text
CoreNodeId = String
EnvId = String
ChatCoreId = String
```

### 2.1 CoreNode

一个运行 Operit Core 的 App 实例就是一个 `CoreNode`，由 `CoreNodeId` 唯一标识。

CoreNode 之间地位对等。两台 CoreNode 建立的连接是双向 Peer Link，不存在固定的服务端节点、客户端节点、主节点或从节点。

CoreNode 不要求位于同一局域网。任意两个节点能够建立网络连接即可成为直接 Peer；没有直接连接的节点通过 Space 内其他 CoreNode 逐跳中转。

每个 CoreNode 内部持有以下运行时状态：

```text
BindingStore
EnvRegistry
RouteTable
LocalCoreProxy
DirectPeerLinks
```

这些是 CoreNode 的内部实现，不是新的领域概念。

### 2.2 Space

```text
Space {
  spaceId
  members: Set<CoreNodeId>
}
```

Space 只表达一件事：这些 CoreNode 属于同一个逻辑网络，可以通过直接 Peer Link 或中间 CoreNode 相互传递请求。

Space 不表达执行位置、资源能力、聊天数据归属或连接方向。

### 2.3 Binding

```text
Binding {
  chatCoreId
  nodeId
  envId
}
```

Binding 只表达：

```text
一个 Chat Core
运行在哪个 Core Node
使用该 Core Node 上的哪个 env
```

示例：

```text
chat-1 -> node-C / windows-main
chat-2 -> node-A / default
chat-3 -> node-D / linux-workspace
```

`ChatCoreId` 是 Agent 和任务对话空间的稳定身份。它不因执行节点或环境改变而改变。

Binding 是 Space 范围内的共享状态。Space 中任意 CoreNode 都能根据 `chatCoreId` 得到同一条 Binding。Binding 的传输复用现有同步系统，不建立另一套归属或一致性模型。

Binding 不负责聊天数据复制。聊天历史和 Chat Core 所需数据仍由现有同步系统传输。

## 3. Env

每个 CoreNode 都持有完整的环境注册表：

```text
EnvRegistry {
  envId -> HostManager
}
```

EnvRegistry 允许同时挂载任意数量的环境：

```text
default        -> HostManager
windows-main   -> HostManager
wsl            -> HostManager
docker-project -> HostManager
```

节点实际注册一个环境还是多个环境，只是该节点当前的数据状态，不改变架构。

`envId` 只是目标 CoreNode 内部的环境名称。Binding 不携带文件系统、终端、浏览器、模型或平台配置。一个 env 对应一个完整的 `HostManager`，Chat Core 在该 env 中运行时，所有 Host 能力都从这个 `HostManager` 取得。

因此 Chat Core 绑定到另一台电脑上的 env 后，文件、终端、浏览器和其他工具自然在目标 CoreNode 的 Host 环境中执行，不建立远程文件系统协议或远程终端协议。

## 4. RoutedLink

现有 `CoreCallRequest`、`CoreWatchRequest` 和 `CorePushRequest` 保持不变，只在 Peer Link 上传输时增加一个外层：

```text
RoutedLink {
  spaceId
  targetNodeId
  ttl
  payload
}
```

`payload` 是现有 Link 消息。`RoutedLink` 不增加以下字段：

```text
sourceNodeId
routeId
ownershipEpoch
circuitId
previousHash
envId
```

`envId` 来自 `Binding(chatCoreId)`，不属于网络路由信封。

请求逐跳代理，响应沿原调用链返回：

```text
A -> B -> C -> D
A <- B <- C <- D
```

现有 `requestId`、`subscriptionId` 和 `pushId` 继续标识调用和流，不建立另一套跨节点流身份。

## 5. 请求流程

假设：

```text
Binding(chat-1) = node-D / linux
```

节点 A 上的 App 操作 `chat-1`：

```text
Flutter A
  -> 本地 CoreNode A
  -> 查询 Binding(chat-1)
  -> targetNodeId = D
  -> 查询 nextHop(D) = B
  -> RoutedLink 发送给 B
  -> B 查询 nextHop(D) = C
  -> RoutedLink 发送给 C
  -> C 查询 nextHop(D) = D
  -> RoutedLink 发送给 D
  -> D 解开 RoutedLink
  -> 查询 Binding(chat-1)，得到 envId = linux
  -> EnvRegistry[linux]
  -> LocalCoreProxy
  -> Chat Core
```

中间节点只执行三项操作：

```text
验证当前节点和目标节点属于 RoutedLink 指定的 Space
减少 ttl
根据 targetNodeId 查询 nextPeer 并转发
```

目标节点负责根据 `chatCoreId` 读取 Binding、选择 env，并把原始 Link payload 交给对应环境中的本地 Core。

不属于 Chat Core 的本机控制请求直接交给本机 `LocalCoreProxy`，不读取 Binding。请求类型由现有 typed Core 路径确定，不通过字符串包含关系猜测。

## 6. RouteTable

RouteTable 是每个 CoreNode 的内部运行时状态：

```text
RouteTable {
  targetNodeId -> nextPeer
}
```

直接相连的 Peer 通过 Link Access 控制流发布可达的 CoreNode 和距离：

```text
A 告诉 B：A 可达，距离 0
B 告诉 C：A 可达，距离 1
C 记录：到 A 的 nextPeer 是 B
```

路由采用距离最短的 nextPeer。可达信息属于 Link Access 的内部路由协议，不形成新的业务对象。

`ttl` 每经过一个 CoreNode 减一。`ttl` 到零时结束该次请求并返回明确的 Link 路由错误，从而终止拓扑变化期间形成的转发循环。

## 7. Call / Watch / Push

### 7.1 Call

每一跳向下游执行现有 Link call，并等待下游响应。响应沿等待中的调用链逐跳返回。

### 7.2 Watch

每一跳向下游打开现有 watch stream，并把下游产生的 `CoreEvent` 持续写给上游。关闭上游订阅时，同一跳关闭对应的下游订阅。

### 7.3 Push

每一跳向下游打开现有 push stream，并把上游的 `CorePushItem` 按原顺序送给下游。关闭上游 push 时，同一跳关闭对应的下游 push。

逐跳代理直接复用现有 Link call/watch/push 生命周期，不建立 Circuit 模型。

## 8. Binding 转移

Chat Core 或 Agent 切换执行电脑与环境，只修改一条 Binding：

```text
原 Binding：
chat-1 -> node-A / local

新 Binding：
chat-1 -> node-C / windows-main
```

Binding 修改完成后：

```text
新 call 根据新 Binding 发往 node-C
现有 watch/push 关闭
watch/push 根据新 Binding 在 node-C 重新打开
已经进入原调用链的 call 沿原调用链完成并返回
```

转移不改变 `ChatCoreId`，不复制 HostManager，也不把文件系统或终端会话塞进 Binding。Chat Core 所需数据由现有同步系统到达目标节点，执行环境由目标节点的 `EnvRegistry` 提供。

## 9. 现有模块的最终职责

### 9.1 RuntimeCoreRouter

当前的全局选择：

```text
Local | Remote(sessionName)
```

替换为按 Chat Core 寻址：

```text
chatCoreId -> Binding -> nodeId / envId
```

它不再把整个 App 切换到某一台远程 Runtime。本地 App 始终首先进入本地 CoreNode。

### 9.2 RemoteLinkServer

当前入口：

```text
RemoteLinkServer -> LocalCoreProxy
```

最终入口：

```text
RemoteLinkServer -> CoreNodeRouter
```

CoreNodeRouter 的分发规则是：

```text
targetNodeId == selfNodeId
  -> 选择 Binding 指定的 env
  -> LocalCoreProxy

targetNodeId != selfNodeId
  -> RouteTable[targetNodeId]
  -> PairedRemoteSession
```

`CoreNodeRouter` 是 CoreNode 的内部组件，不是第四个领域概念。

### 9.3 PairedRemoteSession

保留现有实现，语义固定为：

```text
一个直接相连的邻居 CoreNode Peer Link
```

它不再表示整个 Runtime 当前选中的远端执行位置。

### 9.4 operit-link

现有 call、watch、push、event、error 和 stream 语义保持不变。

### 9.5 operit-link-access

继续负责：

```text
配对
认证
session
双向 Peer Link
服务器监听
RoutedLink 承载
逐跳代理
路由可达信息交换
```

Space、Binding 和 env 的业务含义不进入 `operit-link` 基础协议。

## 10. 最终结构

```text
Flutter
  -> 本地 CoreNodeRouter
       -> BindingStore
       -> EnvRegistry
       -> RouteTable
       -> LocalCoreProxy
       -> Direct Peer Links
```

多个 CoreNode 通过两两双向 Peer Link 组成 Space：

```text
       A -------- B
       |          |
       |          C -------- D
       |                     |
       E --------------------+
```

节点之间没有主被动关系。直接相连的节点负责彼此通信，中间节点负责 Link 请求中转，任意 Chat Core 的执行位置由 Binding 唯一决定。

最终规则只有一句：

> Space 决定哪些 CoreNode 属于同一网络，Binding 决定 Chat Core 在哪个 CoreNode 和 env 运行，RoutedLink 通过两两连接把现有 Link 请求送到该 CoreNode。
