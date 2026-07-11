// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'dart:async';
import 'dart:convert';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'dart:ui_web' as ui_web;

import 'package:flutter/widgets.dart';
import 'package:web/web.dart' as web;
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

import 'content_type.dart';
import 'http_request_factory.dart';
import 'web_navigation_delegate.dart';

@JS('JSON.stringify')
external JSString? _jsonStringify(JSAny? value);

extension _WebWindowJavaScriptExtension on web.Window {
  @JS('eval')
  external JSAny? evaluateJavaScript(String script);
}

extension _WebHeadersExtension on web.Headers {
  external void forEach(JSFunction callback);
}

void _validateIFrameAttributeName(String name) {
  if (name.trim().isEmpty) {
    throw ArgumentError.value(
      name,
      'name',
      'Attribute name must not be empty.',
    );
  }
}

void _setIFrameAttribute(
  web.HTMLIFrameElement iFrame,
  String name,
  String? value,
) {
  _validateIFrameAttributeName(name);
  if (value == null) {
    iFrame.removeAttribute(name);
  } else {
    iFrame.setAttribute(name, value);
  }
}

/// An implementation of [PlatformWebViewControllerCreationParams] using Flutter
/// for Web API.
@immutable
class WebWebViewControllerCreationParams
    extends PlatformWebViewControllerCreationParams {
  /// Creates a new [WebWebViewControllerCreationParams] instance.
  WebWebViewControllerCreationParams({
    @visibleForTesting this.httpRequestFactory = const HttpRequestFactory(),
    this.iFrameAllow,
    this.iFrameSandbox,
    this.iFrameReferrerPolicy,
    Map<String, String?> iFrameAttributes = const <String, String?>{},
  }) : iFrameAttributes = Map<String, String?>.unmodifiable(iFrameAttributes),
       super() {
    _setIFrameAttribute(iFrame, 'allow', iFrameAllow);
    _setIFrameAttribute(iFrame, 'sandbox', iFrameSandbox);
    _setIFrameAttribute(iFrame, 'referrerpolicy', iFrameReferrerPolicy);
    this.iFrameAttributes.forEach((String name, String? value) {
      _setIFrameAttribute(iFrame, name, value);
    });
  }

  /// Creates a [WebWebViewControllerCreationParams] instance based on [PlatformWebViewControllerCreationParams].
  WebWebViewControllerCreationParams.fromPlatformWebViewControllerCreationParams(
    PlatformWebViewControllerCreationParams params, {
    @visibleForTesting
    HttpRequestFactory httpRequestFactory = const HttpRequestFactory(),
    String? iFrameAllow,
    String? iFrameSandbox,
    String? iFrameReferrerPolicy,
    Map<String, String?> iFrameAttributes = const <String, String?>{},
  }) : this(
         httpRequestFactory: httpRequestFactory,
         iFrameAllow: iFrameAllow,
         iFrameSandbox: iFrameSandbox,
         iFrameReferrerPolicy: iFrameReferrerPolicy,
         iFrameAttributes: iFrameAttributes,
       );

  static int _nextIFrameId = 0;

  /// Handles creating and sending URL requests.
  final HttpRequestFactory httpRequestFactory;

  /// Value for the iframe `allow` attribute.
  final String? iFrameAllow;

  /// Value for the iframe `sandbox` attribute while JavaScript is unrestricted.
  final String? iFrameSandbox;

  /// Value for the iframe `referrerpolicy` attribute.
  final String? iFrameReferrerPolicy;

  /// Additional attributes applied to the underlying iframe.
  ///
  /// Values from this map override [iFrameAllow], [iFrameSandbox], and
  /// [iFrameReferrerPolicy] when the same attribute name is present.
  final Map<String, String?> iFrameAttributes;

  /// The underlying element used as the WebView.
  @visibleForTesting
  final web.HTMLIFrameElement iFrame = web.HTMLIFrameElement()
    ..id = 'webView${_nextIFrameId++}'
    ..style.width = '100%'
    ..style.height = '100%'
    ..style.border = 'none'
    ..style.borderStyle = 'none'
    ..style.borderWidth = '0';
}

/// An implementation of [PlatformWebViewController] using Flutter for Web API.
class WebWebViewController extends PlatformWebViewController {
  /// Constructs a [WebWebViewController].
  WebWebViewController(PlatformWebViewControllerCreationParams params)
    : super.implementation(
        params is WebWebViewControllerCreationParams
            ? params
            : WebWebViewControllerCreationParams.fromPlatformWebViewControllerCreationParams(
                params,
              ),
      ) {
    _customSandbox = _webWebViewParams.iFrame.getAttribute('sandbox');
    _messageEventListener = ((web.Event event) {
      _handleWindowMessage(event as web.MessageEvent);
    }).toJS;
    web.window.addEventListener('message', _messageEventListener);
    _webWebViewParams.iFrame.onLoad.listen((_) {
      final String? resolvedUrl = _shouldPreserveLogicalUrl
          ? _currentUrl
          : _tryReadCurrentFrameUrl() ?? _currentUrl;
      if (resolvedUrl != null) {
        _currentUrl = resolvedUrl;
        _navigationDelegate?.onUrlChange?.call(UrlChange(url: resolvedUrl));
        _navigationDelegate?.onPageFinished?.call(resolvedUrl);
      }
      _applyScrollBarStyle();
      _installJavaScriptChannels();
      _installConsoleMessageHook();
      _installJavaScriptAlertDialogHook();
      _installJavaScriptConfirmDialogHook();
      _installJavaScriptTextInputDialogHook();
      _installPlatformPermissionRequestHook();
      _attachScrollListenerToCurrentWindow();
      _navigationDelegate?.onProgress?.call(100);
    });
  }

