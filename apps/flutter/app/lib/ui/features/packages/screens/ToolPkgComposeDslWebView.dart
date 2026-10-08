// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:mime/mime.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/logging/ClientLogger.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import 'ToolPkgComposeDslWebViewResourceLoader.dart';
import 'ToolPkgComposeDslWebViewBridgeRuntime.dart';

export 'ToolPkgComposeDslWebViewBridgeRuntime.dart'
    show composeDslWebViewInternalBridgeName;

const String _composeDslWebViewLogTag = 'ComposeDslWebView';
const GeneratedCoreProxyClients _runtimeClients = GeneratedCoreProxyClients(
  ProxyCoreRuntimeBridge(),
);

typedef ComposeDslWebViewActionDispatcher =
    Future<Object?> Function(String actionId, [Object? payload]);
typedef ComposeDslWebViewRuntimeOptionsProvider =
    Map<String, Object?> Function();

class ComposeDslWebViewHostContext {
  /// Creates the action bridge and its post-response completion observer.
  const ComposeDslWebViewHostContext({
    required this.packageName,
    required this.routeInstanceId,
    required this.executionContextKey,
    required this.dispatchAction,
    required this.runtimeOptionsProvider,
    this.onActionResultDelivered,
  });

  /// Stable plugin package identity, not the transient execution/session key.
  final String packageName;
  final String routeInstanceId;
  final String executionContextKey;
  final ComposeDslWebViewActionDispatcher dispatchAction;
  final ComposeDslWebViewRuntimeOptionsProvider runtimeOptionsProvider;

  /// Observes action results only after the WebView receives its bridge response.
  final ValueChanged<Object?>? onActionResultDelivered;

  /// Executes an action while retaining errors for the requesting WebView.
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

