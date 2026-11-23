# 浏览器会话与工作区展示

浏览器执行属于运行时宿主，会话是否存在不由工作区是否打开决定。通用浏览器使用现有会话协议；Android 的 `visit_web` 由进程级原生 Host 执行，工作区通过平台能力接口直接展示同一个 WebView。其他宿主继续使用运行时浏览器会话。

## 请求链路

```text
browser_* 工具
  → StandardBrowserAutomationTools / BrowserAutomationHost
  → FlutterHostAdapters → RuntimeHostInteractionService
  → RuntimeHostInteractionSubscriber → BrowserAutomationBridge
  → RuntimeBrowserOwnerHost → RuntimeBrowserSessionRegistry
  → RuntimeBrowserOwner 持有的 WebViewController

visit_web 工具
  → AndroidWebVisitHost → AndroidRuntimeHost.webVisitOperation
  → HostWebVisitManager（原生主线程、私有虚拟显示、任务期限与 DOM 提取）
  → PlatformWebVisitHost 观察状态并借用同一个 WebView 给工作区

其他宿主及 Android 独立模块发起的 visit_web
  → FlutterHostAdapters → RuntimeHostInteractionService
  → RuntimeHostInteractionSubscriber → WebVisitBridge
  → RuntimeBrowserOwnerHost → RuntimeWebVisitRegistry / RuntimeWebVisitSession
  → BrowserSessions（现有 create / evaluate / events / close 协议）
  → RuntimeBrowserService → 同一个 RuntimeBrowserOwner
```

Android 独立模块通过应用 owner 的 `PlatformWebVisitHost.visit` 接入相同的原生执行器；模块进程不初始化依赖应用上下文的 WebView。能力选择由 Host 装配及 WebView 平台适配器声明，不按执行失败切换路径。

宿主交互订阅并发处理请求，因此 `visit_web` 等待期间仍可处理它发起的浏览器会话命令。Core 会话及事件协议没有添加 Android 专用操作。

创建浏览器会话时，宿主打开标签的操作直接返回该标签的会话 ID，运行时按这个 ID 返回对应会话。并发创建和宿主尚未挂载时排队的创建都保留各自的返回值与错误；不能通过全局新增会话列表或当前活动标签推断请求归属，否则两个访问可能绑定到同一会话，并在其中一个完成清理时影响另一个。

## 会话与展示分离

- `RuntimeBrowserOwnerSurfaceHost` 为每个存活会话保留一个真实 WebView，始终位于全局宿主。打开、隐藏和切换工作区不会迁移它的 Element 或重新创建平台视图。
- `WorkspaceBrowserSurface` 是普通浏览器和临时网页访问共用的展示入口。原生展示提供 `RuntimeBrowserSurfaceTarget` 锚点；Web 展示继续使用已有的合成画面传输。
- 锚点通过 RenderObject 的 attach、layout、detach 发布展示位置和大小。`RetainedPage` 隐藏嵌套页面时，即使 widget 未 dispose，也会释放锚点。恢复时重新提供锚点，不依赖休眠子树完成构建。
- `RuntimeBrowserSurface` 在相同挂载位置跟随锚点。没有锚点时禁用绘制、触摸和语义，但保留 WebView 及最后的非零视口。首次后台创建使用宿主视口，不使用屏幕外 1×1 布局。
- 锚点只表达展示权，不表达会话存活权。显式关闭会话才销毁浏览器；多个展示锚点使用最后注册的存活锚点，释放后可回到先前锚点。
- TTS 和运行时浮层位于浏览器宿主之上，避免全局浏览器展示覆盖应用浮层。

## 临时访问与等待

`RuntimeWebVisitSession` 在收到请求时直接创建会话并开始加载、检测和提取。工作区展示回调可以尚未注册或已被移除，执行仍可完成；后注册的工作区可以展示仍在执行的访问。展示失败只记录日志。

`WorkspaceWebVisitContent` 只观察 `RuntimeWebVisitTask` 并提供浏览器展示锚点或原生 Host surface，不再创建控制器、导航网页或决定工具结果。原 `WorkspaceWebVisitSessionRegistry`、工作区 Completer、工作区自带提取与 SSL 绕过逻辑已删除。完成回调由工作区 presenter 订阅，因此未选中的访问标签也会在任务结束后移除；销毁工作区不会取消任务。

### Android 离屏访问

