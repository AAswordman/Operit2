# DSL 会话提交顺序与加载性能：2026-10-10

## 提交乱序的根因和修改

日志中的 `Compose update revision is out of order: 16, expected 15` 来自两条交付路径：中间提交由 JS 原生回调直接发送，最终提交则等待 Rust 异步命令任务取走返回值后才发送。前一条命令的最终提交尚未消费时，后一条命令已经能发送更高版本的中间提交。单个 action 内部排队不能保证跨命令的提交顺序。

现在会话的所有响应在 JS 创建提交时通过同一个、由当前调用持有的结构化回调发布；Rust 异步命令返回值仅完成命令，不重复发送最终 UI 响应。分离定时器继续持有原 action 的回调，不借用后来 render 的调用身份。

CoreLink、Flutter 连续版本校验和现有插件 JS 写法保持原有契约。没有添加丢弃版本、跳版本、重置整树或失败后的替代路径。

确定性 Node 用例在修改前收到第 4 次更新时仍期望第 3 次，日志保存在 `target/compose-session-order-red.log`。修改后通过。

## 启用状态统一

生命周期注册和主脚本读取已允许通过启用子包使容器活跃。JS 执行身份校验和 Chat 扩展身份校验此前使用通用 `isPackageEnabled`，只认直接启用的容器，导致 `daily_life` 子包有效时 `com.operit.daily_life` 被误报停用。

两处身份校验现在读取与生命周期相同的 `getEnabledToolPkgContainerRuntimes`，仍要求真实注册的容器，并拒绝不在活跃列表的所有者。SDK 用例验证直接启用、通过子包启用，以及空列表和未登记子包不能启用容器。

## 用户日志的可确认耗时

来源：附件 `61b823a2-40bc-4aa7-93f0-1cc2f9dccd34/已粘贴的文本.txt`。

| 阶段 | 日志记录 |
| --- | --- |
| 角色卡生命周期回调 | 28 次，中位数 397.5 ms，范围 382–2157 ms |
| 侧栏目录 IPC | 551 ms |
| 编辑器两个数据 IPC | 1469 ms、1910 ms |
| 三次实际二进制文件读取 | 90 ms、13 ms、69 ms |

回调通知与 IPC 存在重叠，不能把全部耗时直接相加当成页面加载时间。现有日志没有完整的 DSL 请求开始、HTML ready 和 Flutter 首帧标记，因此不能得出整个页面从点击到可交互的精确耗时。解析结果保存在 `target/compose-log-performance-breakdown.json`。

## 反复调用的已修复热点

模块实例缓存命中前，每次仍会在 QuickJS 中逐字符遍历完整脚本，执行 `__operitHashText`。角色卡 `dist/main.js` 为 441898 字节；其 `toolCallPolicy` 对多数生命周期通知立即返回，但缓存寻址仍先付出整份源码的解释器遍历代价。

通过真实原生 Host 调用一个与主脚本同尺寸的空操作模块，保留模块缓存、源文本传输和函数调度，单独记录哈希耗时：

| 热调用指标 | 修改前 | 修改后 |
| --- | --- | --- |
| 5 次总耗时中位数 | 387.351 ms | 11.272 ms |
| 总耗时范围 | 382.056–394.787 ms | 10.953–11.883 ms |
| 哈希耗时范围 | 379–393 ms | 9–10 ms |

现在通过既有 Host 回调注册机制在 Rust 计算同一个 UTF-16、32 位 wrapping 哈希，再返回相同缓存键。没有增加平台分支，也没有改变缓存失效规则。实测该单条调用路径约快 34.4 倍，不能将此倍率套用到整页加载。

功能用例覆盖空字符串、ASCII、中文、补充平面字符、整数溢出，以及同长度脚本变化后再次回到原脚本的缓存身份。

修改前后的原始记录分别为 `target/compose-native-package-performance-before.log`、`target/compose-native-package-performance-after.log`；样本及比值为 `target/compose-native-package-performance-comparison.json`。

