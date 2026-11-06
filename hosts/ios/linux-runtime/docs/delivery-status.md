# ARM64 开发终端交付状态

## 2026-11-06：多 agent 采样优化，原始 Python 循环约 1.01 秒

保留共享访存检查叶入口、延迟 FP 环境切换、驻留代码页的原生证明刷新和 32B 热操作流/16B 冷元数据。已有证明及驻留页无需重复进入 C++ 验证边界，文件观察周期、共享页锁、代码修改、权限和精确故障继续生效。GP permutation、AddShifted、预取及 validation 预分组等无稳定收益候选已删除。Linux 客体继续装载外部 Node/Python 版本。

在 Apple M5 宿主上，原始 Python 源文件、相同 rootfs、固定函数替换关闭，Release/O3/thin LTO、诊断 OFF，最终交替五对循环中位数 **1.318184 → 1.013432 s**，五对全部变快；完整进程中位数 1.854990 → 1.521557 s。前者是本轮最初冻结的 native-flags 对照，非 Git HEAD 或原 region 版本。checksum 全为 1267865，零 unsupported syscall、零 accelerated call。

单独的删除 C++ 验证边界候选为 1.187445 → 1.026222 s；最终诊断中热区专用 C++ 验证从早期约 295 万次降至约 4.96 万次，约 848 万次页证明刷新在汇编内完成。独立诊断进程客体路径略有变化，计数用于定位，不把诊断时间当性能验收。

**0.5 秒目标未达到，当前仍需约 2.03 倍循环吞吐。** 最终完整宿主构建及 CTest **42/42 通过**，诊断 ON 执行器回归通过；全程未使用设备或模拟器。方法、候选去留、全部限制和原始证据见 [多 agent 采样与优化](performance/2026-11-06-multiagent.md)。

## 2026-11-06：静态汇编后端替换（本日早期历史测量）

生产入口已改为 `ExecutionEngine`、真实 CFG 和构建时生成的 ARM64 汇编，移除旧 region/kernel 执行层及 `KernelEntry` / `bind_*` ABI。跨图保留角色值，有界多态间接缓存、私有页索引、精确前缀退休和代码行证明复用已接通；不生成运行时宿主机器码，也不把 Node/Python 编进宿主。

同一 Release / O3 / thin LTO / 默认部署设置、同一 rootfs 和原始 Python 文件，交替三对的中位数：

| 设置 | 原 region 热循环 → 当前 | 原 region 完整进程 → 当前 |
|---|---|---|
| 默认关闭固定函数加速 | 2.550325 → **2.333802 s** | 6.623871 → **4.026016 s** |
| 开启固定函数加速 | 2.521824 → **2.330999 s** | 6.450428 → **3.951647 s** |

所有 checksum 为 1267865。默认设置的宿主峰值 RSS 中位数从约 3.00 GB 降到 319 MB；这不是客体 resident_bytes。**0.5 秒热循环验收线仍未达到，尚需约 4.7 倍混合负载吞吐；没有全局 SSA 值分配或广泛通用融合，不能宣称已经完成强优化终态。**

最终 macOS Release 完整 CTest **41/41 通过**，含 **5,253 项硬件 ISA 形式审计**；Node 单 CPU version/basic 的固定加速关闭/开启四种组合通过，basic 使用现有 `--jitless`。iOS arm64 和 Simulator arm64 静态库构建通过；未构建 Flutter Runner、未重打包 XCFramework、未运行设备或模拟器。本轮未重跑 Node workers/threadpool/network/default V8 JIT。

测量方法、全部限制、代码范围和构建证据见 [2026-11-06 静态执行器实测](performance/2026-11-06-static-executor.md)；当前结构见 [静态执行器](static-execution.md)。证据冻结于 `tools/ios-runtime/arm64/build/static-execution-2026-11-06/`；最终以 `python-matched-*.json` 为准，较早中间报告的 LTO 设置不同。

## 2026-11-05：区域编译与共享代码依赖优化（历史，已被静态汇编执行器替代）

删除常量/CSE 的逐节点树分配、取指回调类型擦除、每区域独有的代码字节拷贝和状态映射的反复扩容；区域共享不可变代码页。修复快照槽冲突造成的重复所有权，文件/并行校验复用同 epoch/作用域的 64 字节证明，保留精确故障与失效语义。

独立正反序基准：128 条 ALU 区域编译约 **9.27 → 5.20 µs**，混合区域 **18.62 → 14.81 µs**；共享文件单页校验 **9.81 → 4.92 ns**，八页 **9.75 → 6.46 ns**。匿名私有页校验没有加速，约增加 **0.2–0.5 ns**。方法及完整样本见 [代码编译与校验复测](performance/2026-11-05-code-pages.md)。

同场全程采样：编译 **23.35% → 17.03%**，校验 **12.59% → 12.21%**。原样 Python 循环 **2.203204 → 2.125552 s**，完整进程 **3.308711 → 2.965695 s**；分别减少约 **3.52% / 10.37%**，不能用占比推算模块精确秒数。

只验证 macOS ARM64 Release CLI，未构建 iOS、启动模拟器、使用设备或构建 Flutter App。完整 CTest **41/41 通过**；原 Python 基准未修改。证据在 `tools/ios-runtime/arm64/build/code-pages-2026-11-05/`。

## 2026-11-05：内存执行核热路径搬运消除（前一轮快照）

将普通访存、成对访存及二读窗口的慢路径强制独立编译，消除快速命中时不必要的栈帧和寄存器保存；
翻译条目由 72 字节压回 64 字节并对齐，映射 epoch 移至表头。代表性的普通 64 位读取从 240 字节栈帧
变为零栈访问。具体实现、适用场景和全部原始证据见 [内存执行核复测](performance/2026-11-05-memory-kernels.md)。

本轮同场对照的暖私有页普通读取约 **2.00 → 0.90 ns/值**，写入约 **2.27 → 0.86–0.92 ns/值**。
全程主线程栈采样的内存类别 **26.10% → 16.67%**，延后窗口 **29.26% → 17.06%**。
原样 Python 循环中位数 **2.751030 → 2.241368 秒**，完整进程 **3.931307 → 3.349221 秒**。
控制组是上一轮静态区域后端，不是原汇编解释器；采样占比不是各部分的精确秒数。
跨页、TLB 冲突和并行同步路径没有一致的同等幅度收益，不能将暖私有页结果外推到所有访问。

