// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:path/path.dart' as path;
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

import 'windows_webview_types.dart' as native_types;
import 'windows_webview_native.dart' as native_webview;

/// Windows-specific policy for popup windows.
enum WindowsPopupWindowPolicy {
  /// Allow popups to open separate windows.
  allow,

  /// Suppress popup windows.
  deny,

  /// Open popup content in the current window.
  sameWindow,
}

/// Pointer button accepted by a Windows browser compositor surface.
enum WindowsBrowserSurfacePointerButton { none, primary, secondary, tertiary }

/// Browser compositor frame returned by the Windows WebView surface.
class WindowsBrowserSurfaceFrame {
  /// Creates one browser compositor frame.
  const WindowsBrowserSurfaceFrame({
    required this.codec,
    required this.data,
    required this.width,
    required this.height,
  });

  /// Creates a public frame model from the native plugin frame payload.
  factory WindowsBrowserSurfaceFrame.fromNative(
    native_webview.WebviewSurfaceFrame frame,
  ) {
    return WindowsBrowserSurfaceFrame(
      codec: frame.codec,
      data: frame.data,
      width: frame.width,
      height: frame.height,
    );
  }

  final String codec;
  final Uint8List data;
  final double width;
  final double height;
}

/// Creation parameters for [WindowsWebViewController].
@immutable
class WindowsWebViewControllerCreationParams
    extends PlatformWebViewControllerCreationParams {
  /// Creates a new [WindowsWebViewControllerCreationParams].
  const WindowsWebViewControllerCreationParams({
    this.popupWindowPolicy = WindowsPopupWindowPolicy.sameWindow,
  });

  /// Creates a [WindowsWebViewControllerCreationParams] from generic params.
  const WindowsWebViewControllerCreationParams.fromPlatformWebViewControllerCreationParams(
    PlatformWebViewControllerCreationParams params, {
    this.popupWindowPolicy = WindowsPopupWindowPolicy.sameWindow,
  });

  /// How popup windows should be handled.
  final WindowsPopupWindowPolicy popupWindowPolicy;
}

/// Windows-specific creation parameters for [WindowsWebViewWidget].
@immutable
class WindowsWebViewWidgetCreationParams
    extends PlatformWebViewWidgetCreationParams {
  /// Creates a new [WindowsWebViewWidgetCreationParams].
  const WindowsWebViewWidgetCreationParams({
    super.key,
    required super.controller,
    super.layoutDirection,
    super.gestureRecognizers,
    this.scaleFactor,
    this.filterQuality = FilterQuality.none,
    this.layoutControlsSurfaceSize = true,
  });

  /// Creates a [WindowsWebViewWidgetCreationParams] from generic params.
  WindowsWebViewWidgetCreationParams.fromPlatformWebViewWidgetCreationParams(
    PlatformWebViewWidgetCreationParams params, {
    double? scaleFactor,
    FilterQuality filterQuality = FilterQuality.none,
    bool layoutControlsSurfaceSize = true,
  }) : this(
         key: params.key,
         controller: params.controller,
         layoutDirection: params.layoutDirection,
         gestureRecognizers: params.gestureRecognizers,
         scaleFactor: scaleFactor,
         filterQuality: filterQuality,
         layoutControlsSurfaceSize: layoutControlsSurfaceSize,
       );

  /// Optional rasterization scale factor.
  final double? scaleFactor;

  /// Filter quality for the underlying texture.
  final FilterQuality filterQuality;

  /// Whether Flutter layout changes resize the native compositor surface.
  final bool layoutControlsSurfaceSize;
}

/// Creation parameters for [WindowsNavigationDelegate].
@immutable
class WindowsNavigationDelegateCreationParams
    extends PlatformNavigationDelegateCreationParams {
  /// Creates a new [WindowsNavigationDelegateCreationParams].
  const WindowsNavigationDelegateCreationParams();

  /// Creates a [WindowsNavigationDelegateCreationParams] from generic params.
  const WindowsNavigationDelegateCreationParams.fromPlatformNavigationDelegateCreationParams(
    PlatformNavigationDelegateCreationParams params,
  );
}

/// A WebView2-backed implementation of [PlatformWebViewController].
class WindowsWebViewController extends PlatformWebViewController {
  /// Creates a [WindowsWebViewController].
  WindowsWebViewController(PlatformWebViewControllerCreationParams params)
    : _webviewController = native_webview.WebviewController(),
      super.implementation(
        params is WindowsWebViewControllerCreationParams
            ? params
            : WindowsWebViewControllerCreationParams.fromPlatformWebViewControllerCreationParams(
                params,
              ),
      ) {
    _initializationFuture = _initialize();
  }

  final native_webview.WebviewController _webviewController;
  final Map<String, JavaScriptChannelParams> _javaScriptChannelParams =
      <String, JavaScriptChannelParams>{};
  final Map<String, String> _javaScriptChannelScriptIds = <String, String>{};
  final Map<String, String> _virtualHostMappings = <String, String>{};
  final List<StreamSubscription<dynamic>> _subscriptions =
      <StreamSubscription<dynamic>>[];

  Future<void>? _initializationFuture;
  WindowsNavigationDelegate? _currentNavigationDelegate;
  String? _currentUrl;
  String? _pageStartedUrl;
  String? _title;
  String? _userAgent;
  bool _canGoBack = false;
  bool _canGoForward = false;
  bool _verticalScrollBarEnabled = true;
  bool _horizontalScrollBarEnabled = true;
  WebViewOverScrollMode _overScrollMode = WebViewOverScrollMode.always;
  bool _scrollBridgeInstalled = false;
  String? _consoleBridgeScriptId;
  String? _scrollBarStyleScriptId;
  String? _overScrollStyleScriptId;

