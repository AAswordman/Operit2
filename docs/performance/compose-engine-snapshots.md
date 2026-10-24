# Compose 引擎优化与侧栏实测（2026-10-18）

说明：下文原生/Profile 性能数字记录的是各轮当时的实现，其中窗口缓存曾使用显式依赖选项。该选项现已删除，替代机制与本轮验证范围见文末；旧数字不作为新机制已经复测的证据。

前一轮结果：重复构树和提交已经降到约 0.39ms；真实侧栏挂载中，排除显式 Flutter 帧等待的命令耗时降低约 39–44%。整条链路没有达到十倍，首次节点构建和目录加载仍有实际成本。Dart 消息解码和节点应用不是主要瓶颈。

## 测量口径

- 使用真实 `NativeHostJavaScriptRuntimeHost`、QuickJS、CoreLink、插件存储及 macOS Profile 侧栏宿主；使用独立测试数据，保留用户正在运行的 Debug 应用。
- 应用测试为 20 个角色、20 个聊天，每场景 8 次挂载，排除首轮，对后续 7 次取中位数。原生同进程 A/B 交替 AB / BA，排除首轮，保留 14 对样本。
- 命令耗时按 render/action 区间求并集，包含执行、传输和调度，排除命令之间的显式 Flutter 帧等待。不能把互相包含的 JS、IPC、命令阶段直接相加。
- 图片读取和渲染另计。诊断捕获的约 1.8MB 回包是头像的 `BinaryFileContentData/contentBase64`，不是 DSL 节点传输。
- JS `Date.now()` 和命令日志只有毫秒精度；原生 A/B 使用 Rust `Instant`。内容就绪仍包含帧及观察时机，不能当作纯引擎时间。

## 最新真实应用结果

以下最终候选包含模板、字节码缓存、动作合批、列表区间修正、窗口缓存及 memo 生命周期修正。同一候选二进制复跑，没有重新编译。额外的成功 IPC 细分日志在另一次诊断运行采集，不混进这张表。

| 指标：后续样本中位数 | 最初基线 | 候选首次运行 | 同一二进制复跑 |
| --- | ---: | ---: | ---: |
| 预加载侧栏命令区间并集 | 47.5ms | 31.0ms | 29.1ms |
| 无准备侧栏命令区间并集 | 56.5ms | 33.9ms | 31.5ms |
| 预加载侧栏 JS render 累计 | 34ms | 10ms | 9ms |
| 无准备侧栏 JS render 累计 | 30ms | 12ms | 10ms |
| 预加载侧栏 retained commit 累计 | 7ms | 5ms | 4ms |
| 无准备侧栏 retained commit 累计 | 5ms | 4ms | 3ms |
| 预加载侧栏 render / 回包次数 | 5 / 5 | 3 / 2 | 3 / 2 |
| 无准备侧栏 render / 回包次数 | 6 / 6 | 4 / 3 | 4 / 3 |
| 预加载侧栏 Dart 解码 / 节点应用累计 | 0.498 / 0.248ms | 0.407 / 0.209ms | 0.410 / 0.209ms |
| 无准备侧栏 Dart 解码 / 节点应用累计 | 0.643 / 0.356ms | 0.454 / 0.257ms | 0.451 / 0.261ms |
| 预加载侧栏内容就绪 | 101.994ms | 100.859ms | 101.515ms |
| 无准备侧栏内容就绪 | 100.553ms | 87.487ms | 102.213ms |

内容就绪的波动没有随命令工作量同比降低，因此没有宣称视觉加载已达到相同比例提速。命令区间、执行次数和回包数量的变化更稳定。

原始数据：`/tmp/operit-sidebar-engine-before-20261018/`、`/tmp/operit-sidebar-engine-owned-memo-20261018/`、`/tmp/operit-sidebar-engine-owned-memo-repeat-20261018/` 及相邻 `.log`。每目录包含 `application.json`、`analysis.json`、`command-union.json`，后者合并重叠命令。

## 引擎改动及所有权

### 参数与模板

`Generation.js` 在一次遍历中比较并生成快照，只为变化的容器分配副本。对象原地修改仍会被检查，不把插件可变参数直接作为旧回包。回调通过 WeakMap 闭包单元更新 handler，挂载 action ID 仍只归 retained graph 管理。

编译器把固定 UI 参数结构提升为模块级模板，调用处传动态值向量。带可观察枚举、访问器、重复属性、计算属性和参数展开的写法保留原有求值语义；v2 artifact 使用模板，v1 继续执行。主题颜色 `copy()` 是不可枚举的本地方法，不再误注册为 UI 回调。

