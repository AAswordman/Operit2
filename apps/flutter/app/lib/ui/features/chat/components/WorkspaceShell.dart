// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../common/components/AdaptiveSidePanel.dart';
import '../../../common/contributions/PluginWorkspacePresentation.dart';
import '../../../common/markdown/MarkdownLink.dart';
import '../../../common/markdown/MarkdownFileScope.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../main/MainLayoutController.dart';
import '../../../main/layout/SidebarDockController.dart';
import '../../../main/layout/NavigationLayoutMetrics.dart';
import '../viewmodel/WorkspaceFileModels.dart';
import 'workspace/WorkspaceLayoutMetrics.dart';
import 'workspace/WorkspacePanel.dart';
import 'workspace/WorkspaceSession.dart';
import 'workspace/WorkspaceSessionStore.dart';

class WorkspaceShell extends StatelessWidget {
  static final WorkspaceSessionStore sessions = WorkspaceSessionStore();
  static final _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  /// Returns the workspace session shared by phone and desktop shells.
  static WorkspaceSession get session => sessions.current;

  const WorkspaceShell({
    super.key,
    required this.workspaceSession,
    required this.workspaceOpen,
    required this.onWorkspaceOpenChanged,
    required this.currentChatId,
    required this.hasBoundWorkspace,
    required this.workspacePath,
    this.fileDirectory,
    required this.chatCore,
    required this.onListWorkspaceFiles,
    this.onMutateWorkspaceFile,
    required this.onListWorkspaceBindingDirectories,
    required this.onReadWorkspaceTextFile,
    required this.onReadWorkspaceFileBytes,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onCreateWorkspace,
    required this.onBindWorkspace,
    required this.onUnbindWorkspace,
    this.onFileQuote,
    required this.child,
  });

  final WorkspaceSession workspaceSession;
  final bool workspaceOpen;
  final ValueChanged<bool> onWorkspaceOpenChanged;
  final String? currentChatId;
  final bool hasBoundWorkspace;
  final String? workspacePath;
  final String? fileDirectory;
  final GeneratedChatRuntimeHolderMainCoreProxy chatCore;
  final Future<List<WorkspaceFileEntry>> Function(String path)
  onListWorkspaceFiles;
  final WorkspaceFileMutation? onMutateWorkspaceFile;
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
  final void Function(String fileName, String text)? onFileQuote;
  final Widget child;

  /// Opens resolved message paths even while the workspace panel is unmounted.
  Future<void> _openLinkedFile(String path, String language) async {
    final resolved = resolveMarkdownFilePath(
      path,
      fileDirectory,
      decodePath: false,
    );
    await workspaceSession.openFile(
      path: resolved,
      relativePath: resolved,
      readText: onReadWorkspaceTextFile,
      resolveOpener: () async => (await PluginWorkspacePresentation.forFile(
        clients: _clients,
        path: resolved,
        relativePath: resolved,
        language: language,
        chatId: currentChatId,
      )).firstOrNull,
    );
    workspaceSession.isOpen = true;
    if (identical(session, workspaceSession)) {
      onWorkspaceOpenChanged(true);
    }
  }

  /// Builds the workspace panel with plugins rendered as peer tabs.
  @override
  Widget build(BuildContext context) {
    final hasSidebarDropTarget =
        MediaQuery.sizeOf(context).width >= navigationTabletBreakpoint;
    final isOverlay = workspaceSidePanelLayout == SidePanelLayout.overlay;
    final dockController = SidebarDockScope.maybeOf(context);
    final panel = SafeArea(
      top: isOverlay,
      left: false,
      right: false,
      child: WorkspacePanel(
        key: ObjectKey(workspaceSession),
        session: workspaceSession,
        sidebarDockController: dockController,
        currentChatId: currentChatId,
        hasBoundWorkspace: hasBoundWorkspace,
        workspacePath: workspacePath,
        fileDirectory: fileDirectory,
        chatCore: chatCore,
        onListWorkspaceFiles: onListWorkspaceFiles,
        onMutateWorkspaceFile: onMutateWorkspaceFile,
        onListWorkspaceBindingDirectories: onListWorkspaceBindingDirectories,
        onReadWorkspaceTextFile: onReadWorkspaceTextFile,
        onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
        onWriteWorkspaceFileBytes: onWriteWorkspaceFileBytes,
        onOpenWorkspaceFile: onOpenWorkspaceFile,
        onCreateWorkspace: onCreateWorkspace,
        onBindWorkspace: onBindWorkspace,
        onUnbindWorkspace: onUnbindWorkspace,
        onFileQuote: onFileQuote,
        onCloseWorkspace: isOverlay
            ? () => onWorkspaceOpenChanged(false)
            : null,
      ),
    );
    return MainLayoutBackAction(
      enabled: workspaceOpen && isOverlay,
      onBack: () => onWorkspaceOpenChanged(false),
      child: MarkdownLinkScope(
        onOpenFile: (path) =>
            _openLinkedFile(path, Localizations.localeOf(context).languageCode),
        child: AdaptiveSidePanel(
          open: workspaceOpen,
          widePanelPadding: EdgeInsets.zero,
          onOpenChanged: onWorkspaceOpenChanged,
          layout: workspaceSidePanelLayout,
          defaultWidth: workspaceDefaultWidth,
          defaultWidthFraction: workspaceDefaultWidthFraction,
          minWidth: workspaceMinWidth,
          minContentWidth: workspaceMinChatWidth,
          resizeHandleHitWidth: workspaceResizeHandleHitWidth,
          resizeHandleVisualWidth: workspaceResizeHandleVisualWidth,
          resizeHandleHeight: workspaceResizeHandleHeight,
          closedDropTarget: hasSidebarDropTarget && dockController != null
              ? _WorkspacePluginDropTarget(
                  controller: dockController,
                  onAccepted: () => onWorkspaceOpenChanged(true),
                )
              : null,
          panel: isOverlay
              ? ColoredBox(
                  color: Theme.of(context).colorScheme.surface,
                  child: panel,
                )
              : panel,
          child: child,
        ),
      ),
    );
  }
}

class _WorkspacePluginDropTarget extends StatelessWidget {
  const _WorkspacePluginDropTarget({
    required this.controller,
    required this.onAccepted,
  });

  final SidebarDockController controller;
  final VoidCallback onAccepted;

  /// Builds the drop target that opens the workspace plugin tab area.
  @override
  Widget build(BuildContext context) {
    return DragTarget<SidebarDockDragPayload>(
      onWillAcceptWithDetails: (details) => controller.canMove(
        details.data.entryId,
        SidebarDockLocation.secondary,
      ),
      onAcceptWithDetails: (details) {
        controller.move(
          details.data.entryId,
          location: SidebarDockLocation.secondary,
          insertionIndex: controller.secondaryViews.length,
        );
        onAccepted();
      },
      builder: (context, candidateData, rejectedData) {
        return IgnorePointer(
          child: DecoratedBox(
            decoration: candidateData.isEmpty
                ? const BoxDecoration()
                : BoxDecoration(
                    border: Border.all(
                      color: Theme.of(context).colorScheme.primary,
                    ),
                  ),
            child: const SizedBox.expand(),
          ),
        );
      },
    );
  }
}
