# Android system shell 卡死排查记录

日期：2026-11-22。源码基线：`a7920ad59`。
反馈材料：用户提供的 `D:/Downloads/Operit-shell卡死反馈.md`；本次没有取得其中列出的原始日志文件或反馈设备的安装包。

前面的故障分析与行号描述修改前的源码。后面的“事件驱动改造”记录本次实现及验证，不能把历史阻塞链路当作修改后的行为。

## 结论与证据边界

反馈可以支持“调用未正常完成，最终收到 JS 取消错误”。它没有证明 WebSocket 已建立、命令已写入 PTY，或实际进入了 `terminal_observe`。目前不能将本次故障的根因确定为 `terminal_observe` 没有观察超时。

当前源码已经有观察期限，但期限没有覆盖终端准备阶段。准备阶段还包含持锁的同步执行和同步 Binder 调用，具有长时间阻塞其他终端操作的条件。这是源码中能确认的缺口，尚未证明它触发了反馈中的四次故障。

## 反馈与当前实现的差异

当前超级管理员导出 `terminal`，通过 `Tools.System.terminal.run` 执行；SDK 直接调用 `terminal_observe`。回调和迭代器接口也使用 `terminal_observe`，不通过 `web_socket`。

对应源码：

- `plugins/packages/buildin/super_admin.ts:320`、`:348`、`:500`。
- `core/crates/plugin/sdk/src/terminal_runtime.js:14`、`:22`、`:44`。
- `core/crates/plugin/sdk/src/chat_runtime.js:10`。

反馈记录的是 `function=shell`，最后一条工具请求为 `tool=web_socket`。`shell` 入口本身可以来自较早版本；例如历史提交 `7e4991975` 的超级管理员仍有该入口。但该历史版本的终端 SDK 也调用 `terminal_observe`，因此仅用“旧版 shell 入口”不能解释 WebSocket 请求。需要核对实际运行的 App、SDK 和脚本内容；目前无法判定具体属于哪个版本或调用路径。

“执行终端命令”是调用终端 SDK 之前打印的日志，不能作为命令已经送到 Android 的证据。`get_terminal_info` 也只证明能力查询完成，不证明创建过可用会话。

`tool.execute.request` 在工具激活、参数准备及实际执行之前记录。只有该日志而没有完成日志，不能进一步定位到连接、读输出或某个具体操作。

对应源码：`core/crates/tool/services/src/tools/AIToolHandler.rs:1058`。

## 当前源码确认的缺口

### 1. 前台等待期限没有覆盖会话准备

调用顺序为：

```text
runManaged
  → startNamed
      → 取得 managedAdmission
      → availableSelectedSession
          → 枚举会话、解析工作目录、创建或复用会话
          → enterTerminalDirectory
              → 同步 executeInSession(cd, 120000)
      → openExecution
          → 调度用户命令
  → waitManaged
      → 此时才按 waitMs 建立观察期限
```

`enterTerminalDirectory` 为切换工作目录单独传入 **120000 毫秒**。它在复用已有会话时同样可能执行。该阶段一直持有 `managedAdmission`，而 `waitManaged` 的前台计时尚未开始。因此，插件默认的 20 秒并不构成整个终端调用的 20 秒期限。

同一个执行器的其他启动操作和 `closeSession` 也需要 `managedAdmission`；准备阶段阻塞时，它们会等待该锁。此锁由共享终端执行器持有，并非每个 JS 调用各自一把。

对应源码：

- `core/crates/tool/services/src/tools/defaultTool/standard/StandardTerminalStreaming.rs:380`、`:398`、`:408`、`:429`、`:474`、`:495`、`:666`。
- `core/crates/tool/services/src/tools/defaultTool/standard/StandardTerminalTools.rs:156`。
- `core/crates/foundation/host-contract/src/TerminalWorkingDirectory.rs:58`、`:65`。

该缺口解释了为什么不能把插件前台等待参数当作准备阶段的总超时，但不能单独证明反馈中的 49～270 秒都由切目录造成。

### 2. Shizuku 事务没有应用层等待期限，部分调用发生在会话注册表锁内

