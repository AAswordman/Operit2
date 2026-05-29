// ignore_for_file: file_names

import 'package:flutter/services.dart';

import 'ToolApprovalModels.dart';

class ToolApprovalBridge {
  const ToolApprovalBridge({
    MethodChannel channel = const MethodChannel('operit/runtime'),
  }) : _channel = channel;

  final MethodChannel _channel;

  Future<ToolApprovalRequest?> currentPermissionRequest() async {
    final responseText = await _channel.invokeMethod<String>(
      'currentPermissionRequest',
    );
    return ToolApprovalRequest.decode(responseText);
  }

  Future<void> handlePermissionResult(ToolApprovalResult result) async {
    await _channel.invokeMethod<String>(
      'handlePermissionResult',
      result.wireName,
    );
  }
}
