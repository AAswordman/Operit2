// ignore_for_file: file_names

import '../proxy/generated/CoreProxyModels.g.dart';

/// A user-selected external folder registered in the runtime's persistent mount catalog.
class PersistentDirectory {
  /// Stores the VFS mount path and the selected folder's display name.
  const PersistentDirectory({required this.path, required this.label});

  final String path;
  final String label;
}

/// Owns selection, authorization checks and publication outside application storage.
abstract class PersistentDirectoryStorage {
  /// Selects a directory, returning null when the user cancels.
  Future<PersistentDirectory?> select();

  /// Verifies that the original directory still exists and permits read/write access.
  Future<void> check(PersistentDirectory directory);

  /// Captures and verifies a physical SQLite snapshot without transferring chat records to Flutter.
  Future<ChatBackupSnapshot?> create(
    PersistentDirectory directory,
    String name,
  );

  /// Lists direct regular filenames without including subdirectories.
  Future<List<String>> list(PersistentDirectory directory);

  /// Checks SQLite integrity and returns only the snapshot's lightweight metadata.
  Future<ChatBackupSnapshot> inspect(
    PersistentDirectory directory,
    String name,
  );

  /// Restores chat tables atomically from one verified physical database snapshot.
  Future<ChatBackupSnapshot> restore(
    PersistentDirectory directory,
    String name,
  );

  /// Removes one direct child file owned by the backup service.
  Future<void> remove(PersistentDirectory directory, String name);
}
