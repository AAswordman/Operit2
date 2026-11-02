# ARM64 开发终端交付状态

**更新日期：2026-11-03（America/New_York）。当前没有统一冻结验收版本。** 最新主线工具结果为 host 构建会话 **6157 退出 0，所有 host 目标成功，包括 CLI 和 API-tests**。这是接线进行中的初步重建；随后修改的 Interpreter / Memory 接线尚待重建回归。最近一次实际完成的完整基础 CTest 仍为 **4/8 通过**，不能把构建成功写成全测通过。

本文件是唯一最新事实记录。[功能矩阵与计算口径](completeness.md) 记录覆盖范围，[文档索引](README.md) 指向各专题与历史审计。源文件存在、任务完成、构建、静态检查、host 对照、guest 执行和模拟器运行分别记录，不相互替代。以下结果来自主线明确工具反馈或只读磁盘核对；文档 owner 未运行构建、测试或验收。

## 本轮最新事实与证据

| 项目 | 最新已知事实 | 来源 / 证据位置 | 尚未证明 |
|---|---|---|---|
| host 初步重建 | `cmake --build tools/ios-runtime/arm64/build/host --parallel 6`，会话 **6157 退出 0**；所有 host 目标成功，包括 CLI / API-tests。旧 47237 的三个 missing symbols 本轮未复现 | 主线最新确切工具结果；6157 对应完整落盘日志路径待补 | 不是最终冻结快照；不能覆盖此后源码变更，也不是测试运行通过 |
| Interpreter / Memory | 主线已实际把 `Interpreter::revalidate` 改为调用 `Memory::validate_code`，包含 page 同步与 epoch；磁盘已有调用体 | `src/cpu/aarch64/interpreter.cpp`、`src/memory/address_space.h`；主线反馈 | 最新改动尚待重建及代码失效回归；Memory seal token / 锁优化未冻结 |
| 完整基础 CTest | 最近一次真正完成的结果仍为 **4/8**；主线正诊断原四项失败 | `tools/ios-runtime/arm64/build/host/Testing/Temporary/LastTest.log`、`LastTestsFailed.log`；主线确认 | 修复后的统一完整重跑结果尚未提供 |
| Worker | 原始 guest **15 秒**条件未改；单核、四核均失败，详见下表 | `tools/ios-runtime/arm64/build/workers-integrated-latest.json` | 没有取得通过或 wall 加速的证据 |
| Worker 争用 | 采样指向 Memory 全局 `recursive_mutex` 争用 | `tools/ios-runtime/arm64/build/workers-contention-sample.txt`；主线反馈及本次只读栈核对 | 是定位线索，不能据此认定优化已生效 |
| proc 统计与生命周期 | 主线此前指出新旧统计路径并存、退出 Pending 清理不全；本次后续磁盘核对已见 `syscall_proc` 直接调用 controller、相关旧 registry API 查不到，并新增 `discard_pending_signals` 及退出调用 | `src/linux/syscall_proc.cpp`、`src/fs/virtual/procfs.*`、`src/runtime/process/{controller.h,observability.cpp,signals.cpp,lifecycle.cpp}` | 仅更新代码事实；Hilbert 最终接口冻结、同版本构建和退出信号额度回收运行仍待确认 |
| inotify | 核心实现与 syscall 已有；匿名 FD 分类现已包含 inotify。guest 文件操作事件桥接未落盘，Darwin OPEN / ACCESS / CLOSE 与虚拟节点不支持 | `src/linux/events/inotify.{h,cpp}`、`src/fs/descriptor_table.cpp` 的 `OpenDescription::is_anon_inode`、`src/linux/files/metadata.cpp`；主线反馈及源码核对 | 分类缺口有源码修复，尚无对应运行结果；新 guest 用例和 Node watcher 未执行，不能写成完整事件覆盖 |
| iOS 外部 root | 配置 UI、目录 bookmark、安全作用域、Rust / Dart 与原位 rootfs 源码接入已有 | 仓库 `apps/flutter/app/ios/Runner/{Arm64RootFilesystem,Arm64TerminalBridge}.mm`、`apps/flutter/app/lib/core/runtime/Arm64RuntimeConfigurationService.dart`、`apps/flutter/app/lib/ui/features/settings/runtime/Arm64RuntimeSettings.dart` 及 App Rust bridge；主线反馈 | 最新完整 Runner / 模拟器运行未完成；静态证据见下节 |
| 当前协作范围 | 仅保留 **Archimedes 文档、Hilbert 现有接线**两名子 agent，其余已关闭。主线负责构建与测试、CMake、Interpreter / Memory 接线及失败测试修复 | 用户最新授权 | 不再按历史任务槽数描述当前并发，也不以任务数量推算覆盖率 |