### 模块字节码

宿主缓存精确源码及参数名对应的 QuickJS 字节码，最多 128 项 / 16MiB。每页在自己的 Realm 中创建函数并初始化模块，不跨页共享 JS 对象、exports 或回调。编译失败不入缓存，编译工作在全局 LRU 锁外进行；并发插入二次检查，避免重复计费。原生和 Web/WASM 使用相同边界。

### 提交与动作调度

中间通知只消费尚未处理的 reactive 工作；异步动作开始等待、完成以及宿主显式渲染仍观察普通 JS 变量、ref 和 mutable hook。即将结束的动作接管尚未开始的中间工作，已经开始的提交必须先完成。

无变化中间提交仍完成对账、回调更新和清理，但不分配 revision、不发送空回包。最终响应始终保留，发布的 revision 连续；错误不转成成功结果，detached 工作仍通过原拥有者报告。

### memo 与 lazy 窗口

`useMemo` 工厂现在同时拥有独立 generation region 和 reactive execution unit。跳过缓存工厂时保留它的节点编号、订阅和 effects；普通兄弟节点不能占用缓存工厂的编号或回调单元。分支移除会清理订阅和 effects，节点缓存重挂时重新执行工厂。源码替换会刷新生成节点的 memo，普通数据 memo 保持既有 keyed identity。

回归用例证明并修正了两类实际错误：跳过 memo 后兄弟节点覆写缓存行的标签/闭包，以及缓存子组件的编译 frame 清除父订阅、下一次父渲染又释放子订阅。另覆盖依赖变化时复用全部节点后再热重载、直接 `ctx.h`、缓存嵌套、effect 清理和重挂。

`lazyItems` 的显式 builder 依赖选项已删除。运行时区分 SDK 视口回调与普通业务回调，使用已有重组调度和内部内容 revision 管理窗口复用；见文末。角色卡不再维护布局依赖列表，稳定 action 表仍在每次根执行时更新。

### 视口区间

嵌入式 sliver 的区间已经包含 Flutter cache extent，DSL 再追加完整缓存会触发多余 range action。现在这条路径只留小幅补量，普通列表仍保留 8 项预取和 2 项 refill。600px 视口、2 个静态前缀和 24 行初始数据的真实 Widget 回归中，旧代码无必要地请求 `[0,31)`，新代码不请求；滚动 200px 后仍按需补量。预加载侧栏因此少一次根执行和一次回包。

## 原生 QJS 对照及限制

| 同进程对照 | 修改前 | 修改后 | 收益 |
| --- | ---: | ---: | ---: |
| 初期参数快照探针，每轮 4000 次 | 99.975ms | 66.078ms | 约 1.51 倍 |
| UI 模板：真实侧栏重复构树加提交 | 4.546ms | 2.166ms | 约 2.10 倍 |
| 新页面模块构造：源码编译 / 字节码命中 | 6.7049ms | 0.8046ms | 约 8.33 倍 |
| 最终窗口缓存：真实侧栏重复构树加提交 | 2.1082ms | 0.3927ms | 约 5.37 倍 |
| 同次窗口缓存 A/B 的首次构树加提交 | 11.0105ms | 12.3462ms | 首次增加约 1.34ms |

最后两行使用同一新插件源码、不同共享 SDK，在原生 QJS 中处理 388 个节点。新缓存减少后续重复工作，首次仍要建立节点和缓存所有权，不能把重复场景的倍数当作首次加载收益。最早 4.546ms 与当前 0.3927ms 可作累计历史参考，整条命令链路目前仍约 1.6–1.8 倍。

诊断中尝试跳过 normalizer 复制，没有观察到改善；没有把这种不安全的共享可变属性方式加入生产代码。当前侧栏仍是 opaque root，不能为省一次执行而忽略普通 JS/ref 更新。

原始 native A/B：`/tmp/operit-compose-native-ab-20261018/`、`/tmp/operit-compose-lazy-memo-ab-20261018/`；最后目录的 `result-final.json`、`summary-final.json` 对应最终 memo 所有权实现。

## 验证与剩余成本

共享引擎与 JS bridge Node 回归 99 项通过；原生 Compose 17 项通过、1 项显式性能探针忽略。SDK declaration consistency check 通过。角色卡严格 TypeScript 检查、实际包构建及 Profile 集成测试通过。

