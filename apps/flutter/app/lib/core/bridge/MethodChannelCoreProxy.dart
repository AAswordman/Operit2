// ignore_for_file: file_names

import 'dart:convert';
import 'dart:async';

import 'package:flutter/services.dart';

import '../host/HostEnvironmentDescriptor.dart';
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
    final subscriptionText = await _invokeWatchStream(_channel, request);
    if (subscriptionText == null) {
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
      throw error;
    }
    final subscriptionId = subscriptionJson['subscriptionId'] as String;
    yield* _methodChannelWatchPump(_channel).attach(subscriptionId);
  }

  @override
  Future<HostEnvironmentDescriptor> hostDescriptor() async {
    final responseText = await _channel.invokeMethod<String>('hostDescriptor');
    if (responseText == null) {
      throw const CoreLinkError(
        code: 'EMPTY_HOST_DESCRIPTOR',
        message: 'runtime bridge returned empty host descriptor',
      );
    }
    return HostEnvironmentDescriptor.fromJson(
      jsonDecode(responseText) as Map<String, Object?>,
    );
  }
}

final Map<MethodChannel, _MethodChannelWatchPump> _methodChannelWatchPumps =
    <MethodChannel, _MethodChannelWatchPump>{};

_MethodChannelWatchPump _methodChannelWatchPump(MethodChannel channel) {
  return _methodChannelWatchPumps.putIfAbsent(
    channel,
    () => _MethodChannelWatchPump(channel),
  );
}

class _MethodChannelWatchPump {
  _MethodChannelWatchPump(this._channel);

  static const Duration interval = Duration(milliseconds: 24);

  final MethodChannel _channel;
  final Map<String, StreamController<CoreEvent>> _controllers =
      <String, StreamController<CoreEvent>>{};
  Timer? _timer;
  bool _polling = false;

  Stream<CoreEvent> attach(String subscriptionId) {
    final controller = StreamController<CoreEvent>();
    controller.onCancel = () async {
      await _closeSubscription(subscriptionId);
    };
    _controllers[subscriptionId] = controller;
    _timer ??= Timer.periodic(interval, (_) {
      unawaited(_poll());
    });
    unawaited(_poll());
    return controller.stream;
  }

  Future<void> _poll() async {
    if (_polling || _controllers.isEmpty) {
      return;
    }
    _polling = true;
    final subscriptionIds = _controllers.keys.toList(growable: false);
    try {
      final eventsText = await _channel.invokeMethod<String>(
        'pollWatchStreams',
        jsonEncode(subscriptionIds),
      );
      if (eventsText == null) {
        throw const CoreLinkError(
          code: 'EMPTY_RESPONSE',
          message: 'runtime bridge returned empty stream events',
        );
      }
      final decodedEvents = jsonDecode(eventsText);
      if (decodedEvents is Map<String, Object?> &&
          decodedEvents.containsKey('code') &&
          decodedEvents.containsKey('message')) {
        throw CoreLinkError.fromJson(decodedEvents);
      }
      final eventsBySubscription =
          (decodedEvents as Map<String, Object?>).cast<String, Object?>();
      for (final subscriptionId in subscriptionIds) {
        final controller = _controllers[subscriptionId];
        if (controller == null) {
          continue;
        }
        final eventsJson = eventsBySubscription[subscriptionId] as List<Object?>;
        for (final eventJson in eventsJson.cast<Map<String, Object?>>()) {
          final event = CoreEvent.fromJson(eventJson);
          controller.add(event);
          if (event.kind == 'Completed') {
            _controllers.remove(subscriptionId);
            await _channel.invokeMethod<String>(
              'closeWatchStream',
              subscriptionId,
            );
            controller.onCancel = null;
            await controller.close();
            break;
          }
        }
      }
      if (_controllers.isEmpty) {
        _timer?.cancel();
        _timer = null;
      }
    } catch (error, stackTrace) {
      final entries = _controllers.entries.toList(growable: false);
      _controllers.clear();
      _timer?.cancel();
      _timer = null;
      for (final entry in entries) {
        unawaited(
          _channel.invokeMethod<String>('closeWatchStream', entry.key),
        );
        final controller = entry.value;
        controller.addError(error, stackTrace);
        controller.onCancel = null;
        await controller.close();
      }
    } finally {
      _polling = false;
    }
  }

  Future<void> _closeSubscription(String subscriptionId) async {
    _controllers.remove(subscriptionId);
    if (_controllers.isEmpty) {
      _timer?.cancel();
      _timer = null;
    }
    await _channel.invokeMethod<String>('closeWatchStream', subscriptionId);
  }
}

Future<String?> _invokeWatchStream(
  MethodChannel channel,
  CoreWatchRequest request,
) async {
  try {
    return await channel.invokeMethod<String>(
      'watchStream',
      jsonEncode(request.toJson()),
    );
  } on MissingPluginException {
    rethrow;
  }
}
