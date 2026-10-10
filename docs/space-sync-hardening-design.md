# Space 同步/对账加固：路线与设计（Space Sync Hardening Design）

> **文档性质**：路线 + 设计文档（plan + design）。§1–§2 回答"为什么、按什么顺序"；§3 对每项工作给出可直接动工的设计（接口、协议、状态机、验收）。设计描述的是"将要做成什么样"，不是当前行为的规范；动工时若与代码事实冲突，以代码为准并回改本文。
>
> **事实基线**：标注 `[已核实]` 的代码事实于 2026-10-09 在 #236 合入后的 `upstream/main`（`51b92fab`；其后的 `..09be5d7a` 仅 JS bridge 变更，不涉及被引用文件）逐条复核过，动工前不必重新考古。存储布局与 wire 面速查见 §6 附录。
>
> **背景**：一次 Space 成员记录分裂事故的复盘产物。#234（堵住新增污染入口：拒绝自引用 join、append 前校验）与 #236（`reconcileSharedSpace` 成对对账，2026-10-09 合入 `51b92fab`）均已进入上游。本文回答下一个问题：如何让这套新机制在野外活下来，然后如何从结构上消灭这类分歧。
>
> **实施状态（2026-10-09）**：第一梯队四项已在本仓库落地——3.3 Approving 出口、3.1 读路径旁路+quarantine+doctor、3.7 收敛属性测试（P1/P2）、3.2 增量/分页/限流协议。实施中两处偏离原设计并被本文吸收：quarantine 落盘为单文件 jsonl 旁车（§3.1）；增量差集用精确 opId 集合而非向量时钟游标（§3.2，属性测试证伪了时钟方案）。

## 1. 总原则

先让新机制可运维（第一梯队），再做结构手术（第二梯队），快赢穿插。判别每项工作的性质用一条标准：**对外契约变没变**——能力变化要互操作回退和对抗测试护航，实现变化要等价性证明护航（见 §4）。

## 2. 已核实的现状（事实基线）

### 2.1 对账交换是全量列表，无时钟、无上限 `[已核实]`

- wire 入口：`core/crates/node/runtime/src/NodeSpaceService.rs:202`，edge 变体 `RuntimeRemoteLinkService.rs:563`；实现在 `core/crates/node/runtime/src/peer/space_reconcile.rs`。
- `ReconciliationOffer`/`ReconciliationOutcome`（space_reconcile.rs:11-24）只有 `spaceId / operations / deviceProfiles`，**没有时钟字段**；发送与应答都取 `currentSpaceOperations()`，其内部是 `operationsSince(&SyncClock::empty(), …, usize::MAX)`（NetworkControlStore.rs:382-385、673-681）——从空时钟全量导出。
- 去重逻辑 `mergePeerOperations`（space_reconcile.rs:48-64）：按 `opId` 跳过已知、只接受 `originDeviceId == peer`，**对对方 offer 的内容照单全收，无数量/字节上限**。
- `SyncOperationStore::operationsSince(&SyncClock, &[String], usize)` 已存在（SyncOperationStore.rs:561-566），全库唯一传真实时钟的调用方是文件同步桥（RuntimeFileSyncStore.rs:561/590）——增量化的地基现成。`SyncClock` 是向量时钟 `BTreeMap<originDeviceId, sequence>`，`sequence` 由 origin 设备分配、跨副本不变。
- **勘误（实施时发现）**：向量时钟只记录每 origin 的最大 sequence，隐含"前缀连续覆盖"假设；分裂后的副本可持有对端日志的任意子集（这正是 #236 要修的状态），时钟会谎报覆盖完整。增量对账因此不能以时钟为差集依据，见 §3.2 的修正设计。
- 对账挂在 `space_join::request` 的分裂自愈路径上（space_join.rs:249），意味着分页循环的预算上限同时约束 join 触发的对账。

### 2.2 消息上限：受限设备 8KB / 默认 4MB `[已核实]`

- `PeerRuntimeLimits::constrained()` → `maxMessageBytes: 8 * 1024`（HostRuntimePeerService.rs:62-67），ESP32 入口在用（apps/esp32/src/main.rs:262）；默认 `DEFAULT_MAX_PEER_MESSAGE_BYTES = 4MB`（core/crates/node/peer-link/src/lib.rs:14）。
- 超限的后果是**连接被终止**（peer-link/src/transport/inbox.rs:33-38），不是消息被丢弃——受限设备上大日志对账不是"慢"，是"无法完成"。

