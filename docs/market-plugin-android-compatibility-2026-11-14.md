# 市场下载榜前十插件：二代安卓兼容核查（2026-11-14）

本轮以市场首页“下载排行”的 `downloads` 排序、全部分类和全部时间累计下载量为口径，检查前十个条目的发布包。获取日期为 2026-11-14；服务端列表快照时间为 `2026-11-11T09:59:56.525Z`，不能把这个快照当成 11 月 14 日实时统计。列表来源为 `https://static.operit.app/market/v2/lists/all/downloads/page-1.json`；客户端 `MarketPane.dart` 的 `_popular` 同样取自 downloads 列表。

按本次需求，**工作流模板格式、导入、节点语义和运行差异不要求完整兼容**。工作流插件可以检查脚本能否加载和注册，但不能因此宣称其工作流已能执行。

未连接安卓设备。本报告区分原包源码核查、使用当前生成 SDK 的受控运行测试、真实原生 JS 引擎和存储测试。HTTP、终端及 Compose 的受控响应不能证明真机权限、网页反爬、媒体编码或付费语音接口实际成功。

## 前十逐项结论

“适配路径通过”表示下列已测试路径通过，不代表整包所有功能或真机验收通过。

| 排名 | 插件 | 市场版本 | 下载量 | 二代安卓结论（工作流除外） |
| --- | --- | --- | ---: | --- |
| 1 | 🌸温柔巡检 | 0.6.2（包内 0.6.0） | 56,411 | shell 入口已补齐；原始工具的精确使用统计解析、设置面板构建通过。高权限操作须真机验证，保护列表仍含旧宿主包名。 |
| 2 | Conversation lock | 0.2.0 | 39,391 | 当前主运行时内的锁定、聊天隔离、输入拦截和解锁通过；其 Node `fs` 持久化不可用，主引擎重建或应用重启会丢失锁。部分兼容。 |
| 3 | 久未回复主动联系工作流模板 | 1.0.3 | 35,795 | 原始 main 可加载并注册生命周期 hook；主要功能是工作流，按要求不做完整执行兼容结论。 |
| 4 | 传话筒 | 0.6.0 | 32,333 | SharedPreferences 已桥接；原包设置保存、开关、消息注入与增量时间戳适配路径通过，跨引擎配置读取有真实原生引擎验证。 |
| 5 | AI聊天记录转换工具 | 1.20.2 | 29,624 | 模块和界面构建、纯 ChatGPT JSON 转换通过；一键导入及真实总结仍调用已不存在的 v1 内部 Java 类。部分兼容。 |
| 6 | 小红书图文阅读器 | 1.1.1 | 25,519 | 原始 OkHttp 请求适配、文本笔记提取和设置界面构建通过。图片识别需配置外部模型；真实网页和视觉接口未验证。 |
| 7 | 应用冻结增强 | 1.13.0 | 25,134 | 结构化 shell 回执、原包查询/冻结/解除冻结、自动解除启动标记的适配路径通过；真实 dex 执行和后台定时效果须真机验证。 |
| 8 | Moodlet | 1.3.0 | 22,728 | 原包 hook、强制提示词、XML 渲染回调、各设置/显示界面构建通过；真机绘制和点击效果未验证。 |
| 9 | 主动消息守卫-实验版 | 1.0.0 | 21,939 | 当前发布脚本下载 404，对应 GitHub Release 的 assets 为空；无法取得原始产物，不能判断运行兼容性。 |
| 10 | AI的电子嘴巴 | 0.5.5-four-provider-voice-library | 21,697 | 四个服务商的受控正常响应能走到语音标签；MiniMax 十六进制音频分支报 `Buffer is not defined`，另有配置文件读取缺少 await 的插件问题。部分兼容。 |

9 个可下载发布包的归档均已与市场 SHA-256 校验一致，原始 main/工具模块未修改。9 个 manifest 均未声明 `api_version`，当前解析器默认使用 `1.0.0`；市场外层的 `formatVer: toolpkg_v2` 是归档格式，不能当作运行时 API 版本。部分条目标注 `maxAppVer: 1.99.99`；本报告检查执行代码，未把该字段单独当作运行必定失败或安装必定成功的证据。

