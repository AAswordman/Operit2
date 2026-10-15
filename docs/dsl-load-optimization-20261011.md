# DSL 加载与渲染优化记录（2026-10-11）

已完成资源签名、JS 节点生成和 retained 提交三个瓶颈的修改。列表规模从 20 增至 500 条时，首批节点保持 130 个，避免在页面打开时构造、遍历并传输整个列表。

**原因与实现**

1. 内置包缓存检查在命中缓存前仍对整个包计算 SHA-256。现在由 runtime 的 build.rs 对最终打包字节生成签名，通过现有 RuntimePluginAsset 传给包管理器；运行时直接使用该签名。可变资源仍由原有资源快照及失效机制负责，没有增加另一套缓存、过期时间或资源读取链路。
2. 原有 LazyColumn 在 Flutter 端延迟创建 widget，但 JS 已经生成所有选项节点。新增 ctx.lazyItems，只生成视口范围内的节点，并在两端预取两项。范围通知使用现有 action 通道，状态保存在现有 useRef 中，节点和回调继续由原有 retained 生命周期管理。角色卡选择器已改用该接口。
3. retained 提交反复读取公共 mutation proxy，造成递归代理访问及路径分配。Generation 将原来的节点标识和监听表合为一个 WeakMap，提供工厂持有的底层容器给 retained。插件对公共节点的修改继续接受监听，回调绑定、节点标识和增量传输规则保持一致。

内部根失效入口统一为 invalidateRoot。生产代码没有保留 environmentChanged 别名，没有新增平台判断或平台专用实现。

**实测结果**

以下是同一台机器、同一 Windows debug DLL、同一测试视口下的 native 首树帧测试。每个规模预热两次，正式采样八次；表中耗时为最近秩法 p50。各阶段 p50 不能相加当作某一次请求的精确分解。

| 选项数 | 初始节点数：之前 → 现在 | 首树帧总耗时：之前 → 现在 | 总耗时减少 | JS 根构建：之前 → 现在 | retained 提交：之前 → 现在 |
| --- | --- | --- | --- | --- | --- |
| 20 | 189 → 130 | 228.324 → 191.265 ms | 16.2% | 26 → 15 ms | 36 → 9 ms |
| 100 | 909 → 130 | 517.863 → 202.585 ms | 60.9% | 128 → 17 ms | 181 → 10 ms |
| 500 | 4509 → 130 | 2410.735 → 212.647 ms | 91.2% | 733 → 17 ms | 1122 → 10 ms |

500 条测试的总耗时 p95 从 2457.805 ms 降至 231.809 ms；render command 的 p50 从 2258.756 ms 降至 87.510 ms。这个 command 包含 JS 执行、提交和返回的开销，当前 SDK 实验还包含 SDK 安装开销，不能把它称为单独的解析耗时。

测量使用现有 operit_flutter_bridge.dll，在新建 QuickJS 页面上下文中显式装入当前检出的 SDK，随后走生产 CoreLink FFI、类型解码、retained Flutter store 和测试环境的 build/layout/paint。SDK 安装耗时计入 render command。Rust 资源缓存仍来自旧 DLL；新的构建期签名修改通过 cargo check，但收益尚未计入上述数字。

目录数据预先装入测试，因此上述结果不包含业务数据查询、WebView 初始化、GPU 实际呈现和应用启动。引擎获取、会话建立仍包含在总耗时内。日志中的 runtimeBootstrapMs、packageRegistryMs 单列记录，没有计入逐页面的 totalMs。这不是整机冷启动或完整业务点击到最终内容的测量，也没有进行整应用重建。

**列表接口约定**

```typescript
const items = ctx.lazyItems(
  { key: "records", itemCount: records.length, itemExtent: 56, initialItemCount: 12 },
  /** Builds a stable keyed item inside the requested viewport window. */
  index => ctx.UI.Text({ key: records[index].id, text: records[index].name }),
);
return ctx.UI.LazyColumn({ weight: 1, ...items });
```

生成列表需要有界的主轴布局；通过固定高度或有界父容器中的 weight 分配空间。每一项需要稳定且不重复的 key。itemExtent 是包含分隔线和项间距的未缩放主轴尺寸，Flutter 使用当前 TextScaler 在 14 点字号处的缩放比例调整尺寸；调用方应按此约定设计项内容，不能用这个接口表示任意可变高度内容。LazyRow 使用相同协议。

firstIndex 为绝对起始索引，lastIndex 为排他结束索引。Flutter 在帧后合并范围通知，同一列表至多有一个范围 action 在执行；完成后按最新滚动位置请求范围。未送达的索引保留固定布局空间，目录缩减、反向滚动、横向滚动及字体放大已纳入测试。

**耗时日志的含义**

- compose_first_tree_frame：launcher 状态创建到第一次 DSL 树帧。它不是实际鼠标点击时间，也不代表异步数据已经加载完成。
- compose_onload_frame：初始根 onLoad 分派开始，等待其 action 完成后，到下一次 Flutter 帧的耗时。与首树指标一起查看，可以识别业务加载占用；它也不测量 GPU 显示完成。生命周期检查确保旧页面不会记录为新页面的完成帧。

