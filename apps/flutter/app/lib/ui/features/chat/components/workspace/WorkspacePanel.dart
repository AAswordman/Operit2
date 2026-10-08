// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:operit2/core/browser/BrowserSessions.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserSessionRegistry.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/core/web_visit/WebVisitModels.dart';
import '../../../../common/components/RetainedPage.dart';
import '../../../../common/contributions/ToolPkgChatUiCatalog.dart';
import '../../../../main/layout/SidebarDockController.dart';
import '../../../../main/navigation/ToolPkgCatalogChangeBus.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../../../theme/OperitGlassSurface.dart';
import '../../viewmodel/WorkspaceFileModels.dart';
import 'WorkspaceOverviewModels.dart';
import 'WorkspaceOverviewContributionCatalog.dart';
import 'WorkspaceTabContent.dart';
import 'WorkspaceTabModels.dart';
import 'WorkspaceSession.dart';
import 'WorkspaceUnbindDialog.dart';
import 'WorkspaceTabStrip.dart';
import 'browser/WorkspaceBrowserViewStore.dart';
import 'browser/automation/WorkspaceWebVisitSessionRegistry.dart';
import 'file_preview/WorkspaceTextDocument.dart';
import 'file_preview/WorkspaceTextCloseDialog.dart';
import 'terminal/WorkspaceTerminalSessions.dart';
import 'terminal/WorkspaceTerminalLaunch.dart';

class WorkspacePanel extends StatefulWidget {
  const WorkspacePanel({
    super.key,
    this.sidebarDockController,
    required this.session,
    required this.currentChatId,
    required this.hasBoundWorkspace,
    required this.workspacePath,
    required this.chatCore,
    required this.onListWorkspaceFiles,
    required this.onListWorkspaceBindingDirectories,
    required this.onReadWorkspaceTextFile,
    required this.onReadWorkspaceFileBytes,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onCreateWorkspace,
    required this.onBindWorkspace,
    required this.onUnbindWorkspace,
  });

  final WorkspaceSession session;
  final String? currentChatId;
  final SidebarDockController? sidebarDockController;
  final bool hasBoundWorkspace;
  final String? workspacePath;
  final GeneratedChatRuntimeHolderMainCoreProxy chatCore;
  final Future<List<WorkspaceFileEntry>> Function(String path)
  onListWorkspaceFiles;
  final Future<List<WorkspaceFileEntry>> Function(String path)
  onListWorkspaceBindingDirectories;
  final Future<String> Function(String path) onReadWorkspaceTextFile;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path, Uint8List bytes)
  onWriteWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;
  final Future<void> Function(String name) onCreateWorkspace;
  final Future<void> Function(String workspace) onBindWorkspace;
  final Future<void> Function() onUnbindWorkspace;

  @override
  State<WorkspacePanel> createState() => _WorkspacePanelState();
}

class _WorkspacePanelState extends State<WorkspacePanel> {
  static final GeneratedCoreProxyClients _coreClients =
      GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  final BrowserSessions _browserSessions = BrowserSessions();
  final RuntimeBrowserSessionRegistry _browserSessionRegistry =
      RuntimeBrowserSessionRegistry.instance;
  final WorkspaceBrowserViewStore _browserViewStore =
      WorkspaceBrowserViewStore.instance;
  final WorkspaceWebVisitSessionRegistry _webVisitSessionRegistry =
      WorkspaceWebVisitSessionRegistry.instance;
  final WorkspaceTerminalSessions _terminalSessions =
      const WorkspaceTerminalSessions();
  final Map<String, Completer<WebVisitResponse>> _webVisitCompleters =
      <String, Completer<WebVisitResponse>>{};

  /// Returns the workspace state owned by the main chat shell.
  WorkspaceSession get _session => widget.session;
  final GlobalKey _primaryPaneKey = GlobalKey();
  final GlobalKey _secondaryPaneKey = GlobalKey();
  int? _dropHoverPaneIndex;
  _WorkspaceDropZone? _dropHoverZone;
  int _filesListingRevision = 0;
  bool _workspaceUnbindDialogOpen = false;
  List<WorkspaceTerminalSessionInfo> _terminalSessionEntries =
      const <WorkspaceTerminalSessionInfo>[];
  final ValueNotifier<int> _terminalSessionCount = ValueNotifier<int>(0);
  final ValueNotifier<int> _browserSessionCount = ValueNotifier<int>(0);
  List<core_proxy.ChatHistoryListItem>? _workspaceOverviewHistories;
  ({Object error, StackTrace stackTrace})? _workspaceOverviewHistoryError;
  WorkspaceOverviewChatScope? _workspaceOverviewChatScope;
  bool? _workspaceOverviewUseEnglish;
  final WorkspaceOverviewContributionCatalog _workspaceOverviewContributions =
      WorkspaceOverviewContributionCatalog();
  List<WorkspaceFileEntry> _workspaceOverviewMountedFolders =
      const <WorkspaceFileEntry>[];
  bool _workspaceOverviewMountedFoldersLoading = false;
  String? _workspaceOverviewMountedFoldersError;
  int _workspaceOverviewMountedFoldersLoadGeneration = 0;
  int _workspaceOverviewHistoryWatchGeneration = 0;
  StreamSubscription<List<WorkspaceTerminalSessionInfo>>?
  _terminalSessionSubscription;
  StreamSubscription<List<core_proxy.ChatHistoryListItem>>?
  _workspaceOverviewHistorySubscription;
  StreamSubscription<void>? _workspaceOverviewCatalogSubscription;

  /// Subscribes the mounted panel to workspace histories and catalog changes.
  @override
  void initState() {
    super.initState();
    widget.sidebarDockController?.addListener(_handleSidebarDockChanged);
    _syncDockedPluginTabs();
    unawaited(_browserViewStore.ensureLoaded());
    _browserSessionRegistry.addListener(_handleBrowserSessionRegistryChanged);
    _handleBrowserSessionRegistryChanged();
    _registerWebVisitControls();
    _workspaceOverviewContributions.addListener(
      _handleWorkspaceOverviewContributionsChanged,
    );
    _workspaceOverviewCatalogSubscription = ToolPkgCatalogChangeBus.listen(
      _refreshWorkspaceOverviewContributions,
    );
    _watchWorkspaceOverviewHistories();
    unawaited(_refreshWorkspaceOverviewMountedFolders());
    unawaited(_refreshTerminalSessionEntries());
    _terminalSessionSubscription = _terminalSessions.watchSessions().listen(
      (sessions) {
        if (!mounted) {
          return;
        }
        _updateTerminalSessionEntries(sessions);
      },
      onError: (Object error, StackTrace stackTrace) {
        debugPrint('Failed to watch terminal sessions: $error\n$stackTrace');
      },
    );
  }

