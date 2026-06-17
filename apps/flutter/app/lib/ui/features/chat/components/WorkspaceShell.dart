// ignore_for_file: file_names

import 'dart:math' as math;
import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../viewmodel/WorkspaceFileModels.dart';
import 'workspace/WorkspaceLayoutMetrics.dart';
import 'workspace/WorkspacePanel.dart';
import 'workspace/WorkspaceResizeHandle.dart';

class WorkspaceShell extends StatefulWidget {
  const WorkspaceShell({
    super.key,
    required this.workspaceOpen,
    required this.onWorkspaceOpenChanged,
    required this.hasBoundWorkspace,
    required this.workspacePath,
    required this.onListWorkspaceFiles,
    required this.onReadWorkspaceTextFile,
    required this.onReadWorkspaceFileBytes,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenWorkspaceFile,
    required this.onCreateDefaultWorkspace,
    required this.onBindWorkspace,
    required this.child,
  });

  final bool workspaceOpen;
  final ValueChanged<bool> onWorkspaceOpenChanged;
  final bool hasBoundWorkspace;
  final String? workspacePath;
  final Future<List<WorkspaceFileEntry>> Function(String path)
  onListWorkspaceFiles;
  final Future<String> Function(String path) onReadWorkspaceTextFile;
  final Future<Uint8List> Function(String path) onReadWorkspaceFileBytes;
  final Future<void> Function(String path, Uint8List bytes)
  onWriteWorkspaceFileBytes;
  final Future<void> Function(String path) onOpenWorkspaceFile;
  final Future<void> Function(String? projectType) onCreateDefaultWorkspace;
  final Future<void> Function(String workspace) onBindWorkspace;
  final Widget child;

  @override
  State<WorkspaceShell> createState() => _WorkspaceShellState();
}

class _WorkspaceShellState extends State<WorkspaceShell> {
  final GlobalKey _workspacePanelKey = GlobalKey();
  double? _workspaceWidth;
  double? _workspaceDragStartGlobalX;
  double? _workspaceDragStartWidth;
  bool _resizingWorkspace = false;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final useTabletMode = constraints.maxWidth >= workspaceTabletBreakpoint;
        final maxWorkspaceWidth = useTabletMode
            ? math.max(0.0, constraints.maxWidth - workspaceMinTabletChatWidth)
            : constraints.maxWidth;
        final minWorkspaceWidth = useTabletMode
            ? math.min(workspaceMinWidth, maxWorkspaceWidth)
            : constraints.maxWidth;
        final defaultWidth = useTabletMode
            ? workspaceDefaultTabletWidth
            : constraints.maxWidth;
        final workspaceWidth = _resolveWorkspaceWidth(
          defaultWidth,
          minWorkspaceWidth,
          maxWorkspaceWidth,
        );

        if (useTabletMode) {
          return _buildTabletLayout(
            context,
            workspaceWidth,
            minWorkspaceWidth,
            maxWorkspaceWidth,
          );
        }

