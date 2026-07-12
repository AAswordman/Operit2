// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_all_windows/webview_all_windows.dart';

import 'package:operit2/ui/features/chat/components/workspace/browser/WorkspaceBrowserStores.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/WorkspaceBrowserUrlUtils.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/downloads/WorkspaceBrowserDownloadStore.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/permissions/WorkspaceBrowserPermissionStore.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/tabs/WorkspaceBrowserTabModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/userscripts/WorkspaceUserscriptModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/html_preview/WorkspaceHtmlPreviewServer.dart';

import 'RuntimeBrowserAutomationController.dart';
import 'RuntimeBrowserSessionRegistry.dart';

class RuntimeBrowserOwnerUiDelegate {
  const RuntimeBrowserOwnerUiDelegate({
    required this.showAlertDialog,
    required this.showConfirmDialog,
    required this.showPromptDialog,
    required this.openExternalNavigation,
    required this.handlePermissionRequest,
  });

  final Future<void> Function(JavaScriptAlertDialogRequest request)
  showAlertDialog;
  final Future<bool> Function(JavaScriptConfirmDialogRequest request)
  showConfirmDialog;
  final Future<String> Function(JavaScriptTextInputDialogRequest request)
  showPromptDialog;
  final Future<void> Function(String url) openExternalNavigation;
  final Future<void> Function(
    WorkspaceBrowserTabState tab,
    WebViewPermissionRequest request,
  )
  handlePermissionRequest;
}

class RuntimeBrowserOwner extends ChangeNotifier {
  RuntimeBrowserOwner._() {
    _sessionRegistry.setBrowserSessionOpener(
      openBrowserTab: ({url, userAgent, headers}) async {
        await openTab(url: url, userAgent: userAgent, headers: headers);
      },
    );
    _sessionRegistry.setBrowserSessionUpdater(updateSessionMetadata);
  }

  static final RuntimeBrowserOwner instance = RuntimeBrowserOwner._();

  static const String _homeUrl = 'https://www.bing.com';
  static const String _logTag = 'WorkspaceBrowser';
  static const double _defaultZoomFactor = 0.4;
  static const double _minZoomFactor = 0.1;
  static const double _maxZoomFactor = 2.0;
  static const double _zoomStep = 0.1;
  static const String _mobileUserAgent =
      'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 '
      '(KHTML, like Gecko) Chrome/124.0.0.0 Mobile Safari/537.36';
  static const String _desktopUserAgent =
      'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 '
      '(KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36';
  static const GeneratedCoreProxyClients _coreClients =
      GeneratedCoreProxyClients(
        ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
      );

  final WorkspaceBrowserStores stores = WorkspaceBrowserStores();
  final WorkspaceBrowserPermissionStore permissions =
      WorkspaceBrowserPermissionStore();
  final List<WorkspaceBrowserTabState> _tabs = <WorkspaceBrowserTabState>[];
  final Map<String, RuntimeBrowserAutomationController> _automation =
      <String, RuntimeBrowserAutomationController>{};
  final Map<String, String> _defaultUserAgents = <String, String>{};
  final RuntimeBrowserSessionRegistry _sessionRegistry =
      RuntimeBrowserSessionRegistry.instance;

  RuntimeBrowserOwnerUiDelegate? _uiDelegate;
  Object? _uiOwner;
  WorkspaceHtmlPreviewServer? _htmlPreviewServer;
  Future<void>? _loadFuture;
  final String _newTabTitle = 'New tab';
  int _selectedIndex = 0;
  int _openingTabCount = 0;

  List<WorkspaceBrowserTabState> get tabs =>
      List<WorkspaceBrowserTabState>.unmodifiable(_tabs);

  int get selectedIndex => _selectedIndex;

  WorkspaceBrowserTabState? get currentTab {
    if (_tabs.isEmpty) {
      return null;
    }
    return _tabs[_selectedIndex];
  }

  RuntimeBrowserOwnerUiDelegate get _requiredUiDelegate {
    final delegate = _uiDelegate;
    if (delegate == null) {
      throw StateError('Browser UI is not attached');
    }
    return delegate;
  }

