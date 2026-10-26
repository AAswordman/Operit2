# Space CLI 多进程压力测试

运行器：`space_stress_session.py`。启动六个真实 CLI 进程，审批测试临时增加第七个；使用真实 TCP、HTTP、WebSocket 连接，不替换路由器或直接修改业务数据库。每个进程隔离身份、配置和存储，密钥使用测试目录中的文件存储。CLI 在三个桌面平台均按显式配置与当前身份隔离 IPC 实例；运行器不再修改每个进程的 `TMPDIR`。

## 复现

在仓库根目录执行：

```sh
cargo build --locked --manifest-path apps/cli/Cargo.toml --bin operit2 --target-dir apps/cli/target
python3 apps/cli/tests/space_stress_session.py --transport tcp --rounds 3 --operations 30 --stall-seconds 25
python3 apps/cli/tests/space_stress_session.py --transport mixed --replicas --rounds 3 --operations 30 --stall-seconds 25
```

运行器首行输出结果目录；目录包含 `report.json`、`events.jsonl`、运行器快照和每个节点的完整命令、响应、stderr 与持久化数据。任何场景失败都会保留失败结果并以非零状态退出，即便后续恢复成功。

## 覆盖

- 分别建立 A—B—C、D—E—F 两个空间，再经 C—D 合并；检查成员、管理员归属及两端真实五跳路由。
- 六端并发写偏好、读取空间和设备列表；检查没有自动增加捷径配对。
- 使用每次全新的标记验证 A、F 双向偏好同步。
- 三轮暂停中继 C 25 秒、强杀、分区离线检测、离线写入、同身份同端口重启与补传。
- 第七端 G 经 F 提交六跳审批、撤回，并验证过期审批不能生效。
- F 退出本机空间后经原配对重新审批加入。按既有语义，`space leave` 不自动清除其他节点上的成员记录，也不删除配对。
- C 重启更换端口、IPv4 切到 IPv6、再切回；每次恢复后验证全新数据双向传递。
- 六端全部强杀，按相反顺序重启，验证空间及数据恢复。
- 补充 F—A 直连形成环；杀掉 C 后，验证 A 经 F—E—D 绕行并传送新配置。
- `--replicas` 为节点授予业务存储能力，额外验证聊天记录跨五跳同步及远程 watch。

## 边界

此测试在同一宿主机回环接口上运行。进程暂停/强杀模拟中继无响应/离线；换端口和 IPv4/IPv6 覆盖监听地址变化，未测试真实 Wi-Fi 切换、跨物理网段、NAT、公网丢包或 ESP32 资源限制。延迟统计是 CLI 命令响应时间，不是纯网络吞吐或消息投递延迟。聊天场景验证同步与路由，不调用模型。

## 压测发现及修复

1. 空间合并后，存活连接只按配对 ID 缓存回连授权发送状态，导致旧空间授权不再更新。改为按配对 ID 与空间 ID 缓存，空间变化后重新发布当前范围的授权。
2. SQLite 变更记录触发器使用 `INSERT OR REPLACE`，会被外层 UPSERT 的冲突策略覆盖，重复更新时报 `_operit_dirty` 主键冲突并阻断业务同步。改为显式 UPSERT；打开旧库时升级已有触发器。真实 SQLite 回归测试覆盖同事务重复更新、旧库升级与同步重放。
3. 并发 CLI 共用插件 IPC socket 时出现 `Address already in use`。macOS 显式 CLI 配置现已使用独立 IPC 目录；Unix socket 使用文件锁保持所有权，重复启动不能删除已有服务，强杀后可清理残留 socket。
4. 回连通告原来只携带端口，接收端使用出站来源 IP 拼接回连地址。现在通告增加可选的监听 IP，具体绑定地址直接使用，通配绑定和旧通告仍使用观察到的来源 IP；空间、配对和撤销检查继续生效。
5. macOS 接收的 IPC socket 可能继承非阻塞状态，导致阻塞帧读取器在数据到达前退出。接收时显式恢复阻塞模式。
6. 关闭 IPC 会话仅丢弃写端克隆，读线程仍持有 socket，空闲客户端收不到断开通知。写端释放时执行双向 shutdown，唤醒本端及对端读线程。

