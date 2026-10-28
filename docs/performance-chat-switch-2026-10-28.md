# 2026-10-28：实时输出时切换新对话的 CPU 火焰图

本次连接用户正在运行的 Dart VM，采样进程 **PID 17119**，iPhone 18 Pro Max / iOS 27 模拟器，Apple M5，Flutter **debug / JIT**。没有使用昨天的采样代替本次结果。

**已复现并通过交替对照确认：旧对话仍在输出时，即使前台切到新空对话，关闭的手机侧栏里，旧对话的运行状态动画仍在驱动帧。暂停这一个动画后，应用 CPU 从约 23% 降到约 4%；恢复后再次升到约 23%。** 主线程在这些帧中持续处理 semantics，原生 raster 线程也参与工作。

本次交互火焰图、Speedscope 采样、阶段事件及汇总保留在本机 `/tmp/operit-stream-cpu-20261028/`，不随本次提交纳入仓库。修复前主对照位于 `causal/flamegraphs/`；修复后复测位于 `after-cold-local-paint/flamegraphs/`，HTML 文件名均为 `iphone-flamegraphs.html`。

## 实际切换后，CPU 没有回到静置水平

第一轮在 2026-10-28 **15:54:06—15:54:51，Asia/Shanghai** 采集。通过现有 Core 业务入口创建测试对话、使用现有模型配置发送长篇 Markdown 生成请求，实际聊天页面参与渲染，然后创建并切换到新空对话。没有叠加替代聊天页面或模拟流。

| 阶段 | 窗口 | 应用平均 CPU | Dart 样本 |
| --- | ---: | ---: | ---: |
| 输出前静置 | 3 秒 | 1.60% | 3 |
| 输出对话可见 | 10 秒 | 38.79% | 2,156 |
| 新对话切换过渡 | 2 秒 | 26.93% | 231 |
| 已在新空对话，旧对话继续输出 | 12 秒 | 19.33% | 1,132 |
| 返回仍在输出的旧对话 | 8 秒 | 42.92% | 2,051 |
| 再次切到新空对话 | 10 秒 | 23.95% | 1,111 |

切换稳定后，对整个 Flutter Element 树的诊断遍历只找到新对话的 `ChatArea`：`messages=0`、`_bottomFollowTicker.isActive=false`，没有旧对话的 `StreamMarkdownRenderer`。返回旧对话后又观察到正在输出的正文。这里的剩余负载不能简单归结为“旧聊天正文仍挂在页面里布局”。

第一次切换后的 12 秒窗口中，Dart 样本内 `PipelineOwner.flushSemantics` 的 inclusive 占比为 **34.36%**，`flushLayout`、`flushPaint` 各约 **0.09%**。再次切到新对话时，`flushSemantics` 达 **41.76%**。这些百分比是样本栈覆盖比例，不是应用总 CPU 的直接分解。

## 同一条真实输出中的交替暂停、恢复对照

第二轮在 **16:01:24—16:02:04，Asia/Shanghai** 采集。新发起一次长篇生成，在输出中切到新空对话；随后保持页面与真实网络流，交替调用旧对话侧栏 `_marqueeController.stop()` / `.repeat()`。每段 6 秒，结束后恢复正常动画行为。

对照的五个阶段开始和结束时，均通过一次性 `chatStateFlow(...).first` 确认旧对话 `isLoading=true`；遍历树确认当前仍为新空对话，聊天跟随 ticker 关闭。切换动画开关时，匹配到的旧对话运行状态 controller 数量均为 **1**。

| 阶段 | 隐藏的运行状态动画 | 应用平均 CPU | Dart 样本 | Native 样本 |
| --- | --- | ---: | ---: | ---: |
| `full-1` | 运行 | **23.03%** | 560 | 1,404 |
| `paused-1` | 暂停 | **4.31%** | 7 | 229 |
| `full-2` | 恢复运行 | **23.12%** | 571 | 1,366 |
| `paused-2` | 再次暂停 | **3.67%** | 9 | 241 |
| `full-3` | 再次恢复运行 | **20.79%** | 539 | 839，部分窗口 |

