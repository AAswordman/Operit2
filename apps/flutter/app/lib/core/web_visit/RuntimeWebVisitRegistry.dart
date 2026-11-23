// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'package:webview_all/webview_all.dart';
import 'package:operit2/core/browser/BrowserSessions.dart';
import '../logging/ClientLogger.dart';
import 'package:operit2/core/web_visit/RuntimeWebVisitSession.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';
import 'RuntimeWebVisitTask.dart';
import 'HostWebVisitSession.dart';

/// Owns visit tasks independently from optional workspace presentations.
class RuntimeWebVisitRegistry {
  RuntimeWebVisitRegistry({BrowserSessions? sessions}) : _sessions = sessions;

  static final instance = RuntimeWebVisitRegistry();
  final BrowserSessions? _sessions;
  final Map<String, RuntimeWebVisitTask> _visits = {};
  PlatformWebVisitHost? _host;
  StreamSubscription<Map<String, Object?>>? _hostEvents;
  Object? _presenterOwner;
  final Set<String> _presented = {};
  void Function(WebVisitRequest request)? _present;

  RuntimeWebVisitTask? sessionFor(String requestId) => _visits[requestId];

  /// Observes process-owned visits, including work started before Flutter connected.
  void attachHost(PlatformWebVisitHost? host) {
    if (identical(host, _host)) return;
    unawaited(_hostEvents?.cancel());
    _host = host;
    _hostEvents = host?.watchVisits().listen(
      _observeHostVisit,
      onError: (Object error, StackTrace stack) {
        ClientLogger.w(
          'Observing Host web visits failed',
          tag: 'RuntimeWebVisit',
          error: error,
          stackTrace: stack,
        );
      },
    );
  }

  /// Adopts one authoritative task without starting a second browser or navigation.
  void _observeHostVisit(Map<String, Object?> snapshot) {
    final request = WebVisitRequest.decode(jsonEncode(snapshot['request']))!;
    var task = _visits[request.requestId];
    if (task == null) {
      if (snapshot['completed'] == true) return;
      final observed = HostWebVisitSession(request: request, host: _host!);
      _visits[request.requestId] = observed;
      unawaited(_trackHostTask(observed));
      task = observed;
    }
    (task as HostWebVisitSession).apply(snapshot);
    _presentVisit(request);
  }

  /// Releases observation after completion without owning the native browser lifetime.
  Future<void> _trackHostTask(HostWebVisitSession task) async {
    await task.response;
    _visits.remove(task.request.requestId);
    _presented.remove(task.request.requestId);
    await task.disposeSession();
  }

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
    final host = _host;
    if (host != null) {
      final task = HostWebVisitSession(request: request, host: host);
      _visits[request.requestId] = task;
      unawaited(_trackHostTask(task));
      _presentVisit(request);
      try {
        task.complete(
          await host.visit({
            'requestId': request.requestId,
            'url': request.url,
            'headers': request.headers
                .map((header) => {'name': header.name, 'value': header.value})
                .toList(),
            'userAgent': request.userAgent,
            'includeImageLinks': request.includeImageLinks,
          }),
        );
      } catch (error) {
        task.complete({
          'requestId': request.requestId,
          'success': false,
          'error': error.toString(),
        });
      }
      return task.response;
    }
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
