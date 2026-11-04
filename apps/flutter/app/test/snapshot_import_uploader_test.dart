import 'dart:async';
import 'dart:typed_data';

import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/SelectedFileInput.dart';

const _uploadChunkBytes = 1024 * 1024;

/// Emits file bytes in small stream events to exercise portable upload coalescing.
Stream<Uint8List> _fileEvents(Uint8List bytes, int eventBytes) async* {
  for (var offset = 0; offset < bytes.length; offset += eventBytes) {
    final end = (offset + eventBytes).clamp(0, bytes.length);
    yield Uint8List.sublistView(bytes, offset, end);
  }
}

/// Emits a partial source chunk followed by an explicit file-read failure.
Stream<Uint8List> _failingFileEvents() async* {
  yield Uint8List.fromList(<int>[1, 2, 3]);
  throw StateError('source read failed');
}

/// Checks bounded upload chunking, content preservation, and source lifecycle errors.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final originalFileAccess = OperitFolderAccessPlatform.instance;
  tearDown(() => OperitFolderAccessPlatform.instance = originalFileAccess);

  test('coalesces 64 KiB file events into 1 MiB upload chunks', () async {
    final original = Uint8List.fromList(
      List<int>.generate(2 * _uploadChunkBytes + 17, (index) => index % 251),
    );
    final file = SelectedFileInput.fromStream(
      name: 'snapshot.zip',
      byteLength: original.length,
      stream: _fileEvents(original, 64 * 1024),
    );
    final chunks = await file.chunks().toList();
    expect(chunks.map((chunk) => chunk.length), <int>[
      _uploadChunkBytes,
      _uploadChunkBytes,
      17,
    ]);
    final bytes = BytesBuilder(copy: false);
    chunks.forEach(bytes.add);
    expect(bytes.takeBytes(), original);
    await file.close();
  });

  test('splits oversized stream events and preserves empty events', () async {
    final original = Uint8List.fromList(
      List<int>.generate(_uploadChunkBytes + 19, (index) => index % 251),
    );
    final file = SelectedFileInput.fromStream(
      name: 'snapshot.zip',
      byteLength: original.length,
      stream: Stream<Uint8List>.fromIterable(<Uint8List>[
        Uint8List(0),
        original,
        Uint8List(0),
      ]),
    );
    final chunks = await file.chunks().toList();
    expect(chunks.map((chunk) => chunk.length), <int>[_uploadChunkBytes, 19]);
    expect(chunks[0], Uint8List.sublistView(original, 0, _uploadChunkBytes));
    expect(chunks[1], Uint8List.sublistView(original, _uploadChunkBytes));
    await file.close();
  });

  test('an empty input emits no upload chunks', () async {
    final file = SelectedFileInput.fromStream(
      name: 'empty.zip',
      byteLength: 0,
      stream: const Stream<Uint8List>.empty(),
    );
    expect(await file.chunks().toList(), isEmpty);
    await file.close();
  });

  test(
    'a source read failure propagates without emitting a partial chunk',
    () async {
      final file = SelectedFileInput.fromStream(
        name: 'failed.zip',
        byteLength: 10,
        stream: _failingFileEvents(),
      );
      await expectLater(file.readChunk(), throwsStateError);
      await file.close();
    },
  );

  test('closing the file cancels its input stream', () async {
    var cancelled = false;
    final input = StreamController<Uint8List>(
      onCancel: () {
        cancelled = true;
      },
    );
    final file = SelectedFileInput.fromStream(
      name: 'snapshot.zip',
      byteLength: _uploadChunkBytes,
      stream: input.stream,
    );
    input.add(Uint8List(_uploadChunkBytes));
    await file.readChunk();
    await file.close();
    expect(cancelled, isTrue);
    await input.close();
  });

  test('the document channel requests bounded 1 MiB upload chunks', () async {
    const channel = MethodChannel('operit/snapshot_import_input');
    final methods = <String>[];
    OperitFolderAccessAndroid.registerWith();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          methods.add(call.method);
          switch (call.method) {
            case 'pick':
              return <String, Object>{
                'token': 'snapshot-input',
                'name': 'snapshot.zip',
                'byteLength': _uploadChunkBytes,
              };
            case 'readChunk':
              expect(call.arguments['maxBytes'], _uploadChunkBytes);
              return Uint8List(_uploadChunkBytes);
            case 'close':
              return null;
            default:
              throw StateError(
                'unexpected snapshot input method: ${call.method}',
              );
          }
        });
    try {
      final file = (await SelectedFileInput.pickSnapshot())!;
      expect((await file.readChunk()).length, _uploadChunkBytes);
      await file.close();
      expect(methods, <String>['pick', 'readChunk', 'close']);
    } finally {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null);
    }
  });
}
