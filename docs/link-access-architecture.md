# Link / Peer / Transport / Space Architecture

本文按 2026-10-13 的源码描述当前链路。Link 的三个基础操作是 **Call、Watch、Push**。
传输统一和路由行为合规是两个独立结论：当前五种 transport 已统一承载标准 Link
消息，共享客户端具备完整三个操作族；路由行为和 Host 边界仍有待清理项，见第 9 节。

## 1. 本地调用与节点通信

### 本地应用调用

```text
Flutter / CLI
  -> GeneratedCoreProxy / LocalCoreProxy
  -> generated_core_dispatch
  -> ChatRuntimeHolder / ChatRuntimeSlot
  -> ChatServiceCore
```

本地 Proxy 入口进入本地 dispatch。声明了 Space 路由的运行时业务可以通过
`CoreRouteRuntime` 使用 `CoreNodeRouter`；应用侧不自行实现远程连接或配对协议。

### 节点间调用

```text
CoreNodeRouter
  -> RuntimePeerService / HostRuntimePeerService
  -> 已鉴权 Channel：标准 Link 消息的会话加密与关联
  -> PeerLink / PeerConnection
  -> Host transport API
  -> 远端 PeerConnection / Channel
  -> InboundSide / CoreLinkSession<RoutedClient>
  -> PeerRouter
  -> CoreNodeRouter 或 EdgePeerRouter
  -> 目标 Core 对象或 Edge Service
```

Router 负责目标选择、Space 权限和转发。Peer runtime 负责身份、配对、会话、连接
复用和响应关联。Transport 只负责连接、字节收发和 framing。

`PeerRouter` 是认证会话到执行后端的共同边界。完整 Core 使用 `CoreNodeRouter`，
设备 Edge 使用 `EdgePeerRouter`；两者使用同一个 Peer runtime，不另建 Edge 配对
流程。Edge Service 通过 Host API 获取设备能力。

`operit-proxy-edge` 提供类型化服务投影，地址由 `operit-edge-contract` 共享。
完整 CoreNode 拥有身份、Space、持久化及 Agent/Chat 运行时；Edge 提供设备服务及
相应的 Space 控制能力，不等同于完整 CoreNode。设备原生业务、配置与宿主插件的
职责见 [Edge 架构](edge-architecture.md)。

## 2. 标准 Link 的三个操作族

Transport 公共消息是 `PeerMessage`，其未加标签的序列化直接保留 Link envelope：

```text
PeerMessage::Request(CoreLinkRequest)
PeerMessage::Response(CoreLinkResponse)

CoreLinkRequest::Call(CoreCallRequest)
CoreLinkRequest::Watch(Snapshot | Open | Close)
CoreLinkRequest::Push(Open | Item | Close)

CoreLinkResponse::Call(CoreCallResponse)
CoreLinkResponse::Watch(Snapshot | Opened | Event | Closed，或错误)
CoreLinkResponse::Push(Opened | ItemAccepted | Closed，或错误)
```

`watchSnapshot` 属于 Watch，`openPush`、发送 item 和关闭属于 Push 的生命周期，
并不增加基础操作族。`CoreLinkSession` 统一执行三类请求并维护 Watch、Push 资源。

`CoreLinkClient` 和 `CoreLinkSharedClient` 的入口均为 `call`、`watchSnapshot`、
`watch`、`openPush`。`RuntimePeerService` 和 `PeerRouter` 都具备对应的节点通信入口。
共享客户端直接打开输入会话，无须复制客户端或取得可变所有权。

`SessionSpaceClient.openPush` 为输入流分配独立请求 ID，重校绑定的 Space 接入身份，
再通过同一已认证 Peer runtime 打开 Push。注解方法使用 `SpaceBinding` 路由；节点
服务输入方法使用 `Target` 路由，因此上传等服务保留自身标准对象地址和权限检查。
`NodeClient` 使用明确目标节点的相同 Push 路径。

Edge 应用可以通过原生端点声明 Watch / Stream 和 Push / ReverseStream，
当前固件与模拟器使用 `withStreamHandlers` 注册设备语音协议。没有注册对应输入端点时，
Push 返回标准 method-not-found 错误；不能从某个服务不支持 Push 推断 transport 不支持。

## 3. 配对和会话控制

配对与会话由 `HostRuntimePeerService` 实现，也使用标准 Link Call：

| 控制 target | 用途 |
| --- | --- |
| `$peer.pairing` | 身份握手和配对确认 |
| `$peer.session` / `data` | 承载加密后的标准 Link 消息 |
| `$peer.space-channel` | 认证配对会话中的 Space 通道管理 |
| `$peer.keepalive` / `ping` | 会话保活 |

