# 全软件日志分级规范

本规范适用于 Rust runtime、Flutter 客户端、Host 兼容层和 QuickJS 插件。严重程度与诊断深度是两个不同维度：Info/Warn/Error 表达运行结果，V1～V6 表达排查时展开多少细节。数字越大，细节越多；最大 Verbose 等级为 N 时显示 V1～VN。

## 严重程度

| 等级 | 使用范围 |
| --- | --- |
| Error | 操作失败，记录原因及必要上下文；不得改成 Verbose 来隐藏问题。 |
| Warn | 需要关注的异常、被拒绝的操作、协议或配置问题。 |
| Info | 对运行有用的摘要：启动完成、发送接受与结果、实际工具执行结果、导入完成、监听地址或配置发生实际变更。 |
| Debug | 开发者显式输出的临时调试信息；常驻过程追踪必须按下表选择 Verbose 深度。 |
| Verbose | 可按深度展开的诊断细节，不用于承载失败和告警。 |

Info 不记录每个函数的进入/退出、周期轮询、无变化通知、流订阅挂接、逐块数据或一项操作的多份重复摘要。长时间用户操作可以保留一条开始与一条结果，普通短操作通常保留结果即可。

## Verbose 深度

| 等级 | 含义 | 示例 |
| --- | --- | --- |
| V1 | 进程和配置生命周期 | 启动阶段、存储配置应用、后台服务启动步骤。 |
| V2 | 用户操作步骤与汇总计时 | 发送处理阶段、聊天切换、取消、工具结果后续轮次、一次操作的阶段耗时。 |
| V3 | 服务和依赖调用边界 | HTTP 请求/响应阶段、插件 hook/API、工具调用、MCP/设备通信；记录摘要而非完整载荷。 |
| V4 | 组件、会话和资源生命周期 | 插件注册、会话和控制器挂接/释放、侧栏准备、Compose 加载/首帧、资源池调整。 |
| V5 | 内部调度和状态流转 | 路由选择、订阅和 Promise 调度、同步周期、存储流程、缓存命中与状态传播。 |
| V6 | 逐项、高频及完整载荷细节 | 逐 chunk/delta、逐条同步或导入、每次流更新、完整参数/结果、逐节点布局和原始 JS 执行跟踪。 |

按事件本身选择深度，不按整个模块统一归档。例如，同一发送模块的操作阶段是 V2，插件调用是 V3，异步任务调度是 V5，逐块内容是 V6。重复内容应删除，不应依靠最高等级保留所有重复记录。敏感字段的现有保护要求同样适用于 V6。

## 统一入口

Rust：`AppLogger::v_with_level(tag, message, VERBOSE_LEVEL_N)`；结构化事件使用 `ChainLogger::verbose_with_level(tag, event, fields, VERBOSE_LEVEL_N)`。`AppLogger::v` 为 V1，`AppLogger::trace` 和 `ChainLogger::verbose` 为 V5，`AppLogger::trace_detail` 为 V6。

Flutter：`ClientLogger.v(message, verboseLevel: VerboseLevel.levelN, tag: tag)`。新日志明确填写 `verboseLevel`，避免默认值掩盖分类。枚举与 Rust 常量的数字和含义一致。

QuickJS：`console.verbose(level, ...data)`，其中 level 必须是整数 1～6。`console.debug` 对应 Debug；`console.log`/`console.info` 对应 Info，`console.warn`/`console.error` 保留其严重程度。插件作者也应遵守同一张分级表。

Host 层沿用调用方传入的标准严重程度，不按平台另建等级体系。

## 查看范围与文件日志

控制台默认显示 Info 及以上。排查时先启用 Verbose，再选择最大深度：V2 看用户流程，V3 追踪依赖调用，V5 追踪内部流转，V6 查看逐项细节。

Rust 使用 `set_console_min_level(VERBOSE)` 与 `set_console_max_verbose_level(N)`；Flutter 使用 `setConsoleMinimumLevel(ClientLogLevel.verbose)` 与 `setConsoleMaximumVerboseLevel(VerboseLevel.levelN)`。

当前控制台阈值不筛除持久化日志：文件和内存仍保留详细诊断。把 Info 改成 Verbose 会减少默认控制台噪声，但不代表省去了该条日志的格式化和文件写入成本。涉及日志性能的改动必须另外验证实际写入量和耗时，不能把分级调整当作性能测试结果。
