// ignore_for_file: file_names

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';

/// Verifies Dart preserves MessagePack bin values as Uint8List.
void main() {
  test('native bytes use MessagePack bin', () {
    final encoded = encodeCoreLink(Uint8List.fromList(<int>[1, 2, 3, 4]));

    expect(encoded, Uint8List.fromList(<int>[0xc4, 4, 1, 2, 3, 4]));
    expect(decodeCoreLink(encoded), Uint8List.fromList(<int>[1, 2, 3, 4]));
  });

  test('uint64 is decoded without dart2js uint64 accessors', () {
    final decoded = decodeCoreLink(
      Uint8List.fromList(<int>[0xcf, 0, 0, 0, 1, 0, 0, 0, 0]),
    );

    expect(decoded, 0x100000000);
  });

  test('int64 is decoded without dart2js int64 accessors', () {
    final decoded = decodeCoreLink(
      Uint8List.fromList(<int>[
        0xd3,
        0xff,
        0xff,
        0xff,
        0xff,
        0x7f,
        0xff,
        0xff,
        0xff,
      ]),
    );

    expect(decoded, -2147483649);
  });

  test('large integers roundtrip through MessagePack 64-bit forms', () {
    expect(encodeCoreLink(0x100000000).first, 0xcf);
    expect(decodeCoreLink(encodeCoreLink(0x100000000)), 0x100000000);

    expect(encodeCoreLink(-2147483649).first, 0xd3);
    expect(decodeCoreLink(encodeCoreLink(-2147483649)), -2147483649);
  });

  test('native core call uses a fixed MessagePack tuple', () {
    const request = CoreCallRequest(
      requestId: 'request-1',
      targetObjectId: 15,
      methodName: 'getCards',
      args: <String, Object?>{'includeArchived': false},
    );

    final encoded = encodeNativeCoreCallRequest(request);
    final decoded = decodeCoreLink(encoded) as List<Object?>;

    expect(encoded.first, 0x94);
    expect(decoded, <Object?>[
      'request-1',
      15,
      'getCards',
      <String, Object?>{'includeArchived': false},
    ]);
  });

  test('native core result reads fixed success and error tuples', () {
    final success = decodeNativeCoreResult(
      encodeCoreLink(<Object?>[
        0,
        <String, Object?>{'cardCount': 1},
      ]),
    );
    expect(success, <String, Object?>{'cardCount': 1});

    expect(
      () => decodeNativeCoreResult(
        encodeCoreLink(<Object?>[
          1,
          'CARD_NOT_FOUND',
          'Card does not exist',
          <String, Object?>{'cardId': 'card-1'},
          <Object?>['CharacterCardManager.rs', 28, 7],
          'native backtrace',
        ]),
      ),
      throwsA(
        isA<CoreLinkError>()
            .having((error) => error.code, 'code', 'CARD_NOT_FOUND')
            .having((error) => error.details, 'details', <String, Object?>{
              'cardId': 'card-1',
            })
            .having(
              (error) => error.location?.file,
              'location file',
              'CharacterCardManager.rs',
            )
            .having(
              (error) => error.backtrace,
              'backtrace',
              'native backtrace',
            ),
      ),
    );
  });

  test('native push and watch requests use fixed MessagePack tuples', () {
    const request = CorePushRequest(
      requestId: 'push-1',
      targetObjectId: 41,
      methodName: 'interact',
    );

    expect(decodeCoreLink(encodeNativeCorePushOpenRequest(request)), <Object?>[
      'push-1',
      41,
      'interact',
      <String, Object?>{},
    ]);
    expect(
      decodeCoreLink(encodeNativeCorePushItem('push-1', 4, true)),
      <Object?>['push-1', 4, true],
    );

    const watchRequest = CoreWatchRequest(
      requestId: 'watch-1',
      targetObjectId: 15,
      propertyName: 'cards',
      args: null,
    );
    expect(
      decodeCoreLink(encodeNativeCoreWatchSnapshotRequest(watchRequest)),
      <Object?>['watch-1', 15, 'cards', null],
    );
    expect(
      decodeCoreLink(
        encodeNativeCoreWatchStreamRequest('subscription-1', watchRequest),
      ),
      <Object?>['subscription-1', 'watch-1', 15, 'cards', null],
    );
  });

  test('browser reverse stream items use their serializable map form', () {
    const command = RuntimeBrowserCommand(
      action: 'interact',
      sessionId: 'session-1',
      url: null,
      script: null,
      payloadJson: '{"type":"pointer"}',
      userAgent: null,
      headers: <String, String>{},
    );

    final decoded =
        decodeCoreLink(
              encodeNativeCorePushItem('browser-push-1', 0, command.toJson()),
            )
            as List<Object?>;

    expect(decoded[2], command.toJson());
    expect(decoded[2], isNot(isA<RuntimeBrowserCommand>()));
  });

  test('native watch results and events decode without map conversion', () {
    final snapshot = decodeNativeCoreWatchSnapshotResult(
      encodeCoreLink(<Object?>[
        0,
        <Object?>[
          'watch-1',
          15,
          'cards',
          'Snapshot',
          <Object?>['card-1'],
        ],
      ]),
    );
    expect(snapshot.requestId, 'watch-1');
    expect(snapshot.targetObjectId, 15);
    expect(snapshot.kind, 'Snapshot');

    final frame = decodeNativeCoreWatchFrame(
      encodeCoreLink(<Object?>[
        'subscription-1',
        <Object?>[null, 15, 'cards', 'Completed', null],
      ]),
    );
    expect(frame.subscriptionId, 'subscription-1');
    expect(frame.event.requestId, isNull);
    expect(frame.event.kind, 'Completed');

    expect(
      () => decodeNativeCoreWatchFrame(
        encodeCoreLink(<Object?>[
          1,
          'subscription-1',
          'LINK_WATCH_CHANNEL_ERROR',
          'watch channel closed',
        ]),
      ),
      throwsA(
        isA<CoreLinkError>()
            .having((error) => error.code, 'code', 'LINK_WATCH_CHANNEL_ERROR')
            .having(
              (error) => error.message,
              'message',
              'watch channel closed',
            ),
      ),
    );
  });

  test(
    'watch event decoder applies deltas before embedded stream decoding',
    () async {
      final decoder = CoreLinkEventValueDecoder();
      final factory = _EmbeddedStreamFactoryRecorder();
      final snapshot = _rawCoreEvent(
        kind: 'Snapshot',
        value: <Object?>[
          <String, Object?>{'contentStream': null, 'text': 'waiting'},
        ],
      );

      final first = decoder.decode<List<Stream<String>?>>(
        snapshot,
        decode: (bytes) => decodeCoreLink<List<Stream<String>?>>(
          bytes,
          decode: _decodeStreamList,
          embeddedStreamFactory: factory.open,
        ),
      );

      expect(first.single, isNull);

      final delta = _rawCoreEvent(
        kind: 'Delta',
        value: <String, Object?>{
          r'$coreDelta': <Object?>[
            <String, Object?>{
              'op': 'set',
              'path': <Object?>[0, 'contentStream'],
              'value': <String, Object?>{
                r'$coreStream': <String, Object?>{
                  'streamId': 'stream-ai',
                  'targetObjectId': 64,
                  'propertyName': 'openCoreStream',
                  'args': <String, Object?>{'streamId': 'stream-ai'},
                },
              },
            },
          ],
        },
      );

      final second = decoder.decode<List<Stream<String>?>>(
        delta,
        decode: (bytes) => decodeCoreLink<List<Stream<String>?>>(
          bytes,
          decode: _decodeStreamList,
          embeddedStreamFactory: factory.open,
        ),
      );

      expect(factory.openedStreamIds, <String>['stream-ai']);
      expect(await second.single!.first, 'stream chunk');
    },
  );
}

