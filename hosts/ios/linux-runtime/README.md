# Operit ARM64 Linux Runtime

自研的 AArch64 Linux 用户态兼容运行时，目标是在 Apple ARM64 上提供可运行的 Linux 开发终端。核心为 C++17 解释器与 Linux 用户 ABI，独立构建，不链接 iSH。Guest 指令由解释器执行；可选的固定函数替换与宿主库预先编译，不在运行时生成宿主机器码。

生产代码已有 ELF 动态装载、进程 / 线程、fork/COW、exec/wait、信号与定时器、futex、并行 executor、文件 / 挂载、网络、proc/dev、PTY 与终端接口。Flutter / Rust / iOS 桥接以及外部 root 配置、目录 bookmark 和原位 rootfs 接入也已落盘。这是代码范围说明；构建、guest 行为、性能和模拟器验收分别记录在 [唯一最新交付状态](docs/delivery-status.md)。

[功能矩阵](docs/completeness.md) 区分实现、接线和验证，并分开衡量可运行开发终端与完整 Linux 能力。[文档索引](docs/README.md) 汇总专题和历史审计；历史比例或指令样本数不能代表当前整体完成度。

## 目录与边界

```text
linux-runtime/
├── include/operit/arm64_runtime.h  公开 C API
├── src/
│   ├── api/                      C ABI、实例生命周期与异常边界
│   ├── runtime/                  Machine、进程、Scheduler、并行 executor、console
│   ├── cpu/aarch64/              寄存器、解码、块缓存、标量、FP/SIMD、独占访问
│   ├── memory/                   Guest 虚拟地址、权限、稀疏页、COW 与代码失效
│   ├── elf/                      ELF64、PT_INTERP、初始栈与 auxv
│   ├── fs/                       路径、FD / OpenDescription、挂载与虚拟节点
│   ├── linux/                    AArch64 ABI、syscall 分域、信号、网络、终端与限制
│   ├── acceleration/             摘要固定的预编译函数替换
│   └── base/                     宿主 I/O、RAII、随机数与摘要
├── cli/                          命令行执行入口
├── tests/isa/                    指令目录、差分输入与报告工具
├── tests/guest/                  AArch64 Linux ELF 测试源码
├── tests/                        C API、子系统与应用探针
├── benchmarks/                   同负载对照测量
└── docs/                         最新事实、功能矩阵、专题与历史审计
```

构建、下载与 rootfs 制备工具位于仓库 `tools/ios-runtime/arm64/`。CPU 行为留在 `cpu/aarch64/`，Linux ABI 留在 `linux/` 各域，产品 UI / session / bookmark 留在 App 和 bridge 层。

## 构建和执行

以下是使用方法，须在仓库根目录执行；命令列在此处不代表本轮已执行或通过。宿主需要 CMake 与 C++17 编译器，guest 测试需要 AArch64 Linux LLVM clang 和 `ld.lld`。非 Apple 构建需要 OpenSSL Crypto 开发库；Apple / iOS 使用系统 CommonCrypto。

```sh
bash tools/ios-runtime/arm64/build.sh host
python3 tools/ios-runtime/arm64/prepare_rootfs.py
cmake -S hosts/ios/linux-runtime -B tools/ios-runtime/arm64/build/host \
  -DOPERIT_ARM64_ROOTFS="$PWD/tools/ios-runtime/arm64/build/rootfs"
cmake --build tools/ios-runtime/arm64/build/host --parallel 6
ctest --test-dir tools/ios-runtime/arm64/build/host --output-on-failure

tools/ios-runtime/arm64/build/host/operit-arm64 \
  --root tools/ios-runtime/arm64/build/rootfs --stats -- \
  /bin/busybox sh -c 'i=0; while [ "$i" -lt 3 ]; do echo "$i"; i=$((i+1)); done'

bash tools/ios-runtime/arm64/build.sh xcframework
```

XCFramework 目标输出为 `tools/ios-runtime/arm64/build/OperitArm64.xcframework`，包含 iOS arm64 与 Simulator arm64 静态库和公开头文件。host 构建、XCFramework、Runner、模拟器与真机是不同验证层级，结果见交付状态。宿主链接需要 C++ 标准库与系统 `resolv` 库。

## 外部 root 与 API 契约

root 由调用方提供，宿主路径权限来自调用方。公开 `ort_arm64_bind` 可在 load 前将已获授权的宿主目录绑定到 guest 绝对路径，支持只读 flag。CLI 示例：

```sh
operit-arm64 --root /host/linux-root \
  --bind /host/workspace:/workspace \
  --bind-ro /host/resources:/resources \
  -- /usr/bin/python3 /resources/script.py
```

每个 VM 持有独立的 guest 进程 / 任务、内存、FD 和解码状态。零初始化 `ort_arm64_config` 后设置 root 并创建 VM，调用 `ort_arm64_load` 装载 guest 绝对路径，再以预算反复调用 `ort_arm64_run`。预算耗尽保留任务状态；阻塞等待返回 `ORT_ARM64_WAITING`，退出、故障和未支持操作有独立结果。加载失败后应销毁并重建 VM。

同一 VM 的公开调用须串行化；内部 executor 可使用多个宿主执行线程，`execution_threads=0` 自动选择最多四个。I/O 回调在调用 `ort_arm64_run` 的线程同步执行，返回字节数或负的 Linux errno。预算限制执行步数，不限制同步宿主 I/O 的 wall 时间。`ort_arm64_terminal_*` 已提供 PTY 输入、输出、尺寸和信号接口，在 VM 所属串行线程调用；App 会话负责驱动与回收。

固定函数替换默认关闭，`enable_fixed_acceleration` / CLI `--fixed-acceleration` 可启用完整 ELF 摘要固定的 musl `memcpy`、`memmove`、`memset`、`strlen`。范围、失效和计数语义见 [架构文档](docs/architecture.md)，性能证据独立评估。

- [唯一最新交付事实与最终证据待补区](docs/delivery-status.md)
- [24 组功能矩阵与完成度计算口径](docs/completeness.md)
- [文档目录与专题索引](docs/README.md)
- [外部 root、迁移边界与历史性能](docs/status-and-migration.md)
- [工具与可重复测量](../../../tools/ios-runtime/arm64/README.md)
