# 通用悬浮窗与桌面小部件

`ToolPkg.registerDesktopWidget` 注册的是**桌面小部件**：一个应用内路由，以及一个可单独注册的摘要渲染路由。桌面端使用透明桌面层窗口承载，Android 使用启动器 AppWidget 快照，刷新和点击行为由小部件宿主处理。这套 API 不承担桌宠的实时交互与窗口控制。

`ToolPkg.ui` 按用途组织插件界面的运行时呈现能力；`ToolPkg.ui.floatingWindows` 控制**通用悬浮窗**。页面和小部件的注册仍使用现有 `ToolPkg.register*` 接口，Compose 组件仍由 `ctx.UI` 构建。接口覆盖系统悬浮窗和应用内悬浮层，`capabilities().scope` 明确报告实际范围；插件可以按范围决定是否打开。桌宠、计时器、悬浮工具栏等共享这套窗口能力。内容使用已注册的 `compose_dsl` 路由，与普通插件 UI 一样保留实时会话、状态、事件、Canvas、图片与 WebView 能力。插件自行定义角色行为和外观。

## 打开与控制

### 声明窗口内容

多数插件只需 `set(id, content | null)`。它声明窗口现在应显示的内容：同一 ID
首次创建，后续更新原会话并恢复显示；传 `null` 关闭，窗口不存在时也成功。
创建、更新、显示和关闭在宿主同一队列中串行执行，创作者不需要查询窗口列表、
维护句柄缓存或协调开窗期间到达的下一次调用。

```ts
await ToolPkg.ui.floatingWindows.set("status", {
  route: "toolpkg:com.example.status:ui:status",
  state: { message: "正在处理" },
  options: { width: 240, height: 120 },
});
await ToolPkg.ui.floatingWindows.set("status", {
  route: "toolpkg:com.example.status:ui:status",
  state: { message: "处理完成" },
});
await ToolPkg.ui.floatingWindows.set("status", null);
```

`set` 返回 `"shown"`、`"closed"`、`"permission_required"` 或 `"unsupported"`。
后两项明确表示未显示，并关闭该 ID 已有的窗口；不会自动申请系统授权或更换呈现方式。
路由、参数、引擎创建或原生窗口操作错误会拒绝 Promise。需要展示授权入口时，
在设置页查询 `capabilities()`，由用户点击调用 `requestPermission()`。
取得授权后再次调用 `set` 即可显示；`set` 不保存未显示的请求，也不自动重试。

`route` 必须属于当前启用插件。窗口存在期间 `route` 和 `fill` 模式不能改变，
要更换时先传 `null` 关闭。省略 `fill` 保留现有模式，新窗口默认为 `false`。
提供的 `options` 和 `state` 按顶层键合并，省略的字段保持不变，UI 会话不重建。
`state` 接受普通 JSON 对象，可直接传入 SDK 返回的类型化对象；宿主桥接会拒绝
非 JSON 值、循环引用、非有限数字和特殊对象，避免序列化时丢失数据。

### 分步控制窗口

先在 `registerToolPkg()` 中注册普通 UI 路由。注册期间不能调用任何窗口方法。

```ts
const PET_ROUTE = "toolpkg:com.example.pet:ui:pet";

// 在工具、UI 事件或其他运行时回调中调用：
const caps = await ToolPkg.ui.floatingWindows.capabilities();
if (!caps.supported) throw new Error("当前宿主不支持悬浮窗");
// 只接受跨应用悬浮的插件应再检查 caps.scope === "system"。
// "application" 表示只在 Operit 当前应用视图内显示。
if (!caps.permissionGranted) {
  // 只在明确的用户操作中申请权限。Android 打开系统设置后，
  // 用户返回应用时应再次查询 capabilities，再打开窗口。
  await ToolPkg.ui.floatingWindows.requestPermission();
  return;
}

await ToolPkg.ui.floatingWindows.open({
  id: "pet",
  route: PET_ROUTE,
  state: { mood: "happy" },
  options: {
    x: 100, y: 100, width: 200, height: 240,
    alwaysOnTop: true,
    clickThrough: false,
    focusable: false,
    draggable: true,
  },
});

await ToolPkg.ui.floatingWindows.update("pet", { width: 240 }, { mood: "happy" });
await ToolPkg.ui.floatingWindows.hide("pet");
await ToolPkg.ui.floatingWindows.show("pet");
const ownedWindows = await ToolPkg.ui.floatingWindows.list();
await ToolPkg.ui.floatingWindows.close("pet");
```

