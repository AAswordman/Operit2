import 'dart:async';
import 'dart:convert';
import 'dart:ui';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import 'windows_cursor.dart';
import 'windows_webview_cookie.dart';
import 'windows_webview_types.dart';
import 'windows_webview_api.g.dart';
import 'windows_webview_constants.dart';

class HistoryChanged {
  final bool canGoBack;
  final bool canGoForward;
  const HistoryChanged(this.canGoBack, this.canGoForward);
}

class WebviewDownloadEvent {
  final WebviewDownloadEventKind kind;
  final String url;
  final String resultFilePath;
  final int bytesReceived;
  final int totalBytesToReceive;
  const WebviewDownloadEvent(
    this.kind,
    this.url,
    this.resultFilePath,
    this.bytesReceived,
    this.totalBytesToReceive,
  );
}

class WebviewHttpResponseError {
  final String url;
  final String? method;
  final Map<String, String> requestHeaders;
  final int statusCode;
  final Map<String, String> responseHeaders;
  final String? reasonPhrase;

  const WebviewHttpResponseError(
    this.url,
    this.statusCode, {
    this.method,
    this.requestHeaders = const <String, String>{},
    this.responseHeaders = const <String, String>{},
    this.reasonPhrase,
  });
}

class WebviewSurfaceFrame {
  /// Creates one compositor surface frame captured by WebView2.
  const WebviewSurfaceFrame({
    required this.codec,
    required this.data,
    required this.width,
    required this.height,
  });

  /// Decodes one native frame payload from a method channel response.
  factory WebviewSurfaceFrame.fromMap(Map<Object?, Object?> value) {
    return WebviewSurfaceFrame(
      codec: value['codec']! as String,
      data: value['data']! as Uint8List,
      width: (value['width']! as num).toDouble(),
      height: (value['height']! as num).toDouble(),
    );
  }

  final String codec;
  final Uint8List data;
  final double width;
  final double height;
}

Map<String, String> _stringMapFromEvent(Object? value) {
  final Map<String, String> result = <String, String>{};
  if (value case final Map<dynamic, dynamic> rawHeaders) {
    for (final MapEntry<dynamic, dynamic> header in rawHeaders.entries) {
      result['${header.key}'] = '${header.value}';
    }
  }
  return result;
}

typedef PermissionRequestedDelegate =
    FutureOr<WebviewPermissionDecision> Function(
      String url,
      WebviewPermissionKind permissionKind,
      bool isUserInitiated,
    );

typedef JavaScriptDialogRequestedDelegate =
    FutureOr<Map<String, Object?>?> Function(
      String dialogType,
      String url,
      String message,
      String? defaultText,
    );

typedef HttpAuthRequestedDelegate =
    FutureOr<Map<String, Object?>?> Function(String url, String challenge);

typedef SslAuthErrorRequestedDelegate =
    FutureOr<Map<String, Object?>?> Function(String url, int errorStatus);

typedef ScriptID = String;

/// Attempts to translate a button constant such as [kPrimaryMouseButton]
/// to a [PointerButton]
PointerButton getButton(int value) {
  switch (value) {
    case kPrimaryMouseButton:
      return PointerButton.primary;
    case kSecondaryMouseButton:
      return PointerButton.secondary;
    case kTertiaryButton:
      return PointerButton.tertiary;
    default:
      return PointerButton.none;
  }
}

final WindowsWebViewHostApi _hostApi = WindowsWebViewHostApi();

class WebviewValue {
  const WebviewValue({required this.isInitialized});

  final bool isInitialized;

  WebviewValue copyWith({bool? isInitialized}) {
    return WebviewValue(isInitialized: isInitialized ?? this.isInitialized);
  }

  WebviewValue.uninitialized() : this(isInitialized: false);
}

/// Controls a WebView and provides streams for various change events.
class WebviewController extends ValueNotifier<WebviewValue> {
  /// Explicitly initializes the underlying WebView environment
  /// using  an optional [browserExePath], an optional [userDataPath]
  /// and optional Chromium command line arguments [additionalArguments].
  ///
  /// The environment is shared between all WebviewController instances and
  /// can be initialized only once. Initialization must take place before any
  /// WebviewController is created/initialized.
  ///
  /// Throws [PlatformException] if the environment was initialized before.
  static Future<void> initializeEnvironment({
    String? userDataPath,
    String? browserExePath,
    String? additionalArguments,
  }) async {
    return _hostApi.initializeEnvironment(
      WindowsEnvironmentOptions(
        userDataPath: userDataPath,
        browserExePath: browserExePath,
        additionalArguments: additionalArguments,
      ),
    );
  }

