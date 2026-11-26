// Copyright 2013 The Flutter Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'dart:async';
import 'dart:ui';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart' show BinaryMessenger;

import 'package:flutter/widgets.dart' show WidgetsFlutterBinding;

import 'channels/api_implementations.dart';
import 'channels/host_api.dart'
    show FlutterAssetManagerHostApi, InstanceManagerHostApi;
import 'core/instance_manager.dart';

export 'channels/api_implementations.dart'
    show ConsoleMessage, ConsoleMessageLevel, FileChooserMode;

String _singleLineOhosNativeLogValue(Object value) {
  return value.toString().replaceAll(RegExp(r'[\r\n]+'), ' ');
}

/// Root of the OHOS WebView bridge object hierarchy.
class OhosObject with Copyable {
  /// Constructs a [OhosObject] without creating the associated native object.
  ///
  /// This should only be used by subclasses created by this library or to
  /// create copies.
  @protected
  OhosObject.detached({
    BinaryMessenger? binaryMessenger,
    InstanceManager? instanceManager,
  }) : _api = OhosObjectHostApiImpl(
         binaryMessenger: binaryMessenger,
         instanceManager: instanceManager,
       );

  /// Global instance of [InstanceManager].
  static final InstanceManager globalInstanceManager = _initInstanceManager();

  static InstanceManager _initInstanceManager() {
    WidgetsFlutterBinding.ensureInitialized();
    // Clears the native `InstanceManager` on initial use of the Dart one.
    unawaited(
      InstanceManagerHostApi().clear().catchError((
        Object error,
        StackTrace stackTrace,
      ) {
        debugPrint(
          'webview_all_ohos: failed to clear native instances: '
          '${_singleLineOhosNativeLogValue(error)}',
        );
      }),
    );
    return InstanceManager(
      onWeakReferenceRemoved: (int identifier) {
        unawaited(
          OhosObjectHostApiImpl().dispose(identifier).catchError((
            Object error,
            StackTrace stackTrace,
          ) {
            debugPrint(
              'webview_all_ohos: failed to release native instance '
              '$identifier: ${_singleLineOhosNativeLogValue(error)}',
            );
          }),
        );
      },
    );
  }

  /// Host channel implementation for [OhosObject].
  final OhosObjectHostApiImpl _api;

  /// Releases the Dart-side reference to the paired native instance.
  static void dispose(OhosObject instance) {
    instance._api.instanceManager.removeWeakReference(instance);
  }

  @override
  OhosObject copy() {
    return OhosObject.detached();
  }
}

/// A callback interface used by the host application to set the Geolocation
/// permission state for an origin.
@immutable
class GeolocationPermissionsCallback extends OhosObject {
  /// Instantiates a [GeolocationPermissionsCallback] without creating and
  /// attaching to an instance of the associated native class.
  ///
  /// This should only be used outside of tests by subclasses created by this
  /// library or to create a copy.
  @protected
  GeolocationPermissionsCallback.detached({
    super.binaryMessenger,
    super.instanceManager,
  }) : _geolocationPermissionsCallbackApi =
           GeolocationPermissionsCallbackHostApiImpl(
             binaryMessenger: binaryMessenger,
             instanceManager: instanceManager,
           ),
       super.detached();

  final GeolocationPermissionsCallbackHostApiImpl
  _geolocationPermissionsCallbackApi;

  /// Sets the Geolocation permission state for the supplied origin.
  ///
  /// [origin]: The origin for which permissions are set.
  ///
  /// [allow]: Whether or not the origin should be allowed to use the Geolocation API.
  ///
  /// [retain]: Whether the permission should be retained beyond the lifetime of
  /// a page currently being displayed by a WebView.
  Future<void> invoke(String origin, bool allow, bool retain) {
    return _geolocationPermissionsCallbackApi.invokeFromInstances(
      this,
      origin,
      allow,
      retain,
    );
  }

  @override
  GeolocationPermissionsCallback copy() {
    return GeolocationPermissionsCallback.detached(
      binaryMessenger: _geolocationPermissionsCallbackApi.binaryMessenger,
      instanceManager: _geolocationPermissionsCallbackApi.instanceManager,
    );
  }
}

/// An Ohos View that displays web pages.
///
/// **Basic usage**
/// In most cases, we recommend using a standard web browser, like Chrome, to
/// deliver content to the user. To learn more about web browsers, read the
/// guide on invoking a browser with
/// [url_launcher](https://pub.dev/packages/url_launcher).
///
/// WebView objects allow you to display web content as part of your widget
/// layout, but lack some of the features of fully-developed browsers. A WebView
/// is useful when you need increased control over the UI and advanced
/// configuration options that will allow you to embed web pages in a
/// specially-designed environment for your app.
///
/// When a [WebView] is no longer needed [release] must be called.
class WebView extends OhosObject {
  /// Constructs a new WebView.
  ///
  /// Due to changes in Flutter 3.0 the [useHybridComposition] doesn't have
  /// any effect and should not be exposed publicly. More info here:
  /// https://github.com/flutter/flutter/issues/108106
  WebView({
    this.onScrollChanged,
    @visibleForTesting super.binaryMessenger,
    @visibleForTesting super.instanceManager,
  }) : super.detached() {
    api.createFromInstance(this);
  }

  /// Constructs a [WebView] without creating the associated native object.
  ///
  /// This should only be used by subclasses created by this library or to
  /// create copies.
  @protected
  WebView.detached({
    this.onScrollChanged,
    super.binaryMessenger,
    super.instanceManager,
  }) : super.detached();

  /// Host channel implementation for [WebView].
  @visibleForTesting
  static WebViewHostApiImpl api = WebViewHostApiImpl();

  /// The [WebSettings] object used to control the settings for this WebView.
  late final WebSettings settings = WebSettings(this);

  /// Called in response to an internal scroll in this view
  /// (i.e., the view scrolled its own contents).
  ///
  /// This is typically as a result of [scrollBy] or [scrollTo]
  /// having been called.
  final void Function(int left, int top, int oldLeft, int oldTop)?
  onScrollChanged;

  /// Enables debugging of web contents (HTML / CSS / JavaScript) loaded into any WebViews of this application.
  ///
  /// This flag can be enabled in order to facilitate debugging of web layouts
  /// and JavaScript code running inside WebViews. Please refer to [WebView]
  /// documentation for the debugging guide. The default is false.
  static Future<void> setWebContentsDebuggingEnabled(bool enabled) {
    return api.setWebContentsDebuggingEnabled(enabled);
  }

