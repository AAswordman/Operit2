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