最后一次打包结束后，Flutter sliver/lazy 回归 20 项通过；角色卡全套 499 项中 497 项通过，生产归档和打包一致性检查通过。剩余两项 `entity-native.test.mjs` 失败分别是 provider 预期和模型选择键预期，之前未预编译源码也可复现。本轮未修改实体编辑业务以适配这些断言。

日志：`/tmp/operit-compose-memo-reactive-final-20261018.log`、`/tmp/operit-compose-final-native-20261018.log`、`/tmp/operit-compose-lazy-final-flutter-20261018.log`、`/tmp/operit-character-final-validated-20261018.log`、`/tmp/operit-compose-final-codegen-check-20261018.log`。

剩余主要为首次节点创建/retained graph 建立、目录 IPC 中的真实业务读取，以及仍需保留的生命周期求值。头像和 Flutter 帧等待单列。开启 `OPERIT_PROFILE_COMPOSE_STAGES` 可额外输出 IPC target、JS execution、tool 的分段日志；默认不生成成功诊断，不为未启用的 target 计时分配 trace ID 或 phase 缓冲。


## Prepared 首次树与 onLoad

Prepared 目录存在时，首棵树已经具备完整角色内容，但旧流程仍让 `onLoad` 同步等待一次相同目录 IPC。现在首次 `onLoad` 只把刷新放到当前 action 完成后的 microtask，由 detached 生命周期继续执行；首树可以先交付，权威目录结果回来后仍会更新界面。无 prepared 数据的路径保持同步加载和错误传播。

现有 sidebar-native 回归覆盖 prepared 树先出现、IPC 挂起期间默认头像继续异步补齐、权威空目录替换 prepared 内容，以及不同聊天历史拒绝旧 prepared 数据。Profile 中 prepared 样本的 onLoad action 已降到约 2–4ms 级别；同一最终二进制复跑的预加载命令并集为 23.2ms，无准备为 32.9ms。完整命令并集仍受首次 388 节点构建和 retained commit 主导，不能把 onLoad 单项收益直接当作整页倍数。

## 目录读取缓存

进一步检查发现，`character-sidebar.catalog` 的一次冷目录读取会同时涉及 canonical metadata、cards/groups 关系表和 chat binding 查询；之前同一插件生命周期内每次刷新都会重复打开 cards/groups。`DatabaseCharacterRepository` 现在在打开数据库时复用已经为一致性验证加载的完整 state，预置目录快照；后续读取先检查 canonical metadata revision，版本相同则直接复用 cards/groups，版本变化时重新读取并做前后 revision 校验。缓存没有时间过期窗口，角色、分组和同步发布改变 metadata revision 后自动失效。

在真实 Profile 诊断中，目录 IPC 从每次约 12 个存储调用降为 2 个：canonical metadata 一次、chat binding 查询一次。冷首个请求也使用打开阶段的快照，没有额外重新读取目录。目录 target 中位数约为预加载 2ms、无准备 2ms；同一二进制复跑分别为 2ms 和 3ms。预加载命令区间一次运行为 23.4ms，最终 microtask 刷新候选同一二进制复跑为 23.2ms；无准备分别为 30.5ms 和 32.9ms，机器波动仍存在，不能把单次差值误报为稳定收益。稳定收益是存储调用数量从 12 降到 2，以及首次冷目录不会再重复扫描关系表。

缓存验证用例检查同 revision 读取只增加 metadata 检查，创建角色后 revision 改变并能看到新角色。实现见 `plugins/packages/buildin/character_cards/src/storage/database.ts`，测试见 `tests/storage-persistence.test.mjs`。Prepared 回归为 sidebar-native 28 项全通过。

## onLoad 的进一步拆分

额外开启成功 IPC 阶段日志后，20 角色/20 聊天的 7 个后续样本如下。这里的毫秒为 Host 单调时钟粒度，记作 0 不表示不存在亚毫秒开销。

| 目录请求阶段 | 预加载侧栏中位数 | 无准备侧栏中位数 |
| --- | ---: | ---: |
| `character-sidebar.catalog` target 总耗时 | 5ms | 7ms |
| target 中等待执行结果 | 5ms | 7ms |
| JS 活跃阶段（含异步存储等待） | 4ms | 6ms |
| target 引擎获取、脚本读取、注册表各阶段 | 各 0ms | 各 0ms |
| JS scheduler / engine 队列等待 | 各 0ms | 各 0ms |
| 每次请求的真实存储调用数 | 12 | 12 |

