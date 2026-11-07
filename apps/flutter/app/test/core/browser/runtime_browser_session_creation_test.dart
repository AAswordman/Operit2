import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserAutomationController.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserSessionRegistry.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

/// Verifies that overlapping browser creates retain their own owner session.
void main() {
  final registry = RuntimeBrowserSessionRegistry.instance;

  testWidgets('queued creates return their own sessions and errors', (
    tester,
  ) async {
    final first = registry.handleRuntimeBrowserCommand(_create('first'));
    final second = registry.handleRuntimeBrowserCommand(_create('second'));
    final failed = registry.handleRuntimeBrowserCommand(_create('failed'));
    final pending = <String, Completer<String>>{};
    registry.setBrowserSessionOpener(
      openBrowserTab: ({url, userAgent, headers}) {
        expect(userAgent, 'Test agent');
        expect(headers, {'X-Test': 'forwarded'});
        final id = Uri.parse(url!).host;
        if (id != 'failed') _register(registry, id, url);
        return (pending[id] = Completer<String>()).future;
      },
    );
    pending['second']!.complete('second');
    pending['failed']!.completeError(StateError('Create failed'));
    pending['first']!.complete('first');
    await tester.pump();
    expect((await first).session?.sessionId, 'first');
    expect((await second).session?.sessionId, 'second');
    expect((await failed).success, isFalse);
    expect((await failed).error, 'Bad state: Create failed');
    registry.unregister('first');
    registry.unregister('second');
    await tester.pump();
  });

  for (final reverse in [false, true]) {
    testWidgets('concurrent creates keep their sessions (reverse=$reverse)', (
      tester,
    ) async {
      final pending = <String, Completer<String>>{};
      registry.setBrowserSessionOpener(
        openBrowserTab: ({url, userAgent, headers}) {
          final id = Uri.parse(url!).host;
          _register(registry, id, url);
          final completion = pending[id] = Completer<String>();
          return completion.future;
        },
      );
      final first = registry.handleRuntimeBrowserCommand(_create('first'));
      final second = registry.handleRuntimeBrowserCommand(_create('second'));
      expect(pending.keys, ['first', 'second']);
      final order = reverse ? ['second', 'first'] : ['first', 'second'];
      for (final id in order) {
        pending[id]!.complete(id);
        await tester.pump();
      }
      final results = await Future.wait([first, second]);
      expect(results.map((result) => result.success), [true, true]);
      expect(results.map((result) => result.session?.sessionId), [
        'first',
        'second',
      ]);
      for (final id in pending.keys) {
        registry.unregister(id);
      }
      await tester.pump();
    });
  }

  testWidgets('a missing created session fails with another session active', (
    tester,
  ) async {
    _register(registry, 'existing', 'https://existing');
    registry.setBrowserSessionOpener(
      openBrowserTab: ({url, userAgent, headers}) async => 'missing',
    );
    final result = await registry.handleRuntimeBrowserCommand(
      _create('missing'),
    );
    expect(result.success, isFalse);
    expect(
      result.error,
      'Bad state: Created browser session is not registered: missing',
    );
    expect(registry.activeSessionId, 'existing');
    registry.unregister('existing');
    await tester.pump();
  });
}

/// Builds a runtime create command with a distinct target URL.
RuntimeBrowserCommand _create(String host) => RuntimeBrowserCommand(
  action: 'create',
  sessionId: null,
  url: 'https://$host',
  script: null,
  payloadJson: '',
  userAgent: 'Test agent',
  headers: const {'X-Test': 'forwarded'},
);

/// Registers a real registry entry without mounting a platform WebView.
void _register(RuntimeBrowserSessionRegistry registry, String id, String url) {
  registry.register(
    sessionId: id,
    controller: RuntimeBrowserAutomationController(
      controller: WebViewController.fromPlatform(_Controller()),
      onSurfaceFrameRequested: () async {},
    ),
    title: id,
    url: url,
    active: true,
    userAgent: null,
    canGoBack: false,
    canGoForward: false,
    isLoading: true,
    progress: 0,
    selectTab: (_) {},
    closeTab: registry.unregister,
    navigate: (_) async {},
    navigateBack: () async {},
    navigateForward: () async {},
    reload: () async {},
    stop: () async {},
    setZoomFactor: (_) async {},
    supportsPageJavaScript: () => true,
  );
}

class _Controller extends PlatformWebViewController {
  /// Creates a controller using the shared platform interface contract.
  _Controller()
    : super.implementation(const PlatformWebViewControllerCreationParams());
}
