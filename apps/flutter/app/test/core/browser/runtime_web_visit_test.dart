import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/browser/BrowserSessions.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/core/web_visit/RuntimeWebVisitRegistry.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';

WebVisitRequest request([String id = 'visit']) => WebVisitRequest(
  requestId: id,
  url: 'https://example.com',
  headers: const [WebVisitHeader(name: 'X-Test', value: 'forwarded')],
  userAgent: 'Test agent',
  includeImageLinks: true,
);

void main() {
  setUpAll(ClientLogger.initialize);
  testWidgets(
    'visit executes and closes its owner session without any workspace',
    (tester) async {
      final browser = _Browser();
      final registry = RuntimeWebVisitRegistry(sessions: browser);
      final response = registry.visitWeb(request());
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      final result = await _resolve(tester, response);
      expect(result.success, isTrue);
      expect(result.result?.content, 'Page content');
      expect(result.result?.imageLinks, ['https://example.com/image.png']);
      expect(browser.closed, ['session-1']);
      expect(browser.agent, 'Test agent');
      expect(browser.headers, {'X-Test': 'forwarded'});
      expect(registry.sessionFor('visit'), isNull);
    },
  );

  testWidgets(
    'a workspace attaching late presents a running visit exactly once',
    (tester) async {
      final browser = _Browser()..ready = 'loading';
      final registry = RuntimeWebVisitRegistry(sessions: browser);
      final response = registry.visitWeb(request());
      await tester.pump();
      final owner = Object();
      var presented = 0;
      registry.setControls(owner: owner, openWebVisitTab: (_) => presented++);
      registry.setControls(owner: owner, openWebVisitTab: (_) => presented++);
      expect(presented, 1);
      registry.clearControls(Object());
      registry.clearControls(owner);
      browser.finish('session-1');
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect((await _resolve(tester, response)).success, isTrue);
      expect(browser.closed, ['session-1']);
    },
  );

  testWidgets('presentation failures do not fail browser execution', (
    tester,
  ) async {
    final browser = _Browser();
    final registry = RuntimeWebVisitRegistry(sessions: browser);
    registry.setControls(
      owner: Object(),
      openWebVisitTab: (_) => throw StateError('workspace unavailable'),
    );
    final response = registry.visitWeb(request());
    await tester.pump();
    await tester.pump(const Duration(seconds: 1));
    expect((await _resolve(tester, response)).success, isTrue);
  });

  testWidgets(
    'explicit cancellation closes only the requested concurrent visit',
    (tester) async {
      final browser = _Browser()..ready = 'loading';
      final registry = RuntimeWebVisitRegistry(sessions: browser);
      final first = registry.visitWeb(request('first'));
      final second = registry.visitWeb(request('second'));
      await tester.pump();
      registry.cancel('first');
      await tester.pump();
      expect((await _resolve(tester, first)).success, isFalse);
      expect(browser.closed, ['session-1']);
      expect(registry.sessionFor('second'), isNotNull);
      browser.finish('session-2');
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect((await _resolve(tester, second)).success, isTrue);
      expect(browser.closed, ['session-1', 'session-2']);
    },
  );

  testWidgets(
    'a cancelled create closes the session if the host responds late',
    (tester) async {
      final browser = _Browser()
        ..creation = Completer<RuntimeBrowserSessionInfo>();
      final registry = RuntimeWebVisitRegistry(sessions: browser);
      final response = registry.visitWeb(request());
      registry.cancel('visit');
      await tester.pump();
      expect((await _resolve(tester, response)).success, isFalse);
      browser.creation!.complete(_info('late'));
      await tester.pump();
      expect(browser.closed, ['late']);
    },
  );

  testWidgets(
    'a redirect cannot return extraction from the previous document',
    (tester) async {
      final browser = _Browser()
        ..extraction = Completer<RuntimeBrowserCommandResult>();
      final registry = RuntimeWebVisitRegistry(sessions: browser);
      final response = registry.visitWeb(request());
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      final pending = browser.extraction!;
      browser.start('session-1');
      await tester.pump();
      browser.extraction = null;
      pending.complete(
        RuntimeBrowserCommandResult(
          success: true,
          session: _info('session-1'),
          sessions: [],
          resultJson: jsonEncode(
            jsonEncode({
              'url': 'https://old.example',
              'title': 'Old',
              'content': 'Stale',
              'metadata': {},
              'links': [],
              'imageLinks': [],
            }),
          ),
          error: null,
        ),
      );
      await tester.pump();
      expect(registry.sessionFor('visit'), isNotNull);
      browser.finish('session-1');
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect(
        (await _resolve(tester, response)).result?.content,
        'Page content',
      );
    },
  );

  testWidgets(
    'a stalled native evaluation is bounded and releases the session',
    (tester) async {
      final browser = _Browser()
        ..evaluation = Completer<RuntimeBrowserCommandResult>();
      final registry = RuntimeWebVisitRegistry(sessions: browser);
      final response = registry.visitWeb(request());
      await tester.pump();
      await tester.pump(const Duration(seconds: 11));
      expect((await _resolve(tester, response)).success, isFalse);
      expect(browser.closed, ['session-1']);
    },
  );
}