        return _buildPhoneLayout(context, workspaceWidth);
      },
    );
  }

  double _resolveWorkspaceWidth(
    double defaultWidth,
    double minWorkspaceWidth,
    double maxWorkspaceWidth,
  ) {
    final rawWidth = _workspaceWidth ?? defaultWidth;
    return rawWidth.clamp(minWorkspaceWidth, maxWorkspaceWidth).toDouble();
  }

  Widget _buildTabletLayout(
    BuildContext context,
    double workspaceWidth,
    double minWorkspaceWidth,
    double maxWorkspaceWidth,
  ) {
    return Stack(
      clipBehavior: Clip.none,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(child: widget.child),
            AnimatedContainer(
              duration: _workspaceAnimationDuration,
              curve: Curves.easeOutCubic,
              width: widget.workspaceOpen ? workspaceWidth : 0,
            ),
          ],
        ),
        AnimatedPositionedDirectional(
          duration: _workspaceAnimationDuration,
          curve: Curves.easeOutCubic,
          top: 0,
          bottom: 0,
          end: widget.workspaceOpen ? 0 : -workspaceWidth,
          width: workspaceWidth,
          child: Stack(
            clipBehavior: Clip.none,
            children: <Widget>[
              Positioned.fill(child: _buildWorkspacePanel()),
              if (widget.workspaceOpen)
                PositionedDirectional(
                  top: 0,
                  bottom: 0,
                  start: -workspaceResizeHandleHitWidth / 2,
                  width: workspaceResizeHandleHitWidth,
                  child: WorkspaceResizeHandle(
                    onDragStart: (details) {
                      _startWorkspaceResize(
                        details.globalPosition.dx,
                        workspaceWidth,
                      );
                    },
                    onDragUpdate: (details) {
                      _updateWorkspaceWidthFromGlobalX(
                        details.globalPosition.dx,
                        minWorkspaceWidth,
                        maxWorkspaceWidth,
                      );
                    },
                    onDragEnd: _endWorkspaceResize,
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildPhoneLayout(BuildContext context, double workspaceWidth) {
    return Stack(
      clipBehavior: Clip.none,
      children: <Widget>[
        Positioned.fill(child: widget.child),
        if (widget.workspaceOpen)
          Positioned.fill(
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () {
                widget.onWorkspaceOpenChanged(false);
              },
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: Colors.black.withValues(alpha: 0.18),
                ),
              ),
            ),
          ),
        AnimatedPositionedDirectional(
          duration: _workspaceAnimationDuration,
          curve: Curves.easeOutCubic,
          top: 0,
          bottom: 0,
          end: widget.workspaceOpen ? 0 : -workspaceWidth,
          width: workspaceWidth,
          child: _buildWorkspacePanel(),
        ),
      ],
    );
  }

  Widget _buildWorkspacePanel() {
    return WorkspacePanel(
      key: _workspacePanelKey,
      hasBoundWorkspace: widget.hasBoundWorkspace,
      workspacePath: widget.workspacePath,
      onListWorkspaceFiles: widget.onListWorkspaceFiles,
      onReadWorkspaceTextFile: widget.onReadWorkspaceTextFile,
      onReadWorkspaceFileBytes: widget.onReadWorkspaceFileBytes,
      onWriteWorkspaceFileBytes: widget.onWriteWorkspaceFileBytes,
      onOpenWorkspaceFile: widget.onOpenWorkspaceFile,
      onCreateDefaultWorkspace: widget.onCreateDefaultWorkspace,
      onBindWorkspace: widget.onBindWorkspace,
      onRevealRequested: () => widget.onWorkspaceOpenChanged(true),
    );
  }

  Duration get _workspaceAnimationDuration {
    return _resizingWorkspace
        ? Duration.zero
        : const Duration(milliseconds: 220);
  }

  void _startWorkspaceResize(double globalX, double workspaceWidth) {
    setState(() {
      _resizingWorkspace = true;
      _workspaceDragStartGlobalX = globalX;
      _workspaceDragStartWidth = workspaceWidth;
    });
  }

  void _endWorkspaceResize() {
    if (!_resizingWorkspace) {
      return;
    }
    setState(() {
      _resizingWorkspace = false;
      _workspaceDragStartGlobalX = null;
      _workspaceDragStartWidth = null;
    });
  }

  void _updateWorkspaceWidthFromGlobalX(
    double globalX,
    double minWorkspaceWidth,
    double maxWorkspaceWidth,
  ) {
    final dragStartGlobalX = _workspaceDragStartGlobalX;
    final dragStartWidth = _workspaceDragStartWidth;
    if (dragStartGlobalX == null || dragStartWidth == null) {
      return;
    }
    _updateWorkspaceWidth(
      dragStartWidth - (globalX - dragStartGlobalX),
      minWorkspaceWidth,
      maxWorkspaceWidth,
    );
  }

  void _updateWorkspaceWidth(
    double width,
    double minWorkspaceWidth,
    double maxWorkspaceWidth,
  ) {
    setState(() {
      _workspaceWidth = width
          .clamp(minWorkspaceWidth, maxWorkspaceWidth)
          .toDouble();
    });
  }
}
