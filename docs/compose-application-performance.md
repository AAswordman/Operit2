# 真实 macOS Compose 页面性能测试

在仓库根目录运行：

```sh
python3 tools/performance/run_compose_application_benchmark.py --samples 8 --rows 20
```

命令先运行正式 native 构建脚本，准备插件资产与最新生成的 proxy，再以 Profile 模式编译并启动 macOS 应用。这样避免 Xcode 并行编译 Rust/Dart 时，新的生成源码晚于 Dart 编译而使本轮仍测旧解码器。构建和准备均不计入页面测量；使用已保存的应用二进制复跑时跳过准备。测试创建每轮独立的真实存储，调用正式 API 创建角色卡、绑定角色的聊天，交替测量：

- 原生侧栏：无准备数据。
- 原生侧栏：通过正式 `chat.sidebar.prepare` API 准备数据。
- 角色卡主页面：真实原生 WebView 和页面业务请求。

当前图片优化让相同 data URI 共用一个 `MemoryImage` 和字节数组，并在检查确定的 `data:` 前缀后直接命中缓存，避免通用 URI 解析反复扫描大图。缓存最多 64 项，来源文本和图片字节的估算保留量上限为 8 MiB；OS 文件仍按 FileImage 行为读取。原生 DSL 的 VFS 图片直接携带路径，ComposeDslVfsImage 使用 Flutter 的有界图片缓存合并同一会话、同一路径的读取和解码；读取失败会逐出缓存以允许重试，同路径内容更新需要 evict。图片解析失败仍会显示错误。

测试使用 `ChatSidebarTabHost`、`ToolPkgUiLauncherScreen`、正常 macOS MethodChannel/FFI/CoreLink、生产 host interaction 订阅、真实业务插件与存储。没有密钥存储替换，没有模拟 Tools 请求。数据目录在应用自己的 Application Support 内，避免文件夹选择弹窗；不会使用或修改用户配置的 runtime/workspace。

客户端日志使用内存 sink，未运行完整 Operit 主窗口壳和抽屉打开动画。数据创建及准备放在计时之前，因此主服务已经初始化；“首次观察”指本轮第一次打开该页面，不是整个应用或存储冷启动。每次重新挂载会创建新的页面 execution context。

## 输出

默认输出到 `target/dsl-performance/application/<时间>/`：

- `native-prepare.log`：正式 native 构建及 proxy 生成日志（构建新应用时）。
- `run.log`：Flutter 构建及真实应用的 Dart/Rust 日志。
- `application.json`：集成测试原始样本及 Flutter 引擎帧摘要；失败时保留当前工作负载、轮次和等待阶段。
- `report.json` / `report.md`：每场景首次样本、后续样本 p50/p95、分阶段比例及 build/raster 指标。
- `environment.json`：Flutter 版本、机器、供电、负载和工作区状态。
- `result.json`：进程退出码、命令和整轮时间。

计时维度：

- DSL 阶段：read-script、acquire-engine、open-session、render-command。
- 首树帧、首帧等待、onLoad 动作和 onLoad 帧等待。
- 从宿主挂载到 onLoad 完成的观察时间。
- 真实业务内容完成：侧栏验证角色名称；主页面通过既有原生 controller 查询 DOM，要求全部测试角色卡已经渲染。
- 图片控件数、唯一 `MemoryImage` 数和保留的图片字节，用于验证共享效果。
- 实际原生 `FrameTiming` 的 build、raster、totalSpan。整轮监听，按 raster wall time 分配样本，不按延迟的回调到达时间归属。

onLoad 观察轮询间隔为 20ms，WebView 内容观察轮询间隔为 50ms，计入观察指标。精确阶段指标来自生产日志。首树帧和 onLoad 帧不是 GPU 展示时间。阶段会重叠，不相加分位数。挂载、卸载、生命周期和内容等待均有 30 秒上限；缺失/错误/未匹配完成事件会失败，不能把超时页面当成快样本。

## 对照与性能门限

先运行基线：

```sh
python3 tools/performance/run_compose_application_benchmark.py \
  --samples 8 --rows 20 --save-binary \
  --output target/dsl-performance/application/baseline
```

