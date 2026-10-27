# 2026-10-27 macOS 空闲性能与 JavaScript 执行链路改造

本次从真实 macOS Flutter profile 进程定位空闲唤醒，替换取消、超时、Promise 完成和 IPC 接入的事件链路。

## 实现

- 删除 `HostJavaScriptInterruptHandler`、`setHostJavaScriptInterruptHandler` 以及 V8 每 2 毫秒扫描存活 isolate 的线程。Host API 使用取消令牌订阅和 RAII 退订，取消直接终止当前 isolate。
- 截止时间由宿主协作式异步执行器的单次定时器驱动。没有截止时间时，不创建定时等待；截止时间相同的归属切换不重复唤醒定时器任务。
- 每次替换执行归属都更换代次，旧令牌的迟到事件无法终止下一次执行。嵌套执行作用域在正常返回和异常展开时恢复原归属。
- 删除 Promise 每 200 毫秒兜底推进任务。宿主结果到达、最后一份 detached 引用释放时直接通知所属执行器，通知通过已有 requested/scheduled 状态合并。
- 内部定时器订阅使用原始 Promise 方法，调用引用由定时器自身管理，避免被公开 Promise 包装器重复保留。
- 取消先退休会话并拒绝宿主等待，再执行 `finally` 清理。清理使用独立 100 毫秒截止事件；其他调用恢复执行时继续使用自己的截止时间。
- Web 共享内存控制字承载取消状态、截止时间和通知版本。数据帧、确认和取消统一使用 `Atomics.notify` 唤醒，等待时使用精确剩余时间或无限等待，删除 5 毫秒超时重试和固定 120 秒 watchdog。
- macOS Unix socket 接入改为 Tokio I/O readiness 与停止事件的异步选择，删除 20 毫秒检查、专用接入服务循环及同步 join。接入和停止在同一状态锁下串行化，停止后不再登记新会话。会话字节流读取继续使用现有原生流适配器。

## 测量方法

Apple M5，10 核，16 GiB，macOS 26.6（25G72），接交流电。使用项目固定 Flutter `3.41.10-ohos-0.0.2-beta.operit.6`，macOS profile 模式，均在启动后的默认页面无主动界面交互，并保留实际后台服务。

以 `proc_pid_rusage(RUSAGE_INFO_V2)` 的计数差分采样 30 秒。CPU 的 100% 代表一个核心；内存使用 physical footprint；唤醒使用进程 interrupt/package idle 计数。采样期间不附加原生栈分析器。原生 `sample` 在独立窗口运行，用于验证旧线程消失和辨别剩余开销。

本机 CPU 原始计数是 Mach absolute ticks，`mach_timebase_info` 为 125/3 纳秒每 tick。通过约 0.25 秒的 CPU 工作与 `time.process_time()` 交叉核对：原始 6,000,252 ticks 换算为 0.2500105 秒；误当纳秒会得到 0.006 秒。原始基线和改后数据均按正确单位转换。

最终比较数据与一秒粒度原始采样位于 `assets/performance-2026-10-27/`。

| 指标 | 原始基线 | 最终构建，第 1 轮 | 最终构建，第 2 轮 |
|---|---:|---:|---:|
| CPU（单核 100%） | 2.61% | 1.29% | 1.47% |
| 进程 interrupt 唤醒 | 489.98 次/秒 | 51.54 次/秒 | 27.31 次/秒 |
| package idle 唤醒 | 213.75 次/秒 | 19.89 次/秒 | 10.30 次/秒 |
| physical footprint | 243.79 MiB | 211.75 MiB | 285.64 MiB |
| 磁盘读取 | 0 B | 0 B | 28,672 B |
| 磁盘写入 | 32,768 B | 32,768 B | 1,019,904 B |

最终构建两轮各约 30 秒：CPU 相对基线下降约 44%–50%，interrupt 唤醒下降约 89%–94%，package idle 唤醒下降约 91%–95%。第二轮出现约 1 MiB 的后台写入和瞬时 CPU 峰值，因此这两个无界面交互窗口包含真实后台业务活动，不能当作完全相同的纯空载微基准。内存没有表现出稳定收益，本次不宣称内存优化。

最终原生栈采样确认不存在 `operit-v8-interrupts` 线程和旧 IPC listener 服务循环。剩余采样活动包括节点发现、网络授权状态重建、同步 operation/outbox 读取及安全存储访问；这些后台路径不属于已经删除的 JavaScript 扫描机制。


这些结果反映单机短时空闲窗口。后台连接发现、同步和系统调度会使一秒读数波动；未测量瓦特或电池续航，不能把 CPU/唤醒下降比例直接当作耗电下降比例。没有建立业务吞吐或界面帧率的前后对照，因此不据此宣称对应提升。

## 验证

- JavaScript 核心：118 通过，4 个手动性能探针忽略。覆盖取消后的 `finally`、死循环清理的终止与 realm 复用、取消清理截止时间不限制其他调用、共享 Promise 的归属、异步完成通知及 detached 资源收尾。
- 原生宿主调度器：50 通过，1 忽略；使用 `--test-threads=1` 验证真实 V8 中断、精确截止时间、迟到取消和运行时复用。
- 取消令牌：3 通过，覆盖通知一次、迟到订阅、退订和截止事件。
- Apple IPC：7 通过，覆盖端点占用、陈旧 socket 恢复、实例隔离、空闲连接关闭和停止后重启。
- Node 浏览器/桥接：24 通过，包含实际 worker、无周期唤醒的等待、共享内存取消、死循环截止、分块 Unicode 帧和 Promise 生命周期。
- TypeScript `tsc --noEmit` 通过。
- 修改后的 Web Rust 生产模块与真实 Host API 一起，使用独立临时 crate 在 `wasm32-unknown-unknown` 目标下检查通过。
- 完整 Web Rust 构建仍被已有平台边界检查拦截：`core/crates/tool/services/src/tools/transfer/FileTransferTools.rs` 直接使用 `std::fs`。没有关闭或绕过该检查。独立模块检查不等于整个 Web 应用构建通过。
- macOS profile 应用构建及启动成功，Dart VM `getVM` 返回主 isolate。

