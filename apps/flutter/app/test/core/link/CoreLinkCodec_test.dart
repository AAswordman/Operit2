// ignore_for_file: file_names

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';

/// Verifies Dart preserves MessagePack bin values as Uint8List.
void main() {
  test(
    'message outcomes preserve native tags through JSON and MessagePack',
    () {
      final payloads = <Map<String, Object?>>[
        {
          'type': 'committed',
          'status': 'completed',
          'userMessageTimestamp': 123,
          'assistant': {'messageTimestamp': 456, 'variantIndex': 2},
        },
        {
          'type': 'committed',
          'status': 'cancelled',
          'userMessageTimestamp': null,
          'assistant': null,
        },
        {'type': 'not_persisted', 'status': 'completed'},
        {'type': 'blocked', 'message': 'Blocked by input hook'},
        {'type': 'blocked', 'message': null},
        {
          'type': 'consumed',
          'metadata': {'owner': 'character_cards'},
        },
      ];
      for (final payload in payloads) {
        final fromJson = MessageSendOutcome.fromJson(payload);
        expect(fromJson.toJson(), payload);
        final fromMessagePack = decodeCoreLink<MessageSendOutcome>(
          encodeCoreLink(payload),
          decode: MessageSendOutcome.fromMessagePack,
        );
        expect(fromMessagePack.toJson(), payload);
        if (payload['type'] == 'blocked' || payload['type'] == 'consumed') {
          expect(fromJson.status, isNull);
          expect(fromMessagePack.status, isNull);
        }
      }
    },
  );

  /// Verifies arrays are growable at decode time without copying the decoded tree.
  test('raw decoding creates independently growable nested arrays', () {
    final bytes = encodeCoreLink([
      {
        'tokens': ['old'],
      },
      [],
    ]);
    final first = decodeCoreLink<List<Object?>>(bytes);
    final second = decodeCoreLink<List<Object?>>(bytes);
    final tokens = (first[0] as Map<String, Object?>)['tokens'] as List;
    tokens.add('new');
    tokens.removeAt(0);
    (first[1] as List).add(1);
    first.add('tail');
    expect(first, [
      {
        'tokens': ['new'],
      },
      [1],
      'tail',
    ]);
    expect(second, [
      {
        'tokens': ['old'],
      },
      [],
    ]);
    expect(encodeCoreLink(second), bytes);
  });

  test('snapshot and delta byte values cannot mutate the retained base', () {
    final decoder = CoreLinkEventValueDecoder();
    Map<String, Object?> read(CoreLinkValueReader reader) =>
        reader.readValue() as Map<String, Object?>;
    for (final kind in ['Snapshot', 'Changed']) {
      final event = _rawCoreEvent(
        kind: kind,
        value: {
          'bytes': Uint8List.fromList([1, 2]),
          'count': 0,
        },
      );
      final snapshot = decoder.decodeValue(event, decode: read);
      (snapshot['bytes'] as Uint8List)[0] = 99;
      for (var count = 1; count <= 2; count++) {
        final result = decoder.decodeValue(
          _rawCoreEvent(
            kind: 'Delta',
            value: {
              r'$coreDelta': [
                {
                  'op': 'set',
                  'path': ['count'],
                  'value': count,
                },
              ],
            },
          ),
          decode: read,
        );
        expect(result['bytes'], [1, 2]);
        expect(result['count'], count);
        (result['bytes'] as Uint8List)[0] = 88;
      }
    }
  });

  test('typed tree reader traverses nested empty arrays across deltas', () {
    final decoder = CoreLinkEventValueDecoder();
    List<List<int>> read(CoreLinkValueReader reader) => List.generate(
      reader.readArrayLength(),
      (_) => List.generate(reader.readArrayLength(), (_) => reader.readInt()),
    );
    expect(
      decoder.decodeValue(
        _rawCoreEvent(
          kind: 'Snapshot',
          value: [
            [],
            [1, 2],
            [],
            [3],
          ],
        ),
        decode: read,
      ),
      [
        [],
        [1, 2],
        [],
        [3],
      ],
    );
    expect(
      decoder.decodeValue(
        _rawCoreEvent(
          kind: 'Delta',
          value: {
            r'$coreDelta': [
              {
                'op': 'set',
                'path': [1, 0],
                'value': 9,
              },
              {
                'op': 'remove',
                'path': [1, 1],
              },
              {
                'op': 'set',
                'path': [2, 0],
                'value': 4,
              },
            ],
          },
        ),
        decode: read,
      ),
      [
        [],
        [9],
        [4],
        [3],
      ],
    );
    expect(
      decoder.decodeValue(
        _rawCoreEvent(kind: 'Delta', value: {r'$coreDelta': []}),
        decode: read,
      ),
      [
        [],
        [9],
        [4],
        [3],
      ],
    );
    expect(
      () => CoreLinkEventValueDecoder().decodeValue(
        _rawCoreEvent(kind: 'Delta', value: {r'$coreDelta': []}),
        decode: read,
      ),
      throwsStateError,
    );
  });

  test(
    'error labels stay concise while diagnostics retain the full native trace',
    () {
      final trace = List.generate(
        100,
        (index) => '$index: <unknown>',
      ).join('\n');
      final error = CoreLinkError(
        code: 'INTERNAL_ERROR',
        message: 'watch request=42 object=4 property=progress',
        backtrace: trace,
      );
      expect(
        error.toString(),
        'INTERNAL_ERROR: watch request=42 object=4 property=progress',
      );
      expect(error.toDiagnosticString(), contains(trace));
      expect(error.backtrace, trace);
    },
  );
  test('route permission errors expose structured client state', () {
    const error = CoreLinkError(
      code: 'ROUTE_PERMISSION_DENIED',
      message: 'Space route chatMessagesFlow requires capability chat.read',
      details: <String, Object?>{
        'method': 'chatMessagesFlow',
        'requiredCapability': 'chat.read',
        'targetNodeId': 'windows-node',
      },
    );

    expect(error.isRoutePermissionDenied, isTrue);
    expect(error.requiredCapability, 'chat.read');
    expect(error.targetNodeId, 'windows-node');
    expect(error.deniedMethod, 'chatMessagesFlow');
  });

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
      target: 'core/test15',
      methodName: 'getCards',
      args: <String, Object?>{'includeArchived': false},
    );

    final encoded = encodeNativeCoreCallRequest(request);
    final decoded = decodeCoreLink(encoded) as List<Object?>;

    expect(encoded.first, 0x94);
    expect(decoded, <Object?>[
      'request-1',
      'core/test15',
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
      target: 'core/test41',
      methodName: 'interact',
    );

    expect(decodeCoreLink(encodeNativeCorePushOpenRequest(request)), <Object?>[
      'push-1',
      'core/test41',
      'interact',
      <String, Object?>{},
    ]);
    expect(
      decodeCoreLink(encodeNativeCorePushItem('push-1', 4, true)),
      <Object?>['push-1', 4, true],
    );

    const watchRequest = CoreWatchRequest(
      requestId: 'watch-1',
      target: 'core/test15',
      propertyName: 'cards',
      args: null,
    );
    expect(
      decodeCoreLink(encodeNativeCoreWatchSnapshotRequest(watchRequest)),
      <Object?>['watch-1', 'core/test15', 'cards', null],
    );
    expect(
      decodeCoreLink(
        encodeNativeCoreWatchStreamRequest('subscription-1', watchRequest),
      ),
      <Object?>['subscription-1', 'watch-1', 'core/test15', 'cards', null],
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
          'core/test15',
          'cards',
          'Snapshot',
          <Object?>['card-1'],
        ],
      ]),
    );
    expect(snapshot.requestId, 'watch-1');
    expect(snapshot.target, 'core/test15');
    expect(snapshot.kind, 'Snapshot');

    final frame = decodeNativeCoreWatchFrame(
      encodeCoreLink(<Object?>[
        'subscription-1',
        <Object?>[null, 'core/test15', 'cards', 'Completed', null],
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

      final first = decoder.decodeValue<List<Stream<String>?>>(
        snapshot,
        decode: _decodeStreamList,
        embeddedStreamFactory: factory.open,
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
                  'target': 'core/test64',
                  'propertyName': 'openCoreStream',
                  'args': <String, Object?>{'streamId': 'stream-ai'},
                },
              },
            },
          ],
        },
      );

      final second = decoder.decodeValue<List<Stream<String>?>>(
        delta,
        decode: _decodeStreamList,
        embeddedStreamFactory: factory.open,
      );

      expect(factory.openedStreamIds, <String>['stream-ai']);
      expect(await second.single!.first, 'stream chunk');
    },
  );

  test('list deltas decode only touched items and retain other identities', () {
    final decoder = CoreLinkListEventDecoder<Map<String, Object?>>();
    var decodedItems = 0;

    /// Decodes one test item and records the number of typed decodes.
    Map<String, Object?> readItem(CoreLinkValueReader reader) {
      decodedItems += 1;
      return reader.readValue() as Map<String, Object?>;
    }

    final first = decoder.decode(
      _rawCoreEvent(
        kind: 'Snapshot',
        value: [
          {'text': 'old', 'other': 7},
          {'text': 'unchanged'},
        ],
      ),
      decodeItem: readItem,
    );
    expect(decodedItems, 2);
    final second = decoder.decode(
      _rawCoreEvent(
        kind: 'Delta',
        value: {
          r'$coreDelta': [
            {
              'op': 'set',
              'path': [0, 'text'],
              'value': 'new',
            },
            {
              'op': 'set',
              'path': [0, 'other'],
              'value': 8,
            },
            {
              'op': 'set',
              'path': [2],
              'value': {'text': 'appended'},
            },
          ],
        },
      ),
      decodeItem: readItem,
    );
    expect(decodedItems, 4);
    expect(second, [
      {'text': 'new', 'other': 8},
      {'text': 'unchanged'},
      {'text': 'appended'},
    ]);
    expect(identical(first[1], second[1]), isTrue);
    expect(first[0], {'text': 'old', 'other': 7});

    final third = decoder.decode(
      _rawCoreEvent(
        kind: 'Delta',
        value: {
          r'$coreDelta': [
            {
              'op': 'remove',
              'path': [2],
            },
            {
              'op': 'remove',
              'path': [0, 'other'],
            },
          ],
        },
      ),
      decodeItem: readItem,
    );
    expect(decodedItems, 5);
    expect(third, [
      {'text': 'new'},
      {'text': 'unchanged'},
    ]);
    expect(identical(second[1], third[1]), isTrue);
    final fourth = decoder.decode(
      _rawCoreEvent(kind: 'Delta', value: {r'$coreDelta': []}),
      decodeItem: readItem,
    );
    expect(decodedItems, 5);
    expect(identical(third[0], fourth[0]), isTrue);
    final changed = decoder.decode(
      _rawCoreEvent(
        kind: 'Changed',
        value: [
          {'text': 'reset'},
        ],
      ),
      decodeItem: readItem,
    );
    expect(changed, [
      {'text': 'reset'},
    ]);
    expect(decodedItems, 6);
  });

  test(
    'list deltas attach new embedded streams without reopening other items',
    () async {
      final decoder = CoreLinkListEventDecoder<Stream<String>?>();
      final factory = _EmbeddedStreamFactoryRecorder();

      /// Reads the embedded stream field of a single test item.
      Stream<String>? readItem(CoreLinkValueReader reader) {
        final fieldCount = reader.readMapLength();
        Stream<String>? stream;
        for (var index = 0; index < fieldCount; index += 1) {
          final field = reader.readString();
          if (field == 'contentStream') {
            stream = reader.readNullable<Stream<String>>(
              () => reader.readEmbeddedStream<String>(
                (item) => item.readString(),
              ),
            );
          } else {
            reader.skipValue();
          }
        }
        return stream;
      }

      final first = decoder.decode(
        _rawCoreEvent(
          kind: 'Snapshot',
          value: [
            {'contentStream': null},
            {'contentStream': null},
          ],
        ),
        decodeItem: readItem,
        embeddedStreamFactory: factory.open,
      );
      expect(first, [null, null]);
      final second = decoder.decode(
        _rawCoreEvent(
          kind: 'Delta',
          value: {
            r'$coreDelta': [
              {
                'op': 'set',
                'path': [0, 'contentStream'],
                'value': {
                  r'$coreStream': {
                    'streamId': 'stream-ai',
                    'target': 'core/test64',
                    'propertyName': 'openCoreStream',
                    'args': {'streamId': 'stream-ai'},
                  },
                },
              },
            ],
          },
        ),
        decodeItem: readItem,
        embeddedStreamFactory: factory.open,
      );
      expect(factory.openedStreamIds, ['stream-ai']);
      expect(await second.first!.first, 'stream chunk');
      decoder.decode(
        _rawCoreEvent(kind: 'Delta', value: {r'$coreDelta': []}),
        decodeItem: readItem,
        embeddedStreamFactory: factory.open,
      );
      expect(factory.openedStreamIds, ['stream-ai']);
    },
  );

  test(
    'long list decodes one changed message without decoding its history',
    () {
      final decoder = CoreLinkListEventDecoder<Map<String, Object?>>();
      var decodedItems = 0;

      /// Counts typed item decodes in the long-history transport test.
      Map<String, Object?> readItem(CoreLinkValueReader reader) {
        decodedItems += 1;
        return reader.readValue() as Map<String, Object?>;
      }

      final history = decoder.decode(
        _rawCoreEvent(
          kind: 'Snapshot',
          value: List.generate(
            1000,
            (index) => {'id': index, 'content': 'text'},
          ),
        ),
        decodeItem: readItem,
      );
      expect(decodedItems, 1000);
      final updated = decoder.decode(
        _rawCoreEvent(
          kind: 'Delta',
          value: {
            r'$coreDelta': [
              {
                'op': 'set',
                'path': [999, 'content'],
                'value': 'streaming',
              },
            ],
          },
        ),
        decodeItem: readItem,
      );
      expect(decodedItems, 1001);
      expect(identical(history.first, updated.first), isTrue);
      expect(updated.last['content'], 'streaming');
    },
  );

  /// Verifies nested append and removal preserve the optimized list's retained base.
  test('list deltas mutate nested arrays across consecutive events', () {
    for (final kind in ['Snapshot', 'Changed']) {
      final decoder = CoreLinkListEventDecoder<Map<String, Object?>>();
      var decodedItems = 0;

      /// Counts typed decodes to ensure untouched history remains shared.
      Map<String, Object?> readItem(CoreLinkValueReader reader) {
        decodedItems += 1;
        return reader.readValue() as Map<String, Object?>;
      }

      final original = decoder.decode(
        _rawCoreEvent(
          kind: kind,
          value: [
            {
              'segments': [
                {
                  'tokens': ['old'],
                },
              ],
              'bytes': Uint8List.fromList([1, 2]),
            },
            {'text': 'unchanged'},
          ],
        ),
        decodeItem: readItem,
      );
      final appended = decoder.decode(
        _rawCoreEvent(
          kind: 'Delta',
          value: {
            r'$coreDelta': [
              {
                'op': 'set',
                'path': [0, 'segments', 0, 'tokens', 1],
                'value': 'new',
              },
              {
                'op': 'set',
                'path': [0, 'segments', 1],
                'value': {'tokens': []},
              },
            ],
          },
        ),
        decodeItem: readItem,
      );
      expect(appended[0]['segments'], [
        {
          'tokens': ['old', 'new'],
        },
        {'tokens': []},
      ]);
      expect(original[0]['segments'], [
        {
          'tokens': ['old'],
        },
      ]);
      expect(appended[0]['bytes'], isA<Uint8List>());
      expect(appended[0]['bytes'], [1, 2]);
      expect(identical(original[1], appended[1]), isTrue);
      expect(decodedItems, 3);

      final removed = decoder.decode(
        _rawCoreEvent(
          kind: 'Delta',
          value: {
            r'$coreDelta': [
              {
                'op': 'remove',
                'path': [0, 'segments', 0, 'tokens', 0],
              },
              {
                'op': 'remove',
                'path': [0, 'segments', 1],
              },
            ],
          },
        ),
        decodeItem: readItem,
      );
      expect(removed[0]['segments'], [
        {
          'tokens': ['new'],
        },
      ]);
      expect(appended[0]['segments'], [
        {
          'tokens': ['old', 'new'],
        },
        {'tokens': []},
      ]);
      expect(identical(original[1], removed[1]), isTrue);
      expect(decodedItems, 4);
    }
  });

  /// Verifies a root replacement can be patched further in the same delta batch.
  test('list delta root replacements own mutable nested arrays', () {
    final decoder = CoreLinkListEventDecoder<Map<String, Object?>>();

    /// Reads one item without changing the wire tree's collection semantics.
    Map<String, Object?> readItem(CoreLinkValueReader reader) =>
        reader.readValue() as Map<String, Object?>;

    decoder.decode(
      _rawCoreEvent(
        kind: 'Snapshot',
        value: [
          {'text': 'old'},
        ],
      ),
      decodeItem: readItem,
    );
    final result = decoder.decode(
      _rawCoreEvent(
        kind: 'Delta',
        value: {
          r'$coreDelta': [
            {
              'op': 'set',
              'path': [],
              'value': [
                {'tokens': []},
              ],
            },
            {
              'op': 'set',
              'path': [0, 'tokens', 0],
              'value': 'new',
            },
          ],
        },
      ),
      decodeItem: readItem,
    );
    expect(result, [
      {
        'tokens': ['new'],
      },
    ]);
  });

  /// Verifies replaced and appended elements are mutable before their first typed decode.
  test('list delta pending elements support nested growth and shrinkage', () {
    for (final index in [0, 1]) {
      final decoder = CoreLinkListEventDecoder<Map<String, Object?>>();

      /// Reads one item so the regression exercises raw MessagePack array decoding.
      Map<String, Object?> readItem(CoreLinkValueReader reader) =>
          reader.readValue() as Map<String, Object?>;

      final original = decoder.decode(
        _rawCoreEvent(
          kind: 'Snapshot',
          value: [
            {'text': 'original'},
          ],
        ),
        decodeItem: readItem,
      );
      final result = decoder.decode(
        _rawCoreEvent(
          kind: 'Delta',
          value: {
            r'$coreDelta': [
              {
                'op': 'set',
                'path': [index],
                'value': {
                  'tokens': ['old'],
                },
              },
              {
                'op': 'set',
                'path': [index, 'tokens', 1],
                'value': 'new',
              },
              {
                'op': 'remove',
                'path': [index, 'tokens', 0],
              },
            ],
          },
        ),
        decodeItem: readItem,
      );
      expect(result[index], {
        'tokens': ['new'],
      });
      expect(result.length, index + 1);
      expect(original, [
        {'text': 'original'},
      ]);
      if (index == 1) {
        expect(identical(original[0], result[0]), isTrue);
      }
    }
  });

  test('list decoder rejects a delta before the first snapshot', () {
    expect(
      () => CoreLinkListEventDecoder<int>().decode(
        _rawCoreEvent(kind: 'Delta', value: {r'$coreDelta': []}),
        decodeItem: (reader) => reader.readInt(),
      ),
      throwsStateError,
    );
  });
}

/// Creates one raw Core watch event for protocol decoder tests.
CoreEvent _rawCoreEvent({required String kind, required Object? value}) {
  return CoreEvent.raw(
    requestId: 'watch-1',
    target: 'core/test7',
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
    String target,
    String propertyName,
    Object? args,
    T Function(CoreLinkValueReader reader) decode,
  ) {
    openedStreamIds.add(streamId);
    return Stream<T>.value('stream chunk' as T);
  }
}
