import 'dart:async';
import 'dart:convert';
import 'package:desktop_multi_window/desktop_multi_window.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';

part 'src/application_windows.dart';
part 'src/drag_surface.dart';

/// Native accessibility screen coordinates: pixels on Windows/Android,
/// desktop points on macOS and AT-SPI desktop units on X11.
class FloatingScreenBounds {
  const FloatingScreenBounds(this.left, this.top, this.right, this.bottom);
  final double left, top, right, bottom;

  factory FloatingScreenBounds.fromMap(Map value) {
    final edges = [
      for (final key in ['left', 'top', 'right', 'bottom']) value[key],
    ];
    if (edges.any(
      (edge) => edge is! num || !edge.isFinite || edge.abs() > 1000000,
    )) {
      throw ArgumentError('Screen bounds must contain four finite coordinates');
    }
    final bounds = FloatingScreenBounds(
      (edges[0] as num).toDouble(),
      (edges[1] as num).toDouble(),
      (edges[2] as num).toDouble(),
      (edges[3] as num).toDouble(),
    );
    bounds.toMap();
    return bounds;
  }

  Map<String, double> toMap() {
    if ([
          left,
          top,
          right,
          bottom,
        ].any((edge) => !edge.isFinite || edge.abs() > 1000000) ||
        right <= left ||
        bottom <= top) {
      throw ArgumentError(
        'Screen bounds must describe a nonempty finite rectangle',
      );
    }
    return {'left': left, 'top': top, 'right': right, 'bottom': bottom};
  }
}

/// A partial update, expressed in logical pixels. Input passes through the whole
/// rectangle when clickThrough is true; alpha-based hit testing is not implied.
class FloatingWindowOptions {
  const FloatingWindowOptions({
    this.x,
    this.y,
    this.width,
    this.height,
    this.alwaysOnTop,
    this.clickThrough,
    this.focusable,
    this.draggable,
    this.screenBounds,
  });
  final double? x, y, width, height;
  final bool? alwaysOnTop, clickThrough, focusable, draggable;
  final FloatingScreenBounds? screenBounds;

  /// Reports explicit bounds that cannot be combined with host-sized layout.
  bool get hasBounds =>
      x != null ||
      y != null ||
      width != null ||
      height != null ||
      screenBounds != null;

  factory FloatingWindowOptions.fromMap(Map<String, Object?> value) {
    const keys = {
      'x',
      'y',
      'width',
      'height',
      'alwaysOnTop',
      'clickThrough',
      'focusable',
      'draggable',
      'screenBounds',
    };
    if (value.keys.any((key) => !keys.contains(key))) {
      throw ArgumentError('Unknown floating window option');
    }
    for (final key in ['x', 'y', 'width', 'height']) {
      final number = value[key];
      if (number != null && (number is! num || !number.isFinite)) {
        throw ArgumentError('$key must be a finite number');
      }
    }
    for (final key in [
      'alwaysOnTop',
      'clickThrough',
      'focusable',
      'draggable',
    ]) {
      if (value[key] != null && value[key] is! bool) {
        throw ArgumentError('$key must be a boolean');
      }
    }
    return FloatingWindowOptions(
      x: (value['x'] as num?)?.toDouble(),
      y: (value['y'] as num?)?.toDouble(),
      width: (value['width'] as num?)?.toDouble(),
      height: (value['height'] as num?)?.toDouble(),
      alwaysOnTop: value['alwaysOnTop'] as bool?,
      clickThrough: value['clickThrough'] as bool?,
      focusable: value['focusable'] as bool?,
      draggable: value['draggable'] as bool?,
      screenBounds: value['screenBounds'] == null
          ? null
          : FloatingScreenBounds.fromMap(value['screenBounds'] as Map),
    );
  }

  Map<String, Object?> toMap() {
    for (final number in [x, y, width, height]) {
      if (number != null && !number.isFinite)
        throw ArgumentError('Geometry must be finite');
    }
    for (final number in [width, height]) {
      if (number != null && (number < 16 || number > 4096)) {
        throw ArgumentError(
          'Dimensions must be between 16 and 4096 logical pixels',
        );
      }
    }
    return {
      if (x != null) 'x': x,
      if (y != null) 'y': y,
      if (width != null) 'width': width,
      if (height != null) 'height': height,
      if (alwaysOnTop != null) 'alwaysOnTop': alwaysOnTop,
      if (clickThrough != null) 'clickThrough': clickThrough,
      if (focusable != null) 'focusable': focusable,
      if (draggable != null) 'draggable': draggable,
      if (screenBounds != null) 'screenBounds': screenBounds!.toMap(),
    };
  }