  /// Loads the given data into this WebView using a 'data' scheme URL.
  ///
  /// Note that JavaScript's same origin policy means that script running in a
  /// page loaded using this method will be unable to access content loaded
  /// using any scheme other than 'data', including 'http(s)'. To avoid this
  /// restriction, use [loadDataWithBaseURL()] with an appropriate base URL.
  ///
  /// The [encoding] parameter specifies whether the data is base64 or URL
  /// encoded. If the data is base64 encoded, the value of the encoding
  /// parameter must be `'base64'`. HTML can be encoded with
  /// `base64.encode(bytes)` like so:
  /// ```dart
  /// import 'dart:convert';
  ///
  /// final unencodedHtml = '''
  ///   <html><body>'%28' is the code for '('</body></html>
  /// ''';
  /// final encodedHtml = base64.encode(utf8.encode(unencodedHtml));
  /// print(encodedHtml);
  /// ```
  ///
  /// The [mimeType] parameter specifies the format of the data. If WebView
  /// can't handle the specified MIME type, it will download the data. If
  /// `null`, defaults to 'text/html'.
  Future<void> loadData({
    required String data,
    String? mimeType,
    String? encoding,
  }) {
    return api.loadDataFromInstance(this, data, mimeType, encoding);
  }

  /// Loads the given data into this WebView.
  ///
  /// The [baseUrl] is used as base URL for the content. It is used  both to
  /// resolve relative URLs and when applying JavaScript's same origin policy.
  ///
  /// The [historyUrl] is used for the history entry.
  ///
  /// The [mimeType] parameter specifies the format of the data. If WebView
  /// can't handle the specified MIME type, it will download the data. If
  /// `null`, defaults to 'text/html'.
  ///
  /// Note that content specified in this way can access local device files (via
  /// 'file' scheme URLs) only if baseUrl specifies a scheme other than 'http',
  /// 'https', 'ftp', 'ftps', 'about' or 'javascript'.
  ///
  /// If the base URL uses the data scheme, this method is equivalent to calling
  /// [loadData] and the [historyUrl] is ignored, and the data will be treated
  /// as part of a data: URL, including the requirement that the content be
  /// URL-encoded or base64 encoded. If the base URL uses any other scheme, then
  /// the data will be loaded into the WebView as a plain string (i.e. not part
  /// of a data URL) and any URL-encoded entities in the string will not be
  /// decoded.
  ///
  /// Note that the [baseUrl] is sent in the 'Referer' HTTP header when
  /// requesting subresources (images, etc.) of the page loaded using this
  /// method.
  ///
  /// If a valid HTTP or HTTPS base URL is not specified in [baseUrl], then
  /// content loaded using this method will have a `window.origin` value of
  /// `"null"`. This must not be considered to be a trusted origin by the
  /// application or by any JavaScript code running inside the WebView (for
  /// example, event sources in DOM event handlers or web messages), because
  /// malicious content can also create frames with a null origin. If you need
  /// to identify the main frame's origin in a trustworthy way, you should use a
  /// valid HTTP or HTTPS base URL to set the origin.
  Future<void> loadDataWithBaseUrl({
    String? baseUrl,
    required String data,
    String? mimeType,
    String? encoding,
    String? historyUrl,
  }) {
    return api.loadDataWithBaseUrlFromInstance(
      this,
      baseUrl,
      data,
      mimeType,
      encoding,
      historyUrl,
    );
  }

  /// Loads the given URL with additional HTTP headers, specified as a map from name to value.
  ///
  /// Note that if this map contains any of the headers that are set by default
  /// by this WebView, such as those controlling caching, accept types or the
  /// User-Agent, their values may be overridden by this WebView's defaults.
  ///
  /// Also see compatibility note on [evaluateJavascript].
  Future<void> loadUrl(String url, Map<String, String> headers) {
    return api.loadUrlFromInstance(this, url, headers);
  }

  /// Loads the URL with postData using "POST" method into this WebView.
  ///
  /// If url is not a network URL, it will be loaded with [loadUrl] instead, ignoring the postData param.
  Future<void> postUrl(String url, Uint8List data) {
    return api.postUrlFromInstance(this, url, data);
  }

  /// Gets the URL for the current page.
  ///
  /// This is not always the same as the URL passed to
  /// [WebViewClient.onPageStarted] because although the load for that URL has
  /// begun, the current page may not have changed.
  ///
  /// Returns null if no page has been loaded.
  Future<String?> getUrl() {
    return api.getUrlFromInstance(this);
  }

  /// Whether this WebView has a back history item.
  Future<bool> canGoBack() {
    return api.canGoBackFromInstance(this);
  }

  /// Whether this WebView has a forward history item.
  Future<bool> canGoForward() {
    return api.canGoForwardFromInstance(this);
  }

  /// Goes back in the history of this WebView.
  Future<void> goBack() {
    return api.goBackFromInstance(this);
  }

  /// Goes forward in the history of this WebView.
  Future<void> goForward() {
    return api.goForwardFromInstance(this);
  }

  /// Reloads the current URL.
  Future<void> reload() {
    return api.reloadFromInstance(this);
  }

  /// Clears the resource cache.
  ///
  /// Note that the cache is per-application, so this will clear the cache for
  /// all WebViews used.
  Future<void> clearCache(bool includeDiskFiles) {
    return api.clearCacheFromInstance(this, includeDiskFiles);
  }

  /// Asynchronously evaluates JavaScript in the context of the currently displayed page.
  ///
  /// If non-null, the returned value will be any result returned from that
  /// execution.
  ///
  /// The script runs in the currently loaded page. Register a
  /// [JavaScriptChannel] when persistent Dart-to-page communication is needed
  /// across navigations.
  Future<String?> evaluateJavascript(String javascriptString) {
    return api.evaluateJavascriptFromInstance(this, javascriptString);
  }

  /// Gets the title for the current page.
  ///
  /// Returns null if no page has been loaded.
  Future<String?> getTitle() {
    return api.getTitleFromInstance(this);
  }

  /// Sets the absolute scroll position of this WebView.
  ///
  /// Native scroll updates are reported through [onScrollChanged].
  Future<void> scrollTo(int x, int y) {
    return api.scrollToFromInstance(this, x, y);
  }

  /// Moves the current scroll position of this WebView by the given offset.
  ///
  /// Native scroll updates are reported through [onScrollChanged].
  Future<void> scrollBy(int x, int y) {
    return api.scrollByFromInstance(this, x, y);
  }

  /// Return the scrolled left position of this view.
  ///
  /// This is the left edge of the displayed part of your view. You do not
  /// need to draw any pixels farther left, since those are outside of the frame
  /// of your view on screen.
  Future<int> getScrollX() {
    return api.getScrollXFromInstance(this);
  }

  /// Return the scrolled top position of this view.
  ///
  /// This is the top edge of the displayed part of your view. You do not need
  /// to draw any pixels above it, since those are outside of the frame of your
  /// view on screen.
  Future<int> getScrollY() {
    return api.getScrollYFromInstance(this);
  }

  /// Returns the X and Y scroll position of this view.
  Future<Offset> getScrollPosition() {
    return api.getScrollPositionFromInstance(this);
  }

  /// Sets the [WebViewClient] that will receive various notifications and requests.
  ///
  /// This will replace the current handler.
  Future<void> setWebViewClient(WebViewClient webViewClient) {
    return api.setWebViewClientFromInstance(this, webViewClient);
  }

