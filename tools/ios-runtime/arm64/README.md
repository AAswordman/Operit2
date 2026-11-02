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