  /// Get the browser version info including channel name if it is not the
  /// WebView2 Runtime.
  /// Returns [null] if the webview2 runtime is not installed.
  static Future<String?> getWebViewVersion() async {
    return _hostApi.getWebViewVersion();
  }

  late Completer<void> _creatingCompleter;
  int _textureId = 0;
  bool _isDisposed = false;

  Future<void> get ready => _creatingCompleter.future;

  /// Returns the Flutter texture identifier for this WebView compositor surface.
  int get surfaceTextureId {
    if (_isDisposed || !value.isInitialized) {
      throw StateError('WebView compositor surface is not initialized');
    }
    return _textureId;
  }

  /// Moves the compositor surface cursor to a local logical position.
  Future<void> setSurfaceCursorPosition(Offset position) {
    return _setCursorPos(position);
  }

  /// Changes one compositor surface pointer button state.
  Future<void> setSurfacePointerButton(
    PointerButton button, {
    required bool isDown,
  }) {
    return _setPointerButtonState(button, isDown);
  }

  /// Sends a logical scroll delta to the compositor surface.
  Future<void> setSurfaceScrollDelta(double dx, double dy) {
    return _setScrollDelta(dx, dy);
  }

  /// Resizes the compositor surface in logical pixels.
  Future<void> setSurfaceSize(Size size, double scaleFactor) {
    return _setSize(size, scaleFactor);
  }

  /// Dispatches one browser key event through the native WebView input stack.
  Future<void> dispatchSurfaceKeyEvent(String eventJson) {
    return _methodChannel.invokeMethod<void>('dispatchKeyEvent', eventJson);
  }

  /// Captures one compositor frame from the native WebView surface.
  Future<WebviewSurfaceFrame> captureSurfaceFrame() async {
    final result = await _methodChannel.invokeMethod<Object?>(
      'captureSurfaceFrame',
    );
    if (result is! Map<Object?, Object?>) {
      throw StateError('Native browser surface frame is not a map');
    }
    return WebviewSurfaceFrame.fromMap(result);
  }

  PermissionRequestedDelegate? _permissionRequested;
  JavaScriptDialogRequestedDelegate? _javaScriptDialogRequested;
  HttpAuthRequestedDelegate? _httpAuthRequested;
  SslAuthErrorRequestedDelegate? _sslAuthErrorRequested;

  void setPermissionRequestedDelegate(
    PermissionRequestedDelegate? permissionRequested,
  ) {
    _permissionRequested = permissionRequested;
  }

  void setJavaScriptDialogRequestedDelegate(
    JavaScriptDialogRequestedDelegate? javaScriptDialogRequested,
  ) {
    _javaScriptDialogRequested = javaScriptDialogRequested;
  }

  void setHttpAuthRequestedDelegate(
    HttpAuthRequestedDelegate? httpAuthRequested,
  ) {
    _httpAuthRequested = httpAuthRequested;
  }

  void setSslAuthErrorRequestedDelegate(
    SslAuthErrorRequestedDelegate? sslAuthErrorRequested,
  ) {
    _sslAuthErrorRequested = sslAuthErrorRequested;
  }

  late MethodChannel _methodChannel;
  late EventChannel _eventChannel;
  StreamSubscription? _eventStreamSubscription;

  final StreamController<String> _urlStreamController =
      StreamController<String>();

  /// A stream reflecting the current URL.
  Stream<String> get url => _urlStreamController.stream;

  final StreamController<LoadingState> _loadingStateStreamController =
      StreamController<LoadingState>.broadcast();

  final StreamController<WebviewDownloadEvent> _downloadEventStreamController =
      StreamController<WebviewDownloadEvent>.broadcast();

  final StreamController<WebErrorStatus> _onLoadErrorStreamController =
      StreamController<WebErrorStatus>();

