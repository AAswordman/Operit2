# WebView All

[Documentation](https://abandoft.github.io/webview_all) | [中文文档](https://abandoft.github.io/webview_all/zh)

支持所有 Flutter 平台的 WebView 组件，遵守 [webview_flutter 平台接口](https://pub.dev/packages/webview_flutter_platform_interface)。

|     系统     | **支持情况** | **技术实现** |
|-------------|--------------|--------------|
|Android|API 24+|[WebView](https://developer.android.com/reference/android/webkit/WebView)|
|iOS|13.0+|[WKWebView](https://developer.apple.com/documentation/webkit/wkwebview)|
|macOS|10.15+|[WKWebView](https://developer.apple.com/documentation/webkit/wkwebview)|
|Windows|Win10 1809+|[WebView2](https://developer.microsoft.com/microsoft-edge/webview2)|
|Linux|webkit2gtk-4.1|[WebKitGTK](https://webkitgtk.org)|
|OHOS|API 12+|[ArkWeb](https://developer.huawei.com/consumer/cn/doc/harmonyos-references-V5/ts-basic-components-web-V5)|
|Web|Any|[js-interop](https://dart.dev/interop/js-interop)|

## 快速入门

1. 实例化一个 `WebViewController`:

```dart
controller = WebViewController()
  ..setJavaScriptMode(JavaScriptMode.unrestricted)
  ..loadRequest(Uri.parse('https://flutter.dev'));
```

2. 将 controller 传给 `WebViewWidget`:

```dart
@override
Widget build(BuildContext context) {
  return Scaffold(
    appBar: AppBar(title: const Text('Flutter Simple Example')),
    body: WebViewWidget(controller: controller),
  );
}
```

更详细的用法、接口覆盖和平台限制请参考[中文文档](https://abandoft.github.io/webview_all/zh)