  int get activeDownloadCount {
    return stores.downloads.items
        .where(
          (item) =>
              item.state == WorkspaceBrowserDownloadState.pending ||
              item.state == WorkspaceBrowserDownloadState.running,
        )
        .length;
  }

  /// Attaches runtime-owner browser prompts to the application host.
  void attachOwnerUi({
    required Object owner,
    required RuntimeBrowserOwnerUiDelegate delegate,
  }) {
    _uiOwner = owner;
    _uiDelegate = delegate;
  }

  void configureWorkspaceAccess({
    required Future<Uint8List> Function(String path) onReadWorkspaceFileBytes,
    required Future<void> Function(String path, Uint8List bytes)
    onWriteWorkspaceFileBytes,
  }) {
    _htmlPreviewServer = WorkspaceHtmlPreviewServer(
      onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
    );
    stores.downloads.setWorkspaceSaver(onWriteWorkspaceFileBytes);
  }

  /// Detaches runtime-owner browser prompts from the application host.
  void detachOwnerUi(Object owner) {
    if (!identical(_uiOwner, owner)) {
      return;
    }
    _uiOwner = null;
    _uiDelegate = null;
  }

  Future<void> ensureLoaded() {
    final existing = _loadFuture;
    if (existing != null) {
      return existing;
    }
    final next = stores.load();
    _loadFuture = next;
    return next;
  }

  Future<void> openInitialTab({
    String? initialUrl,
    String? initialUserAgent,
    Map<String, String>? initialHeaders,
    String? initialFilePath,
    String? initialWorkspaceHtmlPath,
  }) async {
    await ensureLoaded();
    if (_tabs.isNotEmpty || _openingTabCount > 0) {
      notifyListeners();
      return;
    }
    final explicitFilePath = initialFilePath;
    if (explicitFilePath != null && explicitFilePath.trim().isNotEmpty) {
      await openLocalFileTab(explicitFilePath);
      return;
    }
    final explicitWorkspaceHtmlPath = initialWorkspaceHtmlPath;
    if (explicitWorkspaceHtmlPath != null &&
        explicitWorkspaceHtmlPath.trim().isNotEmpty) {
      await openWorkspaceHtmlTab(
        explicitWorkspaceHtmlPath,
        initialUrl: initialUrl,
      );
      return;
    }
    final explicitUrl = initialUrl;
    if (explicitUrl != null && explicitUrl.trim().isNotEmpty) {
      await openTab(
        url: explicitUrl,
        userAgent: initialUserAgent,
        headers: initialHeaders,
      );
      return;
    }
    await openTab();
  }

