# 设备互联在 Tailscale 下的发现与连接分析

调查日期：2026-11-14（Asia/Shanghai）。
Operit 基线：当前工作区，HEAD `4901ab6e1`。
Tailscale 基线：官方仓库 `main` 的提交 `e0d532cca9beeb8ed235bf30b9198a7c0e0afa02`，提交时间为 2026-11-13 15:05:07 UTC。
该基线用于固定源码证据，不代表用户安装的客户端版本。

## 结论与证据边界

通过 Tailscale 跨物理局域网时，当前“扫描附近设备”的实现无法仅凭 Tailscale 建立的网络通路完成发现。
原因是 Operit 唯一的主动发现入口查询 mDNS 服务 `_operit-link._tcp.local.`，而 Tailscale 的普通隧道包过滤器明确丢弃组播目的地址。
这是可从源码直接确认的机制冲突；增加扫描时长、允许 UDP 5353 或更换扫描网卡，不能单独解决这个冲突。

连接层与发现层应分别判断。Operit 的 HTTP / WebSocket / TCP 连接入口没有拒绝 Tailscale IPv4 地址的规则，已经存在手动地址配对入口。
在系统正常提供 Tailscale 路由、目标监听可达且双方策略允许的前提下，手动指定 Tailscale IPv4 和实际监听端口，应当可以绕过 mDNS 建立连接。
这仍属于源码支持的推断，本次没有连接用户的 tailnet，也没有完成两台真实 Tailscale 设备之间的配对实测。
用户具体遇到的是“扫描不到”“已配对设备离线”还是“手动连接失败”，现有反馈不足以唯一确定。

## Tailscale 侧源码

以下链接均固定到同一提交：

