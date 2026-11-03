# Root、虚拟文件系统与终端节点

本页描述命名空间与节点模型；当前失败、修复和验收证据见 [交付状态](delivery-status.md)，逐组覆盖见 [功能矩阵](completeness.md)。

## Root 与绑定挂载

`src/fs/filesystem.*`、`src/fs/mount/` 以已打开的宿主 FD 锚定 root 和挂载源。路径遍历统一处理 `.`、`..`、symlink、虚拟节点与挂载边界；guest 绝对 symlink 回到 guest 根，不能将宿主路径直接暴露给 guest。

调用方可提供外部 root；运行时不要求其带制备工具标记，也不要求宿主 mount / chroot 权限，访问能力来自宿主 App 的目录授权。公开 `ort_arm64_bind` 在 load 前登记目录绑定，CLI 提供读写 / 只读选项。底层文件系统包含 bind / remount、proc / dev / devpts / tmpfs 与 umount 实现，但当前 guest capability 集合为空，因此 guest 的 `mount` / `umount2` 必须返回 `EPERM`，不能修改共享挂载树。运行时初始化的虚拟节点和宿主侧绑定不经过 guest syscall 权限检查；后续开放 guest 挂载必须先实现相应用户及挂载命名空间的真实权限模型。

`DescriptorTable` 的数字 FD 与共享 `OpenDescription` 分开。dup / fork 共享 offset 与可变 file status，FD_CLOEXEC 属于各自数字 FD。只读挂载约束不只影响 open，还需覆盖写、metadata、共享映射和空间分配。

## 虚拟节点与设备

`src/fs/virtual/` 提供不依赖普通宿主文件的节点与打开对象。VM 构造时挂入 `/dev`、`/proc`，不需要向外部 root 写静态 proc 内容，也不依赖 root 中预置设备文件。虚拟目录和普通目录走同组 open / stat / readlink / getdents / lseek ABI。

- `/dev/null` 读 EOF、写丢弃；`zero` 读零并支持对应映射。
- `/dev/full` 非空写报 `ENOSPC`；random / urandom 使用宿主安全随机源。
- `/dev/fd`、stdin / stdout / stderr 引用 guest FD，继承流保持回调身份。
- PTY、devpts、termios、窗口大小与前台组由 `linux/terminal/` 和 `runtime/console/` 实现，经公开 terminal API 驱动；存在实现不等于所有交互 / 作业控制场景已验收。

字符设备使用 Linux major / minor。节点元数据与数据写入是不同操作，虚拟设备不代表完整 Linux `/dev`。

## 动态 proc 视图

`procfs.cpp` 构建节点，`syscall_proc.cpp` 通过 Scheduler / ProcessController / Memory 提供状态。已有多进程表，不能再把 `/proc/1` 当作唯一进程模型。

| 节点 | 数据来源与语义 |
|---|---|
| `self`、`thread-self`、进程目录、`task/<tid>` | guest 当前身份、进程与存活任务列表 |
| `exe`、`cwd`、`root` | guest 命名空间路径 |
| `cmdline`、`environ`、`auxv` | 按记录的 guest 内存边界逐次范围读取；可观察 argv 内存变化，不统一视为打开时静态快照 |
| `status`、`comm`、`stat` | guest 任务 / 信号 / 内存 / affinity、名称与进程计账；`comm` 支持更新，stat 经 controller 查询 |
| `fd`、`fdinfo` | 当前 FD 链接、offset 和 flags；对象身份来自共享打开描述 |
| `maps`、`statm`、`meminfo` | 实际映射、保护、来源、已物化页与内存限制 |
| `mounts` | root、绑定源与虚拟挂载列表 |

一般 seq 内容与动态范围读取采用不同路径；不能由一种节点的缓存策略推断全部 proc 行为。CPU / 子进程 / fault / I/O 计数必须有实际来源，不能把默认零值当准确统计。proc direct controller 与退出 / PID 复用的最新接线观察及回归范围见交付状态。

## 匿名 FD 与事件边界

Darwin 的 `O_TMPFILE` 只支持带 `O_EXCL`、不允许后续链接落盘的匿名临时文件。不带 `O_EXCL` 的请求在 open 阶段返回 guest `EOPNOTSUPP`，让 apk 等程序使用普通临时文件加 rename 的回退流程；不能先成功创建匿名文件，再在下载完成后的 `/proc/self/fd` 链接阶段失败。Linux 宿主仍使用原生 `O_TMPFILE`，并保留 `O_EXCL`。回归覆盖见 `tests/filesystem.cpp` 与 `tests/test_alpine.py` 的本地仓库索引更新用例；本次 Apple 源码修复待重新编译验证。

eventfd、epoll、timerfd、inotify 等通过共享打开对象保存状态。`OpenDescription::is_anon_inode` 为 metadata 提供公共分类，当前包含 inotify。这类 FD 的 read / poll / ioctl / seek 需要按各自 ABI 检查，不能套用普通文件 offset。

Darwin inotify 使用 kqueue / vnode 与目录快照观察结构和元数据变化，并通过 `InotifyOpenFile` 桥接 guest 的 OPEN / ACCESS / CLOSE。共享 OpenDescription 持有一次 data-open；dup / fork / SCM_RIGHTS 共用该引用，mmap 的 FileBacking 也保留它，最后引用释放才产生 CLOSE。普通 / 定位 / 向量读取、getdents 与文件传输接入 ACCESS；事件按 pinned parent inode 与确切名字归属，rename / unlink 更新该归属，避免把事件发给另一条硬链接；IN_EXCL_UNLINK、one-shot 和未链接 inode 的 CLOSE → DELETE_SELF → IGNORED 顺序均有新增用例。

新桥接尚未编译或运行。虚拟节点 watch、exec 装载路径的 data-open，以及匿名 O_TMPFILE 的父目录临时名字事件仍未接；完全由宿主外部进程执行的 OPEN / ACCESS / CLOSE 没有 kqueue 事件源，不能据此宣称覆盖。新增 `arm64-inotify`、guest fixture 与 Node watcher 的实际运行层级只记在交付状态。

回归入口包括 `tests/virtual_filesystem.cpp`、`tests/filesystem.cpp`、`tests/python/virtual_filesystem.py` 与新增 `tests/guest/inotify.c`。入口列出不表示本轮已执行。
