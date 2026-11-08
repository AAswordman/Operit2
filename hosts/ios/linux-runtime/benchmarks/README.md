# 执行器基准

当前组件目标为 `operit-arm64-execution-bench`，源文件 `execution.cpp`。先预热，再测量 3000 万条客体指令的真实整数回边，输出时间、吞吐量、checksum、编译块数、慢路径次数和状态发布次数。没有复制循环体；结果必须为 `x0=10000000`、`x1=20000000`。

```sh
cmake -S hosts/ios/linux-runtime -B /tmp/operit-executor-release -DCMAKE_BUILD_TYPE=Release
cmake --build /tmp/operit-executor-release --parallel 8
/tmp/operit-executor-release/operit-arm64-execution-bench
```

`python_loop.py` 保持原始 100×100 数学负载，checksum 为 `1267865`。通过 CLI 从外部 Linux root 启动 `/usr/bin/python3 -B` 并执行原始文件内容；使用相同 rootfs、预算、CPU 数及可选固定函数替换设置，对比冻结的旧 CLI 和新 CLI。构建、采样与计时不得并行。报告循环内部时间和完整进程时间；微基准不代表应用加速。

旧 `region_parts.cpp`、`region_edges.cpp`、`state_inputs.cpp`、`memory_kernels.cpp`、`vector_memory.cpp`、`code_pages.cpp` 及其 targets 已删除，它们依赖已移除的 `Kernel`/`bind_*` ABI。原先的性能报告保留为历史材料，不能代表新执行器。当前结构和验收边界见 [静态汇编执行器](../docs/static-execution.md)，最新测试与配对测量见 [交付状态](../docs/delivery-status.md)。

配对工具直接使用冻结 CLI：

```sh
python3 hosts/ios/linux-runtime/tools/profile_execution.py \
  --before /tmp/before-executor --after /tmp/after-executor \
  --root /path/to/rootfs --output /tmp/python-paired \
  --repeats 5 --warmup 1
```

工具固定核验原始源文件 SHA256、checksum、零固定函数替换与零 unsupported syscall，交替执行顺序，保存逐次输出、二进制 hash、循环/进程中位数、每对节省和胜出次数。macOS 的宿主指令/周期来自整个进程，不能写成仅循环的计数。可加 `--sample 3` 在验收计时结束后另行采样；采样窗口由原负载的两个单调时钟系统调用界定，不修改 Python 文件。诊断构建也必须与生产验收分开运行。


## 全局值编号 / 加载调度微基准

`operit-arm64-global-opt-bench cse` 测跨块重复整数表达式；`operit-arm64-global-opt-bench loads` 测可合并的独立加载。每次先预热，再运行 3 亿条客体指令，核对完整 GP 结果、PC、NZCV 与原始语义，并输出 JSON 时间、退休数、慢路径数和 checksum。它们是刻意匹配优化机会的组件测试，不能替代原样 Python 或真实终端负载。

```sh
cmake -S hosts/ios/linux-runtime -B /tmp/operit-global-values \
  -DCMAKE_BUILD_TYPE=Release -DOPERIT_ARM64_GLOBAL_VALUES=ON \
  -DOPERIT_ARM64_MEMORY_SCHEDULING=ON
cmake --build /tmp/operit-global-values --target operit-arm64-global-opt-bench
/tmp/operit-global-values/operit-arm64-global-opt-bench cse
/tmp/operit-global-values/operit-arm64-global-opt-bench loads
```

对照版本将两个开关均设为 OFF，其余构建配置相同；同一轮交替顺序、保存所有原始输出，不能把不同负载的客体指令数直接当作同一工作量。实验的 Python 同场对照、初版退步与最终默认配置见 [实验记录](../docs/performance/2026-11-08-global-values.md)。