修改优化代码后运行相同配置：

```sh
python3 tools/performance/run_compose_application_benchmark.py \
  --samples 8 --rows 20 \
  --baseline target/dsl-performance/application/baseline/report.json \
  --max-regression-percent 15 \
  --output target/dsl-performance/application/current
```

默认门限检查后续打开的首帧、首帧到 onLoad 完成总量、业务内容完成的 p50。超过门限以非零状态退出。数据量、重复数、模式、窗口尺寸、Flutter/系统/机器及供电不同则拒绝比较。不同轮仍可能受系统负载及温度影响，门限是回归筛查，不是统计显著性证明。不要将插电前后的绝对时间下降归因于代码。

保存的 benchmark.app 可以在不重新构建的情况下复跑原基线：

```sh
python3 tools/performance/run_compose_application_benchmark.py \
  --samples 8 --rows 20 \
  --application-binary target/dsl-performance/application/baseline/benchmark.app \
  --output target/dsl-performance/application/baseline-repeat
```

二进制里的样本数和数据量是编译时固定的，必须与命令指定的参数一致。测试不会改变代码来切换基线，也不会切换或重置用户的工作区。

仅检查指标分析和对照判断：

```sh
python3 -m unittest discover -s tools/tests -p 'test_compose*py'
```

也可以加入现有诊断命令：

```sh
python3 tools/performance/run_compose_diagnostics.py --application --skip-pipeline
```

`--skip-pipeline` 跳过旧的 widget/native 合成探针。新测试独立运行，不依赖手动提供应用日志。

## 本次真实测量（2026-10-12）

同一台 arm64 macOS，稳定 AC 供电，Profile，800×600 逻辑尺寸、DPR 2；每场景 8 次，首次样本独立保留，分位数使用剩余 7 次。正式 API 每轮创建 20 张角色卡及 20 条绑定会话。对照使用保存的原基线二进制反向复跑，优化版本也复跑确认。

| 指标 | 原基线复跑 | 最终优化 | 变化 |
| --- | ---: | ---: | ---: |
| 预加载侧栏首帧 p50 | 499.0 ms | 244.0 ms | -51.1% |
| 预加载侧栏到 onLoad 帧 p50 | 566.0 ms | 299.0 ms | -47.2% |
| 预加载侧栏内容观察 p50 | 599.8 ms | 341.1 ms | -43.1% |
| 无准备侧栏到 onLoad 帧 p50 | 532.0 ms | 277.0 ms | -47.9% |
| 主页面首帧 p50 | 33.0 ms | 33.0 ms | +0.0% |
| 主页面内容观察 p50 | 611.9 ms | 620.3 ms | +1.4% |
| 预加载侧栏 build p95 | 318.7 ms | 58.2 ms | -81.7% |

预加载侧栏在内容观察点的 20 份相同 `MemoryImage` 降为 1 份，编码图片字节由 27,385,900 降到 1,369,295。最终运行记录 1,887 次 Direct 工具请求，工具 lifecycle hook 为 0；正常定时 host event 不计为工具 lifecycle hook。9 项默认性能门限均通过。

收益集中在侧栏头像解析与图片重复解码。角色卡主页面没有明显改善；侧栏 render-command p50 仍约 184ms，后续优化应继续分析这个阶段。测试不包含完整应用壳、抽屉动画或持久客户端日志 sink 的额外耗时。

结果文件：`target/dsl-performance/application/baseline-recheck/report.json` 与 `target/dsl-performance/application/final/report.json`。最终 Markdown 表格在同目录 `report.md`；原始应用日志及样本可直接审计。


## 先定位再优化：生产链路细分

```sh
python3 tools/performance/run_compose_application_benchmark.py \
  --samples 8 --rows 20 --trace-stages --save-binary \
  --output target/dsl-performance/application/pipeline-analysis
```

这会在原有 Profile 集成测试中开启诊断计时，测量 Rust→JS 输入转换、JS 模块准备与同步渲染、retained commit、JS→Rust 结果转换、Rust 类型转换、CoreValue 生成、MessagePack 编码/解码、Flutter 节点应用。附带实际结果节点数、字符串字节数与 MessagePack 字节数。无需手动提供日志。

