# iOS 工具调用与消息接收 CPU 堆栈分析

> 新编译进程 PID 78568 的消息复测已另存：[2026-10-27 消息复测报告](performance-ios-message-retest-2026-10-27.md)。下文保留原 PID 31239 的采样。

采样对象是用户已启动的 **iPhone 18 Pro Max / iOS 27 模拟器**，进程 PID 31239。沿用正在运行的 `lib/main.dart` 和已有 `deepseek-flash` 配置，通过 Flutter VM 的 WebSocket 调试接口触发业务；同步使用 Instruments Time Profiler 抓取原生线程。实际聊天界面参与了流式接收与渲染。

- [交互式火焰图与 CPU 曲线](assets/performance-2026-10-27/iphone-flamegraphs.html)：切换阶段、切换 Native / Dart、搜索、点击矩形放大。
- [Speedscope 格式堆栈](assets/performance-2026-10-27/iphone-profiles.speedscope.json)：包含各阶段的原生与 Dart 采样。
- [CPU、线程占比和源码热点数据](assets/performance-2026-10-27/iphone-cpu-stacks.json)。

原始 Instruments 文件保存在本机 `/tmp/operit-performance-20261027/iphone-business-stacks.trace`，原始 Dart 样本为同目录 `current-dart-*.json`。不把包含大量重复符号信息的原始文件复制到仓库。

## 实测 CPU

CPU 为应用所有线程相加，单核满载计 100%。峰值仅从完整落在阶段内的约 100 毫秒采样窗口取值，避免把相邻阶段算进来。各阶段边界由运行中应用发出带 `Timeline.now` 与墙钟时间的 VM Extension 事件记录。

| 阶段 | 平均 CPU | 100 毫秒峰值 | 原生运行样本 | Dart 样本 |
| --- | ---: | ---: | ---: | ---: |
| 业务前静置 | 3.3% | 8.6% | 44 | 3 |
| 日期工具连续 200 次 | 150.9% | 198.0% | 255 | 7 |
| 设备状态工具连续 200 次 | 196.1% | 245.5% | 427 | 9 |
| 实际消息流式接收、渲染 | 46.8% | 235.1% | 4,003 | 1,958 |
| 回复完成后的 3 秒 | 39.9% | 146.8% | 1,185 | 540 |

两种工具各执行了一次独立调用并成功，但单次阶段只有 0–1 个 Dart 样本、3–4 个原生样本，短于 CPU 采样窗口，**不为其编造单次 CPU 平均值或峰值**。连续 200 次是明确标注的压力测试，不等于用户点一次工具的持续负载。

本轮真实回复为 1,505 字符、896 输出 token，收到结束标记，聊天 ID 为 `f51ef9a8-814a-43d3-826b-15dfefe17d4e`。

## 工具调用链：线程调度广播有实质成本

原生栈中能直接看到：

```text
JsEngine::scriptRequestStepTask
  → NativeHostJavaScriptRuntime::callHostJavaScriptFunction
  → V8 → host_callback
  → NativeHostRuntimeTaskSchedulerHost::scheduleHostRuntimeBlockingAsyncTask
  → RuntimeTaskPool::scheduleBlocking
  → RuntimeTaskQueue::sendMany
  → tokio::sync::watch / BigNotify::notify_waiters
  → mio::waker::Waker::wake
  → kevent
```

`notify_waiters` 出现在日期工具压力测试 **12.55%**、设备状态工具压力测试 **17.56%** 的原生运行样本中。该比例包含通知子调用，不等于函数本体的 self CPU。

源码 [runtime_task_scheduler.rs](../hosts/common/operit-host-native-scheduler/src/runtime_task_scheduler.rs) 的共享任务队列采用 `watch` 广播：入队、任务开始 poll、结束 poll、销毁及分派后都会发送变化。普通 runtime 池有 16 个 worker；一次状态变化可以使多个没有拿到任务的 dispatcher 参与调度。任务队列空时，poll 前后仍发送通知。**这是工具链上已找到的具体额外工作，优先优化目标应是按可运行任务和接收者定向通知、取消空队列状态广播。**

不能简单替换为 `notify_one`：现有队列会选择特定的低负载 worker，如果通知被其他 worker 消费，可能滞留任务；需要同时处理接收者选择、队列交接和关闭语义。

其他成本包括 `ToolPackage` 克隆、内存复制和文件元数据访问。日期阶段带克隆函数的样本占 21.18%，设备状态阶段为 11.94%；这些 inclusive 比例和通知比例可能重叠，不能相加。

