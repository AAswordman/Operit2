# Android phone Edge

旧手机产品，独立 APK，使用 `operit-edge-sdk`。兼容目标为 Android 5.0 / API 21，
同时打包 32 位 ARM 和 64 位 ARM。使用 Android framework View/Canvas 与 Java 8，
没有 Flutter、WebView、AndroidX、Google Play Services 或 Wear OS 依赖。

本目录不依赖 `hosts/android`，不启动完整 Core、不执行宿主 ToolPkg，也不复制聊天数据库。
Rust SDK 仍通过自己的共享 Host trait 接入 TCP、调度、节点存储及表情资源；这些基础 I/O
接口是 SDK 的运行条件，不是主应用的 Android Host。TCP 和无 JavaScript 调度器复用
`hosts/common` 的平台无关原生实现；节点存储和 Android 生命周期由本产品负责。

## 首版能力

- 本地 Canvas 表情：happy、neutral、listening、thinking、speaking、sleeping。
  远端使用已有 `edge.robotFace` 服务改变表情，复用 SDK 的空间权限与加密连接。
- 唯一持久节点身份、实际 TCP 监听端口、六位码配对，以及独立的设备空间加入申请。
- 查看入站配对码，发起或取消配对，申请/刷新/取消空间加入，主动退出空间和本机解除配对。
- 前台服务持有节点，屏幕旋转与进入后台不重新创建节点；只有界面可见时读取展示状态。
- 表情只在变化和眨眼时重绘；默认允许熄屏，可显式开启表情界面保持亮屏。

此阶段没有录音、语音识别/播放、聊天界面、直接模型请求、摄像头或手表专用布局。
表情名称是设备状态，不表示这些语音或模型能力已经接通。

## 接入

1. 安装 APK，进入「设备设置」，查看本机的实际 IP 和端口。
2. 在 Operit 手动连接本机的 TCP 地址，输入本机显示的六位码；或者在旧手机中输入
   对端的 TCP IP:端口，并输入对端显示的六位码。主动连接只支持明确的 TCP 地址。
   另一台设备也必须开启 TCP 监听；HTTP/WebSocket 的端口不能作为 TCP 端口使用。
3. 在具备出站配对的设备项中申请加入空间，到空间管理员端审批，再点击刷新申请完成加入。
   配对不会自动授予空间成员资格；申请与审批沿用 SDK 的持久记录。
4. 从同空间已授权节点调用 `edge.robotFace`。离开界面后网络节点仍由前台服务持有。
   「停止节点并退出」关闭连接；再次启动继续使用原身份和配对状态。

首版明确使用 TCP 和手动地址，不启用 mDNS、蓝牙或中继。地址列表会排除回环和链路本地地址，
实际可达性由网络决定。Android 可能在熄屏后限制网络或结束进程；前台服务不代表系统保活保证。
节点进程被系统结束后，需重新打开应用。卸载/清除数据会删除该设备身份；自动备份已禁用。

## 存储边界

节点私有目录中的 `node.json` 是有界逻辑文件快照，最多 128 项、序列化后 2 MiB。
SDK 只看到相对逻辑路径；产品适配层拥有私有目录的真实路径。
每次修改写入 `node.pending`，同步文件后原子发布；Android 上同步父目录。
启动只读取提交文件，损坏文件明确报错，不重建身份或清空配对。
数据由 Android 应用沙箱保护；本产品没有另外提供静态加密或硬件 Keystore。
此预算用于节点控制状态，不是完整业务副本或媒体资源存储。

## 检查和构建

需要 Rust（包括 `armv7-linux-androideabi` 和 `aarch64-linux-android` target）、
Android SDK 35、NDK `27.3.13750724`、JDK 17 或 21，以及 Python。
NDK API 21 与 App 的 minSdk 保持一致。原生库使用 16 KiB ELF 页对齐，
同时兼容 4 KiB 页设备；APK 原生库的 ZIP 对齐由 Android Gradle 插件负责。

从仓库根目录运行，SDK 路径使用本机实际路径：

```powershell
$env:RUSTFLAGS = '-Awarnings'
cargo test --locked --manifest-path edge/boards/android_phone/Cargo.toml --lib
.venv/Scripts/python.exe edge/boards/android_phone/build.py --sdk-dir D:/ProgramData/AndroidSDK --check
```

`--check` 检查两个 Android Rust target，不链接 `.so`，也不产出 APK。
本地测试使用真实 TCP，覆盖配对与空间准入分离、审批加入、重启恢复，以及存储发布失败和损坏状态。
这些检查不等于 API 21 真机或长期后台运行验收。

需要安装包时运行：

```powershell
.venv/Scripts/python.exe edge/boards/android_phone/build.py --sdk-dir D:/ProgramData/AndroidSDK --apk
```

构建脚本先链接两种 ABI 的独立 Edge 库，再通过仓库现有 Gradle 8.14 wrapper 构建本目录的
Android 工程；wrapper 只是构建工具，不会构建或加载 Flutter 主应用。
APK 输出在 `android/app/build/outputs/apk/debug/app-debug.apk`。
直接用 Gradle 打包缺少任一 ABI 库时会明确失败。

发布前还需在真实 32 位 API 21 旧手机上验证安装/加载、中文显示、双向配对、空间审批、
远端表情、断网恢复、熄屏和重启后的行为，再记录实际内存、耗电与安装包大小。
当前最低系统版本是兼容目标，只有对应设备验收后才能标记为已支持。

## 本地验证（2026-11-12）

- 产品测试 3 项通过：真实 TCP 配对、独立审批入空间、远端表情和身份/配对恢复，
  以及原子存储发布失败、损坏快照和非法逻辑路径。测试禁止打开 SQL 日志。
- 共享存储的控制相关测试 24 项、`peer-runtime` 单元测试 129 项通过。
  修正了节点权限读模型与连接可用性检查误用 Core SQL 日志的问题；Core 仍显式读取自己的日志。
- NDK API 21 的 `armv7-linux-androideabi`、`aarch64-linux-android` 检查通过。
- Android Java 编译、资源处理与 Debug Lint 通过。
- Android 运行依赖检查通过，无 `hosts/android`、完整 Core、SQL、V8 或本地模型引擎。

- 两种 ARM 原生发布库已链接，Debug APK 已生成，大小 5,012,883 字节（约 5.0 MB）。
  64 位库的 LOAD 段对齐为 `0x4000`，APK 的 `zipalign -c -P 16 4` 检查通过。
- 已在连接的 PLK110（Android 17 / API 37、64 位 ARM、4 KiB 系统页）上安装并冷启动。
  表情与设备设置正常显示，TCP 监听地址可见，节点服务处于前台运行状态；
  覆盖安装及进程重新启动后节点身份保持不变。修正了新系统的 16 KiB 对齐提示与系统栏遮挡按钮。

尚未执行 API 21 旧手机真机验收、手机与其他节点的实际配对/空间审批/远端表情操作，
也未测量实际内存、耗电或长期后台运行；现代手机上的启动验证不代表这些项目已经通过。