按每次宿主挂载的 wall-clock 区间归属事件，分别报告首帧前和业务内容观察点之前的总量，首次样本单列。先在同一次挂载里累计每个步骤，再计算后续样本的 p50/p95；JSON 保留全部事件与调用标识。

`js-send-inclusive` 包含 `js-to-rust` 和 `rust-typed-response`；`js-stage-invoke_target_function` 包含实际渲染与同步结果发送。不能把父阶段与其内部步骤重复相加。JS 时钟精度为 1ms，Rust/Dart 用单调计时器计量时长；跨语言归属需要稳定的系统墙钟。未覆盖的间隙可能来自队列、传输和帧等待，不能直接归到某个处理步骤。

细分计时只在显式 `--trace-stages` 模式开启。它会增加计时和日志开销，诊断运行不能和未开启诊断的旧二进制作性能收益比较；比较器对此直接报错。优化结论仍需关闭细分诊断后，用相同供电、模式和数据量复跑原基线及候选版本。


通用 Link 边界不按 Dart/Rust 类型名或业务字段形状选择诊断对象。显式诊断开关开启后，对通过这些边界的事件统一计量，再按页面测量区间归属。完整 Stream 的类型解码阶段为 `dart-stream-messagepack`；增量 Flow/State 的阶段为 `dart-watch-read`、`dart-watch-retain`、`dart-watch-reencode` 和 `dart-messagepack`。分析器要求每个页面样本都具有完整的对应解码路径和其余链路阶段，缺失或损坏的数据会导致测试失败。

## 完整 Stream 的重复转换定位（2026-10-12）

完整诊断运行：`target/dsl-performance/application/pipeline-verified/report.json`，24 次页面挂载全部完成并通过阶段覆盖检查。下表为预加载侧栏首帧前步骤的累计耗时，每次挂载先分别求和，再取后续 7 次的 p50。

| 步骤 | p50 (ms) |
| --- | ---: |
| Dart 无类型解码 | 21.877 |
| Dart 保留增量基值 | 0.793 |
| Dart 重新编码 | 56.865 |
| Dart 类型解码 | 21.058 |
| JS 同步渲染 | 23.000 |
| JS retained commit | 14.000 |
| JS→Rust 结果转换 | 4.099 |
| Rust MessagePack 编码 | 3.770 |
| Flutter 节点应用 | 1.236 |

首树更新为 953 个节点，顶层字符串属性合计 36,541,305 字节，首帧前累计 MessagePack 负载约 37.4 MB。无类型解码、保留基值和重新编码在每个样本内合计的 p50 为 79.751 ms；相对于同一次 render-command 耗时的比例 p50 为 46.6%。这是对冗余处理的定位，不能作为关闭诊断后的优化收益。

优化依据已有 `WatchStreamProtocol`：`JsonStream` / `StringStream` 的独立完整事件直接用原始 MessagePack 字节做类型解码；`JsonFlow` / `JsonState` 保留增量解码器。生成器同时处理标量和列表，不依据具体 DTO 名称。Stream 解码器拒绝 Delta 与终止事件，保留类型校验及嵌入 Stream 路由能力。


关闭细分诊断后，用保存的优化前二进制复跑，再复跑新二进制；均为 AC 供电、Profile、20 卡/20 聊天、每场景 8 次，首次样本独立保留。

| 指标 | 原基线复跑 p50 | 优化复跑 p50 | 变化 |
| --- | ---: | ---: | ---: |
| 预加载侧栏加载 | 181.0 ms | 101.1 ms | -44.1% |
| 预加载侧栏首帧 | 236.0 ms | 160.0 ms | -32.2% |
| 预加载侧栏到 onLoad 帧 | 282.0 ms | 215.0 ms | -23.8% |
| 预加载侧栏内容就绪 | 334.8 ms | 260.8 ms | -22.1% |
| 无准备侧栏内容就绪 | 293.4 ms | 221.0 ms | -24.7% |
| 主页面内容就绪 | 606.1 ms | 522.1 ms | -13.9% |

