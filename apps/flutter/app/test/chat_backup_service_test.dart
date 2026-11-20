import 'dart:convert';
import 'package:operit2/core/backup/ChatBackupDirectoryStorage.dart';
import 'package:flutter/services.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/core/backup/ChatBackupService.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/core/runtime/RuntimeBootstrapManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/common/ChatBackupDirectoryPanel.dart';
import 'package:operit2/ui/features/settings/data/DataSettingsPanel.dart';

/// Exercises upgrade prompting, directory loss, daily deduplication and safe retention.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final manager = RuntimeBootstrapManager.instance;
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  const channel = MethodChannel('operit/runtime');
  setUpAll(ClientLogger.initialize);

  setUp(() async {
    messenger.setMockMethodCallHandler(channel, (call) async {
      if (call.method == 'runtimeBootstrapRead') {
        return jsonEncode({'ok': true, 'value': null});
      }
      if (call.method == 'runtimeBootstrapWrite') {
        return jsonEncode({'ok': true});
      }
      throw StateError('Unexpected Host method: ${call.method}');
    });
    await manager.initializeForDetachedWindow();
  });

  tearDown(() => messenger.setMockMethodCallHandler(channel, null));

  test(
    'backup selection mounts the shared folder source and stores only its VFS path',
    () async {
      final original = OperitFolderAccessPlatform.instance;
      final picker = _ExternalFolderPicker();
      OperitFolderAccessPlatform.instance = picker;
      addTearDown(() => OperitFolderAccessPlatform.instance = original);
      final bridge = _MountBridge();
      final service = ChatBackupService(
        clients: GeneratedCoreProxyClients(bridge),
      );
      expect(await service.selectDirectory(), isTrue);
      expect(picker.selections, 1);
      expect(bridge.source, {
        'namespace': '/virtual/test/folders',
        'backend': 'test_documents',
        'root': 'opaque-authorized-folder',
        'name': 'External backups',
      });
      expect(bridge.checkedDirectory, _MountBridge.path);
      expect(jsonDecode(manager.config.chatBackupDirectory!), {
        'path': _MountBridge.path,
        'label': 'External backups',
      });
      service.dispose();
    },
  );

  for (final brightness in Brightness.values) {
    testWidgets(
      'missing directory shows an opaque $brightness dialog and returns to the mounted app after selection',
      (tester) async {
        final storage = _DirectoryStorage();
        final service = ChatBackupService(
          storage: storage,
          clients: GeneratedCoreProxyClients(_ChatBridge()),
        );
        final theme = buildOperitThemeData(
          ColorScheme.fromSeed(seedColor: Colors.blue, brightness: brightness),
          ThemePreferenceDefaults.snapshot,
          fontFamily: 'Aptos',
          fontFamilyFallback: const ['sans-serif'],
        );
        expect(theme.scaffoldBackgroundColor, Colors.transparent);
        await tester.pumpWidget(
          MaterialApp(
            theme: theme,
            locale: const Locale('zh'),
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: ChatBackupHost(
              service: service,
              child: const Scaffold(body: Text('Existing application')),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.text('选择你的备份目录'), findsOneWidget);
        expect(find.byType(Dialog), findsOneWidget);
        final barrier = tester.widget<ModalBarrier>(
          find.descendant(
            of: find.byType(ChatBackupHost),
            matching: find.byType(ModalBarrier),
          ),
        );
        expect(barrier.dismissible, isFalse);
        expect(barrier.color!.a, greaterThan(0));
        final surface = tester.widget<Material>(
          find
              .descendant(
                of: find.byType(Dialog),
                matching: find.byType(Material),
              )
              .first,
        );
        expect(surface.color, theme.colorScheme.surfaceContainerHigh);
        expect(surface.color!.a, 1);
        await tester.tapAt(const Offset(4, 4));
        await tester.pumpAndSettle();
        expect(find.text('选择你的备份目录'), findsOneWidget);
        await tester.runAsync(() async {
          await tester.tap(find.text('选择备份目录'));
          // Drain the channel write and the panel's unawaited initial backup.
          await Future<void>.delayed(Duration.zero);
          await service.checkAndBackup();
        });
        await tester.pumpAndSettle();
        expect(find.text('选择你的备份目录'), findsNothing);
        expect(find.text('Existing application'), findsOneWidget);
        storage.available = false;
        await tester.runAsync(service.checkAndBackup);
        await tester.pumpAndSettle();
        expect(find.text('选择你的备份目录'), findsOneWidget);
        tester.view.physicalSize = const Size(360, 480);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        await tester.pumpAndSettle();
        await tester.ensureVisible(find.text('稍后设置'));
        await tester.tap(find.text('稍后设置'));
        await tester.pumpAndSettle();
        expect(find.byType(Dialog), findsNothing);
        expect(find.text('Existing application'), findsOneWidget);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
        service.dispose();
      },
    );
  }

  test(
    'old bootstrap without directory prompts; cancelled selection preserves configuration',
    () async {
      final storage = _DirectoryStorage()..selection = null;
      final service = ChatBackupService(storage: storage);
      await service.checkAndBackup();
      expect(service.needsDirectory, isTrue);
      expect(await service.selectDirectory(), isFalse);
      expect(service.needsDirectory, isTrue);
      expect(manager.config.chatBackupDirectory, isNull);
      service.dispose();
    },
  );

  test(
    'valid selection survives root edits and checks revoked access even after daily success',
    () async {
      final storage = _DirectoryStorage();
      final bridge = _ChatBridge();
      final service = ChatBackupService(
        storage: storage,
        clients: GeneratedCoreProxyClients(bridge),
      );
      expect(await service.selectDirectory(), isTrue);
      final saved = manager.config.chatBackupDirectory;
      expect(
        manager.config.copyWith(runtimeRoot: '/different').chatBackupDirectory,
        saved,
      );
      await service.checkAndBackup();
      expect(storage.files, hasLength(1));
      await service.checkAndBackup();
      expect(storage.creates, 1);
      storage.available = false;
      await service.checkAndBackup();
      expect(service.needsDirectory, isTrue);
      expect(manager.config.chatBackupDirectory, saved);
      expect(storage.files, hasLength(1));
      service.dispose();
    },
  );

  for (final operation in ['selection', 'access']) {
    test(
      'failed directory $operation rejects selection and preserves the saved destination',
      () async {
        final storage = _DirectoryStorage()
          ..selectFailure = operation == 'selection'
              ? StateError('Directory selection denied')
              : null
          ..checkFailure = operation == 'access'
              ? StateError('Directory access denied')
              : null;
        final service = ChatBackupService(storage: storage);
        expect(await service.selectDirectory(), isFalse);
        expect(manager.config.chatBackupDirectory, isNull);
        expect(service.directory, isNull);
        expect(service.needsDirectory, isTrue);
        expect(service.directorySelectionFailed, isTrue);
        expect(storage.creates, 0);

        storage.checkFailure = null;
        storage.selectFailure = null;
        expect(await service.selectDirectory(), isTrue);
        expect(service.directorySelectionFailed, isFalse);
        expect(service.error, isNull);
        final saved = manager.config.chatBackupDirectory;
        final original = service.directory;
        storage.selection = const PersistentDirectory(
          path: '/virtual/local/folders/unusable',
          label: 'Unusable folder',
        );
        storage.selectFailure = operation == 'selection'
            ? StateError('Directory selection denied')
            : null;
        storage.checkFailure = operation == 'access'
            ? StateError('Directory access denied')
            : null;
        expect(await service.selectDirectory(), isFalse);
        expect(manager.config.chatBackupDirectory, saved);
        expect(service.directory, same(original));
        expect(service.needsDirectory, isFalse);
        expect(service.directorySelectionFailed, isTrue);
        service.dispose();
      },
    );
  }

  testWidgets(
    'unusable backup selection shows actionable folder guidance and stays on setup',
    (tester) async {
      final storage = _DirectoryStorage()
        ..checkFailure = StateError('Directory access denied');
      final service = ChatBackupService(storage: storage);
      var selections = 0;
      await tester.pumpWidget(
        MaterialApp(
          locale: const Locale('zh'),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ChatBackupDirectoryPanel(
              service: service,
              onSelected: () => selections++,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('选择备份目录'));
      await tester.pumpAndSettle();
      expect(
        find.text('无法使用所选文件夹。请在可用的文件夹（如“文档”或“下载”）下新建目录，并选择该目录。'),
        findsOneWidget,
      );
      expect(find.textContaining('Directory access denied'), findsNothing);
      expect(selections, 0);
      expect(manager.config.chatBackupDirectory, isNull);
      expect(find.text('选择备份目录'), findsOneWidget);
      await tester.pumpWidget(const SizedBox.shrink());
      service.dispose();
    },
  );

  test(
    'lost write access is reported even after a successful backup today',
    () async {
      final storage = _DirectoryStorage();
      final service = ChatBackupService(storage: storage);
      expect(await service.selectDirectory(), isTrue);
      await service.checkAndBackup();
      expect(storage.creates, 1);
      final saved = manager.config.chatBackupDirectory;
      storage.checkFailure = StateError('Probe file write denied');
      await service.checkAndBackup();
      expect(service.needsDirectory, isTrue);
      expect(service.error, contains('Probe file write denied'));
      expect(manager.config.chatBackupDirectory, saved);
      expect(storage.creates, 1);
      expect(storage.files, hasLength(1));

      storage.checkFailure = null;
      await service.checkAndBackup();
      expect(service.needsDirectory, isFalse);
      expect(service.error, isNull);
      expect(storage.creates, 1);
      service.dispose();
    },
  );

  test(
    'retention follows successful publication and never deletes manual or foreign backups',
    () async {
      final storage = _DirectoryStorage();
      final bridge = _ChatBridge();
      final service = ChatBackupService(
        storage: storage,
        clients: GeneratedCoreProxyClients(bridge),
      );
      await service.selectDirectory();
      final prefix = 'operit-chats-${manager.activeIdentity.id}-';
      for (var day = 1; day <= 12; day++) {
        final name =
            '${prefix}2000-01-${day.toString().padLeft(2, '0')}-1.sqlite';
        storage.files[name] = _snapshot(name);
      }
      final manual = '${prefix}manual-2000-01-01-1.sqlite';
      final foreign = 'operit-chats-other-install-2000-01-01-1.sqlite';
      storage.files[manual] = _snapshot(manual);
      storage.files[foreign] = _snapshot(foreign);
      storage.failWrite = true;
      await service.checkAndBackup();
      expect(storage.files, hasLength(14));
      expect(storage.removed, isEmpty);
      storage.failWrite = false;
      await service.checkAndBackup();
      expect(storage.files, hasLength(12));
      expect(storage.files, contains(manual));
      expect(storage.files, contains(foreign));
      service.dispose();
    },
  );

  test('daily checks use filenames without opening backup contents', () async {
    final storage = _DirectoryStorage();
    final service = ChatBackupService(storage: storage);
    await service.selectDirectory();
    final now = DateTime.now();
    final day =
        '${now.year}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
    final name = 'operit-chats-${manager.activeIdentity.id}-$day-1.sqlite';
    storage.files[name] = null;
    await service.checkAndBackup();
    await service.checkAndBackup();
    expect(storage.creates, 0);
    expect(storage.files, hasLength(1));
    expect(service.lastSuccess, DateTime.fromMillisecondsSinceEpoch(1));
    service.dispose();
  });

  testWidgets(
    'settings only lists backups on request and restores from an independent folder',
    (tester) async {
      final storage = _DirectoryStorage();
      final service = ChatBackupService(storage: storage);
      await tester.runAsync(service.selectDirectory);
      final configured = manager.config.chatBackupDirectory;
      final source = const PersistentDirectory(
        path: '/virtual/local/folders/another',
        label: 'Another folder',
      );
      storage.selection = source;
      const name =
          'operit-chats-previous-install-2026-01-01-1767268800000.sqlite';
      tester.view.physicalSize = const Size(360, 640);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot: ThemePreferenceDefaults.snapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: Scaffold(
            body: DataSettingsPanel(
              clients: GeneratedCoreProxyClients(_ChatBridge()),
              backupService: service,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = AppLocalizations.of(
        tester.element(find.byType(DataSettingsPanel)),
      )!;
      expect(storage.lists, isEmpty);
      expect(storage.creates, 0);
      await tester.tap(find.text(l10n.chatRestoreTitle));
      await tester.pumpAndSettle();
      expect(find.text(l10n.chatRestoreEmpty), findsOneWidget);
      storage.files[name] = _snapshot(name);
      await tester.tap(find.text(l10n.chatRestoreChooseFolder));
      await tester.pumpAndSettle();
      expect(manager.config.chatBackupDirectory, configured);
      expect(storage.lists.last, source.path);
      expect(find.text('Another folder'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.widgetWithText(FilledButton, l10n.chatRestoreSelected),
            )
            .onPressed,
        isNull,
      );
      await tester.ensureVisible(find.text(name));
      await tester.pumpAndSettle();
      await tester.tap(find.text(name));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.chatRestoreSelected));
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.chatRestoreConfirmDescription(name)),
        findsOneWidget,
      );
      expect(storage.restored, isEmpty);
      await tester.tap(find.widgetWithText(TextButton, l10n.cancel).last);
      await tester.pumpAndSettle();
      expect(storage.restored, isEmpty);
      await tester.tap(find.text(l10n.chatRestoreSelected));
      await tester.pumpAndSettle();
      await tester.tap(
        find.widgetWithText(FilledButton, l10n.chatRestoreAction),
      );
      await tester.pumpAndSettle();
      expect(storage.restored, [name]);
      expect(storage.restoreSource, source.path);
      expect(storage.creates, 0);
      expect(find.text(l10n.chatRestoreSuccess), findsOneWidget);
      await tester.tap(find.text(l10n.ok));
      await tester.pumpAndSettle();
      expect(manager.config.chatBackupDirectory, configured);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      service.dispose();
    },
  );

  test(
    'listing reads filenames and restoring copies only the selected backup',
    () async {
      final storage = _DirectoryStorage();
      final service = ChatBackupService(storage: storage);
      await service.selectDirectory();
      const old = 'operit-chats-old-install-2026-01-01-1.sqlite';
      const another = 'operit-chats-old-install-2026-01-02-2.sqlite';
      storage.files[old] = _snapshot(old);
      storage.files[another] = null;
      storage.files['unrelated.txt'] = null;
      final source = service.configuredDirectory!;
      expect((await service.listSnapshots(source)).map((s) => s.name), [
        another,
        old,
      ]);
      expect(service.error, isNull);
      storage.failWrite = true;
      await service.restoreSnapshot(source, old);
      expect(storage.restored, [old]);
      expect(storage.creates, 0);
      expect(storage.files, hasLength(3));
      service.dispose();
    },
  );
}

/// Supplies metadata for a valid physical database without a native runtime in service tests.
ChatBackupSnapshot _snapshot(String name, {int createdAt = 1}) =>
    ChatBackupSnapshot(name: name, createdAt: createdAt);

/// Models a durable external directory independently of application storage.
class _DirectoryStorage implements PersistentDirectoryStorage {
  PersistentDirectory? selection = const PersistentDirectory(
    path: '/virtual/local/folders/external',
    label: 'User backups',
  );
  bool available = true;
  Object? selectFailure;
  Object? checkFailure;
  bool failWrite = false;
  int creates = 0;
  final files = <String, ChatBackupSnapshot?>{};
  final removed = <String>[];
  final restored = <String>[];
  final lists = <String>[];
  String? restoreSource;

  /// Returns the explicit selection or user cancellation.
  @override
  Future<PersistentDirectory?> select() async {
    final failure = selectFailure;
    if (failure != null) throw failure;
    return selection;
  }

  /// Models revocation without removing retained backup bytes.
  @override
  Future<void> check(PersistentDirectory directory) async {
    if (!available) throw StateError('Directory authorization revoked');
    final failure = checkFailure;
    if (failure != null) throw failure;
  }

  /// Exposes partial-write failure before reporting success.
  @override
  Future<ChatBackupSnapshot> create(
    PersistentDirectory directory,
    String name,
  ) async {
    if (failWrite) throw StateError('Disk full');
    creates++;
    final value = _snapshot(
      name,
      createdAt: DateTime.now().millisecondsSinceEpoch,
    );
    files[name] = value;
    return value;
  }

  /// Lists every user file so retention must filter its ownership.
  @override
  Future<List<String>> list(PersistentDirectory directory) async {
    lists.add(directory.path);
    return files.keys.toList();
  }

  /// Records exactly one restore without creating another backup.
  @override
  Future<ChatBackupSnapshot> restore(
    PersistentDirectory directory,
    String name,
  ) async {
    restored.add(name);
    restoreSource = directory.path;
    return files[name]!;
  }

  /// Records only explicit retention deletions.
  @override
  Future<void> remove(PersistentDirectory directory, String name) async {
    removed.add(name);
    files.remove(name);
  }
}

/// Provides a chat export without starting the native runtime.
class _ChatBridge extends OperitRuntimeBridge {
  /// Rejects unexpected calls because snapshot files never pass through Flutter as chat JSON.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    throw StateError('Unexpected snapshot call: ${request.methodName}');
  }

  /// Rejects unused stream calls.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects unused snapshot calls.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Rejects unused event calls.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}

/// Supplies the same platform-neutral resource descriptor used by workspace folder selection.
class _ExternalFolderPicker extends OperitFolderAccessPlatform {
  int selections = 0;

  /// Returns an authorized resource without exposing a native filesystem path.
  @override
  Future<FolderMountSource?> pickWorkspaceDirectory({
    String? initialDirectory,
  }) async {
    selections++;
    return const FolderMountSource(
      backend: 'test_documents',
      root: 'opaque-authorized-folder',
      namespace: '/virtual/test/folders',
      name: 'External backups',
    );
  }

  /// Rejects physical-path selection because backups must use resource mounts.
  @override
  Future<String?> pickDirectory({String? initialDirectory}) =>
      throw UnimplementedError();

  /// Rejects unused multiple-directory selection.
  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) =>
      throw UnimplementedError();
}

/// Verifies the shared mount registration precedes backup access through its returned VFS path.
class _MountBridge extends _ChatBridge {
  static const path = '/virtual/test/folders/registered-folder';
  Map<String, Object?>? source;
  String? checkedDirectory;

  /// Models the mount catalog and the backup directory accessibility check.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final args = request.args as Map<String, Object?>;
    switch (request.methodName) {
      case 'mountDirectory':
        source =
            jsonDecode(args['sourceJson'] as String) as Map<String, Object?>;
        return encodeCoreLink([0, path]);
      case 'checkDirectory':
        checkedDirectory = args['directory'] as String;
        return encodeCoreLink([0, null]);
      default:
        throw StateError('Unexpected mount call: ${request.methodName}');
    }
  }
}
