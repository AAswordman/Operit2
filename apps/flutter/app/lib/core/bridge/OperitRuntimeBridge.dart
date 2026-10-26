// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';

import '../link/CoreLinkCodec.dart';
import '../link/CoreLinkProtocol.dart';
import '../link/CoreLinkRequestId.dart';

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
    String target,
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
      debugPrint(
        'CoreStreamTrace dart.embedded.reuse streamId=$streamId '
        'target=$target property=$propertyName done=${cached.isDone}',
      );
      return cached.stream as Stream<T>;
    }

    debugPrint(
      'CoreStreamTrace dart.embedded.create streamId=$streamId '
      'target=$target property=$propertyName',
    );
    late final _EmbeddedCoreStream<T> embedded;
    embedded = _EmbeddedCoreStream<T>(
      streamId,
      () {
        final requestId = nextCoreLinkRequestId(prefix: 'embedded-core-stream');
        debugPrint(
          'CoreStreamTrace dart.embedded.open streamId=$streamId '
          'requestId=$requestId target=$target property=$propertyName',
        );
        return watchStream(
          CoreWatchRequest(
            requestId: requestId,
            target: target,
            propertyName: propertyName,
            args: args,
          ),
        );
      },
      (valueBytes, target) {
        return decodeCoreLink<T>(
          valueBytes,
          decode: decode,
          target: target,
          embeddedStreamFactory: openEmbeddedCoreStream,
        );
      },
      // Chat responses are replayable Core watches, not new model requests.
      // Evict only failed chat watches so the next live message descriptor can
      // reopen them; successfully completed streams must keep their identity.
      onFailure: streamId.startsWith('chat-message-stream:')
          ? () {
              if (identical(cache[streamId], embedded)) {
                cache.remove(streamId);
                debugPrint(
                  'CoreStreamTrace dart.embedded.invalidate streamId=$streamId',
                );
              }
            }
          : null,
    );
    cache[streamId] = embedded;
    return embedded.stream;
  }

  /// Calls the application route with an independently allocated request id.
  Future<Object?> callApplication(
    String methodName, {
    Map<String, Object?> args = const {},
  }) {
    return call(
      CoreCallRequest(
        requestId: nextCoreLinkRequestId(),
        target: 'core/application',
        methodName: methodName,
        args: args,
      ),
    );
  }
}

/// Keeps one stable client-side stream proxy for one Core watch source.
class _EmbeddedCoreStream<T> {
  /// Retains the source opener. Each subscriber reads that source directly.
  _EmbeddedCoreStream(
    this._streamId,
    this._open,
    this._decode, {
    VoidCallback? onFailure,
  }) : _onFailure = onFailure;

  final VoidCallback? _onFailure;
  final String _streamId;
  final Stream<CoreEvent> Function() _open;
  final T Function(Uint8List bytes, String target) _decode;
  var _done = false;

  /// Reports whether one physical watch has reached Core Completed.
  bool get isDone => _done;

  /// Exposes one stream identity whose subscribers each open the Core source.
  late final Stream<T> stream = Stream<T>.multi(_listen, isBroadcast: true);

  /// Opens one physical watch for this subscriber and forwards its values.
  void _listen(MultiStreamController<T> controller) {
    final decoder = CoreLinkStreamEventValueDecoder();
    var settled = false;
    debugPrint(
      'CoreStreamTrace dart.embedded.listen streamId=$_streamId source=watch',
    );
    late final StreamSubscription<CoreEvent> subscription;
    try {
      subscription = _open().listen(
        (event) {
          if (settled) {
            return;
          }
          if (event.kind == 'Completed') {
            settled = true;
            _done = true;
            debugPrint(
              'CoreStreamTrace dart.embedded.done streamId=$_streamId '
              'source=watch',
            );
            controller.close();
            unawaited(subscription.cancel());
            return;
          }
          try {
            controller.add(
              decoder.decode<T>(
                event,
                decode: (bytes) => _decode(bytes, event.target),
              ),
            );
          } catch (error, stackTrace) {
            settled = true;
            _failListener(controller, subscription, error, stackTrace);
          }
        },
        onError: (Object error, StackTrace stackTrace) {
          if (settled) {
            return;
          }
          settled = true;
          _failListener(controller, subscription, error, stackTrace);
        },
        onDone: () {
          if (settled) {
            return;
          }
          settled = true;
          if (_onFailure != null) {
            _failListener(
              controller,
              subscription,
              StateError('Core watch closed before Completed'),
              StackTrace.current,
            );
            return;
          }
          _done = true;
          controller.close();
        },
      );
    } catch (error, stackTrace) {
      controller.addError(error, stackTrace);
      _onFailure?.call();
      return;
    }
    controller.onCancel = () {
      return subscription.cancel();
    };
  }

  /// Reports one watch failure and drops the cached source so it can reopen.
  void _failListener(
    MultiStreamController<T> controller,
    StreamSubscription<CoreEvent> subscription,
    Object error,
    StackTrace stackTrace,
  ) {
    debugPrint(
      'CoreStreamTrace dart.embedded.fail streamId=$_streamId error=$error',
    );
    _onFailure?.call();
    controller.addError(error, stackTrace);
    unawaited(subscription.cancel());
  }
}
