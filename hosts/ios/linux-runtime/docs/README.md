# 文档索引

本目录按“最新事实、功能覆盖、专题说明、历史记录”组织。阅读入口为 [运行时 README](../README.md)。

| 文档 | 用途 | 更新规则 |
|---|---|---|
| [delivery-status.md](delivery-status.md) | **唯一最新事实记录**：构建、测试、Worker、iOS 证据及尚待确认项 | 新结果写明命令或来源、适用代码范围和证据路径；保留被替代的失败记录 |
| [completeness.md](completeness.md) | 可核对的 24 组功能矩阵，以及实现、接线、验证的独立统计口径 | 以实际实现体、构建登记、生产调用链和行为证据分列；不复制最新测试汇总 |
| [architecture.md](architecture.md) | 执行层、内存、任务、Linux ABI 和公开 API 的结构与边界 | 描述代码设计；当前集成缺口与验收结果链接到交付状态 |
| [static-execution.md](static-execution.md) | CFG、静态汇编 ABI、跨图角色桥接、精确状态和失效 | 设计与当前测试结果分列；macOS 结果不代表 iOS 设备 |
| [performance/2026-11-06-multiagent.md](performance/2026-11-06-multiagent.md) | 多 agent 采样、动态计数、硬件计数和原始 Python 交替复测 | 诊断与验收计时分开；保留失败实验和真实 0.5 秒差距 |
| [performance/2026-11-06-static-executor.md](performance/2026-11-06-static-executor.md) | 静态汇编后端同构建对照、默认关闭加速的 Python 结果及 0.5 秒差距 | 分开记录热循环、完整进程、正确性和交叉构建 |
| [performance/2026-11-05-code-pages.md](performance/2026-11-05-code-pages.md) | 旧区域后端的编译、共享代码页和验证缓存历史实测 | 独立秒数、正反序样本、原样 Python 与完整回归 |
| [memory-compute-audit.md](memory-compute-audit.md) | Python 计算阶段调用栈、拷贝计数及优化实测 | 区分运行时路径占比、客体指令量与用户数学成本 |
| [node-probe.md](node-probe.md) | Node 探针方法、原始 Worker 截止条件和报告解释 | 区分 host 对照、guest 行为、计时条件和历史结果 |
| [network.md](network.md) | socket、DNS、poll/epoll 与宿主网络边界 | 记录支持范围、调用路径与未支持语义 |
| [virtual-filesystems.md](virtual-filesystems.md) | root、绑定挂载、proc/dev/devpts、虚拟 FD 与 PTY | 记录文件系统模型与限制 |
| [status-and-migration.md](status-and-migration.md) | 外部 root、旧 x86 数据迁移边界与历史性能记录 | 不再作为当前状态总表；最新结果转到交付状态 |
| [audit/2026-11-02.md](audit/2026-11-02.md) | 2026-11-02 历史审计及对应机器证据 | 保留原始记录，不用历史比例代表当前覆盖 |

源码、测试入口和机器报告可以证明不同层级的事实。新增实现不等于接通生命周期，构建成功不等于测试通过，native 对照或语法检查不等于 guest / 模拟器运行。各专题如与当前结果有出入，以 [delivery-status.md](delivery-status.md) 所注明的时间和证据范围为准。

路径约定：文中以 `src/`、`tests/` 开头的路径相对 `hosts/ios/linux-runtime/`；以 `tools/`、`apps/` 开头的路径相对仓库根目录。机器报告的可变文件名（例如 `*-latest.json`）本身不标识冻结版本，需结合交付状态中的运行来源阅读。
