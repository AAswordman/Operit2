# DSL 性能诊断（2026-10-09）

## 版本与结论边界

- 诊断分支：`workspace/snapshot-20261009-002143`，提交 `824dcb7c`。
- 本地分支比远端跟踪记录少两个提交，本次未更新分支历史。
- 正在运行的 Windows Debug 程序，其 VM 返回的 `ToolPkgUiLauncherScreen.dart` 与这个分支不一致。因此不把该程序的耗时归属于当前分支，也未操作其聊天或角色数据。
- 本次不改变业务、调度、渲染或错误处理逻辑。新增工具只做诊断。
- 下表是当前源码的组件微基准，不是用户实际操作的端到端耗时。没有测到 native QuickJS、真实 Core IPC、WebView 通道、Flutter 布局和栅格耗时。

## 实际的界面分层

角色卡管理/编辑页与聊天侧边栏使用 DSL 容器中的 WebView；角色选择弹窗使用原生 DSL 节点。

- `src/host.ts:12` 注册入口，`src/host.ts:66` 开始构造编辑器 WebView。
- `src/ui-selector.ts:204` 构造原生选择界面。
- `web/bridge/events.ts:42` 的普通输入只修改 WebView 本地草稿，不能把它解释为逐字等待原生 DSL 文本队列。
- 普通 WebView host 调用仍经过 `ToolPkgComposeDslWebView.dart:1437` 的 `invoke → executeAction`，然后进入 DSL action stream；不是浏览器 JS 直接访问数据。
- 本机 JS host 的实现位于 `hosts/common/operit-host-native-scheduler/src/javascript_runtime.rs`，使用 `rquickjs`。Node/V8 的微基准不代表该 host 的实际执行速度。

## 已确认的重复工作

### 1. 只读列表也付出完整写事务准备成本

`src/storage/files.ts:50` 的 `run()` 对包括列表在内的操作统一执行：

1. 获取整个仓库的串行锁。
2. `verifyDocuments()` 逐个读取所有 owner 的 `USER.md`，逐个检查其 `.next` 路径。
3. `copyState()` 对整个业务状态 stringify、parse 并校验，范围包括记忆空间等不属于当前列表的数据。
4. 执行具体查询；例如 `listCharacters()` 又深复制角色列表。
5. 校验完整状态，并 stringify 两份完整状态来判断是否发生修改。

`character.list` 与 `group.list` 虽然由选择器放入 `Promise.all`，它们仍分别进入同一个仓库锁，重复执行上述步骤。两次查询不会合并为一次文件检查。

使用真实插件业务代码、隔离临时目录与真实 Node 文件 IO，12 次只读采样：

| 角色数 | 一次角色列表 p50 | 一次列表的文件调用 | 两个列表请求的文件调用 |
| --- | ---: | --- | --- |
| 1 | 0.50 ms | 1 read + 1 exists | 2 read + 2 exists |
| 10 | 4.23 ms | 10 read + 10 exists | 20 read + 20 exists |
| 50 | 25.35 ms | 50 read + 50 exists | 100 read + 100 exists |

每个新增角色包含 16 KiB 的合成 characterSetting。数据、初始化与创建耗时不计入列表测量。真实 Core host 的调用成本未包含，因此时间不是应用实机延迟；文件调用次数是当前真实业务代码的行为。

### 2. 状态动作重复生成、传输完整树

`ToolPkgComposeDslRuntimeScript.rs:266` 构造中间树，`:325` 构造最终树。一次同步状态修改在探针中产生两次完整 render、两份完整响应。

`LazyColumn` 仅对 Flutter 可见 widgets 做懒构建；`src/ui-selector.ts:186` 已在 JS 中遍历全部 options 并生成全部节点，所有节点仍要序列化和解析。

响应包含整个 `state` 和 `memo`；`ToolPkgUiLauncherScreen.dart:607` 又把它们放进后续 action 的 runtime options。大状态因此双向往返，即使本次只修改很小的值。

### 3. Flutter 对同一份内层 JSON 重复解码三次

`compose_dsl/render_models.dart:34` 对每份 intermediate/final 事件执行：

- `tryParse(raw)` 解码整个内层响应并构造节点。
- `actionResultOf(raw)` 再次解码整个内层响应。
- `navigationCommandsOf(raw)` 再次解码整个内层响应。

另外还有外层事件 JSON 的解码。此路径由 `ToolPkgUiLauncherScreen.dart:506` 的 stream listener 同步调用，发生在该 Flutter isolate 上。

### 4. actionStore 随 render 累积回调

`ToolPkgComposeDslBridge.rs:292` 每次遇到函数都生成新 action id 并保存在 `runtime.actionStore`。当前实现没有按树版本清理这个 store。

生产 selector view + 生产 SDK，初始 render 后执行 33 次合成状态动作（8 次预热、25 次采样）：

- 20 行：保留 6,969 个 action handlers。
- 100 行：保留 33,769 个 action handlers。
- 500 行：保留 167,769 个 action handlers。