  void Function(JavaScriptConsoleMessage)? _onConsoleMessageCallback;
  Future<void> Function(JavaScriptAlertDialogRequest)?
  _onJavaScriptAlertDialogCallback;
  Future<bool> Function(JavaScriptConfirmDialogRequest)?
  _onJavaScriptConfirmDialogCallback;
  Future<String> Function(JavaScriptTextInputDialogRequest)?
  _onJavaScriptTextInputDialogCallback;
  void Function(ScrollPositionChange)? _onScrollPositionChangeCallback;

  WindowsWebViewControllerCreationParams get _windowsParams =>
      params as WindowsWebViewControllerCreationParams;

  static const String _channelMessageType = '__windows_webview_all_type';
  static const String _javaScriptChannelMessageType = 'javascriptChannel';
  static const String _consoleMessageType = 'consoleMessage';
  static const String _scrollMessageType = 'scrollPositionChange';

  /// Explicitly initializes the shared WebView2 environment.
  static Future<void> initializeEnvironment({
    String? userDataPath,
    String? browserExePath,
    String? additionalArguments,
  }) {
    return native_webview.WebviewController.initializeEnvironment(
      userDataPath: userDataPath,
      browserExePath: browserExePath,
      additionalArguments: additionalArguments,
    );
  }

  /// Returns the installed WebView2 runtime version, if available.
  static Future<String?> getWebViewVersion() {
    return native_webview.WebviewController.getWebViewVersion();
  }

  /// Returns the process-local Flutter texture id for the real WebView surface.
  Future<int> browserSurfaceTextureId() async {
    await _ensureInitialized();
    return _webviewController.surfaceTextureId;
  }

  /// Resizes the real WebView compositor surface.
  Future<void> resizeBrowserSurface(Size size, double scaleFactor) async {
    await _ensureInitialized();
    await _webviewController.setSurfaceSize(size, scaleFactor);
  }

  /// Moves the real WebView compositor cursor.
  Future<void> moveBrowserSurfaceCursor(Offset position) async {
    await _ensureInitialized();
    await _webviewController.setSurfaceCursorPosition(position);
  }

  /// Changes one real WebView compositor pointer button state.
  Future<void> setBrowserSurfacePointerButton(
    WindowsBrowserSurfacePointerButton button, {
    required bool isDown,
  }) async {
    await _ensureInitialized();
    await _webviewController.setSurfacePointerButton(
      native_types.PointerButton.values[button.index],
      isDown: isDown,
    );
  }

  /// Sends a scroll delta to the real WebView compositor surface.
  Future<void> scrollBrowserSurface(double dx, double dy) async {
    await _ensureInitialized();
    await _webviewController.setSurfaceScrollDelta(dx, dy);
  }

  /// Sends a key event to the real WebView compositor surface.
  Future<void> dispatchBrowserSurfaceKeyEvent({
    required String type,
    required String key,
    required String code,
    required int windowsVirtualKeyCode,
    required int nativeVirtualKeyCode,
    required int modifiers,
    String? text,
    String? unmodifiedText,
  }) async {
    await _ensureInitialized();
    await _webviewController.dispatchSurfaceKeyEvent(
      jsonEncode(<String, Object?>{
        'type': type,
        'key': key,
        'code': code,
        'windowsVirtualKeyCode': windowsVirtualKeyCode,
        'nativeVirtualKeyCode': nativeVirtualKeyCode,
        'modifiers': modifiers,
        'text': text,
        'unmodifiedText': unmodifiedText,
      }),
    );
  }

  /// Captures one frame from the real WebView compositor surface.
  Future<WindowsBrowserSurfaceFrame> captureBrowserSurfaceFrame() async {
    await _ensureInitialized();
    return WindowsBrowserSurfaceFrame.fromNative(
      await _webviewController.captureSurfaceFrame(),
    );
  }

  Future<void> _initialize() async {
    await _webviewController.initialize();
    _webviewController.setJavaScriptDialogRequestedDelegate(
      _handleJavaScriptDialogRequested,
    );
    _webviewController.setHttpAuthRequestedDelegate(_handleHttpAuthRequested);
    _webviewController.setSslAuthErrorRequestedDelegate(_handleSslAuthError);
    await _webviewController.setPopupWindowPolicy(
      switch (_windowsParams.popupWindowPolicy) {
        WindowsPopupWindowPolicy.allow =>
          native_types.WebviewPopupWindowPolicy.allow,
        WindowsPopupWindowPolicy.deny =>
          native_types.WebviewPopupWindowPolicy.deny,
        WindowsPopupWindowPolicy.sameWindow =>
          native_types.WebviewPopupWindowPolicy.sameWindow,
      },
    );

    _subscriptions.addAll(<StreamSubscription<dynamic>>[
      _webviewController.url.listen(_handleUrlChanged),
      _webviewController.title.listen((String title) {
        _title = title;
      }),
      _webviewController.historyChanged.listen((
        native_webview.HistoryChanged value,
      ) {
        _canGoBack = value.canGoBack;
        _canGoForward = value.canGoForward;
      }),
      _webviewController.loadingState.listen(_handleLoadingStateChanged),
      _webviewController.onLoadError.listen(_handleLoadError),
      _webviewController.httpResponseError.listen(_handleHttpResponseError),
      _webviewController.webMessage.listen(_handleWebMessage),
    ]);
  }

  Future<void> _ensureInitialized() async {
    await _initializationFuture!;
  }

  void _handleUrlChanged(String url) {
    _currentUrl = url;
    _pageStartedUrl ??= url;
    _currentNavigationDelegate?._onUrlChange?.call(UrlChange(url: url));
  }

