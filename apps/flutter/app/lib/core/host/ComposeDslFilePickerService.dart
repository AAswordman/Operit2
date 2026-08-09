// ignore_for_file: file_names

import 'dart:convert';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Routes validated Compose DSL picker requests to the current platform owner.
class ComposeDslFilePickerService {
  ComposeDslFilePickerService._();

  static const MethodChannel _androidChannel = MethodChannel(
    'operit/compose_dsl_file_picker',
  );

  /// Opens one picker request serialized by the Rust Compose DSL host boundary.
  static Future<String> open(String requestJson) async {
    final request = _ComposeDslFilePickerRequest.fromJson(requestJson);
    if (kIsWeb) {
      throw UnsupportedError('Compose DSL file picking is unavailable on web');
    }
    return switch (defaultTargetPlatform) {
      TargetPlatform.android => _openAndroid(request),
      TargetPlatform.linux ||
      TargetPlatform.macOS ||
      TargetPlatform.windows => _openDesktop(request),
      TargetPlatform.iOS => throw UnsupportedError(
        'Compose DSL file picking is unavailable on iOS',
      ),
      TargetPlatform.fuchsia => throw UnsupportedError(
        'Compose DSL file picking is unavailable on Fuchsia',
      ),
    };
  }

  /// Sends one fully normalized request to the Android activity owner.
  static Future<String> _openAndroid(
    _ComposeDslFilePickerRequest request,
  ) async {
    final result = await _androidChannel.invokeMethod<String>(
      'open',
      request.toJson(),
    );
    if (result == null) {
      throw StateError('Android Compose DSL file picker returned no result');
    }
    return result;
  }

  /// Opens a document, visual-media, or directory picker provided by file_selector.
  static Future<String> _openDesktop(
    _ComposeDslFilePickerRequest request,
  ) async {
    switch (request.picker) {
      case _ComposeDslFilePickerMode.document:
        return _openDesktopFiles(request, request.mimeTypes);
      case _ComposeDslFilePickerMode.image:
        return _openDesktopFiles(request, const <String>['image/*']);
      case _ComposeDslFilePickerMode.video:
        return _openDesktopFiles(request, const <String>['video/*']);
      case _ComposeDslFilePickerMode.media:
        return _openDesktopFiles(
          request,
          const <String>['image/*', 'video/*'],
        );
      case _ComposeDslFilePickerMode.directory:
        return _openDesktopDirectory();
      case _ComposeDslFilePickerMode.camera:
        throw UnsupportedError('Compose DSL camera capture requires Android');
    }
  }

  /// Opens the desktop file-selector dialog and serializes its selected files.
  static Future<String> _openDesktopFiles(
    _ComposeDslFilePickerRequest request,
    List<String> mimeTypes,
  ) async {
    final groups = mimeTypes.length == 1 && mimeTypes.single == '*/*'
        ? const <XTypeGroup>[]
        : <XTypeGroup>[
            XTypeGroup(label: 'Files', mimeTypes: mimeTypes),
          ];
    final files = request.allowMultiple
        ? await openFiles(acceptedTypeGroups: groups)
        : await _openSingleDesktopFile(groups);
    if (files.isEmpty) {
      return _encodeResult(cancelled: true, files: const <_PickedFile>[]);
    }
    final picked = await Future.wait(
      files.map(_PickedFile.fromXFile),
    );
    return _encodeResult(cancelled: false, files: picked);
  }

  /// Opens the desktop single-file selector and normalizes its nullable result to a list.
  static Future<List<XFile>> _openSingleDesktopFile(
    List<XTypeGroup> groups,
  ) async {
    final file = await openFile(acceptedTypeGroups: groups);
    return file == null ? const <XFile>[] : <XFile>[file];
  }

  /// Opens the desktop directory selector and returns its URI-only result.
  static Future<String> _openDesktopDirectory() async {
    final directory = await getDirectoryPath();
    if (directory == null) {
      return _encodeResult(cancelled: true, files: const <_PickedFile>[]);
    }
    return _encodeResult(
      cancelled: false,
      files: <_PickedFile>[
        _PickedFile(uri: Uri.directory(directory).toString()),
      ],
    );
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

/// Enumerates the normalized picker modes sent by the Rust host boundary.
enum _ComposeDslFilePickerMode {
  document,
  image,
  video,
  media,
  directory,
  camera,
}

/// Stores the fully validated, platform-neutral picker request.
class _ComposeDslFilePickerRequest {
  const _ComposeDslFilePickerRequest({
    required this.picker,
    required this.mimeTypes,
    required this.allowMultiple,
    required this.persistPermission,
  });

  final _ComposeDslFilePickerMode picker;
  final List<String> mimeTypes;
  final bool allowMultiple;
  final bool persistPermission;

  /// Decodes the request generated by the Rust Compose DSL host API.
  factory _ComposeDslFilePickerRequest.fromJson(String rawJson) {
    final decoded = jsonDecode(rawJson);
    if (decoded is! Map<String, dynamic>) {
      throw const FormatException('Compose DSL file picker request must be an object');
    }
    final pickerToken = decoded['picker'];
    if (pickerToken is! String) {
      throw const FormatException('Compose DSL file picker picker must be a string');
    }
    final picker = switch (pickerToken) {
      'document' => _ComposeDslFilePickerMode.document,
      'image' => _ComposeDslFilePickerMode.image,
      'video' => _ComposeDslFilePickerMode.video,
      'media' => _ComposeDslFilePickerMode.media,
      'directory' => _ComposeDslFilePickerMode.directory,
      'camera' => _ComposeDslFilePickerMode.camera,
      _ => throw FormatException(
        'Unsupported Compose DSL file picker mode: $pickerToken',
      ),
    };
    final rawMimeTypes = decoded['mimeTypes'];
    if (rawMimeTypes is! List<Object?> ||
        rawMimeTypes.any((mimeType) => mimeType is! String)) {
      throw const FormatException(
        'Compose DSL file picker mimeTypes must be a string array',
      );
    }
    final allowMultiple = decoded['allowMultiple'];
    final persistPermission = decoded['persistPermission'];
    if (allowMultiple is! bool || persistPermission is! bool) {
      throw const FormatException(
        'Compose DSL file picker booleans are missing or invalid',
      );
    }
    return _ComposeDslFilePickerRequest(
      picker: picker,
      mimeTypes: rawMimeTypes.cast<String>(),
      allowMultiple: allowMultiple,
      persistPermission: persistPermission,
    );
  }

  /// Converts the request to the MethodChannel structure consumed by Android.
  Map<String, Object?> toJson() {
    return <String, Object?>{
      'picker': picker.name,
      'mimeTypes': mimeTypes,
      'allowMultiple': allowMultiple,
      'persistPermission': persistPermission,
    };
  }
}

/// Stores one selected item using the stable public Compose DSL result shape.
class _PickedFile {
  const _PickedFile({
    required this.uri,
    this.path,
    this.name,
    this.mimeType,
    this.size,
  });

  final String uri;
  final String? path;
  final String? name;
  final String? mimeType;
  final int? size;

  /// Builds one result entry from a desktop file_selector result.
  static Future<_PickedFile> fromXFile(XFile file) async {
    final path = file.path;
    return _PickedFile(
      uri: Uri.file(path).toString(),
      path: path,
      name: file.name,
      mimeType: file.mimeType,
      size: await file.length(),
    );
  }

  /// Converts one selected item to the public Compose DSL JSON shape.
  Map<String, Object?> toJson() {
    return <String, Object?>{
      'uri': uri,
      'path': path,
      'name': name,
      'mimeType': mimeType,
      'size': size,
    };
  }
}
