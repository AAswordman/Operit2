# 大量 SQL / preference / 文件积压：真实 CLI 补测（2026-11-16）

结论（日志与栈采样复核后更新）：已定位两处具体的重复工作。TTS 配置查重在循环中反复扫描、反序列化全部配置，新增一条的工作量近似平方增长；新增聊天附带的 binding 逐条应用，每条重写 `clocks.json` 并同步文件及父目录，是接收端应用阶段的主要采样热点。1000 次新增聊天原测约 20.54 秒排空，独立采样复测约 21.26 秒；1000 次同一聊天覆盖导入约 5.19 秒。

原混合场景的 39.86 秒，以及 preference 场景的可见时间，包含测试脚本反复调用 `getAllTtsConfigs` 的干扰。该查询也有嵌套全量扫描，不能将这些时间当成无干扰的同步基准，或据此精确推断 preference 排队等待。脚本已把完整配置校验移到排空后；本次只做了小规模验证，没有重新跑千级混合基准，也没有量化旧测试的查询干扰占比。

初始诊断阶段没有修改生产代码；随后按用户要求修复了 TTS 嵌套扫描与 binding 时钟重复落盘，修复和新测量见文末。本报告保留原始基准及其观察器限制。此前的结构、历史意图与小空间测试见 `performance-space-sync-2026-11-16.md`。

## 方法和适用范围

- 源码 HEAD：`483ba8d12617fe1b32311c465616fa583ea728e3`。
- 当前源码构建的 dev / unoptimized + debuginfo CLI：`apps/cli/target/debug/operit2`；二进制 SHA-256：`3d78906c55d8f611b8707cc5bdd9cbc112969df4a2fae15715d7de186493cff9`。
- macOS arm64，同机两个真实 CLI 进程，TCP loopback，真实配对与 Space 加入，隔离 config/runtime/workspace，关闭发现。测试不涉及日常身份数据。
- 所有 SQL 业务写入走 `chatRuntimeHolderMain.importChatHistoriesFromJson`；每次导入一个聊天和一条 4096 字节正文消息。它会执行多条 SQL，故“1000 次 SQL 事务”不等于“1000 条 SQL 语句”，也不代表任意插件数据库的速度。
- preference 走 `preferences.ttsConfigManager.createTtsConfig`，每次创建不同配置键，无远端语音或模型请求。文件为随机字节，各 1 MiB。
- 接收端 B 在准备积压时停止；A 完成业务写入、文件扫描与 outbox 发布后，按 B 原有 receipt 查询每个 domain 的实际待传操作数，再开始计时并重启 B，触发同步。
- 计时包括 B 启动（约 0.05～0.10 秒）、重连、同步、自动后续 pass；`drained` 是首次显式同步返回后，双方再次显式排空的时间。额外一次双端内容一致性检查单列，不混入 `drained`。
- SQL 可见性通过隔离 B 数据库的只读查询观察，最终逐条核对正文；preference 通过公开查询 API 核对配置；文件通过 SHA-256 核对。原测试每轮调用 `getAllTtsConfigs`，这会引入明显计算干扰，尤其是已有 600 个配置的混合阶段；当前脚本只在排空后执行一次全量配置校验，不再报告 preference 首条/全部可见时刻。SQL 和文件仍每 100 ms 观察一次，可见时间是观测上界。
- 主测试三组按 SQL → preference → mixed 顺序累积数据。混合前已有 1000 个聊天和 300 个配置，混合后共有 2000 个聊天和 600 个配置。同一聊天覆盖导入对照使用另一对新身份。
- 这是停止继续写入后的单向积压追平测试，没有测 release、移动端、真实网络 RTT、多跳、双向持续写入或多 peer 公平性。

## 测量结果

时间单位均为秒，从 B 重启前的计时点开始。

