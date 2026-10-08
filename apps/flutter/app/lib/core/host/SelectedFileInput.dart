// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

const selectedFileChunkBytes = 1024 * 1024;
const _fileInputChannel = MethodChannel('operit/file_input');
const _snapshotInputChannel = MethodChannel('operit/snapshot_import_input');

/// Owns a selected file's metadata and bounded reads, never its complete contents.
class SelectedFileInput {
  SelectedFileInput.fromStream({
    required this.name,
    required this.byteLength,
    required Stream<Uint8List> stream,
    this.mimeType,
  }) : token = null,
       _input = StreamIterator<Uint8List>(stream),
       _nativeChannel = null;

  SelectedFileInput._native({
    required this.token,
    required this.name,
    required this.byteLength,
    this.mimeType,
    MethodChannel channel = _fileInputChannel,
  }) : _input = null,
       _nativeChannel = channel;

  final String name;
  final int? byteLength;
  final String? mimeType;
  final String? token;
  final StreamIterator<Uint8List>? _input;
  final MethodChannel? _nativeChannel;
  Uint8List? _pending;
  bool _closed = false;

  /// Uses the Android photo/document picker without loading file contents,
  /// or the platform's file-backed XFile stream.
  static Future<List<SelectedFileInput>> pick({bool imagesOnly = false}) async {
    if (!kIsWeb && defaultTargetPlatform == TargetPlatform.android) {
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
            ),
          );
        }
        return files;
      } catch (_) {
        // Release even tokens whose descriptors could not be decoded.
        for (final value in values ?? <Object?>[]) {
          if (value is Map && value['token'] is String) {
            try {
              await _fileInputChannel.invokeMethod<void>(
                'close',
                <String, Object?>{'token': value['token']},
              );
            } catch (_) {
              /* Keep the metadata error. */
            }
          }
        }
        rethrow;
      }
    }
    final files = await openFiles(
      acceptedTypeGroups: imagesOnly
          ? const <XTypeGroup>[
              XTypeGroup(
                label: 'image',
                extensions: <String>[
                  'jpg',
                  'jpeg',
                  'png',
                  'webp',
                  'bmp',
                  'gif',
                  'heic',
                ],
                mimeTypes: <String>['image/*'],
              ),
            ]
          : const <XTypeGroup>[],
    );
    final inputs = <SelectedFileInput>[];
    try {
      for (final file in files) {
        inputs.add(await fromXFile(file));
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

  /// Snapshot selection shares the same bounded input owner, but requires a known size.
  static Future<SelectedFileInput?> pickSnapshot() async {
    if (!kIsWeb &&
        (defaultTargetPlatform == TargetPlatform.android ||
            defaultTargetPlatform == TargetPlatform.iOS)) {
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
            /* Preserve the metadata error. */
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
    final file = await openFile(
      acceptedTypeGroups: const [
        XTypeGroup(label: 'Operit snapshot', extensions: ['opsnapshot', 'zip']),
      ],
    );
    return file == null ? null : fromXFile(file);
  }

  static Future<SelectedFileInput> fromXFile(
    XFile file, {
    String? name,
    String? mimeType,
  }) async {
    final length = await file.length();
    return SelectedFileInput.fromStream(
      name: name ?? file.name,
      byteLength: length,
      mimeType: mimeType ?? file.mimeType,
      stream: _readXFile(file, length),
    );
  }

  // Bound reads at the source, not only after an XFile has allocated an event.
  // In particular, cross_file's Web openRead() materializes its entire range.
  static Stream<Uint8List> _readXFile(XFile file, int length) async* {
    for (var offset = 0; offset < length;) {
      final end = math.min(offset + selectedFileChunkBytes, length);
      var read = 0;
      await for (final bytes in file.openRead(offset, end)) {
        read += bytes.length;
        if (bytes.length > selectedFileChunkBytes || read > end - offset) {
          throw StateError('Selected file returned an oversized range');
        }
        yield bytes;
      }
      if (read != end - offset) {
        throw StateError('Selected file ended before its declared length');
      }
      offset = end;
    }
  }

  /// Coalesces small source events and slices large ones into at most one MiB.
  Future<Uint8List> readChunk() async {
    if (_closed) throw StateError('Selected file input is closed');
    if (token != null) {
      final bytes = await _nativeChannel!.invokeMethod<Uint8List>(
        'readChunk',
        <String, Object?>{'token': token, 'maxBytes': selectedFileChunkBytes},
      );
      if (bytes == null || bytes.length > selectedFileChunkBytes) {
        throw StateError('Document input returned an invalid chunk');
      }
      return bytes;
    }
    final builder = BytesBuilder(copy: false);
    while (builder.length < selectedFileChunkBytes) {
      final pending = _pending;
      if (pending != null) {
        builder.add(_take(pending, selectedFileChunkBytes - builder.length));
      } else {
        if (!await _input!.moveNext()) break;
        builder.add(
          _take(_input.current, selectedFileChunkBytes - builder.length),
        );
      }
    }
    return builder.takeBytes();
  }

  Uint8List _take(Uint8List bytes, int limit) {
    if (bytes.length <= limit) {
      _pending = null;
      return bytes;
    }
    _pending = Uint8List.sublistView(bytes, limit);
    return Uint8List.sublistView(bytes, 0, limit);
  }

  Stream<Uint8List> chunks() async* {
    while (true) {
      final bytes = await readChunk();
      if (bytes.isEmpty) return;
      yield bytes;
    }
  }

  /// Closes native handles or cancels the source subscription, including unused selections.
  Future<void> close() async {
    if (_closed) return;
    _closed = true;
    _pending = null;
    if (token != null) {
      await _nativeChannel!.invokeMethod<void>('close', <String, Object?>{
        'token': token,
      });
    } else {
      await _input!.cancel();
    }
  }
}
