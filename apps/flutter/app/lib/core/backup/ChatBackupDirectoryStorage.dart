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

  /// Verifies directory listing and probe file writing, reading and deletion.
  Future<void> check(PersistentDirectory directory);

  /// Copies the complete database file to the selected folder.
  Future<ChatBackupSnapshot> create(PersistentDirectory directory, String name);

  /// Lists direct regular filenames without including subdirectories.
  Future<List<String>> list(PersistentDirectory directory);

  /// Copies the selected file over the application database.
  Future<ChatBackupSnapshot> restore(
    PersistentDirectory directory,
    String name,
  );

  /// Removes one direct child file owned by the backup service.
  Future<void> remove(PersistentDirectory directory, String name);
}
