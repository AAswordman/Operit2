# 2026-11-08：Python / Node 扩大离线 AOT 实验

目标是检查大量构建期 AOT 能否将 Python / Node 工作段或完整进程压到 **50 ms**。两轮均未达到：第二轮 Python 数学循环 315.439 ms，Node 数学循环 275.513 ms，完整进程分别为 563.543 / 1088.305 ms。Node 简单整数循环虽为 33.525 ms，同轮小 AOT 对照原本已为 37.114 ms，不能算新达到的目标。

## 实现与配置

本轮在已有实验 AOT 上新增独立训练集的合并：`--profile` 可重复，CMake 通过 `OPERIT_ARM64_GUEST_AOT_EXTRA_PROFILES` 传入额外文件。相同指令序列累计权重，缓存解码和可编译性判断；跨块边在每份录制内推断后再合并，避免不同程序的相同客体地址串成轨迹。

第二轮还新增 32/64 位 UDIV、SDIV、LSLV、LSRV、ASRV、RORV，以及 CSEL、CSINC、CSINV、CSNEG 的构建期翻译。将原始指令的寄存器字段映射到静态执行器的宿主寄存器；条件选择先恢复客体 NZCV。片段表由首指令索引改为前两条指令的只读哈希索引，保留原始编码、静态操作元数据、代码失效和状态检查。

大配置从 Python / Node 独立训练集与 libpython、Node、musl 三个客体 ELF 提取整数片段，最多 32 条。第二轮为 **65,536 条序列、6,476 条轨迹，10,327,398 个序列候选**；对照为同一源码重新构建的 **4,096 条序列、449 条轨迹**，只使用原 Python 训练集。两者都启用直接链接、延迟调用发现、整数访存和分支片段；SSA / 内存重排、Wasm、诊断关闭，Release / LTO 开启。

这仍是受支持 ARM64 片段的构建期翻译，未实现完整 ELF 的所有指令、控制流和重定位的离线编译，也没有把 Python / JS 脚本本身编译为宿主机器码。代码由宿主构建工具提前生成并链接；运行时匹配已编入镜像的入口。Node 使用 `--jitless`，固定函数替换关闭。

第二轮 CLI 二进制从 **7,800,176 字节增加到 44,315,872 字节**，约 5.68 倍。该大配置保持实验用途，尚无足够证据作为默认发布配置。

## 测量方法

全部数字来自 Apple M5 / macOS，**不是 iPhone 实测**。使用同一仓库 `node-rootfs`、单客体 CPU、清空后固定的客体环境。每个负载先预热一对，再交替启动五对全新进程；预热不跨进程保留图缓存。工作段由客体自行计时，完整进程由宿主计时，包括 spawn、客体初始化、工作与退出。所有结果取中位数，不作为尾延迟或实时保证。

Python 数学使用未修改的 `benchmarks/python_loop.py`，checksum 为 1267865；Node 数学是同一 100×100 算法，checksum 同为 1267865。整数循环为 100,000 次累加，checksum 为 704982704。另测 1,500 次 JSON / 正则 / 哈希操作，以及只输出一行的 Node 空启动。Python JSON / Node 数学 / Node JSON 不是采集 profile 所用的脚本，但标准库和运行时路径可能重叠，不能视为完全独立的运行时覆盖。

全部正式测量返回零状态、零 unsupported syscall、零 accelerated call，工作负载校验一致。诊断录制单独进行，插桩时间不计入性能表。Node 训练保留 120,553 个站点，但导出的 `top_sites` 仅 65,536 项；不是全覆盖 profile。事件计数不能换算成时间占比或准确的客体动态覆盖率。

## 第二轮同源码对照

单位为 ms；小 / 大配置均含本轮新增指令和两字索引，所以此表衡量扩大训练与片段集合的组合效果，不能归因于单独某项改动。

| 负载 | 小 AOT 工作段 | 大 AOT 工作段 | 工作段耗时下降 | 小 AOT 完整进程 | 大 AOT 完整进程 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Python 数学 | 326.202 | 315.439 | 3.30% | 566.452 | 563.543 |
| Python JSON | 284.513 | 250.506 | 11.95% | 683.989 | 651.236 |
| Node 整数 | 37.114 | 33.525 | 9.67% | 843.610 | 841.265 |
| Node 数学 | 305.777 | 275.513 | 9.90% | 1116.528 | 1088.305 |
| Node JSON | 129.579 | 127.543 | 1.57% | 1089.514 | 1061.446 |
| Node 空启动（波动组） | — | — | — | 1418.865 | 1487.233 |

工作段大配置胜出配对数依次为 3/5、5/5、5/5、4/5、4/5，完整进程为 2/5、5/5、4/5、5/5、4/5。Python 数学和 Node JSON 的收益较小，应保留噪声和样本数量限制；不能据此推广为所有 Python / Node 程序都加速。

最后的空启动组小配置范围 843.6–3815.9 ms，大配置 1010.6–3185.4 ms，波动明显。测后观察到较高的后台 Dart / Spotlight 负载，但未证明它是全部波动原因，未暂停用户的其他进程。原始记录完整保留，并用相同冻结二进制单独复测七对：

