// ignore_for_file: file_names

import 'dart:async';
import 'package:flutter/services.dart';

import '../concurrency/AppWorkers.dart';
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
    final requestJson = request.toJson();
    final requestBytes = await AppWorkers.run(
      () => encodeCoreLink(requestJson),
      debugName: 'core-link-call-encode',
    );
    final responseBytes = await _channel.invokeMethod<Uint8List>(
      'call',
      requestBytes,
    );
    if (responseBytes == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty response',
      );
    }
    final response = await AppWorkers.run(
      () => decodeCoreLinkMap(responseBytes),
      debugName: 'core-link-call-decode',
    );
    final result = response['result'] as Map<String, Object?>;
    if (result.containsKey('Ok')) {
      return result['Ok'];
    }
    if (result.containsKey('Err')) {
      final error = CoreLinkError.fromJson(
        result['Err'] as Map<String, Object?>,
      );
      throw error;
    }
    throw const CoreLinkError(
      code: 'INVALID_RESPONSE',
      message: 'runtime bridge response result is invalid',
    );
  }

  /// Opens a client-owned stream on the local platform carrier.
  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    final responseBytes = await _channel.invokeMethod<Uint8List>(
      'pushOpen',
      encodeCoreLink(request.toJson()),
    );
    if (responseBytes == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime push open returned empty response',
      );
    }
    final response = decodeCoreLinkMap(responseBytes);
    if (response.containsKey('code') && response.containsKey('message')) {
      throw CoreLinkError.fromJson(response);
    }
    final pushId = response['pushId'] as String;
    return _MethodChannelCorePushSink(channel: _channel, pushId: pushId);
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async {
    final responseBytes = await _channel.invokeMethod<Uint8List>(
      'watchSnapshot',
      encodeCoreLink(request.toJson()),
    );
    if (responseBytes == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty watch response',
      );
    }
    final response = decodeCoreLinkMap(responseBytes);
    if (response.containsKey('code') && response.containsKey('message')) {
      final error = CoreLinkError.fromJson(response);
      throw error;
    }
    return CoreEvent.fromJson(response);
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
    final subscriptionJson = decodeCoreLinkMap(subscriptionBytes);
    if (subscriptionJson.containsKey('code') &&
        subscriptionJson.containsKey('message')) {
      final error = CoreLinkError.fromJson(subscriptionJson);
      await watchChannel.fail(subscriptionId, error, StackTrace.current);
      throw error;
    }
    if (subscriptionJson['subscriptionId'] != subscriptionId) {
      final error = CoreLinkError(
        code: 'INVALID_RESPONSE',
        message:
            'runtime watch subscription id mismatch: ${subscriptionJson['subscriptionId']}',
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
  void add(Object? args) {
    if (_closed) {
      throw StateError('Link push stream is closed');
    }
    final sequence = _sequence++;
    _tail = _tail.then((_) async {
      final responseBytes = await channel.invokeMethod<Uint8List>(
        'pushItem',
        encodeCoreLink(<String, Object?>{
          'pushId': pushId,
          'sequence': sequence,
          'args': args,
        }),
      );
      if (responseBytes == null) {
        throw const CoreLinkError(
          code: 'EMPTY_RESPONSE',
          message: 'runtime push carrier returned empty response',
        );
      }
      final response = decodeCoreLinkMap(responseBytes);
      if (response.containsKey('code') && response.containsKey('message')) {
        throw CoreLinkError.fromJson(response);
      }
    });
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
    final response = decodeCoreLinkMap(responseBytes);
    if (response.containsKey('code') && response.containsKey('message')) {
      throw CoreLinkError.fromJson(response);
    }
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
        unawaited(
          _channel.invokeMethod<Uint8List>('closeWatchStream', subscriptionId),
        );
      }
    } catch (error, stackTrace) {
      _failAll(error, stackTrace);
    }
  }

  void _failAll(Object error, StackTrace stackTrace) {
    final entries = _controllers.entries.toList(growable: false);
    _controllers.clear();
    for (final entry in entries) {
      unawaited(
        _channel.invokeMethod<Uint8List>('closeWatchStream', entry.key),
      );
      final controller = entry.value;
      controller.addError(error, stackTrace);
      controller.onCancel = null;
      unawaited(controller.close());
    }
  }

  Future<void> _closeSubscription(String subscriptionId) async {
    _controllers.remove(subscriptionId);
    await _channel.invokeMethod<Uint8List>('closeWatchStream', subscriptionId);
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
  final frame = decodeCoreLinkMap(frameBytes);
  return _MethodChannelWatchFrame(
    subscriptionId: frame['subscriptionId'] as String,
    event: CoreEvent.fromJson(frame['event'] as Map<String, Object?>),
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
      encodeCoreLink(<String, Object?>{
        'channelId': 'method-channel-watch',
        'subscriptionId': subscriptionId,
        'request': request.toJson(),
      }),
    );
  } on MissingPluginException {
    rethrow;
  }
}
