# 手机聊天样式在气泡式与命令式之间反复切换

## 结论

2026-11-22 对已连接的 PLK110（`app.operit`，versionCode 49）进行现场取证。
问题发生在同步后的持久化主题配置：手机其他偏好 owner 保存的过期主题身份别名，
会重新进入用户偏好的合并过程，删除命令式主题并将当前选择改为气泡式主题；
Windows 的一致性快照矫正或初始化事务又恢复命令式配置。

已用真实数据库副本和主题配置离线验证：应用这条旧别名并执行当前主题投影，
得到的完整业务配置与现场捕获的气泡式配置完全相等；两种配置的摘要与同步日志完全一致。
现场取证阶段没有修改业务代码、设备设置或运行中的数据库；后续源码修复见本文末节。

## 现场状态

- 手机：PLK110，ADB 序列号 `3B15AE01MYL00000`；`app.operit` 2.0.0，49，
  最近安装更新时间 2026-11-22 14:34:12。
- 手机节点：`core-1795329490415-fa6dc3831494403eac510deec8099b0a`。
- Windows 节点：`core-1789368866635-838e5f2bc3f04c99a8d056c0dfefd31d`，显示名称 `windows-剑`。
- 数据 owner：`runtime/config/preferences/user_preferences.preferences.json`。
- 手机存储根：`files/runtime/identities/identity-1795294876920-3233cee5/`。

| 状态 | 选中主题 | 用户/AI 样式 | 主题数 | 业务 JSON 字节数 | SHA-256 前缀 |
| --- | --- | --- | --- | --- | --- |
| 命令式 | `888c63fe-ee87-4fcf-bec1-8426a394d6e2` | `cursor` / `cursor` | 3 | 12018 | `a206f827c7f2` |
| 气泡式 | `655b786c-9e5f-4e00-970b-58ac7a155929` | `bubble` / `bubble` | 2 | 8972 | `73b74f6be823` |

15:20:44.784 的手机文件为气泡式。15:21:18.393 捕获到包含提交意图的命令式文件，
15:21:18.612 捕获到提交意图清理后的命令式文件。此间读取使用 `adb exec-out run-as ... cat`。
为观察恢复同步后的实际写入，取证过程中将已有 MainActivity 带回前台。

## 同步日志

Windows 是该设备对的协调端，因此关键一致性矫正日志保存在 Windows 的 `operit.log`，
手机日志主要记录同步 pass 和客户端主题分发。

Windows 日志在 14:44:13、14:46:10、14:46:27、14:48:18、14:53:51、14:58:47、
15:03:48、15:06:34、15:11:37 都记录了同一用户偏好 owner 的完整快照矫正。
后续稳定重复的差异为：

```text
sync_consistency.correct ... mode=full_snapshot
owner=runtime/config/preferences/user_preferences.preferences.json
reason=content-size source=Local
localDigest=a206f827c7f2 remoteDigest=73b74f6be823
localContentBytes=Some(12018) remoteContentBytes=Some(8972)
```

两端修订均为：

```text
createdAt=1795329892072
originDeviceId=core-1789368866633-b2c5d3c1b2da9994:preferences
sequence=1812
```

随后有 `sync_consistency.applied`，但后续 pass 又出现相同摘要分歧。
手机客户端的主题分发与这些覆盖写入时间对应，例如 15:11:38 和 15:06:36。

## 根因与代码链路

在手机 `transactions.sqlite` 的 `transaction_owners.state.current.data.aliases` 中，
20 个其他 owner 保存着如下映射：

```text
runtime/config/preferences/user_preferences.preferences.json\u001f888c63fe-ee87-4fcf-bec1-8426a394d6e2
    -> 655b786c-9e5f-4e00-970b-58ac7a155929
```

这些 owner 包括 Space 成员、设备资料、拓扑、角色卡和工作区偏好。
用户偏好自身的已矫正 checkpoint 不含这条映射；同时读取的 Windows owner 状态中也没有该映射。

修复前实现中的传播路径：

1. `core/crates/persistence/store/src/transaction/Kernel.rs` 的 `Registry::apply_pending`
   收集成功 owner 返回的全部 `aliases`，再将结果传给其他 owner 的 `project`。
