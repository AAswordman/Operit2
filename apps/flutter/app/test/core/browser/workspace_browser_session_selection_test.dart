import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/browser/BrowserSessions.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/WorkspaceBrowserViewStore.dart';

void main() {
  late _RecordingBrowserSessions sessions;
  late WorkspaceBrowserViewStore store;

  setUp(() async {
    sessions = _RecordingBrowserSessions();
    store = WorkspaceBrowserViewStore.forTesting(sessions: sessions);
    await store.attachSession(_session('first'));
  });

  tearDown(() {
    store.detachUi();
    for (final tab in store.tabs) {
      store.closeSession(tab.id);
    }
    store.sessionCount.dispose();
    store.permissions.dispose();
    store.dispose();
  });

  test(
    'opening and switching two sessions keeps the requested tab active',
    () async {
      var workspaceCloses = 0;
      store.attachUi(
        delegate: WorkspaceBrowserViewDelegate(
          onCloseRequested: () => workspaceCloses++,
        ),
        onWriteWorkspaceFileBytes: (_, _) async {},
      );
      await store.attachSession(_session('second'));
      final selections = <String>[];
      void recordSelection() => selections.add(store.currentTab!.id);
      store.addListener(recordSelection);

      for (final id in ['second', 'first', 'second', 'second']) {
        store.selectSession(id);
        expect(store.currentTab?.id, id);
      }
      await Future<void>.delayed(Duration.zero);

      expect(selections, ['second', 'first', 'second', 'second']);
      expect(sessions.activations, selections);
      expect(store.tabs.map((tab) => tab.id), ['first', 'second']);
      expect(store.sessionCount.value, 2);
      expect(workspaceCloses, 0);
      store.removeListener(recordSelection);
    },
  );

  test(
    'a delayed activation failure updates its session after switching away',
    () async {
      await store.attachSession(_session('second'));
      final pending = Completer<RuntimeBrowserCommandResult>();
      sessions.pending['first'] = pending;

      store.selectSession('first');
      store.selectSession('second');
      pending.completeError(StateError('coordination task worker closed'));
      await Future<void>.delayed(Duration.zero);

      expect(store.currentTab?.id, 'second');
      expect(
        store.tabById('first')?.errorText,
        contains('coordination task worker closed'),
      );
      expect(store.tabById('second')?.errorText, isNull);
      expect(sessions.activations, ['first', 'second']);
    },
  );

  test(
    'closing a session before activation fails leaves the other session usable',
    () async {
      await store.attachSession(_session('second'));
      final pending = Completer<RuntimeBrowserCommandResult>();
      sessions.pending['first'] = pending;

      store.selectSession('first');
      store.closeSession('first');
      store.selectSession('second');
      pending.completeError(StateError('session closed'));
      await Future<void>.delayed(Duration.zero);

      expect(store.currentTab?.id, 'second');
      expect(store.tabById('first'), isNull);
      expect(store.currentTab?.errorText, isNull);
      expect(store.sessionCount.value, 1);
    },
  );

  test('an unattached session does not change selection or reach Core', () {
    expect(() => store.selectSession('missing'), throwsStateError);
    expect(store.currentTab?.id, 'first');
    expect(sessions.activations, isEmpty);
  });
}

RuntimeBrowserSessionInfo _session(String id) => RuntimeBrowserSessionInfo(
  sessionId: id,
  currentUrl: 'https://$id.example',
  title: id,
  userAgent: null,
  active: true,
  canGoBack: false,
  canGoForward: false,
  isLoading: false,
  progress: 100,
);

class _RecordingBrowserSessions extends BrowserSessions {
  final activations = <String>[];
  final pending = <String, Completer<RuntimeBrowserCommandResult>>{};

  @override
  Stream<BrowserSessionEvent> watchEvents(String sessionId) =>
      const Stream<BrowserSessionEvent>.empty();

  @override
  Future<RuntimeBrowserCommandResult> activate(String sessionId) {
    activations.add(sessionId);
    return pending[sessionId]?.future ??
        Future.value(
          const RuntimeBrowserCommandResult(
            success: true,
            session: null,
            sessions: [],
            resultJson: '',
            error: null,
          ),
        );
  }

  @override
  Future<void> close(String sessionId) async {}
}
