// Copyright 2013 The Flutter Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'dart:async';
import 'dart:ui';

import 'package:flutter/foundation.dart' show debugPrint;
import 'package:flutter/services.dart' show BinaryMessenger, Uint8List;

import 'callback_dispatcher.dart';
import 'host_api.dart';
import '../ohos_webview_native.dart';
import 'types.dart';
import '../core/instance_manager.dart';

export 'types.dart' show ConsoleMessage, ConsoleMessageLevel, FileChooserMode;

String _singleLineOhosLogValue(Object value) {
  return value.toString().replaceAll(RegExp(r'[\r\n]+'), ' ');
}

void _reportOhosApiCallbackError(String description, Object error) {
  debugPrint(
    'webview_all_ohos: $description failed: '
    '${_singleLineOhosLogValue(error)}',
  );
}

T? _callbackInstance<T extends Copyable>(
  InstanceManager instanceManager,
  int identifier,
  String type,
) {
  final T? instance = instanceManager.getInstanceWithWeakReference<T>(
    identifier,
  );
  if (instance == null) {
    debugPrint(
      'webview_all_ohos: ignored stale $type callback for instance $identifier.',
    );
  }
  return instance;
}

void _runCallbackFallback(
  Future<void> Function() operation,
  String description,
) {
  Future<void> future;
  try {
    future = operation();
  } catch (error) {
    _reportOhosApiCallbackError(description, error);
    return;
  }
  unawaited(
    future.then<void>(
      (_) {},
      onError: (Object error, StackTrace stackTrace) {
        _reportOhosApiCallbackError(description, error);
      },
    ),
  );
}

void _runOhosVoidCallbackSafely(String description, void Function() callback) {
  try {
    callback();
  } catch (error) {
    _reportOhosApiCallbackError(description, error);
  }
}

void _runOhosAsyncCallbackWithFallback(
  String description,
  Future<void> Function() callback,
  Future<void> Function() fallback,
) {
  Future<void> future;
  try {
    future = callback();
  } catch (error) {
    _reportOhosApiCallbackError(description, error);
    _runCallbackFallback(fallback, '$description fallback');
    return;
  }
  unawaited(
    future.then<void>(
      (_) {},
      onError: (Object error, StackTrace stackTrace) {
        _reportOhosApiCallbackError(description, error);
        _runCallbackFallback(fallback, '$description fallback');
      },
    ),
  );
}

/// Converts [WebResourceRequestData] to [WebResourceRequest]
WebResourceRequest _toWebResourceRequest(WebResourceRequestData data) {
  return WebResourceRequest(
    url: data.url,
    isForMainFrame: data.isForMainFrame,
    isRedirect: data.isRedirect,
    hasGesture: data.hasGesture,
    method: data.method,
    requestHeaders: data.requestHeaders,
  );
}

/// Converts [WebResourceErrorData] to [WebResourceError].
WebResourceError _toWebResourceError(WebResourceErrorData data) {
  return WebResourceError(
    errorCode: data.errorCode,
    description: data.description,
  );
}

/// Converts [WebResourceResponseData] to [WebResourceResponse].
WebResourceResponse _toWebResourceResponse(WebResourceResponseData data) {
  return WebResourceResponse(
    statusCode: data.statusCode,
    responseHeaders: data.responseHeaders,
    reasonPhrase: data.reasonPhrase,
    mimeType: data.mimeType,
  );
}

/// Handles initialization of Flutter APIs for Ohos WebView.
class OhosWebViewFlutterApis {
  /// Creates a [OhosWebViewFlutterApis].
  OhosWebViewFlutterApis({
    OhosObjectFlutterApiImpl? ohosObjectFlutterApi,
    DownloadListenerFlutterApiImpl? downloadListenerFlutterApi,
    WebViewClientFlutterApiImpl? webViewClientFlutterApi,
    WebChromeClientFlutterApiImpl? webChromeClientFlutterApi,
    JavaScriptChannelFlutterApiImpl? javaScriptChannelFlutterApi,
    FileChooserParamsFlutterApiImpl? fileChooserParamsFlutterApi,
    GeolocationPermissionsCallbackFlutterApiImpl?
    geolocationPermissionsCallbackFlutterApi,
    WebViewFlutterApiImpl? webViewFlutterApi,
    PermissionRequestFlutterApiImpl? permissionRequestFlutterApi,
    CustomViewCallbackFlutterApiImpl? customViewCallbackFlutterApi,
    ViewFlutterApiImpl? viewFlutterApi,
    HttpAuthHandlerFlutterApiImpl? httpAuthHandlerFlutterApi,
    SslAuthHandlerFlutterApiImpl? sslAuthHandlerFlutterApi,
  }) {
    this.ohosObjectFlutterApi =
        ohosObjectFlutterApi ?? OhosObjectFlutterApiImpl();
    this.downloadListenerFlutterApi =
        downloadListenerFlutterApi ?? DownloadListenerFlutterApiImpl();
    this.webViewClientFlutterApi =
        webViewClientFlutterApi ?? WebViewClientFlutterApiImpl();
    this.webChromeClientFlutterApi =
        webChromeClientFlutterApi ?? WebChromeClientFlutterApiImpl();
    this.javaScriptChannelFlutterApi =
        javaScriptChannelFlutterApi ?? JavaScriptChannelFlutterApiImpl();
    this.fileChooserParamsFlutterApi =
        fileChooserParamsFlutterApi ?? FileChooserParamsFlutterApiImpl();
    this.geolocationPermissionsCallbackFlutterApi =
        geolocationPermissionsCallbackFlutterApi ??
        GeolocationPermissionsCallbackFlutterApiImpl();
    this.webViewFlutterApi = webViewFlutterApi ?? WebViewFlutterApiImpl();
    this.permissionRequestFlutterApi =
        permissionRequestFlutterApi ?? PermissionRequestFlutterApiImpl();
    this.customViewCallbackFlutterApi =
        customViewCallbackFlutterApi ?? CustomViewCallbackFlutterApiImpl();
    this.viewFlutterApi = viewFlutterApi ?? ViewFlutterApiImpl();
    this.httpAuthHandlerFlutterApi =
        httpAuthHandlerFlutterApi ?? HttpAuthHandlerFlutterApiImpl();
    this.sslAuthHandlerFlutterApi =
        sslAuthHandlerFlutterApi ?? SslAuthHandlerFlutterApiImpl();
  }