只构建 / 运行 macOS ARM64 Release CLI，**没有构建 iOS、启动模拟器、使用设备或构建 Flutter App**。
最终完整 CTest **41/41 通过**，含 5,253 项硬件 ISA 审计与新增 3,840 组访存 ABI 组合。
证据目录：`tools/ios-runtime/arm64/build/memory-kernels-2026-11-05/`，含冻结的前后二进制、
`binaries.json`、`memory-comparison.json`、`ctest.log`、`python-comparison.json` 及 `profile/`。


## 2026-11-05：静态区域后端组件优化（较早快照）

该轮生产入口为 `RegionEngine`，旧解释器、gadget 汇编和对应私有 ABI 已移除。
执行格式与边界见 [当前静态汇编执行器](static-execution.md)；局部基准、同口径前后采样、
完整回归及实际性能限制见 [2026-11-05 组件复测](performance/2026-11-05-static-regions.md)。

只构建并运行 macOS ARM64 Release CLI；未构建 iOS、启动模拟器或使用设备，未构建 Flutter App。
最终同版本完整 CTest **41/41 通过**，包括 rootfs 集成与 5,253 项硬件 ISA 审计。
原样 Python 工作负载仍慢于旧汇编后端，不能将组件收益称为应用整体提速。

全程主线程栈采样的状态搬运占比从 **15.77% 降至 10.44%**，区域控制从 **12.63% 降至 10.06%**；
内存为 **24.25% → 25.50%**，编译 **14.43% → 19.73%**，校验 **9.08% → 10.51%**。
这是互斥栈样本占比，不是独立硬件周期或绝对节省时间。组件用例和占比的适用范围以复测报告为准。

证据目录：`tools/ios-runtime/arm64/build/region-backend-2026-11-05/`。
其中 `full-regression.log`、`components-binaries.json`、`profile-after-components/` 和组件原始 JSON
对应该历史快照；更早失败或探索测量保留用于说明时序，不为当前代码背书。


## 2026-11-05：静态 ARM64 gadget 解释器（历史，已被上方区域后端替代）

本次修改接入构建时编译的汇编 threaded interpreter，覆盖常用整数、单值/成对内存访问及
条件分支未跳转路径；未覆盖指令、分支跳转和内存检查失败回到原 C++ 路径。公开 C API
不变，运行时不生成可执行代码。该历史方案及其原回退链已被替换，当前实现见 [当前静态汇编执行器](static-execution.md)。

**本轮仅做 macOS arm64 命令行构建和运行；按用户要求没有编译 iOS、启动模拟器或连接真机，
也没有构建或启动 Flutter App。** runtime 为 Release（`-O3 -DNDEBUG`），LTO 关闭。

- 最终同版本相关 CTest **7/7 通过**：`arm64-cpu-validation`、`arm64-cache`、
  `arm64-memory-concurrency`、`arm64-gadgets`、`arm64-filesystem`、`arm64-memory-async-lease`、
  `arm64-runtime`。gadget 测试包含 81,648 组寄存器/条件码差分，以及 SP/XZR、成对访问、
  预算/故障退休和内存回退检查。这是相关子集，不是全量测试或完整 ISA 覆盖。
- 原样数学循环缩小到 100×100，使用相同 Alpine rootfs 和固定函数替换设置，交替运行旧、新
  CLI；每边预热 1 次后取 5 次中位数。旧版 **2.929275 秒**，本版 **2.612029 秒**，
  比值 **1.1215×**，耗时减少约 **10.8%**；每次校验和均为 1267865，未支持 syscall 为零。
  这是数学子负载，不包括原 1000×1000 Pillow 图片生成，不能代表其完整耗时或 iOS 性能。
- 早期只有标量处理器的版本在 Python 启动时崩溃，原因是汇编宏局部标签冲突，已改为唯一标签。
  修复后第一轮对比基本持平；补充成对访存、搬运和分支连续路径后才得到上述最终结果。

证据目录：`tools/ios-runtime/arm64/build/static-gadgets-2026-11-05/`。`build.log`、`ctest.log`、
`ctest-detail.log`、`python-comparison.json` 和 `verification.json` 保留构建输出、逐项测试、
交替样本、二进制与源码 SHA-256；目录还保存当时的 baseline/candidate CLI。
构建目录为 `apps/flutter/app/build/macos/Build/Intermediates.noindex/Runner.build/Debug/Runner.build/DerivedSources/Arm64Runtime/build-arm64`，
其中 `Debug` 属于宿主项目目录名，本次 CMake runtime 配置实际为 Release。

## 2026-11-03 整合记录

**更新日期：2026-11-03（Asia/Shanghai）。本轮关键 syscall 已按下述子集接入独立 runtime；未构建 App、未做 iOS 真机验收。** 下文较早的“未编译 / 用户自行构建”描述保留为历史阶段记录，不覆盖本节结果。Worker 性能优化由用户另行安排，本轮未宣称达到 5 秒目标。

## 关键 syscall 整合

- `rseq`、新版 futex、POSIX 消息队列、System V 消息队列 / 信号量 / 共享内存已进入 syscall 分派和生命周期；POSIX MQ 的 epoll 直接及嵌套 ET 路径已接线。futex 新修复需要在最终集成版本上重跑。
- `io_uring` 已进入分派，但目前仅支持 NOP 和普通文件同步 READ / WRITE 等有限子集，不是完整异步引擎。
- `seccomp` 已接入受限 BPF 过滤及 TSYNC 路径；strict 模式、listener 等不支持。namespace 不支持创建或加入新 namespace，也不完成多线程共享 FS / 文件表的拆分，不能作为容器隔离。
- `ptrace` 已接入调试停止、wait、内存、syscall、受限 fork / clone 自动跟踪及进程生命周期；跟踪关系存在时回退串行。SEIZE、单步等仍不支持，详细边界见 [ptrace 说明](../src/linux/debug/ptrace.md)。

此前独立 macOS Release runtime 的 CTest **35/35 通过**，构建目录为 `tools/ios-runtime/arm64/build/syscall-integration-2026-11-03/`，输出为该目录下的 `ctest-final.log`；该配置不包含 terminal / Alpine rootfs 集成项。随后含 rootfs 的构建在 futex 修复前为 **35/37**，失败为 `arm64-futex2` 和 `arm64-runtime` 中的 futex guest；日志为 `tools/ios-runtime/arm64/build/delivery-review-2026-11-03/ctest-integration.log`。源码继续修改中，最终同版本的全量结果必须重跑，不能引用上述数字宣称通过。

较早版本使用 iOS 模拟器 SDK 成功编译独立静态库 `tools/ios-runtime/arm64/build/syscall-integration-iossim-2026-11-03/liboperit_arm64.a`，不代表当前修改后的源码已编译。这不是 App 构建、模拟器运行或真机测试。

