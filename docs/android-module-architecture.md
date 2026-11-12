# Android 模块应用架构

当前源码版本为 `0.1.0-trial.3`。模块是 `apps/android-module` 应用类型，输出模块文件夹；不另设部署项目，也不把模块定义成一套独立业务或界面。

## 应用、Core、Host 的归属

- `apps/android-module` 拥有模块应用入口、生命周期、安装脚本和目录构建规则。
- 现有 Core 拥有业务、模型调用、会话、插件、事件处理与持久状态。模块不复制这些实现。
- `hosts/android` 实现平台能力，通过 HostManager 与能力接口提供给 Core。Root 是 Host 的执行环境，不是 Dart 页面的平台分支。
- 现有 Dart 页面负责统一的界面和交互，Android Activity 提供原生 Flutter 承载。
- `apps/flutter/native/operit-flutter-bridge` 提供现有 Core 装配、Dart ABI 与本机传输。模块入口以 `android-module-app` feature 编入同一 JNI 库，没有独立 Core executable。

```text
模块 service.sh
  → bin/operit-host-ctl supervise
  → Android app_process（Root，执行 apps/android-module 的 ModuleMain）
  → 同一 liboperit_flutter_bridge.so 的模块 JNI 入口
  → Android 特权 Host 装配 → 现有 Core
                                  ↕ 本机 Link Call / Watch / Push
                         原生 Flutter Activity → 共用 Dart 页面
```

后台进程不创建 Activity 或 Flutter 引擎。界面进程不创建第二个业务 Core，不打开业务数据库，不启动普通 App Core 前台服务。关闭界面只断开客户端，不停止模块 Core。

普通 App 与模块分别构建：普通 App 使用 `app.operit`、显示名称 `Operit2`，模块客户端使用 `app.operit.module`、显示名称 `Operit2 (Module)`，可同时安装和运行。两者具有独立的 Android UID、私有数据、Provider authority 和 Core，不进行运行权交接或自动数据迁移。共享 Kotlin/JNI 类仍使用 `app.operit` 命名空间；模块启动 Activity 使用完整组件名 `app.operit.module/app.operit.MainActivity`。

## 源码目录

```text
apps/android-module/
  src/Runtime.rs                       模块应用启动、环境准备与 Core 生命周期
  android/src/main/kotlin/app/operit/
    ModuleMain.kt                      Android JVM 入口，不依赖 Activity
  android/proguard-rules.pro            保留 app_process 与 JNI 入口
  package/                             module.prop、脚本与控制入口
  build.py                             模块应用的文件夹构建器
  build/module-arm64-v8a/               默认产物目录
hosts/android/src/                     Android Host 的实际能力实现
apps/flutter/native/operit-flutter-bridge/
  src/LocalRuntimeTransport.rs          共用本机传输，不归安装脚本或 Host 管理
  src/platform_runtime/android.rs       普通/特权 Android Host 装配
apps/flutter/app/android/               共享 Android UI、JNI 构建与生命周期路由
apps/flutter/app/lib/                   共用 Dart 页面与 CoreProxy
```

`tools/build_scripts/build_android_module.py` 仅是构建入口转发，不再拥有独立打包逻辑。模块 Runtime 源码由 bridge 的 Android ABI 装配按 `android-module-app` feature 编译；平台实现和业务实现的归属不因共享编译工具链而改变。

## 构建与目录产物

```powershell
.venv/Scripts/python.exe apps/android-module/build.py
```

构建器使用 FVM 与 `--android-project-arg=operitRuntime=module`，Gradle 加入模块应用的 Kotlin 源码、保留规则和 Cargo feature。普通 App 使用默认 `operitRuntime=app`，不会引入模块应用入口。Flutter release 构建不得在同一 checkout 并行运行。

也可通过 `--apk <本版APK> --output <新目录>` 显式组装目录。输出不能是 ZIP，也不能覆盖已存在目录。构建器验证包含 `applicationId: app.operit.module` 的本版应用标记、ModuleMain DEX 入口、模块 JNI 导出、ARM64 ELF、所需原生库及 rootfs 校验和；旧同包名模块 APK 会被拒绝，必须重新构建。

```text
build/module-arm64-v8a/
  module.prop / customize.sh / service.sh / action.sh / uninstall.sh
  bin/operit-host-ctl
  payload/
    operit-android.apk                 唯一的大型负载，含共享库和环境资源
    native-libraries.list              精确展开清单
    archive.sha256                     安装前输入校验
    installed.sha256                   展开后的文件校验
  deployment-manifest.json             格式 3：输入文件与安装后文件分开记录
```

