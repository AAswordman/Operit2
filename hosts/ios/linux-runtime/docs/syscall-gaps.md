# Syscall 缺口与本轮补齐

更新日期：2026-11-03（Asia/Shanghai）。本表来自当前 `src/linux/syscalls.cpp` 中央分派及对应处理函数的源码检查。编号以 Linux v6.12 `include/uapi/asm-generic/unistd.h` 为固定对照，属于 AArch64 LP64 ABI 的本轮清单，不是最新内核全部调用的穷举，也不是兼容率或运行验收结论。

## 关键接口接线与验证

独立 macOS runtime 的本轮构建与 CTest 结果见 [delivery-status.md](delivery-status.md)。这里的“已接线”只覆盖列出的子集；它不代表对应 Linux 子系统完整，也不代表 App 或 iOS 真机已经验收。

| 接口 | 当前已接入范围 | 主要限制 |
|---|---|---|
| `rseq`、`futex_waitv`、`futex_wake/wait/requeue` | 线程注册、调度恢复和新版 futex 的等待、唤醒与重排；有独立及 guest 回归 | futex2 目前接受 32 位数据及受支持的标志组合；其他边界不由这些测试推定 |
| `io_uring` | 425–427 进入真实 ring；支持 NOP 与普通文件的同步 READ/WRITE、有限队列和完成事件 | 没有异步 worker、socket、取消、固定缓冲区等；poll 写就绪也未完整实现，详见 [集成约束](../src/linux/io_uring/INTEGRATION.md) |
| POSIX 消息队列与 System V msg / sem / shm | guest 对象、阻塞及生命周期路径已接入；POSIX MQ 直接和嵌套 epoll ET 有回归 | 其他资源和竞态边界仍需持续验证 |
| `ptrace` | 117、等待和进程生命周期已接线；支持受限 fork / vfork / clone 自动跟踪；真实 Alpine `strace -f` 的子进程 exec / write 路径已实测 | 跟踪期间回退串行；SEIZE、单步等不支持，不能据此推定完整 `strace` 兼容。退出与 exec 的限制见 [ptrace 说明](../src/linux/debug/ptrace.md) |
| `seccomp` | 277 与相关 `prctl` 接入 syscall 分派；有 BPF filter、允许、拒绝、陷阱和终止等子集，TSYNC 及冲突回滚有 guest / 模块回归 | strict 模式、listener 等不支持；不能将有限测试当成完整安全沙箱证明 |
| `unshare` / `setns` | 进入中央分派；对不需要拆分共享状态的受限请求有处理 | 当前生产路径不创建或加入新 namespace，也不完成多线程共享 FS / 文件表的拆分；相关请求拒绝，不能用作容器隔离 |

## 早期 syscall 补齐记录

下表及其旧二进制基线保留当时的状态；“待编译”等描述是历史记录。相应的 `arm64-process-vm`、`arm64-path-extensions`、`arm64-identity` 已包含在本轮独立 runtime CTest 中。

| 编号 | 调用 | 接入点 | 状态 |
|---|---|---|---|
| 270 / 271 | process_vm_readv / process_vm_writev | `syscall_process_vm.cpp` | 新源码及回归用例待编译运行 |
| 276 | renameat2 | `syscall_path.cpp` | 新源码及回归用例待编译运行 |
| 452 | fchmodat2 | `syscall_path.cpp` | 新源码及回归用例待编译运行 |
| 143 / 145 / 147 / 149 | setregid / setreuid / setresuid / setresgid | `syscall_identity.cpp` | 保留宿主文件权限对应的身份约束；新源码待编译运行 |
| 148 / 150 | getresuid / getresgid | `syscall_identity.cpp` | 新源码及回归用例待编译运行 |
| 151 / 152 | setfsuid / setfsgid | `syscall_identity.cpp` | 保留原身份与静默拒绝的 ABI；新源码待编译运行 |

本轮实现边界：