  static const String _scrollBarStyleId = '__webview_all_scrollbars';
  static const String _channelMessageType = '__webview_all_type';
  static const String _javaScriptChannelMessageType = 'javascriptChannel';
  static const String _consoleMessageType = 'consoleMessage';
  static const String _javaScriptAlertDialogMessageType =
      'javaScriptAlertDialog';
  static const String _javaScriptDialogBridgeName = '__webviewAllDialogBridge';
  static final JSObject _javaScriptDialogBridgeRoot = JSObject();
  static bool _isJavaScriptDialogBridgeRootInstalled = false;
  static const String _platformPermissionRequestMessageType =
      'platformPermissionRequest';
  static const String _platformPermissionDecisionMessageType =
      'platformPermissionDecision';
  static const String _javaScriptDisabledSandbox =
      'allow-same-origin allow-forms allow-popups allow-downloads allow-modals';

  WebWebViewControllerCreationParams get _webWebViewParams =>
      params as WebWebViewControllerCreationParams;

  final List<String> _history = <String>[];
  int _historyIndex = -1;
  String? _currentUrl;
  String? _userAgentOverride;
  String? _customSandbox;
  String? _lastHtmlStringContent;
  LoadRequestParams? _lastXhrRequestParams;
  WebNavigationDelegate? _navigationDelegate;
  _NavigationLoadType _lastLoadedType = _NavigationLoadType.none;
  bool _verticalScrollBarEnabled = true;
  bool _horizontalScrollBarEnabled = true;
  JavaScriptMode _javaScriptMode = JavaScriptMode.unrestricted;
  final Map<String, JavaScriptChannelParams> _javaScriptChannels =
      <String, JavaScriptChannelParams>{};
  void Function(JavaScriptConsoleMessage consoleMessage)? _onConsoleMessage;
  Future<void> Function(JavaScriptAlertDialogRequest request)?
  _onJavaScriptAlertDialog;
  Future<bool> Function(JavaScriptConfirmDialogRequest request)?
  _onJavaScriptConfirmDialog;
  Future<String> Function(JavaScriptTextInputDialogRequest request)?
  _onJavaScriptTextInputDialog;
  void Function(PlatformWebViewPermissionRequest request)?
  _onPlatformPermissionRequest;
  void Function(ScrollPositionChange scrollPositionChange)?
  _onScrollPositionChange;
  late final web.EventListener _messageEventListener;
  JSExportedDartFunction? _javaScriptConfirmDialogBridge;
  JSExportedDartFunction? _javaScriptTextInputDialogBridge;
  web.EventListener? _scrollEventListener;
  web.Window? _scrollEventListenerWindow;

  @override
  Future<void> loadFile(String absoluteFilePath) {
    throw UnsupportedError(
      'loadFile is not supported on web. Use loadFlutterAsset or '
      'loadHtmlString instead.',
    );
  }

  @override
  Future<void> loadFileWithParams(LoadFileParams params) {
    return loadFile(params.absoluteFilePath);
  }

  @override
  Future<void> loadFlutterAsset(String key) async {
    final String assetUrl = _resolveFlutterAssetUrl(key);
    await _loadUrl(assetUrl, updateHistory: true);
  }

  @override
  Future<void> loadHtmlString(String html, {String? baseUrl}) async {
    final String content = _injectBaseUrl(html, baseUrl);
    _navigationDelegate?.onPageStarted?.call(baseUrl ?? 'about:blank');
    _navigationDelegate?.onProgress?.call(0);
    _currentUrl = baseUrl ?? 'about:blank';
    _lastHtmlStringContent = content;
    _lastXhrRequestParams = null;
    _lastLoadedType = _NavigationLoadType.html;
    _markLogicalUrlHistoryEntry(_currentUrl!);
    _webWebViewParams.iFrame.srcdoc = content.toJS;
  }

  @override
  Future<void> loadRequest(LoadRequestParams params) async {
    if (!params.uri.hasScheme) {
      throw ArgumentError(
        'LoadRequestParams#uri is required to have a scheme.',
      );
    }

    if (params.headers.isEmpty &&
        (params.body == null || params.body!.isEmpty) &&
        params.method == LoadRequestMethod.get) {
      await _loadUrl(params.uri.toString(), updateHistory: true);
    } else {
      await _updateIFrameFromXhr(params);
    }
  }

  @override
  Future<String?> currentUrl() async {
    return _currentUrl;
  }

  @override
  Future<bool> canGoBack() async {
    return _historyIndex > 0;
  }

  @override
  Future<bool> canGoForward() async {
    return _historyIndex >= 0 && _historyIndex < _history.length - 1;
  }

  @override
  Future<void> goBack() async {
    if (!await canGoBack()) {
      return;
    }
    _historyIndex -= 1;
    await _loadUrl(_history[_historyIndex], updateHistory: false);
  }

  @override
  Future<void> goForward() async {
    if (!await canGoForward()) {
      return;
    }
    _historyIndex += 1;
    await _loadUrl(_history[_historyIndex], updateHistory: false);
  }

  @override
  Future<void> reload() async {
    if (_lastLoadedType == _NavigationLoadType.html) {
      _webWebViewParams.iFrame.srcdoc = (_lastHtmlStringContent ?? '').toJS;
      return;
    }

    if (_lastLoadedType == _NavigationLoadType.xhrResponse &&
        _lastXhrRequestParams != null) {
      await _updateIFrameFromXhr(_lastXhrRequestParams!, updateHistory: false);
      return;
    }

    if (_currentUrl != null) {
      await _loadUrl(_currentUrl!, updateHistory: false);
    }
  }

  @override
  Future<void> clearCache() async {
    try {
      final web.CacheStorage cacheStorage = web.window.caches;
      final JSArray<JSString> keys = await cacheStorage.keys().toDart;
      for (final JSString key in keys.toDart) {
        await cacheStorage.delete(key.toDart).toDart;
      }
    } catch (_) {
      // Best-effort only for the host origin.
    }
  }

  @override
  Future<void> clearLocalStorage() async {
    try {
      web.window.localStorage.clear();
      web.window.sessionStorage.clear();
    } catch (_) {
      // Best-effort only for the host origin.
    }
  }

  @override
  Future<void> setPlatformNavigationDelegate(
    PlatformNavigationDelegate handler,
  ) async {
    _navigationDelegate = handler as WebNavigationDelegate;
  }