| 场景 | 实际待同步操作 | 本地准备 | SQL 全部可见 | preference 全部可见 | 文件全部可见 | 首次显式同步返回 | 双方排空 | 额外内容校验 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| 同一聊天覆盖导入 1000 次，最终保留 1 条消息 | chat 1000 + binding 1 + preferences 1 | 1.988 | 4.566 | — | — | 4.890 | **5.187** | 0.078 |
| 新增 1000 个聊天，各一条 4 KiB 消息 | chat 1000 + binding 1000 + preferences 1 | 17.084 | 18.230 | — | — | 19.354 | **20.537** | 0.971 |
| 创建 300 个不同 TTS preference 配置 | preferences 301 | 65.756 | — | 1.728 | — | 2.506 | **4.108** | 1.102 |
| 再新增 1000 个聊天 + 300 个配置 + 8×1 MiB 文件 | chat 1000 + binding 1000 + preferences 301 + runtime_file 8 | 387.539 | 26.349 | 36.611 | 32.966 | 36.618 | **39.860** | 2.198 |

每个场景多出的 1 条 preference 操作来自同时存在的内部偏好变化，不能把 301 说成 301 个测试配置。覆盖导入对照实际仍保留 1000 条待传 chat 事务，没有因最后只剩一条消息而缩成一次传输。

混合积压的可见进度：

| 数据类型 | 首次观测可见 | 全部观测可见 |
|---|---:|---:|
| SQL 聊天消息 | 3.307 | 26.349 |
| 本轮新增 preference 配置 | 21.353 | 36.611 |
| 文件 | 32.966 | 32.966 |

按排空时间折算，覆盖导入约 193 次业务导入/秒；新增聊天约 49 次业务导入/秒（包括附带 binding 的同步）；混合约 58 条待同步操作/秒并传输文件。这些是当前固定样本的端到端平均数，不能作为持续写入处理能力或网络带宽。

“本地准备慢”和“同步慢”要分别定位。第一组 300 个配置的创建用 65.76 秒；混合组准备总计 387.54 秒，其中 SQL 导入约 25.5 秒、第二组 300 个配置约 361.34 秒，其余为文件写入、扫描、库存查询等。这里包含业务校验、持久化、通知和后台工作，不能把它都计为对端传输耗时。混合从准备开始到排空约 427.40 秒，排空后的独立校验另需 2.20 秒。

## 日志显示的时间占用

新增 1000 聊天的主 pass 实际 19.246 秒：四次远端 `syncApplyOperations` 调用分别为 3637 / 3989 / 4390 / 4442 ms，合计 16.458 秒，约占该 pass 的 85.5%；尾部完整一致性检查 988 ms，corrections=0。同步的应用调用包括接收、验证、owner 路由、业务落盘及相关工作，不能据此断言 16.458 秒全部是 SQLite 执行。

混合主 pass 实际 36.355 秒：五次远端应用调用为 5004 / 5488 / 5924 / 6919 / 3529 ms，合计 26.864 秒；尾部一致性检查 2532 ms，corrections=0。随后自动 pass 又执行 3.313 秒，其中一致性检查 2200 ms。这个跟进 pass 还出现一次约 0.81 秒的应用调用，不能把整轮简单标成“完全无变化的重复校验”。

同一聊天覆盖导入的两次远端应用调用为 1653 / 1752 ms，主 pass 尾部一致性检查 80 ms。对照与新增聊天同时改变了 binding 数量、SQL 最终数据量和业务操作形态，不能用 20.54−5.19 秒来断言单独某项成本。

因此，前一份小空间报告的 18～27 ms 校验成本不能推广到大量数据。owner 数量仍为 26，但数据量增长使健康主 pass 的校验成本达到约 1～2.5 秒。与此同时，本次新增聊天和混合场景中更大的耗时仍在应用阶段。

## 已定位的具体原因：日志与短时调用栈采样