## 首次 DSL 编译仍然昂贵

对真实打包角色卡编辑器执行分阶段原生测量，不打开 WebView、不读取用户数据库：

| 阶段 | 一次本机 debug 测量 |
| --- | --- |
| 初始化 JS 环境并安装观察器 | 62.765 ms |
| 脚本到首次 retained commit | 562.492 ms |
| 其中 DSL 编译 | 552 ms |
| 初次 JS render 与 commit | 各不足 1 ms 的计时分辨率 |
| 同上下文输入刷新 | 2.455、2.514、2.775 ms |

三次刷新后编译调用次数仍为 1。日志在 `target/compose-native-editor-stage-performance.log`。这组测量与真实 UI 加载不等价：后续 HTML 文件读取、IPC 数据请求、WebView 内容绘制和 Flutter 首帧均未计入。

下一项可确认的冷启动优化对象是 `OperitComposeCompiler.compile` 内的首次 Acorn 解析和转换，而不是 retained commit 的 UI 树扫描。反复调用仍需要在原生代码遍历源文本；本次将其从约 390 ms 降到了约 10 ms，没有消除所有源码相关开销。

## 验证

| 范围 | 结果 | 日志 |
| --- | --- | --- |
| 真实 Rust 会话、并发 action、实际角色卡 WebView 回调、分离定时器、无渲染 action、UTF-16 指纹和性能探针 | 6/6 | `target/compose-native-session-order-tests.log` |
| 原生编辑器首次加载与输入刷新阶段探针 | 1/1 | `target/compose-native-editor-stage-performance.log` |
| 原生 JS bridge 全套 | 74/74 | `target/compose-stream-jsbridge-tests.log` |
| 插件 SDK 全套 | 72/72 | `target/compose-stream-sdk-tests.log` |
| DSL 状态、导航、WebView、真实插件屏幕 | 27/27 | `target/compose-stream-node-tests.log` |
| Flutter 侧栏、错误生命周期及 DSL 布局回归 | 94/94 | `target/compose-stream-flutter-tests.log` |
| Rust runtime 及测试检查 | 通过 | `target/compose-stream-runtime-check.log` |
| 独立 SDK 嵌入示例检查 | 通过 | `target/compose-stream-sdk-example-check.log` |

Rust 测试与检查使用 `RUSTFLAGS=-Awarnings`；Flutter 命令在应用目录通过 FVM 运行。Flutter 回归在提交协议修改后运行；随后哈希与启用判断改动重跑原生桥接、SDK 和 runtime 检查。没有执行整套 Flutter 应用打包或人工端到端操作。运行中的应用需要下次构建并重启，才能实际执行新的 Rust 回调和身份校验。

## 会话调用链清理与首次编译优化

DSL 现在只有 retained 会话响应协议。删除了流交付与快照交付的两个模式开关，删除整树、state、memo 返回分支和异步完成时再次交付 UI 的旧分支。每次 render/action 都必须携带自己的响应发送接口；缺失接口直接报错。外部独立 rerender 入口也已删除，输入刷新由 render 命令处理，内部刷新函数继续保留同一 JS 会话。

Rust 执行接口仅保留带必需响应回调的异步 render/action。已删除同步 DSL 入口、返回快照的异步入口、旧 action event stream、旧事件封装函数及 recursive UI tree parser。桌面小部件通过同一个会话的 stream/reverse stream 完成 render 与 onLoad，最终传出 flat node update；CLI 使用现有自动代理建立持久上下行流，直接消费 typed events。CoreLink 源码和协议均未改动，插件原有 JS API 与写法保持不变。

CLI 的输入初值仅在首次 render 传入；后续输入通过明确的输入更新传入，不回传 JS 所拥有的 state/memo。持久流的存在不再被视为 action 忙碌，交互禁用条件只取未完成命令。已完成的命令发送 future 不会再次被轮询。