  final StreamController<WebviewHttpResponseError>
  _httpResponseErrorStreamController =
      StreamController<WebviewHttpResponseError>();

  /// A stream reflecting the current loading state.
  Stream<LoadingState> get loadingState => _loadingStateStreamController.stream;

  Stream<WebviewDownloadEvent> get onDownloadEvent =>
      _downloadEventStreamController.stream;

  /// A stream reflecting the navigation error when navigation completed with an error.
  Stream<WebErrorStatus> get onLoadError => _onLoadErrorStreamController.stream;

  /// A stream reflecting HTTP response status errors.
  Stream<WebviewHttpResponseError> get httpResponseError =>
      _httpResponseErrorStreamController.stream;

  final StreamController<HistoryChanged> _historyChangedStreamController =
      StreamController<HistoryChanged>();

  /// A stream reflecting the current history state.
  Stream<HistoryChanged> get historyChanged =>
      _historyChangedStreamController.stream;

  final StreamController<String> _securityStateChangedStreamController =
      StreamController<String>();

  /// A stream reflecting the current security state.
  Stream<String> get securityStateChanged =>
      _securityStateChangedStreamController.stream;

  final StreamController<String> _titleStreamController =
      StreamController<String>();

  /// A stream reflecting the current document title.
  Stream<String> get title => _titleStreamController.stream;

  final StreamController<SystemMouseCursor> _cursorStreamController =
      StreamController<SystemMouseCursor>.broadcast();

  /// A stream reflecting the current cursor style.
  Stream<SystemMouseCursor> get _cursor => _cursorStreamController.stream;

  final StreamController<dynamic> _webMessageStreamController =
      StreamController<dynamic>();

  Stream<dynamic> get webMessage => _webMessageStreamController.stream;

  final StreamController<bool>
  _containsFullScreenElementChangedStreamController =
      StreamController<bool>.broadcast();

  /// A stream reflecting whether the document currently contains full-screen elements.
  Stream<bool> get containsFullScreenElementChanged =>
      _containsFullScreenElementChangedStreamController.stream;

  WebviewController() : super(WebviewValue.uninitialized());

  /// Initializes the underlying platform view.
  Future<void> initialize() async {
    if (_isDisposed) {
      return Future<void>.value();
    }
    _creatingCompleter = Completer<void>();
    try {
      final reply = await _hostApi.createWebView();
      _textureId = reply.textureId;
      _methodChannel = MethodChannel(
        '$windowsWebViewChannelPrefix/$_textureId',
      );
      _eventChannel = EventChannel(
        '$windowsWebViewChannelPrefix/$_textureId/events',
      );
      _eventStreamSubscription = _eventChannel.receiveBroadcastStream().listen((
        event,
      ) {
        final map = event as Map<dynamic, dynamic>;
        switch (map['type']) {
          case 'urlChanged':
            _urlStreamController.add(map['value']);
            break;
          case 'onLoadError':
            final value = WebErrorStatus.values[map['value']];
            _onLoadErrorStreamController.add(value);
            break;
          case 'httpError':
            final value =
                map['value'] as Map<dynamic, dynamic>? ??
                const <dynamic, dynamic>{};
            _httpResponseErrorStreamController.add(
              WebviewHttpResponseError(
                '${value['url'] ?? ''}',
                (value['statusCode'] as num?)?.toInt() ?? 0,
                method: value['method'] as String?,
                requestHeaders: _stringMapFromEvent(value['requestHeaders']),
                responseHeaders: _stringMapFromEvent(value['responseHeaders']),
                reasonPhrase: value['reasonPhrase'] as String?,
              ),
            );
            break;
          case 'loadingStateChanged':
            final value = LoadingState.values[map['value']];
            _loadingStateStreamController.add(value);
            break;
          case 'downloadEvent':
            final value = WebviewDownloadEvent(
              WebviewDownloadEventKind.values[map['value']['kind']],
              map['value']['url'],
              map['value']['resultFilePath'],
              map['value']['bytesReceived'],
              map['value']['totalBytesToReceive'],
            );
            _downloadEventStreamController.add(value);
            break;
          case 'historyChanged':
            final value = HistoryChanged(
              map['value']['canGoBack'],
              map['value']['canGoForward'],
            );
            _historyChangedStreamController.add(value);
            break;
          case 'securityStateChanged':
            _securityStateChangedStreamController.add(map['value']);
            break;
          case 'titleChanged':
            _titleStreamController.add(map['value']);
            break;
          case 'cursorChanged':
            _cursorStreamController.add(getCursorByName(map['value']));
            break;
          case 'webMessageReceived':
            try {
              final message = json.decode(map['value']);
              _webMessageStreamController.add(message);
            } catch (ex) {
              _webMessageStreamController.addError(ex);
            }
            break;
          case 'containsFullScreenElementChanged':
            _containsFullScreenElementChangedStreamController.add(map['value']);
            break;
        }
      });

      _methodChannel.setMethodCallHandler((call) {
        if (call.method == 'permissionRequested') {
          return _onPermissionRequested(
            call.arguments as Map<dynamic, dynamic>,
          );
        }
        if (call.method == 'javaScriptDialogRequested') {
          return _onJavaScriptDialogRequested(
            call.arguments as Map<dynamic, dynamic>,
          );
        }
        if (call.method == 'httpAuthRequested') {
          return _onHttpAuthRequested(call.arguments as Map<dynamic, dynamic>);
        }
        if (call.method == 'sslAuthError') {
          return _onSslAuthError(call.arguments as Map<dynamic, dynamic>);
        }

        throw MissingPluginException('Unknown method ${call.method}');
      });

      value = value.copyWith(isInitialized: true);
      _creatingCompleter.complete();
    } on PlatformException catch (e) {
      _creatingCompleter.completeError(e);
    }

    return _creatingCompleter.future;
  }

