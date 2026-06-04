// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/services.dart';

import 'WebVisitModels.dart';

class WebVisitBridge {
  const WebVisitBridge({
    MethodChannel channel = const MethodChannel('operit/runtime'),
  }) : _channel = channel;

  final MethodChannel _channel;

  Future<WebVisitRequest?> nextRequest() async {
    final responseText = await _channel.invokeMethod<String>(
      'nextWebVisitRequest',
    );
    return WebVisitRequest.decode(responseText);
  }

  Future<void> handleResult(WebVisitResponse response) async {
    await _channel.invokeMethod<String>(
      'handleWebVisitResult',
      jsonEncode(response.toJson()),
    );
  }
}
