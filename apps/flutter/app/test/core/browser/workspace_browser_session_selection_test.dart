import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

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
    sessions.events.close();
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

  test(
    'explicit application handoff reaches the selected session owner',
    () async {
      await store.attachSession(_session('second'));
      store.selectSession('first');
      await store.openCurrentInExternalApplication();

      expect(sessions.externalNavigations, [
        ('first', 'https://first.example'),
      ]);
      expect(store.currentTab?.url, 'https://first.example');
      expect(sessions.navigations, isEmpty);
    },
  );

  test(
    'application address input preserves the page and reports owner failure on its tab',
    () async {
      await store.attachSession(_session('second'));
      store.selectSession('first');
      final pending = Completer<RuntimeBrowserCommandResult>();
      sessions.pendingNavigation = pending;

      store.navigateCurrent('weixin://chat?id=1');
      expect(sessions.navigations, [('first', 'weixin://chat?id=1')]);
      expect(store.currentTab?.url, 'https://first.example');
      expect(store.currentTab?.addressText, 'https://first.example');
      store.selectSession('second');
      pending.completeError(StateError('application launch refused'));
      await Future<void>.delayed(Duration.zero);

      expect(
        store.tabById('first')?.errorText,
        contains('application launch refused'),
      );
      expect(store.currentTab?.id, 'second');
      expect(store.currentTab?.errorText, isNull);
    },
  );

  test(
    'explicit application failures remain visible to the calling UI',
    () async {
      sessions.externalFailure = StateError('system resolver refused');
      await expectLater(
        store.openCurrentInExternalApplication(),
        throwsStateError,
      );
      expect(store.currentTab?.url, 'https://first.example');
    },
  );

  test(
    'owner download events update the displayed list and controls reach the owner',
    () async {
      final item = <String, Object?>{
        'url': 'https://example.test/file.zip',
        'fileName': 'file.zip',
        'stateIndex': 1,
        'createdAt': DateTime.fromMillisecondsSinceEpoch(0).toIso8601String(),
        'progress': 43,
        'receivedBytes': 43,
        'totalBytes': 100,
      };
      final snapshot = jsonEncode({
        'ownerEpoch': 'test',
        'revision': 1,
        'items': [item],
      });
      sessions.events.add(
        RuntimeBrowserStreamEvent(
          sequence: 1,
          sessionId: 'first',
          eventType: 'downloads',
          session: null,
          resultJson: snapshot,
          frameData: Uint8List(0),
          frameCodec: null,
          frameWidth: null,
          frameHeight: null,
          error: null,
        ),
      );
      expect(store.stores.downloads.items.single.progress, 43);
      sessions.downloadSnapshot = jsonEncode({
        'ownerEpoch': 'test',
        'revision': 2,
        'items': [item..['stateIndex'] = 5],
      });
      await store.stores.downloads.cancel(store.stores.downloads.items.single);
      expect(sessions.downloadCommands.last, (
        'first',
        'cancel',
        'https://example.test/file.zip',
      ));
      expect(store.stores.downloads.items.single.state.name, 'cancelled');
    },
  );
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
  final events = StreamController<BrowserSessionEvent>.broadcast(sync: true);
  final downloadCommands = <(String, String, String?)>[];
  final externalNavigations = <(String, String)>[];
  final navigations = <(String, String)>[];
  Completer<RuntimeBrowserCommandResult>? pendingNavigation;
  Object? externalFailure;
  String downloadSnapshot = '{"ownerEpoch":"test","revision":0,"items":[]}';

  /// Records navigation commands without creating another browser session.
  @override
  Future<RuntimeBrowserCommandResult> navigate(String sessionId, String url) {
    navigations.add((sessionId, url));
    return pendingNavigation!.future;
  }

  /// Records application handoffs sent to the selected browser owner.
  @override
  Future<RuntimeBrowserCommandResult> openExternal(
    String sessionId,
    String url,
  ) async {
    externalNavigations.add((sessionId, url));
    final error = externalFailure;
    if (error != null) throw error;
    return const RuntimeBrowserCommandResult(
      success: true,
      session: null,
      sessions: [],
      resultJson: '',
      error: null,
    );
  }

  /// Supplies owner downloads when the view attaches a session.
  @override
  Future<RuntimeBrowserCommandResult> downloads(
    String sessionId, {
    String operation = 'list',
    String? url,
  }) async {
    downloadCommands.add((sessionId, operation, url));
    return RuntimeBrowserCommandResult(
      success: true,
      session: null,
      sessions: [],
      resultJson: downloadSnapshot,
      error: null,
    );
  }

  @override
  Stream<BrowserSessionEvent> watchEvents(String sessionId) =>
      events.stream.where((event) => event.sessionId == sessionId);

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
