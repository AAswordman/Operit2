# Space 同步/对账加固路线（Space Sync Hardening Roadmap）

> **文档性质**：工作路线文档（plan），描述"将要做什么、按什么顺序、用什么标准验收"，不是当前行为的规范。
>
> **事实基线**：标注 `[已核实]` 的代码事实于 2026-10-09 在 `feat/space-reconcile`（`3d60ddd6`）上逐条核对过，动工前不必重新考古；其余为设计意图。
>
> **背景**：一次 Space 成员记录分裂事故的复盘产物。#234 堵住了新增污染入口（拒绝自引用 join、append 前校验），#236 引入 `reconcileSharedSpace` 成对对账修复已发生的分裂。本文回答下一个问题：如何让这套新机制在野外活下来，然后如何从结构上消灭这类分歧。

## 1. 总原则

先让新机制可运维（第一梯队），再做结构手术（第二梯队），快赢穿插。判别每项工作的性质用一条标准：**对外契约变没变**——能力变化要互操作回退和对抗测试护航，实现变化要等价性证明护航（见 §4）。

## 2. 已核实的现状（事实基线）

### 2.1 对账交换是全量列表，无时钟、无上限 `[已核实]`

- wire 入口：`core/crates/node/runtime/src/NodeSpaceService.rs:202`，edge 变体 `RuntimeRemoteLinkService.rs:563`；实现在 `core/crates/node/runtime/src/peer/space_reconcile.rs`。
- `ReconciliationOffer`/`ReconciliationOutcome`（space_reconcile.rs:11-24）只有 `spaceId / operations / deviceProfiles`，**没有时钟字段**；发送与应答都取 `currentSpaceOperations()`，其内部是 `operationsSince(&SyncClock::empty(), …, usize::MAX)`（NetworkControlStore.rs:382-385、673-681）——从空时钟全量导出。
- 去重逻辑 `mergePeerOperations`（space_reconcile.rs:48-64）：按 `opId` 跳过已知、只接受 `originDeviceId == peer`，**对对方 offer 的内容照单全收，无数量/字节上限**。
- `SyncOperationStore::operationsSince(&SyncClock, &[String], usize)` 已存在（SyncOperationStore.rs:561-566），全库唯一传真实时钟的调用方是文件同步桥（OperitApplication.rs:771/776）——增量化的地基现成。

### 2.2 消息上限：受限设备 8KB / 默认 4MB `[已核实]`

- `PeerRuntimeLimits::constrained()` → `maxMessageBytes: 8 * 1024`（HostRuntimePeerService.rs:62-67），ESP32 入口在用（apps/esp32/src/main.rs:262）；默认 `DEFAULT_MAX_PEER_MESSAGE_BYTES = 4MB`（core/crates/node/peer-link/src/lib.rs:14）。
- 超限的后果是**连接被终止**（peer-link/src/transport/inbox.rs:33-38），不是消息被丢弃——受限设备上大日志对账不是"慢"，是"无法完成"。

### 2.3 解码失败 = 整条读取路径失败，无任何 quarantine `[已核实]`

- 全仓库无 quarantine 概念（零命中）。
- 一行坏日志让 `decodeOperationLog` / `loadOperationLogIndex` / `orderedCommands` **整体返回 Err**（SyncOperationStore.rs:931、959；NetworkControlStore.rs:687），而 `orderedCommands` 喂的不只是对账，还有快照、join、`afterJoined` 控制交换——**存量污染 = 全功能变砖**。
- 对比：replay 期间的鉴权失败会记为 audit `accepted: false` 并继续（NetworkControlStore.rs:731-737）；解码失败没有对应出路。
- 对账路径上，对方 offer 里一条命令反序列化失败会**中止整个交换**（space_reconcile.rs:55-56）。
- 设计含义：**doctor 与 `--repair` 必须绕过 store API 直接读原始 jsonl**；quarantine 本质是对原始日志行做手术。

### 2.4 join 状态机：`Approving` 无出口，且是有意设计 `[已核实]`

状态机内联在 `core/crates/node/runtime/src/peer/space_join.rs`（没有生产版 `join_state_machine.rs`；测试经 `include!` 引入 tests/device_space/join_state_machine.rs，space_join.rs:801-804）。

