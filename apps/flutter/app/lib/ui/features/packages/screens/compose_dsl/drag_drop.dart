// ignore_for_file: file_names
part of '../ToolPkgUiLauncherScreen.dart';

/// A serializable, arbitrary plugin payload. No host business models are involved.
class _ComposeDragPayload {
  const _ComposeDragPayload(this.type, this.data);
  final String type;
  final Object? data;
}

class _ComposeDraggable extends StatelessWidget {
  const _ComposeDraggable({
    required this.props,
    required this.onAction,
    required this.child,
    this.feedback,
    this.childWhenDragging,
  });
  final Map<String, Object?> props;
  final ComposeDslWebViewActionDispatcher onAction;
  final Widget child;
  final Widget? feedback;
  final Widget? childWhenDragging;

  void _emit(String event, [Object? payload]) {
    final id = _actionId(props[event]);
    if (id != null) unawaited(onAction(id, payload));
  }

  @override
  Widget build(BuildContext context) {
    final data = _ComposeDragPayload(_string(props['dragType']), props['data']);
    final axis = switch (_normalizeToken(_string(props['axis']))) {
      'horizontal' => Axis.horizontal,
      'vertical' => Axis.vertical,
      _ => null,
    };
    final max = props['enabled'] == false
        ? 0
        : (_number(props['maxSimultaneousDrags'])?.toInt() ?? 1);
    final dragFeedback = Material(
      color: Colors.transparent,
      // Overlay feedback has unbounded constraints. Reusing an unconstrained
      // fill-width Row/Expanded child would otherwise throw during dragging.
      child:
          feedback ??
          Builder(
            builder: (_) {
              final box = context.findRenderObject();
              final size = box is RenderBox && box.hasSize ? box.size : null;
              return SizedBox(
                width: size?.width,
                height: size?.height,
                child: child,
              );
            },
          ),
    );
    final dragging = childWhenDragging ?? Opacity(opacity: 0.35, child: child);
    void ended(DraggableDetails details) => _emit('onDragEnd', {
      'data': data.data,
      'dragType': data.type,
      'wasAccepted': details.wasAccepted,
      'x': details.offset.dx,
      'y': details.offset.dy,
      'velocityX': details.velocity.pixelsPerSecond.dx,
      'velocityY': details.velocity.pixelsPerSecond.dy,
    });
    if (props['longPress'] == true) {
      return LongPressDraggable<_ComposeDragPayload>(
        data: data,
        axis: axis,
        maxSimultaneousDrags: max,
        delay: Duration(
          milliseconds: _number(props['delayMillis'])?.toInt() ?? 500,
        ),
        dragAnchorStrategy: pointerDragAnchorStrategy,
        feedback: dragFeedback,
        childWhenDragging: dragging,
        onDragStarted: () => _emit('onDragStarted', data.data),
        onDragEnd: ended,
        child: child,
      );
    }
    return Draggable<_ComposeDragPayload>(
      data: data,
      axis: axis,
      maxSimultaneousDrags: max,
      dragAnchorStrategy: pointerDragAnchorStrategy,
      feedback: dragFeedback,
      childWhenDragging: dragging,
      onDragStarted: () => _emit('onDragStarted', data.data),
      onDragEnd: ended,
      child: child,
    );
  }
}

class _ComposeDragTarget extends StatelessWidget {
  const _ComposeDragTarget({
    required this.props,
    required this.onAction,
    required this.child,
  });
  final Map<String, Object?> props;
  final ComposeDslWebViewActionDispatcher onAction;
  final Widget child;

  bool _accepts(_ComposeDragPayload payload) {
    if (props['enabled'] == false) return false;
    final types = props['acceptedTypes'];
    if (types is List && !types.contains(payload.type)) return false;
    final values = props['acceptedData'];
    return values is! List ||
        values.any((value) => _composeInputEquals(value, payload.data));
  }

  @override
  Widget build(BuildContext context) => DragTarget<_ComposeDragPayload>(
    onWillAcceptWithDetails: (details) => _accepts(details.data),
    onAcceptWithDetails: (details) {
      final id = _actionId(props['onDrop']);
      if (id != null) unawaited(onAction(id, details.data.data));
    },
    onLeave: (data) {
      final id = _actionId(props['onLeave']);
      if (id != null && data != null) unawaited(onAction(id, data.data));
    },
    builder: (context, candidate, rejected) => DecoratedBox(
      decoration: BoxDecoration(
        color: candidate.isEmpty
            ? null
            : _color(context, props['hoverBackground']),
        borderRadius: _borderRadius(props['shape']) ?? BorderRadius.zero,
        border: candidate.isEmpty || props['hoverBorderColor'] == null
            ? null
            : Border.all(
                color:
                    _color(context, props['hoverBorderColor']) ??
                    Colors.transparent,
              ),
      ),
      child: child,
    ),
  );
}
