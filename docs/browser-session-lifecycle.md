# 浏览器会话与工作区展示

浏览器执行属于运行时宿主，会话是否存在不由工作区是否打开决定。Android、iOS、macOS、Windows、Linux 共用相同的会话和展示生命周期，平台实现仍由现有 WebView 插件承担；Web 客户端通过现有会话协议观看宿主合成画面。

## 请求链路

```text
browser_* 工具
  → StandardBrowserAutomationTools / BrowserAutomationHost
  → FlutterHostAdapters → RuntimeHostInteractionService
  → RuntimeHostInteractionSubscriber → BrowserAutomationBridge
  → RuntimeBrowserOwnerHost → RuntimeBrowserSessionRegistry
  → RuntimeBrowserOwner 持有的 WebViewController

visit_web 工具
  → FlutterHostAdapters → RuntimeHostInteractionService
  → RuntimeHostInteractionSubscriber → WebVisitBridge
  → RuntimeBrowserOwnerHost → RuntimeWebVisitRegistry / RuntimeWebVisitSession
  → BrowserSessions（现有 create / evaluate / events / close 协议）
  → RuntimeBrowserService → 同一个 RuntimeBrowserOwner
```

宿主交互订阅并发处理请求，因此 `visit_web` 等待期间仍可处理它发起的浏览器会话命令。Core 会话及事件协议没有添加 Android 专用操作。

## 会话与展示分离

- `RuntimeBrowserOwnerSurfaceHost` 为每个存活会话保留一个真实 WebView，始终位于全局宿主。打开、隐藏和切换工作区不会迁移它的 Element 或重新创建平台视图。
- `WorkspaceBrowserSurface` 是普通浏览器和临时网页访问共用的展示入口。原生展示提供 `RuntimeBrowserSurfaceTarget` 锚点；Web 展示继续使用已有的合成画面传输。
- 锚点通过 RenderObject 的 attach、layout、detach 发布展示位置和大小。`RetainedPage` 隐藏嵌套页面时，即使 widget 未 dispose，也会释放锚点。恢复时重新提供锚点，不依赖休眠子树完成构建。
- `RuntimeBrowserSurface` 在相同挂载位置跟随锚点。没有锚点时禁用绘制、触摸和语义，但保留 WebView 及最后的非零视口。首次后台创建使用宿主视口，不使用屏幕外 1×1 布局。
- 锚点只表达展示权，不表达会话存活权。显式关闭会话才销毁浏览器；多个展示锚点使用最后注册的存活锚点，释放后可回到先前锚点。
- TTS 和运行时浮层位于浏览器宿主之上，避免全局浏览器展示覆盖应用浮层。

## 临时访问与等待

`RuntimeWebVisitSession` 在收到请求时直接创建会话并开始加载、检测和提取。工作区展示回调可以尚未注册或已被移除，执行仍可完成；后注册的工作区可以展示仍在执行的访问。展示失败只记录日志。

工作区观察任务状态并提供手动提取、取消等操作，不拥有定时器、加载过程或工具返回的 Completer。工作区销毁不会取消任务；用户明确关闭访问标签才取消该请求。正常完成、失败和取消都会关闭临时会话。创建请求迟到时也会关闭已经失去任务所有者的会话。

`visit_web` 总执行期限为 45 秒，单次脚本和清理分别最多 10 秒，为 Rust 宿主现有的 60 秒请求期限留出返回时间。脚本等待同时观察任务完成，取消时释放本地超时；事件订阅取消和会话关闭同时发起，避免互相等待。`browser_wait_for` 的文本条件由宿主进行限时轮询，不依赖页面内定时器或原生 WebView 对 Promise 返回值的处理。

## 回归验证

Flutter 测试覆盖未打开工作区的访问、延迟展示、展示异常、并发取消、迟到创建、脚本无响应、嵌套休眠、视图实例保持和偏移后的触摸命中。它们验证共享生命周期和控制器契约；各平台真实 WebView 的渲染、输入和后台行为仍需设备验证。