两组完整的恢复/暂停对照分别下降约 **81.3%**、**84.1%**。这证明隐藏的状态动画是本次“切到新对话仍有高 CPU”的主要可移除来源。暂停后仍有真实生成及接收、运行时等工作，不能将 3%—4% 全部视为异常。

`paused-1 / paused-2` 分别只有 7、9 个 Dart 样本，不据此分析小比例函数热点，也不把“没采到某个函数”写成绝对零开销。

## 火焰图中的负载落点

`full-1` 的 Native 运行样本中，**Main Thread 54.49%**，**io.flutter.1.raster 21.01%**；`full-2` 分别为 **54.76% / 22.77%**。暂停阶段不再有相同规模的 UI 帧采样。

Dart 栈中可展开到：

```text
Flutter Animator / BeginFrame
  → SchedulerBinding.handleDrawFrame
  → WidgetsBinding.drawFrame
  → RendererBinding.drawFrame
  → PipelineOwner.flushSemantics
  → _RenderObjectSemantics.ensureGeometry / _updateChildGeometry
  → _SemanticsGeometry.computeChildGeometry
```

`flushSemantics` 在 `full-1 / full-2 / full-3` 的 Dart 样本 inclusive 占比分别为 **29.64% / 32.92% / 29.50%**。动画回调本身在采样里较窄；主要成本是它持续触发的后续帧工作，不能只按 `_tick` 本体的占比判断动画是否昂贵。

修复前源码与这次对照结果吻合（以下行号对应修复前版本）：

1. [CollapsedDrawerContent.dart](../apps/flutter/app/lib/ui/main/components/CollapsedDrawerContent.dart) 第 389—403 行：对话 `isRunning` 时，以 900 ms 周期调用 `_marqueeController.repeat()`，与当前所选聊天无关。
2. [PhoneLayout.dart](../apps/flutter/app/lib/ui/main/layout/PhoneLayout.dart) 第 330—342 行：手机侧栏子树一直保留，通过 `Transform.translate` 移到屏幕外。运行时已观察到 `drawerOpen=false`，侧栏 item 的 `TickerMode.of(context)=true`。
3. [history_chrome.dart](../apps/flutter/packages/operit_ui/lib/src/render/history_chrome.dart) 第 93—102 行：状态点使用 `AnimatedBuilder`，每次动画更新创建新的 `_ThemeMarqueeDotsPainter`。
4. 同一文件第 665 行：`shouldRepaint` 在 progress 变化时返回 true，未覆盖 `shouldRebuildSemantics`。本机正在使用的 Flutter SDK `rendering/custom_paint.dart` 第 244 行，其默认实现委托 `shouldRepaint`；第 472—479 行在 delegate 要求重建时调用 `markNeedsSemanticsUpdate()`。这给出了装饰动画持续使 semantics 变脏的源码路径。

## 已实施的局部更新和双向重绘隔离

- 状态点通过 `CustomPainter(repaint: animation)` 直接读取动画值并绘制，移除逐帧 `AnimatedBuilder`；动画更新不创建新 widget 或 painter，不重新布局，也不使 semantics 变脏。状态点有独立 `RepaintBoundary`，Tooltip 的可访问文本保留。
- 手机侧栏关闭后用 `TickerMode` 停止隐藏子树的 ticker；打开和关闭过渡期间继续正常显示动画，侧栏状态保留。
- 手机主内容保留原有重绘边界，完整侧栏表面补上重绘边界，包含背景、阴影和玻璃层。平板展开及折叠侧栏和主内容也分别设置独立边界。

共享组件测试验证四次动画更新只重绘状态点边界，祖先 build、layout、paint、semantics assembly 计数不增加；更换动画后旧监听器被解除。布局测试覆盖手机动画开／关、平板展开／折叠，再分别覆盖透明背景开／关，共八种组合：真实运行状态点及侧栏数据变化不增加主内容 build、layout、paint 计数；主内容连续五次更新时侧栏没有重绘，侧栏 widget 实例保持不变。还显式触发整个侧栏表面重绘，确认主内容边界没有重新绘制。

布局隔离八项及状态点两项测试通过，既有手机抽屉、系统安全区和对话条目测试通过；六个变更 Dart 文件的定向静态分析无问题。平板宽度变化需要重新分配主内容约束，以上隔离验证针对几何尺寸稳定时的独立更新。

