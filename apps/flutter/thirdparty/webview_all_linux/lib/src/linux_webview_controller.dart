import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:path/path.dart' as path;
import 'package:webview_platform_interface/webview_platform_interface.dart';

import 'linux_navigation_delegate.dart';
import 'linux_webview_constants.dart';
import 'linux_webview_creation_params.dart';
import 'linux_webview_requests.dart';

part 'linux_webview_events.dart';

String _createLinuxOpaqueIdentifier(String prefix) {
  final Random random = Random.secure();
  final String token = List<String>.generate(
    16,
    (_) => random.nextInt(256).toRadixString(16).padLeft(2, '0'),
    growable: false,
  ).join();
  return '${prefix}_$token';
}

Future<void> _disposeFinalizedLinuxWebView(
  _LinuxWebViewDisposal disposal,
) async {
  try {
    await disposal.dispose();
  } catch (_) {
    // The Flutter engine can already be detached when a finalizer runs.
    return;
  }
}

class LinuxWebViewController extends PlatformWebViewController {
  LinuxWebViewController(PlatformWebViewControllerCreationParams params)
    : super.implementation(
        params is LinuxWebViewControllerCreationParams
            ? params
            : LinuxWebViewControllerCreationParams.fromPlatformWebViewControllerCreationParams(
                params,
              ),
      ) {
    _readyFuture = _initialize(WeakReference<LinuxWebViewController>(this));
  }

  static final Finalizer<_LinuxWebViewDisposal> _finalizer =
      Finalizer<_LinuxWebViewDisposal>((_LinuxWebViewDisposal disposal) {
        unawaited(_disposeFinalizedLinuxWebView(disposal));
      });

  static const MethodChannel rootChannel = MethodChannel(
    linuxWebViewChannelPrefix,
  );
  static final RegExp _javaScriptIdentifierPattern = RegExp(
    r'^[A-Za-z_$][A-Za-z0-9_$]*$',
  );

  Future<void>? _readyFuture;
  MethodChannel? _channel;
  EventChannel? _eventChannel;
  StreamSubscription<dynamic>? _eventSubscription;
  _LinuxWebViewDisposal? _nativeDisposal;

  LinuxNavigationDelegate? _navigationDelegate;
  final Map<String, JavaScriptChannelParams> _javaScriptChannels =
      <String, JavaScriptChannelParams>{};
  final Map<String, Completer<Object?>> _pendingAsyncJavaScriptInvocations =
      <String, Completer<Object?>>{};
  final Set<String> _userScriptIdentifiers = <String>{};

  String? _currentUrl;
  String? _title;
  String? _userAgent;
  bool _canGoBack = false;
  bool _canGoForward = false;
  bool _disposed = false;
  int _frameSequence = 0;
  Future<void>? _disposeFuture;
  int _nextAsyncJavaScriptInvocationIdentifier = 0;
  int _nextUserScriptIdentifier = 0;

  void Function(JavaScriptConsoleMessage consoleMessage)? _onConsoleMessage;
  void Function(ScrollPositionChange scrollPositionChange)?
  _onScrollPositionChange;
  void Function(PlatformWebViewPermissionRequest request)? _onPermissionRequest;
  Future<void> Function(JavaScriptAlertDialogRequest request)?
  _onJavaScriptAlertDialog;
  Future<bool> Function(JavaScriptConfirmDialogRequest request)?
  _onJavaScriptConfirmDialog;
  Future<String> Function(JavaScriptTextInputDialogRequest request)?
  _onJavaScriptTextInputDialog;

  Future<void> _initialize(
    WeakReference<LinuxWebViewController> weakThis,
  ) async {
    final int id =
        await rootChannel.invokeMethod<int>('createWebView') ??
        (throw StateError('Failed to create Linux WebView instance.'));
    _channel = MethodChannel('$linuxWebViewChannelPrefix/$id');
    _eventChannel = EventChannel('$linuxWebViewChannelPrefix/$id/events');
    _eventSubscription = _eventChannel!.receiveBroadcastStream().listen(
      (dynamic event) {
        final LinuxWebViewController? target = weakThis.target;
        if (target == null) {
          return;
        }
        try {
          target._handleEvent(event);
        } catch (error, stackTrace) {
          target._reportEventError('event callback', error, stackTrace);
        }
      },
      onError: (Object error, StackTrace stackTrace) {
        weakThis.target?._reportEventError('event channel', error, stackTrace);
      },
      onDone: () {
        final LinuxWebViewController? target = weakThis.target;
        if (target != null && !target._disposed) {
          target._reportEventError(
            'event channel',
            StateError('The native Linux WebView event channel closed.'),
            StackTrace.current,
          );
        }
      },
    );
    final _LinuxWebViewDisposal disposal = _LinuxWebViewDisposal(
      channel: _channel!,
      eventSubscription: _eventSubscription!,
    );
    _nativeDisposal = disposal;
    _finalizer.attach(this, disposal, detach: this);
    try {
      await _applyCreationParams();
    } catch (_) {
      _finalizer.detach(this);
      await disposal.dispose();
      rethrow;
    }
  }

