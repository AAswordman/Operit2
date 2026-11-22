# Shizuku 识别、授权显示与 Android 命令排查

## 证据范围

用户提供的截图显示 Shizuku 13.5 已运行、应用管理列表为空，同时 Operit2 工具与权限页
显示 Shizuku 已授予。聊天消息包含插件无法执行 `dumpsys usagestats` 的转述，未包含
实际工具调用名、参数、退出码或 Android 异常。因此不能把聊天文字当作 Host 的原始错误。

本次对照 OP2、`D:/Code/prog/assistance` Kotlin 版、缓存的 Shizuku 13.1.5 依赖及
已有 release APK；未重建 APK。当前连接的本地设备没有运行 Shizuku，不是截图中
“Shizuku 已运行”的复现场景，不能据此确认原设备故障原因。

## 已排除：OP2 缺少 Shizuku Manifest 声明

两版均依赖 `dev.rikka.shizuku:api:13.1.5` 和 `provider:13.1.5`。Kotlin 版主
Manifest 显式声明 `moe.shizuku.manager.permission.API_V23`；OP2 由 Provider 依赖
合入该权限和 `moe.shizuku.client.V3_SUPPORT=true`。

已核对本地 release manifest 合并报告，并用 `aapt2 dump xmltree` 读取已有的
`apps/flutter/app/build/app/outputs/flutter-apk/app-arm64-v8a-release.apk`
（本地修改时间 2026-11-21）：最终 APK 包含上述权限、识别标记和
`rikka.shizuku.ShizukuProvider`，authority 为 `app.operit.shizuku`。
这只能证明已检查的产物，不能证明截图用户安装的 APK 内容。

上游 Shizuku 的 `ShizukuService.getApplications` 枚举 Android 用户及已安装包；有
权限配置记录的包会列出，否则按 V3_SUPPORT 与 API_V23 识别声明支持的应用。
因此“授权数为 0”和“整个应用列表为空”不是同一个现象；不能只用尚未请求授权解释
整个列表为空。需要核对故障 APK、本机/分身用户和实际 Shizuku/Sui 后端。

参考上游实现：

- https://github.com/RikkaApps/Shizuku/blob/master/server/src/main/java/rikka/shizuku/server/ShizukuService.java
- https://github.com/RikkaApps/Shizuku/blob/master/manager/src/main/java/moe/shizuku/manager/authorization/AuthorizationManager.kt

## 授权页的实际判断

普通 APK：`AndroidPrivilegeAuthorization.shizukuAuthorizationStatus` 先调用
`Shizuku.pingBinder()`，再调用 `Shizuku.checkSelfPermission()`；工具权限页直接将
快照的 `Satisfied` 显示成“已授予”。普通 APK 没有自己的 Shizuku 授权布尔持久缓存。

SDK 13.1.5 内部有 `permissionGranted` 成功缓存；缓存为 true 时
`checkSelfPermission()` 直接返回成功。已用缓存 JAR 的 `javap` 核对，且 Kotlin 版
也使用同一个 SDK 检查。因此这是状态检查的边界，不能单独认定为 OP2 独有根因。
“已授予”并没有验证 UserService 已绑定或实际命令可执行。

OP2 工具权限页在加载、应用恢复及申请完成后重新读取快照，没有订阅 Shizuku Binder
连接/死亡事件；页面保持打开时显示的仍是上一份快照。Kotlin 的 ShizukuAuthorizer
则注册了连接/死亡监听并发出状态变化通知。这是状态更新机制上的差别，但截图没有
同一时刻的底层状态，不能将它直接认定为该次误显示的原因。

外部 Root 模块客户端另有显式逻辑：Shizuku 槽位直接视为满足，以表示 Root Core
不需要重复申请该能力；这也不表示 Shizuku 管理器授予了客户端权限。截图同时显示
Root 未授予，与当前模块客户端的 Root/Shizuku 均满足逻辑不符，不能套用该解释。

已修正授权回调忽略 `grantResult` 的错误：拒绝时返回
`SHIZUKU_AUTHORIZATION_DENIED`。原代码返回 success 后页面还会重新读取权限，所以
该错误本身不能证明截图中“已授予”的来源。

## 已确认并修正：内置 AndroidUtils 命令走 proot

`core/crates/plugin/javascript-bridge/src/javascript/AndroidUtils.script.js` 原先使用
`Tools.System.terminal.hiddenExec` 执行 Android 命令。Android Host 的
`executeHiddenCommand` 固定选择 `PRIMARY_TERMINAL_TYPE="bash"`，结果的 terminal
为 `proot`；该路径不读取 Shizuku 授权、不绑定 Shizuku UserService。

改为通过 canonical terminal 创建显式 `shell` 会话，执行原命令，再关闭本次会话。
canonical 入口避免 v1 兼容终端将调用改写成 Linux 会话。返回类型仍为输出字符串；
非零退出码和超时现在抛出包含实际输出的错误，避免上层解析权限失败文本后返回空数据。

`Tools.System.shell` 是另一条 v1 API：当前源码已显式创建系统 shell，不走上述
AndroidUtils 入口。截图没有实际调用记录，所以此修正不能直接视为该插件已获真机验证。

系统 shell 在创建时选择已有 Root、Shizuku 或应用身份；已创建的会话保持所选传输。
Shizuku UserService 的绑定或 JNI 调用仍可能失败，必须查看实际异常。Kotlin 版
`DebuggerShellExecutor` 有自己的 Shizuku 命令执行链，不经过 OP2 的 proot 终端，
因此其行为不能直接证明 OP2 的每个 JS 入口都路由正确。

## 验证及故障设备所需证据

`tools/tests/android_utils_shell.test.mjs` 使用完整生产脚本进行 Node 行为检查，验证
显式系统 shell、公共 helper、输出/超时保留、权限失败、清理失败与并发会话归属。
Android CI 增加该检查及现有原生接线契约。检查不等同于设备端 Shizuku 集成验证。
本次运行上述行为检查以及 `android_terminal_privileges`、`android_root_commands`、
`android_runtime_contracts`，共 42 项全部通过；`git diff --check` 通过。
未编译 Kotlin/Rust 或 Flutter，未构建、安装新 APK，也未验证故障设备。

要确定截图故障，还需要同一设备上的实际 APK 版本和 Manifest、Android 用户、
Shizuku/Sui 后端、创建会话的 transport 及 `id` 输出、实际工具调用和完整异常。
可在该设备上保留以下日志进行定点核对：`ShizukuApplication`、`OperitTerminal`、
`buildAndroidSystemShellCommand`。避免将截图聊天中的诊断句作为原始错误归因。