新版本初跑与复跑的 9 项回归门限均通过，每轮 1,887 次 Direct 调用、0 次工具 lifecycle hook。结果见 `target/dsl-performance/application/stream-baseline-repeat/report.json`、`stream-direct-repeat/report.json`；首次新版本运行见 `stream-direct-verified/report.json`。主页面内容就绪包含 WebView 的异步请求和观察轮询，该指标不能视为 JS 渲染耗时；主页面首帧 p50 保持 33ms。

构建输入校验：新构建记录 proxy 生成文件在 Dart 编译前后的 SHA256；内容发生变化则拒绝产出有效比较报告，避免把旧生成源码的 AOT 当作当前版本。


优化后完整诊断见 `target/dsl-performance/application/stream-pipeline-verified/report.json`。24 个样本全部通过阶段覆盖检查，编译前后生成文件哈希一致。预加载侧栏的完整 Stream 直接类型解码 p50 为 21.372ms，首帧前 Stream 负载为 37,407,072 字节；其无类型解码/基值保留/重新编码已省去。仍在使用增量协议的是 74 字节状态 Flow，其重新编码 p50 为 0.002ms。JS 同步渲染 p50 为 24ms、retained commit 为 14ms，节点应用为 1.330ms；953 个节点和 36,541,305 字节顶层字符串属性保持一致。诊断数据只用于核查路径，收益表使用关闭诊断的独立对照。


本次定向验证：28 项 Dart 编解码、5 项代码生成、21 项 JS 会话、14 项 Python 指标测试通过；手写 Dart 文件的定向分析无问题。额外直接分析生成的 clients 文件时有 44 条 `unused_element_parameter` 警告，位于本次未修改的构造函数 `objectArgs` 参数，实际 Profile 编译和所有页面测试均通过。


## 头像路径传输对照

2026-10-12，AC、macOS Profile、20 卡/20 聊天，每场景 8 次、首次独立保留。
真实旧二进制基线为 `target/dsl-performance/application/avatar-path-baseline/report.json`，
路径版本为 `target/dsl-performance/application/avatar-path/report.json`，均关闭阶段诊断。
侧栏预加载与原生刷新直接返回默认资源路径和已保存的自定义头像路径，不读取二进制、不在 TS 里生成 data URI。
图片读取由 Flutter VFS provider 走原有 Core 工具链路，Direct 策略、0 lifecycle hook。

| 指标 | 原基线 p50 | 路径版本 p50 | 变化 |
| --- | ---: | ---: | ---: |
| 预加载侧栏首帧 | 161 ms | 81 ms | -49.7% |
| 预加载侧栏到 onLoad 帧 | 215 ms | 132 ms | -38.6% |
| 预加载侧栏内容完成 | 260.8 ms | 179.7 ms | -31.1% |
| 无准备侧栏内容完成 | 222.4 ms | 167.3 ms | -24.8% |
| 主页面内容完成 | 528.9 ms | 526.5 ms | -0.5% |

9 项性能门限通过。图片统计新增 `uniqueVfsImages`；VFS provider 会延迟读取，
不能将 MemoryImage 字节计数为 0 理解成没有读取/解码图片，也不能把创建了 Image 控件当作图片已经就绪。

## 自动导出真实负载与压缩诊断

`--trace-stages` 的诊断运行会通过 bridge 包装器保留每个工作负载最大的 call/watch 编码缓冲区，
没有按业务字段或 DTO 类型猜测。所有页面计时结束后，应用将文件写到自己的临时目录，
外部 runner 复制到报告目录的 `wire-payloads/` 并校验字节数、记录 SHA256。
应用直接写仓库 target 会被 macOS 沙箱拒绝，因此不能使用该路径导出。
正常性能运行不创建捕获包装器；这些诊断样本不能与关闭诊断的样本混用。

```sh
python3 tools/performance/run_compose_application_benchmark.py \
  --samples 8 --rows 20 --trace-stages --save-binary \
  --output target/dsl-performance/application/avatar-path-pipeline-verified

python3 tools/performance/run_link_compression_probe.py \
  target/dsl-performance/application/avatar-path-pipeline-verified/wire-payloads/*.msgpack \
  --output target/dsl-performance/application/avatar-path-compression.json
```

