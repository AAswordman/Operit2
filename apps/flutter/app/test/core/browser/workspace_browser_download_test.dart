import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:webview_all/webview_all.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/downloads/WorkspaceBrowserDownloadSheet.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/downloads/WorkspaceBrowserDownloadStore.dart';

/// Exercises real transfer, persistence, observer, and phone layout boundaries.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'native responses import original bytes and keep simultaneous URLs distinct',
    () async {
      final storage = _Storage();
      final store = _store(
        storage,
        () => throw StateError('Native transfers must not replay HTTP'),
      );
      addTearDown(store.dispose);
      for (final id in ['first', 'second']) {
        await store.acceptEngineDownload(
          'tab',
          WebViewDownloadEvent(
            id: id,
            url: 'https://example.test/export',
            fileName: 'report.pdf',
            state: WebViewDownloadState.completed,
            receivedBytes: 3,
            totalBytes: 3,
            readBytes: () async => Uint8List.fromList([1, 2, 3]),
          ),
        );
      }
      expect(store.items, hasLength(2));
      expect(store.items.map((item) => item.id).toSet(), hasLength(2));
      expect(store.items.map((item) => item.runtimePath).toSet(), hasLength(2));
      expect(
        store.items.every(
          (item) => item.state == WorkspaceBrowserDownloadState.completed,
        ),
        isTrue,
      );
      expect(store.items.every((item) => !item.canRetry), isTrue);
      for (final item in store.items) {
        expect(storage.files[item.runtimePath], base64Encode([1, 2, 3]));
      }
    },
  );

  test(
    'native failures publish the engine reason and cannot replay a consumed request',
    () async {
      final store = _store(
        _Storage(),
        () => throw StateError('Must not request'),
      );
      addTearDown(store.dispose);
      await store.acceptEngineDownload(
        'tab',
        const WebViewDownloadEvent(
          id: 'native',
          url: 'blob:https://example.test/file',
          fileName: 'file.txt',
          state: WebViewDownloadState.failed,
          error: 'Connection interrupted',
        ),
      );
      expect(store.items.single.state, WorkspaceBrowserDownloadState.failed);
      expect(store.items.single.detail, 'Connection interrupted');
      await expectLater(store.retry(store.items.single), throwsStateError);
    },
  );

  test(
    'cancellation stops the native operation and rejects a late completion',
    () async {
      final storage = _Storage();
      final store = _store(storage, () => throw StateError('Must not request'));
      addTearDown(store.dispose);
      var cancelled = false;
      await store.acceptEngineDownload(
        'tab',
        WebViewDownloadEvent(
          id: 'native',
          url: 'https://example.test/export',
          fileName: 'export.bin',
          state: WebViewDownloadState.running,
          cancel: () async {
            cancelled = true;
          },
        ),
      );
      await store.cancel(store.items.single);
      await store.acceptEngineDownload(
        'tab',
        WebViewDownloadEvent(
          id: 'native',
          url: 'https://example.test/export',
          fileName: 'export.bin',
          state: WebViewDownloadState.completed,
          readBytes: () async =>
              throw StateError('Late completion must not be read'),
        ),
      );
      expect(cancelled, isTrue);
      expect(store.items.single.state, WorkspaceBrowserDownloadState.cancelled);
      expect(storage.files, isEmpty);
    },
  );

  test('closing a session invalidates a pending native import', () async {
    final bytes = Completer<Uint8List>();
    final storage = _Storage();
    final store = _store(storage, () => throw StateError('Must not request'));
    addTearDown(store.dispose);
    final completing = store.acceptEngineDownload(
      'tab',
      WebViewDownloadEvent(
        id: 'native',
        url: 'https://example.test/export',
        fileName: 'export.bin',
        state: WebViewDownloadState.completed,
        readBytes: () => bytes.future,
      ),
    );
    store.closeEngineSession('tab');
    bytes.complete(Uint8List.fromList([1]));
    await completing;
    expect(store.items.single.state, WorkspaceBrowserDownloadState.failed);
    expect(storage.files, isEmpty);
  });

  test('duplicate completion events import the engine file once', () async {
    final bytes = Completer<Uint8List>();
    final store = _store(
      _Storage(),
      () => throw StateError('Must not request'),
    );
    addTearDown(store.dispose);
    var reads = 0;
    final event = WebViewDownloadEvent(
      id: 'native',
      url: 'https://example.test/export',
      fileName: 'export.bin',
      state: WebViewDownloadState.completed,
      readBytes: () {
        reads++;
        return bytes.future;
      },
    );
    final pending = store.acceptEngineDownload('tab', event);
    await store.acceptEngineDownload('tab', event);
    bytes.complete(Uint8List.fromList([1]));
    await pending;
    expect(reads, 1);
    expect(store.items.single.state, WorkspaceBrowserDownloadState.completed);
  });

  test('removing a native transfer rejects late engine events', () async {
    final storage = _Storage();
    final store = _store(storage, () => throw StateError('Must not request'));
    addTearDown(store.dispose);
    var cancelled = false;
    await store.acceptEngineDownload(
      'tab',
      WebViewDownloadEvent(
        id: 'native',
        url: 'https://example.test/export',
        fileName: 'export.bin',
        state: WebViewDownloadState.running,
        cancel: () async {
          cancelled = true;
        },
      ),
    );
    await store.remove(store.items.single.id);
    await store.acceptEngineDownload(
      'tab',
      WebViewDownloadEvent(
        id: 'native',
        url: 'https://example.test/export',
        fileName: 'export.bin',
        state: WebViewDownloadState.completed,
        readBytes: () async =>
            throw StateError('Removed files must not reappear'),
      ),
    );
    expect(cancelled, isTrue);
    expect(store.items, isEmpty);
    expect(storage.files, isEmpty);
  });

  test('simultaneous HTTP downloads of one URL cancel independently', () async {
    final first = StreamController<List<int>>();
    final second = StreamController<List<int>>();
    var index = 0;
    final store = _store(
      _Storage(),
      () => _Client(
        (_) async => http.StreamedResponse(
          index++ == 0 ? first.stream : second.stream,
          200,
        ),
      ),
    );
    addTearDown(store.dispose);
    final one = store.startDownload('https://example.test/file');
    final two = store.startDownload('https://example.test/file');
    await Future<void>.delayed(Duration.zero);
    expect(store.items, hasLength(2));
    expect(store.items.map((item) => item.id).toSet(), hasLength(2));
    await store.cancel(store.items.last);
    first.add([1]);
    second.add([2, 3]);
    await first.close();
    await second.close();
    await Future.wait([one, two]);
    expect(store.items.first.state, WorkspaceBrowserDownloadState.completed);
    expect(store.items.first.receivedBytes, 2);
    expect(store.items.last.state, WorkspaceBrowserDownloadState.cancelled);
  });

  test(
    'extensionless downloads preserve request context and server filename',
    () async {
      final storage = _Storage();
      late http.BaseRequest request;
      final store = _store(
        storage,
        () => _Client((value) async {
          request = value;
          return http.StreamedResponse(
            Stream.value([1, 2, 3]),
            200,
            contentLength: 3,
            headers: {
              'content-disposition':
                  "attachment; filename*=UTF-8''%E6%B5%8B%E8%AF%95.apk",
            },
          );
        }),
      );
      addTearDown(store.dispose);
      await store.startDownload(
        'https://example.test/download?id=1',
        headers: {
          'Cookie': 'session=secret',
          'Referer': 'https://example.test/',
        },
      );
      final item = store.items.single;
      expect(request.headers['Cookie'], 'session=secret');
      expect(item.fileName, '测试.apk');
      expect(item.state, WorkspaceBrowserDownloadState.completed);
      expect(item.receivedBytes, 3);
      expect(item.progress, 100);
      expect(storage.files[item.runtimePath], base64Encode([1, 2, 3]));
      String? workspacePath;
      Uint8List? workspaceBytes;
      await store.saveToWorkspace(item.savedPath!, (path, bytes) async {
        workspacePath = path;
        workspaceBytes = bytes;
      });
      expect(workspacePath, item.savedPath);
      expect(workspaceBytes, [1, 2, 3]);
    },
  );

  test(
    'unknown content length publishes received bytes and finishes normally',
    () async {
      final store = _store(
        _Storage(),
        () => _Client(
          (_) async => http.StreamedResponse(Stream.value([1, 2, 3]), 200),
        ),
      );
      addTearDown(store.dispose);
      final progress = <WorkspaceBrowserDownloadItem>[];
      store.addListener(() => progress.add(store.items.single));
      await store.startDownload('https://example.test/file');
      expect(
        progress.any(
          (item) =>
              item.state == WorkspaceBrowserDownloadState.running &&
              item.receivedBytes == 3 &&
              item.totalBytes == null,
        ),
        isTrue,
      );
      expect(store.items.single.state, WorkspaceBrowserDownloadState.completed);
    },
  );

  for (final status in [403, 404, 500]) {
    test('HTTP $status fails without saving an error document', () async {
      final storage = _Storage();
      final store = _store(
        storage,
        () => _Client(
          (_) async => http.StreamedResponse(Stream.value([1]), status),
        ),
      );
      addTearDown(store.dispose);
      await store.startDownload('https://example.test/file.apk');
      expect(store.items.single.state, WorkspaceBrowserDownloadState.failed);
      expect(store.items.single.detail, contains('HTTP $status'));
      expect(storage.files, isEmpty);
    });
  }

  test(
    'truncated response fails before a completed file can be published',
    () async {
      final storage = _Storage();
      final store = _store(
        storage,
        () => _Client(
          (_) async =>
              http.StreamedResponse(Stream.value([1]), 200, contentLength: 5),
        ),
      );
      addTearDown(store.dispose);
      await store.startDownload('https://example.test/file.apk');
      expect(store.items.single.state, WorkspaceBrowserDownloadState.failed);
      expect(storage.files, isEmpty);
    },
  );

  test(
    'a late cancelled request cannot overwrite a restarted transfer',
    () async {
      final first = StreamController<List<int>>();
      final second = StreamController<List<int>>();
      var index = 0;
      final store = _store(
        _Storage(),
        () => _Client(
          (_) async => http.StreamedResponse(
            index++ == 0 ? first.stream : second.stream,
            200,
          ),
        ),
      );
      addTearDown(store.dispose);
      final old = store.startDownload('https://example.test/file');
      await Future<void>.delayed(Duration.zero);
      await store.cancel(store.items.single);
      final next = store.startDownload(
        'https://example.test/file',
        downloadId: store.items.single.id,
      );
      await Future<void>.delayed(Duration.zero);
      first.addError(StateError('old connection closed late'));
      await first.close();
      await old;
      expect(store.items.single.state, WorkspaceBrowserDownloadState.running);
      second.add([2, 3]);
      await second.close();
      await next;
      expect(store.items.single.state, WorkspaceBrowserDownloadState.completed);
      expect(store.items.single.receivedBytes, 2);
    },
  );

  test('same server filenames have independent saved files', () async {
    final storage = _Storage();
    final store = _store(
      storage,
      () => _Client(
        (_) async => http.StreamedResponse(
          Stream.value([1]),
          200,
          headers: {'content-disposition': 'attachment; filename="same.zip"'},
        ),
      ),
    );
    addTearDown(store.dispose);
    await store.startDownload('https://example.test/one');
    await store.startDownload('https://example.test/two');
    expect(store.items.map((item) => item.runtimePath).toSet(), hasLength(2));
    expect(storage.files, hasLength(2));
  });

  test(
    'owner restart resets revisions and stale replies do not regress progress',
    () {
      final observer = _store(
        _Storage(),
        () => throw StateError('observer must not download'),
        observer: true,
      );
      addTearDown(observer.dispose);
      observer.applySnapshot(
        _snapshot(10, _item(progress: 60), epoch: 'first'),
      );
      observer.applySnapshot(_snapshot(9, _item(progress: 30), epoch: 'first'));
      expect(observer.items.single.progress, 60);
      observer.applySnapshot(
        _snapshot(1, _item(progress: 10), epoch: 'restarted'),
      );
      expect(observer.items.single.progress, 10);
    },
  );

  test(
    'process restart marks old active records interrupted and migrates old paths',
    () async {
      final storage = _Storage()
        ..text = jsonEncode([
          _item().toJson(),
          _item(completed: true).toJson()..remove('runtimePath'),
        ]);
      final store = _store(
        storage,
        () => throw StateError('loading must not start requests'),
      );
      addTearDown(store.dispose);
      await store.load();
      expect(store.items.first.state, WorkspaceBrowserDownloadState.paused);
      expect(store.items.last.runtimePath, '/browser-files/archive.zip');
    },
  );

  testWidgets(
    'phone menu remains readable and completion stays in the default list',
    (tester) async {
      tester.view.physicalSize = const Size(320, 740);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final store = _store(
        _Storage(),
        () => throw StateError('observer must not download'),
        observer: true,
      );
      addTearDown(store.dispose);
      store.applySnapshot(_snapshot(1, _item(progress: 50)));
      await tester.pumpWidget(
        MaterialApp(
          locale: const Locale('zh'),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          builder: (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(textScaler: TextScaler.linear(1.4)),
            child: child!,
          ),
          home: Scaffold(
            body: WorkspaceBrowserDownloadSheet(
              store: store,
              onOpenWorkspaceFile: (_) async {},
              onSaveToWorkspace: (_) async {},
            ),
          ),
        ),
      );
      await tester.pump();
      expect(find.textContaining('50%'), findsOneWidget);
      store.applySnapshot(_snapshot(2, _item(completed: true)));
      await tester.pumpAndSettle();
      expect(find.text('archive.zip'), findsOneWidget);
      await tester.tap(
        find.byWidgetPredicate((widget) => widget is PopupMenuButton).first,
      );
      await tester.pumpAndSettle();
      final open = find
          .byWidgetPredicate((widget) => widget is PopupMenuItem)
          .first;
      expect(tester.getSize(open).width, greaterThanOrEqualTo(180));
      expect(tester.takeException(), isNull);
    },
  );
}