  void _handleLoadingStateChanged(native_types.LoadingState state) {
    switch (state) {
      case native_types.LoadingState.none:
        break;
      case native_types.LoadingState.loading:
        final url = _currentUrl ?? _pageStartedUrl ?? '';
        _pageStartedUrl = url;
        _currentNavigationDelegate?._onProgress?.call(0);
        _currentNavigationDelegate?._onPageStarted?.call(url);
        break;
      case native_types.LoadingState.navigationCompleted:
        final url = _currentUrl ?? _pageStartedUrl ?? '';
        _currentNavigationDelegate?._onProgress?.call(100);
        _currentNavigationDelegate?._onPageFinished?.call(url);
        _pageStartedUrl = null;
        break;
    }
  }

  void _handleLoadError(native_types.WebErrorStatus status) {
    _currentNavigationDelegate?._onWebResourceError?.call(
      WindowsWebResourceError(status, url: _currentUrl, isForMainFrame: true),
    );
  }

  void _handleHttpResponseError(native_webview.WebviewHttpResponseError error) {
    final Uri? uri = Uri.tryParse(error.url);
    _currentNavigationDelegate?._onHttpError?.call(
      HttpResponseError(
        request: uri == null
            ? null
            : WindowsWebResourceRequest._(
                uri: uri,
                method: error.method,
                headers: error.requestHeaders,
              ),
        response: WindowsWebResourceResponse._(
          uri: uri,
          statusCode: error.statusCode,
          headers: error.responseHeaders,
          reasonPhrase: error.reasonPhrase,
          mimeType: _mimeTypeFromResponseHeaders(error.responseHeaders),
        ),
      ),
    );
  }

  void _handleWebMessage(dynamic message) {
    if (message is! Map) {
      return;
    }

    final type = message[_channelMessageType] as String?;
    switch (type) {
      case _javaScriptChannelMessageType:
        final channelName = message['channelName'] as String?;
        final params = channelName == null
            ? null
            : _javaScriptChannelParams[channelName];
        if (params != null) {
          params.onMessageReceived(
            JavaScriptMessage(message: '${message['message'] ?? ''}'),
          );
        }
        break;
      case _consoleMessageType:
        final callback = _onConsoleMessageCallback;
        if (callback != null) {
          callback(
            JavaScriptConsoleMessage(
              level: _parseJavaScriptLogLevel(message['level'] as String?),
              message: '${message['message'] ?? ''}',
            ),
          );
        }
        break;
      case _scrollMessageType:
        final callback = _onScrollPositionChangeCallback;
        if (callback != null) {
          callback(
            ScrollPositionChange(
              (message['x'] as num?)?.toDouble() ?? 0.0,
              (message['y'] as num?)?.toDouble() ?? 0.0,
            ),
          );
        }
        break;
      default:
        break;
    }
  }

  Future<Map<String, Object?>?> _handleJavaScriptDialogRequested(
    String dialogType,
    String url,
    String message,
    String? defaultText,
  ) async {
    switch (dialogType) {
      case 'alert':
        final callback = _onJavaScriptAlertDialogCallback;
        if (callback == null) {
          return null;
        }
        await callback(
          JavaScriptAlertDialogRequest(message: message, url: url),
        );
        return <String, Object?>{'action': 'accept'};
      case 'confirm':
        final callback = _onJavaScriptConfirmDialogCallback;
        if (callback == null) {
          return null;
        }
        final bool confirmed = await callback(
          JavaScriptConfirmDialogRequest(message: message, url: url),
        );
        return <String, Object?>{'action': confirmed ? 'confirm' : 'cancel'};
      case 'prompt':
        final callback = _onJavaScriptTextInputDialogCallback;
        if (callback == null) {
          return null;
        }
        final String text = await callback(
          JavaScriptTextInputDialogRequest(
            message: message,
            url: url,
            defaultText: defaultText,
          ),
        );
        return <String, Object?>{'action': 'confirm', 'text': text};
    }

    return null;
  }

  Future<Map<String, Object?>?> _handleHttpAuthRequested(
    String url,
    String challenge,
  ) async {
    final callback = _currentNavigationDelegate?._onHttpAuthRequest;
    if (callback == null) {
      return <String, Object?>{'action': 'cancel'};
    }

    final completer = Completer<Map<String, Object?>>();
    final Uri? uri = Uri.tryParse(url);
    try {
      callback(
        HttpAuthRequest(
          host: uri?.host ?? url,
          realm: _parseHttpAuthRealm(challenge),
          onProceed: (WebViewCredential credential) {
            if (completer.isCompleted) {
              return;
            }
            completer.complete(<String, Object?>{
              'action': 'proceed',
              'user': credential.user,
              'password': credential.password,
            });
          },
          onCancel: () {
            if (!completer.isCompleted) {
              completer.complete(<String, Object?>{'action': 'cancel'});
            }
          },
        ),
      );
    } catch (_) {
      return <String, Object?>{'action': 'cancel'};
    }
    return completer.future;
  }

  String? _parseHttpAuthRealm(String challenge) {
    final match = RegExp(
      r'realm=(?:"([^"]*)"|([^,\s]+))',
      caseSensitive: false,
    ).firstMatch(challenge);
    return match?.group(1) ?? match?.group(2);
  }

  String? _mimeTypeFromResponseHeaders(Map<String, String> headers) {
    for (final MapEntry<String, String> header in headers.entries) {
      if (header.key.toLowerCase() != 'content-type') {
        continue;
      }
      final String mimeType = header.value
          .split(';')
          .first
          .trim()
          .toLowerCase();
      return mimeType.isEmpty ? null : mimeType;
    }
    return null;
  }

  Future<Map<String, Object?>?> _handleSslAuthError(
    String url,
    int errorStatus,
  ) async {
    final callback = _currentNavigationDelegate?._onSslAuthError;
    if (callback == null) {
      return <String, Object?>{'action': 'cancel'};
    }

    final completer = Completer<Map<String, Object?>>();
    final status = _webErrorStatusFromIndex(errorStatus);
    try {
      callback(
        WindowsPlatformSslAuthError(
          description: _sslAuthErrorDescription(url, status),
          onProceed: () async {
            if (!completer.isCompleted) {
              completer.complete(<String, Object?>{'action': 'proceed'});
            }
          },
          onCancel: () async {
            if (!completer.isCompleted) {
              completer.complete(<String, Object?>{'action': 'cancel'});
            }
          },
        ),
      );
    } catch (_) {
      return <String, Object?>{'action': 'cancel'};
    }
    return completer.future;
  }