  FloatingWindowOptions merge(FloatingWindowOptions patch) =>
      FloatingWindowOptions.fromMap({...toMap(), ...patch.toMap()});

  static const defaults = FloatingWindowOptions(
    width: 240,
    height: 240,
    alwaysOnTop: true,
    clickThrough: false,
    focusable: false,
    draggable: false,
  );

  static const _fillDefaults = FloatingWindowOptions(
    alwaysOnTop: true,
    clickThrough: false,
    focusable: false,
    draggable: false,
  );
}

/// Opaque application content kept separate from host presentation options.
class FloatingWindowLaunch {
  /// Carries opaque content and window configuration to the presentation engine.
  const FloatingWindowLaunch({
    required this.payload,
    required this.options,
    this.fill = false,
    this.onBeforeCloseChanged,
  });

  /// The application host keeps one close hook per surface.
  final void Function(Future<void> Function()? callback)? onBeforeCloseChanged;
  final Map<String, Object?> payload;
  final FloatingWindowOptions options;
  final bool fill;
}

/// Owns secondary-engine startup and native surfaces without app-specific data.
class FloatingWindows {
  static const channel = MethodChannel('floating_windows/window');
  static const _kind = 'floating_windows.window';
  static bool get _android =>
      !kIsWeb && defaultTargetPlatform == TargetPlatform.android;
  static bool get _desktop =>
      !kIsWeb &&
      [
        TargetPlatform.macOS,
        TargetPlatform.windows,
        TargetPlatform.linux,
      ].contains(defaultTargetPlatform);
  static Future<void> Function()? beforeClose;

  /// Lets the application retain engine startup diagnostics in its normal log sink.
  static void Function(String message)? onDiagnostic;

  static void _diagnostic(String message) {
    final line = 'platform=${defaultTargetPlatform.name} $message';
    final sink = onDiagnostic;
    if (sink != null) {
      sink(line);
    } else {
      debugPrint('[FloatingWindows] $line');
    }
  }

  static FloatingWindowLaunch? parseLaunch(List<String> arguments) {
    String? text;
    if (arguments.length == 3 && arguments[0] == 'multi_window')
      text = arguments[2];
    if (arguments.length == 2 && arguments[0] == _kind) text = arguments[1];
    if (text == null) return null;
    Object? raw;
    try {
      raw = jsonDecode(text);
    } on FormatException {
      return null;
    }
    if (raw is! Map ||
        raw['kind'] != _kind ||
        raw['payload'] is! Map ||
        raw['options'] is! Map)
      return null;
    return FloatingWindowLaunch(
      payload: (raw['payload'] as Map).cast<String, Object?>(),
      options: FloatingWindowOptions.fromMap(
        (raw['options'] as Map).cast<String, Object?>(),
      ),
      fill: (raw['fill'] as bool?) ?? false,
    );
  }

  static _FloatingWindowsHostState? _applicationHost;
  static int _applicationSerial = 0;

  static void setBeforeClose(
    FloatingWindowLaunch launch,
    Future<void> Function()? callback,
  ) {
    final register = launch.onBeforeCloseChanged;
    if (register != null) {
      register(callback);
    } else {
      beforeClose = callback;
    }
  }

  static Future<Map<String, Object?>> capabilities() async {
    if (_desktop || _android) {
      final native = await channel.invokeMapMethod<String, Object?>(
        'capabilities',
      );
      if (native?['supported'] == true) {
        return {
          ...native!,
          'scope': 'system',
          'systemOverlay': true,
          'applicationOverlay': _applicationHost != null,
        };
      }
    }
    final available = _applicationHost != null;
    return {
      'supported': available,
      'permissionGranted': available,
      'scope': available ? 'application' : 'unsupported',
      'systemOverlay': false,
      'applicationOverlay': available,
      'positioning': available,
      'alwaysOnTop': false,
      'clickThrough': available,
      'focusable': available,
    };
  }