2. `core/crates/persistence/store/src/PreferenceMerge.rs` 的 `applyDeclaredOperations`
   将 `externalAliases` 全部合入，并用 `setMergeAliases` 存入当前 owner。
   这使主题别名可以被与主题无关的 owner 保存、再次输出。
3. 同一函数按照别名删除源主题记录，通过 `rewriteReferences` 修改 `active_theme_config_id`，
   再将被选中的 `snapshot` 投影到 `user_chat_style`、`ai_chat_style` 等普通外观字段。
4. 该变更无需新的用户操作或更高业务修订，即可使手机回到气泡式。
   一致性矫正恢复用户偏好后，其他 owner 中保存的旧映射仍可再次参与后续同步。

此处可以确定的是别名的跨 owner 持久化与再次传播，以及它对本次样式变化的完整解释。
本次未确定最初创建这条主题合并别名的具体操作时间。

## 用户补充与主题来源边界

用户确认从未手动切换到气泡式，另外两个端当前都使用命令式。
本报告中的“旧别名”只描述同步内部保存的身份映射，不表示用户曾经选择气泡式，
也不能将这次问题归因为用户在不同设备上配置了不同样式。

已有主题目录中的 `655b…` 名为 `Default theme`，其记录时间为
2026-11-10 23:28:58 创建、2026-11-19 03:49:22 更新；保存的快照包含气泡式。
SQL 范围版本将该记录的最后一次写入归于 Mac 节点的 preferences 序号 2179。
这只证明存在这份保存的记录及来源节点，不能说明该端当前选用了它，更不能说明它由用户手动创建或修改。

`888c…` 的创建时间为今天 14:38:10.498，与手机 runtime 初始化同一时刻；
`ThemeAppearanceMigration::migrate` 调用 `migrateUnboundAppearance`，会自动创建和选择
名为 `Default theme` 的主题。主题目录中的记录和当前使用的外观必须区分。
当前 `PreferenceMerge::applyDeclaredOperations` 也会根据目录内容自动生成身份别名，
生成或消费别名不要求用户点击样式选项。

本次保存的历史原文已被初始化事务覆盖，未保留最初产生该别名及气泡式字段的完整操作链。
因此，气泡式配置最早如何生成仍待历史证据确认；已证明的故障是旧别名自动重选这份配置，
与用户没有手动使用过气泡式、其他端当前均使用命令式的反馈一致。

## 离线验证

证据保存在当前诊断工作区：

```text
C:\Users\12809\Documents\ChatGPT\operit2\diagnostics\phone-style-20261122\
```

主要文件：

- `operit.log`、`operit.log.1`、`client.log`：手机持久化日志。
- `windows-operit.log`、`windows-operit.log.1`：取证时复制的 Windows 日志。
- `transactions.sqlite`、`transactions-live.sqlite`：手机同步数据库的只读分析副本。
- `user_preferences.json`：命令式配置副本。
- `live-1030f6301d43.json`：现场气泡式配置。
- `live-preferences.jsonl`：文件状态采样时间线。
- `snapshot-local.json`：从手机同步 blob 中读取的 Windows 完整矫正快照。
- `verify_alias_projection.py`、`alias-projection-verification.json`：离线验证及结果。

验证命令：

```powershell
D:\Code\prog\assistance2\.venv\Scripts\python.exe C:\Users\12809\Documents\ChatGPT\operit2\diagnostics\phone-style-20261122\verify_alias_projection.py
```

验证结果：20 个旧别名副本；执行旧别名、主题选择改写和字段投影后，
完整配置与现场气泡式文件完全相等。业务摘要从
`a206f827c7f2aab021cbc00c0b12f45e41a8add35993d8a8aafe61853a19c49e`
变为 `73b74f6be8237e6cd1b1a719bca3f335430da3e2fdc9669fdd89855267537472`。

## 根因边界与源码修复

本质是身份决定的发布归属错误。已有别名键包含 `ownerPath + U+001F + identity`，
但合并函数将整个持久化别名表作为当前 owner 的决定返回给内核。
这张表同时保存自身身份历史和外部引用映射；外部映射再次被发布，才使历史副本能够反过来改写目标 owner。
本地持久化引用映射本身有用途，需要保留其合法消费者语义。

### 历史提交核对