  LinuxWebViewControllerCreationParams get _linuxParams =>
      params as LinuxWebViewControllerCreationParams;

  Future<void> _applyCreationParams() async {
    final Map<String, Object?> settings = <String, Object?>{
      if (_linuxParams.developerExtrasEnabled case final bool value)
        'developerExtrasEnabled': value,
      if (_linuxParams.downloadsEnabled case final bool value)
        'downloadsEnabled': value,
      if (_linuxParams.javascriptCanOpenWindowsAutomatically
          case final bool value)
        'javascriptCanOpenWindowsAutomatically': value,
      if (_linuxParams.mediaPlaybackRequiresUserGesture case final bool value)
        'mediaPlaybackRequiresUserGesture': value,
      if (_linuxParams.mediaPlaybackAllowsInline case final bool value)
        'mediaPlaybackAllowsInline': value,
      if (_linuxParams.pageCacheEnabled case final bool value)
        'pageCacheEnabled': value,
      if (_linuxParams.allowFileAccessFromFileUrls case final bool value)
        'allowFileAccessFromFileUrls': value,
      if (_linuxParams.allowUniversalAccessFromFileUrls case final bool value)
        'allowUniversalAccessFromFileUrls': value,
      if (_linuxParams.zoomTextOnly case final bool value)
        'zoomTextOnly': value,
      if (_linuxParams.defaultFontSize case final int value)
        'defaultFontSize': value,
      if (_linuxParams.defaultMonospaceFontSize case final int value)
        'defaultMonospaceFontSize': value,
      if (_linuxParams.minimumFontSize case final int value)
        'minimumFontSize': value,
      if (_linuxParams.zoomFactor case final double value) 'zoomFactor': value,
    };

    if (settings.isEmpty) {
      return;
    }

    await _channel!.invokeMethod<void>('applySettings', settings);
  }

  Future<void> _ensureReady() async {
    await _readyFuture;
    if (_disposed) {
      throw StateError(
        'This LinuxWebViewController has already been disposed.',
      );
    }
  }

  Future<T?> _invoke<T>(String method, [Object? arguments]) async {
    await _ensureReady();
    return _channel!.invokeMethod<T>(method, arguments);
  }

  void _dispatchEventHandler(
    String operation,
    Future<void> Function() handler,
  ) {
    unawaited(
      handler().catchError((Object error, StackTrace stackTrace) {
        _reportEventError(operation, error, stackTrace);
      }),
    );
  }

  void _reportEventError(
    String operation,
    Object error,
    StackTrace stackTrace,
  ) {
    final String diagnostic = error.toString().replaceAll(
      RegExp(r'[\r\n]+'),
      ' ',
    );
    debugPrint('webview_all_linux: $operation failed: $diagnostic');
  }

  @override
  Future<void> loadFile(String absoluteFilePath) async {
    if (!path.isAbsolute(absoluteFilePath)) {
      throw ArgumentError.value(
        absoluteFilePath,
        'absoluteFilePath',
        'Path must be absolute.',
      );
    }
    final File file = File(absoluteFilePath);
    if (!file.existsSync()) {
      throw ArgumentError.value(
        absoluteFilePath,
        'absoluteFilePath',
        'File does not exist.',
      );
    }
    final String canonicalPath = file.resolveSymbolicLinksSync();

    await _invoke<void>('loadFile', <String, Object?>{'path': canonicalPath});
  }

  @override
  Future<void> loadFileWithParams(LoadFileParams params) {
    return loadFile(params.absoluteFilePath);
  }