### 2.3 解码失败 = 整条读取路径失败，无任何 quarantine `[已核实]`

- 全仓库无 quarantine 概念（零命中）。
- 一行坏日志让 `decodeOperationLog` / `loadOperationLogIndex` / `orderedCommands` **整体返回 Err**（SyncOperationStore.rs:931、959；NetworkControlStore.rs:687），而 `orderedCommands` 喂的不只是对账，还有快照、join、`afterJoined` 控制交换——**存量污染 = 全功能变砖**。
- 控制操作存在**两份副本**：per-origin 日志（`operations/{device}.jsonl`）与控制投影（`runtime/link_access/space_policy.preferences.json`），`orderedCommands` 按 `opId` 合并两者，冲突即报 `Conflicting immutable Space control operation`（NetworkControlStore.rs:673-695）——doctor 必须两份都扫。
- 对比：replay 期间的鉴权失败会记为 audit `accepted: false` 并继续（NetworkControlStore.rs:731-737）；解码失败没有对应出路。
- 对账路径上，对方 offer 里一条命令反序列化失败会**中止整个交换**（space_reconcile.rs:55-56）。
- 设计含义：诊断读路径必须绕过"遇坏行即 Err"的通用 store API 直接读原始 jsonl；quarantine 本质是对原始日志行做手术，但**写入必须走新增的、持有 store 既有 per-origin 锁的专用入口**（见 §3.1）——绕开的是失败语义，不是锁纪律。

### 2.4 join 状态机：`Approving` 无出口，且是有意设计 `[已核实]`

状态机内联在 `core/crates/node/runtime/src/peer/space_join.rs`（没有生产版 `join_state_machine.rs`；测试经 `include!` 引入 tests/device_space/join_state_machine.rs，space_join.rs:801-804）。

- 状态枚举：Pending / Approving / Approved / Rejected / Cancelled / Expired / Joined（NodeSpaceService.rs:33）。
- `Approving` 的出口只有 `completeDecision` 的 Approved/Rejected（space_join.rs:613/616）；`cancelJoin` 网关只对恰好 `Pending` 生效，其余**静默 no-op**（:430-433）；申请人侧 `cancel` 只置 flag（:784-799）。
- **无 Approving 超时**：`LIFETIME_MS = 15min` 只在 `expire()` 里对 Pending 生效（:72-82）；`OFFLINE_GRACE_MS = 30s` 本应管审批人重指派，但 `assign()` 对非 Pending 直接 early-return（:138）——**审批人认领后失联，记录永久卡死**。
- 钉住是有动机的：approve 路径**先写 admission 操作进日志、再写结果记录**（completeDecision，`applyBootstrapOperation` 在状态改写之前）——审批人在两步之间崩溃时，成员资格已经通过日志持久化，而申请人还看着 `Approving`。测试 `only_pending_requests_expire_not_committed_or_claimed_decisions`（tests/device_space/join_state_machine.rs:86）明确编码了这语义。
- **日志性判据已有现成实现**：admission 操作的指纹是 `entityId == "control-review-{requestId}-{assignmentVersion}"` 且 issuer 为审批人（restoreClaimedMemberProfiles，space_join.rs:539-548 已在生产代码用同一判定做 profile 恢复）——任何一端 replay 都能确定性判断"这个决定是否已提交"。
- join 记录是本地墙钟的 preference 文件（INBOUND/OUTBOUND/INBOX/RESULTS，space_join.rs:12-15，PeerStateStore），**不在复制日志里**；控制日志 replay 是时间确定的（replayCommands 只用存储的 createdAt，按 `(createdAt, originDeviceId, sequence)` 全序，NetworkControlStore.rs:702-755、872-874）——给 `Approving` 加超时**不会**破坏 replay 确定性。

### 2.5 RTT 已在算、只进超时；邻接公告已接线，加权路由仍退化跳数 `[已更新 2026-10-10]`