压缩诊断使用本机 Homebrew 的官方 LZ4/Zstd 动态库，Dart 原生 AOT，
比较原始复制+生产 MessagePack 解析、LZ4 压缩+纯 Dart 解压+相同解析、
LZ4/Zstd 原生解压的理论替代路径。
24 轮交替测量、前 4 轮预热丢弃，输出 p50/p95、同轮耗时比和压缩率。
先验证 36 种原生压缩块，包括重叠匹配、长扩展长度、随机数据、空输入，并拒绝畸形块；
每个真实负载也校验解压内容逐字节一致。
诊断通过隔离副本使用生产 Dart reader，仅移除 Flutter 可选日志 import，不修改其编解码逻辑，
记录生产 codec/protocol SHA256。它不等同于生产传输性能：没有 Rust 序列化、调度、完整 FFI、类型化 DSL 解析或页面渲染。
原生解压替代路径是成本下界，尚未接入产品。


压缩诊断复跑结果见 `target/dsl-performance/application/avatar-path-compression-repeat.{json,md}`。
预加载侧栏最大单条更新 893,053 字节；首帧前两条 Stream 合计 893,174 字节，不能混作同一指标。
最大更新中 `id` 字符串占 354,841 字节，`children` 字符串占 324,048 字节，map keys 占 89,502 字节，
20 个头像路径共 1,160 字节。

| 路径 | 字节数 | 压缩 p50 | 解压 p50 | 复制/压缩、解压、解析合计 p50 |
| --- | ---: | ---: | ---: | ---: |
| 原始 MessagePack | 893,053 | — | — | 1.118 ms |
| LZ4 + Dart 解压 | 50,680 | 0.133 ms | 0.189 ms | 1.378 ms |
| Zstd level 1 + 原生解压替代 | 25,553 | 0.208 ms | 0.100 ms | 1.402 ms |

LZ4 缩小 94.33%，Zstd 缩小 97.14%；同轮比值的 p50 分别 1.266、1.280。
初跑同样没有速度收益：原始 1.251ms、LZ4+Dart 1.632ms、Zstd1 原生替代 1.642ms。
这些是探针结果，尚未验证产品中的压缩链路，也没有接入生产协议；不能宣称真实应用因压缩而变快。
普通文件读取仍按现有 BinaryFileContentData 契约返回一次 Base64；页面节点不再重复携带这些字节。
当时的主页面 WebView avatarImage 仍返回 data URI；下述修正已覆盖主页面。

定向验证：28 项侧栏/加载/投影 Node 测试、1 项真实文件准备测试、9 项 VFS 图片测试、14 项 Python 指标测试通过；
严格 TS 检查通过。扩大检查发现工作区全量 Node 中仍有目录/工具/模型/TTS/消息扩展契约失败，
Flutter 侧栏 fixture 中 7 项动作回调计数失败；未将这些当作全量通过。


## 界面先显示、头像后台加载

2026-10-12，主页面、记忆页、原生侧栏、角色选择器、聊天输入菜单及原生角色预览/编辑均移除显示内容前的默认头像等待。
WebView 在页面渲染后通过 requestAnimationFrame 和后续任务开始解析头像引用；图片就绪时只更新头像元素，保留现有页面和操作状态。
头像资源引用或图片文件失败时保留占位内容；引用失败可在后续刷新中重试。
默认头像和同路径并发请求合并，TS 显示代码返回短资源引用，不读取或重复传输 Base64。
离线文档 CSP 允许同源图片；原生资源宿主按文件头识别导入的 .image 文件格式，合并正在进行的文件读取，后续请求仍读取最新文件。
导入头像只在实际复制文件时读取内容，不再永久缓存被选择的源文件。

性能测试以首帧、onLoad 和真实角色内容可见为完成条件，不等待头像解码。
以下结果均为电池供电、macOS Profile、20 卡/20 聊天、每场景 8 次，首次样本独立保留，后续 7 次统计 p50。
旧版本使用先前保存的真实二进制，报告为 target/dsl-performance/application/image-loading-baseline-battery/report.json；
当前版本独立复测为 image-loading-battery-repeat/report.json。