7. 环形直连建立期间，旧五跳路径与新直连同时承载同步请求。连接串行等待每个 Call，形成跨连接等待环；心跳正常，业务却不再返回。普通入站连接与双向连接共用分发循环，独立 Call 并发执行，Watch/Push 会话继续按连接顺序处理。流会话在同一宿主任务内独立推进，打开 Watch 或等待 Push 时仍能执行 Call。两项稳定回归测试分别覆盖转发 Call、打开 Watch 等待后续 Call 的情况，旧实现超时、修复后通过。

8. 多个来源可使用相同的 Call 编号，中继原样复用时会覆盖通道中的待响应项。每一跳现在分配本进程唯一编号，返回时恢复原编号；底层对仍在执行的重复编号直接报错，保留原请求。真实 TCP 回归测试并发发送相同编号的请求，旧实现稳定失败，修复后两次调用均成功。

## 2026-10-15 修复前的实测结果

修复生产问题后，使用最终修正的测试断言得到：

| 运行 | 场景通过数 | CLI 命令数 | 时长 | 命令 p95 |
| --- | ---: | ---: | ---: | ---: |
| 混合协议完整流程 | 26/26 | 5455 | 42.36 s | 9.88 ms |
| 纯 TCP 生命周期补测 | 12/13 | 1366 | 81.48 s | 9.38 ms |
| 混合协议地址切换与绕行专项 | 12/12 | 1122 | 23.5 s | 9.86 ms |

对应命令：

```sh
python3 apps/cli/tests/space_stress_session.py --transport mixed --replicas --rounds 3 --operations 30 --stall-seconds 3
python3 apps/cli/tests/space_stress_session.py --transport tcp --focus lifecycle --operations 10
python3 apps/cli/tests/space_stress_session.py --transport mixed --focus migration --replicas --operations 10
```

此前纯 TCP 与混合协议均另跑过三轮 25 秒暂停、强杀、离线检测和补传，以上故障场景全部通过。纯 TCP 完整压力组执行 5,002 条命令后在 IPv6 切换失败；混合协议的 25 秒组执行 5,725 条命令，末尾环形测试使用了后续已修正的直连断言，原报告仍保留失败状态。

测试脚本曾误把直连跳数要求为 `1`，而接口用空值表示直连，导致环形场景在杀掉 C 之前就超时。修正后分别验证路径收敛与新配置送达，两项均通过。另纠正了 `space leave` 的断言，以及失败后不应重复配对的测试恢复逻辑。这些脚本问题不计为产品故障。

**当时尚未解决的产品失败：纯 TCP 中继从 IPv4 改绑 IPv6 后，拓扑显示在线，但 A 写给 F 的全新偏好在 45 秒内未送达。多次复现；切回 IPv4、全体重启及环形绕行正常。** 日志出现拨号 `127.0.0.1:新端口` 被拒绝，而 C 实际监听 `[::1]:新端口`。`peer/space_channel.rs` 的 `acceptSpaceChannel` 通过连接来源 IP 与公告端口拼接回连地址，此处没有表达“出站来源地址与监听地址属于不同地址族”的情况。随后已按上文修复；修复后的结果见下节。

该批运行仍通过每进程临时目录隔离 IPC。后续复测已移除此测试隔离措施，直接验证 CLI 的配置隔离。

存储集成测试 **19/19**、运行时测试 **199/199** 通过；CLI 构建通过。所有最终测试报告的 `cleanup_errors` 均为空。

本机原始证据（临时目录可能由系统日后清理）：

- 混合协议完整流程：`/tmp/operit-space-stress-qci4bve5/report.json`
- 纯 TCP 生命周期补测：`/tmp/operit-space-stress-tyb62lxc/report.json`
- 混合协议地址切换与绕行专项：`/tmp/operit-space-stress-dda3l9_a/report.json`
- 25 秒纯 TCP 完整组：`/tmp/operit-space-stress-s0xace9e/report.json`
- 25 秒混合协议完整组：`/tmp/operit-space-stress-j9_xem9v/report.json`
- Rust 测试日志：`/tmp/operit-space-stress-store-tests.log`、`/tmp/operit-space-stress-runtime-tests.log`

## 修复后的 IPC 使用方式

