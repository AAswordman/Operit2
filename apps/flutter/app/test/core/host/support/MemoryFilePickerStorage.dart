// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:operit2/core/host/ComposeDslFilePickerStorage.dart';

/// Models committed VFS resources at the runtime host boundary.
class MemoryFilePickerStorage implements ComposeDslFilePickerStorage {
  /// Creates an isolated host storage fixture for one picker test.
  MemoryFilePickerStorage();

  final files = <String, Uint8List>{};
  final allocated = <String>[];
  final deleted = <String>[];
  final directories = <FolderMountSource>[];
  final chunkSizes = <int>[];
  bool failWrite = false;

  /// Allocates one destination inside the production selection namespace.
  @override
  Future<String> createFile(String name) async {
    final path =
        '/app/data/temp/clean_on_exit/file_picker/${allocated.length}/$name';
    allocated.add(path);
    return path;
  }

  /// Publishes complete streams and records their source chunk sizes.
  @override
  Future<void> writeFile(String path, int size, Stream<Uint8List> bytes) async {
    if (failWrite) throw StateError('Selected file storage failed');
    final content = BytesBuilder(copy: false);
    await for (final chunk in bytes) {
      chunkSizes.add(chunk.length);
      expect(chunk.length, lessThanOrEqualTo(1024 * 1024));
      content.add(chunk);
    }
    expect(content.length, size);
    files[path] = content.takeBytes();
  }

  /// Records cleanup so failed multi-file requests cannot leave committed files.
  @override
  Future<void> deleteFile(String path) async {
    deleted.add(path);
    files.remove(path);
  }

  /// Retains the authorized backend descriptor behind a public mount path.
  @override
  Future<String> mountDirectory(FolderMountSource source) async {
    directories.add(source);
    return '${source.namespace}/selected-${directories.length}';
  }
}
