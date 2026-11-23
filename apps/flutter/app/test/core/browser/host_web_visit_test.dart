import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/web_visit/RuntimeWebVisitRegistry.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/automation/WorkspaceWebVisitContent.dart';
import 'package:webview_all/webview_all.dart';

/// Builds the native snapshot shape used by execution and presentation.
Map<String, Object?> snapshot(String id, {bool completed = false}) => {
  'request': {
    'requestId': id,
    'url': 'https://example.test',
    'headers': <Object?>[],
    'userAgent': 'agent',
    'includeImageLinks': true,
  },
  'currentUrl': 'https://example.test/redirected',
  'pageTitle': 'Live page',
  'statusText': 'Reading page',
  'pageLoaded': true,
  'manual': false,
  'completed': completed,
  'response': completed
      ? {
          'requestId': id,
          'success': true,
          'result': {
            'url': 'https://example.test/redirected',
            'title': 'Live page',
            'content': 'Host DOM',
            'metadata': <String, String>{},
            'links': <Object?>[],
            'imageLinks': ['https://example.test/image.png'],
          },
        }
      : null,
};

/// Provides a controllable process Host without invoking any Flutter browser controller.
class VisitHost extends PlatformWebVisitHost {
  final events = StreamController<Map<String, Object?>>.broadcast(sync: true);
  final calls = <Map<String, Object?>>[];
  final actions = <String>[];
  final completion = Completer<Map<String, Object?>>();

  /// Exposes native task state to the runtime observer.
  @override
  Stream<Map<String, Object?>> watchVisits() => events.stream;

  /// Captures explicit execution without creating another task from observer events.
  @override
  Future<Map<String, Object?>> visit(Map<String, Object?> request) {
    calls.add(request);
    events.add(snapshot(request['requestId'] as String));
    return completion.future;
  }

  /// Records controls applied to an already running task.
  @override
  Future<void> control(String requestId, String action) async =>
      actions.add('$requestId:$action');

  /// Identifies the exact host task selected for workspace presentation.
  @override
  Widget buildSurface(String requestId) => Text('native:$requestId');
}

/// Validates process-owned task adoption, presentation and independent cancellation.
void main() {
  setUpAll(ClientLogger.initialize);

  testWidgets(
    'workspace displays the authoritative host surface without starting a second visit',
    (tester) async {
      final host = VisitHost();
      final registry = RuntimeWebVisitRegistry.instance..attachHost(host);
      host.events.add(snapshot('shown'));
      final task = registry.sessionFor('shown')!;
      await tester.pumpWidget(_visitApp(task.request));
      expect(find.text('native:shown'), findsOneWidget);
      expect(host.calls, isEmpty);
      await tester.pumpWidget(const SizedBox());
      expect(host.actions, isEmpty);
      host.events.add(snapshot('shown', completed: true));
      await task.response;
      registry.attachHost(null);
      await host.events.close();
    },
  );

  test(
    'a task started before workspace attachment is adopted without executing again',
    () async {
      final host = VisitHost();
      final registry = RuntimeWebVisitRegistry()..attachHost(host);
      host.events.add(snapshot('running'));
      final task = registry.sessionFor('running')!;
      expect(task.host, same(host));
      expect(task.currentUrl, 'https://example.test/redirected');
      final owner = Object();
      final presented = <String>[];
      registry.setControls(
        owner: owner,
        openWebVisitTab: (request) => presented.add(request.requestId),
      );
      registry.setControls(
        owner: owner,
        openWebVisitTab: (request) => presented.add(request.requestId),
      );
      expect(presented, ['running']);
      expect(host.calls, isEmpty);
      registry.clearControls(owner);
      expect(host.actions, isEmpty);
      host.events.add(snapshot('running', completed: true));
      expect((await task.response).result!.content, 'Host DOM');
      expect(registry.sessionFor('running'), isNull);
      registry.attachHost(null);
      await host.events.close();
    },
  );

  test(
    'concurrent native tasks keep their controls and terminal results isolated',
    () async {
      final host = VisitHost();
      final registry = RuntimeWebVisitRegistry()..attachHost(host);
      host.events.add(snapshot('one'));
      host.events.add(snapshot('two'));
      final one = registry.sessionFor('one')!;
      final two = registry.sessionFor('two')!;
      one.handleRightAction();
      registry.cancel('two');
      expect(host.actions, ['one:extract', 'two:cancel']);
      host.events.add(snapshot('two', completed: true));
      await two.response;
      expect(registry.sessionFor('one'), same(one));
      host.events.add(snapshot('one', completed: true));
      await one.response;
      registry.attachHost(null);
      await host.events.close();
    },
  );

  test(
    'explicit bridge requests execute once and use the same task as native observation',
    () async {
      final host = VisitHost();
      final registry = RuntimeWebVisitRegistry()..attachHost(host);
      final response = registry.visitWeb(
        const WebVisitRequest(
          requestId: 'external',
          url: 'https://example.test',
          headers: [WebVisitHeader(name: 'X-Test', value: 'yes')],
          userAgent: 'agent',
          includeImageLinks: true,
        ),
      );
      final task = registry.sessionFor('external')!;
      expect(host.calls, hasLength(1));
      expect(host.calls.single['headers'], [
        {'name': 'X-Test', 'value': 'yes'},
      ]);
      host.events.add(snapshot('external', completed: true));
      host.completion.complete(
        snapshot('external', completed: true)['response']
            as Map<String, Object?>,
      );
      expect((await response).result!.content, 'Host DOM');
      expect((await task.response).success, isTrue);
      expect(registry.sessionFor('external'), isNull);
      registry.attachHost(null);
      await host.events.close();
    },
  );
}

/// Provides Material controls around a presentation-only visit widget.
Widget _visitApp(WebVisitRequest request) => MaterialApp(
  home: Scaffold(body: WorkspaceWebVisitContent(request: request)),
);