CLI 使用非空 `OPERIT_CLI_CONFIG_DIR` 时，以当前身份 runtime 根路径的 SHA-256 前 16 个十六进制字符作为 IPC 实例标识。同一配置及身份重启后标识保持一致。Windows、Linux、macOS 共用这一选择规则，宿主将实例标识编码为命名管道地址、D-Bus 服务名或临时 socket 目录。不指定 CLI 配置时使用标准 SDK 端点。

第三方 SDK 需要连接指定实例时，为 CLI 和 SDK 客户端同时指定相同的 `OPERIT_PLUGIN_SDK_INSTANCE`，它优先于自动实例选择：

```sh
OPERIT_CLI_CONFIG_DIR=/path/to/profile OPERIT_PLUGIN_SDK_INSTANCE=test-a operit2 cli link session tcp --bind 127.0.0.1:37195
# SDK 客户端进程也设置 OPERIT_PLUGIN_SDK_INSTANCE=test-a
```

显式实例需先启动 CLI。Apple 宿主保留 `OPERIT_PLUGIN_SDK_SOCKET_DIR` 作为底层目录覆盖选项；端点文件锁一直持有到监听和连接清理完成，进程崩溃后操作系统释放锁，下次启动可清理残留 socket。

运行器会清除继承的两种 IPC 覆盖变量，验证 CLI 自身的自动实例隔离。本运行器使用 Unix 进程信号；下文多进程压力结果来自 macOS，不能视为 Linux D-Bus 或 Windows named pipe 的运行验证。


## 2026-10-15 最终修复后的实测结果

以下运行使用最新 CLI，共用系统临时目录，直接验证生产代码中的配置与身份隔离。TCP 与混合协议完整组同时运行，每组六个 CLI，审批场景临时增加第七个。完整组各暂停中继三轮，每轮 25 秒。

| 运行 | 场景通过数 | CLI 命令数 | 时长 | 命令 p95 |
| --- | ---: | ---: | ---: | ---: |
| 纯 TCP 完整组 | 25/25 | 5403 | 105.16 s | 8.93 ms |
| 混合协议完整组（含聊天与 Watch） | 26/26 | 5471 | 117.74 s | 10.43 ms |
| 混合协议地址迁移与绕行专项 | 12/12 | 1793 | 21.17 s | 13.53 ms |

三组均通过，`cleanup_errors` 均为空。完整组覆盖五跳合并、并发配置、暂停、强杀、分区检测、离线补传、六跳审批撤回、退出重入、换端口、IPv4/IPv6 往返、全部强杀后逆序重启及环形绕行。混合协议组还验证五跳聊天同步与远程 Watch；每次迁移及绕行都检查新写入的数据送达。

最终运行命令为本文开头的两条完整组命令，以及：

```sh
python3 apps/cli/tests/space_stress_session.py --transport mixed --replicas --focus migration --operations 20
```

修复期间没有覆盖失败报告：

- IPv4/IPv6 回连修复后，混合协议仍曾在新增 F—A 直连时卡住：`/tmp/operit-space-stress-s_sniwi1/report.json`。
- 加入临时请求追踪后再次抓到等待环：`/tmp/operit-space-stress-wjy1zh57/report.json`；对应请求等待链保存在该目录的 `deadlock-trace.txt`。临时追踪代码已移除。
- 独立 Call 并发后继续发现编号冲突，以及打开 Watch 时阻塞后续 Call。中间失败分别保留在 `/tmp/operit-space-stress-f2i0difm/report.json`、`/tmp/operit-space-stress-69pve2s1/report.json`。
- 转发等待、Watch 等待和重复 Call 编号均补充了先失败、后通过的回归验证。最新完整组通过发生在这些问题全部修复之后。

最终验证：运行时 **204/204**、Edge **39/39**、存储集成 **19/19**、Apple IPC **7/7**、独立 SDK 宿主 IPC **7/7**。CLI 构建、Python 语法检查和 `git diff HEAD --check` 通过。Edge 测试覆盖共享会话与 Core—Edge 配对审批，不代表真实 ESP32 的内存或物理网络测试。