- EWMA `(7*rtt+sample)/8` 与抖动 `(3*var+|rtt-sample|)/4` 在 heartbeat.rs:45-48，唯一消费方是 `timeoutMs()`（:52-57，clamp 5s–60s）；`rtt` 字段私有、无 getter。
- `CoreSpaceLinkAdvertisement` 的生产者已接线：`SpacePersistenceSyncService::synchronizeOnce` 每轮同步先调 `space_topology::publishLocalTopology`（node/runtime/src/peer/space_topology.rs），由 `CoreSpaceStore::publishLocalAdjacency` 写出本机记录——peers = 活跃直连 ∩ Space 成员 ∩ 未断开，每个 peer 一条 `smoothedRttMs = 0` 的 link，TTL 10min / 刷新余量 5min，内容未变不重写。`setDirectPeers`、`publishLocalLinkAdvertisement` 保留给未来测量路径与测试。
- `linkCost`（CoreSpaceStore.rs:1362-1367）= `1000 + rtt*10 + loss*25 + congestion*10`，公式与 device-net-routing-distribution.md 逐字吻合，且在生产路径上（CoreNodeRouter.rs:753、1880）；公告链路 rtt=0 时代价均匀，**路由等价跳数**（§3.5 两分支仍未决，但成员互访不再依赖该决策）。

### 2.6 设备状态流只产 Online/Offline `[已核实]`

- flow 判定就是 `activePeerNodeIds.contains ? Online : Offline`（RuntimeRemoteLinkService.rs:1030-1034）；枚举有四态 Online/Offline/Invalid/RemovedFromSpace（:43-48）。
- **单设备查询 `pairedDeviceStatus` 已经解析 Invalid（:889）和 RemovedFromSpace（:892）**，CLI 在用（apps/cli/src/cli/link.rs:504）——Flutter 侧只差把 flow 接到这套判定上。
- 连接状态枚举新增 `Announced`：不涉及本机的边此前报 `Unknown`，现在有独立语义与 UI 文案（"已宣告"）；设备投影同时暴露 `relayHops` / `relayPath`（多跳成员的跳数与逐跳路径）。

### 2.7 文档漂移与死代码清单 `[已核实]`

- `link-access-architecture.md:104-105、143`：`PeerRequest::Handoff`/`BindingApply` 不存在（真实枚举 7 个变体，core/crates/foundation/link/src/protocol.rs:854-862）。
- `cli-network-control.md:144`：3s 轮询间隔与代码一致（`DISCOVERY_INTERVAL_MS = 3s`）；真漂移是**单次连接尝试上限**——文档 5s vs 代码 `CONNECT_DEADLINE_MS = 30s`（peer/availability.rs:7-9）。
- `device-net-routing-distribution.md`：用规范性现在时写"本文档定义了……"，但 `SharedWatchHub`、`Unicast/SnapshotShared` 分发类、`RouteDecision` 在代码里零命中——**规范现时、实现半成品**。
- 死代码：presence 读写三 API（CoreSpaceStore.rs:651/673/682）零调用者；`validateSpaceJoin`（RuntimeRemoteLinkService.rs:1039）仅测试引用。勘误：`d21e469e` 并未触碰 `validateSpaceJoin`，它是更早 join 路径替换的遗骸。

## 3. 路线与设计

### 第一梯队：让 #236 可运维（1–2 周量级）

#### 3.1 `link doctor`（性价比最高）

**形态与落点**

- 诊断逻辑进 core 做库：`core/crates/node/runtime/src/peer/space_doctor.rs`，输入 `&dyn NodeSpaceContext`，输出可序列化的 `DiagnosisReport`；CLI 加 `operit2 cli link doctor [--json] [--repair]` 薄门面。将来 Flutter 经既有代理面暴露（多数节点是手机，用户不会跑 CLI）。
- 诊断**只读**，全部走原始文件扫描 + 各 store 的读快照；唯一新增的 store 写入口是修复专用（见修复状态机）。

**诊断模型：四层真相 + 一层原始文件**

| 层 | 真相来源 | 读取方式 |
|---|---|---|
| L1 配对 | `peers().pairedPeers()` | store 读 API（无坏行问题） |
| L2 成员记录 | `CoreSpaceStore`（space.members、对端成员记录投影） | store 读 API |
| L3 策略 replay | `NetworkControlStore::currentState()`（控制日志 replay 产物） | store 读 API；若已中毒则置"不可读"标记，改用 R4 影子 replay |
| L4 join 记录 | PeerStateStore 四个 preference 文件（space_merge_*） | store 读 API（serde 失败整文件报告，同样支持原始扫描兜底） |
| R4 原始层 | `operations/*.jsonl` + `space_policy.preferences.json` + `clocks.json` / `export_floors.json` / `devices.json` | **绕过 store API，逐行扫描** |

**检查项（编号即验收用例名）**

