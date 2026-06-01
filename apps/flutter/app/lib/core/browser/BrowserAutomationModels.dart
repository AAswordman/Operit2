// ignore_for_file: file_names

import 'dart:convert';

class BrowserAutomationRequest {
  const BrowserAutomationRequest({
    required this.requestId,
    required this.toolName,
    required this.chatId,
    required this.parameters,
  });

  final String requestId;
  final String toolName;
  final String chatId;
  final Map<String, String> parameters;

  static BrowserAutomationRequest? decode(String? responseText) {
    if (responseText == null || responseText == 'null') {
      return null;
    }
    final json = jsonDecode(responseText) as Map<String, Object?>;
    final parametersText = json['parametersJson'] as String;
    final parametersJson = jsonDecode(parametersText) as Map<String, Object?>;
    return BrowserAutomationRequest(
      requestId: json['requestId'] as String,
      toolName: json['toolName'] as String,
      chatId: json['chatId'] as String,
      parameters: parametersJson.map(
        (key, value) => MapEntry<String, String>(key, value as String),
      ),
    );
  }
}

class BrowserAutomationResponse {
  const BrowserAutomationResponse({
    required this.requestId,
    required this.success,
    required this.result,
    this.error,
  });

  final String requestId;
  final bool success;
  final String result;
  final String? error;

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'requestId': requestId,
      'success': success,
      'result': result,
      'error': error,
    };
  }
}
