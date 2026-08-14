## 1.2.1

* 在 macOS 上忽略 `setBackgroundColor` 并打印日志，避免 WKWebView `opaque` / `backgroundColor` 未实现导致应用异常。
* 完善 `examples/platform` 的Linux 平台，在创建 WebKitGTK 视图前先将 Flutter view 挂到 `GtkOverlay`。

## 1.2.0

* 补齐各平台包对 `webview_flutter_platform_interface` 的覆盖。
* 新增 Linux WebKitGTK 平台专用 controller 创建参数与运行时设置，覆盖 developer extras、JavaScript 自动开窗、媒体播放、page cache、file URL 访问、文本缩放/字体大小、页面缩放与 DevTools 打开能力。
* 新增 Web iframe 平台专用创建参数与运行时属性设置，覆盖 `allow`、`sandbox`、`referrerpolicy` 与自定义 iframe 属性，并在 JavaScript mode 切换时保留用户自定义 sandbox。
* 新增 OHOS ArkWeb 平台专用 controller 创建参数与运行时 WebSettings setter，覆盖 DOM storage、JavaScript 自动开窗、多窗口、viewport/overview、缩放控件、file access、媒体手势策略、support zoom、text zoom 与全屏旋转。
* 为各平台控制器补齐显式 `loadFileWithParams` override。
* 当原生平台未提供证书数据时，平台 SSL 认证错误的 certificate 统一返回 `null`。
* 在转发到平台 Cookie 存储前统一校验通用 WebView Cookie。
* 避免 OHOS 在导航代理放行子 frame 请求后将其重放为主 frame 加载。
* HTTP 状态错误回调会在可用时带上平台专用的请求元数据与响应详情。
* OHOS JavaScript 执行结果改为优先按 JSON 解码，使字符串、数组、对象、布尔值和数字尽可能与其他平台的结构化返回行为一致。
* OHOS POST `loadRequest` 携带自定义请求头时改为明确失败，避免静默丢弃请求头，并记录 ArkWeb 的底层限制。
* Windows 遇到无法映射到通用资源类型的 WebView2 权限请求时改用默认决策，避免向应用暴露空资源请求。
* Linux 权限请求如果不包含任何可识别资源类型，将直接拒绝原生请求，避免向应用暴露空资源请求。
* 为主包封装补充 `WebViewController`、`NavigationDelegate`、权限请求与 `WebViewWidget` 的转发测试。
* 为主包和各平台包增加共享 analyzer lint 配置。
* 将 `examples/platform` 纳入本地验证，并审计其 path 包 lockfile 版本与工作区发布版本一致。
* 将示例 Android 工程更新到当前 Flutter Gradle 模板形态，app 模块不再直接应用 Kotlin Gradle 插件。
* 将示例 iOS 与 macOS 工程迁移为仅使用 Swift Package Manager，并移除模板 CocoaPods 集成。
* 恢复示例应用的 `cupertino_icons` 依赖，确保 Web release 构建包含所引用的图标字体。
* 补充 Linux 权限请求 grant/deny 分发与 Web user-agent reset 行为的回归测试。
* 补齐 OHOS 权限请求 grant 覆盖：支持 camera、microphone、MIDI sysex 与 protected media 资源，并安全拒绝未知资源。
* 移除 Web JavaScript dialog bridge 中触发 Flutter Web wasm dry-run 警告的运行时类型检查，并补充多 WebView dialog bridge 覆盖。
* 为 Windows 与 Linux 补齐 `loadRequest` 的请求 body/header 处理与 HTTP 状态错误回调覆盖。
* 为 Windows 与 Linux 增加原生 local storage 清理能力。
* 补齐 OHOS HTTP error 与 SSL auth 回调桥接。
* 加强 Web 实现：在浏览器允许的范围内补齐同源 JavaScript 执行、JavaScript channel、console 转发、alert/confirm/prompt 转发、滚动、滚动条、over-scroll、JavaScript mode、zoom 与权限请求覆盖。
* 新增显式 Web `PlatformSslAuthError` 实现，将可恢复证书决策标记为不支持，而不是让平台接口方法保持缺失。
* 新增 `WebWebViewWidgetCreationParams`，使 Web 平台与其他 federated 包的平台专用 widget creation params 模式保持一致。

## 1.1.2

* 将 `WebViewCookieManager.getCookies({required Uri domain})` 与上游 `webview_flutter` 公共 API 对齐。

## 1.1.1

* 示例改为使用 `abutil` 包进行平台判断。
* 简化示例应用中的 OHOS 与 Web 平台分支。
* 将平台包的许可证文件同步为主包许可证文本。

## 1.1.0

* 新增 OpenHarmony 平台实现支持。
* 完善 cookie API 覆盖：
  * 在主插件封装层新增通用 `WebViewCookieManager.getCookies({required Uri domain})` API。
  * 为各 federated 平台包实现并验证跨平台 cookie 读取能力。
  * 为 Windows WebView2 增加包含完整 cookie 元数据与删除流程的平台专用 API。
* 加强 Web 平台实现：
  * 为 `loadHtmlString` 与基于 XHR 的 `loadRequest` 保留逻辑 `currentUrl()`，避免向用户暴露内部 `data:` iframe URL。
  * 按 Flutter Web 生成的 `assets/` 目录解析资源，并正确编码资源路径片段。
  * 基于 XHR 的加载失败会通过 `onWebResourceError` 上报，且不再把不支持的自定义 user agent 覆盖误报为已生效。
  * 写入 `document.cookie` 前校验并编码浏览器可见 cookie，在文档中明确 iframe 与浏览器 cookie 限制。
* 完善 Linux 平台实现：
  * 修复 native WebView 可见性同步问题，稳定的 Flutter 帧不再把 GTK/WebKit 视图折叠为 `0x0`。
  * 加强 Linux frame、cookie 与 JavaScript channel 的入参校验，避免非法 native 状态和不安全脚本注入。
* 各平台子插件的更新日志统一为 `webview_all` 的更新内容。
* 统一主插件与各平台子插件的版本号。

## 1.0.3

* 文档更新。
* 依赖更新。

## 1.0.2

* 文档更新。

## 1.0.1

* 文档更新。

## 1.0.0

* 新增 Linux 支持。

## 0.9.3

* 问题修复。

## 0.9.2

* 问题修复。

## 0.9.1

* 重构：包含破坏性变更。

## 0.5.3

* 更新依赖。
  * 修复 MacOS 上 `opaque` 未实现导致的问题。

## 0.5.2

* 文档更新。

## 0.5.1

* 大版本依赖更新。
* 问题修复。

## 0.4.5

* 依赖更新。
* 问题修复。

## 0.4.3

* 依赖更新。

## 0.4.1

* 重构。

## 0.3.7

* 依赖更新。

## 0.3.6

* 文档更新。

## 0.3.5

* 文档更新。

## 0.3.4

* 文档更新。

## 0.3.3

* 文档更新。

## 0.3.1

* 修复 Web 相关问题。

## 0.2.4

* 依赖更新。

## 0.2.3

* 文档更新。

## 0.2.2

* 修复 Web 相关问题。

## 0.2.1

* 初步运行成功。

## 0.1.3

* 问题修复。

## 0.1.2

* 问题修复。

## 0.1.1

* 故事开始。
