part of '../operit_folder_access.dart';

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
    this.sourceJson,
  }) : token = null,
       _input = StreamIterator<Uint8List>(stream),
       _nativeChannel = null;

  SelectedFileInput._native({
    required this.token,
    required this.name,
    required this.byteLength,
    this.mimeType,
    this.sourceJson,
    MethodChannel channel = _fileInputChannel,
  }) : _input = null,
       _nativeChannel = channel;

  final String name;
  final int? byteLength;
  final String? mimeType;
  final String? token;

  /// Persistently authorized source metadata; picker tokens never escape as locators.
  final String? sourceJson;
  final StreamIterator<Uint8List>? _input;
  final MethodChannel? _nativeChannel;
  Future<void> Function()? _release;
  Uint8List? _pending;
  bool _closed = false;

  /// Uses the existing registered file-access adapter.
  static Future<List<SelectedFileInput>> pick({bool imagesOnly = false}) =>
      OperitFolderAccessPlatform.instance.pickFileInputs(
        imagesOnly: imagesOnly,
      );

  static Future<SelectedFileInput?> pickSnapshot({
    List<selector.XTypeGroup> acceptedTypeGroups = const [
      selector.XTypeGroup(
        label: 'Operit snapshot',
        extensions: ['opsnapshot', 'zip'],
      ),
    ],
  }) => OperitFolderAccessPlatform.instance.pickSnapshotInput(
    acceptedTypeGroups: acceptedTypeGroups,
  );

  static Future<SelectedFileInput> fromXFile(
    selector.XFile file, {
    String? name,
    String? mimeType,
  }) => OperitFolderAccessPlatform.instance.fileInput(
    file,
    name: name,
    mimeType: mimeType,
  );

  // Bound reads at the source, not only after an XFile has allocated an event.
  // In particular, cross_file's Web openRead() materializes its entire range.
  static Stream<Uint8List> _readXFile(selector.XFile file, int length) async* {
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
      try {
        await _input!.cancel();
      } finally {
        final release = _release;
        _release = null;
        await release?.call();
      }
    }
  }
}