  /// Injects the supplied [JavascriptChannel] into this WebView.
  ///
  /// The object is injected into all frames of the web page, including all the
  /// iframes, using the supplied name. This allows the object's methods to
  /// be accessed from JavaScript.
  ///
  /// Note that injected objects will not appear in JavaScript until the page is
  /// next (re)loaded. JavaScript should be enabled before injecting the object.
  /// For example:
  ///
  /// ```dart
  /// webview.settings.setJavaScriptEnabled(true);
  /// webView.addJavascriptChannel(JavScriptChannel("injectedObject"));
  /// webView.loadUrl("about:blank", <String, String>{});
  /// webView.loadUrl("javascript:injectedObject.postMessage("Hello, World!")", <String, String>{});
  /// ```
  ///
  /// **Important**
  /// * Because the object is exposed to all the frames, any frame could obtain
  /// the object name and call methods on it. There is no way to tell the
  /// calling frame's origin from the app side, so the app must not assume that
  /// the caller is trustworthy unless the app can guarantee that no third party
  /// content is ever loaded into the WebView even inside an iframe.
  Future<void> addJavaScriptChannel(JavaScriptChannel javaScriptChannel) {
    JavaScriptChannel.api.createFromInstance(javaScriptChannel);
    return api.addJavaScriptChannelFromInstance(this, javaScriptChannel);
  }

  /// Removes a previously injected [JavaScriptChannel] from this WebView.
  ///
  /// Note that the removal will not be reflected in JavaScript until the page
  /// is next (re)loaded. See [addJavaScriptChannel].
  Future<void> removeJavaScriptChannel(JavaScriptChannel javaScriptChannel) {
    JavaScriptChannel.api.createFromInstance(javaScriptChannel);
    return api.removeJavaScriptChannelFromInstance(this, javaScriptChannel);
  }

  /// Registers the interface to be used when content can not be handled by the rendering engine, and should be downloaded instead.
  ///
  /// This will replace the current handler.
  Future<void> setDownloadListener(DownloadListener? listener) {
    return api.setDownloadListenerFromInstance(this, listener);
  }

  /// Sets the chrome handler.
  ///
  /// This is an implementation of [WebChromeClient] for use in handling
  /// JavaScript dialogs, favicons, titles, and the progress. This will replace
  /// the current handler.
  Future<void> setWebChromeClient(WebChromeClient? client) {
    return api.setWebChromeClientFromInstance(this, client);
  }

