# operit_keep_alive

统一的 Flutter 后台驻留插件。业务层通过同一套 MethodChannel 和 lease API 使用能力，平台差异由各宿主实现处理。

该包管理**当前进程中正在运行的 Core**。定时唤醒、任务调度、进程崩溃后的任务恢复由相应系统负责。

## 平台行为

| 平台 | 行为 | 系统边界 |
| --- | --- | --- |
| Windows | 主窗口关闭进入托盘；打开操作恢复原窗口；Explorer 重启后重新注册图标 | 强制结束、注销、关机和睡眠仍由系统控制 |
| macOS | 主窗口关闭进入菜单栏；保留原 Flutter 引擎；持有 lease 时通过 ProcessInfo activity 防止 App Nap | 系统睡眠、强制退出仍结束或暂停执行 |
| Linux | GTK 主窗口关闭进入 StatusNotifier 托盘；保留窗口和 application hold | 需要 libayatana-appindicator3 和提供 StatusNotifier host 的桌面会话；不满足条件时初始化明确失败 |
| Android | 复用应用声明的 Core 前台服务；持久通知和 PARTIAL_WAKE_LOCK 由服务持有 | 通知需要系统授权；Doze、用户停止前台服务、强行停止和厂商进程策略仍由系统控制 |
| iOS | 为持有的 lease 请求 UIApplication 有限后台执行预算；到期发送 protectionChanged | 普通应用不能无限运行通用 Core；到期明确报告 unavailable |
| OHOS | 2-in-1 设备通过 TASK_KEEPING 计算类连续任务和系统通知持有执行保护；接收系统取消事件 | 项目使用 API 18；普通手机/平板的通用计算驻留报告 unavailable，不申请无关任务类型 |
| Web | 保留统一协议和 lease 计数，明确报告 unavailable | 浏览器可冻结或回收页面；关闭标签页结束本地 Runtime |

`continuous` 表示宿主已提供持续驻留机制，不代表操作系统无法停止进程。

## 应用接入

主应用已经接入该包：

1. 主窗口入口初始化 `OperitKeepAlive.instance`。
2. Core 存储配置完成后，`CoreApplicationService` 获取一个 Core 生命周期 lease，覆盖聊天、工作流、事件处理和节点监听。
3. 主窗口关闭时桌面引擎继续驻留。聊天子窗口和桌面组件不初始化独立托盘。
4. 托盘菜单提供“打开”和“退出”；存在 lease 时显式退出会确认。
5. 宿主保护变化通过 `report` 通知，应用将变化写入 CoreApplication 日志。

当前策略是 **Core 整段运行期间保留保护**，包括没有可见聊天窗口的时段。Android 服务自身也持有原生 Core lease，Flutter Activity/引擎销毁不会释放服务的 CPU 锁；服务销毁会释放锁。此策略会增加电量消耗。

Android 已有 `OperitCoreService`，其 `onCreate` 调用本包的 `KeepAliveServiceHost.attach`，`onDestroy` 调用 `detach`。服务保持现有 specialUse 声明和系统事件处理。插件只有在该服务实际完成通知和 CPU 锁接管后才确认 acquire。启动失败、超时、通知权限关闭均会在错误或报告中明确体现。Android 13 及以上在插件初始化并连接 Activity 后请求系统通知权限；每个引擎实例只发起一次请求，Activity 配置重建不会重复弹出。授权回调和应用回到前台后会重新发布当前工作通知，并更新真实保护报告。用户拒绝授权时服务仍按 Android 前台服务规则运行，报告明确说明通知不可见。

OHOS EntryAbility 声明 `backgroundModes: ["taskKeeping"]` 和 `ohos.permission.KEEP_BACKGROUND_RUNNING`。通知返回当前实际 Ability。系统取消后台任务时发送 unavailable 事件，保留真实的工作 lease 计数。

iOS 预算到期后保留真实 lease 计数并报告 unavailable，不在后台反复申请预算。用户实际回到前台后才能重新申请。

Linux 构建依赖 Debian/Ubuntu 的 `libayatana-appindicator3-dev`；桌面会话必须实际运行 StatusNotifier host。初始化或关闭检查失败时会报告原始错误，窗口保持可见。

## 有限任务 API

已完成主入口初始化的进程可以为有限任务单独申请和释放 lease：

```dart
final lease = await OperitKeepAlive.instance.acquire(reason: 'Export archive');
try {
  await exportArchive();
} finally {
  await lease.release();
}
```

也可以由包管理异常路径上的释放：

