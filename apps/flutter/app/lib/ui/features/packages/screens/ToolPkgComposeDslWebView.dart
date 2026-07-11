// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../core/concurrency/AppWorkers.dart';
import '../../../../core/path/OperitClientPaths.dart';
import 'ToolPkgComposeDslWebViewResourceServer.dart';

const String composeDslWebViewInternalBridgeName =
    '__ComposeDslWebViewHostBridge__';

const String _composeDslWebViewBridgeChannelName =
    '__ComposeDslWebViewHostBridgeChannel__';
const String _composeDslWebViewBridgeHtmlMarker =
    'data-operit-webview-bridge-runtime="1"';

typedef ComposeDslWebViewActionDispatcher =
    Future<Object?> Function(String actionId, [Object? payload]);
typedef ComposeDslWebViewRuntimeOptionsProvider =
    Map<String, Object?> Function();

class ComposeDslWebViewHostContext {
  const ComposeDslWebViewHostContext({
    required this.routeInstanceId,
    required this.executionContextKey,
    required this.dispatchAction,
    required this.runtimeOptionsProvider,
  });

  final String routeInstanceId;
  final String executionContextKey;
  final ComposeDslWebViewActionDispatcher dispatchAction;
  final ComposeDslWebViewRuntimeOptionsProvider runtimeOptionsProvider;

  Future<ComposeDslWebViewBlockingActionResult> executeAction({
    required String actionId,
    Object? payload,
  }) async {
    final normalizedActionId = actionId.trim();
    if (normalizedActionId.isEmpty) {
      return const ComposeDslWebViewBlockingActionResult(
        actionResult: null,
        message: 'compose action id is required',
      );
    }
    try {
      return ComposeDslWebViewBlockingActionResult(
        actionResult: await dispatchAction(normalizedActionId, payload),
        message: null,
      );
    } catch (error) {
      final message = error.toString().trim();
      return ComposeDslWebViewBlockingActionResult(
        actionResult: null,
        message: message.isEmpty ? 'compose action dispatch failed' : message,
      );
    }
  }
}

class ComposeDslWebViewBlockingActionResult {
  const ComposeDslWebViewBlockingActionResult({
    required this.actionResult,
    required this.message,
  });

  final Object? actionResult;
  final String? message;
}

class ComposeDslWebViewStateSnapshot {
  const ComposeDslWebViewStateSnapshot({
    required this.url,
    required this.title,
    required this.loading,
    required this.progress,
    required this.canGoBack,
    required this.canGoForward,
  });

  final String? url;
  final String? title;
  final bool loading;
  final int progress;
  final bool canGoBack;
  final bool canGoForward;

  Map<String, Object?> toPayload() => <String, Object?>{
    'url': url,
    'title': title,
    'loading': loading,
    'progress': progress.clamp(0, 100),
    'canGoBack': canGoBack,
    'canGoForward': canGoForward,
  };

  @override
  bool operator ==(Object other) {
    return other is ComposeDslWebViewStateSnapshot &&
        other.url == url &&
        other.title == title &&
        other.loading == loading &&
        other.progress == progress &&
        other.canGoBack == canGoBack &&
        other.canGoForward == canGoForward;
  }

  @override
  int get hashCode =>
      Object.hash(url, title, loading, progress, canGoBack, canGoForward);
}

class ComposeDslWebViewHostRegistry {
  ComposeDslWebViewHostRegistry._();

  static bool _hostInteractionRegistered = false;
  static final Map<String, Map<String, _ComposeDslWebViewControllerBinding>>
  _bindings = <String, Map<String, _ComposeDslWebViewControllerBinding>>{};
  static final Map<String, Map<String, Map<String, Map<String, String>>>>
  _javascriptInterfaceActionIds =
      <String, Map<String, Map<String, Map<String, String>>>>{};

  static void ensureHostInteractionRegistered() {
    if (_hostInteractionRegistered) {
      return;
    }
    _hostInteractionRegistered = true;
  }

  static void bind({
    required String executionContextKey,
    required String routeInstanceId,
    required String controllerKey,
    required WebViewController controller,
    required ComposeDslWebViewStateSnapshot state,
  }) {
    if (executionContextKey.trim().isEmpty || controllerKey.trim().isEmpty) {
      return;
    }
    final scopedBindings = _bindings.putIfAbsent(
      executionContextKey,
      () => <String, _ComposeDslWebViewControllerBinding>{},
    );
    final registeredInterfaces =
        _javascriptInterfaceActionIds[executionContextKey]?[controllerKey] ??
        const <String, Map<String, String>>{};
    scopedBindings[controllerKey] = _ComposeDslWebViewControllerBinding(
      routeInstanceId: routeInstanceId,
      executionContextKey: executionContextKey,
      controllerKey: controllerKey,
      controller: controller,
      state: state,
      javascriptInterfaceActionIds: registeredInterfaces.map(
        (name, methods) => MapEntry(name, Map<String, String>.of(methods)),
      ),
    );
  }

  static void unbind({
    required String executionContextKey,
    required String controllerKey,
    required WebViewController controller,
  }) {
    final scopedBindings = _bindings[executionContextKey];
    if (scopedBindings == null) {
      return;
    }
    final current = scopedBindings[controllerKey];
    if (current?.controller == controller) {
      scopedBindings.remove(controllerKey);
    }
    if (scopedBindings.isEmpty) {
      _bindings.remove(executionContextKey);
    }
  }

  static void clearExecutionContext(String executionContextKey) {
    if (executionContextKey.trim().isEmpty) {
      return;
    }
    _bindings.remove(executionContextKey);
    _javascriptInterfaceActionIds.remove(executionContextKey);
  }

  static void updateState({
    required String executionContextKey,
    required String controllerKey,
    required ComposeDslWebViewStateSnapshot state,
  }) {
    _bindings[executionContextKey]?[controllerKey]?.state = state;
  }

  static String? findJavascriptInterfaceActionId({
    required String executionContextKey,
    required String controllerKey,
    required String interfaceName,
    required String methodName,
  }) {
    final normalizedInterfaceName = interfaceName.trim();
    final normalizedMethodName = methodName.trim();
    if (executionContextKey.trim().isEmpty ||
        controllerKey.trim().isEmpty ||
        normalizedInterfaceName.isEmpty ||
        normalizedMethodName.isEmpty) {
      return null;
    }
    final boundActionId = _bindings[executionContextKey]?[controllerKey]
        ?.javascriptInterfaceActionIds[normalizedInterfaceName]?[normalizedMethodName]
        ?.trim();
    if (boundActionId != null && boundActionId.isNotEmpty) {
      return boundActionId;
    }
    final storedActionId =
        _javascriptInterfaceActionIds[executionContextKey]?[controllerKey]?[normalizedInterfaceName]?[normalizedMethodName]
            ?.trim();
    return storedActionId != null && storedActionId.isNotEmpty
        ? storedActionId
        : null;
  }