  native_types.WebErrorStatus _webErrorStatusFromIndex(int index) {
    if (index < 0 || index >= native_types.WebErrorStatus.values.length) {
      return native_types.WebErrorStatus.WebErrorStatusUnknown;
    }
    return native_types.WebErrorStatus.values[index];
  }

  String _sslAuthErrorDescription(
    String url,
    native_types.WebErrorStatus status,
  ) {
    return 'SSL certificate error for $url: ${status.name}.';
  }

  Future<void> _updateJavaScriptDialogCallbacksEnabled() {
    return _webviewController.setJavaScriptDialogCallbacksEnabled(
      alert: _onJavaScriptAlertDialogCallback != null,
      confirm: _onJavaScriptConfirmDialogCallback != null,
      prompt: _onJavaScriptTextInputDialogCallback != null,
    );
  }

  JavaScriptLogLevel _parseJavaScriptLogLevel(String? level) {
    switch (level) {
      case 'debug':
        return JavaScriptLogLevel.debug;
      case 'error':
        return JavaScriptLogLevel.error;
      case 'info':
        return JavaScriptLogLevel.info;
      case 'warning':
        return JavaScriptLogLevel.warning;
      default:
        return JavaScriptLogLevel.log;
    }
  }

  @override
  Future<void> loadFile(String absoluteFilePath) async {
    await _ensureInitialized();
    final file = File(absoluteFilePath);
    if (!file.existsSync()) {
      throw ArgumentError.value(
        absoluteFilePath,
        'absoluteFilePath',
        'File does not exist.',
      );
    }

    final folderPath = path.dirname(file.absolute.path);
    final fileName = path.basename(file.path);
    const host = 'app-file.webview.flutter.dev';
    final url = Uri.https(host, '/$fileName').toString();
    if (!await _shouldNavigate(url)) {
      return;
    }
    await _setVirtualHostMapping(host, folderPath);
    await _webviewController.loadUrl(url);
  }

  @override
  Future<void> loadFileWithParams(LoadFileParams params) {
    return loadFile(params.absoluteFilePath);
  }

  @override
  Future<void> loadFlutterAsset(String key) async {
    await _ensureInitialized();
    final assetPath = _resolveFlutterAssetPath(key);
    final file = File(assetPath);
    if (!file.existsSync()) {
      throw ArgumentError.value(key, 'key', 'Asset for key "$key" not found.');
    }

    final folderPath = path.dirname(assetPath);
    final fileName = path.basename(assetPath);
    const host = 'flutter-assets.webview.flutter.dev';
    final url = Uri.https(host, '/$fileName').toString();
    if (!await _shouldNavigate(url)) {
      return;
    }
    await _setVirtualHostMapping(host, folderPath);
    await _webviewController.loadUrl(url);
  }

  @override
  Future<void> loadHtmlString(String html, {String? baseUrl}) async {
    await _ensureInitialized();
    final content = baseUrl == null ? html : _injectBaseUrl(html, baseUrl);
    await _webviewController.loadStringContent(content);
  }

  @override
  Future<void> loadRequest(LoadRequestParams params) async {
    await _ensureInitialized();
    if (!params.uri.hasScheme) {
      throw ArgumentError(
        'LoadRequestParams#uri is required to have a scheme.',
      );
    }

    if (!await _shouldNavigate(params.uri.toString())) {
      return;
    }

    await _webviewController.loadRequest(
      url: params.uri.toString(),
      method: params.method.serialize(),
      headers: _serializeRequestHeaders(params.headers),
      body: params.body,
    );
  }

  @override
  Future<String?> currentUrl() async {
    await _ensureInitialized();
    return _currentUrl;
  }

  @override
  Future<bool> canGoBack() async {
    await _ensureInitialized();
    return _canGoBack;
  }

  @override
  Future<bool> canGoForward() async {
    await _ensureInitialized();
    return _canGoForward;
  }

  @override
  Future<void> goBack() async {
    await _ensureInitialized();
    await _webviewController.goBack();
  }

  @override
  Future<void> goForward() async {
    await _ensureInitialized();
    await _webviewController.goForward();
  }

  @override
  Future<void> reload() async {
    await _ensureInitialized();
    await _webviewController.reload();
  }

  @override
  Future<void> clearCache() async {
    await _ensureInitialized();
    await _webviewController.clearCache();
  }

  @override
  Future<void> clearLocalStorage() async {
    await _ensureInitialized();
    await _webviewController.clearLocalStorage();
  }

  @override
  Future<void> setPlatformNavigationDelegate(
    PlatformNavigationDelegate handler,
  ) async {
    await _ensureInitialized();
    _currentNavigationDelegate = handler as WindowsNavigationDelegate;
  }

  @override
  Future<void> runJavaScript(String javaScript) async {
    await _ensureInitialized();
    await _webviewController.executeScript(javaScript);
  }

  @override
  Future<Object> runJavaScriptReturningResult(String javaScript) async {
    await _ensureInitialized();
    final dynamic result = await _webviewController.executeScript(javaScript);
    if (result == null) {
      throw ArgumentError(
        'The JavaScript returned `null` or `undefined`, which is unsupported.',
      );
    }
    return result as Object;
  }

