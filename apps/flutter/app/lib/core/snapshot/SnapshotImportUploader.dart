// ignore_for_file: file_names

import 'dart:async';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart' as core_proxy;

const int _snapshotImportChunkSize = 64 * 1024;
const MethodChannel _snapshotImportInputChannel = MethodChannel(
  'operit/snapshot_import_input',
);

/// Holds one platform-owned selected file that can be read in bounded chunks.
class SnapshotImportFile {
  SnapshotImportFile._native({
    required this.token,
    required this.name,
    required this.byteLength,
  }) : _streamInput = null;

  SnapshotImportFile._stream({
    required this.name,
    required this.byteLength,
    required Stream<Uint8List> stream,
  }) : token = null,
       _streamInput = StreamIterator<Uint8List>(stream);

  final String? token;
  final String name;
  final int byteLength;
  final StreamIterator<Uint8List>? _streamInput;
  Uint8List? _pendingStreamChunk;

  /// Reports whether this Flutter host owns a native document-stream channel.
  static bool get _usesNativeDocumentStream {
    return !kIsWeb &&
        (defaultTargetPlatform == TargetPlatform.android ||
            defaultTargetPlatform == TargetPlatform.iOS);
  }

  /// Opens the platform document picker and returns a bounded-read file handle.
  static Future<SnapshotImportFile?> pick() async {
    if (_usesNativeDocumentStream) {
      return _pickNativeDocumentStream();
    }
    return _pickFileSelectorStream();
  }

  /// Opens Android or iOS system document input without materializing its full contents.
  static Future<SnapshotImportFile?> _pickNativeDocumentStream() async {
    final value = await _snapshotImportInputChannel
        .invokeMapMethod<String, Object?>('pick');
    if (value == null) {
      return null;
    }
    final token = value['token'];
    final name = value['name'];
    final byteLength = value['byteLength'];
    if (token is! String || name is! String || byteLength is! int) {
      throw StateError('Snapshot input channel returned invalid metadata');
    }
    return SnapshotImportFile._native(
      token: token,
      name: name,
      byteLength: byteLength,
    );
  }

  /// Opens a browser or desktop file-selector stream for bounded archive reads.
  static Future<SnapshotImportFile?> _pickFileSelectorStream() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(
          label: 'Operit snapshot',
          extensions: <String>['opsnapshot', 'zip'],
        ),
      ],
    );
    if (file == null) {
      return null;
    }
    return SnapshotImportFile._stream(
      name: file.name,
      byteLength: await file.length(),
      stream: file.openRead(),
    );
  }

  /// Reads one bounded chunk from the platform-owned input stream.
  Future<Uint8List> readChunk() async {
    if (token == null) {
      return _readStreamChunk();
    }
    final bytes = await _snapshotImportInputChannel.invokeMethod<Uint8List>(
      'readChunk',
      <String, Object?>{'token': token!, 'maxBytes': _snapshotImportChunkSize},
    );
    if (bytes == null) {
      throw StateError('Snapshot input channel returned no chunk');
    }
    return bytes;
  }

  /// Closes this platform-owned input stream once uploading has finished.
  Future<void> close() {
    if (token == null) {
      return _streamInput!.cancel();
    }
    return _snapshotImportInputChannel.invokeMethod<void>(
      'close',
      <String, Object?>{'token': token!},
    );
  }

  /// Emits bounded file chunks until the selected input reaches end of stream.
  Stream<Uint8List> chunks() async* {
    while (true) {
      final chunk = await readChunk();
      if (chunk.isEmpty) {
        return;
      }
      yield chunk;
    }
  }

  /// Splits browser or desktop file-stream events into bounded upload chunks.
  Future<Uint8List> _readStreamChunk() async {
    final pending = _pendingStreamChunk;
    if (pending != null) {
      return _takeStreamChunk(pending);
    }
    final input = _streamInput!;
    final hasNext = await input.moveNext();
    if (!hasNext) {
      return Uint8List(0);
    }
    return _takeStreamChunk(input.current);
  }

  /// Takes one bounded upload chunk and retains any remaining input-stream bytes.
  Uint8List _takeStreamChunk(Uint8List bytes) {
    if (bytes.length <= _snapshotImportChunkSize) {
      _pendingStreamChunk = null;
      return bytes;
    }
    _pendingStreamChunk = Uint8List.sublistView(bytes, _snapshotImportChunkSize);
    return Uint8List.sublistView(bytes, 0, _snapshotImportChunkSize);
  }
}

/// Refers to one fully uploaded archive used by snapshot import operations.
class SnapshotImportSession {
  const SnapshotImportSession({
    required this.clients,
    required this.archive,
  });

  final GeneratedCoreProxyClients clients;
  final core_proxy.StagedArchive archive;

  /// Returns the persisted byte length of the staged archive.
  int get byteLength => archive.byteLength;

  /// Reads raw runtime snapshot metadata from the staged archive.
  Future<core_proxy.RawSnapshotManifest> completeRaw() {
    return clients.servicesSnapshotImportManager.inspectRawSnapshot(
      archive: archive,
    );
  }

  /// Restores this uploaded raw runtime snapshot into the selected Runtime.
  Future<void> commitRaw() {
    return clients.servicesSnapshotImportManager.restoreRawSnapshot(
      archive: archive,
    );
  }

  /// Reads Operit1 snapshot metadata from the staged archive.
  Future<core_proxy.Operit1SnapshotPreview> completeOperit1() {
    return clients.servicesSnapshotImportManager.inspectOperit1Snapshot(
      archive: archive,
    );
  }

  /// Imports this uploaded Operit1 snapshot into the selected Runtime.
  Future<core_proxy.Operit1SnapshotImportResult> commitOperit1() {
    return clients.servicesSnapshotImportManager.importOperit1Snapshot(
      archive: archive,
    );
  }

  /// Removes this staged archive when no further consumer needs it.
  Future<void> discard() {
    return clients.servicesArchiveTransferManager.discardArchiveUpload(
      archiveId: archive.archiveId,
    );
  }
}

/// Uploads bounded platform file chunks through the generated archive reverse stream.
class SnapshotImportUploader {
  const SnapshotImportUploader(this.clients);

  final GeneratedCoreProxyClients clients;

  /// Creates a staged archive and uploads exactly the selected file's declared byte length.
  Future<SnapshotImportSession> stage(SnapshotImportFile file) async {
    final archiveId = await clients.servicesArchiveTransferManager
        .beginArchiveUpload(expectedByteLength: file.byteLength);
    try {
      await clients.servicesArchiveTransferManager.writeArchiveUpload(
        archiveId: archiveId,
        bytes: file.chunks(),
      );
      final archive = await clients.servicesArchiveTransferManager
          .completeArchiveUpload(
            archiveId: archiveId,
            expectedByteLength: file.byteLength,
          );
      return SnapshotImportSession(
        clients: clients,
        archive: archive,
      );
    } catch (error, stackTrace) {
      await clients.servicesArchiveTransferManager.discardArchiveUpload(
        archiveId: archiveId,
      );
      Error.throwWithStackTrace(error, stackTrace);
    } finally {
      await file.close();
    }
  }
}