  @override
  Future<void> loadFlutterAsset(String key) async {
    final String assetPath = _resolveFlutterAssetPath(key);
    final File file = File(assetPath);
    if (!file.existsSync()) {
      throw ArgumentError.value(key, 'key', 'Asset for key "$key" not found.');
    }

    final String assetRoot = Directory(
      _flutterAssetsRootPath,
    ).resolveSymbolicLinksSync();
    final String canonicalAssetPath = file.resolveSymbolicLinksSync();
    if (!path.isWithin(assetRoot, canonicalAssetPath)) {
      throw ArgumentError.value(
        key,
        'key',
        'Asset resolves outside the Flutter asset directory.',
      );
    }

    await loadFile(canonicalAssetPath);
  }

  @override
  Future<void> loadHtmlString(String html, {String? baseUrl}) {
    return _invoke<void>('loadHtmlString', <String, Object?>{
      'html': html,
      'baseUrl': baseUrl,
    });
  }

  @override
  Future<void> loadRequest(LoadRequestParams params) async {
    if (!params.uri.hasScheme) {
      throw ArgumentError(
        'LoadRequestParams#uri is required to have a scheme.',
      );
    }

    await _invoke<void>('loadRequest', <String, Object?>{
      'url': params.uri.toString(),
      'method': params.method.serialize(),
      'headers': params.headers,
      'body': params.body,
    });
  }

  @override
  Future<String?> currentUrl() async {
    await _ensureReady();
    return _currentUrl ?? await _channel!.invokeMethod<String>('currentUrl');
  }

  @override
  Future<bool> canGoBack() async {
    await _ensureReady();
    return (await _channel!.invokeMethod<bool>('canGoBack')) ?? _canGoBack;
  }

  @override
  Future<bool> canGoForward() async {
    await _ensureReady();
    return (await _channel!.invokeMethod<bool>('canGoForward')) ??
        _canGoForward;
  }

  @override
  Future<void> goBack() => _invoke<void>('goBack');

  @override
  Future<void> goForward() => _invoke<void>('goForward');

  @override
  Future<void> reload() => _invoke<void>('reload');

  @override
  Future<void> clearCache() => _invoke<void>('clearCache');

  @override
  Future<void> clearLocalStorage() => _invoke<void>('clearLocalStorage');

  @override
  Future<void> setPlatformNavigationDelegate(
    PlatformNavigationDelegate handler,
  ) async {
    _navigationDelegate = handler as LinuxNavigationDelegate;
  }

  @override
  Future<void> runJavaScript(String javaScript) {
    return _invoke<void>('runJavaScript', <String, Object?>{
      'script': javaScript,
    });
  }

  @override
  Future<Object?> runJavaScriptReturningResult(String javaScript) async {
    final Object? result = await _invoke<Object>(
      'runJavaScriptReturningResult',
      <String, Object?>{'script': javaScript},
    );

    if (result case final Map<Object?, Object?> map
        when map['__json__'] is String) {
      return jsonDecode(map['__json__']! as String);
    }

    return result;
  }

  Future<Object?> callAsyncJavaScript(JavaScriptInvocationParams params) async {
    await _ensureReady();
    final String identifier = _createLinuxOpaqueIdentifier(
      'async_${++_nextAsyncJavaScriptInvocationIdentifier}',
    );
    final Completer<Object?> completer = Completer<Object?>();
    _pendingAsyncJavaScriptInvocations[identifier] = completer;

    final Future<Object?> completion = completer.future.timeout(
      params.timeout,
      onTimeout: () {
        _pendingAsyncJavaScriptInvocations.remove(identifier);
        throw TimeoutException(
          'The asynchronous JavaScript invocation timed out.',
          params.timeout,
        );
      },
    );
    try {
      final List<Object?> values = await Future.wait<Object?>(<Future<Object?>>[
        _channel!.invokeMethod<void>('runJavaScript', <String, Object?>{
          'script': _buildAsyncJavaScriptInvocation(identifier, params),
        }),
        completion,
      ], eagerError: true);
      return values[1];
    } catch (error, stackTrace) {
      _pendingAsyncJavaScriptInvocations.remove(identifier);
      if (!completer.isCompleted) {
        completer.completeError(error, stackTrace);
      }
      rethrow;
    }
  }

  Future<bool> isOffscreenWebViewSupported() async => true;

  Future<void> closeOffscreenWebView() => dispose();