- 状态枚举：Pending / Approving / Approved / Rejected / Cancelled / Expired / Joined（NodeSpaceService.rs:33）。
- `Approving` 的出口只有 `completeDecision` 的 Approved/Rejected（space_join.rs:613/616）；`cancelJoin` 网关只对恰好 `Pending` 生效，其余**静默 no-op**（:430-433）；申请人侧 `cancel` 只置 flag（:784-799）。
- **无 Approving 超时**：`LIFETIME_MS = 15min` 只在 `expire()` 里对 Pending 生效（:73-79）；`OFFLINE_GRACE_MS = 30s` 本应管审批人重指派，但 `assign()` 对非 Pending 直接 early-return（:138）——**审批人认领后失联，记录永久卡死**。
- 钉住是有动机的：approve 路径**先写 admission 操作进日志、再写结果记录**（completeDecision，admission applied 在状态改写之前）——审批人在两步之间崩溃时，成员资格已经通过日志持久化，而申请人还看着 `Approving`。测试 `only_pending_requests_expire_not_committed_or_claimed_decisions`（tests/device_space/join_state_machine.rs:86）明确编码了这语义。
- join 记录是本地墙钟的 preference 文件（INBOUND/OUTBOUND/INBOX/RESULTS，space_join.rs:12-15，PeerStateStore），**不在复制日志里**；控制日志 replay 是时间确定的（replayCommands 只用存储的 createdAt，NetworkControlStore.rs:702-755）——给 `Approving` 加超时**不会**破坏 replay 确定性。

### 2.5 RTT 已在算、只进超时；喂路由的入口是死代码 `[已核实]`

- EWMA `(7*rtt+sample)/8` 与抖动 `(3*var+|rtt-sample|)/4` 在 heartbeat.rs:45-48，唯一消费方是 `timeoutMs()`（:52-57，clamp 5s–60s）；`rtt` 字段私有、无 getter。
- `CoreSpaceLinkAdvertisement.smoothedRttMs`（CoreSpaceStore.rs:57）的生产者**只有测试文件**；`setDirectPeers`（:709）、`publishLocalLinkAdvertisement`（:741）全仓库无非测试调用点，也不在 Dart 代理里。
- `linkCost`（CoreSpaceStore.rs:1362-1367）= `1000 + rtt*10 + loss*25 + congestion*10`，公式与 device-net-routing-distribution.md 逐字吻合，且在生产路径上（CoreNodeRouter.rs:753、1880）；但生产中所有链路都走 `UNMEASURED_DIRECT_PEER_COST = 1e9` fallback（:18、:852）——**加权路由在运行时退化为跳数**。

### 2.6 设备状态流只产 Online/Offline `[已核实]`

- flow 判定就是 `activePeerNodeIds.contains ? Online : Offline`（RuntimeRemoteLinkService.rs:1030-1034）；枚举有四态 Online/Offline/Invalid/RemovedFromSpace（:43-48）。
- **单设备查询 `pairedDeviceStatus` 已经解析 Invalid（:889）和 RemovedFromSpace（:892）**，CLI 在用（apps/cli/src/cli/link.rs:504）——Flutter 侧只差把 flow 接到这套判定上。

### 2.7 文档漂移与死代码清单 `[已核实]`

- `link-access-architecture.md:104-105、143`：`PeerRequest::Handoff`/`BindingApply` 不存在（真实枚举 7 个变体，core/crates/foundation/link/src/protocol.rs:854-862）。
- `cli-network-control.md:144`：3s 轮询间隔与代码一致（`DISCOVERY_INTERVAL_MS = 3s`）；真漂移是**单次连接尝试上限**——文档 5s vs 代码 `CONNECT_DEADLINE_MS = 30s`（peer/availability.rs:7-9）。
- `device-net-routing-distribution.md`：用规范性现在时写"本文档定义了……"，但 `SharedWatchHub`、`Unicast/SnapshotShared` 分发类、`RouteDecision` 在代码里零命中——**规范现时、实现半成品**。
- 死代码：presence 读写三 API（CoreSpaceStore.rs:651/673/682）零调用者；`validateSpaceJoin`（RuntimeRemoteLinkService.rs:1041）仅测试引用。勘误：`d21e469e` 并未触碰 `validateSpaceJoin`，它是更早 join 路径替换的遗骸。

## 3. 路线