最终报告（临时目录可能由系统日后清理）：
- 纯 TCP 完整组：`/tmp/operit-space-stress-6n6atfup/report.json`
- 混合协议完整组（含聊天与 Watch）：`/tmp/operit-space-stress-2l2yf7m9/report.json`
- 混合协议地址迁移与绕行专项：`/tmp/operit-space-stress-581lyg7e/report.json`
- 运行时：`/tmp/operit-space-fix-runtime-tests.log`
- Edge：`/tmp/operit-space-fix-edge-tests.log`
- Apple IPC：`/tmp/operit-space-fix-ipc-tests.log`
- 独立 SDK 宿主：`/tmp/operit-space-fix-sdk-host-tests.log`
- CLI 构建：`/tmp/operit-space-fix-cli-build.log`
- 首次死锁回归失败：`/tmp/operit-space-deadlock-before.log`
- Watch 等待回归失败：`/tmp/operit-space-watch-before.log`
- 相同编号并发 Call 回归：`/tmp/operit-space-correlation-before.log`、`/tmp/operit-space-correlation-after.log`

## 平台判断清理后的回归

`OPERIT_CLI_SECRET_STORE=file` 的选择与 IPC 实例隔离已移除 CLI 层的平台判断；文件密钥实现移至公共 native storage，覆盖写入复用现有原子替换接口。

- CLI `cargo build --locked` 通过。
- 公共 IPC 1 项、storage 9 项、Apple IPC 7 项、外部 SDK IPC 7 项测试通过，覆盖文件密钥覆盖与重开、实例间收发隔离、监听占用及关闭。
- SDK 宿主及其测试分别通过 `x86_64-unknown-linux-gnu`、`x86_64-pc-windows-gnu` 的 `cargo check --tests`。这仅验证 IPC 代码与类型，不是两个平台的完整 CLI 构建或运行验证。
- 再次运行混合协议迁移专项：12/12 场景、1,807 条 CLI 命令，25.12 秒，命令 p95 9.67 ms；覆盖五跳聊天与 Watch、端口及 IPv4/IPv6 迁移、全部强杀后逆序重启和环路绕行。`cleanup_errors` 为空。
- 本轮报告：`/tmp/operit-space-stress-alu5rwsn/report.json`；日志：`/tmp/operit-cleanup-space-mixed-migration.log`。

## 真 CLI：系统禁止反向建连与并发撤回

这组运行直接启动仓库构建的 `operit2`，经 stdin/stdout 发 CLI 命令，使用真实 TCP/HTTP/WebSocket 和持久化目录。`--focus directed` 使用 macOS `sandbox-exec` 限制每个 CLI 进程的 TCP/UDP 出站连接：

```text
F -> E -> D -> C -> B -> A
G/P*/R* -> F
```

箭头只表示主动建连方向。A 不允许发起 TCP 连接，其他设备只能连接指定邻居的 TCP 端口；响应与回程 RPC 必须复用已有连接。UDP 被禁止，发现关闭。每份进程策略还用真实 TCP 探针验证：允许端口可连接，禁止端口返回系统 EPERM。限制仅作用于这些子进程，不修改宿主防火墙或网络配置。该测试模式只在 macOS 可用，产品代码没有为测试增加网络开关。

### 这次真实 CLI 测出的故障

1. 原有配对连接只处理正向请求，回程仅在另外建立 Space 连接时可用。禁止反向建连后，C 经 B 申请加入，A 始终拉不到审批记录。首次失败报告保留在 `/tmp/operit-space-stress-2ul7zxbz/report.json`。
2. 补齐配对连接双向派发后，负载重跑发现了入空间时序问题：网关先获知批准、申请人还没完成刷新时，提前到来的回程请求会触发鉴权关闭，打断原来的加入刷新。

修复复用现有连接与派发器：配对凭证失效仍关闭连接；空间尚未就绪的请求由路由层拒绝，保留有效连接和其他进行中的请求。回程通告移到接收循环启动之后；当前空间成员资格、原始配对和撤销规则继续由既有入口检查。

### 修复后的最终结果