Shizuku 服务绑定使用 `ready.get(10, TimeUnit.SECONDS)`，但后续 `binder.transact(..., 0)` 是同步事务，代码没有为每次 start、poll、resize、close 指定应用层等待期限。

Android `listSessions` 在持有整个会话注册表的 Mutex 时，对 Shizuku 会话同步执行 `poll`。远端服务若未返回，其他需要此注册表的操作也会等待。服务端 `onTransact` 又以 `@Synchronized` 串行处理这些事务。

创建会话的提示符等待虽然设置了 10 秒，但它开始于进程启动事务返回之后；创建失败后的资源释放还可能再次同步调用 Shizuku close。因此，这个 10 秒也不是从创建入口到错误返回的完整期限。

对应源码：

- `apps/flutter/app/android/app/src/main/kotlin/app/operit/AndroidTerminalPrivileges.kt:68`、`:86`、`:116`。
- `apps/flutter/app/android/app/src/main/kotlin/app/operit/AndroidTerminalUserService.kt:29`。
- `hosts/android/src/terminal.rs:423`、`:746`、`:800`、`:2074`、`:2122`、`:2529`。

以上是条件性的阻塞路径；反馈未提供所选传输方式或 Binder 状态，不能据此确定设备当时使用 Shizuku，也不能确定远端实际未返回。

### 3. PTY 读取失败没有直接传入命令输出等待状态

PTY reader 遇到 EOF 或读取错误时记录日志并退出。命令输出状态仅包含字节队列和 Condvar，没有对应的读取结束或读取失败状态。

受管理命令以宿主执行超时 `0` 启动，前台观察超时后命令仍由会话持有。输出回调会定期枚举会话，能够检测到进程退出后会话不再存在的情况；但若读取通路失效而进程仍存活，reader 的退出本身不会直接结束命令等待，可能持续占用命令 worker。

对应源码：

- `hosts/android/src/terminal.rs:96`、`:829`、`:855`、`:1029`。
- `core/crates/tool/services/src/tools/defaultTool/standard/StandardTerminalStreaming.rs:271`、`:342`。

## 补充：长命令超时后，新建终端也卡住的路径

用户进一步指出“AI 执行长命令后超时，没有取消原命令，之后新开的终端也卡住”。这是有具体源码支持的复现方向，不需要先确定原反馈的 `web_socket` 归属才有调查价值。

1. `super_admin.terminal` 默认仅等待 20 秒。宿主命令调度实际传入执行超时 `0`，因此前台等待结束不会终止命令，也不会释放执行该命令等待逻辑的 worker。
2. 原会话被识别为忙时，下一条 AI 命令可以选择新一代会话。未结束的命令与新会话因而可能同时累积。
3. Android 没有覆盖 `TerminalHost.scheduleInSessionStreaming` 的默认实现；该实现把命令及其同步输出等待放在 `scheduleHostRuntimeTask` 上。Native scheduler 将它分派到共享阻塞池，当前默认容量为 16 个 worker。
4. 手动新建终端的 `RuntimeTerminalService.startTerminalPty` 也向同一调度接口提交 `start-terminal-pty`。阻塞池被长期等待的命令及其他工作占满时，新的手动终端创建也会排队。终端观察数限制为 32，不能据此保证这个 16 worker 池始终留有启动容量；独立的 PTY reader 服务容量还可能更早拒绝创建。

对应源码：

- `plugins/packages/buildin/super_admin.ts:348`。
- `core/crates/tool/services/src/tools/defaultTool/standard/StandardTerminalStreaming.rs:342`。
- `core/crates/tool/services/src/tools/defaultTool/standard/StandardTerminalTools.rs:110`。
- `core/crates/foundation/host-contract/src/lib.rs:2012`。
- `hosts/common/operit-host-native-scheduler/src/runtime_task_scheduler.rs:9`、`:714`。
- `core/crates/runtime/application/src/services/RuntimeTerminalService.rs:539`。

这确认了资源共享和可能耗尽的路径，尚未用堆积长命令的方式在反馈设备上复现。不能将它表述成“任意一个长命令必然卡住所有新窗口”；单条命令就使其他会话卡住时，仍应检查注册表锁、准入锁和同步 Binder 等路径。

