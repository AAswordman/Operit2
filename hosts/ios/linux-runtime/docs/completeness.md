# 功能覆盖矩阵与完成度口径

核对日期：2026-11-03。本文保留原 **24 组**编号，供逐项查实现体、调用链与缺口。下方矩阵及其中“未编译 / 待运行”等描述保留当时的历史核对口径，并非当前所有测试的状态；**最新结果、证据来源和时序统一见 [delivery-status.md](delivery-status.md)**，当前 syscall 子集见 [syscall-gaps.md](syscall-gaps.md)。本文不另设当前通过率。

本轮独立 runtime 已接入并测试 `rseq`、新版 futex、受限 `io_uring`、POSIX MQ、System V IPC、`ptrace`、`seccomp` 和受限 namespace 调用。namespace 不提供新 namespace 或容器隔离；`ptrace` 支持受限自动 fork/clone 跟踪，但跟踪期间回退串行。各子集当前边界和最新验收以 [syscall-gaps.md](syscall-gaps.md) 和 [delivery-status.md](delivery-status.md) 为准。接线与用例通过不代表各子系统的 Linux 语义完整，也不代表 App 或真机已验收。

## 实现、接线、验证分列

| 维度 | 计入条件 | 不计入的证据 |
|---|---|---|
| 已实现 I | 有实际执行体，处理参数、状态和失败路径；列明已覆盖子范围 | 只有声明、源文件名、解码后空操作或无条件返回成功 |
| 已接线 W | 实现进入构建，中央 / 公开入口实际调用；有状态行为接入 Machine / Scheduler / App 生命周期 | 只有 CMake 条目、没有调用者的 helper、只有测试直接调用 |
| 已验证 V | 对应行为实际执行，保存命令、返回码、输出与代码适用范围 | 构建、静态分析、host 对照、未运行 fixture、其他旧版本或相邻功能通过 |

矩阵的“有实现 / 有接线”均只指该行写出的子范围。“部分”“待核对”“待跑”不能按整组完成。已验证栏保留历史行为证据的性质，不把其转成当前源码的验收结论。

若要按固定 24 组汇总，应先冻结每组子项与验收条件，再分别报告 `I/24`、`W/24`、`V/24`；一组只有全部既定子项满足对应条件时才计 1。待确认项须单列，不能当通过或任意折半。本轮尚无统一冻结版本和逐组完整验收，**不发布新的整组百分比**，也不按代码行数估工时。基础 CTest、ISA 样本、应用探针各自有独立分母，不与 24 组相加。

## 两种目标分别判断

| 目标 | 范围 / 判定条件 | 尚待补齐的关卡 |
|---|---|---|
| 可运行开发终端 | 同一版本上可交互 shell / PTY、外部命令与作业控制、共享目录与持久化、GCC / Git / Python / npm 实际工作流、Node 并发、取消后复用、iOS root 配置和会话恢复 | 失败 CTest 修复后重跑；原时限 Worker；新 guest 事件 / 资源测试；最新 Runner / 模拟器及设备生命周期。各关卡须分别有证据，不能由可启动推断可交付 |
| 完整 Linux 用户态能力 | 明确 AArch64 EL0 指令与 Linux 用户 ABI 范围，按指令形式 / 编码、syscall / flag / 错误、文件系统及进程生命周期逐项覆盖 | 完整目录尚未逐项闭环；存在下文明确未支持语义。运行几个真实应用不能证明全部 Linux 能力；本项目也不执行 Linux 内核 / EL1 |

两种目标使用不同判据，不能把开发终端可用性写成完整 Linux 兼容百分比。真机性能与长稳需要设备证据，不能折入代码实现率。

## 24 组历史核对矩阵

源码路径相对 `hosts/ios/linux-runtime/`；`tools/`、`apps/` 路径相对仓库根。构建登记检查以 `CMakeLists.txt` 为入口，App native 登记检查以 `Runner.xcodeproj/project.pbxproj` 为入口。

