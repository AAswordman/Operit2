# Node 探针方法与报告解释

本页说明可重复输入、验证条件和历史来源。**最新 guest / host 结果、Worker 精确计时及当前待跑项只记录在 [交付状态](delivery-status.md)**；本页列出的命令没有在本轮文档整理中执行。

## 输入与探针范围

历史报告使用仓库 `apps/flutter/app/android/app/src/main/assets/android-runtime/arm64-v8a/rootfs.tar.gz.bin`，记录的 archive SHA-256 为：

```text
141df6d9cb62073a9bb62225c54032e88e43020f2d66bcfb1d271ffc9bbd67fa
```

该历史 root 包含 Alpine aarch64 与 Node `v24.18.1`。工具将其制备到独立 `build/node-rootfs`；输入来源与架构写入报告 provenance。此哈希描述历史输入，本轮未重新提取或运行；外部 root 的一般运行能力不要求工具标记，但本探针脚本要求 build-owned root 标记以约束其输入。

`tests/probe_node.py` 执行真实 guest Node，JS 使用 `--jitless`，关闭 / 开启固定助手两档分别运行。当前脚本默认六类共十二个 guest 变体，启用 external-network 后为七类共十四个；**这是源码中的计划集合，不是已运行数量**。

| 用例 | 行为断言 | 解释边界 |
|---|---|---|
| version | 动态装载、Node 版本输出 | 不能代表进入完整 JS 工作流 |
| basic | JS 执行、Linux / arm64 身份 | 不能代表多线程 / 网络 |
| workers | 四 Worker、40000 次共享计数、Atomics.wait/notify、threadId 和正常退出 | 原始 guest 15000ms 截止为独立门槛 |
| threadpool | 八个异步 PBKDF2、与同步结果一致、回调完成 | 不证明全部 libuv 或 CPU 加速 |
| network | 实际 loopback TCP / UDP 与事件循环 | 不代表公网 DNS / TLS |
| watch | 实际 fs.watch 事件、AbortController 关闭与清理 | host 与 guest 结果分别保存 |
| network-external（可选） | DNS、证书验证、TLS 与 HTTP 响应 | 依赖外网；不得由本地网络结果替代 |

`validated()` 要求进程返回成功、无宿主超时且相应 JSON 行满足断言；Worker 计数、线程池完成数、watch events / closed 等另作检查。整体结果由实际运行变体共同决定，不因其中网络或启动成功就转为通过。

## Worker 时间、并行与诊断

`tests/node/workers.cjs` 保留原始 15 秒 guest 定时器。另有宿主 subprocess timeout / CLI step budget，三者须分开读：host `timed_out=false` 不代表 guest 定时器没有失败；`ort_arm64_result.status` 表示 guest 已退出也不等于 guest exit code 为 0。

`workers-integrated-latest.json` 记录 cpus=1 / 4、host wall、host CPU、平均宿主核和 maximum_parallel_tasks，精确结果见交付状态。内部已有并行 executor，不能沿用旧“始终单宿主线程”的结论；观察到多个任务执行也不能推出 wall 加速。

延长截止条件的诊断只能说明改变条件后的功能行为，不能替代原脚本验收。Memory mutex 采样是性能定位信息，优化结论仍需相同输入和截止条件的重新测量。

## 历史来源与新增入口

历史 `tools/ios-runtime/arm64/build/node-probe.json` 包含 version / basic / workers / threadpool / network / network-external 六类各两档，共十二个 guest 记录；**不含后来新增的 watch**。这与当前默认六类集合不同，不能根据记录数量相同推断 watch 已跑。

该报告的宿主对照记录 Node `v24.21.0`，包含 workers / threadpool / network；宿主版本与 guest 包版本不同，对照用于判断脚本及 host 条件，不能替代 guest。新增 watch 的 native 对照、guest 是否运行及日志来源集中到交付状态。

旧 FIONBIO 问题已有显式 Linux ioctl → 宿主非阻塞状态转换，dup / F_SETFL 共享打开描述状态、管道 / eventfd 等待、信号投递、robust owner-death 与多核执行也已有生产实现。应用脚本只覆盖实际经过的行为，不能据此证明全部线程 / 信号 / 事件 ABI。CPU fixed-point / SIMD 等指令审计独立于 Node，见 [功能矩阵](completeness.md) 与 [历史审计](audit/2026-11-02.md)。

## 复现方法

在仓库根目录执行，报告可变文件会被新运行覆盖；正式验收应另保存对应版本和运行日志：

```sh
python3 tools/ios-runtime/arm64/prepare_rootfs.py \
  --archive apps/flutter/app/android/app/src/main/assets/android-runtime/arm64-v8a/rootfs.tar.gz.bin \
  --sha256 141df6d9cb62073a9bb62225c54032e88e43020f2d66bcfb1d271ffc9bbd67fa \
  --output tools/ios-runtime/arm64/build/node-rootfs

python3 hosts/ios/linux-runtime/tests/probe_node.py \
  --runtime tools/ios-runtime/arm64/build/host/operit-arm64 \
  --root tools/ios-runtime/arm64/build/node-rootfs \
  --output tools/ios-runtime/arm64/build/node-probe.json
```

可附加 `--external-network` 或 `--host-node /absolute/path/to/node`。JS 文件经只读绑定提供，不复制到 root。实际返回码由本次执行决定，不将历史失败写成固定预期，也不将旧通过当作当前回归结果。

## Worker 性能复测（不编译）

`tests/profile_workers.py` 只调用已有 CLI，不运行 guest fixture 编译入口。默认保留原脚本四 Worker / 40000 次计数 / 15000ms 截止，分别测 cpus=1、4，记录二进制和脚本 hash、wall / CPU / stats。`--phases` 增加独立临时副本，并用宿主单调时钟记录阶段日志到达时间；`--sample-seconds 8` 另开一次 macOS 采样。这些诊断不进入原脚本验收结果。

当前性能目标是 **5 秒总 wall**：从新 CLI 进程启动到完整原脚本退出，包含 Node / 四 Worker 初始化。默认 `--repeats 3 --target-seconds 5`，性能目标适用于 `--cpus` 中最大值（默认四核）；可用 `--target-cpus` 显式指定。各轮交替单 / 四核顺序，每次启动独立 CLI，不提前预热 guest；不保证宿主文件页缓存为空。三次功能均通过且三次总 wall 都不超过 5 秒才算性能通过，不能只取最快一次或中位数。报告分别给出 `functional_passed` 与 `performance.passed`，总 `passed` 必须两者满足；单核仍须功能通过，默认不要求单核也达 5 秒。`arm64-worker-profile-validation` 只校验这些判定规则，不能证明引擎已达标。

在仓库根目录、用户已完成构建后运行：

```sh
python3 hosts/ios/linux-runtime/tests/profile_workers.py \
  --runtime tools/ios-runtime/arm64/build/host/operit-arm64 \
  --root tools/ios-runtime/arm64/build/node-npm-check-kxc_6965/root \
  --output tools/ios-runtime/arm64/build/worker-performance-after.json \
  --host-node /Users/ylvira/.local/bin/node \
  --repeats 3 --target-cpus 4 --target-seconds 5 \
  --phases --sample-seconds 8
```

新 C++ 的 resident TLB 前进性、prepared 助手页集 / 失败回退、COW / 写保护及并发页访问回归登记为 `arm64-memory-concurrency`；固定助手 harness 另覆盖热复制前进性、反向并发复制与跨页回退；本轮没有构建或运行该目标。测量及当前源码的精确边界见交付状态本轮性能排查一节。