/// Creates one raw Core watch event for protocol decoder tests.
CoreEvent _rawCoreEvent({required String kind, required Object? value}) {
  return CoreEvent.raw(
    requestId: 'watch-1',
    targetObjectId: 7,
    propertyName: 'chatMessagesFlow',
    kind: kind,
    valueBytes: encodeCoreLink(value),
    decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
  );
}

/// Decodes the small stream-holder shape used by the incremental codec test.
List<Stream<String>?> _decodeStreamList(CoreLinkValueReader reader) {
  final length = reader.readArrayLength();
  return List<Stream<String>?>.generate(length, (_) {
    final fieldCount = reader.readMapLength();
    Stream<String>? contentStream;
    for (var index = 0; index < fieldCount; index += 1) {
      final key = reader.readString();
      if (key == 'contentStream') {
        contentStream = reader.readNullable<Stream<String>>(
          () => reader.readEmbeddedStream<String>((item) => item.readString()),
        );
        continue;
      }
      reader.skipValue();
    }
    return contentStream;
  }, growable: false);
}

/// Records embedded stream openings during codec tests.
class _EmbeddedStreamFactoryRecorder {
  final openedStreamIds = <String>[];

  /// Opens one deterministic test stream for an embedded descriptor.
  Stream<T> open<T>(
    String streamId,
    int targetObjectId,
    String propertyName,
    Object? args,
    T Function(CoreLinkValueReader reader) decode,
  ) {
    openedStreamIds.add(streamId);
    return Stream<T>.value('stream chunk' as T);
  }
}
