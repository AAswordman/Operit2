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

/// Runs daily chat-only backups through the registered persistent directory Host.
class ChatBackupService extends ChangeNotifier {
  /// Creates a process owner; injectable clients and storage keep checks independent of UI.
  ChatBackupService({
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
    PersistentDirectoryStorage? storage,
  }) : _storage = storage ?? MountedChatBackupDirectoryStorage(clients);

  static final ChatBackupService instance = ChatBackupService();
  final PersistentDirectoryStorage _storage;
  Future<void>? _active;
  bool _selecting = false;
  bool _restoring = false;
  bool needsDirectory = false;
  bool promptDeferred = false;
  String? error;
  DateTime? lastSuccess;
  PersistentDirectory? _directory;

  /// Exposes the last validated descriptor without throwing during widget rendering.
  PersistentDirectory? get directory => _directory;

  /// Decodes the machine-local directory record independently of synced preferences.
  PersistentDirectory? _readDirectory() {
    final value = RuntimeBootstrapManager.instance.config.chatBackupDirectory;
    if (value == null) return null;
    final record = jsonDecode(value) as Map<String, Object?>;
    return PersistentDirectory(
      path: record['path'] as String,
      label: record['label'] as String,
    );
  }

  /// Serializes checks so lifecycle changes cannot create duplicate backups or picker routes.
  Future<void> checkAndBackup({bool force = false}) {
    if (_selecting || _restoring) return Future<void>.value();
    final current = _active;
    if (current != null) return current;
    final future = _checkAndBackup(force: force);
    _active = future;
    return future.whenComplete(() => _active = null);
  }

  /// Checks accessibility before exporting and only prunes after verified publication.
  Future<void> _checkAndBackup({required bool force}) async {
    final PersistentDirectory? selected;
    try {
      selected = _readDirectory();
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
    notifyListeners();
    try {
      final now = DateTime.now();
      final identity = RuntimeBootstrapManager.instance.activeIdentity.id;
      final prefix = 'operit-chats-$identity-';
      final pattern = RegExp(
        '^${RegExp.escape(prefix)}(\\d{4}-\\d{2}-\\d{2})-(\\d+)\\.sqlite\$',
      );
      final files = (await _storage.list(
        selected,
      )).where(pattern.hasMatch).toList()..sort();
      final day =
          '${now.year.toString().padLeft(4, '0')}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
      if (!force) {
        for (final name in files.reversed.where(
          (name) => pattern.firstMatch(name)!.group(1) == day,
        )) {
          try {
            final saved = await _storage.inspect(selected, name);
            lastSuccess = DateTime.fromMillisecondsSinceEpoch(saved.createdAt);
            notifyListeners();
            return;
          } catch (failure, stack) {
            ClientLogger.e(
              'Ignoring incomplete chat backup when checking daily success',
              tag: 'ChatBackup',
              error: failure,
              stackTrace: stack,
            );
          }
        }
      }
      final name =
          '$prefix${force ? 'manual-' : ''}$day-${now.millisecondsSinceEpoch}.sqlite';
      final snapshot = await _storage.create(selected, name);
      if (snapshot == null) return;
      lastSuccess = DateTime.fromMillisecondsSinceEpoch(snapshot.createdAt);
      // Retention belongs to this identity and this installation's source ID only.
      // A fresh install's empty database never removes archives from older identities.
      if (!force) {
        final automatic = <String>[...files, name]..sort();
        for (final old in automatic.take(
          (automatic.length - 10).clamp(0, automatic.length),
        )) {
          await _storage.remove(selected, old);
        }
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
    try {
      await _active;
      final selected = await _storage.select();
      if (selected == null) return false;
      await _storage.check(selected);
      await RuntimeBootstrapManager.instance.saveChatBackupDirectory(
        jsonEncode({'path': selected.path, 'label': selected.label}),
      );
      _directory = selected;
      needsDirectory = false;
      promptDeferred = false;
      error = null;
      lastSuccess = null;
      notifyListeners();
      return true;
    } catch (failure, stack) {
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

  /// Lists verified snapshots from every previous installation in the selected external folder.
  Future<List<ChatBackupSnapshot>> listSnapshots() async {
    await _active;
    final selected = _readDirectory();
    if (selected == null) {
      throw StateError('Select an external backup folder first');
    }
    _directory = selected;
    await _storage.check(selected);
    final snapshots = <ChatBackupSnapshot>[];
    final invalid = <String>[];
    for (final name in await _storage.list(selected)) {
      if (!name.startsWith('operit-chats-') || !name.endsWith('.sqlite')) {
        continue;
      }
      try {
        snapshots.add(await _storage.inspect(selected, name));
      } catch (failure) {
        invalid.add('$name: $failure');
      }
    }
    snapshots.sort((a, b) => b.createdAt.compareTo(a.createdAt));
    error = invalid.isEmpty ? null : invalid.join('\n');
    notifyListeners();
    return snapshots;
  }

  /// Preserves a safety snapshot, then restores chat state while lifecycle backup checks are paused.
  Future<bool> restoreSnapshot(String name) async {
    if (_selecting || _restoring) return false;
    _restoring = true;
    try {
      await _active;
      final selected = _readDirectory();
      if (selected == null) {
        throw StateError('Select an external backup folder first');
      }
      await _storage.check(selected);
      await _storage.inspect(selected, name);
      final identity = RuntimeBootstrapManager.instance.activeIdentity.id;
      final safety =
          'operit-chats-$identity-before-restore-${DateTime.now().millisecondsSinceEpoch}.sqlite';
      await _storage.create(selected, safety);
      await _storage.restore(selected, name);
      error = null;
      lastSuccess = null;
      notifyListeners();
      return true;
    } catch (failure, stack) {
      _report(failure, stack, invalidDirectory: false);
      return false;
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
