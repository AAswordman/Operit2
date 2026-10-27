# iOS 重新编译后：发送消息 CPU 火焰图复测

2026-10-27 21:24:14—21:24:37（Asia/Shanghai）采集。使用用户重新编译并启动的 **PID 78568**，iPhone 18 Pro Max / iOS 27 模拟器，Apple M5 主机，Flutter debug。连接现有 Flutter VM 和配置，通过真实业务入口创建可见聊天并发送一条消息，实际界面参与流式接收。没有更换模型、应用配置或 API key。

- [交互式火焰图及 CPU 曲线](assets/performance-2026-10-27/message-retest-78568/iphone-flamegraphs.html)
- [Speedscope 堆栈](assets/performance-2026-10-27/message-retest-78568/iphone-profiles.speedscope.json)
- [CPU 时间、线程及源码热点原始汇总](assets/performance-2026-10-27/message-retest-78568/iphone-cpu-stacks.json)
- [上一轮报告](performance-ios-2026-10-27.md)

## 实测 CPU 时间

CPU 为应用所有线程累计，单核满载为 100%；CPU 秒表示这些线程在阶段内消耗的累计 CPU 时间，不是请求等待时间。通过 `proc_pid_rusage` 累计 user + system 计数，每约 100 ms 采集一次，与应用通过 VM Extension 发出的墙钟和 `Timeline.now` 阶段边界对齐。边界窗口按重叠时间加权，因此 CPU 秒是采样边界内的估计值。峰值只取完整落在阶段内的采样窗口。

| 阶段 | 墙钟时长 | 累计 CPU 时间 | 平均 CPU | 约 100 ms 峰值 | Native 运行样本 | Dart 样本 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 发送前静置 | 3.003 秒 | 0.092 CPU 秒 | 3.08% | 20.88% | 60 | 3 |
| 消息发送、流式接收与真实 UI | 7.928 秒 | 3.221 CPU 秒 | 40.63% | 182.98% | 3,137 | 1,803 |
| 回复完成后的 3 秒 | 3.002 秒 | 0.281 CPU 秒 | 9.37% | 73.02% | 281 | 70 |
| 再等待 5 秒后的静置窗口 | 3.003 秒 | 0.088 CPU 秒 | 2.91% | 11.54% | 35 | 3 |

模型为已有 `deepseek-flash`，本轮回复 **1,441 字符、863 输出 token**，完整收到“流式接收测试完成”标记。聊天 ID：`5d4335d0-bf6f-4b40-8936-967f26c7389e`。发送内容与上一轮相同，要求约 800 字、8 个 Markdown 二级标题，包含加粗和列表，不调用工具。

## 与上一轮对照

| 阶段 | 上轮平均 CPU | 本轮平均 CPU | 相对变化 |
| --- | ---: | ---: | ---: |
| 流式接收与 UI | 46.80% | 40.63% | -13.2% |
| 回复完成后 3 秒 | 39.93% | 9.37% | -76.5% |

上一轮流式持续 8.716 秒、1,505 字符、896 token；本轮持续 7.928 秒。两个模型回复长度、流式分片和 debug 预热状态不同，以上为两次实际记录的对照，不能将变化全部归因于某个改动，也不能推导真机功耗下降比例。两轮相同 3 秒收尾窗口累计 CPU 时间约为 **1.199 → 0.281 CPU 秒**。

## 剩余成本落点

流式阶段原生运行样本按线程划分：

| 线程 | 占本阶段 Native 运行样本 |
| --- | ---: |
| Main Thread（包括 Flutter UI） | 50.43% |
| DartWorker | 22.47% |
| io.flutter.1.raster | 9.15% |
| 其余线程合计 | 17.95% |

`DartWorker` 中后台编译链占全部原生运行样本 **16.07%**，并发 GC 标记任务占 **4.02%**。两者属于上述 DartWorker 的组成，不能再次与线程占比累加：

```text
DartWorker
  → dart::ThreadPool::WorkerLoop
  → dart::BackgroundCompiler::Run
  → dart::Compiler::CompileOptimizedFunction
  → dart::CompileParsedFunctionHelper::Compile
```

本轮进程为 debug/JIT，刚重新编译启动，后台编译出现在本次实测中；不能直接当作 release/AOT 的开销。

Dart UI isolate 栈中，包含子调用的热点如下：

| 调用栈节点 | 占本阶段 Dart 样本 |
| --- | ---: |
| RenderChatMessageSliverList.performLayout | 21.46% |
| RenderChatMessageExtentBox.performLayout | 21.80% |
| RenderMarkdownTextReveal.performLayout | 20.13% |
| RenderParagraph.performLayout | 21.08% |
| TextPainter.layout | 21.63% |
| _NativeParagraph.layout | 20.63% |
| PipelineOwner.flushSemantics | 17.08% |
| RenderMarkdownTextReveal.paint | 4.71% |
| _FfiConnection._receive | 1.72% |

主要文字链路：

```text
RenderChatMessageSliverList.performLayout
  → RenderChatMessageExtentBox.performLayout
  → _LiveBottomStreamSizeRenderObject.performLayout
  → RenderMarkdownTextReveal.performLayout
  → RenderParagraph.performLayout
  → TextPainter.layout
  → _NativeParagraph.layout
```

这些是 inclusive 采样占比：父子节点互相包含，不能相加，也不能直接乘应用总 CPU 得出准确函数 CPU 时间。不同采样栈可能被截断；Native 与 Dart 的采样来源、样本数量和符号覆盖也不同，不能混作同一分母。

本轮可看到新的 `RenderMarkdownTextReveal` 已实际运行。逐字绘制只占部分样本，但新分片到达后正文仍需布局，布局与 semantics 依旧是下一步应继续追踪的成本；FFI 接收栈的 inclusive 占比相对较小。仅凭这次采样不能证明所有布局都由逐字动画触发，也不能把本轮文字布局比例较高等同于绝对 CPU 时间上升。

回复完成后 3 秒只有 70 个 Dart 样本，其中正文 `RenderParagraph.performLayout` / `TextPainter.layout` 未采到样本；这表示该窗口未观察到持续正文重排，并不证明每次布局成本为零。随后静置窗口只有 3 个 Dart 样本，应用 CPU 回到约 2.9%。

## 原始采样与可复核性

原始文件位于本机 `/tmp/operit-performance-20261027/message-retest-78568/`：

- `message.trace`：本轮 Instruments Time Profiler 记录。
- `stacks-profile.xml` / `stacks-native.xml`：符号化 `time-profile` 表，仅取 `Running` 行生成原生火焰图。
- `stacks-sample.xml`：原始 `time-sample` 表，未把其中 Blocked 行算作 CPU。
- `stacks-toc.xml`：记录起点、模拟器信息及 PID。
- `current-dart-*.json`：本轮各阶段 Dart CPU 样本。
- `current-cpu-samples.jsonl`：进程 CPU 计数采样。
- `current-collector.json`、`collector.log`、`iphone_workload.expr`：边界、业务结果及触发脚本。

采样工具自身带来开销。模拟器结果用于定位真实业务中的 CPU 热点，不能直接报告 iPhone 真机温度、瓦数或电池续航改善。
