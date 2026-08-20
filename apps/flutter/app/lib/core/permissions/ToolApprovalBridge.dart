// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';

import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import 'ToolApprovalModels.dart';

class ToolApprovalBridge {
  /// Creates a bridge for tool approval requests.
  const ToolApprovalBridge();

  static const int _requestTimeoutMillis = 60 * 1000;

  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(),
  );
  static final Map<String, int> _requestedAtMillis = <String, int>{};
  static final Map<String, ToolApprovalRequest> _requests =
      <String, ToolApprovalRequest>{};
  static StreamSubscription<RuntimeHostInteractionRequest>? _subscription;

  /// Returns the current pending permission request.
  Future<ToolApprovalRequest?> currentPermissionRequest() async {
    _ensureSubscription();
    _removeExpiredRequests(DateTime.now().millisecondsSinceEpoch);
    if (_requests.isEmpty) {
      return null;
    }
    return _requests.values.first;
  }

  /// Rejects direct result handling for remote permission requests.
  Future<void> handlePermissionResult(ToolApprovalResult result) async {
    throw StateError(
      'permission result must be sent with respondPermissionRequest',
    );
  }

  /// Sends one permission decision to the runtime host interaction service.
  Future<void> respondPermissionRequest(
    ToolApprovalRequest request,
    ToolApprovalResult result,
  ) async {
    final requestId = request.remoteRequestId;
    if (requestId == null) {
      throw StateError('permission request is missing request id');
    }
    try {
      await _clients.servicesRuntimeHostInteractionService
          .respondOwnerHostInteraction(
            requestId: requestId,
            response: RuntimeHostInteractionResponse(
              error: null,
              browserAutomation: null,
              browserSession: null,
              webVisit: null,
              composeWebViewController: null,
              composeFilePicker: null,
              systemCaptureScreenshot: null,
              systemLanguageCode: null,
              systemRecognizeText: null,
              systemOperation: null,
              fileOpen: null,
              fileShare: null,
              audioPlay: null,
              musicPlayback: null,
              bluetooth: null,
              ttsSynthesis: null,
              ttsPlayback: null,
              localInference: null,
              toolPermission: RuntimeHostInteractionToolPermissionResponse(
                result: _resultName(result),
              ),
            ),
          );
    } catch (_) {
      _removeRequest(requestId);
      rethrow;
    }
    _removeRequest(requestId);
  }

  /// Ensures the permission request event subscription is active.
  static void _ensureSubscription() {
    if (_subscription != null) {
      return;
    }
    _subscription = _clients.servicesRuntimeHostInteractionService
        .ownerHostInteractionEvents(
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

  /// Handles one incoming runtime permission request event.
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

  /// Removes approval requests whose broker deadline has elapsed.
  static void _removeExpiredRequests(int nowMillis) {
    final expiredIds = _requests.entries
        .where(
          (entry) =>
              nowMillis - entry.value.requestedAtMillis >=
              _requestTimeoutMillis,
        )
        .map((entry) => entry.key)
        .toList(growable: false);
    for (final requestId in expiredIds) {
      _removeRequest(requestId);
    }
  }

  /// Removes one approval request from the local presentation state.
  static void _removeRequest(String requestId) {
    _requests.remove(requestId);
    _requestedAtMillis.remove(requestId);
  }
}

/// Converts one approval result into the runtime protocol value.
String _resultName(ToolApprovalResult result) {
  return switch (result) {
    ToolApprovalResult.allow => 'allow',
    ToolApprovalResult.deny => 'deny',
    ToolApprovalResult.allowSession => 'allow_session',
  };
}
