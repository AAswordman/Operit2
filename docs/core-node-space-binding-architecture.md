# CoreNode and Binding Architecture

本文档定义 Operit 多设备协作的最终架构。目标是让 CLI、桌面端、移动端和其他运行
Operit Core 的应用共同组成一个设备空间。设备空间内的所有设备持续拥有同一份持久化
业务数据，Agent 可以在工具调用边界把后续执行切换到任意设备。

## 0. 用户身份

用户身份是设备空间、配对和业务数据之上的本地顶层隔离：

```text
runtime/identities/<identityId>
workspace/identities/<identityId>
```

每个身份独立拥有设备身份、直接配对记录、设备空间成员关系、聊天、配置和工作区。身份
不进入设备空间同步，也不通过 Link 传播。Flutter 在全局抽屉切换身份，CLI 在 Core 启动
前通过 `identity` 命令选择身份；切换身份会重启应用并使用目标目录。浏览器端的 Worker、
OPFS、IndexedDB、本机密钥、下载任务和跨标签运行时所有权也全部按活动身份隔离。

执行模型只包含两个概念：

```text
CoreNode
Binding
```

`Space` 是节点成员、路由和持久化同步的范围。它不是执行容器，不拥有 Agent、聊天或
运行时。

代码中的聊天运行模块只是 CoreNode 内部实现，不是可寻址的分布式实体，也不拥有独立
身份、网络路由或绑定关系。

架构中不引入 Chat Core、Agent Core、Env、Placement、Lease、Circuit、Capability 或
远程数据放置 Ownership 模型。

## 1. CoreNode

一个运行 Operit Core 的应用实例就是一个 `CoreNode`：

```text
CoreNode {
  nodeId
  OperitApplication
  LocalCoreProxy
  HostManager
}
```

Flutter App、CLI 和其他平台 App 使用同一种 CoreNode 身份。每个 CoreNode 只有当前应用
实例已有的一套 `OperitApplication + LocalCoreProxy + HostManager`。文件系统、终端、
浏览器、HTTP 和平台交互都来自该节点自己的 `HostManager`。

CoreNode 不持有环境注册表。需要使用另一台设备或另一套 Host 能力时，Agent 切换到
另一个 CoreNode。

CoreNode 之间地位对等。服务监听方、连接发起方和请求执行方只是一次连接或调用中的
角色，不形成主节点和从节点。

## 2. Space

```text
Space {
  spaceId
  members: Set<nodeId>
}
```

Space 只负责三件事：

```text
确定哪些 CoreNode 属于同一个协作范围
确定哪些已配对连接可以承载 Space 流量
确定持久化数据应当在哪些 CoreNode 之间持续同步
```

Space 不表达执行位置，也不表达数据归属。加入同一 Space 的 CoreNode 都保存完整的
业务数据副本。

### 2.1 发现与加入

发现、配对和加入设备空间使用两个明确步骤：

```text
发现一个可连接的 Operit
  -> 使用现有配对握手建立直接信任
  -> 用户明确选择是否加入对方设备空间
  -> 双方通过普通 Link call 采用同一个 Space 投影
  -> 双方开始交换成员可达信息和持久化同步操作
```

配对完成后不会自动加入设备空间。加入和退出不是新的 Link 协议族，也没有
`space/join-request` 一类协议路径；它们是 `RuntimeRemoteLinkService` 上的普通 service
call。配对只建立两个节点之间的直接可信连接。Space 成员关系使请求可以经过其他成员逐跳
到达目标节点，但不会把中间节点与目标节点伪装成直接配对关系。

退出设备空间只重写本机成员记录，保留完整业务数据和直接配对记录，并创建新的单设备
空间。直接配对设备会先双向交换设备空间投影；只有双方 `spaceId` 相同，才继续交换聊天、
配置、Binding 和文件操作。因此退出状态可以传播，但退出后不会继续跨空间同步业务数据。

每个身份首次启动时都会创建一个单设备空间，初始名称使用该设备的显示名称。用户可以
独立重命名设备空间；退出共享空间后，新建的单设备空间同样使用本机设备显示名称。

一个直接配对连接称为 Peer Link。多个 Peer Link 组成 Space 的实际网络拓扑：

```text
A -------- B -------- C
|                     |
+--------- D ---------+
```

节点通过相邻 Peer Link 发布自己知道的 Space 成员和距离。每个节点据此维护内部
RouteTable。RouteTable 是运行时实现，不是领域概念。

## 3. Binding

Binding 使用业务方法声明的不透明 key：

```text
Binding {
  key
  nodeId
  generation
}
```

它只表达一句话：