  @override
  Future<void> addJavaScriptChannel(
    JavaScriptChannelParams javaScriptChannelParams,
  ) async {
    await _ensureInitialized();
    final name = javaScriptChannelParams.name;
    if (_javaScriptChannelParams.containsKey(name)) {
      throw ArgumentError(
        'A JavaScriptChannel with name `$name` already exists.',
      );
    }

    _javaScriptChannelParams[name] = javaScriptChannelParams;
    final script =
        '''
      window.$name = {
        postMessage: function(message) {
          window.chrome.webview.postMessage({
            "$_channelMessageType": "$_javaScriptChannelMessageType",
            "channelName": "$name",
            "message": String(message)
          });
        }
      };
    ''';
    final scriptId = await _webviewController
        .addScriptToExecuteOnDocumentCreated(script);
    if (scriptId != null) {
      _javaScriptChannelScriptIds[name] = scriptId;
    }
  }

  @override
  Future<void> removeJavaScriptChannel(String javaScriptChannelName) async {
    await _ensureInitialized();
    _javaScriptChannelParams.remove(javaScriptChannelName);
    final scriptId = _javaScriptChannelScriptIds.remove(javaScriptChannelName);
    if (scriptId != null) {
      await _webviewController.removeScriptToExecuteOnDocumentCreated(scriptId);
    }
  }

  @override
  Future<String?> getTitle() async {
    await _ensureInitialized();
    return _title;
  }

  @override
  Future<void> scrollTo(int x, int y) async {
    await runJavaScript('window.scrollTo($x, $y);');
  }

  @override
  Future<void> scrollBy(int x, int y) async {
    await runJavaScript('window.scrollBy($x, $y);');
  }

  @override
  Future<void> setVerticalScrollBarEnabled(bool enabled) async {
    await _ensureInitialized();
    if (_verticalScrollBarEnabled == enabled) {
      return;
    }

    _verticalScrollBarEnabled = enabled;
    await _updateScrollBarStyle();
  }

  @override
  Future<void> setHorizontalScrollBarEnabled(bool enabled) async {
    await _ensureInitialized();
    if (_horizontalScrollBarEnabled == enabled) {
      return;
    }

    _horizontalScrollBarEnabled = enabled;
    await _updateScrollBarStyle();
  }

  @override
  bool supportsSetScrollBarsEnabled() {
    return true;
  }

  @override
  Future<Offset> getScrollPosition() async {
    final value = await runJavaScriptReturningResult(
      '({x: window.scrollX || window.pageXOffset || 0, '
      'y: window.scrollY || window.pageYOffset || 0})',
    );
    final position = value as Map<Object?, Object?>;
    return Offset(
      (position['x'] as num?)?.toDouble() ?? 0,
      (position['y'] as num?)?.toDouble() ?? 0,
    );
  }

  @override
  Future<void> enableZoom(bool enabled) async {
    await _ensureInitialized();
    await _webviewController.setZoomControlEnabled(enabled);
  }

  @override
  Future<void> setBackgroundColor(Color color) async {
    await _ensureInitialized();
    await _webviewController.setBackgroundColor(color);
  }

  @override
  Future<void> setJavaScriptMode(JavaScriptMode javaScriptMode) async {
    await _ensureInitialized();
    await _webviewController.setJavaScriptEnabled(
      javaScriptMode == JavaScriptMode.unrestricted,
    );
  }

  @override
  Future<void> setUserAgent(String? userAgent) async {
    await _ensureInitialized();
    _userAgent = userAgent;
    await _webviewController.setUserAgent(userAgent);
  }

  @override
  Future<void> setOnPlatformPermissionRequest(
    void Function(PlatformWebViewPermissionRequest request) onPermissionRequest,
  ) async {
    await _ensureInitialized();
    _webviewController.setPermissionRequestedDelegate((
      String url,
      native_types.WebviewPermissionKind permissionKind,
      bool isUserInitiated,
    ) async {
      final Set<WebViewPermissionResourceType> types = _toPermissionTypes(
        permissionKind,
      );
      if (types.isEmpty) {
        return native_types.WebviewPermissionDecision.none;
      }
      final request = _WindowsWebViewPermissionRequest(types: types);
      onPermissionRequest(request);
      return request.decision.future;
    });
  }

  @override
  Future<String?> getUserAgent() async {
    await _ensureInitialized();
    return _userAgent ?? await _webviewController.getUserAgent();
  }

  @override
  Future<void> setOnConsoleMessage(
    void Function(JavaScriptConsoleMessage consoleMessage) onConsoleMessage,
  ) async {
    await _ensureInitialized();
    _onConsoleMessageCallback = onConsoleMessage;
    if (_consoleBridgeScriptId != null) {
      return;
    }

    final script =
        '''
      (function() {
        if (window.__flutterWindowsConsoleHookInstalled) {
          return;
        }
        window.__flutterWindowsConsoleHookInstalled = true;
        function stringifyArg(arg) {
          if (typeof arg === 'string') {
            return arg;
          }
          try {
            const json = JSON.stringify(arg);
            return json === undefined ? String(arg) : json;
          } catch (_) {
            return String(arg);
          }
        }
        function emit(level, args) {
          window.chrome.webview.postMessage({
            "$_channelMessageType": "$_consoleMessageType",
            "level": level,
            "message": Array.from(args).map(stringifyArg).join(' ')
          });
        }
        ['log', 'info', 'warn', 'error', 'debug'].forEach(function(level) {
          const original = console[level];
          console[level] = function() {
            emit(level === 'warn' ? 'warning' : level, arguments);
            if (original) {
              original.apply(console, arguments);
            }
          };
        });
      })();
    ''';
    _consoleBridgeScriptId = await _webviewController
        .addScriptToExecuteOnDocumentCreated(script);
  }