- 小配置空启动中位数 **819.472 ms**。
- 大配置空启动中位数 **823.643 ms**。

复测结果同样远高于 50 ms。该复测不替换原表，也不用于证明启动有稳定改善或回退。

## 第一轮失败记录

第一轮已经扩大到 65,536 条序列、6,382 条轨迹，使用单字匹配索引，尚无本轮新整数指令。对照为此前冻结的 4,096 小镜像。五对中位数如下，不能把跨轮数据相减当作某一改动的因果收益。

| 负载 | 小 AOT 工作段 | 大 AOT 工作段 | 小 AOT 完整进程 | 大 AOT 完整进程 |
| --- | ---: | ---: | ---: | ---: |
| Python 数学 | 321.251 | 315.338 | 566.854 | 603.893 |
| Python JSON | 285.696 | 254.470 | 689.971 | 708.611 |
| Node 整数 | 36.074 | 33.572 | 867.799 | 1059.636 |
| Node 数学 | 317.597 | 287.019 | 1156.837 | 1300.524 |
| Node JSON | 131.188 | 141.768 | 1102.746 | 1262.447 |
| Node 空启动 | — | — | 834.322 | 999.695 |

第一轮大配置所有完整进程中位数均回退，不能只选择工作段的正收益发布。第二轮扩大指令覆盖并改进索引后，结果仍不足以支持 50 ms。

## 验证与边界

本轮 fixture 配置的完整 CTest **46/46** 通过，生成器 **37/37** 通过。运行 CTest 时启用 `OPERIT_TEST_REQUIRE_GUEST_AOT=1`、`OPERIT_TEST_REQUIRE_PROFILE_SUPERBLOCKS=1`、`OPERIT_TEST_REQUIRE_COMBINED_AOT=1`，显式要求既有片段 / 轨迹和组合入口命中。

新增整数差分测试覆盖两种宽度、全部新操作、条件选择的多个条件编码及全部 16 种 NZCV、除零、带符号极值 / -1、移位数 0 / 31 / 32 / 63 / 64 / 65 / 极值、0–3 指令预算、串行 / 并行内存模式和修改源代码后的缓存失效。每次对照单指令语义检查架构状态和退休数，并要求两指令 AOT 入口确实安装，避免仅测到回退执行。

最新生成器和 fixture 的 iOS device arm64 静态库交叉编译成功。该验证使用测试片段集合，**未用 65,536 条大配置构建完整 iOS App、重新打包生产 XCFramework、安装 iPhone 或验证分发**；不能将 macOS 结果作为移动端性能承诺。本轮测试针对上述冻结输入和配置，也不替代共享工作区其他并行变更的独立验收。

从实现和结果看，扩大数量只能消除命中片段内部的一部分操作分派。当前仍保留 Python / Node 自身解释器、未翻译的 ARM64 指令、片段边界状态同步、受检访存，以及运行时构图和客体启动。它们在本轮没有逐项时间归因，不能给出未经测量的百分比。仅 Node 空启动已约 824 ms；Python / Node 数学工作段还分别需要约 6.31 / 5.51 倍吞吐才能达到 50 ms。更完整的运行时离线翻译、减少跨片段边界和初始化成本，需要新的实现与实测，当前不能宣称只增加 AOT 数量即可达标。

## 证据与复现

所有自身实验数据在仓库根目录下 `tools/ios-runtime/arm64/build/library-aot-2026-11-08/`：

- `benchmark-round1/`：第一轮完整原始 stdout / stderr、命令、环境和 summary。
- `benchmark-round2/`：第二轮五对完整数据，逐条含解析后的工作段、checksum 和 stats。
- `benchmark-round2-startup-recheck/`：空启动波动后的独立七对复测。
- `round2-provenance.json`：冻结二进制、构建源码和 profile SHA-256、数量与测试范围。
- `round2-configure-command.json`、`round2-control-configure-command.json`：完整 CMake 参数数组；对应 `*-CMakeCache.txt` 保存实际配置。
- `round2-build.log`、`round2-control-build.log`、`round2-tests.log`、`round2-ios-build.log`：生成数量、构建与测试日志。
- `round2-benchmark-tool.py`、`startup-recheck-benchmark-tool.py`：两次测量所用的工具快照；冻结二进制为 `control-4096-round2`、`candidate-65536-round2`。

在仓库根目录、停止本任务构建与测试后复现五对（输出目录必须全新）：

```sh
python3 hosts/ios/linux-runtime/tools/benchmark_library_aot.py \
  --control tools/ios-runtime/arm64/build/library-aot-2026-11-08/control-4096-round2 \
  --candidate tools/ios-runtime/arm64/build/library-aot-2026-11-08/candidate-65536-round2 \
  --root tools/ios-runtime/arm64/build/node-rootfs \
  --output tools/ios-runtime/arm64/build/library-aot-2026-11-08/reproduce-round2 \
  --repeats 5
```

只复测某项可加 `--workload node-startup` 等参数；多个 `--workload` 可重复。
