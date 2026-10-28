// ignore_for_file: file_names

import 'dart:async';

import 'package:operit2/core/web_visit/WebVisitModels.dart';

class WorkspaceWebVisitSessionRegistry {
  WorkspaceWebVisitSessionRegistry._();

  static final WorkspaceWebVisitSessionRegistry instance =
      WorkspaceWebVisitSessionRegistry._();

  Future<WebVisitResponse> Function(WebVisitRequest request)? _controls;
  final List<Completer<void>> _controlWaiters = <Completer<void>>[];

  void setControls({
    required Future<WebVisitResponse> Function(WebVisitRequest request)
    openWebVisitTab,
  }) {
    _controls = openWebVisitTab;
    final waiters = List<Completer<void>>.of(_controlWaiters);
    _controlWaiters.clear();
    for (final waiter in waiters) {
      if (!waiter.isCompleted) {
        waiter.complete();
      }
    }
  }

  void clearControls() {
    _controls = null;
  }

  Future<WebVisitResponse> visitWeb(WebVisitRequest request) async {
    await _waitForControls();
    return _controls!(request);
  }

  Future<void> _waitForControls() {
    if (_controls != null) {
      return Future<void>.value();
    }
    final completer = Completer<void>();
    _controlWaiters.add(completer);
    return completer.future;
  }
}