## 修复后真实流复测

在同一应用进程的新 Dart isolate 中，通过实际聊天入口发起生成，再切换到新空对话。保持旧对话持续输出，依次关闭、打开、再次关闭侧栏；三段开始和结束均确认旧对话 `isLoading=true`。

| 阶段 | 应用平均 CPU | Dart 样本 |
| --- | ---: | ---: |
| 静置 | 3.26% | 3 |
| 输出对话可见 | 43.14% | 1,374 |
| 新空对话，侧栏关闭 | **3.46%** | 3 |
| 新空对话，侧栏打开 | **24.92%** | 510 |
| 新空对话，侧栏再次关闭 | **3.41%** | 3 |

该轮 Native 覆盖所有阶段的完整窗口。CPU 同样按采样区间与阶段窗口重叠时间加权。该录制验证状态点局部绘制和隐藏侧栏停帧，早于完整侧栏表面及平板重绘边界的补充；后者由上述布局测试独立验证。第一次仅热重载后的保留状态存在 ticker 模式不一致，未将该次数据用作修复后结论。

侧栏可见时依然有帧成本。在这段 510 个 Dart 样本中，`flushSemantics` inclusive 占 16.27%，其中 `debugCheckForParentData` 占 12.16%；没有采到此前的 `computeChildGeometry` 热点。当前 Flutter SDK 的 `PipelineOwner.flushSemantics` 在 debug 断言中即使没有待更新的 semantics 节点也会检查整棵渲染树。因此，debug 全树检查不能等同于应用重建了主内容或重绘了主内容。独立边界也不消除最终场景合成、栅格化及透明背景采样成本。本轮未据此宣称可见侧栏或正文实时输出的全部 CPU 问题已经解决。

用户提到后台占用正常，本次结果与“前台动画持续驱动帧”的解释一致；本轮未独立录制后台窗口，不将后台行为写成已经实测的结论。当前对话可见时，正文排版与绘制也有额外成本，不能把它们全部归于侧栏。

## 采样范围及可复核性

CPU 来自 `proc_pid_rusage` 的累计 user + system 时间，约每 100 ms 读取；平均值按与阶段窗口重叠的时间加权，峰值只取完全落在窗口中的采样区间。所有应用线程合计，单核满载计 100%。Dart 使用 `getCpuSamples`，Native 使用 Instruments Time Profiler，仅纳入标为 `Running` 的样本。阶段边界由应用内 `Timeline.now` 与墙钟时间一起标记。

CPU 和 Dart 覆盖全部列出的阶段。Native 有明确的记录范围：

- 第一轮 Native 录制结束于 **15:54:28.341**，只覆盖第一次新对话稳定窗口的前半段，未覆盖后面返回旧对话及再次切换。第一轮结论中的整段 CPU 与 Dart 不受这一截断影响；不将部分 Native 样本冒充整个阶段。
- 第二轮 Native 为 **16:00:51.531—16:02:02.129**；`full-1 / paused-1 / full-2 / paused-2` 完整覆盖，`full-3` 覆盖 **59.18%**。本机交互式图表单独显示每段 Native 覆盖率。

第二轮采样监听器最初收到了 DDS 重放的第一轮历史事件；已从 DDS Extension history 按本轮时间重新提取边界，重新导出对应 VM 时间段的 CPU 样本，避免混用旧阶段。汇总标记 `recoveredFromDdsExtensionHistory=true`。原始事件一并保存，便于复核。

Native 与 Dart 是不同采样集合，inclusive 栈互相包含，不能相加，也不能直接乘应用总 CPU 得出某个函数的准确 CPU 时间。debug/JIT、模拟器、调试器和采样器均可能影响绝对数值；本次结论依赖同一运行进程内重复的暂停/恢复对照，不外推真机功耗。

原始 `.trace`、完整函数表、约 100 ms CPU 计数、注入表达式及诊断脚本保存在本机 `/tmp/operit-stream-cpu-20261028/`，主对照位于其 `causal/` 子目录。测试通过真实业务入口创建了多组测试对话，保留在应用历史中。