  static Future<bool> requestPermission() async {
    final support = await capabilities();
    if (support['scope'] == 'application') return true;
    if (support['supported'] != true) return false;
    return await channel.invokeMethod<bool>('requestPermission') ?? false;
  }

  /// Opens a bounded or host-sized surface in the available scope.
  static Future<FloatingWindow> open({
    required Map<String, Object?> payload,
    FloatingWindowOptions options = const FloatingWindowOptions(),
    bool fill = false,
    Duration startupTimeout = const Duration(seconds: 30),
  }) async {
    final stopwatch = Stopwatch()..start();
    _diagnostic(
      'open.begin route=${payload['route']} windowId=${payload['windowId']} '
      'startupTimeoutMs=${startupTimeout.inMilliseconds}',
    );
    if (startupTimeout <= Duration.zero)
      throw ArgumentError.value(startupTimeout, 'startupTimeout');
    if (fill && (options.hasBounds || options.draggable == true)) {
      throw ArgumentError('A fill window cannot set bounds or enable dragging');
    }
    final support = await capabilities();
    if (support['supported'] != true || support['permissionGranted'] != true) {
      throw StateError(
        'Floating windows are unsupported or overlay permission is missing',
      );
    }
    final normalized =
        (fill || options.screenBounds != null
                ? FloatingWindowOptions._fillDefaults
                : FloatingWindowOptions.defaults)
            .merge(options);
    if (normalized.alwaysOnTop == false && support['alwaysOnTop'] != true) {
      throw UnsupportedError('This host requires alwaysOnTop');
    }
    if (support['scope'] == 'application') {
      if (options.screenBounds != null) {
        throw UnsupportedError('Native screen bounds require a system surface');
      }
      return _applicationHost!.open(payload, normalized, fill: fill);
    }
    _diagnostic(
      'open.create scope=${support['scope']} elapsedMs=${stopwatch.elapsedMilliseconds}',
    );
    final encoded = jsonEncode({
      'kind': _kind,
      'payload': payload,
      'options': normalized.toMap(),
      if (fill) 'fill': true,
    });
    late final FloatingWindow window;
    if (_android) {
      window = FloatingWindow._(
        (await channel.invokeMethod<String>('create', {'arguments': encoded}))!,
        fill: fill,
      );
    } else {
      final controller = await WindowController.create(
        WindowConfiguration(
          hiddenAtLaunch: true,
          transparent: true,
          arguments: encoded,
        ),
      );
      window = FloatingWindow._(controller.windowId, fill: fill);
    }
    _diagnostic(
      'open.created nativeId=${window.id} elapsedMs=${stopwatch.elapsedMilliseconds}',
    );
    var probes = 0;
    var lastReply = 'pending';
    final progress = Timer.periodic(const Duration(seconds: 5), (_) {
      _diagnostic(
        'open.waiting nativeId=${window.id} probes=$probes lastReply=$lastReply '
        'elapsedMs=${stopwatch.elapsedMilliseconds}',
      );
    });
    try {
      final deadline = DateTime.now().add(startupTimeout);
      while (true) {
        try {
          final remaining = deadline.difference(DateTime.now());
          if (remaining <= Duration.zero)
            throw TimeoutException('Floating window startup timed out');
          probes++;
          if (await window._command('ready').timeout(remaining) == true) break;
          lastReply = 'not-ready';
        } on WindowChannelException catch (error) {
          lastReply = error.code;
          if (![
            'CHANNEL_NOT_FOUND',
            'CHANNEL_NOT_REGISTERED',
            'CHANNEL_UNREGISTERED',
            'NO_HANDLER',
          ].contains(error.code))
            rethrow;
        } on PlatformException catch (error) {
          lastReply = error.code;
          if (!['CHANNEL_NOT_FOUND', 'NO_HANDLER'].contains(error.code))
            rethrow;
        } on MissingPluginException catch (_) {
          lastReply = 'missing-plugin';
          /* The child has not installed its handler. */
        }
        if (DateTime.now().isAfter(deadline))
          throw TimeoutException('Floating window startup timed out');
        await Future<void>.delayed(const Duration(milliseconds: 50));
      }
      _diagnostic(
        'open.ready nativeId=${window.id} probes=$probes elapsedMs=${stopwatch.elapsedMilliseconds}',
      );
      return window;
    } catch (error, stack) {
      _diagnostic(
        'open.failed nativeId=${window.id} probes=$probes lastReply=$lastReply '
        'elapsedMs=${stopwatch.elapsedMilliseconds} error=$error',
      );
      // The owner transport can destroy children that never installed a handler.
      try {
        await _destroyNative(window.id).timeout(const Duration(seconds: 5));
        _diagnostic('open.reclaimed nativeId=${window.id}');
      } catch (cleanupError, cleanupStack) {
        FlutterError.reportError(
          FlutterErrorDetails(
            exception: cleanupError,
            stack: cleanupStack,
            library: 'floating_windows',
            context: ErrorDescription('reclaiming failed child startup'),
          ),
        );
      }
      Error.throwWithStackTrace(error, stack);
    } finally {
      progress.cancel();
    }
  }

