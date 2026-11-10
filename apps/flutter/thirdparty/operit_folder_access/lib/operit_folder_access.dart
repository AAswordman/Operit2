import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:file_selector/file_selector.dart' as selector;
import 'package:flutter/services.dart';
import 'package:file_selector_platform_interface/file_selector_platform_interface.dart'
    show FileSelectorPlatform;

part 'src/selected_file_input.dart';
part 'src/incoming_file_input.dart';

/// A platform-selected mount source, not an assumed physical file path.
class FolderMountSource {
  const FolderMountSource({
    required this.backend,
    required this.root,
    required String namespace,
    required this.name,
  }) : _namespace = namespace;

  final String backend;
  final String root;
  final String _namespace;
  String get namespace => _namespace == '/mnt' || _namespace.startsWith('/mnt/')
      ? '/virtual${_namespace.substring(4)}'
      : _namespace;
  final String name;

  factory FolderMountSource.native(String path) {
    final normalized = path
        .replaceAll(r'\', '/')
        .replaceAll(RegExp(r'/+$'), '');
    final label = normalized.split('/').last;
    return FolderMountSource(
      backend: 'native',
      root: path,
      namespace: '/virtual/local/folders',
      name: label.isEmpty ? path : label,
    );
  }

  factory FolderMountSource.fromMap(Map<String, Object?> map) =>
      FolderMountSource(
        backend: map['backend'] as String,
        root: map['root'] as String,
        namespace: map['namespace'] as String,
        name: map['name'] as String,
      );

  /// The binding API accepts this descriptor and returns a persistent VFS mount.
  String encode() =>
      'operit-mount:${jsonEncode(<String, String>{'backend': backend, 'root': root, 'namespace': namespace, 'name': name})}';
}

/// Unified folder selection. Platform registration, not business code, chooses
/// the implementation. Storage-root selection returns native paths; workspace
/// selection may return a persistently authorized document-tree URI.
abstract final class OperitFolderAccess {
  /// Returns an empty list when the user cancels. Platform limitations and
  /// permission errors are propagated to the caller.
  static Future<List<String>> pickDirectories({String? initialDirectory}) =>
      OperitFolderAccessPlatform.instance.pickDirectories(
        initialDirectory: initialDirectory,
      );

  /// Selects a mount source. Android returns an authorized SAF tree URI;
  /// other platforms retain their native/security-scoped directory selection.
  static Future<FolderMountSource?> pickWorkspaceDirectory({
    String? initialDirectory,
  }) => OperitFolderAccessPlatform.instance.pickWorkspaceDirectory(
    initialDirectory: initialDirectory,
  );

  /// Selects authorized folder sources through the registered platform adapter.
  static Future<List<FolderMountSource>> pickWorkspaceDirectories({
    String? initialDirectory,
  }) => OperitFolderAccessPlatform.instance.pickWorkspaceDirectories(
    initialDirectory: initialDirectory,
  );

  static Future<String?> pickDirectory({String? initialDirectory}) =>
      OperitFolderAccessPlatform.instance.pickDirectory(
        initialDirectory: initialDirectory,
      );
}

/// Defaults to file_selector and its registered native/web platform delegates.
abstract class OperitFolderAccessPlatform {
  static OperitFolderAccessPlatform instance = _FileSelectorFolderAccess();

  /// Registers external file delivery; platforms without an inbox have no events.
  Future<void> receiveIncomingFiles({
    required void Function(List<SelectedFileInput>) onFiles,
    required void Function(Object, StackTrace) onError,
  }) async {}

  /// Metadata and bounded input ownership use the same registered adapter as
  /// folder selection. App code does not choose a platform or interpret paths.
  Future<List<SelectedFileInput>> pickFileInputs({
    bool imagesOnly = false,
  }) async {
    final files = await selector.openFiles(
      acceptedTypeGroups: imagesOnly
          ? const [
              selector.XTypeGroup(
                label: 'image',
                extensions: [
                  'jpg',
                  'jpeg',
                  'png',
                  'webp',
                  'bmp',
                  'gif',
                  'heic',
                  'heif',
                ],
                mimeTypes: ['image/*'],
                uniformTypeIdentifiers: ['public.image'],
                webWildCards: ['image/*'],
              ),
            ]
          : const [],
    );
    final inputs = <SelectedFileInput>[];
    try {
      for (final file in files) {
        inputs.add(await fileInput(file));
      }
      return inputs;
    } catch (_) {
      for (final input in inputs) {
        try {
          await input.close();
        } catch (_) {
          /* Keep the selection error. */
        }
      }
      rethrow;
    }
  }

