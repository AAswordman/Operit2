// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/foundation.dart';
import 'package:mime/mime.dart';
import 'package:operit_folder_access/operit_folder_access.dart';

import 'ComposeDslFilePickerStorage.dart';
import 'SelectedFileInput.dart';

/// Opens Compose DSL file-picker requests through Flutter's platform selector.
class ComposeDslFilePickerService {
  /// Prevents instantiation of the host picker service.
  ComposeDslFilePickerService._();

  /// Opens one picker request serialized by the Rust Compose DSL host boundary.
  static Future<String> open(
    String requestJson, {
    ComposeDslFilePickerStorage storage =
        const RuntimeComposeDslFilePickerStorage(),
  }) async {
    final request = _ComposeDslFilePickerRequest.fromJson(requestJson);
    if (request.picker == _ComposeDslFilePickerMode.directory) {
      final sources = request.allowMultiple
          ? await OperitFolderAccess.pickWorkspaceDirectories()
          : await _openSingleDirectory();
      final picked = <_PickedFile>[];
      for (final source in sources) {
        final path = await storage.mountDirectory(source);
        picked.add(_PickedFile.fromDirectory(path, source.name));
      }
      return _encodeResult(cancelled: sources.isEmpty, files: picked);
    }
    final typeGroups = _typeGroupsFor(request);
    final files = request.allowMultiple
        ? await openFiles(acceptedTypeGroups: typeGroups)
        : await _openSingleFile(typeGroups);
    final picked = <_PickedFile>[];
    try {
      for (final file in files) {
        picked.add(
          await _PickedFile.fromXFile(file, request.maxContentBytes, storage),
        );
      }
      return _encodeResult(cancelled: files.isEmpty, files: picked);
    } catch (_) {
      for (final file in picked) {
        await _discardFile(storage, file.path);
      }
      rethrow;
    }
  }

  /// Removes an unpublished selection while reporting cleanup errors to the host log.
  static Future<void> _discardFile(
    ComposeDslFilePickerStorage storage,
    String path,
  ) async {
    try {
      await storage.deleteFile(path);
    } catch (error) {
      debugPrint('Failed to discard selected file: $error');
    }
  }

  /// Builds the platform-neutral file-selector filter for one validated picker mode.
  static List<XTypeGroup> _typeGroupsFor(_ComposeDslFilePickerRequest request) {
    if (request.mimeTypes.isNotEmpty) {
      return request.mimeTypes
          .map(_typeGroupForMimeType)
          .toList(growable: false);
    }
    return switch (request.picker) {
      _ComposeDslFilePickerMode.document => const <XTypeGroup>[],
      _ComposeDslFilePickerMode.photo => <XTypeGroup>[_photoTypeGroup],
      _ComposeDslFilePickerMode.video => <XTypeGroup>[_videoTypeGroup],
      _ComposeDslFilePickerMode.media => <XTypeGroup>[_mediaTypeGroup],
      _ComposeDslFilePickerMode.directory => throw StateError(
        'Directories do not use file filters',
      ),
    };
  }

  /// Supplies equivalent MIME, extension and Apple type filters to every adapter.
  static XTypeGroup _typeGroupForMimeType(String mimeType) {
    switch (mimeType) {
      case 'image/*':
        return _photoTypeGroup;
      case 'video/*':
        return _videoTypeGroup;
      case 'audio/*':
        return const XTypeGroup(
          label: 'Audio',
          extensions: [
            'mp3',
            'wav',
            'ogg',
            'opus',
            'flac',
            'aac',
            'm4a',
            'aiff',
          ],
          mimeTypes: ['audio/*'],
          uniformTypeIdentifiers: ['public.audio'],
          webWildCards: ['audio/*'],
        );
      case '*/*':
        return const XTypeGroup(label: 'Files');
    }
    final extension = extensionFromMime(mimeType);
    if (extension == null) {
      throw UnsupportedError('Unsupported file-picker MIME type: $mimeType');
    }
    return XTypeGroup(
      label: mimeType,
      extensions: [extension],
      mimeTypes: [mimeType],
      webWildCards: [mimeType],
    );
  }

  /// Opens one directory and represents cancellation as an empty selection.
  static Future<List<FolderMountSource>> _openSingleDirectory() async {
    final source = await OperitFolderAccess.pickWorkspaceDirectory();
    return source == null ? const <FolderMountSource>[] : [source];
  }

  /// Opens the single-file selector and normalizes a cancellation to an empty list.
  static Future<List<XFile>> _openSingleFile(
    List<XTypeGroup> typeGroups,
  ) async {
    final file = await openFile(acceptedTypeGroups: typeGroups);
    return file == null ? const <XFile>[] : <XFile>[file];
  }

  /// Serializes the host-independent picker result returned to Compose DSL JavaScript.
  static String _encodeResult({
    required bool cancelled,
    required List<_PickedFile> files,
  }) {
    return jsonEncode(<String, Object?>{
      'cancelled': cancelled,
      'files': files.map((file) => file.toJson()).toList(growable: false),
    });
  }
}

/// Enumerates the cross-platform picker modes accepted by the Rust host boundary.
enum _ComposeDslFilePickerMode { document, photo, video, media, directory }

const XTypeGroup _photoTypeGroup = XTypeGroup(
  label: 'Photos',
  extensions: <String>[
    'jpg',
    'jpeg',
    'png',
    'gif',
    'webp',
    'heic',
    'heif',
    'bmp',
    'tif',
    'tiff',
  ],
  mimeTypes: <String>['image/*'],
  uniformTypeIdentifiers: <String>['public.image'],
  webWildCards: <String>['image/*'],
);

