# 设备空间 Rust 契约测试

本目录集中设备空间、审批、撤回、权限、多跳路由、持久化与同步边界测试。
**测试文件存在、语法检查通过，不等于行为测试通过，更不代表所有边缘场景已经覆盖。**

## 运行

在仓库根目录执行（默认只做登记、模块接线、记录路径和 Rust 语法检查，不编译）：

```powershell
./core/crates/node/runtime/tests/device_space/run.ps1
```

明确执行 Rust 行为测试（此命令会编译测试目标，忽略 Rust 警告）：

```powershell
./core/crates/node/runtime/tests/device_space/run.ps1 -Run
./core/crates/node/runtime/tests/device_space/run.ps1 -Run -Repeat 10
```

等价 Cargo 过滤器为 `-p operit-node-runtime --lib device_space`。
必须使用 `--lib`：这些源文件由原模块通过 `include!` 接入，既可测试私有协议边界，
又不必为了测试扩大生产 API 的公开范围。所有用例名称登记在 `coverage.json`。

## 覆盖矩阵（50 个测试函数，部分内部遍历多个故障/顺序组合）

| 文件 | 验证内容 | 层次 |
| --- | --- | --- |
| `join_lifecycle.rs` | 独立管理员申请；拒绝；重启 facade；重复批准；退出与重新申请；两申请者隔离；提交响应丢失 | 实际 facade/router + 独立 Host 存储 |
| `cancellation.rs` | 撤回后两端磁盘记录；B 审批列表消失；旧审批失效；业务/身份/配对文件不变；提交前失败、处理后丢响应；撤回确认丢失；延迟刷新；断线重试；claim 与撤回两种顺序；连续 12 次撤回重申请 | 实际协议 + 可控传输 |
| `reviewer_assignment.rs` | 最近合法审批人、非审批设备不弹窗、离线宽限与转移、旧 assignment 失效 | 多节点实际路由 |
| `protocol_boundaries.rs` | 错误身份/空间/revision/profile；第三端撤回；申请端管理员不能自批；过期；目标换空间；相反审批决定冲突 | 身份与授权入口 |
| `space_merge.rs` | AB 与 C-D-(E,F)-G 整组合并；逐跳迁移；profile/权限/拓扑完整；不完整快照不写成员 | 实际空间投影交换 |
| `routing_contracts.rs` | 环路多跳；网络分区/重连；撤销中继权限；TTL；目标移除但连接仍存在；Binding 所有者迁移与旧写冲突 | 实际 router + Store |
| `policy_contracts.rs` | 权限日志倒序、轮转、重复投递；伪造 issuer；最后管理员保护；审批权限变更 | 实际权限重放 |
| `persistence_contracts.rs` | 二进制文件 A-B-C；删除防旧数据复活；缺块、截断、错误哈希；同时写与所有两事件顺序；路径越界；未加入/撤回端不可读取同步日志 | 实际 Store + 路由拒绝 |
| `transports.rs` | TCP 配对/回向鉴权；客户端单向 HTTP/WebSocket；监听能力原子校验；发现与配对过滤 | 真实 Host 传输 |
| `join_state_machine.rs` | 全终态×全响应组合、过期边界、已 claim 不自动过期、定向中继距离 | 纯状态机 |
| `facade_state.rs` | 加入投影验证、观察订阅释放、连接状态映射 | facade 状态 |

## 每次撤回必须核对的事实

- A 的 `space_merge_outbound.preferences.json` 和 B 的 `space_merge_inbound.preferences.json` 状态。
- B 的 `incomingDeviceSpaceJoins()` 不再返回已取消项；持有旧弹窗也不能成功审批。
- review inbox 是已拉取记录的存档，不把“存档还在”视作仍可审批；权威状态在 inbound。
- RESULT_RECORDS 不应出现已取消申请的新批准结果。
- A/B 的 Space id、成员文件、设备资料、用户资产、身份与配对凭证保持原始字节。
- 取消未到达 B 时不能声称 B 已取消；收到 B 的确认后，旧轮询不能复活 Pending。
- claim 已先提交时，当前协议禁止撤销已占用的决定；测试明确断言 Approving，而非假装 Cancelled。

申请记录路径与生产常量由结构检查核对，避免协议换版本后测试读取旧路径而漏检。
不读取开发者的真实运行目录；每个节点使用独立测试 Host。替换全局 Host 的用例持有统一锁。