- `D1_member_record_vs_replay`：对 L3 可读的库，影子比较 L2 成员记录与 replay 产物（含 `alignRemoteMemberRecord` 的目标态）；不一致报 divergence（这正是 #236 对账针对的病）。
- `D2_pairing_vs_member`：已配对但无成员记录 / 有记录但配对已解除。
- `D3_join_vs_log`：对每条 `Approving` 记录：日志已有其 admission 指纹（§2.4 判据）→ 报"可恢复（可自动补完 Approved）"；无 admission 且审批人失联超 `OFFLINE_GRACE_MS` → 报"卡死（重指派可修，对应 §3.3）"。
- `D4_raw_line_scan`：逐行 `serde_json` 解析 per-origin 日志，报每条坏行的 device、字节 offset、行号、错误类别（非 UTF-8 / JSON 损坏 / SyncOperation 字段缺失）；**只报告，不动数据**。
- `D5_payload_decode`：对控制域好行做 `NetworkControlCommandRecord` 反序列化（含加密 payload 的解密，`PreferencesEncryption` 在本机可用）；区分"信封损坏"与"解密失败（密钥缺失 ≠ 污染）"。
- `D6_dual_copy_consistency`：jsonl 与 `space_policy.preferences.json` 投影按 `opId` 对账，复用 `orderedCommands` 的冲突规则；单侧缺失报"投影漂移"。
- `D7_clock_floor_sanity`：`clocks.json`/`export_floors.json` 与日志最高 sequence 对照；floor 越过现存行、时钟落后于日志均为异常。
- `D8_availability_snapshot`：availability worker / 心跳状态只读报告（对应 §2.5/§2.6 的观测面）。

**修复状态机（`--repair`，默认关闭）**

```
plan（只读生成 RepairPlan：待隔离行清单 + 预期效果）
→ backup（目标文件复制为 <file>.bak-<utc-timestamp>，失败即中止）
→ apply（隔离，见下）
→ selfcheck（验收等式，见下；不过等式 → 从备份还原并报错）
```

- quarantine = **移入旁路文件而非删除**：`operations/{id}.jsonl.quarantine` 单文件 jsonl 旁车（每行一条 `{deviceId, byteOffset, byteLength, lineHash, quarantinedAt, line}`；实施时放弃"目录+逐行文件"形态——RuntimeStorageHost 不承诺目录语义，单文件在受限主机上可原子重写且可逐行追加），完全可逆。
- 写入路径：新增 `SyncOperationStore::quarantineLines(deviceId, &[offset])` 专用入口——内部持有与 append 相同的 per-origin 锁，做字节级行剔除 + 索引缓存失效。不经过任何通用 append/read API；这是对 §2.3"绕过 store API"的精确化：绕开失败语义，不绕锁纪律。
- **selfcheck 等式**：坏行会让"修复前 replay"根本无法计算（Err），所以等式不能写成"修复后 == 修复前减被隔离行"。正确等式：**修复后 store 的 `orderedCommands`+replay 输出 == doctor 影子 replay（R4 层好行，剔除被隔离行）的输出**。影子 replay 逻辑与 3.4 投影泵同源，一处实现两处用。
- 埋计数器：`runtime/diagnostics/space_doctor_counters.preferences.json`——divergence 检出次数、quarantine 条数、对账成功率/轮次（对账侧在 space_reconcile 成功路径埋点）。

**前置增强（从"可选"升级为有依赖关系的工作项）**：读路径对坏行"跳过并旁路记录"，使单行污染不再炸整读（`decodeOperationLog`/`loadOperationLogIndex` 改为收集坏行继续，坏行清单暴露给 doctor）。形式上是解码策略的实现变化，效果上等价于新保证（读不再被单行毒死）。**注意：它同时是 3.4 快照故事的前置**（快照操作对旧版本 peer 是"未知命令"，会触发 §2.3 的整读失败，见 §3.4）——优先级因此高于原文档的"可选"定位。

#### 3.2 对账增量化 + 限流（最大的契约变更，按能力变化对待）

**设计目标**：offer/outcome 从"报全量"改为"报持有集合、补差集、按页装"，使 8KB 受限设备与长期离线节点都能完成对账；同时给接受/发送两侧装上上限。**分页与差集同属一个协议版本，限流与增量化必须同 PR**——增量交换存在之前，"未知操作数上限"会误伤合法追赶。

