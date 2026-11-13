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