- `process_vm_*` 只访问同一引擎实例的 guest 任务和地址空间；按 guest REALCREDS、dumpable、capability 检查跨进程访问，支持 iovec 快照与已复制前缀的部分返回。进程权限不由宿主 PID 或宿主 root 权限授予。生产调用仍位于 worker 屏障后的串行 syscall 阶段。
- `renameat2` 的 NOREPLACE / EXCHANGE 使用宿主原子接口；WHITEOUT 明确返回 EOPNOTSUPP。交换额外更新 Darwin data-open 跟踪中的两端引用；普通重命名继续使用原回调。
- `fchmodat2` 支持 EMPTY_PATH、NOFOLLOW、pinned dirfd / cwd，沿用虚拟文件及只读挂载约束；符号链接改 mode 明确拒绝。Linux 宿主旧 headers 缺少 `SYS_fchmodat2` 时，不能安全处理的 O_PATH 描述符返回 EOPNOTSUPP。
- `getres*` 使用已有 guest credential registry，逐项按 real → effective → saved 写出；同步将旧 getuid / geteuid / getgid / getegid 和 setuid / setgid 路由到同一处理函数。身份 setter 仅允许不改变现有 real / effective / saved / filesystem 状态的请求；不调用宿主 setter，也不模拟已完成的权限切换。`setfs*` 拒绝变化时静默返回实际旧 filesystem ID。无 controller 的独立嵌入场景仅使用可可靠读取的宿主信息，读不到 saved / filesystem ID 时返回 ENOSYS。

2026-11-03 使用已有 CLI 和已安装的 Alpine rootfs 调用以上 12 个编号，全部返回 errno 38（ENOSYS）。原始命令、CLI SHA-256、stdout / stderr 保存在 `tools/ios-runtime/arm64/build/reconciliation-current/syscall-extensions-baseline.json`。这是旧二进制的缺口证据，不能证明新源码行为通过。

新回归入口：`arm64-process-vm`、`arm64-path-extensions`、`arm64-identity`。这里只登记 CMake / CTest 源码；用户负责构建，本轮不运行编译器，也不运行会编译 guest fixture 的 `tests/test_runtime.py`。

## 仍无中央路由的主要调用示例

下面只列已对照源码确认的例子；同一调用内部未实现的 flag、权限及生命周期组合另计。

| 领域 | 缺口示例 | 补齐需要的实际能力 |
|---|---|---|
| 异步 I/O | io_setup / destroy / submit / cancel / getevents (0–4)、io_pgetevents (292) | guest 请求队列、异步完成、取消、FD 与缓冲区生命周期、poll / 信号交互 |
| 锁页 / NUMA | mlock / munlock / mlockall / munlockall (228–231)、mlock2 (284)、mbind / mempolicy / move_pages 等 | 真正的锁页、RLIMIT_MEMLOCK 记账，或明确的 guest NUMA 模型；不能靠返回成功补齐 |
| 安全 / 调试 | bpf (280)、landlock (444–446)、userfaultfd (282) | 实际访问控制、过滤或页故障协议 |
| 文件 / 事件 | mknodat (33)、vmsplice (75)、sync (81)、syncfs (267)、fanotify (262–263) | 路径与节点类型隔离、管道页生命周期、限定 guest 挂载范围的写回和事件投递 |
| 命名空间 / 挂载 | pivot_root (41)、chroot (51)、open_tree / move_mount / fsopen / fsconfig / fsmount / fspick (428–433)、mount_setattr (442) | guest 命名空间和挂载树、共享规则、FD 引用与已有工作区挂载规划的一致生命周期 |
| 系统与特权接口 | sethostname (161)、setdomainname (162)、clock_settime (112)、reboot (142)、module / kexec 等 | 需要 guest 状态或明确不支持的特权操作；不能修改宿主系统或伪造执行成功 |

## 阅读与验收边界

中央存在 `case` 只表示能到达处理函数，不表示支持该 syscall 的所有语义。12 个新增编号也不表示新增 12 项完整 Linux 子系统。原 [completeness.md](completeness.md) 的 24 组能力范围继续保留；运行结果统一记入 [delivery-status.md](delivery-status.md)。

## 早期静态检查记录

- 11 个相关 C/C++ 文件已做语法树检查：未发现新增语法树错误，3 个解析器已有局限与 HEAD 一致。这不是编译或 C++ 类型检查。
- 已核对固定版本的 12 个 ABI 编号、18 个新增 / 迁移路由、两个新源文件的 CMake 登记、三个独立回归入口、EXCHANGE 回调及本轮文件的 diff 空白检查。
- 上述检查结果与文件 SHA-256 保存于 `tools/ios-runtime/arm64/build/reconciliation-current/syscall-extensions-source-review.json`。
- Darwin 原子重命名与 chmod 的纯宿主 API 检查不等于新引擎的 guest 回归通过。三个新增 C++ 测试均未编译、未运行。