| # | 范围 | 已实现：执行体与子范围 | 已接线：实际路径 | 已验证：现有证据层级 | 具体缺口 |
|---|---|---|---|---|---|
| 1 | Alpine / 外部 root | `Filesystem` 接受 root；`tools/ios-runtime/arm64/{prepare_rootfs.py,build_rootfs_linux.sh,package_rootfs.py}` 制备工具 | CLI / C API → Machine → Filesystem；iOS 配置见 22 | 有基础 Alpine 历史运行记录 | 外部目录授权失效 / 恢复、损坏升级与最新 iOS 原位运行待验 |
| 2 | ELF / 动态解释器 | `src/elf/{loader,initial_stack}.cpp`、`runtime/process/image_loader.cpp` | CMake → `Machine::load` / exec 生命周期 → loader | 有动态 Alpine 与真实语言历史运行 | 新装载失败回滚、exec 与并发边界待验 |
| 3 | CPU / FP / SIMD | `decoder/execute`、`vector_indexed/integer/widen`、`vector_fp_fixed/indexed` 有执行体；LDTR/STTR、FPCR/FPSR 分支存在；新增 CTR/DCZID 与 DC/IC/ZVA | CMake → execute → simd / floating；缓存路径 decoder → Op::Cache → Memory 排序 / 译码失效 | 有 CPU 测试及历史 ISA 抽样；新增缓存 / 生成代码 ELF 用例，旧二进制复现缺口，新实现未编译 | 尺寸 / lane / signedness / QC、FP 控制与异常、别名 / 保留编码未闭环；FPCR trap 请求仍不支持 |
| 4 | 文件 / FD / 内存 / 挂载 | `fs/descriptor_table` 的共享 OpenDescription；`fs/mount/*`、`syscall_io/path/mount/memory`、`memory/*` | 中央分派 → 文件 / 映射；clone / exec 管理内存与 FD；Interpreter 已调用 validate_code | 已有行为用例，也存在未关闭的 filesystem / VFS 失败 | seals、共享写回、映射失效 / 并发、挂载移动 / 卸载与 FD 重用回归未闭环；`mremap` / `MAP_SHARED` 已有实现 |
| 5 | 现代文件 API | `linux/files/{metadata,open_how,memory_file,close_range,allocation,advice}.cpp`；`events/inotify.cpp` | CMake、中央 file_control / open / inotify、fcntl / I/O / poll；匿名 FD 分类含 inotify | 新增接口大多待对应运行；seals 有明确失败记录 | Darwin guest OPEN / ACCESS / CLOSE 桥接已写但未运行；虚拟节点、exec 装载与宿主外部 data-open 仍有缺口；宿主受限的空间 / 缓存模式有边界 |
| 6 | procfs / devfs | `fs/virtual/{procfs,devfs,node}.cpp`；cmdline / environ / auxv 按 guest 内存范围读 | `mount_process_filesystems()`；磁盘 `syscall_proc` 已直接调用 `ProcessController::proc_stat_accounting`，相关旧 registry API 已查不到 | 有历史 VFS / Python 运行；新 direct 接线仅源码观察 | 待主线确认冻结与旧 registry 全部收敛；CPU / 子进程 / fault / I/O 统计来源及退出 / PID 复用回归 |
| 7 | PTY / termios | `linux/terminal/{manager,termios}.cpp`、`runtime/console/*` | C API terminal → Machine → TerminalManager；process / group_session / signal hooks 已见 | 有历史 PTY 探针；基础 terminal 仍有失败待重跑 | 新 hook 修复、全屏程序、长会话与控制终端边界待验 |
| 8 | shell job control / cancel | `runtime/process/{signals,lifecycle}.cpp`、终端 manager；App SIGINT / polling / 输入回调取消与回收 | setpgid / setsid / 信号 → Machine；Rust streaming → native session owner | 有外部命令 / 流式输出旧记录；旧 Runner 取消保持 session 失败 | 最新取消后复用、后台作业、会话恢复未验收 |
| 9 | process / pidfd | Machine clone / exec / wait / exit；`linux/process/pidfd.cpp` 的 guest 身份、权限与 FD 引用 | 中央 clone / clone3 / exec / wait / pidfd；create / exit / reap 通知接入生命周期 | 有 fork / GCC 历史行为；新路径未完整验 | clone3 / pidfd 权限、退出竞态 / 复用 / 回收组合待跑 |
| 10 | signals / timers | `linux/signals/*`、`timers/{process_timers,timer_file,timer_service}.cpp` | 中央 signal / timer；Machine poll / deadline、Pending dequeue / discard observers、signalfd；退出丢弃调用已见 | 有真实 / CPU timer 历史投递；新增 441 epoll_pwait2 接入现有 Scheduler / epoll 与 mask 清理，待编译回归 | SIGPENDING charge 释放、预留、overrun、exec / 退出 / 中断组合待跑；未支持时钟报错 |
| 11 | threads / futex | `syscall_threads.cpp`、`threads/{futex_pi.cpp,scheduling.h}`、Scheduler/task、robust owner-death | 中央调度 / futex；274 / 275 → sched_attr；Scheduler effective_priority → PI；退出清理；affinity → executor | 有基础任务 / 线程历史行为；新 PI requeue 仅源码与用例，未编译 | WAIT_REQUEUE_PI / CMP_REQUEUE_PI 已接线；优先级捐赠、共享进程 / 退出 / 超时 / 信号和单 / 四核 guest 回归待运行；PI / robust PI 与新 sched_attr 未完成验收；FIFO / RR 持久队列、CPU 时间片、guest yield 与单 / 四核 affinity 派发已写，未编译运行；安全点抢占、普通公平调度与完整实时语义仍待验 / 补 |
| 12 | 多核 executor | `runtime/tasks/executor.cpp`、`runtime/parallel.cpp` 与 per-task CPU 计账 | CMake → `Machine::run` → `run_parallel` → 持久 executor；syscall / 信号安全点结算 | 有 parallel compute / atomics / join 历史运行及 Worker 并行观测 | Worker 尚未达原时限；Memory 锁优化 / seal token 未冻结；设备扩展性待验 |
| 13 | Node / libuv | 真实 Node ELF 通过线程 / 文件 / 网络 ABI 执行；`tests/node/*` 是行为脚本 | `probe_node.py` → CLI → guest Node；watch 已入探针列表 | 历史基础 / 线程池 / 网络通过；watch 仅 host 对照 | 整体 Node 尚未通过；新 watcher guest 与当前源码回归未跑 |
| 14 | Node Worker | 原脚本 4 Worker / 40000 原子计数 / 15000ms guest 截止 | Node → clone / futex / 共享内存 → Machine 并行执行；单 / 四核报告存在 | 两档均实际运行失败；精确数据见交付状态 | 原截止条件、退出 / 同步功能与 wall 性能关卡未通过 |
| 15 | npm | 真实 npm、本地 fixture 依赖安装与脚本执行 | development probe → `tests/development/npm.sh` → `npm install --offline` / verify | 有本地离线历史通过 | 外网 registry、native addon、复杂依赖与新版本待验 |
| 16 | Python / SQLite | 真实 Python / stdlib / SQLite、VFS 与网络走 Linux ABI | `probe_python.py` → environment / VFS / network / external network | 有历史探针通过 | 更多扩展 / 大项目与当前改动回归未证明 |
| 17 | venv / pip | `tests/development/python.sh` 创建 venv，执行 SQLite 与 `pip --version` | development probe 实际调用 guest 工具 | 有上述范围历史通过 | 在线包安装、构建扩展 / 依赖求解未被该用例验证 |
| 18 | apk / 开发包 | 制备工具调用 apk；root 中有包数据库 / 安装日志与 GCC / Git | root 制备 / 校验 / 打包 / CLI 运行链存在 | 有历史安装记录，预装工具运行不等于包管理闭环 | 当前 runtime 内 update / add / remove、联网 / 签名 / 失败恢复待独立证据 |
| 19 | GCC / Git | `tests/development/{compiler,git}.sh` 的 pthread / fork 编译运行和本地 Git 工作流 | development probe → guest 编译器 / Git → 进程 / 文件 API | 有对应两项历史工作流通过 | 大项目、Git 网络、长构建与本轮回归未验 |
| 20 | network / DNS / TLS | `linux/network/{socket,message_batch,unix_namespace,resolver}.cpp`、`syscall_socket*.cpp` | 中央 socket 分派；mmsg、SCM_RIGHTS / credentials、poll / 中断有生产调用 | 有 Python / Node 真实网络历史行为；accept 失败未关闭 | 新 message / control / 选项语义、引用环 / 并发及 accept 修复待验；完整错误队列等未实现 |
| 21 | limits / accounting | `linux/process/{resource_limits,limit_enforcement}.cpp`、`system/{clocks,information}.cpp`、`runtime/process/{accounting,observability}.cpp` | NOFILE→FD；AS/DATA/STACK→映射 / brk / exec；FSIZE→写 / 分配；NPROC→clone；SIGPENDING→队列 / timer；CPU→安全点；NICE / RTPRIO→目标调度权限；RTTIME→线程持续 RT CPU / 进程信号 | 有旧 NOFILE / AS；新 resources 用例仅接测试入口 | FSIZE/NPROC/SIGPENDING/timer预留/CPU hard limit 未跑；NICE / RTPRIO 的限额变更与调度联动已写未运行；RTTIME 的线程计时、唤醒重置与软 / 硬信号已写未运行；MEMLOCK 等仍未支持；统计准确性待验 |
| 22 | iOS root / UI / bridges | App `Arm64RootFilesystem.mm` / `Arm64TerminalBridge.mm`、Dart 配置 service / UI、Rust bridge | Xcode Sources 登记；设置→service→channel→公开配置 API→root / VM；configurationId 传递 | 只有本轮 Dart 静态分析 / simulator harness 语法证据；详情见交付状态 | 最新完整 Runner / 模拟器 / 外部 root 断言未执行 |
| 23 | iOS cancel / lifecycle | native session polling / SIGINT / 回收、Rust / Dart generation | streaming 回调；公开 `arm64RuntimeConfigure` 拒绝 live 切换与 `closeExistingSessions`；fixture 有断言 | 旧 Runner 取消保持 session 失败；新断言未执行 | 后台 / 重启 / 授权恢复、取消后复用和并发 session 未验 |
| 24 | device 性能 / 长稳 | 宿主指标采集可复用；**设备专项交付结论未建立** | iOS 入口存在，真机持续运行 / 恢复与指标验证链未完成 | 无本轮真机性能 / 长稳通过证据 | 设备基线、长会话、后台恢复、内存压力与真实工作流长期运行未验 |