  static bool registerJavascriptInterface({
    required String executionContextKey,
    required String controllerKey,
    required String interfaceName,
    required Map<String, String> methodActionIds,
  }) {
    final normalizedInterfaceName = interfaceName.trim();
    final normalizedMethods = <String, String>{};
    for (final entry in methodActionIds.entries) {
      final methodName = entry.key.trim();
      final actionId = entry.value.trim();
      if (methodName.isNotEmpty && actionId.isNotEmpty) {
        normalizedMethods[methodName] = actionId;
      }
    }
    if (executionContextKey.trim().isEmpty ||
        controllerKey.trim().isEmpty ||
        normalizedInterfaceName.isEmpty ||
        normalizedMethods.isEmpty) {
      return false;
    }
    final scopedInterfaces = _javascriptInterfaceActionIds.putIfAbsent(
      executionContextKey,
      () => <String, Map<String, Map<String, String>>>{},
    );
    final controllerInterfaces = scopedInterfaces.putIfAbsent(
      controllerKey,
      () => <String, Map<String, String>>{},
    );
    controllerInterfaces[normalizedInterfaceName] = normalizedMethods;
    _bindings[executionContextKey]?[controllerKey]
            ?.javascriptInterfaceActionIds[normalizedInterfaceName] =
        Map<String, String>.of(normalizedMethods);
    return true;
  }

  static bool unregisterJavascriptInterface({
    required String executionContextKey,
    required String controllerKey,
    required String interfaceName,
  }) {
    final normalizedInterfaceName = interfaceName.trim();
    if (executionContextKey.trim().isEmpty ||
        controllerKey.trim().isEmpty ||
        normalizedInterfaceName.isEmpty) {
      return false;
    }
    final bindingRemoved =
        _bindings[executionContextKey]?[controllerKey]
            ?.javascriptInterfaceActionIds
            .remove(normalizedInterfaceName) !=
        null;
    final controllerInterfaces =
        _javascriptInterfaceActionIds[executionContextKey]?[controllerKey];
    final storedRemoved =
        controllerInterfaces?.remove(normalizedInterfaceName) != null;
    if (controllerInterfaces?.isEmpty == true) {
      _javascriptInterfaceActionIds[executionContextKey]?.remove(controllerKey);
    }
    if (_javascriptInterfaceActionIds[executionContextKey]?.isEmpty == true) {
      _javascriptInterfaceActionIds.remove(executionContextKey);
    }
    return bindingRemoved || storedRemoved;
  }

  static Map<String, List<String>> listJavascriptInterfaces({
    required String executionContextKey,
    required String controllerKey,
  }) {
    final interfaces =
        _bindings[executionContextKey]?[controllerKey]
            ?.javascriptInterfaceActionIds ??
        _javascriptInterfaceActionIds[executionContextKey]?[controllerKey];
    if (interfaces == null) {
      return const <String, List<String>>{};
    }
    return interfaces.map((name, methods) {
      final methodNames = methods.keys.toList(growable: false)..sort();
      return MapEntry(name, methodNames);
    });
  }

  static Future<String> handleControllerCommand(String payloadJson) async {
    final payload = await AppWorkers.run(
      () => _decodeJsonObject(payloadJson),
      debugName: 'compose-webview-controller-command-decode',
    );
    if (payload == null) {
      return _bridgeError('invalid webview controller command payload');
    }
    final executionContextKey = _string(payload['executionContextKey']).trim();
    final controllerKey = _string(payload['key']).trim();
    final command = _string(payload['command']).trim();
    if (executionContextKey.isEmpty ||
        controllerKey.isEmpty ||
        command.isEmpty) {
      return _bridgeError(
        'webview controller command is missing required fields',
      );
    }
    final commandPayload = _stringMap(payload['payload']);
    final binding = _bindings[executionContextKey]?[controllerKey];
    if (binding == null) {
      return switch (command) {
        'getState' => _bridgeSuccess(null),
        'addJavascriptInterface' => _registerJavascriptInterfaceCommand(
          executionContextKey: executionContextKey,
          controllerKey: controllerKey,
          payload: commandPayload,
        ),
        'removeJavascriptInterface' => _unregisterJavascriptInterfaceCommand(
          executionContextKey: executionContextKey,
          controllerKey: controllerKey,
          payload: commandPayload,
        ),
        _ => _bridgeError(
          "webview controller '$controllerKey' is not bound in route '$executionContextKey'",
        ),
      };
    }
    final controller = binding.controller;
    try {
      switch (command) {
        case 'loadUrl':
          final url = _string(commandPayload['url']).trim();
          if (url.isEmpty) {
            return _bridgeError(
              'webview controller loadUrl requires a non-empty url',
            );
          }
          await controller.loadRequest(
            Uri.parse(url),
            headers: _toStringMap(commandPayload['headers']),
          );
          return _bridgeSuccess(null);
        case 'loadHtml':
          final html = _string(commandPayload['html']);
          final options = _stringMap(commandPayload['options']);
          await controller.loadHtmlString(
            _injectComposeDslWebViewBridgeRuntimeIntoHtml(html),
            baseUrl: _string(options['baseUrl']).trim().ifNotEmpty,
          );
          return _bridgeSuccess(null);
        case 'reload':
          await controller.reload();
          return _bridgeSuccess(null);
        case 'goBack':
          if (await controller.canGoBack()) {
            await controller.goBack();
          }
          return _bridgeSuccess(null);
        case 'goForward':
          if (await controller.canGoForward()) {
            await controller.goForward();
          }
          return _bridgeSuccess(null);
        case 'evaluateJavascript':
          final script = _string(commandPayload['script']);
          final result = await controller.runJavaScriptReturningResult(script);
          final decoded = await AppWorkers.run(
            () => _decodePlainJsonValue(result),
            debugName: 'compose-webview-evaluate-result-decode',
          );
          return _bridgeSuccess(decoded);
        case 'getState':
          return _bridgeSuccess(binding.state.toPayload());
        case 'addJavascriptInterface':
          final result = _registerJavascriptInterfaceCommand(
            executionContextKey: executionContextKey,
            controllerKey: controllerKey,
            payload: commandPayload,
          );
          await _refreshComposeDslJavascriptInterfaces(controller);
          return result;
        case 'removeJavascriptInterface':
          final result = _unregisterJavascriptInterfaceCommand(
            executionContextKey: executionContextKey,
            controllerKey: controllerKey,
            payload: commandPayload,
          );
          await _refreshComposeDslJavascriptInterfaces(controller);
          return result;
        default:
          return _bridgeError(
            'unsupported webview controller command: $command',
          );
      }
    } catch (error) {
      final message = error.toString().trim();
      return _bridgeError(
        message.isEmpty ? 'webview controller command failed' : message,
      );
    }
  }

