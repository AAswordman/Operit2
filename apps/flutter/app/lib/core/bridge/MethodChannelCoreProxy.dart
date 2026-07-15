// ignore_for_file: file_names

import 'dart:async';
import 'package:flutter/services.dart';

import '../link/CoreLinkProtocol.dart';
import '../link/CoreLinkCodec.dart';
import 'CoreProxy.dart';

class MethodChannelCoreProxy extends CoreProxy {
  const MethodChannelCoreProxy({
    MethodChannel channel = const MethodChannel('operit/runtime'),
  }) : _channel = channel;

  final MethodChannel _channel;

  @override
  Future<Object?> call(CoreCallRequest request) async {
    final responseBytes = await _channel.invokeMethod<Uint8List>(
      'call',
      encodeNativeCoreCallRequest(request),
    );
    if (responseBytes == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty response',
      );
    }
    return decodeNativeCoreResult(responseBytes);
  }

  /// Opens a client-owned stream on the local platform carrier.
  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    final responseBytes = await _channel.invokeMethod<Uint8List>(
      'pushOpen',
      encodeNativeCorePushOpenRequest(request),
    );
    if (responseBytes == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime push open returned empty response',
      );
    }
    final pushId = decodeNativeCorePushOpenResult(responseBytes);
    return _MethodChannelCorePushSink(channel: _channel, pushId: pushId);
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async {
    final responseBytes = await _channel.invokeMethod<Uint8List>(
      'watchSnapshot',
      encodeNativeCoreWatchSnapshotRequest(request),
    );
    if (responseBytes == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty watch response',
      );
    }
    return decodeNativeCoreWatchSnapshotResult(responseBytes);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    final watchChannel = _methodChannelWatchChannel(_channel);
    final subscriptionId = watchChannel.nextSubscriptionId();
    final events = watchChannel.attach(subscriptionId);
    final Uint8List? subscriptionBytes;
    try {
      subscriptionBytes = await _invokeWatchStream(
        _channel,
        subscriptionId,
        request,
      );
    } catch (error, stackTrace) {
      await watchChannel.fail(subscriptionId, error, stackTrace);
      rethrow;
    }
    if (subscriptionBytes == null) {
      await watchChannel.fail(
        subscriptionId,
        const CoreLinkError(
          code: 'EMPTY_RESPONSE',
          message: 'runtime bridge returned empty stream subscription',
        ),
        StackTrace.current,
      );
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty stream subscription',
      );
    }
    final String openedSubscriptionId;
    try {
      openedSubscriptionId = decodeNativeCoreWatchStreamResult(
        subscriptionBytes,
      );
    } catch (error, stackTrace) {
      await watchChannel.fail(subscriptionId, error, stackTrace);
      rethrow;
    }
    if (openedSubscriptionId != subscriptionId) {
      final error = CoreLinkError(
        code: 'INVALID_RESPONSE',
        message:
            'runtime watch subscription id mismatch: $openedSubscriptionId',
      );
      await watchChannel.fail(subscriptionId, error, StackTrace.current);
      throw error;
    }
    yield* events;
  }
}

class _MethodChannelCorePushSink implements CorePushSink {
  /// Creates one ordered local push stream.
  _MethodChannelCorePushSink({required this.channel, required this.pushId});

  final MethodChannel channel;
  final String pushId;
  Future<void> _tail = Future<void>.value();
  int _sequence = 0;
  bool _closed = false;

  /// Queues one item for ordered local dispatch.
  @override
  Future<void> add(Object? args) {
    if (_closed) {
      throw StateError('Link push stream is closed');
    }
    final sequence = _sequence++;
    _tail = _tail.then((_) async {
      final responseBytes = await channel.invokeMethod<Uint8List>(
        'pushItem',
        encodeNativeCorePushItem(pushId, sequence, args),
      );
      if (responseBytes == null) {
        throw const CoreLinkError(
          code: 'EMPTY_RESPONSE',
          message: 'runtime push carrier returned empty response',
        );
      }
      decodeNativeCoreVoidResult(responseBytes);
    });
    return _tail;
  }

  /// Waits for all local items and completes this stream.
  @override
  Future<void> close() async {
    if (_closed) {
      return;
    }
    _closed = true;
    await _tail;
    final responseBytes = await channel.invokeMethod<Uint8List>(
      'pushClose',
      pushId,
    );
    if (responseBytes == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime push close returned empty response',
      );
    }
    decodeNativeCoreVoidResult(responseBytes);
  }
}

