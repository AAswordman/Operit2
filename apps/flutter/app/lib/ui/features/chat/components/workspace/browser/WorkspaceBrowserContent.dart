// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../../common/layout/ApplicationZoom.dart';
import '../../../../../common/layout/OverlayGeometry.dart';
import '../../../../../theme/OperitGlassSurface.dart';
import 'WorkspaceBrowserViewStore.dart';
import 'bookmarks/WorkspaceBrowserBookmarkSheet.dart';
import 'chrome/WorkspaceBrowserMenuSheet.dart';
import 'chrome/WorkspaceBrowserSiteDataSheet.dart';
import 'chrome/WorkspaceBrowserUrlBar.dart';
import 'downloads/WorkspaceBrowserDownloadSheet.dart';
import 'history/WorkspaceBrowserHistorySheet.dart';
import 'permissions/WorkspaceBrowserPermissionSheet.dart';
import 'surface/WorkspaceBrowserSurface.dart';
import 'tabs/WorkspaceBrowserTabModels.dart';
import 'userscripts/WorkspaceUserscriptSheet.dart';

class WorkspaceBrowserContent extends StatefulWidget {
  const WorkspaceBrowserContent({
    super.key,
    this.sessionId,
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

  final String? sessionId;
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
  final WorkspaceBrowserViewStore _sessionStore =
      WorkspaceBrowserViewStore.instance;
  final FocusNode _browserFocusNode = FocusNode();
  final GlobalKey _menuButtonKey = GlobalKey();
  final GlobalKey _zoomButtonKey = GlobalKey();
  OverlayEntry? _menuPopupEntry;
  OverlayEntry? _zoomPopupEntry;
  OverlayEntry? _panelPopupEntry;
  bool _initialized = false;

  WorkspaceBrowserTabState? get _activeTab {
    final explicitId = widget.sessionId;
    if (explicitId != null && explicitId.isNotEmpty) {
      final tab = _sessionStore.tabById(explicitId);
      if (tab != null) {
        return tab;
      }
    }
    return _sessionStore.currentTab;
  }

  void _syncActiveSession() {
    final explicitId = widget.sessionId;
    if (explicitId != null && explicitId.isNotEmpty) {
      if (_sessionStore.currentTab?.id != explicitId) {
        try {
          _sessionStore.selectSession(explicitId);
        } catch (_) {}
      }
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _attachBrowserUi();
    _syncActiveSession();
    if (_initialized) {
      return;
    }
    _initialized = true;
    if (widget.sessionId == null || widget.sessionId!.isEmpty) {
      unawaited(_initialize());
    }
  }

  @override
  void didUpdateWidget(covariant WorkspaceBrowserContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    _attachBrowserUi();
    if (oldWidget.sessionId != widget.sessionId) {
      _syncActiveSession();
    }
  }

  @override
  void dispose() {
    _dismissMenuPopup();
    _dismissZoomPopup();
    _dismissPanelPopup();
    _browserFocusNode.dispose();
    _sessionStore.detachUi();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _sessionStore,
      builder: (context, child) {
        final tab = _activeTab;
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
            return ApplicationZoomExclusion(
              child: Focus(
                focusNode: _browserFocusNode,
                autofocus: true,
                child: CallbackShortcuts(
                  bindings: <ShortcutActivator, VoidCallback>{
                    const SingleActivator(
                      LogicalKeyboardKey.minus,
                      control: true,
                    ): _sessionStore.zoomOut,
                    const SingleActivator(
                      LogicalKeyboardKey.numpadSubtract,
                      control: true,
                    ): _sessionStore.zoomOut,
                    const SingleActivator(
                      LogicalKeyboardKey.equal,
                      control: true,
                    ): _sessionStore.zoomIn,
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
                  child: Column(
                    children: <Widget>[
                      WorkspaceBrowserUrlBar(
                        tab: tab,
                        isBookmarked: isBookmarked,
                        onSubmitted: (url) {
                          _syncActiveSession();
                          _sessionStore.navigateCurrent(url);
                        },
                        onToggleBookmark: () {
                          _syncActiveSession();
                          _sessionStore.toggleBookmark();
                        },
                        onBack: () {
                          _syncActiveSession();
                          _sessionStore.goBack();
                        },
                        onForward: () {
                          _syncActiveSession();
                          _sessionStore.goForward();
                        },
                        onRefreshOrStop: () {
                          _syncActiveSession();
                          _sessionStore.refreshOrStop();
                        },
                        onOpenMenu: _toggleMenuPopup,
                        onNewTab: () => widget.onOpenBrowserTab(),
                        menuButtonKey: _menuButtonKey,
                      ),
                      Expanded(child: _buildBrowserSurface(tab)),
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

  /// Builds the browser viewport for the active session.
  Widget _buildBrowserSurface(WorkspaceBrowserTabState tab) {
    return WorkspaceBrowserSurface(
      key: ValueKey<String>('workspace-browser-surface-${tab.id}'),
      tab: tab,
      store: _sessionStore,
    );
  }

  void _attachBrowserUi() {
    _sessionStore.attachUi(
      delegate: WorkspaceBrowserViewDelegate(
        onActivateRequested: widget.onActivateRequested,
        onCloseRequested: widget.onCloseRequested,
      ),
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
    _dismissZoomPopup();
    final renderBox =
        _menuButtonKey.currentContext?.findRenderObject() as RenderBox?;
    if (renderBox == null || !renderBox.attached) {
      return;
    }
    final overlay = Overlay.of(context);
    final mediaQuery = MediaQuery.of(context);
    final screenSize = mediaQuery.size;
    final targetRect = overlayTargetRectOf(context, renderBox);
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
              child: AnimatedBuilder(
                animation: Listenable.merge(<Listenable>[
                  _sessionStore,
                  _sessionStore.stores.downloads,
                ]),
                builder: (context, child) {
                  final tab = _sessionStore.currentTab;
                  if (tab == null) {
                    return const SizedBox.shrink();
                  }
                  return AnimatedBuilder(
                    animation: tab,
                    builder: (context, child) => OperitGlassSurface(
                      color: Theme.of(context).colorScheme.surfaceContainer,
                      layer: OperitGlassSurfaceLayer.card,
                      transparentAlpha: 1,
                      enableBackdropFilter: false,
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
                          zoomLabel: '${tab.zoomPercent}%',
                          onZoomOut: _sessionStore.zoomOut,
                          zoomButtonKey: _zoomButtonKey,
                          onZoomMenuRequested: _toggleZoomPopup,
                          onZoomIn: _sessionStore.zoomIn,
                          desktopMode: tab.desktopMode,
                          onDesktopModeChanged: (enabled) {
                            _dismissMenuPopup();
                            unawaited(_sessionStore.setDesktopMode(enabled));
                          },
                          onLoadMenuCommands: () {
                            return _sessionStore.loadMenuCommands();
                          },
                          onRunMenuCommand: (index) {
                            _dismissMenuPopup();
                            unawaited(_sessionStore.runMenuCommand(index));
                          },
                          activeDownloadCount:
                              _sessionStore.activeDownloadCount,
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),
          ],
        );
      },
    );
    overlay.insert(_menuPopupEntry!);
  }

  void _dismissMenuPopup() {
    _dismissZoomPopup();
    _menuPopupEntry?.remove();
    _menuPopupEntry = null;
  }

  static const List<double> _zoomFactors = <double>[
    0.25,
    0.4,
    0.5,
    0.67,
    0.8,
    1.0,
    1.25,
    1.5,
    1.75,
    2.0,
  ];

  /// Shows the zoom choices above the browser menu entry.
  void _toggleZoomPopup() {
    if (_zoomPopupEntry != null) {
      _dismissZoomPopup();
      return;
    }
    final anchor = _zoomButtonKey.currentContext?.findRenderObject();
    final overlay = Overlay.of(context);
    final overlayRenderObject = overlay.context.findRenderObject();
    if (anchor is! RenderBox ||
        !anchor.attached ||
        overlayRenderObject is! RenderBox) {
      return;
    }
    final anchorOffset = anchor.localToGlobal(
      Offset.zero,
      ancestor: overlayRenderObject,
    );
    final overlaySize = overlayRenderObject.size;
    const popupWidth = 112.0;
    const itemHeight = 36.0;
    final popupHeight = itemHeight * _zoomFactors.length;
    final left = (anchorOffset.dx + (anchor.size.width - popupWidth) / 2)
        .clamp(8.0, overlaySize.width - popupWidth - 8.0)
        .toDouble();
    final below = anchorOffset.dy + anchor.size.height + 4;
    final above = anchorOffset.dy - popupHeight - 4;
    final top = below + popupHeight <= overlaySize.height - 8
        ? below
        : above.clamp(8.0, overlaySize.height - popupHeight - 8.0).toDouble();
    final tab = _sessionStore.currentTab;
    if (tab == null) {
      return;
    }
    _zoomPopupEntry = OverlayEntry(
      builder: (context) {
        return Stack(
          children: <Widget>[
            Positioned.fill(
              child: GestureDetector(
                behavior: HitTestBehavior.translucent,
                onTap: _dismissZoomPopup,
                child: const SizedBox.expand(),
              ),
            ),
            Positioned(
              left: left,
              top: top,
              width: popupWidth,
              height: popupHeight,
              child: Material(
                color: Theme.of(context).colorScheme.surfaceContainer,
                elevation: 8,
                borderRadius: BorderRadius.circular(8),
                clipBehavior: Clip.antiAlias,
                child: AnimatedBuilder(
                  animation: tab,
                  builder: (context, child) {
                    return ListView.builder(
                      padding: EdgeInsets.zero,
                      itemExtent: itemHeight,
                      itemCount: _zoomFactors.length,
                      itemBuilder: (context, index) {
                        final factor = _zoomFactors[index];
                        final selected =
                            (tab.zoomFactor - factor).abs() < 0.005;
                        return InkWell(
                          onTap: () {
                            _dismissZoomPopup();
                            _sessionStore.setZoomFactor(factor);
                          },
                          child: Container(
                            padding: const EdgeInsets.symmetric(horizontal: 14),
                            color: selected
                                ? Theme.of(context).colorScheme.primaryContainer
                                      .withValues(alpha: 0.32)
                                : null,
                            alignment: Alignment.centerLeft,
                            child: Text(
                              '${(factor * 100).round()}%',
                              style: Theme.of(context).textTheme.bodyMedium,
                            ),
                          ),
                        );
                      },
                    );
                  },
                ),
              ),
            ),
          ],
        );
      },
    );
    overlay.insert(_zoomPopupEntry!);
  }

  void _dismissZoomPopup() {
    _zoomPopupEntry?.remove();
    _zoomPopupEntry = null;
  }

  /// Positions a browser panel relative to its menu anchor inside the Overlay.
  void _showPanelPopup(Widget child, {double preferredWidth = 320}) {
    _dismissPanelPopup();
    final overlay = Overlay.of(context);
    final mediaQuery = MediaQuery.of(context);
    final screenSize = mediaQuery.size;
    final horizontalPadding = 12.0 + mediaQuery.padding.left;
    final rightPadding = 12.0 + mediaQuery.padding.right;
    final renderBox =
        _menuButtonKey.currentContext?.findRenderObject() as RenderBox?;
    if (renderBox == null || !renderBox.attached || !renderBox.hasSize) {
      throw StateError('Browser panel anchor is not laid out.');
    }
    final targetBottom = overlayTargetRectOf(context, renderBox).bottom;
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
      WorkspaceBrowserSiteDataSheet(
        evaluate: _sessionStore.evaluateValue,
        clearCookies: _sessionStore.clearCookies,
      ),
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
          return _sessionStore.loadMenuCommands();
        },
        onRunMenuCommand: (index) {
          return _sessionStore.runMenuCommand(index);
        },
      ),
      preferredWidth: 420,
    );
  }
}