| 运行 | 场景 | CLI 命令 | 时长 | 命令 p95 | 报告 |
| --- | ---: | ---: | ---: | ---: | --- |
| 严格单向建连，混合 TCP/HTTP/WS | 17/17 | 2686 | 122.74 s | 37.48 ms | `/tmp/operit-space-stress-1u_t0kh4/report.json` |
| 严格单向建连，纯 TCP | 15/15 | 1638 | 26.85 s | 19.30 ms | `/tmp/operit-space-stress-gvjx5xu7/report.json` |
| 混合协议地址迁移与绕行回归 | 12/12 | 2412 | 76.19 s | 9.41 ms | `/tmp/operit-space-stress-y_4_yzcb/report.json` |

共 44/44 场景、6736 条 CLI 命令通过。三组最终报告的 `cleanup_errors` 都为空，所有 CLI 子进程已回收。严格单向混合组同时运行两类撤回竞态；其单组峰值为 14 个真实 CLI（基础链六端，加八个同时申请的端）。

覆盖五跳成员与权限收敛、双向配置载荷同步、聊天创建同步、远程 Watch、六跳撤回、重复撤回与重新申请、申请人强杀重启、网关离线时撤回并重启申请人、审批人被中继故障隔开时撤回、离线配置补传，以及整链强杀后倒序重启。

撤回与审批竞争测试使用真实并发命令和不同发送间隔。混合组八轮得到四次 `cancelled`、四次 `joined`；TCP 组六轮得到五次 `cancelled`、一次 `joined`。撤回先成功时，旧审批被拒绝，申请人保持独立空间；审批进入提交并成功完成时，最终状态为 `joined`，撤回不回滚已提交的加入。随后混合组八个、TCP组六个申请人同时申请、撤回与审批，最终均为取消；每个申请人都强杀重启，再确认终态与成员关系保持一致。

复现最终三组：

```sh
cargo build --locked --manifest-path apps/cli/Cargo.toml
python3 apps/cli/tests/space_stress_session.py --focus directed --transport mixed --replicas --rounds 8 --operations 100
python3 apps/cli/tests/space_stress_session.py --focus directed --transport tcp --replicas --rounds 6 --operations 60
python3 apps/cli/tests/space_stress_session.py --transport mixed --replicas --focus migration --operations 20
```

同一版 runtime 的完整库回归 204/204 通过。CLI 场景运行于 macOS 的本机真实网络连接；没有验证实际 Wi-Fi/蜂窝网络切换、真实 NAT 行为、跨机器链路或 ESP32 实机资源限制，不能据此保证所有网络与版本组合。


## 单向建连闭环补测（2026-10-15）

新增 `--focus directed-ring`，复用上述 macOS 进程网络限制。先以 `F -> E -> D -> C -> B -> A` 完成入空间和五跳双向同步，再仅由 A 主动配对 F，闭合成 `A -> F -> E -> D -> C -> B -> A`。六端端口预先分配，系统仅允许每个进程主动连接箭头所指邻居；每端策略均经允许/禁止端口探针验证。箭头代表建连方向，已建立的连接仍可双向传输。

覆盖全部 30 个有序节点对的最短路、六端同时写入后每端观察全部标记、并发读写压力、C 进程强杀后的 A/F/E/D 双向绕行、C 重启补齐离线配置，以及全部强杀后倒序重启并再次全端同步。强杀保留空间成员资格；本轮没有执行成员退出测试。

| 运行 | 场景 | CLI 命令 | 时长 | 报告 |
| --- | ---: | ---: | ---: | --- |
| TCP/HTTP/WS 混合闭环 | 10/10 | 8010 | 49.31 s | `/tmp/operit-space-stress-ftchakoj/report.json` |
| 纯 TCP 闭环 | 10/10 | 6518 | 21.62 s | `/tmp/operit-space-stress-476hshyh/report.json` |

两组共 20/20 场景、14528 条命令通过，清理错误均为空。第一组旧版报告的 `topology` 字段仍写作 `chain`，实际 `focus=directed-ring`，事件和六条配对记录可确认闭环；runner 已将该字段修正为 `ring`。

混合协议组第三轮配置传播花费 25.462 秒。B 日志记录一次 `PEER_HEARTBEAT_TIMEOUT`，相关同步调用耗时 24597 ms；恢复后全部配置收敛。超时的根因尚未定位，不能将场景最终通过解释为始终无卡顿或无瞬时故障。纯 TCP 组三轮并发同步各用时约 1.2–1.5 秒。