> 这个 key 对应的下一次路由执行由哪个 CoreNode 完成。

Binding 层不知道 key 是否来自聊天、任务或其他业务对象。聊天路由直接使用现有 `chatId`
作为 key，但该含义只存在于 Chat 业务代码和方法注解中。Binding 不表示业务数据位于目标
节点；Agent、聊天、消息、记忆、工具结果、配置和 Binding 本身都由 Space 持久化同步
服务复制到所有成员。

同一个 Agent 的不同聊天可以绑定不同 CoreNode。切换 Binding 不会移动 Agent，也不会
移动其对话。Agent 在当前工具调用上下文中已经拥有 `chatId`，因此切换工具只需要目标
`nodeId`：

```text
list_core_nodes() -> { currentNodeId, nodeIds }
switch_core(nodeId)
```

`list_core_nodes` 只读取本机已经同步的 Space 成员视图。Agent 使用它获得精确节点 ID，
不引入 EnvRegistry、设备选择器或新的绑定概念。

每个需要路由的业务 key 必须有一条 Binding。创建可执行聊天时，Chat 仓库单独写入
`Binding(chatId)`。每次比较写入成功后 `generation` 单调递增，它同时标识目标节点即将
执行的 continuation。Binding 缺失或指向非 Space 成员时，请求返回明确错误。

Binding 的冲突顺序复用持久化同步操作的设备序列和向量时钟。同步元数据不是 Binding
的业务字段。

## 4. 两条独立的数据路径

### 4.1 持久化同步

`SpacePersistenceSyncService` 是 CoreNode 启动后长期运行的服务。它在每条直接 Peer
Link 上持续交换本地操作日志和向量时钟：

```text
preferences
chat
object data
agents and memories
tool calls and tool results
Binding
execution completion records
```

节点断开后保留自己的操作日志；重新连接时从双方时钟差异继续同步。同步不是由用户
触发的一次事务，也不依赖当前 Binding。

Runtime 文件的归属由 `RuntimeStorageLayout` 中的路径定义直接声明：

```text
RuntimeStoragePathDefinition {
  path
  ownership: Space | CoreNode | Ephemeral
  shape: Exact | Tree | RelativeFile
}
```

`RuntimeStorageRepository` 读取这份结构化元数据决定写入是否记录同步 operation。
`RuntimeFileSyncStore` 只处理已经声明为 Space 的文件，不维护 Skill、ToolPkg 配置、
USER.md、浏览器数据或临时目录等业务路径清单。未知路径和结构不合法的路径直接报错。

下列数据保持节点本地，不进入 Space 持久化同步：

```text
CoreNode 身份私钥
两两配对密钥和 session
HostManager 平台状态
本机监听地址
本机平台能力和临时交互句柄
```

### 4.2 实时 Core 传输

实时传输承载：

```text
Core call
Core watch
Core push
stream event
Agent turn continuation
Host interaction response
Space 路由控制消息
```

它不负责复制持久化业务数据。实时 continuation 可以要求目标节点先应用到指定同步
时钟；该时钟只是交接屏障，不表示数据迁移。

## 5. 转发层

现有 `CoreCallRequest`、`CoreWatchRequest`、`CorePushRequest`、`CoreEvent` 和
`CorePushItem` 保持不变。经过 Peer Link 时只增加一个外层：

```text
RoutedLink {
  spaceId
  targetNodeId
  ttl
  payload
}
```

`payload` 是现有 Link 消息或 Space 控制消息。外层不携带 chatId、Binding、envId、
文件路径或 Host 配置。

请求路径为：

```text
origin -> next peer -> ... -> target CoreNode
origin <- next peer <- ... <- target CoreNode
```

响应沿正在等待的逐跳调用链返回。现有 `requestId`、`subscriptionId` 和 `pushId` 继续
标识逻辑调用与流，不建立第二套流身份。

### 5.1 RouteTable

每个 CoreNode 保存 Space 内同步得到的连接拓扑，并在实际转发时只允许当前活跃的直接
Peer Link 作为第一跳：

```text
(spaceId, targetNodeId) -> nextPeer
```

同步拓扑用于计算最短路径，Peer Link 注册表用于证明第一跳此刻在线。HTTP 长响应流完成
握手后 Peer Link 才进入注册表；双向心跳持续刷新连接活性，心跳过期或 carrier 关闭时立即
撤销直接 Peer Link，并把新的本机连接投影作为持久化操作同步给其他设备。