不再额外携带 Core ELF、另一份 JNI 库或另一份 rootfs 压缩包。安装时展开的文件来自 APK，代码没有重复编译成另一个后台 executable。APK 在系统应用布局下仍需压缩条目及展开库，这是现有 Android 原生库打包方式的磁盘开销；不能把目录去重描述成零额外磁盘占用。终端环境也仍属于当前应用负载，没有宣称已变成可选组件。

KernelSU 接收 ZIP 时，只需将模块产物目录的内容归档，根部直接是 `module.prop`。ZIP 是管理器的传输格式，模块应用的标准构建输出仍是目录。

## 安装后布局与启动

安装需要已安装且启用的 KernelSU mounting metamodule。安装器检查其 `metamodule` 标记、挂载脚本和禁用/删除标记；缺少时直接中止，不添加另一种安装路径。

安装器先校验目录输入，再从 APK 展开明确列出的 ARM64 原生库及 rootfs，移动 APK，校验最终每个输出文件并设置权限和 `system_file` SELinux 标签。没有 `pm install`、平台签名、priv-app 声明、Zygisk 注入或全局 SELinux permissive。

```text
/data/adb/modules/operit2/
  module.prop / customize.sh / service.sh / action.sh / uninstall.sh
  bin/operit-host-ctl
  system/app/Operit/
    Operit.apk                        开机挂载到 /system/app/Operit/Operit.apk
    lib/arm64/*.so                    Activity 与 Root app_process 使用同一组库
  runtime/assets/
    rootfs.tar.gz / rootfs.tar.gz.sha256
  payload/                            展开清单与输入/输出校验记录
  deployment-manifest.json
```

Android 开机扫描 APK，注册 `app.operit.module` 与 `app.operit.MainActivity`。后台等待启动完成和用户 0 解锁，核对 PackageManager 的精确 APK 路径及模块客户端的系统应用 UID，然后以 APK 为 CLASSPATH 启动 `app.operit.ModuleMain`。入口检查真实 UID 0 并加载同一 Rust JNI 库，进入共享 Core 启动链。模块数据位于 `/data/adb/operit2`，不迁移普通 App 数据。

```text
/data/adb/operit2/
  run/                                文件锁、PID、状态与私有连接凭据
  users/0/
    bootstrap.json
    runtime/identities/identity-module/
    workspaces/identities/identity-module/
    secrets/
    host/                             解压后的终端环境和执行程序链接
    logs/
```

Core 将凭据发布到模块客户端私有的 `/data/user/0/app.operit.module/files/host-runtime/client.json`，校验 UID 并复制完整 SELinux 标签；停止时也只清理该客户端凭据。模块客户端不提供普通 App 的数据救援快捷方式。传输使用 loopback 随机端口与双向 HMAC 随机挑战，保留 Call/Watch/Push 和二进制附件。通信协议仍为 1，没有 TLS/载荷加密；最多 16 个连接、32 个队列批次、64 MiB 单帧及 256 个附件。

控制命令为 `bin/operit-host-ctl start|stop|status|open`。停止只针对经进程归属检查的本模块进程。禁用或移除后重启，系统挂载消失；持久数据保留，不执行 `pm uninstall`。当前仅支持 Android 用户 0。

## 权限与能力边界

Android Root Host 根据实际 UID 注册权限状态，文件管理、Root、Shizuku 与后台驻留槽位无需从客户端重复申请。普通 App 的共享目录确认仍先检查并请求存储授权；取消或拒绝不创建 Core。

客户端通知、定位、蓝牙、悬浮窗等仍按 Android 实际能力与权限运行。系统应用身份不等于所有能力已实现。桌面接管、系统助手入口、熄屏唤醒以及其他深层系统能力不属于这次文件夹与安装链重整已经实现的功能。

当前通知、OCR、音频、WebView 等 owner 能力仍依赖已连接的 Flutter 订阅；本次没有将它们全部迁入独立的后台 Android 服务，不能宣称关闭界面后这些能力已持续可用。后续系统能力应由 Host 提供，Core 与 Dart 通过统一能力合同调用。

## 验收边界

目录构建与安装脚本检查不等于实机运行。新 ModuleMain/JNI 入口需要重新构建，旧 APK 不可作为本版成品。真实重启后的挂载、PackageManager 扫描、Root app_process/JNI 加载、SELinux Enforcing 下 Core ready、Flutter 附着、关闭界面后后台存活及禁用后挂载移除，都必须单独记录实机结果。

旧 trial.1 的 APK 自动安装验收和 trial.2 的 ZIP 校验不能替代上述结果。