### 第一梯队：让 #236 可运维（1–2 周量级）

#### 3.1 `link doctor`（性价比最高）

功能：

1. 比对外四层真相（配对、成员记录、策略回放、join 记录）找不一致；
2. 绕过 store API 扫描原始 jsonl，报告解码失败的命令（存量污染检测）；
3. 报告 availability worker / 心跳状态。

设计约束：

- 诊断逻辑进 core crate 做库，CLI 只是薄门面；将来 Flutter 可暴露——多数节点是手机，用户不会跑 CLI。
- 顺手埋计数器：divergence 检出次数、quarantine 条数、对账成功率/轮次。
- `--repair` 安全设计前置：**默认只读**；`--repair` 显式开启；quarantine = **移入旁路文件而非删除**（可逆）；动手前自动备份；修后自校验"quarantine 后 replay 结果 == 原 replay 结果减去被隔离命令"。
- 可选的运行时增强（与工具解耦）：让读路径对坏行"跳过并旁路记录"，使单行污染不再炸整读——形式上是解码策略的实现变化，效果上等价于新保证（读不再被单行毒死），且是 doctor 能读到数据的前提之一。

#### 3.2 对账增量化 + 限流（最大的契约变更，按能力变化对待）

- offer/outcome 增加时钟字段，改走 `operationsSince`："报时钟、补差异"。
- **分块是必然需求**：8KB 下长时间离线后一次合法增量回填同样超限，需分页交换。
- **版本回退**：现 offer 无版本字段；对方不认新格式退回全量交换。preview 阶段允许破坏，但混合舰队行为要显式声明。
- **限流与增量化必须同 PR**：增量交换存在之前，"未知操作数上限"会误伤合法追赶。
- 上限要**双向**：接受侧（`mergePeerOperations` 今天照单全收）与发送侧（非受限链路 4MB 的 `Vec<SyncOperation>` 也是资源压力向量）。
- **合入门禁：最小收敛属性测试**（见 3.7）——这改的是收敛协议本身，现有安全网只有手写定点用例。

#### 3.3 `Approving` 出口

- 首选**修已有机制**而非发明新转移：让 `assign()` 的 `OFFLINE_GRACE` 重指派对"认领后失联"生效（space_join.rs:138 的 early-return 就是实际 bug 点）。
- 若加 `Approving→Cancelled`：判据必须**日志性**——"控制日志中不存在该 join 的 admission 操作"（任意一端 replay 可确定性判定），**不能用"尚无 accepted 回执"**（本地视角、有竞态：approve 先写 admission 再写结果，崩溃窗口内成员资格已持久化而回执未达，见 §2.4）。
- 这条改动与现有测试编码的设计意图（钉住语义）冲突，需连测试与意图一起改，PR 里写明新语义。
- join 记录本地墙钟、不入复制日志（§2.4），超时方案不破坏 replay 时间确定性。

### 第二梯队：记录即投影（季度量级，唯一的结构性实现变化）

#### 3.4 成员记录降格为可重建投影

三步走，每步独立可合入、每步验收"等价性"：

1. **立规矩，且编译期强制**：把记录写入 API 藏进只有 replay applier 持有的 capability/受限可见模块后面——纪律变类型错误，不靠 review 记忆。
2. **投影泵**：`records = f(log replay)` 重建函数，doctor 用它做影子校验（diff=0 是迁移每一步的验收线）。
3. **逐个迁移写入点**：`observePairedDeviceSpace` 只翻译公告为日志操作；`alignRemoteMemberRecord` 退化为 replay 的副作用。

终局（必须在动工前补全设计，而不是事后）：

- `PeerSpaceSnapshot` 砍 `space` 投影字段、只留 ops+profiles。**这是交换格式形状变化，属于设备互联兼容面**，混合版本期要处理。
- **压缩/快照故事**：日志成为唯一真相源后无界增长，replay 重建与 doctor 全量校验成本随日志线性涨；需要周期性 snapshot + 前缀截断。一旦快照真实性/签名进入协议，该项就越界成能力变化，要显式划线。
- profiles 留在日志外是**有意划界**（密钥材料不入日志），在文档中明说，作为残留的双真相应被理解和测试。

#### 3.5 RTT 喂路由，或删掉加权路由（二选一，别悬着）