纯 TCP 首次尝试在入空间时过早断言了审批人：待处理请求可暂时没有 `reviewerDeviceId`，报告保留于 `/tmp/operit-space-stress-xycdlwlw/report.json`。测试现通过真实 CLI 刷新等待审批人分配后再批准，仍验证批准前不合并成员，未更改产品行为。

```sh
cargo build --locked --manifest-path apps/cli/Cargo.toml
python3 apps/cli/tests/space_stress_session.py --focus directed-ring --transport mixed --replicas --rounds 3 --operations 60
python3 apps/cli/tests/space_stress_session.py --focus directed-ring --transport tcp --replicas --rounds 3 --operations 60
```


## 中继节点主动退出与断链恢复（2026-10-15）

原链为 `A — B — C — D`，四端属于空间 S。C 主动退出后创建自己的独立空间；A、B、D 保留 S。B、D 收到 C 的退出记录后删除 C 的成员资格和相关路由边，B 将记录继续同步给 A。此时 A/B 的图为 `A — B` 加不可达的 D，D 的图保留 A/B 成员但无法到达它们；没有自动创建 B—D 连接，也没有把 D 迁移到新空间。退出记录到达前，离线端暂时持有旧视图。

实现复用现有配对 Link、`node.space` 快照交换和控制日志。设备只能直接声明自己退出，其他节点通过原空间控制日志转发这一事实。退出先持久化自身 `LeaveSpace` 与待通知邻居，再切换本地空间；不等待邻居在线。待发记录跨重启保留，只有邻居返回完全相同的持久化控制操作才确认送达。自愿退出保留配对，既用于离线补发，也允许之后重新申请加入。重新入空间后的历史退出记录不能再次移除设备。记录退出后、本地空间切换前崩溃，由现有同步循环完成恢复，无新增传输协议或后台调度器。

轻量端同样使用该流程：即使双方都没有 Core 执行运行时，已有配置同步循环遇到空间不匹配时，也会通过配对快照取得并确认退出记录。退出时提前清理旧审批缓存，为持久化退出记录腾出空间，保持既有 7000 字节逻辑 NVS 限额。Core 请求加入已退出 Edge 的新空间时，会先收到 Edge 离开旧空间的事实；取消这次申请应保留已收到的退出事实，不能恢复 Edge 的旧成员资格。

真实 CLI 新增 `--focus departure`，操作系统限制初始主动建连方向为 `D -> C -> B -> A`；箭头仅表示拨号方向，建立的连接可双向传输。七个场景覆盖：系统网络限制探针、四端三跳入空间、在线退出与拓扑清理、重新加入后旧记录不生效、两邻居离线及退出端两次重启补发、分区保留空间与独立写配置、D 改连 B 后不重新申请即恢复双向同步。开启 `--replicas` 时，还验证分区两侧创建的聊天元数据在恢复后互相同步，不涉及模型对话。

| 运行 | 场景 | CLI 命令 | 时长 | 报告 |
| --- | ---: | ---: | ---: | --- |
| TCP 退出与重连 | 7/7 | 266 | 28.79 s | `/tmp/operit-space-stress-fcav_4y2/report.json` |
| TCP/HTTP/WS 混合退出与重连 | 7/7 | 226 | 16.31 s | `/tmp/operit-space-stress-5767wsmd/report.json` |
| TCP 退出与聊天元数据同步 | 7/7 | 219 | 11.19 s | `/tmp/operit-space-stress-tkbpc_ps/report.json` |
| 混合协议单向闭环回归 | 9/9 | 3485 | 50.20 s | `/tmp/operit-space-stress-fui4o05c/report.json` |

共 30/30 场景、4196 条真实 CLI 命令通过。四份报告的 `cleanup_errors` 均为空，逐一检查启动事件所记子进程 PID，均已退出。报告目录保留 `online-exit-{A,B,D}-topology.json`、`offline-retried-exit-{A,B,D}-topology.json` 和 `bridged-{A,B,D}-topology.json`，均来自新增只读 `space topology` 命令。