## 传话筒：修复 SharedPreferences 断点

原包使用 `Java.getApplicationContext().getSharedPreferences("toolpkg_messenger", 0)` 存储开关、桥接配置、显示名称和每个来源的最后注入时间。原二代 Java 上下文仅实现 `getApplicationContext`/`toString`，因此读取捕获异常后返回默认关闭，写入失败。

按收敛修改范围的要求，SharedPreferences 的生产实现现在全部位于 `core/crates/plugin/sdk/src/compat/v1/install.js`，包括 Java 上下文包装和存储适配。已撤掉新增的 Rust Store/Host 方法、引擎绑定和通用 Java 桥接分支，也不再修改 codegen 加载清单。只对 API `1.0.0`/`1.0.1` 提供适配，v2 仍调用原 Java 接口。支持：

- `getBoolean/getString/getInt/getLong/getFloat/getStringSet`、`contains/getAll`。
- `edit()`、链式 `put*`、`remove/clear`、`apply/commit`。
- 类型校验，缺省值与已保存的 `false`、`0`、空字符串区分；集合防御性复制。
- 整批编辑提交，`clear` 在待提交的 put/remove 之前生效；失败不报告成功。

底层复用现有 `__operitNativeGetEnvForCall` / `__operitNativeSetEnv`，即插件同步 getEnv/setEnv 使用的配置通道；已安装插件沿用现有 `EnvPreferences`，注册时沿用现有 owner/scope 规则。兼容层将带类型的整份偏好保存为一个 JSON 配置值，使用 `__OPERIT_V1_PREFS_` 私有前缀及无碰撞的名称编码，不覆盖其他插件配置键。归属由原生已有调用上下文决定，不增加新的宿主接口。

现有 DataStore API 是异步的，无法直接代替旧插件同步 getter 而不改变插件行为，因此此方案复用已有同步配置通道。`apply()` 返回前完成该通道的配置写入，`commit()` 成功返回 true、写入失败返回 false；不增加异步待刷盘队列。保存和同步策略遵循插件原有安装作用域，不再单独强制存为设备本地文件。同一偏好的一次 editor 通过一个配置写入发布，但跨引擎并发仍是现有配置通道的后写覆盖语义，没有额外扩展 DataStore 事务保证。`long` 支持 JS 安全整数范围，拒绝精度丢失；支持 mode 0/4 的调用形式，未实现 SharedPreferences 监听器和其他 Android Java 类。旧应用的 SharedPreferences XML 不自动导入。

## Android shell 与 v1 终端修复

在 v1 SDK 内补充 `Tools.System.shell(command)`，只向 API `1.0.0`/`1.0.1` 暴露，继续保留 `Tools.System.terminal` 命名空间：

- 使用当前调用上下文确认安卓环境，明确请求原生 `terminal.create(name, 'shell')`。普通 v1 Linux/Python/FFmpeg 终端依旧选择其原有运行环境。
- 复用安卓宿主已有 Root → Shizuku → 应用权限选择逻辑；插件调用不会自动获得授权。`Tools.SoftwareSettings.exec` 是 Operit CLI，不能当作 Android shell。
- 每次命令创建独立会话，20 秒超时，成功、非零退出、超时和执行异常均清理自己创建的会话；并发调用不会共用同名会话。
- 返回旧版声明的 `{command, output, exitCode}`，补充 `toString()` 返回 output。温柔巡检可从其 JSON 化回执提取文本，应用冻结增强可直接读取 output/exitCode。
- 非零退出和超时抛错，避免旧插件将 fulfilled Promise 当成成功。清理失败也不会把成功路径误报为成功；执行和清理同时失败时保留原始执行错误。