**差集依据的修正（实施时由 3.7 属性测试证伪原设计）**：原方案以向量时钟为增量游标（"报时钟、补差异"）。P1 测试里"随机子集预合并"制造了对端日志的洞（持有 a:1,2,7-10 却缺 3-6），最大序列时钟声称已覆盖到 10，对端据此永不补发——这正是分裂事故后副本的真实形态。落地协议改为**精确 opId 集合对账**：offer/outcome 各携带发送方的完整操作 id 集合（`haveOpIds`），接收方回放"对方集合未覆盖的差集"分页；集合超出页预算时降级为 `None`（= 请发全量分页），正确性永不因预算破坏。时钟字段保留在报文中，仅作 export-floor 缺口（`historyGap`）的 advisory。

**wire 变更（全部新字段 serde default，旧端反序列化自动忽略未知字段，不炸）**

```rust
struct ReconciliationOffer {
    spaceId: String,
    operations: Vec<SyncOperation>,
    deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    // ── v2 新增 ──
    protocolVersion: Option<u32>,        // 缺失 = v1 对端
    have: Option<SyncClock>,             // advisory：发送方时钟（供 floors/historyGap 判定）
    maxPageBytes: Option<u32>,           // 发送方愿意收的页上限
    haveOpIds: Option<Vec<String>>,      // 发送方持有的全部操作 id；None = 集合超预算，请回全量分页
}
struct ReconciliationOutcome {
    …同上…, members: Vec<String>,
    floors: Option<SyncClock>,           // 网关各 origin 的 export floor
    more: Option<bool>,                  // 本消息 operations 是否被截断
    historyGap: Option<Vec<String>>,     // 对方要的 sequence 已在 floor 之下、无法回填的 origin
    haveOpIds: Option<Vec<String>>,      // 网关合并后的完整 id 集合；None = 超预算
}
```

**交换循环（发起方 I，网关 G；每轮双向各带一页，差集按精确 id 集合）**

1. 首轮 offer：对端能力未知时 `operations` 为**全量**（等价今天的行为，v1 网关照常）；`haveOpIds = I 的完整 id 集合`（超预算则 None）。`maxPageBytes` 声明自己能收的页上限（默认 6KB，受限链路安全）。
2. G 合并 offer（`mergePeerOperations` 语义不变：opId 去重、`originDeviceId == peer`、spaceId 校验 + 4096/4MB 硬顶），回 outcome：`operations = G 持有且 offer.haveOpIds 未覆盖的差集`按 `offer.maxPageBytes` 截断一页，`more` 标截断，`haveOpIds = G 合并后的完整集合`（超预算 None），附 `maxPageBytes`/`floors`/`historyGap`。
3. I 合并 outcome；若 `outcome.more == true` 或 I 仍持有 `outcome.haveOpIds` 未覆盖的操作，发起下一轮：offer 带"差集"按 `outcome.maxPageBytes` 截断的一页。循环直至双方 `more == false` 且按集合判定的双向差集为空。
4. 对齐副作用（`importDeviceProfiles` / `alignRemoteMemberRecord` / 排除时的 `leave`）幂等，循环结束后执行一次；终止条件未达成而轮次预算（`MAX_RECONCILE_ROUNDS = 32`）耗尽时，返回 `converged = false`，由 availability worker 择机重跑（每轮合并都是幂等前缀推进，中断无部分提交风险——安全性论据写进 PR）。
5. `historyGap` 处理：I 对 gap origin 不再等待回填，行为等同今天（收并集）；doctor 计数器记录。控制域 floor 今天只在 leave 时推进（`markLocalOperationsUnexportable`），gap 语义无害；此判断写进 PR 供 review 复核。

**版本回退与混合舰队（显式声明）**

- 能力缓存：`runtime/link_access/space_reconcile_caps.preferences.json` 记录 peer→v2/v1；outcome 是否带 `protocolVersion = 2` 即判定（v2 网关即使收到 v1 形状的 offer 也回 v2 报文，旧端 serde 忽略未知字段）。
- 对无缓存 peer：首轮 offer 携带全量（v1 语义，v1 对端照常工作）；若对方是 v2，首轮即完成增量协商，后续对账全增量。**混合舰队语义 = 对新端首触全量、其后增量，对旧端永远全量**；只损失效率，无正确性风险。preview 阶段接受该破坏窗口。

**限流（同 PR）**