  static String _registerJavascriptInterfaceCommand({
    required String executionContextKey,
    required String controllerKey,
    required Map<String, Object?> payload,
  }) {
    final name = _string(payload['name']).trim();
    final methodActionIds = _extractComposeDslJavascriptInterfaceMethods(
      payload['object'],
    );
    if (name.isEmpty || methodActionIds.isEmpty) {
      return _bridgeError(
        'webview controller addJavascriptInterface requires a non-empty name and at least one function method',
      );
    }
    final registered = registerJavascriptInterface(
      executionContextKey: executionContextKey,
      controllerKey: controllerKey,
      interfaceName: name,
      methodActionIds: methodActionIds,
    );
    return registered
        ? _bridgeSuccess(null)
        : _bridgeError(
            'failed to register webview javascript interface: $name',
          );
  }

  static String _unregisterJavascriptInterfaceCommand({
    required String executionContextKey,
    required String controllerKey,
    required Map<String, Object?> payload,
  }) {
    final name = _string(payload['name']).trim();
    if (name.isEmpty) {
      return _bridgeError(
        'webview controller removeJavascriptInterface requires a non-empty name',
      );
    }
    unregisterJavascriptInterface(
      executionContextKey: executionContextKey,
      controllerKey: controllerKey,
      interfaceName: name,
    );
    return _bridgeSuccess(null);
  }
}

class ComposeDslWebView extends StatefulWidget {
  const ComposeDslWebView({
    super.key,
    required this.props,
    required this.onAction,
    required this.hostContext,
  });

  final Map<String, Object?> props;
  final ComposeDslWebViewActionDispatcher onAction;
  final ComposeDslWebViewHostContext? hostContext;

  @override
  State<ComposeDslWebView> createState() => _ComposeDslWebViewState();
}

class _ComposeDslWebViewState extends State<ComposeDslWebView> {
  late final WebViewController _controller;
  late final Widget _webViewWidget;
  late _ComposeDslWebViewRequest _request;
  String? _loadKey;
  String? _settingsKey;
  String? _error;
  int _progress = 0;
  bool _loading = false;
  String? _currentUrl;
  String? _title;
  bool _canGoBack = false;
  bool _canGoForward = false;
  ComposeDslWebViewStateSnapshot? _lastStateSnapshot;
  _ComposeDslWebViewControllerDescriptor? _boundControllerDescriptor;
  String? _boundExecutionContextKey;
  ComposeDslWebViewResourceServer? _resourceServer;

  @override
  void initState() {
    super.initState();
    _request = _ComposeDslWebViewRequest.build(
      widget.props,
      allowBlank: _controllerDescriptor(widget.props) != null,
    );
    _currentUrl = _request.url ?? _request.baseUrl ?? 'about:blank';
    _controller = _createWebViewController()
      ..setJavaScriptMode(
        _bool(widget.props['javaScriptEnabled'], defaultValue: true)
            ? JavaScriptMode.unrestricted
            : JavaScriptMode.disabled,
      )
      ..setBackgroundColor(Colors.transparent)
      ..setNavigationDelegate(_navigationDelegate());
    _webViewWidget = WebViewWidget(controller: _controller);
    if (_supportsComposeDslPageHooks) {
      _controller
        ..addJavaScriptChannel(
          _composeDslWebViewBridgeChannelName,
          onMessageReceived: _handleBridgeMessage,
        )
        ..setOnConsoleMessage(_handleConsoleMessage);
    }
    if (_supportsJavaScriptDialogCallbacks) {
      _controller
        ..setOnJavaScriptAlertDialog((request) async {})
        ..setOnJavaScriptConfirmDialog((request) async => true)
        ..setOnJavaScriptTextInputDialog(
          (request) async => request.defaultText ?? '',
        );
    }
    _applyControllerSettingsIfNeeded(force: true);
    _bindControllerIfNeeded();
    _scheduleLoad();
  }

  /// Creates a WebView controller with platform-supported host callbacks.
  WebViewController _createWebViewController() {
    if (!kIsWeb) {
      return WebViewController(
        onPermissionRequest: (request) {
          request.grant();
        },
      );
    }
    return WebViewController();
  }

  /// Returns whether this WebView owns a controllable Compose page.
  bool get _supportsComposeDslPageHooks {
    if (!kIsWeb) {
      return true;
    }
    return _request.html != null || _usesResourceServer;
  }

  @override
  void didUpdateWidget(covariant ComposeDslWebView oldWidget) {
    super.didUpdateWidget(oldWidget);
    final nextRequest = _ComposeDslWebViewRequest.build(
      widget.props,
      allowBlank: _controllerDescriptor(widget.props) != null,
    );
    final nextKey = _contentKey(widget.props);
    final shouldLoad = nextKey != _loadKey;
    _request = nextRequest;
    _applyControllerSettingsIfNeeded();
    _bindControllerIfNeeded();
    if (shouldLoad) {
      _scheduleLoad();
    }
  }

  bool get _supportsJavaScriptDialogCallbacks {
    return _supportsComposeDslPageHooks &&
        (kIsWeb || defaultTargetPlatform != TargetPlatform.windows);
  }

