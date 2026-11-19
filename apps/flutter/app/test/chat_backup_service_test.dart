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
import 'package:operit2/ui/common/ChatBackupDirectoryPanel.dart';
import 'package:operit2/ui/common/ChatBackupRestorePanel.dart';

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

  test(
    'partial daily files do not suppress a complete backup; empty chats never prune',
    () async {
      final storage = _DirectoryStorage();
      final bridge = _ChatBridge();
      storage.empty = true;
      final service = ChatBackupService(
        storage: storage,
        clients: GeneratedCoreProxyClients(bridge),
      );
      await service.selectDirectory();
      final now = DateTime.now();
      final day =
          '${now.year}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
      final broken = 'operit-chats-${manager.activeIdentity.id}-$day-1.sqlite';
      storage.files[broken] = null;
      await service.checkAndBackup();
      expect(storage.files, hasLength(1));
      expect(storage.removed, isEmpty);
      storage.empty = false;
      await service.checkAndBackup();
      expect(storage.files, hasLength(2));
      expect(service.lastSuccess, isNotNull);
      service.dispose();
    },
  );

  testWidgets(
    'restore page shows dated snapshots and requires replacement confirmation',
    (tester) async {
      final storage = _DirectoryStorage();
      final service = ChatBackupService(storage: storage);
      await tester.runAsync(service.selectDirectory);
      const name = 'operit-chats-previous-install-2026-01-01-1.sqlite';
      storage.files[name] = _snapshot(name);
      var restored = false;
      await tester.pumpWidget(
        MaterialApp(
          locale: const Locale('zh'),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: SingleChildScrollView(
              child: ChatBackupRestorePanel(
                service: service,
                onRestored: () => restored = true,
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.textContaining(name), findsOneWidget);
      expect(find.textContaining('1 个对话 · 3 条消息'), findsOneWidget);
      expect(storage.creates, 0);
      await tester.tap(find.text('恢复'));
      await tester.pumpAndSettle();
      expect(find.text('恢复这份聊天快照？'), findsOneWidget);
      expect(storage.restored, isEmpty);
      await tester.tap(find.text('取消'));
      await tester.pumpAndSettle();
      expect(storage.creates, 0);
      await tester.tap(find.text('恢复'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, '恢复'));
      await tester.pumpAndSettle();
      expect(storage.restored, [name]);
      expect(restored, isTrue);
      expect(find.text('聊天记录已恢复。'), findsOneWidget);
      storage.files.clear();
      storage.selection = const PersistentDirectory(
        path: '/virtual/local/folders/another',
        label: 'Another folder',
      );
      await tester.runAsync(() async {
        await service.selectDirectory();
        // Drain the directory observer's real-zone snapshot load before advancing fake frames.
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();
      expect(find.textContaining(name), findsNothing);
      expect(find.text('此目录中没有可恢复的聊天数据库快照。'), findsOneWidget);
      expect(find.text('Another folder'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      service.dispose();
    },
  );

  test(
    'previous-install snapshots are listed and restoration saves current chats first',
    () async {
      final storage = _DirectoryStorage();
      final service = ChatBackupService(storage: storage);
      await service.selectDirectory();
      const old = 'operit-chats-old-install-2026-01-01-1.sqlite';
      const broken = 'operit-chats-old-install-2026-01-02-2.sqlite';
      storage.files[old] = _snapshot(old);
      storage.files[broken] = null;
      expect((await service.listSnapshots()).map((s) => s.name), [old]);
      expect(service.error, contains(broken));
      storage.failWrite = true;
      expect(await service.restoreSnapshot(old), isFalse);
      expect(storage.restored, isEmpty);
      storage.failWrite = false;
      expect(await service.restoreSnapshot(old), isTrue);
      expect(storage.restored, [old]);
      expect(
        storage.files.keys.where(
          (name) => name.startsWith(
            'operit-chats-${manager.activeIdentity.id}-before-restore-',
          ),
        ),
        hasLength(1),
      );
      expect(storage.files[old]!.createdAt, 1);
      service.dispose();
    },
  );
}

/// Supplies metadata for a valid physical database without a native runtime in service tests.
ChatBackupSnapshot _snapshot(String name, {int createdAt = 1}) =>
    ChatBackupSnapshot(
      name: name,
      createdAt: createdAt,
      chatCount: 1,
      messageCount: 3,
    );

/// Models a durable external directory independently of application storage.
class _DirectoryStorage implements PersistentDirectoryStorage {
  PersistentDirectory? selection = const PersistentDirectory(
    path: '/virtual/local/folders/external',
    label: 'User backups',
  );
  bool available = true;
  bool failWrite = false;
  bool empty = false;
  int creates = 0;
  final files = <String, ChatBackupSnapshot?>{};
  final removed = <String>[];
  final restored = <String>[];

  /// Returns the explicit selection or user cancellation.
  @override
  Future<PersistentDirectory?> select() async => selection;

  /// Models revocation without removing retained backup bytes.
  @override
  Future<void> check(PersistentDirectory directory) async {
    if (!available) throw StateError('Directory authorization revoked');
  }

  /// Exposes partial-write failure before reporting success.
  @override
  Future<ChatBackupSnapshot?> create(
    PersistentDirectory directory,
    String name,
  ) async {
    if (failWrite) throw StateError('Disk full');
    creates++;
    if (empty) return null;
    final value = _snapshot(
      name,
      createdAt: DateTime.now().millisecondsSinceEpoch,
    );
    files[name] = value;
    return value;
  }

  /// Lists every user file so retention must filter its ownership.
  @override
  Future<List<String>> list(PersistentDirectory directory) async =>
      files.keys.toList();

  /// Reads existing bytes for daily archive verification.
  @override
  Future<ChatBackupSnapshot> inspect(
    PersistentDirectory directory,
    String name,
  ) async {
    final value = files[name];
    if (value == null) throw StateError('Invalid SQLite database');
    return value;
  }

  /// Records restoration only after the caller has saved its safety snapshot.
  @override
  Future<ChatBackupSnapshot> restore(
    PersistentDirectory directory,
    String name,
  ) async {
    restored.add(name);
    return inspect(directory, name);
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