  Future<SelectedFileInput?> pickSnapshotInput({
    required List<selector.XTypeGroup> acceptedTypeGroups,
  }) async {
    final delegate = FileSelectorPlatform.instance;
    final file = await delegate.openFileWithProgress(
      acceptedTypeGroups: acceptedTypeGroups,
      onProgress: (_, _) {},
    );
    if (file == null) return null;
    try {
      final input = await fileInput(file);
      input._release = () => delegate.releaseFile(file);
      return input;
    } catch (_) {
      await delegate.releaseFile(file);
      rethrow;
    }
  }

  /// A source is optional: opaque picker URIs and byte-backed files must remain
  /// streams unless their owning adapter can provide persistent authorization.
  Future<FolderMountSource?> fileSource(
    selector.XFile file, {
    String? name,
  }) async => null;

  Future<SelectedFileInput> fileInput(
    selector.XFile file, {
    String? name,
    String? mimeType,
  }) async {
    final length = await file.length();
    final source = await fileSource(file, name: name);
    return SelectedFileInput.fromStream(
      name: name ?? file.name,
      byteLength: length,
      mimeType: mimeType ?? file.mimeType,
      sourceJson: source == null
          ? null
          : jsonEncode({
              'namespace': source.namespace,
              'backend': source.backend,
              'root': source.root,
              'name': source.name,
            }),
      stream: SelectedFileInput._readXFile(file, length),
    );
  }

  Future<String?> pickDirectory({String? initialDirectory});

  Future<FolderMountSource?> pickWorkspaceDirectory({
    String? initialDirectory,
  }) async {
    final path = await pickDirectory(initialDirectory: initialDirectory);
    return path == null ? null : FolderMountSource.native(path);
  }

  /// Converts native directory selections into explicit mount sources.
  Future<List<FolderMountSource>> pickWorkspaceDirectories({
    String? initialDirectory,
  }) async => (await pickDirectories(
    initialDirectory: initialDirectory,
  )).map(FolderMountSource.native).toList(growable: false);

  Future<List<String>> pickDirectories({String? initialDirectory});
}

class _FileSelectorFolderAccess extends OperitFolderAccessPlatform {
  @override
  Future<FolderMountSource?> fileSource(
    selector.XFile file, {
    String? name,
  }) async {
    // File selectors also return opaque URIs and in-memory blobs. Only native
    // absolute paths belong to the native backend; never treat a token as one.
    final path = file.path;
    if (!path.startsWith('/') && !RegExp(r'^[a-zA-Z]:[\\/]').hasMatch(path)) {
      return null;
    }
    return FolderMountSource(
      backend: 'native',
      root: path,
      namespace: '/virtual/local/attachments',
      name: name ?? file.name,
    );
  }

  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) async =>
      (await selector.getDirectoryPaths(
        initialDirectory: initialDirectory,
      )).whereType<String>().toList(growable: false);

  @override
  Future<String?> pickDirectory({String? initialDirectory}) =>
      selector.getDirectoryPath(initialDirectory: initialDirectory);
}

/// Installed by Flutter's generated macOS plugin registrant. No runtime
/// platform checks or distribution policy live in Dart.
class OperitFolderAccessMacOS extends _FileSelectorFolderAccess {
  static const MethodChannel _channel = MethodChannel('operit/folder_access');

  @override
  Future<FolderMountSource?> fileSource(
    selector.XFile file, {
    String? name,
  }) async {
    final source = await super.fileSource(file, name: name);
    if (source != null) {
      await _channel.invokeMethod<void>('retainFile', {'path': source.root});
    }
    return source;
  }

  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) async =>
      await _channel.invokeListMethod<String>(
        'pickDirectories',
        <String, Object?>{'initialDirectory': initialDirectory},
      ) ??
      const <String>[];

  static void registerWith() {
    OperitFolderAccessPlatform.instance = OperitFolderAccessMacOS();
  }

  @override
  Future<String?> pickDirectory({String? initialDirectory}) =>
      _channel.invokeMethod<String>('pickDirectory', <String, Object?>{
        'initialDirectory': initialDirectory,
      });
}