用临时 Alpine 3.23.7 rootfs 副本和 `strace 6.17` 实测，`strace -f` 在 1 / 4 核均跟踪到子进程的 exec、write 和退出，未知 syscall 为零。该测试没有修改产品 rootfs，也不证明完整 strace 兼容；最终集成版本仍须复验。完整调用范围和已知限制见 [syscall-gaps.md](syscall-gaps.md)。

## 四项失败修复（历史）

此前四项失败修复已构建独立 runtime CLI 与测试并通过回归；未构建 App。

修复前，18 项实际执行的 CTest 中 14 项通过；`arm64-isa-audit` 当时未执行。再次单独运行四项失败，**0/4 通过**，日志为 `tools/ios-runtime/arm64/build/failure-fixes-evidence/before-ctest.log`。按用户要求分给四个子代理，分别负责 inotify、路径扩展、调度和 realtime，最终由主线冻结源码统一构建验收。

本轮修复与核实结果：

- **inotify 实现缺陷**：APFS 对同 inode 硬链接的无操作重命名仍发送 vnode 提示，原桥接误报 OVERFLOW / MOVE_SELF；现结合元数据区分实际变化。Darwin `RENAME_SWAP` 只有单边原生重命名提示，现由成功交换后的桥接补齐两侧 inode 身份，保留双向事件和各自配对的非零 cookie。真实快照丢失、队列耗尽仍报告 OVERFLOW。
- **路径测试 ABI 错误**：改正 AArch64 `O_DIRECTORY=0040000`、`O_NOFOLLOW=0100000`，保留原符号链接 chmod 的 EOPNOTSUPP 期望，并补充 readlinkat、dup、rename/unlink 后 FD 身份等回归。原实现未因测试错误而改动。
- **调度测试前置状态错误**：原测试将父线程剩余 90ms 时间片套给新建后 exec 的子线程。现分别验证新线程完整时间片、exec 保留调用者剩余时间片，以及策略变更不重置时间片；未修改调度实现。
- **realtime 用例时序错误**：原始失败与上述调度测试同根因。首次完整 19/19 通过后的重复测试又抓到 worker 并行断言偶发失败，计时证明确有 worker CPU 记账，但 1µs 限制可能先于同时活跃状态触发。现先在无限额下建立真实并行状态，再启用原 1µs 软/硬限制，新增 worker CPU 时间增长与 RTTIME 计数断言，未提高阈值或修改 worker 性能实现。

**最终统一验收：19/19 CTest 通过（含 ISA audit、runtime guest、terminal、Alpine）；四项原失败各连续运行 10 次，40/40 通过。** realtime 子代理另完成正式 CTest 100/100，以及修正用例链接首次冻结运行时库的独立 100/100 验证。未删除或跳过原失败断言。

最终证据目录为 `tools/ios-runtime/arm64/build/failure-fixes-evidence/attempt-2/`，其中 `all-ctest.log`、`repeat-fixed-ctest.log` 保存完整输出，`verification.json` 保存命令、源码与二进制 SHA-256、19 项结果及 40 次成功执行计数。固定源码位于 `tools/ios-runtime/arm64/build/failure-fixes-integration-2/source`。首次验收抓到的偶发失败仍保留于上级证据目录；证据索引为 `tools/ios-runtime/arm64/build/failure-fixes-evidence/README.md`。以上证明本轮测试集合通过，不代表此前全部长期交付目标已完成。

磁盘已有 `tools/ios-runtime/arm64/build/reconciliation-current/precheck-ctest.log` 的 **9/9 通过历史记录**。因此原文“最近一次完整基础 CTest 为 4/8”已经过时；下面的 8 项失败表与整合期协作表保留为历史。9/9 也不覆盖本轮缓存指令、epoll、inotify 桥接、PI futex requeue、调度属性 / FIFO / RR / RTTIME 或此前身份与 apk 源码修改，不能据此声称当前源码通过。

[功能矩阵](completeness.md) 的原 24 组范围继续保留。实现、生产调用、编译和实际运行分别记录；新加用例和旧二进制复现不等于新实现验收。

## 早期 syscall 补齐（历史）

已登记 `process_vm_readv/writev`、`renameat2`、`fchmodat2`、`getresuid/getresgid` 与受限身份设置，共 12 个此前没有中央路由的编号。实现细节、限制和仍缺调用见 [syscall-gaps.md](syscall-gaps.md)。独立回归入口为 `arm64-process-vm`、`arm64-path-extensions`、`arm64-identity`；用户自行构建，本轮未编译或执行这些新用例。旧 CLI 的 12 项 ENOSYS 基线保存于 `tools/ios-runtime/arm64/build/reconciliation-current/syscall-extensions-baseline.json`，只证明旧二进制缺口。

## JS Worker 性能排查（历史，2026-11-03）

用户后续指定 **5 秒**目标。验收按独立 CLI 启动至原始四 Worker / 40000 次累加脚本正常退出的宿主总 wall，默认四核连续三次全部 ≤5 秒；不计诊断副本成绩，不取最快一次替代全部样本，不改原脚本的 15000ms 功能截止。单核保留功能与耗时对照。`profile_workers.py` 已加入该目标及独立功能 / 性能判定，六项 Python 回归验证边界、慢样本、失败 / 缺失样本和 CPU 选择；**5 秒尚未达成，新引擎未编译或测量**。

本轮运行的是已有 Release CLI，SHA-256 `dd6bbc8be4fd3566cd492b7f13dbda8a61cb4e83397d5e407e9840438376d2ed`；**未编译下面的新源码，也没有取得优化后的性能结果**。原始 `tests/node/workers.cjs` 的四 Worker、40000 次共享累加和 15000ms 截止均未修改，继续使用 guest `--jitless` 与固定助手。

证据目录：`tools/ios-runtime/arm64/build/worker-performance-current/`。`baseline.json` 记录原脚本单核 wall 24.988s / host CPU 16.133s、四核 wall 32.540s / host CPU 46.377s，两档都触发 guest 超时；四核确实达到 `maximum_parallel_tasks=4`。`phases.json` 中同一原脚本宿主 Node 对照 0.093s 通过。耗时随运行条件有明显波动，不能从这一次测量推出固定速度比。

额外的临时副本只增加阶段日志，不用于验收。`host-timed-phases.json` 使用宿主单调时钟记录 stderr 到达：四核主脚本于 1.645s 开始，四 Worker 全部 ready 于 16.517s，最后结果于 16.722s；15 秒 guest 定时器于 16.684s 触发。由这次诊断可见，大部分耗时发生在 Worker 启动 / 初始化阶段；释放屏障到最后结果收取约 0.204s，包含计算和消息回传。最初诊断里的 guest `performance.now()` 差值曾出现回退，因此阶段结论采用独立的宿主收取时间，不使用那个差值。

