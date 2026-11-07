# ARM64 大区域直接生成 Wasm

编译器在 `src/cpu/aarch64/wasm_region.cpp`，实际入口在 `ExecutionEngine::run`。
区域发现后通过 `build_instruction_graph` 构建与原生后端共用的 `BlockGraph`，调用 `optimize_graph`，再由 Wasm 后端降低优化后的 `InstructionNode`。共用基本块划分、指令副作用分析、常量传播/折叠、死结果消除和无用 NZCV 计算消除；原生 `StaticOperation` 降低不参与 Wasm 生成。
条件分支的目标和落空路径都作为基本块入口，块间不传播常量事实。死结果和标志位消除以访存、控制流和辅助操作为边界，常量事实按指令副作用失效。原始指令节点和 PC 保留，用于预算准入、退休计数与精确退出；删除或折叠发射代码不改变客体指令数。`WasmRegion` 提供消除数、常量结果数和无用标志位计算数，供集成验证检查优化器实际生效。
直接生成 baseline Wasm 的整数运算、部分浮点与 SIMD 操作、普通访存和控制流，没有把 ARM64 解释器编译进 Wasm。
构建开关为 `OPERIT_ARM64_WASM`，默认 `OFF`。关闭时执行入口跳过 Wasm，且不编译或链接 WebKit 执行后端；开启时没有热度门槛或按退休量禁用的规则。

直接 CMake 构建使用 `-DOPERIT_ARM64_WASM=ON`。iOS/macOS App 打包脚本也默认关闭，可通过环境变量 `OPERIT_ARM64_WASM=ON` 开启；这是构建配置，不提供运行时设置。

Wasm 区域最多发现 16384 条指令。相连的基本块、直接调用及其返回尽可能在同一个模块里执行。
回边及跨组边使用两级 `br_table`；跨模块的已编译入口通过共享 Wasm 函数表调度，客体寄存器直接保存在共享线性内存中。

Apple 上使用 WKWebView，运行时工作线程等待主线程调度 WebContent。
主线程调用和非 Apple 平台继续使用原有执行器。
每个执行器会话使用独立 WKWebView，保留自己的模块、页和入口表；交替运行其他会话不会重新上传已安装的模块。执行器销毁或失效时，WebView 在主线程释放。
不同会话不再持有全局执行锁，可以独立等待和推进；同一会话的调用仍串行化。WebKit 的进程调度和实际 CPU 并行度由系统决定。系统调用、未支持的指令和区域外地址仍需 native 参与。
预算不足以完成当前基本块时，由原执行器执行剩余精确指令数。
iOS 终端运行预算为 268435456 条；当已有调度条件允许延迟记账时，同步扩大时间片，多任务和定时器等情况仍使用原有调度策略。

访存使用多个驻留的私有页；缺页仍需要向 native 请求，直接映射的页表槽冲突也可能导致重复请求。文件页在执行边界重新验证，共享页和并行地址空间不能直接按私有页处理。
跨页访问和尚未支持或不能安全加速的指令仍通过原生语义执行。
Wasm 写入通过 Memory 回写，包括相同值的写入，以保留 COW 和独占保留失效语义。
脏页在首次写入时登记，交互时只回传登记的页。状态、模块和页通过 `WKURLSchemeHandler` 以二进制帧传递，不经过 Base64；进程间仍存在数据复制。
内存权威副本和 guest 调度仍在 native，当前实现尚不是独立常驻执行域。

macOS 26.6、Apple M5 上，原始 100×100 Python 负载在交替执行的五次测量中，Wasm 热循环中位数为 0.433 秒，原生为 0.517 秒，checksum 均为 1267865。另一次带打点的运行中，Wasm 两次外层进入分别执行约 2.684 亿和 2274 万条客体指令；区域内部仍有约 1598 万次函数调度。以上不是 iOS 真机测量。

只保留实际执行路径的集成验证，已合并独立微核工具里的重复编译和运行脚本。

```sh
cmake -S hosts/ios/linux-runtime -B /tmp/operit-wasm -G Ninja \
  -DCMAKE_BUILD_TYPE=Release -DOPERIT_ARM64_LTO=OFF -DOPERIT_ARM64_WASM=ON
cmake --build /tmp/operit-wasm --target operit-arm64-wasm
ctest --test-dir /tmp/operit-wasm -R arm64-wasm --output-on-failure
```

`operit-arm64-wasm` 无参数执行集成验证；传入与 `operit-arm64` 相同的参数可运行原负载。
前者提供 macOS 主线程事件循环，让实际运行时在工作线程使用 WebKit。
后者在主线程执行，因此可作相同 C++ 构建的原生执行器对照。
两者都没有 Wasm 启用参数。