- `id` 是插件内的窗口 ID，长度为 1–128，不能有首尾空白。同一插件重复打开同一个 ID 会报错；不同插件可以使用相同 ID。每个插件最多同时拥有 16 个窗口。
- `route` 必须是当前启用插件注册的完整 Compose UI 路由，不是注册项的短 `id`。`state` 是该 UI 会话的初始状态对象。
- `open` 的 `fill: true` 让窗口覆盖当前范围的完整区域。系统范围覆盖主显示器，应用范围覆盖应用视图。该模式在打开时选定，跟随显示器或视图尺寸变化，不接受手动宽高、坐标或拖动。`list()` 返回当前测量尺寸，可能超过普通窗口的尺寸上限。
- 系统范围还可传入 `options.screenBounds: {left, top, right, bottom}`，直接使用原生无障碍提供的屏幕边界；Windows、Android 使用物理像素，macOS 使用屏幕点坐标，X11 使用 AT-SPI 桌面坐标，由原生端适配。它优先于逻辑坐标和宽高，不受普通窗口的 4096 逻辑像素上限约束，不能与 `fill` 合用。`update` 可更新该边界以跟随目标窗口；应用范围不接受外部屏幕坐标。
- 默认大小为 `240 × 240`，透明、无边框、置顶、不抢键盘焦点。宽高范围为 16–4096 逻辑像素。`x/y` 在系统范围使用主屏幕左上角坐标，允许负数；应用范围使用应用视图左上角坐标，显示时约束在当前视图内，随视图尺寸变化重新约束。
- `update(id, options, state?)` 只更新提供的窗口选项；可选的第三个参数按顶层键合并 UI 状态，并在原有 UI 会话中重新渲染。隐藏、显示和更新窗口不重建 UI 会话。
- `draggable: true` 由宿主处理整窗拖动：按下时记录指针屏幕坐标和窗口位置，移动时使用「窗口起点 + 当前屏幕坐标 − 按下屏幕坐标」，松手结束。系统窗口在原生输入线程移动，应用内窗口在 Flutter 宿主直接移动，中途不经过 JS/Rust，也不累加局部 delta。默认不启用。
- `list()` 读取宿主当前几何信息，包含拖动后的实际位置。修改外观时省略 `x/y` 可保留当前位置。
- `clickThrough` 是整个矩形窗口的输入穿透，不是按透明像素自动计算点击区域。开启后，插件需要从其他 UI 或运行时入口恢复交互。
- `capabilities` 中的 `alwaysOnTop` 表示是否可切换置顶状态；Android 系统悬浮窗固定在普通应用之上，应用内悬浮层固定在应用内容之上；这两种情况该字段为 `false`，且 `alwaysOnTop: false` 会报错。
- `scope` 为 `system`、`application` 或 `unsupported`；`systemOverlay` 和 `applicationOverlay` 分别报告系统和应用层支持。应用范围无需系统悬浮授权，`requestPermission()` 直接返回 `true`；Android 授权仍必须由用户操作触发。
- `close` 确认关闭请求后，宿主异步释放 UI 会话、流和引擎，允许窗口从自己的事件回调中关闭自身。窗口记录立即移除。
- 插件身份由 Rust 执行宿主提供。插件只能操作自己的窗口，传入其他插件名不能改变窗口归属。
- 停用或卸载插件后，宿主关闭对应窗口。应用内目录变更立即触发检查；其他来源的变更最多每 5 秒检查一次。窗口不跨应用退出恢复。

## 麦克风录音

插件可以在自己的可见悬浮窗里，从用户点击触发一次有时限的录音。麦克风授权使用宿主的系统权限流程，同一宿主一次只允许一个悬浮窗录音。结束或丢弃录音后才能开始下一段。隐藏、关闭窗口或卸载插件会丢弃录音并释放麦克风。