  @override
  Future<void> setOnScrollPositionChange(
    void Function(ScrollPositionChange scrollPositionChange)?
    onScrollPositionChange,
  ) async {
    await _ensureInitialized();
    _onScrollPositionChangeCallback = onScrollPositionChange;
    if (_scrollBridgeInstalled || onScrollPositionChange == null) {
      return;
    }

    _scrollBridgeInstalled = true;
    await _webviewController.addScriptToExecuteOnDocumentCreated('''
      (function() {
        if (window.__flutterWindowsScrollHookInstalled) {
          return;
        }
        window.__flutterWindowsScrollHookInstalled = true;
        window.addEventListener('scroll', function() {
          window.chrome.webview.postMessage({
            "$_channelMessageType": "$_scrollMessageType",
            "x": window.scrollX || window.pageXOffset || 0,
            "y": window.scrollY || window.pageYOffset || 0
          });
        }, { passive: true });
      })();
      ''');
  }

  @override
  Future<void> setOnJavaScriptAlertDialog(
    Future<void> Function(JavaScriptAlertDialogRequest request)
    onJavaScriptAlertDialog,
  ) async {
    await _ensureInitialized();
    _onJavaScriptAlertDialogCallback = onJavaScriptAlertDialog;
    await _updateJavaScriptDialogCallbacksEnabled();
  }

  @override
  Future<void> setOnJavaScriptConfirmDialog(
    Future<bool> Function(JavaScriptConfirmDialogRequest request)
    onJavaScriptConfirmDialog,
  ) async {
    await _ensureInitialized();
    _onJavaScriptConfirmDialogCallback = onJavaScriptConfirmDialog;
    await _updateJavaScriptDialogCallbacksEnabled();
  }

  @override
  Future<void> setOnJavaScriptTextInputDialog(
    Future<String> Function(JavaScriptTextInputDialogRequest request)
    onJavaScriptTextInputDialog,
  ) async {
    await _ensureInitialized();
    _onJavaScriptTextInputDialogCallback = onJavaScriptTextInputDialog;
    await _updateJavaScriptDialogCallbacksEnabled();
  }

  @override
  Future<void> setOverScrollMode(WebViewOverScrollMode mode) async {
    await _ensureInitialized();
    if (_overScrollMode == mode) {
      return;
    }

    _overScrollMode = mode;
    await _updateOverScrollStyle();
  }

  /// Opens the browser devtools for this WebView.
  Future<void> openDevTools() async {
    await _ensureInitialized();
    await _webviewController.openDevTools();
  }

  /// Suspends the WebView.
  Future<void> suspend() async {
    await _ensureInitialized();
    await _webviewController.suspend();
  }

  /// Resumes a suspended WebView.
  Future<void> resume() async {
    await _ensureInitialized();
    await _webviewController.resume();
  }

  /// Sets the popup policy for this WebView.
  Future<void> setPopupWindowPolicy(WindowsPopupWindowPolicy policy) async {
    await _ensureInitialized();
    await _webviewController.setPopupWindowPolicy(switch (policy) {
      WindowsPopupWindowPolicy.allow =>
        native_types.WebviewPopupWindowPolicy.allow,
      WindowsPopupWindowPolicy.deny =>
        native_types.WebviewPopupWindowPolicy.deny,
      WindowsPopupWindowPolicy.sameWindow =>
        native_types.WebviewPopupWindowPolicy.sameWindow,
    });
  }

  /// Sets the WebView2 zoom factor.
  Future<void> setZoomFactor(double zoomFactor) async {
    await _ensureInitialized();
    await _webviewController.setZoomFactor(zoomFactor);
  }

  /// Toggles whether the network cache is ignored for requests.
  Future<void> setCacheDisabled(bool disabled) async {
    await _ensureInitialized();
    await _webviewController.setCacheDisabled(disabled);
  }

  Future<void> _setVirtualHostMapping(String host, String folderPath) async {
    final normalizedPath = path.normalize(folderPath);
    if (_virtualHostMappings[host] == normalizedPath) {
      return;
    }

    _virtualHostMappings[host] = normalizedPath;
    await _webviewController.addVirtualHostNameMapping(
      host,
      normalizedPath,
      native_types.WebviewHostResourceAccessKind.allow,
    );
  }

  Future<bool> _shouldNavigate(String url) async {
    final callback = _currentNavigationDelegate?._onNavigationRequest;
    if (callback == null) {
      return true;
    }

    final decision = await callback(
      NavigationRequest(url: url, isMainFrame: true),
    );
    return decision == NavigationDecision.navigate;
  }

  String _serializeRequestHeaders(Map<String, String> headers) {
    final buffer = StringBuffer();
    for (final MapEntry<String, String> header in headers.entries) {
      if (header.key.isEmpty ||
          header.key.contains('\r') ||
          header.key.contains('\n') ||
          header.value.contains('\r') ||
          header.value.contains('\n')) {
        throw ArgumentError.value(
          headers,
          'headers',
          'HTTP request headers must not be empty or contain CR/LF.',
        );
      }
      buffer.write(header.key);
      buffer.write(': ');
      buffer.write(header.value);
      buffer.write('\r\n');
    }
    return buffer.toString();
  }

  Future<void> _updateScrollBarStyle() async {
    if (_scrollBarStyleScriptId != null) {
      await _webviewController.removeScriptToExecuteOnDocumentCreated(
        _scrollBarStyleScriptId!,
      );
      _scrollBarStyleScriptId = null;
    }

    final script = _scrollBarStyleScript();
    if (!_verticalScrollBarEnabled || !_horizontalScrollBarEnabled) {
      _scrollBarStyleScriptId = await _webviewController
          .addScriptToExecuteOnDocumentCreated(script);
    }
    await _webviewController.executeScript(script);
  }

