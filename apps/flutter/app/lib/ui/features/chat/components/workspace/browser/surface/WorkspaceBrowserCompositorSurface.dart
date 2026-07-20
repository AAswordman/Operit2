// ignore_for_file: file_names

import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../WorkspaceBrowserViewStore.dart';
import '../tabs/WorkspaceBrowserTabModels.dart';

class WorkspaceBrowserCompositorSurface extends StatefulWidget {
  /// Creates a workspace view over the owner WebView compositor texture.
  const WorkspaceBrowserCompositorSurface({
    super.key,
    required this.tab,
    required this.store,
  });

  final WorkspaceBrowserTabState tab;
  final WorkspaceBrowserViewStore store;

  /// Creates the mutable state for the shared compositor surface.
  @override
  State<WorkspaceBrowserCompositorSurface> createState() =>
      _WorkspaceBrowserCompositorSurfaceState();
}

class _WorkspaceBrowserCompositorSurfaceState
    extends State<WorkspaceBrowserCompositorSurface> {
  final FocusNode _focusNode = FocusNode();
  Size? _reportedSize;
  double? _reportedScaleFactor;

  /// Releases local resources when the widget is destroyed.
  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  /// Builds the shared texture and forwards pointer input to the owner.
  @override
  Widget build(BuildContext context) {
    final errorText = widget.tab.errorText;
    if (errorText != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Text(errorText, textAlign: TextAlign.center),
        ),
      );
    }
    final descriptor = widget.tab.surfaceDescriptor;
    if (descriptor == null) {
      return const Center(child: CircularProgressIndicator());
    }
    return LayoutBuilder(
      builder: (context, constraints) {
        final size = Size(constraints.maxWidth, constraints.maxHeight);
        final scaleFactor = View.of(context).devicePixelRatio;
        _scheduleResizeReport(size, scaleFactor);
        return KeyboardListener(
          focusNode: _focusNode,
          onKeyEvent: _handleKeyEvent,
          child: Listener(
            behavior: HitTestBehavior.opaque,
            onPointerMove: (event) => _sendPointerEvent('move', event),
            onPointerDown: (event) {
              _focusNode.requestFocus();
              _sendPointerEvent('down', event);
            },
            onPointerUp: (event) => _sendPointerEvent('up', event),
            onPointerCancel: (event) => _sendPointerEvent('cancel', event),
            onPointerSignal: _handlePointerSignal,
            onPointerPanZoomUpdate: _handlePointerPanZoomUpdate,
            child: MouseRegion(
              cursor: SystemMouseCursors.basic,
              child: _buildSurfaceContent(descriptor),
            ),
          ),
        );
      },
    );
  }

  /// Builds the compositor content for the negotiated transport.
  Widget _buildSurfaceContent(WorkspaceBrowserSurfaceDescriptor descriptor) {
    switch (descriptor.transport) {
      case 'localTexture':
        final textureId = descriptor.textureId;
        if (descriptor.platform != 'windows' || textureId == null) {
          throw StateError('Invalid local browser surface descriptor');
        }
        return Texture(textureId: textureId, filterQuality: FilterQuality.none);
      case 'encodedStream':
        final streamId = descriptor.streamId;
        if (streamId == null || streamId.isEmpty) {
          throw StateError('Invalid encoded browser surface descriptor');
        }
        final frame = widget.tab.surfaceFrame;
        if (frame == null) {
          return const ColoredBox(color: Colors.black);
        }
        return _RawBrowserSurfaceFrame(frame: frame);
      default:
        throw StateError('Invalid browser surface transport');
    }
  }

  /// Schedules a compositor resize after Flutter finishes layout.
  void _scheduleResizeReport(Size size, double scaleFactor) {
    if (_reportedSize == size && _reportedScaleFactor == scaleFactor) {
      return;
    }
    _reportedSize = size;
    _reportedScaleFactor = scaleFactor;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      unawaited(widget.store.resizeSurface(widget.tab.id, size, scaleFactor));
    });
  }

  /// Sends one raw Flutter pointer event to the owner compositor.
  void _sendPointerEvent(String eventType, PointerEvent event) {
    widget.store.pointerSurface(
      widget.tab.id,
      eventType: eventType,
      pointer: event.pointer,
      position: event.localPosition,
      buttons: event.buttons,
    );
  }

  /// Registers browser wheel input with Flutter's pointer signal resolver.
  void _handlePointerSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent) {
      GestureBinding.instance.pointerSignalResolver.register(
        event,
        _handleResolvedPointerScroll,
      );
    }
  }

  /// Sends a resolved mouse wheel event to the owner compositor.
  void _handleResolvedPointerScroll(PointerSignalEvent event) {
    final scrollEvent = event as PointerScrollEvent;
    _sendScrollDelta(-scrollEvent.scrollDelta.dx, -scrollEvent.scrollDelta.dy);
  }

  /// Sends the dominant touchpad pan axis to the owner compositor.
  void _handlePointerPanZoomUpdate(PointerPanZoomUpdateEvent event) {
    final delta = event.panDelta;
    if (delta.dx.abs() > delta.dy.abs()) {
      _sendScrollDelta(-delta.dx, 0);
      return;
    }
    _sendScrollDelta(0, delta.dy);
  }

  /// Sends one normalized scroll delta to the owner compositor.
  void _sendScrollDelta(double dx, double dy) {
    widget.store.scrollSurface(widget.tab.id, dx, dy);
  }

  /// Forwards one Flutter key event to the owner browser surface.
  void _handleKeyEvent(KeyEvent event) {
    widget.store.keySurface(widget.tab.id, _browserKeyPayload(event));
  }

  /// Builds one Chrome DevTools key payload from a Flutter key event.
  Map<String, Object?> _browserKeyPayload(KeyEvent event) {
    final isUp = event is KeyUpEvent;
    final character = isUp ? null : event.character;
    return <String, Object?>{
      'keyEventType': isUp ? 'keyUp' : 'keyDown',
      'key': _browserKey(event.logicalKey, character),
      'code': _browserCode(event.physicalKey),
      'windowsVirtualKeyCode': _windowsVirtualKeyCode(event.logicalKey),
      'nativeVirtualKeyCode': _windowsVirtualKeyCode(event.logicalKey),
      'modifiers': _keyboardModifiers(),
      'text': character,
      'unmodifiedText': character,
    };
  }

  /// Returns the browser key name for a logical key.
  String _browserKey(LogicalKeyboardKey key, String? character) {
    final named = _browserKeyNames[key];
    if (named != null) {
      return named;
    }
    if (character != null && character.length == 1) {
      return character;
    }
    return key.keyLabel;
  }

  /// Returns the browser physical code for a physical key.
  String _browserCode(PhysicalKeyboardKey key) {
    final usage = key.usbHidUsage;
    if (usage >= 0x00070004 && usage <= 0x0007001d) {
      final letter = String.fromCharCode(
        'A'.codeUnitAt(0) + usage - 0x00070004,
      );
      return 'Key$letter';
    }
    if (usage >= 0x0007001e && usage <= 0x00070026) {
      return 'Digit${usage - 0x0007001d}';
    }
    if (usage == 0x00070027) {
      return 'Digit0';
    }
    return _browserCodes[usage] ?? key.debugName ?? '';
  }

  /// Returns the Windows virtual-key code for a logical key.
  int _windowsVirtualKeyCode(LogicalKeyboardKey key) {
    final mapped = _windowsVirtualKeyCodes[key];
    if (mapped != null) {
      return mapped;
    }
    final label = key.keyLabel;
    if (label.length == 1) {
      return label.toUpperCase().codeUnitAt(0);
    }
    return 0;
  }

  /// Returns the Chrome modifier bitmask for currently pressed modifiers.
  int _keyboardModifiers() {
    var modifiers = 0;
    final keyboard = HardwareKeyboard.instance;
    if (keyboard.isAltPressed) {
      modifiers |= 1;
    }
    if (keyboard.isControlPressed) {
      modifiers |= 2;
    }
    if (keyboard.isMetaPressed) {
      modifiers |= 4;
    }
    if (keyboard.isShiftPressed) {
      modifiers |= 8;
    }
    return modifiers;
  }

  static final Map<LogicalKeyboardKey, String> _browserKeyNames =
      <LogicalKeyboardKey, String>{
        LogicalKeyboardKey.enter: 'Enter',
        LogicalKeyboardKey.tab: 'Tab',
        LogicalKeyboardKey.backspace: 'Backspace',
        LogicalKeyboardKey.delete: 'Delete',
        LogicalKeyboardKey.escape: 'Escape',
        LogicalKeyboardKey.arrowLeft: 'ArrowLeft',
        LogicalKeyboardKey.arrowRight: 'ArrowRight',
        LogicalKeyboardKey.arrowUp: 'ArrowUp',
        LogicalKeyboardKey.arrowDown: 'ArrowDown',
        LogicalKeyboardKey.home: 'Home',
        LogicalKeyboardKey.end: 'End',
        LogicalKeyboardKey.pageUp: 'PageUp',
        LogicalKeyboardKey.pageDown: 'PageDown',
        LogicalKeyboardKey.space: ' ',
      };

  static const Map<int, String> _browserCodes = <int, String>{
    0x00070028: 'Enter',
    0x00070029: 'Escape',
    0x0007002a: 'Backspace',
    0x0007002b: 'Tab',
    0x0007002c: 'Space',
    0x0007004a: 'Home',
    0x0007004b: 'PageUp',
    0x0007004c: 'Delete',
    0x0007004d: 'End',
    0x0007004e: 'PageDown',
    0x0007004f: 'ArrowRight',
    0x00070050: 'ArrowLeft',
    0x00070051: 'ArrowDown',
    0x00070052: 'ArrowUp',
  };

  static final Map<LogicalKeyboardKey, int> _windowsVirtualKeyCodes =
      <LogicalKeyboardKey, int>{
        LogicalKeyboardKey.backspace: 0x08,
        LogicalKeyboardKey.tab: 0x09,
        LogicalKeyboardKey.enter: 0x0d,
        LogicalKeyboardKey.escape: 0x1b,
        LogicalKeyboardKey.space: 0x20,
        LogicalKeyboardKey.pageUp: 0x21,
        LogicalKeyboardKey.pageDown: 0x22,
        LogicalKeyboardKey.end: 0x23,
        LogicalKeyboardKey.home: 0x24,
        LogicalKeyboardKey.arrowLeft: 0x25,
        LogicalKeyboardKey.arrowUp: 0x26,
        LogicalKeyboardKey.arrowRight: 0x27,
        LogicalKeyboardKey.arrowDown: 0x28,
        LogicalKeyboardKey.delete: 0x2e,
      };
}