原日志将时间定位在 `syncApplyOperations`，但没有 owner 内部计时，不能把这段直接归为 SQLite 执行耗时。补充 macOS `/usr/bin/sample` 的 8 秒采样后，可以进一步定位。下面的比例是采样窗口内的调用栈占比，包含阻塞等待，不是整轮精确耗时比例或 CPU 占用比例。

### TTS 创建和全量查询存在嵌套全量扫描

`core/crates/runtime/application/src/data/preferences/TtsConfigManager.rs:548` 的 `assertTtsConfigVoiceDoesNotExist` 遍历所有配置 ID，每个 ID 又调用 `readExistingTtsConfig`（524 行）。后者调用 `readConfigList`（480 行），重新遍历所有 preference 条目、反序列化每个 TTS 配置并排序。因此已有 N 个配置时，新增一条的查重会产生约 N² 次配置反序列化，并有重复排序；连续创建 N 条时累计工作量近似立方增长（另有排序成本）。已有 600 条时，单次新增就约有 36 万次重复解码。

`getAllTtsConfigs`（109 行）也先取得 ID 列表，再对每个 ID 调用上述读取逻辑，存在同样的嵌套扫描。这解释了为什么测试观察器会干扰混合同步。

在主轮 A 最终数据的隔离副本上，新增 6 条配置并采样，无对端参与；每条约 2 秒，5887 个创建调用样本中约 95% 进入查重。后台 `publishPreferenceOutboxes → publishOutbox → lockTransaction` 主要阻塞在事务锁上。主轮后 300 次配置创建耗费约 361 秒，属于这个业务 API 的本地准备成本，不能归为 preference 网络传输，也不能据此声称所有 preference 写入都一样慢。

### 为什么在 TTS 暴露，以及已核对的范围

这轮 preference 工作负载选用了 TTS 配置创建，因此放大了 TTS 业务 API 的问题；这不是每种 preference 同步都必经的 TTS 调用。

历史上，`readExistingTtsConfig` 在 2026-06-22 已先读取独立 ID 列表检查成员，2026-07-04 的查重循环复用这个读取方法。2026-10-14 的 `9ae1e558a6` 将偏好迁移到原生 JSON，并将 `readConfigList` 改为从完整记录派生成员列表、校验记录 ID，移除独立列表键；旧的逐条读取和查重调用链仍然保留。原本的列表读取因此升级为完整配置扫描与解码，嵌套调用放大了成本。由完整记录派生成员关系的设计可以保留，问题在于每读取一条又重复派生一次。

本次静态对照了同目录的 STT、模型和主题管理器：STT 的 `readExistingConfig` 直接按键取记录；模型的 `loadProviderFromDataStore` 也直接按键读取，全量列表会额外解码但没有逐条再次全量扫描；主题的 `readState` 一次读出记录集合后复用。公共 Preferences 快照以 `Arc<HashMap<...>>` 共享，复制快照本身不会逐条深复制所有配置。上述检查未发现同样的 TTS 嵌套扫描链，不等于已证明其他模块不存在性能问题，也没有进行全仓穷尽审计或这些模块的大规模实测。

### binding 逐条重写同步时钟并刷盘

`core/crates/runtime/application/src/core/application/SyncConsistency.rs` 按 owner 应用操作时，对 binding 逐条调用 `CoreNodeBindingStore::applySyncedOperation`。其 `applyOperation` 在 `core/crates/persistence/store/src/CoreNodeBindingStore.rs:242` 调用单条 `appendOperation`；进入 `SyncOperationStore.rs:1919` 的 `appendOperations` 后，每次都会 `writeLocalClockValue`，即使时钟没有变化也没有跳过写入。

最终路径为 `writeLocalClockValue → writeJson → writeBytesAtomically → commitInternal → file.sync_all`，随后原子替换文件并 `syncParentDirectory → sync_all`。实际文件名为 `clocks.json`。此前口头更新中的单数 `clock.json` 是笔误。