## 最后一次完整基础 CTest

以下为已完成运行的记录，分母是该轮 **8 个基础 CTest 条目**，不是 24 组功能完成度。后续代码修复没有对应重跑结果时，保留原失败。

| 条目 | 已完成结果 | 已知失败 / 后续状态 |
|---|---|---|
| `arm64-cpu-validation` | 通过 | 限该轮测试范围；新增 ISA 族与编码边界没有因此全部验收 |
| `arm64-filesystem` | 失败 | `SEAL_WRITE ignored an existing writable shared mapping`；修复与回归待主线结果 |
| `arm64-virtual-filesystem` | 失败 | `virtual shared status flags`；断言语义已修正，待重跑 |
| `arm64-network` | 失败 | accept 相关路径超时；已有修复反馈，待重跑 |
| `arm64-tasks` | 通过 | 不外推到新增 PI / 生命周期边界 |
| `arm64-runtime` | 通过 | 该轮日志 22 个内部用例通过，包含多核共享原子同步及真实 / CPU 定时器；不含后来新增的 inotify / resources |
| `arm64-terminal` | 失败 | `terminal foreground assignment requires a guest process-group session hook`；已有修复反馈，待重跑 |
| `arm64-alpine` | 通过 | 限该轮动态 Alpine 与固定助手范围 |

VFS 的修正保留断言：按 Linux v6.12 的 ARM64 定义检查 `O_LARGEFILE = 1 << 17` 不可变位，并检查 `dup` 共享的可变状态。`F_SETFL` 清除可变 flags 后仍须保留该不可变位。依据为上游 v6.12 的 `arch/arm64/include/uapi/asm/fcntl.h`、`fs/open.c` 与 `fs/fcntl.c`；测试入口为 `tests/virtual_filesystem.cpp`。这不是删除断言，也不是已经重跑通过。

## Worker 原始截止条件

证据为 `tools/ios-runtime/arm64/build/workers-integrated-latest.json`，`guest_deadline_ms=15000`。wall / CPU 均取宿主测量字段；平均宿主核为 host CPU / host wall。

| execution_threads | host wall 秒 | host CPU 秒 | 平均宿主核 | maximum_parallel_tasks | 结果 |
|---|---:|---:|---:|---:|---|
| 1 | 17.3143 | 11.5156 | 0.6651 | 1 | `returncode=1`，`passed=false`；guest 原始 15 秒超时 |
| 4 | 26.1383 | 63.0879 | 2.4136 | 4 | `returncode=1`，`passed=false`；guest 原始 15 秒超时 |

报告的 host `timed_out=false` 仅表示宿主 subprocess 没有触发宿主超时；stderr 的 `worker probe timed out` 与返回码仍表明 guest 失败。四核存在并行活动，但 wall 比单核更长，不能称为加速或通过。Memory 锁争用采样不能替代优化后的同条件测量。

## 已新增测试与静态证据

| 入口 | 实际断言 / 覆盖范围 | 已完成证据层级 | 尚待执行 |
|---|---|---|---|
| `tests/guest/inotify.c` → `tests/test_runtime.py` | 真实 create、成对 rename cookie、delete、read、拒绝 pread | 已落盘并接测试入口 | guest 未跑 |
| `tests/guest/resources.c` → `tests/test_runtime.py` | FSIZE / NPROC / SIGPENDING、timer 信号预分配、CPU hard limit 实际终止计算 | 已落盘并接测试入口 | guest 未跑 |
| `tests/node/watch.cjs` | 真实 `fs.watch` 事件、AbortController 关闭与清理 | 主线报告 macOS native Node 对照通过；完整日志路径待补 | guest 未跑 |
| iOS 相关 7 个 Dart 文件 | 配置 UI / service / bootstrap / session 等静态分析 | 主线报告 analyze 无 error、**3 lint**；文件清单 / 日志待补 | 最新 App 构建与运行没有因此通过 |
| `tools/tests/fixtures/arm64_simulator_smoke.mm` | 公开 `arm64RuntimeConfigure`；external root 原位、配置 / bookmark roundtrip、拒绝 live 切换、`closeExistingSessions` 回收、只读 bind 实写失败 | 仅 sim-arm64 Objective-C++ **整单元 `-fsyntax-only` 通过**；输出路径待补 | 最新完整链接与模拟器运行未完成，新增断言待执行 |

## 本轮最终统一证据区（待主线提供）

此区专供同一冻结时点的证据。6157 初步构建及上述历史测试不自动填入最终通过栏。

