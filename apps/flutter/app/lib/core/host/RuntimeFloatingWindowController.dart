// ignore_for_file: file_names
import 'dart:async';
import 'dart:convert';
import 'package:floating_windows/floating_windows.dart';
import 'package:flutter/foundation.dart';
import '../../data/preferences/UserPreferencesManager.dart';
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../logging/ClientLogger.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../../ui/main/navigation/ToolPkgCatalogChangeBus.dart';
import 'RuntimeWindowRecording.dart';
import 'VoiceCallCamera.dart';

class _OwnedFloatingWindow {
  _OwnedFloatingWindow(
    this.window,
    this.route,
    this.fill,
    this.options,
    this.state,
  );
  final FloatingWindow window;
  final String route;
  final bool fill;
  FloatingWindowOptions options;
  Map<String, Object?> state;
  bool visible = true;
  RuntimeWindowRecording? recording;

  /// Reports measured geometry separately from the validated configuration.
  Future<Map<String, Object?>> info(String id) async {
    final bounds = await window.getBounds();
    return {
      'id': id,
      'route': route,
      'visible': visible,
      'options': {
        ...options.toMap(),
        'x': bounds.x,
        'y': bounds.y,
        'width': bounds.width,
        'height': bounds.height,
      },
    };
  }
}

/// The owner engine serializes lifecycle changes and indexes plugin-local IDs.
class RuntimeFloatingWindowController {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  static final _windows = <String, _OwnedFloatingWindow>{};
  static final _owners = <String, String>{};
  static final _ids = <String, String>{};
  static Future<void> _tail = Future<void>.value();
  static Timer? _audit;
  static StreamSubscription<void>? _catalog;
  static RuntimeWindowRecording? _activeRecording;
  static int _serial = 0;

  /// Records lifecycle boundaries at info level and routine updates at debug level.
  static void _log(String message, {bool routine = false}) {
    if (!ClientLogger.isInitialized) return;
    if (routine) {
      ClientLogger.d(message, tag: 'RuntimeFloatingWindow');
    } else {
      ClientLogger.i(message, tag: 'RuntimeFloatingWindow');
    }
  }

  /// Serializes lifecycle mutations while letting a pending recording finish independently.
  static Future<Object?> command(Map<String, Object?> request) {
    final serial = ++_serial;
    final waiting = Stopwatch()..start();
    final identity =
        'command=$serial owner=${request['packageName']} '
        'operation=${request['operation']} id=${request['id']}';
    _log('$identity queued', routine: true);
    final work = _tail.then((_) async {
      final elapsed = Stopwatch()..start();
      _log(
        '$identity begin queueWaitMs=${waiting.elapsedMilliseconds}',
        routine: waiting.elapsedMilliseconds < 250,
      );
      try {
        final result = await _command(request);
        _log(
          '$identity done elapsedMs=${elapsed.elapsedMilliseconds} '
          'totalMs=${waiting.elapsedMilliseconds}',
          routine: elapsed.elapsedMilliseconds < 250,
        );
        return result;
      } catch (error, stack) {
        if (ClientLogger.isInitialized) {
          ClientLogger.e(
            '$identity failed elapsedMs=${elapsed.elapsedMilliseconds} '
            'totalMs=${waiting.elapsedMilliseconds}',
            tag: 'RuntimeFloatingWindow',
            error: error,
            stackTrace: stack,
          );
        }
        rethrow;
      }
    });
    _tail = work.then<void>((_) {}, onError: (Object _, StackTrace stack) {});
    return work.then((value) async {
      if (value case (final _OwnedFloatingWindow window, final String chatId)) {
        return _captureCameraFrame(window, chatId);
      }
      if (value is! (_OwnedFloatingWindow, RuntimeWindowRecording)) {
        return value;
      }
      final (window, capture) = value;
      try {
        final audio = await capture.result;
        if (audio == null) return null;
        return {
          'audio': audio.bytes,
          'fileName': audio.fileName,
          'contentType': audio.contentType,
        };
      } finally {
        if (identical(_activeRecording, capture)) _activeRecording = null;
        if (identical(window.recording, capture)) window.recording = null;
      }
    });
  }

  /// Discards any microphone capture when the owner connection is uninstalled.
  static void cancelRecording() {
    final capture = _activeRecording;
    if (capture != null) unawaited(capture.finish(discard: true));
  }