真实原生 JS 引擎测试另外发现 `internal_runtime.js` 原先直接读取全局 `getState/getCallerName/getChatId`，但这些函数位于插件模块前置代码，SDK 闭包并没有这些全局函数。现在从活动调用的 params 读取同一组状态/调用者/聊天字段，修复原生执行 `getState is not defined`。普通 v1 终端、Node 启动器、FFmpeg 共用此修复。

## 仍有的具体限制

### 温柔巡检与应用冻结增强

温柔巡检原包设置面板使用 WebView DSL；当前控制器和 JS 接口存在，构建 Compose frame 通过。精确使用统计用受控 `dumpsys` 事件验证了 10 分钟的前后台区间计算。聚合统计仍需使用情况访问权限，隐藏应用等操作仍需足够系统权限。其保护列表硬编码 `com.ai.assistance.operit`，二代宿主是 `app.operit`，不能假设旧保护列表自动覆盖二代。

应用冻结增强将包内 dex 拷贝到共享目录，通过 `app_process ... AppSuspender` 执行。受控运行验证了状态字段、冻结/解除冻结和 nohup 启动确认标记，但没有实际执行 Android dex，也没有证明定时进程在系统回收、重启或权限断开后继续工作。

温柔巡检旧工作流使用 Kotlin `__type`，二代 `parseWorkflow` 要求当前节点 `type`，原包模板校验会报 `节点类型 取值无效：undefined`。此项记录为已知工作流差异，按要求不纳入完整适配。

### Conversation lock

原包在 try/catch 内通过 `require('fs')` 持久化，ToolPkg 模块加载器并不提供 Node fs。原包捕获错误后仍能保留内存锁。测试验证锁定 chat-one 不影响 chat-two、解锁后放行，并重新锁定后重建主引擎确认锁消失。不能承诺跨重启持续锁定；这和每次工具调用都重置主引擎不是一回事。

### AI聊天记录转换工具

`operit_archive_importer.js` 的一键导入依赖 `com.ai.assistance.operit.data.repository.ChatHistoryManager`、`ChatFormat` 等旧内部类，原包在二代返回 unsupported；`summary_tools.js` 的真实总结依赖 `com.ai.assistance.operit.api.chat.EnhancedAIService`。SharedPreferences 和 shell 桥接无法提供这些业务对象。

纯 ChatGPT JSON 转换已经对原始 converter 验证消息数与会话数。其他转换仍依赖文件权限、受管理 Linux 环境、Python3/标准库及解压工具；没有跑全部真实导入文件的端到端流水线。

### 小红书图文阅读器

受控笔记 HTML 通过实际 OkHttp 适配器返回，原包成功提取标题和正文；没有注入另一个 OkHttp 实现。图片下载、视觉服务、实际小红书访问和反爬结果仍需外部环境验证，不能据此保证任意链接成功。

### AI的电子嘴巴

原包四个 provider（Qwen、MiniMax、MOSS、ElevenLabs）配置环境变量后，用受控 HTTP、文件和 FFmpeg 计划/执行回执完成语音标签生成；设置面板与语音气泡构建通过。受控响应没有验证真实编码器、音频内容、实际播放或付费服务。

MiniMax 的 `data.audio` 若是 URL 可以走已测路径，若是十六进制字节串则调用 `Buffer.from(hex, 'hex')`，当前引擎没有 Node Buffer 全局，原包会失败。即便补 Buffer，原包后续直接把该对象传入期待 Base64 字符串的 `Files.writeBinary`，仍需要处理数据格式，不能只加一个名字伪装成功。

原包 `loadVoiceBarConfig()` 对异步 `Tools.Files.read()` 没有 await，同时同步检查 exists/content，因此只写 JSON 配置文件的值不能按预期读到。设置页同时写入环境变量时仍可走环境变量路径。这属于插件自身的同步/异步假设，需要与 shell 兼容问题分别处理。

## 验证与复现