final Map<MethodChannel, _MethodChannelWatchChannel>
_methodChannelWatchChannels = <MethodChannel, _MethodChannelWatchChannel>{};

_MethodChannelWatchChannel _methodChannelWatchChannel(MethodChannel channel) {
  return _methodChannelWatchChannels.putIfAbsent(
    channel,
    () => _MethodChannelWatchChannel(channel),
  );
}

class _MethodChannelWatchChannel {
  _MethodChannelWatchChannel(this._channel) {
    _channel.setMethodCallHandler(_handleMethodCall);
  }

  final MethodChannel _channel;
  final Map<String, StreamController<CoreEvent>> _controllers =
      <String, StreamController<CoreEvent>>{};
  Future<void> _dispatchTail = Future<void>.value();
  int _nextSubscriptionIndex = 0;

  String nextSubscriptionId() {
    return 'method-channel-watch-${DateTime.now().microsecondsSinceEpoch}-${_nextSubscriptionIndex++}';
  }

  Stream<CoreEvent> attach(String subscriptionId) {
    final controller = StreamController<CoreEvent>();
    controller.onCancel = () async {
      await _closeSubscription(subscriptionId);
    };
    _controllers[subscriptionId] = controller;
    return controller.stream;
  }

  Future<void> fail(
    String subscriptionId,
    Object error,
    StackTrace stackTrace,
  ) async {
    final controller = _controllers.remove(subscriptionId);
    if (controller == null) {
      return;
    }
    controller.onCancel = null;
    controller.addError(error, stackTrace);
    await controller.close();
  }

  Future<Object?> _handleMethodCall(MethodCall call) async {
    switch (call.method) {
      case 'watchChannelEvent':
        final frameBytes = call.arguments as Uint8List;
        _dispatchTail = _dispatchTail.then((_) => _dispatch(frameBytes));
        unawaited(_dispatchTail);
        return null;
      default:
        throw MissingPluginException(
          'No implementation found for method ${call.method} on channel ${_channel.name}',
        );
    }
  }

  Future<void> _dispatch(Uint8List frameBytes) async {
    try {
      final frame = _parseMethodChannelWatchFrame(frameBytes);
      final subscriptionId = frame.subscriptionId;
      final controller = _controllers[subscriptionId];
      if (controller == null) {
        return;
      }
      final event = frame.event;
      controller.add(event);
      if (event.kind == 'Completed') {
        _controllers.remove(subscriptionId);
        controller.onCancel = null;
        unawaited(controller.close());
        unawaited(_closeNativeWatchStream(_channel, subscriptionId));
      }
    } catch (error, stackTrace) {
      _failAll(error, stackTrace);
    }
  }

  void _failAll(Object error, StackTrace stackTrace) {
    final entries = _controllers.entries.toList(growable: false);
    _controllers.clear();
    for (final entry in entries) {
      unawaited(_closeNativeWatchStream(_channel, entry.key));
      final controller = entry.value;
      controller.addError(error, stackTrace);
      controller.onCancel = null;
      unawaited(controller.close());
    }
  }

  Future<void> _closeSubscription(String subscriptionId) async {
    _controllers.remove(subscriptionId);
    await _closeNativeWatchStream(_channel, subscriptionId);
  }
}

class _MethodChannelWatchFrame {
  const _MethodChannelWatchFrame({
    required this.subscriptionId,
    required this.event,
  });

  final String subscriptionId;
  final CoreEvent event;
}

_MethodChannelWatchFrame _parseMethodChannelWatchFrame(Uint8List frameBytes) {
  final frame = decodeNativeCoreWatchFrame(frameBytes);
  return _MethodChannelWatchFrame(
    subscriptionId: frame.subscriptionId,
    event: frame.event,
  );
}

Future<Uint8List?> _invokeWatchStream(
  MethodChannel channel,
  String subscriptionId,
  CoreWatchRequest request,
) async {
  try {
    return await channel.invokeMethod<Uint8List>(
      'watchStream',
      encodeNativeCoreWatchStreamRequest(subscriptionId, request),
    );
  } on MissingPluginException {
    rethrow;
  }
}

/// Closes one native watch subscription and validates its direct acknowledgement.
Future<void> _closeNativeWatchStream(
  MethodChannel channel,
  String subscriptionId,
) async {
  final responseBytes = await channel.invokeMethod<Uint8List>(
    'closeWatchStream',
    subscriptionId,
  );
  if (responseBytes == null) {
    throw const CoreLinkError(
      code: 'EMPTY_RESPONSE',
      message: 'runtime watch close returned empty response',
    );
  }
  decodeNativeCoreVoidResult(responseBytes);
}
