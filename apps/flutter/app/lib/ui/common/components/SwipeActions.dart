// ignore_for_file: file_names

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

/// Requires an intentional, in-bounds swipe before invoking an action.
class SwipeActions extends StatefulWidget {
  /// Creates a horizontal swipe surface without velocity-based actions.
  const SwipeActions({
    super.key,
    this.onStartAction,
    this.onEndAction,
    this.enabled = true,
    this.actionThreshold = 0.40,
    required this.background,
    required this.secondaryBackground,
    required this.child,
  });

  final VoidCallback? onStartAction;
  final VoidCallback? onEndAction;
  final bool enabled;
  final double actionThreshold;
  final Widget background;
  final Widget secondaryBackground;
  final Widget child;

  /// Creates the swipe animation and gesture state.
  @override
  State<SwipeActions> createState() => _SwipeActionsState();
}

class _SwipeActionsState extends State<SwipeActions>
    with SingleTickerProviderStateMixin {
  late final AnimationController _offset;
  late Offset _origin;
  late double _width;
  bool _cancelled = false;

  /// Initializes the horizontal translation animation.
  @override
  void initState() {
    super.initState();
    _offset = AnimationController.unbounded(vsync: this);
  }

  /// Releases the animation ticker when the row is removed.
  @override
  void dispose() {
    _offset.dispose();
    super.dispose();
  }

  /// Measures the stationary row rather than its translated content.
  Rect _getBounds() {
    final box = context.findRenderObject()! as RenderBox;
    return box.localToGlobal(Offset.zero) & box.size;
  }

  /// Starts a new single-pointer swipe from the actual press position.
  void _onDown(DragDownDetails details) {
    _offset.stop();
    _offset.value = 0;
    _origin = details.globalPosition;
    _width = _getBounds().width;
    _cancelled = false;
  }

  /// Translates the row only after horizontal intent has been established.
  void _onUpdate(DragUpdateDetails details) {
    if (_cancelled) {
      return;
    }
    _offset.value = details.globalPosition.dx - _origin.dx;
  }

  /// Cancels the entire gesture when it loses arbitration or leaves the row.
  void _onCancel() {
    _cancelled = true;
    _settle();
  }

  /// Invokes an action by release distance, never by fling velocity.
  void _onEnd(DragEndDetails details) {
    final distance = _offset.value;
    final invokeAction =
        !_cancelled && distance.abs() >= _width * widget.actionThreshold;
    _settle();
    if (invokeAction) {
      final renameSide = Directionality.of(context) == TextDirection.ltr
          ? distance > 0
          : distance < 0;
      if (renameSide) {
        widget.onStartAction?.call();
      } else {
        widget.onEndAction?.call();
      }
    }
  }

  /// Animates the content to its resting position.
  void _settle() {
    _offset.animateTo(
      0,
      duration: const Duration(milliseconds: 160),
      curve: Curves.easeOut,
    );
  }

  /// Builds a stationary hit region around the translated conversation row.
  @override
  Widget build(BuildContext context) {
    if (!widget.enabled) return widget.child;
    final textDirection = Directionality.of(context);
    return RawGestureDetector(
      behavior: HitTestBehavior.opaque,
      gestures: <Type, GestureRecognizerFactory>{
        _SwipeRecognizer:
            GestureRecognizerFactoryWithHandlers<_SwipeRecognizer>(
              () => _SwipeRecognizer(
                debugOwner: this,
                getBounds: _getBounds,
                onLeaveBounds: _onCancel,
              ),
              (recognizer) {
                recognizer
                  ..gestureSettings = MediaQuery.gestureSettingsOf(context)
                  ..dragStartBehavior = DragStartBehavior.down
                  ..onlyAcceptDragOnThreshold = true
                  ..onDown = _onDown
                  ..onUpdate = _onUpdate
                  ..onEnd = _onEnd
                  ..onCancel = _onCancel;
              },
            ),
      },
      child: AnimatedBuilder(
        animation: _offset,
        child: widget.child,
        builder: (context, child) {
          final renameSide = textDirection == TextDirection.ltr
              ? _offset.value > 0
              : _offset.value < 0;
          return ClipRect(
            child: Stack(
              children: <Widget>[
                if (_offset.value != 0)
                  Positioned.fill(
                    child: renameSide
                        ? widget.background
                        : widget.secondaryBackground,
                  ),
                Transform.translate(
                  offset: Offset(_offset.value, 0),
                  child: child,
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}

/// Arbitrates deliberate horizontal pointer drags within the original row.
class _SwipeRecognizer extends HorizontalDragGestureRecognizer {
  /// Creates a bounded recognizer using Flutter's shared gesture arena.
  _SwipeRecognizer({
    super.debugOwner,
    required this.getBounds,
    required this.onLeaveBounds,
  });

  static const double _intentDistance = 32;
  static const double _horizontalDominance = 2;
  final Rect Function() getBounds;
  final VoidCallback onLeaveBounds;
  int? _pointer;
  late Offset _origin;
  late Rect _bounds;
  Offset _movement = Offset.zero;

  /// Limits action gestures to one pressed pointer at a time.
  @override
  bool isPointerAllowed(PointerEvent event) =>
      _pointer == null && super.isPointerAllowed(event);

  /// Keeps trackpad scrolling separate from pressed-pointer row actions.
  @override
  bool isPointerPanZoomAllowed(PointerPanZoomStartEvent event) => false;

  /// Captures the fixed row bounds before any content translation occurs.
  @override
  void addAllowedPointer(PointerDownEvent event) {
    _pointer = event.pointer;
    _origin = event.position;
    _bounds = getBounds();
    _movement = Offset.zero;
    super.addAllowedPointer(event);
  }

  /// Rejects out-of-bounds or predominantly vertical motion permanently.
  @override
  void handleEvent(PointerEvent event) {
    if (event.pointer != _pointer) {
      return;
    }
    if (event is PointerMoveEvent || event is PointerUpEvent) {
      _movement = event.position - _origin;
      final verticalIntent =
          _movement.dy.abs() >= kTouchSlop &&
          _movement.dx.abs() < _movement.dy.abs() * _horizontalDominance;
      if (!_bounds.contains(event.position) || verticalIntent) {
        onLeaveBounds();
        super.rejectGesture(event.pointer);
        return;
      }
    }
    if (event is PointerCancelEvent) {
      onLeaveBounds();
    }
    super.handleEvent(event);
  }

  /// Requires meaningful horizontal displacement before accepting the drag.
  @override
  bool hasSufficientGlobalDistanceToAccept(
    PointerDeviceKind pointerDeviceKind,
    double? deviceTouchSlop,
  ) =>
      _movement.dx.abs() >= _intentDistance &&
      _movement.dx.abs() >= _movement.dy.abs() * _horizontalDominance &&
      super.hasSufficientGlobalDistanceToAccept(
        pointerDeviceKind,
        deviceTouchSlop,
      );

  /// Clears pointer ownership when Flutter finishes the tracked gesture.
  @override
  void didStopTrackingLastPointer(int pointer) {
    super.didStopTrackingLastPointer(pointer);
    _pointer = null;
  }
}