`AndroidWebVisitHost` 通过已注册的 JNI application Host 进入 `HostWebVisitManager`。执行不需要 Flutter engine、Activity、Widget build 或 Flutter 帧。每个任务在 Android 主 looper 创建独立 WebView，并使用 `Presentation` 将它附着到 1280×720、160 dpi 的私有虚拟显示。ImageReader 持续消费帧，避免缓冲区填满阻塞 compositor；不向 Flutter 传输截图或视频，也不申请悬浮窗权限。

Flutter 的 `PlatformWebVisitHost` 只提供观察、显式访问、用户控制和 surface 接口。连接时重放仍在执行的任务；普通本地 Core 直接通过 JNI 开始访问，不等待 Flutter 观察者上线。原生平台视图借用执行中的同一个 WebView，不重新请求 URL，不克隆 Cookie 或 DOM。窗口隐藏或平台视图释放时，原生视图回到私有显示；引擎退出只释放观察者和借用视图。

原生主线程每 250 ms 同步采样 DOM，等待 document complete 且正文、元数据和链接连续稳定 1 秒后返回。采样不使用页面定时器、滚动、`innerText` 或 Flutter 定时器。真实 SPA 在提取完成后继续请求的内容不能由这种稳定性判据预测；用户可在任务期限内切换手动模式并明确提取当前文档。验证码通过明确的挑战元素判断，保留实际页面供用户处理。采样会检查导航代数，旧文档的迟到响应不会当成新页面结果。

总期限为 45 秒，单次脚本期限为 8 秒，JNI worker 最多等待 50 秒。HTTP 主框架错误、网络错误、renderer 退出和脚本错误均返回明确失败。成功、失败、取消均移除采样和期限回调，并在发布完成状态后立即释放 WebView 与私有显示资源，不等待 Flutter 下一帧或平台视图释放。使用现有 Core 前台服务管理进程后台任务，不新增页面唤醒屏幕行为。

这些更改针对 `visit_web`。通用 `browser_*` 会话仍沿用其既有 Host；这里没有把所有交互浏览器迁移到原生访问执行器。

工作区观察任务状态并提供手动提取、取消等操作，不拥有定时器、加载过程或工具返回的 Completer。工作区销毁不会取消任务；用户明确关闭访问标签才取消该请求。正常完成、失败和取消都会关闭临时会话。创建请求迟到时也会关闭已经失去任务所有者的会话。

`visit_web` 总执行期限为 45 秒，单次脚本和清理分别最多 10 秒，为 Rust 宿主现有的 60 秒请求期限留出返回时间。脚本等待同时观察任务完成，取消时释放本地超时；事件订阅取消和会话关闭同时发起，避免互相等待。`browser_wait_for` 的文本条件由宿主进行限时轮询，不依赖页面内定时器或原生 WebView 对 Promise 返回值的处理。

## 外部应用跳转

自定义应用协议通过真实 WebView 的导航代理进入 owner，先阻止网页加载，再显示打开确认。确认与系统打开按正在处理的 URL 合并，重复事件不会叠加同一个确认框；取消确认不调用系统。找不到处理应用、链接格式错误、系统拒绝及平台异常都会显示失败，并通过原有浏览器命令结果返回调用方。

Windows 在原生 `NavigationStarting`、`FrameNavigationStarting` 和 `NewWindowRequested` 接收应用协议，取消该次导航后转交 Flutter。被取消的应用导航不会当成普通页面加载错误。Android 的主框架及应用协议子框架、Apple 的 navigation action、Linux 的 WebKitGTK policy 和鸿蒙的 ArkWeb override 共用 owner 的确认入口。普通网页子框架保持自身的页面导航。

Android 使用原生 `Intent.parseUri` 解析 `intent:`，保留目标 package、真实数据 URI 与附加参数，限定为可从浏览器发起的 `ACTION_VIEW`，移除显式 component、selector 和 URI 授权标记。该协议由 Android 宿主解释；其他宿主明确报告不支持。文件、content、脚本及页面内部资源不能通过这个应用打开接口启动。

鸿蒙由浏览器插件跟踪当前 Ability，并等待 `startAbility` 完成才返回成功；系统异步拒绝会通过通道返回错误。Ability 分离后不能使用过期 context 发起打开。ArkWeb 的普通网页子框架保持在引擎内，应用协议子框架交给 owner。

浏览器菜单提供“打开外部应用”，通过 `openExternal` 会话命令把当前 HTTP(S) 页面交给浏览器所属设备的系统解析器。关联应用或默认浏览器由操作系统决定；普通 HTTP(S) 点击仍按网页导航处理。Web 查看端也沿用此命令，打开动作发生在浏览器宿主设备。