`worker-4cpu.sample.txt` 的 8 秒采样及 `sample-summary.json` 显示：排除宿主空闲工作队列后，26650 个线程栈样本中，9596 个（约 36%）处在页锁 / 全局访问门同步路径，10304 个（约 39%）处在 executor barrier 等待。这是线程栈采样，**不是 CPU 占比，也不表示去掉 barrier 能加速 39%**。栈明确包含 `PageGuard::lock(std::shared_mutex&) → libc++ shared_mutex → pthread_mutex → __psynch_mutexwait`，以及 `PageAccess → sched_yield` / `AccessSynchronization::close()`。当前问题已不能只用历史的单个全局 recursive_mutex 解释。

针对这两处路径写入以下源码修复：

- `Memory::PageAccess` 在 reader gate 内重新缓存已有 resident 页，普通软件 TLB 冲突不再进入全局 writer gate；缺页分配、fork / 私有文件 COW、权限失败仍走原排他路径。
- `ReservationState` 改用短临界区 `PageMutex`：并发读使用原子计数，待写者阻止新读者进入，短暂自旋后 yield。保留页级读写互斥、嵌套 guard、普通写与独占指令的共同锁域；没有移除共享内存同步。
- 新增 `arm64-memory-concurrency`：以持有 reader gate 的另一线程验证冲突 TLB 的前进性，同时覆盖 fork COW、写保护、并发双字读写一致性和四线程 40000 次嵌套原子更新。
- 新增 `tests/profile_workers.py`：仅运行已构建的 CLI，记录二进制 / 原脚本 hash、原条件单 / 四核结果、宿主对照；可额外运行临时阶段副本和独立 macOS 采样，二者不改变原条件通过判定。

本轮已完成五份相关 C++ 源码的语法树解析、Python 解析、`git diff --check`；诊断副本在宿主 Node 验证四份 ready / result / exit 与 40000 次计数，宿主阶段收取器的超时清理也已执行。**新 C++ 尚未编译、类型检查或运行；并发回归及新版本原始 15 秒 Worker 均待用户构建后复测。** JITless 解释执行、每线程重复译码和逐批次协调仍可能有额外成本，当前两处源码修复不等于完整 Worker 性能交付。

### 固定助手的排他范围继续收窄（5 秒目标，源码未构建）

在同一份旧 CLI 采样上按调用链再次汇总，进入 `FixedComponents::execute` 的 748 个线程栈样本中，596 个处于 writer gate close、102 个处于 writer mutex 路径，见 `worker-performance-current/fixed-helper-sample-summary.json`。这是已有二进制的调用栈定位证据，不是新优化运行结果；也不能由该子集直接推算 wall 提速。

源码现将已经驻留、无需 COW、源 / 目的数据各自在单页内的 memcpy / memmove / memset 改走 `Memory::PreparedAccess`：reader gate 保持映射稳定；收集全部认证代码页与源 / 目的物理页；对重复页合并写权限，按物理 reservation 指针统一排序后取得页锁。这样不同 Worker 反向复制也遵守同一加锁顺序，memmove 的 memcpy 尾调用及代码 / 数据别名共享相同锁域。最多准备 16 个页条目，超出容量退回原路径；没有删掉可执行代码指纹、修改代码后的停止检查或 guest 指令预算。

`Memory::accessible` 仅查询映射权限，改为 reader gate 查询。助手代码校验通过 `validate_code` 的受锁复制比较完成，不再在 prepared reader 内调用 raw-span translate；所记录 epoch 取校验前值，其他代码并发写只会导致下次重新验证。没有驻留的页、私有文件 / fork COW、跨页数据、较大操作及 strlen 保留全局排他回退；准备阶段的 MemoryFault 也先退回既有路径，不提前改变 guest 的 fault PC / 指令退休顺序。

`memory_concurrency.cpp` 新增准备失败后 writer 回退、同页读写合并、执行 / 写别名、COW、权限和容量边界。`fixed_components.cpp` 新增 hot memcpy 在另一线程持有 reader gate 时的前进性、4 线程反向源 / 目的复制以及跨页源和目的回退；现有被修改 / 恢复 / 卸载代码的拒绝检查保留。四份涉及的 C++ 已通过语法树解析，Python 的六项 Worker 性能判定回归及 diff 检查通过；**未编译、未类型检查、未执行新 C++，没有 5 秒成绩**。原 24 组功能范围和完整交付目标继续保留。

## 当前源码进展与验证边界