  String _scrollBarStyleScript() {
    final css = StringBuffer();
    if (!_verticalScrollBarEnabled) {
      css.writeln('*::-webkit-scrollbar:vertical { width: 0 !important; }');
    }
    if (!_horizontalScrollBarEnabled) {
      css.writeln('*::-webkit-scrollbar:horizontal { height: 0 !important; }');
    }

    return '''
      (function() {
        const styleId = '__flutter_webview_all_scrollbars';
        const css = ${jsonEncode(css.toString())};
        let style = document.getElementById(styleId);
        if (!css) {
          if (style) {
            style.remove();
          }
          return;
        }
        if (!style) {
          style = document.createElement('style');
          style.id = styleId;
          (document.head || document.documentElement).appendChild(style);
        }
        style.textContent = css;
      })();
      ''';
  }

  Future<void> _updateOverScrollStyle() async {
    if (_overScrollStyleScriptId != null) {
      await _webviewController.removeScriptToExecuteOnDocumentCreated(
        _overScrollStyleScriptId!,
      );
      _overScrollStyleScriptId = null;
    }

    final script = _overScrollStyleScript();
    if (_overScrollMode != WebViewOverScrollMode.always) {
      _overScrollStyleScriptId = await _webviewController
          .addScriptToExecuteOnDocumentCreated(script);
    }
    await _webviewController.executeScript(script);
  }

  String _overScrollStyleScript() {
    final String value = switch (_overScrollMode) {
      WebViewOverScrollMode.always => '',
      WebViewOverScrollMode.ifContentScrolls => 'contain',
      WebViewOverScrollMode.never => 'none',
    };

    return '''
      (function() {
        const styleId = '__flutter_webview_all_overscroll';
        const value = ${jsonEncode(value)};
        let style = document.getElementById(styleId);
        if (!value) {
          if (style) {
            style.remove();
          }
          return;
        }
        if (!style) {
          style = document.createElement('style');
          style.id = styleId;
          (document.head || document.documentElement).appendChild(style);
        }
        style.textContent =
            'html, body { overscroll-behavior: ' + value + ' !important; }';
      })();
      ''';
  }

  String _injectBaseUrl(String html, String baseUrl) {
    final baseTag = '<base href="$baseUrl">';
    final headExp = RegExp(r'<head[^>]*>', caseSensitive: false);
    final match = headExp.firstMatch(html);
    if (match != null) {
      return html.replaceRange(match.end, match.end, baseTag);
    }
    return '<head>$baseTag</head>$html';
  }

  String _resolveFlutterAssetPath(String key) {
    return path.joinAll(<String>[
      path.dirname(Platform.resolvedExecutable),
      'data',
      'flutter_assets',
      ...key.split('/'),
    ]);
  }

  Set<WebViewPermissionResourceType> _toPermissionTypes(
    native_types.WebviewPermissionKind permissionKind,
  ) {
    switch (permissionKind) {
      case native_types.WebviewPermissionKind.camera:
        return <WebViewPermissionResourceType>{
          WebViewPermissionResourceType.camera,
        };
      case native_types.WebviewPermissionKind.microphone:
        return <WebViewPermissionResourceType>{
          WebViewPermissionResourceType.microphone,
        };
      default:
        return const <WebViewPermissionResourceType>{};
    }
  }
}

/// WebView widget implementation for Windows.
class WindowsWebViewWidget extends PlatformWebViewWidget {
  /// Creates a [WindowsWebViewWidget].
  WindowsWebViewWidget(PlatformWebViewWidgetCreationParams params)
    : super.implementation(
        params is WindowsWebViewWidgetCreationParams
            ? params
            : WindowsWebViewWidgetCreationParams.fromPlatformWebViewWidgetCreationParams(
                params,
              ),
      );

  WindowsWebViewWidgetCreationParams get _windowsParams =>
      params as WindowsWebViewWidgetCreationParams;

  @override
  Widget build(BuildContext context) {
    final controller = params.controller as WindowsWebViewController;
    return FutureBuilder<void>(
      future: controller._ensureInitialized(),
      builder: (BuildContext context, AsyncSnapshot<void> snapshot) {
        if (snapshot.hasError) {
          return const SizedBox.shrink();
        }

        if (snapshot.connectionState != ConnectionState.done) {
          return const SizedBox.shrink();
        }

        return KeyedSubtree(
          key: params.key,
          child: native_webview.Webview(
            controller._webviewController,
            scaleFactor: _windowsParams.scaleFactor,
            filterQuality: _windowsParams.filterQuality,
            layoutControlsSurfaceSize: _windowsParams.layoutControlsSurfaceSize,
          ),
        );
      },
    );
  }
}

/// Windows implementation of [PlatformNavigationDelegate].
class WindowsNavigationDelegate extends PlatformNavigationDelegate {
  /// Creates a [WindowsNavigationDelegate].
  WindowsNavigationDelegate(PlatformNavigationDelegateCreationParams params)
    : super.implementation(
        params is WindowsNavigationDelegateCreationParams
            ? params
            : WindowsNavigationDelegateCreationParams.fromPlatformNavigationDelegateCreationParams(
                params,
              ),
      );

  PageEventCallback? _onPageFinished;
  PageEventCallback? _onPageStarted;
  ProgressCallback? _onProgress;
  WebResourceErrorCallback? _onWebResourceError;
  HttpResponseErrorCallback? _onHttpError;
  HttpAuthRequestCallback? _onHttpAuthRequest;
  SslAuthErrorCallback? _onSslAuthError;
  NavigationRequestCallback? _onNavigationRequest;
  UrlChangeCallback? _onUrlChange;

  @override
  Future<void> setOnNavigationRequest(
    NavigationRequestCallback onNavigationRequest,
  ) async {
    _onNavigationRequest = onNavigationRequest;
  }

  @override
  Future<void> setOnPageStarted(PageEventCallback onPageStarted) async {
    _onPageStarted = onPageStarted;
  }

  @override
  Future<void> setOnPageFinished(PageEventCallback onPageFinished) async {
    _onPageFinished = onPageFinished;
  }

