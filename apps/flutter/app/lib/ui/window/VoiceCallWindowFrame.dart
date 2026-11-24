// ignore_for_file: file_names
import 'dart:async';

import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/material.dart';

/// Changes only the host geometry; its child keeps the live call and camera.
class VoiceCallWindowFrame extends StatefulWidget {
  const VoiceCallWindowFrame({
    super.key,
    required this.launch,
    required this.builder,
  });
  final FloatingWindowLaunch launch;
  final Widget Function(bool fullscreen, Widget toolbar, Widget? resizeHandle)
  builder;

  @override
  State<VoiceCallWindowFrame> createState() => VoiceCallWindowFrameState();
}

class VoiceCallWindowFrameState extends State<VoiceCallWindowFrame> {
  late bool _fullscreen = widget.launch.fill;
  bool _busy = false;
  String? _error;
  FloatingWindowOptions _restore = const FloatingWindowOptions(
    x: 24,
    y: 48,
    width: 340,
    height: 460,
  );
  Future<void>? _resizing;
  Offset? _resizeAnchor;
  Offset? _resizePosition;
  bool _resizeReleased = false;
  int? _resizePointer;

  Future<void> setFullscreen(bool value) async {
    if (_busy || _resizing != null || value == _fullscreen) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      if (value) _restore = await FloatingWindows.currentBounds(widget.launch);
      await FloatingWindows.updateCurrent(
        widget.launch,
        value
            ? const FloatingWindowOptions(draggable: false)
            : _restore.merge(const FloatingWindowOptions(draggable: true)),
        fill: value,
      );
      if (mounted) setState(() => _fullscreen = value);
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  void _beginResize(PointerDownEvent event) {
    if (_busy || _fullscreen || _resizing != null) return;
    _resizePointer = event.pointer;
    _resizeAnchor = _resizePosition = event.position;
    _resizeReleased = false;
    // Stop the host's whole-window drag before this pointer crosses its threshold.
    _resizing = _resize();
  }

  Future<void> _resize() async {
    try {
      await FloatingWindows.updateCurrent(
        widget.launch,
        const FloatingWindowOptions(draggable: false),
      );
      final start = await FloatingWindows.currentBounds(widget.launch);
      Offset? applied;
      while (mounted) {
        final position = _resizePosition!;
        if (position != applied) {
          final delta = position - _resizeAnchor!;
          final bounds = FloatingWindowOptions(
            width: (start.width! + delta.dx).clamp(260.0, 4096.0),
            height: (start.height! + delta.dy).clamp(360.0, 4096.0),
          );
          await FloatingWindows.updateCurrent(widget.launch, bounds);
          applied = position;
        }
        if (_resizeReleased && applied == _resizePosition) break;
        await WidgetsBinding.instance.endOfFrame;
      }
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    } finally {
      try {
        if (mounted) {
          await FloatingWindows.updateCurrent(
            widget.launch,
            const FloatingWindowOptions(draggable: true),
          );
        }
      } catch (error) {
        if (mounted) setState(() => _error = error.toString());
      }
      _resizing = null;
      _resizePointer = null;
    }
  }

  @override
  Widget build(BuildContext context) {
    final zh = Localizations.localeOf(context).languageCode == 'zh';
    final toolbar = Material(
      color: Theme.of(context).colorScheme.surface.withValues(alpha: 0.45),
      borderRadius: BorderRadius.circular(24),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          IconButton(
            key: const ValueKey('call-toggle-fullscreen'),
            tooltip: _fullscreen
                ? (zh ? '还原小窗' : 'Restore window')
                : (zh ? '全屏' : 'Full screen'),
            onPressed: _busy
                ? null
                : () => unawaited(setFullscreen(!_fullscreen)),
            icon: Icon(_fullscreen ? Icons.fullscreen_exit : Icons.fullscreen),
          ),
          if (!_fullscreen)
            const Padding(
              padding: EdgeInsets.only(right: 10),
              child: Icon(Icons.drag_indicator, size: 18),
            ),
          if (_error != null)
            Tooltip(message: _error!, child: const Icon(Icons.error_outline)),
        ],
      ),
    );
    final handle = _fullscreen
        ? null
        : Listener(
            onPointerDown: _beginResize,
            onPointerMove: (event) {
              if (event.pointer == _resizePointer) {
                _resizePosition = event.position;
              }
            },
            onPointerUp: (event) {
              if (event.pointer == _resizePointer) _resizeReleased = true;
            },
            onPointerCancel: (event) {
              if (event.pointer == _resizePointer) _resizeReleased = true;
            },
            child: MouseRegion(
              cursor: SystemMouseCursors.resizeDownRight,
              child: GestureDetector(
                key: const ValueKey('call-resize-handle'),
                behavior: HitTestBehavior.opaque,
                onPanUpdate: (_) {},
                child: Tooltip(
                  message: zh ? '拖动调整大小' : 'Drag to resize',
                  child: const SizedBox(
                    width: 36,
                    height: 36,
                    child: Icon(Icons.open_in_full, size: 18),
                  ),
                ),
              ),
            ),
          );
    return widget.builder(_fullscreen, toolbar, handle);
  }
}
