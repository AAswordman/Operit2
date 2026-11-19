// ignore_for_file: file_names

import 'dart:convert';
import 'package:operit_folder_access/operit_folder_access.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import '../backup/ChatBackupDirectoryStorage.dart';

/// Uses generic folder selection and the project's VFS without extending third-party APIs.
class MountedChatBackupDirectoryStorage implements PersistentDirectoryStorage {
  /// Binds backup I/O to the local runtime's existing external mount facilities.
  const MountedChatBackupDirectoryStorage(this.clients);
  final GeneratedCoreProxyClients clients;

  /// Uses the same picker and mount registration as external workspace folders.
  @override
  Future<PersistentDirectory?> select() async {
    final source = await OperitFolderAccess.pickWorkspaceDirectory();
    if (source == null) return null;
    final path = await clients.servicesFileSelectionService.mountDirectory(
      sourceJson: jsonEncode({
        'namespace': source.namespace,
        'backend': source.backend,
        'root': source.root,
        'name': source.name,
      }),
    );
    return PersistentDirectory(path: path, label: source.name);
  }

  /// Verifies the saved capability and an actual write/read/delete round trip.
  @override
  Future<void> check(PersistentDirectory directory) async {
    await clients.servicesChatBackupDirectoryService.checkDirectory(
      directory: directory.path,
    );
  }

  /// Publishes through the runtime's verified temporary-file protocol.
  @override
  Future<ChatBackupSnapshot?> create(
    PersistentDirectory directory,
    String name,
  ) => clients.servicesChatBackupDirectoryService.createBackup(
    directory: directory.path,
    name: name,
  );

  /// Enumerates direct filenames through the selected resource backend.
  @override
  Future<List<String>> list(PersistentDirectory directory) => clients
      .servicesChatBackupDirectoryService
      .listBackups(directory: directory.path);

  /// Reads a previous archive for integrity verification.
  @override
  Future<ChatBackupSnapshot> inspect(
    PersistentDirectory directory,
    String name,
  ) => clients.servicesChatBackupDirectoryService.inspectBackup(
    directory: directory.path,
    name: name,
  );

  /// Applies the snapshot inside Core without replacing open database files in Flutter.
  @override
  Future<ChatBackupSnapshot> restore(
    PersistentDirectory directory,
    String name,
  ) => clients.servicesChatBackupDirectoryService.restoreBackup(
    directory: directory.path,
    name: name,
  );

  /// Removes a file selected by the application's retention policy.
  @override
  Future<void> remove(PersistentDirectory directory, String name) => clients
      .servicesChatBackupDirectoryService
      .removeBackup(directory: directory.path, name: name);
}
