// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../../../../l10n/generated/app_localizations.dart';
import 'WorkspaceBrowserStores.dart';
import 'automation/WorkspaceBrowserAutomationController.dart';
import 'automation/WorkspaceBrowserSessionRegistry.dart';
import 'bookmarks/WorkspaceBrowserBookmarkSheet.dart';
import 'chrome/WorkspaceBrowserExternalNavigationDialog.dart';
import 'chrome/WorkspaceBrowserJavaScriptDialogs.dart';
import 'chrome/WorkspaceBrowserMenuSheet.dart';
import 'chrome/WorkspaceBrowserSiteDataSheet.dart';
import 'chrome/WorkspaceBrowserUrlBar.dart';
import 'downloads/WorkspaceBrowserDownloadSheet.dart';
import 'downloads/WorkspaceBrowserDownloadStore.dart';
import 'history/WorkspaceBrowserHistorySheet.dart';
import '../html_preview/WorkspaceHtmlPreviewServer.dart';
import 'permissions/WorkspaceBrowserPermissionDialog.dart';
import 'permissions/WorkspaceBrowserPermissionSheet.dart';
import 'permissions/WorkspaceBrowserPermissionStore.dart';
import 'tabs/WorkspaceBrowserTabModels.dart';
import 'userscripts/WorkspaceUserscriptModels.dart';
import 'userscripts/WorkspaceUserscriptSheet.dart';

class WorkspaceBrowserContent extends StatefulWidget {
  const WorkspaceBrowserContent({
    super.key,
    required this.chatId,
    this.initialUrl,
    this.initialFilePath,
    this.initialWorkspaceHtmlPath,
    this.workspacePath,
    required this.onReadWorkspaceTextFile,
    required this.onReadWorkspaceFileBytes,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onOpenBrowserTab,
  });

  final String chatId;
  final String? initialUrl;
  final String? initialFilePath;
  final String? initialWorkspaceHtmlPath;
  final String? workspacePath;
  final Future<String> Function(String path) onReadWorkspaceTextFile;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path, Uint8List bytes)
  onWriteWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;
  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  onOpenBrowserTab;

  @override
  State<WorkspaceBrowserContent> createState() =>
      _WorkspaceBrowserContentState();
}

class _WorkspaceBrowserContentState extends State<WorkspaceBrowserContent> {
  static const String _homeUrl = 'https://www.bing.com';
  static const String _desktopUserAgent =
      'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 '
      '(KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36';

  final WorkspaceBrowserStores _stores = WorkspaceBrowserStores();
  final List<WorkspaceBrowserTabState> _tabs = <WorkspaceBrowserTabState>[];
  final Map<String, WorkspaceBrowserAutomationController> _automation =
      <String, WorkspaceBrowserAutomationController>{};
  final Map<String, String> _defaultUserAgents = <String, String>{};
  final WorkspaceBrowserSessionRegistry _sessionRegistry =
      WorkspaceBrowserSessionRegistry.instance;
  final WorkspaceBrowserPermissionStore _permissionStore =
      WorkspaceBrowserPermissionStore();
  late final WorkspaceHtmlPreviewServer _htmlPreviewServer;
  final GlobalKey _menuButtonKey = GlobalKey();
  OverlayEntry? _menuPopupEntry;
  OverlayEntry? _panelPopupEntry;
  int _selectedIndex = 0;

  WorkspaceBrowserTabState get _currentTab => _tabs[_selectedIndex];

  @override
  void initState() {
    super.initState();
    _htmlPreviewServer = WorkspaceHtmlPreviewServer(
      onReadWorkspaceFileBytes: widget.onReadWorkspaceFileBytes,
    );
    _stores.downloads.setWorkspaceSaver(widget.onWriteWorkspaceFileBytes);
    _initialize();
  }

