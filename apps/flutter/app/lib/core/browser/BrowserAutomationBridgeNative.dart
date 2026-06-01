// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/services.dart';

import 'BrowserAutomationModels.dart';

class BrowserAutomationBridge {
  const BrowserAutomationBridge({
    MethodChannel channel = const MethodChannel('operit/runtime'),
  }) : _channel = channel;

  final MethodChannel _channel;

  Future<BrowserAutomationRequest?> nextRequest() async {
    final responseText = await _channel.invokeMethod<String>(
      'nextBrowserAutomationRequest',
    );
    return BrowserAutomationRequest.decode(responseText);
  }

  Future<void> handleResult(BrowserAutomationResponse response) async {
    await _channel.invokeMethod<String>(
      'handleBrowserAutomationResult',
      jsonEncode(response.toJson()),
    );
  }
}
