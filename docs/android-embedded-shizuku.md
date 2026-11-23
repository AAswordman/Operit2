# Android 内置系统服务

OP2 内置 owner-only Shizuku 分支，沿用 API 13、Binder 与 UserService 协议，
只服务本应用。工具与权限中的 Shizuku 设置入口可管理 Root 启动、无线调试配对、
自动发现、电脑 ADB 激活、恢复开关及外部 Shizuku/Sui 选择。
入口在已授权和未连接时均可打开；状态仍来自真实 Binder 授权检查。
新终端连接按服务端 Binder 区分后端，并使用独立的 UserService 连接标签；
已经建立的终端仍持有原 Binder，不将旧会话 PID 发给另一个服务端。

## 许可与来源

- Shizuku：Apache-2.0，固定源码版本与许可见 `tools/android-shizuku/README.md`。
- Shizuku-API：MIT；保留使用中的 server-shared 来源及许可。
- 许可随库打入 Android assets，修改的上游文件包含 Operit 修改说明。
- Stellar（MPL-2.0）和 ShizukuX（Apache-2.0）仅研究生命周期机制，未复制代码。
  未引入黑域代码。

## 故障修正

Android 17 的 `IPackageManager.getInstalledPackages(long,int)` 返回
`PackageInfoList`，旧兼容库的返回类型为 `ParceledListSlice`；用户列表也变更了签名。
内置服务只连接 OP2，因此已删除独立管理器的应用列表、第三方授权配置与全量包扫描。
启动发布和 UserService 仍需要枚举用户，统一由 `SystemUsers` 调用实际系统签名，
错误保留原始原因。

## 内置结构与界面

`app.operit.shizuku` 集中管理首页、配对、Binder Provider 和恢复生命周期。
`tools/android-shizuku` 是 OP2 的 Android 库模块，只参与 OP2 APK 构建；
没有独立应用入口、独立 APK 或发布配置。已移除 rish 客户端及其原生库，
命令继续通过 OP2 现有 UserService/JNI 终端实现执行。

首页卡片、主题、图标、配对引导和通知 RemoteInput 来自原版源码。
配对时进入系统无线调试页面，在 OP2 通知中输入六位配对码；
首页不再提供手填端口和配对码表单。未引用的管理器资源和文案已裁掉。

Root 激活时由原生启动器先离开应用内存 cgroup，再一次性设置 shell 附加组、
GID/UID 2000 和 SELinux shell 域，然后才创建守护进程与 ART 服务。
转换失败即拒绝启动。服务及其 UserService/命令子进程继承该身份，
内置链路不依赖每条命令启动时检查 UID 0 再降权。
外部 Root Shizuku 仍可能运行在 UID 0，因此现有外部传输的身份约束继续保留；
显式 Root 命令和 Root 模块本身仍执行用户授权的 Root 操作。

## 生命周期

启动器持有实例锁、脱离会话并关闭继承的描述符，不将服务生命绑定到 ADB 连接。
最多重启五个异常退出的子进程；用户停止、APK 被替换/移除时退出守护。
更新后由新 APK 的恢复入口解析新路径。

用户打开恢复开关后，ADB 模式在解锁前通过设备保护存储中的配对身份执行一次有界激活；
首次解锁、应用更新和 Binder 死亡可调度带 Binder 验证的恢复。
Android JobScheduler 负责等待网络和失败后的指数退避，不增加常驻轮询。
Root 恢复使用原先授权的 Root 通道；ADB 恢复使用保存的配对身份、重新发现当前端口，
开启恢复时由内置服务授予 WRITE_SECURE_SETTINGS，保存并关闭 ADB 授权超时，
恢复开发者选项、USB 与无线调试开关。系统仍须允许重新开启无线调试。
关闭恢复或停止内置服务会取消任务，不自动切到外部服务或另一种身份。

断开 Wi-Fi 不主动终止已经启动的服务。关机必然终止进程，恢复指重新激活；
非 Root 的自动恢复仍受 Android/OEM 无线调试政策约束。

## 验证记录（2026-11-23）

- 内置库 Java/Kotlin、OP2 Kotlin 接线与 Manifest 合并通过；原生配对库/启动器覆盖
  arm64-v8a、armeabi-v7a、x86、x86_64。
- 故障手机前期独立诊断复用生产启动器和当时的包枚举实现：Root 启动后的真实身份
  为 UID/GID 2000、`u:r:shell:s0`，有效 capabilities 为 0；用户 0/10 分别枚举
  510/469 个包，均含 app.operit。
- 杀死诊断子进程后出现新的 PID，身份和枚举结果相同；诊断正常退出后守护停止。
- 独立 DEX 加载内置服务构造与命令实现，以 UID 2000 执行 `id` 和
  `dumpsys usagestats` 成功，exit=0。此检查直接调用服务，不经过已安装旧 APK 的
  Provider/UserService 接入链。
- Flutter 设置页静态分析通过；设置入口用 widget 测试覆盖已满足/未连接仍可操作。

- 原版界面版本已打入 OP2 debug APK 并保留数据安装到故障手机；此次复用已有
  Flutter/Rust 产物，重建 Android Host、内置库和 APK。
- 清理后的 Java/Kotlin、Manifest、资源以及四种 ABI 的原生启动器检查通过；
  APK 已重新安装，确认不包含 rish。实机点击 Root 启动后，首页显示
  “内置 Shizuku 正在运行 · API 13 · UID 2000 · 已授权”，服务进程实际属于 shell。
- 实机已打开原版首页和配对引导，系统配对窗口开启后，OP2 通知已显示
  “已找到配对服务”。尚未通过通知完成配对，不能视为完整配对成功。

完整 OP2 Provider/UserService 调用和真实重启恢复尚未完成验证；
没有重启手机或切断其 Wi-Fi。独立诊断成功不代表完整应用接入链已经验证。
