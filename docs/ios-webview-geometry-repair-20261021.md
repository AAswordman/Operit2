# iOS WebView 布局崩溃修复（2026-10-21）

## 反馈证据与判断边界

下载附件 `testflight_feedback-11.zip`、`testflight_feedback-12.zip` 都来自 2.0.0 (18)。两份异常栈均为 Flutter 合成平台视图 → `WebViewImpl.frame.setter` → `WKWebView.setFrame` → UIScrollView 调整滚动位置/安全区域边距 → `CALayer.setBounds` 抛异常 → SIGABRT。

市场“渡 · Ferry”的首次 onLoad 会更新 WebView key，是可能造成原生视图重建的一种触发方式；现有附件不足以证明它就是上述崩溃的根因。宿主修复不依赖修改该插件或任何市场包。

附件未提供异常名称、原因文本或几何数值。防护覆盖输入非法几何值，以及 QuartzCore 对非法几何值抛出的 `CALayerInvalidGeometry`，不能据此宣称复现了用户设备上的完整故障过程。

## 实现

`WebViewImpl` 在 iOS 继承 Objective-C 实现的 `OperitSafeWebView`；macOS 继续继承 WKWebView。CocoaPods 与 Swift Package Manager 都登记新增原生代码。

- 检查 frame/bounds 的坐标、尺寸与坐标加尺寸是否有限，拒绝负尺寸，允许正常的零尺寸挂载/卸载。
- 在调用 WKWebView 的尺寸更新前关闭自动安全区边距调整，修正非有限的滚动偏移/内容边距；保留原有 Flutter 边距补偿行为。
- 在 Objective-C 中包围 frame、bounds、layoutSubviews 和 safeAreaInsetsDidChange 的系统调用。Swift `do/catch` 无法捕获这种 NSException。
- 只隔离 `CALayerInvalidGeometry`。发生非法输入或该异常时，隐藏并停用受影响的原生 WebView，停止继续布局；不尝试复用可能已被系统部分修改的视图。
- 离开本次 UIKit 布局调用后停止页面加载，通过现有 WKNavigationDelegate → onWebResourceError 通道报告一次错误，页面可以使用已有加载失败 UI。后续正常创建的新 WebView 不受影响。
- 日志记录失败原因、frame、bounds、contentOffset 和 contentInset，方便新版真机反馈继续定位原始触发值。

## 验证

在本机 iOS 27.0 模拟器运行以下命令：

```sh
ruby tools/tests/run_ios_webview_geometry_tests.rb
```

脚本用 CocoaPods 构建生产 WebView 模块，在临时 UIKit 测试应用中链接它，不需要构建 App 的 Rust/runtime 资源。可传入 simulator UDID 选择设备；测试产物和 xcresult 路径会打印到终端。

共 11 项 XCTest 通过，包含：

- 零尺寸、正常尺寸更新、父视图自动缩放与多次创建/销毁。
- NaN、无穷大、负尺寸和坐标加尺寸溢出；非法初始 frame 与 bounds。
- 在 WKWebView superclass 入口注入布局异常，错误只报告一次，失败后重复合成不再进入系统布局，其他 WebView 仍可工作。
- 将真实 UIScrollView.layer.bounds 设置为非法值，由 QuartzCore 实际抛出 `CALayerInvalidGeometry`，确认进程存活且可使用新 WebView。
- 通过正式 Pigeon API 构造的 WebViewImpl 同样执行防护；原有三项边距行为回归。
- 无关原生异常仍保留原始异常，不被布局防护吞掉。

iOS simulator arm64、macOS arm64 的生产 WebView CocoaPods target 均编译通过；SwiftPM manifest 可解析，新增 Objective-C target 在 macOS 可编译。

尚未在反馈对应的 iOS 真机/系统版本上复现，未制作或发布新的 TestFlight 包。

## OCR 是另一条故障链

`testflight_feedback-14.zip` 的系统 TextRecognition 内存分配失败发生在 Vision 的内部工作线程，不能靠此处 Objective-C 的几何异常隔离处理。后续已单独加入图片尺寸/像素限制、解码缩图和识别内存峰值控制，见 [iOS OCR 内存峰值控制](ios-ocr-memory-repair-20261021.md)。真机原始场景仍待验证。