会话先编码内层 `PeerMessage`，进行加密，再用 `$peer.session/data` Call 的二进制
args 发送。接收侧检查序号和认证标签，解密并恢复原始 Call、Watch、Push 消息。
因此，在原始 transport 上观察加密业务时看到的是会话 Call；业务语义来自解密后的
标准 Link 消息。加密逻辑属于 Peer runtime，不属于 transport。

匿名连接不能进入普通业务 dispatch。认证后每个业务入口仍重校授权。
配对授权方向、Space 成员准入、业务权限分别检查，单向配对不产生反向授权。

握手包含两个独立版本：

* `PeerStateStore::PAIRING_SERVICE_VERSION = 1` 是持久化凭据格式
  （`StoredInbound` / `StoredOutbound`），保持不变；已有配对继续有效。
* `Hello.pairingProtocol` / `HelloReply.pairingProtocol` 协商新配对流程，当前为 `2`。
  新建配对要求双方都支持 `2`；旧版本会收到 `PAIRING_PROTOCOL_UPGRADE_REQUIRED`，
  需要先升级，而不是静默退回旧确认方式。

`start` 阶段仍以 X25519 + transcript 绑定交换临时密钥并把 root 存入 pending。
`finish` 阶段不再发送可用 root 离线校验的 `HMAC(root, id||code)`，而是把六位配对码
作为 SPAKE2（Ed25519 group）口令：PAKE 消息通过 transcript 派生的身份串绑定到本次
交换，PAKE 输出再与 start root、finish transcript 一起 HKDF 出长期 session secret。
因此主动中间人即使分别与两端完成 X25519 交换，也必须通过完整在线交换逐次猜测配对码，
无法离线验证。确认次数同时受单个 pending 的 5 次上限和跨会话令牌桶（按来源与全局
两层）限制；`start` 也受创建速率限制，避免通过轮换会话放大猜测窗口。

HTTP 承载统一的 Link 消息。配对、认证和会话管理使用相同的 Call 消息入口，
由 Peer runtime 校验身份和权限。

## 4. Space 路由元数据

```text
RoutedCoreRequest<T> {
  spaceId
  originNodeId
  targetNodeId
  ttl
  routeKind: Target | SpaceRoute | SpaceBinding
  payload: CoreCallRequest | CoreWatchRequest | CorePushRequest
}
```

`peer/dispatch.rs` 在 Call、Watch、Push Open 的标准请求 args 中编码完整的
`RoutedCoreRequest`。`RoutedClient` 解码并校验请求关联，再交给 `PeerRouter`。
原始业务参数仍保留在内层 payload 的 args 中。

路由元数据只在创建操作时提交。Watch Close 使用 requestId；Push Item/Close 使用
pushId，已打开的流保持所属会话和路由。

`Target` 使用目标节点的本地对象地址；`SpaceRoute` 使用注解生成的 Space 地址；
`SpaceBinding` 由接入 Router 解析 Binding 后继续执行。逐跳保留 `originNodeId`、
最终 `targetNodeId` 和 `routeKind`，并处理 TTL、成员资格、撤销和中继权限。

`target` 是稳定字符串地址。本地 Proxy/SDK 使用 `core/<schema_key>`，Space route
使用 `space/<target_type>/<method>`；声明顺序不分配协议地址，改名是破坏性变更。

## 5. 五种 transport 的共同边界

`HostPeerLink` 按明确的 `PeerTransport` 选择 Host 适配器：

| Transport | Host I/O | 共用消息层 |
| --- | --- | --- |
| HTTP | `HttpHost` / `HttpServerHost` | `FramedPeerConnection` |
| WebSocket | Host WebSocket / HTTP server upgrade | `FramedPeerConnection` |
| TCP | `TcpHost` | `FramedPeerConnection` |
| Serial | `SerialPortHost` | `FramedPeerConnection::newSerial` |
| Bluetooth | `BluetoothHost` 的 Classic 连接能力 | `FramedPeerConnection` |

所有适配器实现内部 `ByteConnection`，统一使用 `encodeLink` / `decodeLink`。
普通字节流采用长度 framing；串口使用 OPS1 和 CRC framing。差别只在字节承载，
不改变 Call、Watch、Push 的业务消息。MessagePack 是产品 Link 的统一编码。

HTTP 在同一连接标识下用持续 GET 响应体接收字节、用 POST 发送字节；连接标识只
关联 HTTP 请求，不证明节点身份。WebSocket 同样收发统一 framing 的二进制数据。
监听能力由安装的 Host API 能力决定。