  static bool _useApplicationWindow(String route) =>
      route == voiceCallRoute && voiceCallCameraNeedsActivity;

  static Future<Map<String, Object?>> _capabilities({
    required bool applicationOnly,
  }) async {
    final support = await FloatingWindows.capabilities(
      applicationOnly: applicationOnly,
    );
    if (!applicationOnly) return support;
    final native = await FloatingWindows.capabilities();
    return {
      ...support,
      'recordingEchoCancellation': native['recordingEchoCancellation'],
      'systemOverlay': native['systemOverlay'],
    };
  }

  /// Applies one authenticated plugin command in host arrival order.
  static Future<Object?> _command(Map<String, Object?> request) async {
    final owner = request['packageName'];
    if (owner is! String || owner.isEmpty) {
      throw ArgumentError('Missing authenticated window owner');
    }
    final operation = request['operation'];
    if (operation == 'capabilities') {
      return _capabilities(
        applicationOnly:
            owner == voiceCallPackage && voiceCallCameraNeedsActivity,
      );
    }
    if (operation == 'requestPermission') {
      return FloatingWindows.requestPermission();
    }
    await _prune();
    if (operation == 'list') {
      return [
        for (final entry in _windows.entries)
          if (_owners[entry.key] == owner)
            await entry.value.info(_ids[entry.key]!),
      ];
    }
    final id = request['id'];
    if (id is! String || id.isEmpty || id.trim() != id || id.length > 128) {
      throw ArgumentError(
        'Window id must be a trimmed string of 1–128 characters',
      );
    }
    final key = jsonEncode([owner, id]);
    if (operation == 'set') return _set(owner, id, key, request);
    if (operation == 'open') {
      if (_windows.containsKey(key)) {
        throw StateError('Window already exists: $id');
      }
      return (await _open(owner, id, key, request)).info(id);
    }
    final owned = _windows[key];
    if (owned == null) {
      throw StateError('Unknown window owned by this plugin: $id');
    }
    switch (operation) {
      case 'update':
        await _update(owned, request);
      case 'show':
        await owned.window.show();
        owned.visible = true;
      case 'hide':
        if (owner == voiceCallPackage &&
            owned.route == voiceCallRoute &&
            voiceCallCameraSupported) {
          await owned.window.contentCommand(voiceCallCameraStop);
        }
        await owned.window.hide();
        _discardRecording(owned);
        owned.visible = false;
      case 'close':
        await _close(key, owned);
        return null;
      case 'captureCameraFrame':
        if (!owned.visible ||
            owned.route != voiceCallRoute ||
            owner != voiceCallPackage ||
            !voiceCallCameraSupported) {
          return null;
        }
        final chatId = request['chatId'];
        if (chatId is! String || chatId.isEmpty || chatId.length > 128) {
          throw ArgumentError('A valid call chat id is required');
        }
        return (owned, chatId);
      case 'record':
        if (!owned.visible) throw StateError('The recording window is hidden');
        if (_activeRecording != null) {
          throw StateError('A floating window is already recording');
        }
        final maxDuration =
            _duration(request, 'maxDurationMs', 1000, 20000) ?? 15000;
        final silence = _duration(request, 'silenceMs', 300, 3000);
        final echoCancellation = request['echoCancellation'] == true;
        if (echoCancellation &&
            (await FloatingWindows.capabilities())['recordingEchoCancellation'] !=
                true) {
          throw UnsupportedError('Recording echo cancellation is unavailable');
        }
        final capture = RuntimeWindowRecording(
          maxDuration,
          silence,
          echoCancellation: echoCancellation,
        );
        _activeRecording = capture;
        owned.recording = capture;
        return (owned, capture);
      case 'recordingLevel':
        return owned.recording?.level ?? 0.0;
      case 'recordingHasSpeech':
        return owned.recording?.hasSpeech ?? false;
      case 'finishRecording':
      case 'cancelRecording':
        final capture = owned.recording;
        if (capture != null && identical(_activeRecording, capture)) {
          unawaited(capture.finish(discard: operation == 'cancelRecording'));
        }
        return null;
      default:
        throw ArgumentError('Unknown floating window operation: $operation');
    }
    return owned.info(id);
  }

