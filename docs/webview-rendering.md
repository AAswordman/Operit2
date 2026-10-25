# WebView rendering and zoom

The Flutter application supports Android, iOS, macOS, Windows, Linux, OHOS and
Web. Application zoom and each browser's page zoom remain independent. Visually,
content is scaled by their product; returning application zoom to 1x must not
reset the browser's selected page zoom.

## Rendering strategy

`WebViewScaleScope` supplies application zoom without changing the host DPR.
`PlatformWebViewController.requiresNativeApplicationZoom` selects the strategy
by backend capability, not by a macOS-only branch in the shared widget.

| Backend | Application zoom | Page zoom | Input mapping |
| --- | --- | --- | --- |
| macOS / WKWebView | Unscaled native viewport + native content zoom | WKWebView pageZoom | AppKit uses a translation-only native frame |
| Linux / WebKitGTK | Unscaled native viewport + native content zoom | WebKitGTK zoom level | GTK receives a translation-only frame; no unsupported scale hiding |
| iOS / WKWebView | Flutter/UIKit composition | WKWebView pageZoom | Existing UIKit platform-view mapping |
| Android / WebView | Flutter platform-view composition | Existing WebView zoom API | Existing platform-view input mapping |
| Windows / WebView2 | Flutter/native composition | WebView2 zoom factor | Pointer forwarding uses Flutter-local coordinates |
| OHOS / ArkWeb | Flutter/OHOS platform-view composition | Main-frame CSS zoom via a per-engine bridge | Existing platform-view input mapping |
| Web / iframe | Flutter/DOM composition | Clipped, inversely sized and scaled iframe | Browser DOM transforms map pointer input, including cross-origin frames |

Native-overlay backends cancel the application paint scale around the viewport,
lay out the viewport at its displayed size, and send
`applicationZoom * pageZoom` to the native page API. `NativeWebViewZoom` retains
both factors, validates input and serializes native updates. The same widget
structure is retained at every scale to avoid recreating the platform view.

Composited backends retain application paint scaling. Sending the application
factor to their page APIs as well would scale content twice, so their capability
flag stays false. The iframe page-zoom implementation does not inspect its
embedded document and works with cross-origin content and JavaScript disabled.

Older ArkWeb SDKs lack an absolute native browser-zoom API. The OHOS page-zoom
bridge therefore uses page-wide CSS zoom with the application's JavaScript-enabled
browser. It retains a main-frame document-start script separately from user
scripts, applies zoom to the current document, and restores preexisting inline
root zoom when reset. Application zoom itself does not require page JavaScript.
## Windows composition

WebView2 supplies a composition visual directly to the Windows engine. Flutter
platform-view layers determine placement, transforms, clipping and paint order.
Backing stores use premultiplied-alpha DXGI swapchains. Native browser display
uses composition visuals, without a Flutter Texture or continuous screen capture.

ANGLE preserves framebuffer orientation. DirectComposition appends layers with
`AddVisual(visual, FALSE, nullptr)` in Flutter's back-to-front paint order. Flutter
dialogs and modal barriers must cover the browser and intercept pointer input.

The SDK version is pinned in [`.fvmrc`](../apps/flutter/app/.fvmrc). Run SDK and
application commands from `apps/flutter/app`:

```powershell
fvm install --skip-pub-get
fvm flutter precache --windows
fvm flutter pub get
fvm flutter run -d windows
```

## iOS geometry

The iOS backend uses `OperitSafeWebView` to validate finite frame and bounds
values before UIKit layout. Invalid geometry disables the affected native view;
`CALayerInvalidGeometry` is isolated at the Objective-C boundary and reported
once through the existing navigation-error channel after the layout call ends.
The failed view is no longer laid out or reused.

## Maintenance checks

From `apps/flutter/app`:

```powershell
fvm flutter test test/application_zoom_webview_test.dart test/windows_native_composition_test.dart
fvm flutter test test/webview_zoom_browser_test.dart --platform chrome
```

[`windows_native_composition_probe.dart`](../apps/flutter/app/tool/windows_native_composition_probe.dart)
is a separate acceptance fixture for clipping, ordering, dialogs and pointer
routing. [`webview_zoom.test.mjs`](../tools/tests/webview_zoom.test.mjs) checks
script and channel contracts. Native rendering, IME and gesture behavior must be
checked on the corresponding devices after engine or SDK changes.