  /// Reloads localized catalog display data when the host language changes.
  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final useEnglish = Localizations.localeOf(context).languageCode == 'en';
    if (_workspaceOverviewUseEnglish != useEnglish) {
      _workspaceOverviewUseEnglish = useEnglish;
      _refreshWorkspaceOverviewContributions();
    }
  }

  /// Rebinds subscriptions and projections when the actual chat or workspace changes.
  @override
  void didUpdateWidget(covariant WorkspacePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.sidebarDockController != widget.sidebarDockController) {
      oldWidget.sidebarDockController?.removeListener(
        _handleSidebarDockChanged,
      );
      widget.sidebarDockController?.addListener(_handleSidebarDockChanged);
      _syncDockedPluginTabs();
    }
    _registerWebVisitControls();
    if (oldWidget.chatCore != widget.chatCore) {
      _watchWorkspaceOverviewHistories();
    }
    final workspaceBindingChanged =
        oldWidget.currentChatId != widget.currentChatId ||
        oldWidget.hasBoundWorkspace != widget.hasBoundWorkspace ||
        oldWidget.workspacePath != widget.workspacePath;
    if (workspaceBindingChanged) {
      _refreshWorkspaceOverviewContributions();
      unawaited(_refreshWorkspaceOverviewMountedFolders());
    }
    if (!oldWidget.hasBoundWorkspace && widget.hasBoundWorkspace) {
      _replacePickerTabWithFilesTab();
    }
  }

  /// Releases subscriptions and invalidates every asynchronous overview request.
  @override
  void dispose() {
    widget.sidebarDockController?.removeListener(_handleSidebarDockChanged);
    _webVisitSessionRegistry.clearControls();
    _browserSessionRegistry.removeListener(
      _handleBrowserSessionRegistryChanged,
    );
    ++_workspaceOverviewHistoryWatchGeneration;
    ++_workspaceOverviewMountedFoldersLoadGeneration;
    unawaited(_workspaceOverviewHistorySubscription?.cancel());
    unawaited(_workspaceOverviewCatalogSubscription?.cancel());
    _workspaceOverviewContributions.removeListener(
      _handleWorkspaceOverviewContributionsChanged,
    );
    _workspaceOverviewContributions.dispose();
    for (final entry in _webVisitCompleters.entries) {
      final completer = entry.value;
      if (!completer.isCompleted) {
        completer.complete(
          WebVisitResponse(
            requestId: entry.key,
            success: false,
            error: 'visit_web workspace closed',
          ),
        );
      }
    }
    _webVisitCompleters.clear();
    unawaited(_terminalSessionSubscription?.cancel());
    _terminalSessionCount.dispose();
    _browserSessionCount.dispose();
    super.dispose();
  }

  /// Refreshes peer plugin tabs after a sidebar dock movement.
  void _handleSidebarDockChanged() {
    if (!mounted) {
      return;
    }
    setState(_syncDockedPluginTabs);
  }

  /// Reconciles docked plugin entries with the shared workspace tab list.
  void _syncDockedPluginTabs() {
    final controller = widget.sidebarDockController;
    if (controller == null) {
      return;
    }
    final dockedById = <String, SidebarDockedPluginView>{
      for (final view in controller.secondaryViews) view.entry.entryId: view,
    };
    _session.tabs.removeWhere(
      (tab) =>
          tab.kind == WorkspaceTabKind.plugin &&
          !dockedById.containsKey(tab.pluginEntryId),
    );
    _session.secondaryTabs.removeWhere(
      (tab) =>
          tab.kind == WorkspaceTabKind.plugin &&
          !dockedById.containsKey(tab.pluginEntryId),
    );
    final existingIds = <String>{
      for (final tab in <WorkspaceTab>[
        ..._session.tabs,
        ..._session.secondaryTabs,
      ])
        if (tab.kind == WorkspaceTabKind.plugin && tab.pluginEntryId != null)
          tab.pluginEntryId!,
    };
    String? addedPluginEntryId;
    for (final view in controller.secondaryViews) {
      if (existingIds.contains(view.entry.entryId)) {
        continue;
      }
      _session.tabs.add(
        WorkspaceTab(
          kind: WorkspaceTabKind.plugin,
          title: view.entry.title,
          icon: view.entry.icon,
          closable: false,
          pluginEntryId: view.entry.entryId,
          pluginPackageName: view.route.ownerPackageName,
          pluginUiModuleId: view.route.toolPkgUiModuleId,
        ),
      );
      addedPluginEntryId = view.entry.entryId;
    }
    _session.selectedIndex = _clampSelectedIndex(
      _session.selectedIndex,
      _session.tabs,
    );
    _session.secondarySelectedIndex = _clampSelectedIndex(
      _session.secondarySelectedIndex,
      _session.secondaryTabs,
    );
    if (addedPluginEntryId != null) {
      _session.selectedIndex = _session.tabs.indexWhere(
        (tab) => tab.pluginEntryId == addedPluginEntryId,
      );
    }
    _collapseEmptyPane();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final panel = OperitGlassSurface(
      color: theme.colorScheme.surface,
      layer: OperitGlassSurfaceLayer.panel,
      transparentAlpha: 0.035,
      borderRadius: BorderRadius.zero,
      material: true,
      clip: false,
      child: SizedBox.expand(
        child: DecoratedBox(
          decoration: BoxDecoration(
            border: BorderDirectional(
              start: BorderSide(color: theme.colorScheme.outlineVariant),
            ),
          ),
          child: _buildWorkspaceLayout(),
        ),
      ),
    );
    return panel;
  }

  /// Builds the workspace layout with one or two independently tabbed panes.
  Widget _buildWorkspaceLayout() {
    if (_session.splitAxis == null) {
      return _buildWorkspacePane(0);
    }
    final firstPane = _session.secondaryPaneFirst ? 1 : 0;
    final secondPane = _session.secondaryPaneFirst ? 0 : 1;
    final first = Expanded(
      flex: (_session.splitRatio * 1000).round().clamp(200, 800),
      child: _buildWorkspacePane(firstPane),
    );
    final second = Expanded(
      flex: ((1 - _session.splitRatio) * 1000).round().clamp(200, 800),
      child: _buildWorkspacePane(secondPane),
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        final extent = _session.splitAxis == WorkspaceSplitAxis.vertical
            ? constraints.maxWidth
            : constraints.maxHeight;
        final divider = _WorkspacePaneDivider(
          axis: _session.splitAxis!,
          extent: extent,
          onDelta: (delta, totalExtent) {
            if (totalExtent <= 0) {
              return;
            }
            setState(() {
              final change = delta / totalExtent;
              _session.splitRatio = (_session.splitRatio + change).clamp(
                0.2,
                0.8,
              );
            });
          },
        );
        return _session.splitAxis == WorkspaceSplitAxis.vertical
            ? Row(children: <Widget>[first, divider, second])
            : Column(children: <Widget>[first, divider, second]);
      },
    );
  }

  /// Builds one pane, including its tab strip, content stack, and drop zones.
  Widget _buildWorkspacePane(int paneIndex) {
    final paneTabs = _tabsForPane(paneIndex);
    final selectedIndex = _selectedIndexForPane(paneIndex);
    final content = Column(
      children: <Widget>[
        WorkspaceTabStrip(
          tabs: paneTabs,
          selectedIndex: selectedIndex,
          onSelected: (index) => _selectTab(index, paneIndex: paneIndex),
          onClosed: (index) => _closeTab(index, paneIndex: paneIndex),
        ),
        Expanded(
          child: IndexedStack(
            index: selectedIndex,
            children: <Widget>[
              for (final (tabIndex, tab) in paneTabs.indexed)
                KeyedSubtree(
                  key: ValueKey<String>('pane-$paneIndex-${_tabIdentity(tab)}'),
                  child: RetainedPage(
                    active: tabIndex == selectedIndex,
                    child: Builder(
                      builder: (_) =>
                          _buildTabContent(tab, paneIndex: paneIndex),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ],
    );
    final paneKey = paneIndex == 0 ? _primaryPaneKey : _secondaryPaneKey;
    return SizedBox.expand(
      key: paneKey,
      child: DragTarget<Object>(
        onWillAcceptWithDetails: (details) =>
            _canAcceptWorkspaceDrop(details.data),
        onMove: (details) {
          final zone = _dropZoneForGlobalOffset(
            paneKey.currentContext!,
            details.offset,
          );
          if (_dropHoverPaneIndex == paneIndex && _dropHoverZone == zone) {
            return;
          }
          setState(() {
            _dropHoverPaneIndex = paneIndex;
            _dropHoverZone = zone;
          });
        },
        onLeave: (data) {
          if (_dropHoverPaneIndex != paneIndex) {
            return;
          }
          setState(() {
            _dropHoverPaneIndex = null;
            _dropHoverZone = null;
          });
        },
        onAcceptWithDetails: (details) {
          final zone = _dropZoneForGlobalOffset(
            paneKey.currentContext!,
            details.offset,
          );
          _clearDropHover();
          _acceptWorkspaceDrop(details.data, paneIndex: paneIndex, zone: zone);
        },
        builder: (context, candidateData, rejectedData) {
          final zone = _dropHoverPaneIndex == paneIndex ? _dropHoverZone : null;
          return Stack(
            fit: StackFit.expand,
            children: <Widget>[
              content,
              if (candidateData.isNotEmpty && zone != null)
                _buildDropOverlay(context, zone),
            ],
          );
        },
      ),
    );
  }

  /// Returns whether a plugin or regular tab can enter a workspace pane.
  bool _canAcceptWorkspaceDrop(Object data) {
    if (data is SidebarDockDragPayload) {
      final controller = widget.sidebarDockController;
      return controller != null &&
          controller.canMove(data.entryId, SidebarDockLocation.secondary);
    }
    if (data is WorkspaceTabDragPayload) {
      return _containsWorkspaceTab(data.tab);
    }
    return false;
  }

  /// Returns whether a dragged regular tab is currently present in a pane.
  bool _containsWorkspaceTab(WorkspaceTab tab) {
    return _session.tabs.contains(tab) || _session.secondaryTabs.contains(tab);
  }

  /// Returns the mutable tab list belonging to a physical pane index.
  List<WorkspaceTab> _tabsForPane(int paneIndex) {
    return paneIndex == 1 ? _session.secondaryTabs : _session.tabs;
  }

  /// Returns the selected tab index belonging to a physical pane index.
  int _selectedIndexForPane(int paneIndex) {
    final tabs = _tabsForPane(paneIndex);
    final value = identical(tabs, _session.tabs)
        ? _session.selectedIndex
        : _session.secondarySelectedIndex;
    return _clampSelectedIndex(value, tabs);
  }

  /// Keeps one selected index valid for a mutable workspace tab list.
  int _clampSelectedIndex(int value, List<WorkspaceTab> tabs) {
    if (tabs.isEmpty) {
      return 0;
    }
    return value.clamp(0, tabs.length - 1).toInt();
  }

  /// Converts a global drag position into one of the five pane drop zones.
  _WorkspaceDropZone _dropZoneForGlobalOffset(
    BuildContext paneContext,
    Offset globalOffset,
  ) {
    // Existing splits accept tabs into the hovered group without splitting it.
    if (_session.splitAxis != null) {
      return _WorkspaceDropZone.center;
    }
    final renderObject = paneContext.findRenderObject()! as RenderBox;
    final local = renderObject.globalToLocal(globalOffset);
    final size = renderObject.size;
    if (local.dx <= size.width * 0.25) {
      return _WorkspaceDropZone.left;
    }
    if (local.dx >= size.width * 0.75) {
      return _WorkspaceDropZone.right;
    }
    if (local.dy <= size.height * 0.25) {
      return _WorkspaceDropZone.top;
    }
    if (local.dy >= size.height * 0.75) {
      return _WorkspaceDropZone.bottom;
    }
    return _WorkspaceDropZone.center;
  }

  /// Builds the visible highlight for the currently hovered drop zone.
  Widget _buildDropOverlay(BuildContext context, _WorkspaceDropZone zone) {
    final color = Theme.of(context).colorScheme.primary;
    final decoration = BoxDecoration(
      color: color.withValues(alpha: 0.14),
      border: Border.all(color: color, width: 2),
    );
    if (zone == _WorkspaceDropZone.center) {
      return IgnorePointer(child: DecoratedBox(decoration: decoration));
    }
    final horizontal =
        zone == _WorkspaceDropZone.left || zone == _WorkspaceDropZone.right;
    final alignment = switch (zone) {
      _WorkspaceDropZone.left => Alignment.centerLeft,
      _WorkspaceDropZone.right => Alignment.centerRight,
      _WorkspaceDropZone.top => Alignment.topCenter,
      _WorkspaceDropZone.bottom => Alignment.bottomCenter,
      _WorkspaceDropZone.center => Alignment.center,
    };
    return IgnorePointer(
      child: Align(
        alignment: alignment,
        child: FractionallySizedBox(
          widthFactor: horizontal ? 0.5 : 1,
          heightFactor: horizontal ? 1 : 0.5,
          child: DecoratedBox(decoration: decoration),
        ),
      ),
    );
  }

  /// Clears any active workspace drop-zone highlight.
  void _clearDropHover() {
    if (_dropHoverPaneIndex == null && _dropHoverZone == null) {
      return;
    }
    setState(() {
      _dropHoverPaneIndex = null;
      _dropHoverZone = null;
    });
  }

  /// Accepts a plugin or regular tab drop into a pane or directional split.
  void _acceptWorkspaceDrop(
    Object data, {
    required int paneIndex,
    required _WorkspaceDropZone zone,
  }) {
    if (data is WorkspaceTabDragPayload) {
      _moveWorkspaceTabToPane(data.tab, paneIndex: paneIndex, zone: zone);
      return;
    }
    if (data is! SidebarDockDragPayload) {
      return;
    }
    _acceptPluginDrop(data, paneIndex: paneIndex, zone: zone);
  }

  /// Accepts a plugin drop into a pane or creates a directional split.
  void _acceptPluginDrop(
    SidebarDockDragPayload payload, {
    required int paneIndex,
    required _WorkspaceDropZone zone,
  }) {
    final controller = widget.sidebarDockController;
    if (controller == null ||
        !controller.canMove(payload.entryId, SidebarDockLocation.secondary)) {
      return;
    }
    final alreadySecondary = controller.secondaryViews.any(
      (view) => view.entry.entryId == payload.entryId,
    );
    if (!alreadySecondary) {
      controller.move(
        payload.entryId,
        location: SidebarDockLocation.secondary,
        insertionIndex: controller.secondaryViews.length,
      );
    }
    final view = controller.secondaryViews.firstWhere(
      (item) => item.entry.entryId == payload.entryId,
    );
    final pluginTab = WorkspaceTab(
      kind: WorkspaceTabKind.plugin,
      title: view.entry.title,
      icon: view.entry.icon,
      closable: false,
      pluginEntryId: view.entry.entryId,
      pluginPackageName: view.route.ownerPackageName,
      pluginUiModuleId: view.route.toolPkgUiModuleId,
    );
    setState(() {
      _removePluginFromPanes(payload.entryId);
      if (_session.splitAxis == null && zone != _WorkspaceDropZone.center) {
        _session.splitAxis =
            zone == _WorkspaceDropZone.left || zone == _WorkspaceDropZone.right
            ? WorkspaceSplitAxis.vertical
            : WorkspaceSplitAxis.horizontal;
        _session.secondaryPaneFirst =
            zone == _WorkspaceDropZone.left || zone == _WorkspaceDropZone.top;
        _session.secondaryTabs.add(pluginTab);
        _session.secondarySelectedIndex = 0;
        _session.splitRatio = 0.5;
        return;
      }
      final targetTabs = _tabsForPane(paneIndex);
      final targetIndex = targetTabs.indexWhere(
        (tab) => tab.pluginEntryId == payload.entryId,
      );
      if (targetIndex >= 0) {
        targetTabs[targetIndex] = pluginTab;
        _setSelectedIndexForPane(paneIndex, targetIndex);
      } else {
        targetTabs.add(pluginTab);
        _setSelectedIndexForPane(paneIndex, targetTabs.length - 1);
      }
      _collapseEmptyPane();
    });
  }

  /// Removes an empty secondary group after a completed tab mutation.
  void _collapseEmptyPane() {
    if (_session.secondaryTabs.isNotEmpty) {
      return;
    }
    _session.splitAxis = null;
    _session.secondaryPaneFirst = false;
    _session.splitRatio = 0.5;
    _session.secondarySelectedIndex = 0;
    _dropHoverPaneIndex = null;
    _dropHoverZone = null;
  }

  /// Moves a regular workspace tab into a pane or creates a directional split.
  void _moveWorkspaceTabToPane(
    WorkspaceTab tab, {
    required int paneIndex,
    required _WorkspaceDropZone zone,
  }) {
    final sourceTabs = _session.tabs.contains(tab)
        ? _session.tabs
        : (_session.secondaryTabs.contains(tab)
              ? _session.secondaryTabs
              : null);
    if (sourceTabs == null) {
      return;
    }
    setState(() {
      if (_session.splitAxis == null && zone != _WorkspaceDropZone.center) {
        if (identical(sourceTabs, _session.tabs)) {
          _session.tabs.remove(tab);
        }
        _session.splitAxis =
            zone == _WorkspaceDropZone.left || zone == _WorkspaceDropZone.right
            ? WorkspaceSplitAxis.vertical
            : WorkspaceSplitAxis.horizontal;
        _session.secondaryPaneFirst =
            zone == _WorkspaceDropZone.left || zone == _WorkspaceDropZone.top;
        _session.secondaryTabs
          ..clear()
          ..add(tab);
        _session.secondarySelectedIndex = 0;
        _session.selectedIndex = _clampSelectedIndex(
          _session.selectedIndex,
          _session.tabs,
        );
        _session.splitRatio = 0.5;
        return;
      }
      sourceTabs.remove(tab);
      final targetTabs = _tabsForPane(paneIndex);
      targetTabs.remove(tab);
      targetTabs.add(tab);
      _setSelectedIndexForPane(paneIndex, targetTabs.length - 1);
      _session.selectedIndex = _clampSelectedIndex(
        _session.selectedIndex,
        _session.tabs,
      );
      _session.secondarySelectedIndex = _clampSelectedIndex(
        _session.secondarySelectedIndex,
        _session.secondaryTabs,
      );
      _collapseEmptyPane();
    });
  }

  /// Removes one plugin tab from both pane lists and keeps their selection valid.
  void _removePluginFromPanes(String entryId) {
    _session.tabs.removeWhere((tab) => tab.pluginEntryId == entryId);
    _session.secondaryTabs.removeWhere((tab) => tab.pluginEntryId == entryId);
    _session.selectedIndex = _clampSelectedIndex(
      _session.selectedIndex,
      _session.tabs,
    );
    _session.secondarySelectedIndex = _clampSelectedIndex(
      _session.secondarySelectedIndex,
      _session.secondaryTabs,
    );
  }

  /// Updates the selected index belonging to a physical pane.
  void _setSelectedIndexForPane(int paneIndex, int index) {
    final tabs = _tabsForPane(paneIndex);
    final value = _clampSelectedIndex(index, tabs);
    if (identical(tabs, _session.tabs)) {
      _session.selectedIndex = value;
    } else {
      _session.secondarySelectedIndex = value;
    }
  }

  void _registerWebVisitControls() {
    _webVisitSessionRegistry.setControls(openWebVisitTab: _openWebVisitTab);
  }

  /// Watches actual history summaries and rejects events from a replaced chat core.
  void _watchWorkspaceOverviewHistories() {
    final generation = ++_workspaceOverviewHistoryWatchGeneration;
    unawaited(_workspaceOverviewHistorySubscription?.cancel());
    _workspaceOverviewHistories = null;
    _workspaceOverviewHistoryError = null;
    _refreshWorkspaceOverviewContributions();
    _workspaceOverviewHistorySubscription = widget.chatCore
        .chatHistoryListItemsFlow()
        .listen(
          (histories) {
            if (!mounted ||
                generation != _workspaceOverviewHistoryWatchGeneration) {
              return;
            }
            _workspaceOverviewHistories =
                List<core_proxy.ChatHistoryListItem>.unmodifiable(histories);
            _workspaceOverviewHistoryError = null;
            _refreshWorkspaceOverviewContributions();
          },
          onError: (Object error, StackTrace stackTrace) {
            if (!mounted ||
                generation != _workspaceOverviewHistoryWatchGeneration) {
              return;
            }
            _workspaceOverviewHistories = null;
            _workspaceOverviewHistoryError = (
              error: error,
              stackTrace: stackTrace,
            );
            _refreshWorkspaceOverviewContributions();
          },
        );
  }

  /// Refreshes registered contributions using only the selected workspace summaries.
  void _refreshWorkspaceOverviewContributions() {
    if (!mounted) {
      return;
    }
    final historyError = _workspaceOverviewHistoryError;
    if (historyError != null) {
      _workspaceOverviewChatScope = null;
      _workspaceOverviewContributions.reportError(
        historyError.error,
        historyError.stackTrace,
      );
      return;
    }
    final histories = _workspaceOverviewHistories;
    final useEnglish = _workspaceOverviewUseEnglish;
    if (histories == null || useEnglish == null) {
      _workspaceOverviewChatScope = null;
      _workspaceOverviewContributions.awaitHistories();
      return;
    }
    final WorkspaceOverviewChatScope scope;
    try {
      scope = WorkspaceOverviewChatScope.fromHistories(
        histories: histories,
        currentChatId: widget.currentChatId,
        hasBoundWorkspace: widget.hasBoundWorkspace,
      );
    } catch (error, stackTrace) {
      _workspaceOverviewChatScope = null;
      _workspaceOverviewContributions.reportError(error, stackTrace);
      return;
    }
    _workspaceOverviewChatScope = scope;
    unawaited(
      _workspaceOverviewContributions.refresh(
        catalog: ToolPkgChatUiCatalog(
          clients: _coreClients,
          useEnglish: useEnglish,
        ),
        chats: scope.chats,
      ),
    );
  }

  /// Rebuilds the mounted overview for explicit loading, success, and error states.
  void _handleWorkspaceOverviewContributionsChanged() {
    if (!mounted) {
      return;
    }
    setState(() {});
  }

  /// Refreshes mounted folders shown by the workspace overview.
  Future<void> _refreshWorkspaceOverviewMountedFolders() async {
    final loadGeneration = ++_workspaceOverviewMountedFoldersLoadGeneration;
    if (!widget.hasBoundWorkspace) {
      setState(() {
        _workspaceOverviewMountedFolders = const <WorkspaceFileEntry>[];
        _workspaceOverviewMountedFoldersLoading = false;
        _workspaceOverviewMountedFoldersError = null;
      });
      return;
    }

    setState(() {
      _workspaceOverviewMountedFolders = const <WorkspaceFileEntry>[];
      _workspaceOverviewMountedFoldersLoading = true;
      _workspaceOverviewMountedFoldersError = null;
    });
    try {
      final entries = await widget.onListWorkspaceFiles('');
      final mountedFolders = entries
          .where((entry) => entry.isDirectory)
          .toList(growable: false);
      if (!mounted ||
          loadGeneration != _workspaceOverviewMountedFoldersLoadGeneration) {
        return;
      }
      setState(() {
        _workspaceOverviewMountedFolders =
            List<WorkspaceFileEntry>.unmodifiable(mountedFolders);
        _workspaceOverviewMountedFoldersLoading = false;
      });
    } catch (error, stackTrace) {
      debugPrint(
        'Failed to refresh workspace overview folders: $error\n$stackTrace',
      );
      if (!mounted ||
          loadGeneration != _workspaceOverviewMountedFoldersLoadGeneration) {
        return;
      }
      setState(() {
        _workspaceOverviewMountedFoldersLoading = false;
        _workspaceOverviewMountedFoldersError = error.toString();
      });
    }
  }

  /// Converts mounted folder entries into overview display models.
  List<WorkspaceMountedFolder> _workspaceOverviewMountedFolderModels() {
    final mountedFolders = <WorkspaceMountedFolder>[];
    for (final entry in _workspaceOverviewMountedFolders) {
      final name = entry.name.trim();
      final path = entry.path.trim();
      final relativePath = entry.relativePath.trim();
      if (name.isEmpty || path.isEmpty || relativePath.isEmpty) {
        continue;
      }
      mountedFolders.add(
        WorkspaceMountedFolder(
          name: name,
          path: path,
          relativePath: relativePath,
        ),
      );
    }
    return List<WorkspaceMountedFolder>.unmodifiable(mountedFolders);
  }

  /// Keeps host counts independent of plugin-owned contribution usage records.
  WorkspaceOverviewUsage _workspaceOverviewUsage() {
    final workspaceName = _workspaceOverviewChatScope?.workspaceName?.trim();
    return WorkspaceOverviewUsage(
      workspaceName: workspaceName == null || workspaceName.isEmpty
          ? null
          : workspaceName,
      conversationCount: _workspaceOverviewChatScope?.chats.length,
      contributions: _workspaceOverviewContributions.state,
      mountedFolders: _workspaceOverviewMountedFolderModels(),
      mountedFoldersLoading: _workspaceOverviewMountedFoldersLoading,
      mountedFoldersError: _workspaceOverviewMountedFoldersError,
    );
  }

  /// Selects a tab in the requested physical workspace pane.
  void _selectTab(int index, {int paneIndex = 0}) {
    setState(() {
      _setSelectedIndexForPane(paneIndex, index);
    });
  }

  void _openSingletonTab(WorkspaceTab tab) {
    final existingIndex = _session.tabs.indexWhere(
      (item) => item.kind == tab.kind,
    );
    setState(() {
      if (existingIndex >= 0) {
        _session.tabs[existingIndex] = tab;
        _session.selectedIndex = existingIndex;
      } else {
        _session.tabs.add(tab);
        _session.selectedIndex = _session.tabs.length - 1;
      }
    });
  }

  void _openBrowserTab({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
    String? userAgent,
    Map<String, String>? headers,
  }) {
    unawaited(
      _openBrowserTabAsync(
        url: url,
        localFilePath: localFilePath,
        workspaceHtmlPath: workspaceHtmlPath,
        userAgent: userAgent,
        headers: headers,
      ),
    );
  }

  Future<void> _openBrowserTabAsync({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
    String? userAgent,
    Map<String, String>? headers,
  }) async {
    final htmlPath = workspaceHtmlPath?.trim();
    if (htmlPath != null && htmlPath.isNotEmpty) {
      await _browserViewStore.openWorkspaceHtmlTab(htmlPath, initialUrl: url);
    } else {
      final filePath = localFilePath?.trim();
      if (filePath != null && filePath.isNotEmpty) {
        await _browserViewStore.openLocalFileTab(filePath);
      } else {
        await _browserViewStore.openTab(
          url: url,
          userAgent: userAgent,
          headers: headers,
        );
      }
    }
    if (!mounted) {
      return;
    }
    _openBrowserWorkspaceTab();
  }

  void _openBrowserWorkspaceTab() {
    if (!mounted) {
      return;
    }
    final existingIndex = _session.tabs.indexWhere(
      (item) => item.kind == WorkspaceTabKind.browser,
    );
    setState(() {
      if (existingIndex >= 0) {
        _session.selectedIndex = existingIndex;
        return;
      }
      _session.tabs.add(
        const WorkspaceTab(
          kind: WorkspaceTabKind.browser,
          title: '',
          icon: Icons.public,
        ),
      );
      _session.selectedIndex = _session.tabs.length - 1;
    });
  }

  Future<WebVisitResponse> _openWebVisitTab(WebVisitRequest request) {
    final completer = Completer<WebVisitResponse>();
    _webVisitCompleters[request.requestId] = completer;
    final tab = WorkspaceTab(
      kind: WorkspaceTabKind.webVisit,
      title: _webVisitTabTitle(request),
      icon: Icons.travel_explore,
      webVisitRequest: request,
    );
    setState(() {
      _session.tabs.add(tab);
      _session.selectedIndex = _session.tabs.length - 1;
    });
    return completer.future;
  }

  String _webVisitTabTitle(WebVisitRequest request) {
    final uri = Uri.tryParse(request.url.trim());
    if (uri != null && uri.host.isNotEmpty) {
      return uri.host;
    }
    return 'visit_web';
  }

  /// Builds one tab's content with callbacks bound to its physical pane.
  Widget _buildTabContent(WorkspaceTab tab, {int paneIndex = 0}) {
    return WorkspaceTabContent(
      tab: tab,
      workspacePath: widget.workspacePath,
      workspaceUsage: _workspaceOverviewUsage(),
      terminalSessionCountListenable: _terminalSessionCount,
      browserSessionCountListenable: _browserSessionCount,
      onListWorkspaceFiles: widget.onListWorkspaceFiles,
      onListWorkspaceBindingDirectories:
          widget.onListWorkspaceBindingDirectories,
      onReadWorkspaceTextFile: widget.onReadWorkspaceTextFile,
      onReadWorkspaceFileBytes: widget.onReadWorkspaceFileBytes,
      onWriteWorkspaceFileBytes: widget.onWriteWorkspaceFileBytes,
      onOpenWorkspaceFile: widget.onOpenWorkspaceFile,
      onOpenFile: _openFileTab,
      onOpenFolder: _openMountedFolder,
      onAddFolder: _openWorkspaceBindingPickerTab,
      filesListingRevision: _filesListingRevision,
      onOpenTerminal: _createAndOpenTerminalSession,
      onOpenTerminalSessions: _showTerminalSessionPicker,
      onOpenBrowserSessions: _showBrowserSessionPicker,
      onOpenBrowser: _openBrowserTab,
      onFinishWebVisit: (tab, response) =>
          _finishWebVisitTab(tab, response, paneIndex: paneIndex),
      onActivateCurrentTab: () =>
          _selectWorkspaceTab(tab, paneIndex: paneIndex),
      onCloseCurrentTab: () => _closeWorkspaceTab(tab, paneIndex: paneIndex),
      onOpenWorkspaceCreator: _showCreateWorkspaceDialog,
      onBindWorkspace: _bindWorkspaceFolder,
      onUnbindWorkspace: _showUnbindWorkspaceDialog,
      onChooseExistingWorkspace: _openWorkspaceBindingPickerTab,
      splitMarkdownContent: (content) =>
          widget.chatCore.splitMarkdownContent(content: content),
    );
  }

  /// Creates and opens a manual terminal session using the host-declared type.
  Future<void> _createAndOpenTerminalSession() async {
    final terminalInfo = await _terminalSessions.terminalInfo();
    if (!mounted) {
      return;
    }
    final terminal = await _selectTerminalType(terminalInfo);
    if (!mounted || terminal == null) {
      return;
    }
    await launchWorkspaceTerminal(
      context: context,
      terminal: terminal.terminal,
      terminalType: terminal.terminalType,
      launch: () async {
        final workingDirectory = _manualTerminalWorkingDirectory();
        final sessionId = await _terminalSessions.startPtySession(
          sessionName: _nextManualTerminalSessionName(),
          terminal: terminal.terminal,
          terminalType: terminal.terminalType,
          workingDirectory: workingDirectory,
          rows: 24,
          columns: 80,
        );
        final sessions = await _terminalSessions.listSessions();
        final session = sessions.firstWhere(
          (item) => item.sessionId == sessionId,
        );
        if (!mounted) {
          return;
        }
        _updateTerminalSessionEntries(sessions);
        _openTerminalSessionTab(session);
      },
    );
  }

  /// Prompts the user to choose one terminal implementation exposed by the active host.
  Future<core_proxy.RuntimeTerminalTypeInfo?> _selectTerminalType(
    core_proxy.RuntimeTerminalInfo terminalInfo,
  ) {
    return showDialog<core_proxy.RuntimeTerminalTypeInfo>(
      context: context,
      builder: (context) {
        final theme = Theme.of(context);
        final availableTypes = terminalInfo.types
            .where((item) => item.available)
            .toList(growable: false);
        return AlertDialog(
          title: const Text('选择终端类型'),
          content: SizedBox(
            width: 460,
            child: availableTypes.isEmpty
                ? Text(
                    '当前平台没有可用的终端类型。',
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  )
                : RadioGroup<String>(
                    groupValue:
                        '${terminalInfo.terminal}\u0000${terminalInfo.terminalType}',
                    onChanged: (value) {
                      final selected = availableTypes.firstWhere(
                        (item) =>
                            '${item.terminal}\u0000${item.terminalType}' ==
                            value,
                      );
                      Navigator.of(context).pop(selected);
                    },
                    child: ListView.separated(
                      shrinkWrap: true,
                      itemCount: terminalInfo.types.length,
                      separatorBuilder: (context, index) =>
                          const Divider(height: 1),
                      itemBuilder: (context, index) {
                        final item = terminalInfo.types[index];
                        return ListTile(
                          enabled: item.available,
                          leading: Radio<String>(
                            value: '${item.terminal}\u0000${item.terminalType}',
                            enabled: item.available,
                          ),
                          title: Text(
                            '${item.terminal} / ${item.terminalType}',
                          ),
                          subtitle: Text(item.description),
                          onTap: item.available
                              ? () => Navigator.of(context).pop(item)
                              : null,
                        );
                      },
                    ),
                  ),
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('取消'),
            ),
          ],
        );
      },
    );
  }

  /// Returns a VFS locator whose filesystem is resolved by the selected terminal host.
  String _manualTerminalWorkingDirectory() {
    if (!widget.hasBoundWorkspace) {
      return '/app/data';
    }
    final workspaceDirectory = widget.workspacePath?.trim();
    if (workspaceDirectory == null || workspaceDirectory.isEmpty) {
      throw StateError('工作区路径为空');
    }
    return workspaceDirectory;
  }

  String _nextManualTerminalSessionName() {
    final manualCount = _terminalSessionEntries
        .where((session) => session.sessionName.trim().startsWith('手动终端'))
        .length;
    return '手动终端 ${manualCount + 1}';
  }

  Future<void> _showTerminalSessionPicker() async {
    final sessions = await _terminalSessions.listSessions();
    if (!mounted) {
      return;
    }
    _updateTerminalSessionEntries(sessions);
    await showDialog<void>(
      context: context,
      builder: (context) {
        final theme = Theme.of(context);
        var dialogSessions = sessions;
        final closingSessionIds = <String>{};
        return StatefulBuilder(
          builder: (context, setDialogState) {
            Future<void> closeSession(
              WorkspaceTerminalSessionInfo session,
            ) async {
              setDialogState(() {
                closingSessionIds.add(session.sessionId);
              });
              _removeTerminalTabsForSession(session.sessionId);
              await _closeManualTerminalSession(session.sessionId);
              final updatedSessions = await _terminalSessions.listSessions();
              if (!mounted) {
                return;
              }
              _updateTerminalSessionEntries(updatedSessions);
              setDialogState(() {
                closingSessionIds.remove(session.sessionId);
                dialogSessions = updatedSessions;
              });
            }

            return AlertDialog(
              title: const Text('终端会话'),
              contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 8),
              content: SizedBox(
                width: 520,
                child: dialogSessions.isEmpty
                    ? Text(
                        '当前没有终端会话',
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      )
                    : ConstrainedBox(
                        constraints: const BoxConstraints(maxHeight: 420),
                        child: ListView.separated(
                          shrinkWrap: true,
                          itemCount: dialogSessions.length,
                          separatorBuilder: (context, index) =>
                              const Divider(height: 1),
                          itemBuilder: (context, index) {
                            final session = dialogSessions[index];
                            final isClosing = closingSessionIds.contains(
                              session.sessionId,
                            );
                            return ListTile(
                              dense: true,
                              contentPadding: EdgeInsets.zero,
                              title: Text(session.title),
                              subtitle: Padding(
                                padding: const EdgeInsets.only(top: 4),
                                child: Text(
                                  _terminalSessionSubtitle(session),
                                  maxLines: 3,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                              trailing: IconButton(
                                tooltip: '结束进程',
                                onPressed: isClosing
                                    ? null
                                    : () => closeSession(session),
                                icon: isClosing
                                    ? const SizedBox.square(
                                        dimension: 18,
                                        child: CircularProgressIndicator(
                                          strokeWidth: 2,
                                        ),
                                      )
                                    : const Icon(Icons.stop_circle_outlined),
                              ),
                              onTap: () {
                                Navigator.of(context).pop();
                                _openTerminalSessionTab(session);
                              },
                            );
                          },
                        ),
                      ),
              ),
              actions: <Widget>[
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text('关闭'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  Future<void> _showBrowserSessionPicker() async {
    final sessions = await _browserSessions.listSessions();
    if (!mounted) {
      return;
    }
    await showDialog<void>(
      context: context,
      builder: (context) {
        final theme = Theme.of(context);
        var dialogSessions = sessions;
        return StatefulBuilder(
          builder: (context, setDialogState) {
            Future<void> closeSession(String sessionId) async {
              await _browserSessions.close(sessionId);
              final updated = await _browserSessions.listSessions();
              if (!context.mounted) {
                return;
              }
              setDialogState(() {
                dialogSessions = updated;
              });
            }

            return AlertDialog(
              title: const Text('浏览器会话'),
              contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 8),
              content: SizedBox(
                width: 520,
                child: dialogSessions.isEmpty
                    ? Text(
                        '当前没有浏览器会话',
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      )
                    : ConstrainedBox(
                        constraints: const BoxConstraints(maxHeight: 420),
                        child: ListView.separated(
                          shrinkWrap: true,
                          itemCount: dialogSessions.length,
                          separatorBuilder: (context, index) =>
                              const Divider(height: 1),
                          itemBuilder: (context, index) {
                            final session = dialogSessions[index];
                            return ListTile(
                              dense: true,
                              contentPadding: EdgeInsets.zero,
                              title: Text(session.title),
                              subtitle: Padding(
                                padding: const EdgeInsets.only(top: 4),
                                child: Text(
                                  _browserSessionSubtitle(session),
                                  maxLines: 3,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                              trailing: IconButton(
                                tooltip: '关闭会话',
                                onPressed: () {
                                  unawaited(closeSession(session.sessionId));
                                },
                                icon: const Icon(Icons.close),
                              ),
                              onTap: () async {
                                Navigator.of(context).pop();
                                await _browserViewStore.attachSession(session);
                                if (!mounted) {
                                  return;
                                }
                                _openBrowserWorkspaceTab();
                                _browserViewStore.selectSession(
                                  session.sessionId,
                                );
                              },
                            );
                          },
                        ),
                      ),
              ),
              actions: <Widget>[
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text('关闭'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  void _updateTerminalSessionEntries(
    List<WorkspaceTerminalSessionInfo> sessions,
  ) {
    _terminalSessionEntries = sessions;
    _terminalSessionCount.value = sessions.length;
  }

  /// Updates terminal session entries from the runtime snapshot.
  Future<void> _refreshTerminalSessionEntries() async {
    final sessions = await _terminalSessions.listSessions();
    if (!mounted) {
      return;
    }
    _updateTerminalSessionEntries(sessions);
  }

  /// Updates the browser session counter from the owner registry.
  void _handleBrowserSessionRegistryChanged() {
    _browserSessionCount.value = _browserSessionRegistry.sessions.length;
  }

  /// Removes closed terminal sessions from both tab groups.
  void _removeTerminalTabsForSession(String sessionId) {
    final selectedTab = _session.tabs[_session.selectedIndex];
    setState(() {
      _session.tabs.removeWhere((tab) => tab.terminalSessionId == sessionId);
      final secondarySelected = _session.secondaryTabs.isEmpty
          ? null
          : _session.secondaryTabs[_session.secondarySelectedIndex];
      _session.secondaryTabs.removeWhere(
        (tab) => tab.terminalSessionId == sessionId,
      );
      _session.secondarySelectedIndex = _clampSelectedIndex(
        secondarySelected == null
            ? 0
            : _session.secondaryTabs.indexOf(secondarySelected),
        _session.secondaryTabs,
      );
      _collapseEmptyPane();
      final preservedIndex = _session.tabs.indexOf(selectedTab);
      if (preservedIndex >= 0) {
        _session.selectedIndex = preservedIndex;
      } else {
        _session.selectedIndex = _session.selectedIndex
            .clamp(0, _session.tabs.length - 1)
            .toInt();
      }
    });
  }

  String _terminalSessionSubtitle(WorkspaceTerminalSessionInfo session) {
    final workingDir = session.workingDir.trim();
    const prefix = 'PTY';
    if (workingDir.isNotEmpty) {
      return 'UUID · ${session.sessionId}\n$prefix · $workingDir';
    }
    return 'UUID · ${session.sessionId}\n$prefix · ${session.terminalType}';
  }

  String _browserSessionSubtitle(core_proxy.RuntimeBrowserSessionInfo session) {
    return 'UUID · ${session.sessionId}\n${session.currentUrl}';
  }

  void _openTerminalSessionTab(WorkspaceTerminalSessionInfo session) {
    final existingIndex = _session.tabs.indexWhere(
      (tab) => tab.terminalSessionId == session.sessionId,
    );
    final tab = WorkspaceTab(
      kind: WorkspaceTabKind.terminal,
      title: session.title,
      icon: Icons.terminal,
      terminalSessionId: session.sessionId,
      terminalType: session.terminalType,
      terminalWorkingDir: session.workingDir,
    );
    setState(() {
      if (existingIndex >= 0) {
        _session.tabs[existingIndex] = tab;
        _session.selectedIndex = existingIndex;
      } else {
        _session.tabs.add(tab);
        _session.selectedIndex = _session.tabs.length - 1;
      }
    });
  }

  /// Shows the workspace creation dialog without opening a setup tab.
  void _showCreateWorkspaceDialog() {
    showDialog<void>(
      context: context,
      builder: (context) =>
          _CreateWorkspaceDialog(onCreateWorkspace: _createWorkspace),
    );
  }

  /// Confirms detaching only this chat, keeping workspace files intact.
  Future<void> _showUnbindWorkspaceDialog() async {
    final chatId = widget.currentChatId;
    if (_workspaceUnbindDialogOpen || chatId == null) {
      return;
    }
    final workspacePath = widget.workspacePath;
    final onUnbindWorkspace = widget.onUnbindWorkspace;
    _workspaceUnbindDialogOpen = true;
    try {
      await showDialog<void>(
        context: context,
        barrierDismissible: false,
        builder: (context) => WorkspaceUnbindDialog(
          onUnbindWorkspace: () async {
            // A chat switch while the confirmation is open must not detach
            // the newly selected chat or a different workspace binding.
            if (!mounted ||
                widget.currentChatId != chatId ||
                widget.workspacePath != workspacePath) {
              throw StateError(
                AppLocalizations.of(context)!.workspaceUnbindChanged,
              );
            }
            await onUnbindWorkspace();
          },
        ),
      );
    } finally {
      _workspaceUnbindDialogOpen = false;
    }
  }

  /// Opens the file tree for one mounted workspace folder.
  void _openMountedFolder(WorkspaceMountedFolder folder) {
    final relativePath = folder.relativePath.trim();
    final title = folder.name.trim();
    setState(() {
      _session.tabIdentitySequence += 1;
      _session.tabs.add(
        WorkspaceTab(
          kind: WorkspaceTabKind.files,
          title: title,
          icon: Icons.folder_outlined,
          filePath: relativePath,
          identityToken: 'mounted-folder-$_session.tabIdentitySequence',
        ),
      );
      _session.selectedIndex = _session.tabs.length - 1;
    });
  }

  /// Opens the workspace folder picker for binding or mounting.
  void _openWorkspaceBindingPickerTab() {
    final l10n = AppLocalizations.of(context)!;
    _openSingletonTab(
      WorkspaceTab(
        kind: WorkspaceTabKind.workspacePicker,
        title: widget.hasBoundWorkspace ? l10n.workspaceAddFolderTitle : '',
        icon: Icons.folder_open_outlined,
      ),
    );
  }

  /// Binds or mounts the selected folder, then shows the workspace file tree.
  Future<void> _bindWorkspaceFolder(String workspace) async {
    await widget.onBindWorkspace(workspace);
    if (!mounted) {
      return;
    }
    if (widget.hasBoundWorkspace) {
      unawaited(_refreshWorkspaceOverviewMountedFolders());
    }
    setState(() {
      _filesListingRevision += 1;
    });
    _replacePickerTabWithFilesTab();
  }

  /// Creates a named workspace and switches the workspace panel back to files.
  Future<void> _createWorkspace(String name) async {
    await widget.onCreateWorkspace(name);
    if (!mounted) {
      return;
    }
    if (widget.hasBoundWorkspace) {
      unawaited(_refreshWorkspaceOverviewMountedFolders());
    }
    setState(() {
      _filesListingRevision += 1;
    });
    _replacePickerTabWithFilesTab();
  }

  /// Replaces the workspace picker tab with the workspace files tab.
  void _replacePickerTabWithFilesTab() {
    final targetIndex = _session.tabs.indexWhere(
      (tab) => tab.kind == WorkspaceTabKind.workspacePicker,
    );
    if (targetIndex < 0) {
      return;
    }
    final selectedTab = _session.tabs[_session.selectedIndex];
    final selectedWasTarget =
        selectedTab.kind == WorkspaceTabKind.workspacePicker;
    setState(() {
      _session.tabs.removeWhere(
        (tab) => tab.kind == WorkspaceTabKind.workspacePicker,
      );
      var filesIndex = _session.tabs.indexWhere(
        (tab) => tab.kind == WorkspaceTabKind.files,
      );
      if (filesIndex < 0) {
        _session.tabs.add(
          const WorkspaceTab(
            kind: WorkspaceTabKind.files,
            title: '',
            icon: Icons.folder_outlined,
          ),
        );
        filesIndex = _session.tabs.length - 1;
      }
      if (selectedWasTarget) {
        _session.selectedIndex = filesIndex;
        return;
      }
      final preservedIndex = _session.tabs.indexOf(selectedTab);
      _session.selectedIndex = preservedIndex >= 0
          ? preservedIndex
          : _session.selectedIndex.clamp(0, _session.tabs.length - 1).toInt();
    });
  }

  /// Builds a stable workspace tab identity for diffing tab lists.
  String _tabIdentity(WorkspaceTab tab) {
    return <String>[
      tab.kind.name,
      tab.filePath ?? '',
      tab.absolutePath ?? '',
      tab.url ?? '',
      tab.workspaceHtmlPath ?? '',
      tab.webVisitRequest?.requestId ?? '',
      tab.terminalSessionId ?? '',
      tab.pluginEntryId ?? '',
      tab.pluginPackageName ?? '',
      tab.pluginUiModuleId ?? '',
      tab.identityToken,
      tab.title,
    ].join('|');
  }

  /// Confirms unsaved changes before closing a tab from its physical pane.
  Future<void> _closeTab(int index, {int paneIndex = 0}) async {
    final tabs = _tabsForPane(paneIndex);
    if (index < 0 || index >= tabs.length) {
      return;
    }
    final tab = tabs[index];
    if (!tab.closable) {
      return;
    }
    final document = tab.textDocument;
    if (document != null && (document.isDirty || document.isSaving)) {
      final confirmed = await confirmWorkspaceTextClose(
        context,
        document: document,
        write: (text) => widget.onWriteWorkspaceFileBytes(
          tab.filePath!,
          Uint8List.fromList(utf8.encode(text)),
        ),
      );
      if (!mounted || !confirmed) return;
    }
    // Tab positions can change while the close dialog is open.
    final currentIndex = tabs.indexOf(tab);
    if (currentIndex < 0) return;
    setState(() {
      final selectedIndex = _selectedIndexForPane(paneIndex);
      tabs.removeAt(currentIndex);
      if (selectedIndex == currentIndex) {
        _setSelectedIndexForPane(paneIndex, currentIndex - 1);
      } else if (selectedIndex > currentIndex) {
        _setSelectedIndexForPane(paneIndex, selectedIndex - 1);
      }
      _collapseEmptyPane();
    });
    if (tab.kind == WorkspaceTabKind.plugin) {
      final entryId = tab.pluginEntryId;
      final controller = widget.sidebarDockController;
      if (entryId != null && controller != null) {
        controller.move(
          entryId,
          location: SidebarDockLocation.primary,
          insertionIndex: controller.primaryEntries.length,
        );
      }
    }
    if (tab.kind == WorkspaceTabKind.terminal) {
      final sessionId = tab.terminalSessionId;
      if (sessionId != null && sessionId.trim().isNotEmpty) {
        unawaited(_closeManualTerminalSession(sessionId));
      }
    }
    if (tab.kind == WorkspaceTabKind.webVisit) {
      final request = tab.webVisitRequest;
      if (request != null) {
        _completeWebVisitRequest(
          request.requestId,
          WebVisitResponse(
            requestId: request.requestId,
            success: false,
            error: 'visit_web cancelled',
          ),
        );
      }
    }
  }

  /// Selects a tab after a content widget requests activation.
  void _selectWorkspaceTab(WorkspaceTab tab, {int paneIndex = 0}) {
    final tabs = _tabsForPane(paneIndex);
    final index = tabs.indexOf(tab);
    if (index < 0) {
      return;
    }
    setState(() {
      _setSelectedIndexForPane(paneIndex, index);
    });
  }

  /// Closes a tab after its content widget requests closure.
  void _closeWorkspaceTab(WorkspaceTab tab, {int paneIndex = 0}) {
    final tabs = _tabsForPane(paneIndex);
    final index = tabs.indexOf(tab);
    if (index < 0) {
      return;
    }
    unawaited(_closeTab(index, paneIndex: paneIndex));
  }

  /// Completes a web visit and removes its tab from the owning pane.
  void _finishWebVisitTab(
    WorkspaceTab tab,
    WebVisitResponse response, {
    int paneIndex = 0,
  }) {
    final request = tab.webVisitRequest;
    if (request == null) {
      return;
    }
    _completeWebVisitRequest(request.requestId, response);
    final tabs = _tabsForPane(paneIndex);
    final index = tabs.indexOf(tab);
    if (index >= 0) {
      setState(() {
        final selectedIndex = _selectedIndexForPane(paneIndex);
        tabs.removeAt(index);
        if (selectedIndex == index) {
          _setSelectedIndexForPane(paneIndex, index - 1);
        } else if (selectedIndex > index) {
          _setSelectedIndexForPane(paneIndex, selectedIndex - 1);
        }
        _collapseEmptyPane();
      });
    }
  }

  void _completeWebVisitRequest(String requestId, WebVisitResponse response) {
    final completer = _webVisitCompleters.remove(requestId);
    if (completer != null && !completer.isCompleted) {
      completer.complete(response);
    }
  }

  /// Closes the host PTY after removed terminal widgets dispose listeners.
  Future<void> _closeManualTerminalSession(String sessionId) async {
    await WidgetsBinding.instance.endOfFrame;
    await _terminalSessions.closePtySession(sessionId);
  }

  /// Activates an existing file tab without reloading its editable draft.
  bool _activateOpenFile(String absolutePath) {
    for (final paneIndex in <int>[0, 1]) {
      final tabs = _tabsForPane(paneIndex);
      final index = tabs.indexWhere(
        (tab) =>
            tab.kind == WorkspaceTabKind.filePreview &&
            tab.absolutePath == absolutePath,
      );
      if (index >= 0) {
        _selectWorkspaceTab(tabs[index], paneIndex: paneIndex);
        return true;
      }
    }
    return false;
  }

  /// Opens a file once and gives editable formats a persistent document.
  Future<void> _openFileTab(WorkspaceFileEntry entry) async {
    if (_activateOpenFile(entry.path)) return;
    final previewKind = workspacePreviewKindForPath(entry.path);
    WorkspaceTextDocument? document;
    // HTML is loaded by the preview server; pre-reading its text duplicates
    // the core/VFS round trip and allocates an unused copy of the document.
    if (previewKind == WorkspaceFilePreviewKind.text ||
        previewKind == WorkspaceFilePreviewKind.markdown) {
      document = WorkspaceTextDocument(
        await widget.onReadWorkspaceTextFile(entry.relativePath),
      );
    }

    if (!mounted) {
      return;
    }

    // Another open request may complete while this file read is pending.
    if (_activateOpenFile(entry.path)) return;
    final tab = WorkspaceTab(
      kind: WorkspaceTabKind.filePreview,
      title: entry.name,
      icon: workspacePreviewIconForKind(previewKind),
      filePath: entry.relativePath,
      absolutePath: entry.path,
      textDocument: document,
      previewKind: previewKind,
    );
    setState(() {
      _session.tabs.add(tab);
      _session.selectedIndex = _session.tabs.length - 1;
    });
  }
}

enum _WorkspaceDropZone { center, left, right, top, bottom }

/// Renders and handles the draggable divider between workspace panes.
class _WorkspacePaneDivider extends StatelessWidget {
  const _WorkspacePaneDivider({
    required this.axis,
    required this.extent,
    required this.onDelta,
  });

  final WorkspaceSplitAxis axis;
  final double extent;
  final void Function(double delta, double extent) onDelta;

  /// Builds a resize handle with the cursor matching its orientation.
  @override
  Widget build(BuildContext context) {
    final vertical = axis == WorkspaceSplitAxis.vertical;
    final color = Theme.of(context).colorScheme.outlineVariant;
    return MouseRegion(
      cursor: vertical
          ? SystemMouseCursors.resizeColumn
          : SystemMouseCursors.resizeRow,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onHorizontalDragUpdate: vertical
            ? (details) => onDelta(details.delta.dx, extent)
            : null,
        onVerticalDragUpdate: vertical
            ? null
            : (details) => onDelta(details.delta.dy, extent),
        child: SizedBox(
          width: vertical ? 9 : double.infinity,
          height: vertical ? double.infinity : 9,
          child: Center(
            child: DecoratedBox(
              decoration: BoxDecoration(color: color.withValues(alpha: 0.7)),
              child: SizedBox(
                width: vertical ? 1 : double.infinity,
                height: vertical ? double.infinity : 1,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Displays the workspace creation form and owns its text controller.
class _CreateWorkspaceDialog extends StatefulWidget {
  const _CreateWorkspaceDialog({required this.onCreateWorkspace});

  final Future<void> Function(String name) onCreateWorkspace;

  /// Creates the state that owns the workspace creation form.
  @override
  State<_CreateWorkspaceDialog> createState() => _CreateWorkspaceDialogState();
}

/// Manages the workspace creation form lifecycle.
class _CreateWorkspaceDialogState extends State<_CreateWorkspaceDialog> {
  final TextEditingController _nameController = TextEditingController();
  bool _dialogBusy = false;
  String? _dialogError;

  /// Submits the workspace name and closes the dialog after creation.
  Future<void> _submitCreateWorkspace() async {
    if (_dialogBusy) {
      return;
    }
    final l10n = AppLocalizations.of(context)!;
    final name = _nameController.text.trim();
    if (name.isEmpty) {
      setState(() {
        _dialogError = l10n.workspaceNameHint;
      });
      return;
    }
    setState(() {
      _dialogBusy = true;
      _dialogError = null;
    });
    try {
      await widget.onCreateWorkspace(name);
      if (mounted) {
        Navigator.of(context).pop();
      }
    } catch (error, stackTrace) {
      debugPrint('Workspace creation failed: $error\n$stackTrace');
      if (mounted) {
        setState(() {
          _dialogError = error.toString();
        });
      }
    } finally {
      if (mounted) {
        setState(() {
          _dialogBusy = false;
        });
      }
    }
  }

  /// Releases the controller owned by the dialog state.
  /// Releases subscriptions and invalidates every asynchronous overview request.
  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  /// Builds the workspace creation form.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.workspaceCreateTitle),
      content: SizedBox(
        width: 420,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              TextField(
                controller: _nameController,
                autofocus: true,
                enabled: !_dialogBusy,
                decoration: InputDecoration(
                  labelText: l10n.workspaceNameLabel,
                  hintText: l10n.workspaceNameHint,
                ),
              ),
              if (_dialogError != null) ...<Widget>[
                const SizedBox(height: 10),
                Text(
                  _dialogError!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
              const SizedBox(height: 14),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _dialogBusy ? null : () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: _dialogBusy
              ? null
              : () {
                  unawaited(_submitCreateWorkspace());
                },
          child: Text(l10n.bind),
        ),
      ],
    );
  }
}
