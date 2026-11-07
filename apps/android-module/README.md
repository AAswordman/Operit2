# Android 模块应用

`apps/android-module` 是与 Flutter 等入口并列的一种应用类型。它复用现有 Core、Android Host 和 Dart 界面，产物是模块文件夹。模块具有 Root 执行环境；系统能力仍通过 Host 接口提供，不在 Core 或 Dart 页面中添加模块专属平台判断。

## 源码

```text
apps/android-module/
  src/Runtime.rs                     模块应用的 Core 启动、数据和生命周期
  android/src/main/kotlin/
    app/operit/ModuleMain.kt          Android app_process 入口
  android/proguard-rules.pro         模块入口保留规则
  package/                           安装、开机、Action、停止、卸载脚本
  build.py                           模块应用自己的目录构建器
  build/module-arm64-v8a/             构建产物，已由仓库 build 规则忽略
```

Android 模块入口与 Flutter 客户端共享 `liboperit_flutter_bridge.so`。Gradle 仅在 `operitRuntime=module` 时加入模块 Kotlin 源码与 `android-module-app` Cargo feature，应用入口通过已有 bridge 的 Android ABI 装配进入同一 Core；不再构建或打包第二份 Core ELF。Android 平台能力的具体实现继续放在 `hosts/android`。

## 构建

从仓库根目录执行：

```powershell
.venv/Scripts/python.exe apps/android-module/build.py
```

构建器通过现有 Android 工程和 FVM 编译共享 Dart 页面与 JNI 库，并输出 `apps/android-module/build/module-arm64-v8a/`。输出目录必须不存在，避免覆盖已有产物。也可以显式指定新目录，或提供本版兼容 APK：

```powershell
.venv/Scripts/python.exe apps/android-module/build.py --apk <本版模块APK> --output <新模块目录>
```

旧入口 `tools/build_scripts/build_android_module.py` 仅转发到应用自己的构建器。没有独立 Core 输入，不接收 ZIP 输出。旧 trial APK 不含新的模块 JVM/JNI 入口，构建器会明确拒绝，不能复用它伪造本版产物。

## 发布

App 和全量发布（`tools/release/build_release.py --scope app|full`）以及 GitHub Actions 的 Android Flutter Build 都会同时生成普通 APK 和模块安装 ZIP。模块作为独立发布产物进入版本命名、Actions 收集和发布校验。

单独构建模块发布产物：

```powershell
.venv/Scripts/python.exe tools/build_scripts/build_android_module_release.py --enforce-lockfile
```

发布脚本调用本应用构建器，保留模块目录 `tools/release/work/android-module/module-arm64-v8a/`，并将其内容归档到 `tools/release/dist/operit2-module-android-arm64-v8a-<发布版本>.zip`，ZIP 根部直接是 `module.prop`。模块 `version` 使用共享 Cargo 发布版本，`versionCode` 使用本次 Flutter build number；普通 APK 与模块使用相同的签名材料和平台版本。发布脚本不向手机安装模块。

## 安装

模块目录根部包含 `module.prop`、`customize.sh`、`service.sh`、`action.sh`、`uninstall.sh`、`bin/operit-host-ctl` 和 `payload/`。这是模块 App 的标准产物，不是另一层部署项目。KernelSU 管理器要求 ZIP 作为传输格式时，压缩这个目录的**内容**即可，ZIP 根部应直接出现 `module.prop`；归档本身不改变模块应用类型。

安装器校验输入后，从唯一的 APK 中展开 JNI 库与终端资源，将原 APK 移到模块的 `system/app/Operit/Operit.apk`。不执行 `pm install`。需要已有启用的 KernelSU mounting metamodule，并在安装后重启。

KernelSU 默认的“卸载模块”策略会让应用进程看不到挂载的系统 APK。安装后，在 KernelSU 的应用配置中显示系统应用，为 `app.operit` 选择自定义配置并关闭“卸载模块”；桌面和 SystemUI 也需要读取这个 APK 的图标、主题和通知资源，应为它们保留挂载。无需为这些界面进程打开“超级用户”开关，Root Core 由模块服务启动。保留全局默认策略，只调整上述应用，重启使已有系统界面进程使用新配置。

替换数据分区的同包名 App 前应备份它的私有数据，并卸载旧 App，避免数据分区 APK 覆盖系统模块或签名不一致。模块业务数据位于 `/data/adb/operit2`，更新模块时保留该目录。

后台使用 `app_process` 启动模块入口、加载共享 JNI 库，由 Android Host 以真实 Root UID 装配 Core。Flutter Activity 连接这个 Core。详细进程边界、数据目录与验收限制见 [架构说明](../../docs/android-module-architecture.md)。
