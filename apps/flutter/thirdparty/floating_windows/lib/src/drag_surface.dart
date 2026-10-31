part of '../floating_windows.dart';

/// Keeps both anchors from pointer-down. Rebuilds and moving local coordinates
/// must never reset the origin or be accumulated as deltas.
class _ApplicationDragSurface extends StatefulWidget {
  const _ApplicationDragSurface({
    required this.enabled,
    required this.origin,
    required this.onMove,
    required this.child,
  });
  final bool enabled;
  final Offset origin;
  final ValueChanged<Offset> onMove;
  final Widget child;
  @override
  State<_ApplicationDragSurface> createState() =>
      _ApplicationDragSurfaceState();
}

class _ApplicationDragSurfaceState extends State<_ApplicationDragSurface> {
  Offset? _pointerDown;
  Offset? _windowDown;
  Offset? _latestPosition;
  int? _pointer;
  @override
  Widget build(BuildContext context) => Listener(
    onPointerDown: (event) {
      if (!widget.enabled || _pointer != null) return;
      _pointer = event.pointer;
      _pointerDown = event.position;
      _latestPosition = event.position;
      _windowDown = widget.origin;
    },
    onPointerMove: (event) {
      if (event.pointer == _pointer) _latestPosition = event.position;
    },
    onPointerUp: (event) {
      if (event.pointer == _pointer) _pointer = null;
    },
    onPointerCancel: (event) {
      if (event.pointer == _pointer) _pointer = null;
    },
    child: GestureDetector(
      behavior: HitTestBehavior.opaque,
      dragStartBehavior: DragStartBehavior.down,
      onPanStart: widget.enabled
          ? (details) {
              if (_pointerDown != null)
                widget.onMove(_windowDown! + _latestPosition! - _pointerDown!);
            }
          : null,
      onPanUpdate: widget.enabled
          ? (details) {
              if (_pointerDown != null)
                widget.onMove(_windowDown! + _latestPosition! - _pointerDown!);
            }
          : null,
      child: widget.child,
    ),
  );
}
