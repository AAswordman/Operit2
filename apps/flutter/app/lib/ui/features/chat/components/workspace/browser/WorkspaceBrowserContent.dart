// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../theme/OperitGlassSurface.dart';
import 'WorkspaceBrowserSessionStore.dart';
import 'bookmarks/WorkspaceBrowserBookmarkSheet.dart';
import 'chrome/WorkspaceBrowserExternalNavigationDialog.dart';
import 'chrome/WorkspaceBrowserJavaScriptDialogs.dart';
import 'chrome/WorkspaceBrowserMenuSheet.dart';
import 'chrome/WorkspaceBrowserSiteDataSheet.dart';
import 'chrome/WorkspaceBrowserUrlBar.dart';
import 'downloads/WorkspaceBrowserDownloadSheet.dart';
import 'history/WorkspaceBrowserHistorySheet.dart';
import 'permissions/WorkspaceBrowserPermissionDialog.dart';
import 'permissions/WorkspaceBrowserPermissionSheet.dart';
import 'tabs/WorkspaceBrowserTabModels.dart';
import 'userscripts/WorkspaceUserscriptSheet.dart';

class WorkspaceBrowserContent extends StatefulWidget {
  const WorkspaceBrowserContent({
    super.key,
    this.initialUrl,
    this.initialUserAgent,
    this.initialHeaders,
    this.initialFilePath,
    this.initialWorkspaceHtmlPath,
    this.workspacePath,
    required this.onReadWorkspaceTextFile,
    required this.onReadWorkspaceFileBytes,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onOpenBrowserTab,
    required this.onActivateRequested,
    required this.onCloseRequested,
  });

  final String? initialUrl;
  final String? initialUserAgent;
  final Map<String, String>? initialHeaders;
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
  final VoidCallback onActivateRequested;
  final VoidCallback onCloseRequested;

  @override
  State<WorkspaceBrowserContent> createState() =>
      _WorkspaceBrowserContentState();
}

class _WorkspaceBrowserContentState extends State<WorkspaceBrowserContent> {
  final WorkspaceBrowserSessionStore _sessionStore =
      WorkspaceBrowserSessionStore.instance;
  final FocusNode _browserFocusNode = FocusNode();
  final GlobalKey _menuButtonKey = GlobalKey();
  OverlayEntry? _menuPopupEntry;
  OverlayEntry? _panelPopupEntry;
  bool _initialized = false;