  Future<bool?> _onPermissionRequested(Map<dynamic, dynamic> args) async {
    if (_permissionRequested == null) {
      return null;
    }

    final url = args['url'] as String?;
    final permissionKindIndex = args['permissionKind'] as int?;
    final isUserInitiated = args['isUserInitiated'] as bool?;

    if (url != null && permissionKindIndex != null && isUserInitiated != null) {
      final WebviewPermissionKind? permissionKind =
          _webviewPermissionKindFromIndex(permissionKindIndex);
      if (permissionKind == null) {
        return null;
      }
      final decision = await _permissionRequested!(
        url,
        permissionKind,
        isUserInitiated,
      );

      switch (decision) {
        case WebviewPermissionDecision.allow:
          return true;
        case WebviewPermissionDecision.deny:
          return false;
        default:
          return null;
      }
    }

    return null;
  }

  WebviewPermissionKind? _webviewPermissionKindFromIndex(int index) {
    if (index < 0 || index >= WebviewPermissionKind.values.length) {
      return null;
    }
    return WebviewPermissionKind.values[index];
  }

  Future<Map<String, Object?>?> _onJavaScriptDialogRequested(
    Map<dynamic, dynamic> args,
  ) async {
    final callback = _javaScriptDialogRequested;
    if (callback == null) {
      return null;
    }

    final dialogType = args['dialogType'] as String?;
    final url = args['url'] as String?;
    final message = args['message'] as String?;
    if (dialogType == null || url == null || message == null) {
      return null;
    }

    return callback(dialogType, url, message, args['defaultText'] as String?);
  }

  Future<Map<String, Object?>?> _onHttpAuthRequested(
    Map<dynamic, dynamic> args,
  ) async {
    final callback = _httpAuthRequested;
    if (callback == null) {
      return null;
    }

    final url = args['url'] as String?;
    final challenge = args['challenge'] as String?;
    if (url == null || challenge == null) {
      return null;
    }

    return callback(url, challenge);
  }

  Future<Map<String, Object?>?> _onSslAuthError(
    Map<dynamic, dynamic> args,
  ) async {
    final callback = _sslAuthErrorRequested;
    if (callback == null) {
      return null;
    }

    final url = args['url'] as String?;
    final errorStatus = args['errorStatus'] as int?;
    if (url == null || errorStatus == null) {
      return null;
    }

    return callback(url, errorStatus);
  }

  @override
  Future<void> dispose() async {
    await _creatingCompleter.future;
    if (!_isDisposed) {
      _isDisposed = true;
      await _eventStreamSubscription?.cancel();
      await _hostApi.disposeWebView(_textureId);
    }
    super.dispose();
  }

