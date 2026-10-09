# JavaScript / Rust 结构化桥接

## 唯一 host 传输契约

运行时不再安装 `NativeInterface` 全局对象。插件使用 `toolCall`、`Tools.*`、
`ToolPkg.*`、Compose context、`CryptoJS`、`Jimp`、`pako`、`Java` / `Kotlin`、
`console` 和标准定时器 API。内部 `__operitNative*` 绑定由 SDK 管理，插件不应直接调用。

`HostJavaScriptRuntime` 必须实现：

- `registerHostJavaScriptJsonFunction`：同步传递拥有所有权的结构化参数和结果。
- `registerHostJavaScriptAsyncJsonFunction`：创建由 host 持有完成句柄的 scoped Promise。
- `settleHostJavaScriptPromise`：在所属执行器完成 Promise，释放 resolve / reject 句柄。
- `cancelHostJavaScriptPromises`：释放指定 scope 的待完成句柄。
- `callHostJavaScriptFunction`：直接调用 runtime 生命周期函数，不生成调用源码。

这些接口没有默认实现、能力选择开关或字符串 transport。原有 string / void callback
注册接口已删除；同步无返回值操作以结构化 null 表示完成，调用者不读取这个返回值。
同步能力保留同步语义，异步 API 使用同一套 host Promise 契约，不把所有能力强行改成异步。

## Promise 调用过程

1. SDK 验证公共调用形式，保留所属执行会话的引用。
2. Host 将参数转换成拥有所有权的 `serde_json::Value`，分配 request id，
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

- Native host 使用 rquickjs 的值转换、Promise 和 Persistent 完成句柄。
- Web host 的值快照和 Promise 注册表位于 `hosts/web/src/javascript_promises.js`，
  Rust host 通过函数调用和对象属性 API 传递数据。转换捕获原始内建函数，
  不依赖插件可修改的 JSON、Map 或 Array 方法。
- 业务 Rust 和 Flutter 使用统一 host 契约，不增加业务平台分支。
- Flutter owner 的外部消息协议仍由 Flutter host adapter 编解码，不向插件暴露 JSON transport。
- 脚本执行入口通过 `callHostJavaScriptFunction` 传递结构化参数，不把参数嵌入调用源码。
- 最终脚本结果和外部 listener 的文本协议在 Rust 边界序列化一次，不在 JS / Rust 之间反复编码。

转换保留对象属性、数组空位、`toJSON`、日期、包装值与有限数值的 JSON 值语义。
不可表示的顶层值、BigInt、循环、getter 错误和超出限制的数据明确报错。
Host 对象写入使用 own data properties，`__proto__` 不触发原型 setter。
待完成 Promise 上限为 4096；转换最大深度为 128，单次参数列表节点预算为 1,000,000。
转换结束后再次检查 Promise 数量，防止 getter 重入绕过上限。

重复完成或已取消 request id 不重新调用完成函数；取消只释放指定 scope 的句柄。
定时器使用独立 scope，支持 `clearTimeout`、`clearInterval` 和所属会话清理。
清理 interval 的待完成 timeout 同时移除 interval 状态，不创建全局定时器 callback。

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