  @override
  void didUpdateWidget(covariant WorkspaceBrowserContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.chatId != widget.chatId) {
      _syncSessionRegistry();
    }
    if (oldWidget.initialUrl != widget.initialUrl &&
        widget.initialUrl?.trim().isNotEmpty == true) {
      _addTab(widget.initialUrl!);
    }
    if (oldWidget.initialFilePath != widget.initialFilePath &&
        widget.initialFilePath?.trim().isNotEmpty == true) {
      _addTabForLocalFile(widget.initialFilePath!);
    }
    if (oldWidget.initialWorkspaceHtmlPath != widget.initialWorkspaceHtmlPath &&
        widget.initialWorkspaceHtmlPath?.trim().isNotEmpty == true) {
      unawaited(_addTabForWorkspaceHtml(widget.initialWorkspaceHtmlPath!));
    }
  }

  @override
  void dispose() {
    _dismissMenuPopup();
    _dismissPanelPopup();
    for (final tab in _tabs) {
      _sessionRegistry.unregister(tab.id);
      tab.dispose();
    }
    _htmlPreviewServer.stop();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_tabs.isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    final tab = _currentTab;
    final isBookmarked = _stores.bookmarks.contains(tab.url);
    return AnimatedBuilder(
      animation: Listenable.merge(<Listenable>[tab, _stores.downloads]),
      builder: (context, child) {
        return Column(
          children: <Widget>[
            WorkspaceBrowserUrlBar(
              tab: tab,
              isBookmarked: isBookmarked,
              onSubmitted: _navigateCurrent,
              onToggleBookmark: _toggleBookmark,
              onBack: _goBack,
              onForward: _goForward,
              onRefreshOrStop: _refreshOrStop,
              onOpenMenu: _toggleMenuPopup,
              menuButtonKey: _menuButtonKey,
            ),
            Expanded(
              child: Stack(
                children: <Widget>[
                  WebViewWidget(
                    key: ValueKey<String>(tab.id),
                    controller: tab.controller,
                  ),
                  if (tab.errorText != null)
                    _BrowserErrorOverlay(
                      message: tab.errorText!,
                      onRetry: () => tab.controller.reload(),
                    ),
                ],
              ),
            ),
          ],
        );
      },
    );
  }

  Future<void> _initialize() async {
    await _stores.load();
    if (!mounted) {
      return;
    }
    await _openInitialTab();
  }

  Future<void> _openInitialTab() async {
    final explicitFilePath = widget.initialFilePath;
    if (explicitFilePath != null && explicitFilePath.trim().isNotEmpty) {
      _addTabForLocalFile(explicitFilePath);
      return;
    }
    final explicitWorkspaceHtmlPath = widget.initialWorkspaceHtmlPath;
    if (explicitWorkspaceHtmlPath != null &&
        explicitWorkspaceHtmlPath.trim().isNotEmpty) {
      await _addTabForWorkspaceHtml(
        explicitWorkspaceHtmlPath,
        initialUrl: widget.initialUrl,
      );
      return;
    }
    final explicitUrl = widget.initialUrl;
    if (explicitUrl != null && explicitUrl.trim().isNotEmpty) {
      _addTab(explicitUrl);
      return;
    }
    _addTab(_homeUrl);
  }

  void _addTab(String rawUrl) {
    final url = normalizeWorkspaceBrowserUrl(rawUrl);
    final tab = _createTab(url);
    _configureTab(tab);
    setState(() {
      _tabs.add(tab);
      _selectedIndex = _tabs.length - 1;
    });
    _syncSessionRegistry();
    tab.controller.loadRequest(Uri.parse(url));
  }

  void _addTabForLocalFile(String absolutePath) {
    final tab = _createTab('file://$absolutePath', localFilePath: absolutePath);
    _configureTab(tab);
    setState(() {
      _tabs.add(tab);
      _selectedIndex = _tabs.length - 1;
    });
    _syncSessionRegistry();
    tab.controller.loadFile(absolutePath);
  }

  Future<void> _addTabForWorkspaceHtml(
    String relativePath, {
    String? initialUrl,
  }) async {
    final uri = await _htmlPreviewServer.start(relativePath);
    final url = initialUrl?.trim().isNotEmpty == true
        ? initialUrl!.trim()
        : uri.toString();
    _addTab(url);
  }

  WorkspaceBrowserTabState _createTab(String url, {String? localFilePath}) {
    final l10n = AppLocalizations.of(context)!;
    final controller = WebViewController(
      onPermissionRequest: _handlePermissionRequest,
    );
    final tab = WorkspaceBrowserTabState(
      id: DateTime.now().microsecondsSinceEpoch.toString(),
      initialUrl: url,
      controller: controller,
      title: l10n.newTab,
      localFilePath: localFilePath,
    );
    final automationController = WorkspaceBrowserAutomationController(
      controller: tab.controller,
    );
    tab.controller.getUserAgent().then((value) {
      if (value != null) {
        _defaultUserAgents[tab.id] = value;
      }
    });
    _automation[tab.id] = automationController;
    _sessionRegistry.register(
      chatId: widget.chatId,
      sessionId: tab.id,
      controller: automationController,
      title: tab.title,
      url: tab.url,
      active: true,
      selectTab: _selectBrowserSession,
      closeTab: _closeBrowserSession,
      navigate: _navigateCurrent,
      navigateBack: _goBack,
    );
    return tab;
  }

  void _configureTab(WorkspaceBrowserTabState tab) {
    tab.controller
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(Colors.transparent)
      ..setOnConsoleMessage((message) {
        _automation[tab.id]?.addConsoleMessage(message);
        _stores.userscripts.addLog('console', message.message);
      })
      ..addJavaScriptChannel(
        'OperitUserscriptStorage',
        onMessageReceived: (message) {
          _stores.userscripts.handleStorageMessage(message.message);
        },
      )
      ..addJavaScriptChannel(
        'OperitUserscriptRuntime',
        onMessageReceived: (message) {
          _stores.userscripts.handleRuntimeMessage(message.message);
          setState(() {});
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
          return showWorkspaceBrowserAlertDialog(context, request);
        })
        ..setOnJavaScriptConfirmDialog((request) {
          return showWorkspaceBrowserConfirmDialog(context, request);
        })
        ..setOnJavaScriptTextInputDialog((request) {
          return showWorkspaceBrowserPromptDialog(context, request);
        });
    }
    tab.controller.setNavigationDelegate(
      NavigationDelegate(
        onNavigationRequest: (request) async {
          if (_isDownloadUrl(request.url)) {
            unawaited(_stores.downloads.startDownload(request.url));
            return NavigationDecision.prevent;
          }
          if (_isExternalAppUrl(request.url)) {
            final confirmed =
                await showWorkspaceBrowserExternalNavigationDialog(
                  context,
                  request.url,
                );
            if (confirmed) {
              await launchUrl(
                Uri.parse(request.url),
                mode: LaunchMode.externalApplication,
              );
            }
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
          if (!_isWorkspaceHtmlPreviewUrl(tab.url)) {
            _stores.history.add(url: tab.url, title: tab.title);
          }
        },
        onUrlChange: (change) {
          if (tab.isDisposed) {
            return;
          }
          final url = change.url;
          if (url != null) {
            tab.update(url: url, addressText: url);
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
          final l10n = AppLocalizations.of(context)!;
          tab.update(errorText: l10n.sslCertificateError, isLoading: false);
        },
      ),
    );
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
    await _stores.userscriptRuntime.injectForUrl(
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
      widget.onOpenBrowserTab(url: url);
      return;
    }
    if (action == 'close' && _tabs.length > 1) {
      _closeTab(_selectedIndex);
    }
  }

  void _navigateCurrent(String rawUrl) {
    final url = normalizeWorkspaceBrowserUrl(rawUrl);
    _currentTab.update(url: url, addressText: url, errorText: null);
    _currentTab.controller.loadRequest(Uri.parse(url));
  }

  void _goBack() {
    _currentTab.controller.goBack();
  }

  void _goForward() {
    _currentTab.controller.goForward();
  }

  void _refreshOrStop() {
    final tab = _currentTab;
    if (tab.isLoading) {
      tab.controller.runJavaScript('window.stop();');
      tab.update(isLoading: false);
      return;
    }
    tab.controller.reload();
  }

  void _toggleBookmark() {
    _stores.bookmarks.toggle(url: _currentTab.url, title: _currentTab.title);
    setState(() {});
  }

  void _closeTab(int index) {
    if (_tabs.length <= 1) {
      return;
    }
    final removed = _tabs.removeAt(index);
    _automation.remove(removed.id);
    _sessionRegistry.unregister(removed.id);
    removed.dispose();
    setState(() {
      if (_selectedIndex >= _tabs.length) {
        _selectedIndex = _tabs.length - 1;
      } else if (_selectedIndex > index) {
        _selectedIndex -= 1;
      }
    });
    _syncSessionRegistry();
  }

  void _selectBrowserSession(String sessionId) {
    final index = _tabs.indexWhere((tab) => tab.id == sessionId);
    if (index < 0) {
      return;
    }
    setState(() => _selectedIndex = index);
    _syncSessionRegistry();
  }

  void _closeBrowserSession(String sessionId) {
    final index = _tabs.indexWhere((tab) => tab.id == sessionId);
    if (index < 0) {
      return;
    }
    _closeTab(index);
  }

  void _toggleMenuPopup() {
    if (_menuPopupEntry != null) {
      _dismissMenuPopup();
      return;
    }
    final renderBox =
        _menuButtonKey.currentContext?.findRenderObject() as RenderBox?;
    if (renderBox == null || !renderBox.attached) {
      return;
    }
    final overlay = Overlay.of(context);
    final mediaQuery = MediaQuery.of(context);
    final screenSize = mediaQuery.size;
    final targetOffset = renderBox.localToGlobal(Offset.zero);
    final targetRect = Rect.fromLTWH(
      targetOffset.dx,
      targetOffset.dy,
      renderBox.size.width,
      renderBox.size.height,
    );
    final horizontalPadding = 12.0 + mediaQuery.padding.left;
    final rightPadding = 12.0 + mediaQuery.padding.right;
    final availableWidth = screenSize.width - horizontalPadding - rightPadding;
    final popupWidth = availableWidth < 220.0 ? availableWidth : 220.0;
    final maxLeft = screenSize.width - rightPadding - popupWidth;
    final left = (targetRect.right - popupWidth)
        .clamp(horizontalPadding, maxLeft)
        .toDouble();
    final top = targetRect.bottom + 8;
    final maxHeight = (screenSize.height - top - mediaQuery.padding.bottom - 12)
        .clamp(96.0, 360.0)
        .toDouble();

    _menuPopupEntry = OverlayEntry(
      builder: (context) {
        return Stack(
          children: <Widget>[
            Positioned.fill(
              child: GestureDetector(
                behavior: HitTestBehavior.translucent,
                onTap: _dismissMenuPopup,
                child: const SizedBox.expand(),
              ),
            ),
            Positioned(
              left: left,
              top: top,
              width: popupWidth,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () {},
                child: Material(
                  color: Colors.transparent,
                  child: Card(
                    margin: EdgeInsets.zero,
                    elevation: 4,
                    color: Theme.of(context).colorScheme.surfaceContainer,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: ConstrainedBox(
                      constraints: BoxConstraints(maxHeight: maxHeight),
                      child: WorkspaceBrowserMenuSheet(
                        onHistory: () {
                          _dismissMenuPopup();
                          _openHistorySheet();
                        },
                        onBookmarks: () {
                          _dismissMenuPopup();
                          _openBookmarkSheet();
                        },
                        onDownloads: () {
                          _dismissMenuPopup();
                          _openDownloadSheet();
                        },
                        onUserscripts: () {
                          _dismissMenuPopup();
                          _openUserscriptSheet();
                        },
                        onPermissions: () {
                          _dismissMenuPopup();
                          _openPermissionSheet();
                        },
                        onClearStorage: () {
                          _dismissMenuPopup();
                          _openSiteDataSheet();
                        },
                        desktopMode: _currentTab.desktopMode,
                        onDesktopModeChanged: (enabled) {
                          _dismissMenuPopup();
                          _setDesktopMode(enabled);
                        },
                        onLoadMenuCommands: () {
                          return _stores.userscriptRuntime.menuCommands(
                            _currentTab.controller,
                          );
                        },
                        onRunMenuCommand: (index) {
                          _dismissMenuPopup();
                          unawaited(
                            _stores.userscriptRuntime.runMenuCommand(
                              _currentTab.controller,
                              index,
                            ),
                          );
                        },
                        activeDownloadCount: _activeDownloadCount,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
    overlay.insert(_menuPopupEntry!);
  }

  void _dismissMenuPopup() {
    _menuPopupEntry?.remove();
    _menuPopupEntry = null;
  }

  void _showPanelPopup(Widget child, {double preferredWidth = 320}) {
    _dismissPanelPopup();
    final overlay = Overlay.of(context);
    final mediaQuery = MediaQuery.of(context);
    final screenSize = mediaQuery.size;
    final horizontalPadding = 12.0 + mediaQuery.padding.left;
    final rightPadding = 12.0 + mediaQuery.padding.right;
    final renderBox =
        _menuButtonKey.currentContext?.findRenderObject() as RenderBox?;
    final targetBottom = renderBox == null || !renderBox.attached
        ? 0.0
        : renderBox.localToGlobal(Offset.zero).dy + renderBox.size.height;
    final top = math.max(12.0 + mediaQuery.padding.top, targetBottom + 24);
    final availableWidth = screenSize.width - horizontalPadding - rightPadding;
    final popupWidth = availableWidth < preferredWidth
        ? availableWidth
        : preferredWidth;
    final left = screenSize.width - rightPadding - popupWidth;
    final maxHeight = (screenSize.height - top - mediaQuery.padding.bottom - 16)
        .clamp(160.0, 360.0)
        .toDouble();
    _panelPopupEntry = OverlayEntry(
      builder: (context) {
        return Stack(
          children: <Widget>[
            Positioned.fill(
              child: GestureDetector(
                behavior: HitTestBehavior.translucent,
                onTap: _dismissPanelPopup,
                child: const SizedBox.expand(),
              ),
            ),
            Positioned(
              left: left,
              top: top,
              width: popupWidth,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () {},
                child: Material(
                  color: Colors.transparent,
                  child: Card(
                    margin: EdgeInsets.zero,
                    elevation: 4,
                    color: Theme.of(context).colorScheme.surfaceContainer,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: ConstrainedBox(
                      constraints: BoxConstraints(maxHeight: maxHeight),
                      child: child,
                    ),
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
    overlay.insert(_panelPopupEntry!);
  }

  void _dismissPanelPopup() {
    _panelPopupEntry?.remove();
    _panelPopupEntry = null;
  }

  void _openSiteDataSheet() {
    _showPanelPopup(
      WorkspaceBrowserSiteDataSheet(controller: _currentTab.controller),
    );
  }

  void _openPermissionSheet() {
    _showPanelPopup(WorkspaceBrowserPermissionSheet(store: _permissionStore));
  }

  Future<void> _setDesktopMode(bool enabled) async {
    _currentTab.update(desktopMode: enabled);
    if (enabled) {
      await _currentTab.controller.setUserAgent(_desktopUserAgent);
    } else if (_defaultUserAgents.containsKey(_currentTab.id)) {
      await _currentTab.controller.setUserAgent(
        _defaultUserAgents[_currentTab.id],
      );
    }
    await _currentTab.controller.reload();
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

  void _openHistorySheet() {
    _showPanelPopup(
      WorkspaceBrowserHistorySheet(
        store: _stores.history,
        onChanged: () => setState(() {}),
        onOpen: (url) {
          _dismissPanelPopup();
          _navigateCurrent(url);
        },
      ),
    );
  }

  void _openBookmarkSheet() {
    _showPanelPopup(
      WorkspaceBrowserBookmarkSheet(
        store: _stores.bookmarks,
        onChanged: () => setState(() {}),
        onOpen: (url) {
          _dismissPanelPopup();
          _navigateCurrent(url);
        },
      ),
    );
  }

  void _openDownloadSheet() {
    _showPanelPopup(
      WorkspaceBrowserDownloadSheet(
        store: _stores.downloads,
        onOpenWorkspaceFile: widget.onOpenWorkspaceFile,
      ),
    );
  }

  void _openUserscriptSheet() {
    _showPanelPopup(
      WorkspaceUserscriptSheet(
        store: _stores.userscripts,
        onChanged: () => setState(() {}),
        onReadWorkspaceTextFile: widget.onReadWorkspaceTextFile,
        onLoadMenuCommands: () {
          return _stores.userscriptRuntime.menuCommands(_currentTab.controller);
        },
        onRunMenuCommand: (index) {
          return _stores.userscriptRuntime.runMenuCommand(
            _currentTab.controller,
            index,
          );
        },
      ),
      preferredWidth: 420,
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

  Future<void> _handlePermissionRequest(
    WebViewPermissionRequest request,
  ) async {
    if (!mounted) {
      await request.deny();
      return;
    }
    final uri = Uri.tryParse(_currentTab.url);
    final origin = uri == null || uri.host.isEmpty
        ? _currentTab.url
        : '${uri.scheme}://${uri.host}';
    final allowed = await showDialog<bool>(
      context: context,
      builder: (context) {
        return WorkspaceBrowserPermissionDialog(
          origin: origin,
          types: request.types,
        );
      },
    );
    if (allowed == true) {
      _permissionStore.record(
        origin: origin,
        types: request.types.toList(growable: false),
        allowed: true,
      );
      await request.grant();
      return;
    }
    _permissionStore.record(
      origin: origin,
      types: request.types.toList(growable: false),
      allowed: false,
    );
    await request.deny();
  }

  void _syncSessionRegistry() {
    for (var index = 0; index < _tabs.length; index += 1) {
      final tab = _tabs[index];
      _sessionRegistry.update(
        chatId: widget.chatId,
        sessionId: tab.id,
        title: tab.title,
        url: tab.url,
        active: index == _selectedIndex,
      );
    }
  }

  int get _activeDownloadCount {
    return _stores.downloads.items
        .where(
          (item) =>
              item.state == WorkspaceBrowserDownloadState.pending ||
              item.state == WorkspaceBrowserDownloadState.running,
        )
        .length;
  }
}

class _BrowserErrorOverlay extends StatelessWidget {
  const _BrowserErrorOverlay({required this.message, required this.onRetry});

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    return ColoredBox(
      color: theme.colorScheme.surface,
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(
                Icons.error_outline,
                size: 42,
                color: theme.colorScheme.error,
              ),
              const SizedBox(height: 12),
              Text(
                l10n.pageLoadFailed,
                style: theme.textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                message,
                textAlign: TextAlign.center,
                style: theme.textTheme.bodySmall,
              ),
              const SizedBox(height: 16),
              FilledButton.icon(
                onPressed: onRetry,
                icon: const Icon(Icons.refresh),
                label: Text(l10n.retry),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

String normalizeWorkspaceBrowserUrl(String raw) {
  final value = raw.trim();
  final uri = Uri.tryParse(value);
  if (uri != null && uri.hasScheme) {
    return value;
  }
  if (value.contains('.') && !value.contains(' ')) {
    return 'https://$value';
  }
  return Uri.https('www.bing.com', '/search', <String, String>{
    'q': value,
  }).toString();
}

bool _isWorkspaceHtmlPreviewUrl(String url) {
  final uri = Uri.tryParse(url);
  if (uri == null) {
    return false;
  }
  final host = uri.host.toLowerCase();
  final isLoopback = host == '127.0.0.1' || host == 'localhost';
  if (!isLoopback) {
    return false;
  }
  final path = uri.path.toLowerCase();
  return path.endsWith('.html') || path.endsWith('.htm');
}
