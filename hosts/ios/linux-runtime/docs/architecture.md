# ARM64 Linux 运行时架构

本页描述生产代码的模块与执行模型。当前接线变化、构建和运行证据统一见 [交付状态](delivery-status.md)，覆盖范围见 [功能矩阵](completeness.md)。设计已存在不表示所有边界已验收。

## 执行路径与依赖

```text
Flutter / Rust / iOS native bridge 或 CLI
  → include/operit/arm64_runtime.h
  → src/api/c_api.cpp
  → runtime/Machine（进程集合、共享调度与生命周期）
      ├─ process/image_loader → elf/loader、initial_stack → fs、Memory
      ├─ tasks/Scheduler → 每个 Task 的 CPU、TLS、等待、信号与计账
      ├─ run_parallel → tasks/ParallelExecutor → ExecutionEngine / Memory
      ├─ cpu/ExecutionEngine → 控制流图、静态汇编、语义函数、SIMD、exclusive
      ├─ linux/Syscalls → 文件 / 映射 / 进程 / 信号 / 定时器 / 网络 / 终端
      ├─ process/signals、timers、accounting、observability、wait
      └─ acceleration/FixedComponents → 固定清单、摘要、Memory
```

CPU 层不打开文件、不转换 Linux errno、不持有产品 session。SVC 返回 trap，由 Machine 在安全点调用 syscall 层，再恢复任务。ELF 层装载 guest 映像与启动栈。App UI、目录授权与会话驱动位于引擎之外，标准 I/O 与 PTY 经 C API 接出。

## 进程、任务与并行

一个 Machine 管理多个 guest 进程与线程。`runtime/process/process.h` 保存进程身份、父子关系、进程组 / session、映像、信号 / 定时器及退出状态。clone 按已支持 flags 共享资源或建立进程 / COW 地址空间；exec 替换映像并处理线程、FD 和信号状态；wait / reap 回收 guest 身份。未支持组合返回错误。

`runtime/tasks/` 保存线程的寄存器、TLS、signal mask、clear_child_tid、robust-list、调度优先级、affinity 和等待状态。阻塞 syscall 让出执行，futex、poll、定时器与信号在调度边界恢复任务。PI 优先级传入 Scheduler 的有效优先级计算，robust owner-death 与退出有清理路径；未支持操作见功能矩阵。

`execution_threads` 控制内部宿主执行线程数，0 自动选择最多四个。Machine 可调用 `run_parallel`，将可运行任务交给持久 ParallelExecutor；工作阶段执行 guest 指令，停止后由串行安全点处理 syscall、信号与生命周期。每个执行任务保留独立 ExecutionEngine 状态与 CPU 计账。对同一 VM 的公开 C API 调用仍必须串行化。

多线程存在不等于获得加速。Memory 共享锁、块缓存重校验和调度成本需要同条件测量；Worker / 争用记录集中在交付状态。

## CPU、内存与代码失效

Guest 使用 AArch64 寄存器和独立虚拟地址空间；初始栈按 ABI 对齐。`ExecutionEngine` 将唯一客体 PC 构成控制流图，完成纯整数常量传播和死值/标志处理，再按统一驻留 ABI 选择构建时生成的静态汇编执行单元。详见 [静态汇编执行器](static-execution.md)。

所有图固定驻留 guest X0–X2、X19–X20、SP、X30 和 V0–V7。跨图边在缓存代次、代码版本及观察作用域守卫后继续使用原有宿主寄存器，不再换位或保存/重载驻留值；其他寄存器直接使用架构 shadow。预算不足时执行原始指令短尾；系统调用、故障和外部返回恢复精确架构状态。已删除旧 region/kernel 的 C++ 函数指针 ABI、musttail 模板工厂、短预算缓存变体和两个入口值的桥接特例。

Guest 采用 4 KiB 页、稀疏映射、访问权限、双路 TLB、COW 与文件/共享映射。原生访问只接受经过检查的 serial 私有翻译；COW、共享页、跨页、可执行页写入与并行访问继续走同步内存路径。映射和驻留量分开计量，驻留限制不等于宿主进程全部内存。