  @override
  Future<Offset> getScrollPosition() async {
    final web.Window window = _requireAccessibleContentWindow(
      'Reading scroll position is only supported for same-origin iframe content.',
    );
    return Offset(window.scrollX, window.scrollY);
  }

  @override
  Future<void> scrollTo(int x, int y) async {
    _requireAccessibleContentWindow(
      'Scrolling iframe content is only supported for same-origin iframe content.',
    ).scrollTo(x.toJS, y);
  }

  @override
  Future<void> scrollBy(int x, int y) async {
    _requireAccessibleContentWindow(
      'Scrolling iframe content is only supported for same-origin iframe content.',
    ).scrollBy(x.toJS, y);
  }

  web.Window _requireAccessibleContentWindow(String message) {
    try {
      final web.Window? window = _webWebViewParams.iFrame.contentWindow;
      if (window == null) {
        throw UnsupportedError(message);
      }
      // Probe the JavaScript entrypoint because hook installation uses eval.
      window.scrollX;
      _evaluateJavaScriptInWindow(window, 'undefined');
      return window;
    } catch (_) {
      throw UnsupportedError(message);
    }
  }

  web.Document? _tryReadContentDocument() {
    try {
      return _webWebViewParams.iFrame.contentDocument;
    } catch (_) {
      return null;
    }
  }

  web.Window? _tryReadAccessibleContentWindow() {
    try {
      final web.Window? window = _webWebViewParams.iFrame.contentWindow;
      if (window == null) {
        return null;
      }
      // Probe the exact capability used below instead of a weaker frame read.
      _evaluateJavaScriptInWindow(window, 'undefined');
      return window;
    } catch (_) {
      return null;
    }
  }

  /// Evaluates JavaScript in a frame window already proven to be accessible.
  JSAny? _evaluateJavaScriptInWindow(web.Window window, String script) {
    return window.evaluateJavaScript(script);
  }

  /// Evaluates JavaScript in the current iframe when browser rules allow it.
  bool _tryEvaluateJavaScriptInCurrentWindow(String script) {
    try {
      final web.Window? window = _tryReadAccessibleContentWindow();
      if (window == null) {
        return false;
      }
      _evaluateJavaScriptInWindow(window, script);
      return true;
    } catch (_) {
      return false;
    }
  }

  void _applyScrollBarStyle() {
    final web.Document? document = _tryReadContentDocument();
    if (document == null) {
      return;
    }

    final StringBuffer css = StringBuffer();
    if (!_verticalScrollBarEnabled) {
      css.writeln('*::-webkit-scrollbar:vertical { width: 0 !important; }');
    }
    if (!_horizontalScrollBarEnabled) {
      css.writeln('*::-webkit-scrollbar:horizontal { height: 0 !important; }');
    }

    final web.Element? existing = document.getElementById(_scrollBarStyleId);
    if (css.isEmpty) {
      existing?.remove();
      return;
    }

    final web.Element style = existing ?? document.createElement('style');
    if (existing == null) {
      style.id = _scrollBarStyleId;
      (document.head ?? document.documentElement)?.append(style);
    }
    style.textContent = css.toString();
  }

  void _emitScrollPositionChange() {
    final callback = _onScrollPositionChange;
    final web.Window? window = _tryReadAccessibleContentWindow();
    if (callback == null || window == null) {
      return;
    }

    callback(ScrollPositionChange(window.scrollX, window.scrollY));
  }

  void _detachScrollListener() {
    final web.Window? window = _scrollEventListenerWindow;
    final web.EventListener? listener = _scrollEventListener;
    if (window != null && listener != null) {
      try {
        window.removeEventListener('scroll', listener);
      } catch (_) {}
    }
    _scrollEventListenerWindow = null;
  }

  void _attachScrollListenerToCurrentWindow() {
    final callback = _onScrollPositionChange;
    if (callback == null) {
      _detachScrollListener();
      return;
    }

    final web.Window? window = _tryReadAccessibleContentWindow();
    if (window == null || identical(window, _scrollEventListenerWindow)) {
      return;
    }

    _detachScrollListener();
    final web.EventListener listener = ((web.Event event) {
      _emitScrollPositionChange();
    }).toJS;
    window.addEventListener('scroll', listener);
    _scrollEventListener = listener;
    _scrollEventListenerWindow = window;
  }

  void _setIframeScrollBarVisibility({
    bool? verticalEnabled,
    bool? horizontalEnabled,
  }) {
    _verticalScrollBarEnabled = verticalEnabled ?? _verticalScrollBarEnabled;
    _horizontalScrollBarEnabled =
        horizontalEnabled ?? _horizontalScrollBarEnabled;
    _applyScrollBarStyle();
  }

  @override
  Future<void> setVerticalScrollBarEnabled(bool enabled) async {
    _setIframeScrollBarVisibility(verticalEnabled: enabled);
  }

  @override
  Future<void> setHorizontalScrollBarEnabled(bool enabled) async {
    _setIframeScrollBarVisibility(horizontalEnabled: enabled);
  }

  @override
  bool supportsSetScrollBarsEnabled() {
    return true;
  }

  @override
  Future<void> setOnScrollPositionChange(
    void Function(ScrollPositionChange scrollPositionChange)?
    onScrollPositionChange,
  ) async {
    _onScrollPositionChange = onScrollPositionChange;
    _attachScrollListenerToCurrentWindow();
  }

  @override
  Future<void> enableZoom(bool enabled) async {
    if (enabled) {
      _webWebViewParams.iFrame.style.removeProperty('touch-action');
    } else {
      _webWebViewParams.iFrame.style.setProperty('touch-action', 'pan-x pan-y');
    }
  }

  @override
  Future<void> setBackgroundColor(Color color) async {
    _webWebViewParams.iFrame.style.backgroundColor = _cssColorFrom(color);
  }

  @override
  Future<void> setJavaScriptMode(JavaScriptMode javaScriptMode) async {
    _javaScriptMode = javaScriptMode;
    _applySandboxForJavaScriptMode();
  }