火焰图仅纳入 Instruments 标记为 `Running` 的样本。`kevent` 同时存在于通知路径和 Tokio reactor 路径，**不单凭 syscall 名称判断业务忙等或等待耗 CPU**。

## 消息接收：界面帧工作是主要热点

流式阶段，原生采样中主线程占 **52.6%**，raster 线程占 **10.1%**。二者合计约 62.7% 的本阶段原生运行样本。

Dart 侧的主要调用链：

```text
SchedulerBinding.handleDrawFrame
  → WidgetsBinding / RendererBinding.drawFrame
  → PipelineOwner.flushLayout
  → RenderViewport / RenderSliverList
  → RenderChatMessageSliverList.performLayout
  → RenderChatMessageExtentBox.performLayout
  → RenderParagraph.performLayout
  → TextPainter.layout
  → _NativeParagraph.layout
```

| Dart 调用链节点 | 流式阶段 Dart 样本占比 |
| --- | ---: |
| `PipelineOwner.flushLayout` | 28.40% |
| `RenderChatMessageSliverList.performLayout` | 24.46% |
| `RenderChatMessageExtentBox.performLayout` | 23.65% |
| `RenderParagraph.performLayout` | 17.52% |
| `TextPainter.layout` | 17.47% |
| `_NativeParagraph.layout` | 15.07% |
| `PipelineOwner.flushSemantics` | 16.80% |
| `_FfiConnection._receive` | 3.17% |
| `_FfiConnection._receiveFrame` | 2.35% |

这些数字的分母是 **Dart 采样集合**，与原生线程占比、进程 CPU 百分比不同。表中父子调用高度重叠，不能相加；`performLayout` 包装层占比高也不代表包装层自身计算慢。可确认的是大量工作落在它下面的布局和排版路径。

对应源码：

- [ChatMessageExtent.dart](../apps/flutter/app/lib/ui/features/chat/components/ChatMessageExtent.dart)：行高度测量和自定义 sliver 布局。
- [ChatArea.dart](../apps/flutter/app/lib/ui/features/chat/components/ChatArea.dart)：流式高度变化、逐帧底部跟随、消息定位几何更新。
- [FfiCoreTransport.dart](../apps/flutter/app/lib/core/bridge/FfiCoreTransport.dart)：消息桥接接收，当前采样占比低于布局链。

## 回复完成后：仍有帧工作，但不能宣称永久忙转

完成后的 3 秒平均 CPU 仍为 **39.9%**。原生样本主线程占 61.7%、raster 占 15.5%；Dart 中 `flushSemantics` 占 **38.52%**、`flushLayout` 占 **8.15%**。

`ChatArea` 的 `_beginBottomFollowCompletion → _tickBottomFollow → jumpTo` 路径允许回复完成后继续逐帧追赶底部。其当前误差修正系数是 0.8/秒，停止阈值是 1 像素：没有新增内容时，较大的剩余滚动距离仍可能逐步收敛并驱动多帧布局、绘制和语义树更新。

**后续应验证完成阶段的滚动是否驱动了这些帧，并比较限制收尾帧工作后的 CPU。** 这轮已有函数堆栈和源码依据，但没有做关闭自动跟随的对照试验，因此不把全部完成后 CPU 归因于该 ticker，也不宣称发生永久空转。

## 对手机发热能得出的结论

这轮找到了两类真实 CPU 工作：工具链的调度广播，以及消息接收和完成后的界面布局、排版、语义树更新。持续接收时的额外 CPU 工作是后续降低耗电和发热应优先减少的对象。

但是，当前是 **Apple M5 上的 iOS 模拟器、Flutter debug 模式并附加采样**。Debug 检查、JIT/GC、调试语义树和模拟器图形后端都会影响占比；这里没有读取到物理 iPhone 的温度、瓦数或降频记录。**本报告定位的是可优化的代码工作，不能据此给出手机温度或声称真机发热已解决。**

本轮未修改上述生产热点实现。已有 Markdown 修改来自其他工作，未覆盖；已输出分析数据和可重用的火焰图生成工具。

## 复现产物生成

```sh
python3 tools/benchmarks/ios_cpu_flamegraph.py \
  --root /tmp/operit-performance-20261027 \
  --output docs/assets/performance-2026-10-27
```

生成器只依赖 Python 标准库；浏览器产物不依赖外部脚本。