  static Future<void> _destroyNative(String id) async {
    if (_android) {
      await channel.invokeMethod<void>('destroy', {'id': id});
    } else {
      await WindowController.fromWindowId(id).close();
    }
  }

  static Future<void> run({
    required List<String> arguments,
    required Widget Function(FloatingWindowLaunch) windowBuilder,
    required Future<void> Function() application,
  }) async {
    final launch = parseLaunch(arguments);
    if (launch == null) {
      await application();
      return;
    }
    final stopwatch = Stopwatch()..start();
    final identity =
        'windowId=${launch.payload['windowId']} route=${launch.payload['route']}';
    _diagnostic('child.launch $identity');
    final content = ValueNotifier<FloatingWindowLaunch>(launch);
    final dragPointers = <int>{};
    var ready = false;
    Object? startupError;
    var closing = false;
    Future<Object?> receive(MethodCall call) async {
      if (call.method == 'ready') {
        if (startupError != null)
          throw PlatformException(
            code: 'STARTUP_FAILED',
            message: startupError.toString(),
          );
        return ready;
      }
      if (call.method == 'close') {
        if (!closing) {
          closing = true;
          // A plugin may close itself from an in-flight UI action. Reply before
          // draining that session to avoid waiting on the caller's own promise.
          Timer.run(() async {
            try {
              await beforeClose?.call().timeout(const Duration(seconds: 5));
              runApp(const SizedBox.shrink());
              await WidgetsBinding.instance.endOfFrame.timeout(
                const Duration(milliseconds: 250),
                onTimeout: () {},
              );
            } catch (error, stack) {
              FlutterError.reportError(
                FlutterErrorDetails(
                  exception: error,
                  stack: stack,
                  library: 'floating_windows',
                  context: ErrorDescription('draining floating window'),
                ),
              );
            } finally {
              await channel.invokeMethod<void>('close');
            }
          });
        }
        return null;
      }
      if (closing) throw StateError('Floating window is closing');
      if (call.method == 'update') {
        final args = (call.arguments as Map).cast<String, Object?>();
        final payload = args['payload'] as Map?;
        final options = Map<String, Object?>.of(args)..remove('payload');
        await channel.invokeMethod<void>('update', options);
        final previous = content.value;
        content.value = FloatingWindowLaunch(
          payload: {
            ...previous.payload,
            if (payload != null) ...payload.cast<String, Object?>(),
          },
          options: previous.options.merge(
            FloatingWindowOptions.fromMap(options),
          ),
          fill: previous.fill,
        );
        return null;
      }
      return channel.invokeMethod<Object?>(call.method, call.arguments);
    }

    channel.setMethodCallHandler((call) async {
      if (call.method == 'dragStarted') {
        for (final pointer in dragPointers.toList()) {
          GestureBinding.instance.cancelPointer(pointer);
        }
        return null;
      }
      if (_android) return receive(call);
      throw MissingPluginException();
    });
    if (!_android) {
      final controller = await WindowController.fromCurrentEngine();
      await controller.setWindowMethodHandler(receive);
      _diagnostic(
        'child.handler $identity nativeId=${controller.windowId} elapsedMs=${stopwatch.elapsedMilliseconds}',
      );
    }
    try {
      await channel.invokeMethod<void>('configure', {
        'role': _kind,
        ...launch.options.toMap(),
        if (launch.fill) 'fill': true,
      });
      _diagnostic(
        'child.configured $identity elapsedMs=${stopwatch.elapsedMilliseconds}',
      );
      runApp(
        Listener(
          onPointerDown: (event) => dragPointers.add(event.pointer),
          onPointerUp: (event) => dragPointers.remove(event.pointer),
          onPointerCancel: (event) => dragPointers.remove(event.pointer),
          child: ValueListenableBuilder<FloatingWindowLaunch>(
            valueListenable: content,
            builder: (_, current, _) => windowBuilder(current),
          ),
        ),
      );
      // Hidden macOS engines may suspend frames until their window is visible.
      // Showing cannot depend on a first frame that itself depends on showing.
      final firstFrame = WidgetsBinding.instance.endOfFrame;
      await channel.invokeMethod<void>('show');
      _diagnostic(
        'child.shown $identity elapsedMs=${stopwatch.elapsedMilliseconds}',
      );
      await firstFrame;
      ready = true;
      _diagnostic(
        'child.ready $identity elapsedMs=${stopwatch.elapsedMilliseconds}',
      );
    } catch (error) {
      _diagnostic(
        'child.failed $identity elapsedMs=${stopwatch.elapsedMilliseconds} error=$error',
      );
      startupError = error;
      rethrow;
    }
  }