  @override
  void dispose() {
    _emitLifecycle('disposed', forceEmit: true);
    final descriptor = _boundControllerDescriptor;
    final boundExecutionContextKey = _boundExecutionContextKey;
    if (descriptor != null && boundExecutionContextKey != null) {
      ComposeDslWebViewHostRegistry.unbind(
        executionContextKey: boundExecutionContextKey,
        controllerKey: descriptor.key,
        controller: _controller,
      );
    }
    unawaited(_resourceServer?.close());
    unawaited(_controller.loadHtmlString('<html></html>'));
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final errorText = _error;
    return Stack(
      fit: StackFit.expand,
      children: <Widget>[
        _webViewWidget,
        if (_progress > 0 && _progress < 100)
          const Positioned(
            top: 0,
            left: 0,
            right: 0,
            child: LinearProgressIndicator(),
          ),
        if (errorText != null && errorText.trim().isNotEmpty)
          Positioned(
            left: 12,
            right: 12,
            bottom: 12,
            child: Material(
              color: Theme.of(context).colorScheme.errorContainer,
              borderRadius: BorderRadius.circular(8),
              child: Padding(
                padding: const EdgeInsets.all(10),
                child: Text(
                  errorText,
                  style: TextStyle(
                    color: Theme.of(context).colorScheme.onErrorContainer,
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }

  NavigationDelegate _navigationDelegate() {
    return NavigationDelegate(
      onNavigationRequest: (request) async {
        final requestUrl = _originalUrlFor(request.url);
        final actionId = _callbackIds.onShouldOverrideUrlLoading;
        final hostContext = widget.hostContext;
        if (actionId == null || hostContext == null) {
          return _navigationDecisionForAllowedUrl(request.url, requestUrl);
        }
        final result = await hostContext.executeAction(
          actionId: actionId,
          payload: <String, Object?>{
            'url': requestUrl,
            'method': 'GET',
            'headers': const <String, String>{},
            'isMainFrame': request.isMainFrame,
            'hasGesture': false,
            'isRedirect': false,
            'scheme': Uri.tryParse(requestUrl)?.scheme,
          },
        );
        if (result.message != null && result.message!.trim().isNotEmpty) {
          return _navigationDecisionForAllowedUrl(request.url, requestUrl);
        }
        final decision = _parseNavigationDecision(result.actionResult);
        if (decision == null) {
          return _navigationDecisionForAllowedUrl(request.url, requestUrl);
        }
        switch (decision.action) {
          case 'cancel':
            return NavigationDecision.prevent;
          case 'rewrite':
            final rewrittenUrl = decision.url?.trim();
            if (rewrittenUrl != null && rewrittenUrl.isNotEmpty) {
              unawaited(
                _controller.loadRequest(
                  await _navigationUriFor(rewrittenUrl),
                  headers: decision.headers,
                ),
              );
              return NavigationDecision.prevent;
            }
            return _navigationDecisionForAllowedUrl(request.url, requestUrl);
          case 'external':
            final externalUrl = decision.url?.trim().ifNotEmpty ?? requestUrl;
            final launched = await launchUrl(
              Uri.parse(externalUrl),
              mode: LaunchMode.externalApplication,
            );
            return launched
                ? NavigationDecision.prevent
                : _navigationDecisionForAllowedUrl(request.url, requestUrl);
          default:
            return _navigationDecisionForAllowedUrl(request.url, requestUrl);
        }
      },
      onPageStarted: (url) {
        _currentUrl = _originalUrlFor(url);
        _loading = true;
        _progress = 0;
        _updateStateSnapshot(forceEmit: true);
        _emit(_callbackIds.onPageStarted, <String, Object?>{
          'url': _currentUrl,
          'title': _title,
          'canGoBack': _canGoBack,
          'canGoForward': _canGoForward,
        });
      },
      onPageFinished: (url) async {
        _currentUrl = _originalUrlFor(url);
        _loading = false;
        _progress = 100;
        await _refreshStateFromController();
        if (_supportsComposeDslPageHooks) {
          await _installComposeDslWebViewBridgeRuntime(_controller);
          await _refreshComposeDslJavascriptInterfaces(_controller);
        }
        _emit(_callbackIds.onPageFinished, <String, Object?>{
          'url': _currentUrl,
          'title': _title,
          'canGoBack': _canGoBack,
          'canGoForward': _canGoForward,
        });
      },
      onProgress: (progress) {
        _progress = progress.clamp(0, 100);
        _loading = _progress < 100;
        _updateStateSnapshot();
        _emit(_callbackIds.onProgressChanged, <String, Object?>{
          'progress': _progress,
          'url': _currentUrl,
          'title': _title,
        });
        if (mounted) {
          setState(() {});
        }
      },
      onUrlChange: (change) {
        final url = change.url == null
            ? null
            : _originalUrlFor(change.url!).trim();
        if (url == null || url.isEmpty) {
          return;
        }
        _currentUrl = url;
        _updateStateSnapshot();
        _emit(_callbackIds.onUrlChanged, <String, Object?>{
          'url': url,
          'isMainFrame': true,
          'method': null,
        });
      },
      onWebResourceError: (error) {
        final errorUrl = error.url == null ? null : _originalUrlFor(error.url!);
        final payload = <String, Object?>{
          'errorCode': error.errorCode,
          'description': error.description,
          'url': errorUrl,
          'isMainFrame': error.isForMainFrame ?? true,
        };
        _emit(_callbackIds.onReceivedError, payload);
        if (error.isForMainFrame != false && mounted) {
          setState(() {
            _error = error.description;
            _loading = false;
          });
        }
      },
      onHttpError: (error) {
        final requestUrl = error.request?.uri.toString();
        _emit(_callbackIds.onReceivedHttpError, <String, Object?>{
          'statusCode': error.response?.statusCode,
          'reasonPhrase': null,
          'url': requestUrl == null ? null : _originalUrlFor(requestUrl),
          'isMainFrame': true,
        });
      },
      onSslAuthError: (error) {
        _emit(_callbackIds.onReceivedSslError, <String, Object?>{
          'primaryError': null,
          'url': null,
        });
        error.cancel();
      },
    );
  }

  Future<NavigationDecision> _navigationDecisionForAllowedUrl(
    String currentUrl,
    String targetUrl,
  ) async {
    final server = _resourceServer;
    if (!_usesResourceServer ||
        server == null ||
        server.ownsUrl(currentUrl) ||
        !server.matchesCurrentOrigin(targetUrl)) {
      return NavigationDecision.navigate;
    }
    unawaited(
      _controller.loadRequest(
        await server.localUriFor(targetUrl, isMainFrame: true),
      ),
    );
    return NavigationDecision.prevent;
  }

  Future<Uri> _navigationUriFor(String url) async {
    final server = _resourceServer;
    if (_usesResourceServer &&
        server != null &&
        server.matchesCurrentOrigin(url)) {
      return server.localUriFor(url, isMainFrame: true);
    }
    return Uri.parse(url);
  }

  void _scheduleLoad() {
    _loadKey = _contentKey(widget.props);
    unawaited(_load());
  }

  String _contentKey(Map<String, Object?> props) {
    return jsonEncode(<String, Object?>{
      'url': props['url'],
      'html': props['html'],
      'baseUrl': props['baseUrl'],
      'mimeType': props['mimeType'],
      'encoding': props['encoding'],
      'headers': props['headers'],
    });
  }

  String _controllerSettingsKey(Map<String, Object?> props) {
    return jsonEncode(<String, Object?>{
      'javaScriptEnabled': props['javaScriptEnabled'],
      'supportZoom': props['supportZoom'],
      'userAgent': props['userAgent'],
      'verticalScrollBarEnabled': props['verticalScrollBarEnabled'],
      'horizontalScrollBarEnabled': props['horizontalScrollBarEnabled'],
    });
  }

  bool get _usesResourceServer {
    return _callbackIds.onInterceptRequest != null &&
        widget.hostContext != null;
  }

  ComposeDslWebViewResourceServer _ensureResourceServer() {
    return _resourceServer ??= ComposeDslWebViewResourceServer(
      dispatchDecision: _dispatchInterceptRequestDecision,
    );
  }

  Future<Object?> _dispatchInterceptRequestDecision(
    Map<String, Object?> payload,
  ) async {
    final actionId = _callbackIds.onInterceptRequest;
    final hostContext = widget.hostContext;
    if (actionId == null || hostContext == null) {
      throw StateError('onInterceptRequest is not bound');
    }
    final result = await hostContext.executeAction(
      actionId: actionId,
      payload: payload,
    );
    final message = result.message?.trim();
    if (message != null && message.isNotEmpty) {
      throw StateError(message);
    }
    return result.actionResult;
  }

  Future<Uri> _webViewUriFor(String url, {required bool isMainFrame}) async {
    if (!_usesResourceServer) {
      return Uri.parse(url);
    }
    return _ensureResourceServer().localUriFor(url, isMainFrame: isMainFrame);
  }

  String _originalUrlFor(String url) {
    return _resourceServer?.originalUrlFor(url) ?? url;
  }

  Future<void> _load() async {
    try {
      if (mounted) {
        setState(() {
          _error = null;
          _progress = 0;
          _loading = true;
        });
      }
      if (_supportsComposeDslPageHooks) {
        await _refreshComposeDslJavascriptInterfaces(_controller);
      }
      if (_request.url != null) {
        final uri = await _webViewUriFor(_request.url!, isMainFrame: true);
        await _controller.loadRequest(uri, headers: _request.headers);
      } else if (_request.html != null) {
        await _controller.loadHtmlString(
          _injectComposeDslWebViewBridgeRuntimeIntoHtml(_request.html!),
          baseUrl: _request.baseUrl,
        );
      } else {
        await _controller.loadRequest(Uri.parse('about:blank'));
      }
      _emitLifecycle('created', forceEmit: true);
    } catch (error) {
      if (mounted) {
        setState(() {
          _error = error.toString();
          _loading = false;
        });
      }
    }
  }

  void _applyControllerSettingsIfNeeded({bool force = false}) {
    final nextSettingsKey = _controllerSettingsKey(widget.props);
    if (!force && nextSettingsKey == _settingsKey) {
      return;
    }
    _settingsKey = nextSettingsKey;
    final javaScriptEnabled = _bool(
      widget.props['javaScriptEnabled'],
      defaultValue: true,
    );
    unawaited(
      _controller.setJavaScriptMode(
        javaScriptEnabled
            ? JavaScriptMode.unrestricted
            : JavaScriptMode.disabled,
      ),
    );
    unawaited(
      _controller.enableZoom(
        _bool(widget.props['supportZoom'], defaultValue: true),
      ),
    );
    final userAgent = _string(widget.props['userAgent']).trim();
    if (userAgent.isNotEmpty) {
      unawaited(_controller.setUserAgent(userAgent));
    }
    final verticalScrollbarEnabled = _bool(
      widget.props['verticalScrollBarEnabled'],
      defaultValue: true,
    );
    final horizontalScrollbarEnabled = _bool(
      widget.props['horizontalScrollBarEnabled'],
      defaultValue: true,
    );
    unawaited(
      _controller.setVerticalScrollBarEnabled(verticalScrollbarEnabled),
    );
    unawaited(
      _controller.setHorizontalScrollBarEnabled(horizontalScrollbarEnabled),
    );
  }

  void _bindControllerIfNeeded() {
    final descriptor = _controllerDescriptor(widget.props);
    final hostContext = widget.hostContext;
    final current = _boundControllerDescriptor;
    final currentExecutionContextKey = _boundExecutionContextKey;
    final nextBindable =
        descriptor != null &&
        hostContext != null &&
        (descriptor.executionContextKey == null ||
            descriptor.executionContextKey ==
                hostContext.executionContextKey) &&
        (descriptor.routeInstanceId == null ||
            descriptor.routeInstanceId == hostContext.routeInstanceId);
    if (current != null &&
        (!nextBindable || current.key != descriptor.key) &&
        currentExecutionContextKey != null) {
      ComposeDslWebViewHostRegistry.unbind(
        executionContextKey: currentExecutionContextKey,
        controllerKey: current.key,
        controller: _controller,
      );
      _boundControllerDescriptor = null;
      _boundExecutionContextKey = null;
    }
    if (descriptor == null || hostContext == null || !nextBindable) {
      return;
    }
    _boundControllerDescriptor = descriptor;
    _boundExecutionContextKey = hostContext.executionContextKey;
    ComposeDslWebViewHostRegistry.bind(
      executionContextKey: hostContext.executionContextKey,
      routeInstanceId: hostContext.routeInstanceId,
      controllerKey: descriptor.key,
      controller: _controller,
      state: _stateSnapshot(),
    );
  }

  Future<void> _refreshStateFromController() async {
    final currentUrl = await _controller.currentUrl();
    _currentUrl = currentUrl == null ? null : _originalUrlFor(currentUrl);
    _title = await _controller.getTitle();
    _canGoBack = await _controller.canGoBack();
    _canGoForward = await _controller.canGoForward();
    _updateStateSnapshot(forceEmit: true);
    if (mounted) {
      setState(() {});
    }
  }

  ComposeDslWebViewStateSnapshot _stateSnapshot() {
    return ComposeDslWebViewStateSnapshot(
      url: _currentUrl,
      title: _title,
      loading: _loading,
      progress: _progress,
      canGoBack: _canGoBack,
      canGoForward: _canGoForward,
    );
  }

  void _updateStateSnapshot({bool forceEmit = false}) {
    final snapshot = _stateSnapshot();
    final previous = _lastStateSnapshot;
    _lastStateSnapshot = snapshot;
    final descriptor = _boundControllerDescriptor;
    final hostContext = widget.hostContext;
    if (descriptor != null && hostContext != null) {
      ComposeDslWebViewHostRegistry.updateState(
        executionContextKey: hostContext.executionContextKey,
        controllerKey: descriptor.key,
        state: snapshot,
      );
    }
    if (forceEmit || snapshot != previous) {
      _emit(_callbackIds.onStateChanged, snapshot.toPayload());
    }
  }

  void _emitLifecycle(String type, {bool forceEmit = false}) {
    _updateStateSnapshot(forceEmit: forceEmit);
    final snapshot = _stateSnapshot();
    _emit(_callbackIds.onLifecycleEvent, <String, Object?>{
      'type': type,
      ...snapshot.toPayload(),
    });
  }

  void _emit(String? actionId, Object? payload) {
    final normalizedActionId = actionId?.trim();
    if (normalizedActionId == null || normalizedActionId.isEmpty) {
      return;
    }
    widget.onAction(normalizedActionId, payload);
  }

  _ComposeDslWebViewCallbackIds get _callbackIds {
    return _ComposeDslWebViewCallbackIds(
      onPageStarted: _actionId(widget.props['onPageStarted']),
      onPageFinished: _actionId(widget.props['onPageFinished']),
      onReceivedError: _actionId(widget.props['onReceivedError']),
      onReceivedHttpError: _actionId(widget.props['onReceivedHttpError']),
      onReceivedSslError: _actionId(widget.props['onReceivedSslError']),
      onDownloadStart: _actionId(widget.props['onDownloadStart']),
      onConsoleMessage: _actionId(widget.props['onConsoleMessage']),
      onUrlChanged: _actionId(widget.props['onUrlChanged']),
      onProgressChanged: _actionId(widget.props['onProgressChanged']),
      onStateChanged: _actionId(widget.props['onStateChanged']),
      onLifecycleEvent: _actionId(widget.props['onLifecycleEvent']),
      onShouldOverrideUrlLoading: _actionId(
        widget.props['onShouldOverrideUrlLoading'],
      ),
      onInterceptRequest: _actionId(widget.props['onInterceptRequest']),
    );
  }

  Future<void> _handleBridgeMessage(JavaScriptMessage message) async {
    final messageText = message.message;
    final payload = await AppWorkers.run(
      () => _decodeJsonObject(messageText),
      debugName: 'compose-webview-bridge-message-decode',
    );
    if (payload == null) {
      return;
    }
    final requestId = _string(payload['id']).trim();
    final type = _string(payload['type']).trim();
    try {
      final data = await _handleBridgeRequest(
        type: type,
        payload: payload['payload'],
      );
      await _postBridgeResponse(requestId, <String, Object?>{
        'success': true,
        'data': data,
      });
    } catch (error) {
      await _postBridgeResponse(requestId, <String, Object?>{
        'success': false,
        'message': error.toString(),
      });
    }
  }

  Future<Object?> _handleBridgeRequest({
    required String type,
    required Object? payload,
  }) async {
    final hostContext = widget.hostContext;
    final descriptor = _boundControllerDescriptor;
    switch (type) {
      case 'controllerCommand':
        final commandPayload = await AppWorkers.run(
          () => jsonEncode(payload),
          debugName: 'compose-webview-controller-command-encode',
        );
        final result =
            await ComposeDslWebViewHostRegistry.handleControllerCommand(
              commandPayload,
            );
        return AppWorkers.run(
          () => _decodePlainJsonValue(result),
          debugName: 'compose-webview-controller-command-result-decode',
        );
      case 'listInterfaces':
        if (hostContext == null || descriptor == null) {
          return const <String, List<String>>{};
        }
        return ComposeDslWebViewHostRegistry.listJavascriptInterfaces(
          executionContextKey: hostContext.executionContextKey,
          controllerKey: descriptor.key,
        );
      case 'invoke':
        if (hostContext == null || descriptor == null) {
          return null;
        }
        final map = _stringMap(payload);
        final interfaceName = _string(map['interfaceName']);
        final methodName = _string(map['methodName']);
        final actionId =
            ComposeDslWebViewHostRegistry.findJavascriptInterfaceActionId(
              executionContextKey: hostContext.executionContextKey,
              controllerKey: descriptor.key,
              interfaceName: interfaceName,
              methodName: methodName,
            );
        if (actionId == null) {
          return null;
        }
        final result = await hostContext.executeAction(
          actionId: actionId,
          payload: await AppWorkers.run(
            () => _decodePlainJsonValue(map['args']),
            debugName: 'compose-webview-invoke-args-decode',
          ),
        );
        return result.actionResult;
      case 'dispatchAction':
        if (hostContext == null) {
          return null;
        }
        final map = _stringMap(payload);
        final actionId = _string(map['actionId']).trim();
        if (actionId.isEmpty) {
          return null;
        }
        final result = await hostContext.executeAction(
          actionId: actionId,
          payload: map['payload'],
        );
        return result.actionResult;
      case 'pickFiles':
        return _pickFiles(_stringMap(payload));
      default:
        throw StateError('unsupported webview bridge request: $type');
    }
  }

  Future<void> _postBridgeResponse(
    String requestId,
    Map<String, Object?> response,
  ) async {
    if (requestId.isEmpty) {
      return;
    }
    final payload = await AppWorkers.run(
      () => jsonEncode(<String, Object?>{'id': requestId, ...response}),
      debugName: 'compose-webview-bridge-response-encode',
    );
    await _controller.runJavaScript('''
      if (typeof window.__operitComposeDslWebViewHostReceive === 'function') {
        window.__operitComposeDslWebViewHostReceive($payload);
      }
    ''');
  }

  void _handleConsoleMessage(JavaScriptConsoleMessage message) {
    _emit(_callbackIds.onConsoleMessage, <String, Object?>{
      'message': message.message,
      'sourceId': null,
      'lineNumber': null,
      'level': message.level.name,
    });
  }

  Future<List<Map<String, Object?>>> _pickFiles(
    Map<String, Object?> options,
  ) async {
    final accepts = _stringList(options['accepts']);
    final acceptedTypeGroups = accepts.isEmpty
        ? const <XTypeGroup>[]
        : <XTypeGroup>[XTypeGroup(label: 'files', extensions: accepts)];
    final multiple = _bool(options['multiple']);
    final files = multiple
        ? await openFiles(acceptedTypeGroups: acceptedTypeGroups)
        : <XFile>[?await openFile(acceptedTypeGroups: acceptedTypeGroups)];
    final stagedDirectory = await OperitClientPaths.composeDslWebviewFilesDir();
    if (!await stagedDirectory.exists()) {
      await stagedDirectory.create(recursive: true);
    }
    final stagedFiles = <Map<String, Object?>>[];
    for (final file in files) {
      final source = File(file.path);
      final sourceName = file.name.trim().isEmpty
          ? source.uri.pathSegments.last
          : file.name.trim();
      final stagedPath =
          '${stagedDirectory.path}${Platform.pathSeparator}${DateTime.now().microsecondsSinceEpoch}_$sourceName';
      final staged = await source.copy(stagedPath);
      stagedFiles.add(<String, Object?>{
        'path': staged.path,
        'name': sourceName,
        'size': await staged.length(),
        'mimeType': file.mimeType,
      });
    }
    return stagedFiles;
  }
}

class _ComposeDslWebViewControllerDescriptor {
  const _ComposeDslWebViewControllerDescriptor({
    required this.key,
    required this.routeInstanceId,
    required this.executionContextKey,
  });

  final String key;
  final String? routeInstanceId;
  final String? executionContextKey;
}

class _ComposeDslWebViewRequest {
  const _ComposeDslWebViewRequest({
    required this.url,
    required this.html,
    required this.baseUrl,
    required this.mimeType,
    required this.encoding,
    required this.headers,
  });

  final String? url;
  final String? html;
  final String? baseUrl;
  final String mimeType;
  final String encoding;
  final Map<String, String> headers;

  static _ComposeDslWebViewRequest build(
    Map<String, Object?> props, {
    required bool allowBlank,
  }) {
    final url = _string(props['url']).trim().ifNotEmpty;
    final html = _string(props['html']).ifNotEmpty;
    if (!allowBlank && url == null && html == null) {
      throw StateError("WebView requires either 'url' or 'html'.");
    }
    return _ComposeDslWebViewRequest(
      url: url,
      html: html,
      baseUrl: _string(props['baseUrl']).trim().ifNotEmpty,
      mimeType: _string(props['mimeType']).trim().ifNotEmpty ?? 'text/html',
      encoding: _string(props['encoding']).trim().ifNotEmpty ?? 'UTF-8',
      headers: _toStringMap(props['headers']),
    );
  }
}

class _ComposeDslWebViewCallbackIds {
  const _ComposeDslWebViewCallbackIds({
    required this.onPageStarted,
    required this.onPageFinished,
    required this.onReceivedError,
    required this.onReceivedHttpError,
    required this.onReceivedSslError,
    required this.onDownloadStart,
    required this.onConsoleMessage,
    required this.onUrlChanged,
    required this.onProgressChanged,
    required this.onStateChanged,
    required this.onLifecycleEvent,
    required this.onShouldOverrideUrlLoading,
    required this.onInterceptRequest,
  });

  final String? onPageStarted;
  final String? onPageFinished;
  final String? onReceivedError;
  final String? onReceivedHttpError;
  final String? onReceivedSslError;
  final String? onDownloadStart;
  final String? onConsoleMessage;
  final String? onUrlChanged;
  final String? onProgressChanged;
  final String? onStateChanged;
  final String? onLifecycleEvent;
  final String? onShouldOverrideUrlLoading;
  final String? onInterceptRequest;
}

class _ComposeDslWebViewNavigationDecision {
  const _ComposeDslWebViewNavigationDecision({
    required this.action,
    required this.url,
    required this.headers,
  });

  final String action;
  final String? url;
  final Map<String, String> headers;
}

class _ComposeDslWebViewControllerBinding {
  _ComposeDslWebViewControllerBinding({
    required this.routeInstanceId,
    required this.executionContextKey,
    required this.controllerKey,
    required this.controller,
    required this.state,
    required this.javascriptInterfaceActionIds,
  });

  final String routeInstanceId;
  final String executionContextKey;
  final String controllerKey;
  final WebViewController controller;
  ComposeDslWebViewStateSnapshot state;
  final Map<String, Map<String, String>> javascriptInterfaceActionIds;
}

_ComposeDslWebViewControllerDescriptor? _controllerDescriptor(
  Map<String, Object?> props,
) {
  final rawController = props['controller'];
  if (rawController is! Map<Object?, Object?>) {
    return null;
  }
  final marker = rawController['__composeWebViewController'];
  if (marker != true) {
    return null;
  }
  final key = _string(rawController['key']).trim();
  if (key.isEmpty) {
    return null;
  }
  return _ComposeDslWebViewControllerDescriptor(
    key: key,
    routeInstanceId: _string(
      rawController['routeInstanceId'],
    ).trim().ifNotEmpty,
    executionContextKey: _string(
      rawController['executionContextKey'],
    ).trim().ifNotEmpty,
  );
}

_ComposeDslWebViewNavigationDecision? _parseNavigationDecision(Object? raw) {
  final rawMap = _stringMap(raw);
  final action = raw is String ? raw.trim() : _string(rawMap['action']).trim();
  if (action.isEmpty) {
    return null;
  }
  return switch (action) {
    'allow' || 'cancel' => _ComposeDslWebViewNavigationDecision(
      action: action,
      url: null,
      headers: const <String, String>{},
    ),
    'rewrite' =>
      _string(rawMap['url']).trim().isEmpty
          ? null
          : _ComposeDslWebViewNavigationDecision(
              action: action,
              url: _string(rawMap['url']).trim(),
              headers: _toStringMap(rawMap['headers']),
            ),
    'external' => _ComposeDslWebViewNavigationDecision(
      action: action,
      url: _string(rawMap['url']).trim().ifNotEmpty,
      headers: const <String, String>{},
    ),
    _ => null,
  };
}

Future<void> _refreshComposeDslJavascriptInterfaces(
  WebViewController controller,
) {
  return controller.runJavaScript('''
    (function() {
      if (typeof window.__operitInstallComposeDslJavascriptInterfaces === 'function') {
        window.__operitInstallComposeDslJavascriptInterfaces();
      }
    })();
  ''');
}

Future<void> _installComposeDslWebViewBridgeRuntime(
  WebViewController controller,
) {
  return controller.runJavaScript(_buildComposeDslWebViewBridgeRuntimeScript());
}

String _injectComposeDslWebViewBridgeRuntimeIntoHtml(String html) {
  if (html.contains(_composeDslWebViewBridgeHtmlMarker)) {
    return html;
  }
  final scriptTag = _buildComposeDslWebViewBridgeRuntimeScriptTag();
  final headClose = RegExp('</head>', caseSensitive: false);
  if (headClose.hasMatch(html)) {
    return html.replaceFirst(headClose, '$scriptTag</head>');
  }
  final headOpen = RegExp(r'<head[^>]*>', caseSensitive: false);
  final headOpenMatch = headOpen.firstMatch(html);
  if (headOpenMatch != null) {
    final headTag = headOpenMatch.group(0)!;
    return html.replaceFirst(headOpen, '$headTag$scriptTag');
  }
  final htmlOpen = RegExp(r'<html[^>]*>', caseSensitive: false);
  final htmlOpenMatch = htmlOpen.firstMatch(html);
  if (htmlOpenMatch != null) {
    final htmlTag = htmlOpenMatch.group(0)!;
    return html.replaceFirst(htmlOpen, '$htmlTag<head>$scriptTag</head>');
  }
  return '$scriptTag$html';
}

String _buildComposeDslWebViewBridgeRuntimeScriptTag() {
  final scriptBody = _buildComposeDslWebViewBridgeRuntimeScript().replaceAll(
    '</script>',
    '<\\/script>',
  );
  return '<script $_composeDslWebViewBridgeHtmlMarker>$scriptBody</script>';
}

String _buildComposeDslWebViewBridgeRuntimeScript() {
  final hiddenBridgeNameJson = jsonEncode(composeDslWebViewInternalBridgeName);
  final channelNameJson = jsonEncode(_composeDslWebViewBridgeChannelName);
  return '''
    (function() {
      var hiddenBridgeName = $hiddenBridgeNameJson;
      var channelName = $channelNameJson;
      var channel = window[channelName];
      if (!channel || typeof channel.postMessage !== 'function') {
        return;
      }
      var sequence = 0;
      var pending = {};
      function send(type, payload) {
        sequence += 1;
        var id = String(Date.now()) + ':' + String(sequence);
        return new Promise(function(resolve, reject) {
          pending[id] = { resolve: resolve, reject: reject };
          channel.postMessage(JSON.stringify({
            id: id,
            type: type,
            payload: payload === undefined ? null : payload
          }));
        });
      }
      window.__operitComposeDslWebViewHostReceive = function(message) {
        var envelope = typeof message === 'string' ? JSON.parse(message) : message;
        if (!envelope || !envelope.id || !pending[envelope.id]) {
          return;
        }
        var callbacks = pending[envelope.id];
        delete pending[envelope.id];
        if (envelope.success === false) {
          callbacks.reject(new Error(String(envelope.message || '')));
        } else {
          callbacks.resolve(envelope.data);
        }
      };
      function defineReadonly(target, key, value) {
        Object.defineProperty(target, key, {
          configurable: true,
          enumerable: true,
          writable: false,
          value: value
        });
      }
      var hiddenBridge = {
        handleControllerCommand: function(payload) {
          return send('controllerCommand', payload);
        },
        listInterfaces: function() {
          return send('listInterfaces', {});
        },
        invoke: function(interfaceName, methodName, argsJson) {
          return send('invoke', {
            interfaceName: interfaceName,
            methodName: methodName,
            args: argsJson
          });
        },
        dispatchAction: function(actionId, payload) {
          return send('dispatchAction', {
            actionId: actionId,
            payload: payload === undefined ? null : payload
          });
        },
        pickFiles: function(options) {
          return send('pickFiles', options || {});
        }
      };
      defineReadonly(window, hiddenBridgeName, hiddenBridge);
      function installInterfaces() {
        return hiddenBridge.listInterfaces().then(function(descriptors) {
          descriptors = descriptors || {};
          var installed =
            window.__operitComposeDslInstalledJavascriptInterfaces &&
            typeof window.__operitComposeDslInstalledJavascriptInterfaces === 'object'
              ? window.__operitComposeDslInstalledJavascriptInterfaces
              : {};
          for (var previousInterfaceName in installed) {
            if (
              Object.prototype.hasOwnProperty.call(installed, previousInterfaceName) &&
              !Object.prototype.hasOwnProperty.call(descriptors, previousInterfaceName)
            ) {
              try {
                delete window[previousInterfaceName];
              } catch (_deleteError) {
              }
            }
          }
          window.__operitComposeDslInstalledJavascriptInterfaces = {};
          for (var interfaceName in descriptors) {
            if (!Object.prototype.hasOwnProperty.call(descriptors, interfaceName)) {
              continue;
            }
            var methodNames = Array.isArray(descriptors[interfaceName])
              ? descriptors[interfaceName]
              : [];
            var hostObject = {};
            window[interfaceName] = hostObject;
            for (var i = 0; i < methodNames.length; i += 1) {
              (function(targetObject, resolvedInterfaceName, resolvedMethodName) {
                defineReadonly(targetObject, resolvedMethodName, function() {
                  var args = [];
                  for (var argIndex = 0; argIndex < arguments.length; argIndex += 1) {
                    args.push(arguments[argIndex]);
                  }
                  return hiddenBridge.invoke(
                    resolvedInterfaceName,
                    resolvedMethodName,
                    JSON.stringify(args)
                  );
                });
              })(hostObject, interfaceName, String(methodNames[i] || '').trim());
            }
            window.__operitComposeDslInstalledJavascriptInterfaces[interfaceName] = true;
          }
        });
      }
      window.__operitInstallComposeDslJavascriptInterfaces = installInterfaces;
      installInterfaces();
    })();
  ''';
}

String _bridgeSuccess(Object? data) {
  return jsonEncode(<String, Object?>{'success': true, 'data': data});
}

String _bridgeError(String message) {
  return jsonEncode(<String, Object?>{'success': false, 'message': message});
}

Map<String, Object?>? _decodeJsonObject(String raw) {
  final trimmed = raw.trim();
  if (trimmed.isEmpty) {
    return null;
  }
  final decoded = jsonDecode(trimmed);
  if (decoded is Map<Object?, Object?>) {
    return decoded.map((key, value) => MapEntry(key.toString(), value));
  }
  return null;
}

Object? _decodePlainJsonValue(Object? raw) {
  if (raw is! String) {
    return raw;
  }
  final trimmed = raw.trim();
  if (trimmed.isEmpty) {
    return null;
  }
  try {
    return jsonDecode(trimmed);
  } catch (_) {
    return raw;
  }
}

Map<String, String> _extractComposeDslJavascriptInterfaceMethods(
  Object? value,
) {
  final methods = _stringMap(value);
  final result = <String, String>{};
  for (final entry in methods.entries) {
    final methodName = entry.key.trim();
    final actionId = _actionId(entry.value);
    if (methodName.isNotEmpty && actionId != null) {
      result[methodName] = actionId;
    }
  }
  return result;
}

String? _actionId(Object? raw) {
  if (raw is Map<Object?, Object?>) {
    final value = raw['__actionId'] ?? raw['actionId'];
    final actionId = value?.toString().trim();
    return actionId == null || actionId.isEmpty ? null : actionId;
  }
  final text = raw?.toString().trim();
  if (text == null || text.isEmpty) {
    return null;
  }
  return text.startsWith('__action:')
      ? text.substring('__action:'.length).trim()
      : text;
}

Map<String, Object?> _stringMap(Object? raw) {
  if (raw is Map<Object?, Object?>) {
    return raw.map((key, value) => MapEntry(key.toString(), value));
  }
  return <String, Object?>{};
}

Map<String, String> _toStringMap(Object? raw) {
  if (raw is! Map<Object?, Object?>) {
    return const <String, String>{};
  }
  final result = <String, String>{};
  for (final entry in raw.entries) {
    final key = entry.key?.toString().trim() ?? '';
    if (key.isNotEmpty && entry.value != null) {
      result[key] = entry.value.toString();
    }
  }
  return result;
}

List<String> _stringList(Object? raw) {
  if (raw is List<Object?>) {
    return raw
        .map((value) => value?.toString().trim() ?? '')
        .where((value) => value.isNotEmpty)
        .toList(growable: false);
  }
  final text = raw?.toString().trim();
  return text == null || text.isEmpty ? const <String>[] : <String>[text];
}

String _string(Object? raw) => raw?.toString() ?? '';

bool _bool(Object? raw, {bool defaultValue = false}) {
  if (raw == null) {
    return defaultValue;
  }
  if (raw is bool) {
    return raw;
  }
  final text = raw.toString().trim().toLowerCase();
  if (text == 'true' || text == '1' || text == 'yes') {
    return true;
  }
  if (text == 'false' || text == '0' || text == 'no') {
    return false;
  }
  return defaultValue;
}

extension _NonEmptyString on String {
  String? get ifNotEmpty => isEmpty ? null : this;
}