  @override
  Future<void> setOnHttpError(HttpResponseErrorCallback onHttpError) async {
    _onHttpError = onHttpError;
  }

  @override
  Future<void> setOnProgress(ProgressCallback onProgress) async {
    _onProgress = onProgress;
  }

  @override
  Future<void> setOnWebResourceError(
    WebResourceErrorCallback onWebResourceError,
  ) async {
    _onWebResourceError = onWebResourceError;
  }

  @override
  Future<void> setOnUrlChange(UrlChangeCallback onUrlChange) async {
    _onUrlChange = onUrlChange;
  }

  @override
  Future<void> setOnHttpAuthRequest(
    HttpAuthRequestCallback onHttpAuthRequest,
  ) async {
    _onHttpAuthRequest = onHttpAuthRequest;
  }

  @override
  Future<void> setOnSSlAuthError(SslAuthErrorCallback onSslAuthError) async {
    _onSslAuthError = onSslAuthError;
  }
}

/// Windows implementation of [PlatformSslAuthError].
class WindowsPlatformSslAuthError extends PlatformSslAuthError {
  /// Creates a [WindowsPlatformSslAuthError].
  WindowsPlatformSslAuthError({
    required String description,
    required Future<void> Function() onProceed,
    required Future<void> Function() onCancel,
  }) : _onProceed = onProceed,
       _onCancel = onCancel,
       super(certificate: null, description: description);

  final Future<void> Function() _onProceed;
  final Future<void> Function() _onCancel;

  @override
  Future<void> proceed() {
    return _onProceed();
  }

  @override
  Future<void> cancel() {
    return _onCancel();
  }
}

/// Windows implementation of [WebResourceRequest].
class WindowsWebResourceRequest extends WebResourceRequest {
  /// Creates a [WindowsWebResourceRequest].
  const WindowsWebResourceRequest._({
    required super.uri,
    this.method,
    this.headers = const <String, String>{},
  });

  /// The HTTP method reported by WebView2, when available.
  final String? method;

  /// The HTTP request headers reported by WebView2.
  final Map<String, String> headers;
}

/// Windows implementation of [WebResourceResponse].
class WindowsWebResourceResponse extends WebResourceResponse {
  /// Creates a [WindowsWebResourceResponse].
  const WindowsWebResourceResponse._({
    required super.uri,
    required super.statusCode,
    required super.headers,
    this.reasonPhrase,
    this.mimeType,
  });

  /// The HTTP reason phrase reported by WebView2, when available.
  final String? reasonPhrase;

  /// The response MIME type parsed from WebView2 headers, when available.
  final String? mimeType;
}

/// Windows error mapping for WebView2 load failures.
class WindowsWebResourceError extends WebResourceError {
  /// Creates a [WindowsWebResourceError].
  WindowsWebResourceError(
    native_types.WebErrorStatus status, {
    super.url,
    super.isForMainFrame,
  }) : super(
         errorCode: status.index,
         description: status.name,
         errorType: _toErrorType(status),
       );

  static WebResourceErrorType? _toErrorType(
    native_types.WebErrorStatus status,
  ) {
    switch (status) {
      case native_types.WebErrorStatus.WebErrorStatusTimeout:
        return WebResourceErrorType.timeout;
      case native_types.WebErrorStatus.WebErrorStatusCannotConnect:
      case native_types.WebErrorStatus.WebErrorStatusConnectionAborted:
      case native_types.WebErrorStatus.WebErrorStatusConnectionReset:
      case native_types.WebErrorStatus.WebErrorStatusServerUnreachable:
        return WebResourceErrorType.connect;
      case native_types.WebErrorStatus.WebErrorStatusDisconnected:
        return WebResourceErrorType.io;
      case native_types.WebErrorStatus.WebErrorStatusHostNameNotResolved:
        return WebResourceErrorType.hostLookup;
      case native_types.WebErrorStatus.WebErrorStatusOperationCanceled:
        return WebResourceErrorType.unknown;
      case native_types.WebErrorStatus.WebErrorStatusRedirectFailed:
        return WebResourceErrorType.redirectLoop;
      case native_types
          .WebErrorStatus
          .WebErrorStatusValidAuthenticationCredentialsRequired:
        return WebResourceErrorType.authentication;
      case native_types
          .WebErrorStatus
          .WebErrorStatusValidProxyAuthenticationRequired:
        return WebResourceErrorType.proxyAuthentication;
      case native_types
          .WebErrorStatus
          .WebErrorStatusCertificateCommonNameIsIncorrect:
      case native_types.WebErrorStatus.WebErrorStatusCertificateExpired:
      case native_types
          .WebErrorStatus
          .WebErrorStatusClientCertificateContainsErrors:
      case native_types.WebErrorStatus.WebErrorStatusCertificateRevoked:
      case native_types.WebErrorStatus.WebErrorStatusCertificateIsInvalid:
        return WebResourceErrorType.failedSslHandshake;
      case native_types
          .WebErrorStatus
          .WebErrorStatusErrorHTTPInvalidServerResponse:
      case native_types.WebErrorStatus.WebErrorStatusUnexpectedError:
      case native_types.WebErrorStatus.WebErrorStatusUnknown:
        return WebResourceErrorType.unknown;
    }
  }
}

class _WindowsWebViewPermissionRequest
    extends PlatformWebViewPermissionRequest {
  _WindowsWebViewPermissionRequest({required super.types});

  final Completer<native_types.WebviewPermissionDecision> decision =
      Completer<native_types.WebviewPermissionDecision>();

  @override
  Future<void> grant() async {
    if (!decision.isCompleted) {
      decision.complete(native_types.WebviewPermissionDecision.allow);
    }
  }

  @override
  Future<void> deny() async {
    if (!decision.isCompleted) {
      decision.complete(native_types.WebviewPermissionDecision.deny);
    }
  }
}