- 接收侧：`mergePeerOperations` 增加页级硬顶（`MAX_RECONCILE_PAGE_OPS = 4096` / 4MB）；超限**整页拒绝并中止交换**（不再照单全收），计对抗计数器（doctor `oversizeOffersRefused`）。未知 origin、跨 space 拒绝规则不变。
- 发送侧：分页产出本身就是上限；对未声明预算的 v1 对端沿用全量（今天的资源压力面不变，不恶化）。
- 受限链路自适应：各端默认声明 6KB 页预算（`DEFAULT_RECONCILE_PAGE_BYTES`，8KB 链路留 envelope 余量；RuntimePeerService 不暴露链路上限，故取保守常量而非按 `PeerRuntimeLimits` 自适应——待上游加访问器后可升级），解决 ESP32 8KB 下的合法回填。
- 集合预算：`haveOpIds` 序列化超 6KB 时降级为 None（= 请发全量分页），差集正确性永不因预算破坏。

**合入门禁**

- 3.7 最小收敛属性测试**先于本 PR 合入**（它改的是收敛协议本身，现有安全网只有手写定点用例）。
- 互操作矩阵：{v2, v1} × {发起方, 网关} 四格各有定点测试；垃圾注入：超大 offer、伪造 origin、跨 space、 lied-`maxPageBytes`。

#### 3.3 `Approving` 出口

原则：**修已有机制，不发明新转移**；一切判据日志性（§2.4 现成指纹）。

**补丁 1——已提交决策的本地恢复（修"审批人 complete 中途崩溃"）**

- 位置：网关侧 `reconcile()`（assignedRequests / claimDecision 已途经）。
- 判定：`record.status == Approving` 且 `admissionCommitted(record) == true`（从 restoreClaimedMemberProfiles:539-548 抽出的共享判定函数：日志存在 `entityId == "control-review-{id}-{assignmentVersion}"`、issuer 为审批人、AdmitSpace 匹配的操作）。
- 动作：本地补完 Approved 状态（admission 已在日志，无需重放写；profile 恢复走既有 `restoreClaimedMemberProfiles`）。零新 wire 消息，申请人经既有 `refresh()` 轮询看到 Approved。

**补丁 2——未提交决策的重指派（修 `assign()` early-return，实际 bug 点）**

- `assign()`（space_join.rs:138）的 early-return 细化为三分支：
  - `Pending` → 现逻辑不变；
  - `Approving` 且 `!admissionCommitted(record)` 且审批人不可达持续 `OFFLINE_GRACE_MS`（`unavailableSince` 字段已有）→ 允许走重指派，`assignmentVersion += 1`；
  - 其余 → 维持 early-return（**已提交（日志有 admission）的记录永远钉住**）。
- 旧审批人的 in-flight `claim`/`complete` 被 `validateAssignment`（绑定 reviewerDeviceId + assignmentVersion）拒绝；其本地 RESULTS 缓存因 assignmentVersion 不匹配在下次 decide 时报"Stored decision differs"并要求重新决定——可接受，写进 PR 说明。
- 与现有测试的关系：`only_pending_requests_expire_not_committed_or_claimed_decisions` 编码的"claimed 即钉住"收紧为"**committed（日志已见 admission）才钉住**"；测试拆为两条：已提交不重指派/不过期；未提交且审批人失联超 grace 重指派。PR 里写明新语义。

**补丁 3（可选）——申请人侧 `Approving → Cancelled`**

- 网关 `cancelJoin`（:430-433）接受 `Approving` 的前提是 `!admissionCommitted(record)`；判据日志性，任意一端 replay 可判。若补丁 1+2 已消除实际卡死场景，本补丁可不做——**默认不做**，留观察期。

### 第二梯队：记录即投影（季度量级，唯一的结构性实现变化）

#### 3.4 成员记录降格为可重建投影

三步走，每步独立可合入、每步验收"等价性"（投影泵 diff=0）：

1. **立规矩，且编译期强制**。两个相关 crate（`CoreSpaceStore` 与 replay applier `NetworkControlStore`）同在 `operit-store`，无需跨 crate 手术：把 `CoreSpaceStore` 的成员记录写方法收进 `pub(crate)`（或收敛到 `projection` 模块 `pub(super)`），replay applier 是唯一持写面的一方；runtime 侧经公共 API 的直写点（`observePairedDeviceSpace`、`alignRemoteMemberRecord`、`adopt/adoptAt`、`leave` 的直写部分）改为提交日志操作后由 replay 物化。**步骤 0（并入本步）**：枚举 `CoreSpaceStore` 全部写方法，逐个标注"留（replay 内部）/ 转（改为日志操作）/ 删"，清单进 PR。
2. **投影泵**：`project(log: &[SyncOperation]) -> MemberRecordState` 纯函数；doctor 影子校验（§3.1 selfcheck）与属性测试（§3.7 P3）复用同一实现。
3. **逐个迁移写入点**：`observePairedDeviceSpace` 只翻译公告为日志操作；`alignRemoteMemberRecord` 退化为 replay 的副作用；最后删直写 API（编译期保证无残留调用）。