独立复跑 1000 次新增聊天（1000 个 chat 操作、1000 个 binding 操作，接收端没有 TTS 配置），约 21.26 秒排空，接近原来的 20.54 秒。8 秒采样窗口内，4977 个同步应用栈样本中，3311 个在 binding 路径，约占三分之二；其中 2944 个落在时钟持久化路径，约占应用栈样本的 59%。文件和父目录同步是明显的等待叶节点。另有 applied-operation 元数据持久化等成本，不能把全部应用时间都算成时钟文件。

因此，之前“应用阶段花了 16～27 秒”的描述过于粗；至少在已独立复现的千条新增聊天场景中，binding 的重复同步落盘是主要热点，不能笼统称为 SQL 慢。覆盖同一聊天只产生一个 binding，与其更快的现象相符，但两组仍不是只改变 binding 数量的严格单变量对照。

采样产物：

- TTS：`/tmp/operit-space-stress-tlsaefw5/preference.sample.txt`，运行日志 `/tmp/operit-sync-hotspots-profile20261116.log`。
- SQL/binding：`/tmp/operit-space-stress-ppmyk2lt/sql-apply.sample.txt`，运行日志 `/tmp/operit-sync-sql-profile20261116.log`。
- 原主轮接收日志：`/tmp/operit-space-stress-nzauxuye/B/runtime/identities/df64d21c692d467eaf1ff8eddc6803b4/logs/operit.log`。

## 当前调度为什么会产生等待

- 六个 domain 共同分页，常见每页最多 512 条，同时有默认 4 MiB 序列化字节预算。不是为 SQL、preference、文件各开一条独立同步通道。
- journal 与 SQL source 的操作合并后按事务顺序选页。`transaction/Kernel.rs` 在已进入页面的 owner 之间优先应用 preference，这不意味着后面页面里的新 preference 能越过前面的积压。
- `SpacePersistenceSyncService.rs::applySynchronizedOperationBatch` 先等待本页依赖 Blob 的补齐，再应用本页业务操作。保证了文件依赖的正确性，也把同页非文件操作的可见时刻与文件传输耦合。
- 文件并发为 8 个 Blob，每块 64 KiB；单个远端源 Blob 的读取块仍是顺序 RPC。并发上限不是“整个同步所有步骤都能八路并发”。
- 同一 peer 有同步执行锁，页面顺序推进，最后还要校验内容。本轮混合日志确有后续 pass 等待前一个 pass 约 36.349 秒的记录；两节点场景没有证明其他 peer 饥饿。
- 64 页仅限制内层循环，最终校验不稳定时外层仍可重试。本轮静态积压完成后均正常退出，没有复现持续写入时无界持锁的问题。

代码定位：`core/crates/node/runtime/src/SpacePersistenceSyncService.rs` 的 `SYNC_DOMAINS`、`synchronizeDomains`、`applySynchronizedOperationBatch`、`synchronizeBlobReferences`；`core/crates/persistence/store/src/transaction/Kernel.rs::export/apply_pending`；`core/crates/persistence/store/src/SyncOperationStore.rs::DEFAULT_SYNC_PAGE_BYTES`。

## 基于这轮数据的决定

1. 先修 TTS 的嵌套扫描：一次读取和校验配置，复用解码结果查重；全量查询也只扫描一次。保持身份校验、规范化及错误行为，不能以跳过校验代替消除重复工作。这是本地创建慢的确定问题。
2. 同步应用优先处理 binding 的逐条元数据落盘：评估按安全的应用批次持久化时钟及应用标记，跳过确实未变化的时钟写入。原始接收记录已在 `receiveOperations` 批量存储，但“已接收”与“已应用”仍需严格区分，不能直接提前标记成功。需要以崩溃恢复、重复重放和冲突语义验证批处理边界，不能全局删除 `sync_all`。
3. 调度、Blob 二进制载荷、外部文件 watcher 和秒级内容校验仍有优化价值；现有证据支持先消除上述重复工作，再用无配置查询干扰的同规格积压重新测量，而不是先放宽正确性边界。轻量操作能否越过积压还要保留事务顺序、因果/alias 和文件依赖，不能任意乱序。
4. 旧混合场景 39.86 秒和约 21 秒首条 preference 可见时间仅保留为原观察器下的记录。去除查询干扰后尚未进行同规格混合重测，不给出推算的新速度。