```dart
await OperitKeepAlive.instance.run(
  reason: 'Export archive',
  work: exportArchive,
);
```

`release()` 对同一个 lease 是单次操作；并发调用共享同一个结果。宿主错误、缺失或损坏的报告会直接抛出。`lease.report` 是获取时的快照，`OperitKeepAlive.instance.report` 是实时状态。

## 检查

从 `apps/flutter/app` 执行：

```text
fvm flutter pub get
fvm flutter pub get --directory=../thirdparty/operit_keep_alive
fvm flutter test --no-pub ../thirdparty/operit_keep_alive/test/operit_keep_alive_test.dart
fvm dart analyze ../thirdparty/operit_keep_alive/lib ../thirdparty/operit_keep_alive/test lib/main.dart lib/core/application/CoreApplicationService.dart
```

测试覆盖初始化握手、宿主错误、异常释放、并发释放、跨引擎 lease ID、错误报告校验、有限后台保护到期事件。托盘交互、移动端切后台和操作系统取消任务需要在对应设备上验证。

## 工作信息与系统展示

`work_activity.dart` 定义完整的版本化工作快照：独立工作标识、执行代次、稳定标题、当前步骤、开始/结束时间、步骤进度、待确认请求和允许的操作。`activation` 是应用提供的不透明跳转数据；宿主只传递它，应用负责解释。

主应用的 `WorkActivityPresentationService` 订阅 Core 的 `WorkActivityService.workUpdatesFlow`。Core 在真实的执行入口、处理状态发布点、权限请求和结束路径上更新工作记录，覆盖主会话、悬浮会话和独立会话。单次发送在交给响应流之前失败或被取消，会结束相应工作记录。结束记录最多保留 32 项，正在运行的记录不参与清理。

展示和执行保护分别报告能力：

- `workPresentationCapabilities()` 返回 `updatesOwner`、`surface` 和 `liveUpdates`。
- `publishWork(snapshot, labels)` 发布一份完整的系统展示。客户端展示版本独立递增，语言和授权变化可以刷新同一份 Core 快照。
- `workActions` 传递 `open` / `stop`、准确的工作标识和不透明跳转数据。安装接收器后调用 `enableWorkActions()`，释放冷启动时排队的点击操作。
- `workErrors` 传递原始的宿主展示和操作错误。

| 宿主 | 工作展示 | 更新所有者 |
| --- | --- | --- |
| Android | 现有 Core 前台服务通知；Android 16 接入 ProgressStyle 和 promoted ongoing 请求；显示阶段、真实计时、步骤进度、多任务计数、查看和停止操作 | Rust 直接订阅进程工作流，通过 JNI 发给服务；不依赖 Flutter 引擎订阅 |
| Windows / macOS / Linux | 托盘或菜单栏中的工作汇总、逐项状态与耗时、查看和停止操作 | 保留的主 Flutter 引擎 |
| iOS | 已授权的本地工作通知及操作按钮；ActivityKit / Live Activities 未接入 | 受系统执行预算约束的 Flutter 引擎 |
| OHOS | 已授权的服务信息通知及查看/停止操作；终态通知具有系统删除时间 | Flutter 引擎 |
| Web | `surface` 为 `none`，不提供系统工作展示 | 页面中的 Flutter 引擎 |

Android 使用 `POST_PROMOTED_NOTIFICATIONS` 和 `android.requestPromotedOngoing` 请求系统展示，实际提升权限通过 `canPostPromotedNotifications()` 查询。系统和厂商决定具体的岛/状态芯片样式。用户关闭某次工作的通知后，该次工作不会反复请求展示。结束后移除停止按钮，结束计时固定，结果展示 60 秒后恢复服务驻留信息。

步骤进度的数值范围在展示协议中为 0–100，`scope` 标识它对应的工具步骤。Core 的 0–1 进度在发布时换算；没有明确步骤进度时展示阶段与计时，不构造整项工作的完成百分比。

停止操作在 Core 的运行时锁内验证执行代次并预留取消状态，过期按钮会收到明确错误。点击“查看并确认”进入既有的对应会话确认界面。系统展示支持中英文，语言随 Flutter 的语言变化同步。

检查新增行为时，从 `apps/flutter/app` 执行：

```text
fvm flutter test --no-pub ../thirdparty/operit_keep_alive/test
```

Core 的工作生命周期、权限叠加、终态保护、发送失败清理和过期停止操作由 `WorkActivityService::tests` 覆盖。通知、托盘菜单和岛的实际显示需要对应系统设备验证。