## 重点接口：实现体与生产调用核对

下表细化既有分组，不增加 24 组分母。列出的源在 CMake，中央调用可沿 `src/linux/syscalls.cpp` 跟进；涉及生命周期的 API 仍须结合最新状态中的待冻结项。

| 接口 | 实现与路由 | 有状态接线 / 已知限制 |
|---|---|---|
| indexed / doubling / polynomial / pairwise 整数 SIMD | `vector_indexed.cpp` / `vector_multiply.h` / integer / widen / arithmetic → `execute_simd` 的 widen / integer 路径 | 实际写寄存器 / QC；尚待新增族尺寸 / lane / 饱和 / 保留编码硬件差分 |
| statx / statfs / xattr / openat2 | `files/metadata.cpp` / `open_how.cpp` → file / open 分派 | O_PATH、只读挂载与路径约束有处理；anon-inode 由共享分类器提供，含 inotify；跨平台语义仍有边界 |
| memfd / seals | `files/memory_file.cpp` → descriptor / fcntl；写 / resize / mmap / mprotect 约束 | token / 共享映射一致性待冻结与失败回归；不以设置 seal 成功推断约束成功 |
| readahead / fadvise64 / sync_file_range | 213 / 223 / 84 → `dispatch_file_control` → `files/advice.cpp` | 真实 FD 宿主操作，`require_data_descriptor` 拒绝 O_PATH；Darwin 有界预热 / pread 不改 offset，range 同步退化到整文件同步 |
| fallocate / close_range | 47 / 436 → `dispatch_file_control` → allocation / close_range | FSIZE / memfd guard、record-lock before_close 已调用；受限分配模式报错；共享 FD / 锁 / 终端交互待验 |
| inotify | 26–28 → `dispatch_inotify` → `InotifyFile`；read / poll / FIONREAD；Machine safe point pump | Darwin 由 OpenDescription 的 InotifyOpenFile 接 open / read / readv / getdents / transfer 与最后关闭；跟踪重命名、unlink / EXCL_UNLINK、共享 FD 及 mmap 引用；Linux 使用原生事件避免重复。新桥接未编译，虚拟节点 / exec 装载 / 宿主外部 data-open 尚未覆盖 |
| signalfd / POSIX / CPU timers | 74 → signal_file；102 / 103 / 107–111 → timer → `ProcessController::process_timer` | Machine poll / deadline 与 Pending observers 已见；退出 discard 新调用只作源码记录，charge / overrun 运行待验 |
| PI futex / 调度 | 98 → PI helper；Scheduler effective_priority / robust 退出；调度查询与设置中央入口存在 | WAIT_REQUEUE_PI / CMP_REQUEUE_PI 分别进入两阶段条件等待 / PI 迁移；Scheduler 按实时优先级与 FIFO 选 source waiter；信号按迁移阶段处理；新实现未编译运行 |
| 调度属性 / NICE / RTPRIO | 274 / 275 → 版本化 sched_attr → Task 字段；NICE / RTPRIO → prlimit / setrlimit → 原有调度权限校验与 runnable 选择 | 支持普通 / Batch / Idle / FIFO / RR 的既有 guest 字段、reset / keep 标志、48 / 56 字节与零扩展 ABI；原生 host 调度不被修改。Deadline、自定义 runtime 与利用率请求明确不支持；FIFO 持久队列、RR 100ms CPU 时间片与 124 / 127 syscall 已接源码；策略切换保留余量、PI 回落队首、不同 CPU 匹配与串行回退已写用例，未编译运行；不等于完整 Linux 实时调度或普通 CFS |
| RLIMIT_RTTIME | setrlimit / prlimit → 可变限额；串行 stop_accounting 与 worker barrier → Scheduler 的每线程连续 RT 服务计数；安全点 → ResourceLimits 阈值 / 共享软限推进 → process-directed SI_KERNEL SIGXCPU / SIGKILL | 仅有限额时累计实际线程 CPU 时间，包含 PI 暂时提权；阻塞唤醒 / 真正恢复停住任务重置，yield / 抢占不重置；普通策略不累计。`arm64-realtime` 新增真实 CPU 与信号流水线用例，未编译执行；阈值检查在解释器安全点，不模拟 Linux 原生 tick 精度 |
| pidfd | 424 / 434 / 438、waitid → guest pidfd | Machine create / exit / reap 通知与 guest 凭据 / FD 引用连接；新竞态待验 |
| message batch / credentials | 211 / 212 / 243 / 269 → `syscall_socket_message.cpp` / `network/message_batch.cpp` | 已有真实消息、control / 部分完成与 guest 凭据路径；不等于全部 ancillary 或宿主外部 IPC 支持 |
| proc accounting | `syscall_proc.cpp` → `ProcessController::proc_stat_accounting` → `Machine` / observability | 本次磁盘已查见 direct 调用、相关旧 registry API 消失；最终接口状态与计数来源待主线冻结复核 |