  /// Binds a controller and its owner-aware navigation resolver.
  static void bind({
    required String executionContextKey,
    required String routeInstanceId,
    required String controllerKey,
    required WebViewController controller,
    required Future<Uri> Function(String url) resolveNavigationUri,
    required Future<void> Function() prepareDocumentStartBridge,
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
      resolveNavigationUri: resolveNavigationUri,
      prepareDocumentStartBridge: prepareDocumentStartBridge,
      state: state,
      javascriptInterfaceActionIds: registeredInterfaces.map(
        (name, methods) => MapEntry(name, Map<String, String>.of(methods)),
      ),
    );
    ClientLogger.i(
      'event=controller_bind context=$executionContextKey route=$routeInstanceId '
      'key=$controllerKey boundKeys=${scopedBindings.keys.join(',')}',
      tag: _composeDslWebViewLogTag,
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
      ClientLogger.i(
        'event=controller_unbind context=$executionContextKey key=$controllerKey '
        'remainingKeys=${scopedBindings.keys.join(',')}',
        tag: _composeDslWebViewLogTag,
      );
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
    ClientLogger.i(
      'event=controller_context_clear context=$executionContextKey',
      tag: _composeDslWebViewLogTag,
    );
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
    final payload = _decodeJsonObject(payloadJson);
    if (payload == null) {
      return _bridgeError('invalid webview controller command payload');
    }
    final executionContextKey = _string(payload['executionContextKey']).trim();
    final controllerKey = _string(payload['key']).trim();
    final command = _string(payload['command']).trim();
    if (executionContextKey.isEmpty ||
        controllerKey.isEmpty ||
        command.isEmpty) {
      ClientLogger.e(
        'event=controller_command_rejected reason=missing_fields '
        'context=$executionContextKey key=$controllerKey command=$command',
        tag: _composeDslWebViewLogTag,
      );
      return _bridgeError(
        'webview controller command is missing required fields',
      );
    }
    final commandPayload = _stringMap(payload['payload']);
    final binding = _bindings[executionContextKey]?[controllerKey];
    final commandDetail = command == 'evaluateJavascript'
        ? 'scriptChars=${_string(commandPayload['script']).length}'
        : 'payloadKeys=${commandPayload.keys.join(',')}';
    ClientLogger.d(
      'event=controller_command_start context=$executionContextKey '
      'key=$controllerKey command=$command $commandDetail '
      'bound=${binding != null}',
      tag: _composeDslWebViewLogTag,
    );
    if (binding == null) {
      final boundKeys = _bindings[executionContextKey]?.keys.join(',') ?? '';
      if (command == 'getState') {
        ClientLogger.d(
          'event=controller_command_pending_binding '
          'context=$executionContextKey key=$controllerKey command=$command '
          'boundKeys=$boundKeys',
          tag: _composeDslWebViewLogTag,
        );
        return _bridgeSuccess(null);
      }
      if (command == 'addJavascriptInterface') {
        ClientLogger.d(
          'event=controller_command_pending_binding '
          'context=$executionContextKey key=$controllerKey command=$command '
          'boundKeys=$boundKeys',
          tag: _composeDslWebViewLogTag,
        );
        return _registerJavascriptInterfaceCommand(
          executionContextKey: executionContextKey,
          controllerKey: controllerKey,
          payload: commandPayload,
        );
      }
      if (command == 'removeJavascriptInterface') {
        ClientLogger.d(
          'event=controller_command_pending_binding '
          'context=$executionContextKey key=$controllerKey command=$command '
          'boundKeys=$boundKeys',
          tag: _composeDslWebViewLogTag,
        );
        return _unregisterJavascriptInterfaceCommand(
          executionContextKey: executionContextKey,
          controllerKey: controllerKey,
          payload: commandPayload,
        );
      }
      ClientLogger.e(
        'event=controller_command_missing_binding '
        'context=$executionContextKey key=$controllerKey command=$command '
        'boundKeys=$boundKeys',
        tag: _composeDslWebViewLogTag,
      );
      return _bridgeError(
        "webview controller '$controllerKey' is not bound in route '$executionContextKey'",
      );
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
            await binding.resolveNavigationUri(url),
            headers: _toStringMap(commandPayload['headers']),
          );
          return _bridgeSuccess(null);
        case 'loadHtml':
          final html = _string(commandPayload['html']);
          final options = _stringMap(commandPayload['options']);
          await binding.prepareDocumentStartBridge();
          await controller.loadHtmlString(
            injectComposeDslWebViewBridgeRuntimeIntoHtml(
              html,
              javascriptInterfaces: listJavascriptInterfaces(
                executionContextKey: executionContextKey,
                controllerKey: controllerKey,
              ),
            ),
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
          final decoded = _decodePlainJsonValue(result);
          ClientLogger.d(
            'event=controller_command_done context=$executionContextKey '
            'key=$controllerKey command=$command',
            tag: _composeDslWebViewLogTag,
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
          await binding.prepareDocumentStartBridge();
          await _refreshComposeDslJavascriptInterfaces(
            controller,
            javascriptInterfaces: listJavascriptInterfaces(
              executionContextKey: executionContextKey,
              controllerKey: controllerKey,
            ),
          );
          return result;
        case 'removeJavascriptInterface':
          final result = _unregisterJavascriptInterfaceCommand(
            executionContextKey: executionContextKey,
            controllerKey: controllerKey,
            payload: commandPayload,
          );
          await binding.prepareDocumentStartBridge();
          await _refreshComposeDslJavascriptInterfaces(
            controller,
            javascriptInterfaces: listJavascriptInterfaces(
              executionContextKey: executionContextKey,
              controllerKey: controllerKey,
            ),
          );
          return result;
        default:
          return _bridgeError(
            'unsupported webview controller command: $command',
          );
      }
    } catch (error) {
      final message = error.toString().trim();
      ClientLogger.e(
        'event=controller_command_failed context=$executionContextKey '
        'key=$controllerKey command=$command',
        tag: _composeDslWebViewLogTag,
        error: error,
        stackTrace: StackTrace.current,
      );
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
  /// Keeps embedded page gestures from being claimed by ancestor navigation.
  static final Set<Factory<OneSequenceGestureRecognizer>>
  _pageGestureRecognizers = <Factory<OneSequenceGestureRecognizer>>{
    Factory<EagerGestureRecognizer>(EagerGestureRecognizer.new),
  };

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
  ComposeDslWebViewResourceLoader? _resourceLoader;
  Future<void>? _resourceRegistration;
  bool _servingVfsFiles = false;
  Brightness? _brightness;
  Future<void> _themeUpdate = Future<void>.value();
  Future<void> _bridgeChannelRegistration = Future<void>.value();
  Future<void> _bridgeScriptUpdate = Future<void>.value();
  String? _bridgeUserScriptIdentifier;
  String? _bridgeUserScriptSource;

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
    _webViewWidget = WebViewWidget(
      controller: _controller,
      gestureRecognizers: _pageGestureRecognizers,
    );
    if (_supportsComposeDslPageHooks) {
      _bridgeChannelRegistration = _controller.addJavaScriptChannel(
        composeDslWebViewBridgeChannelName,
        onMessageReceived: _handleBridgeMessage,
      );
      _controller.setOnConsoleMessage(_handleConsoleMessage);
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
  }

  /// Applies the resolved app theme before the first navigation and on theme changes.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final brightness = Theme.of(context).brightness;
    if (_brightness == brightness) return;
    final initial = _brightness == null;
    _brightness = brightness;
    _themeUpdate = _controller.setPreferredColorScheme(brightness);
    if (initial) {
      _scheduleLoad();
    } else {
      unawaited(_reportThemeUpdate());
    }
  }

  /// Surfaces native theme failures in the same error UI as browser load failures.
  Future<void> _reportThemeUpdate() async {
    try {
      await _themeUpdate;
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    }
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
    return widget.hostContext != null ||
        _request.html != null ||
        _usesResourceLoader;
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
    unawaited(_disposeBridgeUserScript());
    unawaited(_disposeResourceLoader());
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
          await _refreshComposeDslJavascriptInterfaces(
            _controller,
            javascriptInterfaces: _javascriptInterfaces,
          );
        }
        _emit(_callbackIds.onPageFinished, <String, Object?>{
          'url': _currentUrl,
          'title': _title,
          'canGoBack': _canGoBack,
          'canGoForward': _canGoForward,
        });
      },
      onProgress: (progress) {
        final nextProgress = progress.clamp(0, 100);
        if (_progress == nextProgress) return;
        final wasIndicatorVisible = _progress > 0 && _progress < 100;
        _progress = nextProgress;
        _loading = _progress < 100;
        _updateStateSnapshot();
        _emit(_callbackIds.onProgressChanged, <String, Object?>{
          'progress': _progress,
          'url': _currentUrl,
          'title': _title,
        });
        final isIndicatorVisible = _progress > 0 && _progress < 100;
        if (mounted && wasIndicatorVisible != isIndicatorVisible) {
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
    final server = _resourceLoader;
    if (!_usesResourceLoader ||
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
    final server = _resourceLoader;
    if (_usesResourceLoader &&
        server != null &&
        server.matchesCurrentOrigin(url)) {
      return server.localUriFor(url, isMainFrame: true);
    }
    return _webViewUriFor(url, isMainFrame: true);
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

  /// Routes VFS resources and explicit interception through the native resource loader.
  bool get _usesResourceLoader {
    return _servingVfsFiles ||
        (_callbackIds.onInterceptRequest != null && widget.hostContext != null);
  }

  /// Snapshots registered methods without delaying page startup on a host message.
  Map<String, List<String>> get _javascriptInterfaces =>
      ComposeDslWebViewHostRegistry.listJavascriptInterfaces(
        executionContextKey: widget.hostContext?.executionContextKey ?? '',
        controllerKey: _boundControllerDescriptor?.key ?? '',
      );

  /// Serializes native document-start registration before navigation or interface updates.
  Future<void> _prepareDocumentStartBridge() {
    _bridgeScriptUpdate = _bridgeScriptUpdate.then((_) async {
      await _bridgeChannelRegistration;
      if (!mounted || !_supportsComposeDslPageHooks) return;
      final source = buildComposeDslWebViewBridgeRuntimeScript(
        javascriptInterfaces: _javascriptInterfaces,
      );
      if (source == _bridgeUserScriptSource) return;
      final previous = _bridgeUserScriptIdentifier;
      if (previous != null) {
        await _controller.removeDocumentStartJavaScript(previous);
        _bridgeUserScriptIdentifier = null;
        _bridgeUserScriptSource = null;
      }
      _bridgeUserScriptIdentifier = await _controller
          .addDocumentStartJavaScript(source);
      _bridgeUserScriptSource = source;
    });
    return _bridgeScriptUpdate;
  }

  /// Releases the native script handle when the owning widget is disposed.
  Future<void> _disposeBridgeUserScript() async {
    try {
      await _bridgeScriptUpdate;
      final identifier = _bridgeUserScriptIdentifier;
      if (identifier != null) {
        await _controller.removeDocumentStartJavaScript(identifier);
        _bridgeUserScriptIdentifier = null;
        _bridgeUserScriptSource = null;
      }
    } catch (error) {
      ClientLogger.e(
        'Document-start bridge cleanup failed: $error',
        tag: _composeDslWebViewLogTag,
      );
    }
  }

  /// Registers the shared resource handler before loading plugin documents.
  Future<ComposeDslWebViewResourceLoader> _ensureResourceLoader() async {
    if (_resourceLoader == null) {
      final scheme = _controller.localResourceScheme;
      if (scheme == null) {
        throw UnsupportedError(
          'This platform does not support native DSL WebView resources',
        );
      }
      final host = widget.hostContext;
      if (host == null) {
        throw StateError('Local WebView resources require a plugin identity');
      }
      final loader = ComposeDslWebViewResourceLoader(
        packageName: host.packageName,
        scheme: scheme,
        dispatchDecision: _dispatchInterceptRequestDecision,
        readFileBytes: (path) async =>
            base64.decode(await _readVfsFileBase64(path)),
      );
      _resourceLoader = loader;
      _resourceRegistration = _controller.setLocalResourceHandler(
        loader.handleRequest,
      );
    }
    await _resourceRegistration;
    if (!mounted) throw StateError('WebView has been disposed');
    return _resourceLoader!;
  }

  Future<void> _disposeResourceLoader() async {
    final loader = _resourceLoader;
    if (loader == null) return;
    await loader.close();
    try {
      await _resourceRegistration;
      await _controller.setLocalResourceHandler(null);
    } catch (error) {
      debugPrint('DSL WebView resource cleanup: $error');
    }
  }

  /// Dispatches local resources to VFS and network resources to the plugin callback.
  Future<Object?> _dispatchInterceptRequestDecision(
    Map<String, Object?> payload,
  ) async {
    final uri = Uri.parse(_string(payload['url']));
    if (uri.scheme == 'file') {
      return _readVfsResource(uri, _string(payload['method']));
    }
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

  /// Reads local browser resources through the core VFS and host file API.
  Future<Object?> _readVfsResource(Uri uri, String method) async {
    if (uri.host.isNotEmpty || (method != 'GET' && method != 'HEAD')) {
      throw StateError('VFS WebView resources require a local GET or HEAD URL');
    }
    final contentBase64 = await _readVfsFileBase64(
      Uri.decodeComponent(uri.path),
    );
    return <String, Object?>{
      'action': 'respond',
      'response': <String, Object?>{
        'statusCode': 200,
        'mimeType':
            lookupMimeType(uri.path) ??
            (throw StateError('Unknown resource MIME type')),
        'base64': method == 'HEAD' ? '' : contentBase64,
      },
    };
  }

  /// Reads resource paths through the shared VFS and host file API.
  Future<String> _readVfsFileBase64(String path) async {
    final result = await _runtimeClients.application
        .aiToolHandler()
        .executeTool(
          tool: core_proxy.CoreOperitToolsToolExecutionManagerAiTool(
            name: 'read_file_binary',
            parameters: [
              core_proxy.CoreOperitToolsToolExecutionManagerToolParameter(
                name: 'path',
                value: path,
              ),
            ],
          ),
        );
    if (!result.success) {
      throw StateError('VFS resource $path: ${result.error}');
    }
    final data = result.result.value;
    if (data is! core_proxy.BinaryFileContentData) {
      throw StateError('VFS resource did not return binary file content');
    }
    return data.contentBase64;
  }

  /// Resolves every local navigation through the native VFS resource loader.
  Future<Uri> _webViewUriFor(String url, {required bool isMainFrame}) async {
    await _prepareDocumentStartBridge();
    _servingVfsFiles = Uri.parse(url).scheme == 'file';
    if (!_usesResourceLoader) {
      return Uri.parse(url);
    }
    if (_servingVfsFiles) {
      await _refreshComposeDslJavascriptInterfaces(
        _controller,
        javascriptInterfaces: _javascriptInterfaces,
      );
    }
    final loader = await _ensureResourceLoader();
    return loader.localUriFor(url, isMainFrame: isMainFrame);
  }

  String _originalUrlFor(String url) {
    return _resourceLoader?.originalUrlFor(url) ?? url;
  }

  /// Loads the document only after its theme and host message channel are ready.
  Future<void> _load() async {
    try {
      await _themeUpdate;
      await _bridgeChannelRegistration;
      await _prepareDocumentStartBridge();
      if (!mounted) return;
      if (mounted) {
        setState(() {
          _error = null;
          _progress = 0;
          _loading = true;
        });
      }
      if (_supportsComposeDslPageHooks) {
        await _refreshComposeDslJavascriptInterfaces(
          _controller,
          javascriptInterfaces: _javascriptInterfaces,
        );
      }
      if (_request.url != null) {
        final uri = await _webViewUriFor(_request.url!, isMainFrame: true);
        await _controller.loadRequest(uri, headers: _request.headers);
      } else if (_request.html != null) {
        await _controller.loadHtmlString(
          injectComposeDslWebViewBridgeRuntimeIntoHtml(
            _request.html!,
            javascriptInterfaces: _javascriptInterfaces,
          ),
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
      resolveNavigationUri: (url) => _webViewUriFor(url, isMainFrame: true),
      prepareDocumentStartBridge: _prepareDocumentStartBridge,
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

  /// Responds to the requesting document before notifying presentation owners.
  Future<void> _handleBridgeMessage(JavaScriptMessage message) async {
    final messageText = message.message;
    final payload = _decodeJsonObject(messageText);
    if (payload == null) {
      return;
    }
    final requestId = _string(payload['id']).trim();
    final type = _string(payload['type']).trim();
    final hostContext = widget.hostContext;
    Object? data;
    try {
      data = await _handleBridgeRequest(
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
      return;
    }
    if (type == 'invoke' || type == 'dispatchAction') {
      hostContext?.onActionResultDelivered?.call(data);
    }
  }

  /// Dispatches an exact bridge operation and propagates action failures.
  Future<Object?> _handleBridgeRequest({
    required String type,
    required Object? payload,
  }) async {
    final hostContext = widget.hostContext;
    final descriptor = _boundControllerDescriptor;
    switch (type) {
      case 'controllerCommand':
        final commandPayload = jsonEncode(payload);
        final result =
            await ComposeDslWebViewHostRegistry.handleControllerCommand(
              commandPayload,
            );
        return _decodePlainJsonValue(result);
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
          payload: _decodePlainJsonValue(map['args']),
        );
        if (result.message != null) {
          throw StateError(result.message!);
        }
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
        if (result.message != null) {
          throw StateError(result.message!);
        }
        return result.actionResult;
      case 'pickFiles':
        return _pickFiles(_stringMap(payload));
      default:
        throw StateError('unsupported webview bridge request: $type');
    }
  }

  /// Awaits delivery of the response to the owning JavaScript document.
  Future<void> _postBridgeResponse(
    String requestId,
    Map<String, Object?> response,
  ) async {
    if (requestId.isEmpty) {
      return;
    }
    final payload = jsonEncode(<String, Object?>{'id': requestId, ...response});
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
    final stagedDirectory = await _runtimeClients
        .repositoryRuntimeStorageRepository
        .composeDslWebViewFilesDirPath();
    final stagedFiles = <Map<String, Object?>>[];
    for (final file in files) {
      final sourceName = file.name.trim().isEmpty
          ? Uri.file(file.path).pathSegments.last
          : file.name.trim();
      final stagedPath =
          '$stagedDirectory/${DateTime.now().microsecondsSinceEpoch}_$sourceName';
      final bytes = await file.readAsBytes();
      await _runtimeClients.repositoryRuntimeStorageRepository.writeBase64(
        path: stagedPath,
        base64Content: base64Encode(bytes),
      );
      stagedFiles.add(<String, Object?>{
        'path': stagedPath,
        'name': sourceName,
        'size': bytes.length,
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
    required this.resolveNavigationUri,
    required this.prepareDocumentStartBridge,
    required this.state,
    required this.javascriptInterfaceActionIds,
  });

  final String routeInstanceId;
  final String executionContextKey;
  final String controllerKey;
  final WebViewController controller;
  final Future<Uri> Function(String url) resolveNavigationUri;
  final Future<void> Function() prepareDocumentStartBridge;
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

/// Applies the current host descriptors without asynchronous interface discovery.
Future<void> _refreshComposeDslJavascriptInterfaces(
  WebViewController controller, {
  required Map<String, List<String>> javascriptInterfaces,
}) {
  final descriptorsJson = jsonEncode(
    javascriptInterfaces,
  ).replaceAll('<', r'\u003c');
  return controller.runJavaScript('''
    (function() {
      if (typeof window.__operitInstallComposeDslJavascriptInterfaces === 'function') {
        window.__operitInstallComposeDslJavascriptInterfaces($descriptorsJson);
      }
    })();
  ''');
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
