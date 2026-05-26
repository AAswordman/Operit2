// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/foundation.dart';
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
    debugPrint(
      '[OperitCoreProxy] call -> ${request.targetPath.key}.${request.methodName} '
      'id=${request.requestId} args=${_argKeys(request.args)}',
    );
    final responseText = await _channel.invokeMethod<String>('call', requestText);
    if (responseText == null) {
      debugPrint('[OperitCoreProxy] call <- empty id=${request.requestId}');
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty response',
      );
    }
    final response = jsonDecode(responseText) as Map<String, Object?>;
    final result = response['result'] as Map<String, Object?>;
    if (result.containsKey('Ok')) {
      debugPrint(
        '[OperitCoreProxy] call <- ok ${request.targetPath.key}.${request.methodName} '
        'id=${request.requestId} type=${result['Ok'].runtimeType}',
      );
      return result['Ok'];
    }
    if (result.containsKey('Err')) {
      final error = CoreLinkError.fromJson(result['Err'] as Map<String, Object?>);
      debugPrint(
        '[OperitCoreProxy] call <- err ${request.targetPath.key}.${request.methodName} '
        'id=${request.requestId} $error',
      );
      throw error;
    }
    debugPrint('[OperitCoreProxy] call <- invalid id=${request.requestId}');
    throw const CoreLinkError(
      code: 'INVALID_RESPONSE',
      message: 'runtime bridge response result is invalid',
    );
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async {
    debugPrint(
      '[OperitCoreProxy] watchSnapshot -> ${request.targetPath.key}.${request.propertyName} '
      'id=${request.requestId} args=${_argKeys(request.args)}',
    );
    final responseText = await _channel.invokeMethod<String>(
      'watchSnapshot',
      jsonEncode(request.toJson()),
    );
    if (responseText == null) {
      debugPrint('[OperitCoreProxy] watchSnapshot <- empty id=${request.requestId}');
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty watch response',
      );
    }
    final response = jsonDecode(responseText) as Map<String, Object?>;
    if (response.containsKey('code') && response.containsKey('message')) {
      final error = CoreLinkError.fromJson(response);
      debugPrint(
        '[OperitCoreProxy] watchSnapshot <- err ${request.targetPath.key}.${request.propertyName} '
        'id=${request.requestId} $error',
      );
      throw error;
    }
    final event = CoreEvent.fromJson(response);
    debugPrint(
      '[OperitCoreProxy] watchSnapshot <- ${event.kind} '
      '${request.targetPath.key}.${request.propertyName} id=${request.requestId}',
    );
    return event;
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    debugPrint(
      '[OperitCoreProxy] watchStream -> ${request.targetPath.key}.${request.propertyName} '
      'id=${request.requestId} args=${_argKeys(request.args)}',
    );
    final subscriptionText = await _invokeWatchStream(_channel, request);
    if (subscriptionText == null) {
      debugPrint('[OperitCoreProxy] watchStream <- empty id=${request.requestId}');
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
      debugPrint(
        '[OperitCoreProxy] watchStream <- err ${request.targetPath.key}.${request.propertyName} '
        'id=${request.requestId} $error',
      );
      throw error;
    }
    final subscriptionId = subscriptionJson['subscriptionId'] as String;
    debugPrint(
      '[OperitCoreProxy] watchStream subscribed id=${request.requestId} '
      'subscription=$subscriptionId',
    );
    var completed = false;
    try {
      while (!completed) {
        await Future<void>.delayed(const Duration(milliseconds: 24));
        final eventsText = await _channel.invokeMethod<String>(
          'pollWatchStream',
          subscriptionId,
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
        final eventsJson = decodedEvents as List<Object?>;
        if (eventsJson.isNotEmpty) {
          debugPrint(
            '[OperitCoreProxy] watchStream poll subscription=$subscriptionId '
            'events=${eventsJson.length}',
          );
        }
        for (final eventJson in eventsJson.cast<Map<String, Object?>>()) {
          final event = CoreEvent.fromJson(eventJson);
          yield event;
          if (event.kind == 'Completed') {
            completed = true;
          }
        }
      }
    } finally {
      await _channel.invokeMethod<String>('closeWatchStream', subscriptionId);
      debugPrint('[OperitCoreProxy] watchStream closed subscription=$subscriptionId');
    }
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

Future<String?> _invokeWatchStream(
  MethodChannel channel,
  CoreWatchRequest request,
) async {
  try {
    return await channel.invokeMethod<String>(
      'watchStream',
      jsonEncode(request.toJson()),
    );
  } on MissingPluginException catch (error) {
    debugPrint(
      '[OperitCoreProxy] watchStream missing native method. '
      'The current Flutter runner was built without operit/runtime.watchStream. '
      'A full native rebuild/restart is required. $error',
    );
    rethrow;
  }
}

String _argKeys(Object? args) {
  if (args is Map<String, Object?>) {
    return args.keys.join(',');
  }
  return args.runtimeType.toString();
}