```sh
cargo build --locked --manifest-path apps/cli/Cargo.toml
python3 apps/cli/tests/space_stress_session.py --focus departure --transport tcp
python3 apps/cli/tests/space_stress_session.py --focus departure --transport mixed
python3 apps/cli/tests/space_stress_session.py --focus departure --transport tcp --replicas
python3 apps/cli/tests/space_stress_session.py --focus directed-ring --transport mixed --replicas --rounds 2 --operations 30
cargo test --locked --manifest-path core/Cargo.toml -p operit-node-runtime --lib
cargo test --locked --manifest-path core/Cargo.toml -p operit-node-edge --lib -- --test-threads=1
```

Runtime 完整库测试 208/208 通过，包括退出鉴权、多跳传播、精确确认、重启恢复、最后管理员退出不自动提权等回归。日志为 `/tmp/operit-departure-runtime-tests-final.log`。 Edge 完整串行库回归 40/40 通过，日志为 `/tmp/operit-departure-edge-tests-complete.log`；另运行 `cargo check --locked --manifest-path core/Cargo.toml -p operit-node-edge --lib`，确认未启用完整 Core 的轻量端库也编译通过。

中间失败记录保留：`/tmp/operit-departure-edge-tests.log` 记录退出分配超出 NVS 和一次监听器重启后等待连接超时；NVS 问题通过提前清理旧审批缓存修复，连接等待用例在后续完整串行运行通过。`/tmp/operit-departure-edge-tests-final.log` 中另一个失败是取消重入申请测试仍预期 Edge 留在旧空间，现改为验证收到退出记录后的成员视图，并继续严格验证取消意图跨双方重启保存。

本轮是本机真实 CLI 网络与轻量端 TCP 测试，没有验证 ESP32 实机、真实 NAT 或跨机器网络。前述闭环心跳超时根因仍未解决；最终闭环回归的故障绕行恢复场景用时 27.514 秒，不能把最终收敛理解为始终无卡顿或保证固定恢复时间。


## 同一空间增加通道的严格回归（2026-10-15）

补强 `--focus departure`：C 退出后，D 先与 B 配对恢复两跳路由，再保留 B 通道、增加与 A 的直连。增加通道前后逐端比较空间 ID、名称、成员、完整控制状态、入站/出站申请和管理员控制日志，要求完全一致，没有新申请、审批、退出或重新入空间记录。空间投影的版本号允许追平：首次严格比较时 D 从 revision 9 补齐到 10，但成员与所有控制记录均未变化，因此版本号不作为成员变化的判据。

新增第八个场景关闭 B 并强杀重启 D，确认 A/D 的直连仍有效，配置继续双向同步；恢复 B 后再次验证成员身份、申请及控制日志没有变化。本轮没有发现同空间配对被拒绝或自动触发入空间的问题，只修改 CLI 回归脚本，没有新增产品逻辑。`space join` 用于申请成员资格，同空间增加传输路径只执行配对。

| 运行 | 场景 | CLI 命令 | 时长 | 报告 |
| --- | ---: | ---: | ---: | --- |
| TCP | 8/8 | 330 | 22.35 s | `/tmp/operit-space-stress-acrlhxf4/report.json` |
| TCP/HTTP/WS 混合 | 8/8 | 295 | 14.23 s | `/tmp/operit-space-stress-xrs24nk2/report.json` |
| 混合协议及聊天元数据同步 | 8/8 | 309 | 14.78 s | `/tmp/operit-space-stress-8lk8s4ar/report.json` |

共 24/24 场景、934 条真实 CLI 命令通过，三个报告均无清理错误。新增证据文件为 `channel-membership-before.json`、`bridged-membership-after.json`、`direct-membership-after.json` 和 `direct-restarted-membership-after.json`。

失败尝试同样保留：`/tmp/operit-space-stress-et0wqf4k/report.json` 的断言只差上述投影版本号；`/tmp/operit-space-stress-3zh72gmf/report.json` 在初始聊天创建阶段遇到 `Character records changed during the read; operation was not started`，尚未执行新增通道场景。后者原样重跑通过，未据此认定插件初始化竞态已解决，也未修改聊天插件逻辑。

```sh
python3 apps/cli/tests/space_stress_session.py --focus departure --transport tcp
python3 apps/cli/tests/space_stress_session.py --focus departure --transport mixed
python3 apps/cli/tests/space_stress_session.py --focus departure --transport mixed --replicas
```

