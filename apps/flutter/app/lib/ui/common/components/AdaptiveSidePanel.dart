// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

/// Hosts a resizable trailing panel on wide layouts and an overlay panel on phones.
class AdaptiveSidePanel extends StatefulWidget {
  /// Creates a responsive trailing panel around the primary content.
  const AdaptiveSidePanel({
    super.key,
    required this.open,
    required this.onOpenChanged,
    required this.panel,
    required this.child,
    this.breakpoint = 600,
    this.defaultWidth = 360,
    this.minWidth = 280,
    this.minContentWidth = 320,
    this.resizeHandleHitWidth = 24,
    this.resizeHandleVisualWidth = 3,
    this.resizeHandleHeight = 56,
  });

  final bool open;
  final ValueChanged<bool> onOpenChanged;
  final Widget panel;
  final Widget child;
  final double breakpoint;
  final double defaultWidth;
  final double minWidth;
  final double minContentWidth;
  final double resizeHandleHitWidth;
  final double resizeHandleVisualWidth;
  final double resizeHandleHeight;

  /// Creates the state that tracks the panel width and drag interaction.
  @override
  State<AdaptiveSidePanel> createState() => _AdaptiveSidePanelState();
}

class _AdaptiveSidePanelState extends State<AdaptiveSidePanel> {
  double? _panelWidth;
  double? _dragStartGlobalX;
  double? _dragStartWidth;
  bool _resizing = false;

  /// Builds the responsive side-panel layout from the available constraints.
  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final useWideLayout = constraints.maxWidth >= widget.breakpoint;
        final maximumPanelWidth = useWideLayout
            ? math.max(0.0, constraints.maxWidth - widget.minContentWidth)
            : constraints.maxWidth;
        final minimumPanelWidth = useWideLayout
            ? math.min(widget.minWidth, maximumPanelWidth)
            : constraints.maxWidth;
        final panelWidth = _resolvePanelWidth(
          widget.defaultWidth,
          minimumPanelWidth,
          maximumPanelWidth,
        );
        if (useWideLayout) {
          return _buildWideLayout(
            panelWidth,
            minimumPanelWidth,
            maximumPanelWidth,
          );
        }
        return _buildPhoneLayout(panelWidth);
      },
    );
  }

  /// Resolves the persisted width into the current layout limits.
  double _resolvePanelWidth(
    double defaultWidth,
    double minimum,
    double maximum,
  ) {
    final rawWidth = _panelWidth ?? defaultWidth;
    return rawWidth.clamp(minimum, maximum).toDouble();
  }

  /// Builds the side-by-side layout used at and above the configured breakpoint.
  Widget _buildWideLayout(double width, double minimum, double maximum) {
    return Stack(
      clipBehavior: Clip.none,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(child: widget.child),
            AnimatedContainer(
              duration: _animationDuration,
              curve: Curves.easeOutCubic,
              width: widget.open ? width : 0,
            ),
          ],
        ),
        AnimatedPositionedDirectional(
          duration: _animationDuration,
          curve: Curves.easeOutCubic,
          top: 0,
          bottom: 0,
          end: widget.open ? 0 : -width,
          width: width,
          child: Stack(
            clipBehavior: Clip.none,
            children: <Widget>[
              Positioned.fill(child: widget.panel),
              if (widget.open)
                PositionedDirectional(
                  top: 0,
                  bottom: 0,
                  start: -widget.resizeHandleHitWidth / 2,
                  width: widget.resizeHandleHitWidth,
                  child: _AdaptiveSidePanelResizeHandle(
                    visualWidth: widget.resizeHandleVisualWidth,
                    height: widget.resizeHandleHeight,
                    onDragStart: (details) {
                      _startResize(details.globalPosition.dx, width);
                    },
                    onDragUpdate: (details) {
                      _updateWidthFromGlobalX(
                        details.globalPosition.dx,
                        minimum,
                        maximum,
                      );
                    },
                    onDragEnd: _endResize,
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }

  /// Builds the overlay layout used below the configured breakpoint.
  Widget _buildPhoneLayout(double width) {
    return Stack(
      clipBehavior: Clip.none,
      children: <Widget>[
        Positioned.fill(child: widget.child),
        if (widget.open)
          Positioned.fill(
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () => widget.onOpenChanged(false),
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: Colors.black.withValues(alpha: 0.18),
                ),
              ),
            ),
          ),
        AnimatedPositionedDirectional(
          duration: _animationDuration,
          curve: Curves.easeOutCubic,
          top: 0,
          bottom: 0,
          end: widget.open ? 0 : -width,
          width: width,
          child: widget.panel,
        ),
      ],
    );
  }

  /// Starts one drag-resize interaction from the current panel width.
  void _startResize(double globalX, double width) {
    setState(() {
      _resizing = true;
      _dragStartGlobalX = globalX;
      _dragStartWidth = width;
    });
  }

  /// Applies a width derived from the active drag position.
  void _updateWidthFromGlobalX(double globalX, double minimum, double maximum) {
    final dragStartGlobalX = _dragStartGlobalX;
    final dragStartWidth = _dragStartWidth;
    if (dragStartGlobalX == null || dragStartWidth == null) {
      return;
    }
    _updateWidth(
      dragStartWidth - (globalX - dragStartGlobalX),
      minimum,
      maximum,
    );
  }

  /// Stores a clamped panel width while a drag-resize interaction is active.
  void _updateWidth(double width, double minimum, double maximum) {
    setState(() {
      _panelWidth = width.clamp(minimum, maximum).toDouble();
    });
  }

  /// Completes the active drag-resize interaction.
  void _endResize(DragEndDetails details) {
    if (!_resizing) {
      return;
    }
    setState(() {
      _resizing = false;
      _dragStartGlobalX = null;
      _dragStartWidth = null;
    });
  }

  /// Returns the transition duration appropriate for the current resize state.
  Duration get _animationDuration =>
      _resizing ? Duration.zero : const Duration(milliseconds: 220);
}

class _AdaptiveSidePanelResizeHandle extends StatelessWidget {
  /// Creates the drag target displayed at the leading edge of a wide side panel.
  const _AdaptiveSidePanelResizeHandle({
    required this.visualWidth,
    required this.height,
    required this.onDragStart,
    required this.onDragUpdate,
    required this.onDragEnd,
  });

  final double visualWidth;
  final double height;
  final GestureDragStartCallback onDragStart;
  final GestureDragUpdateCallback onDragUpdate;
  final GestureDragEndCallback onDragEnd;

  /// Builds the panel resize gesture detector and its visible affordance.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return MouseRegion(
      cursor: SystemMouseCursors.resizeColumn,
      child: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onHorizontalDragStart: onDragStart,
        onHorizontalDragUpdate: onDragUpdate,
        onHorizontalDragEnd: onDragEnd,
        child: Center(
          child: Container(
            width: visualWidth,
            height: height,
            decoration: BoxDecoration(
              color: colorScheme.outlineVariant,
              borderRadius: BorderRadius.circular(2),
            ),
          ),
        ),
      ),
    );
  }
}
