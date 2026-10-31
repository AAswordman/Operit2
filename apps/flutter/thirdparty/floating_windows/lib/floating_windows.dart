import 'dart:async';
import 'dart:convert';
import 'package:desktop_multi_window/desktop_multi_window.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';

part 'src/application_windows.dart';
part 'src/drag_surface.dart';

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
  });
  final double? x, y, width, height;
  final bool? alwaysOnTop, clickThrough, focusable, draggable;

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
}

/// Opaque application content kept separate from host presentation options.
class FloatingWindowLaunch {
  const FloatingWindowLaunch({
    required this.payload,
    required this.options,
    this.onBeforeCloseChanged,
  });

  /// The application host keeps one close hook per surface.
  final void Function(Future<void> Function()? callback)? onBeforeCloseChanged;
  final Map<String, Object?> payload;
  final FloatingWindowOptions options;
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

  static Future<FloatingWindow> open({
    required Map<String, Object?> payload,
    FloatingWindowOptions options = FloatingWindowOptions.defaults,
    Duration startupTimeout = const Duration(seconds: 30),
  }) async {
    if (startupTimeout <= Duration.zero)
      throw ArgumentError.value(startupTimeout, 'startupTimeout');
    final support = await capabilities();
    if (support['supported'] != true || support['permissionGranted'] != true) {
      throw StateError(
        'Floating windows are unsupported or overlay permission is missing',
      );
    }
    final normalized = FloatingWindowOptions.defaults.merge(options);
    if (normalized.alwaysOnTop == false && support['alwaysOnTop'] != true) {
      throw UnsupportedError('This host requires alwaysOnTop');
    }
    if (support['scope'] == 'application') {
      return _applicationHost!.open(payload, normalized);
    }
    final encoded = jsonEncode({
      'kind': _kind,
      'payload': payload,
      'options': normalized.toMap(),
    });
    late final FloatingWindow window;
    if (_android) {
      window = FloatingWindow._(
        (await channel.invokeMethod<String>('create', {'arguments': encoded}))!,
      );
    } else {
      final controller = await WindowController.create(
        WindowConfiguration(
          hiddenAtLaunch: true,
          transparent: true,
          arguments: encoded,
        ),
      );
      window = FloatingWindow._(controller.windowId);
    }
    try {
      final deadline = DateTime.now().add(startupTimeout);
      while (true) {
        try {
          final remaining = deadline.difference(DateTime.now());
          if (remaining <= Duration.zero)
            throw TimeoutException('Floating window startup timed out');
          if (await window._command('ready').timeout(remaining) == true) break;
        } on WindowChannelException catch (error) {
          if (![
            'CHANNEL_NOT_FOUND',
            'CHANNEL_NOT_REGISTERED',
            'CHANNEL_UNREGISTERED',
            'NO_HANDLER',
          ].contains(error.code))
            rethrow;
        } on PlatformException catch (error) {
          if (!['CHANNEL_NOT_FOUND', 'NO_HANDLER'].contains(error.code))
            rethrow;
        } on MissingPluginException catch (_) {
          /* The child has not installed its handler. */
        }
        if (DateTime.now().isAfter(deadline))
          throw TimeoutException('Floating window startup timed out');
        await Future<void>.delayed(const Duration(milliseconds: 50));
      }
      return window;
    } catch (error, stack) {
      // The owner transport can destroy children that never installed a handler.
      try {
        await _destroyNative(window.id).timeout(const Duration(seconds: 5));
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
      await (await WindowController.fromCurrentEngine()).setWindowMethodHandler(
        receive,
      );
    }
    try {
      await channel.invokeMethod<void>('configure', {
        'role': _kind,
        ...launch.options.toMap(),
      });
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
      await WidgetsBinding.instance.endOfFrame;
      await channel.invokeMethod<void>('show');
      ready = true;
    } catch (error) {
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
  const FloatingWindow._(this.id);
  final String id;
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

  Future<void> update(
    FloatingWindowOptions options, {
    Map<String, Object?>? payload,
  }) async => _command('update', {
    ...options.toMap(),
    if (payload != null) 'payload': payload,
  });
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
