# DSL JS-owned state/memo 验证（2026-10-10）

## 本次范围

保留插件现有 JS API；CoreLink 本身不改动，继续通过应用级 Stream / ReverseStream 传输 retained 节点更新。

- retained 会话的初次、普通、异步中间及最终响应不含 `state` / `memo`，不读取这两个快照 getter，也不在 Rust 中比较整份快照。
- Flutter 只在会话首次初始化发送初始 state/memo；后续宿主输入以 `__operit_input_state` 更新，不以 Dart 快照覆盖 JS 状态。
- 同 execution context 重新执行脚本保留 live bundle、ref、memo 和 Promise；不同 context 创建独立 bundle。
- 显式非 retained 的快照调用仍维持原契约。这是单独的调用模式，不是错误重试或降级路径。
- UI 属性、actionResult、导航命令仍要经过结构化跨边界转换；不能将本改动理解成所有通信均零拷贝或完全没有任何 JSON 使用。

## 验证中修正的问题

1. 宿主 input 写入 stateStore 后没有触发依赖失效，已接入与 useState 相同的 stateChanged 通知。
2. 输入更新可能复用不同 execution context 的 bundle，已先验证 context 身份。
3. 同一个 compiled entry 的完整 render 需要刷新环境相关表达式，已显式失效 root；普通 action 不作无条件整根失效。
4. 原生 DSL 测试在 compile_main_script 阶段触发 QuickJS 栈上限。Host 工作线程原已保留 16 MiB 栈，现为这些线程明确配置 4 MiB 引擎栈限额；不改变非 Host 管理线程的默认限额，不取消栈保护。
5. 导航与 plan_mode 测试启动器补齐生产运行时依赖；Flutter fixture 按首次初始化与后续合并输入建模。

## 检查结果

| 范围 | 结果 | 说明 |
|---|---|---|
| Node 五组回归 | 27/27 | state/memo、导航、WebView bridge、document-start 静态契约、实际 plan_mode 屏幕 |
| 原生 QuickJS DSL | 8/8 | 包括新增的 state/memo 不导出、同会话重载、Host 输入更新 |
| 原生关闭/重开专项 | 1/1 | 显式 destroy 后拒绝旧 action；新引擎 revision=1，count/ref 从初始值开始 |
| JS bridge 全套串行 | 70/70 | `--test-threads=1`；后续专项增加 destroy 断言并单独重新通过 |
| JS bridge 全套默认并行 | 68/70 | `async_tool_call_yields_to_ready_javascript_promise` 和 `javascript_timer_can_win_race_against_async_tool_call` 的时序断言失败；未修改断言，未据此宣称并行全绿 |
| ToolPkgManager 生命周期 | 8/8 | 最终 lease 释放、容器清理、并发获取与所有权 |
| Flutter 定向回归 | 56/56 | 输入菜单、侧栏、action scheduler；包含后续命令不再发送 state/memo 的断言 |
| retained 局部生成探针 | 通过 | compiled 20/100/500 行：每次编辑 rootRuns=0、expressionRuns=2、childChecks=0；非可分离写法仍可能重执行 root |

新增 JS 验证包括 4 MiB state、禁止遍历的 enumerable getter、循环 memo、Promise、函数引用、嵌套修改与删除、异步 action 中重新载入、action 拒绝后的继续操作。回调从实际提交的节点记录读取，测试不通过额外 render 偷偷刷新回调。

## 复现命令

```powershell
# 仓库根目录
node --test tools/tests/compose_dsl_session_state.test.mjs tools/tests/compose_dsl_navigation.test.mjs tools/tests/compose_dsl_webview_bridge.test.mjs tools/tests/compose_dsl_document_start_hosts.test.mjs tools/tests/tui_compose_plugin_screens.test.mjs
node tools/performance/compose_dsl_retained_probe.mjs --require-local-generation
$env:RUSTFLAGS='-Awarnings'
cargo test --manifest-path core/Cargo.toml -p operit-js-bridge --lib -- --test-threads=1
cargo test --manifest-path core/Cargo.toml -p operit-plugin-sdk --lib toolpkg::ToolPkgManager::tests -- --test-threads=1

# apps/flutter/app 目录
fvm flutter test --no-pub --no-test-assets test/chat_input_menu_presentation_test.dart test/chat_sidebar_tabs_test.dart test/compose_dsl_action_scheduler_test.dart --reporter expanded
```

## 尚不能据此宣称的结果

- 未做全平台实机验收，也没有新的真实应用端到端首帧耗时。
- Node 的待完成 action 重载测试不等价于原生关闭页面时所有外部异步工具都已被取消。
- state/memo 是会话内存，不是持久化存储；释放最后的引擎所有权后重新打开页面应重新初始化。
- 未测试所有第三方插件；现有公共写法无需迁移，不代表任意插件行为均已覆盖。

## 后续定位：到底哪里消耗调用栈

使用当前 `core/target/debug/deps` 的 rquickjs 库做独立分阶段探针，线程栈为 16 MiB，分别设置引擎限额。插件样本仅 167 字节；添加当前运行时包装后为 20,617 字节。

| 引擎栈限额 | QuickJS new Function（原始/包装） | Acorn.parse 原始脚本 | Acorn.parse 包装脚本 |
|---|---|---|---|
| 1024 KiB | 均通过 | 栈超限 | 栈超限 |
| 1200 KiB | 均通过 | 通过 | 栈超限 |
| 1400 KiB | 均通过 | 通过 | 通过 |
| 1600/1800/2048 KiB | 均通过 | 通过 | 通过 |

同样运行完整 OperitComposeCompiler.compile，其结果与上面的 Acorn.parse 一致。这是当前样本在独立调用环境下的结果，不是生产所有脚本的最低安全限额或精确峰值测量。

错误直接落在 `ToolPkgComposeDslCompiler.js:45` 调用的 Acorn 解析过程，栈顶映射为：

```text
parseExprSubscripts (acorn.js:2855)
  parseExprAtom (3020)
    parseLiteral (3159)
      next (5405)
        nextToken (5444)
          readToken (5453)
            getTokenFromCode (5700)
              finishToken (5546)
                updateContext (2470)
                  update.call(...)
```

本地 QuickJS Debug 目标文件中 `JS_CallInternal` 的函数序言包含：

```asm
mov eax, 0x3860
call __chkstk
sub rsp, rax
```

即每次进入该函数先预留 14,432 字节（约 14.1 KiB）的固定帧，此外还有保存寄存器、返回地址和函数局部值/操作数的动态 alloca。QuickJS 的 JS 函数调用会再次进入该解释器函数；Acorn 递归下降解析使这些帧叠加。因此 1 MiB 并不意味着能够容纳很深的 Acorn 调用链。

定位结论：不是 state/memo 大小，不是 Flutter/CoreLink，不是已确认的无限递归，也不是当前样本的 QuickJS 原生语法编译失败；直接触发点是 Acorn 的 JS 解析调用链，当前 Windows Debug 解释器的大固定栈帧显著放大了开销。包装源码加重需求，但删除包装本身不足以让这个样本在 1 MiB 下通过。Release 和其他平台的帧大小尚未测量，不能照搬此数值。

临时复现程序及完整输出在 `target/dsl-stack-diagnostic/`，未修改生产逻辑。