首次编译仅解析插件源码；内部 DSL runtime 在编译后拼入主模块，依赖模块不注入入口 runtime。普通模块与 DSL 模块的实例缓存键显式区分。AST 的直接子节点列表用 WeakMap 复用；源码重建只拼接含实际 DSL 改写的范围，未改写子树不逐层构建源码字符串。对角色卡、计划模式、工作流及 message_insert 的 46 份实际 JavaScript 模块验证，改写后的编译输出逐字一致。现有测试需要查看树或 JS 存储时，由测试辅助代码投影真实的 stream 响应；生产代码没有为测试保留快照模式。

| 原生编辑器阶段 | 优化前 | 当前实测 |
| --- | --- | --- |
| 脚本执行至首次 commit | 572.280 ms | 428.862–446.185 ms |
| Acorn parse | 432 ms | 305–323 ms |
| 编译总耗时 | 563 ms | 420–436 ms |
| 同上下文输入刷新 | 2.400–2.757 ms | 1.749–2.199 ms |

这是本机 debug 原生 Host 的阶段测量，冷阶段约减少 22–25%。新增的稀疏源码重建已验证输出一致；单次冷测量的波动不足以单独量化它的加速比例。剩余首次耗时主要在解释器里的 Acorn parse，不能由此宣称整个页面点击至可交互只需要约 440 ms。引擎 bootstrap 也单独测量，最终样本为 62.510 ms，未包含在脚本执行至 commit 一项。数据 IPC、HTML 资源、WebView 与 Flutter 首帧均未包含在本表。最终代码复测为 446.185 ms，日志见 `target/compose-single-stream-editor-performance-latest.log`。

局部生成探针在 20/100/500 行的可分离同步屏幕上验证：单状态编辑的根函数重执行为 0、表达式执行为 2、retained 子节点检查为 0。这个结果不能推广为任意包含副作用、异步入口或插件自行循环的屏幕都能完全局部重执行。

定时器回归原先把脚本初始化也算进固定 100 ms 阈值，无法区分初始化及调度耗时与真正的 Host 阻塞。此前失败时定时器已赢得竞速，失败的是整段耗时断言。测试改为现有受控 Host 调用：工具等待显式释放，JS 定时器必须先返回；释放后还需收到工具完成事件。最终串行全套原生测试通过，测试没有放宽阻塞行为要求。

最终验证记录：

- 原生 JS bridge 74 项与 SDK 72 项：`target/compose-single-stream-rust-final.log`。
- retained store 与真实原生会话 6 项：`target/compose-single-stream-sessions-final.log`；2 个性能探针按需单独运行。
- DSL/工作流/计划模式/角色卡/宿主边界 Node 93 项：`target/compose-single-stream-node-final.log`。
- CLI Compose 5 项：`target/compose-single-stream-cli-tests-final.log`。
- Flutter launcher、typed action、桌面小部件、错误生命周期与首帧探针：109 项通过、1 项跳过；`target/compose-single-stream-flutter.log`。
- Flutter lazy viewport、侧栏、动作调度、WebView、布局及附件注册回归：58 项通过；`target/compose-single-stream-flutter-regression.log`。
- 自动代理检查与独立 SDK 嵌入示例检查通过：`target/compose-single-stream-proxy-check-final.log`、`target/compose-single-stream-sdk-example-check.log`。
- 编译输出等价及 dirty scope 探针：`target/compose-sparse-emission-equivalence.log`、`target/compose-single-stream-retained-final.log`。

`file_selector_ios` 的分析报错来自该独立 vendored 包缺少 package_config。已生成其本地依赖解析，并通过 pubspec_overrides 对齐应用使用的 vendored platform_interface 与 image_picker_ios；单文件分析结果为 No issues found，没有修改 picker 的业务实现。其 example 的无关 lockfile 更新已撤销。