`ttl` 每转发一次减一。`ttl` 到零、目标不属于 Space、nextPeer 不存在或 nextPeer 等于
上一跳时返回明确的路由错误。Binding 路由在发送前发现目标不可达时，使用
`expectedRemote -> localNodeId` 的比较写入由当前设备接管；比较冲突时重新读取已经提交的
Binding。中间设备返回明确的不可达错误时执行同一接管规则。显式指定设备的控制调用不
修改 Binding。

旧设备恢复在线后只恢复持久化双向同步，不自动夺回 Binding。下一次执行继续服从已经同步
收敛的新 Binding。

转发期间不得持有 Core、路由表或连接集合的互斥锁等待下游网络响应。每一跳先复制所需
路由和连接句柄，释放锁后再执行网络调用。

### 5.2 Call

每一跳向 nextPeer 发起一个下游 call 并等待结果。目标节点解开 RoutedLink，把原始
`CoreCallRequest` 交给自己的 `LocalCoreProxy`。响应沿原调用栈逐跳返回。

### 5.3 Watch

每一跳在 nextPeer 上打开同一个逻辑订阅，并把下游 `CoreEvent` 按顺序写给上游。上游
关闭、下游完成或 Peer Link 断开时，该跳释放对应订阅。

Binding 不参与普通 watch 转发。持久化聊天界面优先观察本地同步数据；明确观察某个
CoreNode 实时状态的 watch 才发送到该节点。

### 5.4 Push

每一跳固定打开时解析出的 nextPeer，并保持同一 `pushId` 内的 sequence 顺序。关闭
上游 push 时，该跳关闭下游 push。路由中断返回流错误，不把同一个 push 静默改送另一
节点。

## 6. 请求路由

本地 App 始终首先进入本机 CoreNodeRouter：

```text
Flutter / CLI / App
  -> local CoreNodeRouter
       -> ordinary request
            -> LocalCoreProxy
       -> annotated Binding request
            -> Binding(key)
            -> target CoreNode
       -> transport-directed CoreNode request
            -> target CoreNode
```

聊天历史读取、配置编辑和其他持久化数据操作在本地 Core 执行，由持续同步传播。只有
模型执行、工具循环、取消当前执行和实时执行状态进入 Binding 路由。

请求类型由真实 Core 方法上的 `#[operit_core_route(...)]` 声明，经 typed Core codegen
生成，不通过对象名、方法名或路径字符串猜测业务含义。Flutter、CLI 和其他客户端仍调用
普通 Core 方法，不创建 Binding 引用路径。请求显式携带 Binding key 时直接使用；未携带
时，本机 `CoreNodeRouter` 调用注解声明的 resolver 读取当前 key，把 key 注入真实请求后
再选择 Binding 目标。未标注的方法始终留在本地 Core。

## 7. 工具边界切换 CoreNode

当前模型工具循环在同一个进程内从工具结果直接进入下一轮模型请求。CoreNode 切换必须
发生在下面这个边界：

```text
工具调用已经完成
工具结果已经持久化
下一轮模型请求尚未发起
```

完整流程：

```text
Binding(chat-1) = node-A

1. node-A 的模型产生 switch_core(node-D)
2. node-A 验证 node-D 是可达的 Space 成员
3. node-A 在一个事务内持久化 assistant 工具调用、工具结果、chat metadata 和同步操作
4. 响应业务向 Router 返回只包含目标和不透明 payload 的 route transition
5. Router 比较写入 Binding(chat-1) = node-D，得到新的 generation 和 operation
6. Router 把 Binding record 和 operation 放入 Link route context，把不透明 transition 作为业务参数发送到 node-D
7. node-D 的 Router 在调用业务恢复方法前应用并校验 Binding operation，但不提前推进持久化同步 clock
8. node-D 的响应业务只接收 Link 定义的 route key/generation，等待 requiredClock
9. node-D 加载已同步 assistant 消息并使用 switch_core 的工具结果发起下一轮模型请求
10. node-D 的输出事件继续写入原来的逻辑响应流
```

`requiredClock` 是持久化同步屏障。execution generation 来自 Binding 比较写入，不复用
模型进程内部的 execution 或 round 身份。

Router 在比较写入前验证目标属于 Space 且 Link 路由可达。Binding 的比较写入成功后，
它就是新的唯一执行位置；后续持久化同步屏障或 continuation 失败必须返回明确错误，不
自动把 Binding 改回源节点。

### 7.1 单一执行者

每轮模型请求开始前，CoreNode 必须确认 `Binding(chatId)` 指向自己。Binding 更新使用
当前值比较写入，只有当前执行节点能够提交 Agent 发起的切换。旧节点观察到 Binding 已
改变后不得继续下一轮模型请求。