/// Android-only registration; ordinary storage roots still use file_selector.
class OperitFolderAccessAndroid extends _FileSelectorFolderAccess
    with _IncomingFileAccess {
  static const MethodChannel _channel = MethodChannel('operit/folder_access');

  static void registerWith() {
    OperitFolderAccessPlatform.instance = OperitFolderAccessAndroid();
  }

  @override
  Future<List<SelectedFileInput>> pickFileInputs({
    bool imagesOnly = false,
  }) async {
    final values = await _fileInputChannel.invokeListMethod<Object?>(
      imagesOnly ? 'pickImages' : 'pickFiles',
      imagesOnly ? null : const <String, Object?>{'mimeTypes': <String>[]},
    );
    final files = <SelectedFileInput>[];
    try {
      for (final value in values ?? <Object?>[]) {
        final info = Map<String, Object?>.from(value! as Map);
        final token = info['token'];
        final name = info['name'];
        final length = info['byteLength'];
        if (token is! String ||
            name is! String ||
            (length != null && (length is! int || length < 0))) {
          throw StateError('Document input returned invalid file metadata');
        }
        files.add(
          SelectedFileInput._native(
            token: token,
            name: name,
            byteLength: length as int?,
            mimeType: info['mimeType'] as String?,
            sourceJson: info['sourceJson'] as String?,
          ),
        );
      }
      return files;
    } catch (_) {
      for (final value in values ?? <Object?>[]) {
        if (value is Map && value['token'] is String) {
          try {
            await _fileInputChannel.invokeMethod<void>('close', {
              'token': value['token'],
            });
          } catch (_) {
            /* Keep the metadata error. */
          }
        }
      }
      rethrow;
    }
  }

  @override
  Future<SelectedFileInput?> pickSnapshotInput({
    required List<selector.XTypeGroup> acceptedTypeGroups,
  }) async {
    final info = await _snapshotInputChannel.invokeMapMethod<String, Object?>(
      'pick',
    );
    if (info == null) return null;
    final token = info['token'];
    final name = info['name'];
    final length = info['byteLength'];
    if (token is! String || name is! String || length is! int || length < 0) {
      if (token is String) {
        try {
          await _snapshotInputChannel.invokeMethod<void>('close', {
            'token': token,
          });
        } catch (_) {
          /* Keep the metadata error. */
        }
      }
      throw StateError('Snapshot input returned invalid metadata');
    }
    return SelectedFileInput._native(
      token: token,
      name: name,
      byteLength: length,
      channel: _snapshotInputChannel,
    );
  }

  // A generic XFile on Android carries no persisted SAF grant. Only the native
  // picker above may return a document source descriptor after retaining access.
  @override
  Future<FolderMountSource?> fileSource(
    selector.XFile file, {
    String? name,
  }) async => null;

  /// Retains the SAF authorization returned by Android's single-tree selector.
  @override
  Future<List<FolderMountSource>> pickWorkspaceDirectories({
    String? initialDirectory,
  }) async {
    final source = await pickWorkspaceDirectory(
      initialDirectory: initialDirectory,
    );
    return source == null ? const <FolderMountSource>[] : [source];
  }

  @override
  Future<FolderMountSource?> pickWorkspaceDirectory({
    String? initialDirectory,
  }) async {
    final selection = await _channel.invokeMapMethod<String, Object?>(
      'pickWorkspaceDirectory',
      <String, Object?>{'initialDirectory': initialDirectory},
    );
    return selection == null ? null : FolderMountSource.fromMap(selection);
  }
}

/// Registers iOS file delivery while retaining the system file-selector adapter.
class OperitFolderAccessIOS extends _FileSelectorFolderAccess
    with _IncomingFileAccess {
  /// Installs the adapter selected by Flutter's generated plugin registrant.
  static void registerWith() {
    OperitFolderAccessPlatform.instance = OperitFolderAccessIOS();
  }
}
