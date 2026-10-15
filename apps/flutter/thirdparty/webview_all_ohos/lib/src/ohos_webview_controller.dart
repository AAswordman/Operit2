// Copyright 2013 The Flutter Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// The OHOS Flutter SDK exposes these platform view APIs, while upstream
// Flutter does not.
// ignore_for_file: cast_to_non_type, undefined_class, undefined_method

import 'dart:async';
import 'dart:convert';
import 'dart:math';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:webview_platform_interface/webview_platform_interface.dart';

import 'ohos_webview_proxy.dart';
import 'ohos_webview_native.dart' as ohos_webview;
import 'channels/api_implementations.dart';
import 'channels/constants.dart';
import 'core/instance_manager.dart';
import 'ohos_platform_views.dart';
import 'core/weak_reference.dart';

final Expando<Set<PageEventCallback>> _ohosControllerPageStartedCallbacks =
    Expando<Set<PageEventCallback>>();
final Expando<Set<PageEventCallback>> _ohosControllerPageFinishedCallbacks =
    Expando<Set<PageEventCallback>>();

String _createOhosOpaqueIdentifier(String prefix) {
  final Random random = Random.secure();
  final String token = List<String>.generate(
    16,
    (_) => random.nextInt(256).toRadixString(16).padLeft(2, '0'),
    growable: false,
  ).join();
  return '${prefix}_$token';
}

void _reportOhosCallbackError(String callbackName, Object error) {
  debugPrint(
    'webview_all_ohos: $callbackName failed: '
    '${error.toString().replaceAll(RegExp(r'[\r\n]+'), ' ')}',
  );
}

void _runOhosAsyncCallbackSafely(
  String callbackName,
  Future<void> Function() callback,
) {
  Future<void> future;
  try {
    future = callback();
  } catch (error) {
    _reportOhosCallbackError(callbackName, error);
    return;
  }
  unawaited(
    future.then<void>(
      (_) {},
      onError: (Object error, StackTrace stackTrace) {
        _reportOhosCallbackError(callbackName, error);
      },
    ),
  );
}

/// Object specifying creation parameters for creating a [OhosWebViewController].
///
/// When adding additional fields make sure they can be null or have a default
/// value to avoid breaking changes. See [PlatformWebViewControllerCreationParams] for
/// more information.
@immutable
class OhosWebViewControllerCreationParams
    extends PlatformWebViewControllerCreationParams {
  /// Creates a new [OhosWebViewControllerCreationParams] instance.
  OhosWebViewControllerCreationParams({
    this.isAllowFullScreenRotate = false,
    this.domStorageEnabled = true,
    this.javaScriptCanOpenWindowsAutomatically = true,
    this.supportMultipleWindows = true,
    this.loadWithOverviewMode = true,
    this.useWideViewPort = true,
    this.displayZoomControls = false,
    this.builtInZoomControls = true,
    this.allowFileAccess,
    this.mediaPlaybackRequiresUserGesture,
    this.supportZoom,
    this.textZoom,
    @visibleForTesting this.ohosWebViewProxy = const OhosWebViewProxy(),
    @visibleForTesting ohos_webview.WebStorage? ohosWebStorage,
  }) : ohosWebStorage = ohosWebStorage ?? ohos_webview.WebStorage.instance,
       super();

  /// Creates a [OhosWebViewControllerCreationParams] instance based on [PlatformWebViewControllerCreationParams].
  factory OhosWebViewControllerCreationParams.fromPlatformWebViewControllerCreationParams(
    // Recommended placeholder to prevent being broken by platform interface.
    // ignore: avoid_unused_constructor_parameters
    PlatformWebViewControllerCreationParams params, {
    bool? isAllowFullScreenRotate = false,
    bool domStorageEnabled = true,
    bool javaScriptCanOpenWindowsAutomatically = true,
    bool supportMultipleWindows = true,
    bool loadWithOverviewMode = true,
    bool useWideViewPort = true,
    bool displayZoomControls = false,
    bool builtInZoomControls = true,
    bool? allowFileAccess,
    bool? mediaPlaybackRequiresUserGesture,
    bool? supportZoom,
    int? textZoom,
    @visibleForTesting
    OhosWebViewProxy ohosWebViewProxy = const OhosWebViewProxy(),
    @visibleForTesting ohos_webview.WebStorage? ohosWebStorage,
  }) {
    return OhosWebViewControllerCreationParams(
      isAllowFullScreenRotate: isAllowFullScreenRotate,
      domStorageEnabled: domStorageEnabled,
      javaScriptCanOpenWindowsAutomatically:
          javaScriptCanOpenWindowsAutomatically,
      supportMultipleWindows: supportMultipleWindows,
      loadWithOverviewMode: loadWithOverviewMode,
      useWideViewPort: useWideViewPort,
      displayZoomControls: displayZoomControls,
      builtInZoomControls: builtInZoomControls,
      allowFileAccess: allowFileAccess,
      mediaPlaybackRequiresUserGesture: mediaPlaybackRequiresUserGesture,
      supportZoom: supportZoom,
      textZoom: textZoom,
      ohosWebViewProxy: ohosWebViewProxy,
      ohosWebStorage: ohosWebStorage ?? ohos_webview.WebStorage.instance,
    );
  }

  /// Enables or disables full screen rotate within WebView.
  final bool? isAllowFullScreenRotate;

  /// Whether DOM storage is enabled.
  final bool domStorageEnabled;

  /// Whether JavaScript may open windows automatically.
  final bool javaScriptCanOpenWindowsAutomatically;

  /// Whether the WebView should support multiple windows.
  final bool supportMultipleWindows;

  /// Whether pages are loaded in overview mode.
  final bool loadWithOverviewMode;

  /// Whether wide viewport support is enabled.
  final bool useWideViewPort;

  /// Whether on-screen zoom controls are displayed.
  final bool displayZoomControls;

  /// Whether built-in zoom mechanisms are enabled.
  final bool builtInZoomControls;

  /// Whether file access is enabled.
  ///
  /// When null, the platform default is left unchanged.
  final bool? allowFileAccess;

  /// Whether media playback requires a user gesture.
  ///
  /// When null, the platform default is left unchanged.
  final bool? mediaPlaybackRequiresUserGesture;

  /// Whether zoom support is enabled.
  ///
  /// When null, the platform default is left unchanged.
  final bool? supportZoom;

  /// Initial text zoom in percent.
  ///
  /// When null, the platform default is left unchanged.
  final int? textZoom;

  /// Handles constructing objects and calling static methods for the Ohos WebView
  /// native library.
  @visibleForTesting
  final OhosWebViewProxy ohosWebViewProxy;

  /// Manages the JavaScript storage APIs provided by the [ohos_webview.WebView].
  @visibleForTesting
  final ohos_webview.WebStorage ohosWebStorage;
}

/// Ohos-specific resources that can require permissions.
class OhosWebViewPermissionResourceType extends WebViewPermissionResourceType {
  const OhosWebViewPermissionResourceType._(super.name);

  /// A resource that will allow sysex messages to be sent to or received from
  /// MIDI devices.
  static const OhosWebViewPermissionResourceType midiSysex =
      OhosWebViewPermissionResourceType._('midiSysex');

  /// A resource that belongs to a protected media identifier.
  static const OhosWebViewPermissionResourceType protectedMediaId =
      OhosWebViewPermissionResourceType._('protectedMediaId');
}

/// Implementation of the [PlatformWebViewController] with the Ohos WebView API.
class OhosWebViewController extends PlatformWebViewController {
  /// Creates a new [OhosWebViewController].
  OhosWebViewController(PlatformWebViewControllerCreationParams params)
    : super.implementation(
        params is OhosWebViewControllerCreationParams
            ? params
            : OhosWebViewControllerCreationParams.fromPlatformWebViewControllerCreationParams(
                params,
              ),
      ) {
    final OhosWebViewControllerCreationParams creationParams =
        _ohosWebViewParams;
    _webView.settings.setAllowFullScreenRotate(
      creationParams.isAllowFullScreenRotate ?? false,
    );
    _webView.settings.setDomStorageEnabled(creationParams.domStorageEnabled);
    _webView.settings.setJavaScriptCanOpenWindowsAutomatically(
      creationParams.javaScriptCanOpenWindowsAutomatically,
    );
    _webView.settings.setSupportMultipleWindows(
      creationParams.supportMultipleWindows,
    );
    _webView.settings.setLoadWithOverviewMode(
      creationParams.loadWithOverviewMode,
    );
    _webView.settings.setUseWideViewPort(creationParams.useWideViewPort);
    _webView.settings.setDisplayZoomControls(
      creationParams.displayZoomControls,
    );
    _webView.settings.setBuiltInZoomControls(
      creationParams.builtInZoomControls,
    );
    if (creationParams.allowFileAccess case final bool enabled) {
      _webView.settings.setAllowFileAccess(enabled);
    }
    if (creationParams.mediaPlaybackRequiresUserGesture
        case final bool require) {
      _webView.settings.setMediaPlaybackRequiresUserGesture(require);
    }
    if (creationParams.supportZoom case final bool enabled) {
      _webView.settings.setSupportZoom(enabled);
    }
    if (creationParams.textZoom case final int textZoom) {
      _webView.settings.setTextZoom(textZoom);
    }

    _webView.setWebChromeClient(_webChromeClient);
  }

  OhosWebViewControllerCreationParams get _ohosWebViewParams =>
      params as OhosWebViewControllerCreationParams;

  /// The native [ohos_webview.WebView] being controlled.
  late final ohos_webview.WebView _webView = _ohosWebViewParams.ohosWebViewProxy
      .createOhosWebView(
        onScrollChanged: withWeakReferenceTo(this, (
          WeakReference<OhosWebViewController> weakReference,
        ) {
          return (int left, int top, int oldLeft, int oldTop) {
            final void Function(ScrollPositionChange)? callback =
                weakReference.target?._onScrollPositionChangedCallback;
            callback?.call(
              ScrollPositionChange(left.toDouble(), top.toDouble()),
            );
          };
        }),
      );