  /// Sets or removes an attribute on the underlying iframe element.
  ///
  /// Passing `null` removes the attribute. When [name] is `sandbox`, the value
  /// is preserved as the unrestricted-mode sandbox and will be restored when
  /// [setJavaScriptMode] switches JavaScript back to unrestricted mode.
  Future<void> setIFrameAttribute(String name, String? value) async {
    _validateIFrameAttributeName(name);
    if (name.toLowerCase() == 'sandbox') {
      _customSandbox = value;
      _applySandboxForJavaScriptMode();
      return;
    }

    _setIFrameAttribute(_webWebViewParams.iFrame, name, value);
  }

  /// Sets or removes the iframe `allow` attribute.
  Future<void> setIFrameAllow(String? allow) {
    return setIFrameAttribute('allow', allow);
  }

  /// Sets or removes the iframe `sandbox` attribute.
  Future<void> setIFrameSandbox(String? sandbox) {
    return setIFrameAttribute('sandbox', sandbox);
  }

  /// Sets or removes the iframe `referrerpolicy` attribute.
  Future<void> setIFrameReferrerPolicy(String? referrerPolicy) {
    return setIFrameAttribute('referrerpolicy', referrerPolicy);
  }

  void _applySandboxForJavaScriptMode() {
    if (_javaScriptMode == JavaScriptMode.disabled) {
      _webWebViewParams.iFrame.setAttribute(
        'sandbox',
        _javaScriptDisabledSandbox,
      );
    } else {
      _setIFrameAttribute(_webWebViewParams.iFrame, 'sandbox', _customSandbox);
    }
  }

  @override
  Future<void> setUserAgent(String? userAgent) async {
    if (userAgent == null) {
      _userAgentOverride = null;
      return;
    }

    throw UnsupportedError(
      'Overriding the user agent is not supported by the web iframe implementation.',
    );
  }

  @override
  Future<String?> getUserAgent() async {
    return _userAgentOverride ?? web.window.navigator.userAgent;
  }

  @override
  Future<void> runJavaScript(String javaScript) async {
    if (_javaScriptMode == JavaScriptMode.disabled) {
      throw StateError('JavaScript execution is disabled for this WebView.');
    }
    _requireAccessibleContentWindow(
      'Running JavaScript is only supported for same-origin iframe content.',
    ).evaluateJavaScript(javaScript);
  }

  @override
  Future<Object> runJavaScriptReturningResult(String javaScript) async {
    if (_javaScriptMode == JavaScriptMode.disabled) {
      throw StateError('JavaScript execution is disabled for this WebView.');
    }
    final JSAny? result = _requireAccessibleContentWindow(
      'Running JavaScript is only supported for same-origin iframe content.',
    ).evaluateJavaScript(javaScript);
    return _dartObjectFromJavaScriptResult(result);
  }

  Object _dartObjectFromJavaScriptResult(JSAny? result) {
    if (result == null) {
      throw ArgumentError(
        'The JavaScript returned `null` or `undefined`, which is unsupported.',
      );
    }

    final JSString? jsonResult;
    try {
      jsonResult = _jsonStringify(result);
    } catch (error) {
      throw UnsupportedError(
        'The JavaScript result could not be serialized: $error',
      );
    }

    if (jsonResult == null) {
      throw ArgumentError(
        'The JavaScript returned `null` or `undefined`, which is unsupported.',
      );
    }

    final Object? decoded = jsonDecode(jsonResult.toDart);
    if (decoded == null) {
      throw ArgumentError(
        'The JavaScript returned `null` or `undefined`, which is unsupported.',
      );
    }
    return decoded;
  }

  void _handleWindowMessage(web.MessageEvent event) {
    final JSString? jsonData = _jsonStringify(event.data);
    if (jsonData == null) {
      return;
    }

    final Object? decoded;
    try {
      decoded = jsonDecode(jsonData.toDart);
    } catch (_) {
      return;
    }

    if (decoded is! Map<String, dynamic> ||
        decoded['webViewId'] != _webWebViewParams.iFrame.id) {
      return;
    }

    switch (decoded[_channelMessageType]) {
      case _javaScriptChannelMessageType:
        final String? channelName = decoded['channelName'] as String?;
        final JavaScriptChannelParams? channel = channelName == null
            ? null
            : _javaScriptChannels[channelName];
        if (channel == null) {
          return;
        }
        channel.onMessageReceived(
          JavaScriptMessage(message: '${decoded['message'] ?? ''}'),
        );
        return;
      case _consoleMessageType:
        final void Function(JavaScriptConsoleMessage consoleMessage)? callback =
            _onConsoleMessage;
        if (callback == null) {
          return;
        }
        callback(
          JavaScriptConsoleMessage(
            level: _javaScriptLogLevelFromString(decoded['level'] as String?),
            message: '${decoded['message'] ?? ''}',
          ),
        );
        return;
      case _javaScriptAlertDialogMessageType:
        final Future<void> Function(JavaScriptAlertDialogRequest request)?
        callback = _onJavaScriptAlertDialog;
        if (callback == null) {
          return;
        }
        unawaited(
          callback(
            JavaScriptAlertDialogRequest(
              message: '${decoded['message'] ?? ''}',
              url: '${decoded['url'] ?? _currentUrl ?? 'about:blank'}',
            ),
          ),
        );
        return;
      case _platformPermissionRequestMessageType:
        final void Function(PlatformWebViewPermissionRequest request)?
        callback = _onPlatformPermissionRequest;
        final String? requestId = decoded['requestId'] as String?;
        if (callback == null || requestId == null) {
          return;
        }
        callback(
          WebWebViewPermissionRequest._(
            types: _permissionTypesFromMessage(decoded['types']),
            onDecision: (bool granted) {
              _sendPlatformPermissionDecision(requestId, granted);
            },
          ),
        );
        return;
    }
  }

  Set<WebViewPermissionResourceType> _permissionTypesFromMessage(
    Object? value,
  ) {
    if (value is! List<Object?>) {
      return const <WebViewPermissionResourceType>{};
    }

    return value
        .map<WebViewPermissionResourceType?>((Object? type) {
          return switch (type) {
            'camera' => WebViewPermissionResourceType.camera,
            'microphone' => WebViewPermissionResourceType.microphone,
            _ => null,
          };
        })
        .whereType<WebViewPermissionResourceType>()
        .toSet();
  }

