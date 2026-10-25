# 设备空间大同步进度

## 现有链路与问题

1. `space_join::refresh` 校验审批结果并导入成员、身份和拓扑，发布 `Joined`。
2. Full Core 的 `RuntimeRemoteLinkService::afterJoined` 直接等待 `synchronizeReachablePeer`；失败只写日志。与此同时，常驻同步服务由持久化变更、连接变化、每两秒的文件扫描唤醒。
3. 常驻服务 `synchronizeOnce` 先交换空间投影，再通过 change gate 对可达成员执行权限交换、设备偏好交换、版本检查和业务同步。业务同步要求两端都有存储权限。
4. `synchronizeDomains` 比较 receipt，合并双向差异，按操作数和传输字节分页；先补齐引用文件，再向两端提交操作。文件按 64 KiB 分块，关闭传输后再次检查文件是否落盘。
5. 每轮最多 64 页。接收成功不等于业务应用成功；receipt 中的 pending、errors 和缺失文件会留给后续轮次。
6. UI 的 `deviceSpaceSnapshotFlow` 只有成员和拓扑，没有同步执行信息。独立创建的 facade 还会创建独立的同步状态，首次同步与常驻同步没有共用 gate。

## 修改原则

- 此链路同时覆盖首次加入、离线后积累的大批增量、路由切换和失败恢复；展示条件不依赖是否首次加入。离线成员继续通过原有可达性检查跳过，失败遵循原有指数退避。
- 一个 CoreNode 共享一个同步服务状态。后台同步、加入后的唤醒及路由切换前的同步使用相同 gate，同一对端串行执行。
- 加入完成只唤醒原有后台调度，不等待业务数据全部同步后才向界面返回。
- 同步过程直接产生状态：检查差异、交换记录、传输文件、等待重试、完成。状态流独立于拓扑流，避免进度更新触发拓扑和权限重新加载。隐藏任务只保留本地计数；等待同步结果的同时竞争一个延时，快任务完成即取消延时，不为每次小同步留下独立定时任务。
- 一次任务可以跨多个分页轮次及失败重试；到达页数上限、重复页、业务 pending 或错误均不能标记完成。
- 双向文件传输共用一段分块循环，保留逐块权限检查和落盘校验。统计已确认交换的记录与实际发送的文件字节；当前文件有真实大小才能显示比例。没有总量时不构造总体百分比。字节统计表示传输量，重传也计入。
- 服务仅在任务持续 3 秒，且达到 256 条交换记录、1 MiB 实际传输量或当前文件大小后发布可见进度；小同步的开始、批次和完成均不产生界面事件，也不额外统计全部待同步数据。显示后跨轮次保留，完成后保留 3 秒。界面由根部 `RuntimeActivityOverlay` 统一订阅，页面切换不影响任务展示；仅完成收起使用定时器。
- 权限撤销、成员退出或空间切换时清除不再适用的进度。

## 验证范围

Rust 检查共享服务、任务跨轮次/重试、真实批次和文件统计、分页退出的 pending 语义；设备空间行为测试覆盖加入及现有 idle gate。Flutter 检查小同步静默、慢同步显示、文件进度、完成收起、重试、空间切换及与包加载共用的展开/折叠界面。通过真实代理生成工具更新客户端契约。

## 本次验证（2026-10-25）

- `cargo check -p operit-proxy-local`：通过，使用原有生成器生成代理契约。
- 先前设置页方案通过 50 项 Flutter 测试；当前已替换为全局任务浮层，下方记录最终全局方案的验证结果。
- Rust：进度状态 4 项、原同步服务 8 项测试全部通过。连续 20 次小同步只收到初始空快照，没有进度事件。
- 设备空间行为回归：97 项中 95 项通过、2 项失败。空闲检测用例暂停真实小增量响应；最终全局方案已改为即使超过三秒仍保持静默，重建 facade 复用同一个 gate。
- 两项失败为 `doctor_detects_quarantines_and_selfchecks_damaged_lines` 和 `target_space_change_does_not_block_withdrawal_of_an_old_request`。在隔离的 `6c1fa579` 源码副本上复用相同依赖、单独编译并运行，两项均在原断言失败，确认不由本次同步修改引入。
- 当前 macOS 测试链接需要显式补充本地模型依赖的 Accelerate：`cargo rustc -p operit-node-runtime --lib --profile test -- -l framework=Accelerate`，随后运行生成的测试程序。没有修改项目链接配置。
- 原有 `check_structure.py` 因 `coverage.json` 与测试清单不一致而失败；本次未新增或重命名该目录的测试用例，也未修改原有清单。