  late final ohos_webview.WebChromeClient
  _webChromeClient = _ohosWebViewParams.ohosWebViewProxy.createOhosWebChromeClient(
    onProgressChanged: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (ohos_webview.WebView webView, int progress) {
        if (weakReference.target?._currentNavigationDelegate?._onProgress !=
            null) {
          weakReference.target!._currentNavigationDelegate!._onProgress!(
            progress,
          );
        }
      };
    }),
    onGeolocationPermissionsShowPrompt: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (
        String origin,
        ohos_webview.GeolocationPermissionsCallback callback,
      ) async {
        final OnGeolocationPermissionsShowPrompt? onShowPrompt =
            weakReference.target?._onGeolocationPermissionsShowPrompt;
        if (onShowPrompt != null) {
          final GeolocationPermissionsResponse response = await onShowPrompt(
            GeolocationPermissionsRequestParams(origin: origin),
          );
          return callback.invoke(origin, response.allow, response.retain);
        } else {
          // default don't allow
          return callback.invoke(origin, false, false);
        }
      };
    }),
    onGeolocationPermissionsHidePrompt: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (ohos_webview.WebChromeClient instance) {
        final OnGeolocationPermissionsHidePrompt? onHidePrompt =
            weakReference.target?._onGeolocationPermissionsHidePrompt;
        if (onHidePrompt != null) {
          onHidePrompt();
        }
      };
    }),
    onShowCustomView: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (
        _,
        ohos_webview.View view,
        ohos_webview.CustomViewCallback callback,
      ) {
        final OhosWebViewController? webViewController = weakReference.target;
        if (webViewController == null) {
          _runOhosAsyncCallbackSafely(
            'custom view fallback',
            callback.onCustomViewHidden,
          );
          return;
        }
        final OnShowCustomWidgetCallback? onShowCallback =
            webViewController._onShowCustomWidgetCallback;
        if (onShowCallback == null) {
          _runOhosAsyncCallbackSafely(
            'custom view fallback',
            callback.onCustomViewHidden,
          );
          return;
        }
        onShowCallback(
          OhosCustomViewWidget.private(
            controller: webViewController,
            customView: view,
          ),
          () => _runOhosAsyncCallbackSafely(
            'custom view dismissal',
            callback.onCustomViewHidden,
          ),
        );
      };
    }),
    onHideCustomView: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (ohos_webview.WebChromeClient instance) {
        final OnHideCustomWidgetCallback? onHideCustomViewCallback =
            weakReference.target?._onHideCustomWidgetCallback;
        if (onHideCustomViewCallback != null) {
          onHideCustomViewCallback();
        }
      };
    }),
    onShowFileChooser: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (
        ohos_webview.WebView webView,
        ohos_webview.FileChooserParams params,
      ) async {
        if (weakReference.target?._onShowFileSelectorCallback != null) {
          return weakReference.target!._onShowFileSelectorCallback!(
            FileSelectorParams._fromFileChooserParams(params),
          );
        }
        return <String>[];
      };
    }),
    onConsoleMessage: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (
        ohos_webview.WebChromeClient webChromeClient,
        ohos_webview.ConsoleMessage consoleMessage,
      ) {
        final void Function(JavaScriptConsoleMessage)? callback =
            weakReference.target?._onConsoleLogCallback;
        if (callback != null) {
          JavaScriptLogLevel logLevel;
          switch (consoleMessage.level) {
            // OHOS maps `console.debug` to `MessageLevel.TIP` on current engines.
            // `MessageLevel.DEBUG` if not being used.
            case ConsoleMessageLevel.debug:
            case ConsoleMessageLevel.tip:
              logLevel = JavaScriptLogLevel.debug;
              break;
            case ConsoleMessageLevel.error:
              logLevel = JavaScriptLogLevel.error;
              break;
            case ConsoleMessageLevel.warning:
              logLevel = JavaScriptLogLevel.warning;
              break;
            case ConsoleMessageLevel.unknown:
            case ConsoleMessageLevel.log:
              logLevel = JavaScriptLogLevel.log;
              break;
          }

          callback(
            JavaScriptConsoleMessage(
              level: logLevel,
              message: consoleMessage.message,
            ),
          );
        }
      };
    }),
    onPermissionRequest: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (_, ohos_webview.PermissionRequest request) {
        final void Function(PlatformWebViewPermissionRequest)? callback =
            weakReference.target?._onPermissionRequestCallback;
        if (callback == null) {
          _runOhosAsyncCallbackSafely(
            'permission request fallback',
            request.deny,
          );
          return;
        } else {
          final Set<WebViewPermissionResourceType> types = request.resources
              .map<WebViewPermissionResourceType?>((String type) {
                switch (type) {
                  case ohos_webview.PermissionRequest.videoCapture:
                    return WebViewPermissionResourceType.camera;
                  case ohos_webview.PermissionRequest.audioCapture:
                    return WebViewPermissionResourceType.microphone;
                  case ohos_webview.PermissionRequest.midiSysex:
                    return OhosWebViewPermissionResourceType.midiSysex;
                  case ohos_webview.PermissionRequest.protectedMediaId:
                    return OhosWebViewPermissionResourceType.protectedMediaId;
                }

                // Type not supported.
                return null;
              })
              .whereType<WebViewPermissionResourceType>()
              .toSet();

          // If the request didn't contain any permissions recognized by the
          // implementation, deny by default.
          if (types.isEmpty) {
            _runOhosAsyncCallbackSafely(
              'permission request fallback',
              request.deny,
            );
            return;
          }

          callback(
            OhosWebViewPermissionRequest._(types: types, request: request),
          );
        }
      };
    }),
    onJsAlert: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (String url, String message) async {
        final Future<void> Function(JavaScriptAlertDialogRequest)? callback =
            weakReference.target?._onJavaScriptAlert;
        if (callback != null) {
          final JavaScriptAlertDialogRequest request =
              JavaScriptAlertDialogRequest(message: message, url: url);

          await callback.call(request);
        }
        return;
      };
    }),
    onJsConfirm: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (String url, String message) async {
        final Future<bool> Function(JavaScriptConfirmDialogRequest)? callback =
            weakReference.target?._onJavaScriptConfirm;
        if (callback != null) {
          final JavaScriptConfirmDialogRequest request =
              JavaScriptConfirmDialogRequest(message: message, url: url);
          final bool result = await callback.call(request);
          return result;
        }
        return false;
      };
    }),
    onJsPrompt: withWeakReferenceTo(this, (
      WeakReference<OhosWebViewController> weakReference,
    ) {
      return (String url, String message, String defaultValue) async {
        final Future<String> Function(JavaScriptTextInputDialogRequest)?
        callback = weakReference.target?._onJavaScriptPrompt;
        if (callback != null) {
          final JavaScriptTextInputDialogRequest request =
              JavaScriptTextInputDialogRequest(
                message: message,
                url: url,
                defaultText: defaultValue,
              );
          final String result = await callback.call(request);
          return result;
        }
        return defaultValue;
      };
    }),
  );

  /// The native [ohos_webview.FlutterAssetManager] allows managing assets.
  late final ohos_webview.FlutterAssetManager _flutterAssetManager =
      _ohosWebViewParams.ohosWebViewProxy.createFlutterAssetManager();

  final Map<String, OhosJavaScriptChannelParams> _javaScriptChannelParams =
      <String, OhosJavaScriptChannelParams>{};

  OhosNavigationDelegate? _currentNavigationDelegate;

  Future<List<String>> Function(FileSelectorParams)?
  _onShowFileSelectorCallback;

  OnGeolocationPermissionsShowPrompt? _onGeolocationPermissionsShowPrompt;

  OnGeolocationPermissionsHidePrompt? _onGeolocationPermissionsHidePrompt;

  OnShowCustomWidgetCallback? _onShowCustomWidgetCallback;

  OnHideCustomWidgetCallback? _onHideCustomWidgetCallback;

  void Function(PlatformWebViewPermissionRequest)? _onPermissionRequestCallback;

  void Function(JavaScriptConsoleMessage consoleMessage)? _onConsoleLogCallback;

  Future<void> Function(JavaScriptAlertDialogRequest request)?
  _onJavaScriptAlert;
  Future<bool> Function(JavaScriptConfirmDialogRequest request)?
  _onJavaScriptConfirm;
  Future<String> Function(JavaScriptTextInputDialogRequest request)?
  _onJavaScriptPrompt;

  void Function(ScrollPositionChange scrollPositionChange)?
  _onScrollPositionChangedCallback;

  bool _verticalScrollBarEnabled = true;
  bool _horizontalScrollBarEnabled = true;
  WebViewOverScrollMode _overScrollMode = WebViewOverScrollMode.always;
  int _nextAsyncJavaScriptInvocationIdentifier = 0;
  int _navigationGeneration = 0;
  late final PageEventCallback _controllerPageStartedCallback =
      withWeakReferenceTo(this, (
        WeakReference<OhosWebViewController> weakReference,
      ) {
        return (_) {
          final OhosWebViewController? controller = weakReference.target;
          if (controller != null) {
            controller._navigationGeneration += 1;
          }
        };
      });
  late final PageEventCallback _controllerPageFinishedCallback =
      withWeakReferenceTo(this, (
        WeakReference<OhosWebViewController> weakReference,
      ) {
        return (_) {
          final OhosWebViewController? controller = weakReference.target;
          if (controller != null) {
            _runOhosAsyncCallbackSafely(
              'viewport style update',
              controller._applyViewportStyle,
            );
          }
        };
      });

  static const String _viewportStyleElementId =
      '__webview_all_ohos_viewport_style';

  /// Whether to enable the platform's webview content debugging tools.
  ///
  /// Defaults to false.
  static Future<void> enableDebugging(
    bool enabled, {
    @visibleForTesting OhosWebViewProxy webViewProxy = const OhosWebViewProxy(),
  }) {
    return webViewProxy.setWebContentsDebuggingEnabled(enabled);
  }

  /// Identifier used to retrieve the underlying native `WebView`.
  ///
  /// This is typically used by other plugins to retrieve the native `WebView`
  /// from an `InstanceManager`.
  int get webViewIdentifier =>
      // ignore: invalid_use_of_visible_for_testing_member
      ohos_webview.WebView.api.instanceManager.getIdentifier(_webView)!;

  @override
  String get localResourceScheme => 'https';

  @override
  Future<void> setLocalResourceHandler(WebViewLocalResourceHandler? handler) =>
      WebViewLocalResourceBridge.setHandler(webViewIdentifier, handler);

  @override
  Future<void> loadFile(String absoluteFilePath) async {
    final String url = absoluteFilePath.startsWith('file://')
        ? absoluteFilePath
        : Uri.file(absoluteFilePath).toString();

    // File access must be enabled before navigation starts. Keeping the two
    // channel calls ordered also ensures that a settings failure is surfaced
    // instead of racing an apparently successful load.
    await _webView.settings.setAllowFileAccess(true);
    _navigationGeneration += 1;
    await _webView.loadUrl(url, <String, String>{});
  }

  @override
  Future<void> loadFileWithParams(LoadFileParams params) {
    return loadFile(params.absoluteFilePath);
  }

  @override
  Future<void> loadFlutterAsset(String key) async {
    final String assetFilePath = await _flutterAssetManager
        .getAssetFilePathByName(key);
    final List<String> pathElements = assetFilePath.split('/');
    final String fileName = pathElements.removeLast();
    final List<String?> paths = await _flutterAssetManager.list(
      pathElements.join('/'),
    );

    if (!paths.contains(fileName)) {
      throw ArgumentError('Asset for key "$key" not found.', 'key');
    }
    await _webView.settings.setAllowFileAccess(true);
    final String url = 'resources/rawfile/$assetFilePath';
    _navigationGeneration += 1;
    return _webView.loadUrl(url, <String, String>{});
  }

  @override
  Future<void> loadHtmlString(String html, {String? baseUrl}) {
    _navigationGeneration += 1;
    return _webView.loadDataWithBaseUrl(
      baseUrl: baseUrl,
      data: html,
      mimeType: 'text/html',
      encoding: 'UTF-8',
    );
  }

  @override
  Future<void> loadRequest(LoadRequestParams params) {
    if (!params.uri.hasScheme) {
      throw ArgumentError('WebViewRequest#uri is required to have a scheme.');
    }

    switch (params.method) {
      case LoadRequestMethod.get:
        _navigationGeneration += 1;
        return _webView.loadUrl(params.uri.toString(), params.headers);
      case LoadRequestMethod.post:
        if (params.headers.isNotEmpty) {
          throw UnsupportedError(
            'Custom request headers for POST requests are not supported by '
            'the OHOS ArkWeb postUrl API.',
          );
        }
        _navigationGeneration += 1;
        return _webView.postUrl(
          params.uri.toString(),
          params.body ?? Uint8List(0),
        );
    }
  }

  @override
  Future<String?> currentUrl() => _webView.getUrl();

  @override
  Future<bool> canGoBack() => _webView.canGoBack();

  @override
  Future<bool> canGoForward() => _webView.canGoForward();

  @override
  Future<void> goBack() {
    _navigationGeneration += 1;
    return _webView.goBack();
  }

  @override
  Future<void> goForward() {
    _navigationGeneration += 1;
    return _webView.goForward();
  }

  @override
  Future<void> reload() {
    _navigationGeneration += 1;
    return _webView.reload();
  }

  @override
  Future<void> clearCache() => _webView.clearCache(true);

  @override
  Future<void> clearLocalStorage() =>
      _ohosWebViewParams.ohosWebStorage.deleteAllData();

  @override
  Future<void> setPlatformNavigationDelegate(
    covariant OhosNavigationDelegate handler,
  ) async {
    final OhosNavigationDelegate? previousDelegate = _currentNavigationDelegate;
    if (previousDelegate != null) {
      _ohosControllerPageStartedCallbacks[previousDelegate]?.remove(
        _controllerPageStartedCallback,
      );
      _ohosControllerPageFinishedCallbacks[previousDelegate]?.remove(
        _controllerPageFinishedCallback,
      );
    }
    _currentNavigationDelegate = handler;
    (_ohosControllerPageStartedCallbacks[handler] ??= <PageEventCallback>{})
        .add(_controllerPageStartedCallback);
    (_ohosControllerPageFinishedCallbacks[handler] ??= <PageEventCallback>{})
        .add(_controllerPageFinishedCallback);
    await Future.wait(<Future<void>>[
      handler.setOnLoadRequest(loadRequest),
      _webView.setWebViewClient(handler.ohosWebViewClient),
      _webView.setDownloadListener(handler.ohosDownloadListener),
    ]);
    await _applyViewportStyle();
  }

  @override
  Future<void> runJavaScript(String javaScript) {
    return _webView.evaluateJavascript(javaScript);
  }

  @override
  Future<Object?> runJavaScriptReturningResult(String javaScript) async {
    final String? result = await _webView.evaluateJavascript(javaScript);
    if (result == null) return null;

    try {
      return jsonDecode(result);
    } on FormatException {
      return num.tryParse(result) ?? result;
    }
  }

  /// Invokes an asynchronous script through this controller's ArkWeb bridge.
  Future<Object?> callAsyncJavaScript(JavaScriptInvocationParams params) async {
    final int generation = _navigationGeneration;
    final Stopwatch stopwatch = Stopwatch()..start();
    final String identifier = _createOhosOpaqueIdentifier(
      'webview_all_${++_nextAsyncJavaScriptInvocationIdentifier}',
    );
    final String encodedIdentifier = jsonEncode(identifier);
    final String argumentNames = params.arguments.keys.join(',');
    final String argumentValues = params.arguments.keys
        .map((String name) => 'args[${jsonEncode(name)}]')
        .join(',');
    Duration remainingTime() => params.timeout - stopwatch.elapsed;
    Future<String?> evaluateWithinDeadline(String javaScript) {
      final Duration remaining = remainingTime();
      if (remaining <= Duration.zero) {
        return Future<String?>.error(
          TimeoutException(
            'The asynchronous JavaScript invocation timed out.',
            params.timeout,
          ),
        );
      }
      return _webView
          .evaluateJavascript(javaScript)
          .timeout(
            remaining,
            onTimeout: () => throw TimeoutException(
              'The asynchronous JavaScript invocation timed out.',
              params.timeout,
            ),
          );
    }

    try {
      final Duration setupRemaining = remainingTime();
      if (setupRemaining <= Duration.zero) {
        throw TimeoutException(
          'The asynchronous JavaScript invocation timed out.',
          params.timeout,
        );
      }
      final int timeoutMilliseconds = max(1, setupRemaining.inMilliseconds);
      await evaluateWithinDeadline('''
      (()=>{
        const results = window.__webviewAllAsyncResults =
            window.__webviewAllAsyncResults || Object.create(null);
        const slot = {result: null};
        results[$encodedIdentifier] = slot;
        const expiresAt = Date.now() + $timeoutMilliseconds;
        const release = () => {
          if (results[$encodedIdentifier] === slot) {
            delete results[$encodedIdentifier];
          }
        };
        const settle = (result) => {
          if (results[$encodedIdentifier] !== slot || Date.now() >= expiresAt) {
            release();
            return;
          }
          slot.result = result;
        };
        setTimeout(release, $timeoutMilliseconds);
        const args = ${jsonEncode(params.arguments)};
        const postError = (error) => {
          settle({
            state: 'rejected',
            name: error && error.name ? String(error.name) : 'Error',
            message: error && error.message ? String(error.message) : String(error),
            stack: error && error.stack ? String(error.stack) : null
          });
        };
        Promise.resolve().then(() => (async function($argumentNames) {
${params.functionBody}
        })($argumentValues)).then((value) => {
          try {
            const encoded = JSON.stringify(value);
            settle({
              state: 'fulfilled',
              value: encoded === undefined ? null : encoded
            });
          } catch (error) {
            postError(error);
          }
        }, postError);
      })();
    ''');

      while (stopwatch.elapsed < params.timeout) {
        if (generation != _navigationGeneration) {
          throw const JavaScriptExecutionException(
            name: 'NavigationError',
            message: 'The page navigated before JavaScript completed.',
          );
        }
        final String? rawResult = await evaluateWithinDeadline('''
          (()=>{
            const values = window.__webviewAllAsyncResults;
            const slot = values && values[$encodedIdentifier];
            const value = slot && slot.result;
            return value ? JSON.stringify(value) : null;
          })();
        ''');
        final Map<String, dynamic>? result = _decodeAsyncJavaScriptPollResult(
          rawResult,
        );
        if (result != null) {
          if (result['state'] == 'fulfilled') {
            final Object? encodedValue = result['value'];
            return encodedValue is String ? jsonDecode(encodedValue) : null;
          }
          throw JavaScriptExecutionException(
            name: result['name']?.toString(),
            message:
                result['message']?.toString() ??
                'The JavaScript promise was rejected.',
            javaScriptStackTrace: result['stack']?.toString(),
          );
        }
        final Duration delayRemaining = remainingTime();
        if (delayRemaining <= Duration.zero) {
          break;
        }
        const Duration pollingInterval = Duration(milliseconds: 20);
        await Future<void>.delayed(
          delayRemaining.compareTo(pollingInterval) < 0
              ? delayRemaining
              : pollingInterval,
        );
      }
      throw TimeoutException(
        'The asynchronous JavaScript invocation timed out.',
        params.timeout,
      );
    } finally {
      if (generation == _navigationGeneration &&
          remainingTime() > Duration.zero) {
        try {
          await evaluateWithinDeadline('''
            if (window.__webviewAllAsyncResults) {
              delete window.__webviewAllAsyncResults[$encodedIdentifier];
            }
          ''');
        } catch (_) {
          // Navigation can replace the document before cleanup completes.
        }
      }
    }
  }

  Map<String, dynamic>? _decodeAsyncJavaScriptPollResult(String? rawResult) {
    if (rawResult == null || rawResult == 'null') {
      return null;
    }
    try {
      Object? decoded = jsonDecode(rawResult);
      if (decoded is String) {
        decoded = jsonDecode(decoded);
      }
      return decoded is Map<String, dynamic> ? decoded : null;
    } on FormatException {
      return null;
    }
  }

  /// Registers document-start code with ArkWeb's native script list.
  @override
  Future<String> addUserScript(WebViewUserScript userScript) async {
    final identifier = await const MethodChannel('operit/webview_user_scripts')
        .invokeMethod<String>('add', {
          'viewIdentifier': webViewIdentifier,
          'source': buildUserScriptSource(
            userScript,
            platformHandlesMainFrameOnly: false,
          ),
        });
    if (identifier == null) {
      throw StateError('ArkWeb did not return a script handle');
    }
    return identifier;
  }

  /// Removes a document-start registration after ArkWeb commits the new list.
  @override
  Future<void> removeUserScript(String identifier) =>
      const MethodChannel('operit/webview_user_scripts').invokeMethod<void>(
        'remove',
        {'viewIdentifier': webViewIdentifier, 'identifier': identifier},
      );

  /// Removes application-owned document-start scripts without removing native proxies.
  @override
  Future<void> removeAllUserScripts() => const MethodChannel(
    'operit/webview_user_scripts',
  ).invokeMethod<void>('removeAll', {'viewIdentifier': webViewIdentifier});

  @override
  Future<void> addJavaScriptChannel(
    JavaScriptChannelParams javaScriptChannelParams,
  ) {
    if (javaScriptChannelParams.name.isEmpty) {
      throw ArgumentError.value(
        javaScriptChannelParams.name,
        'javaScriptChannelParams.name',
        'JavaScript channel names must not be empty.',
      );
    }
    final OhosJavaScriptChannelParams ohosJavaScriptParams =
        javaScriptChannelParams is OhosJavaScriptChannelParams
        ? javaScriptChannelParams
        : OhosJavaScriptChannelParams.fromJavaScriptChannelParams(
            javaScriptChannelParams,
          );

    // When JavaScript channel with the same name exists make sure to remove it
    // before registering the new channel.
    if (_javaScriptChannelParams.containsKey(ohosJavaScriptParams.name)) {
      _webView.removeJavaScriptChannel(ohosJavaScriptParams._javaScriptChannel);
    }

    _javaScriptChannelParams[ohosJavaScriptParams.name] = ohosJavaScriptParams;

    return _webView.addJavaScriptChannel(
      ohosJavaScriptParams._javaScriptChannel,
    );
  }

  @override
  Future<void> removeJavaScriptChannel(String javaScriptChannelName) async {
    final OhosJavaScriptChannelParams? javaScriptChannelParams =
        _javaScriptChannelParams[javaScriptChannelName];
    if (javaScriptChannelParams == null) {
      return;
    }

    _javaScriptChannelParams.remove(javaScriptChannelName);
    return _webView.removeJavaScriptChannel(
      javaScriptChannelParams._javaScriptChannel,
    );
  }

  @override
  Future<String?> getTitle() => _webView.getTitle();

  @override
  Future<void> scrollTo(int x, int y) => _webView.scrollTo(x, y);

  @override
  Future<void> scrollBy(int x, int y) => _webView.scrollBy(x, y);

  @override
  Future<void> setVerticalScrollBarEnabled(bool enabled) async {
    if (_verticalScrollBarEnabled == enabled) {
      return;
    }

    _verticalScrollBarEnabled = enabled;
    await _applyViewportStyle();
  }

  @override
  Future<void> setHorizontalScrollBarEnabled(bool enabled) async {
    if (_horizontalScrollBarEnabled == enabled) {
      return;
    }

    _horizontalScrollBarEnabled = enabled;
    await _applyViewportStyle();
  }

  @override
  bool supportsSetScrollBarsEnabled() => true;

  @override
  Future<Offset> getScrollPosition() {
    return _webView.getScrollPosition();
  }

  @override
  Future<void> enableZoom(bool enabled) =>
      _webView.settings.setSupportZoom(enabled);

  @override
  Future<void> setBackgroundColor(Color color) =>
      _webView.settings.setBackgroundColor(color);

  @override
  Future<void> setJavaScriptMode(JavaScriptMode javaScriptMode) => _webView
      .settings
      .setJavaScriptEnabled(javaScriptMode == JavaScriptMode.unrestricted);

  @override
  Future<void> setUserAgent(String? userAgent) =>
      _webView.settings.setUserAgentString(userAgent);

  @override
  Future<void> setOnScrollPositionChange(
    void Function(ScrollPositionChange scrollPositionChange)?
    onScrollPositionChange,
  ) async {
    _onScrollPositionChangedCallback = onScrollPositionChange;
  }

  /// Enables or disables full screen rotate within WebView.
  Future<void> setAllowFullScreenRotate(bool enabled) {
    return _webView.settings.setAllowFullScreenRotate(enabled);
  }

  /// Sets whether DOM storage is enabled.
  Future<void> setDomStorageEnabled(bool enabled) {
    return _webView.settings.setDomStorageEnabled(enabled);
  }

  /// Sets whether JavaScript may open windows automatically.
  Future<void> setJavaScriptCanOpenWindowsAutomatically(bool enabled) {
    return _webView.settings.setJavaScriptCanOpenWindowsAutomatically(enabled);
  }

  /// Sets whether the WebView should support multiple windows.
  Future<void> setSupportMultipleWindows(bool support) {
    return _webView.settings.setSupportMultipleWindows(support);
  }

  /// Sets whether pages are loaded in overview mode.
  Future<void> setLoadWithOverviewMode(bool overview) {
    return _webView.settings.setLoadWithOverviewMode(overview);
  }

  /// Sets whether wide viewport support is enabled.
  Future<void> setUseWideViewPort(bool use) {
    return _webView.settings.setUseWideViewPort(use);
  }

  // The OHOS platform view already composes application paint zoom. Only page
  // zoom is sent to ArkWeb; multiplying application zoom here would double it.
  late final NativeWebViewZoom _pageZoom = NativeWebViewZoom(
    applyZoom: (factor) => const MethodChannel('operit/webview_zoom')
        .invokeMethod<void>('setPageZoom', <String, Object?>{
          'viewIdentifier': webViewIdentifier,
          'zoomFactor': factor,
        }),
  );

  @override
  Future<void> setZoomFactor(double zoomFactor) =>
      _pageZoom.setPageZoomFactor(zoomFactor);

  /// Sets whether on-screen zoom controls are displayed.
  Future<void> setDisplayZoomControls(bool enabled) {
    return _webView.settings.setDisplayZoomControls(enabled);
  }

  /// Sets whether built-in zoom mechanisms are enabled.
  Future<void> setBuiltInZoomControls(bool enabled) {
    return _webView.settings.setBuiltInZoomControls(enabled);
  }

  /// Enables or disables file access within WebView.
  Future<void> setAllowFileAccess(bool enabled) {
    return _webView.settings.setAllowFileAccess(enabled);
  }

  /// Sets whether the WebView should support zooming.
  Future<void> setSupportZoom(bool support) {
    return _webView.settings.setSupportZoom(support);
  }

  /// Sets the restrictions that apply on automatic media playback.
  Future<void> setMediaPlaybackRequiresUserGesture(bool require) {
    return _webView.settings.setMediaPlaybackRequiresUserGesture(require);
  }

  /// Sets the text zoom of the page in percent.
  ///
  /// The default is 100.
  Future<void> setTextZoom(int textZoom) =>
      _webView.settings.setTextZoom(textZoom);

  /// Sets the callback that is invoked when the client should show a file
  /// selector.
  Future<void> setOnShowFileSelector(
    Future<List<String>> Function(FileSelectorParams params)?
    onShowFileSelector,
  ) {
    _onShowFileSelectorCallback = onShowFileSelector;
    return _webChromeClient.setSynchronousReturnValueForOnShowFileChooser(
      onShowFileSelector != null,
    );
  }

  /// Sets a callback that notifies the host application that web content is
  /// requesting permission to access the specified resources.
  ///
  /// Only invoked on Ohos versions 21+.
  @override
  Future<void> setOnPlatformPermissionRequest(
    void Function(PlatformWebViewPermissionRequest request) onPermissionRequest,
  ) async {
    _onPermissionRequestCallback = onPermissionRequest;
  }

  /// Sets the callback that is invoked when the client request handle geolocation permissions.
  ///
  /// Param [onShowPrompt] notifies the host application that web content from the specified origin is attempting to use the Geolocation API,
  /// but no permission state is currently set for that origin.
  ///
  /// The host application should invoke the specified callback with the desired permission state.
  /// See GeolocationPermissions for details.
  ///
  /// This method is only called for requests originating from secure origins
  /// such as https on current OHOS WebView engines.
  /// On non-secure origins geolocation requests are automatically denied.
  ///
  /// Param [onHidePrompt] notifies the host application that a request for Geolocation permissions,
  /// made with a previous call to onGeolocationPermissionsShowPrompt() has been canceled.
  /// Any related UI should therefore be hidden.
  Future<void> setGeolocationPermissionsPromptCallbacks({
    OnGeolocationPermissionsShowPrompt? onShowPrompt,
    OnGeolocationPermissionsHidePrompt? onHidePrompt,
  }) async {
    _onGeolocationPermissionsShowPrompt = onShowPrompt;
    _onGeolocationPermissionsHidePrompt = onHidePrompt;
  }

  /// Sets the callbacks that are invoked when the host application wants to
  /// show or hide a custom widget.
  ///
  /// The most common use case these methods are invoked a video element wants
  /// to be displayed in fullscreen.
  ///
  /// The [onShowCustomWidget] notifies the host application that web content
  /// from the specified origin wants to be displayed in a custom widget. After
  /// this call, web content will no longer be rendered in the `WebViewWidget`,
  /// but will instead be rendered in the custom widget. The application may
  /// explicitly exit fullscreen mode by invoking `onCustomWidgetHidden` in the
  /// [onShowCustomWidget] callback (ex. when the user presses the back
  /// button). However, this is generally not necessary as the web page will
  /// often show its own UI to close out of fullscreen. Regardless of how the
  /// WebView exits fullscreen mode, WebView will invoke [onHideCustomWidget],
  /// signaling for the application to remove the custom widget. If this value
  /// is `null` when passed to an `OhosWebViewWidget`, a default handler
  /// will be set.
  ///
  /// The [onHideCustomWidget] notifies the host application that the custom
  /// widget must be hidden. After this call, web content will render in the
  /// original `WebViewWidget` again.
  Future<void> setCustomWidgetCallbacks({
    required OnShowCustomWidgetCallback? onShowCustomWidget,
    required OnHideCustomWidgetCallback? onHideCustomWidget,
  }) async {
    _onShowCustomWidgetCallback = onShowCustomWidget;
    _onHideCustomWidgetCallback = onHideCustomWidget;
  }

  /// Sets a callback that notifies the host application of any log messages
  /// written to the JavaScript console.
  @override
  Future<void> setOnConsoleMessage(
    void Function(JavaScriptConsoleMessage consoleMessage) onConsoleMessage,
  ) async {
    _onConsoleLogCallback = onConsoleMessage;

    return _webChromeClient.setSynchronousReturnValueForOnConsoleMessage(
      _onConsoleLogCallback != null,
    );
  }

  @override
  Future<String?> getUserAgent() => _webView.settings.getUserAgentString();

  @override
  Future<void> setOverScrollMode(WebViewOverScrollMode mode) async {
    if (_overScrollMode == mode) {
      return;
    }

    _overScrollMode = mode;
    await _applyViewportStyle();
  }

  Future<void> _applyViewportStyle() async {
    final StringBuffer css = StringBuffer();
    if (!_verticalScrollBarEnabled) {
      css.writeln('*::-webkit-scrollbar:vertical { width: 0 !important; }');
    }
    if (!_horizontalScrollBarEnabled) {
      css.writeln('*::-webkit-scrollbar:horizontal { height: 0 !important; }');
    }

    final String? overScrollBehavior = switch (_overScrollMode) {
      WebViewOverScrollMode.always => null,
      WebViewOverScrollMode.ifContentScrolls => 'contain',
      WebViewOverScrollMode.never => 'none',
    };
    if (overScrollBehavior != null) {
      css.writeln(
        'html, body { overscroll-behavior: $overScrollBehavior !important; }',
      );
    }

    final String script =
        '''
      (function() {
        var styleId = ${jsonEncode(_viewportStyleElementId)};
        var css = ${jsonEncode(css.toString())};
        var style = document.getElementById(styleId);
        if (!css) {
          if (style && style.parentNode) {
            style.parentNode.removeChild(style);
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

    try {
      await _webView.evaluateJavascript(script);
    } catch (_) {
      // The document can be unavailable before the first page load.
    }
  }

  @override
  Future<void> setOnJavaScriptAlertDialog(
    Future<void> Function(JavaScriptAlertDialogRequest request)
    onJavaScriptAlertDialog,
  ) async {
    _onJavaScriptAlert = onJavaScriptAlertDialog;
    return _webChromeClient.setSynchronousReturnValueForOnJsAlert(true);
  }

  @override
  Future<void> setOnJavaScriptConfirmDialog(
    Future<bool> Function(JavaScriptConfirmDialogRequest request)
    onJavaScriptConfirmDialog,
  ) async {
    _onJavaScriptConfirm = onJavaScriptConfirmDialog;
    return _webChromeClient.setSynchronousReturnValueForOnJsConfirm(true);
  }

  @override
  Future<void> setOnJavaScriptTextInputDialog(
    Future<String> Function(JavaScriptTextInputDialogRequest request)
    onJavaScriptTextInputDialog,
  ) async {
    _onJavaScriptPrompt = onJavaScriptTextInputDialog;
    return _webChromeClient.setSynchronousReturnValueForOnJsPrompt(true);
  }
}

/// Ohos implementation of [PlatformWebViewPermissionRequest].
class OhosWebViewPermissionRequest extends PlatformWebViewPermissionRequest {
  OhosWebViewPermissionRequest._({
    required super.types,
    required ohos_webview.PermissionRequest request,
  }) : _request = request;

  final ohos_webview.PermissionRequest _request;
  final Completer<void> _completion = Completer<void>();

  Future<void> _complete(Future<void> Function() action) {
    if (_completion.isCompleted) {
      return Future<void>.value();
    }
    _completion.complete();
    return action();
  }

  @override
  Future<void> grant() {
    return _complete(() {
      final List<String> resources = types
          .map<String?>(_ohosResourceFromPermissionType)
          .whereType<String>()
          .toList();
      if (resources.isEmpty) {
        return _request.deny();
      }
      return _request.grant(resources);
    });
  }

  @override
  Future<void> deny() {
    return _complete(_request.deny);
  }

  static String? _ohosResourceFromPermissionType(
    WebViewPermissionResourceType type,
  ) {
    switch (type) {
      case WebViewPermissionResourceType.camera:
        return ohos_webview.PermissionRequest.videoCapture;
      case WebViewPermissionResourceType.microphone:
        return ohos_webview.PermissionRequest.audioCapture;
      case OhosWebViewPermissionResourceType.midiSysex:
        return ohos_webview.PermissionRequest.midiSysex;
      case OhosWebViewPermissionResourceType.protectedMediaId:
        return ohos_webview.PermissionRequest.protectedMediaId;
    }

    return null;
  }
}

/// Signature for the `setGeolocationPermissionsPromptCallbacks` callback responsible for request the Geolocation API.
typedef OnGeolocationPermissionsShowPrompt =
    Future<GeolocationPermissionsResponse> Function(
      GeolocationPermissionsRequestParams request,
    );

/// Signature for the `setGeolocationPermissionsPromptCallbacks` callback responsible for request the Geolocation API is cancel.
typedef OnGeolocationPermissionsHidePrompt = void Function();

/// Signature for the `setCustomWidgetCallbacks` callback responsible for showing the custom view.
typedef OnShowCustomWidgetCallback =
    void Function(Widget widget, void Function() onCustomWidgetHidden);

/// Signature for the `setCustomWidgetCallbacks` callback responsible for hiding the custom view.
typedef OnHideCustomWidgetCallback = void Function();

/// A request params used by the host application to set the Geolocation permission state for an origin.
@immutable
class GeolocationPermissionsRequestParams {
  /// [origin]: The origin for which permissions are set.
  const GeolocationPermissionsRequestParams({required this.origin});

  /// [origin]: The origin for which permissions are set.
  final String origin;
}

/// A response used by the host application to set the Geolocation permission state for an origin.
@immutable
class GeolocationPermissionsResponse {
  /// [allow]: Whether or not the origin should be allowed to use the Geolocation API.
  ///
  /// [retain]: Whether the permission should be retained beyond the lifetime of
  /// a page currently being displayed by a WebView.
  const GeolocationPermissionsResponse({
    required this.allow,
    required this.retain,
  });

  /// Whether or not the origin should be allowed to use the Geolocation API.
  final bool allow;

  /// Whether the permission should be retained beyond the lifetime of
  /// a page currently being displayed by a WebView.
  final bool retain;
}

/// Mode of how to select files for a file chooser.
enum FileSelectorMode {
  /// Open single file and requires that the file exists before allowing the
  /// user to pick it.
  open,

  /// Similar to [open] but allows multiple files to be selected.
  openMultiple,

  /// Allows picking a nonexistent file and saving it.
  save,
}

/// Parameters received when the `WebView` should show a file selector.
@immutable
class FileSelectorParams {
  /// Constructs a [FileSelectorParams].
  const FileSelectorParams({
    required this.isCaptureEnabled,
    required this.acceptTypes,
    this.filenameHint,
    required this.mode,
  });

  factory FileSelectorParams._fromFileChooserParams(
    ohos_webview.FileChooserParams params,
  ) {
    final FileSelectorMode mode;
    switch (params.mode) {
      case ohos_webview.FileChooserMode.open:
        mode = FileSelectorMode.open;
        break;
      case ohos_webview.FileChooserMode.openMultiple:
        mode = FileSelectorMode.openMultiple;
        break;
      case ohos_webview.FileChooserMode.save:
        mode = FileSelectorMode.save;
        break;
    }

    return FileSelectorParams(
      isCaptureEnabled: params.isCaptureEnabled,
      acceptTypes: params.acceptTypes,
      mode: mode,
      filenameHint: params.filenameHint,
    );
  }

  /// Preference for a live media captured value (e.g. Camera, Microphone).
  final bool isCaptureEnabled;

  /// A list of acceptable MIME types.
  final List<String> acceptTypes;

  /// The file name of a default selection if specified, or null.
  final String? filenameHint;

  /// Mode of how to select files for a file selector.
  final FileSelectorMode mode;
}

/// An implementation of [JavaScriptChannelParams] with the Ohos WebView API.
///
/// See [OhosWebViewController.addJavaScriptChannel].
@immutable
class OhosJavaScriptChannelParams extends JavaScriptChannelParams {
  /// Constructs a [OhosJavaScriptChannelParams].
  OhosJavaScriptChannelParams({
    required super.name,
    required super.onMessageReceived,
    @visibleForTesting OhosWebViewProxy webViewProxy = const OhosWebViewProxy(),
  }) : assert(name.isNotEmpty),
       _javaScriptChannel = webViewProxy.createJavaScriptChannel(
         name,
         postMessage: withWeakReferenceTo(onMessageReceived, (
           WeakReference<void Function(JavaScriptMessage)> weakReference,
         ) {
           return (String message) {
             if (weakReference.target != null) {
               weakReference.target!(JavaScriptMessage(message: message));
             }
           };
         }),
       );

  /// Constructs a [OhosJavaScriptChannelParams] using a
  /// [JavaScriptChannelParams].
  OhosJavaScriptChannelParams.fromJavaScriptChannelParams(
    JavaScriptChannelParams params, {
    @visibleForTesting OhosWebViewProxy webViewProxy = const OhosWebViewProxy(),
  }) : this(
         name: params.name,
         onMessageReceived: params.onMessageReceived,
         webViewProxy: webViewProxy,
       );

  final ohos_webview.JavaScriptChannel _javaScriptChannel;
}

/// Object specifying creation parameters for creating a [OhosWebViewWidget].
///
/// When adding additional fields make sure they can be null or have a default
/// value to avoid breaking changes. See [PlatformWebViewWidgetCreationParams] for
/// more information.
@immutable
class OhosWebViewWidgetCreationParams
    extends PlatformWebViewWidgetCreationParams {
  /// Creates [OhosWebWidgetCreationParams].
  OhosWebViewWidgetCreationParams({
    super.key,
    required super.controller,
    super.layoutDirection,
    super.gestureRecognizers,
    this.displayWithHybridComposition = false,
    @visibleForTesting InstanceManager? instanceManager,
    @visibleForTesting
    this.platformViewsServiceProxy = const PlatformViewsServiceProxy(),
  }) : instanceManager =
           instanceManager ?? ohos_webview.OhosObject.globalInstanceManager;

  /// Constructs a [WebKitWebViewWidgetCreationParams] using a
  /// [PlatformWebViewWidgetCreationParams].
  OhosWebViewWidgetCreationParams.fromPlatformWebViewWidgetCreationParams(
    PlatformWebViewWidgetCreationParams params, {
    bool displayWithHybridComposition = false,
    @visibleForTesting InstanceManager? instanceManager,
    @visibleForTesting
    PlatformViewsServiceProxy platformViewsServiceProxy =
        const PlatformViewsServiceProxy(),
  }) : this(
         key: params.key,
         controller: params.controller,
         layoutDirection: params.layoutDirection,
         gestureRecognizers: params.gestureRecognizers,
         displayWithHybridComposition: displayWithHybridComposition,
         instanceManager: instanceManager,
         platformViewsServiceProxy: platformViewsServiceProxy,
       );

  /// Maintains instances used to communicate with the native objects they
  /// represent.
  ///
  /// This field is exposed for testing purposes only and should not be used
  /// outside of tests.
  @visibleForTesting
  final InstanceManager instanceManager;

  /// Proxy that provides access to the platform views service.
  ///
  /// This service allows creating and controlling platform-specific views.
  @visibleForTesting
  final PlatformViewsServiceProxy platformViewsServiceProxy;

  /// Whether the [WebView] will be displayed using the Hybrid Composition
  /// PlatformView implementation.
  ///
  /// For most use cases, this flag should be set to false. Hybrid Composition
  /// can have performance costs but doesn't have the limitation of rendering to
  /// an Ohos SurfaceTexture. See
  /// * https://flutter.dev/docs/development/platform-integration/platform-views#performance
  /// * https://github.com/flutter/flutter/issues/104889
  /// * https://github.com/flutter/flutter/issues/116954
  ///
  /// Defaults to false.
  final bool displayWithHybridComposition;

  @override
  int get hashCode => Object.hash(
    controller,
    layoutDirection,
    displayWithHybridComposition,
    platformViewsServiceProxy,
    instanceManager,
  );

  @override
  bool operator ==(Object other) {
    return other is OhosWebViewWidgetCreationParams &&
        controller == other.controller &&
        layoutDirection == other.layoutDirection &&
        displayWithHybridComposition == other.displayWithHybridComposition &&
        platformViewsServiceProxy == other.platformViewsServiceProxy &&
        instanceManager == other.instanceManager;
  }
}

/// An implementation of [PlatformWebViewWidget] with the Ohos WebView API.
class OhosWebViewWidget extends PlatformWebViewWidget {
  /// Constructs a [WebKitWebViewWidget].
  OhosWebViewWidget(PlatformWebViewWidgetCreationParams params)
    : super.implementation(
        params is OhosWebViewWidgetCreationParams
            ? params
            : OhosWebViewWidgetCreationParams.fromPlatformWebViewWidgetCreationParams(
                params,
              ),
      );

  OhosWebViewWidgetCreationParams get _ohosParams =>
      params as OhosWebViewWidgetCreationParams;

  @override
  Widget build(BuildContext context) {
    _trySetDefaultOnShowCustomWidgetCallbacks(context);
    return PlatformViewLink(
      // Setting a default key using `params` ensures the `PlatformViewLink`
      // recreates the PlatformView when changes are made.
      key:
          _ohosParams.key ??
          ValueKey<OhosWebViewWidgetCreationParams>(
            params as OhosWebViewWidgetCreationParams,
          ),
      viewType: ohosWebViewPlatformViewType,
      surfaceFactory:
          (BuildContext context, PlatformViewController controller) {
            return OhosViewSurface(
              controller: controller as OhosViewController,
              gestureRecognizers: _ohosParams.gestureRecognizers,
              hitTestBehavior: PlatformViewHitTestBehavior.opaque,
            );
          },
      onCreatePlatformView: (PlatformViewCreationParams params) {
        return _initOhosView(
            params,
            displayWithHybridComposition:
                _ohosParams.displayWithHybridComposition,
          )
          ..addOnPlatformViewCreatedListener(params.onPlatformViewCreated)
          ..create();
      },
    );
  }

  OhosViewController _initOhosView(
    PlatformViewCreationParams params, {
    required bool displayWithHybridComposition,
  }) {
    if (displayWithHybridComposition) {
      return _ohosParams.platformViewsServiceProxy.initExpensiveOhosView(
        id: params.id,
        viewType: ohosWebViewPlatformViewType,
        layoutDirection: _ohosParams.layoutDirection,
        creationParams: _ohosParams.instanceManager.getIdentifier(
          (_ohosParams.controller as OhosWebViewController)._webView,
        ),
        creationParamsCodec: const StandardMessageCodec(),
      );
    } else {
      return _ohosParams.platformViewsServiceProxy.initSurfaceOhosView(
        id: params.id,
        viewType: ohosWebViewPlatformViewType,
        layoutDirection: _ohosParams.layoutDirection,
        creationParams: _ohosParams.instanceManager.getIdentifier(
          (_ohosParams.controller as OhosWebViewController)._webView,
        ),
        creationParamsCodec: const StandardMessageCodec(),
      );
    }
  }

  // Attempt to handle custom views with a default implementation if it has not
  // been set.
  void _trySetDefaultOnShowCustomWidgetCallbacks(BuildContext context) {
    final OhosWebViewController controller =
        _ohosParams.controller as OhosWebViewController;

    if (controller._onShowCustomWidgetCallback == null) {
      controller.setCustomWidgetCallbacks(
        onShowCustomWidget:
            (Widget widget, OnHideCustomWidgetCallback callback) {
              Navigator.of(context).push(
                MaterialPageRoute<void>(
                  builder: (BuildContext context) => widget,
                  fullscreenDialog: true,
                ),
              );
            },
        onHideCustomWidget: () {
          Navigator.of(context).pop();
        },
      );
    }
  }
}

/// Represents a Flutter wrapper around a native OHOS view that is created by
/// the host platform when web content needs to be displayed in fullscreen mode.
///
/// The [OhosCustomViewWidget] cannot be manually instantiated and is
/// provided to the host application through the callbacks specified using the
/// [OhosWebViewController.setCustomWidgetCallbacks] method.
///
/// The [OhosCustomViewWidget] is initialized internally and should only be
/// exposed as a [Widget] externally. The type [OhosCustomViewWidget] is
/// visible for testing purposes only and should never be called externally.
@visibleForTesting
class OhosCustomViewWidget extends StatelessWidget {
  /// Creates a [OhosCustomViewWidget].
  ///
  /// The [OhosCustomViewWidget] should only be instantiated internally.
  /// This constructor is visible for testing purposes only and should
  /// never be called externally.
  @visibleForTesting
  OhosCustomViewWidget.private({
    super.key,
    required this.controller,
    required this.customView,
    @visibleForTesting InstanceManager? instanceManager,
    @visibleForTesting
    this.platformViewsServiceProxy = const PlatformViewsServiceProxy(),
  }) : instanceManager =
           instanceManager ?? ohos_webview.OhosObject.globalInstanceManager;

  /// The reference to the OHOS native view that should be shown.
  final ohos_webview.View customView;

  /// The [PlatformWebViewController] that allows controlling the native web
  /// view.
  final PlatformWebViewController controller;

  /// Maintains instances used to communicate with the native objects they
  /// represent.
  ///
  /// This field is exposed for testing purposes only and should not be used
  /// outside of tests.
  @visibleForTesting
  final InstanceManager instanceManager;

  /// Proxy that provides access to the platform views service.
  ///
  /// This service allows creating and controlling platform-specific views.
  @visibleForTesting
  final PlatformViewsServiceProxy platformViewsServiceProxy;

  OhosWebViewWidgetCreationParams get _ohosParams =>
      controller.params as OhosWebViewWidgetCreationParams;

  @override
  Widget build(BuildContext context) {
    return OhosView(
      key:
          _ohosParams.key ??
          ValueKey<OhosWebViewWidgetCreationParams>(_ohosParams),
      viewType: ohosWebViewPlatformViewType,
      layoutDirection: _ohosParams.layoutDirection,
      creationParams: _ohosParams.instanceManager.getIdentifier(
        (_ohosParams.controller as OhosWebViewController)._webView,
      ),
      creationParamsCodec: const StandardMessageCodec(),
      gestureRecognizers: _ohosParams.gestureRecognizers,
    );
  }
}

/// Signature for the `loadRequest` callback responsible for loading the [url]
/// after a navigation request has been approved.
typedef LoadRequestCallback = Future<void> Function(LoadRequestParams params);

/// Error returned in `WebView.onWebResourceError` when a web resource loading error has occurred.
@immutable
class OhosWebResourceError extends WebResourceError {
  /// Creates a new [OhosWebResourceError].
  OhosWebResourceError._({
    required super.errorCode,
    required super.description,
    super.isForMainFrame,
    super.url,
  }) : super(errorType: _errorCodeToErrorType(errorCode));

  /// Gets the URL for which the failing resource request was made.
  @Deprecated('Please use `url`.')
  String? get failingUrl => url;

  static WebResourceErrorType? _errorCodeToErrorType(int errorCode) {
    switch (errorCode) {
      case ohos_webview.WebViewClient.errorAuthentication:
        return WebResourceErrorType.authentication;
      case ohos_webview.WebViewClient.errorBadUrl:
        return WebResourceErrorType.badUrl;
      case ohos_webview.WebViewClient.errorConnect:
        return WebResourceErrorType.connect;
      case ohos_webview.WebViewClient.errorFailedSslHandshake:
        return WebResourceErrorType.failedSslHandshake;
      case ohos_webview.WebViewClient.errorFile:
        return WebResourceErrorType.file;
      case ohos_webview.WebViewClient.errorFileNotFound:
        return WebResourceErrorType.fileNotFound;
      case ohos_webview.WebViewClient.errorHostLookup:
        return WebResourceErrorType.hostLookup;
      case ohos_webview.WebViewClient.errorIO:
        return WebResourceErrorType.io;
      case ohos_webview.WebViewClient.errorProxyAuthentication:
        return WebResourceErrorType.proxyAuthentication;
      case ohos_webview.WebViewClient.errorRedirectLoop:
        return WebResourceErrorType.redirectLoop;
      case ohos_webview.WebViewClient.errorTimeout:
        return WebResourceErrorType.timeout;
      case ohos_webview.WebViewClient.errorTooManyRequests:
        return WebResourceErrorType.tooManyRequests;
      case ohos_webview.WebViewClient.errorUnknown:
        return WebResourceErrorType.unknown;
      case ohos_webview.WebViewClient.errorUnsafeResource:
        return WebResourceErrorType.unsafeResource;
      case ohos_webview.WebViewClient.errorUnsupportedAuthScheme:
        return WebResourceErrorType.unsupportedAuthScheme;
      case ohos_webview.WebViewClient.errorUnsupportedScheme:
        return WebResourceErrorType.unsupportedScheme;
    }

    if (errorCode < 0) {
      return WebResourceErrorType.unknown;
    }

    throw ArgumentError(
      'Could not find a WebResourceErrorType for errorCode: $errorCode',
    );
  }
}

/// Object specifying creation parameters for creating a [OhosNavigationDelegate].
///
/// When adding additional fields make sure they can be null or have a default
/// value to avoid breaking changes. See [PlatformNavigationDelegateCreationParams] for
/// more information.
@immutable
class OhosNavigationDelegateCreationParams
    extends PlatformNavigationDelegateCreationParams {
  /// Creates a new [OhosNavigationDelegateCreationParams] instance.
  const OhosNavigationDelegateCreationParams._({
    @visibleForTesting this.ohosWebViewProxy = const OhosWebViewProxy(),
  }) : super();

  /// Creates a [OhosNavigationDelegateCreationParams] instance based on [PlatformNavigationDelegateCreationParams].
  factory OhosNavigationDelegateCreationParams.fromPlatformNavigationDelegateCreationParams(
    // Recommended placeholder to prevent being broken by platform interface.
    // ignore: avoid_unused_constructor_parameters
    PlatformNavigationDelegateCreationParams params, {
    @visibleForTesting
    OhosWebViewProxy ohosWebViewProxy = const OhosWebViewProxy(),
  }) {
    return OhosNavigationDelegateCreationParams._(
      ohosWebViewProxy: ohosWebViewProxy,
    );
  }

  /// Handles constructing objects and calling static methods for the Ohos WebView
  /// native library.
  @visibleForTesting
  final OhosWebViewProxy ohosWebViewProxy;
}

/// Ohos details of the change to a web view's url.
class OhosUrlChange extends UrlChange {
  /// Constructs an [OhosUrlChange].
  const OhosUrlChange({required super.url, required this.isReload});

  /// Whether the url is being reloaded.
  final bool isReload;
}

/// A place to register callback methods responsible to handle navigation events
/// triggered by the [ohos_webview.WebView].
class OhosNavigationDelegate extends PlatformNavigationDelegate {
  /// Creates a new [OhosNavigationDelegate].
  OhosNavigationDelegate(PlatformNavigationDelegateCreationParams params)
    : super.implementation(
        params is OhosNavigationDelegateCreationParams
            ? params
            : OhosNavigationDelegateCreationParams.fromPlatformNavigationDelegateCreationParams(
                params,
              ),
      ) {
    final WeakReference<OhosNavigationDelegate> weakThis =
        WeakReference<OhosNavigationDelegate>(this);

    _webViewClient = (this.params as OhosNavigationDelegateCreationParams)
        .ohosWebViewProxy
        .createOhosWebViewClient(
          onPageFinished: (ohos_webview.WebView webView, String url) {
            final OhosNavigationDelegate? target = weakThis.target;
            final PageEventCallback? callback = target?._onPageFinished;
            if (callback != null) {
              callback(url);
            }
            if (target != null) {
              for (final PageEventCallback controllerCallback
                  in List<PageEventCallback>.of(
                    _ohosControllerPageFinishedCallbacks[target] ??
                        const <PageEventCallback>{},
                  )) {
                controllerCallback(url);
              }
            }
          },
          onPageStarted: (ohos_webview.WebView webView, String url) {
            final OhosNavigationDelegate? target = weakThis.target;
            if (target != null) {
              for (final PageEventCallback controllerCallback
                  in List<PageEventCallback>.of(
                    _ohosControllerPageStartedCallbacks[target] ??
                        const <PageEventCallback>{},
                  )) {
                controllerCallback(url);
              }
            }
            final PageEventCallback? callback = target?._onPageStarted;
            if (callback != null) {
              callback(url);
            }
          },
          onReceivedRequestError:
              (
                ohos_webview.WebView webView,
                ohos_webview.WebResourceRequest request,
                ohos_webview.WebResourceError error,
              ) {
                final WebResourceErrorCallback? callback =
                    weakThis.target?._onWebResourceError;
                if (callback != null) {
                  callback(
                    OhosWebResourceError._(
                      errorCode: error.errorCode,
                      description: error.description,
                      url: request.url,
                      isForMainFrame: request.isForMainFrame,
                    ),
                  );
                }
              },
          onReceivedHttpError:
              (
                ohos_webview.WebView webView,
                ohos_webview.WebResourceRequest request,
                ohos_webview.WebResourceResponse response,
              ) {
                final HttpResponseErrorCallback? callback =
                    weakThis.target?._onHttpError;
                final Uri? uri = Uri.tryParse(request.url);
                if (callback != null) {
                  callback(
                    HttpResponseError(
                      request: uri == null
                          ? null
                          : OhosWebResourceRequest._(
                              uri: uri,
                              isForMainFrame: request.isForMainFrame,
                              isRedirect: request.isRedirect,
                              hasGesture: request.hasGesture,
                              method: request.method,
                              headers: request.requestHeaders,
                            ),
                      response: OhosWebResourceResponse._(
                        uri: uri,
                        statusCode: response.statusCode,
                        headers: response.responseHeaders,
                        reasonPhrase: response.reasonPhrase,
                        mimeType: response.mimeType,
                      ),
                    ),
                  );
                }
              },
          onReceivedError:
              (
                ohos_webview.WebView webView,
                int errorCode,
                String description,
                String failingUrl,
              ) {
                final WebResourceErrorCallback? callback =
                    weakThis.target?._onWebResourceError;
                if (callback != null) {
                  callback(
                    OhosWebResourceError._(
                      errorCode: errorCode,
                      description: description,
                      url: failingUrl,
                      isForMainFrame: true,
                    ),
                  );
                }
              },
          requestLoading:
              (
                ohos_webview.WebView webView,
                ohos_webview.WebResourceRequest request,
              ) {
                weakThis.target?._handleNavigation(
                  request.url,
                  headers: request.requestHeaders,
                  isForMainFrame: request.isForMainFrame,
                );
              },
          urlLoading: (ohos_webview.WebView webView, String url) {
            weakThis.target?._handleNavigation(url, isForMainFrame: true);
          },
          doUpdateVisitedHistory:
              (ohos_webview.WebView webView, String url, bool isReload) {
                final UrlChangeCallback? callback =
                    weakThis.target?._onUrlChange;
                if (callback != null) {
                  callback(OhosUrlChange(url: url, isReload: isReload));
                }
              },
          onReceivedHttpAuthRequest:
              (
                ohos_webview.WebView webView,
                ohos_webview.HttpAuthHandler httpAuthHandler,
                String host,
                String realm,
              ) {
                final void Function(HttpAuthRequest)? callback =
                    weakThis.target?._onHttpAuthRequest;
                if (callback != null) {
                  bool completed = false;
                  void complete(Future<void> Function() action) {
                    if (completed) {
                      return;
                    }
                    completed = true;
                    _runOhosAsyncCallbackSafely(
                      'HTTP authentication decision',
                      action,
                    );
                  }

                  final HttpAuthRequest request = HttpAuthRequest(
                    onProceed: (WebViewCredential credential) {
                      complete(
                        () => httpAuthHandler.proceed(
                          credential.user,
                          credential.password,
                        ),
                      );
                    },
                    onCancel: () {
                      complete(httpAuthHandler.cancel);
                    },
                    host: host,
                    realm: realm,
                  );
                  try {
                    callback(request);
                  } catch (error) {
                    if (completed) {
                      debugPrint(
                        'webview_all_ohos: HTTP authentication callback failed '
                        'after completing its decision; the completed decision '
                        'was preserved: '
                        '${error.toString().replaceAll(RegExp(r'[\r\n]+'), ' ')}',
                      );
                    } else {
                      debugPrint(
                        'webview_all_ohos: HTTP authentication callback failed; '
                        'the request was canceled: '
                        '${error.toString().replaceAll(RegExp(r'[\r\n]+'), ' ')}',
                      );
                      complete(httpAuthHandler.cancel);
                    }
                  }
                } else {
                  _runOhosAsyncCallbackSafely(
                    'HTTP authentication fallback',
                    httpAuthHandler.cancel,
                  );
                }
              },
          onReceivedSslAuthError:
              (
                ohos_webview.WebView webView,
                ohos_webview.SslAuthHandler sslAuthHandler,
                String url,
                int errorCode,
                String description,
              ) {
                final void Function(PlatformSslAuthError)? callback =
                    weakThis.target?._onSslAuthError;
                if (callback != null) {
                  final OhosPlatformSslAuthError error =
                      OhosPlatformSslAuthError._(
                        handler: sslAuthHandler,
                        description: description.isEmpty
                            ? 'SSL certificate error for $url.'
                            : description,
                        url: url,
                        errorCode: errorCode,
                      );
                  try {
                    callback(error);
                  } catch (callbackError) {
                    if (error._isCompleted) {
                      debugPrint(
                        'webview_all_ohos: SSL authentication callback failed '
                        'after completing its decision; the completed decision '
                        'was preserved: '
                        '${callbackError.toString().replaceAll(RegExp(r'[\r\n]+'), ' ')}',
                      );
                    } else {
                      debugPrint(
                        'webview_all_ohos: SSL authentication callback failed; '
                        'the request was canceled: '
                        '${callbackError.toString().replaceAll(RegExp(r'[\r\n]+'), ' ')}',
                      );
                      _runOhosAsyncCallbackSafely(
                        'SSL authentication fallback',
                        error.cancel,
                      );
                    }
                  }
                } else {
                  _runOhosAsyncCallbackSafely(
                    'SSL authentication fallback',
                    sslAuthHandler.cancel,
                  );
                }
              },
        );

    _downloadListener = (this.params as OhosNavigationDelegateCreationParams)
        .ohosWebViewProxy
        .createDownloadListener(
          onDownloadStart:
              (
                String url,
                String userAgent,
                String contentDisposition,
                String mimetype,
                int contentLength,
              ) {
                if (weakThis.target != null) {
                  weakThis.target?._handleNavigation(url, isForMainFrame: true);
                }
              },
        );
  }

  OhosNavigationDelegateCreationParams get _ohosParams =>
      params as OhosNavigationDelegateCreationParams;

  late final ohos_webview.WebChromeClient _webChromeClient = _ohosParams
      .ohosWebViewProxy
      .createOhosWebChromeClient();

  /// Gets the native [ohos_webview.WebChromeClient] that is bridged by this [OhosNavigationDelegate].
  ///
  /// Used by the [OhosWebViewController] to set the `ohos_webview.WebView.setWebChromeClient`.
  @Deprecated(
    'This value is not used by `OhosWebViewController` and has no effect on the `WebView`.',
  )
  ohos_webview.WebChromeClient get ohosWebChromeClient => _webChromeClient;

  late final ohos_webview.WebViewClient _webViewClient;

  /// Gets the native [ohos_webview.WebViewClient] that is bridged by this [OhosNavigationDelegate].
  ///
  /// Used by the [OhosWebViewController] to set the `ohos_webview.WebView.setWebViewClient`.
  ohos_webview.WebViewClient get ohosWebViewClient => _webViewClient;

  late final ohos_webview.DownloadListener _downloadListener;

  /// Gets the native [ohos_webview.DownloadListener] that is bridged by this [OhosNavigationDelegate].
  ///
  /// Used by the [OhosWebViewController] to set the `ohos_webview.WebView.setDownloadListener`.
  ohos_webview.DownloadListener get ohosDownloadListener => _downloadListener;

  PageEventCallback? _onPageFinished;
  PageEventCallback? _onPageStarted;
  ProgressCallback? _onProgress;
  HttpResponseErrorCallback? _onHttpError;
  WebResourceErrorCallback? _onWebResourceError;
  NavigationRequestCallback? _onNavigationRequest;
  LoadRequestCallback? _onLoadRequest;
  UrlChangeCallback? _onUrlChange;
  HttpAuthRequestCallback? _onHttpAuthRequest;
  SslAuthErrorCallback? _onSslAuthError;

  void _handleNavigation(
    String url, {
    required bool isForMainFrame,
    Map<String, String> headers = const <String, String>{},
  }) {
    final LoadRequestCallback? onLoadRequest = _onLoadRequest;
    final NavigationRequestCallback? onNavigationRequest = _onNavigationRequest;

    if (onNavigationRequest == null || onLoadRequest == null) {
      return;
    }

    FutureOr<NavigationDecision> returnValue;
    try {
      returnValue = onNavigationRequest(
        NavigationRequest(url: url, isMainFrame: isForMainFrame),
      );
    } catch (error) {
      _reportOhosCallbackError('navigation request callback', error);
      return;
    }

    void handleDecision(NavigationDecision decision) {
      if (decision == NavigationDecision.navigate && isForMainFrame) {
        _runOhosAsyncCallbackSafely(
          'approved navigation load',
          () => onLoadRequest(
            LoadRequestParams(uri: Uri.parse(url), headers: headers),
          ),
        );
      }
    }

    if (returnValue is NavigationDecision) {
      handleDecision(returnValue);
    } else {
      unawaited(
        returnValue.then<void>(
          handleDecision,
          onError: (Object error, StackTrace stackTrace) {
            _reportOhosCallbackError('navigation request callback', error);
          },
        ),
      );
    }
  }

  /// Invoked when loading the url after a navigation request is approved.
  Future<void> setOnLoadRequest(LoadRequestCallback onLoadRequest) async {
    _onLoadRequest = onLoadRequest;
  }

  @override
  Future<void> setOnNavigationRequest(
    NavigationRequestCallback onNavigationRequest,
  ) async {
    _onNavigationRequest = onNavigationRequest;
    return _webViewClient.setSynchronousReturnValueForShouldOverrideUrlLoading(
      true,
    );
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

/// OHOS implementation of [WebResourceRequest].
class OhosWebResourceRequest extends WebResourceRequest {
  /// Creates a new [OhosWebResourceRequest].
  const OhosWebResourceRequest._({
    required super.uri,
    required this.isForMainFrame,
    required this.hasGesture,
    required this.method,
    this.isRedirect,
    this.headers = const <String, String>{},
  });

  /// Whether the request was made for the main frame.
  final bool isForMainFrame;

  /// Whether the request was a server-side redirect, when available.
  final bool? isRedirect;

  /// Whether a user gesture was associated with the request.
  final bool hasGesture;

  /// The HTTP method associated with the request.
  final String method;

  /// The HTTP request headers reported by ArkWeb.
  final Map<String, String> headers;
}

/// OHOS implementation of [WebResourceResponse].
class OhosWebResourceResponse extends WebResourceResponse {
  /// Creates a new [OhosWebResourceResponse].
  const OhosWebResourceResponse._({
    required super.uri,
    required super.statusCode,
    required super.headers,
    this.reasonPhrase,
    this.mimeType,
  });

  /// The HTTP reason phrase reported by ArkWeb, when available.
  final String? reasonPhrase;

  /// The response MIME type reported by ArkWeb, when available.
  final String? mimeType;
}

/// OHOS implementation of [PlatformSslAuthError].
class OhosPlatformSslAuthError extends PlatformSslAuthError {
  OhosPlatformSslAuthError._({
    required ohos_webview.SslAuthHandler handler,
    required String description,
    required this.url,
    required this.errorCode,
  }) : _handler = handler,
       super(certificate: null, description: description);

  final ohos_webview.SslAuthHandler _handler;
  final Completer<void> _completion = Completer<void>();

  bool get _isCompleted => _completion.isCompleted;

  /// URL that triggered the recoverable SSL certificate error.
  final String url;

  /// Native OHOS SSL error code when available.
  final int errorCode;

  Future<void> _complete(Future<void> Function() action) {
    if (_completion.isCompleted) {
      return Future<void>.value();
    }
    _completion.complete();
    return action();
  }

  @override
  Future<void> proceed() {
    return _complete(_handler.proceed);
  }

  @override
  Future<void> cancel() {
    return _complete(_handler.cancel);
  }
}
