// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import 'RetainedPage.dart';

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
    this.defaultWidthFraction,
    this.minWidth = 280,
    this.minContentWidth = 320,
    this.resizeHandleHitWidth = 24,
    this.resizeHandleVisualWidth = 3,
    this.resizeHandleHeight = 56,
    this.closedDropTarget,
    this.widePanelPadding = const EdgeInsets.all(8),
    this.animate = true,
  });

  final bool open;
  final ValueChanged<bool> onOpenChanged;
  final Widget panel;
  final Widget child;
  final double breakpoint;
  final double defaultWidth;
  final double? defaultWidthFraction;
  final double minWidth;
  final double minContentWidth;
  final double resizeHandleHitWidth;
  final double resizeHandleVisualWidth;
  final double resizeHandleHeight;
  final Widget? closedDropTarget;

  /// Embedded panes can meet the host edges without adding a second frame.
  final EdgeInsetsGeometry widePanelPadding;
  final bool animate;

  /// Creates the state that tracks the panel width and drag interaction.
  @override
  State<AdaptiveSidePanel> createState() => _AdaptiveSidePanelState();
}

class _AdaptiveSidePanelState extends State<AdaptiveSidePanel> {
  double? _panelWidth;
  bool _resizing = false;
  bool _closing = false;

  /// Keeps the panel live only until its visible closing transition completes.
  @override
  void didUpdateWidget(covariant AdaptiveSidePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.open) {
      _closing = false;
    } else if (oldWidget.open) {
      _closing = _animationDuration > Duration.zero;
    }
  }

  /// Suspends the retained panel after it leaves the visible viewport.
  void _finishPanelTransition() {
    if (_closing && !widget.open) {
      setState(() => _closing = false);
    }
  }

  /// Changes only geometry when the available width crosses the breakpoint.
  @override
  Widget build(BuildContext context) {
    // Retain the content widget across LayoutBuilder's constraint-only frames.
    final content = widget.child;
    return LayoutBuilder(
      builder: (context, constraints) {
        final useWideLayout = constraints.maxWidth >= widget.breakpoint;
        final maximumPanelWidth = useWideLayout
            ? math.max(0.0, constraints.maxWidth - widget.minContentWidth)
            : constraints.maxWidth;
        final minimumPanelWidth = useWideLayout
            ? math.min(widget.minWidth, maximumPanelWidth)
            : constraints.maxWidth;
        final targetDefaultWidth =
            widget.defaultWidthFraction != null && useWideLayout
            ? math.max(
                widget.defaultWidth,
                constraints.maxWidth * widget.defaultWidthFraction!,
              )
            : widget.defaultWidth;
        final panelWidth = _resolvePanelWidth(
          targetDefaultWidth,
          minimumPanelWidth,
          maximumPanelWidth,
        );
        return _buildLayout(
          content: content,
          useWideLayout: useWideLayout,
          width: panelWidth,
          minimum: minimumPanelWidth,
          maximum: maximumPanelWidth,
        );
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

  /// Uses the same ancestry for wide and overlay layouts so neither the chat
  /// nor the panel is unmounted by a navigation sidebar width animation.
  Widget _buildLayout({
    required Widget content,
    required bool useWideLayout,
    required double width,
    required double minimum,
    required double maximum,
  }) {
    return Stack(
      clipBehavior: Clip.none,
      children: <Widget>[
        Row(
          // Positioned chat bodies must fill the viewport, not shrink to
          // their non-positioned overlays and then be vertically centered.
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            // Primary menus must use this pane's bounds, not the full window.
            Expanded(child: Overlay.wrap(child: content)),
            AnimatedContainer(
              duration: _animationDuration,
              curve: Curves.easeOutCubic,
              width: useWideLayout && widget.open ? width : 0,
            ),
          ],
        ),
        if (!useWideLayout && widget.open)
          Positioned.fill(
            key: const ValueKey<String>('sidePanelDismissBarrier'),
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onTap: () => widget.onOpenChanged(false),
              child: const ColoredBox(color: Colors.transparent),
            ),
          ),
        AnimatedPositionedDirectional(
          // The dismiss barrier must not shift the retained panel's Stack slot.
          key: const ValueKey<String>('sidePanelLayer'),
          onEnd: _finishPanelTransition,
          duration: _animationDuration,
          curve: Curves.easeOutCubic,
          top: 0,
          bottom: 0,
          end: widget.open ? 0 : -width,
          width: width,
          child: Stack(
            clipBehavior: Clip.none,
            children: <Widget>[
              Positioned.fill(
                child: RetainedPage(
                  active: widget.open || _closing,
                  child: useWideLayout
                      ? Padding(
                          padding: widget.widePanelPadding,
                          child: widget.panel,
                        )
                      : widget.panel,
                ),
              ),
              if (useWideLayout && widget.open)
                PositionedDirectional(
                  top: 0,
                  bottom: 0,
                  start: -4,
                  width: 16,
                  child: _AdaptiveSidePanelResizeHandle(
                    onDragStart: (_) {
                      _startResize(width);
                    },
                    onDragUpdate: (details) {
                      _updateWidthFromDelta(
                        -details.delta.dx,
                        minimum,
                        maximum,
                      );
                    },
                    onDragEnd: _endResize,
                  ),
                ),
              if (useWideLayout &&
                  !widget.open &&
                  widget.closedDropTarget != null)
                PositionedDirectional(
                  top: 0,
                  bottom: 0,
                  end: 0,
                  width: widget.resizeHandleHitWidth * 2,
                  child: widget.closedDropTarget!,
                ),
            ],
          ),
        ),
      ],
    );
  }

  /// Starts one drag-resize interaction from the current panel width.
  void _startResize(double width) {
    setState(() {
      _resizing = true;
      _panelWidth = width;
    });
  }

  /// Applies a local drag delta so resizing follows the zoomed viewport.
  void _updateWidthFromDelta(double delta, double minimum, double maximum) {
    _updateWidth(_panelWidth! + delta, minimum, maximum);
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
    });
  }

  /// Returns the transition duration appropriate for the current resize state.
  Duration get _animationDuration => _resizing || !widget.animate
      ? Duration.zero
      : const Duration(milliseconds: 220);
}

class _AdaptiveSidePanelResizeHandle extends StatefulWidget {
  const _AdaptiveSidePanelResizeHandle({
    required this.onDragStart,
    required this.onDragUpdate,
    required this.onDragEnd,
  });

  final GestureDragStartCallback onDragStart;
  final GestureDragUpdateCallback onDragUpdate;
  final GestureDragEndCallback onDragEnd;

  @override
  State<_AdaptiveSidePanelResizeHandle> createState() =>
      _AdaptiveSidePanelResizeHandleState();
}

class _AdaptiveSidePanelResizeHandleState
    extends State<_AdaptiveSidePanelResizeHandle> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return MouseRegion(
      cursor: SystemMouseCursors.resizeColumn,
      onEnter: (_) => setState(() => _hovered = true),
      onExit: (_) => setState(() => _hovered = false),
      child: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onHorizontalDragStart: widget.onDragStart,
        onHorizontalDragUpdate: widget.onDragUpdate,
        onHorizontalDragEnd: widget.onDragEnd,
        child: Center(
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 150),
            width: _hovered ? 2.5 : 0.0,
            height: double.infinity,
            decoration: BoxDecoration(
              color: colorScheme.primary.withValues(alpha: 0.65),
              borderRadius: BorderRadius.circular(2),
            ),
          ),
        ),
      ),
    );
  }
}