| 页面指标 | 旧版本 p50 | 当前版本 p50 | 变化 |
| --- | ---: | ---: | ---: |
| 主页面首帧 | 33 ms | 33 ms | 0.0% |
| 主页面到 onLoad 帧 | 49 ms | 49 ms | 0.0% |
| 主页面真实内容可见 | 913.8 ms | 242.0 ms | -73.5% |
| 无准备侧栏真实内容可见 | 211.3 ms | 220.4 ms | +4.3% |
| 预加载侧栏真实内容可见 | 268.3 ms | 249.0 ms | -7.2% |

当前版本首次成功运行的主页面内容可见 p50 为 238.2 ms，独立复跑为 242.0 ms。
9 项回归门限均通过；本轮 1,890 次 Direct 工具调用、0 次工具 lifecycle hook。
侧栏结果未证明稳定的速度收益；主页面内容观察包含 50 ms DOM 轮询、WebView 启动和业务请求，不等同于 GPU 呈现时间或 JS 同步渲染时间。
首次构建测量在最后一次挂载遇到生命周期日志超时，该失败轮未作为有效报告；供电变化的运行同样被拒绝。

最终定向验证：72 项 Node 测试、25 项 Flutter 图片/资源加载测试、严格 TS 检查通过。
浏览器验证覆盖页面先显示、默认和自定义头像的短引用、安装资源的 CSP、仅更新头像、缺失图片占位及失败后刷新重试；
原生验证覆盖头像资源挂起时列表、选择、编辑和取消仍可使用。
扩大检查中的 file-entrypoints 仍有一项目录错误先后顺序断言失败（预期 Models、实际 Catalog），没有把工作区全量测试宣称为通过。


## WebView 启动接口与完成观察延迟

2026-10-12，AC、macOS Profile、20 卡/20 聊天，每项 8 次，后续 7 次取 p50。
旧二进制复测：`target/dsl-performance/application/webview-stages-ac-baseline-repeat/report.json`；
当前版本：`target/dsl-performance/application/webview-bootstrap-ac/report.json`。

浏览器改为一次 `loadScreen` 返回具体 `ScreenBootstrap`（主题、当前屏幕、完整 snapshot），
删除启动时分别获取主题、屏幕、snapshot 的接口往返。
宿主仍通过原 `character-memory.request` IPC 获取新 snapshot；所有实际 Tools 调用保留规范 toolCall 链路。
主题应用后发布界面，头像继续后台加载。刷新和编辑结束仍重新读取权威数据。

| 主页面指标 | 旧二进制复测 p50 | 当前 p50 | 变化 |
| --- | ---: | ---: | ---: |
| 浏览器脚本启动到内容 DOM 完成 | 41 ms | 23 ms | -43.9% |
| 挂载到 DOM 完成估计 | 163.2 ms | 145.9 ms | -10.6% |
| 宿主观察到内容完成 | 166.2 ms | 166.9 ms | +0.4% |
| DSL 首帧 | 33 ms | 33 ms | 0.0% |

DOM 完成估计从每个样本的宿主观测总时间扣除浏览器 User Timing 测出的检测延迟，
仍包含原生查询和返回开销，不是 GPU 呈现时间。当前检测延迟 p50 约 21 ms，旧版约 3 ms；
50 ms 轮询掩盖了阶段收益，不能宣称宿主观测总时间降低。
初轮旧版的脚本启动阶段为 39 ms、DOM 完成估计 160.3 ms，与复测方向一致。
仅并行调用原有接口的版本没有收益，最终实现采用一次具体启动响应。

runner 自动读取 User Timing / Navigation Timing，输出 `webContentStages`，并对
`app-to-content`、`mount-to-dom-estimate` 增加独立回归门限，本次 11 项门限通过，Direct 调用 0 lifecycle hook。
16 项 Python 指标测试、26 项架构/回调/打包测试通过。
扩大 WebKit 测试 52 项中 49 项通过，图谱创建链接的两处弹窗点击失败及其父测试失败，未宣称全量通过。
角色页、记忆页、附件完成/取消、安装产物和实际文件持久化的启动回归通过。