  static bool _haveBeenSetUp = false;

  /// Mutable instance containing all Flutter Apis for Ohos WebView.
  ///
  /// This should only be changed for testing purposes.
  static OhosWebViewFlutterApis instance = OhosWebViewFlutterApis();

  /// Handles callback methods for native OHOS bridge objects.
  late final OhosObjectFlutterApiImpl ohosObjectFlutterApi;

  /// Flutter Api for [DownloadListener].
  late final DownloadListenerFlutterApiImpl downloadListenerFlutterApi;

  /// Flutter Api for [WebViewClient].
  late final WebViewClientFlutterApiImpl webViewClientFlutterApi;

  /// Flutter Api for [WebChromeClient].
  late final WebChromeClientFlutterApiImpl webChromeClientFlutterApi;

  /// Flutter Api for [JavaScriptChannel].
  late final JavaScriptChannelFlutterApiImpl javaScriptChannelFlutterApi;

  /// Flutter Api for [FileChooserParams].
  late final FileChooserParamsFlutterApiImpl fileChooserParamsFlutterApi;

  /// Flutter Api for [GeolocationPermissionsCallback].
  late final GeolocationPermissionsCallbackFlutterApiImpl
  geolocationPermissionsCallbackFlutterApi;

  /// Flutter Api for [WebView].
  late final WebViewFlutterApiImpl webViewFlutterApi;

  /// Flutter Api for [PermissionRequest].
  late final PermissionRequestFlutterApiImpl permissionRequestFlutterApi;

  /// Flutter Api for [CustomViewCallback].
  late final CustomViewCallbackFlutterApiImpl customViewCallbackFlutterApi;

  /// Flutter Api for [View].
  late final ViewFlutterApiImpl viewFlutterApi;

  /// Flutter Api for [HttpAuthHandler].
  late final HttpAuthHandlerFlutterApiImpl httpAuthHandlerFlutterApi;

  /// Flutter Api for [SslAuthHandler].
  late final SslAuthHandlerFlutterApiImpl sslAuthHandlerFlutterApi;

  /// Ensures all the Flutter APIs have been setup to receive calls from native code.
  void ensureSetUp() {
    if (!_haveBeenSetUp) {
      OhosWebViewCallbackDispatcher.instance.setUp(this);
      _haveBeenSetUp = true;
    }
  }
}

/// Handles method calls to native OHOS bridge objects.
class OhosObjectHostApiImpl extends OhosObjectHostApi {
  /// Constructs a [OhosObjectHostApiImpl].
  OhosObjectHostApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager,
       super(binaryMessenger: binaryMessenger);

  /// Receives binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;
}

/// Handles callback methods for native OHOS bridge objects.
class OhosObjectFlutterApiImpl {
  /// Constructs a [OhosObjectFlutterApiImpl].
  OhosObjectFlutterApiImpl({InstanceManager? instanceManager})
    : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;

  void dispose(int identifier) {
    instanceManager.remove(identifier);
  }
}

/// Host api implementation for [WebView].
class WebViewHostApiImpl extends WebViewHostApi {
  /// Constructs a [WebViewHostApiImpl].
  WebViewHostApiImpl({super.binaryMessenger, InstanceManager? instanceManager})
    : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instances ids to objects.
  Future<void> createFromInstance(WebView instance) {
    return create(instanceManager.addDartCreatedInstance(instance));
  }