  Future<WorkspaceBrowserTabState> openTab({
    String? url,
    String? userAgent,
    Map<String, String>? headers,
  }) async {
    final stopwatch = Stopwatch()..start();
    _openingTabCount += 1;
    try {
      final rawUrl = url?.trim();
      final nextUrl = rawUrl == null || rawUrl.isEmpty ? _homeUrl : rawUrl;
      await ensureLoaded();
      final normalizedUrl = normalizeWorkspaceBrowserUrl(nextUrl);
      final requestHeaders = headers ?? const <String, String>{};
      ClientLogger.i(
        'openTab start url=$normalizedUrl headers=${requestHeaders.length} userAgentSet=${userAgent != null}',
        tag: _logTag,
      );
      final tab = _createTab(
        normalizedUrl,
        userAgent: userAgent,
        headers: requestHeaders,
        capabilities: _capabilitiesForUrl(normalizedUrl),
      );
      _configureTab(tab);
      await _applyUserAgentForTab(tab);
      _tabs.add(tab);
      _selectedIndex = _tabs.length - 1;
      notifyListeners();
      _syncSessionRegistry();
      await tab.controller.loadRequest(
        Uri.parse(normalizedUrl),
        headers: requestHeaders,
      );
      ClientLogger.i(
        'openTab done tab=${tab.id} elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
      );
      return tab;
    } catch (error, stackTrace) {
      ClientLogger.e(
        'openTab failed elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
      rethrow;
    } finally {
      _openingTabCount -= 1;
    }
  }

  Future<WorkspaceBrowserTabState> openLocalFileTab(String absolutePath) async {
    _openingTabCount += 1;
    try {
      await ensureLoaded();
      final tab = _createTab(
        'file://$absolutePath',
        localFilePath: absolutePath,
        capabilities: _capabilitiesForUrl('file://$absolutePath'),
      );
      _configureTab(tab);
      await _applyUserAgentForTab(tab);
      _tabs.add(tab);
      _selectedIndex = _tabs.length - 1;
      notifyListeners();
      _syncSessionRegistry();
      await tab.controller.loadFile(absolutePath);
      return tab;
    } finally {
      _openingTabCount -= 1;
    }
  }

  Future<WorkspaceBrowserTabState> openWorkspaceHtmlTab(
    String relativePath, {
    String? initialUrl,
  }) async {
    _openingTabCount += 1;
    try {
      await ensureLoaded();
      final server = _htmlPreviewServer;
      if (server == null) {
        throw StateError('Workspace HTML preview server is not attached');
      }
      final uri = await server.start(relativePath);
      final url = initialUrl?.trim().isNotEmpty == true
          ? initialUrl!.trim()
          : uri.toString();
      return await openTab(url: url);
    } finally {
      _openingTabCount -= 1;
    }
  }

  void selectSession(String sessionId) {
    final index = _tabs.indexWhere((tab) => tab.id == sessionId);
    if (index < 0) {
      throw StateError('Browser session is not registered');
    }
    _selectedIndex = index;
    notifyListeners();
    _syncSessionRegistry();
  }

  void closeSession(String sessionId) {
    final index = _tabs.indexWhere((tab) => tab.id == sessionId);
    if (index < 0) {
      throw StateError('Browser session is not registered');
    }
    _closeTabAt(index);
  }

  void closeCurrentTab() {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    closeSession(tab.id);
  }

  void closeTabAt(int index) {
    if (index < 0 || index >= _tabs.length) {
      return;
    }
    _closeTabAt(index);
  }

  void _closeTabAt(int index) {
    final removed = _tabs.removeAt(index);
    _automation.remove(removed.id);
    _defaultUserAgents.remove(removed.id);
    _sessionRegistry.unregister(removed.id);
    removed.dispose();
    if (_tabs.isEmpty) {
      _selectedIndex = 0;
      notifyListeners();
      return;
    }
    if (_selectedIndex >= _tabs.length) {
      _selectedIndex = _tabs.length - 1;
    } else if (_selectedIndex > index) {
      _selectedIndex -= 1;
    }
    notifyListeners();
    _syncSessionRegistry();
  }

  void navigateCurrent(String rawUrl) {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    final url = normalizeWorkspaceBrowserUrl(rawUrl);
    tab.update(url: url, addressText: url, errorText: null);
    _syncSessionRegistry();
    unawaited(
      tab.controller.loadRequest(Uri.parse(url), headers: tab.requestHeaders),
    );
  }

  /// Updates request metadata on one real owner WebView session.
  Future<void> updateSessionMetadata({
    required String sessionId,
    required String? userAgent,
    required Map<String, String> headers,
  }) async {
    final tab = _tabs.where((item) => item.id == sessionId).single;
    tab.updateRequestMetadata(userAgent: userAgent, headers: headers);
    await _applyUserAgentForTab(tab);
    await tab.controller.reload();
  }

  void goBack() {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    unawaited(tab.controller.goBack());
  }

  void goForward() {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    unawaited(tab.controller.goForward());
  }

  void refreshOrStop() {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    if (tab.isLoading) {
      stopCurrentLoad();
      return;
    }
    unawaited(tab.controller.reload());
  }

  /// Reloads the current browser session.
  void reloadCurrent() {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    unawaited(tab.controller.reload());
  }

  /// Stops loading in the current browser session.
  void stopCurrentLoad() {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    if (_supportsPageJavaScript(tab)) {
      unawaited(tab.controller.runJavaScript('window.stop();'));
    }
    tab.update(isLoading: false, progress: 100);
    _syncSessionRegistry(eventType: 'stopped', sessionId: tab.id);
  }

  void toggleBookmark() {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    stores.bookmarks.toggle(url: tab.url, title: tab.title);
    notifyListeners();
  }

  Future<void> setDesktopMode(bool enabled) async {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    tab.update(desktopMode: enabled);
    await _applyUserAgentForTab(tab);
    await tab.controller.reload();
  }

  void zoomIn() {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    unawaited(_setZoomFactor(tab.zoomFactor + _zoomStep));
  }

  void zoomOut() {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    unawaited(_setZoomFactor(tab.zoomFactor - _zoomStep));
  }

  void resetZoom() {
    unawaited(_setZoomFactor(_defaultZoomFactor));
  }

  Future<void> _setZoomFactor(double zoomFactor) async {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    final nextZoomFactor = zoomFactor
        .clamp(_minZoomFactor, _maxZoomFactor)
        .toDouble();
    if ((tab.zoomFactor - nextZoomFactor).abs() < 0.0001) {
      return;
    }
    tab.update(zoomFactor: nextZoomFactor);
    await _applyZoomFactor(tab);
  }

  WorkspaceBrowserTabState _createTab(
    String url, {
    required WorkspaceBrowserSessionCapabilities capabilities,
    Map<String, String> headers = const <String, String>{},
    String? localFilePath,
    String? userAgent,
  }) {
    late final WorkspaceBrowserTabState tab;
    final controller = capabilities.permissionRequests
        ? WebViewController(
            onPermissionRequest: (request) {
              return _handlePermissionRequest(tab, request);
            },
          )
        : WebViewController();
    tab = WorkspaceBrowserTabState(
      id: DateTime.now().microsecondsSinceEpoch.toString(),
      initialUrl: url,
      controller: controller,
      title: _newTabTitle,
      capabilities: capabilities,
      localFilePath: localFilePath,
      preferredUserAgent: userAgent,
      requestHeaders: Map<String, String>.unmodifiable(headers),
    );
    late final RuntimeBrowserAutomationController automationController;
    automationController = RuntimeBrowserAutomationController(
      controller: tab.controller,
      onSurfaceFrameRequested: () =>
          _publishSurfaceFrame(tab.id, automationController),
    );
    _automation[tab.id] = automationController;
    tab.controller.getUserAgent().then((value) {
      if (value != null) {
        _defaultUserAgents[tab.id] = value;
      }
    });
    _sessionRegistry.register(
      sessionId: tab.id,
      controller: automationController,
      title: tab.title,
      url: tab.url,
      active: true,
      userAgent: tab.preferredUserAgent,
      canGoBack: tab.canGoBack,
      canGoForward: tab.canGoForward,
      isLoading: tab.isLoading,
      progress: tab.progress,
      selectTab: selectSession,
      closeTab: closeSession,
      navigate: navigateCurrent,
      navigateBack: goBack,
      navigateForward: goForward,
      reload: reloadCurrent,
      stop: stopCurrentLoad,
      supportsPageJavaScript: () => _supportsPageJavaScript(tab),
    );
    return tab;
  }

  /// Configures one owner-host browser session controller.
  void _configureTab(WorkspaceBrowserTabState tab) {
    tab.controller
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(Colors.transparent);
    if (tab.capabilities.pageHooks) {
      tab.controller
        ..setOnConsoleMessage((message) {
          _automation[tab.id]?.addConsoleMessage(message);
          stores.userscripts.addLog('console', message.message);
        })
        ..addJavaScriptChannel(
          'OperitUserscriptStorage',
          onMessageReceived: (message) {
            stores.userscripts.handleStorageMessage(message.message);
          },
        )
        ..addJavaScriptChannel(
          'OperitUserscriptRuntime',
          onMessageReceived: (message) {
            stores.userscripts.handleRuntimeMessage(message.message);
            notifyListeners();
          },
        )
        ..addJavaScriptChannel(
          'OperitBrowserPopup',
          onMessageReceived: (message) {
            unawaited(_handlePopupMessage(tab.id, message.message));
          },
        )
        ..addJavaScriptChannel(
          'OperitBrowserNetwork',
          onMessageReceived: (message) {
            _automation[tab.id]?.addNetworkRequest(message.message);
          },
        );
    }
    if (tab.capabilities.javaScriptDialogs) {
      tab.controller
        ..setOnJavaScriptAlertDialog((request) {
          return _requiredUiDelegate.showAlertDialog(request);
        })
        ..setOnJavaScriptConfirmDialog((request) {
          return _requiredUiDelegate.showConfirmDialog(request);
        })
        ..setOnJavaScriptTextInputDialog((request) {
          return _requiredUiDelegate.showPromptDialog(request);
        });
    }
    tab.controller.setNavigationDelegate(
      NavigationDelegate(
        onNavigationRequest: (request) async {
          final logicalUrl = _logicalUrl(request.url);
          if (_isDownloadUrl(logicalUrl)) {
            unawaited(stores.downloads.startDownload(logicalUrl));
            return NavigationDecision.prevent;
          }
          if (_isExternalAppUrl(logicalUrl)) {
            await _requiredUiDelegate.openExternalNavigation(logicalUrl);
            return NavigationDecision.prevent;
          }
          return NavigationDecision.navigate;
        },
        onPageStarted: (url) {
          if (tab.isDisposed) {
            return;
          }
          final logicalUrl = _logicalUrl(url);
          ClientLogger.d(
            'page started tab=${tab.id} url=$logicalUrl',
            tag: _logTag,
          );
          final hasLogicalUrl = logicalUrl.isNotEmpty;
          tab.update(
            url: hasLogicalUrl ? logicalUrl : null,
            addressText: hasLogicalUrl ? logicalUrl : null,
            isLoading: true,
            progress: 0,
            errorText: null,
          );
          _syncSessionRegistry(eventType: 'started', sessionId: tab.id);
          if (_supportsPageJavaScript(tab)) {
            _injectUserscripts(
              tab,
              logicalUrl,
              WorkspaceUserscriptRunAt.documentStart,
            );
            unawaited(_installBrowserChromeHooks(tab));
          }
        },
        onProgress: (progress) {
          if (tab.isDisposed) {
            return;
          }
          tab.update(progress: progress, isLoading: progress < 100);
          _syncSessionRegistry(eventType: 'progress', sessionId: tab.id);
        },
        onPageFinished: (url) async {
          if (tab.isDisposed) {
            return;
          }
          final logicalUrl = _logicalUrl(url);
          ClientLogger.d(
            'page finished tab=${tab.id} url=$logicalUrl',
            tag: _logTag,
          );
          if (tab.isDisposed) {
            return;
          }
          if (tab.desktopMode && _supportsPageJavaScript(tab)) {
            await _applyDesktopViewport(tab);
          }
          if (tab.isDisposed) {
            return;
          }
          if (_supportsPageJavaScript(tab)) {
            await _injectUserscripts(
              tab,
              logicalUrl,
              WorkspaceUserscriptRunAt.documentEnd,
            );
          }
          if (tab.isDisposed) {
            return;
          }
          await Future<void>.delayed(const Duration(milliseconds: 1));
          if (tab.isDisposed) {
            return;
          }
          if (_supportsPageJavaScript(tab)) {
            await _injectUserscripts(
              tab,
              logicalUrl,
              WorkspaceUserscriptRunAt.documentIdle,
            );
          }
          if (tab.isDisposed) {
            return;
          }
          await _updateTabState(tab, isLoading: false);
          if (tab.isDisposed) {
            return;
          }
          if (!isWorkspaceHtmlPreviewUrl(tab.url)) {
            stores.history.add(url: logicalUrl, title: tab.title);
          }
          _automation[tab.id]?.requestSurfaceFrame();
        },
        onUrlChange: (change) {
          if (tab.isDisposed) {
            return;
          }
          final url = change.url;
          if (url != null) {
            final logicalUrl = _logicalUrl(url);
            if (logicalUrl.isNotEmpty) {
              tab.update(url: logicalUrl, addressText: logicalUrl);
              _syncSessionRegistry(eventType: 'urlChanged', sessionId: tab.id);
            }
          }
        },
        onWebResourceError: (error) {
          if (tab.isDisposed) {
            return;
          }
          ClientLogger.e(
            'web resource error tab=${tab.id} url=${error.url} description=${error.description}',
            tag: _logTag,
          );
          tab.update(errorText: error.description, isLoading: false);
          _syncSessionRegistry(
            eventType: 'error',
            sessionId: tab.id,
            error: error.description,
          );
        },
        onHttpError: (error) {
          if (tab.isDisposed) {
            return;
          }
          ClientLogger.w(
            'http error tab=${tab.id} status=${error.response?.statusCode} url=${error.request?.uri}',
            tag: _logTag,
          );
          tab.update(
            errorText: 'HTTP ${error.response?.statusCode ?? ''}',
            isLoading: false,
          );
          _syncSessionRegistry(
            eventType: 'error',
            sessionId: tab.id,
            error: 'HTTP ${error.response?.statusCode ?? ''}',
          );
        },
        onSslAuthError: (request) {
          if (tab.isDisposed) {
            return;
          }
          request.cancel();
          tab.update(errorText: 'SSL certificate error', isLoading: false);
        },
      ),
    );
    unawaited(_applyZoomFactor(tab));
  }

  /// Updates the owner-host browser session snapshot from the controller.
  Future<void> _updateTabState(
    WorkspaceBrowserTabState tab, {
    required bool isLoading,
  }) async {
    if (tab.isDisposed) {
      return;
    }
    final controllerUrl = await tab.controller.currentUrl();
    if (tab.isDisposed) {
      return;
    }
    final url = controllerUrl == null ? null : _logicalUrl(controllerUrl);
    final title = await tab.controller.getTitle();
    if (tab.isDisposed) {
      return;
    }
    final canGoBack = await tab.controller.canGoBack();
    if (tab.isDisposed) {
      return;
    }
    final canGoForward = await tab.controller.canGoForward();
    if (tab.isDisposed) {
      return;
    }
    tab.update(
      url: url ?? tab.url,
      addressText: url ?? tab.url,
      title: (title == null || title.trim().isEmpty) ? tab.url : title,
      canGoBack: canGoBack,
      canGoForward: canGoForward,
      isLoading: isLoading,
      progress: isLoading ? tab.progress : 100,
    );
    _syncSessionRegistry();
  }

  Future<void> _injectUserscripts(
    WorkspaceBrowserTabState tab,
    String url,
    WorkspaceUserscriptRunAt runAt,
  ) async {
    if (tab.isDisposed) {
      return;
    }
    if (!_supportsPageJavaScript(tab)) {
      return;
    }
    await stores.userscriptRuntime.injectForUrl(
      tab.controller,
      url,
      runAt: runAt,
    );
  }

  Future<void> _installBrowserChromeHooks(WorkspaceBrowserTabState tab) {
    if (tab.isDisposed) {
      return Future<void>.value();
    }
    if (!_supportsPageJavaScript(tab)) {
      return Future<void>.value();
    }
    return tab.controller.runJavaScript(r'''
(function() {
  if (window.__operitBrowserChromeHooksInstalled) return;
  window.__operitBrowserChromeHooksInstalled = true;
  const originalOpen = window.open;
  window.open = function(url, target, features) {
    if (url && window.OperitBrowserPopup && window.OperitBrowserPopup.postMessage) {
      window.OperitBrowserPopup.postMessage(JSON.stringify({
        action: 'open',
        url: String(url)
      }));
      return null;
    }
    return originalOpen.call(window, url, target, features);
  };
  const originalClose = window.close;
  window.close = function() {
    if (window.OperitBrowserPopup && window.OperitBrowserPopup.postMessage) {
      window.OperitBrowserPopup.postMessage(JSON.stringify({ action: 'close' }));
      return;
    }
    originalClose.call(window);
  };
  function reportNetwork(entry) {
    if (window.OperitBrowserNetwork && window.OperitBrowserNetwork.postMessage) {
      window.OperitBrowserNetwork.postMessage(JSON.stringify(entry));
    }
  }
  const originalFetch = window.fetch;
  window.fetch = function(input, init) {
    const method = init && init.method ? String(init.method) : 'GET';
    const url = typeof input === 'string' ? input : String(input && input.url || input);
    const startedAt = Date.now();
    return originalFetch.apply(this, arguments).then(function(response) {
      reportNetwork({
        type: 'fetch',
        method: method,
        url: url,
        status: response.status,
        statusText: response.statusText,
        durationMs: Date.now() - startedAt
      });
      return response;
    }).catch(function(error) {
      reportNetwork({
        type: 'fetch',
        method: method,
        url: url,
        error: String(error),
        durationMs: Date.now() - startedAt
      });
      throw error;
    });
  };
  const OriginalXMLHttpRequest = window.XMLHttpRequest;
  window.XMLHttpRequest = function() {
    const xhr = new OriginalXMLHttpRequest();
    let method = 'GET';
    let url = '';
    const originalOpen = xhr.open;
    xhr.open = function(nextMethod, nextUrl) {
      method = String(nextMethod || 'GET');
      url = String(nextUrl || '');
      return originalOpen.apply(xhr, arguments);
    };
    const startedAt = Date.now();
    xhr.addEventListener('loadend', function() {
      reportNetwork({
        type: 'xhr',
        method: method,
        url: url,
        status: xhr.status,
        statusText: xhr.statusText,
        durationMs: Date.now() - startedAt
      });
    });
    return xhr;
  };
})();
''');
  }

  /// Handles a popup command emitted by one real owner WebView session.
  Future<void> _handlePopupMessage(
    String sourceSessionId,
    String rawMessage,
  ) async {
    final message = jsonDecode(rawMessage) as Map<String, Object?>;
    final action = message['action'] as String;
    if (action == 'open') {
      final url = message['url'] as String;
      final opened = await openTab(url: url);
      final session = _sessionRegistry.sessions
          .where((item) => item.sessionId == opened.id)
          .single;
      await _publishBrowserSessionEvent(
        sessionId: sourceSessionId,
        eventType: 'popupCreated',
        resultJson: jsonEncode(session.toRuntimeBrowserSessionInfo().toJson()),
      );
      return;
    }
    if (action == 'close') {
      closeCurrentTab();
    }
  }

  bool get _usesMobileUserAgentByDefault {
    return !kIsWeb &&
        (defaultTargetPlatform == TargetPlatform.windows ||
            defaultTargetPlatform == TargetPlatform.linux ||
            defaultTargetPlatform == TargetPlatform.macOS);
  }

  String? _defaultUserAgentForTab(WorkspaceBrowserTabState tab) {
    if (_usesMobileUserAgentByDefault) {
      return _mobileUserAgent;
    }
    return _defaultUserAgents[tab.id];
  }

  Future<void> _applyUserAgentForTab(WorkspaceBrowserTabState tab) async {
    final userAgent = _requestedUserAgent(
      preferredUserAgent: tab.preferredUserAgent,
      desktopMode: tab.desktopMode,
      defaultUserAgent: _defaultUserAgentForTab(tab),
    );
    if (kIsWeb) {
      return;
    }
    if (userAgent == null || userAgent.trim().isEmpty) {
      return;
    }
    await tab.controller.setUserAgent(userAgent);
  }

  /// Resolves the user agent owned by a browser session runtime.
  String? _requestedUserAgent({
    required String? preferredUserAgent,
    required bool desktopMode,
    String? defaultUserAgent,
  }) {
    final preferred = preferredUserAgent?.trim();
    if (preferred != null && preferred.isNotEmpty) {
      return preferred;
    }
    if (desktopMode) {
      return _desktopUserAgent;
    }
    return defaultUserAgent;
  }

  /// Returns the logical page URL shown by the browser session.
  String _logicalUrl(String url) {
    return url.trim();
  }

  /// Returns the browser host capabilities for one URL.
  WorkspaceBrowserSessionCapabilities _capabilitiesForUrl(String url) {
    final pageJavaScript = _pageJavaScriptSupportedByHost(url);
    return WorkspaceBrowserSessionCapabilities(
      pageJavaScript: pageJavaScript,
      pageHooks: pageJavaScript,
      permissionRequests: !kIsWeb,
      javaScriptDialogs:
          pageJavaScript &&
          (kIsWeb || defaultTargetPlatform != TargetPlatform.windows),
    );
  }

  /// Returns whether the owner host can execute page JavaScript for this URL.
  bool _pageJavaScriptSupportedByHost(String url) {
    if (kIsWeb) {
      return false;
    }
    return true;
  }

  /// Returns whether the session exposes page JavaScript to host commands.
  bool _supportsPageJavaScript(WorkspaceBrowserTabState tab) {
    if (tab.isDisposed) {
      return false;
    }
    if (!tab.capabilities.pageJavaScript) {
      return false;
    }
    return _pageJavaScriptSupportedByHost(tab.url);
  }

  Future<void> _applyZoomFactor(WorkspaceBrowserTabState tab) async {
    if (defaultTargetPlatform != TargetPlatform.windows) {
      return;
    }
    final platform = tab.controller.platform;
    if (platform is! WindowsWebViewController) {
      return;
    }
    await platform.setZoomFactor(tab.zoomFactor);
  }

  Future<void> _applyDesktopViewport(WorkspaceBrowserTabState tab) {
    if (tab.isDisposed) {
      return Future<void>.value();
    }
    if (!_supportsPageJavaScript(tab)) {
      return Future<void>.value();
    }
    return tab.controller.runJavaScript(r'''
(function() {
  let viewport = document.querySelector('meta[name="viewport"]');
  if (!viewport) {
    viewport = document.createElement('meta');
    viewport.setAttribute('name', 'viewport');
    document.head.appendChild(viewport);
  }
  viewport.setAttribute('content', 'width=1280, initial-scale=1.0');
})();
''');
  }

  void _handlePermissionRequest(
    WorkspaceBrowserTabState tab,
    WebViewPermissionRequest request,
  ) {
    _requiredUiDelegate.handlePermissionRequest(tab, request);
  }

  /// Synchronizes browser tab state into the runtime session registry.
  void _syncSessionRegistry({
    String eventType = 'updated',
    String? sessionId,
    String? error,
  }) {
    for (var index = 0; index < _tabs.length; index += 1) {
      final tab = _tabs[index];
      _sessionRegistry.update(
        sessionId: tab.id,
        title: tab.title,
        url: tab.url,
        active: index == _selectedIndex,
        userAgent: tab.preferredUserAgent,
        canGoBack: tab.canGoBack,
        canGoForward: tab.canGoForward,
        isLoading: tab.isLoading,
        progress: tab.progress,
      );
    }
    if (sessionId != null) {
      unawaited(
        _publishBrowserSessionEvent(
          sessionId: sessionId,
          eventType: eventType,
          error: error,
        ),
      );
    }
  }

  /// Publishes one browser session state event to runtime watchers.
  Future<void> _publishBrowserSessionEvent({
    required String sessionId,
    required String eventType,
    String resultJson = '',
    Uint8List? frameData,
    String? frameCodec,
    int? frameWidth,
    int? frameHeight,
    String? error,
  }) {
    final session = _sessionRegistry.sessions
        .where((item) => item.sessionId == sessionId)
        .firstOrNull;
    if (session == null) {
      return Future<void>.value();
    }
    return _coreClients.servicesRuntimeBrowserService
        .publishBrowserSessionEvent(
          event: RuntimeBrowserSessionEvent(
            sessionId: sessionId,
            eventType: eventType,
            session: session.toRuntimeBrowserSessionInfo(),
            resultJson: resultJson,
            frameData: frameData ?? Uint8List(0),
            frameCodec: frameCodec,
            frameWidth: frameWidth,
            frameHeight: frameHeight,
            error: error,
          ),
        );
  }

  /// Captures and publishes one frame from the real owner WebView surface.
  Future<void> _publishSurfaceFrame(
    String sessionId,
    RuntimeBrowserAutomationController controller,
  ) async {
    final frame = await controller.captureSurfaceFrame();
    await _publishBrowserSessionEvent(
      sessionId: sessionId,
      eventType: 'surfaceFrame',
      frameData: frame.data,
      frameCodec: frame.codec,
      frameWidth: frame.width.round(),
      frameHeight: frame.height.round(),
    );
  }

  bool _isDownloadUrl(String url) {
    final lower = url.toLowerCase();
    return lower.endsWith('.zip') ||
        lower.endsWith('.apk') ||
        lower.endsWith('.exe') ||
        lower.endsWith('.dmg') ||
        lower.endsWith('.tar.gz') ||
        lower.endsWith('.7z');
  }

  bool _isExternalAppUrl(String url) {
    final uri = Uri.tryParse(url);
    if (uri == null) {
      return false;
    }
    return uri.hasScheme &&
        uri.scheme != 'http' &&
        uri.scheme != 'https' &&
        uri.scheme != 'file' &&
        uri.scheme != 'about' &&
        uri.scheme != 'data' &&
        uri.scheme != 'blob' &&
        uri.scheme != 'javascript';
  }
}