  /// Returns host identities for pruning externally closed surfaces.
  static Future<Set<String>> liveIds() async {
    final ids = _applicationHost?._windows.keys.toSet() ?? <String>{};
    if (_android)
      ids.addAll(await channel.invokeListMethod<String>('list') ?? []);
    if (_desktop) {
      // A compositor may become unavailable after a native window was opened.
      // Existing engines still need ownership tracking and explicit teardown.
      ids.addAll((await WindowController.getAll()).map((w) => w.windowId));
    }
    return ids;
  }
}

class FloatingWindow {
  /// Keeps the host identity and immutable layout policy together.
  const FloatingWindow._(this.id, {this.fill = false});
  final String id;
  final bool fill;
  Future<Object?> _command(String method, [Object? arguments]) {
    if (id.startsWith('application:')) {
      final host = FloatingWindows._applicationHost;
      if (host == null)
        return Future.error(StateError('Application window host was disposed'));
      return host.command(id, method, arguments);
    }
    if (FloatingWindows._android)
      return FloatingWindows.channel.invokeMethod<Object?>('invoke', {
        'id': id,
        'method': method,
        'arguments': arguments,
      });
    return WindowController.fromWindowId(
      id,
    ).invokeMethod<Object?>(method, arguments);
  }

  /// Applies a partial window update while preserving the selected layout mode.
  Future<void> update(
    FloatingWindowOptions options, {
    Map<String, Object?>? payload,
  }) async {
    if (id.startsWith('application:') && options.screenBounds != null) {
      throw UnsupportedError('Native screen bounds require a system surface');
    }
    if (fill && (options.hasBounds || options.draggable == true)) {
      throw ArgumentError('A fill window cannot set bounds or enable dragging');
    }
    await _command('update', {
      ...options.toMap(),
      if (payload != null) 'payload': payload,
    });
  }

  Future<FloatingWindowOptions> getBounds() async =>
      FloatingWindowOptions.fromMap(
        Map<String, Object?>.from((await _command('getBounds')) as Map),
      );
  Future<void> show() async => _command('show');
  Future<void> hide() async => _command('hide');

  /// Requests session drainage first. An unavailable child is reclaimed by its
  /// native owner so a broken Dart channel cannot leave an orphaned surface.
  Future<void> close({Duration timeout = const Duration(seconds: 8)}) async {
    if (timeout <= Duration.zero) {
      throw ArgumentError.value(timeout, 'timeout');
    }
    if (id.startsWith('application:')) {
      await _command('close');
      return;
    }
    try {
      await _command('close').timeout(timeout);
    } catch (error, stack) {
      try {
        await FloatingWindows._destroyNative(id).timeout(timeout);
      } catch (_) {
        Error.throwWithStackTrace(error, stack);
      }
    }
  }
}