页面 `window.open` 的应用协议会在创建标签之前处理，不留下空标签；普通弹窗的相对 URL 按来源页面解析。地址栏输入应用协议不会替换原页面地址，打开错误保留在发起操作的标签上。确认框内容支持滚动和选择，适应移动端长 Intent 链接。

本地回归覆盖协议分类、确认取消、重复事件、系统拒绝、平台异常、弹窗路由、Core 命令结果，以及 Windows 原生事件到导航代理和 Android Intent 通道的连接。Android 原生测试检查 Intent 解析调用、目标保留和授权清理；鸿蒙原生控制流测试使用替换的 Flutter 通道与 Ability 检查异步结果和子框架边界，不代表 ArkTS 原生编译。各平台实际安装应用的关联与系统解析结果仍需对应设备验收。

## 下载与文件去向

下载执行与文件保留属于 `RuntimeBrowserOwner`。工作区的下载 Store 是观察者，通过已有 Core 浏览器命令的 `downloads` action 读取列表、取消、重新下载和移除记录；通过 `downloads` 会话事件接收实时状态。每次挂载会话都会读取完整列表，快照包含宿主实例标记与递增版本，避免较早的命令响应覆盖较新的进度。关闭工作区展示不取消正在执行的下载。

各平台通过两种明确的引擎契约接入，业务 Dart 不判断操作系统，也不按扩展名猜测下载：

- Android 的 `DownloadListener` 通过通用 `NavigationRequest` 传递下载标记、User-Agent、Cookie 和 Content-Disposition，由 owner HTTP 下载器保存普通 HTTP(S) 文件；下载回调不会重新导航网页。Android 公开监听器没有原始响应体，不能保证消费一次的 POST 附件可重放。
- Windows 的 WebView2 `DownloadStarting`、字节变化和状态变化直接上报 `WebViewDownloadEvent`。每个操作独立保留事件 token、取消句柄和临时目录；中断原因进入失败记录。同步回调不申请未完成的 deferral。
- Linux 的 WebKitGTK `download-started`、`decide-destination`、`received-data`、`failed` 和 `finished` 上报同一契约，并按所属 WebView 隔离共享 context 中的任务。
- iOS/macOS 的导航 action 下载标记、附件响应或不可显示 MIME 由 WKDownload 接管，原生代理报告进度、完成和失败。Windows、Linux、Apple 保留原始引擎操作，因此原始 POST、Cookie 与响应体不由 Dart 重新请求。
- 鸿蒙通过每个视图的 ArkWeb `WebDownloadDelegate` 保留原始响应，`WebDownloadItem` 提供独立标识、字节数、完成文件与取消操作。代理可在视图挂载前注册，控制器附着后安装，实际文件只写入该传输的私有缓存目录。
- 页面 `blob:` / `data:` 生成文件通过 document-start 注入拦截显式下载点击，使用 64 KiB 分块、序号和逐块确认传到 owner；原生监听器报告的 blob/data 也使用这个通道。原始 fetch 在确认之前开始，避免网页立即 revokeObjectURL 导致已点击的文件丢失。取消通过 AbortController 中止实际读取，页面关闭会明确标为中断。
- Web 界面的下载列表、保存、打开和取消仍通过已连接浏览器 owner 的 Core 会话；浏览器的跨域 iframe 不具备读取任意远端网页下载的权限。

文件先保存在运行时浏览器下载目录。完成记录保存真实 `runtimePath`，工作区相对路径 `savedPath` 仅表示明确的导出目标；不会把尚未写入工作区的文件声明为已保存到工作区。点击“打开文件”会先复制到当前工作区，再使用现有工作区文件打开能力；“保存到工作区”只复制文件并显示目标。查看位置显示可选择、可复制的真实路径，不再把目录交给文件打开接口。每次传输使用独立任务标识及保存文件名，同一个链接和同名文件也能同时下载。原生临时文件导入运行时存储后释放，只有写入成功才发布完成。

下载界面默认显示全部记录，完成后不会从当前列表消失；浏览器地址栏下方保留最近任务状态及下载入口。任务显示已接收字节、已知总大小和百分比；未知总大小显示不定进度，不伪造百分比。菜单约束只控制菜单宽度，触发按钮单独保留 44×44 点击区域；筛选与操作按钮可以换行。

HTTP 非成功状态和不完整传输会标为失败，文件写入成功后才标为完成。取消和移除会释放实际 HTTP 客户端，旧连接的迟到回调不能覆盖新下载。进程结束遗留的活动记录会转为中断；普通 HTTP 任务可重新下载，原生及页面生成文件需从原网页重新发起，界面不会把这些操作伪装为可重放的 GET。当前不提供断点续传，因此界面不显示暂停／继续操作。

