// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../../common/components/AdaptiveSidePanel.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../main/layout/SidebarDockController.dart';
import '../../../main/layout/NavigationLayoutMetrics.dart';
import '../viewmodel/WorkspaceFileModels.dart';
import 'workspace/WorkspaceLayoutMetrics.dart';
import 'workspace/WorkspacePanel.dart';
import 'workspace/WorkspaceSession.dart';

class WorkspaceShell extends StatelessWidget {
  static final WorkspaceSession _session = WorkspaceSession();

  /// Returns the workspace session shared by phone and desktop shells.
  static WorkspaceSession get session => _session;

  const WorkspaceShell({
    super.key,
    required this.workspaceOpen,
    required this.onWorkspaceOpenChanged,
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
    required this.child,
  });

  final bool workspaceOpen;
  final ValueChanged<bool> onWorkspaceOpenChanged;
  final String? currentChatId;
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
  final Widget child;

  /// Builds the workspace panel with plugins rendered as peer tabs.
  @override
  Widget build(BuildContext context) {
    final dockController =
        MediaQuery.sizeOf(context).width >= navigationTabletBreakpoint
        ? SidebarDockScope.maybeOf(context)
        : null;
    return AdaptiveSidePanel(
      open: workspaceOpen,
      onOpenChanged: onWorkspaceOpenChanged,
      breakpoint: workspaceTabletBreakpoint,
      defaultWidth: workspaceDefaultTabletWidth,
      defaultWidthFraction: workspaceDefaultWidthFraction,
      minWidth: workspaceMinWidth,
      minContentWidth: workspaceMinTabletChatWidth,
      resizeHandleHitWidth: workspaceResizeHandleHitWidth,
      resizeHandleVisualWidth: workspaceResizeHandleVisualWidth,
      resizeHandleHeight: workspaceResizeHandleHeight,
      closedDropTarget: _WorkspacePluginDropTarget(
        controller: dockController,
        onAccepted: () => onWorkspaceOpenChanged(true),
      ),
      panel: SafeArea(
        top: false,
        left: false,
        right: false,
        child: WorkspacePanel(
          session: _session,
          sidebarDockController: dockController,
          currentChatId: currentChatId,
          hasBoundWorkspace: hasBoundWorkspace,
          workspacePath: workspacePath,
          chatCore: chatCore,
          onListWorkspaceFiles: onListWorkspaceFiles,
          onListWorkspaceBindingDirectories: onListWorkspaceBindingDirectories,
          onReadWorkspaceTextFile: onReadWorkspaceTextFile,
          onReadWorkspaceFileBytes: onReadWorkspaceFileBytes,
          onWriteWorkspaceFileBytes: onWriteWorkspaceFileBytes,
          onOpenWorkspaceFile: onOpenWorkspaceFile,
          onCreateWorkspace: onCreateWorkspace,
          onBindWorkspace: onBindWorkspace,
          onUnbindWorkspace: onUnbindWorkspace,
        ),
      ),
      child: child,
    );
  }
}

class _WorkspacePluginDropTarget extends StatelessWidget {
  const _WorkspacePluginDropTarget({
    required this.controller,
    required this.onAccepted,
  });

  final SidebarDockController? controller;
  final VoidCallback onAccepted;

  /// Builds the drop target that opens the workspace plugin tab area.
  @override
  Widget build(BuildContext context) {
    final dockController = controller;
    if (dockController == null) {
      return const SizedBox.expand();
    }
    return DragTarget<SidebarDockDragPayload>(
      onWillAcceptWithDetails: (details) => dockController.canMove(
        details.data.entryId,
        SidebarDockLocation.secondary,
      ),
      onAcceptWithDetails: (details) {
        dockController.move(
          details.data.entryId,
          location: SidebarDockLocation.secondary,
          insertionIndex: dockController.secondaryViews.length,
        );
        onAccepted();
      },
      builder: (context, candidateData, rejectedData) {
        return DecoratedBox(
          decoration: candidateData.isEmpty
              ? const BoxDecoration()
              : BoxDecoration(
                  border: Border.all(
                    color: Theme.of(context).colorScheme.primary,
                  ),
                ),
          child: const SizedBox.expand(),
        );
      },
    );
  }
}