Future<WebVisitResponse> _resolve(
  WidgetTester tester,
  Future<WebVisitResponse> future,
) async {
  // Stream cancellation may complete on the real event queue even when the
  // extraction timers are controlled by the widget test's fake clock.
  await tester.runAsync(() => Future<void>.delayed(Duration.zero));
  WebVisitResponse? response;
  Object? error;
  future.then<void>(
    (value) {
      response = value;
    },
    onError: (Object value) {
      error = value;
    },
  );
  for (
    var attempt = 0;
    attempt < 30 && response == null && error == null;
    attempt++
  ) {
    await tester.pump(const Duration(milliseconds: 100));
  }
  expect(error, isNull);
  expect(
    response,
    isNotNull,
    reason: 'Visit did not finish after its scheduled work',
  );
  return response!;
}

RuntimeBrowserSessionInfo _info(String id) => RuntimeBrowserSessionInfo(
  sessionId: id,
  currentUrl: 'https://example.com',
  title: 'Example',
  userAgent: null,
  active: true,
  canGoBack: false,
  canGoForward: false,
  isLoading: false,
  progress: 100,
);

class _Browser extends BrowserSessions {
  int count = 0;
  String ready = 'complete';
  String? agent;
  Map<String, String>? headers;
  Completer<RuntimeBrowserSessionInfo>? creation;
  Completer<RuntimeBrowserCommandResult>? evaluation;
  Completer<RuntimeBrowserCommandResult>? extraction;
  final streams = <String, StreamController<BrowserSessionEvent>>{};
  final closed = <String>[];

  @override
  Future<RuntimeBrowserSessionInfo> createSession({
    required String initialUrl,
    String? userAgent,
    Map<String, String> headers = const {},
  }) async {
    agent = userAgent;
    this.headers = headers;
    if (creation != null) return creation!.future;
    return _info('session-${++count}');
  }

  @override
  Stream<BrowserSessionEvent> watchEvents(String sessionId) =>
      (streams[sessionId] = StreamController<BrowserSessionEvent>(
        onCancel: () async {},
      )).stream;

  void start(String id) => streams[id]!.add(
    RuntimeBrowserStreamEvent(
      sequence: 2,
      sessionId: id,
      eventType: 'started',
      session: _info(id),
      resultJson: '',
      frameData: Uint8List(0),
      frameCodec: null,
      frameWidth: null,
      frameHeight: null,
      error: null,
    ),
  );

  void finish(String id) => streams[id]!.add(
    RuntimeBrowserStreamEvent(
      sequence: 1,
      sessionId: id,
      eventType: 'finished',
      session: _info(id),
      resultJson: '',
      frameData: Uint8List(0),
      frameCodec: null,
      frameWidth: null,
      frameHeight: null,
      error: null,
    ),
  );

  @override
  Future<RuntimeBrowserCommandResult> evaluate(
    String sessionId,
    String script,
  ) async {
    if (evaluation != null) return evaluation!.future;
    final Object? value;
    if (script.contains('readyState: document.readyState')) {
      value = jsonEncode({'readyState': ready, 'url': 'https://example.com'});
    } else if (script.contains('captcha/i')) {
      value = false;
    } else if (script.contains('const includeImages')) {
      if (extraction != null) return extraction!.future;
      value = jsonEncode({
        'url': 'https://example.com',
        'title': 'Example',
        'content': 'Page content',
        'metadata': {'description': 'Example'},
        'links': <Object>[],
        'imageLinks': ['https://example.com/image.png'],
      });
    } else {
      value = null;
    }
    return RuntimeBrowserCommandResult(
      success: true,
      session: _info(sessionId),
      sessions: [],
      resultJson: jsonEncode(value),
      error: null,
    );
  }

  @override
  Future<void> close(String sessionId) async {
    closed.add(sessionId);
    await streams[sessionId]?.close();
  }
}