- [wgengine/filter/filter.go](https://github.com/tailscale/tailscale/blob/e0d532cca9beeb8ed235bf30b9198a7c0e0afa02/wgengine/filter/filter.go#L443)：`RunIn()` 和 `RunOut()` 都先执行 `pre()`。
- [filter.go:703](https://github.com/tailscale/tailscale/blob/e0d532cca9beeb8ed235bf30b9198a7c0e0afa02/wgengine/filter/filter.go#L703)：`q.Dst.Addr().IsMulticast()` 时直接返回 `Drop` 和 `ReasonMulticast`。
- [net/tstun/wrap.go:816](https://github.com/tailscale/tailscale/blob/e0d532cca9beeb8ed235bf30b9198a7c0e0afa02/net/tstun/wrap.go#L816)：隧道出站路径调用该过滤器，非 Accept 时丢包。
- [net/tsaddr/tsaddr.go](https://github.com/tailscale/tailscale/blob/e0d532cca9beeb8ed235bf30b9198a7c0e0afa02/net/tsaddr/tsaddr.go#L29)：Tailscale IPv4 分配范围来自 `100.64.0.0/10`。
- [client/local/local.go:789](https://github.com/tailscale/tailscale/blob/e0d532cca9beeb8ed235bf30b9198a7c0e0afa02/client/local/local.go#L789)：本地客户端 API 有包含 peer 信息的 `Status()`，与 `StatusWithoutPeers()` 区分；状态通过 `/localapi/v0/status` 获取。

我们的 mDNS 目的地址为 IPv4 `224.0.0.251:5353` / IPv6 `[ff02::fb]:5353`，均属于上述组播判断范围。
这里不需要假设所有包一定进入 TUN：即使应用成功把 mDNS 包交给普通 Tailscale 隧道，该过滤规则也无法让它抵达远端。
Tailscale 设备之间能够单播访问，并不意味着它们共享本地 mDNS 的发现域。
如果两台设备还在同一物理局域网内，本地 mDNS 仍可能通过该局域网发现对方，这不能证明组播经过了 Tailscale。

## Operit 扫描调用链

1. `apps/flutter/app/lib/ui/common/DeviceSpaceDiscoveryPanel.dart:237`：`_scan()` 调用 `discoverPeers(timeoutMs: 5000)`。
2. `core/crates/node/runtime/src/HostRuntimePeerService.rs:1044`：交给 Host 执行 `discover("_operit-link._tcp.local.", timeoutMs)`。
3. `hosts/common/operit-host-native-common/src/ServiceDiscovery.rs`：Windows / Linux / Android / OHOS 的共享原生实现以 `mdns-sd` 浏览、注册和刷新服务。`advertise()` 调用 `enable_addr_auto()`。
4. `hosts/apple/src/service_discovery.rs`：macOS / iOS 通过系统 Bonjour 的 `DNSServiceBrowse`、SRV/TXT 解析及地址回调实现同一发现契约。
5. `HostRuntimePeerService.rs:1337`：读取 `nodeId`、`displayName`、`transports`，根据已解析地址和端口构造连接候选。

当前流程不枚举 Tailscale peer，不调用 Tailscale LocalAPI，也不主动探测 `100.x.x.x` 上的 Operit 服务。
Android 的 Wi-Fi multicast lock 只处理本机 Wi-Fi 组播接收资源，不能改变远端 Tailscale 的包过滤规则。
现有 `docs/device-space-lan-discovery.md` 也明确约定了 mDNS 和多播网络前提。

## 会导致“用不了”的几个独立环节

| 用户现象 | 当前实现及原因 | 证据性质 |
| --- | --- | --- |
| 两端 Tailscale 可达，但扫描列表为空 | 唯一发现入口依赖 mDNS；普通 Tailscale 隧道丢弃组播 | 源码可确认 |
| 在家里已配对，离开家后设备离线 | 重连候选为 mDNS 新地址加已保存 endpoint；没有从原来的 LAN IP 自动得到 Tailscale IP 的映射 | 源码可确认；具体用户情况需 endpoint 日志 |
| 之前手动连接成功，目标重启后失败 | 默认自动端口尝试复用旧端口；冲突时改为系统分配，跨 Tailscale 的 mDNS 无法发布新端口给另一端 | 条件性故障路径，源码可确认 |
| 输入 Tailscale 地址仍然失败 | 还需验证监听地址、实际端口、单播访问策略以及配对握手；不能由扫描结论直接解释 | 尚未实测 |

### 地址选择和重连

`core/crates/node/runtime/src/peer/discovery.rs:65` 的 `reconnectionEndpoints()` 先收集该 node 的已发现地址，然后补上保存的 endpoint。
没有新的发现记录时，只能重试已保存地址；仅开启 Tailscale 不会把 `192.168.*` 改写为 `100.*`。
成功握手并证明持有既有配对秘密后，才会保存成功使用的 endpoint 和 transport。

`HostRuntimePeerService.rs:1326` 的添加设备结果每个 node 只返回一个候选；排序依据地址族、协议和地址字符串，不探测候选可达性。
如果借助同一物理 LAN 或额外的组播转发获取了多个地址，也不能认为列表首选地址必然适合当前访问路径。
这个问题与纯 Tailscale 下根本收不到 mDNS 记录是两个阶段的问题。

### 监听和连接码

`PeerStateStore.rs:133` 的初始配置是 `0.0.0.0:0`，HTTP 和 WebSocket，自动端口且启用发现。
`HostRuntimePeerService.rs:1147` 的监听逻辑会保存实际绑定地址，后续尽量复用；端口冲突且满足自动模式条件时才退回端口 0。
因此不能假定所有安装都监听 37195，也不能声称每次重启都会换端口。

`PeerConnectionCode.rs:107` 的地址校验排除未指定、回环、组播和链路本地地址，没有排除 `100.64.0.0/10`。
`ServiceDiscovery.rs:627` 枚举系统网卡地址；`PeerConnectionCode.rs:115` 将与监听地址族匹配的可用地址编入连接码，最多八个。
系统能枚举出 Tailscale IPv4 且使用 IPv4 通配监听时，该地址有机会进入连接码；不同平台是否暴露虚拟网卡地址，需要真机核实。
默认监听是 IPv4，因此排障先用 Tailscale IPv4；不能直接假定默认配置同时接受 Tailscale IPv6。

## 目前可以采用的连接方法

在被连接端开启接收连接，确认它正在监听的协议和实际端口。
若要长期手动连接，可在现有监听设置中选择固定的未占用端口，例如 37195，绑定 `0.0.0.0` 或具体可达的本机 IPv4 地址。
固定端口是建议配置，不是当前产品保证的默认值。

在连接端从“添加设备”进入已有的手动连接入口，填写：

```text
http://100.x.y.z:实际端口/link
```

其中 `100.x.y.z` 必须替换成被连接端真实的 Tailscale IPv4。
使用显式协议和 `/link` 路径，可以避开裸地址协议探测和显式 URL 不补路径的问题。
双方随后完成现有配对码确认流程。已经配对的离线设备可以直接在其操作菜单中使用“更新直连地址”，填入同一形式的 Tailscale 地址，无需重新配对。该入口由 `RuntimeSettingsPanel.dart:483` 的离线判断提供，`RuntimeSettingsPanel.dart:1176` 调用 `updatePairedDeviceEndpoint()` 保存并触发探测。

若手动连接也失败，优先在发起端测试真实服务端口：

```sh
nc -vz 100.x.y.z 实际端口
```

端口不通，继续核对目标实际监听、Tailscale 访问策略和本机防火墙；端口通而配对失败，再看 `RuntimePeerService` 的握手错误。
Tailscale 网络连通性检查不能代替对 Operit 实际服务端口的验证。

## 建议修改方向

1. 近期：在扫描空结果、手动连接和监听配置处说明 mDNS 适用范围，显示当前可用地址及实际端口，并提供清晰的 VPN 地址连接说明。
2. 重连：每个已配对 node 支持持久保存多个候选 endpoint，例如 LAN 地址和 Tailscale 地址，按当前可达性尝试。地址更新继续通过现有配对身份验证。
3. 自动发现：增加可插拔的单播发现来源。桌面端可以评估 Tailscale LocalAPI 获取 peer 列表，再通过约定端口探测 Operit 服务；普通远程设备也可使用用户配置的地址或其他服务目录。必须同时解决 Operit 服务端口发现，只有 Tailscale peer IP 不够。
4. 端口：为跨网自动发现明确固定发现端口或服务目录中的端口声明，业务连接仍可使用协商得到的端口。
5. 移动端：单独验证可获得的 Tailscale 集成方式和系统接口，不将桌面 daemon 的本地 API 当作全平台前提。

不建议枚举整个 `100.64.0.0/10` 网段来替代发现，也不建议把增加 mDNS 扫描超时作为核心修复。

## 已执行验证

在 `core/` 中执行：

```sh
cargo test --locked -p operit-node-runtime --no-default-features --features peer-runtime --lib discovery
```

15 项通过，覆盖发现结果过滤、已配对设备排除、已认证地址更新、端口变化重连、多地址回退、后台刷新及订阅恢复等行为。
随后对同一次编译的测试程序运行 `PeerConnectionCode::tests`（4 项通过）和 `HostRuntimePeerService::pairing_transport_tests`（2 项通过）。
合计 21 项现有测试通过。

这些测试验证本地实现的既有行为；未运行 Tailscale 官方 Go 测试，未完成真实 tailnet 双机验证。
要最终归因到用户这一次故障，还需要其平台、是否扫描为空、手动地址结果、目标监听地址/端口，以及 `ServiceDiscovery` / `RuntimePeerService` 相关日志。


## 曾评估的专用方案（已撤下推荐，保留调查记录）

2026-11-14 后续约束：不适配 Tailscale 或其他 VPN 的专用 API。
下文保留为先前候选方案的调查记录，不作为实施方向。
当前设计参见 [通用 IP 网络设备发现](device-space-network-discovery-design.md)：统一单播发现入口，通用候选地址来源，以及按需引入的授权发现目录。

### 当时的候选：固定单播发现入口、候选设备目录、双向发现

将“找到对端的地址”与“得到 Operit 实际监听端口”分成两个阶段解决。
局域网继续使用已有 mDNS；VPN 路径通过相同的扫描界面汇总单播发现结果。
第一阶段面向电脑与电脑、电脑与手机，不要求用户手输 IP，也不需要另部署一个服务器。
这里的“无需服务器”仅适用于存在一个能读取 Tailscale 设备目录的 Operit 节点的情况。

1. 接收端在一个约定 TCP 端口提供小型发现服务，业务 HTTP/WS/TCP 端口继续保持现有配置。
   发现端口例如可评估 `37196`，这是待选择的产品协议常量，不是目前已有端口，也不是无冲突保证。
   该服务只在用户开启接收与发现时运行。端口占用必须报告，不能静默改成随机端口。
2. 桌面 Host 从可访问的 Tailscale 本地 API 获取可见 peer，或在已安装且可调用 CLI 的环境中使用 `tailscale status --json`。
   只向这些候选设备发起有界的单播探测，不扫描整个 `100.64.0.0/10`。
   不假设每台安装图形客户端的电脑都具有相同 IPC 路径、权限或可执行文件路径。
3. 对候选地址固定发现端口发出 `POST /operit-discovery/v1/hello`，正文包含协议版本、请求随机数、本机 Operit nodeId、显示名、正在运行的业务端口和 transports。
   对端回传同类信息，并回显请求随机数；不传配对密码或会话秘密。
4. 收到探测的节点也将探测者登记为候选。该次候选的连接 IP 取自 Host 提供的真实 socket 来源地址，业务端口来自发现信息。
   因此电脑探测手机时，手机也能发现电脑，不需要手机自行枚举所有 Tailscale peer。
   探测请求只有在发起者也有可用接收监听时，才作为可连接设备反向登记。
5. 将这些发现记录转成现有 `DiscoveredService`，复用 nodeId 去重、已配对设备过滤、配对确认与重连流程。
   单播记录只提供连接线索，实际设备身份仍由现有配对握手确认。

协议示意（设计草案）：

```text
电脑读取 Tailscale 设备目录
  -> 向手机:固定发现端口发送自己的 nodeId / 业务端口
  <- 手机回复自己的 nodeId / 业务端口

电脑的发现列表出现手机
手机的发现列表出现电脑

选择设备 -> 连接回复中的业务端口 -> 现有配对确认
```

这解决了“知道 IP 却不知道随机业务端口”的问题：固定的是轻量发现入口，而非所有业务连接端口。
也解决了“手机取不到设备目录”的常见电脑—手机场景：由能取目录的电脑发起双向握手。
该发现端口与业务端口都必须被实际网络路径和访问策略允许；实现不能绕过这些限制。

### 扫描时序与生命周期

桌面节点需要在发现功能开启时执行限频后台探测，并在可见设备变化时触发刷新；不能只在电脑打开扫描对话框时探测。
否则用户只在手机点击扫描，电脑可能完全不发起查询。

单播发现缓存应持有来源与过期时间，订阅需要支持到期移除。当前 `DiscoveredService` 没有 TTL 字段，运行时 `discoveryRecords` 也会保留旧记录，落地时需要补齐这一契约。
手机发现列表应能随订阅异步更新，不应将原有五秒 mDNS 快照结束视为所有 VPN 发现都结束。
反向发现的首次出现速度受桌面后台探测周期影响；不能在没有额外触发通道时承诺手机点击后五秒内必然发现。

每个 nodeId 应保留经过握手确认的多个候选地址，例如 LAN IPv4、Tailscale IPv4。
以后已配对设备可直接查询已知地址的固定发现端口，得到最新业务端口，再通过已有会话身份认证。
这可让已经建立联系的设备恢复连接，不必每次重新枚举 VPN 全部设备。
不能将发现消息里声称的 nodeId 直接视为已认证身份。

### 纯手机首次发现的边界

如果双方都是手机、从未建立过联系、彼此没有候选地址，也没有可访问的目录或发现中介，固定单播端口本身不提供“往哪个 IP 发包”的答案。
要保证该场景也能自动发现，需要新增至少一种共同可达的候选来源，例如经过授权的设备发现目录，或可供本应用使用的 Tailscale 集成。
这里不假定产品已有账号体系，也不把公开 tailnet 名称当作授权凭据。
一个已经被自动发现的桌面节点，也可以进一步充当已知候选的介绍节点；手机拿到线索后仍应直接探测目标并完成自身认证。

核对到的 Android 官方客户端实现中，LocalAPI 通过 Go/Kotlin 应用内调用进入 `localAPIHandler.ServeHTTP()`，该实现不能证明第三方应用可像桌面一样读取它。
Manifest 没有提供一个明确用于向普通第三方应用返回 peer 列表的接口。
所以不能把“在 Android 调用桌面的 localapi socket”当作现成解决方案。
iOS 第三方访问能力在本次调查中尚未核实。

新增官方源码证据：

- [Tailscale CLI status.go](https://github.com/tailscale/tailscale/blob/e0d532cca9beeb8ed235bf30b9198a7c0e0afa02/cmd/tailscale/cli/status.go#L49)：存在 `--json`；源码也警告 JSON 格式可能变化，实现只依赖所需字段。
- [ipn/ipnstate/ipnstate.go](https://github.com/tailscale/tailscale/blob/e0d532cca9beeb8ed235bf30b9198a7c0e0afa02/ipn/ipnstate/ipnstate.go#L80)：状态有 peer 表，peer 包含 TailscaleIPs、DNSName 等信息；它是网络设备目录，不直接提供 Operit 的随机监听端口。
- [Android libtailscale/localapi.go](https://github.com/tailscale/tailscale-android/blob/340e5567441f06d446cc3489deba780930c141ed/libtailscale/localapi.go#L132)：应用内部处理 LocalAPI 请求。
- [AndroidManifest.xml](https://github.com/tailscale/tailscale-android/blob/340e5567441f06d446cc3489deba780930c141ed/android/src/main/AndroidManifest.xml)：检查其导出组件，未找到明确的第三方 peer 列表接口。Android 源码固定提交时间为 2026-11-13 18:00:32 UTC。

### 与现有代码的接合点

- `ServiceDiscoveryHost`：组合局域网与单播 provider，对 Core 保持统一的浏览、发布和订阅入口；平台专属设备目录读取由 Host 负责。
- `ServiceAdvertisement`：使用当前运行时已提供的实际业务端口和 TXT 元数据，避免另存一份会过期的端口配置；绑定范围需与接收端设置保持一致。
- `HttpServerHost`：现有契约提供 HTTP 监听与 `RemoteAddress`，可供轻量发现服务使用，不需要让 Core 自己创建原生 socket。
- `HostRuntimePeerService`：复用现有候选过滤及认证；增加来源、过期处理和持久化多地址能力。
- `DeviceSpaceDiscoveryPanel`：统一呈现 mDNS、VPN 单播及已知地址查询的结果，并支持异步到达。

该专用候选方案已经撤下推荐，当前实施方向以通用 IP 网络设备发现设计为准。
这部分是已核对代码接合点的设计方案，尚未实现或经过 tailnet 真机验证。


## 实施状态

后续已实现不依赖 VPN 品牌的通用目录发现逻辑，替换了早期实现中的已知地址固定端口探测和地址历史旁路，使用和限制见 [通用 IP 网络设备发现：实现与使用](device-space-network-discovery-design.md)。上文的专用 VPN 候选设计仍为撤回的研究方案，不代表当前实现。