保留不可变事务、接收与应用分离、授权、文件 hash/长度校验、提交和最终内容校验。这轮没有证据支持通过放宽正确性边界换取速度。

## 产物与复跑

- 千级主轮：`/tmp/operit-space-stress-nzauxuye`，总计 543.198 秒，含全部本地准备、同步和独立校验。每端 pass 均 7 次开始、7 次完成、0 次失败。
- 同一聊天覆盖导入：`/tmp/operit-space-stress-ammb1swl`，总计 11.213 秒。每端 pass 均 3 次开始、3 次完成、0 次失败。
- 每组最终 26 个 owner 的内容 digest 相等，双方 errors / pendingOwners / skipped 为空；SQL 正文、全部新增配置和文件 hash 校验通过。
- 主日志：`/tmp/operit-sync-backlog-full-20261116.log`；覆盖导入日志：`/tmp/operit-sync-backlog-replace-20261116.log`。目录内有 `events.jsonl`、`backlog-results.json`、双方 CLI 会话和应用日志。

```sh
python3 apps/cli/tests/sync_backlog_session.py \
  --binary apps/cli/target/debug/operit2

python3 apps/cli/tests/sync_backlog_session.py \
  --binary apps/cli/target/debug/operit2 \
  --sql-shape replace --scenarios sql
```

脚本在 finally 中结束本次创建的全部 CLI 进程，保留隔离日志与数据用于复查。

最终脚本另以 10 次 SQL 导入、5 个配置、2×64 KiB 文件完整复跑三组场景通过；日志 `/tmp/operit-sync-backlog-final-smoke-20261116.log`。


观察器修正后，用 10 次 SQL 导入、5 个配置、2×64 KiB 文件重新跑完三组真实 CLI 场景，6.953 秒全部通过；最终 26 个 owner 内容一致，错误/pending/skipped 为空，配置和文件内容核对通过。产物 `/tmp/operit-space-stress-zghm8zdr`，日志 `/tmp/operit-sync-backlog-observer-smoke-20261116.log`。这只是脚本正确性验证，不替代千级性能重测。


## 按用户要求修复（2026-11-16）

TTS 新增 `readConfigRecords`，一次解码、检查记录 ID 并排序。查重直接遍历这些记录并执行原有规范化；全量查询在同一个偏好快照上复用记录，不再逐个调用会扫描全库的单条查询。保留记录身份错误、无效配置报错、排序、规范化及更新时排除当前配置的行为。单条查询仍校验其他记录的身份，以保留原有可见错误语义。

Binding 在原始操作持久化后、各 owner 应用前，通过 `prepareSyncedOperations` 批量发布有效 binding 的时钟。`appendOperations` 增加实际变更检测，各 owner 后续应用和重放不再重复写未变化的 `clocks.json`。保持现有 owner 路径及独立应用结果，不合并各 owner 的成功状态；准备阶段不写已应用标记、不清 pending，不删刷盘。无效 binding 留给自己的 owner 报错。每条应用仍有原始记录检查和 entity-version 等持久化，不能声称消除了所有逐条开销。

验证：CLI 离线 dev 构建成功；现存 CLI 锁文件与依赖声明不一致，构建临时更新后已按原始字节恢复。TTS 单元测试 3 项、binding 批次/写入失败恢复/无效 owner 隔离测试 3 项、分页字节预算测试 1 项、事务业务契约测试 32 项，共 39 项通过。Binding 测试实际统计时钟写入次数，批次准备写一次，随后逐条应用和重复重放均不增加时钟写入；注入时钟写入失败后，新 host 重开保留原始 pending 并能完成重试。