/// Creates a real store using an in-memory Core storage boundary.
WorkspaceBrowserDownloadStore _store(
  _Storage storage,
  http.Client Function() factory, {
  bool observer = false,
}) => WorkspaceBrowserDownloadStore(
  runtimeStorage: GeneratedCoreProxyClients(
    storage,
  ).repositoryRuntimeStorageRepository,
  storagePath: () async => '/downloads.json',
  downloadDirectoryPath: () async => '/browser-files',
  clientFactory: factory,
  observer: observer,
);

/// Builds one transfer record shared by state and layout checks.
WorkspaceBrowserDownloadItem _item({
  bool completed = false,
  int progress = 0,
}) => WorkspaceBrowserDownloadItem(
  url: 'https://example.test/archive.zip',
  fileName: 'archive.zip',
  state: completed
      ? WorkspaceBrowserDownloadState.completed
      : WorkspaceBrowserDownloadState.running,
  createdAt: DateTime.fromMillisecondsSinceEpoch(0),
  progress: progress,
  receivedBytes: 512,
  totalBytes: 1024,
  savedPath: 'downloads/archive.zip',
  runtimePath: '/browser-files/archive.zip',
);

/// Encodes the same revision envelope used by the browser owner.
String _snapshot(
  int revision,
  WorkspaceBrowserDownloadItem item, {
  String epoch = 'test',
}) => jsonEncode({
  'ownerEpoch': epoch,
  'revision': revision,
  'items': [item.toJson()],
});

class _Client extends http.BaseClient {
  /// Creates a controllable HTTP boundary without a real network connection.
  _Client(this.handler);
  final Future<http.StreamedResponse> Function(http.BaseRequest) handler;

  /// Supplies controlled responses, including late events after cancellation.
  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) =>
      handler(request);
}

class _Storage extends OperitRuntimeBridge {
  String? text;
  final Map<String, String> files = {};

  /// Implements only storage methods used by the download pipeline.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final args = request.args as Map<String, Object?>;
    Object? result;
    switch (request.methodName) {
      case 'readText':
        result = text;
      case 'writeText':
        text = args['content'] as String;
      case 'writeBase64':
        files[args['path'] as String] = args['base64Content'] as String;
      case 'readBase64':
        result = files[args['path']];
      default:
        throw StateError('Unexpected storage call: ${request.methodName}');
    }
    return encodeCoreLink([0, result]);
  }

  /// Rejects push streams outside this storage test boundary.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects watches outside this storage test boundary.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects watches outside this storage test boundary.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}