自动化验证覆盖共享存储与展示协议、生成文件分块传输、Windows/Linux/Apple/鸿蒙适配事件和 Android 下载请求上下文；Linux 原生回归使用真实 WebKitGTK 响应。Apple 和鸿蒙原生代码及各平台真实设备的下载触发、登录态、系统文件打开仍需在对应环境验收。当前生成文件和 HTTP 导入接口会把完整文件载入 owner 内存，大文件内存占用仍受设备容量限制。

## 外部应用回归验证

在 `apps/flutter/app` 运行共享路由、会话命令及窄屏确认框测试：

```sh
fvm flutter test test/core/browser/runtime_browser_external_navigation_test.dart test/core/browser/runtime_browser_session_creation_test.dart test/core/browser/workspace_browser_session_selection_test.dart test/core/browser/browser_menu_commands_test.dart
```

平台适配测试分别位于 `webview_flutter_platform_interface/test/external_navigation_test.dart`、`webview_flutter_android/test/android_navigation_delegate_test.dart`、`webview_flutter_android/test/android_webview_controller_test.dart`、`webview_all_windows/test/webview_windows_test.dart` 和 `webview_all_ohos/test/ohos_external_navigation_test.dart`。Android 原生回归运行 `WebViewExternalNavigationTest`、`WebViewClientTest`。仓库根目录可用 Node 24 运行 `node --test tools/tests/browser_external_navigation_ohos.test.mjs`，检查生产 ArkTS 控制流的异步打开结果和 Ability 生命周期边界。

Android 控制器完整测试在 Windows 上执行时，既有四个包含 `?`、`<`、`>` 的 Unix 文件路径用例会因 `Uri.file` 使用宿主路径规则而失败；外部应用通道用例可独立执行。该问题与设备上的应用协议解析分开记录。

## 下载回归验证

在 `apps/flutter/app` 运行：

```sh
fvm flutter test test/core/browser/workspace_browser_page_download_test.dart test/core/browser/workspace_browser_download_test.dart test/core/browser/workspace_browser_session_selection_test.dart test/workspace_external_open_test.dart test/core/browser/browser_menu_commands_test.dart
```

覆盖无后缀 URL、Content-Disposition 文件名、未知长度、HTTP 失败、传输截断、取消后重试的迟到回调、同名文件、工作区导出、宿主重启、状态与控制协议、320 像素窄屏和放大文字下的菜单布局。Android 插件的 `android_navigation_delegate_test.dart` 同时检查下载标记、请求上下文与禁止重新导航的行为；Windows/Linux/Apple/鸿蒙的适配器测试检查原生事件、原始文件读取和取消路由。`node --test tools/tests/browser_download_bridge.test.mjs` 直接运行生产注入脚本，验证 blob/data 二进制、分块、取消和立即撤销 object URL。

## 会话回归验证

Flutter 测试覆盖并发创建的正序与逆序返回、排队创建及错误传播、未打开工作区的访问、延迟展示、展示异常、并发完成与取消、迟到创建、脚本无响应、嵌套休眠、视图实例保持和偏移后的触摸命中。它们验证共享生命周期和控制器契约；各平台真实 WebView 的渲染、输入和后台行为仍需设备验证。

### 离屏访问回归

`.github/workflows/web-visit-checks.yml` 运行生产 DOM 提取脚本和 Flutter 会话回归：

```sh
node --test tools/tests/web_visit_extraction.test.mjs
# apps/flutter/app
fvm flutter test test/core/browser/host_web_visit_test.dart test/core/browser/runtime_web_visit_test.dart test/workspace_mount_picker_test.dart
```

原生设备用例位于 `android/app/src/androidTest/java/app/operit/HostWebVisitInstrumentedTest.java`，使用真实 HTTP fixture 和系统 WebView，检查没有 Flutter/Activity 时的动态页面动画帧、HTTP 错误，以及视图转移前后对象身份和 DOM 连续性。在配置好项目 Android 构建环境后可运行：

```sh
# apps/flutter/app/android
./gradlew :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.class=app.operit.HostWebVisitInstrumentedTest
```

本次本地检查运行了 Node 提取测试、Flutter 会话测试和原生 Java 类型检查；未构建 APK，未执行上述设备用例。熄屏、应用后台、真实工作区触摸输入和各厂商 WebView 的虚拟显示行为仍需安装包含此更改的应用进行设备验收。