**验证**

- 相关 Compose DSL 与角色卡 Node 测试：489 项通过；新增 lazyItems 测试包含滚动后保留行的回调身份及目录缩减。
- Flutter 综合验证：34 项通过；最终有界布局修改后，lazy、viewport、生命周期相关 22 项再次通过。这是重叠测试集，不合计为 56 项。
- 原生加载链路 probe、retained 行为和性能 probe、角色卡严格 TypeScript 与架构检查通过。
- Rust runtime cargo check 通过，运行时设置 RUSTFLAGS=-Awarnings。
- 修改涉及的 Dart 文件通过格式化和静态分析。

没有在生产代码中增加错误兜底、重试链路、第二套节点注册表或平台分支。

**复测与原始数据**

从仓库根目录准备当前 SDK 实验夹具：

```powershell
node tools/performance/compose_dsl_load_breakdown_probe.mjs --prepare-native-fixtures --native-current-sdk
```

在 apps/flutter/app 运行：

```powershell
$env:RUSTFLAGS='-Awarnings'
fvm flutter test --no-pub --no-test-assets `
  --dart-define=OPERIT_DSL_LOAD_PIPELINE_PROBE=true `
  --dart-define=OPERIT_DSL_PROBE_CURRENT_SDK=true `
  --dart-define=OPERIT_DSL_PROBE_LIBRARY=D:/Code/prog/assistance2/apps/flutter/app/build/windows/x64/runner/Debug/operit_flutter_bridge.dll `
  test/compose_dsl_load_pipeline_probe_test.dart --reporter expanded
```

使用正常重建、内嵌当前 SDK 的 DLL 时，准备夹具和测试命令都不传 current SDK 实验选项，避免重复安装 SDK。probe 默认不启用。

- 基线：target/dsl-performance/optimization/before-native-flutter.json
- 最终 native 测量：target/dsl-performance/current-load/native-flutter-current-sdk.json，measuredAt 为 2026-10-11T11:53:47.032217Z。
- 用户点击日志提取：target/dsl-performance/current-load/click-log-extracted.json
- 验证日志：target/dsl-performance/optimization/ 下的 node-tests.log、flutter-tests-final.log、lazy-tests-final.log、native-current-sdk-final.log、retained-check.log 和 rust-check-final.log。
- 测量入口：tools/performance/compose_dsl_load_breakdown_probe.mjs、apps/flutter/app/test/compose_dsl_load_pipeline_probe_test.dart。

**继续优化与统一链路清理（2026-10-11，最终复测）**

这一轮缩减每个引擎重复编译、读取源码和遍历代理的成本：

- `HostJavaScriptBootstrap` 在同一 host 内只保存一份可信 bootstrap 的编译结果，Native 和 Web 在现有 host 实现中执行。每个 runtime 独立反序列化及运行，页面的全局变量、闭包、状态与 host 绑定仍独立。编译失败保存原始错误，源码身份变更明确拒绝；字节码不从插件或磁盘读取。
- bootstrap 源码通过 `OnceLock` 组装一次。CryptoJS、Jimp、UINode、AndroidUtils、OkHttp3、pako 使用同一描述表，在首次访问公开 export 时初始化。未使用的库不读取、不编译；同库 export 共享初始化身份，错误保持为终止状态。
- Acorn 可信源码从 eager bootstrap 移出，通过统一 host 绑定在实际源代码编译时读取。预编译页面不读取、不初始化解析器。普通初始化失败保留原始错误，中断也不会重新初始化或发布局部 exports。
- Generation 的 primitive 快照减少路径字符串和临时容器；retained 读取 Generation 持有的原始构造容器，避免代理重复访问及套叠。UI registry 每种节点类型复用一个 factory。
- Flutter launcher 的脚本和 screenPath 查询用一次 `Future.wait` 并行等待。

删除了 `JsBootstrapModule`、`buildRuntimeBootstrapModules` 及其专用路径组装函数。它们已没有生产调用，保留会形成重复组装入口。对应测试改为检查唯一生产 bootstrap。重复的旧 bootstrap 性能 probe 也已合并为一个明确的三策略对照；全量加载对照只在 `#[cfg(test)]` 中存在。

原有 Chat、扩展记录和目录测试仍模拟已删除的公开 callback ID、JSON 文本回调及旧目录 binding。测试现已统一使用共享的 `installScopedHostRuntime` 和生产 scoped host Promise 协议，保留记录隔离、严格输入验证、流式释放、错误和调用归属断言；仅删除已失效的 JSON 回调字节用例。没有恢复旧生产接口，也没有排除这些测试。

**真实 QuickJS bootstrap 对照**

独立运行，20 次正式采样、2 次预热，每次创建新的 native runtime；三种策略使用同一份当前 SDK。测量只包含 bootstrap 编译/装载及执行，不包含业务查询、CoreLink 或 Flutter。

