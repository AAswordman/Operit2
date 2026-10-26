// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:operit_folder_access/operit_folder_access.dart';

import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../proxy/generated/CoreProxyClients.g.dart';

/// Publishes selected resources through the runtime's host storage capabilities.
abstract interface class ComposeDslFilePickerStorage {
  /// Allocates one public VFS destination for a selected file.
  Future<String> createFile(String name);

  /// Publishes a bounded stream after checking its exact byte length.
  Future<void> writeFile(String path, int size, Stream<Uint8List> bytes);

  /// Removes a file whose picker operation failed before returning its result.
  Future<void> deleteFile(String path);

  /// Registers an authorized directory source as a VFS mount.
  Future<String> mountDirectory(FolderMountSource source);
}

/// Uses the same generated Core transport for every Flutter platform.
class RuntimeComposeDslFilePickerStorage
    implements ComposeDslFilePickerStorage {
  /// Binds storage calls to the runtime that owns the host interaction.
  const RuntimeComposeDslFilePickerStorage();

  /// Resolves the host-backed file selection service through the Core proxy.
  GeneratedServicesFileSelectionServiceCoreProxy get _service =>
      const GeneratedCoreProxyClients(
        ProxyCoreRuntimeBridge(),
      ).servicesFileSelectionService;

  /// Allocates a destination without exposing the source's physical locator.
  @override
  Future<String> createFile(String name) => _service.createFile(fileName: name);

  /// Streams file bytes into a private host writer before publishing the file.
  @override
  Future<void> writeFile(String path, int size, Stream<Uint8List> bytes) =>
      _service.writeFile(path: path, expectedByteLength: size, bytes: bytes);

  /// Removes a destination after a failed selection operation.
  @override
  Future<void> deleteFile(String path) => _service.deleteFile(path: path);

  /// Preserves the platform adapter's authorized directory backend and root.
  @override
  Future<String> mountDirectory(FolderMountSource source) =>
      _service.mountDirectory(
        sourceJson: jsonEncode({
          'namespace': source.namespace,
          'backend': source.backend,
          'root': source.root,
          'name': source.name,
        }),
      );
}