  void _sendPlatformPermissionDecision(String requestId, bool granted) {
    _webWebViewParams.iFrame.contentWindow?.postMessage(
      <String, Object?>{
        _channelMessageType: _platformPermissionDecisionMessageType,
        'webViewId': _webWebViewParams.iFrame.id,
        'requestId': requestId,
        'granted': granted,
      }.jsify(),
      '*'.toJS,
    );
  }

  JavaScriptLogLevel _javaScriptLogLevelFromString(String? level) {
    return switch (level) {
      'debug' => JavaScriptLogLevel.debug,
      'error' => JavaScriptLogLevel.error,
      'info' => JavaScriptLogLevel.info,
      'warning' => JavaScriptLogLevel.warning,
      _ => JavaScriptLogLevel.log,
    };
  }

  void _installJavaScriptChannels() {
    if (_javaScriptMode == JavaScriptMode.disabled) {
      return;
    }
    if (_javaScriptChannels.isEmpty) {
      return;
    }

    for (final JavaScriptChannelParams channel in _javaScriptChannels.values) {
      _tryEvaluateJavaScriptInCurrentWindow(
        _javaScriptChannelScript(channel.name),
      );
    }
  }

  String _javaScriptChannelScript(String name) {
    return '''
      (function() {
        const channelName = ${jsonEncode(name)};
        const webViewId = ${jsonEncode(_webWebViewParams.iFrame.id)};
        window[channelName] = {
          postMessage: function(message) {
            window.parent.postMessage({
              "$_channelMessageType": "$_javaScriptChannelMessageType",
              "webViewId": webViewId,
              "channelName": channelName,
              "message": String(message)
            }, "*");
          }
        };
      })();
      ''';
  }

  String _removeJavaScriptChannelScript(String name) {
    return '''
      (function() {
        const channelName = ${jsonEncode(name)};
        try {
          delete window[channelName];
        } catch (_) {
          window[channelName] = undefined;
        }
      })();
      ''';
  }

  @override
  Future<void> addJavaScriptChannel(
    JavaScriptChannelParams javaScriptChannelParams,
  ) async {
    final String name = javaScriptChannelParams.name;
    if (name.isEmpty) {
      throw ArgumentError.value(
        name,
        'javaScriptChannelParams.name',
        'JavaScript channel names must not be empty.',
      );
    }
    if (_javaScriptChannels.containsKey(name)) {
      throw ArgumentError(
        'A JavaScriptChannel with name `$name` already exists.',
      );
    }

    _javaScriptChannels[name] = javaScriptChannelParams;
    if (_javaScriptMode == JavaScriptMode.disabled) {
      return;
    }
    _tryEvaluateJavaScriptInCurrentWindow(_javaScriptChannelScript(name));
  }

  @override
  Future<void> removeJavaScriptChannel(String javaScriptChannelName) async {
    _javaScriptChannels.remove(javaScriptChannelName);
    _tryEvaluateJavaScriptInCurrentWindow(
      _removeJavaScriptChannelScript(javaScriptChannelName),
    );
  }

  void _installConsoleMessageHook() {
    if (_javaScriptMode == JavaScriptMode.disabled) {
      return;
    }
    if (_onConsoleMessage == null) {
      return;
    }

    _tryEvaluateJavaScriptInCurrentWindow('''
      (function() {
        if (window.__webviewAllConsoleHookInstalled) {
          return;
        }
        window.__webviewAllConsoleHookInstalled = true;
        const webViewId = ${jsonEncode(_webWebViewParams.iFrame.id)};
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
          window.parent.postMessage({
            "$_channelMessageType": "$_consoleMessageType",
            "webViewId": webViewId,
            "level": level,
            "message": Array.from(args).map(stringifyArg).join(' ')
          }, "*");
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
      ''');
  }

  @override
  Future<String?> getTitle() async {
    try {
      return _webWebViewParams.iFrame.contentDocument?.title;
    } catch (_) {
      return null;
    }
  }

  @override
  Future<void> setOnConsoleMessage(
    void Function(JavaScriptConsoleMessage consoleMessage) onConsoleMessage,
  ) async {
    _onConsoleMessage = onConsoleMessage;
    _installConsoleMessageHook();
  }

  void _installJavaScriptAlertDialogHook() {
    if (_javaScriptMode == JavaScriptMode.disabled) {
      return;
    }
    if (_onJavaScriptAlertDialog == null) {
      return;
    }

    _tryEvaluateJavaScriptInCurrentWindow('''
      (function() {
        if (window.__webviewAllAlertDialogHookInstalled) {
          return;
        }
        window.__webviewAllAlertDialogHookInstalled = true;
        const webViewId = ${jsonEncode(_webWebViewParams.iFrame.id)};
        window.alert = function(message) {
          window.parent.postMessage({
            "$_channelMessageType": "$_javaScriptAlertDialogMessageType",
            "webViewId": webViewId,
            "message": String(message),
            "url": window.location.href
          }, "*");
        };
      })();
      ''');
  }

  @override
  Future<void> setOnJavaScriptAlertDialog(
    Future<void> Function(JavaScriptAlertDialogRequest request)
    onJavaScriptAlertDialog,
  ) async {
    _onJavaScriptAlertDialog = onJavaScriptAlertDialog;
    _installJavaScriptAlertDialogHook();
  }

  JSObject _ensureJavaScriptDialogBridgeRoot() {
    if (!_isJavaScriptDialogBridgeRootInstalled) {
      globalContext[_javaScriptDialogBridgeName] = _javaScriptDialogBridgeRoot;
      _isJavaScriptDialogBridgeRootInstalled = true;
    }
    return _javaScriptDialogBridgeRoot;
  }