取消能力并非完全缺失。正常前台等待结束时，插件返回 `sessionId`、`commandRunning=true` 和 `close` 提示；`input` 支持发送 Ctrl+C，`close` 用于终止并释放会话。若整个 JS 执行先被取消，正常返回结果可能来不及交给 AI，AI 不一定获得这个 sessionId。流观察的 `close(handle)` 仅分离订阅，与终止会话的 `closeSession(sessionId)` 也不是同一个操作。

因此，问题同时涉及运行命令的资源占用、调用取消后的任务可见性，以及取消操作是否能在阻塞时独立完成。需要保证显式后台命令的既有语义，同时让异常命令可发现、可终止，并避免其输出等待长期占用通用阻塞 worker。

## JS 取消错误的含义

普通脚本执行入口没有默认的整个异步调用完成期限；显式带期限的入口另行传入 timeout。调用 Future 被丢弃也会触发 `JsScriptRequestLease::drop`，进而取消对应执行。Native Promise 清理使用统一字符串 `JavaScript execution was cancelled`。

因此，反馈将此错误直接归为“JS 执行超时”证据不足。应查同一 callId 的 `kind=js_cancel`、`timedOut`、`runtimeAlive`，以及取消发生前的上层停止、失败或运行时释放日志。`timedOut=false` 本身也不足以确定是谁发起了取消。

对应源码：

- `core/crates/plugin/javascript-bridge/src/javascript/JsEngine.rs:249`、`:256`、`:391`、`:886`、`:916`。
- `hosts/common/operit-host-native-scheduler/src/javascript_engine.rs:408`。
- `core/crates/tool/services/src/tools/packTool/RuntimePackageManager.rs:649`。

## 已完成验证

运行以下现有检查，17 项全部通过：

```powershell
node --test tools/tests/terminal_streaming.test.mjs tools/tests/super_admin_terminal_routing.test.mjs tools/tests/android_terminal_privileges.test.mjs
```

其中包含 JS SDK 调用行为、超级管理员路由和 Android 桥接源码契约检查；不等同于 Android Rust 运行时或真实 Shizuku 会话验证。

ADB 当前连接的是 **PLK110**，安装 `app.operit` 的 `versionName=2.0.0`、`versionCode=49`。该设备不存在反馈指定的 identity 日志目录，型号也与反馈所述小米不一致，不能将其视为已确认的反馈设备。

在该设备上运行现有 `android_terminal_lifecycle.test.mjs`，1 项通过。检查使用生产 `system_shell_init.sh`，验证真实 Android 交互式 shell 的提示符、退出码、工作目录、环境保持和 Ctrl+C。它通过 ADB 启动 shell，没有覆盖 App 内的 Shizuku Binder、PTY 转发、终端准备和 JS 完整链路。

以上是最初只读调查阶段的验证；当时没有构建或修改业务代码。本次没有执行反馈材料中的系统配置写入命令。

## 继续确定本次根因所需的证据

1. 反馈设备的 App 版本、构建号、APK 来源，以及实际加载的超级管理员脚本；有条件时提供脚本摘要或内容以核对 `scriptBytes=47577`。
2. 原始 `operit.log`、`operit.log.1`、`toolpkg.log`、`toolpkg.log.1`、`client.log`，保留四个 callId 前后的完整上下文。需核实 `web_socket` 请求的实际操作、是否存在 `terminal_observe`，以及 `kind=js_cancel`。
3. 同期 logcat 中的 `OperitTerminal` 和 Android PTY 启动、提示符、reader 退出、Shizuku 绑定或事务错误。先通过启动日志确认实际 transport，再调查对应通路。

后续修复应围绕准备阶段的期限、短期会话预留、阻塞操作的宿主调度和读取终态传播展开。不能仅给 `next` 增加一个计时器：当前已有观察期限，而且同步准备或 Binder 阻塞可能发生在观察开始之前。需要保留前台等待结束后命令继续运行的现有契约，避免把正常后台任务当作异常杀死。

## 事件驱动改造

本次移除了“每条命令把同步输出等待放进通用阻塞池”的调度路径。`TerminalCommandExecution` 是唯一的命令状态机，Host 仅在提交、发送中断、最终释放资源时进入明确的同步 OS 边界。输出等待和中断后的 shell 就绪确认通过 Waker 挂起；有执行期限时，各阶段只注册一次异步截止计时器。同步 `executeInSession` 接口通过同一状态机的事件适配器保留兼容性，不再维护独立的旧命令解析循环。