  Future<bool> isUserScriptInjectionSupported(
    WebViewUserScriptInjectionTime injectionTime,
  ) async {
    await _ensureReady();
    return injectionTime == WebViewUserScriptInjectionTime.documentStart;
  }

  @override
  Future<String> addUserScript(WebViewUserScript userScript) async {
    final String identifier = 'webview_all_${++_nextUserScriptIdentifier}';
    await _invoke<void>('addUserScript', <String, Object?>{
      'identifier': identifier,
      'source': buildUserScriptSource(
        userScript,
        platformHandlesMainFrameOnly: true,
      ),
      'mainFrameOnly': userScript.forMainFrameOnly,
    });
    _userScriptIdentifiers.add(identifier);
    return identifier;
  }

  @override
  Future<void> removeUserScript(String identifier) async {
    if (!_userScriptIdentifiers.contains(identifier)) {
      return;
    }
    await _invoke<void>('removeUserScript', <String, Object?>{
      'identifier': identifier,
    });
    _userScriptIdentifiers.remove(identifier);
  }

  @override
  Future<void> removeAllUserScripts() async {
    if (_userScriptIdentifiers.isEmpty) {
      return;
    }
    await _invoke<void>('removeAllUserScripts');
    _userScriptIdentifiers.clear();
  }

  @override
  Future<void> addJavaScriptChannel(
    JavaScriptChannelParams javaScriptChannelParams,
  ) async {
    final String name = javaScriptChannelParams.name;
    if (!_javaScriptIdentifierPattern.hasMatch(name)) {
      throw ArgumentError.value(
        name,
        'javaScriptChannelParams.name',
        'JavaScript channel names must be valid JavaScript identifiers.',
      );
    }

    if (_javaScriptChannels.containsKey(name)) {
      throw ArgumentError(
        'A JavaScriptChannel with name `$name` already exists.',
      );
    }

    _javaScriptChannels[name] = javaScriptChannelParams;
    await _invoke<void>('addJavaScriptChannel', <String, Object?>{
      'name': name,
    });
  }

  @override
  Future<void> removeJavaScriptChannel(String javaScriptChannelName) async {
    _javaScriptChannels.remove(javaScriptChannelName);
    await _invoke<void>('removeJavaScriptChannel', <String, Object?>{
      'name': javaScriptChannelName,
    });
  }

  @override
  Future<String?> getTitle() async {
    await _ensureReady();
    return _title ?? await _channel!.invokeMethod<String>('getTitle');
  }

  @override
  Future<void> scrollTo(int x, int y) {
    return _invoke<void>('scrollTo', <String, Object?>{'x': x, 'y': y});
  }

  @override
  Future<void> scrollBy(int x, int y) {
    return _invoke<void>('scrollBy', <String, Object?>{'x': x, 'y': y});
  }

  @override
  Future<void> setVerticalScrollBarEnabled(bool enabled) {
    return _invoke<void>('setVerticalScrollBarEnabled', <String, Object?>{
      'enabled': enabled,
    });
  }

  @override
  Future<void> setHorizontalScrollBarEnabled(bool enabled) {
    return _invoke<void>('setHorizontalScrollBarEnabled', <String, Object?>{
      'enabled': enabled,
    });
  }

  @override
  bool supportsSetScrollBarsEnabled() => true;

  @override
  Future<Offset> getScrollPosition() async {
    final Map<Object?, Object?>? offset = await _invoke<Map<Object?, Object?>>(
      'getScrollPosition',
    );
    return Offset(
      (offset?['x'] as num?)?.toDouble() ?? 0,
      (offset?['y'] as num?)?.toDouble() ?? 0,
    );
  }

  @override
  Future<void> enableZoom(bool enabled) {
    return _invoke<void>('enableZoom', <String, Object?>{'enabled': enabled});
  }

  @override
  Future<void> setBackgroundColor(Color color) {
    return _invoke<void>('setBackgroundColor', <String, Object?>{
      'r': color.r,
      'g': color.g,
      'b': color.b,
      'a': color.a,
    });
  }

  @override
  Future<void> setJavaScriptMode(JavaScriptMode javaScriptMode) {
    return _invoke<void>('setJavaScriptMode', <String, Object?>{
      'enabled': javaScriptMode == JavaScriptMode.unrestricted,
    });
  }

  @override
  Future<void> setUserAgent(String? userAgent) async {
    _userAgent = userAgent;
    await _invoke<void>('setUserAgent', <String, Object?>{
      'userAgent': userAgent,
    });
  }

