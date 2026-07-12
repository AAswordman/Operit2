## 1.2.1

* Ignore `setBackgroundColor` on macOS and log the skipped call, avoiding application exceptions caused by unimplemented WKWebView `opaque` / `backgroundColor`.
* Improve the `examples/platform` Linux platform by attaching the Flutter view to a `GtkOverlay` before creating WebKitGTK views.

## 1.2.0

* Complete `webview_flutter_platform_interface` coverage across the federated platform packages.
* Add Linux WebKitGTK-specific controller creation parameters and runtime settings for developer extras, JavaScript window opening, media playback, page cache, file URL access, text zoom/font sizing, page zoom, and DevTools opening.
* Add Web iframe-specific creation parameters and runtime attribute setters for `allow`, `sandbox`, `referrerpolicy`, and custom iframe attributes while preserving custom sandbox values across JavaScript mode changes.
* Add OHOS ArkWeb-specific controller creation parameters and runtime WebSettings setters for DOM storage, JavaScript window opening, multiple windows, viewport/overview mode, zoom controls, file access, media gesture policy, support zoom, text zoom, and full-screen rotation.
* Add explicit cross-platform `loadFileWithParams` controller overrides.
* Return `null` for platform SSL auth error certificates when the native platform does not provide certificate data.
* Validate generic WebView cookies before forwarding them to platform cookie stores.
* Avoid replaying OHOS sub-frame navigation requests as main-frame loads after navigation delegate approval.
* Include platform-specific request metadata and response details when reporting HTTP status errors where available.
* Decode OHOS JavaScript evaluation results through JSON so strings, arrays, objects, booleans, and numbers match the structured result behavior of the other platforms where possible.
* Make OHOS POST `loadRequest` calls with custom headers fail explicitly instead of silently dropping headers, and document the ArkWeb limitation.
* Return the default WebView2 permission decision for unsupported Windows permission kinds instead of surfacing empty resource requests to applications.
* Deny Linux permission requests that contain no recognized resource types instead of surfacing empty resource requests to applications.
* Add main wrapper forwarding tests for `WebViewController`, `NavigationDelegate`, permission requests, and `WebViewWidget`.
* Add shared analyzer lint configuration for the main and federated platform packages.
* Add `examples/platform` to local validation and audit its path package lockfile versions against the workspace release version.
* Update the example Android project to the current Flutter Gradle template shape so it no longer applies the Kotlin Gradle plugin from the app module.
* Migrate the example iOS and macOS projects to Swift Package Manager-only integration and remove their template CocoaPods integration.
* Restore the example app's `cupertino_icons` dependency so Web release builds have all referenced icon fonts.
* Add regression coverage for Linux permission request grant/deny dispatch and Web user-agent reset behavior.
* Complete OHOS permission request grant coverage for camera, microphone, MIDI sysex, and protected media resources, with unknown resources denied safely.
* Remove a Web JavaScript dialog bridge runtime type check that triggered Flutter Web wasm dry-run warnings, and add multi-WebView dialog bridge coverage.
* Add Windows and Linux request body/header handling and HTTP status error coverage for `loadRequest`.
* Add Windows and Linux native local storage clearing.
* Complete OHOS HTTP error and SSL auth callback bridging.
* Harden the Web implementation with same-origin JavaScript execution, JavaScript channels, console forwarding, alert/confirm/prompt forwarding, scrolling, scrollbars, over-scroll, JavaScript mode, zoom, and permission request coverage where browsers allow it.
* Add an explicit Web `PlatformSslAuthError` implementation that reports unsupported recoverable certificate decisions instead of leaving the platform interface methods missing.
* Add `WebWebViewWidgetCreationParams` so the Web platform matches the platform-specific widget creation-params pattern used by other federated packages.

## 1.1.2

* Align `WebViewCookieManager.getCookies({required Uri domain})` with the upstream `webview_flutter` public API.

## 1.1.1

* Update the example app to use `abutil` for platform detection.
* Simplify the example app's OHOS and Web platform branches.
* Synchronize platform package license files with the main package license.

## 1.1.0

* Add OpenHarmony platform implementation support.
* Improve cookie API coverage:
  * Add the common `WebViewCookieManager.getCookies({required Uri domain})` API to the main plugin wrapper.
  * Implement and validate cross-platform cookie reads for the federated platform packages.
  * Add Windows WebView2-specific cookie APIs for full cookie metadata and deletion workflows.
* Harden the Web platform implementation:
  * Preserve logical `currentUrl()` values for `loadHtmlString` and XHR-backed `loadRequest` calls instead of exposing internal `data:` iframe URLs.
  * Resolve Flutter web assets through the generated `assets/` directory and encode asset path segments correctly.
  * Report XHR-backed load failures through `onWebResourceError` and keep unsupported user agent overrides from being reported as applied.
  * Validate and encode browser-visible cookies before writing to `document.cookie`, and document iframe and browser cookie limitations.
* Improve the Linux platform implementation:
  * Fix native WebView visibility synchronization so a stable Flutter frame no longer collapses the GTK/WebKit view to `0x0`.
  * Harden Linux frame, cookie, and JavaScript channel inputs to avoid invalid native state and unsafe script injection.
* Keep federated platform package changelogs aligned with the main `webview_all` updates.
* Synchronize the main plugin and federated platform package versions.

## 1.0.3

* doc update
* dep update

## 1.0.2

* doc update

## 1.0.1

* doc update

## 1.0.0

* add linux support

## 0.9.3

* bug fix

## 0.9.2

* bug fix

## 0.9.1

* refactor: breaking changes

## 0.5.3

* update dependences
    * fix the error opaque is not implemented on MacOs

## 0.5.2

* doc update  

## 0.5.1

* big dep update  
* bug fix

## 0.4.5

* dep update  
* bug fix

## 0.4.3

* dep update

## 0.4.1

* refactor

## 0.3.7

* dep update

## 0.3.6

* doc update

## 0.3.5

* doc update

## 0.3.4

* doc update

## 0.3.3

* doc update

## 0.3.1

* fix bugs about web

## 0.2.4

* dep update

## 0.2.3

* doc update

## 0.2.2

* fix bugs about web

## 0.2.1

* run successfully

## 0.1.3

* fix bugs

## 0.1.2

* fix bugs

## 0.1.1

* Our story begins
