# ARM64 runtime tooling

运行时实现归 `hosts/ios/linux-runtime/`。这里仅负责构建、下载、输入校验和可重建产物；不保存 CPU 或 Linux 兼容层实现。

| 路径 | 职责 |
| --- | --- |
| `build.sh` | host、iOS device、iOS Simulator、XCFramework |
| `rootfs.json` | 固定原版 Alpine AArch64 输入和 SHA-256 |
| `prepare_rootfs.py` | 校验、受限提取、架构检查、原子发布测试 root |
| `build_rootfs_linux.sh` | 在 Linux 上安装完整 Alpine AArch64 开发工具与 Node 24 |
| `package_rootfs.py` | 验证并发布 Runner 的 ARM64 根目录归档和摘要 |
| `downloads/` | 忽略提交的原始下载缓存 |
| `build/host/` | 宿主库、CLI、测试程序 |
| `build/ios/`, `build/ios-simulator/` | SDK 对应的 arm64 静态库 |
| `build/OperitArm64.xcframework/` | 可供宿主接入的库与公开头文件 |
| `build/rootfs/` | 带来源标记的测试 root，不是产品用户数据 |

从仓库根目录运行：

```sh
bash tools/ios-runtime/arm64/build.sh host
python3 tools/ios-runtime/arm64/prepare_rootfs.py
python3 -m unittest discover -s tools/tests -p test_arm64_rootfs.py -v
cmake -S hosts/ios/linux-runtime -B tools/ios-runtime/arm64/build/host \
  -DOPERIT_ARM64_ROOTFS="$PWD/tools/ios-runtime/arm64/build/rootfs"
cmake --build tools/ios-runtime/arm64/build/host --parallel
ctest --test-dir tools/ios-runtime/arm64/build/host --output-on-failure
bash tools/ios-runtime/arm64/build.sh xcframework
```

默认 root 固定为 Alpine 3.23.6 aarch64。传入自有归档时必须提供摘要：

```sh
python3 tools/ios-runtime/arm64/prepare_rootfs.py \
  --archive /absolute/path/root.tar.gz --sha256 EXPECTED_SHA256 \
  --output tools/ios-runtime/arm64/build/another-root
```

提取拒绝越界路径、重复路径、特殊设备节点、危险硬链接和归档 symlink 父路径；保留 Linux 根目录语义的绝对 symlink。已有来源不匹配的目录或遗留 `.importing` 目录不会自动清除。该脚本不重建安装后的系统清单，也不将 root 中每个文件视为不可变；完整摘要用于识别导入来源。

检查宿主内存/未定义行为与运行可重复测量：

```sh
cmake -S hosts/ios/linux-runtime -B tools/ios-runtime/arm64/build/sanitizers \
  -DCMAKE_BUILD_TYPE=Debug -DOPERIT_ARM64_SANITIZERS=ON \
  -DOPERIT_ARM64_ROOTFS="$PWD/tools/ios-runtime/arm64/build/rootfs"
cmake --build tools/ios-runtime/arm64/build/sanitizers --parallel
ctest --test-dir tools/ios-runtime/arm64/build/sanitizers --output-on-failure
python3 hosts/ios/linux-runtime/benchmarks/run.py \
  --runtime tools/ios-runtime/arm64/build/host/operit-arm64 \
  --root tools/ios-runtime/arm64/build/rootfs \
  --repeats 7 --warmup 2 --output tools/ios-runtime/arm64/build/benchmark.json
```

可用 `OPERIT_AARCH64_CLANG`、`OPERIT_AARCH64_LLD` 指定交叉汇编/链接工具。不要把 `ld.lld` 的调用路径解析成通用 `lld`：LLVM 依赖驱动文件名选择链接模式。

## 可选客体 AOT 排序

客体 AOT 默认关闭。`OPERIT_ARM64_GUEST_AOT_DIVERSE` 可选地将已经选中的较长模式所包含的短模式延后，优先为其他模式留出名额；这只是启发式，短模式仍可能在不同的块边界或优化结果下单独命中，必须用目标工作负载验证。构建生成并签名的处理器不依赖运行时 JIT；运行时仍核对客体指令、操作边界及代码有效性，不匹配时回退。

另一项默认关闭的实验 `OPERIT_ARM64_GUEST_AOT_COVERAGE` 根据训练块中尚未覆盖的指令边及其样本权重选择模式；模式在同一块中重复出现时按边去重。它仅支持 profile 输入，不能与 `DIVERSE` 或 ELF 输入同时使用。离线边覆盖不等同于运行时命中率：优化、块边界、匹配顺序和代码有效性检查都会影响实际结果，因此需要分别验证性能与正确性。App 构建脚本仍使用上面的 `DIVERSE` 配置，不会自动启用此选项。

一次 Apple M5 macOS 实验中，同一 Python/Node 混合 profile、16384 项、不启用轨迹，预热后交替比较两轮，每轮九对：Node JSON 完整进程中位数从 721.5/720.2 ms 降至 711.7/712.6 ms；Python 数学分别为 481.2/482.7 ms 和 474.1/483.7 ms，未表现出一致改善。Node 使用 `--jitless`，每次运行均检查结果和运行时状态。两份二进制大小分别约 13.03/12.83 MiB。原始样本和输入哈希保存在本机临时目录 `/tmp/operit-aot-coverage-bench-20261109/` 和 `/tmp/operit-aot-coverage-confirm-20261109/`，不属于仓库；这些实验不代表 iPhone 性能，也未达到 100 ms 目标。