  /// Imports the frame into the call chat using the regular attachment path.
  static Future<Map<String, Object?>?> _captureCameraFrame(
    _OwnedFloatingWindow owned,
    String chatId,
  ) async {
    final bytes = await owned.window
        .contentCommand(voiceCallCameraCapture)
        .timeout(const Duration(seconds: 5));
    if (bytes == null || !owned.visible || !_windows.containsValue(owned)) {
      return null;
    }
    if (bytes is! Uint8List ||
        bytes.isEmpty ||
        bytes.length > voiceCallFrameLimit) {
      throw StateError('Invalid camera frame');
    }
    const fileName = 'camera-frame.jpg';
    final transfer = _clients.servicesAttachmentTransferManager;
    final uploadId = await transfer.beginAttachmentUpload(
      fileName: fileName,
      expectedByteLength: bytes.length,
    );
    await transfer.writeSelectedAttachment(
      chatId: chatId,
      uploadId: uploadId,
      fileName: fileName,
      expectedByteLength: bytes.length,
      bytes: Stream.fromIterable([bytes, Uint8List(0)]),
    );
    final attachment = await transfer.completeSelectedAttachment(
      chatId: chatId,
      uploadId: uploadId,
      fileName: fileName,
      expectedByteLength: bytes.length,
    );
    return owned.visible && _windows.containsValue(owned)
        ? attachment.toJson()
        : null;
  }

  /// Reconciles desired content without exposing handle or permission bookkeeping to creators.
  static Future<String> _set(
    String owner,
    String id,
    String key,
    Map<String, Object?> request,
  ) async {
    final visible = request['visible'];
    if (visible is! bool) {
      throw ArgumentError('Window visibility must be a boolean');
    }
    final owned = _windows[key];
    if (!visible) {
      if (owned != null) await _close(key, owned);
      return 'closed';
    }
    final route = request['route'];
    if (route is! String || route.isEmpty) {
      throw ArgumentError('Window route is required');
    }
    if (owned != null &&
        (owned.route != route ||
            (request['fill'] != null && owned.fill != request['fill']))) {
      throw StateError(
        'Close the window before changing its route or fill mode',
      );
    }
    final support = await _capabilities(
      applicationOnly: _useApplicationWindow(route),
    );
    final String? unavailable = support['supported'] != true
        ? 'unsupported'
        : support['permissionGranted'] != true
        ? 'permission_required'
        : null;
    if (unavailable != null) {
      _log('set unavailable owner=$owner id=$id reason=$unavailable');
      if (owned != null) await _close(key, owned);
      return unavailable;
    }
    if (owned == null) {
      await _open(owner, id, key, request);
    } else {
      await _update(owned, request);
      if (!owned.visible) {
        await owned.window.show();
        owned.visible = true;
      }
    }
    return 'shown';
  }

  /// Creates one live route session after ownership and duplicate checks.
  static Future<_OwnedFloatingWindow> _open(
    String owner,
    String id,
    String key,
    Map<String, Object?> request,
  ) async {
    final stopwatch = Stopwatch()..start();
    _log('open.begin owner=$owner id=$id route=${request['route']}');
    if (_owners.values.where((value) => value == owner).length >= 16) {
      throw StateError('A plugin may own at most 16 floating windows');
    }
    final options = FloatingWindowOptions.fromMap(
      ((request['options'] as Map?) ?? {}).cast<String, Object?>(),
    );
    final fill = (request['fill'] as bool?) ?? false;
    final route = request['route'] as String;
    final applicationOnly = _useApplicationWindow(route);
    final support = await _capabilities(applicationOnly: applicationOnly);
    if ((options.x != null ||
            options.y != null ||
            options.screenBounds != null) &&
        support['positioning'] != true) {
      throw UnsupportedError(
        'This host does not support absolute window positioning',
      );
    }
    if (request['state'] != null && request['state'] is! Map) {
      throw ArgumentError('Window state must be an object');
    }
    final window = await FloatingWindows.open(
      applicationOnly: applicationOnly,
      payload: {
        'plugin': request['plugin'],
        'route': route,
        'state': request['state'] ?? {},
        'windowId': id,
        'locale': UserPreferencesManager.resolvedAppLocale.languageCode,
      },
      options: options,
      fill: fill,
    );
    final owned = _OwnedFloatingWindow(
      window,
      route,
      fill,
      FloatingWindowOptions.defaults.merge(options),
      Map<String, Object?>.from((request['state'] as Map?) ?? {}),
    );
    _windows[key] = owned;
    _owners[key] = owner;
    _ids[key] = id;
    _startAudit();
    _log(
      'open.done owner=$owner id=$id nativeId=${window.id} '
      'elapsedMs=${stopwatch.elapsedMilliseconds}',
    );
    return owned;
  }