```ts
const windows = ToolPkg.ui.floatingWindows;
const recording = await windows.record('call', {
    maxDurationMs: 15000,
    silenceMs: 900,
});
if (recording !== null) {
    const { text } = await Tools.Stt.transcribe(recording);
    console.log(text);
}
```

- `record(id, options?)` 等待完成并返回 `{audio, fileName, contentType}`，音频为 16 kHz、单声道、16 位 PCM WAV。
- 录音期间可调用 `recordingLevel(id)` 获取归一化音量（`0` 到 `1`），或用 `recordingHasSpeech(id)` 查询当前录音是否已检测到语音。
- 当 `capabilities().recordingEchoCancellation` 为 `true` 时，`record(id, { echoCancellation: true })` 可请求开启录音回声消除，并仅保留语音开始前的一小段录音。
- `maxDurationMs` 可设为 1000–20000 毫秒，默认 15000。`silenceMs` 可设为 300–3000 毫秒，设置后检测到一段声音才开始判断尾部静音，整段没有达到发声阈值时返回 `null`。
- 从另一个界面事件调用 `finishRecording(id)` 可提前结束采集，WAV 由原来的 `record` 调用返回。`cancelRecording(id)` 丢弃采集，`record` 返回 `null`。权限或音频设备错误会拒绝 Promise。
- 真实可用范围取决于宿主麦克风权限和系统的后台录音规则。系统悬浮窗权限只决定窗口能否显示。

连续录音和播报的用法见 [语音通话插件](../packages/external/voice_call/README.md)。

## 宿主实现

调用通过 `ToolPkg.ui.floatingWindows`、异步原生绑定、认证后的 `JsExecutionHost` 和 `FloatingWindowHost`，由 Flutter 主引擎经 `floating_windows` 驱动当前窗口范围。

窗口请求异步等待 Flutter 主引擎的响应，Web 与原生使用同一个请求和响应协议。Flutter 主引擎串行管理窗口生命周期。系统悬浮窗各自拥有独立的 Flutter 引擎与插件 UI 会话，子引擎跳过主进程初始化与通知订阅。应用内悬浮层使用同一主引擎的独立 UI 会话，各自保存关闭回调、焦点策略、可见状态和输入穿透状态，隐藏时保留会话并暂停 Flutter ticker。

`apps/flutter/thirdparty/floating_windows` 不依赖 Operit、Rust、插件注册模型或 Compose DSL。它只接收不透明的应用数据和窗口选项，提供引擎启动、准备完成确认、窗口控制和失败回收；`FloatingWindowsHost` 提供不依赖原生插件的应用内实现。应用层负责渲染插件 UI 与释放会话。

桌面引擎创建和消息传输复用 `thirdparty/desktop_multi_window` 0.3.1。保留上游 Apache-2.0 许可证，补丁记录见该包的 `OPERIT_PATCHES.md`。窗口样式与系统悬浮层代码保留在 Flutter 插件内。

## 平台范围与验证

| 平台 | 实现 |
| --- | --- |
| macOS | 透明无边框窗口、位置与尺寸、置顶、输入穿透、可选键盘焦点；已通过本机原生开窗与回收检查 |
| Windows | 独立引擎、DWM 透明窗口、位置与尺寸、置顶、输入穿透、可选键盘焦点；尚需 Windows 设备验证 |
| Linux / X11 | GTK RGBA 窗口，需要合成器；创建引擎前选择透明 visual；尚需 Linux 设备验证 |
| Linux / Wayland | 应用内悬浮，位置相对于当前应用视图；不宣称跨应用置顶 |
| Android | `TYPE_APPLICATION_OVERLAY`，需要系统悬浮窗权限；独立 FlutterTextureView 与引擎，随拥有者引擎释放；尚需 Android 设备验证 |
| iOS | 应用内悬浮，与应用当前视图同生命周期 |
| OpenHarmony | 应用内悬浮；当前应用未配置全局悬浮所需受限权限与原生窗口适配 |
| Web | 应用视图内悬浮；不弹浏览器新窗口、不依赖弹窗许可 |

