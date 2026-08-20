// ignore_for_file: file_names

import 'dart:async';
import 'dart:collection';
import 'dart:typed_data';

import '../link/CoreLinkCodec.dart';
import '../link/CoreLinkProtocol.dart';
import 'CoreProxy.dart';
import 'OperitRuntimeBridge.dart';
import 'PlatformCoreProxy.dart';

class ProxyCoreRuntimeBridge extends OperitRuntimeBridge {
  const ProxyCoreRuntimeBridge({CoreProxy? coreProxy})
    : _coreProxyOverride = coreProxy;

  static final Map<CoreProxy, Map<String, WeakReference<_EmbeddedCoreStream<dynamic>>>>
  _embeddedStreamsByProxy =
      HashMap<CoreProxy,
        Map<String, WeakReference<_EmbeddedCoreStream<dynamic>>>>.identity();

  final CoreProxy? _coreProxyOverride;

  /// Returns the local platform proxy unless a caller explicitly supplies one.
  CoreProxy get _coreProxy => _coreProxyOverride ?? platformCoreProxy;

  /// Sends one Core call through the selected platform proxy without decoding its payload.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    return _coreProxy.callBytes(request);
  }

  /// Sends one control call through the selected platform proxy without decoding its payload.
  @override
  Future<Uint8List> callControlBytes(CoreCallRequest request) {
    return _coreProxy.callControlBytes(request);
  }

  /// Opens a client-owned Link input stream.
  @override
  Future<CorePushSink> push(CorePushRequest request) {
    return _coreProxy.push(request);
  }

  /// Opens one embedded Core stream and decodes each payload directly.
  @override
  Stream<T> openEmbeddedCoreStream<T>(
    String streamId,
    CoreObjectPath targetPath,
    String propertyName,
    Object? args,
    T Function(CoreLinkValueReader reader) decode,
  ) {
    final cache = _embeddedStreamsByProxy.putIfAbsent(
      _coreProxy,
      () => <String, WeakReference<_EmbeddedCoreStream<dynamic>>>{},
    );
    _removeCollectedEmbeddedStreams(cache);
    final key = streamId;
    final cached = cache[key]?.target;
    if (cached != null) {
      return cached.stream as Stream<T>;
    }
    cache.remove(key);

    final stream = _EmbeddedCoreStream<T>(
      () => _coreProxy.watchStream(
        CoreWatchRequest(
          requestId:
              'embedded-core-stream-${DateTime.now().microsecondsSinceEpoch}',
          targetPath: targetPath,
          propertyName: propertyName,
          args: args,
        ),
      ),
      (event) {
        final valueBytes = event.valueBytes;
        if (valueBytes == null) {
          throw StateError('Embedded Core stream event has no payload bytes');
        }
        return decodeCoreLink<T>(
          valueBytes,
          decode: decode,
          targetPath: event.targetPath,
          embeddedStreamFactory: openEmbeddedCoreStream,
        );
      },
    );
    cache[key] = WeakReference<_EmbeddedCoreStream<dynamic>>(stream);
    return stream.stream;
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) {
    return _coreProxy.watchSnapshot(request);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    return _coreProxy.watchStream(request);
  }
}
/// Removes embedded stream entries whose logical message no longer owns them.
void _removeCollectedEmbeddedStreams(
  Map<String, WeakReference<_EmbeddedCoreStream<dynamic>>> cache,
) {
  cache.removeWhere((_, reference) => reference.target == null);
}
/// Keeps one stable client-side stream proxy for one Core watch source.
class _EmbeddedCoreStream<T> {
  _EmbeddedCoreStream(this._open, this._decode);

  final Stream<CoreEvent> Function() _open;
  final T Function(CoreEvent event) _decode;
  final StreamController<T> _events = StreamController<T>.broadcast(sync: true);
  final List<T> _replay = <T>[];
  Object? _terminalError;
  StackTrace? _terminalStackTrace;
  var _started = false;
  var _done = false;

  /// Exposes one broadcast stream that preserves all events for late listeners.
  late final Stream<T> stream = Stream<T>.multi(_listen, isBroadcast: true);

  /// Attaches one listener and replays the stream's already received events.
  void _listen(MultiStreamController<T> controller) {
    if (_done) {
      _replayTo(controller);
      _finishListener(controller);
      return;
    }

    final subscription = _events.stream.listen(
      controller.add,
      onError: (Object error, StackTrace stackTrace) {
        controller.addError(error, stackTrace);
      },
      onDone: controller.close,
    );
    controller.onCancel = subscription.cancel;
    _replayTo(controller);
    _start();
  }

  /// Starts the physical Core watch once, on the first UI subscription.
  void _start() {
    if (_started) {
      return;
    }
    _started = true;
    try {
      _open().listen(
        _handleEvent,
        onError: (Object error, StackTrace stackTrace) {
          _fail(error, stackTrace);
        },
        onDone: _complete,
      );
    } catch (error, stackTrace) {
      _fail(error, stackTrace);
    }
  }

  /// Decodes and publishes one physical Core event to every local listener.
  void _handleEvent(CoreEvent event) {
    if (event.kind == 'Completed') {
      _complete();
      return;
    }
    try {
      final value = _decode(event);
      _replay.add(value);
      _events.add(value);
    } catch (error, stackTrace) {
      _fail(error, stackTrace);
    }
  }

  /// Replays values already received before a listener was attached.
  void _replayTo(MultiStreamController<T> controller) {
    for (final value in _replay) {
      controller.add(value);
    }
  }

  /// Completes one listener after replaying a terminal stream state.
  void _finishListener(MultiStreamController<T> controller) {
    final error = _terminalError;
    if (error != null) {
      controller.addError(error, _terminalStackTrace ?? StackTrace.current);
    }
    controller.close();
  }

  /// Marks the logical stream complete and closes all active listeners.
  void _complete() {
    if (_done) {
      return;
    }
    _done = true;
    unawaited(_events.close());
  }

  /// Publishes one terminal stream error and closes all active listeners.
  void _fail(Object error, StackTrace stackTrace) {
    if (_done) {
      return;
    }
    _terminalError = error;
    _terminalStackTrace = stackTrace;
    _done = true;
    _events.addError(error, stackTrace);
    unawaited(_events.close());
  }
}
