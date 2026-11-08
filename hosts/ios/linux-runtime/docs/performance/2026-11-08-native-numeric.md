# 2026-11-08：原生数值指令回退优化

本轮保留的改动让原始 Python 数学工作段中位数从 **323.311 降到 304.111 ms（5.94%）**，`--jitless` Node 同类工作段从 **301.097 降到 279.693 ms（7.11%）**。完整进程分别改善 3.22% / 1.74%。JSON、整数循环和启动没有证明稳定收益，50 ms 目标仍未达到。

这是 Apple M5 macOS 的自身版本对照，不是 iPhone 成绩，也没有重建第三方运行时比较。不能由此宣称已经超过第三方或消除此前的整体差距。

## 最终保留的实现

在既有静态汇编执行器中增加以下有界处理器，不产生运行时机器码：

- `SCVTF`：有符号 32/64 位 GP 转 double；`UCVTF` 补充 32 位源，并让驻留 GP/FP 直接相接，减少临时搬运。
- `FCVTZS`：double 转有符号 32/64 位 GP，保留饱和、异常状态和 W 写入清零高半部的语义。
- `FCMP` 的零比较、`FCMPE` 的寄存器/零比较，保留 NZCV 和 FPSR。
- `SMULL/UMULL` 及 `SMADDL/UMADDL/SMSUBL/UMSUBL`。源操作数用 W，结果与加数用 X；零加数直接执行，其他加数复用既有 Operand 暂存。

同步补齐解码、GP/向量读写效果、静态降低和诊断名称。保留原先的浮点环境切换、访存权限/代码失效检查、跨图校验和退休预算。运行时仍装载原有客体 Python/Node；没有替换数学任务或启用固定函数助手。

CLI 从 7,800,176 增到 **7,982,656 字节（增加 182,480 字节，约 178 KiB）**。没有保留此前 44 MB 的扩大 AOT 镜像作为本轮最终版。

## 相同输入的七对测量

Release / LTO，单 CPU，4,096 条小 AOT 配置及相同 rootfs。Node 全部带 `--jitless`；Wasm、诊断、固定函数替换关闭。每项先预热，再交替运行七对，预热行不计入中位数；原始 stdout/stderr、checksum、命令和哈希均保留。

下表单位为 ms；“工作段”取客体内部计时，“完整进程”包含宿主启动命令、客体初始化、工作和退出：

| 负载 | 对照工作段 | 最终工作段 | 对照完整进程 | 最终完整进程 |
|---|---:|---:|---:|---:|
| Python 数学 | 323.311 | **304.111** | 563.322 | **545.178** |
| Python JSON | 283.025 | 284.684 | 681.645 | 679.572 |
| Node 整数 | 35.786 | 35.968 | 839.070 | 835.962 |
| Node 数学 | 301.097 | **279.693** | 1106.895 | **1087.612** |
| Node JSON | 128.105 | 128.032 | 1068.695 | 1073.149 |
| Node 空启动 | — | — | 820.342 | 805.841 |

Python 数学七对均变快，Node 数学六对变快。逐对降幅的中位数分别为 **4.80% / 6.60%**；这与“两个样本中位数的降幅”是不同统计量，不把前者省略后当成所有样本的统一收益。JSON 和整数段的变化较小，不作为正收益宣传；Node 整数原本已经低于 50 ms。

`/usr/bin/time -l` 记录的**完整进程**宿主退休指令中位数：Python 7,441,806,296 → 7,110,695,050，Node 数学 13,980,688,978 → 13,463,428,480。相应 cycles 中位数 2,416,574,506 → 2,324,865,618，以及 4,434,487,692 → 4,332,877,040。它们用于佐证指令开销减少，不能当作数学段本身的精确硬件归因。

这些都是单台机器的有界样本。不同轮的宿主负载和绝对时间有变化，不跨轮相减来扩大某项改动的收益。

## 未保留的实验

以下候选的源码、冻结二进制和原始测量留在证据目录，生产源码已撤下；没有保留新的开关、缓存字段或更大的 Bridge/PIC/ExecutionFrame。