这些数量也包含 SDK 对组件 props 中函数的注册，不全是行点击事件。它证明回调累计保留，并不等于已经测得实际用户会话的内存或 GC 暂停时间。当前上下文释放结束这次观察范围，不能直接清空回调表：正在执行的 action、WebView 接口与旧树拥有的 callback 需要生命周期分析。

### 5. 侧边栏读取聊天扩展是串行 N 次 host 调用

`src/ui-sidebar.ts:173` 在全量 snapshot 后逐个 `await Tools.Chat.readExtension`。群组视图还逐分类查询 conversation groups，每次又进入仓库操作。

`web/sidebar.ts:50` 初始化顺序为 `currentTheme → currentSidebar → catalog`。每一步都有跨层等待。这里的 host 耗时需端到端采样；本次只确认调用顺序，不给出虚构的毫秒占比。

原生 DSL 文本输入另外受 `compose_dsl/action_scheduler.dart:8` 的逐次等待约束，普通 action 也会等待文本队列排空。不能把这个事实用于解释上述 HTML 编辑器的普通打字。

## 组件微基准

使用生产 SDK 脚本和生产 selectorView，合成只读行，并在根节点添加一个只修改 probe state 的诊断 action。该 action 不是产品选择/保存操作。JS 排除 runtime 初始化与 TypeScript 内存转换，预热 8 次、采样 25 次。

Dart 探针从当前源码精确抽取原解析类及依赖函数，以独立 Dart JIT 运行；没有重新实现解析器，没有 Flutter native-assets hook。预热 12 次、采样 40 次。

| 合成场景 | JS action + 两份响应编码 p50 | 单份响应的 Dart 生产解析 p50 | 两份响应总字节 |
| --- | ---: | ---: | ---: |
| 20 行 | 2.03 ms | 1.88 ms | 77,820 |
| 100 行 | 10.13 ms | 7.58 ms | 374,350 |
| 500 行 | 51.02 ms | 42.53 ms | 1,863,358 |
| 20 行 + 1 MiB state | 10.74 ms | 16.23 ms | 2,174,972 |
| 20 行 + 4 MiB state | 37.43 ms | 35.53 ms | 8,466,428 |

表格记录本次最后一轮各探针的 p50。独立复测存在 JIT、GC 和系统调度波动，不能当作稳定的产品延迟指标。

JS 一栏包含 render、内层 JSON 编码、外层事件编码；不包含请求编码，其分项保存在 `js-probe.json`。Dart 一栏每份响应测一次，一次状态动作有两份响应。不要把不同进程的分项直接相加成实机 wall time，也不要把数据当作 release 构建或 native QuickJS 的成绩。

## 复现

仓库根目录：

```powershell
node tools/performance/compose_dsl_probe.mjs
node tools/performance/prepare_compose_dsl_parser_probe.mjs
node --test tools/performance/compose_dsl_storage_probe.mjs
```

在 `apps/flutter/app`：

```powershell
fvm dart --packages=../../../target/dsl-performance/empty-package-config.json ../../../target/dsl-performance/dart_parser_probe.dart ../../../target/dsl-performance
```

报告写入 `target/dsl-performance/`，不包含真实用户记录：`js-probe.json`、`dart-parser-probe.json`、`dart-parser-source.json`、`storage-probe.json`。源文件摘要用于核对解析器版本。

已有导航和角色选择行为测试共 15 项通过；仓库 IO 探针另有 4 项通过。

尝试运行 Flutter test 时，项目 native-assets hook 自动调用插件 SDK codegen，被已有 `registration.rs:286` 的 `RegistrationVisitor.unavailable` 字段错误阻断。没有为这处无关问题修改业务代码，也未继续进行 Rust/Flutter 应用构建；Dart 解析探针独立执行成功。

## 后续定位与优化顺序

1. 实机按动作关联记录：WebView invoke、DSL action 入队/开始、仓库锁等待、文档验证、状态复制/校验、IPC、JS render、响应字节与序列化、Flutter parse、首个展示更新帧。
2. 拆分只读查询与写事务，保证读取一致性与文档状态验证要求，不为普通列表重复做全库写准备。
3. 单次解析内层响应，避免单个小结果使全树 JSON 重复 decode。
4. 分析 state/memo 的拥有者与跨边界必要数据，减少稳定上下文中的完整状态往返。
5. 根据 action 与树的生命周期回收 callback；控制最终/中间树重复工作，不能牺牲长动作的中间进度与导航语义。
6. 独立处理侧边栏 N 次串行读取，以及大原生 DSL 列表在 JS/传输阶段没有虚拟化的问题。

目前结论：成本已经明确包含业务只读事务、重复整树处理、JSON 往返与跨层串行等待；没有证据可以把延迟简单归因于“JavaScript 语言本身很慢”。



## 2026-10-09 实现审计：局部传输不等于局部生成

当前未完成端到端优化交付。新增探针直接统计实际 `ctx.UI` 节点构造次数，不能再用 upsert 数量代表生成成本。

| 合成列表行数 | 每次编辑根函数执行次数 | 每次编辑生成节点数 | 每次编辑传输变更记录数 |
| --- | ---: | ---: | ---: |
| 20 | 2 | 88 | 2 |
| 100 | 2 | 408 | 2 |
| 500 | 2 | 2008 | 2 |

