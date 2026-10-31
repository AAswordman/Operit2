part of '../floating_windows.dart';

/// Application-scoped surfaces for iOS, Web, OHOS, and compositors that do not
/// permit global positioning. The capability scope explicitly reports this.
class FloatingWindowsHost extends StatefulWidget {
  const FloatingWindowsHost({
    super.key,
    required this.child,
    required this.windowBuilder,
  });
  final Widget child;
  final Widget Function(FloatingWindowLaunch) windowBuilder;
  @override
  State<FloatingWindowsHost> createState() => _FloatingWindowsHostState();
}

class _ApplicationWindow {
  _ApplicationWindow(this.payload, this.options);
  Map<String, Object?> payload;
  FloatingWindowOptions options;
  Future<void> Function()? beforeClose;
  bool visible = true;
  bool closing = false;
}

class _FloatingWindowsHostState extends State<FloatingWindowsHost> {
  final _windows = <String, _ApplicationWindow>{};

  @override
  void initState() {
    super.initState();
    if (FloatingWindows._applicationHost != null) {
      throw StateError('Only one FloatingWindowsHost may own an engine');
    }
    FloatingWindows._applicationHost = this;
  }

  FloatingWindow open(
    Map<String, Object?> payload,
    FloatingWindowOptions options,
  ) {
    final id = 'application:${++FloatingWindows._applicationSerial}';
    setState(() {
      _windows[id] = _ApplicationWindow(payload, options);
    });
    return FloatingWindow._(id);
  }

  Future<Object?> command(String id, String method, Object? arguments) async {
    final window = _windows[id];
    if (window == null || window.closing)
      throw StateError('Window is closed: $id');
    switch (method) {
      case 'ready':
        return true;
      case 'getBounds':
        return {
          'x': window.options.x ?? 24,
          'y': window.options.y ?? 80,
          'width': window.options.width,
          'height': window.options.height,
        };
      case 'update':
        final args = Map<String, Object?>.from(arguments as Map);
        final payload = args.remove('payload') as Map?;
        final patch = FloatingWindowOptions.fromMap(args);
        if (patch.alwaysOnTop == false)
          throw UnsupportedError(
            'Application surfaces remain above application content',
          );
        setState(() {
          window.options = window.options.merge(patch);
          if (payload != null)
            window.payload = {
              ...window.payload,
              ...payload.cast<String, Object?>(),
            };
        });
      case 'show':
        setState(() {
          window.visible = true;
        });
      case 'hide':
        setState(() {
          window.visible = false;
        });
      case 'close':
        window.closing = true;
        // ACK before draining: closing may be requested by this session itself.
        Timer.run(() async {
          try {
            await window.beforeClose?.call().timeout(
              const Duration(seconds: 5),
            );
          } catch (error, stack) {
            FlutterError.reportError(
              FlutterErrorDetails(
                exception: error,
                stack: stack,
                library: 'floating_windows',
                context: ErrorDescription('draining application surface'),
              ),
            );
          } finally {
            if (mounted)
              setState(() {
                _windows.remove(id);
              });
          }
        });
      default:
        throw ArgumentError('Unknown window command: $method');
    }
    return null;
  }

  @override
  void dispose() {
    if (identical(FloatingWindows._applicationHost, this))
      FloatingWindows._applicationHost = null;
    // Window widget disposal releases its UI session as part of app teardown.
    _windows.clear();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Directionality(
    textDirection: TextDirection.ltr,
    child: MediaQuery.fromView(
      view: View.of(context),
      child: LayoutBuilder(
        builder: (context, constraints) {
          return Stack(
            fit: StackFit.expand,
            children: [
              widget.child,
              for (final entry in _windows.entries)
                _surface(entry.key, entry.value, constraints),
            ],
          );
        },
      ),
    ),
  );

  Widget _surface(String id, _ApplicationWindow window, BoxConstraints bounds) {
    final options = window.options;
    final width = options.width!.clamp(0.0, bounds.maxWidth);
    final height = options.height!.clamp(0.0, bounds.maxHeight);
    return Positioned(
      key: ValueKey(id),
      left: (options.x ?? 24).clamp(0.0, bounds.maxWidth - width),
      top: (options.y ?? 80).clamp(0.0, bounds.maxHeight - height),
      width: width,
      height: height,
      child: Offstage(
        offstage: !window.visible,
        child: TickerMode(
          enabled: window.visible,
          child: IgnorePointer(
            ignoring: options.clickThrough!,
            child: ExcludeFocus(
              excluding: !options.focusable!,
              child: _ApplicationDragSurface(
                enabled: options.draggable == true,
                origin: Offset(
                  (options.x ?? 24).clamp(0.0, bounds.maxWidth - width),
                  (options.y ?? 80).clamp(0.0, bounds.maxHeight - height),
                ),
                onMove: (position) {
                  if (!mounted || window.closing) return;
                  setState(() {
                    window.options = window.options.merge(
                      FloatingWindowOptions(
                        x: position.dx.clamp(0.0, bounds.maxWidth - width),
                        y: position.dy.clamp(0.0, bounds.maxHeight - height),
                      ),
                    );
                  });
                },
                child: widget.windowBuilder(
                  FloatingWindowLaunch(
                    payload: window.payload,
                    options: options,
                    onBeforeCloseChanged: (callback) {
                      window.beforeClose = callback;
                    },
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
