// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_all_windows/webview_all_windows.dart';

import '../html_preview/WorkspaceHtmlPreviewServer.dart';
import 'WorkspaceBrowserStores.dart';
import 'WorkspaceBrowserUrlUtils.dart';
import 'automation/WorkspaceBrowserAutomationController.dart';
import 'automation/WorkspaceBrowserSessionRegistry.dart';
import 'downloads/WorkspaceBrowserDownloadStore.dart';
import 'permissions/WorkspaceBrowserPermissionStore.dart';
import 'tabs/WorkspaceBrowserTabModels.dart';
import 'userscripts/WorkspaceUserscriptModels.dart';

class WorkspaceBrowserUiDelegate {
  const WorkspaceBrowserUiDelegate({
    required this.showAlertDialog,
    required this.showConfirmDialog,
    required this.showPromptDialog,
    required this.openExternalNavigation,
    required this.handlePermissionRequest,
    required this.onActivateRequested,
    required this.onCloseRequested,
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
  final VoidCallback onActivateRequested;
  final VoidCallback onCloseRequested;
}

class WorkspaceBrowserSessionStore extends ChangeNotifier {
  WorkspaceBrowserSessionStore._() {
    _sessionRegistry.setBrowserSessionOpener(
      openBrowserTab: ({url, userAgent, headers}) async {
        await openTab(url: url, userAgent: userAgent, headers: headers);
      },
    );
  }

  static final WorkspaceBrowserSessionStore instance =
      WorkspaceBrowserSessionStore._();

  static const String _homeUrl = 'https://www.bing.com';
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

  final WorkspaceBrowserStores stores = WorkspaceBrowserStores();
  final WorkspaceBrowserPermissionStore permissions =
      WorkspaceBrowserPermissionStore();
  final List<WorkspaceBrowserTabState> _tabs = <WorkspaceBrowserTabState>[];
  final Map<String, WorkspaceBrowserAutomationController> _automation =
      <String, WorkspaceBrowserAutomationController>{};
  final Map<String, String> _defaultUserAgents = <String, String>{};
  final WorkspaceBrowserSessionRegistry _sessionRegistry =
      WorkspaceBrowserSessionRegistry.instance;

  WorkspaceBrowserUiDelegate? _uiDelegate;
  Object? _uiOwner;
  WorkspaceHtmlPreviewServer? _htmlPreviewServer;
  Future<void>? _loadFuture;
  String _newTabTitle = 'New tab';
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

  WorkspaceBrowserUiDelegate get _requiredUiDelegate {
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

  void attachUi({
    required Object owner,
    required String newTabTitle,
    required WorkspaceBrowserUiDelegate delegate,
    required Future<Uint8List> Function(String path) onReadWorkspaceFileBytes,
    required Future<void> Function(String path, Uint8List bytes)
    onWriteWorkspaceFileBytes,
  }) {
    _uiOwner = owner;
    _uiDelegate = delegate;
    _newTabTitle = newTabTitle;
    configureWorkspaceAccess(
      onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
      onWriteWorkspaceFileBytes: onWriteWorkspaceFileBytes,
    );
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

  void detachUi(Object owner) {
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
    _openingTabCount += 1;
    try {
      final rawUrl = url?.trim();
      final nextUrl = rawUrl == null || rawUrl.isEmpty ? _homeUrl : rawUrl;
      await ensureLoaded();
      final normalizedUrl = normalizeWorkspaceBrowserUrl(nextUrl);
      final tab = _createTab(normalizedUrl, userAgent: userAgent);
      _configureTab(tab);
      await _applyUserAgentForTab(tab);
      _tabs.add(tab);
      _selectedIndex = _tabs.length - 1;
      notifyListeners();
      _syncSessionRegistry();
      await tab.controller.loadRequest(
        Uri.parse(normalizedUrl),
        headers: headers ?? const <String, String>{},
      );
      return tab;
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
    _uiDelegate?.onActivateRequested();
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
      _uiDelegate?.onCloseRequested();
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
    unawaited(tab.controller.loadRequest(Uri.parse(url)));
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
      unawaited(tab.controller.runJavaScript('window.stop();'));
      tab.update(isLoading: false);
      return;
    }
    unawaited(tab.controller.reload());
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
    String? localFilePath,
    String? userAgent,
  }) {
    late final WorkspaceBrowserTabState tab;
    final controller = WebViewController(
      onPermissionRequest: (request) {
        return _handlePermissionRequest(tab, request);
      },
    );
    tab = WorkspaceBrowserTabState(
      id: DateTime.now().microsecondsSinceEpoch.toString(),
      initialUrl: url,
      controller: controller,
      title: _newTabTitle,
      localFilePath: localFilePath,
      preferredUserAgent: userAgent,
    );
    final automationController = WorkspaceBrowserAutomationController(
      controller: tab.controller,
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
      selectTab: selectSession,
      closeTab: closeSession,
      navigate: navigateCurrent,
      navigateBack: goBack,
    );
    return tab;
  }

  void _configureTab(WorkspaceBrowserTabState tab) {
    tab.controller
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(Colors.transparent)
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
          _handlePopupMessage(message.message);
        },
      )
      ..addJavaScriptChannel(
        'OperitBrowserNetwork',
        onMessageReceived: (message) {
          _automation[tab.id]?.addNetworkRequest(message.message);
        },
      );
    if (_supportsJavaScriptDialogCallbacks) {
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
          if (_isDownloadUrl(request.url)) {
            unawaited(stores.downloads.startDownload(request.url));
            return NavigationDecision.prevent;
          }
          if (_isExternalAppUrl(request.url)) {
            await _requiredUiDelegate.openExternalNavigation(request.url);
            return NavigationDecision.prevent;
          }
          return NavigationDecision.navigate;
        },
        onPageStarted: (url) {
          if (tab.isDisposed) {
            return;
          }
          tab.update(
            url: url,
            addressText: url,
            isLoading: true,
            progress: 0,
            errorText: null,
          );
          _syncSessionRegistry();
          _injectUserscripts(tab, url, WorkspaceUserscriptRunAt.documentStart);
          unawaited(_installBrowserChromeHooks(tab));
        },
        onProgress: (progress) {
          if (tab.isDisposed) {
            return;
          }
          tab.update(progress: progress, isLoading: progress < 100);
        },
        onPageFinished: (url) async {
          if (tab.isDisposed) {
            return;
          }
          if (tab.desktopMode) {
            await _applyDesktopViewport(tab);
          }
          if (tab.isDisposed) {
            return;
          }
          await _injectUserscripts(
            tab,
            url,
            WorkspaceUserscriptRunAt.documentEnd,
          );
          if (tab.isDisposed) {
            return;
          }
          await Future<void>.delayed(const Duration(milliseconds: 1));
          if (tab.isDisposed) {
            return;
          }
          await _injectUserscripts(
            tab,
            url,
            WorkspaceUserscriptRunAt.documentIdle,
          );
          if (tab.isDisposed) {
            return;
          }
          await _updateTabState(tab, isLoading: false);
          if (tab.isDisposed) {
            return;
          }
          if (!isWorkspaceHtmlPreviewUrl(tab.url)) {
            stores.history.add(url: tab.url, title: tab.title);
          }
        },
        onUrlChange: (change) {
          if (tab.isDisposed) {
            return;
          }
          final url = change.url;
          if (url != null) {
            tab.update(url: url, addressText: url);
            _syncSessionRegistry();
          }
        },
        onWebResourceError: (error) {
          if (tab.isDisposed) {
            return;
          }
          tab.update(errorText: error.description, isLoading: false);
        },
        onHttpError: (error) {
          if (tab.isDisposed) {
            return;
          }
          tab.update(
            errorText: 'HTTP ${error.response?.statusCode ?? ''}',
            isLoading: false,
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

  bool get _supportsJavaScriptDialogCallbacks {
    return kIsWeb || defaultTargetPlatform != TargetPlatform.windows;
  }

  Future<void> _updateTabState(
    WorkspaceBrowserTabState tab, {
    required bool isLoading,
  }) async {
    if (tab.isDisposed) {
      return;
    }
    final url = await tab.controller.currentUrl();
    if (tab.isDisposed) {
      return;
    }
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

  void _handlePopupMessage(String rawMessage) {
    final message = jsonDecode(rawMessage) as Map<String, Object?>;
    final action = message['action'] as String;
    if (action == 'open') {
      final url = message['url'] as String;
      unawaited(openTab(url: url));
      _sessionRegistry.revealBrowserTab();
      return;
    }
    if (action == 'close') {
      closeCurrentTab();
    }
  }

  bool get _usesMobileUserAgentByDefault {
    return defaultTargetPlatform == TargetPlatform.windows ||
        defaultTargetPlatform == TargetPlatform.linux ||
        defaultTargetPlatform == TargetPlatform.macOS;
  }

  String? _defaultUserAgentForTab(WorkspaceBrowserTabState tab) {
    if (_usesMobileUserAgentByDefault) {
      return _mobileUserAgent;
    }
    return _defaultUserAgents[tab.id];
  }

  Future<void> _applyUserAgentForTab(WorkspaceBrowserTabState tab) async {
    final preferredUserAgent = tab.preferredUserAgent?.trim();
    final userAgent =
        preferredUserAgent != null && preferredUserAgent.isNotEmpty
        ? preferredUserAgent
        : tab.desktopMode
        ? _desktopUserAgent
        : _defaultUserAgentForTab(tab);
    if (userAgent == null || userAgent.trim().isEmpty) {
      return;
    }
    await tab.controller.setUserAgent(userAgent);
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

  Future<void> _handlePermissionRequest(
    WorkspaceBrowserTabState tab,
    WebViewPermissionRequest request,
  ) async {
    await _requiredUiDelegate.handlePermissionRequest(tab, request);
  }

  void _syncSessionRegistry() {
    for (var index = 0; index < _tabs.length; index += 1) {
      final tab = _tabs[index];
      _sessionRegistry.update(
        sessionId: tab.id,
        title: tab.title,
        url: tab.url,
        active: index == _selectedIndex,
      );
    }
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
