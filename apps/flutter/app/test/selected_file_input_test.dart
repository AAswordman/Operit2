import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/SelectedFileInput.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'source is split into bounded chunks with exact binary contents',
    () async {
      final original = Uint8List(selectedFileChunkBytes * 2 + 19);
      for (var i = 0; i < original.length; i++) {
        original[i] = i % 251;
      }
      final file = SelectedFileInput.fromStream(
        name: '文档.apk',
        byteLength: original.length,
        stream: Stream<Uint8List>.fromIterable([
          Uint8List(0),
          original,
          Uint8List(0),
        ]),
      );
      final chunks = await file.chunks().toList();
      expect(chunks.map((chunk) => chunk.length), [
        selectedFileChunkBytes,
        selectedFileChunkBytes,
        19,
      ]);
      final actual = BytesBuilder(copy: false);
      chunks.forEach(actual.add);
      expect(actual.takeBytes(), original);
      await file.close();
    },
  );

  test('small events are coalesced and empty files remain empty', () async {
    final file = SelectedFileInput.fromStream(
      name: 'file',
      byteLength: 3,
      stream: Stream<Uint8List>.fromIterable([
        Uint8List.fromList([0]),
        Uint8List.fromList([1, 255]),
      ]),
    );
    expect(await file.readChunk(), [0, 1, 255]);
    expect(await file.readChunk(), isEmpty);
    await file.close();
    final empty = SelectedFileInput.fromStream(
      name: 'empty',
      byteLength: 0,
      stream: const Stream.empty(),
    );
    expect(await empty.chunks().toList(), isEmpty);
    await empty.close();
  });

  test('source failure propagates and closing cancels input once', () async {
    var cancelled = 0;
    final stream = StreamController<Uint8List>(
      onCancel: () {
        cancelled++;
      },
    );
    final file = SelectedFileInput.fromStream(
      name: 'broken',
      byteLength: null,
      stream: stream.stream,
    );
    stream.addError(StateError('source read failed'));
    await expectLater(file.readChunk(), throwsStateError);
    await file.close();
    await file.close();
    expect(cancelled, 1);
    await stream.close();
    await expectLater(file.readChunk(), throwsStateError);
  });

  test(
    'Android photo picker returns metadata only and reads each token lazily',
    () async {
      const channel = MethodChannel('operit/file_input');
      final calls = <MethodCall>[];
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            switch (call.method) {
              case 'pickImages':
                expect(call.arguments, isNull);
                return [
                  {
                    'token': 'first',
                    'name': '大图.png',
                    'byteLength': 143388128,
                    'mimeType': 'image/png',
                  },
                  {
                    'token': 'unused',
                    'name': 'unknown.png',
                    'byteLength': null,
                  },
                ];
              case 'readChunk':
                expect(call.arguments['maxBytes'], selectedFileChunkBytes);
                return Uint8List.fromList([0, 255]);
              case 'close':
                return null;
              default:
                throw StateError('Unexpected ${call.method}');
            }
          });
      try {
        final files = await SelectedFileInput.pick(imagesOnly: true);
        expect(calls.map((call) => call.method), ['pickImages']);
        expect(files.first.byteLength, 143388128);
        expect(files.last.byteLength, isNull);
        expect(await files.first.readChunk(), [0, 255]);
        for (final file in files) {
          await file.close();
          await file.close();
        }
        expect(calls.map((call) => call.method), [
          'pickImages',
          'readChunk',
          'close',
          'close',
        ]);
        expect(calls.last.arguments['token'], 'unused');
      } finally {
        debugDefaultTargetPlatformOverride = null;
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
            .setMockMethodCallHandler(channel, null);
      }
    },
  );

  test('Android ordinary files still use the document picker', () async {
    const channel = MethodChannel('operit/file_input');
    final calls = <MethodCall>[];
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          if (call.method == 'pickFiles') {
            expect(call.arguments['mimeTypes'], isEmpty);
            return [
              {
                'token': 'document',
                'name': 'notes.txt',
                'byteLength': 12,
                'mimeType': 'text/plain',
              },
            ];
          }
          if (call.method == 'close') return null;
          throw StateError('Unexpected ${call.method}');
        });
    try {
      final files = await SelectedFileInput.pick();
      expect(calls.map((call) => call.method), ['pickFiles']);
      expect(files.single.name, 'notes.txt');
      expect(files.single.mimeType, 'text/plain');
      await files.single.close();
      expect(calls.map((call) => call.method), ['pickFiles', 'close']);
    } finally {
      debugDefaultTargetPlatformOverride = null;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null);
    }
  });

  for (final imagesOnly in [true, false]) {
    final method = imagesOnly ? 'pickImages' : 'pickFiles';
    test('Android $method cancellation returns no attachments', () async {
      const channel = MethodChannel('operit/file_input');
      final calls = <MethodCall>[];
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            expect(call.method, method);
            return <Object?>[];
          });
      try {
        expect(await SelectedFileInput.pick(imagesOnly: imagesOnly), isEmpty);
        expect(calls, hasLength(1));
      } finally {
        debugDefaultTargetPlatformOverride = null;
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
            .setMockMethodCallHandler(channel, null);
      }
    });

    test('Android $method releases every token on invalid metadata', () async {
      const channel = MethodChannel('operit/file_input');
      final closedTokens = <String>[];
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            if (call.method == method) {
              return [
                {'token': 'valid', 'name': 'photo.png', 'byteLength': 12},
                {'token': 'invalid', 'name': 'broken.png', 'byteLength': -1},
                {'token': 'unvisited', 'name': 'later.png', 'byteLength': null},
              ];
            }
            if (call.method == 'close') {
              closedTokens.add(call.arguments['token'] as String);
              return null;
            }
            throw StateError('Unexpected ${call.method}');
          });
      try {
        await expectLater(
          SelectedFileInput.pick(imagesOnly: imagesOnly),
          throwsStateError,
        );
        expect(closedTokens, ['valid', 'invalid', 'unvisited']);
      } finally {
        debugDefaultTargetPlatformOverride = null;
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
            .setMockMethodCallHandler(channel, null);
      }
    });
  }
}