| 项目 | 已写实现与实际路由 | 本轮证据 / 待验证 |
|---|---|---|
| V8 缓存寄存器 | `cpu/aarch64/cache.h` 统一 guest 的 64 字节缓存模型；`execute.cpp` 读取 CTR_EL0 / DCZID_EL0，保留只读寄存器写入的特权异常 | 旧 CLI 执行 `tests/test_cache.py` 在 `0xd53b0023` 复现退出 125；新代码未编译 |
| 缓存维护 / 生成代码 | decoder → `Op::Cache` → DC CVAU / CVAC / CIVAC、IC IVAU、DC ZVA；`Memory::synchronize_code` 验证 guest 地址、内存排序并使译码缓存重新校验；ZVA 精确清零对齐的 64 字节并遵守写权限 | C++ 用例覆盖寄存器、内存与权限边界；无需编译即可生成的 ELF 用例覆盖改写代码再执行、ZVA 相邻块；已用 llvm-objdump 核对编码 / 跳转。旧 CLI 在 ZVA 同样退出 125；新实现待运行 |
| epoll_pwait2 | AArch64 syscall 441 → `dispatch_events` → 现有 epoll / Scheduler；复用 timespec 解码与饱和 deadline、临时信号 mask、阻塞与唤醒清理；不修改输入 timespec | 现有 Alpine Python `ctypes.syscall(441, ...)` 复现 ENOSYS，统计 last_unsupported_syscall=441。新增 C++ 与真实 guest timerfd/eventfd 用例并接入现有测试入口，新代码待编译运行 |
| Darwin inotify data-open | 新增 `InotifyOpenFile`，共享 OpenDescription 接 OPEN / ACCESS / 两种 CLOSE；普通 / 定位 / 向量 / 目录读取和文件传输接 ACCESS；按 pinned parent + name 跟踪 hardlink、rename / unlink / 覆盖目标、IN_EXCL_UNLINK；mmap 保留引用，最后关闭后才释放未链接 inode 的 DELETE_SELF / IGNORED；Linux 用原生事件，向量读取保留 native readv/preadv 的 EOF 行为 | 旧 CLI + Alpine Python ctypes 已复现 OPEN / ACCESS / CLOSE_WRITE / CLOSE_NOWRITE 全部 errno 95，CREATE 可注册。基线日志：`tools/ios-runtime/arm64/build/reconciliation-current/inotify-guest-bridge-baseline.json`。新增 `tests/inotify.cpp` / `arm64-inotify` 和 guest 生命周期回归；仅完成源码解析与 diff 检查，未编译运行。虚拟节点、exec 装载、匿名 O_TMPFILE 的父目录名字事件、宿主外部 data-open 仍未覆盖 |
| PI futex requeue | syscall 98 的 op 11 / 12 → `threads/futex_pi.cpp`；条件等待阶段不捐赠，迁移后接现有 PI 队列与传递捐赠，空闲目标先提交所有权再唤醒；保留绝对超时，按迁移阶段区分无 SA_RESTART 的自动重启 / EAGAIN；普通 WAKE / REQUEUE 拒绝混用 PI waiter，错误目标与部分转移按项报错；source 按实时优先级 / 同级 FIFO 排序 | 旧 CLI 两操作均复现 errno 38，基线 `tools/ios-runtime/arm64/build/reconciliation-current/futex-requeue-pi-baseline.json`。新增 `tests/futex_pi.cpp` / `arm64-futex-pi` 覆盖参数、顺序、部分迁移、超时、真实信号帧恢复、捐赠链、退出、共享别名 / 不同进程地址空间与失效映射；`guest/futex_requeue_pi.c` 登记单 / 四核共享与私有、空闲与占用目标。仅源码解析及接线检查通过，未编译运行；竞态、robust PI 与完整应用工作流仍待验 |
| 调度属性 / NICE / RTPRIO | syscall 274 / 275 → `dispatch_threads` → `threads/scheduling.h` 的版本化 ABI → Task / 现有调度权限校验；`ResourceLimits::exchange` 开放 NICE / RTPRIO 变更，仍受 guest 硬限额与能力约束；普通 nice、实时优先级、reset-on-fork 与 KEEP_POLICY / KEEP_PARAMS 接入既有字段与 fork 重置；未知扩展、只读输入、短输出与 size 反馈按项处理 | 旧 CLI 的两个接口均复现 errno 38：`tools/ios-runtime/arm64/build/reconciliation-current/scheduling-attributes-baseline.json`。新增 `arm64-scheduling` C++ 用例与 `guest/scheduling.c`，覆盖 ABI、权限、限额、继承、错误不修改状态与实际 runnable 选择；只做源码解析 / 接线检查，未编译运行。Deadline、自定义 runtime / 利用率 / reclaim / overrun 标志明确不支持；FIFO / RR 的新执行接线另列下行；不能因此宣称调度子系统完整 |
| FIFO / RR 执行与 guest yield | Scheduler 持久实时队列；高优先级抢占保留 FIFO 位置，阻塞 / 唤醒与 PI 降级更新队列；RR 从串行及 worker 的实测线程 CPU 计时扣除 100ms 预算，yield / 策略切换保留余量，新任务取得完整预算；124 → guest yield、127 → guest interval。多核匹配不同 eligible CPU；不足两个任务的批次不推进串行轮转 | 旧 CLI 的 127 复现 errno 38，124 返回 0 仅为旧行为：`tools/ios-runtime/arm64/build/reconciliation-current/realtime-scheduling-baseline.json`。新增 `arm64-realtime` 直接生成 FIFO / RR ELF，C++ harness 覆盖单 / 四核与实际 yield / CPU 预算；另加 PI 队列回落、策略切换与单 CPU 绑定的串行回退回归。已生成并反汇编 ELF，源码解析及 diff 检查通过；**全部新 C++ 未编译、未执行**。抢占发生于解释器安全点（串行 4096、并行最多 32768 指令），普通策略仍为按优先级轮转，不是完整 Linux CFS / 实时子系统 |
| 连续实时 CPU / RLIMIT_RTTIME | 开放 164 / 261 的 RTTIME 变更，继承 / 硬限权限沿用既有 guest 规则；每线程累计有有限额时的实际实时 CPU 服务，普通 PI owner 暂时进入 RT 类也计入；真实唤醒重置，yield / 抢占保留；软限发进程级 SI_KERNEL SIGXCPU 并推进共享软限 1 秒，硬限优先发 SIGKILL。事件泵先结算并检查，再服务 timer 唤醒 | 旧 CLI 修改 RTTIME 复现 errno 95，见 `tools/ios-runtime/arm64/build/reconciliation-current/rttime-baseline.json`。新增阈值 / 单位 / 权限 / 继承 / 溢出 / 唤醒 / PI 回落用例；`arm64-realtime` 加入循环 gettid / yield 的原始 ELF，覆盖线程间不聚合计时、屏蔽 SIGXCPU 后硬限杀进程、普通策略不计入，以及 FIFO / RR 单 / 四核实际 CPU 触发。已生成并反汇编 ELF，源码解析 / 接线检查通过；**全部新 C++ 未编译、未执行**，不等同原生 Linux tick 精度或资源限制整组通过 |
| apk 索引落盘 | Darwin 不支持可发布的 O_TMPFILE 时在 open 阶段返回 EOPNOTSUPP；apk 可使用命名临时文件与 rename 回退 | 本地 HTTP 仓库在旧 CLI 复现 EXDEV；在独立 VM 中触发 apk 回退后索引保存通过。此项是能力回退，完整 linkable O_TMPFILE / proc FD 链接仍未实现 |
| macOS Super Admin shell | `super_admin.ts` 的 macOS state 已声明 shell；类型 shell 进入 AppleTerminalHost 的 ARM64 后端，原生 bash / zsh 各用独立会话 | 插件路由 / 输出 18 项 JS 测试通过；App 未重新编译 |

`arm64-cache` 已加入 CTest，不需要 guest 编译器。`arm64-cpu-validation` 与 `arm64-virtual-filesystem` 增加对应边界断言；`arm64-runtime` 新增 guest events 与 futex_requeue_pi fixture（后者复用 threads.S），由既有测试编译阶段准备。`arm64-futex-pi` 与 `arm64-scheduling` 已登记独立 C++ 目标；`arm64-realtime` 复用 scheduling 目标，以 Python 生成 FIFO / RR / watchdog 原始 ELF 并要求 guest dispatch 与 RTTIME 两个独立成功标记，不调用编译器。新增 scheduling guest 也接入原有 fixture 编译入口。**本轮没有运行这个会编译 guest fixtures 的测试入口。**