- 最终相关 Node 回归：**80 通过，0 失败，4 个可选集成测试跳过**（共 84 项）。其中市场前十原包测试 26 项、独立 shell 测试 7 项、SharedPreferences 测试 10 项；缺失的 #9 未伪造源码测试。
- 9 个原始 main/工具模块加载注册通过；温柔巡检、传话筒、转换工具、小红书、Moodlet、电子嘴巴 6 组原始界面构建通过。这是 Compose frame 构建检查，不是真机 UI 验收。
- shell 独立测试覆盖两个 v1 版本、完整回执、非零退出、超时、原生拒绝、创建/清理失败、非法参数、安卓环境限制、保留函数的版本门禁和并发隔离。
- 最终原生 JS 引擎相关测试 **4 通过，0 失败**，覆盖 v1 偏好使用已有环境配置读写、销毁引擎后原包读取，以及 shell 调用、错误清理和普通 v1 终端选择；两个 v1 API 版本均覆盖。偏好测试不再增加存储 Host 方法，使用已有测试宿主的环境配置 Map；它验证真实 JS 引擎与原生绑定，不单独证明生产配置文件落盘。终端返回值也由测试宿主控制，实际 JS bootstrap 和 SDK 使用生产实现。

外部发布包没有写入仓库。下载并校验后，可把各包解压到 `TOP10_DIR/01/unpacked` 等位置，第 9 项缺失，运行：

```sh
OPERIT_MARKET_TOP10_DIR=/path/to/TOP10_DIR \
  node --test tools/tests/market_top10_android.test.mjs

OPERIT_MARKET_GUARDIAN_DIR=/path/to/extracted/guardian \
  node --test tools/tests/v1_android_shell.test.mjs

OPERIT_MARKET_MESSENGER_DIR=/path/to/extracted/messenger \
  node --test tools/tests/v1_shared_preferences.test.mjs

OPERIT_MARKET_MESSENGER_CORE=/path/to/extracted/messenger/dist/packages/messenger_core.js \
  cargo test --manifest-path core/Cargo.toml -p operit-js-bridge --lib v1_
```

未设置外部包环境变量时，相关原包测试明确跳过；Rust 偏好引擎测试改用内置最小样例继续验证现有配置读写链路。4 个最终跳过项是另外两个可选 Ferry 原包集成测试及两个需要本机 FFmpeg 的媒体集成测试，不计作通过。

已知仓库现存检查问题：全量 v1 适配测试的 `results.d.ts` 来源哈希和 `source.json` 记录不一致，HEAD 原文件即如此；兼容层全量 TypeScript 检查有 `isDefault`、隐式 any、未声明图片注册 helper 和角色 API callDependency 类型错误。收敛后的 SharedPreferences 代码未增加类型错误；不能把这些现存问题写成全仓检查通过。

## 发布包 SHA-256

| 排名 | 归档 SHA-256 |
| --- | --- |
| 1 | `399100843ebd97720f044d40c55e3bc978d038eb3920fd1707f91e1901b2a452` |
| 2 | `cf5c37b130c73a6400e9fab03d445107fb8f4c400877d4bfc7607ea184a3742d` |
| 3 | `d164fbf5c5c8a5eb6998172bf5609cbeae1a0f9686d7f3a704d7fb7cec5d2613` |
| 4 | `fbc8fa49c9b445105cc5aec3f282fdacddd8e660232930ced792f752941dfcd4` |
| 5 | `bf2b6e863ab7a98c86477f29c2ffac787641225bb82789d06335c86f87b4c36d` |
| 6 | `df5871461bc21e77c8b4de81b3263a60e4e1967fc7d73117b19057b540cd25b3` |
| 7 | `4014ea4f6c2927eeab011a322f7ee1a10d738318a00adad4af3a07b4be5b5fdf` |
| 8 | `3866cd1115c34f5bfde9723c269845a95f7cbb4d41c543ba77098b605737c911` |
| 9 | 未取得：下载 404，对应 Release assets 为空 |
| 10 | `ff29435b9625ee3483a2711d81e3a22f2d43a769938aa63986545a2d8d16bf19` |