  /// Updates only supplied options and state without resetting the live session.
  static Future<void> _update(
    _OwnedFloatingWindow owned,
    Map<String, Object?> request,
  ) async {
    final patch = FloatingWindowOptions.fromMap(
      ((request['options'] as Map?) ?? {}).cast<String, Object?>(),
    );
    final rawState = request['state'];
    if (rawState != null && rawState is! Map) {
      throw ArgumentError('Window state must be an object');
    }
    if (owned.route == voiceCallRoute &&
        rawState is Map &&
        rawState['voiceCallFullscreen'] == true) {
      await owned.window.contentCommand('voiceCall.enterFullscreen');
    }
    final state = rawState == null
        ? null
        : {...owned.state, ...Map<String, Object?>.from(rawState as Map)};
    await owned.window.update(
      patch,
      payload: state == null ? null : {'state': state},
    );
    owned.options = owned.options.merge(patch);
    if (state != null) owned.state = state;
  }

  /// Closes the native surface before releasing its plugin-local identity.
  static Future<void> _close(String key, _OwnedFloatingWindow owned) async {
    final stopwatch = Stopwatch()..start();
    final identity =
        'owner=${_owners[key]} id=${_ids[key]} nativeId=${owned.window.id}';
    _log('close.begin $identity');
    _discardRecording(owned);
    await owned.window.close();
    _remove(key);
    _log('close.done $identity elapsedMs=${stopwatch.elapsedMilliseconds}');
  }

  /// Validates a supplied capture interval without rounding or clamping it.
  static int? _duration(
    Map<String, Object?> request,
    String field,
    int min,
    int max,
  ) {
    final value = request[field];
    if (value == null) return null;
    if (value is! int || value < min || value > max) {
      throw ArgumentError('$field must be an integer between $min and $max');
    }
    return value;
  }

  /// Stops capture for a window without waiting on a permission dialog.
  static void _discardRecording(_OwnedFloatingWindow owned) {
    final capture = owned.recording;
    owned.recording = null;
    if (capture != null && identical(_activeRecording, capture)) {
      unawaited(capture.finish(discard: true));
    }
  }

  /// Removes registry entries and stops observers when no windows remain.
  static void _remove(String key) {
    final owned = _windows.remove(key);
    if (owned != null) _discardRecording(owned);
    _owners.remove(key);
    _ids.remove(key);
    if (_windows.isEmpty) {
      _audit?.cancel();
      _audit = null;
      _catalog?.cancel();
      _catalog = null;
    }
  }

  /// Removes handles whose surfaces have already been closed externally.
  static Future<void> _prune() async {
    if (_windows.isEmpty) return;
    final live = await FloatingWindows.liveIds();
    for (final entry in _windows.entries.toList()) {
      if (!live.contains(entry.value.window.id)) {
        _log(
          'prune owner=${_owners[entry.key]} id=${_ids[entry.key]} '
          'nativeId=${entry.value.window.id} reason=native-window-absent',
        );
        _remove(entry.key);
      }
    }
  }

  /// Releases windows belonging to disabled or uninstalled packages.
  static void _startAudit() {
    if (_audit != null) return;

    /// Schedules catalog cleanup in the same lifecycle queue as plugin commands.
    void reconcile() {
      final work = _tail.then((_) async {
        await _prune();
        if (_windows.isEmpty) return;
        final enabled =
            (await _clients.application
                    .packageManager()
                    .getEnabledToolPkgContainerRuntimes())
                .map((plugin) => plugin.packageName)
                .toSet();
        for (final entry in _windows.entries.toList()) {
          if (!enabled.contains(_owners[entry.key])) {
            _discardRecording(entry.value);
            await entry.value.window.close();
            _remove(entry.key);
          }
        }
      });
      _tail = work.catchError((Object error, StackTrace stack) {
        FlutterError.reportError(
          FlutterErrorDetails(
            exception: error,
            stack: stack,
            library: 'floating windows',
            context: ErrorDescription('reconciling plugin windows'),
          ),
        );
      });
    }

    _audit = Timer.periodic(const Duration(seconds: 5), (_) => reconcile());
    _catalog = ToolPkgCatalogChangeBus.listen(reconcile);
  }
}