具体变化：

- Android、Linux、共享 POSIX PTY、Windows 接入同一调度契约，删除各自的 `execute*PtyCommandInSession` 和 `cancelTimedOut*` 阻塞等待循环。
- 完整协议标记代表命令完成和初始 shell 就绪，删除额外等待 150ms 静默的判断。分片标记在下一次输出事件继续解析。
- `TerminalCommandBuffer` 在同一把锁下读取字节并注册 Waker，保留 EOF/读取失败状态；关闭会话或 reader 退出会唤醒没有输出的命令。最终标记与 EOF 同批到达时，先接受完整的完成标记。
- 中断确认期间保留会话忙碌状态；不持有会话表锁等待。无法确认恢复的会话明确关闭，避免把未恢复的 shell 当作空闲会话复用。进程关闭也在移除注册表条目、释放锁之后执行。
- Android 和 Unix PTY reader 通过 Tokio `AsyncFd` 等待内核 I/O 就绪；关闭使用通知唤醒，删除每会话占据一个 service worker 的读取路径和定时 poll。Windows portable-pty 仍使用其同步管道读端，读到的数据唤醒异步命令观察器。
- iOS/macOS ARM64 桥将同步命令执行与 20ms 信号量轮询改成原生会话队列的拥有型回调。最后一帧之后才交还 completion context；原生关闭也必须完成这个回调。ARM64 原始输入由原生队列接管，移除 Rust 的 1ms 写入重试 sleep。
- 浏览器使用 `SharedArrayBuffer` 和 `Atomics.waitAsync` 订阅输出、完成及关闭，删除 Rust 50ms 轮询。生产者注册时检查已保留的输出/结果，避免错过先到达的事件。
- Core 的 `waitManaged`、`next` 使用通知和一次截止计时器；移除输出回调中的周期性会话枚举。会话快照发布放到显式的同步 Host 边界，避免输出通知顺带执行同步 Binder 枚举。

主要实现位置：

- `core/crates/foundation/host-contract/src/TerminalCommand.rs`
- `core/crates/tool/services/src/tools/defaultTool/standard/StandardTerminalStreaming.rs`
- `hosts/common/operit-host-native-scheduler/src/fd_io.rs`
- 各平台 terminal Host 和 Apple `Arm64TerminalBridge.mm`
- `apps/flutter/app/web/runtime/src/operit_runtime_bridge.ts`

### 边界和后续验证

这次消除的是命令/中断的长期等待、Unix 读端的定时轮询，以及观察接口的周期性唤醒。进程创建、初始化提示符等待、明确调用的同步 Host 接口、写入背压、进程回收和 Shizuku Binder 事务仍有同步边界，不能宣称整个应用已不存在阻塞。尤其 `availableSelectedSession` 的目录准备仍走同步接口并持有准入锁；同步 Binder 的应用期限也没有在本次改造中解决。反馈中单次命令就让所有窗口停住的情况，仍需要日志和设备验证这些路径。

共享状态机测试覆盖 32 个静默观察器不申请等待 worker/轮询计时器、分片完成标记、超时后等待中断确认、静默关闭和完成标记与 EOF 的先后。Unix reader 测试使用 32 对真实 socket 验证超过 service pool 容量的静默读端能收到数据并由关闭事件退出。浏览器测试覆盖先注册通知、输出先到、完成和静默失败，且不创建轮询计时器。

`hosts/tests/terminal-contracts` 直接引用平台源文件进行本地 Rust 类型检查，避免为检查终端适配器链接整个 App/推理引擎。Windows/Linux 的共享状态机、平台源码、SDK/浏览器契约，以及 Linux 的真实 Unix I/O 和 PTY 测试均在本地运行；本次不新增 workflow。Apple 原生 smoke fixture 已迁移到新的回调 ABI，测试调用方也使用事件等待，没有保留生产旧桥供测试调用。

本次没有生成新 App/APK，也未在反馈设备验证完整 Shizuku 链路。源码检查、单元测试、跨目标类型检查和真实设备故障复现是不同层次的证据。
