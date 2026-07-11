// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';

import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import 'ToolApprovalModels.dart';

class ToolApprovalBridge {
  const ToolApprovalBridge();

  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );
  static final Map<String, int> _requestedAtMillis = <String, int>{};
  static final Map<String, ToolApprovalRequest> _requests =
      <String, ToolApprovalRequest>{};
  static StreamSubscription<RuntimeHostInteractionRequest>? _subscription;

  Future<ToolApprovalRequest?> currentPermissionRequest() async {
    _ensureSubscription();
    if (_requests.isEmpty) {
      return null;
    }
    return _requests.values.first;
  }

  Future<void> handlePermissionResult(ToolApprovalResult result) async {
    throw StateError(
      'permission result must be sent with respondPermissionRequest',
    );
  }

  Future<void> respondPermissionRequest(
    ToolApprovalRequest request,
    ToolApprovalResult result,
  ) async {
    final requestId = request.remoteRequestId;
    if (requestId == null) {
      throw StateError('permission request is missing request id');
    }
    await _clients.servicesRuntimeHostInteractionService
        .respondOwnerHostInteraction(
          requestId: requestId,
          response: RuntimeHostInteractionResponse(
            browserAutomation: null,
            browserSession: null,
            webVisit: null,
            composeWebViewController: null,
            systemCaptureScreenshot: null,
            systemLanguageCode: null,
            systemRecognizeText: null,
            audioPlay: null,
            musicPlayback: null,
            bluetooth: null,
            ttsSynthesis: null,
            ttsPlayback: null,
            toolPermission: RuntimeHostInteractionToolPermissionResponse(
              result: _resultName(result),
            ),
          ),
        );
    _requests.remove(requestId);
    _requestedAtMillis.remove(requestId);
  }

  static void _ensureSubscription() {
    if (_subscription != null) {
      return;
    }
    _subscription = _clients.servicesRuntimeHostInteractionService
        .ownerHostInteractionEventsChanges(
          kinds: <RuntimeHostInteractionKind>[
            RuntimeHostInteractionKind.toolPermission,
          ],
        )
        .listen(
          (event) => unawaited(_handleEvent(event)),
          onError: (Object error, StackTrace stackTrace) {
            FlutterError.reportError(
              FlutterErrorDetails(
                exception: error,
                stack: stackTrace,
                library: 'tool approval bridge',
                context: ErrorDescription('listening tool permission stream'),
              ),
            );
          },
        );
  }

  static Future<void> _handleEvent(
    RuntimeHostInteractionRequest request,
  ) async {
    try {
      final payload = request.toolPermission;
      if (payload == null) {
        throw StateError('tool permission payload is missing');
      }
      _requests[request.requestId] = ToolApprovalRequest(
        tool: ToolApprovalTool.fromHostPayload(payload.tool),
        description: payload.description,
        requestedAtMillis: _requestedAtMillis.putIfAbsent(
          request.requestId,
          () => DateTime.now().millisecondsSinceEpoch,
        ),
        remoteRequestId: request.requestId,
      );
    } catch (error, stackTrace) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'tool approval bridge',
          context: ErrorDescription('handling tool permission stream event'),
        ),
      );
    }
  }
}

String _resultName(ToolApprovalResult result) {
  return switch (result) {
    ToolApprovalResult.allow => 'allow',
    ToolApprovalResult.deny => 'deny',
    ToolApprovalResult.allowSession => 'allow_session',
  };
}
