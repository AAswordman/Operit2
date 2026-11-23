// ignore_for_file: file_names

import 'dart:async';
import 'package:webview_all/webview_all.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import '../logging/ClientLogger.dart';
import 'RuntimeWebVisitTask.dart';
import 'WebVisitModels.dart';

/// Observes a process-owned task; losing this Dart object never stops its browser.
class HostWebVisitSession extends RuntimeWebVisitTask {
  /// Binds an observed task to its authoritative platform Host.
  HostWebVisitSession({required this.request, required this.host});

  @override
  final WebVisitRequest request;
  @override
  final PlatformWebVisitHost host;
  final _completion = Completer<WebVisitResponse>();
  String _url = '';
  String _title = '';
  String _status = 'Loading page';
  bool _manual = false;
  bool _loaded = false;
  bool _disposed = false;

  /// Updates presentation state from a native task snapshot and completes exactly once.
  void apply(Map<String, Object?> snapshot) {
    if (_disposed || _completion.isCompleted) return;
    _url = snapshot['currentUrl'] as String;
    _title = snapshot['pageTitle'] as String;
    _status = snapshot['statusText'] as String;
    _manual = snapshot['manual'] as bool;
    _loaded = snapshot['pageLoaded'] as bool;
    if (snapshot['completed'] == true) {
      complete(Map<String, Object?>.from(snapshot['response'] as Map));
    }
    notifyListeners();
  }

  /// Decodes the Host response through the shared public result schema.
  void complete(Map<String, Object?> terminal) {
    if (_disposed || _completion.isCompleted) return;
    final result = terminal['result'];
    _completion.complete(
      WebVisitResponse(
        requestId: terminal['requestId'] as String,
        success: terminal['success'] as bool,
        error: terminal['error'] as String?,
        result: result == null
            ? null
            : WebVisitResult.fromJson(Map<String, Object?>.from(result as Map)),
      ),
    );
  }

  /// Sends explicit user controls and reports a broken control channel visibly.
  Future<void> _control(String action) async {
    try {
      await host.control(request.requestId, action);
    } catch (error, stack) {
      ClientLogger.w(
        'Web visit control failed: $action',
        tag: 'RuntimeWebVisit',
        error: error,
        stackTrace: stack,
      );
      if (!_disposed) {
        _status = 'Control failed: $error';
        notifyListeners();
      }
    }
  }

  /// Releases only this observer; the Host owns native cleanup.
  @override
  Future<void> disposeSession() async {
    if (_disposed) return;
    _disposed = true;
    dispose();
  }

  @override
  Future<WebVisitResponse> get response => _completion.future;
  @override
  RuntimeBrowserSessionInfo? get browserSession => null;
  @override
  String get statusText => _status;
  @override
  String get leftButtonText => _manual ? 'Cancel' : 'Manual mode';
  @override
  String get rightButtonText => 'Extract now';
  @override
  String get helperText => '';
  @override
  bool get pageLoaded => _loaded;
  @override
  String get currentUrl => _url;
  @override
  String get pageTitle => _title;

  /// Pauses automatic extraction or cancels a task already in manual mode.
  @override
  void handleLeftAction() => unawaited(_control(_manual ? 'cancel' : 'manual'));

  /// Requests extraction from the currently displayed document.
  @override
  void handleRightAction() => unawaited(_control('extract'));

  /// Cancels the authoritative native task.
  @override
  void cancel() => unawaited(_control('cancel'));
}