复现：`node tools/performance/compose_dsl_retained_probe.mjs --require-local-generation`。
该验收门禁目前明确失败：节点生成成本随整表大小增长。普通探针通过只证明 JS 层差量输出和回调生命周期，不证明局部生成，也不证明 Host/CoreLink/Flutter 端到端无 UI 树 JSON。

当前根因：`__operit_build_compose_response` 每次执行完整入口；`composition.commit` 在整棵树创建后才遍历比较。`ctx.UI` 代理拿到参数时，参数中的子节点已经求值。要在保留插件源码写法的前提下跳过子表达式，需要在求值之前建立可重执行的依赖作用域，例如内部 AST 编译转换；不能靠运行时普通值的 Proxy 恢复已丢失的依赖信息。闭包更新、循环身份、条件分支、异步及副作用语义必须纳入该转换的测试。

自动代理仍未接通：现有生成器不支持 factory 对象的 ReverseStream 方法。将 session 工厂移到 application 根对象的尝试仍被生成器明确拒绝，该尝试已撤销。不能修改 CoreLink 或自动代理生成器来绕过用户约束；应用层入口需使用生成器已经支持的服务对象接入形式。

验收必须分开记录：JS 节点生成数、宿主结构化值转换成本、流上传输记录数、Flutter 实际重建范围以及 UI 树 JSON 编解码次数。仅保留 `serde_json::Value` 并不代表发生 JSON 文本编解码，但也不代表已实现零拷贝或局部生成。


## Retained dirty-scope commit

The initial retained commit used replacement maps and whole-graph walks. It now keeps one persistent ID-to-node index and an occurrence-local dirty queue. Ordinary nested property mutation enqueues only the node occurrence; a changed node whose type and key stay stable retains its handle, so ancestors and sibling groups are not revisited. Child references are enumerated only when a parent's child/slot structure changes. Key/type replacement and branch removal traverse the replaced/removed branch only. Callback deletion is driven by callback slots owned by the changed or removed occurrence, not by a scan of all nodes or all registered actions.

The production-script regression probe verifies 20/100/500-row compiled screens at **2 visited commit nodes, 0 child-reference checks, and 2 generated nodes per text edit**. A direct retained-node test verifies a dirty leaf under retained ancestors visits one node and checks no siblings; the next unchanged commit visits zero nodes. It also covers shared node occurrences, nested property writes, callback release/remount, key/type changes, branch removal, named-slot removal, descriptors, and duplicate keys.

Reproduce with `node tools/performance/compose_dsl_retained_probe.mjs --require-local-generation`. This measures Node/V8 simulation, not production QuickJS/Flutter latency. The screen function still executes, and plugin-authored loops or arbitrary non-UI computations still run as written; the compiler skips UI node construction at unchanged call scopes. This optimization must not be described as skipping all plugin JavaScript work.


## State-indexed execution units (in progress)

Added a separate reactive scheduler and compiler partition for statically keyed state reads in synchronous straight-line render functions. It keeps captured state bindings live, orders derived bindings ahead of consumers, independently executes UI expressions, and disposes inactive branch subscriptions. Unpartitioned control-flow remains an explicit broad execution unit with per-response execution semantics. Host theme updates invalidate the root explicitly.

The reactive production-script probe checks 20/100/500-row screens: zero root executions per text edit, two expression executions and zero commit child-reference checks. It verifies current captured state and derived values in callbacks, asynchronous writes, reorder and conditional removal. Node/V8 timings include the test harness's full downstream-reference validation; they are not real QuickJS/Flutter measurements.

Scope limitations: compiler partitioning currently requires a statically keyed useState binding and a synchronous straight-line body returning a UI constructor. Complex helper/control-flow frames are not yet independently partitioned. This is not universal Compose recomposition support. Existing character-card tests: 21 of 23 passed; two reject expectations use old action IDs after the corresponding callback property has been removed, currently receiving `compose action not found` rather than the controller's previous finished/busy error. Those failures remain explicit, not suppressed. The test harness now loads the same generation/retained/reactive modules as the production SDK bootstrap.


## Mounted callback ownership compatibility fix

A callback slot now belongs to its mounted node occurrence, independently of whether that property is present in the current UI record. Removing `onClick` no longer releases the occurrence-owned action ID. Already issued events can therefore reach the plugin's original busy/finished guards. Reintroducing the property reuses its ID and installs the current closure. Removing the occurrence releases all of its slots, including currently hidden ones. No hidden property is added back to the UI record, and no global action scan, timer-based retention, or stale-ID recovery path was added.

The earlier two character-card failures are resolved: the original selector-ui and ui-consumers suites now pass 23/23 without changing their expectations. The production-script probe also verifies 100 disable/enable cycles, stable action ID and callback count, refreshed captured values, hidden UI properties, preserved busy/finished errors, and final release on unmount. A mounted occurrence retains one callback per property path it has owned; removing arbitrary unique property paths is not an unmount. Full removal/type/key replacement still ends the occurrence lifetime; this patch does not extend old events across those boundaries.