例如，在仓库根目录中可以把两个独立执行诊断 profile 压缩为序列 AOT 输入，然后用于实验构建：

```sh
python3 hosts/ios/linux-runtime/tools/compact_guest_aot_profile.py \
  --profile /path/to/python-profile.json --profile /path/to/node-profile.json \
  --output /tmp/operit-sequence-profile.json
bash tools/ios-runtime/arm64/build.sh ios \
  -DOPERIT_ARM64_GUEST_AOT_PROFILE=/tmp/operit-sequence-profile.json \
  -DOPERIT_ARM64_GUEST_AOT_LIMIT=16384 -DOPERIT_ARM64_GUEST_AOT_MAX_LENGTH=32 \
  -DOPERIT_ARM64_GUEST_AOT_SCORE_LENGTH=ON -DOPERIT_ARM64_GUEST_AOT_DIVERSE=ON \
  -DOPERIT_ARM64_GUEST_AOT_BRANCHES=ON -DOPERIT_ARM64_GUEST_AOT_INDIRECT=ON \
  -DOPERIT_ARM64_GUEST_AOT_FOLDED=ON -DOPERIT_ARM64_GUEST_AOT_INLINE_MEMORY=ON \
  -DOPERIT_ARM64_GUEST_AOT_REGISTER_MEMORY=ON
```

压缩结果记录输入的 SHA-256，但不包含控制流轨迹所需的地址和边，不能作为轨迹训练数据。训练样本、代码体积、内存消耗和未训练程序均会影响结果；实验选项不会自动接入 App 默认构建，也没有代替真机验证。

要在 iOS App 的构建脚本中显式启用上述 16384 项配置，可将 `OPERIT_ARM64_GUEST_AOT_PROFILE` 设为压缩 profile 的绝对路径，并确保它传入 Xcode 的构建环境。未设置或设为空时，脚本会清除该构建目录中缓存的客体 AOT 输入，防止上一次实验意外留在后续 App 构建中。请使用独立的构建输出目录管理实验产物。

## 可选宿主 PGO

Clang 的宿主代码 PGO 默认关闭。它根据真实运行负载优化构建期编译的运行时 C++，与客体程序的离线 AOT 是独立选项，不需要运行时 JIT。以下示例要求 root 中包含 Python 3.12、Node 及其库，并使用新的 benchmark 输出目录：

```sh
cmake -S hosts/ios/linux-runtime -B /tmp/operit-pgo-train \
  -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF -DOPERIT_ARM64_PGO_GENERATE=ON
cmake --build /tmp/operit-pgo-train --parallel --target operit-arm64
mkdir -p /tmp/operit-pgo-raw
LLVM_PROFILE_FILE=/tmp/operit-pgo-raw/profile-%p.profraw \
  python3 hosts/ios/linux-runtime/tools/benchmark_library_aot.py \
  --control /tmp/operit-pgo-train/operit-arm64 \
  --candidate /tmp/operit-pgo-train/operit-arm64 \
  --root /absolute/path/to/rootfs --output /tmp/operit-pgo-workloads \
  --repeats 1 --workload python-math --workload node-startup --workload node-math
xcrun llvm-profdata merge -output=/tmp/operit-host.profdata /tmp/operit-pgo-raw/*.profraw
cmake -S hosts/ios/linux-runtime -B /tmp/operit-pgo-release \
  -DCMAKE_BUILD_TYPE=Release -DOPERIT_ARM64_PGO_PROFILE=/tmp/operit-host.profdata
cmake --build /tmp/operit-pgo-release --parallel
```

收集和使用 profile 时应使用匹配的源代码、Clang 版本与构建选项；如果启用客体 AOT，也要在两次配置中使用相同的客体 AOT 参数。实际部署前应选择有代表性的负载训练，并在目标设备及未参与训练的负载上验证性能和正确性。不要把已有的 `.profraw` 混入新的训练；profile 更新后构建会重新编译受影响目标。iOS 构建可传入同一个 profile 选项，但 Mac 测量结果不代表 iPhone 性能。

完整能力与产品接管条件见 [运行时状态](../../../hosts/ios/linux-runtime/docs/status-and-migration.md)。

## 产品环境与 iOS 接入

完整环境在 Linux 上生成：

```sh
sudo bash tools/ios-runtime/arm64/build_rootfs_linux.sh
```

输出 `apps/flutter/app/ios/Runner/arm64-rootfs.tar.gz` 及摘要；包含 Node/npm、Python/pip、Git、SSH、编译工具、常用终端工具与应用依赖。Linux x86_64 构建机使用 qemu-user-static/binfmt 安装 AArch64 软件包；iOS 执行使用仓库内的独立解释器。

Runner 的 `Build ARM64 Runtime` 阶段调用 `ios/build_arm64_runtime.sh`，将静态库、公开头文件与受检根目录归档放入本次构建目录。根目录首次安装到 `Library/Application Support/operit-arm64/rootfs-arm64`，后续启动复用，不覆盖用户安装的软件。

在已启动的 ARM64 iOS 模拟器上执行生产桥接测试：

```sh
python3 tools/tests/run_arm64_simulator_smoke.py
```

它运行实际 shell、Python、Node/npm、实时输出与取消、PTY 缩放、宿主目录读写、退出状态、跨进程持久化，并检查越界归档被拒绝。