  @override
  WebView copy() {
    return WebView.detached(
      onScrollChanged: onScrollChanged,
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}

/// Manages cookies globally for all webviews.
class CookieManager extends OhosObject {
  /// Instantiates a [CookieManager] without creating and attaching to an
  /// instance of the associated native class.
  ///
  /// This should only be used outside of tests by subclasses created by this
  /// library or to create a copy for an [InstanceManager].
  @protected
  CookieManager.detached({super.binaryMessenger, super.instanceManager})
    : _cookieManagerApi = CookieManagerHostApiImpl(
        binaryMessenger: binaryMessenger,
        instanceManager: instanceManager,
      ),
      super.detached();

  static final CookieManager _instance = CookieManagerHostApiImpl()
      .attachInstanceFromInstances(CookieManager.detached());

  final CookieManagerHostApiImpl _cookieManagerApi;

  /// Access a static field synchronously.
  static CookieManager get instance {
    OhosWebViewFlutterApis.instance.ensureSetUp();
    return _instance;
  }

  /// Sets a single cookie (key-value pair) for the given URL. Any existing
  /// cookie with the same host, path and name will be replaced with the new
  /// cookie. The cookie being set will be ignored if it is expired. To set
  /// multiple cookies, your application should invoke this method multiple
  /// times.
  ///
  /// The value parameter must follow the format of the Set-Cookie HTTP
  /// response header defined by RFC6265bis. This is a key-value pair of the
  /// form "key=value", optionally followed by a list of cookie attributes
  /// delimited with semicolons (ex. "key=value; Max-Age=123"). Please consult
  /// the RFC specification for a list of valid attributes.
  ///
  /// Note: if specifying a value containing the "Secure" attribute, url must
  /// use the "https://" scheme.
  ///
  /// Params:
  /// url – the URL for which the cookie is to be set
  /// value – the cookie as a string, using the format of the 'Set-Cookie' HTTP response header
  Future<void> setCookie(String url, String value) {
    return _cookieManagerApi.setCookieFromInstances(this, url, value);
  }

  /// Returns cookies for the given URL as a Cookie request header string.
  Future<String> getCookies(String url) {
    return _cookieManagerApi.getCookiesFromInstances(this, url);
  }

  /// Removes all cookies.
  ///
  /// The returned future resolves to true if any cookies were removed.
  Future<bool> removeAllCookies() {
    return _cookieManagerApi.removeAllCookiesFromInstances(this);
  }

  /// Sets whether the WebView should allow third party cookies to be set.
  ///
  /// The default behavior is controlled by the platform WebView engine and may
  /// vary across HarmonyOS/OpenHarmony versions.
  Future<void> setAcceptThirdPartyCookies(WebView webView, bool accept) {
    return _cookieManagerApi.setAcceptThirdPartyCookiesFromInstances(
      this,
      webView,
      accept,
    );
  }

  @override
  CookieManager copy() {
    return CookieManager.detached(
      binaryMessenger: _cookieManagerApi.binaryMessenger,
      instanceManager: _cookieManagerApi.instanceManager,
    );
  }
}

/// Manages settings state for a [WebView].
///
/// When a WebView is first created, it obtains a set of default settings. These
/// default settings will be returned from any getter call. A WebSettings object
/// obtained from [WebView.settings] is tied to the life of the WebView. If a
/// WebView has been destroyed, any method call on [WebSettings] will throw an
/// Exception.
class WebSettings extends OhosObject {
  /// Constructs a [WebSettings].
  ///
  /// This constructor is only used for testing. An instance should be obtained
  /// with [WebView.settings].
  @visibleForTesting
  WebSettings(
    WebView webView, {
    @visibleForTesting super.binaryMessenger,
    @visibleForTesting super.instanceManager,
  }) : super.detached() {
    api.createFromInstance(this, webView);
  }

  /// Constructs a [WebSettings] without creating the associated native object.
  ///
  /// This should only be used by subclasses created by this library or to
  /// create copies.
  @protected
  WebSettings.detached({super.binaryMessenger, super.instanceManager})
    : super.detached();

  /// Host channel implementation for [WebSettings].
  @visibleForTesting
  static WebSettingsHostApiImpl api = WebSettingsHostApiImpl();

  /// Sets whether the DOM storage API is enabled.
  ///
  /// The default value is false.
  Future<void> setDomStorageEnabled(bool flag) {
    return api.setDomStorageEnabledFromInstance(this, flag);
  }

  /// Tells JavaScript to open windows automatically.
  ///
  /// This applies to the JavaScript function `window.open()`. The default is
  /// false.
  Future<void> setJavaScriptCanOpenWindowsAutomatically(bool flag) {
    return api.setJavaScriptCanOpenWindowsAutomaticallyFromInstance(this, flag);
  }

  /// Sets whether the WebView should support multiple windows.
  ///
  /// Native window creation requests are delivered through [WebChromeClient].
  ///
  /// The default is false.
  Future<void> setSupportMultipleWindows(bool support) {
    return api.setSupportMultipleWindowsFromInstance(this, support);
  }

  /// Sets the background color of this WebView.
  Future<void> setBackgroundColor(Color color) {
    return api.setBackgroundColorFromInstance(this, color.toARGB32());
  }

  /// Tells the WebView to enable JavaScript execution.
  ///
  /// The default is false.
  Future<void> setJavaScriptEnabled(bool flag) {
    return api.setJavaScriptEnabledFromInstance(this, flag);
  }

  /// Sets the WebView's user-agent string.
  ///
  /// If the string is empty, the system default value will be used. Changing
  /// the user-agent while loading a web page can cause the WebView to
  /// initiate loading once again.
  Future<void> setUserAgentString(String? userAgentString) {
    return api.setUserAgentStringFromInstance(this, userAgentString);
  }

  /// Sets whether the WebView requires a user gesture to play media.
  ///
  /// The default is true.
  Future<void> setMediaPlaybackRequiresUserGesture(bool require) {
    return api.setMediaPlaybackRequiresUserGestureFromInstance(this, require);
  }

  /// Sets whether the WebView should support zooming using its on-screen zoom controls and gestures.
  ///
  /// The particular zoom mechanisms that should be used can be set with
  /// [setBuiltInZoomControls].
  ///
  /// The default is true.
  Future<void> setSupportZoom(bool support) {
    return api.setSupportZoomFromInstance(this, support);
  }

  /// Sets whether the WebView loads pages in overview mode, that is, zooms out the content to fit on screen by width.
  ///
  /// This setting is taken into account when the content width is greater than
  /// the width of the WebView control, for example, when [setUseWideViewPort]
  /// is enabled.
  ///
  /// The default is false.
  Future<void> setLoadWithOverviewMode(bool overview) {
    return api.setLoadWithOverviewModeFromInstance(this, overview);
  }

  /// Sets whether the WebView should enable support for the "viewport" HTML meta tag or should use a wide viewport.
  ///
  /// When the value of the setting is false, the layout width is always set to
  /// the width of the WebView control in device-independent (CSS) pixels. When
  /// the value is true and the page contains the viewport meta tag, the value
  /// of the width specified in the tag is used. If the page does not contain
  /// the tag or does not provide a width, then a wide viewport will be used.
  Future<void> setUseWideViewPort(bool use) {
    return api.setUseWideViewPortFromInstance(this, use);
  }

  /// Sets whether the WebView should display on-screen zoom controls when using the built-in zoom mechanisms.
  ///
  /// See [setBuiltInZoomControls]. The default is true. However, on-screen zoom
  /// controls are deprecated in Ohos so it's recommended to set this to
  /// false.
  Future<void> setDisplayZoomControls(bool enabled) {
    return api.setDisplayZoomControlsFromInstance(this, enabled);
  }

  /// Sets whether the WebView should use its built-in zoom mechanisms.
  ///
  /// The built-in zoom mechanisms comprise on-screen zoom controls, which are
  /// displayed over the WebView's content, and the use of a pinch gesture to
  /// control zooming. Whether or not these on-screen controls are displayed can
  /// be set with [setDisplayZoomControls]. The default is false.
  ///
  /// The built-in mechanisms are the only currently supported zoom mechanisms,
  /// so it is recommended that this setting is always enabled. However,
  /// on-screen zoom controls are deprecated in Ohos so it's recommended to
  /// disable [setDisplayZoomControls].
  Future<void> setBuiltInZoomControls(bool enabled) {
    return api.setBuiltInZoomControlsFromInstance(this, enabled);
  }

  /// Enables or disables file access within WebView.
  ///
  /// This enables or disables file system access only. Assets and resources are
  /// still accessible using file:///ohos_asset and file:///ohos_res. The
  /// platform default can vary by device and system WebView version.
  Future<void> setAllowFileAccess(bool enabled) {
    return api.setAllowFileAccessFromInstance(this, enabled);
  }

  /// Sets the text zoom of the page in percent.
  ///
  /// The default is 100.
  Future<void> setTextZoom(int textZoom) {
    return api.setSetTextZoomFromInstance(this, textZoom);
  }

  /// Gets the WebView's user-agent string.
  Future<String> getUserAgentString() {
    return api.getUserAgentStringFromInstance(this);
  }

  /// Enables or disables full screen rotate within WebView.
  Future<void> setAllowFullScreenRotate(bool enabled) {
    return api.setAllowFullScreenRotateInstance(this, enabled);
  }

  @override
  WebSettings copy() {
    return WebSettings.detached(
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}

/// Exposes a channel to receive calls from javaScript.
///
/// See [WebView.addJavaScriptChannel].
class JavaScriptChannel extends OhosObject {
  /// Constructs a [JavaScriptChannel].
  JavaScriptChannel(
    this.channelName, {
    required this.postMessage,
    @visibleForTesting super.binaryMessenger,
    @visibleForTesting super.instanceManager,
  }) : super.detached() {
    OhosWebViewFlutterApis.instance.ensureSetUp();
    api.createFromInstance(this);
  }

  /// Constructs a [JavaScriptChannel] without creating the associated TS
  /// object.
  ///
  /// This should only be used by subclasses created by this library or to
  /// create copies.
  @protected
  JavaScriptChannel.detached(
    this.channelName, {
    required this.postMessage,
    super.binaryMessenger,
    super.instanceManager,
  }) : super.detached();

  /// Host channel implementation for [JavaScriptChannel].
  @visibleForTesting
  static JavaScriptChannelHostApiImpl api = JavaScriptChannelHostApiImpl();

  /// Used to identify this object to receive messages from javaScript.
  final String channelName;

  /// Callback method when javaScript calls `postMessage` on the object instance passed.
  final void Function(String message) postMessage;

  @override
  JavaScriptChannel copy() {
    return JavaScriptChannel.detached(
      channelName,
      postMessage: postMessage,
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}

/// Receive various notifications and requests for [WebView].
class WebViewClient extends OhosObject {
  /// Constructs a [WebViewClient].
  WebViewClient({
    this.onPageStarted,
    this.onPageFinished,
    this.onReceivedRequestError,
    this.onReceivedHttpError,
    @Deprecated('Only called on Ohos version < 23.') this.onReceivedError,
    this.requestLoading,
    this.urlLoading,
    this.doUpdateVisitedHistory,
    this.onReceivedHttpAuthRequest,
    this.onReceivedSslAuthError,
    @visibleForTesting super.binaryMessenger,
    @visibleForTesting super.instanceManager,
  }) : super.detached() {
    OhosWebViewFlutterApis.instance.ensureSetUp();
    api.createFromInstance(this);
  }

  /// Constructs a [WebViewClient] without creating the associated native object.
  ///
  /// This should only be used by subclasses created by this library or to
  /// create copies.
  @protected
  WebViewClient.detached({
    this.onPageStarted,
    this.onPageFinished,
    this.onReceivedRequestError,
    this.onReceivedHttpError,
    @Deprecated('Only called on Ohos version < 23.') this.onReceivedError,
    this.requestLoading,
    this.urlLoading,
    this.doUpdateVisitedHistory,
    this.onReceivedHttpAuthRequest,
    this.onReceivedSslAuthError,
    super.binaryMessenger,
    super.instanceManager,
  }) : super.detached();

  /// @ohos.web.netErrorList (ArkWeb网络协议栈错误列表)
  /// https://developer.huawei.com/consumer/cn/doc/harmonyos-references-V5/js-apis-neterrorlist-V5

  /// User authentication failed on server.
  static const int errorAuthentication = -100;

  /// Malformed URL.
  static const int errorBadUrl = -300;

  /// Failed to connect to the server.
  static const int errorConnect = -106;

  /// Failed to perform SSL handshake.
  static const int errorFailedSslHandshake = -148;

  /// Generic file error.
  static const int errorFile = -16;

  /// File not found.
  static const int errorFileNotFound = -808;

  /// Server or proxy hostname lookup failed.
  static const int errorHostLookup = -121;

  /// Failed to read or write to the server.
  static const int errorIO = -1;

  /// User authentication failed on proxy.
  static const int errorProxyAuthentication = -115;

  /// Too many redirects.
  static const int errorRedirectLoop = -303;

  /// Connection timed out.
  static const int errorTimeout = -409;

  /// Too many requests during this load.
  static const int errorTooManyRequests = -114;

  /// Generic error.
  static const int errorUnknown = -9;

  /// Resource load was canceled by Safe Browsing.
  static const int errorUnsafeResource = -311;

  /// Unsupported authentication scheme (not basic or digest).
  static const int errorUnsupportedAuthScheme = -339;

  /// Unsupported URI scheme.
  static const int errorUnsupportedScheme = -301;

  /// Host channel implementation for [WebViewClient].
  @visibleForTesting
  static WebViewClientHostApiImpl api = WebViewClientHostApiImpl();

  /// Notify the host application that a page has started loading.
  ///
  /// This method is called once for each main frame load so a page with iframes
  /// or framesets will call onPageStarted one time for the main frame. This
  /// also means that [onPageStarted] will not be called when the contents of an
  /// embedded frame changes, i.e. clicking a link whose target is an iframe, it
  /// will also not be called for fragment navigations (navigations to
  /// #fragment_id).
  final void Function(WebView webView, String url)? onPageStarted;

  /// Notify the host application that a page has finished loading.
  ///
  /// This method is called for main-frame navigations. Receiving an
  /// [onPageFinished] callback does not guarantee that the next frame drawn by
  /// WebView will reflect the final visual state of the DOM.
  final void Function(WebView webView, String url)? onPageFinished;

  /// Report web resource loading error to the host application.
  ///
  /// These errors usually indicate inability to connect to the server. Note
  /// that unlike the deprecated version of the callback, the new version will
  /// be called for any resource (iframe, image, etc.), not just for the main
  /// page. Thus, it is recommended to perform minimum required work in this
  /// callback.
  final void Function(
    WebView webView,
    WebResourceRequest request,
    WebResourceError error,
  )?
  onReceivedRequestError;

  /// Report HTTP error responses to the host application.
  final void Function(
    WebView webView,
    WebResourceRequest request,
    WebResourceResponse response,
  )?
  onReceivedHttpError;

  /// Report an error to the host application.
  ///
  /// These errors are unrecoverable (i.e. the main resource is unavailable).
  /// The errorCode parameter corresponds to one of the error* constants.
  @Deprecated('Only called on Ohos version < 23.')
  final void Function(
    WebView webView,
    int errorCode,
    String description,
    String failingUrl,
  )?
  onReceivedError;

  /// When the current [WebView] wants to load a URL.
  ///
  /// The value set by [setSynchronousReturnValueForShouldOverrideUrlLoading]
  /// indicates whether the [WebView] loaded the request.
  final void Function(WebView webView, WebResourceRequest request)?
  requestLoading;

  /// When the current [WebView] wants to load a URL.
  ///
  /// The value set by [setSynchronousReturnValueForShouldOverrideUrlLoading]
  /// indicates whether the [WebView] loaded the URL.
  final void Function(WebView webView, String url)? urlLoading;

  /// Notify the host application to update its visited links database.
  final void Function(WebView webView, String url, bool isReload)?
  doUpdateVisitedHistory;

  /// This callback is only called for requests that require HTTP authentication.
  final void Function(
    WebView webView,
    HttpAuthHandler handler,
    String host,
    String realm,
  )?
  onReceivedHttpAuthRequest;

  /// This callback is called for recoverable SSL certificate errors.
  final void Function(
    WebView webView,
    SslAuthHandler handler,
    String url,
    int errorCode,
    String description,
  )?
  onReceivedSslAuthError;

  /// Sets the required synchronous return value for the native method,
  /// `WebViewClient.shouldOverrideUrlLoading(...)`.
  ///
  /// The native method, `WebViewClient.shouldOverrideUrlLoading(...)`, requires
  /// a boolean to be returned and this method sets the returned value for all
  /// calls to the native method.
  ///
  /// Setting this to true causes the current [WebView] to abort loading any URL
  /// received by [requestLoading] or [urlLoading], while setting this to false
  /// causes the [WebView] to continue loading a URL as usual.
  ///
  /// Defaults to false.
  Future<void> setSynchronousReturnValueForShouldOverrideUrlLoading(
    bool value,
  ) {
    return api.setShouldOverrideUrlLoadingReturnValueFromInstance(this, value);
  }

  @override
  WebViewClient copy() {
    return WebViewClient.detached(
      onPageStarted: onPageStarted,
      onPageFinished: onPageFinished,
      onReceivedRequestError: onReceivedRequestError,
      onReceivedHttpError: onReceivedHttpError,
      // ignore: deprecated_member_use_from_same_package
      onReceivedError: onReceivedError,
      requestLoading: requestLoading,
      urlLoading: urlLoading,
      doUpdateVisitedHistory: doUpdateVisitedHistory,
      onReceivedHttpAuthRequest: onReceivedHttpAuthRequest,
      onReceivedSslAuthError: onReceivedSslAuthError,
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}

/// The interface to be used when content can not be handled by the rendering
/// engine for [WebView], and should be downloaded instead.
class DownloadListener extends OhosObject {
  /// Constructs a [DownloadListener].
  DownloadListener({
    required this.onDownloadStart,
    @visibleForTesting super.binaryMessenger,
    @visibleForTesting super.instanceManager,
  }) : super.detached() {
    OhosWebViewFlutterApis.instance.ensureSetUp();
    api.createFromInstance(this);
  }

  /// Constructs a [DownloadListener] without creating the associated TS
  /// object.
  ///
  /// This should only be used by subclasses created by this library or to
  /// create copies.
  @protected
  DownloadListener.detached({
    required this.onDownloadStart,
    super.binaryMessenger,
    super.instanceManager,
  }) : super.detached();

  /// Host channel implementation for [DownloadListener].
  @visibleForTesting
  static DownloadListenerHostApiImpl api = DownloadListenerHostApiImpl();

  /// Notify the host application that a file should be downloaded.
  final void Function(
    String url,
    String userAgent,
    String contentDisposition,
    String mimetype,
    int contentLength,
  )
  onDownloadStart;

  @override
  DownloadListener copy() {
    return DownloadListener.detached(
      onDownloadStart: onDownloadStart,
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}

/// Responsible for request the Geolocation API.
typedef GeolocationPermissionsShowPrompt =
    Future<void> Function(
      String origin,
      GeolocationPermissionsCallback callback,
    );

/// Responsible for request the Geolocation API is Cancel.
typedef GeolocationPermissionsHidePrompt =
    void Function(WebChromeClient instance);

/// Signature for the callback that is responsible for showing a custom view.
typedef ShowCustomViewCallback =
    void Function(
      WebChromeClient instance,
      View view,
      CustomViewCallback callback,
    );

/// Signature for the callback that is responsible for hiding a custom view.
typedef HideCustomViewCallback = void Function(WebChromeClient instance);

/// Handles JavaScript dialogs, favicons, titles, and the progress for [WebView].
class WebChromeClient extends OhosObject {
  /// Constructs a [WebChromeClient].
  WebChromeClient({
    this.onProgressChanged,
    this.onShowFileChooser,
    this.onPermissionRequest,
    this.onGeolocationPermissionsShowPrompt,
    this.onGeolocationPermissionsHidePrompt,
    this.onShowCustomView,
    this.onHideCustomView,
    this.onConsoleMessage,
    this.onJsAlert,
    this.onJsConfirm,
    this.onJsPrompt,
    @visibleForTesting super.binaryMessenger,
    @visibleForTesting super.instanceManager,
  }) : super.detached() {
    OhosWebViewFlutterApis.instance.ensureSetUp();
    api.createFromInstance(this);
  }

  /// Constructs a [WebChromeClient] without creating the associated TS
  /// object.
  ///
  /// This should only be used by subclasses created by this library or to
  /// create copies.
  @protected
  WebChromeClient.detached({
    this.onProgressChanged,
    this.onShowFileChooser,
    this.onPermissionRequest,
    this.onGeolocationPermissionsShowPrompt,
    this.onGeolocationPermissionsHidePrompt,
    this.onShowCustomView,
    this.onHideCustomView,
    this.onConsoleMessage,
    this.onJsAlert,
    this.onJsConfirm,
    this.onJsPrompt,
    super.binaryMessenger,
    super.instanceManager,
  }) : super.detached();

  /// Host channel implementation for [WebChromeClient].
  @visibleForTesting
  static WebChromeClientHostApiImpl api = WebChromeClientHostApiImpl();

  /// Notify the host application that a file should be downloaded.
  final void Function(WebView webView, int progress)? onProgressChanged;

  /// Indicates the client should show a file chooser.
  ///
  /// To handle the request for a file chooser with this callback, passing true
  /// to [setSynchronousReturnValueForOnShowFileChooser] is required. Otherwise,
  /// the returned list of strings will be ignored and the client will use the
  /// default handling of a file chooser request.
  ///
  /// Only invoked on Ohos versions 21+.
  final Future<List<String>> Function(
    WebView webView,
    FileChooserParams params,
  )?
  onShowFileChooser;

  /// Notify the host application that web content is requesting permission to
  /// access the specified resources and the permission currently isn't granted
  /// or denied.
  ///
  /// Only invoked on Ohos versions 21+.
  final void Function(WebChromeClient instance, PermissionRequest request)?
  onPermissionRequest;

  /// Indicates the client should handle geolocation permissions.
  final GeolocationPermissionsShowPrompt? onGeolocationPermissionsShowPrompt;

  /// Notify the host application that a request for Geolocation permissions,
  /// made with a previous call to [onGeolocationPermissionsShowPrompt] has been
  /// canceled.
  final GeolocationPermissionsHidePrompt? onGeolocationPermissionsHidePrompt;

  /// Notify the host application that the current page has entered full screen
  /// mode.
  ///
  /// After this call, web content will no longer be rendered in the WebView,
  /// but will instead be rendered in `view`.
  final ShowCustomViewCallback? onShowCustomView;

  /// Notify the host application that the current page has exited full screen
  /// mode.
  final HideCustomViewCallback? onHideCustomView;

  /// Report a JavaScript console message to the host application.
  final void Function(WebChromeClient instance, ConsoleMessage message)?
  onConsoleMessage;

  /// Notify the host application that the web page wants to display a
  /// JavaScript alert() dialog.
  final Future<void> Function(String url, String message)? onJsAlert;

  /// Notify the host application that the web page wants to display a
  /// JavaScript confirm() dialog.
  final Future<bool> Function(String url, String message)? onJsConfirm;

  /// Notify the host application that the web page wants to display a
  /// JavaScript prompt() dialog.
  final Future<String?> Function(
    String url,
    String message,
    String defaultValue,
  )?
  onJsPrompt;

  /// Sets the required synchronous return value for the native method,
  /// `WebChromeClient.onShowFileChooser(...)`.
  ///
  /// The native method, `WebChromeClient.onShowFileChooser(...)`, requires
  /// a boolean to be returned and this method sets the returned value for all
  /// calls to the native method.
  ///
  /// Setting this to true indicates that all file chooser requests should be
  /// handled by [onShowFileChooser] and the returned list of Strings will be
  /// returned to the WebView. Otherwise, the client will use the default
  /// handling and the returned value in [onShowFileChooser] will be ignored.
  ///
  /// Requires [onShowFileChooser] to be nonnull.
  ///
  /// Defaults to false.
  Future<void> setSynchronousReturnValueForOnShowFileChooser(bool value) {
    if (value && onShowFileChooser == null) {
      throw StateError(
        'Setting this to true requires `onShowFileChooser` to be nonnull.',
      );
    }
    return api.setSynchronousReturnValueForOnShowFileChooserFromInstance(
      this,
      value,
    );
  }

  /// Sets the required synchronous return value for the native method,
  /// `WebChromeClient.onShowFileChooser(...)`.
  ///
  /// The native method, `WebChromeClient.onConsoleMessage(...)`, requires
  /// a boolean to be returned and this method sets the returned value for all
  /// calls to the native method.
  ///
  /// Setting this to true indicates that the client is handling all console
  /// messages.
  ///
  /// Requires [onConsoleMessage] to be nonnull.
  ///
  /// Defaults to false.
  Future<void> setSynchronousReturnValueForOnConsoleMessage(bool value) {
    if (value && onConsoleMessage == null) {
      throw StateError(
        'Setting this to true requires `onConsoleMessage` to be nonnull.',
      );
    }
    return api.setSynchronousReturnValueForOnConsoleMessageFromInstance(
      this,
      value,
    );
  }

  /// Sets the required synchronous return value for the ets method,
  /// `WebChromeClient.onJsAlert(...)`.
  ///
  /// The ets method, `WebChromeClient.onJsAlert(...)`, requires
  /// a boolean to be returned and this method sets the returned value for all
  /// calls to the ets method.
  ///
  /// Setting this to true indicates that the client is handling all console
  /// messages.
  ///
  /// Requires [onJsAlert] to be nonnull.
  ///
  /// Defaults to false.
  Future<void> setSynchronousReturnValueForOnJsAlert(bool value) {
    if (value && onJsAlert == null) {
      throw StateError(
        'Setting this to true requires `onJsAlert` to be nonnull.',
      );
    }
    return api.setSynchronousReturnValueForOnJsAlertFromInstance(this, value);
  }

  /// Sets the required synchronous return value for the ets method,
  /// `WebChromeClient.onJsConfirm(...)`.
  ///
  /// The ets method, `WebChromeClient.onJsConfirm(...)`, requires
  /// a boolean to be returned and this method sets the returned value for all
  /// calls to the ets method.
  ///
  /// Setting this to true indicates that the client is handling all console
  /// messages.
  ///
  /// Requires [onJsConfirm] to be nonnull.
  ///
  /// Defaults to false.
  Future<void> setSynchronousReturnValueForOnJsConfirm(bool value) {
    if (value && onJsConfirm == null) {
      throw StateError(
        'Setting this to true requires `onJsConfirm` to be nonnull.',
      );
    }
    return api.setSynchronousReturnValueForOnJsConfirmFromInstance(this, value);
  }

  /// Sets the required synchronous return value for the ets method,
  /// `WebChromeClient.onJsPrompt(...)`.
  ///
  /// The ets method, `WebChromeClient.onJsPrompt(...)`, requires
  /// a boolean to be returned and this method sets the returned value for all
  /// calls to the ets method.
  ///
  /// Setting this to true indicates that the client is handling all console
  /// messages.
  ///
  /// Requires [onJsPrompt] to be nonnull.
  ///
  /// Defaults to false.
  Future<void> setSynchronousReturnValueForOnJsPrompt(bool value) {
    if (value && onJsPrompt == null) {
      throw StateError(
        'Setting this to true requires `onJsPrompt` to be nonnull.',
      );
    }
    return api.setSynchronousReturnValueForOnJsPromptFromInstance(this, value);
  }

  @override
  WebChromeClient copy() {
    return WebChromeClient.detached(
      onProgressChanged: onProgressChanged,
      onShowFileChooser: onShowFileChooser,
      onPermissionRequest: onPermissionRequest,
      onGeolocationPermissionsShowPrompt: onGeolocationPermissionsShowPrompt,
      onGeolocationPermissionsHidePrompt: onGeolocationPermissionsHidePrompt,
      onShowCustomView: onShowCustomView,
      onHideCustomView: onHideCustomView,
      onConsoleMessage: onConsoleMessage,
      onJsAlert: onJsAlert,
      onJsConfirm: onJsConfirm,
      onJsPrompt: onJsPrompt,
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}

/// This class defines a permission request and is used when web content
/// requests access to protected resources.
///
/// Only supported on Ohos versions >= 21.
class PermissionRequest extends OhosObject {
  /// Instantiates a [PermissionRequest] without creating and attaching to an
  /// instance of the associated native class.
  ///
  /// This should only be used outside of tests by subclasses created by this
  /// library or to create a copy for an [InstanceManager].
  @protected
  PermissionRequest.detached({
    required this.resources,
    required super.binaryMessenger,
    required super.instanceManager,
  }) : _permissionRequestApi = PermissionRequestHostApiImpl(
         binaryMessenger: binaryMessenger,
         instanceManager: instanceManager,
       ),
       super.detached();

  /// Resource belongs to audio capture device, like microphone.
  static const String audioCapture = 'TYPE_AUDIO_CAPTURE';

  /// Resource will allow sysex messages to be sent to or received from MIDI
  /// devices.
  static const String midiSysex = 'TYPE_MIDI_SYSEX';

  /// Resource belongs to video capture device, like camera.
  static const String videoCapture = 'TYPE_VIDEO_CAPTURE';

  /// Resource belongs to protected media identifier.
  static const String protectedMediaId =
      'ohos.webkit.resource.PROTECTED_MEDIA_ID';

  final PermissionRequestHostApiImpl _permissionRequestApi;

  /// Resources the web page is trying to access.
  final List<String> resources;

  /// Call this method to get the resources the web page is trying to access.
  Future<void> grant(List<String> resources) {
    return _permissionRequestApi.grantFromInstances(this, resources);
  }

  /// Call this method to grant origin the permission to access the given
  /// resources.
  Future<void> deny() {
    return _permissionRequestApi.denyFromInstances(this);
  }

  @override
  PermissionRequest copy() {
    return PermissionRequest.detached(
      resources: resources,
      binaryMessenger: _permissionRequestApi.binaryMessenger,
      instanceManager: _permissionRequestApi.instanceManager,
    );
  }
}

/// Parameters received when a [WebChromeClient] should show a file chooser.
class FileChooserParams extends OhosObject {
  /// Constructs a [FileChooserParams] without creating the associated TS
  /// object.
  ///
  /// This should only be used by subclasses created by this library or to
  /// create copies.
  @protected
  FileChooserParams.detached({
    required this.isCaptureEnabled,
    required this.acceptTypes,
    required this.filenameHint,
    required this.mode,
    super.binaryMessenger,
    super.instanceManager,
  }) : super.detached();

  /// Preference for a live media captured value (e.g. Camera, Microphone).
  final bool isCaptureEnabled;

  /// A list of acceptable MIME types.
  final List<String> acceptTypes;

  /// The file name of a default selection if specified, or null.
  final String? filenameHint;

  /// Mode of how to select files for a file chooser.
  final FileChooserMode mode;

  @override
  FileChooserParams copy() {
    return FileChooserParams.detached(
      isCaptureEnabled: isCaptureEnabled,
      acceptTypes: acceptTypes,
      filenameHint: filenameHint,
      mode: mode,
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}

/// Encompasses parameters to the [WebViewClient.requestLoading] method.
class WebResourceRequest {
  /// Constructs a [WebResourceRequest].
  WebResourceRequest({
    required this.url,
    required this.isForMainFrame,
    required this.isRedirect,
    required this.hasGesture,
    required this.method,
    required this.requestHeaders,
  });

  /// The URL for which the resource request was made.
  final String url;

  /// Whether the request was made in order to fetch the main frame's document.
  final bool isForMainFrame;

  /// Whether the request was a result of a server-side redirect.
  ///
  /// Only supported on Ohos version >= 24.
  final bool? isRedirect;

  /// Whether a gesture (such as a click) was associated with the request.
  final bool hasGesture;

  /// The method associated with the request, for example "GET".
  final String method;

  /// The headers associated with the request.
  final Map<String, String> requestHeaders;
}

/// Encapsulates information about errors occurred during loading of web resources.
///
/// See [WebViewClient.onReceivedRequestError].
class WebResourceError {
  /// Constructs a [WebResourceError].
  WebResourceError({required this.errorCode, required this.description});

  /// The integer code of the error (e.g. [WebViewClient.errorAuthentication].
  final int errorCode;

  /// Describes the error.
  final String description;
}

/// Encapsulates information about HTTP error responses.
///
/// See [WebViewClient.onReceivedHttpError].
class WebResourceResponse {
  /// Constructs a [WebResourceResponse].
  WebResourceResponse({
    required this.statusCode,
    required this.responseHeaders,
    this.reasonPhrase,
    this.mimeType,
  });

  /// The HTTP status code.
  final int statusCode;

  /// The HTTP response headers.
  final Map<String, String> responseHeaders;

  /// The HTTP reason phrase, when provided by ArkWeb.
  final String? reasonPhrase;

  /// The response MIME type, when provided by ArkWeb.
  final String? mimeType;
}

/// Manages Flutter assets that are part of Ohos's app bundle.
class FlutterAssetManager {
  /// Constructs the [FlutterAssetManager].
  const FlutterAssetManager();

  /// Host channel implementation for [FlutterAssetManager].
  @visibleForTesting
  static FlutterAssetManagerHostApi api = FlutterAssetManagerHostApi();

  /// Lists all assets at the given path.
  ///
  /// The assets are returned as a `List<String>`. The `List<String>` only
  /// contains files which are direct childs
  Future<List<String?>> list(String path) => api.list(path);

  /// Gets the relative file path to the Flutter asset with the given name.
  Future<String> getAssetFilePathByName(String name) =>
      api.getAssetFilePathByName(name);
}

/// Manages the JavaScript storage APIs provided by the [WebView].
class WebStorage extends OhosObject {
  /// Constructs a [WebStorage].
  ///
  /// This constructor is only used for testing. An instance should be obtained
  /// with [WebStorage.instance].
  @visibleForTesting
  WebStorage({
    @visibleForTesting super.binaryMessenger,
    @visibleForTesting super.instanceManager,
  }) : super.detached() {
    OhosWebViewFlutterApis.instance.ensureSetUp();
    api.createFromInstance(this);
  }

  /// Constructs a [WebStorage] without creating the associated native object.
  ///
  /// This should only be used by subclasses created by this library or to
  /// create copies.
  @protected
  WebStorage.detached({super.binaryMessenger, super.instanceManager})
    : super.detached();

  /// Host channel implementation for [WebStorage].
  @visibleForTesting
  static WebStorageHostApiImpl api = WebStorageHostApiImpl();

  /// The singleton instance of this class.
  static WebStorage instance = WebStorage();

  /// Clears all storage currently being used by the JavaScript storage APIs.
  Future<void> deleteAllData() {
    return api.deleteAllDataFromInstance(this);
  }

  @override
  WebStorage copy() {
    return WebStorage.detached(
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}

/// The basic building block for native OHOS view components bridged to Flutter.
class View extends OhosObject {
  /// Instantiates a [View] without creating and attaching to an
  /// instance of the associated native class.
  ///
  /// This should only be used outside of tests by subclasses created by this
  /// library or to create a copy for an [InstanceManager].
  @protected
  View.detached({super.binaryMessenger, super.instanceManager})
    : super.detached();

  @override
  View copy() {
    return View.detached(
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}

/// A callback interface used by the host application to notify the current page
/// that its custom view has been dismissed.
///
/// Callback used by the native WebChromeClient custom-view flow.
class CustomViewCallback extends OhosObject {
  /// Instantiates a [CustomViewCallback] without creating and attaching to an
  /// instance of the associated native class.
  ///
  /// This should only be used outside of tests by subclasses created by this
  /// library or to create a copy for an [InstanceManager].
  @protected
  CustomViewCallback.detached({super.binaryMessenger, super.instanceManager})
    : _customViewCallbackApi = CustomViewCallbackHostApiImpl(
        binaryMessenger: binaryMessenger,
        instanceManager: instanceManager,
      ),
      super.detached();

  final CustomViewCallbackHostApiImpl _customViewCallbackApi;

  /// Invoked when the host application dismisses the custom view.
  Future<void> onCustomViewHidden() {
    return _customViewCallbackApi.onCustomViewHiddenFromInstances(this);
  }

  @override
  CustomViewCallback copy() {
    return CustomViewCallback.detached(
      binaryMessenger: _customViewCallbackApi.binaryMessenger,
      instanceManager: _customViewCallbackApi.instanceManager,
    );
  }
}

/// Represents a request for HTTP authentication.
///
/// Instances of this class are created by the [WebView] and passed to
/// [WebViewClient.onReceivedHttpAuthRequest]. The host application must call
/// either [HttpAuthHandler.proceed] or [HttpAuthHandler.cancel] to set the
/// WebView's response to the request.
class HttpAuthHandler extends OhosObject {
  /// Constructs a [HttpAuthHandler].
  HttpAuthHandler({super.binaryMessenger, super.instanceManager})
    : super.detached();

  /// Host channel implementation for [HttpAuthHandler].
  @visibleForTesting
  static HttpAuthHandlerHostApiImpl api = HttpAuthHandlerHostApiImpl();

  /// Instructs the WebView to cancel the authentication request.
  Future<void> cancel() {
    return api.cancelFromInstance(this);
  }

  /// Instructs the WebView to proceed with the authentication with the provided
  /// credentials.
  Future<void> proceed(String username, String password) {
    return api.proceedFromInstance(this, username, password);
  }

  /// Gets whether the credentials stored for the current host are suitable for
  /// use.
  ///
  /// Credentials are not suitable if they have previously been rejected by the
  /// server for the current request.
  Future<bool> useHttpAuthUsernamePassword() {
    return api.useHttpAuthUsernamePasswordFromInstance(this);
  }

  @override
  HttpAuthHandler copy() {
    return HttpAuthHandler(
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}

/// Represents a request to recover from an SSL certificate error.
///
/// Instances of this class are created by the [WebView] and passed to
/// [WebViewClient.onReceivedSslAuthError]. The host application must call
/// either [proceed] or [cancel] to set the WebView's response to the request.
class SslAuthHandler extends OhosObject {
  /// Constructs a [SslAuthHandler].
  SslAuthHandler({super.binaryMessenger, super.instanceManager})
    : super.detached();

  /// Host channel implementation for [SslAuthHandler].
  @visibleForTesting
  static SslAuthHandlerHostApiImpl api = SslAuthHandlerHostApiImpl();

  /// Instructs the WebView to terminate communication with the server.
  Future<void> cancel() {
    return api.cancelFromInstance(this);
  }

  /// Instructs the WebView to ignore the SSL certificate error.
  ///
  /// This should only be used for controlled test environments.
  Future<void> proceed() {
    return api.proceedFromInstance(this);
  }

  @override
  SslAuthHandler copy() {
    return SslAuthHandler(
      binaryMessenger: _api.binaryMessenger,
      instanceManager: _api.instanceManager,
    );
  }
}
