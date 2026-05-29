// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'WorkspaceLayoutMetrics.dart';

class WorkspaceResizeHandle extends StatefulWidget {
  const WorkspaceResizeHandle({
    super.key,
    required this.onDragStart,
    required this.onDragUpdate,
    this.onDragEnd,
  });

  final GestureDragStartCallback onDragStart;
  final GestureDragUpdateCallback onDragUpdate;
  final VoidCallback? onDragEnd;

  @override
  State<WorkspaceResizeHandle> createState() => _WorkspaceResizeHandleState();
}

class _WorkspaceResizeHandleState extends State<WorkspaceResizeHandle> {
  bool _hovered = false;
  bool _pressed = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final active = _hovered || _pressed;
    final handleColor = active
        ? theme.colorScheme.primary
        : theme.colorScheme.outlineVariant;
    final trackColor = active
        ? theme.colorScheme.primary.withValues(alpha: 0.08)
        : Colors.transparent;
    return MouseRegion(
      cursor: SystemMouseCursors.resizeColumn,
      onEnter: (_) {
        setState(() {
          _hovered = true;
        });
      },
      onExit: (_) {
        setState(() {
          _hovered = false;
        });
      },
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onHorizontalDragStart: (details) {
          setState(() {
            _pressed = true;
          });
          widget.onDragStart(details);
        },
        onHorizontalDragUpdate: widget.onDragUpdate,
        onHorizontalDragEnd: (_) {
          setState(() {
            _pressed = false;
          });
          widget.onDragEnd?.call();
        },
        onHorizontalDragCancel: () {
          setState(() {
            _pressed = false;
          });
          widget.onDragEnd?.call();
        },
        onLongPressStart: (_) {
          setState(() {
            _pressed = true;
          });
        },
        onLongPressEnd: (_) {
          setState(() {
            _pressed = false;
          });
        },
        child: SizedBox(
          width: workspaceResizeHandleHitWidth,
          height: double.infinity,
          child: Center(
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 120),
              width: workspaceResizeHandleTrackWidth,
              color: trackColor,
              child: Center(
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 120),
                  width: workspaceResizeHandleVisualWidth,
                  height: workspaceResizeHandleHeight,
                  decoration: BoxDecoration(
                    color: handleColor,
                    borderRadius: BorderRadius.circular(
                      workspaceResizeHandleVisualWidth,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
