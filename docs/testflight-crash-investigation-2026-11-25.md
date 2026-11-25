# TestFlight 崩溃反馈核查（2026-11-25）

核查截止北京时间 2026-11-25 18:45:43。通过 App Store Connect API 拉取全部分页，共返回 188 条崩溃提交、196 条普通/截图反馈。原始数据包含测试者信息，仅保存于被 Git 忽略的 `tools/release/work/tf-build58-feedback-20261125T104524365Z/`，不提交原始日志。

## 版本归属

| 版本 | 平台 / Build | 本次查询结果 |
| --- | --- | --- |
| preview.46 | iOS / 58；macOS / 58 | 均无崩溃提交，也无普通反馈 |
| preview.43 | iOS / 50 | 无崩溃提交；两条普通反馈 |
| preview.40 | iOS / 47 | 五条崩溃提交，取得三份日志，两份 crashLog 返回 404 |
| preview.35 | macOS / 42 | 最近有退出期间崩溃和 sandbox 初始化崩溃 |
| 旧版 | iOS / 27 | 最新两条崩溃反馈均来自此 Build |

“没有提交”仅指本次 API 返回结果，不等于没有发生崩溃，也不代表设备回归通过。不能将提交时间当作版本归属。

## 最新两条：iOS Build 27，V8 初始化分配失败

- 11 月 25 日 18:32 提交：“正常使用跟AI发完消息就崩了”。实际崩溃于 18:31:33。
- 11 月 25 日 02:14 提交：“我在给AI发消息AI一直在卡没有回复”。
- 两份日志都是 iPhone12,8 / iOS 18.4.1，明确标注 `Version: 2.0.0 (27)`。
- 相同栈：`SegmentedTable<JSDispatchEntry>::Initialize` → `FatalProcessOutOfMemory` → `Heap::RecordStats`，最终 `EXC_BAD_ACCESS`，地址 `0x967`。
- 触发于创建 JavaScript runtime / V8 isolate，不能根据用户描述将其归因于网络或模型回复。

当前构建已经更换匹配的源码 V8 构建、禁用 pointer compression / sandbox / cppgc caged heap，并让同一工作线程共享 isolate。这些改动与旧栈相关，但本次没有该设备上新版的复现结果，不能据此关闭原症状。

## Build 47：两条 Flutter 原生崩溃仍涉及当前引擎

两份日志的 Flutter image UUID 都为 `67A6A3B8-4500-3528-8E36-D54FAF9B037D`，与本地 Build 58 archive 中 `Flutter.framework.dSYM` 一致。已用 `dwarfdump --uuid` 核对，并用 `atos` 按各自加载地址解析；此处没有借用不匹配的新版符号。

### 原生视图布局抛出异常

- 11 月 22 日 18:59，iPhone15,4 / iOS 26.4，反馈“太容易崩了”。
- 主线程 `SIGABRT`。异常栈为 `CA::Layer::set_position` → `CALayer setFrame` → `UIView setFrame` → `FlutterPlatformViewsController compositeView:withParams:`。
- `atos` 定位到 `FlutterPlatformViewsController.mm:665`，即设置 clipping view frame 的代码附近。
- 当前引擎直接以 `params.finalBoundingRect()` 和屏幕倍率计算 frame，未在该处校验有限值。非有限几何值是待验证方向；日志未提供完整异常 reason 或 Dart 调用栈，尚不能断言是哪个控件或哪次变换产生。
- 需要覆盖嵌入式原生视图的滚动、缩放、页面切换和尺寸变化，取得触发输入后修复并回归。当前无证据证明 preview.46 已解决。

### 无障碍焦点对象访问崩溃

- 11 月 22 日 20:16，iPad15,7 / iOS 27.2.1，反馈“我给它分享视频，开了一点后台，然后崩溃了”。
- 主线程 `EXC_BAD_ACCESS`，系统提示 possible pointer authentication failure。
- 栈顶 `SemanticsObject(UIFocusSystem) parentFocusEnvironment`，`atos` 定位 `SemanticsObject+UIFocusSystem.mm:64`。
- 调用链经过 `UIAccessibility` 和 `WKContentViewAccessibility`。对应源码是 `self.parent.focusItem ?: self.bridge->view()`，bridge 为弱引用；对象/bridge 的生命周期是优先检查方向。
- 尚未在原设备复现，不能将“分享视频”直接认定为底层根因。当前引擎 UUID 相同，不能按已升级 SDK 就关闭。

### 其他三条

- 11 月 20 日，iPhone16,1 / iOS 27.0：`EXC_CRASH (SIGKILL)`，`RUNNINGBOARD 0xdead10cc`，属于系统终止。该代码通常与后台挂起时仍持有文件/数据库锁有关，但本日志没有足够证据定位具体持锁者，不能笼统归为 OOM。
- 11 月 22 日，iPhone10,3 / iOS 16.7.16：反馈聊天/包管理闪退和文字图标消失，crashLog 返回 404。
- 11 月 22 日，iPhone10,2 / iOS 16.7.11：启动约 9 秒的报告，crashLog 返回 404。

## macOS Build 42

### 退出期间的文件监听注册表竞争

- 11 月 24 日 01:18，Mac16,10 / macOS 27.2.1。
- 主线程处于 `NSApplication terminate:` → `exit` → `__cxa_finalize_ranges`，正在运行全局对象析构。
- 同时后台终端 pump 在线程 75 调用 `Machine::poll_runtime_events` → `InotifyFile::service_all` → `std::mutex::lock`，抛出 `std::system_error` 后 abort。
- 当前 `hosts/ios/linux-runtime/src/linux/events/inotify.cpp` 的 `instances()` 仍返回有静态析构的注册表，后台 pump 与退出阶段注册表生命周期竞争是有栈和源码支持的重点方向。
- 本次未修改关闭流程，也没有复现退出竞争；需要以后台终端仍活动时退出为回归场景，检查停止/排空任务与全局注册表生命周期。

### 六条 sandbox 初始化失败

- 11 月 22 日，同一型号 Mac14,5 / macOS 26.6.2，共六条报告。
- 都在 `_libsecinit_appsandbox.cold.9` → dyld 初始化期间 `SIGTRAP`，尚未进入应用主入口。
- 日志不足以区分签名/授权/容器注册的具体原因；不能与终端锁竞争合并，也不能直接等同于本次打包期间已处理的 provisioning profile 文件权限验证问题。

## 本次处理结果与后续优先级

已完成云端反馈拉取、版本交叉核对、最近 14 条崩溃的日志获取尝试（12 份有效日志、2 份 404）、两条 iOS Flutter 栈的匹配符号验证，以及当前源码对照。

优先处理当前仍使用同一引擎的两个 iOS Flutter 原生崩溃；Mac 退出期间的后台任务/注册表生命周期次之。旧 Build 27 的 V8 故障需用当前版设备复测判断，不应混称为 preview.46 新增回归。

本次仅调查并记录，没有修改应用或 SDK 源码，没有重建或上传新版本，也没有进行真机/桌面复现验证。
