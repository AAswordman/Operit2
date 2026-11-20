// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'ChatBackupDirectoryStorage.dart';
import '../host/MountedChatBackupDirectoryStorage.dart';

import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../logging/ClientLogger.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import '../runtime/RuntimeBootstrapManager.dart';

/// Copies the database daily through the registered persistent directory Host.
class ChatBackupService extends ChangeNotifier {
  /// Creates a process owner; injectable clients and storage keep checks independent of UI.
  ChatBackupService({
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
    PersistentDirectoryStorage? storage,
    DateTime Function()? now,
  }) : _storage = storage ?? MountedChatBackupDirectoryStorage(clients),
       _now = now ?? DateTime.now;

  static final ChatBackupService instance = ChatBackupService();
  final PersistentDirectoryStorage _storage;
  final DateTime Function() _now;
  Future<void>? _active;
  bool _selecting = false;
  bool _restoring = false;
  bool needsDirectory = false;
  bool promptDeferred = false;
  bool directorySelectionFailed = false;
  String? error;
  DateTime? lastSuccess;
  PersistentDirectory? _directory;

  /// Exposes the last validated descriptor without throwing during widget rendering.
  PersistentDirectory? get directory => _directory;

  /// Indicates that a scheduled or manual backup check is running.
  bool get backingUp => _active != null;

  /// Decodes the machine-local directory record independently of synced preferences.
  PersistentDirectory? get configuredDirectory {
    final value = RuntimeBootstrapManager.instance.config.chatBackupDirectory;
    if (value == null) return null;
    final record = jsonDecode(value) as Map<String, Object?>;
    return PersistentDirectory(
      path: record['path'] as String,
      label: record['label'] as String,
    );
  }

  /// Serializes checks so lifecycle changes cannot create duplicate backups or picker routes.
  Future<void> checkAndBackup() {
    if (_selecting || _restoring) return Future<void>.value();
    final current = _active;
    if (current != null) return current;
    final future = Future<void>.microtask(_checkAndBackup).whenComplete(() {
      _active = null;
      notifyListeners();
    });
    _active = future;
    notifyListeners();
    return future;
  }

  /// Checks daily filenames and copies the database once when today's backup is absent.
  Future<void> _checkAndBackup() async {
    final PersistentDirectory? selected;
    try {
      selected = configuredDirectory;
      if (_directory?.path != selected?.path) lastSuccess = null;
      _directory = selected;
    } catch (failure, stack) {
      _directory = null;
      _report(failure, stack, invalidDirectory: true);
      return;
    }
    if (selected == null) {
      needsDirectory = true;
      notifyListeners();
      return;
    }
    try {
      await _storage.check(selected);
    } catch (failure, stack) {
      _report(failure, stack, invalidDirectory: true);
      return;
    }
    needsDirectory = false;
    promptDeferred = false;
    error = null;
    directorySelectionFailed = false;
    notifyListeners();
    try {
      final now = _now().toLocal();
      final identity = RuntimeBootstrapManager.instance.activeIdentity.id;
      final prefix = 'operit-chats-$identity-';
      final day =
          '${now.year.toString().padLeft(4, '0')}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
      final backups = <({String name, String day, int timestamp})>[];
      final pattern = RegExp(
        '^${RegExp.escape(prefix)}(?:manual-)?(\\d{4}-\\d{2}-\\d{2})-(\\d+)\\.sqlite\$',
      );
      for (final name in await _storage.list(selected)) {
        final match = pattern.firstMatch(name);
        if (match == null) continue;
        backups.add((
          name: name,
          day: match.group(1)!,
          timestamp: int.parse(match.group(2)!),
        ));
      }
      if (!backups.any((backup) => backup.day == day)) {
        final name = '$prefix$day-${now.millisecondsSinceEpoch}.sqlite';
        final snapshot = await _storage.create(selected, name);
        backups.add((name: name, day: day, timestamp: snapshot.createdAt));
      }
      backups.sort((a, b) {
        final dateOrder = a.day.compareTo(b.day);
        if (dateOrder != 0) return dateOrder;
        final timeOrder = a.timestamp.compareTo(b.timestamp);
        return timeOrder != 0 ? timeOrder : a.name.compareTo(b.name);
      });
      lastSuccess = DateTime.fromMillisecondsSinceEpoch(backups.last.timestamp);
      for (final old in backups.take(
        (backups.length - 10).clamp(0, backups.length),
      )) {
        await _storage.remove(selected, old.name);
      }
      notifyListeners();
    } catch (failure, stack) {
      _report(failure, stack, invalidDirectory: false);
      // A revoked grant or removed folder must also reopen the directory page.
      try {
        await _storage.check(selected);
      } catch (failure, stack) {
        _report(failure, stack, invalidDirectory: true);
      }
    }
  }

  /// Lets the Host obtain a new persistent grant, preserving the old record on cancel or error.
  Future<bool> selectDirectory() async {
    if (_selecting || _restoring) return false;
    _selecting = true;
    var validatingDirectory = false;
    try {
      await _active;
      validatingDirectory = true;
      final selected = await _storage.select();
      if (selected == null) return false;
      await _storage.check(selected);
      validatingDirectory = false;
      await RuntimeBootstrapManager.instance.saveChatBackupDirectory(
        jsonEncode({'path': selected.path, 'label': selected.label}),
      );
      _directory = selected;
      needsDirectory = false;
      promptDeferred = false;
      error = null;
      directorySelectionFailed = false;
      lastSuccess = null;
      notifyListeners();
      return true;
    } catch (failure, stack) {
      directorySelectionFailed = validatingDirectory;
      _report(
        failure,
        stack,
        invalidDirectory: directory == null || needsDirectory,
      );
      return false;
    } finally {
      _selecting = false;
    }
  }

  /// Opens a folder for this recovery session without changing the backup destination.
  Future<PersistentDirectory?> selectRestoreDirectory() => _storage.select();

  /// Lists backups in the chosen source by filename without reading their contents.
  Future<List<ChatBackupSnapshot>> listSnapshots(
    PersistentDirectory source,
  ) async {
    await _active;
    final snapshots = <ChatBackupSnapshot>[];
    final pattern = RegExp(r'^operit-chats-.+-(\d+)\.sqlite$');
    for (final name in await _storage.list(source)) {
      final match = pattern.firstMatch(name);
      if (match == null) continue;
      snapshots.add(
        ChatBackupSnapshot(name: name, createdAt: int.parse(match.group(1)!)),
      );
    }
    snapshots.sort((a, b) => b.createdAt.compareTo(a.createdAt));
    return snapshots;
  }

  /// Copies the selected backup back while lifecycle backup checks are paused.
  Future<void> restoreSnapshot(PersistentDirectory source, String name) async {
    if (_selecting || _restoring) {
      throw StateError('Another database backup operation is in progress');
    }
    _restoring = true;
    try {
      await _active;
      await _storage.restore(source, name);
    } finally {
      _restoring = false;
    }
  }

  /// Defers setup for this process only, while keeping the unprotected status visible in settings.
  void deferDirectoryPrompt() {
    promptDeferred = true;
    notifyListeners();
  }

  /// Retains actionable failures while keeping the lifecycle observer free of uncaught errors.
  void _report(
    Object failure,
    StackTrace stack, {
    required bool invalidDirectory,
  }) {
    error = '$failure';
    needsDirectory = invalidDirectory;
    ClientLogger.e(
      'Chat backup failed',
      tag: 'ChatBackup',
      error: failure,
      stackTrace: stack,
    );
    notifyListeners();
  }
}