构建日志 `/tmp/operit-sync-fix-build-20261116.log`；测试日志 `/tmp/operit-sync-fix-tts-tests-20261116.log`、`/tmp/operit-sync-fix-store-tests-20261116.log`。构建二进制 SHA-256：`efffdb2ea22a5107fbc4c501d0ce96ca6b82c402b134b7c03632766d5d28130a`（原 HEAD 加本轮工作区改动，dev / unoptimized + debuginfo）。


修复后以相同的千级积压参数重新跑完 SQL → preference → mixed 三组，改正后的观察器不在同步计时内枚举 TTS 配置。无编译或其他压测并行运行。总计 163.204 秒，包括全部准备、同步和独立校验。三组均成功，双方最终 26 个 owner 内容 digest 一致，errors / pendingOwners / skipped 为空，新增 SQL 正文、配置及文件 hash 均核对通过。

| 指标（秒） | 修复前 | 修复后 | 解释 |
|---|---:|---:|---|
| 新增 1000 个聊天，本地准备 | 17.084 | 16.904 | 输入业务形态相同 |
| 新增 1000 个聊天，同步排空 | 20.537（独立采样轮 21.260） | **11.300** | 比原主轮约减少 45% |
| 首批 300 个 TTS 配置，本地准备 | 65.756 | **20.396** | B 停止时的创建及发布准备，排除对端传输 |
| 已有 300 个配置，再创建 300 个配置的 API 循环 | 361.336 | **约 44.460** | 由同一工作负载进度事件计算，约减少 88% |
| 混合场景全部本地准备 | 387.539 | **71.261** | 包括 1000 次 SQL 导入、300 个新配置、8 MiB 文件及发布 |
| 混合同步排空 | 39.860（受旧观察器干扰） | **29.118** | 同时有代码修复及观察器修正，不能把全部降幅归因于生产修改 |

修复后混合场景 SQL 全部可见 16.501 秒，文件全部可见 22.948 秒，首次显式同步返回 25.954 秒；独立校验另计 2.270 秒。仅 preference 的排空为 3.889 秒，原轮为 4.108 秒，但原轮有配置查询干扰，不据此声称传输改善百分比。剩余同步成本包括原始操作检查、各 owner 应用和应用标记持久化、文件传输及最终内容验证，本修复不承诺消除这些成本。

千级产物：`/tmp/operit-space-stress-6paik3qd`，日志 `/tmp/operit-sync-fix-backlog-full-20261116.log`。

另在原主轮已有 600 个配置的 A 数据隔离副本上复测，新增 6 条各耗时 0.182～0.228 秒（旧采样轮约 2 秒/条）；607 条配置的完整查询耗时 0.089 秒。该副本没有活跃对端，旧轮同时开了栈采样，新轮没有采样，故这个单条比较用于确认数量级改善，不视为严格等观测条件的精确加速比。产物 `/tmp/operit-space-stress-pv7mmauo`，日志 `/tmp/operit-sync-fix-tts-existing600-20261116.log`。


最终收尾补充了“仅存在 binding pending 时才执行时钟批次准备”的入口判断，避免纯 preference 和空队列额外打开 binding store。该判断在上述 SQL/mixed 基准中为真，处理逻辑一致；最终二进制另做小规模三组 CLI 验证。最终二进制 SHA-256：`beafbbbc551884a60c7ffad897a76f381b2697f2d1b0b58f041bf6cf4294cdb5`，构建日志 `/tmp/operit-sync-fix-build-final-20261116.log`。

最终二进制小规模三组真实 CLI 验证全部通过，11.869 秒；产物 `/tmp/operit-space-stress-4ylgqq1v`，日志 `/tmp/operit-sync-fix-final-smoke-20261116.log`。全部本轮 CLI 测试进程均已退出，临时隔离产物保留。
