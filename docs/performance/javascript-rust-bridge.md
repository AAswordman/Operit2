# JavaScript / Rust 结构化桥接

## 唯一 host 传输契约

运行时不再安装 `NativeInterface` 全局对象。插件使用 `toolCall`、`Tools.*`、
`ToolPkg.*`、Compose context、`CryptoJS`、`Jimp`、`pako`、`Java` / `Kotlin`、
`console` 和标准定时器 API。内部 `__operitNative*` 绑定由 SDK 管理，插件不应直接调用。

`HostJavaScriptRuntime` 必须实现：

- `registerHostJavaScriptFunction`：同步传递结构化参数或拥有所有权的二进制缓冲区，返回结构化结果。
- `registerHostJavaScriptAsyncJsonFunction`：创建由 host 持有完成句柄的 scoped Promise。
- `settleHostJavaScriptPromise`：在所属执行器完成 Promise，释放 resolve / reject 句柄。
- `cancelHostJavaScriptPromises`：释放指定 scope 的待完成句柄。
- `callHostJavaScriptFunction`：直接调用 runtime 生命周期函数，不生成调用源码。

这些接口没有默认实现、能力选择开关或字符串 transport。原有 string / void callback
注册接口已删除；同步无返回值操作以结构化 null 表示完成，调用者不读取这个返回值。
同步能力保留同步语义，异步 API 使用同一套 host Promise 契约，不把所有能力强行改成异步。

## Promise 调用过程

1. SDK 验证公共调用形式，通过必须存在的 `__operitHostSnapshotJsonObject`
   同步取得工具参数快照，保留所属执行会话的引用。
2. Host 接收拥有所有权的 `serde_json::Value`，分配 request id，
   在 runtime 所属执行器保留 Promise 完成句柄。
3. 运行时处理工具、跨上下文 IPC、依赖、图片、资源、WASM、Compose 或包管理请求。
4. 结果队列携带 request id 和结构化值，在所属执行器完成 Promise。
5. SDK 激活原会话、解释明确的数据契约，在公共 Promise continuation 之后释放引用。

`__operitInvokeHostAsync` 是公共包装器共用的内部生命周期入口。
`toolCall` 使用 `__operitNativeCallToolStructured` 并解包明确的工具结果信封。
图片、资源、WASM 和包管理直接接收 host 结果；请求失败使用 Promise rejection，
不会把失败文本当成路径、解密结果或成功返回值。

异步 API 的 Promise 表示完成契约，不承诺底层工作一定并行。工具执行使用原有 host
executor，cooperative continuation 使用 host 明确实现的非阻塞 executor。

禁止在全局对象安装 callback ID 完成函数；请求与结果不使用 JSON 文本协议，
不生成 callback 调用源码。应用数据中的字符串始终保持字符串，不按内容猜测并解析。

## Host 边界与生命周期

- Native host 使用 V8 的值转换及 Promise 句柄；Compose 的 Uint8Array 脱离 JS 后转交 backing-store 所有权，不再递归转换布局对象。
- Web host 的值快照和 Promise 注册表位于 `hosts/web/src/javascript_promises.js`，
  Rust host 通过函数调用和对象属性 API 传递数据。转换捕获原始内建函数，
  不依赖插件可修改的 JSON、Map 或 Array 方法。
- 业务 Rust 和 Flutter 使用统一 host 契约，不增加业务平台分支。
- Flutter owner 的外部消息协议仍由 Flutter host adapter 编解码，不向插件暴露 JSON transport。
- 脚本执行入口通过 `callHostJavaScriptFunction` 传递结构化参数，不把参数嵌入调用源码。
- 最终脚本结果和外部 listener 的文本协议在 Rust 边界序列化一次，不在 JS / Rust 之间反复编码。

转换有两个明确的输入契约：

- 公共工具参数只接受普通 JSON 数据：拒绝访问器、隐藏属性、symbol、数组空位和额外属性、
  非有限数值、非普通对象和循环。读取自有属性描述符，拒绝访问器时不执行 getter。
- 生命周期函数返回值和其他通用绑定保留 JSON 的 `toJSON`、日期、包装值及数组空位语义。
  不可表示的顶层值、BigInt、循环、getter 错误和超出限制的数据明确报错。

通用转换识别并取走已经验证的工具参数快照，不再遍历其中的字段。这两个契约分别服务工具
参数和通用绑定，不使用策略开关选择转换路径。
Host 对象写入使用 own data properties，`__proto__` 不触发原型 setter。
待完成 Promise 上限为 4096；转换最大深度为 128，单次参数列表节点预算为 1,000,000。
转换结束后再次检查 Promise 数量，防止 getter 重入绕过上限。

重复完成或已取消 request id 不重新调用完成函数；取消只释放指定 scope 的句柄。
定时器使用独立 scope，支持 `clearTimeout`、`clearInterval` 和所属会话清理。
清理 interval 的待完成 timeout 同时移除 interval 状态，不创建全局定时器 callback。

## Native 所有权与实现边界

```text
SDK toolCall / Tools.*
  → 必须存在的 host 快照函数：在 host 边界严格读取参数
  → 私有 OwnedJson：持有最终 Rust 树，绑定只取走一次
  → JsEngine / 工具宿主：移动参数，宿主任务调度到常驻 worker
  → 结果队列：移动结果到原 QuickJS 所属线程
  → writer：立即创建完整的普通 JS 对象和数组
  → 完成 Promise，回收 Rust 结果中的可复用分配
```

普通 `toolCall` 的工具参数在 Native 边界严格读取一次。部分生成的严格 SDK 方法仍会先
调用 JS 版 `__operitSnapshotJsonObject` 做参数预校验；Chat 延迟流也会提前快照，防止首次
拉取前调用者修改请求。JS 版还服务 Web host，因此需要保留。SDK 预校验会带来一次额外
遍历，不应将 Native 边界的优化表述为所有公共 API 都只遍历一次。