真机上的大文件、弱网、移动端后台恢复还需实际验收；本次自动化结果不等于这些场景已经通过。

## 双端原生联测与截图

`integration_test/device_space_sync_native_test.dart` 在两端分别创建独立存储目录，
通过真实 HTTP Peer Link 配对、申请加入、审批并启动原有后台同步。测试协调器仅负责
阶段屏障与系统截图，不提供业务响应或注入同步进度。包加载预览单独使用示例包状态，
文件名以 `package-*-preview` 标注；同步进度与拓扑订阅始终使用原生 Core。

在仓库根目录启动协调器：

```sh
python3 apps/flutter/app/integration_test/device_space_sync_coordinator.py \
  --output docs/assets/sync-progress-2026-10-25
```

然后在 `apps/flutter/app` 目录的两个终端分别运行：

```sh
fvm flutter test integration_test/device_space_sync_native_test.dart -d macos --reporter expanded
fvm flutter test integration_test/device_space_sync_native_test.dart -d <iOS模拟器UDID> --reporter expanded
```

场景包括首次加入传输 96 MiB 工作区文件、暂停后台同步后积压 96 MiB 增量再恢复、
最后同步一个小文本。大文件核对完整 SHA-256；两端观察真实进度流和全局浮层，切换页面验证全局展示，
小文本需落盘且无新的可见同步状态。协调器保存原始系统截图与阶段日志。

## 全局任务入口

`OperitTheme` 根部保留一个 `RuntimeActivityOverlay`。它直接消费现有包加载状态与
大同步状态，不新增通用任务框架。原 `PluginLoadingOverlay` 与设置页的
`SpaceSyncProgressCard`、同步订阅一起移除。

折叠态为 232 × 64 的圆角卡片：任务图标、两行摘要、底部细进度条；多任务增加
数量提示。点击展开，在同一个可滚动面板中查看各设备同步、包启动详情和原有日志。
卡片支持拖动并约束在安全区域内。跳过包加载只调用原有包加载接口，不中断同步。

### 原生联测定位的服务复用问题

生成代理每次调用都会重建 facade；原 `localApplicationSharedClient()` 每次创建新的
`Arc`，导致同步服务注册表无法把后台 worker 与进度订阅识别为同一应用。
`LocalCoreProxy` 现在通过共享 `Weak` 缓存复用仍存活的应用客户端，避免强引用环，
同时保留不同应用实例的隔离。原生双端测试会实际经过这条代理链路。

## 最终全局方案验证（2026-10-25）

- macOS 与 iOS 27.0 / iPhone 18 Pro Max 模拟器：完整原生集成测试均通过。
- 首次加入传输 96 MiB 文件、断开监听并积压 96 MiB 文件后恢复增量：均落盘并核对 SHA-256。
- 6 字节小文本两端实际同步成功，无新增可见进度事件，截图中无同步浮层。
- 切换到工作空间页面后仍显示同一个全局同步浮层；完成后正常收起。
- 原生联测发现“仅三秒门槛”误展示慢的小同步，因此增加实际工作量门槛；不额外遍历待同步数据。
- Flutter：全局浮层、设备设置、加入弹窗共 50 项通过；相关界面与集成测试静态分析无问题。
- Rust：进度状态 5 项、同步分页与错误处理 8 项、原调度 gate 4 项通过。新增慢小同步静默、延时后才跨越记录门槛的状态测试。
- 两端原始截图及 `verification.json`、`native-events.json` 见 `docs/assets/sync-progress-2026-10-25/`。
- `compact-preview.png` 仅裁切并拼排原始系统截图。包加载截图为示例包状态的原生界面预览；同步截图来自真实网络传输。

实机、弱网与系统长时间后台挂起未在本轮验收范围内；这里的双端结果为 macOS 与 iOS 模拟器前台联测。