这些包含关系不能相加。12 次调用均为 `default:storage_request`，包含角色/组关系目录、聊天绑定及一致性读取；并发工具的累计时间也不能当作墙钟时间。诊断运行里预加载目录耗时范围 4–8ms，无准备为 4–7ms。当前主要是实际目录计算和存储读取，未发现每次 IPC 都重新初始化引擎或等待一个固定调度周期。

无额外 IPC 诊断的同一二进制复跑中，旧同步刷新路径的预加载首次 render command 中位数 22.3ms、onLoad action 7.2ms；无准备首次 render 8.4ms、onLoad action 23.3ms。当前 microtask 刷新候选的一个代表样本为预加载首次 render 25.2ms、onLoad action 2.0ms；机器波动仍然存在。无准备场景在 onLoad 中首次建立实际内容，不能把整个 action 都记作 IPC 开销；这些独立分位数也不能直接相加得到命令并集。

诊断原始日志和逐样本 trace 汇总：`/tmp/operit-sidebar-engine-ipc-20261018.log`、`/tmp/operit-sidebar-engine-background-microtask-20261018/`、`/tmp/operit-sidebar-engine-background-microtask-repeat-20261018/`、`/tmp/operit-sidebar-engine-ipc-20261018/sidebar-prepared-ipc.json`、`/tmp/operit-sidebar-engine-ipc-20261018/sidebar-unprepared-ipc.json`。


## 后续排查：滚动补窗中的重复生成（2026-10-18）

发现首棵树和滚动补窗是两个不同问题。临时窗口 sweep 最初被角色卡的 4 条预览限制误导：23 条会话只生成 7 个业务行，`initialItemCount` 从 8 改到 24 不会改变这棵 134 节点的树。解除预览限制的诊断样本中，4/8/12/16 个首批行分别产生 83/179/275/371 个节点，脚本至提交中位约 19.9/29.5/41.2/50.9ms。没有把这些临时修改加入插件，也没有缩小初始窗口来制造首屏空白。

主要修复：

- `Lazy.js` 将目录依赖与视口范围分开比较。目录输入不变时，移动窗口只比较范围及一个数字 generation，不再因范围变化重新 stringify 整个目录。
- 每一项通过 `Generation.memoItem` 获得绝对索引对应的 memo region；重叠项不执行 builder，只为进入窗口的项构造节点。离开窗口的 region、effects 和回调由既有所有者释放，缓存不会随滚过的目录增长。
- 旧路径把模板 occurrence 按整窗内的调用顺序编号。滑动后，相同业务行的无 key 子树可能借用了别行的生成位置，造成重建和重发。每项自己的 region 固定了这个归属，保留原来的 retained ID 与 callback 所有权。
- `Generation.scope` 改为每个 call site 一组 occurrence 数组及 pass 计数，减少逐节点的字符串拼接和重复 Map/Set 操作；`Nodes.build` 对标量属性先走直接赋值，避免不需要的节点、数组检查和遍历状态分配。
- 角色卡对宿主输入的严格解析通过 `useMemo` 复用。宿主更新快照时重新校验；仅视口变化时不再逐条验证、复制聊天历史，也不会生成一整组新输入对象。

### 最终真实角色卡 A/B

使用实际 `dist/ui/chat-sidebar/index.ui.js`，100 条独立测试会话、已准备且展开的角色目录，保持 24 项窗口，每次移动 2 项。基线恢复修改前的 Lazy、Generation 和插件产物；候选使用当前版本。8 对 fresh-session 样本交替 AB/BA，排除第一对，剩余每种 7 个会话、140 次滚动命令取中位数。使用 Rust `Instant` 测量 session 命令执行到完成，响应消费、标题/引用校验在计时外。

| 指标 | 修改前 | 修改后 |
| --- | ---: | ---: |
| 滚动补窗命令 | 85.527ms | 10.189ms |
| 每次滚动的 upserts 中位数 | 93 | 49 |
| fresh session 首次脚本提交至命令完成 | 81.243ms | 83.334ms |
| 首棵 retained 节点数 | 563 | 563 |

滚动命令约提升 **8.4 倍**，减少约 **88.1%** 时间；这不是冷启动的提升。这个展开场景首次约增加 2.1ms（2.6%），是新增项所有权及缓存初始化的成本。

该探针是本机 **Debug 原生 QuickJS + 宿主调度 + retained 流传输**，运行真实插件代码与合成输入，排除目录 IPC、引擎 bootstrap、Flutter 帧、图片 IO 和 GPU。没有重启或替换用户运行中的 macOS 应用，不能把它描述为现有窗口的视觉时间或 Release 绝对性能。

### 机制对照和首次检查