`PeerConnection` 对外提供 `send`、`receive`、`close`；`PeerLink` 提供 `connect`、
`listen`。三个基础操作是这些 I/O 接口承载的协议，不是五种 transport 各自实现
三套业务方法。

### 载体自动识别（配对拨号）

用户只输入 `ip:port` 时，`startPairing` 以 `transport = null` 表示"由地址自行判定"：

* 地址带 URL scheme 时直接映射载体（`http://`/`ws://`/`tcp://`），不做网络探测。
* 裸 `ip:port` 用一次一次性 TCP 探测连接分类（`operit_peer_link::detectTransport`）：
  发送一个 WebSocket 升级请求，按首个响应判定——`HTTP/… 101` 为 WebSocket、
  任何其他 `HTTP/…` 状态为 HTTP、无字节即断开（长度帧接收方丢弃了不可读的帧头）
  为 TCP。探测连接随即关闭，正式连接用判定出的载体重新建立。
* 判定后地址按载体规范化（HTTP/WS 补全 `scheme://…/link`），pending 配对记录保存
  规范化后的端点，后续重拨与观测地址刷新使用同一路径。
* 探测对目标无持久副作用：帧接收方会计一次被丢弃的坏帧；HTTP 服务方只看到一次
  未完成的升级请求。


## 6. 双向通道和流生命周期

握手双方声明 `duplex` 能力，Peer runtime 为具备能力的通道建立请求分发和响应关联。
Space 通道可以通过已认证连接向拨号侧发起请求。串口等独占载体还有
SessionReturn 机制，用同一物理连接承载认证后的返回方向。相关能力和授权由 runtime
管理，不由传输地址或客户端自报 nodeId 推导。

Watch 使用 `CoreRequestId`，Push 使用 pushId 和递增 sequence。`CoreLinkSession`
维护资源、顺序和关闭；Peer runtime 负责连接中的关联、取消及断线清理。
嵌入式 Watch 的重新绑定属于 Router 的流逻辑，不能把物理连接结束等同于所有
Router 对外逻辑流的业务完成。

## 7. 同步和设备工具

Binding 和持久化同步使用 `$node.sync` 的标准 Call 或 Push，经 Router 校验并交给
`peer/sync_dispatch.rs`。
同步要求相同 Space 的认证成员及相应 storage-provider 权限；同步 Push 在数据块和
最终提交时继续检查授权。

`NodeServices` 在节点内共享同一个 `RuntimePeerService`。Space 回连准入由
`space_policy::spaceChannelScope` 集中判断。`space_policy::RoutingPolicy` 从控制日志
读取断开节点和中继权限，公开路由计划及实际下一跳选择复用相同策略。

```text
Tools.device(nodeId).Files.read(path)
  -> DeviceToolRuntime
  -> CoreNodeRouter / Peer runtime / 中继 Router
  -> 目标 DeviceToolService / 工具注册表

Tools.device(nodeId).Edge.execute(...) / Io.execute(...)
  -> 同一个 DeviceToolRuntime 和路由入口
  -> 目标 Edge Service / Host capability
```

JS bridge 与硬件工具共用 `DeviceToolRuntime`。设备工具和硬件 target 不为每种
transport 建立独立连接或业务协议。

## 8. 架构约束

- Transport 只通过 Host API 执行 I/O，不持有配对、Space 策略或业务 dispatch。
- Peer runtime 只提交 Router 选定的下一跳，业务对象选择和业务授权属于 Router。
- 应用侧使用类型化 Proxy，不自行拼接远程路由或建立第二套节点通信协议。
- 目标不可达必须保持目标语义并明确报告错误，禁止静默改成本机执行。
- 禁止未经明确要求增加失败后替换执行路径或重放请求的回退逻辑。
- 平台相关能力和地址处理应由 Host 兼容层提供，不在上层复制平台实现。

## 9. 当前实现的待清理项

以下是源码现状，不是对新增行为的授权：

| 问题 | 源码位置 | 影响 |
| --- | --- | --- |
| 不可达 Binding 改成本机 | `CoreNodeRouter.rs::effectiveBinding` | 本次请求的有效执行节点被静默替换；不符合禁止回退的约束 |
| 部分路由失败后更换下一跳重发 | `route_forwarding.rs::forwardOperation` | 普通 target 可重提交；设备工具和硬件 target 被排除，仍需按禁止回退要求整改 |
| 串口地址规范化包含平台判断 | `peer-link/src/transport/serial.rs::endpointKey` | Windows 端口规范化仍位于 transport，Host 边界未完全收敛 |

Transport 消息协议和共享 Link 客户端均已覆盖 Call、Watch、Push。
Router 执行路径和 Host 边界仍需要上述代码层整改。
