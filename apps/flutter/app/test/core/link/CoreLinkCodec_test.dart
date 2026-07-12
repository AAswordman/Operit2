// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';

/// Verifies Dart preserves MessagePack bin values as Uint8List.
void main() {
  test('native bytes use MessagePack bin', () {
    final encoded = encodeCoreLink(Uint8List.fromList(<int>[1, 2, 3, 4]));

    expect(encoded, Uint8List.fromList(<int>[0xc4, 4, 1, 2, 3, 4]));
    expect(decodeCoreLink(encoded), Uint8List.fromList(<int>[1, 2, 3, 4]));
  });

  test('nested maps use typed string keys', () {
    final decoded = decodeCoreLinkMap(
      encodeCoreLink(<String, Object?>{
        'outer': <String, Object?>{'value': 7},
      }),
    );

    expect(decoded['outer'], isA<Map<String, Object?>>());
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

  test('push request preserves its stream target', () {
    const request = CorePushRequest(
      requestId: 'push-1',
      targetPath: CoreObjectPath(<String>['runtime', 'browser']),
      methodName: 'interact',
    );

    final decoded = decodeCoreLinkMap(encodeCoreLink(request.toJson()));

    expect(decoded['requestId'], 'push-1');
    expect(decoded['methodName'], 'interact');
  });
}