  @override
  Future<String?> getUserAgent() async {
    await _ensureReady();
    return _userAgent ?? await _channel!.invokeMethod<String>('getUserAgent');
  }

  @override
  Future<void> setOnPlatformPermissionRequest(
    void Function(PlatformWebViewPermissionRequest request) onPermissionRequest,
  ) async {
    _onPermissionRequest = onPermissionRequest;
  }

  @override
  Future<void> setOnConsoleMessage(
    void Function(JavaScriptConsoleMessage consoleMessage) onConsoleMessage,
  ) async {
    _onConsoleMessage = onConsoleMessage;
    await _invoke<void>('setOnConsoleMessage', <String, Object?>{
      'enabled': true,
    });
  }

  @override
  Future<void> setOnScrollPositionChange(
    void Function(ScrollPositionChange scrollPositionChange)?
    onScrollPositionChange,
  ) async {
    _onScrollPositionChange = onScrollPositionChange;
    await _invoke<void>('setOnScrollPositionChange', <String, Object?>{
      'enabled': onScrollPositionChange != null,
    });
  }

  @override
  Future<void> setOnJavaScriptAlertDialog(
    Future<void> Function(JavaScriptAlertDialogRequest request)
    onJavaScriptAlertDialog,
  ) async {
    _onJavaScriptAlertDialog = onJavaScriptAlertDialog;
    await _updateJavaScriptDialogCallbacksEnabled();
  }

  @override
  Future<void> setOnJavaScriptConfirmDialog(
    Future<bool> Function(JavaScriptConfirmDialogRequest request)
    onJavaScriptConfirmDialog,
  ) async {
    _onJavaScriptConfirmDialog = onJavaScriptConfirmDialog;
    await _updateJavaScriptDialogCallbacksEnabled();
  }

  @override
  Future<void> setOnJavaScriptTextInputDialog(
    Future<String> Function(JavaScriptTextInputDialogRequest request)
    onJavaScriptTextInputDialog,
  ) async {
    _onJavaScriptTextInputDialog = onJavaScriptTextInputDialog;
    await _updateJavaScriptDialogCallbacksEnabled();
  }

  Future<void> _updateJavaScriptDialogCallbacksEnabled() {
    return _invoke<void>(
      'setJavaScriptDialogCallbacksEnabled',
      <String, Object?>{
        'alert': _onJavaScriptAlertDialog != null,
        'confirm': _onJavaScriptConfirmDialog != null,
        'prompt': _onJavaScriptTextInputDialog != null,
      },
    );
  }

  @override
  Future<void> setOverScrollMode(WebViewOverScrollMode mode) async {
    await _invoke<void>('setOverScrollMode', <String, Object?>{
      'mode': switch (mode) {
        WebViewOverScrollMode.always => 'always',
        WebViewOverScrollMode.ifContentScrolls => 'ifContentScrolls',
        WebViewOverScrollMode.never => 'never',
      },
    });
  }

  Future<void> setFrame(Rect rect, {Rect? clipRect, required bool visible}) {
    final Rect effectiveClip = clipRect ?? rect;
    final int sequence = ++_frameSequence;
    return _invoke<void>('setFrame', <String, Object?>{
      'x': rect.left,
      'y': rect.top,
      'width': rect.width,
      'height': rect.height,
      'clipX': effectiveClip.left,
      'clipY': effectiveClip.top,
      'clipWidth': effectiveClip.width,
      'clipHeight': effectiveClip.height,
      'visible': visible,
      'sequence': sequence,
    });
  }

  /// Enables WebKitGTK developer extras for this WebView.
  Future<void> setDeveloperExtrasEnabled(bool enabled) {
    return _invoke<void>('setDeveloperExtrasEnabled', <String, Object?>{
      'enabled': enabled,
    });
  }

  /// Opens the WebKitGTK web inspector for this WebView.
  Future<void> openDevTools() => _invoke<void>('openDevTools');

  /// Sets whether this WebView can start downloads. Enabled by default.
  ///
  /// When disabled, new downloads are cancelled before saving a file. Downloads
  /// already in progress are not cancelled. Other WebViews are not affected.
  Future<void> setDownloadsEnabled(bool enabled) {
    return _invoke<void>('setDownloadsEnabled', <String, Object?>{
      'enabled': enabled,
    });
  }