  void _updateJavaScriptDialogBridge() {
    if (_onJavaScriptConfirmDialog == null &&
        _onJavaScriptTextInputDialog == null) {
      return;
    }

    final JSObject bridge = JSObject();
    if (_onJavaScriptConfirmDialog != null) {
      _javaScriptConfirmDialogBridge ??= ((JSString message, JSString url) {
        return _handleJavaScriptConfirmDialog(message.toDart, url.toDart).toJS;
      }).toJS;
      bridge['confirm'] = _javaScriptConfirmDialogBridge;
    }
    if (_onJavaScriptTextInputDialog != null) {
      _javaScriptTextInputDialogBridge ??=
          ((JSString message, JSString url, JSString? defaultText) {
            return _handleJavaScriptTextInputDialog(
              message.toDart,
              url.toDart,
              defaultText?.toDart,
            ).toJS;
          }).toJS;
      bridge['prompt'] = _javaScriptTextInputDialogBridge;
    }

    _ensureJavaScriptDialogBridgeRoot()[_webWebViewParams.iFrame.id] = bridge;
  }

  T _completeJavaScriptDialogSynchronously<T>(
    Future<T> future,
    String dialogName,
  ) {
    bool completed = false;
    T? result;
    Object? error;
    StackTrace? stackTrace;

    future.then(
      (T value) {
        completed = true;
        result = value;
      },
      onError: (Object exception, StackTrace stack) {
        completed = true;
        error = exception;
        stackTrace = stack;
      },
    );

    if (!completed) {
      throw UnsupportedError(
        'JavaScript $dialogName dialog callbacks on web must complete '
        'synchronously because browser JavaScript dialogs require a '
        'synchronous return value. Return a SynchronousFuture from the '
        'callback.',
      );
    }
    if (error != null) {
      Error.throwWithStackTrace(error!, stackTrace ?? StackTrace.current);
    }
    return result as T;
  }

  bool _handleJavaScriptConfirmDialog(String message, String url) {
    final Future<bool> Function(JavaScriptConfirmDialogRequest request)?
    callback = _onJavaScriptConfirmDialog;
    if (callback == null) {
      return false;
    }

    return _completeJavaScriptDialogSynchronously<bool>(
      callback(JavaScriptConfirmDialogRequest(message: message, url: url)),
      'confirm',
    );
  }

  String _handleJavaScriptTextInputDialog(
    String message,
    String url,
    String? defaultText,
  ) {
    final Future<String> Function(JavaScriptTextInputDialogRequest request)?
    callback = _onJavaScriptTextInputDialog;
    if (callback == null) {
      return defaultText ?? '';
    }

    return _completeJavaScriptDialogSynchronously<String>(
      callback(
        JavaScriptTextInputDialogRequest(
          message: message,
          url: url,
          defaultText: defaultText,
        ),
      ),
      'prompt',
    );
  }

  void _installJavaScriptConfirmDialogHook() {
    if (_javaScriptMode == JavaScriptMode.disabled) {
      return;
    }
    if (_onJavaScriptConfirmDialog == null) {
      return;
    }

    _updateJavaScriptDialogBridge();
    _tryEvaluateJavaScriptInCurrentWindow('''
      (function() {
        if (window.__webviewAllConfirmDialogHookInstalled) {
          return;
        }
        window.__webviewAllConfirmDialogHookInstalled = true;
        const webViewId = ${jsonEncode(_webWebViewParams.iFrame.id)};
        const originalConfirm = window.confirm.bind(window);
        window.confirm = function(message) {
          const bridgeRoot = window.parent &&
              window.parent[${jsonEncode(_javaScriptDialogBridgeName)}];
          const bridge = bridgeRoot && bridgeRoot[webViewId];
          if (!bridge || !bridge.confirm) {
            return originalConfirm(message);
          }
          return Boolean(bridge.confirm(String(message), window.location.href));
        };
      })();
      ''');
  }

  void _installJavaScriptTextInputDialogHook() {
    if (_javaScriptMode == JavaScriptMode.disabled) {
      return;
    }
    if (_onJavaScriptTextInputDialog == null) {
      return;
    }

    _updateJavaScriptDialogBridge();
    _tryEvaluateJavaScriptInCurrentWindow('''
      (function() {
        if (window.__webviewAllTextInputDialogHookInstalled) {
          return;
        }
        window.__webviewAllTextInputDialogHookInstalled = true;
        const webViewId = ${jsonEncode(_webWebViewParams.iFrame.id)};
        const originalPrompt = window.prompt.bind(window);
        window.prompt = function(message, defaultText) {
          const bridgeRoot = window.parent &&
              window.parent[${jsonEncode(_javaScriptDialogBridgeName)}];
          const bridge = bridgeRoot && bridgeRoot[webViewId];
          if (!bridge || !bridge.prompt) {
            return originalPrompt(message, defaultText);
          }
          const result = bridge.prompt(
            String(message),
            window.location.href,
            defaultText == null ? null : String(defaultText)
          );
          return result == null ? null : String(result);
        };
      })();
      ''');
  }

  void _installPlatformPermissionRequestHook() {
    if (_javaScriptMode == JavaScriptMode.disabled) {
      return;
    }
    if (_onPlatformPermissionRequest == null) {
      return;
    }

    _tryEvaluateJavaScriptInCurrentWindow('''
      (function() {
        if (window.__webviewAllPermissionRequestHookInstalled) {
          return;
        }
        if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
          return;
        }

        window.__webviewAllPermissionRequestHookInstalled = true;
        const webViewId = ${jsonEncode(_webWebViewParams.iFrame.id)};
        const originalGetUserMedia =
          navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);
        const pending = new Map();
        let nextRequestId = 0;

        window.addEventListener('message', function(event) {
          const data = event.data;
          if (!data ||
              data["$_channelMessageType"] !== "$_platformPermissionDecisionMessageType" ||
              data.webViewId !== webViewId) {
            return;
          }

          const request = pending.get(data.requestId);
          if (!request) {
            return;
          }
          pending.delete(data.requestId);
          if (data.granted) {
            request.resolve();
          } else {
            request.reject(new DOMException(
              'Permission denied by host application.',
              'NotAllowedError'
            ));
          }
        });

        navigator.mediaDevices.getUserMedia = function(constraints) {
          const requestedTypes = [];
          if (constraints && constraints.video) {
            requestedTypes.push('camera');
          }
          if (constraints && constraints.audio) {
            requestedTypes.push('microphone');
          }
          if (requestedTypes.length === 0) {
            return originalGetUserMedia(constraints);
          }

          const requestId = String(++nextRequestId);
          const permission = new Promise(function(resolve, reject) {
            pending.set(requestId, { resolve: resolve, reject: reject });
          });
          window.parent.postMessage({
            "$_channelMessageType": "$_platformPermissionRequestMessageType",
            "webViewId": webViewId,
            "requestId": requestId,
            "types": requestedTypes
          }, "*");

          return permission.then(function() {
            return originalGetUserMedia(constraints);
          });
        };
      })();
      ''');
  }