目标节点按 `(chatId, executionGeneration)` 识别 continuation。进程内集合只记录正在执行的
generation；启动前读取 assistant 消息的 `completedExecutionGeneration`，已经完成
相同或更高 generation 时直接返回，不重复调用模型或工具。

正常完成时，Binding Store 以不透明 key、当前节点和 generation 建立本地执行门控；门控
持有期间，最终 assistant message、结构化 parts、chat metadata、
`completedExecutionGeneration` 和同步操作在同一个 SQLite 事务内提交。Binding
Store 不读取任何 Chat 类型或表。

### 7.2 响应流连续性

响应流属于一次用户发起的 execution，不属于执行 CoreNode。最初接受请求的
CoreNodeRouter 保持上游流；当前执行节点只产生该 execution 的后续事件。

发生切换时，当前执行节点把 handoff 结果返回给 origin router，origin router 根据新的
Binding 打开下一段 continuation，并把事件继续写入同一个上游 stream。多次切换不会
形成 A -> B -> C 的永久转发链。

## 8. 现有模块职责

### 8.1 operit-store

负责持久化操作日志、向量时钟、Binding 存储以及各业务域操作的应用。它不知道网络
拓扑和实时 Core 调用。

### 8.2 operit-link

继续定义 call、watch、push、event、error、stream 和 route context 的协议语义。它不读取
Binding 存储，也不知道配对、Space 拓扑或任何 Chat 业务类型。

### 8.3 operit-link-access

负责现有配对、认证、session 和 Peer Link 承载，并增加：

```text
通过普通 call 交换设备空间投影
成员与路由拓扑发布
RoutedLink 承载
逐跳 call/watch/push 转发
```

### 8.4 operit-core-proxy

本机 `CoreNodeRouter` 负责：

```text
执行 codegen 生成的 call/watch/push 路由分类
从本机 Core 解析当前 Binding key
读取 Binding
选择本地 Core 或 Space 路由
保持 execution 的上游响应流
执行 codegen 生成的通用 Binding watch transition 门控
在业务恢复调用前安装 Link 携带的 Binding route context
```

具体 routed call/watch、resolver、transition 和 resume 方法由真实 Core 方法注解声明。
transition 和 resume 钩子只进入内部 dispatch 与路由元数据，不生成 Flutter、CLI 或 Rust
公共客户端方法。
`CoreNodeRouter` 只处理 route、Binding record 和 Link 调用，不包含 Chat 方法名、字段或
ContinueTurn 业务分支。

`SpacePersistenceSyncService` 遍历每个直接出站配对，先交换双方设备空间投影，再根据
`spaceId` 决定是否执行持久化业务同步。现有配对和 session 存储继续由 Link Access 持有。

### 8.5 operit-runtime

保持单个 `OperitApplication` 和 `HostManager`。模型工具循环在处理工具结果后识别
`switch_core` 的控制结果，生成 handoff 并停止本地下一轮模型请求。

跨平台差异继续全部通过 Host API 和现有 Flutter bridge 表达。普通 Rust 和 Dart 业务
代码不增加平台分支。

## 9. 安全与成员边界

每一跳只接受已配对 Peer 的签名流量。收到 RoutedLink 后必须验证：

```text
当前节点属于 spaceId
上一跳属于 spaceId
targetNodeId 属于 spaceId
payload 类型允许在该 Peer Link 上转发
```

中间节点保留真实上一跳 session 作为审计来源。目标 CoreNode 获得 origin nodeId 和经过
的相邻 session 审计记录，不能把所有多跳请求折叠成最后一个中继节点的控制者身份。

设备身份、配对 secret 和 Host 授权保持节点本地。Space 成员资格不授予其他节点直接
读取本机文件或终端的能力；只有在该 CoreNode 上执行的 Agent 工具通过本机 HostManager
访问这些能力。

## 10. 必须保持的约束

```text
一个 CoreNode 只有一个 OperitApplication 和一个 HostManager
Space 内所有持久化业务数据持续同步
Binding 只决定 chat 的下一步执行节点
Binding 改变不移动数据
工具结果持久化完成后才能交接下一轮模型调用
一个 chat 的同一执行轮次只有一个 CoreNode 可以推进
转发只复用已有 Link call/watch/push 身份
两两配对仍是所有直接网络信任的基础
路由和网络等待期间不持有全局 Core 互斥锁
Binding 目标不可达时由发起设备原子接管，显式设备调用仍返回明确错误
```

最终关系是：

> Space 把多个已配对 CoreNode 组成同一个持续同步网络；Binding 决定一个聊天的下一步
> 在哪个 CoreNode 执行；实时转发层把现有 Link 调用和 Agent continuation 送到该节点。