- **接线分支 = 能力变化**：heartbeat EWMA → 已存在的 `publishLocalLinkAdvertisement`，纯接线。行为变化要想清楚：实测链路 cost ~10³–10⁴ 永远碾压未实测的 10⁹ fallback，**两跳实测会压过一跳未实测直连**；EWMA 自带平滑，仍建议加迟滞（cost 变化超阈值才换路）。
- **删除分支 = 实现变化**，可证行为不变（生产今天全走 fallback，Dijkstra 已退化跳数）；同时把 device-net-routing-distribution.md 改成 plan 文档并补齐缺失部分（§2.7）。
- 决策用能力问题回答（"要不要质量感知多跳"），不用代码考古回答。

#### 3.6 `Invalid`/`RemovedFromSpace` 接进状态流

core 侧判定已存在（§2.6），flow 复用 `pairedDeviceStatus` 的解析即可，工作量小。`RemovedFromSpace` 的语义来源（收敛策略排除）依赖 #236 语义稳定后再标。

### 第三梯队：正确性可证明 + 清尾（背景任务）

#### 3.7 收敛属性测试（最小版提前为 3.2 的门禁）

核心不变量：任意操作序列 × 任意到达顺序 × 任意分裂状态 → 两端合并后 replay 结果一致。3.4 的投影泵等价性校验直接复用这套测试。

#### 3.8 n 成员记账泛化

等 #236 在真实多设备环境跑出分裂数据后再决定是否升级为周期性 gossip（可能由 Sync 服务顺路捎带，不新开通道），不预设计。

#### 3.9 清尾

§2.7 全部清单 + 本文档勘误记录。

## 4. 能力变化 vs 实现变化（验收分轨）

判据：**对外契约变没变**（新操作/新状态/新保证/新参与者 = 能力变化；契约不变、内部重排、验收等价性 = 实现变化）。

| 条目 | 类别 | 备注 |
|---|---|---|
| doctor 只读诊断 | 能力（新增可观测面） | 系统第一次能自述病情 |
| doctor `--repair` | 能力（新恢复保证） | 坏一行不再全砖 |
| 对账增量化+限流 | **能力（最大契约变更）** | wire 协议、参与者类别、抗垃圾保证三重变 |
| Approving 出口 | 能力（生命周期新转移） | 判据必须日志性 |
| 记录即投影 | **实现（唯一大头）** | 必须行为保持 |
| RTT 接线 / 删除 | 能力 / 实现 | 唯一的真二选一分叉 |
| UI 状态暴露 | 能力暴露（非创造） | core 已有判定，差一根线 |
| 属性测试 | 实现（证明装置） | 不改产品行为 |
| n-member gossip | 能力（延后） | 收敛语义扩展 |
| 清尾 | 实现 | 零契约风险 |

验收门槛分轨：

- **能力变化 PR**：互操作/版本回退说明、对抗性测试（上限、垃圾注入）、用户可见面（l10n、截图、文档）。
- **实现变化 PR**：等价性证明（投影泵 diff=0、属性测试新旧实现同过）。说不清等价性的记录即投影 PR 应被打回。

两个边界陷阱（一反一正）：

- 3.2 长得像 refactor（"只是发增量"），实际是协议变更——不能按实现变化放行。
- 3.4 长得像行为修复（"消灭一类 bug"），必须按实现变化执行——规范从来都承诺收敛到那些记录，重构只是让承诺机械成立；迁移中每个"顺手修好的分歧 case"都要能区分"本来就该这样"还是"迁移漂移"，区分手段是属性测试。

对"是否破坏设备互联逻辑"的准确表述：整条路线**不动"设备之间约定什么"的语义**（收敛结果、成员资格真相、权限边界全部保持设计意图）；会动"设备之间怎么说话"——一部分是补全（能力变化，需版本回退护航），一部分是内部重排（实现变化，需等价性证明护航）。

## 5. 执行纪律

1. **日志应用路径之外不写成员记录**——从 review 约定起步，3.4 第 1 步落地为编译期强制。
2. 改收敛/交换协议的 PR 必须带最小属性测试门禁。
3. 每个能力变化带版本回退；每个实现变化带等价性证明。
4. 诊断/修复工具自身的安全性按被修复对象同级对待（`--repair` 默认只读、可逆、备份、自校验）。