  @override
  Future<void> setOnJavaScriptConfirmDialog(
    Future<bool> Function(JavaScriptConfirmDialogRequest request)
    onJavaScriptConfirmDialog,
  ) async {
    _onJavaScriptConfirmDialog = onJavaScriptConfirmDialog;
    _installJavaScriptConfirmDialogHook();
  }

  @override
  Future<void> setOnJavaScriptTextInputDialog(
    Future<String> Function(JavaScriptTextInputDialogRequest request)
    onJavaScriptTextInputDialog,
  ) async {
    _onJavaScriptTextInputDialog = onJavaScriptTextInputDialog;
    _installJavaScriptTextInputDialogHook();
  }

  @override
  Future<void> setOnPlatformPermissionRequest(
    void Function(PlatformWebViewPermissionRequest request) onPermissionRequest,
  ) async {
    _onPlatformPermissionRequest = onPermissionRequest;
    _installPlatformPermissionRequestHook();
  }

  @override
  Future<void> setOverScrollMode(WebViewOverScrollMode mode) async {
    _webWebViewParams.iFrame.style.setProperty(
      'overscroll-behavior',
      switch (mode) {
        WebViewOverScrollMode.always => 'auto',
        WebViewOverScrollMode.ifContentScrolls => 'contain',
        WebViewOverScrollMode.never => 'none',
      },
    );
  }

  Future<void> _loadUrl(String url, {required bool updateHistory}) async {
    if (!await _shouldNavigate(url)) {
      return;
    }

    _currentUrl = url;
    _navigationDelegate?.onPageStarted?.call(url);
    _navigationDelegate?.onProgress?.call(0);
    _navigationDelegate?.onUrlChange?.call(UrlChange(url: url));

    if (updateHistory) {
      if (_historyIndex < _history.length - 1) {
        _history.removeRange(_historyIndex + 1, _history.length);
      }
      _history.add(url);
      _historyIndex = _history.length - 1;
    }

    _lastLoadedType = _NavigationLoadType.url;
    _lastXhrRequestParams = null;
    _webWebViewParams.iFrame.src = url;
  }

  Future<bool> _shouldNavigate(String url) async {
    final NavigationRequestCallback? callback =
        _navigationDelegate?.onNavigationRequest;
    if (callback == null) {
      return true;
    }

    final NavigationDecision decision = await callback(
      NavigationRequest(url: url, isMainFrame: true),
    );
    return decision == NavigationDecision.navigate;
  }

  /// Performs an AJAX request defined by [params].
  Future<void> _updateIFrameFromXhr(
    LoadRequestParams params, {
    bool updateHistory = true,
  }) async {
    if (!await _shouldNavigate(params.uri.toString())) {
      return;
    }

    _currentUrl = params.uri.toString();
    _navigationDelegate?.onPageStarted?.call(_currentUrl!);
    _navigationDelegate?.onProgress?.call(0);
    _navigationDelegate?.onUrlChange?.call(UrlChange(url: _currentUrl));

    final web.Response response;
    try {
      response =
          await _webWebViewParams.httpRequestFactory.request(
                params.uri.toString(),
                method: params.method.serialize(),
                requestHeaders: params.headers,
                sendData: params.body,
              )
              as web.Response;
    } catch (error) {
      _navigationDelegate?.onWebResourceError?.call(
        WebResourceError(
          errorCode: 0,
          description: error.toString(),
          errorType: WebResourceErrorType.connect,
          isForMainFrame: true,
          url: params.uri.toString(),
        ),
      );
      rethrow;
    }

    final Map<String, String> responseHeaders = _headersFromResponse(response);
    final String? contentTypeHeader = responseHeaders['content-type'];
    final String header = contentTypeHeader ?? 'text/html';

    if (response.status >= 400) {
      _navigationDelegate?.onHttpError?.call(
        HttpResponseError(
          request: WebWebResourceRequest._(
            uri: params.uri,
            method: params.method.serialize().toUpperCase(),
            headers: params.headers,
            isForMainFrame: true,
          ),
          response: WebWebResourceResponse._(
            uri: params.uri,
            statusCode: response.status,
            headers: responseHeaders,
            mimeType: _mimeTypeFromContentTypeHeader(contentTypeHeader),
            reasonPhrase: response.statusText.isEmpty
                ? null
                : response.statusText,
          ),
        ),
      );
    }

    final contentType = ContentType.parse(header);
    final Encoding encoding = Encoding.getByName(contentType.charset) ?? utf8;

    if (updateHistory) {
      _markLogicalUrlHistoryEntry(params.uri.toString());
    }

    _lastXhrRequestParams = params;
    _lastLoadedType = _NavigationLoadType.xhrResponse;
    _webWebViewParams.iFrame.src = Uri.dataFromString(
      (await response.text().toDart).toDart,
      mimeType: contentType.mimeType,
      encoding: encoding,
    ).toString();
  }

  String _injectBaseUrl(String html, String? baseUrl) {
    if (baseUrl == null || baseUrl.isEmpty) {
      return html;
    }

    final String baseTag = '<base href="$baseUrl">';
    final RegExp headExp = RegExp(r'<head[^>]*>', caseSensitive: false);
    final Match? match = headExp.firstMatch(html);
    if (match != null) {
      return html.replaceRange(match.end, match.end, baseTag);
    }
    return '<head>$baseTag</head>$html';
  }

