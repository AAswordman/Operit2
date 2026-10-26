import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/CoreByteTransport.dart';
import 'package:operit2/core/bridge/FfiCoreTransport.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';

/// Verifies that obsolete native ABI descriptors cannot establish a connection.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  for (final version in [1, 2]) {
    test(
      'rejects ABI v$version before binding session or function addresses',
      () async {
        const channel = MethodChannel('operit/runtime');
        final messenger =
            TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
        final transport = FfiCoreTransport();
        var connectionAttempts = 0;
        messenger.setMockMethodCallHandler(channel, (call) async {
          expect(call.method, 'connectCoreFfi');
          connectionAttempts++;
          return jsonEncode({'version': version});
        });
        addTearDown(() async {
          await transport.close();
          messenger.setMockMethodCallHandler(channel, null);
        });

        await expectLater(
          transport.request(
            CoreTransportOperation.call,
            encodeCoreLinkMessage(null),
          ),
          throwsA(
            isA<StateError>().having(
              (error) => error.message,
              'message',
              'Unsupported Core FFI or Dart native API version',
            ),
          ),
        );
        expect(connectionAttempts, 1);
      },
    );
  }
}
