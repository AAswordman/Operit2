# JavaScript 调用上下文归属

## 问题与修复边界

诊断日志中的 `Module read requires an active resource owner` 来自共享 JS 桥接层。
一次页面动作在等待 Host 结果后恢复时，JS 已经恢复调用 ID，Rust 却只恢复了中断归属，
没有恢复该调用的模块资源能力。上一个引擎执行步骤结束时已经清空资源上下文，
因此恢复后的首次 `require` 会失败。

本次修复位于 `operit-js-bridge`，沿用现有 Host 与 JS 调用协议，不更改插件公开 API。
页面使用创建该调用时捕获的资源快照；包运行时使用其绑定的资源 Host。
没有有效调用身份时，资源访问仍然明确失败。

## Commit 历史与链路演进

| Commit | 日期 | 与本次问题相关的变化 |
| --- | --- | --- |
| `ea96a90d5` | 2026-10-07 | 将调用拆成短状态任务，在等待异步结果时释放引擎状态，允许同引擎 IPC 与其他调用推进 |
| `0c2e06f49` | 2026-10-09 | 引入显式 `JsTextResourceSource`，模块读取必须具有资源拥有者 |
| `8c3701890` | 2026-10-23 | JS 调用切换通过 `__operitNativeActivateCall` 通知原生层，但该入口只切换中断归属 |
| `7b369bc74` | 2026-10-24 | Promise 订阅按精确调用 ID 保留归属，主调用结束后仍可保留后台工作 |
| `a81dcf259` | 2026-10-27 | 引入中断作用域和有界取消清理，避免一个调用的取消影响其他调用 |
| `72c6240b8` | 2026-10-28 | ToolPkg 的包身份和执行 Host 绑定到其运行时，IPC 使用明确拥有者 |
| `432945f6b` | 2026-11-14 | 统一异步编译和原生执行身份，续体继续使用现有调用激活入口 |

这些变化已经提供了调用注册表、资源能力和准确的 JS 身份切换。
缺口是原生激活入口未同时选择完整调用上下文；本次将这三个部分接通。

## 归属规则

`JsCallContext` 是一次调用的完整能力记录：执行 Host、资源来源、环境状态、
中间结果回调、执行监听器和中断信号。原生层只保留一个当前上下文
`CURRENT_CALL_CONTEXT`，不再分别维护 Host、资源、环境和回调的线程本地副本。

`CURRENT_ACTIVE_CALL_CONTEXTS` 保存本次引擎步骤可以访问的调用：
正在等待主结果的调用，以及仍持有 Promise 或定时器的后台调用。
激活入口按精确 ID 取出其中一条记录，同步选择调用能力与取消预算。
空 ID 或未知 ID 清除当前能力，不改变其他调用的注册记录。

引擎步骤开始时只安装活动调用表，不预先授权引擎默认 Host。
具体 JS 回调必须通过现有身份切换选择自己的能力。
模块读取使用当前上下文的 `textResourceSource`；
按 ID 路由的环境、执行事件和中间结果使用同一个调用表中的记录。

## 生命周期

| 阶段 | 归属和释放方式 |
| --- | --- |
| 创建调用 | 捕获 Host、环境拥有者和资源来源，注册完整 `JsCallContext` 后执行插件入口 |
| 等待 Host / IPC | 调用保留在 `pendingScriptExecutions`，释放引擎状态，当前能力随步骤退出而释放 |
| Host Promise 完成 | Host 传输按所属 ID 恢复 JS 身份；Promise 续体再按订阅者 ID 激活完整原生上下文 |
| 定时器 | 注册时保留拥有者，触发时按该 ID 激活；取消和完成释放引用 |
| 页面动作 / effect / 主题 / WebView | 沿用 Compose 的动作派发和回调恢复链；异步恢复后通过同一个激活入口选择资源快照 |
| 并发重渲染 | 新调用可以更新页面的后续资源快照；已挂起动作的上下文仍持有原来的 `Arc` 快照 |
| 主结果完成 | 在发布主结果前检查并保留后台引用；后台调用继续使用原 Host、环境、回调和资源来源 |
| 后台工作完成 | 最后一个引用释放后，在所属调用下执行清理，再移除 JS 与原生保留记录 |
| 取消 | 使用已有 100 ms 清理预算，清理订阅、定时器和被拒绝 Promise 的 `finally`；清理期间仍选择被取消调用的能力 |
| 嵌套执行 / 步骤结束 | `ExecutionScope` 成对保存、恢复当前能力、活动调用表、注册参数和中断作用域，避免跨引擎能力残留 |

