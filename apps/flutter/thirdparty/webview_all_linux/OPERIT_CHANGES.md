# Operit integration

Based on webview_all_linux 1.4.4, retaining the upstream license and native
WebKitGTK implementation. The package is local so zoom changes are reproducible
without modifying the global Pub cache.

`LinuxWebViewController` opts into native application zoom. The common wrapper
removes application paint scaling and supplies a translation-only native frame,
which the upstream GTK geometry observer requires. WebKitGTK receives the product
of application zoom and page zoom through its existing `setZoomFactor` method.
Both factors are retained independently and native updates are serialized by
`NativeWebViewZoom`. Custom initial page zoom is also retained.

The fork uses local Flutter lint configuration and removes four upstream
`@override` annotations for adapter-specific extension methods that are not
members of Operit's shared interface. Their implementations remain unchanged.

The fork implements Operit's socket-free local resource contract. It registers
the `operit-vfs` URI scheme on each WebKit context, marks it secure and
CORS-enabled, and serves requests through `operit/webview_resources` to the
Dart handler registered by `setLocalResourceHandler`. Requests are kept alive
until the Dart bridge returns bytes, so management documents and their
subresources load entirely from the plugin VFS on Linux.