  String? _tryReadCurrentFrameUrl() {
    try {
      final String? href =
          _webWebViewParams.iFrame.contentWindow?.location.href;
      if (href == null || href.startsWith('data:')) {
        return _currentUrl;
      }
      return href;
    } catch (_) {
      return _currentUrl;
    }
  }

  bool get _shouldPreserveLogicalUrl {
    return _lastLoadedType == _NavigationLoadType.html ||
        _lastLoadedType == _NavigationLoadType.xhrResponse;
  }

  void _markLogicalUrlHistoryEntry(String url) {
    if (_historyIndex < _history.length - 1) {
      _history.removeRange(_historyIndex + 1, _history.length);
    }
    _history.add(url);
    _historyIndex = _history.length - 1;
  }

  String _resolveFlutterAssetUrl(String key) {
    if (key.isEmpty) {
      throw ArgumentError.value(key, 'key', 'Asset key must not be empty.');
    }

    final String normalized = key.startsWith('/') ? key.substring(1) : key;
    if (normalized.isEmpty) {
      throw ArgumentError.value(key, 'key', 'Asset key must not be empty.');
    }

    final String encodedKey = normalized
        .split('/')
        .map(Uri.encodeComponent)
        .join('/');
    return Uri.base.resolve('assets/$encodedKey').toString();
  }

  Map<String, String> _headersFromResponse(web.Response response) {
    final Map<String, String> headers = <String, String>{};
    response.headers.forEach(
      ((JSString value, JSString name, JSAny _) {
        headers[name.toDart] = value.toDart;
      }).toJS,
    );
    return headers;
  }

  String? _mimeTypeFromContentTypeHeader(String? header) {
    final String? mimeType = header?.split(';').first.trim().toLowerCase();
    return mimeType == null || mimeType.isEmpty ? null : mimeType;
  }

  String _cssColorFrom(Color color) {
    final int alpha = (color.a * 255.0).round().clamp(0, 255);
    final int red = (color.r * 255.0).round().clamp(0, 255);
    final int green = (color.g * 255.0).round().clamp(0, 255);
    final int blue = (color.b * 255.0).round().clamp(0, 255);
    return 'rgba($red, $green, $blue, ${alpha / 255})';
  }
}

enum _NavigationLoadType { none, url, html, xhrResponse }

/// Web implementation of [WebResourceRequest] for XHR-backed loads.
class WebWebResourceRequest extends WebResourceRequest {
  /// Creates a new [WebWebResourceRequest].
  const WebWebResourceRequest._({
    required super.uri,
    this.method,
    this.headers = const <String, String>{},
    this.isForMainFrame,
  });

  /// The HTTP method used for the request, when known.
  final String? method;

  /// The request headers used for the request.
  final Map<String, String> headers;

  /// Whether this request was made for the main frame, when known.
  final bool? isForMainFrame;
}

/// Web implementation of [WebResourceResponse] for Fetch-backed loads.
class WebWebResourceResponse extends WebResourceResponse {
  /// Creates a new [WebWebResourceResponse].
  const WebWebResourceResponse._({
    required super.uri,
    required super.statusCode,
    required super.headers,
    this.mimeType,
    this.reasonPhrase,
  });

  /// The MIME type parsed from the response `content-type` header, when known.
  final String? mimeType;

  /// The HTTP status text reported by Fetch, when available.
  final String? reasonPhrase;
}

/// Web implementation of [PlatformWebViewPermissionRequest].
class WebWebViewPermissionRequest extends PlatformWebViewPermissionRequest {
  WebWebViewPermissionRequest._({
    required super.types,
    required void Function(bool granted) onDecision,
  }) : _decision = _WebWebViewPermissionDecision(onDecision);

  final _WebWebViewPermissionDecision _decision;

  @override
  Future<void> grant() async {
    _decision.decide(true);
  }

  @override
  Future<void> deny() async {
    _decision.decide(false);
  }
}

class _WebWebViewPermissionDecision {
  _WebWebViewPermissionDecision(this._onDecision);

  final void Function(bool granted) _onDecision;
  bool _hasDecision = false;

  void decide(bool granted) {
    if (_hasDecision) {
      return;
    }
    _hasDecision = true;
    _onDecision(granted);
  }
}

/// Web-specific creation parameters for [WebWebViewWidget].
@immutable
class WebWebViewWidgetCreationParams
    extends PlatformWebViewWidgetCreationParams {
  /// Creates a new [WebWebViewWidgetCreationParams].
  const WebWebViewWidgetCreationParams({
    super.key,
    required super.controller,
    super.layoutDirection,
    super.gestureRecognizers,
  });

  /// Creates a [WebWebViewWidgetCreationParams] from generic params.
  WebWebViewWidgetCreationParams.fromPlatformWebViewWidgetCreationParams(
    PlatformWebViewWidgetCreationParams params,
  ) : this(
        key: params.key,
        controller: params.controller,
        layoutDirection: params.layoutDirection,
        gestureRecognizers: params.gestureRecognizers,
      );
}

/// An implementation of [PlatformWebViewWidget] using Flutter for Web API.
class WebWebViewWidget extends PlatformWebViewWidget {
  /// Constructs a [WebWebViewWidget].
  WebWebViewWidget(PlatformWebViewWidgetCreationParams params)
    : super.implementation(
        params is WebWebViewWidgetCreationParams
            ? params
            : WebWebViewWidgetCreationParams.fromPlatformWebViewWidgetCreationParams(
                params,
              ),
      ) {
    final controller = params.controller as WebWebViewController;
    ui_web.platformViewRegistry.registerViewFactory(
      controller._webWebViewParams.iFrame.id,
      (int viewId) => controller._webWebViewParams.iFrame,
    );
  }

  @override
  Widget build(BuildContext context) {
    return HtmlElementView(
      key: params.key,
      viewType: (params.controller as WebWebViewController)
          ._webWebViewParams
          .iFrame
          .id,
    );
  }
}