  /// Loads the given [url].
  Future<void> loadUrl(String url) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.loadUrl(_textureId, url);
  }

  /// Loads a request with the supplied HTTP method, headers, and optional body.
  Future<void> loadRequest({
    required String url,
    required String method,
    required String headers,
    Uint8List? body,
  }) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.loadRequest(
      _textureId,
      WindowsLoadRequestData(
        url: url,
        method: method,
        headers: headers,
        body: body,
      ),
    );
  }

  /// Loads a document from the given string.
  Future<void> loadStringContent(String content) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.loadStringContent(_textureId, content);
  }

  /// Reloads the current document.
  Future<void> reload() async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.reload(_textureId);
  }

  /// Stops all navigations and pending resource fetches.
  Future<void> stop() async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.stop(_textureId);
  }

  /// Navigates the WebView to the previous page in the navigation history.
  Future<void> goBack() async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.goBack(_textureId);
  }

  /// Navigates the WebView to the next page in the navigation history.
  Future<void> goForward() async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.goForward(_textureId);
  }

  /// Adds the provided JavaScript [script] to a list of scripts that should be run after the global
  /// object has been created, but before the HTML document has been parsed and before any
  /// other script included by the HTML document is run.
  ///
  /// Returns a [ScriptID] on success which can be used for [removeScriptToExecuteOnDocumentCreated].
  ///
  /// see https://docs.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2?view=webview2-1.0.1264.42#addscripttoexecuteondocumentcreated
  Future<ScriptID?> addScriptToExecuteOnDocumentCreated(String script) async {
    if (_isDisposed) {
      return null;
    }
    assert(value.isInitialized);
    return _hostApi.addScriptToExecuteOnDocumentCreated(_textureId, script);
  }

  /// Removes the script identified by [scriptId] from the list of registered scripts.
  ///
  /// see https://docs.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2?view=webview2-1.0.1264.42#removescripttoexecuteondocumentcreated
  Future<void> removeScriptToExecuteOnDocumentCreated(ScriptID scriptId) async {
    if (_isDisposed) {
      return null;
    }
    assert(value.isInitialized);
    return _hostApi.removeScriptToExecuteOnDocumentCreated(
      _textureId,
      scriptId,
    );
  }

  /// Runs the JavaScript [script] in the current top-level document rendered in
  /// the WebView and returns its result.
  ///
  /// see https://docs.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2?view=webview2-1.0.1264.42#executescript
  Future<dynamic> executeScript(String script) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);

    final data = await _hostApi.executeScript(_textureId, script);
    return jsonDecode(data);
  }

  /// Posts the given JSON-formatted message to the current document.
  Future<void> postWebMessage(String message) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.postWebMessage(_textureId, message);
  }

  /// Sets the user agent value.
  ///
  /// Passing null resets the WebView to the WebView2 default user agent.
  Future<void> setUserAgent(String? userAgent) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setUserAgent(_textureId, userAgent);
  }

  /// Returns the current user agent value from WebView2.
  Future<String?> getUserAgent() async {
    if (_isDisposed) {
      return null;
    }
    assert(value.isInitialized);
    return _hostApi.getUserAgent(_textureId);
  }

  /// Sets whether JavaScript execution is enabled.
  Future<void> setJavaScriptEnabled(bool enabled) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setJavaScriptEnabled(_textureId, enabled);
  }

  /// Clears browser cookies.
  Future<void> clearCookies() async {
    await clearCookiesWithResult();
  }

  /// Clears browser cookies and returns whether any cookies were deleted.
  Future<bool> clearCookiesWithResult() async {
    if (_isDisposed) {
      return false;
    }
    assert(value.isInitialized);
    return _hostApi.clearCookies(_textureId);
  }

  /// Sets a browser cookie.
  Future<void> setCookie({
    required String name,
    required String value,
    required String domain,
    required String path,
    DateTime? expires,
    bool? isHttpOnly,
    bool? isSecure,
    int? sameSite,
  }) async {
    if (_isDisposed) {
      return;
    }
    assert(this.value.isInitialized);
    return _hostApi.setCookie(
      _textureId,
      WindowsCookieData(
        name: name,
        value: value,
        domain: domain,
        path: path,
        expires: expires == null ? null : expires.millisecondsSinceEpoch / 1000,
        isHttpOnly: isHttpOnly,
        isSecure: isSecure,
        sameSite: sameSite,
      ),
    );
  }

  /// Returns the cookies visible for [url].
  Future<List<WindowsWebViewCookie>> getCookies(String url) async {
    if (_isDisposed) {
      return <WindowsWebViewCookie>[];
    }
    assert(value.isInitialized);
    final result = await _hostApi.getCookies(_textureId, url);

    return result.whereType<WindowsCookieData>().map((
      WindowsCookieData cookie,
    ) {
      return WindowsWebViewCookie(
        name: cookie.name,
        value: cookie.value,
        domain: cookie.domain,
        path: cookie.path,
        expires: cookie.expires == null
            ? null
            : DateTime.fromMillisecondsSinceEpoch(
                (cookie.expires! * 1000).round(),
              ),
        isHttpOnly: cookie.isHttpOnly,
        isSecure: cookie.isSecure,
        sameSite: cookie.sameSite == null
            ? null
            : WindowsWebViewCookieSameSite.values[cookie.sameSite!],
        isSession: cookie.isSession,
      );
    }).toList();
  }

  /// Deletes a browser cookie.
  Future<void> deleteCookie(WindowsWebViewCookie cookie) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.deleteCookie(
      _textureId,
      WindowsCookieData(
        name: cookie.name,
        value: cookie.value,
        domain: cookie.domain,
        path: cookie.path,
        expires: cookie.expires == null
            ? null
            : cookie.expires!.millisecondsSinceEpoch / 1000,
        isHttpOnly: cookie.isHttpOnly,
        isSecure: cookie.isSecure,
        sameSite: cookie.sameSite?.index,
        isSession: cookie.isSession,
      ),
    );
  }

  /// Deletes cookies matching [name] and [url].
  Future<void> deleteCookiesWithNameAndUrl(String name, String url) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.deleteCookiesWithNameAndUrl(_textureId, name, url);
  }

  /// Deletes cookies matching [name], [domain], and [path].
  Future<void> deleteCookiesWithNameDomainAndPath(
    String name,
    String domain,
    String path,
  ) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.deleteCookiesWithNameDomainAndPath(
      _textureId,
      name,
      domain,
      path,
    );
  }

  /// Clears browser cache.
  Future<void> clearCache() async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.clearCache(_textureId);
  }

  /// Clears DOM storage for the current WebView profile.
  Future<void> clearLocalStorage() async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.clearLocalStorage(_textureId);
  }

  /// Toggles ignoring cache for each request. If true, cache will not be used.
  Future<void> setCacheDisabled(bool disabled) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setCacheDisabled(_textureId, disabled);
  }

  /// Opens the Browser DevTools in a separate window
  Future<void> openDevTools() async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.openDevTools(_textureId);
  }

  /// Sets the background color to the provided [color].
  ///
  /// Due to a limitation of the underlying WebView implementation,
  /// semi-transparent values are not supported.
  /// Any non-zero alpha value will be considered as opaque (0xff).
  Future<void> setBackgroundColor(Color color) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setBackgroundColor(
      _textureId,
      color.toARGB32().toSigned(32),
    );
  }

  /// Sets whether user-initiated zooming is enabled.
  Future<void> setZoomControlEnabled(bool enabled) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setZoomControlEnabled(_textureId, enabled);
  }

  /// Sets the zoom factor.
  Future<void> setZoomFactor(double zoomFactor) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setZoomFactor(_textureId, zoomFactor);
  }

  /// Sets the [WebviewPopupWindowPolicy].
  Future<void> setPopupWindowPolicy(
    WebviewPopupWindowPolicy popupPolicy,
  ) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setPopupWindowPolicy(_textureId, popupPolicy.index);
  }

  /// Enables native JavaScript dialog interception for the selected dialog
  /// kinds.
  Future<void> setJavaScriptDialogCallbacksEnabled({
    required bool alert,
    required bool confirm,
    required bool prompt,
  }) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setJavaScriptDialogCallbacksEnabled(
      _textureId,
      alert,
      confirm,
      prompt,
    );
  }

  /// Suspends the web view.
  Future<void> suspend() async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.suspend(_textureId);
  }

  /// Resumes the web view.
  Future<void> resume() async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.resume(_textureId);
  }

  /// Adds a Virtual Host Name Mapping.
  ///
  /// Please refer to
  /// [Microsofts](https://docs.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_3#setvirtualhostnametofoldermapping)
  /// documentation for more details.
  Future<void> addVirtualHostNameMapping(
    String hostName,
    String folderPath,
    WebviewHostResourceAccessKind accessKind,
  ) async {
    if (_isDisposed) {
      return;
    }

    return _hostApi.setVirtualHostNameMapping(
      _textureId,
      WindowsVirtualHostMappingData(
        hostName: hostName,
        path: folderPath,
        accessKind: accessKind.index,
      ),
    );
  }

  /// Removes a Virtual Host Name Mapping.
  ///
  /// Please refer to
  /// [Microsofts](https://docs.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_3#clearvirtualhostnametofoldermapping)
  /// documentation for more details.
  Future<void> removeVirtualHostNameMapping(String hostName) async {
    if (_isDisposed) {
      return;
    }
    return _hostApi.clearVirtualHostNameMapping(_textureId, hostName);
  }

  /// Limits the number of frames per second to the given value.
  Future<void> setFpsLimit([int? maxFps = 0]) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setFpsLimit(_textureId, maxFps ?? 0);
  }

  /// Sends a Pointer (Touch) update
  Future<void> _setPointerUpdate(
    WebviewPointerEventKind kind,
    int pointer,
    Offset position,
    double size,
    double pressure,
  ) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setPointerUpdate(
      _textureId,
      WindowsPointerUpdateData(
        pointer: pointer,
        event: kind.index,
        x: position.dx,
        y: position.dy,
        size: size,
        pressure: pressure,
      ),
    );
  }

  /// Moves the virtual cursor to [position].
  Future<void> _setCursorPos(Offset position) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setCursorPos(
      _textureId,
      WindowsPointData(x: position.dx, y: position.dy),
    );
  }

  /// Indicates whether the specified [button] is currently down.
  Future<void> _setPointerButtonState(PointerButton button, bool isDown) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setPointerButton(
      _textureId,
      WindowsPointerButtonData(button: button.index, isDown: isDown),
    );
  }

  /// Sets the horizontal and vertical scroll delta.
  Future<void> _setScrollDelta(double dx, double dy) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setScrollDelta(_textureId, WindowsPointData(x: dx, y: dy));
  }

  /// Sets the surface size to the provided [size].
  Future<void> _setSize(Size size, double scaleFactor) async {
    if (_isDisposed) {
      return;
    }
    assert(value.isInitialized);
    return _hostApi.setSize(
      _textureId,
      WindowsSizeData(
        width: size.width,
        height: size.height,
        scaleFactor: scaleFactor,
      ),
    );
  }
}