| 实验 | 对照条件与结果 | 决定 |
|---|---|---|
| 最近间接目标缓存 | 早期首轮噪声大，组合复测未形成可靠收益 | 删除 |
| Bridge 校验在原生区间复用 | 在最终数值优化上单独测七对：Python 数学 295.455 → 321.315，Node 数学 275.854 → 278.365 | 删除 |
| 浮点序列离线 AOT | 同数值优化七对：Python 数学 300.921 → 302.460，Node 数学 280.167 → 280.090；Python JSON 285.014 → 289.638 | 删除 |
| 串行块入口版本检查复用 | 同数值优化七对：Python 数学 292.360 → 294.569，Node 数学 272.582 → 275.666；退休指令略少但耗时未改善 | 删除 |

早期 Bridge 组合在负载波动很大的一轮中曾出现较好中位数，较稳定复测不支持该结论，不能引用它来声称约 27% 改善。浮点 AOT 虽通过强制入口、预算/故障/FP 状态差分测试，性能仍不支持保留。理论上少检查或少分派并不自动等于更快。

## 验证与适用边界

最终 fixture 构建的完整 CTest **46/46** 通过，包含 **37 项 AOT 生成器测试、5,253 项 ISA 形式审计**。运行时显式设置 `OPERIT_TEST_REQUIRE_GUEST_AOT=1`、`OPERIT_TEST_REQUIRE_PROFILE_SUPERBLOCKS=1`、`OPERIT_TEST_REQUIRE_COMBINED_AOT=1`，要求既有片段和组合入口命中。

新增原生数值差分回归要求静态处理器确实选中，比较完整 CPU 状态和退休数，覆盖驻留/溢出/零寄存器、W 高半部、寄存器重叠、带符号乘法极值与加减数、浮点截断/饱和、正负零、无穷、quiet/signaling NaN、FPCR 舍入及 DN/FZ、sticky FPSR、NZCV、串行/并行模式和每条预算前缀。保留间接缓存替换、代码修改、文件截断、权限及桥接回收回归。

最终源码已通过 **iOS device arm64 静态库交叉编译**。此处采用 fixture AOT 配置；未重新打包生产 XCFramework、链接完整 iOS App、安装设备或测量 iPhone，也不替代共享工作区其他变更的独立验证。

## 证据与复现

证据根目录为仓库根目录下 `tools/ios-runtime/arm64/build/native-optimization-2026-11-08/`：

- `numeric-round1/`：最终正收益测量，含 `environment.json`、`measurements.jsonl`、`summary.json`。
- `candidate-final` 与 `candidate-numeric`：SHA-256 均为 `1aa41808cab3b7f33ee9f9c5306d7ace7bf976f78499fead1f4cc14f577f8cef`。清理实验后重新构建的最终 CLI 与已测候选逐字节相同。
- `final-provenance.json`、`final-source/`、`final-*-CMakeCache.txt`：最终源码哈希、快照、配对统计和实际配置。
- `final-*-build.log`、`final-fixture-test.log`、`final-fixture-last-test.log`：完整构建/回归输出。
- `recent-round1/`、`interval-round1/`、`fp-round1/`：早期含噪声的实验，原样保留。
- `numeric-interval-round1/`、`fp-aot-round1/`、`block-epoch-round1/`：单独增量对照；`rejected-*-source/` 保存撤下的实验实现。

复测时停止本任务的编译/测试，使用全新输出目录：

```sh
python3 hosts/ios/linux-runtime/tools/benchmark_library_aot.py \
  --control tools/ios-runtime/arm64/build/library-aot-2026-11-08/control-4096-round2 \
  --candidate tools/ios-runtime/arm64/build/native-optimization-2026-11-08/candidate-final \
  --root tools/ios-runtime/arm64/build/node-rootfs \
  --output tools/ios-runtime/arm64/build/native-optimization-2026-11-08/reproduce-final \
  --repeats 7 --host-counters
```
