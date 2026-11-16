import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/host/AppToastController.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceFilePreviewContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/downloads/WorkspaceBrowserDownloadSheet.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/downloads/WorkspaceBrowserDownloadStore.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceFilePreviewActionBar.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

enum _Entry { binary, toolbar, download }

const _timeout = CoreLinkError(
  code: 'COMMAND_ERROR',
  message: 'host interaction timed out: workspace-open-test-1',
  details: <String, Object?>{
    'errorType': 'String',
    'fields': <String, Object?>{'value': 'workspace-open-test-1'},
  },
  backtrace: 'test native backtrace',
);
const _timeoutNotice = 'Opening the file timed out. Please try again.';

final _diagnostics = StringBuffer();

final _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAAANSURBVBhXY2Bg+P8fAAMCAf/Jsq3uAAAAAElFTkSuQmCC',
);

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(ClientLogger.initialize);

  for (final entry in _Entry.values) {
    fileTest('${entry.name} awaits the result and suppresses duplicate opens', (
      tester,
    ) async {
      final pending = Completer<void>();
      final opened = <String>[];
      final store = _DownloadStore('archive.zip');
      addTearDown(store.dispose);
      await _pumpEntry(tester, entry, (path) {
        opened.add(path);
        return pending.future;
      }, store: store);

      // Keep the original event callback to emulate a second event already
      // delivered before the disabled UI has rebuilt.
      final open = _openCallback(tester, entry);
      open();
      open();
      await tester.pump();
      expect(opened, <String>['archive.zip']);
      _expectBusy(tester, entry);
      expect(AppToastController.instance.notices.value, isEmpty);

      pending.completeError(_timeout);
      await tester.pumpAndSettle();
      expect(
        AppToastController.instance.notices.value.single.text,
        _timeoutNotice,
      );
      expect(tester.takeException(), isNull);
      _expectReady(tester, entry);
      final log = _diagnostics.toString();
      expect(log, contains('Details:'));
      expect(log, contains('workspace-open-test-1'));
      expect(log, contains('test native backtrace'));
      await tester.pumpWidget(const SizedBox.shrink());
    });

    fileTest('${entry.name} becomes available after a successful host result', (
      tester,
    ) async {
      final pending = Completer<void>();
      var count = 0;
      final store = _DownloadStore('archive.zip');
      addTearDown(store.dispose);
      await _pumpEntry(tester, entry, (_) {
        count++;
        return count == 1 ? pending.future : Future<void>.value();
      }, store: store);

      _openCallback(tester, entry)();
      await tester.pump();
      _expectBusy(tester, entry);
      expect(count, 1);
      pending.complete();
      await tester.pumpAndSettle();
      _expectReady(tester, entry);
      _openCallback(tester, entry)();
      await tester.pumpAndSettle();
      expect(count, 2);
      expect(AppToastController.instance.notices.value, isEmpty);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });

    fileTest('${entry.name} consumes a late failure after disposal', (
      tester,
    ) async {
      final pending = Completer<void>();
      final store = _DownloadStore('archive.zip');
      addTearDown(store.dispose);
      await _pumpEntry(tester, entry, (_) => pending.future, store: store);
      _openCallback(tester, entry)();
      await tester.pump();
      await tester.pumpWidget(const SizedBox.shrink());

      pending.completeError(_timeout);
      await tester.pump();
      expect(AppToastController.instance.notices.value, isEmpty);
      expect(tester.takeException(), isNull);
    });

    fileTest('${entry.name} ignores the old result after its file changes', (
      tester,
    ) async {
      final previous = Completer<void>();
      final current = Completer<void>();
      final opened = <String>[];
      final store = _DownloadStore('archive.zip');
      addTearDown(store.dispose);
      Future<void> open(String path) {
        opened.add(path);
        return path == 'archive.zip' ? previous.future : current.future;
      }

      await _pumpEntry(tester, entry, open, store: store);
      final staleEvent = _openCallback(tester, entry);
      staleEvent();
      await tester.pump();
      store.replacePath('new-archive.zip');
      await _pumpEntry(
        tester,
        entry,
        open,
        store: store,
        path: 'new-archive.zip',
      );
      staleEvent();
      expect(opened, <String>['archive.zip']);
      _openCallback(tester, entry)();
      await tester.pump();
      _expectBusy(tester, entry);

      previous.completeError(_timeout);
      await tester.pump();
      _expectBusy(tester, entry);
      expect(AppToastController.instance.notices.value, isEmpty);
      current.complete();
      await tester.pumpAndSettle();
      expect(opened, <String>['archive.zip', 'new-archive.zip']);
      _expectReady(tester, entry);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  fileTest('only the COMMAND_ERROR timeout contract uses the timeout notice', (
    tester,
  ) async {
    const cases = <(CoreLinkError, String)>[
      (
        CoreLinkError(
          code: 'COMMAND_ERROR',
          message: 'host interaction timed out',
        ),
        _timeoutNotice,
      ),
      (
        CoreLinkError(
          code: 'COMMAND_ERROR',
          message: 'No application can open this file',
        ),
        'File operation failed: No application can open this file',
      ),
      (
        CoreLinkError(
          code: 'OTHER_ERROR',
          message: 'host interaction timed out: other-error',
        ),
        'File operation failed: host interaction timed out: other-error',
      ),
      (
        CoreLinkError(
          code: 'COMMAND_ERROR',
          message: 'host interaction timed out: ',
        ),
        'File operation failed: host interaction timed out:',
      ),
    ];
    for (final (error, expected) in cases) {
      AppToastController.instance.reset();
      await _pumpEntry(tester, _Entry.binary, (_) => Future<void>.error(error));
      _openCallback(tester, _Entry.binary)();
      await tester.pumpAndSettle();
      expect(AppToastController.instance.notices.value.single.text, expected);
      expect(tester.takeException(), isNull);
    }
    await tester.pumpWidget(const SizedBox.shrink());
  });

  fileTest('file capabilities are removed from local feedback and diagnostics', (
    tester,
  ) async {
    const uri =
        'content://app.operit.openfiles/1:2/%2Fprivate%2Fworkspace%2Farchive.zip';
    const path = '/private/workspace/Project/archive.zip';
    const error = CoreLinkError(
      code: 'COMMAND_ERROR',
      message: 'Viewer refused $uri for $path',
      details: <String, Object?>{
        'errorType': 'String',
        'fields': <String, Object?>{'uri': uri, 'path': path},
      },
      backtrace: 'viewer_failure correlation-test-id at $path',
    );
    await _pumpEntry(
      tester,
      _Entry.binary,
      (_) =>
          Future<void>.error(error, StackTrace.fromString('openFile at $path')),
    );
    _openCallback(tester, _Entry.binary)();
    await tester.pumpAndSettle();
    final diagnostic = _diagnostics.toString();
    final notice = AppToastController.instance.notices.value.single.text;
    for (final sensitive in <String>[uri, path]) {
      expect(diagnostic, isNot(contains(sensitive)));
      expect(notice, isNot(contains(sensitive)));
    }
    expect(diagnostic, contains('Details:'));
    expect(diagnostic, contains('String'));
    expect(diagnostic, contains('correlation-test-id'));
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  fileTest('a ZIP uses binary preview without reading workspace bytes', (
    tester,
  ) async {
    final pending = Completer<void>();
    var reads = 0;
    var opens = 0;
    await tester.pumpWidget(
      _app(
        _preview(
          'large-archive.zip',
          (_) {
            opens++;
            return pending.future;
          },
          read: (_) async {
            reads++;
            return _png;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(reads, 0);
    _openCallback(tester, _Entry.binary)();
    await tester.pump();
    expect(reads, 0);
    expect(opens, 1);
    pending.complete();
    await tester.pumpAndSettle();
    expect(reads, 0);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  fileTest('the loaded preview toolbar preserves the external-open Future', (
    tester,
  ) async {
    final pending = Completer<void>();
    var reads = 0;
    var opens = 0;
    await tester.pumpWidget(
      _app(
        _preview(
          'picture.png',
          (_) {
            opens++;
            return pending.future;
          },
          read: (_) async {
            reads++;
            return _png;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    for (final element in find.byType(Image).evaluate()) {
      await tester.runAsync(
        () => precacheImage((element.widget as Image).image, element),
      );
    }
    await tester.pumpAndSettle();
    expect(reads, 1);
    _openCallback(tester, _Entry.toolbar)();
    await tester.pump();
    _expectBusy(tester, _Entry.toolbar);
    expect(opens, 1);
    pending.completeError(_timeout);
    await tester.pumpAndSettle();
    expect(
      AppToastController.instance.notices.value.single.text,
      _timeoutNotice,
    );
    expect(reads, 1);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  fileTest('download location failures use the same awaited local feedback', (
    tester,
  ) async {
    final store = _DownloadStore('downloads/archive.zip');
    addTearDown(store.dispose);
    final pending = Completer<void>();
    final opened = <String>[];
    await _pumpEntry(tester, _Entry.download, (path) {
      opened.add(path);
      return pending.future;
    }, store: store);
    final dynamic menu = tester.widget(_downloadMenu());
    final List<PopupMenuEntry<dynamic>> actions = menu.itemBuilder(
      tester.element(_downloadMenu()),
    );
    menu.onSelected(
      actions.whereType<PopupMenuItem<dynamic>>().elementAt(1).value,
    );
    await tester.pump();
    expect(opened, <String>['downloads']);
    _expectBusy(tester, _Entry.download);
    pending.completeError(StateError('Cannot open a directory as a file'));
    await tester.pumpAndSettle();
    expect(
      AppToastController.instance.notices.value.single.text,
      'File operation failed: Bad state: Cannot open a directory as a file',
    );
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

/// Captures emitted diagnostics and removes toast timers before widget invariant checks.
void fileTest(String description, WidgetTesterCallback callback) {
  testWidgets(description, (tester) async {
    AppToastController.instance.reset();
    _diagnostics.clear();
    try {
      await runZoned(
        () => callback(tester),
        zoneSpecification: ZoneSpecification(
          print: (_, _, _, line) => _diagnostics.writeln(line),
        ),
      );
    } finally {
      AppToastController.instance.reset();
      await tester.pump();
    }
  });
}

Widget _app(Widget child) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: MaterialApp(
    locale: const Locale('en'),
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  ),
);

Widget _preview(
  String path,
  Future<void> Function(String) open, {
  Future<Uint8List> Function(String)? read,
}) => WorkspaceFilePreviewContent(
  tab: WorkspaceTab(
    kind: WorkspaceTabKind.filePreview,
    title: path,
    icon: Icons.insert_drive_file_outlined,
    filePath: path,
    previewKind: workspacePreviewKindForPath(path),
  ),
  onReadWorkspaceFileBytes:
      read ??
      ((_) =>
          throw StateError('The binary ZIP preview must not read its bytes')),
  onWriteWorkspaceFileBytes: (_, _) async {},
  onOpenWorkspaceFile: open,
  onOpenBrowser: ({url, localFilePath, workspaceHtmlPath}) {},
  splitMarkdownContent: (_) async => [],
);

Future<void> _pumpEntry(
  WidgetTester tester,
  _Entry entry,
  Future<void> Function(String) open, {
  String path = 'archive.zip',
  _DownloadStore? store,
}) async {
  final child = switch (entry) {
    _Entry.binary => _preview(path, open),
    _Entry.toolbar => WorkspaceFilePreviewActionBar(
      canOpenInBrowser: false,
      onOpenInBrowser: () {},
      canOpenWorkspaceFile: true,
      workspaceFileIdentity: path,
      onOpenWorkspaceFile: () => open(path),
    ),
    _Entry.download => WorkspaceBrowserDownloadSheet(
      store: store!,
      onOpenWorkspaceFile: open,
    ),
  };
  await tester.pumpWidget(_app(child));
  if (entry == _Entry.download) {
    await tester.tap(find.text('Completed'));
  }
  await tester.pumpAndSettle();
}

Finder _filledButton() =>
    find.byWidgetPredicate((widget) => widget is FilledButton);

Finder _toolbarButton() => find.byWidgetPredicate(
  (widget) =>
      widget is IconButton &&
      (widget.tooltip == 'Open file' || widget.tooltip == 'Opening file…'),
);

Finder _downloadMenu() =>
    find.byWidgetPredicate((widget) => widget is PopupMenuButton<dynamic>);

/// Captures the actual event callback, including one that outlives a rebuild.
VoidCallback _openCallback(WidgetTester tester, _Entry entry) {
  switch (entry) {
    case _Entry.binary:
      return tester.widget<FilledButton>(_filledButton()).onPressed!;
    case _Entry.toolbar:
      return tester.widget<IconButton>(_toolbarButton()).onPressed!;
    case _Entry.download:
      final menu = tester.widget<PopupMenuButton<dynamic>>(_downloadMenu());
      final open = menu
          .itemBuilder(tester.element(_downloadMenu()))
          .whereType<PopupMenuItem<dynamic>>()
          .first;
      return () => (menu as dynamic).onSelected(open.value);
  }
}

void _expectBusy(WidgetTester tester, _Entry entry) {
  switch (entry) {
    case _Entry.binary:
      expect(tester.widget<FilledButton>(_filledButton()).onPressed, isNull);
      expect(find.text('Opening file…'), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    case _Entry.toolbar:
      expect(tester.widget<IconButton>(_toolbarButton()).onPressed, isNull);
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    case _Entry.download:
      expect(find.text('Opening file…'), findsOneWidget);
      expect(find.byType(LinearProgressIndicator), findsOneWidget);
      final menu = tester.widget<PopupMenuButton<dynamic>>(_downloadMenu());
      final actions = menu
          .itemBuilder(tester.element(_downloadMenu()))
          .whereType<PopupMenuItem<dynamic>>()
          .take(2);
      expect(actions.every((action) => !action.enabled), isTrue);
  }
}

void _expectReady(WidgetTester tester, _Entry entry) {
  switch (entry) {
    case _Entry.binary:
      expect(tester.widget<FilledButton>(_filledButton()).onPressed, isNotNull);
    case _Entry.toolbar:
      expect(tester.widget<IconButton>(_toolbarButton()).onPressed, isNotNull);
    case _Entry.download:
      final menu = tester.widget<PopupMenuButton<dynamic>>(_downloadMenu());
      final actions = menu
          .itemBuilder(tester.element(_downloadMenu()))
          .whereType<PopupMenuItem<dynamic>>()
          .take(2);
      expect(actions.every((action) => action.enabled), isTrue);
  }
}

/// Supplies completed rows without fetching bytes or touching Core storage.
class _DownloadStore extends WorkspaceBrowserDownloadStore {
  _DownloadStore(this._path)
    : super(
        runtimeStorage: GeneratedCoreProxyClients(
          const _UnusedBridge(),
        ).repositoryRuntimeStorageRepository,
        storagePath: () async => '/unused-downloads.json',
        downloadDirectoryPath: () async => '/unused-downloads',
      );

  String _path;

  @override
  List<WorkspaceBrowserDownloadItem> get items =>
      <WorkspaceBrowserDownloadItem>[
        WorkspaceBrowserDownloadItem(
          url: 'https://example.test/archive.zip',
          fileName: _path.split('/').last,
          state: WorkspaceBrowserDownloadState.completed,
          createdAt: DateTime.fromMillisecondsSinceEpoch(0),
          progress: 100,
          savedPath: _path,
        ),
      ];

  void replacePath(String path) {
    _path = path;
    notifyListeners();
  }
}

class _UnusedBridge extends OperitRuntimeBridge {
  const _UnusedBridge();

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) =>
      throw StateError('These UI tests must not access Core storage');

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}