## 当时仍需推进的原范围（历史）

本节保留先前阶段的待办；部分接口随后已接入并测试，以本页顶部的关键 syscall 整合为准。

以下均未因为本轮修改而完成：完整 EL0 / FP / SIMD 形式与异常；ELF / exec 失败回滚；匿名文件、共享映射、FD 生命周期与锁；Darwin inotify 新桥接的运行验收，以及虚拟节点 / exec 装载 / 宿主外部 data-open 覆盖；PI futex requeue 新路径、跨核竞态与 robust PI 的运行验收；调度新属性 / NICE / RTPRIO 及 FIFO / RR 队列、时间片、guest yield / interval、PI 队列与串并行回退的运行验收；完整普通公平调度与精确实时抢占；RTTIME 线程计时与信号流水线的运行验收；MEMLOCK / CORE / MSGQUEUE 等资源限制剩余约束；中央路由缺失的 syscall（包括 io_uring、fanotify、seccomp、ptrace、namespace 接口）；网络选项、消息与错误队列；原条件 Worker；App 取消、恢复与设备长稳。继续按 `completeness.md` 的 24 组核对实现、调用者和验证证据，不能由本轮几个接口推断整组完成。

目录管理继续由 native 负责；用户要求的统一工作区挂载尚未开展，不恢复已删除的 Dart 目录配置与选择器。App 的 Node `--jitless` 兼容默认值在真实 JIT 工作流通过前保持现状。

## 整合期事实与证据（历史，已被上方记录补充或替代）

| 项目 | 最新已知事实 | 来源 / 证据位置 | 尚未证明 |
|---|---|---|---|
| host 初步重建 | `cmake --build tools/ios-runtime/arm64/build/host --parallel 6`，会话 **6157 退出 0**；所有 host 目标成功，包括 CLI / API-tests。旧 47237 的三个 missing symbols 本轮未复现 | 主线最新确切工具结果；6157 对应完整落盘日志路径待补 | 不是最终冻结快照；不能覆盖此后源码变更，也不是测试运行通过 |
| Interpreter / Memory | 主线已实际把 `Interpreter::revalidate` 改为调用 `Memory::validate_code`，包含 page 同步与 epoch；磁盘已有调用体 | `src/cpu/aarch64/interpreter.cpp`、`src/memory/address_space.h`；主线反馈 | 最新改动尚待重建及代码失效回归；Memory seal token / 锁优化未冻结 |
| 完整基础 CTest | 最近一次真正完成的结果仍为 **4/8**；主线正诊断原四项失败 | `tools/ios-runtime/arm64/build/host/Testing/Temporary/LastTest.log`、`LastTestsFailed.log`；主线确认 | 修复后的统一完整重跑结果尚未提供 |
| Worker | 原始 guest **15 秒**条件未改；本轮旧 CLI 单 / 四核仍失败，当前测量见上节；下表保留历史记录 | `tools/ios-runtime/arm64/build/worker-performance-current/baseline.json`；历史 `workers-integrated-latest.json` | 两处同步源码优化未构建，尚无优化后通过或 wall 加速证据 |
| Worker 争用 | 本轮栈显示页级 `shared_mutex` 进入宿主 mutex 等待，以及全局访问门 / TLB refill 的 yield；历史 recursive_mutex 仅为此前记录 | `tools/ios-runtime/arm64/build/worker-performance-current/worker-4cpu.sample.txt`；历史 `workers-contention-sample.txt` | 已写 resident TLB 及轻量页锁源码；未构建，不声称提速 |
| proc 统计与生命周期 | 主线此前指出新旧统计路径并存、退出 Pending 清理不全；本次后续磁盘核对已见 `syscall_proc` 直接调用 controller、相关旧 registry API 查不到，并新增 `discard_pending_signals` 及退出调用 | `src/linux/syscall_proc.cpp`、`src/fs/virtual/procfs.*`、`src/runtime/process/{controller.h,observability.cpp,signals.cpp,lifecycle.cpp}` | 仅更新代码事实；Hilbert 最终接口冻结、同版本构建和退出信号额度回收运行仍待确认 |
| inotify | 核心实现与 syscall 已有；匿名 FD 分类现已包含 inotify。guest 文件操作事件桥接未落盘，Darwin OPEN / ACCESS / CLOSE 与虚拟节点不支持 | `src/linux/events/inotify.{h,cpp}`、`src/fs/descriptor_table.cpp` 的 `OpenDescription::is_anon_inode`、`src/linux/files/metadata.cpp`；主线反馈及源码核对 | 分类缺口有源码修复，尚无对应运行结果；新 guest 用例和 Node watcher 未执行，不能写成完整事件覆盖 |
| iOS 外部 root | 配置 UI、目录 bookmark、安全作用域、Rust / Dart 与原位 rootfs 源码接入已有 | 仓库 `apps/flutter/app/ios/Runner/{Arm64RootFilesystem,Arm64TerminalBridge}.mm`、`apps/flutter/app/lib/core/runtime/Arm64RuntimeConfigurationService.dart`、`apps/flutter/app/lib/ui/features/settings/runtime/Arm64RuntimeSettings.dart` 及 App Rust bridge；主线反馈 | 最新完整 Runner / 模拟器运行未完成；静态证据见下节 |
| 当前协作范围 | 仅保留 **Archimedes 文档、Hilbert 现有接线**两名子 agent，其余已关闭。主线负责构建与测试、CMake、Interpreter / Memory 接线及失败测试修复 | 用户最新授权 | 不再按历史任务槽数描述当前并发，也不以任务数量推算覆盖率 |

## 早期 8 项基础 CTest（历史）

以下保留早期运行的记录，分母是当时 **8 个基础 CTest 条目**。后来已有上方引用的 9/9 记录；本表不能用作当前失败清单或 24 组完成度。

| 条目 | 已完成结果 | 已知失败 / 后续状态 |
|---|---|---|
| `arm64-cpu-validation` | 通过 | 限该轮测试范围；新增 ISA 族与编码边界没有因此全部验收 |
| `arm64-filesystem` | 失败 | `SEAL_WRITE ignored an existing writable shared mapping`；修复与回归待主线结果 |
| `arm64-virtual-filesystem` | 失败 | `virtual shared status flags`；断言语义已修正，待重跑 |
| `arm64-network` | 失败 | accept 相关路径超时；已有修复反馈，待重跑 |
| `arm64-tasks` | 通过 | 不外推到新增 PI / 生命周期边界 |
| `arm64-runtime` | 通过 | 该轮日志 22 个内部用例通过，包含多核共享原子同步及真实 / CPU 定时器；不含后来新增的 inotify / resources |
| `arm64-terminal` | 失败 | `terminal foreground assignment requires a guest process-group session hook`；已有修复反馈，待重跑 |
| `arm64-alpine` | 通过 | 限该轮动态 Alpine 与固定助手范围 |

