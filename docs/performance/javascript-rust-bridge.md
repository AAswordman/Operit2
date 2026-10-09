# JavaScript ↔ Rust 桥接优化

验证日期：2026-10-09。实现基于 `upstream/main` 的 `aaf216b4`。

## 改动范围

### 不再拼接回调源码

`HostJavaScriptRuntime::callHostJavaScriptFunction` 提供直接调用全局函数的能力。
原生 QuickJS 与 Web QuickJS 均覆盖该接口，通过引擎调用 API 传递参数。
普通异步回调、定时器和取消会话不再需要动态构造源码、转义完整结果并 `eval`。
找不到函数或属性不可调用时仍忽略；回调异常仍作为 Host 错误返回。

第三方 Host 可以继续编译：接口提供与旧协议兼容的源码回退实现。
因此“消除 eval”是内置 Host 的保证，不是第三方 Host 的默认保证。

### 原生结构化工具通道

原生运行时可注册 `__operitNativeCallToolStructured(scope, type, name, params)`：

1. 在 JS 所属线程将参数转换为拥有所有权的 `serde_json::Value`。
2. 创建原生 Promise，把 resolve/reject 句柄留在引擎内部的请求表中。
3. 用请求 ID 和 Rust 数据提交工具任务；JS 句柄不跨线程。
4. 工具任务完成后以 Rust 值送回引擎所属线程。
5. 直接创建 JS 数据并完成 Promise，不再 stringify/parse 整棵请求或结果树。

SDK 的公共 `toolCall` 包装保留原有参数规范化、执行上下文激活、引用计数与
延迟释放顺序。插件继续使用 `await toolCall(...)` / `await Tools.xxx(...)`。
SDK 仍浅复制参数；这是既有行为，并非将任意 JS 对象按引用交给后台 Rust。

工具结果保留既有 `success` / `message` / `data` / `dataType` 信封。
业务失败仍先作为正常结果信封完成原生 Promise，再由既有公共 JS 结果解析器抛错。
提交失败、非法参数转换和结果转换失败才是桥接错误。

Web 暂不声明结构化 Promise 能力；普通工具请求和流式请求仍使用旧字符串协议，
但结果回调已经直接调用。SDK 的 `onIntermediateResult` 继续走旧流式入口。
二进制仍使用现有 base64 / 大数据句柄协议，没有新增零拷贝 buffer 通道。

### 参数语义与边界

支持 null、bool、有限 JS number、string、array 和 enumerable own object keys；
遵循 JSON.stringify 的核心值语义：

- 对象中的 undefined、function、symbol 字段省略，数组对应元素变成 null。
- NaN / Infinity 变成 null；顶层不可序列化值按既有工具协议转成空对象。
- `toJSON`、Date、装箱基础类型、getter、代理数组按 JSON 的相应规则处理。
- BigInt 和循环引用报错；允许共享但不循环的子对象。
- 超出 JS 精确整数范围时，少量标量走最短十进制数值转换，以保持原 JSON 协议
  的 `serde_json::Number` 语义；不会因此序列化整棵参数树。
- 结果对象/数组创建 own data properties；`__proto__` 或继承索引 setter
  不会导致原型修改或丢失结果元素。

限制：转换深度最多 128，单次值转换最多 1,000,000 个节点；每个原生运行时最多
4,096 个待完成的结构化 Promise，含 getter/toJSON 重入创建的请求。
字符串没有新增单独字节上限，仍受进程可用内存约束；这不是无限内存安全保证。
这些转换限制会拒绝旧协议可能接受的极端深/宽数据，调用者需要分块处理。

取消按 execution scope 释放内部 Promise 句柄；迟到/重复结果忽略，不会恢复已取消会话。
取消不等于中断后台工具的业务执行，也不会主动 reject 已取消会话留下的 JS Promise。
运行时销毁前释放所有 persistent 句柄和捕获的转换内建函数。
结果超过转换限制会 reject，而不是删除句柄后永远留下一个未完成的 await。

### 复用唤醒执行器，不改变阻塞工具的隔离

新增 opt-in `scheduleHostRuntimeCooperativeAsyncTask`，原生实现使用两个常驻 owner
线程和 LocalSet。`!Send` Future 在 owner 线程创建并始终由该线程轮询；任务工厂
panic 不会终止整个 worker。共享 I/O runtime 仍为 process lifetime，子任务不会
因父请求完成而关闭。

目前只有 `operit-js-detached` 后续任务唤醒使用此通道，保留原有唤醒合并逻辑。
工具执行仍走原来的独立任务线程，因为工具可能阻塞、依赖 TLS 或线程隔离。
没有全局把工具替换成 tokio::spawn，也没有宣称所有异步工具都已线程池化。
第三方调度器默认委托既有异步调度实现；受限平台的既有本地执行器同样保持兼容。