class Webview extends StatefulWidget {
  final WebviewController controller;
  final double? width;
  final double? height;

  /// An optional scale factor. Defaults to [FlutterView.devicePixelRatio] for
  /// rendering in native resolution.
  /// Setting this to 1.0 will disable high-DPI support.
  /// This should only be needed to mimic old behavior before high-DPI support
  /// was available.
  final double? scaleFactor;

  /// Whether this widget's Flutter layout owns the native surface dimensions.
  final bool layoutControlsSurfaceSize;

  /// The [FilterQuality] used for scaling the texture's contents.
  /// Defaults to [FilterQuality.none] as this renders in native resolution
  /// unless specifying a [scaleFactor].
  final FilterQuality filterQuality;

  /// Creates a Flutter texture backed by one native WebView compositor.
  const Webview(
    this.controller, {
    this.width,
    this.height,
    this.scaleFactor,
    this.filterQuality = FilterQuality.none,
    this.layoutControlsSurfaceSize = true,
  });

  @override
  _WebviewState createState() => _WebviewState();
}

class _WebviewState extends State<Webview> {
  final GlobalKey _key = GlobalKey();
  final _downButtons = <int, PointerButton>{};

  PointerDeviceKind _pointerKind = PointerDeviceKind.unknown;

  MouseCursor _cursor = SystemMouseCursors.basic;