纯机制探针为 24 行、每行 24 节点，每次移动 2 行。相同进程交替 A/B 后，旧路径约 57.63ms，新路径约 4.62ms（约 12.5 倍）；upserts 从 555 降到 49。连续 20 次滑动，含首次的 builder 次数从 504 降到 64。该合成树的全量数据更新约 33.94ms → 40.75ms，首次约 38.79ms → 42.24ms，说明该优化依赖窗口重叠，不能宣称全量重建也变快。

在之前的 7 行/134 节点角色卡样本中，脚本到完成约 22.70ms → 22.17ms，无变化输入刷新约 1.317ms → 1.314ms，基本持平。这次检查发生在额外增加输入解析 memo 之前。

### 复跑与校验

`ToolPkgComposeDslSessionTests.rs` 保留两个显式 ignored 探针：

```sh
cargo test --manifest-path core/Cargo.toml -p operit-tools native_lazy_window_overlap_performance_probe --lib -- --ignored --nocapture
cargo test --manifest-path core/Cargo.toml -p operit-tools native_character_sidebar_window_performance_probe --lib -- --ignored --nocapture
```

默认只测候选。`OPERIT_LAZY_BASELINE_SOURCE`、`OPERIT_GENERATION_BASELINE_SOURCE` 可加载历史 SDK 源码；真实侧栏对照另用 `OPERIT_SIDEBAR_BASELINE_SOURCE` 加载历史 UI 产物。对照注入、源文件读取与数据生成均在计时外。真实侧栏探针验证准备目录的 context 与校验后的 JS 输入完全匹配，每步检查全部应显示会话的标题与 retained 子节点引用，拒绝用失败或 fallback 场景产生性能数字。

Node 回归覆盖仅创建新进入项、回调 ID 保留、离窗删除与返回重建、effects 精确释放、热更新、嵌套无 key 子树及滚动不重扫大目录。原生测试的产物断言已更新到当前 v2，同时保留 v1 执行兼容用例。

原始结果：`/tmp/operit-real-sidebar-window-final-ab-20261018.{log,json}`、`/tmp/operit-lazy-overlap-ab3-20261018.{log,json}`、`/tmp/operit-sidebar-current-{before,after}-20261018.log`。历史 SDK 和插件对照源位于 `/tmp/operit-compose-lazy-before-20261018.js`、`/tmp/operit-compose-generation-before-scopes-20261018.js`、`/tmp/operit-sidebar-before-item-cache-20261018.ui.js`。临时窗口 sweep 已移到 `/tmp/operit-lazy-window-experiments-20261018/`，没有留在工作区。

最终回归：Compose/bridge Node 104 项、角色卡侧栏与路由/回调 60 项、原生 session 8 项、原生 QuickJS Compose 11 项全部通过。两项窗口性能探针已显式执行通过；默认 session 测试跳过 4 个显式性能探针。角色卡 host/browser 严格 TypeScript、插件构建、runtime assets 同步及归档一致性检查通过，`git diff --check` 通过。同步使用 `--no-hot-reload`，运行中应用未重启，本轮未提交或推送。

## 删除显式列表依赖，改用引擎失效边界（2026-10-18）

- 从 Rust SDK 源定义、生成的 TypeScript 声明、角色卡源代码和产物移除列表的显式依赖参数。类型契约限定列表选项为 key、itemCount、itemExtent、initialItemCount。
- 视口移动使用已有的局部重组/依赖排空路径；action registry 私有识别 SDK range handler，generation trampoline 随当前闭包更新分类。业务包装函数不能继承“只有视口变化”的资格。
- 普通业务回调（含 no-render）、state 写入、宿主输入、实际主题变化、显式渲染和源码替换使内容 revision 失效。没有把任意 JS 对象假定为 immutable，也没有把这套保守失效机制表述为 Kotlin 编译器完整的稳定性推断。
- 纯视口变化保留重叠项；普通业务回调会保守执行窗口 builder。这会牺牲旧显式依赖方案对无关业务回调的跳过，换取不要求插件作者手动维护依赖和不遗漏普通 JS/ref 变化。
- Node 回归验证：无参数时只构建进入项、离窗释放、回调稳定、effect 生命周期、热更新、宿主输入、循环 mutable ref、no-render 变更、异步业务与滚动交错、回调槽从视口切到业务，以及随 range 事件交付的新主题。
- 本轮按用户要求未执行 Rust、Flutter 或平台编译。原生吞吐与端到端耗时需要用户恢复编译后再测，不能沿用旧 8.4 倍数字声称新机制已达到相同结果。
