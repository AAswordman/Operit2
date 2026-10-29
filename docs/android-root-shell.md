# Android Root / su 封装

## 组件职责

- `AndroidRootCommandRouter`：自动/强制模式、自定义 su argv、设备 Root 迹象与
  实际授权区分、身份探测、执行路由。可以通过假后端进行 JVM 行为测试。
- `AndroidRootShell`：管理私有 libsu Shell，启用 mount-master，初始化上限 10 秒；
  每条 libsu 命令有独立 deadline，失败/超时后关闭失效 Shell，非 Root Shell 不缓存。
- `AndroidCommandProcessRunner`：直接 exec 和 Shizuku 共用的 stdout/stderr 并发读取、
  超时、关闭 stdin、清理进程和输出流。直接 exec / Shizuku 保留原始字节输出；
  libsu 仍使用其按行文本收集 API。
- `AndroidPrivilegeAuthorization`：显式用户同意 + 当前 UID 验证，而不是持久化
  `true` 就永远视为已授权。未同意时，状态查询不调用 su，以免打开界面就触发授权弹窗。

授权检查和截图默认使用 `RootAuto`，遵循同一套设置。自动选择会优先复查最近成功的
传输；首次探测遇到 KernelSU / APatch 的版本标识优先直接 exec，否则优先 libsu。
失败时只在**身份探测阶段**尝试另一种方式。版本输出、su 文件存在等仅是 Root 迹象，
实际授权必须满足 `id -u` 退出码为 0，且输出恰好为 `0`。

不会永久缓存授权成功或拒绝；撤销权限后下次状态查询会重新验证，拒绝后也能重试。
直接 exec 的用户命令在同一 su 进程内再次检查 UID，不允许降级成普通用户执行。
用户命令启动后不会换传输重试，避免有副作用的命令执行两次。

## 模式与自定义 su

已有 MethodChannel `hostOnboardingRequestPermission` 对 `android.root` 新增两个
**可选**字段；现有 Flutter 调用不传字段时，沿用已保存设置，首次默认为自动模式：

```dart
await channel.invokeMethod<void>('hostOnboardingRequestPermission', {
  'hostId': 'android',
  'requirementId': 'android.root',
  'rootExecutionMode': 'exec', // auto / libsu / exec
  'suCommand': "'/custom path/su' --flag",
});
```

这提供原生配置入口，尚未新增 Flutter 模式选择/自定义命令的设置界面。
自定义 su 按参数列表解析，支持带空格的路径、引号、转义；不会作为 shell 表达式执行。
修改模式或命令会清除旧的用户同意标志，必须通过本次授权探测确认新配置。
强制模式不会自动切换到另一种执行方式。自定义 su 不会悄悄替换成其他安装路径。
默认 `su` 在 PATH 不可执行时还会尝试存在的常见 su 绝对路径。

Owner `execute_privileged_command` 支持 `root` / `root_auto`，同时保留
`root_libsu` / `root_exec` / `shizuku`。显式的 `root_libsu` 和 `root_exec`
保持指定传输，不会自动切换。

授权请求总预算为 30 秒；已同意后的状态复查总预算为 10 秒，并放到后台线程。
RootAuto 单条命令的传输探测与实际执行共用调用者传入的超时预算。

## Rust Android 系统终端

Super Admin 的非 Linux/proot 终端走 `hosts/android/src/terminal.rs`，在创建系统 shell
会话时由 Rust 按 **Root → Shizuku → 普通 sh** 选择身份；TS 不负责这项路由。
Linux/proot 的 bash 启动链路保持不变。

- Root：已有用户同意后，重新用 `id -u` 验证直接 su argv，再通过该 argv 启动
  持久 PTY。遵循保存的自定义 su 路径和参数；不借用 libsu 的逐行命令会话。
  su 内再次验证 UID 为 0，防止以普通身份执行原本选定的 Root 会话。
- Shizuku：Root 当前不可用且已有 Shizuku 授权时，绑定专用 UserService。
  特权进程持有真实 PTY，双向普通管道通过 Binder 传给应用，避免跨身份直接访问
  devpts；调整窗口、查询退出码和结束子进程仍由特权服务处理。
- 普通 sh：两项授权均不可用时，以应用自身 UID 启动 `/system/bin/sh`。

终端创建不会主动发起新的授权申请。身份在会话创建时确定，后续命令不重新路由，
启动失败也不重放用户命令；授权变化后需新建会话。三种身份均保留 `cd`、环境变量、
标准输入和终端尺寸等会话状态。Shizuku 的默认工作目录从应用私有目录调整为
`/sdcard`，不携带应用私有动态库路径；显式指定的目录不会被替换。
应用退出或服务销毁时回收子进程和管道，管道 EOF 会停止应用侧读取任务。

## 维护检查

原生接线契约（无需 Android SDK）：

```sh
node --test tools/tests/android_root_commands.test.mjs \
  tools/tests/android_terminal_privileges.test.mjs \
  tools/tests/android_runtime_contracts.test.mjs \
  tools/tests/super_admin_terminal_routing.test.mjs
```

真实 Kotlin 行为测试（Java 17+，Kotlin 2.2.20）：

```sh
JAVA_HOME=/path/to/java KOTLINC=/path/to/kotlinc \
  python3 tools/tests/test_android_root_commands.py
```

覆盖自动选择、KernelSU/APatch、强制模式、权限撤销/重试、自定义 su、拒绝普通 UID、
终端直接 su argv 探测、中断、共享 deadline、不重复执行、双管道大量输出、stdin EOF、
二进制输出、超时清理。
没有 Kotlin 编译器时 Python 测试显式报告 skipped，不将其视为行为验证通过。

还需要在 Magisk / KernelSU / APatch 真机上验证 Root 管理器授权弹窗、拒绝后重试、
权限撤销、截图和长时间命令超时，以及 Shizuku 的多会话、窗口调整、Ctrl-C 和服务断连；
JVM 测试不能替代设备级兼容性验证。