  WorkspaceBrowserTabState get _currentTab => _sessionStore.currentTab!;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _attachBrowserUi();
    if (_initialized) {
      return;
    }
    _initialized = true;
    unawaited(_initialize());
  }

  @override
  void didUpdateWidget(covariant WorkspaceBrowserContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    _attachBrowserUi();
    if (oldWidget.initialUrl != widget.initialUrl &&
        widget.initialUrl?.trim().isNotEmpty == true) {
      unawaited(
        _sessionStore.openTab(
          url: widget.initialUrl!,
          userAgent: widget.initialUserAgent,
          headers: widget.initialHeaders,
        ),
      );
    }
    if (oldWidget.initialFilePath != widget.initialFilePath &&
        widget.initialFilePath?.trim().isNotEmpty == true) {
      unawaited(_sessionStore.openLocalFileTab(widget.initialFilePath!));
    }
    if (oldWidget.initialWorkspaceHtmlPath != widget.initialWorkspaceHtmlPath &&
        widget.initialWorkspaceHtmlPath?.trim().isNotEmpty == true) {
      unawaited(
        _sessionStore.openWorkspaceHtmlTab(
          widget.initialWorkspaceHtmlPath!,
          initialUrl: widget.initialUrl,
        ),
      );
    }
  }

  @override
  void dispose() {
    _dismissMenuPopup();
    _dismissPanelPopup();
    _browserFocusNode.dispose();
    _sessionStore.detachUi(this);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _sessionStore,
      builder: (context, child) {
        final tab = _sessionStore.currentTab;
        if (tab == null) {
          return const Center(child: CircularProgressIndicator());
        }
        return AnimatedBuilder(
          animation: Listenable.merge(<Listenable>[
            tab,
            _sessionStore.stores.downloads,
          ]),
          builder: (context, child) {
            final isBookmarked = _sessionStore.stores.bookmarks.contains(
              tab.url,
            );
            return Focus(
              focusNode: _browserFocusNode,
              autofocus: true,
              child: CallbackShortcuts(
                bindings: <ShortcutActivator, VoidCallback>{
                  const SingleActivator(LogicalKeyboardKey.minus, control: true):
                      _sessionStore.zoomOut,
                  const SingleActivator(
                    LogicalKeyboardKey.numpadSubtract,
                    control: true,
                  ): _sessionStore.zoomOut,
                  const SingleActivator(LogicalKeyboardKey.equal, control: true):
                      _sessionStore.zoomIn,
                  const SingleActivator(
                    LogicalKeyboardKey.equal,
                    control: true,
                    shift: true,
                  ): _sessionStore.zoomIn,
                  const SingleActivator(
                    LogicalKeyboardKey.numpadAdd,
                    control: true,
                  ): _sessionStore.zoomIn,
                  const SingleActivator(
                    LogicalKeyboardKey.digit0,
                    control: true,
                  ): _sessionStore.resetZoom,
                  const SingleActivator(
                    LogicalKeyboardKey.numpad0,
                    control: true,
                  ): _sessionStore.resetZoom,
                },
                child: GestureDetector(
                  behavior: HitTestBehavior.translucent,
                  onTapDown: (_) => _browserFocusNode.requestFocus(),
                  child: Column(
                    children: <Widget>[
                      WorkspaceBrowserUrlBar(
                        tab: tab,
                        isBookmarked: isBookmarked,
                        onSubmitted: _sessionStore.navigateCurrent,
                        onToggleBookmark: _sessionStore.toggleBookmark,
                        onBack: _sessionStore.goBack,
                        onForward: _sessionStore.goForward,
                        onRefreshOrStop: _sessionStore.refreshOrStop,
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
                  ),
                ),
              ),
            );
          },
        );
      },
    );
  }

  void _attachBrowserUi() {
    final l10n = AppLocalizations.of(context)!;
    _sessionStore.attachUi(
      owner: this,
      newTabTitle: l10n.newTab,
      delegate: WorkspaceBrowserUiDelegate(
        showAlertDialog: (request) {
          return showWorkspaceBrowserAlertDialog(context, request);
        },
        showConfirmDialog: (request) {
          return showWorkspaceBrowserConfirmDialog(context, request);
        },
        showPromptDialog: (request) {
          return showWorkspaceBrowserPromptDialog(context, request);
        },
        openExternalNavigation: _openExternalNavigation,
        handlePermissionRequest: _handlePermissionRequest,
        onActivateRequested: widget.onActivateRequested,
        onCloseRequested: widget.onCloseRequested,
      ),
      onReadWorkspaceFileBytes: widget.onReadWorkspaceFileBytes,
      onWriteWorkspaceFileBytes: widget.onWriteWorkspaceFileBytes,
    );
  }

  Future<void> _initialize() {
    return _sessionStore.openInitialTab(
      initialUrl: widget.initialUrl,
      initialUserAgent: widget.initialUserAgent,
      initialHeaders: widget.initialHeaders,
      initialFilePath: widget.initialFilePath,
      initialWorkspaceHtmlPath: widget.initialWorkspaceHtmlPath,
    );
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
                child: OperitGlassSurface(
                  color: Theme.of(
                    context,
                  ).colorScheme.surfaceContainer.withValues(alpha: 0.62),
                  layer: OperitGlassSurfaceLayer.card,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                    color: Theme.of(
                      context,
                    ).colorScheme.outlineVariant.withValues(alpha: 0.2),
                  ),
                  shadows: const <BoxShadow>[
                    BoxShadow(
                      color: Color(0x22000000),
                      blurRadius: 18,
                      offset: Offset(0, 8),
                    ),
                  ],
                  material: true,
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
                      zoomLabel: '${_currentTab.zoomPercent}%',
                      onZoomOut: _sessionStore.zoomOut,
                      onZoomReset: _sessionStore.resetZoom,
                      onZoomIn: _sessionStore.zoomIn,
                      desktopMode: _currentTab.desktopMode,
                      onDesktopModeChanged: (enabled) {
                        _dismissMenuPopup();
                        unawaited(_sessionStore.setDesktopMode(enabled));
                      },
                      onLoadMenuCommands: () {
                        return _sessionStore.stores.userscriptRuntime
                            .menuCommands(_currentTab.controller);
                      },
                      onRunMenuCommand: (index) {
                        _dismissMenuPopup();
                        unawaited(
                          _sessionStore.stores.userscriptRuntime
                              .runMenuCommand(_currentTab.controller, index),
                        );
                      },
                      activeDownloadCount: _sessionStore.activeDownloadCount,
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
                child: OperitGlassSurface(
                  color: Theme.of(
                    context,
                  ).colorScheme.surfaceContainer.withValues(alpha: 0.62),
                  layer: OperitGlassSurfaceLayer.card,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                    color: Theme.of(
                      context,
                    ).colorScheme.outlineVariant.withValues(alpha: 0.2),
                  ),
                  shadows: const <BoxShadow>[
                    BoxShadow(
                      color: Color(0x22000000),
                      blurRadius: 18,
                      offset: Offset(0, 8),
                    ),
                  ],
                  material: true,
                  child: ConstrainedBox(
                    constraints: BoxConstraints(maxHeight: maxHeight),
                    child: child,
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
    _showPanelPopup(
      WorkspaceBrowserPermissionSheet(store: _sessionStore.permissions),
    );
  }

  void _openHistorySheet() {
    _showPanelPopup(
      WorkspaceBrowserHistorySheet(
        store: _sessionStore.stores.history,
        onChanged: () {
          if (mounted) {
            setState(() {});
          }
        },
        onOpen: (url) {
          _dismissPanelPopup();
          _sessionStore.navigateCurrent(url);
        },
      ),
    );
  }

  void _openBookmarkSheet() {
    _showPanelPopup(
      WorkspaceBrowserBookmarkSheet(
        store: _sessionStore.stores.bookmarks,
        onChanged: () {
          if (mounted) {
            setState(() {});
          }
        },
        onOpen: (url) {
          _dismissPanelPopup();
          _sessionStore.navigateCurrent(url);
        },
      ),
    );
  }

  void _openDownloadSheet() {
    _showPanelPopup(
      WorkspaceBrowserDownloadSheet(
        store: _sessionStore.stores.downloads,
        onOpenWorkspaceFile: widget.onOpenWorkspaceFile,
      ),
    );
  }

  void _openUserscriptSheet() {
    _showPanelPopup(
      WorkspaceUserscriptSheet(
        store: _sessionStore.stores.userscripts,
        onChanged: () {
          if (mounted) {
            setState(() {});
          }
        },
        onReadWorkspaceTextFile: widget.onReadWorkspaceTextFile,
        onLoadMenuCommands: () {
          return _sessionStore.stores.userscriptRuntime.menuCommands(
            _currentTab.controller,
          );
        },
        onRunMenuCommand: (index) {
          return _sessionStore.stores.userscriptRuntime.runMenuCommand(
            _currentTab.controller,
            index,
          );
        },
      ),
      preferredWidth: 420,
    );
  }

  Future<void> _openExternalNavigation(String url) async {
    if (!mounted) {
      throw StateError('Browser UI is not mounted');
    }
    final confirmed = await showWorkspaceBrowserExternalNavigationDialog(
      context,
      url,
    );
    if (!confirmed) {
      return;
    }
    await launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication);
  }

  Future<void> _handlePermissionRequest(
    WorkspaceBrowserTabState tab,
    WebViewPermissionRequest request,
  ) async {
    if (!mounted) {
      throw StateError('Browser UI is not mounted');
    }
    final uri = Uri.tryParse(tab.url);
    final origin = uri == null || uri.host.isEmpty
        ? tab.url
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
      _sessionStore.permissions.record(
        origin: origin,
        types: request.types.toList(growable: false),
        allowed: true,
      );
      await request.grant();
      return;
    }
    _sessionStore.permissions.record(
      origin: origin,
      types: request.types.toList(growable: false),
      allowed: false,
    );
    await request.deny();
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
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: OperitGlassSurface(
          color: theme.colorScheme.surfaceContainerHighest.withValues(
            alpha: 0.42,
          ),
          layer: OperitGlassSurfaceLayer.card,
          borderRadius: BorderRadius.circular(18),
          border: Border.all(
            color: theme.colorScheme.outlineVariant.withValues(alpha: 0.2),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 20),
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
      ),
    );
  }
}