| 初始化策略 | 源码字节数 | p50 | p95 |
| --- | ---: | ---: | ---: |
| 六个库全量加载、parser 源码内嵌、每次源码编译 | 694375 | 29.484 ms | 32.949 ms |
| 辅助库和 parser 源码按需读取、每次源码编译 | 362183 | 24.157 ms | 27.491 ms |
| 按需源码、host 共享可信编译结果 | 362183 | 8.924 ms | 10.048 ms |

bootstrap 的 p50 减少 69.7%，约为原来的 1/3.3。host 首次编译并执行为单次 39.173 ms；8.924 ms 是已生成编译结果后创建新页面 runtime 的中位数，不能用于宣称 host 首次启动耗时。源码体积减少 47.8%。

**同 DLL 的 SDK 首树帧对照**

核对发现当前测试 DLL 在 2026-10-11 20:28:25（Asia/Shanghai）更新过，晚于上表第一轮测量。因此另做了同一 DLL 的优化前后 SDK 对照，不能直接把之前的 191–213 ms 与最新数值之差全部归因于这一轮 SDK。

两次对照使用完全相同的 DLL、预编译选择器页面和其余 SDK 模块，只替换 Generation、Retained 与 Nodes 的优化前后实现。优化前 Generation/Retained 从当前 git index 获取，Nodes 从 HEAD 获取；源码快照仅用于独立性能夹具。每个规模预热两次、正式采样八次，各阶段采用最近秩法中位数。

| 选项数 | 首树节点数 | 首树帧 p50：SDK 优化前 → 后 | JS 根构建 p50：前 → 后 | retained p50：前 → 后 |
| --- | ---: | ---: | ---: | ---: |
| 20 | 130 | 163.931 → 145.661 ms | 16 → 14 ms | 9 → 8 ms |
| 100 | 130 | 150.272 → 146.753 ms | 16 → 14 ms | 9 → 8 ms |
| 500 | 130 | 164.666 → 161.947 ms | 15 → 15 ms | 9 → 8 ms |

500 条样本的 p95 从 194.701 ms 到 164.885 ms。8 次采样及 debug 测试帧存在波动，特别是小列表的帧时间变化，不能仅据此保证生产尾延迟。500 条根构建中位数未进一步下降；不声称这项优化让完整首树帧再快一个数量级。

DLL SHA256 为 `4fb96c403cecc33a17ed000eaf34ad765a0179515d531ab2b8f60347b76cb307`，在两次对照结束后再次核对未变化。当前 SDK 通过实验夹具装入 DLL，安装开销计入 render command。DLL 生成时间早于最终源码，不能把该首树实验当作所有最新 Rust 优化的整应用集成测量；独立 bootstrap 结果也不能从首树总耗时中相减推导新的总耗时。

最终同 DLL 实验中，engine acquire 的 p50 为 33–35 ms，session 建立约 3 ms，render command 为 54–79 ms，Flutter debug 测试帧约 43–51 ms。render command 包含 SDK 安装、根构建和 retained，不是单独的 DSL 解析时间。root 和 retained 两项已经约为 14–15 ms 与 8 ms；原来的全列表膨胀成本已消除。本实验仍使用预加载业务目录，未覆盖业务查询、WebView、GPU 呈现或整应用冷启动。

**最终验证与原始结果**

- Native host：26 项测试通过，包含共享编译结果下的 runtime 状态和绑定隔离、源码身份拒绝、原始编译错误。
- JS bridge Rust：81 项单元测试及唯一 bootstrap 路径测试通过；手动 probe 单独通过。
- Web wasm32 与 runtime 的 cargo check 通过；平台差异留在 host 实现。
- Dart 静态检查无问题；相关 Flutter 行为测试 33 项通过、1 项 opt-in probe 跳过，另行开启的真实首树帧测试通过。
- 角色卡严格 TypeScript、架构检查和包产物校验通过。构建包与 Core production 内置包哈希一致。
- 设置页中 TTS/Theme 生成模型的名称引用同步到准确的新类型，保持现有设置行为。

相关日志及夹具均位于 `target/dsl-performance/optimization/`：

- `second-bootstrap-probe-final.log`：最终三策略 QuickJS 测量。
- `second-native-same-dll-baseline.json`、`second-native-same-dll-final.json`：同 DLL 的 SDK 原始样本。
- `second-native-dll-identity.json`：DLL 身份及 SDK 对照来源。
- `second-native-same-dll-baseline.log`、`second-native-same-dll-final.log`：Flutter 原生首树帧测试输出。
- `second-rust-cleanup-tests.log`、`second-host-tests-final.log`、`second-web-check-final.log`、`second-runtime-check.log`：Rust 验证。
- `second-flutter-tests-final.log`、`second-dart-analyze.log`：Flutter 行为和静态检查。
- `second-cleanup-node-tests.log`、`second-unified-contract-tests.log`：产物/初始化与统一 host 协议测试。

复测 bootstrap：

```powershell
$env:RUSTFLAGS='-Awarnings'
cargo test --manifest-path core/Cargo.toml -p operit-js-bridge --lib `
  native_bootstrap_bytecode_probe -- --ignored --nocapture --test-threads=1
```
