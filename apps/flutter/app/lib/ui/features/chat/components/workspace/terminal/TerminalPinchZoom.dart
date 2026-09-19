import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';

/// Resizes terminal text without taking single-finger scrolling gestures.
class TerminalPinchZoom extends StatefulWidget {
  const TerminalPinchZoom({super.key, required this.builder});

  final Widget Function(BuildContext context, double scale) builder;

  @override
  State<TerminalPinchZoom> createState() => _TerminalPinchZoomState();
}

class _TerminalPinchZoomState extends State<TerminalPinchZoom> {
  final Map<int, Offset> _touches = {};
  double _scale = 1;
  double _startScale = 1;
  double? _startDistance;

  double get _distance {
    final points = _touches.values.take(2).toList();
    return (points[0] - points[1]).distance;
  }

  void _down(PointerDownEvent event) {
    if (event.kind != PointerDeviceKind.touch) return;
    _touches[event.pointer] = event.localPosition;
    _resetBaseline();
  }

  void _resetBaseline() {
    _startScale = _scale;
    _startDistance = _touches.length == 2 ? _distance : null;
  }

  void _move(PointerMoveEvent event) {
    if (!_touches.containsKey(event.pointer)) return;
    _touches[event.pointer] = event.localPosition;
    final distance = _startDistance;
    if (_touches.length != 2 || distance == null || distance < 1) return;
    final scale = (_startScale * _distance / distance).clamp(0.6, 2.5);
    if ((scale - _scale).abs() < 0.005) return;
    setState(() => _scale = scale);
  }

  void _up(PointerEvent event) {
    _touches.remove(event.pointer);
    _resetBaseline();
  }

  @override
  Widget build(BuildContext context) => Listener(
    behavior: HitTestBehavior.opaque,
    onPointerDown: _down,
    onPointerMove: _move,
    onPointerUp: _up,
    onPointerCancel: _up,
    child: widget.builder(context, _scale),
  );
}