  /// Sets whether JavaScript may open windows automatically.
  Future<void> setJavaScriptCanOpenWindowsAutomatically(bool enabled) {
    return _invoke<void>(
      'setJavaScriptCanOpenWindowsAutomatically',
      <String, Object?>{'enabled': enabled},
    );
  }

  /// Sets whether media playback requires a user gesture.
  Future<void> setMediaPlaybackRequiresUserGesture(bool require) {
    return _invoke<void>(
      'setMediaPlaybackRequiresUserGesture',
      <String, Object?>{'require': require},
    );
  }

  /// Sets whether inline media playback is allowed.
  Future<void> setMediaPlaybackAllowsInline(bool allow) {
    return _invoke<void>('setMediaPlaybackAllowsInline', <String, Object?>{
      'allow': allow,
    });
  }

  /// Enables or disables WebKitGTK's page cache.
  Future<void> setPageCacheEnabled(bool enabled) {
    return _invoke<void>('setPageCacheEnabled', <String, Object?>{
      'enabled': enabled,
    });
  }

  /// Sets whether file URLs can read other file URLs.
  Future<void> setAllowFileAccessFromFileUrls(bool allow) {
    return _invoke<void>('setAllowFileAccessFromFileUrls', <String, Object?>{
      'allow': allow,
    });
  }

  /// Sets whether file URLs can access all origins.
  Future<void> setAllowUniversalAccessFromFileUrls(bool allow) {
    return _invoke<void>(
      'setAllowUniversalAccessFromFileUrls',
      <String, Object?>{'allow': allow},
    );
  }

  /// Sets whether zooming affects only text.
  Future<void> setZoomTextOnly(bool enabled) {
    return _invoke<void>('setZoomTextOnly', <String, Object?>{
      'enabled': enabled,
    });
  }

  /// Sets the default proportional font size in CSS pixels.
  Future<void> setDefaultFontSize(int fontSize) {
    return _invoke<void>('setDefaultFontSize', <String, Object?>{
      'fontSize': fontSize,
    });
  }

  /// Sets the default monospace font size in CSS pixels.
  Future<void> setDefaultMonospaceFontSize(int fontSize) {
    return _invoke<void>('setDefaultMonospaceFontSize', <String, Object?>{
      'fontSize': fontSize,
    });
  }

  /// Sets the minimum font size in CSS pixels.
  Future<void> setMinimumFontSize(int fontSize) {
    return _invoke<void>('setMinimumFontSize', <String, Object?>{
      'fontSize': fontSize,
    });
  }

  late final NativeWebViewZoom _nativeZoom = NativeWebViewZoom(
    initialPageZoomFactor: _linuxParams.zoomFactor ?? 1,
    applyZoom: (double factor) =>
        _invoke<void>('setZoomFactor', <String, Object?>{'zoomFactor': factor}),
  );

  // GTK overlays cannot represent Flutter paint scaling. The common wrapper
  // supplies a translation-only frame and WebKitGTK scales the page content.
  @override
  bool get requiresNativeApplicationZoom => true;

  @override
  Future<void> setApplicationZoomFactor(double zoomFactor) =>
      _nativeZoom.setApplicationZoomFactor(zoomFactor);

  /// Sets page zoom without replacing the application's zoom factor.
  @override
  Future<void> setZoomFactor(double zoomFactor) =>
      _nativeZoom.setPageZoomFactor(zoomFactor);

  Future<void> dispose() {
    return _disposeFuture ??= _dispose();
  }

  Future<void> _dispose() async {
    _disposed = true;
    _cancelPendingAsyncJavaScriptInvocations(
      const JavaScriptExecutionException(
        name: 'AbortError',
        message:
            'The WebView controller was disposed before JavaScript completed.',
      ),
    );
    _finalizer.detach(this);
    Object? initializationError;
    StackTrace? initializationStackTrace;
    try {
      await _readyFuture;
    } catch (error, stackTrace) {
      initializationError = error;
      initializationStackTrace = stackTrace;
    }
    await _nativeDisposal?.dispose();
    _eventSubscription = null;
    _javaScriptChannels.clear();
    _userScriptIdentifiers.clear();
    _navigationDelegate = null;
    _onConsoleMessage = null;
    _onScrollPositionChange = null;
    _onPermissionRequest = null;
    _onJavaScriptAlertDialog = null;
    _onJavaScriptConfirmDialog = null;
    _onJavaScriptTextInputDialog = null;
    if (initializationError != null) {
      Error.throwWithStackTrace(
        initializationError,
        initializationStackTrace ?? StackTrace.current,
      );
    }
  }