## 正确性验证

```sh
cargo test --manifest-path core/Cargo.toml -p operit-js-bridge --locked
cargo test --manifest-path hosts/common/operit-host-native-scheduler/Cargo.toml --locked
cargo test --manifest-path hosts/common/operit-host-native-scheduler/Cargo.toml --no-default-features --locked
cargo test --manifest-path hosts/common/operit-host-native-scheduler/Cargo.toml --features esp32-compat --locked
cargo check --manifest-path apps/cli/Cargo.toml --locked
node --test tools/tests/javascript_bridge_structured.test.mjs \
  tools/tests/plugin_bridge_contracts.test.mjs \
  tools/tests/toolpkg_public_api.test.mjs \
  tools/tests/toolpkg_api_compatibility.test.mjs
```

Node 公共 API 测试需先在 `plugins/packages/buildin/workflow` 安装锁定依赖。
新增测试覆盖：大文本、不经过工具 JSON 文本、JSON 值语义、业务失败、非法参数后恢复、
二进制结果、重复完成、按 scope 取消、待完成数量上限、销毁清理、结果超限拒绝、
own data properties、worker 复用、`!Send` 线程亲和性和工厂 panic 后继续调度。
Node 测试直接执行 SDK 源码，检查旧 Host 和流式回退、引用释放时序、失败释放及
乱序完成的调用上下文。

原生测试与 CLI 构建通过。Web runtime 文件在独立 crate、`quickjs-wasm-rs 3.1.0`、
`wasm32-unknown-unknown` 目标下类型检查通过；并未执行真实浏览器集成测试。
完整 Web workspace 仍受既有平台边界 guard（MountRegistry 的 std::fs、MCPBridge
的 std::time）与 mio 的 wasm 目标依赖问题阻塞。干净旧 main 的 guard 也报同样违规。
不能将独立类型检查描述为完整 Web 构建通过。独立类型检查可按如下方式复现：

```sh
ROOT="$(pwd)"
CHECK="$(mktemp -d)"
mkdir -p "$CHECK/src"
cat > "$CHECK/Cargo.toml" <<EOF
[package]
name = "operit-web-js-runtime-check"
version = "0.1.0"
edition = "2021"
[dependencies]
anyhow = "1"
quickjs-wasm-rs = "=3.1.0"
serde_json = "1"
tokio = { version = "1", default-features = false, features = ["sync"] }
operit-host-api = { path = "$ROOT/core/crates/foundation/host-api" }
EOF
printf '#[path = "%s/hosts/web/src/javascript_runtime.rs"]\npub mod javascript_runtime;\n' \
  "$ROOT" > "$CHECK/src/lib.rs"
cargo check --manifest-path "$CHECK/Cargo.toml" --target wasm32-unknown-unknown
```

SDK 包完整测试在当前分支与干净旧基线均为 37 成功、3 失败：
`destroysEveryContextOwnedByContainer`、`rejectsContextOwnershipMismatch`、
`retainsExecutionEngineUntilEveryLeaseIsReleased` 缺少注册 container 的前置条件。
串行运行也有相同失败；本次没有顺手修改无关生命周期测试。

## Release 往返基准

```sh
cargo test --release --manifest-path core/Cargo.toml -p operit-js-bridge \
  bridge_roundtrip_benchmark -- --ignored --nocapture --test-threads=1
```

环境：本机 Apple M5 / arm64、Rust 1.95.0、Release；使用仓库锁定的 rquickjs
及其 core/sys 0.12.0（core workspace 锁文件）。ASCII 单字符串参数与 echo 结果，每组先预热，
取五批 × 500 次串行调用的批均值中位数。包括真实引擎 owner 线程、工具任务调度、
唤醒、Promise jobs 与公共 SDK 包装，不包括业务工具 I/O。

同一个测试进程中对比三条路径；baseline 使用测试适配器重建旧式 JSON + 回调 eval
和逐次唤醒线程调度，不是三个独立 checkout 的整应用启动耗时。

| payload | 旧式 JSON + eval | JSON + 直接回调 | 结构化 Promise |
| --- | ---: | ---: | ---: |
| 64 B | 42.31 μs | 35.49 μs | 29.90 μs |
| 4 KiB | 78.24 μs | 50.65 μs | 31.15 μs |
| 64 KiB | 600.98 μs | 261.94 μs | 38.72 μs |

观察到的大文本收益主要来自避免转义/拼源码/eval 与整树 JSON 文本往返。
不同运行轮次受其他构建、系统负载和线程调度影响，绝对耗时有波动；测试不设
耗时断言。不能推论真实网络/文件工具或整个插件能获得同等加速。

后续应另行设计工具非阻塞能力声明、批量请求和二进制 buffer/句柄所有权，
而不是直接把所有工具塞进常驻本地执行器。
