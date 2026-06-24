// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/services.dart';

import '../link/CoreLinkProtocol.dart';
import 'CoreProxy.dart';

class MethodChannelCoreProxy extends CoreProxy {
  const MethodChannelCoreProxy({
    MethodChannel channel = const MethodChannel('operit/runtime'),
  }) : _channel = channel;

  final MethodChannel _channel;

  @override
  Future<Object?> call(CoreCallRequest request) async {
    final requestText = jsonEncode(request.toJson());
    final responseText = await _channel.invokeMethod<String>(
      'call',
      requestText,
    );
    if (responseText == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty response',
      );
    }
    final response = jsonDecode(responseText) as Map<String, Object?>;
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

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async {
    final responseText = await _channel.invokeMethod<String>(
      'watchSnapshot',
      jsonEncode(request.toJson()),
    );
    if (responseText == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty watch response',
      );
    }
    final response = jsonDecode(responseText) as Map<String, Object?>;
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
    final String? subscriptionText;
    try {
      subscriptionText = await _invokeWatchStream(
        _channel,
        subscriptionId,
        request,
      );
    } catch (error, stackTrace) {
      await watchChannel.fail(subscriptionId, error, stackTrace);
      rethrow;
    }
    if (subscriptionText == null) {
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
    final subscriptionJson =
        jsonDecode(subscriptionText) as Map<String, Object?>;
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
        final frameText = call.arguments as String;
        _dispatch(frameText);
        return null;
      default:
        throw MissingPluginException(
          'No implementation found for method ${call.method} on channel ${_channel.name}',
        );
    }
  }

  void _dispatch(String frameText) {
    try {
      final frame = jsonDecode(frameText) as Map<String, Object?>;
      final subscriptionId = frame['subscriptionId'] as String;
      final controller = _controllers[subscriptionId];
      if (controller == null) {
        return;
      }
      final event = CoreEvent.fromJson(frame['event'] as Map<String, Object?>);
      controller.add(event);
      if (event.kind == 'Completed') {
        _controllers.remove(subscriptionId);
        controller.onCancel = null;
        unawaited(controller.close());
        unawaited(_channel.invokeMethod<String>(
          'closeWatchStream',
          subscriptionId,
        ));
      }
    } catch (error, stackTrace) {
      _failAll(error, stackTrace);
    }
  }

  void _failAll(Object error, StackTrace stackTrace) {
    final entries = _controllers.entries.toList(growable: false);
    _controllers.clear();
    for (final entry in entries) {
      unawaited(_channel.invokeMethod<String>('closeWatchStream', entry.key));
      final controller = entry.value;
      controller.addError(error, stackTrace);
      controller.onCancel = null;
      unawaited(controller.close());
    }
  }

  Future<void> _closeSubscription(String subscriptionId) async {
    _controllers.remove(subscriptionId);
    await _channel.invokeMethod<String>('closeWatchStream', subscriptionId);
  }
}

Future<String?> _invokeWatchStream(
  MethodChannel channel,
  String subscriptionId,
  CoreWatchRequest request,
) async {
  try {
    return await channel.invokeMethod<String>(
      'watchStream',
      jsonEncode(<String, Object?>{
        'channelId': 'method-channel-watch',
        'subscriptionId': subscriptionId,
        'request': request.toJson(),
      }),
    );
  } on MissingPluginException {
    rethrow;
  }
}
