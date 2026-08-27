// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';

import '../link/CoreLinkCodec.dart';
import '../link/CoreLinkProtocol.dart';

abstract class OperitRuntimeBridge {
  const OperitRuntimeBridge();

  /// Stores stable embedded stream wrappers for each runtime bridge instance.
  static final Expando<Map<String, _EmbeddedCoreStream<dynamic>>>
  _embeddedStreamsByBridge = Expando<Map<String, _EmbeddedCoreStream<dynamic>>>(
    'operit-embedded-core-streams',
  );

  /// Sends one encoded Core call and returns its MessagePack response unchanged.
  Future<Uint8List> callBytes(CoreCallRequest request);

  /// Decodes one Core call through the generic Link value representation.
  Future<Object?> call(CoreCallRequest request) async {
    return decodeNativeCoreResult(await callBytes(request));
  }

  /// Sends one control call and returns its MessagePack response unchanged.
  Future<Uint8List> callControlBytes(CoreCallRequest request) =>
      callBytes(request);

  /// Executes a control call concurrently with serialized runtime work.
  Future<Object?> callControl(CoreCallRequest request) async {
    return decodeNativeCoreResult(await callControlBytes(request));
  }

  /// Opens a client-owned stream targeting one Core method.
  Future<CorePushSink> push(CorePushRequest request);

  /// Reads one watch snapshot without materializing its payload in the bridge.
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request);

  /// Opens one watch stream and forwards raw Core events unchanged.
  Stream<CoreEvent> watchStream(CoreWatchRequest request);

  /// Opens one embedded stream through the generic Core property route.
  Stream<T> openEmbeddedCoreStream<T>(
    String streamId,
    int targetObjectId,
    String propertyName,
    Object? args,
    T Function(CoreLinkValueReader reader) decode,
  ) {
    final cache =
        _embeddedStreamsByBridge[this] ??
        (_embeddedStreamsByBridge[this] =
            <String, _EmbeddedCoreStream<dynamic>>{});
    final cached = cache[streamId];
    if (cached != null) {
      return cached.stream as Stream<T>;
    }

    final stream = _EmbeddedCoreStream<T>(
      streamId,
      () => watchStream(
        CoreWatchRequest(
          requestId:
              'embedded-core-stream-${DateTime.now().microsecondsSinceEpoch}',
          targetObjectId: targetObjectId,
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
          targetObjectId: event.targetObjectId,
          embeddedStreamFactory: openEmbeddedCoreStream,
        );
      },
    );
    cache[streamId] = stream;
    return stream.stream;
  }

  Future<Object?> callApplication(
    String methodName, {
    Map<String, Object?> args = const {},
  }) {
    return call(
      CoreCallRequest(
        requestId: 'flutter-${DateTime.now().microsecondsSinceEpoch}',
        targetObjectId: 0,
        methodName: methodName,
        args: args,
      ),
    );
  }
}

/// Keeps one stable client-side stream proxy for one Core watch source.
class _EmbeddedCoreStream<T> {
  _EmbeddedCoreStream(this._streamId, this._open, this._decode);

  final String _streamId;
  final Stream<CoreEvent> Function() _open;
  final T Function(CoreEvent event) _decode;
  final CoreLinkEventValueDecoder _valueDecoder = CoreLinkEventValueDecoder();
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
    controller.onCancel = () {
      return subscription.cancel();
    };
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
      final completeValueBytes = _valueDecoder.completeValueBytes(event);
      final completeEvent = CoreEvent.raw(
        requestId: event.requestId,
        targetObjectId: event.targetObjectId,
        propertyName: event.propertyName,
        kind: event.kind,
        valueBytes: completeValueBytes,
        decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
      );
      final value = _decode(completeEvent);
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
    debugPrint(
      'CoreStreamTrace dart.embedded.fail streamId=$_streamId error=$error',
    );
    _events.addError(error, stackTrace);
    unawaited(_events.close());
  }
}