const XTypeGroup _videoTypeGroup = XTypeGroup(
  label: 'Videos',
  extensions: <String>['mp4', 'mov', 'm4v', 'avi', 'mkv', 'webm', '3gp'],
  mimeTypes: <String>['video/*'],
  uniformTypeIdentifiers: <String>['public.movie'],
  webWildCards: <String>['video/*'],
);

const XTypeGroup _mediaTypeGroup = XTypeGroup(
  label: 'Photos and videos',
  extensions: <String>[
    'jpg',
    'jpeg',
    'png',
    'gif',
    'webp',
    'heic',
    'heif',
    'bmp',
    'tif',
    'tiff',
    'mp4',
    'mov',
    'm4v',
    'avi',
    'mkv',
    'webm',
    '3gp',
  ],
  mimeTypes: <String>['image/*', 'video/*'],
  uniformTypeIdentifiers: <String>['public.image', 'public.movie'],
  webWildCards: <String>['image/*', 'video/*'],
);

/// Stores the fully validated, platform-neutral picker request.
class _ComposeDslFilePickerRequest {
  /// Stores the validated host request fields.
  const _ComposeDslFilePickerRequest({
    required this.picker,
    required this.allowMultiple,
    required this.mimeTypes,
    required this.maxContentBytes,
  });

  final _ComposeDslFilePickerMode picker;
  final bool allowMultiple;
  final List<String> mimeTypes;
  final int? maxContentBytes;

  /// Decodes the request generated by the Rust Compose DSL host API.
  factory _ComposeDslFilePickerRequest.fromJson(String rawJson) {
    final decoded = jsonDecode(rawJson);
    if (decoded is! Map<String, dynamic>) {
      throw const FormatException(
        'Compose DSL file picker request must be an object',
      );
    }
    final pickerToken = decoded['picker'];
    if (pickerToken is! String) {
      throw const FormatException(
        'Compose DSL file picker picker must be a string',
      );
    }
    final picker = switch (pickerToken) {
      'document' => _ComposeDslFilePickerMode.document,
      'photo' || 'image' => _ComposeDslFilePickerMode.photo,
      'video' => _ComposeDslFilePickerMode.video,
      'media' => _ComposeDslFilePickerMode.media,
      'directory' => _ComposeDslFilePickerMode.directory,
      _ => throw FormatException(
        'Unsupported Compose DSL file picker mode: $pickerToken',
      ),
    };
    final allowMultiple = decoded['allowMultiple'];
    if (allowMultiple is! bool) {
      throw const FormatException(
        'Compose DSL file picker allowMultiple is missing or invalid',
      );
    }
    final maxContentBytes = decoded['maxContentBytes'];
    if (maxContentBytes != null &&
        (maxContentBytes is! int || maxContentBytes <= 0)) {
      throw const FormatException('maxContentBytes must be a positive integer');
    }
    if (maxContentBytes != null &&
        picker == _ComposeDslFilePickerMode.directory) {
      throw const FormatException(
        'Directory selections do not have file contents',
      );
    }
    return _ComposeDslFilePickerRequest(
      picker: picker,
      maxContentBytes: maxContentBytes as int?,
      allowMultiple: allowMultiple,
      mimeTypes: (decoded['mimeTypes'] as List<dynamic>).cast<String>(),
    );
  }
}

/// Stores one selected item using the stable public Compose DSL result shape.
class _PickedFile {
  /// Stores a selected resource after publication through the VFS.
  const _PickedFile({
    required this.uri,
    required this.path,
    required this.name,
    required this.mimeType,
    required this.size,
    this.contentBase64,
  });

  final String uri;
  final String path;
  final String name;
  final String? mimeType;
  final int size;
  final String? contentBase64;

  /// Builds a directory result from its registered VFS mount.
  factory _PickedFile.fromDirectory(String path, String name) =>
      _PickedFile(uri: path, path: path, name: name, mimeType: null, size: 0);

  /// Publishes selected bytes into host storage before returning a public VFS path.
  static Future<_PickedFile> fromXFile(
    XFile file,
    int? maxContentBytes,
    ComposeDslFilePickerStorage storage,
  ) async {
    final input = await SelectedFileInput.fromXFile(file);
    String? path;
    try {
      final size = input.byteLength!;
      if (maxContentBytes != null && size > maxContentBytes) {
        throw StateError(
          'Selected file exceeds the $maxContentBytes byte limit',
        );
      }
      path = await storage.createFile(file.name);
      final content = maxContentBytes == null
          ? null
          : BytesBuilder(copy: false);

      /// Checks the source size while forwarding bounded chunks to the host writer.
      Stream<Uint8List> chunks() async* {
        var received = 0;
        await for (final chunk in input.chunks()) {
          received += chunk.length;
          if (received > size) {
            throw StateError('Selected file exceeds its declared byte length');
          }
          content?.add(chunk);
          yield chunk;
        }
        if (received != size) {
          throw StateError('Selected file byte length changed during import');
        }
      }

      await storage.writeFile(path, size, chunks());
      return _PickedFile(
        uri: path,
        path: path,
        name: file.name,
        mimeType: file.mimeType,
        size: size,
        contentBase64: content == null
            ? null
            : base64Encode(content.takeBytes()),
      );
    } catch (_) {
      if (path != null) {
        await ComposeDslFilePickerService._discardFile(storage, path);
      }
      rethrow;
    } finally {
      await input.close();
    }
  }

  /// Converts one selected item to the public Compose DSL JSON shape.
  Map<String, Object?> toJson() {
    return <String, Object?>{
      'uri': uri,
      'path': path,
      'name': name,
      'mimeType': mimeType,
      'size': size,
      if (contentBase64 != null) 'contentBase64': contentBase64,
    };
  }
}