Android 12 及以上的整窗穿透会将该窗口透明度限制到系统允许的最大遮挡透明度；多个重叠悬浮窗仍受系统组合透明度规则约束。AppWidget、应用内悬浮和系统悬浮使用不同的能力范围，不互相冒充。

完整操作示例见 `plugins/packages/external/floating_window_demo`。该示例注册工具箱入口，演示权限查询、创建桌宠、点击更新状态、移动、隐藏、显示与自关闭。


OpenHarmony 官方文档将 `ohos.permission.SYSTEM_FLOAT_WINDOW` 归为 `system_basic` / `system_grant` 受限权限，需要相应 ACL/签名配置，不能通过通用用户授权按钮取得。当前以普通应用可用的应用内实现覆盖该平台；具备该权限的系统悬浮适配仍需另行实现与设备验证。参考：[受限权限](https://github.com/openharmony/docs/blob/master/en/application-dev/security/AccessToken/restricted-permissions.md)、[createWindow](https://github.com/openharmony/docs/blob/master/en/application-dev/reference/apis-arkui/arkts-apis-window-f.md)。

当前使用的 OHOS Flutter 嵌入层中，`FlutterView` 构造函数从 Ability 的
`WindowStage.getMainWindowSync()` 取得宿主窗口，并据此注册尺寸、避让区、键盘及 UIContext。
后续系统悬浮适配需要先提供可绑定独立窗口的视图接口，并处理窗口隐藏/恢复时的 surface 生命周期，
然后接入独立引擎、插件注册和受限权限检查。当前设备权限、嵌入层接口与设备验证均未完成。

## 大肥鱼插件与验证

`plugins/packages/external/deepseek_dafeiyu` 是独立的大肥鱼桌宠插件，包含本地透明动画、拖动、喂食交互和工具箱设置页。三视图取自 MIT 开源项目，原素材、许可证、固定版本和动画重建脚本一并保存。

- 设置保存后重新创建桌宠；大小、动画、踏步展示与整窗穿透可配置，穿透后可从工具箱恢复。
- 插件用 `scope` 显示当前平台的真实范围，不隐式申请权限。
- 包级测试覆盖桌面通道未注册重试、永不返回的 ready 超时、原生失败回收，以及应用内多窗口隐藏/恢复与关闭失败回收。
- 本机原生检查入口：在 `apps/flutter/app` 执行 `flutter run -d macos -t tool/floating_windows_smoke.dart`；可用 `--dart-define=OPERIT_DAFEIYU_SPRITE=/absolute/path/idle.webp` 显示真实动画素材。

完整插件检查可在 `apps/flutter/app` 执行：

```sh
flutter run -d macos -t tool/dafeiyu_plugin_smoke.dart --no-pub
```

该入口使用系统临时目录中的独立运行时，导入和启用打包的大肥鱼插件，加载设置页，
通过设置按钮调用 `ToolPkg.ui.floatingWindows`；子窗口检查图片解码和双击喂食后的精确回复。
成功需要同时看到 `DAFEIYU_PET_SMOKE_PASSED` 和 `DAFEIYU_PLUGIN_SMOKE_PASSED`。
日志报告的 `root` 目录保存界面截图和临时运行数据。进程退出不等于检查通过。
检查结束后执行 `flutter build macos --debug --no-pub` 恢复正常应用入口的构建产物。

使用同一台桌面主机检查应用内呈现路径：

```sh
flutter run -d macos -t tool/dafeiyu_plugin_smoke.dart --no-pub \
  --dart-define=OPERIT_FLOATING_SMOKE_APPLICATION=true
```

该模式在测试入口选择应用内宿主，保留真实 Rust、插件 UI、资源解码和交互链路；
它不是 iOS 或 OHOS 的设备测试。最终标记同时报告 `scope=application` 或 `scope=system`。
浏览器的窗口生命周期和输入穿透可在 `apps/flutter/thirdparty/floating_windows` 执行：

```sh
flutter test --platform chrome test/application_windows_test.dart
```

关闭请求在子引擎通道失联或超时时通过原生拥有者回收窗口，避免窗口永久留在系统桌面。
应用内宿主重建后也不会重用旧窗口 ID，旧句柄无法控制新宿主的窗口。