## 复测

在 `apps/flutter/app` 中启动：

```sh
env FLUTTER_XCODE_ARCHS=arm64 ./.fvm/flutter_sdk/bin/flutter run -d macos --profile --no-pub --vm-service-port=8181
```

本机 Xcode 构建的临时架构覆盖仅用于启动验证，没有修改发布架构设置。

回到仓库根目录，用应用 PID 测量：

```sh
python3 tools/benchmarks/macos_idle.py PID --seconds 30 --output idle.json
```

原生采样和完整构建/测试日志保存在本次工作目录 `/tmp/operit-performance-20261027/`。


## 消息与日常生活工具实测

2026-10-27，macOS profile 集成测试使用当前身份、现有 `deepseek-flash` 模型配置，通过生成的 Core proxy 调用真实聊天服务、工具分发器、JavaScript 包和宿主 API。工具独立调用，不依赖模型自主选择工具。发送真实请求，未替换模型服务。测试入口是静态页面，未启动完整主界面及其全部后台服务，因此本节空闲指标不直接与前文正常应用窗口比较。

初次运行通过，但发现 Flutter 的 `LiveTestWidgetsFlutterBinding.handleDrawFrame` 在非 benchmark 模式会持续请求下一帧，导致静态测试页面空闲也有约 8% CPU 和约 450 次/秒唤醒。这是测试框架的采样干扰。最终测试在初次绘制后切换为 `benchmark`，停止测试页面主动请求帧；只在阶段起止读取进程计数。该调整仅作用于集成测试。

最终复测成功，数据位于 `assets/performance-2026-10-27/chat-tools.json`：

| 阶段 | 次数 | 总耗时 | 平均每次耗时 | 本进程 CPU 时间 | 阶段平均 CPU（单核 100%） |
|---|---:|---:|---:|---:|---:|
| `daily_life:get_current_date` | 5 | 17.250 ms | 3.450 ms | 23.451 ms | 135.95% |
| `daily_life:device_status` | 5 | 42.631 ms | 8.526 ms | 22.411 ms | 52.57% |
| 消息提交至完整回复 | 1 | 915.915 ms | 915.915 ms | 175.492 ms | 19.16% |

工具各 5 次均成功。消息内容为“这是一条性能回归测试消息。请只回复：收到，消息发送正常。”，实际回复为“收到，消息发送正常。”，收到带完成时间的助手消息。停止测试进程并恢复正常主应用后，通过仅限该测试聊天的只读 SQLite 查询，核实 1 条用户消息、1 条已完成助手消息及双方完整文本均已落盘。模型记录等待时间 749 ms、输出时间 101 ms，输入 4070 tokens、输出 6 tokens。消息完成通过流订阅等待，没有新增轮询。

CPU 时间是进程内所有线程之和，不包含 `device_status` 启动的 `uname` / `sw_vers` 子进程，也不包含远端模型计算。当前时间工具阶段只有 17 ms，多线程并行可使阶段平均 CPU 超过 100%；该值不是峰值，也不代表持续占用。工具延迟是同一批 5 次调用总耗时除以次数，没有采集延迟分位数。

调用前 10 秒空闲窗口平均 CPU 1.12%、interrupt 唤醒 28.49 次/秒；调用后 10 秒平均 CPU 1.59%、唤醒 32.20 次/秒。这个短窗口未出现持续高 CPU 或恢复到旧扫描频率的现象。没有测量实际功耗，也没有建立业务性能的改动前后对照。

`device_status` 在 Apple 宿主目前返回的 `cpuInfo` 是 `arm64` 架构，并非 CPU 占用率；内存和电池等字段还有占位值。因此本表 CPU 数据取自 macOS `proc_pid_rusage`，不能把工具里的占位值当成实际硬件状态。

复测会使用当前身份创建测试聊天并发送真实模型请求，需显式开启，默认测试不会发送消息。在 `apps/flutter/app` 执行：

```sh
env FLUTTER_XCODE_ARCHS=arm64 ./.fvm/flutter_sdk/bin/flutter drive \
  --target=integration_test/chat_and_daily_life_smoke_test.dart \
  --driver=test_driver/live_smoke_driver.dart \
  -d macos --profile --no-pub --dart-define=OPERIT_LIVE_SMOKE=true
```

集成测试通过；两个新增 Dart 文件的定向 `flutter analyze --no-pub` 通过。完整本地日志为 `/tmp/operit-performance-20261027/live-smoke-no-frame-loop.log`。

测试后恢复完整主应用，VM `getVM` 返回主 isolate。额外 30.11 秒无交互窗口平均 CPU **1.66%**、一秒中位数 **0.076%**、一秒最高 **14.88%**；interrupt 唤醒 **27.74 次/秒**、package idle 唤醒 **11.53 次/秒**。窗口有约 1.60 MB 后台写入，均值包含间歇后台活动；一秒最高值不是瞬时峰值。原始数据在 `assets/performance-2026-10-27/normal-after-chat-tools.json`。