VFS 的修正保留断言：按 Linux v6.12 的 ARM64 定义检查 `O_LARGEFILE = 1 << 17` 不可变位，并检查 `dup` 共享的可变状态。`F_SETFL` 清除可变 flags 后仍须保留该不可变位。依据为上游 v6.12 的 `arch/arm64/include/uapi/asm/fcntl.h`、`fs/open.c` 与 `fs/fcntl.c`；测试入口为 `tests/virtual_filesystem.cpp`。这不是删除断言，也不是已经重跑通过。

## Worker 原始截止条件

证据为 `tools/ios-runtime/arm64/build/workers-integrated-latest.json`，`guest_deadline_ms=15000`。wall / CPU 均取宿主测量字段；平均宿主核为 host CPU / host wall。

| execution_threads | host wall 秒 | host CPU 秒 | 平均宿主核 | maximum_parallel_tasks | 结果 |
|---|---:|---:|---:|---:|---|
| 1 | 17.3143 | 11.5156 | 0.6651 | 1 | `returncode=1`，`passed=false`；guest 原始 15 秒超时 |
| 4 | 26.1383 | 63.0879 | 2.4136 | 4 | `returncode=1`，`passed=false`；guest 原始 15 秒超时 |

报告的 host `timed_out=false` 仅表示宿主 subprocess 没有触发宿主超时；stderr 的 `worker probe timed out` 与返回码仍表明 guest 失败。四核存在并行活动，但 wall 比单核更长，不能称为加速或通过。Memory 锁争用采样不能替代优化后的同条件测量。

## 已新增测试与静态证据

- 新增 `arm64-inotify`，通过真实 Syscalls 调用覆盖 data-open / dup 最后关闭、标量与向量读取（含 Linux 的 EOF 差异）、目录读取、sendfile、硬链接 no-op rename、跨目录移动及父目录重命名、覆盖仍打开的目标、unlink / IN_EXCL_UNLINK / 删除顺序、部分 munmap 与 one-shot。guest inotify fixture 新增 OPEN / ACCESS / 两种 CLOSE、dup / rename 与 O_PATH 场景；**这些用例尚未执行**。
- 新增桥接依据 Linux v6.12 的 `include/linux/fsnotify.h`、`fs/read_write.c`、`fs/readdir.c`、`fs/file_table.c` 核对事件触发位置；上游原文取至 `/tmp/operit-linux-{fsnotify.h,read_write.c,readdir.c,file_table.c}` 供本轮检查。源码解析使用临时目录中的 tree-sitter，未调用编译器；解析器对既有花括号默认参数的误报按 HEAD 对照记录，不等同 C++ 类型检查。

| 入口 | 实际断言 / 覆盖范围 | 已完成证据层级 | 尚待执行 |
|---|---|---|---|
| `tests/guest/inotify.c` → `tests/test_runtime.py` | 真实 create、成对 rename cookie、delete、read、拒绝 pread | 已落盘并接测试入口 | guest 未跑 |
| `tests/guest/resources.c` → `tests/test_runtime.py` | FSIZE / NPROC / SIGPENDING、timer 信号预分配、CPU hard limit 实际终止计算 | 已落盘并接测试入口 | guest 未跑 |
| `tests/node/watch.cjs` | 真实 `fs.watch` 事件、AbortController 关闭与清理 | 主线报告 macOS native Node 对照通过；完整日志路径待补 | guest 未跑 |
| iOS 相关 7 个 Dart 文件 | 配置 UI / service / bootstrap / session 等静态分析 | 主线报告 analyze 无 error、**3 lint**；文件清单 / 日志待补 | 最新 App 构建与运行没有因此通过 |
| `tools/tests/fixtures/arm64_simulator_smoke.mm` | 公开 `arm64RuntimeConfigure`；external root 原位、配置 / bookmark roundtrip、拒绝 live 切换、`closeExistingSessions` 回收、只读 bind 实写失败 | 仅 sim-arm64 Objective-C++ **整单元 `-fsyntax-only` 通过**；输出路径待补 | 最新完整链接与模拟器运行未完成，新增断言待执行 |

## 本轮最终统一证据区（待主线提供）

此区专供同一冻结时点的证据。6157 初步构建及上述历史测试不自动填入最终通过栏。

| 所需记录 | 当前状态 | 待补内容 |
|---|---|---|
| 代码 / 接口冻结范围 | 待确认 | 主线与 Hilbert 同一时点文件和接口状态；构建期间是否继续改动 |
| 最终 host 构建 | 待运行 / 待结果 | 冻结后的命令、退出码、CLI / API-tests 目标、完整日志路径 |
| 完整基础 CTest | 待统一结果 | 本轮实际条目 / 分母、原 8 项基础测试逐项结果、失败输出及代码范围；若启用 ISA 审计另列；新 fixtures 是否实际执行 |
| 新 guest / Node watcher | 待运行 | inotify / resources / watch 的 guest 命令、返回码与日志 |
| Worker 回归 | 待运行 | 原始 15 秒条件、单核 / 四核 host wall / CPU、返回码与 JSON |
| ISA 新增覆盖 | 待结果 | 新族尺寸 / lane / signedness / QC / 保留编码差分范围与报告 |
| iOS Runner / 模拟器 | 待运行 | 完整构建、公开 API 配置场景、会话回收及只读 bind 实际运行结果 |
| 真机 / 长会话 | 尚无本轮证据 | 设备范围、后台 / 恢复、目录授权失效与长会话运行记录 |

## 构建历史与被替代的状态

| 历史尝试 | 当时的结果 | 随后事实与解释 |
|---|---|---|
| procfs helper 冲突 | `process` helper 与 namespace 冲突，构建失败 | 已改名 `process_directory`；不把失败尝试改写为成功 |
| task 枚举缺口 | `lifecycle.cpp` 使用不存在的 `task_ids`，构建失败 | Darwin 已补声明与定义；之后 host 初步重建成功 |
| 源列表先于文件 | CMake 已列 `observability.cpp` 而文件未落盘，构建失败 | 文件随后落盘；不能继续说“文件缺失” |
| 会话 **47237** | **退出 2**。库和 cpu / tasks / network / vfs / filesystem 二进制链接；CLI / API-tests 缺 `Machine::poll_runtime_events()`、`Machine::next_runtime_deadline()`、`Machine::bind_runtime_signal_queues()` | 三个定义随后出现于 `src/runtime/process/timers.cpp` / `signals.cpp`；当时 CLI 不存在。该轮是改动中的混合版本 |
| 会话 **6157** | **退出 0，所有 host 目标成功，包括 CLI / API-tests** | 47237 的三个链接缺口本轮未复现；仍属接线中的初步重建，后续 Interpreter 改动待重建回归 |