**终局设计（动工前已补全，动工时按此执行）**

- **`PeerSpaceSnapshot` 形状收缩**：砍 `space` 投影字段、只留 `spaceId/spaceName/spaceRevision` 标识 + `ops + profiles (+topology)`；接收端 replay 重建投影。**这是交换格式形状变化（join、对账、观察四条 wire 路径共用）**：加 `snapshotVersion: u32`（serde default = 1），v2 发送方按对端能力缓存决定发 v1 全量还是 v2 收缩版；混合版本期双形状并存，能力判定复用 §3.2 的 caps 文件。
- **压缩/快照故事——推荐旁车文件方案，不进日志**：日志成为唯一真相源后无界增长，replay 重建与 doctor 全量校验成本线性涨。方案：`operations/control_snapshot.json` 旁车文件存 `{clock, 前缀投影状态}`，replay 装快照后只重放 clock 之后的操作；截断 = 把 export floor 推进到快照 clock（控制域首次真正使用 floor 语义）+ 物理剔除前缀行（走 §3.1 的锁内专用入口）。**不走"快照作为新日志操作"的路线**：未知 command 会让旧版本 peer 的 `decodeControlOperation` 整读失败（§2.3），把兼容风险从"协商"变成"爆炸"；旁车文件对旧端不可见，无此问题。截断的启用门槛：能力缓存显示**全部近期活跃副本**支持快照引导；长期离线的旧设备回联时按 §3.2 floors 协商暴露缺口，降级为引导重建（preview 阶段显式接受，写进 PR 的互操作说明）。一旦快照真实性/签名进入协议，该项越界成能力变化——**默认不做签名**（传输层已认证），划线于此。
- **profiles 留在日志外是有意划界**（密钥材料不入日志），在 `docs/core-node-space-binding-architecture.md` 明说，作为残留的双真相应被理解和测试（doctor D2/D6 覆盖）。

#### 3.5 RTT 喂路由，或删掉加权路由（二选一，别悬着）

**现状（2026-10-10）**：邻接公告已接线（见 §2.5）且不携带测量，多跳可达性不再依赖本节决策；两个分支仍未定，注意"两跳实测压过一跳未实测直连"的落点已从"全部 fallback"变为"均匀 1000 代价 + 未实测 seed fallback"。

两个分支的 PR 形态都设计完毕，**决策本身是能力问题（"要不要质量感知多跳"），留给产品拍板**；框内给出建议默认：

- **接线分支 = 能力变化**：heartbeat EWMA → 已存在的 `publishLocalLinkAdvertisement`，纯接线。行为变化要想清楚：实测链路 cost ~10³–10⁴ 永远碾压未实测的 10⁹ fallback，**两跳实测会压过一跳未实测直连**；EWMA 自带平滑，仍建议加迟滞（cost 变化超阈值才换路，阈值进常量并测试）。验收：双真机拓扑下路由选择可观测、 advertisement 面新增字段的兼容声明。
- **删除分支 = 实现变化**，可证行为不变（生产今天全走 fallback，Dijkstra 已退化跳数）；删除 `linkCost`/`UNMEASURED_DIRECT_PEER_COST`/`smoothedRttMs` 及死代码三 API，同时把 device-net-routing-distribution.md 改成 plan 文档并补齐缺失部分（§2.7）。验收：属性测试/既有定点测试全过 + 文档同步。
- **建议默认**：若近期无质量感知多跳的产品需求，先走删除分支（纯实现变化、零行为风险、顺手清 §2.7 一半欠账）；接线作为独立能力提案另起。该推荐可被产品决策推翻，但不要长期悬着。

#### 3.6 `Invalid`/`RemovedFromSpace` 接进状态流

core 侧判定已存在（§2.6），设计：把 RuntimeRemoteLinkService.rs:1030-1034 的 flow 判定抽成与 `pairedDeviceStatus` 共用的 `resolvePairedDeviceStatus(...)`，flow 构建与单设备查询走同一函数；Flutter 侧接四态 + l10n（app_en/app_zh 各四条文案）。工作量小。`RemovedFromSpace` 的语义来源（收敛策略排除）依赖 #236 语义稳定——#236 已合入，验收场景定为：三设备分裂 → 对账后被排除端 UI 显示 `RemovedFromSpace` 而非 Offline。