class _RawBrowserSurfaceFrame extends StatefulWidget {
  /// Creates a widget that paints a raw browser compositor frame.
  const _RawBrowserSurfaceFrame({required this.frame});

  final WorkspaceBrowserSurfaceFrame frame;

  /// Creates the raw frame painter state.
  @override
  State<_RawBrowserSurfaceFrame> createState() =>
      _RawBrowserSurfaceFrameState();
}

class _RawBrowserSurfaceFrameState extends State<_RawBrowserSurfaceFrame> {
  ui.Image? _image;
  WorkspaceBrowserSurfaceFrame? _decodedFrame;

  /// Decodes the first raw frame after mount.
  @override
  void initState() {
    super.initState();
    _decodeFrame();
  }

  /// Decodes a replacement raw frame when Link delivers a new one.
  @override
  void didUpdateWidget(_RawBrowserSurfaceFrame oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(widget.frame, oldWidget.frame)) {
      _decodeFrame();
    }
  }

  /// Releases decoded image resources.
  @override
  void dispose() {
    _image?.dispose();
    super.dispose();
  }

  /// Builds the decoded image surface.
  @override
  Widget build(BuildContext context) {
    final image = _image;
    if (image == null) {
      return const ColoredBox(color: Colors.black);
    }
    return RawImage(
      image: image,
      fit: BoxFit.fill,
      filterQuality: FilterQuality.none,
    );
  }

  /// Decodes one raw BGRA frame into a Flutter image.
  void _decodeFrame() {
    final frame = widget.frame;
    if (frame.codec != 'raw/bgra') {
      throw StateError('Invalid browser surface frame codec');
    }
    if (frame.width <= 0 || frame.height <= 0) {
      throw StateError('Invalid browser surface frame dimensions');
    }
    _decodedFrame = frame;
    ui.decodeImageFromPixels(
      frame.data,
      frame.width,
      frame.height,
      ui.PixelFormat.bgra8888,
      (ui.Image image) {
        if (!mounted || !identical(_decodedFrame, frame)) {
          image.dispose();
          return;
        }
        final oldImage = _image;
        setState(() {
          _image = image;
        });
        oldImage?.dispose();
      },
    );
  }
}