离线设备单方面清除配对、退出且不再联网时，原空间无法收到它的本地退出事实。现有管理员 `space remove <device>` 可由原空间提交成员移除记录，不依赖被移除端在线；记录经仍有效的路径同步，其他被隔离的正常成员继续属于原空间。自动离线清理策略本轮仅讨论，未实现，也没有把断网或路由失效直接转为成员退出。


## 无通知离开后的管理员移除（2026-10-15）

新增 `--focus silent-removal`，用真实 CLI 验证用户指定顺序：先形成 `D -> C -> B -> A` 单向拨号链；强杀 C 后以禁止所有主动网络连接的进程策略重启，清除 C 的两份配对，再执行本地退出并关闭 C。没有退出通知到达原空间。D 改连 B 后，A/B/D 仍属于原空间且仍将 C 视为成员；管理员 A 此时执行 `space remove C`。

A/B/D 最终成员列表均为 A/B/D，控制状态中的成员和身份映射不再包含 C，设备拓扑不含 C 及其连线，本地旧 C 配对也已清理；A 的审计记录包含接受的管理员移除操作。D 保持原空间和普通用户身份。A/D 经 B 继续两跳双向同步配置。随后强杀全部 A/B/D 并倒序重启，再次验证移除结果、原空间身份及双向同步，C 没有被旧快照恢复。

| 运行 | 场景 | CLI 命令 | 时长 | 报告 |
| --- | ---: | ---: | ---: | --- |
| TCP | 5/5 | 165 | 8.86 s | `/tmp/operit-space-stress-6z55yjy8/report.json` |
| TCP/HTTP/WS 混合 | 5/5 | 175 | 11.69 s | `/tmp/operit-space-stress-t147qrtv/report.json` |

共 10/10 场景、340 条命令通过，两个报告均无清理错误。逐端证据保留在 `removed-{A,B,D}.json`、`restarted-{A,B,D}.json`，管理员操作日志在 `removal-audit.json`。此场景复用现有产品逻辑，无运行时代码改动。

```sh
python3 apps/cli/tests/space_stress_session.py --focus silent-removal --transport tcp
python3 apps/cli/tests/space_stress_session.py --focus silent-removal --transport mixed
```


## 两个 CLI 单向配对与状态同步（2026-10-15）

新增 `--focus directed-pair`，仅启动两个隔离的真实 CLI。进程网络策略只允许 B 主动连接 A，禁止 A 主动发起任何 TCP 连接，并执行允许/禁止端口探针。全程仅由 B 发起一次配对；A 的配对记录为 inbound=true/outbound=false，B 为 inbound=false/outbound=true，没有反向再次配对。

配对本身保留双方独立空间，不生成加入申请。B 正常申请加入 A 的空间并获批后，两端空间信息（包括名称、版本、成员）和完整控制状态相同；身份均为 A=admin、B=user；两端设备列表均包含双方，且双方查看对方状态均为 Online。A 修改空间名称，B 自动同步；配置载荷通过既有连接双向同步。

分别强杀 B、A：存活端的设备列表和状态接口均显示对方 Offline，但保留两端成员资格。存活端为离线端写入配置后，重启离线端，无需再次配对或申请加入即恢复双方 Online、同步离线配置；重连后再次验证双向配置同步。

纯 TCP 共 6/6 场景、164 条真实 CLI 命令通过，用时 7.89 秒。报告为 `/tmp/operit-space-stress-15p5lkrv/report.json`，清理错误为空，所有测试 CLI 子进程均已退出。 此次使用工作区已有的 `apps/cli/target/debug/operit2` 二进制。收尾时工作区存在另外的未解决合并冲突：`core/crates/node/edge/src/space_admission_tests.rs` 和 `core/crates/tool/services/src/tools/AIToolHandler.rs`，且部分运行时源码晚于该二进制。因此这次运行证明现有二进制的行为，不代表当前合并中的源码已经构建验证。未修改或处理这些合并冲突。逐端证据保留为 `joined-{A,B}.json`、`renamed-{A,B}.json`、`restarted-{A,B}-{A,B}.json`；离线状态为 `offline-B-seen-by-A.json`、`offline-A-seen-by-B.json`。本轮只补回归，无产品代码改动。

```sh
python3 apps/cli/tests/space_stress_session.py --focus directed-pair --transport tcp
```