代码图共享不可变代码快照，覆盖的每个代码页都参与版本和字节校验。文件代码在新的观察作用域重新检查；缓存淘汰先使外来桥接失效再回收目标图。没有运行时机器码生成，也不依赖客体程序或语言版本白名单。独占访问、共享映射、seals、外部文件修改与截断继续服从 Memory 的语义。

`cpu/aarch64/` 分为 decoder、block_graph、optimizer、register_allocator、static_lowering、static_executor、execution_engine、semantics 和各 ISA 语义模块。`semantics` 保留完整参考语义，用于复杂形式和短预算尾部；测试中的参考执行不经图编译器。宿主 FPCR/FPSR 在汇编执行边界与语义调用边界受到隔离。

## Linux ABI、文件系统与事件

`linux/abi.h` 定义 guest 布局；Linux 打开标志、errno、stat、timespec、socket 地址与 control 消息显式转换，不能直接复制 Darwin ABI。`syscalls.cpp` 负责中央路由与统计，分域处理器拥有实际操作；未知 syscall 返回 `-ENOSYS` 并记录编号。

`DescriptorTable` 区分数字 FD 属性与共享 `OpenDescription`，dup / fork / SCM_RIGHTS 可共享 offset 和状态。文件 / FD、file transfer、缓存 / 同步、空间分配、metadata、openat2 与 memfd 有独立实现，不能继续将 sendfile 等整体写成未实现。宿主受限语义须保留具体错误或说明降级边界。

Filesystem 用已打开 FD 锚定 root / 绑定源，逐段处理路径、symlink、挂载和 `..`。已有只读 / 可写绑定、guest mount、proc / dev / devpts / tmpfs 和虚拟节点；路径与元数据边界见 [虚拟文件系统专题](virtual-filesystems.md)。APFS 行为不等于全部 Linux 文件系统语义。

信号包含 guest frame / 投递 / mask / pending / 返回路径；定时器包含 real / CPU、timerfd、ProcessTimers。Machine 和 Scheduler 的运行 / idle 安全点调用 runtime event poll 与 deadline；Pending observers 连接 timer overrun / 额度。proc accounting 经 ProcessController 查询 Machine。最新生命周期修复是否已冻结并验收见交付状态。

真实 socket、DNS resolver、poll/epoll 与消息转换见 [网络专题](network.md)。inotify 在 Linux 与 Darwin 使用不同宿主后端，Darwin vnode / snapshot 不覆盖所有 Linux 事件；不支持掩码与虚拟节点有明确边界。

## 固定函数替换

加速默认关闭。完整 ELF 的 SHA-256 必须命中固定清单，入口 PC 也须匹配；实现与宿主库预先编译。当前范围为 musl 的 `memcpy`、`memmove`、`memset`、`strlen`，没有任意 guest 程序 AOT 或动态代码生成。

装载器对匹配映像保留代码快照。epoch 变化后检查权限与内容，修改 / 卸载时回到解释器。替换遵守 AAPCS 参数、返回值与 LR，所有数据读写仍经 guest 内存检查；未命中或范围无效时回退。已认证 musl 的长 memcpy 可按页和预算分段续跑；共享、文件映射、可执行目标或物理页别名不使用批量复制。memmove 重叠与零长度遵循显式处理。

替换按实际完成的 guest 指令数消耗预算，可以在预算边界暂停。`accelerated_calls` 计加速片段，不等同于完整函数调用；`accelerated_bytes` 只统计单段完成的函数，不包含所有续跑的长拷贝，不能作为全部加速字节数。两种模式应比较相同输出的执行 / wall 时间，不能只用计数器推断加速比。历史测量见 [迁移与性能记录](status-and-migration.md)。

## 验证入口与证据边界

真实 guest ELF、原版 Alpine、C API / 子系统测试、硬件差分、Node / Python / 开发工作流、sanitizer 与 iOS harness 提供不同层级的证据；入口存在不表示已经运行。`tests/isa/catalog.py` 提供独立输入，`audit.py` 生成硬件差分报告；架构 JSON 的旧 remaining 文本可能早于新实现，不能单独作为当前缺口。

完整指令能力需要按形式与编码覆盖，真实应用只能提供所经路径的行为证据。CI 定义、SDK 编译、语法检查与 host 对照均独立记录；最新结果与待补范围统一到 [交付状态](delivery-status.md)。[2026-11-02 审计](audit/2026-11-02.md) 保留历史时点。
