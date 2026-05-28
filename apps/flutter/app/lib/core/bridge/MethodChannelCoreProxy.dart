// ignore_for_file: file_names

import 'dart:convert';

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
  } on MissingPluginException {
    rethrow;
  }
}