### 第三梯队：正确性可证明 + 清尾（背景任务）

#### 3.7 收敛属性测试（最小版提前为 3.2 的门禁）

- 不变量 P1（收敛）：任意操作序列 × 任意到达顺序 × 任意分裂初态（A 有 X、B 有 Y）→ 跑完交换（v1 全量与 v2 分页两种模式、两种角色分配）后两端 replay 状态相等（memberNodeIds / roles / policies）。
- 不变量 P2（单调）：合并只增不减，replay(after) ⊇ replay(before) 的已接受操作集合。
- 不变量 P3（等价性，供 3.4）：`project(log) == records` 对任意日志成立。
- 生成器：小文法（InitSpace/AdmitSpace/RemoveNode/DisconnectNode/PolicyUpdate/Leave，1–3 设备）+ 变异（重复、跨 origin 乱序、分裂前缀、第三方未知操作必须被拒）。
- 落点：`core/crates/node/runtime/tests/device_space/`（复用既有 `include!` 测试装配模式）；框架选型（手写随机 vs proptest）在 PR 里定，oracle 以本节为准。3.4 的投影泵等价性校验直接复用。

#### 3.8 n 成员记账泛化

等 #236（已合入）在真实多设备环境跑出分裂数据后再决定是否升级为周期性 gossip（可能由 Sync 服务顺路捎带，不新开通道），不预设计。

#### 3.9 清尾

§2.7 全部清单，拆四个微 PR：link-access 文档枚举勘误；cli-network-control 时限勘误（5s → `CONNECT_DEADLINE_MS = 30s`）；routing 文档降级为 plan（或按 §3.5 删除分支同步处理）；死代码删除 + 本文档勘误记录。

## 4. 能力变化 vs 实现变化（验收分轨）

判据：**对外契约变没变**（新操作/新状态/新保证/新参与者 = 能力变化；契约不变、内部重排、验收等价性 = 实现变化）。

| 条目 | 类别 | 备注 |
|---|---|---|
| doctor 只读诊断 | 能力（新增可观测面） | 系统第一次能自述病情 |
| doctor `--repair` | 能力（新恢复保证） | 坏一行不再全砖 |
| 读路径跳过旁路（3.1 前置增强） | 实现（效果=新保证） | 3.4 快照故事的硬前置 |
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
4. 诊断/修复工具自身的安全性按被修复对象同级对待（`--repair` 默认只读、可逆、备份、自校验；自校验等式用影子 replay 版，见 §3.1）。

## 6. 附录：存储与 wire 速查（设计引用的实现事实）

**同步日志根**（`RuntimeStorageLayout::RUNTIME_SYNC_PREFERENCES_PAYLOADS_DIR_PATH` 之下，`SyncOperationStore`）：

| 路径 | 内容 |
|---|---|
| `operations/{safeDeviceId}.jsonl` | per-origin 操作日志；每行一条 `SyncOperation` JSON（payload 可能是 `StoredEncryptedSyncPayload` 加密信封，AAD 绑定操作头）；按 sequence 字节偏移索引 |
| `clocks.json` / `devices.json` / `export_floors.json` / `local_device_id` / `entity_versions.jsonl` | 向量时钟、origin 注册表、导出楼层、本机标识、实体版本 |
| `runtime/link_access/space_policy.preferences.json` | 控制投影（控制操作第二副本，`orderedCommands` 合并，冲突即错） |
| `runtime/link_access/space_merge_{inbound,outbound,review_inbox,review_results}.preferences.json` | join 记录（本地墙钟，不入复制日志）；`space_join_*` 为已废弃旧 schema，leave 时删除 |
| `runtime/link_access/space_reconcile_caps.preferences.json` | （3.2 新增）peer 协议能力缓存 |
| `runtime/diagnostics/space_doctor_counters.preferences.json` | （3.1 新增）诊断计数器 |

**wire 面**（`node.space` 目标）：`snapshot` / `deviceSpace` / `observeSpaceSnapshot` / `observePairedDeviceSpace` / `requestJoin` / `joinStatus` / `cancelJoin` / `reconcileSharedSpace`；审批目标 `node.space.approval`：`assigned` / `claim` / `complete`。传输上限：受限 8KB / 默认 4MB，超限断连（非丢包）。

**确定性来源**：控制日志 replay 全序 = `(createdAt, originDeviceId, sequence)`；`sequence` 由 origin 分配、跨副本稳定——向量时钟因此可作增量游标。