  /// Helper method to convert the instances ids to objects.
  Future<void> loadDataFromInstance(
    WebView instance,
    String data,
    String? mimeType,
    String? encoding,
  ) {
    return loadData(
      instanceManager.getIdentifier(instance)!,
      data,
      mimeType,
      encoding,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> loadDataWithBaseUrlFromInstance(
    WebView instance,
    String? baseUrl,
    String data,
    String? mimeType,
    String? encoding,
    String? historyUrl,
  ) {
    return loadDataWithBaseUrl(
      instanceManager.getIdentifier(instance)!,
      baseUrl,
      data,
      mimeType,
      encoding,
      historyUrl,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> loadUrlFromInstance(
    WebView instance,
    String url,
    Map<String, String> headers,
  ) {
    return loadUrl(instanceManager.getIdentifier(instance)!, url, headers);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> postUrlFromInstance(
    WebView instance,
    String url,
    Uint8List data,
  ) {
    return postUrl(instanceManager.getIdentifier(instance)!, url, data);
  }

  /// Helper method to convert instances ids to objects.
  Future<String?> getUrlFromInstance(WebView instance) {
    return getUrl(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instances ids to objects.
  Future<bool> canGoBackFromInstance(WebView instance) {
    return canGoBack(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instances ids to objects.
  Future<bool> canGoForwardFromInstance(WebView instance) {
    return canGoForward(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> goBackFromInstance(WebView instance) {
    return goBack(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> goForwardFromInstance(WebView instance) {
    return goForward(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> reloadFromInstance(WebView instance) {
    return reload(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> clearCacheFromInstance(WebView instance, bool includeDiskFiles) {
    return clearCache(
      instanceManager.getIdentifier(instance)!,
      includeDiskFiles,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<String?> evaluateJavascriptFromInstance(
    WebView instance,
    String javascriptString,
  ) {
    return evaluateJavascript(
      instanceManager.getIdentifier(instance)!,
      javascriptString,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<String?> getTitleFromInstance(WebView instance) {
    return getTitle(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> scrollToFromInstance(WebView instance, int x, int y) {
    return scrollTo(instanceManager.getIdentifier(instance)!, x, y);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> scrollByFromInstance(WebView instance, int x, int y) {
    return scrollBy(instanceManager.getIdentifier(instance)!, x, y);
  }

  /// Helper method to convert instances ids to objects.
  Future<int> getScrollXFromInstance(WebView instance) {
    return getScrollX(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instances ids to objects.
  Future<int> getScrollYFromInstance(WebView instance) {
    return getScrollY(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instances ids to objects.
  Future<Offset> getScrollPositionFromInstance(WebView instance) async {
    final WebViewPoint position = await getScrollPosition(
      instanceManager.getIdentifier(instance)!,
    );
    return Offset(position.x.toDouble(), position.y.toDouble());
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setWebViewClientFromInstance(
    WebView instance,
    WebViewClient webViewClient,
  ) {
    return setWebViewClient(
      instanceManager.getIdentifier(instance)!,
      instanceManager.getIdentifier(webViewClient)!,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> addJavaScriptChannelFromInstance(
    WebView instance,
    JavaScriptChannel javaScriptChannel,
  ) {
    return addJavaScriptChannel(
      instanceManager.getIdentifier(instance)!,
      instanceManager.getIdentifier(javaScriptChannel)!,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> removeJavaScriptChannelFromInstance(
    WebView instance,
    JavaScriptChannel javaScriptChannel,
  ) {
    return removeJavaScriptChannel(
      instanceManager.getIdentifier(instance)!,
      instanceManager.getIdentifier(javaScriptChannel)!,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setDownloadListenerFromInstance(
    WebView instance,
    DownloadListener? listener,
  ) {
    return setDownloadListener(
      instanceManager.getIdentifier(instance)!,
      listener != null ? instanceManager.getIdentifier(listener) : null,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setWebChromeClientFromInstance(
    WebView instance,
    WebChromeClient? client,
  ) {
    return setWebChromeClient(
      instanceManager.getIdentifier(instance)!,
      client != null ? instanceManager.getIdentifier(client) : null,
    );
  }
}

/// Flutter API implementation for [WebView].
///
/// This class may handle instantiating and adding Dart instances that are
/// attached to a native instance or receiving callback methods from an
/// overridden native class.
class WebViewFlutterApiImpl {
  /// Constructs a [WebViewFlutterApiImpl].
  WebViewFlutterApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Receives binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;
  void create(int identifier) {
    instanceManager.addHostCreatedInstance(WebView.detached(), identifier);
  }

  void onScrollChanged(
    int webViewInstanceId,
    int left,
    int top,
    int oldLeft,
    int oldTop,
  ) {
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    if (webViewInstance != null) {
      _runOhosVoidCallbackSafely(
        'scroll callback',
        () => webViewInstance.onScrollChanged?.call(left, top, oldLeft, oldTop),
      );
    }
  }
}

/// Host api implementation for [WebSettings].
class WebSettingsHostApiImpl extends WebSettingsHostApi {
  /// Constructs a [WebSettingsHostApiImpl].
  WebSettingsHostApiImpl({
    super.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instances ids to objects.
  Future<void> createFromInstance(WebSettings instance, WebView webView) {
    return create(
      instanceManager.addDartCreatedInstance(instance),
      instanceManager.getIdentifier(webView)!,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setDomStorageEnabledFromInstance(
    WebSettings instance,
    bool flag,
  ) {
    return setDomStorageEnabled(instanceManager.getIdentifier(instance)!, flag);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setJavaScriptCanOpenWindowsAutomaticallyFromInstance(
    WebSettings instance,
    bool flag,
  ) {
    return setJavaScriptCanOpenWindowsAutomatically(
      instanceManager.getIdentifier(instance)!,
      flag,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setSupportMultipleWindowsFromInstance(
    WebSettings instance,
    bool support,
  ) {
    return setSupportMultipleWindows(
      instanceManager.getIdentifier(instance)!,
      support,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setBackgroundColorFromInstance(WebSettings instance, int color) {
    return setBackgroundColor(instanceManager.getIdentifier(instance)!, color);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setJavaScriptEnabledFromInstance(
    WebSettings instance,
    bool flag,
  ) {
    return setJavaScriptEnabled(instanceManager.getIdentifier(instance)!, flag);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setUserAgentStringFromInstance(
    WebSettings instance,
    String? userAgentString,
  ) {
    return setUserAgentString(
      instanceManager.getIdentifier(instance)!,
      userAgentString,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setMediaPlaybackRequiresUserGestureFromInstance(
    WebSettings instance,
    bool require,
  ) {
    return setMediaPlaybackRequiresUserGesture(
      instanceManager.getIdentifier(instance)!,
      require,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setSupportZoomFromInstance(WebSettings instance, bool support) {
    return setSupportZoom(instanceManager.getIdentifier(instance)!, support);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setSetTextZoomFromInstance(WebSettings instance, int textZoom) {
    return setTextZoom(instanceManager.getIdentifier(instance)!, textZoom);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setLoadWithOverviewModeFromInstance(
    WebSettings instance,
    bool overview,
  ) {
    return setLoadWithOverviewMode(
      instanceManager.getIdentifier(instance)!,
      overview,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setUseWideViewPortFromInstance(WebSettings instance, bool use) {
    return setUseWideViewPort(instanceManager.getIdentifier(instance)!, use);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setDisplayZoomControlsFromInstance(
    WebSettings instance,
    bool enabled,
  ) {
    return setDisplayZoomControls(
      instanceManager.getIdentifier(instance)!,
      enabled,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setBuiltInZoomControlsFromInstance(
    WebSettings instance,
    bool enabled,
  ) {
    return setBuiltInZoomControls(
      instanceManager.getIdentifier(instance)!,
      enabled,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setAllowFileAccessFromInstance(
    WebSettings instance,
    bool enabled,
  ) {
    return setAllowFileAccess(
      instanceManager.getIdentifier(instance)!,
      enabled,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<String> getUserAgentStringFromInstance(WebSettings instance) {
    return getUserAgentString(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setAllowFullScreenRotateInstance(
    WebSettings instance,
    bool enabled,
  ) {
    return setAllowFullScreenRotate(
      instanceManager.getIdentifier(instance)!,
      enabled,
    );
  }
}

/// Host api implementation for [JavaScriptChannel].
class JavaScriptChannelHostApiImpl extends JavaScriptChannelHostApi {
  /// Constructs a [JavaScriptChannelHostApiImpl].
  JavaScriptChannelHostApiImpl({
    super.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instances ids to objects.
  Future<void> createFromInstance(JavaScriptChannel instance) async {
    if (instanceManager.getIdentifier(instance) == null) {
      final int identifier = instanceManager.addDartCreatedInstance(instance);
      await create(identifier, instance.channelName);
    }
  }
}

/// Flutter api implementation for [JavaScriptChannel].
class JavaScriptChannelFlutterApiImpl {
  /// Constructs a [JavaScriptChannelFlutterApiImpl].
  JavaScriptChannelFlutterApiImpl({InstanceManager? instanceManager})
    : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;
  void postMessage(int instanceId, String message) {
    final JavaScriptChannel? instance = _callbackInstance<JavaScriptChannel>(
      instanceManager,
      instanceId,
      'JavaScriptChannel',
    );
    if (instance != null) {
      _runOhosVoidCallbackSafely(
        'JavaScript channel callback',
        () => instance.postMessage(message),
      );
    }
  }
}

/// Host api implementation for [WebViewClient].
class WebViewClientHostApiImpl extends WebViewClientHostApi {
  /// Constructs a [WebViewClientHostApiImpl].
  WebViewClientHostApiImpl({
    super.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instances ids to objects.
  Future<void> createFromInstance(WebViewClient instance) async {
    if (instanceManager.getIdentifier(instance) == null) {
      final int identifier = instanceManager.addDartCreatedInstance(instance);
      return create(identifier);
    }
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setShouldOverrideUrlLoadingReturnValueFromInstance(
    WebViewClient instance,
    bool value,
  ) {
    return setSynchronousReturnValueForShouldOverrideUrlLoading(
      instanceManager.getIdentifier(instance)!,
      value,
    );
  }
}

/// Flutter api implementation for [WebViewClient].
class WebViewClientFlutterApiImpl {
  /// Constructs a [WebViewClientFlutterApiImpl].
  WebViewClientFlutterApiImpl({InstanceManager? instanceManager})
    : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;
  void onPageFinished(int instanceId, int webViewInstanceId, String url) {
    final WebViewClient? instance = _callbackInstance<WebViewClient>(
      instanceManager,
      instanceId,
      'WebViewClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    if (instance != null && webViewInstance != null) {
      _runOhosVoidCallbackSafely(
        'page finished callback',
        () => instance.onPageFinished?.call(webViewInstance, url),
      );
    }
  }

  void onPageStarted(int instanceId, int webViewInstanceId, String url) {
    final WebViewClient? instance = _callbackInstance<WebViewClient>(
      instanceManager,
      instanceId,
      'WebViewClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    if (instance != null && webViewInstance != null) {
      _runOhosVoidCallbackSafely(
        'page started callback',
        () => instance.onPageStarted?.call(webViewInstance, url),
      );
    }
  }

  void onReceivedError(
    int instanceId,
    int webViewInstanceId,
    int errorCode,
    String description,
    String failingUrl,
  ) {
    final WebViewClient? instance = _callbackInstance<WebViewClient>(
      instanceManager,
      instanceId,
      'WebViewClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    if (instance != null &&
        webViewInstance != null &&
        // ignore: deprecated_member_use_from_same_package
        instance.onReceivedError != null) {
      _runOhosVoidCallbackSafely(
        'resource error callback',
        // ignore: deprecated_member_use_from_same_package
        () => instance.onReceivedError!(
          webViewInstance,
          errorCode,
          description,
          failingUrl,
        ),
      );
    }
  }

  void onReceivedRequestError(
    int instanceId,
    int webViewInstanceId,
    WebResourceRequestData request,
    WebResourceErrorData error,
  ) {
    final WebViewClient? instance = _callbackInstance<WebViewClient>(
      instanceManager,
      instanceId,
      'WebViewClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    if (instance != null &&
        webViewInstance != null &&
        instance.onReceivedRequestError != null) {
      _runOhosVoidCallbackSafely(
        'request error callback',
        () => instance.onReceivedRequestError!(
          webViewInstance,
          _toWebResourceRequest(request),
          _toWebResourceError(error),
        ),
      );
    }
  }

  void onReceivedHttpError(
    int instanceId,
    int webViewInstanceId,
    WebResourceRequestData request,
    WebResourceResponseData response,
  ) {
    final WebViewClient? instance = _callbackInstance<WebViewClient>(
      instanceManager,
      instanceId,
      'WebViewClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    if (instance != null &&
        webViewInstance != null &&
        instance.onReceivedHttpError != null) {
      _runOhosVoidCallbackSafely(
        'HTTP error callback',
        () => instance.onReceivedHttpError!(
          webViewInstance,
          _toWebResourceRequest(request),
          _toWebResourceResponse(response),
        ),
      );
    }
  }

  void requestLoading(
    int instanceId,
    int webViewInstanceId,
    WebResourceRequestData request,
  ) {
    final WebViewClient? instance = _callbackInstance<WebViewClient>(
      instanceManager,
      instanceId,
      'WebViewClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    if (instance != null &&
        webViewInstance != null &&
        instance.requestLoading != null) {
      _runOhosVoidCallbackSafely(
        'request loading callback',
        () => instance.requestLoading!(
          webViewInstance,
          _toWebResourceRequest(request),
        ),
      );
    }
  }

  void urlLoading(int instanceId, int webViewInstanceId, String url) {
    final WebViewClient? instance = _callbackInstance<WebViewClient>(
      instanceManager,
      instanceId,
      'WebViewClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    if (instance != null && webViewInstance != null) {
      _runOhosVoidCallbackSafely(
        'URL loading callback',
        () => instance.urlLoading?.call(webViewInstance, url),
      );
    }
  }

  void doUpdateVisitedHistory(
    int instanceId,
    int webViewInstanceId,
    String url,
    bool isReload,
  ) {
    final WebViewClient? instance = _callbackInstance<WebViewClient>(
      instanceManager,
      instanceId,
      'WebViewClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    if (instance != null && webViewInstance != null) {
      _runOhosVoidCallbackSafely(
        'visited history callback',
        () => instance.doUpdateVisitedHistory?.call(
          webViewInstance,
          url,
          isReload,
        ),
      );
    }
  }

  void onReceivedHttpAuthRequest(
    int instanceId,
    int webViewInstanceId,
    int httpAuthHandlerInstanceId,
    String host,
    String realm,
  ) {
    final WebViewClient? instance = _callbackInstance<WebViewClient>(
      instanceManager,
      instanceId,
      'WebViewClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    final HttpAuthHandler? httpAuthHandlerInstance =
        _callbackInstance<HttpAuthHandler>(
          instanceManager,
          httpAuthHandlerInstanceId,
          'HttpAuthHandler',
        );
    if (httpAuthHandlerInstance == null) {
      return;
    }
    if (instance == null || webViewInstance == null) {
      _runCallbackFallback(
        () => httpAuthHandlerInstance.cancel(),
        'HTTP authentication fallback',
      );
      return;
    }
    final void Function(WebView, HttpAuthHandler, String, String)? callback =
        instance.onReceivedHttpAuthRequest;
    if (callback == null) {
      _runCallbackFallback(
        () => httpAuthHandlerInstance.cancel(),
        'HTTP authentication fallback',
      );
      return;
    }
    try {
      callback(webViewInstance, httpAuthHandlerInstance, host, realm);
    } catch (error) {
      _reportOhosApiCallbackError('HTTP authentication callback', error);
      _runCallbackFallback(
        () => httpAuthHandlerInstance.cancel(),
        'HTTP authentication fallback',
      );
    }
  }

  void onReceivedSslAuthError(
    int instanceId,
    int webViewInstanceId,
    int sslAuthHandlerInstanceId,
    String url,
    int errorCode,
    String description,
  ) {
    final WebViewClient? instance = _callbackInstance<WebViewClient>(
      instanceManager,
      instanceId,
      'WebViewClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    final SslAuthHandler? sslAuthHandlerInstance =
        _callbackInstance<SslAuthHandler>(
          instanceManager,
          sslAuthHandlerInstanceId,
          'SslAuthHandler',
        );
    if (sslAuthHandlerInstance == null) {
      return;
    }
    if (instance == null || webViewInstance == null) {
      _runCallbackFallback(
        () => sslAuthHandlerInstance.cancel(),
        'SSL authentication fallback',
      );
      return;
    }
    final void Function(WebView, SslAuthHandler, String, int, String)?
    callback = instance.onReceivedSslAuthError;
    if (callback == null) {
      _runCallbackFallback(
        () => sslAuthHandlerInstance.cancel(),
        'SSL authentication fallback',
      );
      return;
    }
    try {
      callback(
        webViewInstance,
        sslAuthHandlerInstance,
        url,
        errorCode,
        description,
      );
    } catch (error) {
      _reportOhosApiCallbackError('SSL authentication callback', error);
      _runCallbackFallback(
        () => sslAuthHandlerInstance.cancel(),
        'SSL authentication fallback',
      );
    }
  }
}

/// Host api implementation for [DownloadListener].
class DownloadListenerHostApiImpl extends DownloadListenerHostApi {
  /// Constructs a [DownloadListenerHostApiImpl].
  DownloadListenerHostApiImpl({
    super.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instances ids to objects.
  Future<void> createFromInstance(DownloadListener instance) async {
    if (instanceManager.getIdentifier(instance) == null) {
      final int identifier = instanceManager.addDartCreatedInstance(instance);
      return create(identifier);
    }
  }
}

/// Flutter api implementation for [DownloadListener].
class DownloadListenerFlutterApiImpl {
  /// Constructs a [DownloadListenerFlutterApiImpl].
  DownloadListenerFlutterApiImpl({InstanceManager? instanceManager})
    : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;
  void onDownloadStart(
    int instanceId,
    String url,
    String userAgent,
    String contentDisposition,
    String mimetype,
    int contentLength,
  ) {
    final DownloadListener? instance = _callbackInstance<DownloadListener>(
      instanceManager,
      instanceId,
      'DownloadListener',
    );
    if (instance != null) {
      _runOhosVoidCallbackSafely(
        'download callback',
        () => instance.onDownloadStart(
          url,
          userAgent,
          contentDisposition,
          mimetype,
          contentLength,
        ),
      );
    }
  }
}

/// Host api implementation for [DownloadListener].
class WebChromeClientHostApiImpl extends WebChromeClientHostApi {
  /// Constructs a [WebChromeClientHostApiImpl].
  WebChromeClientHostApiImpl({
    super.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instances ids to objects.
  Future<void> createFromInstance(WebChromeClient instance) async {
    if (instanceManager.getIdentifier(instance) == null) {
      final int identifier = instanceManager.addDartCreatedInstance(instance);
      return create(identifier);
    }
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setSynchronousReturnValueForOnShowFileChooserFromInstance(
    WebChromeClient instance,
    bool value,
  ) {
    return setSynchronousReturnValueForOnShowFileChooser(
      instanceManager.getIdentifier(instance)!,
      value,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setSynchronousReturnValueForOnConsoleMessageFromInstance(
    WebChromeClient instance,
    bool value,
  ) {
    return setSynchronousReturnValueForOnConsoleMessage(
      instanceManager.getIdentifier(instance)!,
      value,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setSynchronousReturnValueForOnJsAlertFromInstance(
    WebChromeClient instance,
    bool value,
  ) {
    return setSynchronousReturnValueForOnJsAlert(
      instanceManager.getIdentifier(instance)!,
      value,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setSynchronousReturnValueForOnJsConfirmFromInstance(
    WebChromeClient instance,
    bool value,
  ) {
    return setSynchronousReturnValueForOnJsConfirm(
      instanceManager.getIdentifier(instance)!,
      value,
    );
  }

  /// Helper method to convert instances ids to objects.
  Future<void> setSynchronousReturnValueForOnJsPromptFromInstance(
    WebChromeClient instance,
    bool value,
  ) {
    return setSynchronousReturnValueForOnJsPrompt(
      instanceManager.getIdentifier(instance)!,
      value,
    );
  }
}

/// Flutter api implementation for [DownloadListener].
class WebChromeClientFlutterApiImpl {
  /// Constructs a [DownloadListenerFlutterApiImpl].
  WebChromeClientFlutterApiImpl({InstanceManager? instanceManager})
    : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;
  void onProgressChanged(int instanceId, int webViewInstanceId, int progress) {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      instanceId,
      'WebChromeClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    if (instance != null && webViewInstance != null) {
      _runOhosVoidCallbackSafely(
        'progress callback',
        () => instance.onProgressChanged?.call(webViewInstance, progress),
      );
    }
  }

  Future<List<String?>> onShowFileChooser(
    int instanceId,
    int webViewInstanceId,
    int paramsInstanceId,
  ) async {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      instanceId,
      'WebChromeClient',
    );
    final WebView? webViewInstance = _callbackInstance<WebView>(
      instanceManager,
      webViewInstanceId,
      'WebView',
    );
    final FileChooserParams? params = _callbackInstance<FileChooserParams>(
      instanceManager,
      paramsInstanceId,
      'FileChooserParams',
    );
    if (instance != null &&
        webViewInstance != null &&
        params != null &&
        instance.onShowFileChooser != null) {
      try {
        return await instance.onShowFileChooser!(webViewInstance, params);
      } catch (error) {
        _reportOhosApiCallbackError('file chooser callback', error);
      }
    }

    return const <String?>[];
  }

  void onGeolocationPermissionsShowPrompt(
    int instanceId,
    int paramsInstanceId,
    String origin,
  ) {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      instanceId,
      'WebChromeClient',
    );
    final GeolocationPermissionsCallback? callback =
        _callbackInstance<GeolocationPermissionsCallback>(
          instanceManager,
          paramsInstanceId,
          'GeolocationPermissionsCallback',
        );
    if (callback == null) {
      return;
    }
    final GeolocationPermissionsShowPrompt? onShowPrompt =
        instance?.onGeolocationPermissionsShowPrompt;
    if (onShowPrompt == null) {
      _runCallbackFallback(
        () => callback.invoke(origin, false, false),
        'geolocation permission fallback',
      );
    } else {
      _runOhosAsyncCallbackWithFallback(
        'geolocation permission callback',
        () => onShowPrompt(origin, callback),
        () => callback.invoke(origin, false, false),
      );
    }
  }

  void onGeolocationPermissionsHidePrompt(int identifier) {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      identifier,
      'WebChromeClient',
    );
    final GeolocationPermissionsHidePrompt? onHidePrompt =
        instance?.onGeolocationPermissionsHidePrompt;
    if (onHidePrompt != null) {
      _runOhosVoidCallbackSafely(
        'geolocation hide prompt callback',
        () => onHidePrompt(instance!),
      );
    }
  }

  void onPermissionRequest(int instanceId, int requestInstanceId) {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      instanceId,
      'WebChromeClient',
    );
    final PermissionRequest? request = _callbackInstance<PermissionRequest>(
      instanceManager,
      requestInstanceId,
      'PermissionRequest',
    );
    if (request == null) {
      return;
    }
    if (instance?.onPermissionRequest == null) {
      _runCallbackFallback(() => request.deny(), 'permission request fallback');
      return;
    }
    try {
      instance!.onPermissionRequest!(instance, request);
    } catch (error) {
      _reportOhosApiCallbackError('permission request callback', error);
      _runCallbackFallback(() => request.deny(), 'permission request fallback');
    }
  }

  void onShowCustomView(
    int instanceId,
    int viewIdentifier,
    int callbackIdentifier,
  ) {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      instanceId,
      'WebChromeClient',
    );
    final View? view = _callbackInstance<View>(
      instanceManager,
      viewIdentifier,
      'View',
    );
    final CustomViewCallback? callback = _callbackInstance<CustomViewCallback>(
      instanceManager,
      callbackIdentifier,
      'CustomViewCallback',
    );
    if (instance?.onShowCustomView != null &&
        view != null &&
        callback != null) {
      try {
        instance!.onShowCustomView!(instance, view, callback);
        return;
      } catch (error) {
        _reportOhosApiCallbackError('custom view callback', error);
      }
    }
    if (callback != null) {
      _runCallbackFallback(
        () => callback.onCustomViewHidden(),
        'custom view fallback',
      );
    }
  }

  void onHideCustomView(int instanceId) {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      instanceId,
      'WebChromeClient',
    );
    if (instance?.onHideCustomView != null) {
      _runOhosVoidCallbackSafely(
        'hide custom view callback',
        () => instance!.onHideCustomView!(instance),
      );
    }
  }

  void onConsoleMessage(int instanceId, ConsoleMessage message) {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      instanceId,
      'WebChromeClient',
    );
    if (instance != null) {
      _runOhosVoidCallbackSafely(
        'console message callback',
        () => instance.onConsoleMessage?.call(instance, message),
      );
    }
  }

  Future<void> onJsAlert(int instanceId, String url, String message) async {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      instanceId,
      'WebChromeClient',
    );
    try {
      await instance?.onJsAlert?.call(url, message);
    } catch (error) {
      _reportOhosApiCallbackError('JavaScript alert callback', error);
    }
  }

  Future<bool> onJsConfirm(int instanceId, String url, String message) async {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      instanceId,
      'WebChromeClient',
    );
    try {
      return await instance?.onJsConfirm?.call(url, message) ?? false;
    } catch (error) {
      _reportOhosApiCallbackError('JavaScript confirm callback', error);
      return false;
    }
  }

  Future<String?> onJsPrompt(
    int instanceId,
    String url,
    String message,
    String defaultValue,
  ) async {
    final WebChromeClient? instance = _callbackInstance<WebChromeClient>(
      instanceManager,
      instanceId,
      'WebChromeClient',
    );
    try {
      final callback = instance?.onJsPrompt;
      if (callback == null) return defaultValue;
      return await callback(url, message, defaultValue);
    } catch (error) {
      _reportOhosApiCallbackError('JavaScript prompt callback', error);
      return defaultValue;
    }
  }
}

/// Host api implementation for [WebStorage].
class WebStorageHostApiImpl extends WebStorageHostApi {
  /// Constructs a [WebStorageHostApiImpl].
  WebStorageHostApiImpl({
    super.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instances ids to objects.
  Future<void> createFromInstance(WebStorage instance) async {
    if (instanceManager.getIdentifier(instance) == null) {
      final int identifier = instanceManager.addDartCreatedInstance(instance);
      return create(identifier);
    }
  }

  /// Helper method to convert instances ids to objects.
  Future<void> deleteAllDataFromInstance(WebStorage instance) {
    return deleteAllData(instanceManager.getIdentifier(instance)!);
  }
}

/// Flutter api implementation for [FileChooserParams].
class FileChooserParamsFlutterApiImpl {
  /// Constructs a [FileChooserParamsFlutterApiImpl].
  FileChooserParamsFlutterApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Receives binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;
  void create(
    int instanceId,
    bool isCaptureEnabled,
    List<String?> acceptTypes,
    FileChooserMode mode,
    String? filenameHint,
  ) {
    instanceManager.addHostCreatedInstance(
      FileChooserParams.detached(
        isCaptureEnabled: isCaptureEnabled,
        acceptTypes: acceptTypes.cast(),
        mode: mode,
        filenameHint: filenameHint,
        binaryMessenger: binaryMessenger,
        instanceManager: instanceManager,
      ),
      instanceId,
    );
  }
}

/// Host api implementation for [GeolocationPermissionsCallback].
class GeolocationPermissionsCallbackHostApiImpl
    extends GeolocationPermissionsCallbackHostApi {
  /// Constructs a [GeolocationPermissionsCallbackHostApiImpl].
  GeolocationPermissionsCallbackHostApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager,
       super(binaryMessenger: binaryMessenger);

  /// Sends binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with java objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instances ids to objects.
  Future<void> invokeFromInstances(
    GeolocationPermissionsCallback instance,
    String origin,
    bool allow,
    bool retain,
  ) {
    return invoke(
      instanceManager.getIdentifier(instance)!,
      origin,
      allow,
      retain,
    );
  }
}

/// Flutter API implementation for [GeolocationPermissionsCallback].
///
/// This class may handle instantiating and adding Dart instances that are
/// attached to a native instance or receiving callback methods from an
/// overridden native class.
class GeolocationPermissionsCallbackFlutterApiImpl {
  /// Constructs a [GeolocationPermissionsCallbackFlutterApiImpl].
  GeolocationPermissionsCallbackFlutterApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Receives binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;
  void create(int instanceId) {
    instanceManager.addHostCreatedInstance(
      GeolocationPermissionsCallback.detached(
        binaryMessenger: binaryMessenger,
        instanceManager: instanceManager,
      ),
      instanceId,
    );
  }
}

/// Host api implementation for [PermissionRequest].
class PermissionRequestHostApiImpl extends PermissionRequestHostApi {
  /// Constructs a [PermissionRequestHostApiImpl].
  PermissionRequestHostApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager,
       super(binaryMessenger: binaryMessenger);

  /// Sends binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instance ids to objects.
  Future<void> grantFromInstances(
    PermissionRequest instance,
    List<String> resources,
  ) {
    return grant(instanceManager.getIdentifier(instance)!, resources);
  }

  /// Helper method to convert instance ids to objects.
  Future<void> denyFromInstances(PermissionRequest instance) {
    return deny(instanceManager.getIdentifier(instance)!);
  }
}

/// Flutter API implementation for [PermissionRequest].
///
/// This class may handle instantiating and adding Dart instances that are
/// attached to a native instance or receiving callback methods from an
/// overridden native class.
class PermissionRequestFlutterApiImpl {
  /// Constructs a [PermissionRequestFlutterApiImpl].
  PermissionRequestFlutterApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Receives binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;
  void create(int identifier, List<String?> resources) {
    instanceManager.addHostCreatedInstance(
      PermissionRequest.detached(
        resources: resources.cast<String>(),
        binaryMessenger: binaryMessenger,
        instanceManager: instanceManager,
      ),
      identifier,
    );
  }
}

/// Host api implementation for [CustomViewCallback].
class CustomViewCallbackHostApiImpl extends CustomViewCallbackHostApi {
  /// Constructs a [CustomViewCallbackHostApiImpl].
  CustomViewCallbackHostApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager,
       super(binaryMessenger: binaryMessenger);

  /// Sends binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instance ids to objects.
  Future<void> onCustomViewHiddenFromInstances(CustomViewCallback instance) {
    return onCustomViewHidden(instanceManager.getIdentifier(instance)!);
  }
}

/// Flutter API implementation for [CustomViewCallback].
///
/// This class may handle instantiating and adding Dart instances that are
/// attached to a native instance or receiving callback methods from an
/// overridden native class.
class CustomViewCallbackFlutterApiImpl {
  /// Constructs a [CustomViewCallbackFlutterApiImpl].
  CustomViewCallbackFlutterApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Receives binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;
  void create(int identifier) {
    instanceManager.addHostCreatedInstance(
      CustomViewCallback.detached(
        binaryMessenger: binaryMessenger,
        instanceManager: instanceManager,
      ),
      identifier,
    );
  }
}

/// Flutter API implementation for [View].
///
/// This class may handle instantiating and adding Dart instances that are
/// attached to a native instance or receiving callback methods from an
/// overridden native class.
class ViewFlutterApiImpl {
  /// Constructs a [ViewFlutterApiImpl].
  ViewFlutterApiImpl({this.binaryMessenger, InstanceManager? instanceManager})
    : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Receives binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;
  void create(int identifier) {
    instanceManager.addHostCreatedInstance(
      View.detached(
        binaryMessenger: binaryMessenger,
        instanceManager: instanceManager,
      ),
      identifier,
    );
  }
}

/// Host api implementation for [CookieManager].
class CookieManagerHostApiImpl extends CookieManagerHostApi {
  /// Constructs a [CookieManagerHostApiImpl].
  CookieManagerHostApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager,
       super(binaryMessenger: binaryMessenger);

  /// Sends binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;

  /// Helper method to convert instance ids to objects.
  CookieManager attachInstanceFromInstances(CookieManager instance) {
    attachInstance(instanceManager.addDartCreatedInstance(instance));
    return instance;
  }

  /// Helper method to convert instance ids to objects.
  Future<void> setCookieFromInstances(
    CookieManager instance,
    String url,
    String value,
  ) {
    return setCookie(instanceManager.getIdentifier(instance)!, url, value);
  }

  /// Helper method to convert instance ids to objects.
  Future<String> getCookiesFromInstances(CookieManager instance, String url) {
    return getCookies(instanceManager.getIdentifier(instance)!, url);
  }

  /// Helper method to convert instance ids to objects.
  Future<bool> removeAllCookiesFromInstances(CookieManager instance) {
    return removeAllCookies(instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instance ids to objects.
  Future<void> setAcceptThirdPartyCookiesFromInstances(
    CookieManager instance,
    WebView webView,
    bool accept,
  ) {
    return setAcceptThirdPartyCookies(
      instanceManager.getIdentifier(instance)!,
      instanceManager.getIdentifier(webView)!,
      accept,
    );
  }
}

/// Host api implementation for [HttpAuthHandler].
class HttpAuthHandlerHostApiImpl extends HttpAuthHandlerHostApi {
  /// Constructs a [HttpAuthHandlerHostApiImpl].
  HttpAuthHandlerHostApiImpl({
    super.binaryMessenger,
    InstanceManager? instanceManager,
  }) : _instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager _instanceManager;

  /// Helper method to convert instance ids to objects.
  Future<void> cancelFromInstance(HttpAuthHandler instance) {
    return cancel(_instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instance ids to objects.
  Future<void> proceedFromInstance(
    HttpAuthHandler instance,
    String username,
    String password,
  ) {
    return proceed(
      _instanceManager.getIdentifier(instance)!,
      username,
      password,
    );
  }

  /// Helper method to convert instance ids to objects.
  Future<bool> useHttpAuthUsernamePasswordFromInstance(
    HttpAuthHandler instance,
  ) {
    return useHttpAuthUsernamePassword(
      _instanceManager.getIdentifier(instance)!,
    );
  }
}

/// Flutter API implementation for [HttpAuthHandler].
class HttpAuthHandlerFlutterApiImpl {
  /// Constructs a [HttpAuthHandlerFlutterApiImpl].
  HttpAuthHandlerFlutterApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Receives binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;
  void create(int instanceId) {
    instanceManager.addHostCreatedInstance(HttpAuthHandler(), instanceId);
  }
}

/// Host api implementation for [SslAuthHandler].
class SslAuthHandlerHostApiImpl extends SslAuthHandlerHostApi {
  /// Constructs a [SslAuthHandlerHostApiImpl].
  SslAuthHandlerHostApiImpl({
    super.binaryMessenger,
    InstanceManager? instanceManager,
  }) : _instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager _instanceManager;

  /// Helper method to convert instance ids to objects.
  Future<void> cancelFromInstance(SslAuthHandler instance) {
    return cancel(_instanceManager.getIdentifier(instance)!);
  }

  /// Helper method to convert instance ids to objects.
  Future<void> proceedFromInstance(SslAuthHandler instance) {
    return proceed(_instanceManager.getIdentifier(instance)!);
  }
}

/// Flutter API implementation for [SslAuthHandler].
class SslAuthHandlerFlutterApiImpl {
  /// Constructs a [SslAuthHandlerFlutterApiImpl].
  SslAuthHandlerFlutterApiImpl({
    this.binaryMessenger,
    InstanceManager? instanceManager,
  }) : instanceManager = instanceManager ?? OhosObject.globalInstanceManager;

  /// Receives binary data across the Flutter platform barrier.
  ///
  /// If it is null, the default BinaryMessenger will be used which routes to
  /// the host platform.
  final BinaryMessenger? binaryMessenger;

  /// Maintains instances stored to communicate with native language objects.
  final InstanceManager instanceManager;

  void create(int instanceId) {
    instanceManager.addHostCreatedInstance(
      SslAuthHandler(
        binaryMessenger: binaryMessenger,
        instanceManager: instanceManager,
      ),
      instanceId,
    );
  }
}
