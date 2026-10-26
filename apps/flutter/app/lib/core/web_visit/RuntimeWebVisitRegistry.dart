// ignore_for_file: file_names

import 'package:operit2/core/browser/BrowserSessions.dart';
import '../logging/ClientLogger.dart';
import 'package:operit2/core/web_visit/RuntimeWebVisitSession.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';

/// Owns visit tasks independently from optional workspace presentations.
class RuntimeWebVisitRegistry {
  RuntimeWebVisitRegistry({BrowserSessions? sessions}) : _sessions = sessions;

  static final instance = RuntimeWebVisitRegistry();
  final BrowserSessions? _sessions;
  final Map<String, RuntimeWebVisitSession> _visits = {};
  Object? _presenterOwner;
  final Set<String> _presented = {};
  void Function(WebVisitRequest request)? _present;

  RuntimeWebVisitSession? sessionFor(String requestId) => _visits[requestId];

  void setControls({
    required Object owner,
    required void Function(WebVisitRequest request) openWebVisitTab,
  }) {
    if (!identical(owner, _presenterOwner)) _presented.clear();
    _presenterOwner = owner;
    _present = openWebVisitTab;
    for (final visit in List.of(_visits.values)) {
      _presentVisit(visit.request);
    }
  }

  void clearControls(Object owner) {
    if (!identical(owner, _presenterOwner)) return;
    _presenterOwner = null;
    _present = null;
    _presented.clear();
  }

  Future<WebVisitResponse> visitWeb(WebVisitRequest request) async {
    final existing = _visits[request.requestId];
    if (existing != null) return existing.response;
    final visit = RuntimeWebVisitSession(request: request, sessions: _sessions);
    _visits[request.requestId] = visit;
    visit.start();
    try {
      _presentVisit(request);
      return await visit.response;
    } finally {
      _visits.remove(request.requestId);
      _presented.remove(request.requestId);
      await visit.disposeSession();
    }
  }

  void _presentVisit(WebVisitRequest request) {
    if (_present == null || _presented.contains(request.requestId)) return;
    try {
      _present!(request);
      _presented.add(request.requestId);
    } catch (error, stack) {
      ClientLogger.w(
        'Presenting visit ${request.requestId} failed',
        tag: 'RuntimeWebVisit',
        error: error,
        stackTrace: stack,
      );
    }
  }

  void cancel(String requestId) => _visits[requestId]?.cancel();
}