## 故障模型

`fixtures.rs` 的链路实际调用目标 router，只在明确的位置注入：

1. 请求送达前失败；
2. 目标处理完成、响应丢失；
3. 目标处理完成、响应由 oneshot 闸门暂停；
4. 显式移除/恢复某条活动连接。

并发用例用闸门控制交错，不用随机 sleep；超时用于防止死锁使测试无限挂起。
权限重放遍历多种投递排列，文件冲突固定同一事件时间来检验 origin-id 决胜规则。

## 不能混淆的覆盖边界 / 发布前仍需补齐

- `persistence_contracts.rs` 调用真实 Blob/Operation/Preferences 基础设施，
  但并未启动完整 `OperitApplication.syncApplyOperations` 或后台 `synchronizeOnce`。
  **A-B-C Store 的文件传播不等于完整多跳网络文件同步通过。**
- facade 重建验证磁盘记录再读取，不等于清空进程缓存后的真实进程重启。
- 尚缺：每一个持久化写点失败/进程被杀、磁盘满、部分 fsync、全部跨文件原子性。
- 尚缺：双向同时合并两个空间、两个不同目标同时批准同一源空间、合并中源成员增加/退出/移除的完整冲突矩阵。
- 尚缺：真实应用服务下的大文件分块中断续传、同步批次边界、空间切换时增量时钟与旧文件隔离。
- 尚缺：跨版本协议、Web/移动后台挂起/弱网、真实设备时钟偏移与多进程重启。
- UI 另外保留 `apps/flutter/app/test/space_join_dialog_test.dart`：慢轮询可撤回、迟到响应、错误提示。

任何上述缺口不能通过跳过错误、伪造 profile、跳过测试或把权限校验移除来“修绿”。
行为回归失败应保留失败证据、修生产根因，并补对应不变量。

## 最近验证（2026-10-25）

- 结构检查：21 个 Rust 文件、97 个登记测试，通过。清单已补齐实际接入的用例。
- `cargo test --locked --offline --manifest-path core/Cargo.toml -p operit-node-runtime --lib device_space -- --test-threads=1`：
  **98 通过、0 失败、0 忽略**。过滤器也选中本目录之外的空间相关用例，因此实际数量略多于清单。
- 新增 Core 经中继同步权限的回归，确保 endpoint 协商不要求间接 Core 额外建立直连配对。
- 权限重放只读已提交控制日志，不再在配对检查时发布业务 preference outbox。
  取消申请的字节级持久化断言继续保留；文件写入边界测试在准备阶段显式读取时钟后再截取基线。
- 这些结果不代表 Core 托管 Edge 授权或 ESP32 真机迁移已验收。

## 历史验证（2026-10-06）

以下是当时的失败记录，不能作为当前测试结果。

已按用户要求实际编译并运行 Rust 行为测试，使用 `-Awarnings` 忽略编译警告。

- 结构检查：13 个 Rust 文件、50 个登记测试，通过。
- `cargo test -p operit-node-runtime --lib device_space -- --test-threads=1`：
  **50 通过、1 失败、0 忽略**。过滤器额外选中了原有的空间观察测试，因此实际执行 51 个。
- 失败：`policy_replay_converges_with_duplicate_reversed_and_rotated_delivery`。
- 对刚编译的测试二进制单独重复执行该用例 3 次，3 次均失败（退出码 101）。
- 完整日志：`latest-run.log`；单例重复日志：`policy-repro.log`。

首次执行暴露的测试自身问题已修正：拓扑 fixture 使用序列化接口而非访问私有字段；
Binding 测试使用真实 Store；本机 reviewer claim 进入本机审批 dispatcher，不向自己发网络请求。
没有跳过或放宽业务断言，没有修改生产代码来掩盖剩余失败。

剩余失败的代码原因：`NetworkControlStore.applySyncedOperation` 调用
`SyncOperationStore.appendOperations`；后者在接收较高 sequence 并推进 origin 时钟后，
会丢弃随后抵达的、日志中尚不存在的较低 sequence 操作。
本用例的 Bootstrap 因乱序被丢弃，权限重放结果 `initialized=false`。
这是当前持久化/权限日志对乱序输入的真实不收敛问题；测试证明该输入路径存在问题，
不代表已确认此前某次真实用户反馈就是由此引发。

`coverage.json` 是用例清单，不是通过报告，也不是代码覆盖率报告。