QuickJS 句柄和原始地址只在所属线程使用；跨 worker 的是拥有所有权的 Rust 值。
返回值在 Promise 完成前已完全构造，后续字段读取直接访问普通 JS 对象。
工具的业务执行沿用 `ToolExecutionPolicy::Direct`；上述会话引用和 Promise 清理
属于运行时资源管理，不会触发 AI 工具生命周期钩子。AI 发起的工具调用仍走其正常流程。

Native 代码职责分布：

- `javascript_runtime.rs`：唯一构造入口、绑定注册、Promise 句柄、所属线程与任务投递。
  创建时捕获 intrinsics 并安装快照函数；测试使用同一构造入口。
- `javascript_values.rs`：通用 JSON 值语义、共享深度和节点预算、识别已验证的快照。
- `javascript_values/snapshot.rs`：严格快照、一次性所有权交接和 Rust 缓冲复用。
- `javascript_values/writer.rs`：立即将 Rust 值构造成 JS 对象，处理 own property 和句柄释放。
- `javascript_allocator.rs`：QuickJS 运行时内的小块分配复用，不替换全局 Rust 分配器。

Rust 缓冲池每个引擎的保留预算为 1 MiB；QuickJS 小块空闲池的总上限为 512 KiB
（含块头），单类保留的 payload 上限为 128 KiB。这些限制针对空闲复用内存，
不是引擎的总内存上限。缓冲复用不作为业务结果缓存，每次调用均重新读取并覆盖数据。
通用转换、严格快照及输出构造都保留显式深度和节点限制。

## 模块资源与明确失败

执行会话持有明确的资源 owner：Compose 页面快照、绑定包的资源 host、或执行 host。
CommonJS 读取仅使用该 owner。资源不存在使用结构化 null 表示；真实读取错误直接报错，
不转成空字符串。Compose render、rerender 与 action 继续使用不可变页面快照，避免重入包管理锁。

配置目录直接返回绝对 VFS 路径，创建或权限失败直接抛错。
CryptoJS 和 pako 不解析错误字符串；AES 失败不会产生空明文。
Java 包装器直接传递参数数组并包装明确的对象句柄，不捕获字段错误后猜测其他成员类型。

工具 host 的 `execute_tool_call` 只提供最终结果。请求 `onIntermediateResult` 在提交前明确报错，
不伪装流式工具执行；脚本自身的 `sendIntermediateResult` / `emit` 能力保持独立。

已归档的 `plugins/types-v1` 保留原始来源快照，不参与当前运行时绑定。
当前 SDK 和 `plugins/types` 已删除整个旧命名空间及其算法类型。
二进制结果的显式 base64 / binary handle 属于数据契约；公共 API 版本路由与 v1 语义适配器
也不属于旧传输协议。

## 检查

```sh
node --test tools/tests/javascript_bridge_structured.test.mjs \
  tools/tests/javascript_bridge_hosts.test.mjs \
  tools/tests/javascript_bridge_operations.test.mjs \
  tools/tests/plugin_bridge_contracts.test.mjs \
  tools/tests/toolpkg_public_api.test.mjs \
  tools/tests/toolpkg_api_compatibility.test.mjs \
  tools/tests/compose_dsl_navigation.test.mjs \
  tools/tests/workflow_ui.test.mjs \
  plugins/tools/plugin_loading.test.mjs
```

测试执行实际 SDK、完整生产 bootstrap、内嵌库和 Web host 值注册表，覆盖无 JSON 文本、
应用字符串保真、明确失败、对象安全、scope 取消、重复完成、重入上限和会话引用释放。
Rust `cargo check --tests` 检查 host trait 实现、运行时与工具 host 调用方。
Node 执行 Web host JavaScript 不等同于真实浏览器 / wasm 集成测试。

手动 benchmark 只测量结构化链路，必须显式提供规模：

```sh
OPERIT_JS_BRIDGE_BENCH_SIZES=64,4096,65536 \
OPERIT_JS_BRIDGE_BENCH_ITERATIONS=500 \
cargo test --release --manifest-path core/Cargo.toml -p operit-js-bridge \
  bridge_roundtrip_benchmark -- --ignored --nocapture --test-threads=1
```

基准涵盖 engine owner、工具调度、Promise jobs 与 SDK 包装，不包含真实业务 I/O。
历史协议比较不再是可执行测试路径，也不能当作本次修改的性能结论。

固定结构记录的基准可分别观察额外 Rust 深拷贝、所有权转移和完整读取：

```sh
OPERIT_JS_BRIDGE_BENCH_ITERATIONS=2000 \
OPERIT_JS_BRIDGE_BENCH_ROWS=256 \
OPERIT_JS_BRIDGE_BENCH_MODES=host,hostOwned,hostReadAll \
cargo test --release --manifest-path core/Cargo.toml -p operit-js-bridge \
  bridge_payload_benchmark -- --ignored --nocapture --test-threads=1
```

每条记录包含 id、name、enabled 及嵌套的 prompt、tags。`host` 包含测试 echo 工具的
额外 Rust 深拷贝；`hostOwned` 移动所有权返回；`hostReadAll` 在后者基础上每次校验全部
字段。输出是去掉一轮预热后五轮「每次调用平均耗时」的中位数，不是单次调用的 p95。
Native 值转换单元测试另有忽略的 `owned_value_codec_benchmark`，用于分离输入、输出和
Rust 深拷贝成本；它不模拟完整调用中的缓冲池回收过程，不能替代整条链路的基准。