  void _cancelPendingAsyncJavaScriptInvocations(Object error) {
    final List<Completer<Object?>> pending = _pendingAsyncJavaScriptInvocations
        .values
        .toList(growable: false);
    _pendingAsyncJavaScriptInvocations.clear();
    for (final Completer<Object?> completer in pending) {
      if (!completer.isCompleted) {
        completer.completeError(error);
      }
    }
  }

  void _handleAsyncJavaScriptMessage(String message) {
    Object? decoded;
    try {
      decoded = jsonDecode(message);
    } on FormatException {
      return;
    }
    if (decoded is! Map<String, dynamic>) {
      return;
    }
    final Object? rawIdentifier = decoded['identifier'];
    if (rawIdentifier is! String) {
      return;
    }
    final String identifier = rawIdentifier;
    final Completer<Object?>? completer = _pendingAsyncJavaScriptInvocations
        .remove(identifier);
    if (completer == null || completer.isCompleted) {
      return;
    }
    if (decoded['success'] == true) {
      completer.complete(decoded['value']);
      return;
    }
    completer.completeError(
      JavaScriptExecutionException(
        name: decoded['name']?.toString(),
        message:
            decoded['message']?.toString() ?? 'JavaScript execution failed.',
        javaScriptStackTrace: decoded['stack']?.toString(),
      ),
    );
  }

  String _buildAsyncJavaScriptInvocation(
    String identifier,
    JavaScriptInvocationParams params,
  ) {
    final String argumentNames = params.arguments.keys.join(',');
    final String argumentValues = params.arguments.keys
        .map((String name) => '__arguments[${jsonEncode(name)}]')
        .join(',');
    return '''
(()=>{
  const __identifier=${jsonEncode(identifier)};
  const __arguments=${jsonEncode(params.arguments)};
  const __post=(payload)=>window.webkit.messageHandlers.__webview_all_async_javascript.postMessage(JSON.stringify(payload));
  const __postError=(error)=>__post({
    identifier:__identifier,
    success:false,
    name:error&&error.name?String(error.name):null,
    message:error&&error.message?String(error.message):String(error),
    stack:error&&error.stack?String(error.stack):null
  });
  Promise.resolve()
    .then(()=>(async function($argumentNames){
${params.functionBody}
    })($argumentValues))
    .then((value)=>{
      try {
        const encoded=JSON.stringify(value);
        __post({identifier:__identifier,success:true,value:encoded===undefined?null:JSON.parse(encoded)});
      } catch(error) {
        __postError(error);
      }
    },__postError);
})();
''';
  }

  String get _flutterAssetsRootPath {
    return path.joinAll(<String>[
      path.dirname(Platform.resolvedExecutable),
      'data',
      'flutter_assets',
    ]);
  }

  String _resolveFlutterAssetPath(String key) {
    final List<String> segments = key.split('/');
    if (key.isEmpty ||
        key.startsWith('/') ||
        key.contains(r'\') ||
        segments.any(
          (String segment) =>
              segment.isEmpty || segment == '.' || segment == '..',
        )) {
      throw ArgumentError.value(
        key,
        'key',
        'Asset key must be a normalized relative path.',
      );
    }
    return path.joinAll(<String>[_flutterAssetsRootPath, ...segments]);
  }
}

class _LinuxWebViewDisposal {
  _LinuxWebViewDisposal({
    required this.channel,
    required this.eventSubscription,
  });

  final MethodChannel channel;
  final StreamSubscription<dynamic> eventSubscription;
  Future<void>? _disposeFuture;

  Future<void> dispose() {
    return _disposeFuture ??= _dispose();
  }

  Future<void> _dispose() async {
    Object? disposalError;
    StackTrace? disposalStackTrace;
    try {
      await eventSubscription.cancel();
    } catch (error, stackTrace) {
      disposalError = error;
      disposalStackTrace = stackTrace;
    }
    try {
      await channel.invokeMethod<void>('dispose');
    } catch (error, stackTrace) {
      disposalError ??= error;
      disposalStackTrace ??= stackTrace;
    }
    if (disposalError != null) {
      Error.throwWithStackTrace(
        disposalError,
        disposalStackTrace ?? StackTrace.current,
      );
    }
  }
}