| 提交 | 已建立的行为 | 本次处理 |
| --- | --- | --- |
| `cc89821aa`，2026-10-19 | 持久化别名，重定向旧会话写入，打开插件库时修复孤立行。 | 保留引用缓存和旧 ID 写入能力。 |
| `b42d23314`，2026-10-30 | 统一事务内核，并加入持久化别名表与外部别名合并、保存及返回内核的组合。 | 修正偏好参与者输出的 namespace 归属。 |
| `bba3cb403`、`0c956c09c`，2026-10-31 | 限制同步文件归属，合并独立初始化 checkpoint。 | 文件归属之外，身份决定也必须遵守归属；历史初始化通过相同输出边界。 |
| `24c382083`、`506ba7c8d`，2026-11-14 | 完整校正、精确覆盖 pending/skip 身份、校正后验证。 | 保留既有校正、CAS 和覆盖规则。 |
| `a361a004e` 至 `191ec4da8`，2026-11-18 | 已覆盖的 opaque 历史在空交付时复用当前状态，按声明的实体和引用检查。 | 保留 shortcut，其返回值同样只发布自身别名。 |
| `a75cb9874`，2026-11-18 | 接收端缺失已封存原件时才发送完整 SQL checkpoint。 | 保留既有传输门槛。 |
| `13167592f`，2026-11-19 | 强制校正使用业务大小、业务修订及节点顺序选源。 | 保留，这解释了现场命令式完整状态反复胜出。 |
| `82429518b`，2026-11-20 | 连接和同步统一使用协议 v26。 | 为本次合并语义修复追加 v27。 |

10 月 30 日以前已有跨 owner 传递外部别名的代码；上述差异能够确认当前持久化与再次发布的组合，
不能据此认定最初主题别名的创建原因。

### 实现

修改集中在 `core/crates/persistence/store/src/PreferenceMerge.rs`：

1. 入口按现有 namespace 过滤外部输入，排除属于当前 owner 的外部副本。
   当前 owner 的身份决定来自自身 checkpoint 和自身历史，不能被消费者保存的副本覆盖。
2. 普通合并仍保存完整引用映射，完成引用改写和主题投影后，返回内核的 map 只包含自身 namespace。
   空交付 shortcut 执行同样的输出过滤。
3. 外部输入只能影响引用，删除 shortcut 中对本 owner 目录的冗余外部检查。
   是否吸收外部决定直接检查声明的引用值，避免仅凭缓存中有相同映射就判定已经吸收。
4. 引用改写直接操作当前 owner 的私有事务视图，删除仅包着单个 owner 的 map 和整份 Preferences 克隆。
   文件发布和失败回滚仍由既有 owner 提交机制负责。

这些边界同样作用于已保存的旧缓存、旧初始化重放和空交付，因此历史副本无需先被物理清空，
就会失去重新发布身份决定的能力。原始事务、opId、fingerprint、收据和合法别名继续保留。
已经被错误别名改写的主题状态通过现有完整校正恢复；此前 20 份外部副本不能再把它改回去。

协议账本追加 v27，要求参与同步的 Android、Windows、Mac 使用一致语义。
主题 UUID、样式值、平台顺序及强制校正选源规则均由原有结构处理。

### 回归与代码量

扩展 `plugin_preferences.rs` 中现有的跨原生 owner / 插件引用测试，覆盖：

- 合法别名合并后，插件继续持久化外部引用映射。
- 原生 owner 完整校正，撤掉旧别名并恢复内容不同的源主题。
- 重新接收包含旧缓存的初始化，并再次进行空交付；输出只能包含发布 owner 的 namespace。
- 原生 owner 的完整 Data（含业务值、身份映射、版本和删除事实）仍与校正结果相同。

业务源码净减少 8 行；回归测试净增加 23 行；协议账本增加 4 行，代码及协议合计净增加 19 行。
文档调整单独统计，未用空白压缩、无关代码删除抵消本次代码量。
验证通过：`plugin_preferences` 25 项、`preference_migration_context` 5 项、
`preference_snapshot_recovery` 12 项、`preference_sync_versions` 10 项、
`transaction_checkpoint_laws` 9 项、`plugin_storage` 的 alias 相关 6 项，共 67 项。
`tools/check_peer_protocol.py --base HEAD` 通过，确认 v27 追加且已发布的历史版本未被改写。
设备安装与实机观察须使用修复后的应用；本次未更改运行中的手机设置或数据库。