## 早期 syscall 扩展（历史记录）

新增 270 / 271（process_vm 读写）、276（renameat2）、452（fchmodat2）及 143 / 145 / 147–152（身份查询与受限设置）的路由、实现与独立回归源码。`RENAME_EXCHANGE` 另接 Darwin 已打开文件的双向 dentry 跟踪。12 个编号均已用旧 CLI 复现 ENOSYS；新源码未编译、未执行。详细能力边界、剩余 syscall 清单与证据见 [syscall-gaps.md](syscall-gaps.md)，不能将新增路由数换算为完整 syscall 兼容率。

## 明确未支持、接线待确认与验证待补

| 类型 | 项目 | 处理原则 / 补齐范围 |
|---|---|---|
| 未支持 / 覆盖未闭环 | FPCR trap 请求未支持；完整 AArch64 EL0 指令形式与编码目录尚未闭环 | 具体缺失形式须对照源码逐项确认；`tests/isa/architecture.json` 的旧 remaining 文本不能直接等同当前代码缺口 |
| 部分实现 / 待验证 | Darwin guest OPEN / ACCESS / CLOSE 已接线未编译；虚拟节点 watch、exec 装载与宿主外部 OPEN / ACCESS / CLOSE 仍未覆盖 | 新增 `arm64-inotify` 与真实 guest 生命周期回归；不能将源码或 host vnode 通知等同完整 Linux 事件验收 |
| 已接线 / 待验证 | FUTEX_WAIT_REQUEUE_PI / FUTEX_CMP_REQUEUE_PI | 旧 CLI 对两项仍返回 ENOSYS；新增 arm64-futex-pi 单元及真实线程 guest（单 / 四核）尚未编译执行，不能把接线记为行为通过 |
| 已接线 / 待验证 | RLIMIT_NICE / RTPRIO 变更与 sched_setattr / sched_getattr | 已写版本 / 权限 / 限额 / 继承、FIFO / RR 队列与 CPU 计时、guest yield / interval、PI 队列与单 / 四核派发回归，未编译；`arm64-realtime` 直接生成 ELF，不调用 guest 编译器。尚无新代码执行证据，普通策略仍为按优先级轮转 |
| 未支持 | RLIMIT_MEMLOCK / CORE / MSGQUEUE 的变更与相应未覆盖约束；sched_attr 的 Deadline、自定义运行时间片与利用率控制 | 不仅是开放限额字段；需真实锁页 / 核心转储 / 消息队列约束。当前相关变更或请求明确报错；RSS / LOCKS 按 Linux 保留但不强制的语义另计 |
| 部分实现 / 未支持 | `io_uring`、seccomp、ptrace、setns、unshare 已接分派但仅覆盖受限子集；fanotify 等仍缺路由 | 限制及回归见 [syscall-gaps.md](syscall-gaps.md)；不能把接口接线等同完整异步 I/O、调试器或容器隔离 |
| 未支持 | 尚未覆盖的网络选项 / ancillary 类型 / 错误队列，以及文件系统类型 / flags | 按具体操作列支持 / 错误，不能统一返回成功 |
| 未建立 | 真机开发工作流、性能与长稳交付结论 | 缺设备运行与指标链，不由 host 多核或模拟器编译代替 |
| 接线待冻结 | Interpreter validate_code、proc direct controller、退出 Pending 回收、Memory seal / 锁优化 | 有新增调用体的项已在上表更新；仍需同一版本构建和生命周期复核 |
| 验证待补 | 新 inotify / resources / PI requeue / scheduling guest / RTTIME watchdog、Node watch、消息与文件 API 边界、新 SIMD / FP、iOS 配置与会话回收 | 已写用例、native 对照和语法检查单列；实际运行前均保持待验 |

最新失败的关闭、新通过结果与最终证据路径只更新 [交付状态](delivery-status.md)。[迁移专题](status-and-migration.md) 说明外部 root 与旧数据边界；[历史审计](audit/2026-11-02.md) 保留原时点证据。