| 所需记录 | 当前状态 | 待补内容 |
|---|---|---|
| 代码 / 接口冻结范围 | 待确认 | 主线与 Hilbert 同一时点文件和接口状态；构建期间是否继续改动 |
| 最终 host 构建 | 待运行 / 待结果 | 冻结后的命令、退出码、CLI / API-tests 目标、完整日志路径 |
| 完整基础 CTest | 待统一结果 | 本轮实际条目 / 分母、原 8 项基础测试逐项结果、失败输出及代码范围；若启用 ISA 审计另列；新 fixtures 是否实际执行 |
| 新 guest / Node watcher | 待运行 | inotify / resources / watch 的 guest 命令、返回码与日志 |
| Worker 回归 | 待运行 | 原始 15 秒条件、单核 / 四核 host wall / CPU、返回码与 JSON |
| ISA 新增覆盖 | 待结果 | 新族尺寸 / lane / signedness / QC / 保留编码差分范围与报告 |
| iOS Runner / 模拟器 | 待运行 | 完整构建、公开 API 配置场景、会话回收及只读 bind 实际运行结果 |
| 真机 / 长会话 | 尚无本轮证据 | 设备范围、后台 / 恢复、目录授权失效与长会话运行记录 |

## 构建历史与被替代的状态

| 历史尝试 | 当时的结果 | 随后事实与解释 |
|---|---|---|
| procfs helper 冲突 | `process` helper 与 namespace 冲突，构建失败 | 已改名 `process_directory`；不把失败尝试改写为成功 |
| task 枚举缺口 | `lifecycle.cpp` 使用不存在的 `task_ids`，构建失败 | Darwin 已补声明与定义；之后 host 初步重建成功 |
| 源列表先于文件 | CMake 已列 `observability.cpp` 而文件未落盘，构建失败 | 文件随后落盘；不能继续说“文件缺失” |
| 会话 **47237** | **退出 2**。库和 cpu / tasks / network / vfs / filesystem 二进制链接；CLI / API-tests 缺 `Machine::poll_runtime_events()`、`Machine::next_runtime_deadline()`、`Machine::bind_runtime_signal_queues()` | 三个定义随后出现于 `src/runtime/process/timers.cpp` / `signals.cpp`；当时 CLI 不存在。该轮是改动中的混合版本 |
| 会话 **6157** | **退出 0，所有 host 目标成功，包括 CLI / API-tests** | 47237 的三个链接缺口本轮未复现；仍属接线中的初步重建，后续 Interpreter 改动待重建回归 |

历史协作快照曾为 16 个任务槽，其中 7 执行中、9 completed 占槽；随后要求仍写代码的 6 路保存并冻结。它们不是“16 路一直写代码”的证据，也不是当前两名保留子 agent 的状态。旧 **14/24 = 58.3%** 只属于旧基线，不能表示当前实现、接线或验收比例。

## 历史报告与证据解释

机器证据根目录为仓库 `tools/ios-runtime/arm64/build/`。可变报告文件可能被后续运行覆盖；没有适用代码范围的新报告，不能为当前源码背书。

| 历史证据 | 已记录范围 | 解释限制 |
|---|---|---|
| `host/isa-audit.json` | 5251 个历史样本通过，`profile_complete=false` | 不是完整 ISA，也不代表后来新增指令族通过 |
| `development-latest.json` | 历史 5/5 工作流 | npm 是本地 fixture 的 `npm install --offline`；Python 是 venv / SQLite / `pip --version`，不等于在线安装包 |
| `python-probe.json` | 历史 8/8 探针 | 不覆盖全部 Python 包 / 原生扩展 |
| `node-probe.json` | 历史基础、线程池、网络通过，Worker 失败，整体 `passed=false` | 最新 Worker 定量记录优先使用上文 integrated 报告 |
| `feature-probe.json` | 历史 PTY / signals 探针 | 不覆盖最新 terminal 失败与修复 |
| `simulator-smoke.log`、`ios-app-test.log` | 历史 bridge / App 探针，旧 Runner 有 cancellation-keep-session 失败记录 | 不能宣称最新完整 App、取消后复用或新版 harness 已通过 |
| [2026-11-02 审计](audit/2026-11-02.md) | 当时审计与原始证据 | 保留历史，不作为当前覆盖率 |

仍待主线补齐的关键事实是：冻结范围、最终 host 与完整 CTest 结果、proc direct controller / 旧 registry 收敛与退出 Pending 额度释放的统一回归、Memory / seal 优化后的回归，以及 guest watcher / 资源限制与新版模拟器完整运行。未知项保留“待确认”，不折算新的完成百分比。
