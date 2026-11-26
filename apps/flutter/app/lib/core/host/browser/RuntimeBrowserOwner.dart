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
import 'package:operit2/core/runtime/RuntimeFileBytes.dart';
import 'package:webview_all/webview_all.dart';

import 'package:operit2/ui/features/chat/components/workspace/browser/WorkspaceBrowserStores.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/WorkspaceBrowserUrlUtils.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/downloads/WorkspaceBrowserDownloadStore.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/permissions/WorkspaceBrowserPermissionStore.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/tabs/WorkspaceBrowserTabModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/userscripts/WorkspaceUserscriptModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/html_preview/WorkspaceHtmlPreviewServer.dart';

import 'RuntimeBrowserAutomationController.dart';
import 'RuntimeBrowserPageDownloads.dart';
import 'RuntimeBrowserExternalNavigation.dart';
import 'RuntimeBrowserSessionRegistry.dart';
import 'RuntimeBrowserVfsResources.dart';

class RuntimeBrowserOwnerUiDelegate {
  const RuntimeBrowserOwnerUiDelegate({
    required this.showAlertDialog,
    required this.showConfirmDialog,
    required this.showPromptDialog,
    required this.openExternalNavigation,
    required this.handlePermissionRequest,
  });

  final Future<void> Function(
    JavaScriptAlertDialogRequest request,
    Future<void> dismissed,
  )
  showAlertDialog;
  final Future<bool> Function(
    JavaScriptConfirmDialogRequest request,
    Future<void> dismissed,
  )
  showConfirmDialog;
  final Future<String?> Function(
    JavaScriptTextInputDialogRequest request,
    Future<void> dismissed,
  )
  showPromptDialog;
  final Future<void> Function(WorkspaceBrowserTabState tab, String url)
  openExternalNavigation;
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
        final tab = await openTab(
          url: url,
          userAgent: userAgent,
          headers: headers,
        );
        return tab.id;
      },
    );
    _sessionRegistry.setBrowserSessionUpdater(updateSessionMetadata);
    _sessionRegistry.setDownloadControl(_controlDownload);
    _sessionRegistry.setExternalNavigationHandler(_openExternalApplication);
    stores.downloads.addListener(_publishDownloads);
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
  final RuntimeBrowserVfsResources _vfsResources = RuntimeBrowserVfsResources(
    (path) => readRuntimeFileBytes(_coreClients, path),
  );

  final WorkspaceBrowserStores stores = WorkspaceBrowserStores(
    clients: _coreClients,
  );
  final WorkspaceBrowserPermissionStore permissions =
      WorkspaceBrowserPermissionStore();
  final List<WorkspaceBrowserTabState> _tabs = <WorkspaceBrowserTabState>[];
  final Map<String, RuntimeBrowserAutomationController> _automation =
      <String, RuntimeBrowserAutomationController>{};
  final Map<String, String> _defaultUserAgents = <String, String>{};
  final Map<String, int> _workspaceSurfaceSessionClaims = <String, int>{};
  final RuntimeBrowserSessionRegistry _sessionRegistry =
      RuntimeBrowserSessionRegistry.instance;

  RuntimeBrowserOwnerUiDelegate? _uiDelegate;
  Object? _uiOwner;
  WorkspaceHtmlPreviewServer? _htmlPreviewServer;
  Future<void>? _loadFuture;
  bool _loaded = false;
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

  /// Returns the owner tab with the requested session identifier.
  WorkspaceBrowserTabState? tabForSession(String sessionId) {
    for (final tab in _tabs) {
      if (tab.id == sessionId) {
        return tab;
      }
    }
    return null;
  }

  /// Returns whether a session is currently mounted in the workspace tree.
  bool isWorkspaceSurfaceSession(String sessionId) {
    return (_workspaceSurfaceSessionClaims[sessionId] ?? 0) > 0;
  }

  /// Marks one owner WebView as mounted by the workspace tree.
  void attachWorkspaceSurfaceSession(String sessionId) {
    final current = _workspaceSurfaceSessionClaims[sessionId] ?? 0;
    _workspaceSurfaceSessionClaims[sessionId] = current + 1;
    if (current == 0) {
      notifyListeners();
    }
  }

  /// Marks one owner WebView as no longer mounted by the workspace tree.
  void detachWorkspaceSurfaceSession(String sessionId) {
    final current = _workspaceSurfaceSessionClaims[sessionId] ?? 0;
    if (current <= 1) {
      if (_workspaceSurfaceSessionClaims.remove(sessionId) != null) {
        notifyListeners();
      }
    } else {
      _workspaceSurfaceSessionClaims[sessionId] = current - 1;
    }
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

  /// Configures workspace-backed file access for owner browser sessions.
  void configureWorkspaceAccess({
    required Future<Uint8List> Function(String path) onReadWorkspaceFileBytes,
    required Future<void> Function(String path, Uint8List bytes)
    onWriteWorkspaceFileBytes,
  }) {
    _htmlPreviewServer = WorkspaceHtmlPreviewServer(
      onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
    );
  }

  /// Detaches runtime-owner browser prompts from the application host.
  void detachOwnerUi(Object owner) {
    if (!identical(_uiOwner, owner)) {
      return;
    }
    _uiOwner = null;
    _uiDelegate = null;
  }

  /// Loads persistent owner browser stores once per successful initialization.
  Future<void> ensureLoaded() {
    if (_loaded) {
      return Future<void>.value();
    }
    final existing = _loadFuture;
    if (existing != null) {
      return existing;
    }
    final next = stores.load().then((_) {
      _loaded = true;
    });
    _loadFuture = next;
    return next.whenComplete(() {
      if (identical(_loadFuture, next)) {
        _loadFuture = null;
      }
    });
  }

  /// Opens the initial owner browser tab for explicit startup targets.
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
      ClientLogger.v(
        'openTab start url=$normalizedUrl headers=${requestHeaders.length} userAgentSet=${userAgent != null}',
        tag: _logTag,
        verboseLevel: VerboseLevel.level2,
      );
      final tab = _createTab(
        normalizedUrl,
        userAgent: userAgent,
        headers: requestHeaders,
        capabilities: _capabilitiesForUrl(normalizedUrl),
      );
      await _attachTab(tab);
      await tab.controller.loadRequest(
        _vfsResources.navigationUri(
          normalizedUrl,
          tab.controller.localResourceScheme,
        ),
        headers: requestHeaders,
      );
      ClientLogger.v(
        'openTab done tab=${tab.id} elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
        verboseLevel: VerboseLevel.level2,
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
      await _attachTab(tab);
      await tab.controller.loadFile(absolutePath);
      return tab;
    } finally {
      _openingTabCount -= 1;
    }
  }

  /// Mounts the native view before awaiting configuration that requires attachment.
  Future<void> _attachTab(WorkspaceBrowserTabState tab) async {
    _tabs.add(tab);
    _selectedIndex = _tabs.length - 1;
    notifyListeners();
    try {
      await _configureTab(tab);
      _requireLiveTab(tab);
      await _applyUserAgentForTab(tab);
      _requireLiveTab(tab);
    } catch (_) {
      if (!tab.isDisposed) {
        _tabs.remove(tab);
        _selectedIndex = _tabs.isEmpty
            ? 0
            : _selectedIndex.clamp(0, _tabs.length - 1);
        unawaited(tab.controller.dispose());
        _releaseTab(tab);
        notifyListeners();
      }
      rethrow;
    }
    _syncSessionRegistry();
  }

  /// Prevents asynchronous setup from reviving a tab closed by its owner.
  void _requireLiveTab(WorkspaceBrowserTabState tab) {
    if (tab.isDisposed) {
      throw StateError('The browser tab closed during initialization');
    }
  }

  /// Opens a browser tab backed by the workspace HTML preview server.
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

  /// Removes the tab from the owner and updates the selected session.
  void _closeTabAt(int index) {
    final removed = _tabs.removeAt(index);
    _releaseTab(removed);
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

  /// Drops the registry and automation state before disposing the tab's browser.
  void _releaseTab(WorkspaceBrowserTabState tab) {
    final downloads = _pageDownloads.remove(tab.id);
    if (downloads != null) unawaited(downloads.close());
    stores.downloads.closeEngineSession(tab.id);
    _automation.remove(tab.id)?.dialogs.close();
    _defaultUserAgents.remove(tab.id);
    _sessionRegistry.unregister(tab.id);
    _workspaceSurfaceSessionClaims.remove(tab.id);
    tab.dispose();
  }

  /// Navigates the current real owner WebView.
  Future<void> navigateCurrent(String rawUrl) async {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    final url = normalizeWorkspaceBrowserUrl(rawUrl);
    if (isRuntimeBrowserExternalUrl(url)) {
      await _requiredUiDelegate.openExternalNavigation(tab, url);
      return;
    }
    tab.update(url: url, addressText: url, errorText: null);
    _syncSessionRegistry();
    await tab.controller.loadRequest(
      _vfsResources.navigationUri(url, tab.controller.localResourceScheme),
      headers: tab.requestHeaders,
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

  /// Navigates the current real owner WebView backward.
  Future<void> goBack() async {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    await _navigateTabBack(tab);
  }

  /// Navigates the current real owner WebView forward.
  Future<void> goForward() async {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    await _navigateTabForward(tab);
  }

  /// Reloads or stops the current real owner WebView.
  Future<void> refreshOrStop() async {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    if (tab.isLoading) {
      await stopCurrentLoad();
      return;
    }
    await tab.controller.reload();
  }

  /// Reloads the current browser session.
  Future<void> reloadCurrent() async {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    await tab.controller.reload();
  }

  /// Stops loading in the current browser session.
  Future<void> stopCurrentLoad() async {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    if (_supportsPageJavaScript(tab)) {
      await tab.controller.runJavaScript('window.stop();');
    }
    tab.update(isLoading: false, progress: 100);
    _syncSessionRegistry(eventType: 'stopped', sessionId: tab.id);
  }

  /// Sends native WebView back navigation and records history state.
  Future<void> _navigateTabBack(WorkspaceBrowserTabState tab) async {
    final beforeRawUrl = await tab.controller.currentUrl();
    final beforeUrl = beforeRawUrl == null
        ? tab.url
        : _logicalUrl(beforeRawUrl);
    final beforeCanGoBack = await tab.controller.canGoBack();
    final beforeCanGoForward = await tab.controller.canGoForward();
    final stopwatch = Stopwatch()..start();
    try {
      await tab.controller.goBack();
      final afterRawUrl = await tab.controller.currentUrl();
      final afterUrl = afterRawUrl == null ? tab.url : _logicalUrl(afterRawUrl);
      final afterCanGoBack = await tab.controller.canGoBack();
      final afterCanGoForward = await tab.controller.canGoForward();
      _applyNativeNavigationState(
        tab,
        url: afterUrl,
        canGoBack: afterCanGoBack,
        canGoForward: afterCanGoForward,
      );
      stopwatch.stop();
      ClientLogger.v(
        'back native tab=${tab.id} beforeCanGoBack=$beforeCanGoBack '
        'beforeCanGoForward=$beforeCanGoForward '
        'afterCanGoBack=$afterCanGoBack afterCanGoForward=$afterCanGoForward '
        'beforeUrl=$beforeUrl afterUrl=$afterUrl '
        'elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
        verboseLevel: VerboseLevel.level2,
      );
    } catch (error, stackTrace) {
      stopwatch.stop();
      ClientLogger.e(
        'back native failed tab=${tab.id} beforeCanGoBack=$beforeCanGoBack '
        'beforeUrl=$beforeUrl elapsedMs=${stopwatch.elapsedMilliseconds}',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
      rethrow;
    }
  }

  /// Sends native WebView forward navigation and synchronizes history state.
  Future<void> _navigateTabForward(WorkspaceBrowserTabState tab) async {
    await tab.controller.goForward();
    final afterRawUrl = await tab.controller.currentUrl();
    final afterUrl = afterRawUrl == null ? tab.url : _logicalUrl(afterRawUrl);
    final afterCanGoBack = await tab.controller.canGoBack();
    final afterCanGoForward = await tab.controller.canGoForward();
    _applyNativeNavigationState(
      tab,
      url: afterUrl,
      canGoBack: afterCanGoBack,
      canGoForward: afterCanGoForward,
    );
  }

  /// Applies native WebView history flags to the runtime session stream.
  void _applyNativeNavigationState(
    WorkspaceBrowserTabState tab, {
    required String url,
    required bool canGoBack,
    required bool canGoForward,
  }) {
    if (tab.isDisposed) {
      return;
    }
    tab.update(
      url: url,
      addressText: url,
      canGoBack: canGoBack,
      canGoForward: canGoForward,
    );
    _syncSessionRegistry(eventType: 'history', sessionId: tab.id);
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

  /// Sets the zoom factor for one owner browser session.
  Future<void> setZoomFactor(String sessionId, double zoomFactor) async {
    final tab = tabForSession(sessionId);
    if (tab == null) {
      throw StateError('Browser session is not registered');
    }
    await _setTabZoomFactor(tab, zoomFactor);
  }

  /// Sets the zoom factor for the active owner browser session.
  Future<void> _setZoomFactor(double zoomFactor) async {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No active browser session');
    }
    await _setTabZoomFactor(tab, zoomFactor);
  }

  /// Applies a normalized zoom factor to one owner browser tab.
  Future<void> _setTabZoomFactor(
    WorkspaceBrowserTabState tab,
    double zoomFactor,
  ) async {
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
      if (!tab.isDisposed && value != null) {
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
      setZoomFactor: (zoomFactor) => setZoomFactor(tab.id, zoomFactor),
      supportsPageJavaScript: () => _supportsPageJavaScript(tab),
    );
    return tab;
  }

  final _pageDownloads = <String, RuntimeBrowserPageDownloads>{};

  /// Configures browser behavior shared by every owner WebView session.
  Future<void> _configureTab(WorkspaceBrowserTabState tab) async {
    if (tab.controller.localResourceScheme != null) {
      await tab.controller.setLocalResourceHandler((request) {
        if (Uri.tryParse(tab.url)?.scheme != 'file') {
          throw StateError('VFS resources require a file browser session');
        }
        return _vfsResources.handle(request);
      });
      _requireLiveTab(tab);
    }
    tab.controller
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(Colors.transparent)
      ..enableZoom(true);
    await _automation[tab.id]?.applyDefaultSurfaceSize();
    _requireLiveTab(tab);
    await tab.controller.platform.setOnDownloadEvent((event) {
      if (!tab.isDisposed) {
        unawaited(stores.downloads.acceptEngineDownload(tab.id, event));
      }
    });
    _requireLiveTab(tab);
    if (tab.capabilities.pageHooks) {
      final downloads = RuntimeBrowserPageDownloads(
        emit: (event) => stores.downloads.acceptEngineDownload(tab.id, event),
        runJavaScript: tab.controller.runJavaScript,
      );
      _pageDownloads[tab.id] = downloads;
      await tab.controller.addJavaScriptChannel(
        'OperitBrowserDownloads',
        onMessageReceived: (message) {
          unawaited(downloads.handle(message.message));
        },
      );
      _requireLiveTab(tab);
      await tab.controller.addDocumentStartJavaScript(
        runtimeBrowserPageDownloadScript,
      );
      _requireLiveTab(tab);
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
    if (tab.capabilities.javaScriptDialogs &&
        tab.controller.platform.supportsAsynchronousJavaScriptDialogs) {
      final dialogs = _automation[tab.id]!.dialogs;
      tab.controller
        ..setOnJavaScriptAlertDialog((request) {
          return dialogs
              .open(
                type: 'alert',
                message: request.message,
                url: request.url,
                present: (dismissed) =>
                    _requiredUiDelegate.showAlertDialog(request, dismissed),
              )
              .then<void>((_) {});
        })
        ..setOnJavaScriptConfirmDialog((request) {
          return dialogs
              .open(
                type: 'confirm',
                message: request.message,
                url: request.url,
                present: (dismissed) =>
                    _requiredUiDelegate.showConfirmDialog(request, dismissed),
              )
              .then((value) => value as bool);
        })
        ..setOnJavaScriptTextInputDialog((request) {
          return dialogs
              .open(
                type: 'prompt',
                message: request.message,
                url: request.url,
                defaultText: request.defaultText,
                present: (dismissed) =>
                    _requiredUiDelegate.showPromptDialog(request, dismissed),
              )
              .then((value) => value as String?);
        });
    }
    await tab.controller.setNavigationDelegate(
      NavigationDelegate(
        onNavigationRequest: (request) async {
          final logicalUrl = _logicalUrl(request.url);
          if (request.isDownload &&
              (Uri.parse(logicalUrl).scheme == 'blob' ||
                  Uri.parse(logicalUrl).scheme == 'data')) {
            await tab.controller.runJavaScript(
              'globalThis.__operitPageDownloads.start(${jsonEncode(logicalUrl)});',
            );
            return NavigationDecision.prevent;
          }
          if (request.isDownload) {
            unawaited(
              stores.downloads.startDownload(
                logicalUrl,
                headers: <String, String>{
                  ...tab.requestHeaders,
                  'Referer': tab.url,
                  if (tab.preferredUserAgent != null)
                    'User-Agent': tab.preferredUserAgent!,
                  ...request.headers,
                },
                contentDisposition: request.contentDisposition,
              ),
            );
            return NavigationDecision.prevent;
          }
          if (isRuntimeBrowserExternalUrl(logicalUrl)) {
            unawaited(_requestExternalNavigation(tab, logicalUrl));
            return NavigationDecision.prevent;
          }
          return NavigationDecision.navigate;
        },
        onPageStarted: (url) {
          if (tab.isDisposed) {
            return;
          }
          unawaited(_pageDownloads[tab.id]?.close());
          final logicalUrl = _logicalUrl(url);
          ClientLogger.v(
            'page started tab=${tab.id} url=$logicalUrl',
            tag: _logTag,
            verboseLevel: VerboseLevel.level2,
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
          ClientLogger.v(
            'page finished tab=${tab.id} url=$logicalUrl',
            tag: _logTag,
            verboseLevel: VerboseLevel.level2,
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
          await _updateTabState(tab, isLoading: false, eventType: 'finished');
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
    String eventType = 'updated',
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
    _syncSessionRegistry(eventType: eventType, sessionId: tab.id);
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
      final source = _tabs
          .where((tab) => tab.id == sourceSessionId)
          .singleOrNull;
      if (source == null || source.isDisposed) return;
      final opened = await openRuntimeBrowserPopup<WorkspaceBrowserTabState>(
        sourceUrl: source.url,
        target: message['url'] as String,
        openExternal: (url) => _requestExternalNavigation(source, url),
        openTab: (url) => openTab(url: url),
      );
      if (opened == null) return;
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
    return _vfsResources.logicalUrl(url.trim());
  }

  /// Returns the browser host capabilities for one URL.
  WorkspaceBrowserSessionCapabilities _capabilitiesForUrl(String url) {
    const pageJavaScript = true;
    return WorkspaceBrowserSessionCapabilities(
      pageJavaScript: pageJavaScript,
      pageHooks: pageJavaScript,
      permissionRequests: !kIsWeb,
      javaScriptDialogs: pageJavaScript,
    );
  }

  /// Returns whether the session exposes page JavaScript to host commands.
  bool _supportsPageJavaScript(WorkspaceBrowserTabState tab) {
    if (tab.isDisposed) {
      return false;
    }
    if (!tab.capabilities.pageJavaScript) {
      return false;
    }
    return tab.controller.platform.supportsPageJavaScript;
  }

  /// Applies the stored zoom factor through the tab's WebView host.
  Future<void> _applyZoomFactor(WorkspaceBrowserTabState tab) async {
    await tab.controller.setZoomFactor(tab.zoomFactor);
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

  /// Publishes authoritative download snapshots on existing browser streams.
  void _publishDownloads() {
    final snapshot = stores.downloads.encodeSnapshot();
    for (final tab in _tabs) {
      unawaited(
        _publishBrowserSessionEvent(
          sessionId: tab.id,
          eventType: 'downloads',
          resultJson: snapshot,
        ),
      );
    }
  }

  /// Executes acknowledged download controls on the owner rather than the viewer.
  Future<String> _controlDownload(String operation, String? url) async {
    await ensureLoaded();
    if (operation != 'list') {
      final item = stores.downloads.items.singleWhere((item) => item.id == url);
      switch (operation) {
        case 'cancel':
          await stores.downloads.cancel(item);
        case 'retry':
          await stores.downloads.retry(item);
        case 'remove':
          await stores.downloads.remove(item.id);
        default:
          throw StateError('Unknown browser download operation: $operation');
      }
    }
    return stores.downloads.encodeSnapshot();
  }

  /// Resolves a page handoff and publishes failures to every viewer of its session.
  Future<void> _requestExternalNavigation(
    WorkspaceBrowserTabState tab,
    String url,
  ) async {
    try {
      await _requiredUiDelegate.openExternalNavigation(tab, url);
    } catch (error) {
      if (tab.isDisposed) return;
      tab.update(errorText: error.toString());
      _syncSessionRegistry(
        eventType: 'error',
        sessionId: tab.id,
        error: error.toString(),
      );
    }
  }

  /// Opens an explicitly selected URL through the owning device's application resolver.
  Future<void> _openExternalApplication(String sessionId, String url) {
    final tab = _tabs.singleWhere((tab) => tab.id == sessionId);
    final scheme = Uri.parse(url).scheme;
    if (!isRuntimeBrowserExternalUrl(url) &&
        scheme != 'https' &&
        scheme != 'http') {
      throw ArgumentError(
        'Only application and HTTP(S) links can be opened externally',
      );
    }
    return _requiredUiDelegate.openExternalNavigation(tab, url);
  }
}