`cleanupCallSession` 在执行订阅清理前先安装被清理调用的 JS 身份与运行时，
结束时恢复仍存活的外层调用。即使清理抛错，`finally` 仍会清理定时器、移除会话并恢复身份。
取消当前调用时保留原来的父调用恢复规则。

原生 `advanceJavaScriptExecutions` 在完成终态处理和取消清理后才退出作用域，
保证清理期间调用表仍然存在；外层任务不再额外清空刚恢复的上下文。
包注册也显式创建完整上下文，注册专用参数在作用域退出时成对恢复。

## 回归验证

`src/javascript/tests/JsCallContextTests.rs` 使用真实原生 V8 引擎和现有 Host 测试设施，
覆盖八个场景：

1. 精确切换 Host 与资源；空、未知或已退休身份不能获得其他调用能力。
2. 嵌套作用域在切换到其他引擎注册表后仍能恢复外层能力。
3. 页面动作等待 Host 后首次加载模块，并发重渲染不改变旧动作快照。
4. 定时器回调首次加载页面模块。
5. 页面 effect 安装的主题订阅在 Host 等待后加载模块。
6. 保留的 WebView JS 接口在 Host 等待后加载模块。
7. 主结果已返回、期间执行过其他调用后，后台 Promise 仍能加载所属包模块。
8. 挂起动作被取消后，订阅清理和 `finally` 使用原快照，后续调用仍正常。

页面用例同时断言模块读取不重新进入插件管理 Host，避免引入管理器重入。
修复前，Host 等待与重渲染、定时器和后台加载用例均失败；前两项复现了用户日志中的原始错误。

JS 生命周期测试补充普通清理、取消清理、清理异常以及父调用恢复，
并与 Promise 归属、结构化桥接、Host 和 Web Worker 传输测试一起执行。
本次保留这些回归用例与本地验收命令，不新增或修改工作流。

本地命令（从仓库根目录执行；PowerShell 先设置 `$env:RUSTFLAGS = '-Awarnings'`）：

```text
cargo test --locked --manifest-path core/Cargo.toml -p operit-js-bridge --lib call_context_tests -- --test-threads=1
node --test core/crates/plugin/javascript-bridge/tests/promise-session-lifecycle.test.mjs tools/tests/call_continuation_context.test.mjs tools/tests/javascript_bridge_structured.test.mjs tools/tests/javascript_bridge_hosts.test.mjs tools/tests/web_javascript_runtime.test.mjs
```

2026-11-20 本地验证：原生新增用例 8 项通过；桥接库扩展回归 155 项通过、
4 项手动性能测试忽略、1 项既有失败单独排除；上述 JS 组 50 项通过。
扩展 Node 测试中另发现两项失败，均用 `HEAD` 原始引擎和 bootstrap 复现：

- 角色卡已打包页面 fixture 在 `renderManagement` 对缺失字段调用 `.map`；
  原生 `compose_dsl_character_card_webview_interface_survives_native_commits` 也有相同的既有失败。
- Java 包装器 fixture 未建立调用会话，触发 `ToolPkg API requires an active execution call id.`。

这些既有失败没有通过放宽调用身份要求来绕过。
本次未构建应用安装包，也未执行 Android 实机或原始子代理包的端到端复测。