历史协作快照曾为 16 个任务槽，其中 7 执行中、9 completed 占槽；随后要求仍写代码的 6 路保存并冻结。它们不是“16 路一直写代码”的证据，也不是当前两名保留子 agent 的状态。旧 **14/24 = 58.3%** 只属于旧基线，不能表示当前实现、接线或验收比例。

## 历史报告与证据解释

机器证据根目录为仓库 `tools/ios-runtime/arm64/build/`。可变报告文件可能被后续运行覆盖；没有适用代码范围的新报告，不能为当前源码背书。

| 历史证据 | 已记录范围 | 解释限制 |
|---|---|---|
| `host/isa-audit.json` | 5251 个历史样本通过，`profile_complete=false` | 不是完整 ISA，也不代表后来新增指令族通过 |
| `development-latest.json` | 历史 5/5 工作流 | npm 是本地 fixture 的 `npm install --offline`；Python 是 venv / SQLite / `pip --version`，不等于在线安装包 |
| `python-probe.json` | 历史 8/8 探针 | 不覆盖全部 Python 包 / 原生扩展 |
| `node-probe.json` | 历史基础、线程池、网络通过，Worker 失败，整体 `passed=false` | 最新 Worker 定量记录优先使用上文 integrated 报告 |
| `feature-probe.json` | 历史 PTY / signals 探针 | 不覆盖最新 terminal 失败与修复 |
| `simulator-smoke.log`、`ios-app-test.log` | 历史 bridge / App 探针，旧 Runner 有 cancellation-keep-session 失败记录 | 不能宣称最新完整 App、取消后复用或新版 harness 已通过 |
| [2026-11-02 审计](audit/2026-11-02.md) | 当时审计与原始证据 | 保留历史，不作为当前覆盖率 |

仍待主线补齐的关键事实是：冻结范围、最终 host 与完整 CTest 结果、proc direct controller / 旧 registry 收敛与退出 Pending 额度释放的统一回归、Memory / seal 优化后的回归，以及 guest watcher / 资源限制与新版模拟器完整运行。未知项保留“待确认”，不折算新的完成百分比。


## 2026-11-05 内存与计算开销复核（macOS CLI）

详见 [memory-compute-audit.md](memory-compute-audit.md)。导入后主线程采样主要在静态/C++ 指令解释执行（约 58%），其次内存执行（17%）、代码校验（12%）、调度记账等（9%）和专门浮点路径（3%）。仅最后一项不代表算法的全部数学成本。已消除浮点 scalar 写回约 154 万次动态长度通用拷贝；交替测量观察到约 2.6% 时间减少，有样本波动。整页快照和 FP 控制寄存器条件写原型未获益，已移除。

最终七项 macOS 回归通过，ISA 硬件差分 5253 个形式通过。CPU 限额测试原版在一秒前先耗尽指令预算，已将测试预算从 5 亿提高到 20 亿，不修改运行时 CPU 限额。冻结日志、采样、计数、计时与 CLI 位于 `tools/ios-runtime/arm64/build/memory-compute-audit-2026-11-05/`。本次没有 iOS 构建、模拟器或设备验证。


## 2026-11-05 已落地的解释器热路径修复（macOS CLI）

在上一轮审计版本上，将串行代码校验改为有界 64 字节快照与 revision 复用；允许当前 scope 已验证、且完成 COW 的私有文件页走静态 gadget 直接访存；单个普通计算任务保持连续 CPU 记账并跳过无用的并行 batch 选择。syscall、定时器、多任务、RT 策略和有限 CPU/RT 限额仍结算，文件截断、权限、自修改代码与并行校验仍检查。连续区间会包含此前部分未计的轮询/调度开销。

最终各预热一次、五轮交替计时，Python 循环中位数 1.298184 → 1.145850 秒，耗时减少 11.73%，吞吐约 1.133×；每次 checksum=1267865、退出 0、unsupported_syscalls=0。独立探针显示 thread_info 从 146,903 → 353，libc 可见代码比对从 11,712,538 → 873,265 次；固定宽度内联比对仍存在。最终九项回归与 5253 个 ISA 硬件差分形式通过。新增覆盖重叠块、槽冲突、五片段最大块、并行切换、私有文件 scope 验证/COW/截断。

详见 [内存与计算审计](memory-compute-audit.md#热路径修复后的复测)。冻结证据：`tools/ios-runtime/arm64/build/hotpath-fixes-2026-11-05/`。本轮仅 macOS Release CLI；未构建 iOS、未启动模拟器或设备。


## 2026-11-05 删除代码校验分片缓存（最新 macOS CLI）

删除此前引入的 64 字节快照、4096 槽缓存、每块五个 revision 和二次比对，恢复在页访问保护内单次比对原始代码块；每个解释器移除 352 KiB 快照，每块移除 40 字节元数据。ReadGuard 保存已取得的线程状态，解锁及 PageAccess 的 writer 判断复用它，并消除同次文件 generation 判断中的重复读取。此前私有文件页直访和单任务记账优化保留。

最终五轮交替计时 1.161388 → 1.063516 秒，耗时减少 8.43%，吞吐 1.092×；全部 checksum=1267865、退出 0、无 unsupported syscall。四秒主线程采样中，代码校验由旧快照版的 17.54% 降至 14.02%，不当作精确周期或绝对耗时降幅。

文件通知最小探针漏掉未 msync 的共享映射直接写入，因此未用于替代内容校验；新增测试覆盖该情形。共用构建目录 BUILD_TESTING=OFF 时的旧测试不作本轮证据，已在独立 macOS Release、arm64、deployment target 13.0、LTO OFF 目录重新构建全部请求目标，最终九项回归和 5253 个 ISA 差分形式通过。未构建 iOS 或启动模拟器/设备。

详见 [审计记录](memory-compute-audit.md#删除代码校验分片缓存)。本轮冻结目录：`tools/ios-runtime/arm64/build/code-validation-cleanup-2026-11-05/`。旧 64 字节快照方案和 17.54% 数据属于被替代的历史版本。
