// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:webview_all/webview_all.dart' show WebViewScaleScope;

/// Defines the discrete, user-controlled interface zoom levels.
abstract final class ApplicationZoom {
  static const List<double> levels = <double>[
    0.7,
    0.8,
    0.9,
    1.0,
    1.1,
    1.2,
    1.3,
    1.4,
    1.5,
  ];

  /// Returns the adjacent zoom level in the requested direction.
  static double step(double current, int direction) {
    final index = levels.indexOf(current);
    if (index < 0) {
      throw ArgumentError.value(current, 'current', 'Unknown zoom level');
    }
    final nextIndex = (index + direction).clamp(0, levels.length - 1).toInt();
    return levels[nextIndex];
  }
}

/// Shares the effective interface zoom and its existing persistence callback.
class ApplicationZoomScope extends InheritedWidget {
  const ApplicationZoomScope({
    super.key,
    required this.zoom,
    required this.onZoomChanged,
    required super.child,
  });

  final double zoom;
  final ValueChanged<double> onZoomChanged;

  /// Observes zoom so controls also reflect application keyboard shortcuts.
  static ApplicationZoomScope of(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<ApplicationZoomScope>();
    if (scope == null) {
      throw StateError('ApplicationZoom scope not found');
    }
    return scope;
  }

  @override
  bool updateShouldNotify(ApplicationZoomScope oldWidget) =>
      zoom != oldWidget.zoom || onZoomChanged != oldWidget.onZoomChanged;
}

/// Marks a subtree whose own zoom shortcuts must keep their key events.
class ApplicationZoomExclusion extends StatelessWidget {
  /// Creates a zoom shortcut exclusion scope.
  const ApplicationZoomExclusion({super.key, required this.child});

  final Widget child;

  /// Builds the exclusion scope around a nested zoomable surface.
  @override
  Widget build(BuildContext context) => child;
}

/// Applies explicit interface zoom without changing the host device pixel ratio.
class ApplicationZoomHost extends StatefulWidget {
  /// Creates the application viewport and its process-wide zoom shortcuts.
  const ApplicationZoomHost({
    super.key,
    required this.zoom,
    required this.onZoomChanged,
    required this.child,
    this.shortcutsEnabled = true,
  });

  final double zoom;
  final ValueChanged<double> onZoomChanged;
  final Widget child;
  final bool shortcutsEnabled;

  /// Creates the lifetime-bound global keyboard handler.
  @override
  State<ApplicationZoomHost> createState() => _ApplicationZoomHostState();
}

class _ApplicationZoomHostState extends State<ApplicationZoomHost> {
  static const Map<SingleActivator, int> _shortcuts = <SingleActivator, int>{
    SingleActivator(LogicalKeyboardKey.equal, control: true): 1,
    SingleActivator(LogicalKeyboardKey.equal, control: true, shift: true): 1,
    SingleActivator(LogicalKeyboardKey.add, control: true): 1,
    SingleActivator(LogicalKeyboardKey.add, control: true, shift: true): 1,
    SingleActivator(LogicalKeyboardKey.numpadAdd, control: true): 1,
    SingleActivator(LogicalKeyboardKey.minus, control: true): -1,
    SingleActivator(LogicalKeyboardKey.numpadSubtract, control: true): -1,
    SingleActivator(LogicalKeyboardKey.digit0, control: true): 0,
    SingleActivator(LogicalKeyboardKey.numpad0, control: true): 0,
    SingleActivator(LogicalKeyboardKey.equal, meta: true): 1,
    SingleActivator(LogicalKeyboardKey.equal, meta: true, shift: true): 1,
    SingleActivator(LogicalKeyboardKey.add, meta: true): 1,
    SingleActivator(LogicalKeyboardKey.add, meta: true, shift: true): 1,
    SingleActivator(LogicalKeyboardKey.numpadAdd, meta: true): 1,
    SingleActivator(LogicalKeyboardKey.minus, meta: true): -1,
    SingleActivator(LogicalKeyboardKey.numpadSubtract, meta: true): -1,
    SingleActivator(LogicalKeyboardKey.digit0, meta: true): 0,
    SingleActivator(LogicalKeyboardKey.numpad0, meta: true): 0,
  };

  /// Registers application shortcuts before browser and terminal key handlers.
  @override
  void initState() {
    super.initState();
    FocusManager.instance.addEarlyKeyEventHandler(_handleKeyEvent);
  }

  /// Removes the process-wide handler when the application root is released.
  @override
  void dispose() {
    FocusManager.instance.removeEarlyKeyEventHandler(_handleKeyEvent);
    super.dispose();
  }

  /// Handles only explicit zoom shortcuts without consuming ordinary typing.
  KeyEventResult _handleKeyEvent(KeyEvent event) {
    if (!widget.shortcutsEnabled) {
      return KeyEventResult.ignored;
    }
    final focusedContext = FocusManager.instance.primaryFocus?.context;
    if (focusedContext
            ?.findAncestorWidgetOfExactType<ApplicationZoomExclusion>() !=
        null) {
      return KeyEventResult.ignored;
    }
    for (final entry in _shortcuts.entries) {
      if (entry.key.accepts(event, HardwareKeyboard.instance)) {
        final zoom = entry.value == 0
            ? 1.0
            : ApplicationZoom.step(widget.zoom, entry.value);
        widget.onZoomChanged(zoom);
        return KeyEventResult.handled;
      }
    }
    return KeyEventResult.ignored;
  }

  /// Lays out the Navigator and its overlays in the same zoomed coordinate space.
  @override
  Widget build(BuildContext context) {
    final mediaQuery = MediaQuery.of(context);
    final zoom = widget.zoom;
    final zoomedSize = mediaQuery.size / zoom;
    return ApplicationZoomScope(
      zoom: zoom,
      onZoomChanged: widget.onZoomChanged,
      child: MediaQuery(
        data: mediaQuery.copyWith(
          size: zoomedSize,
          padding: mediaQuery.padding / zoom,
          viewPadding: mediaQuery.viewPadding / zoom,
          viewInsets: mediaQuery.viewInsets / zoom,
          systemGestureInsets: mediaQuery.systemGestureInsets / zoom,
        ),
        child: FittedBox(
          fit: BoxFit.fill,
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: zoomedSize.width,
            height: zoomedSize.height,
            child: WebViewScaleScope(scale: zoom, child: widget.child),
          ),
        ),
      ),
    );
  }
}
