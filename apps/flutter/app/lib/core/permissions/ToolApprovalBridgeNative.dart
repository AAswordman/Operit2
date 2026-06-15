// ignore_for_file: file_names

import 'package:flutter/services.dart';

import '../runtime/RuntimeConnectionManager.dart';
import '../bridge/RemoteCoreProxy.dart';
import 'ToolApprovalModels.dart';

class ToolApprovalBridge {
  const ToolApprovalBridge({
    MethodChannel channel = const MethodChannel('operit/runtime'),
  }) : _channel = channel;

  final MethodChannel _channel;
  static final Map<String, int> _remoteRequestedAtMillis = <String, int>{};

  Future<ToolApprovalRequest?> currentPermissionRequest() async {
    if (RuntimeConnectionManager.instance.config.mode ==
        RuntimeConnectionMode.remote) {
      return _currentRemotePermissionRequest();
    }
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

  Future<void> respondPermissionRequest(
    ToolApprovalRequest request,
    ToolApprovalResult result,
  ) async {
    final remoteRequestId = request.remoteRequestId;
    if (remoteRequestId == null) {
      await handlePermissionResult(result);
      return;
    }
    final remoteProxy =
        RuntimeConnectionManager.instance.coreProxy as RemoteCoreProxy;
    await remoteProxy.respondHostInteraction(
      requestId: remoteRequestId,
      result: _remoteResultName(result),
    );
    _remoteRequestedAtMillis.remove(remoteRequestId);
  }

  Future<ToolApprovalRequest?> _currentRemotePermissionRequest() async {
    final remoteProxy =
        RuntimeConnectionManager.instance.coreProxy as RemoteCoreProxy;
    final request = await remoteProxy.pollHostInteraction(timeoutMs: 500);
    if (request == null) {
      return null;
    }
    if (request.kind != 'tool_permission') {
      return null;
    }
    final payload = request.payload;
    final toolJson = payload['tool'];
    final description = payload['description'];
    if (toolJson is! Map<String, Object?> || description is! String) {
      await remoteProxy.respondHostInteraction(
        requestId: request.requestId,
        result: 'deny',
      );
      _remoteRequestedAtMillis.remove(request.requestId);
      return null;
    }
    return ToolApprovalRequest(
      tool: ToolApprovalTool.fromJson(toolJson),
      description: description,
      requestedAtMillis: _remoteRequestedAtMillis.putIfAbsent(
        request.requestId,
        () => DateTime.now().millisecondsSinceEpoch,
      ),
      remoteRequestId: request.requestId,
    );
  }
}

String _remoteResultName(ToolApprovalResult result) {
  return switch (result) {
    ToolApprovalResult.allow => 'allow',
    ToolApprovalResult.deny => 'deny',
    ToolApprovalResult.alwaysAllow => 'always_allow',
  };
}