  WebviewController get _controller => widget.controller;

  StreamSubscription? _cursorSubscription;

  @override
  void initState() {
    super.initState();

    if (widget.layoutControlsSurfaceSize) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _reportSurfaceSize());
    }

    _cursorSubscription = _controller._cursor.listen((cursor) {
      setState(() {
        _cursor = cursor;
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return (widget.height != null && widget.width != null)
        ? SizedBox(
            key: _key,
            width: widget.width,
            height: widget.height,
            child: _buildInner(),
          )
        : SizedBox.expand(key: _key, child: _buildInner());
  }

  Widget _buildInner() {
    return NotificationListener<SizeChangedLayoutNotification>(
      onNotification: (notification) {
        if (!widget.layoutControlsSurfaceSize) {
          return false;
        }
        _reportSurfaceSize();
        return true;
      },
      child: SizeChangedLayoutNotifier(
        child: _controller.value.isInitialized
            ? Listener(
                onPointerHover: (ev) {
                  // ev.kind is for whatever reason not set to touch
                  // even on touch input
                  if (_pointerKind == PointerDeviceKind.touch) {
                    // Ignoring hover events on touch for now
                    return;
                  }
                  _controller._setCursorPos(ev.localPosition);
                },
                onPointerDown: (ev) {
                  _pointerKind = ev.kind;
                  if (ev.kind == PointerDeviceKind.touch) {
                    _controller._setPointerUpdate(
                      WebviewPointerEventKind.down,
                      ev.pointer,
                      ev.localPosition,
                      ev.size,
                      ev.pressure,
                    );
                    return;
                  }
                  final button = getButton(ev.buttons);
                  _downButtons[ev.pointer] = button;
                  _controller._setPointerButtonState(button, true);
                },
                onPointerUp: (ev) {
                  _pointerKind = ev.kind;
                  if (ev.kind == PointerDeviceKind.touch) {
                    _controller._setPointerUpdate(
                      WebviewPointerEventKind.up,
                      ev.pointer,
                      ev.localPosition,
                      ev.size,
                      ev.pressure,
                    );
                    return;
                  }
                  final button = _downButtons.remove(ev.pointer);
                  if (button != null) {
                    _controller._setPointerButtonState(button, false);
                  }
                },
                onPointerCancel: (ev) {
                  _pointerKind = ev.kind;
                  final button = _downButtons.remove(ev.pointer);
                  if (button != null) {
                    _controller._setPointerButtonState(button, false);
                  }
                },
                onPointerMove: (ev) {
                  _pointerKind = ev.kind;
                  if (ev.kind == PointerDeviceKind.touch) {
                    _controller._setPointerUpdate(
                      WebviewPointerEventKind.update,
                      ev.pointer,
                      ev.localPosition,
                      ev.size,
                      ev.pressure,
                    );
                  } else {
                    _controller._setCursorPos(ev.localPosition);
                  }
                },
                onPointerSignal: (signal) {
                  if (signal is PointerScrollEvent) {
                    _controller._setScrollDelta(
                      -signal.scrollDelta.dx,
                      -signal.scrollDelta.dy,
                    );
                  }
                },
                onPointerPanZoomUpdate: (signal) {
                  if (signal.panDelta.dx.abs() > signal.panDelta.dy.abs()) {
                    _controller._setScrollDelta(-signal.panDelta.dx, 0);
                  } else {
                    _controller._setScrollDelta(0, signal.panDelta.dy);
                  }
                },
                child: MouseRegion(
                  cursor: _cursor,
                  child: Texture(
                    textureId: _controller._textureId,
                    filterQuality: widget.filterQuality,
                  ),
                ),
              )
            : const SizedBox(),
      ),
    );
  }

  void _reportSurfaceSize() async {
    final box = _key.currentContext?.findRenderObject() as RenderBox?;
    if (box != null) {
      await _controller.ready;
      unawaited(
        _controller._setSize(
          box.size,
          widget.scaleFactor ?? View.of(_key.currentContext!).devicePixelRatio,
        ),
      );
    }
  }

  @override
  void dispose() {
    super.dispose();
    _cursorSubscription?.cancel();
  }
}
