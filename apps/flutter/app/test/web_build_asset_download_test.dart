import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:flutter_test/flutter_test.dart';

import '../hook/verified_asset_download.dart';

void main() {
  late Directory root;
  late File destination;
  const expectedBytes = <int>[1, 2, 3, 4];
  final expectedDigest = sha256.convert(expectedBytes).toString();

  setUp(() async {
    root = await Directory.systemTemp.createTemp('operit-web-asset-test-');
    destination = File('${root.path}/v86/seabios.bin');
  });
  tearDown(() async {
    await root.delete(recursive: true);
  });

  test(
    'a verified cached artifact can be used without a network connection',
    () async {
      await destination.parent.create(recursive: true);
      await destination.writeAsBytes(expectedBytes);
      await downloadVerifiedWebRuntimeAsset(
        Uri.parse('http://127.0.0.1:1/unreachable'),
        destination,
        expectedDigest,
      );
      expect(await destination.readAsBytes(), expectedBytes);
    },
  );

  for (final cached in <bool>[false, true]) {
    test(
      'a ${cached ? 'corrupt' : 'missing'} cache downloads and verifies the artifact',
      () async {
        if (cached) {
          await destination.parent.create(recursive: true);
          await destination.writeAsBytes(<int>[99]);
        }
        final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
        addTearDown(() => server.close(force: true));
        var requests = 0;
        server.listen((request) async {
          requests++;
          request.response.add(expectedBytes);
          await request.response.close();
        });
        await downloadVerifiedWebRuntimeAsset(
          Uri.parse('http://127.0.0.1:${server.port}/artifact'),
          destination,
          expectedDigest,
        );
        expect(requests, 1);
        expect(await destination.readAsBytes(), expectedBytes);
      },
    );
  }

  test(
    'a downloaded digest mismatch does not publish unverified bytes',
    () async {
      await destination.parent.create(recursive: true);
      await destination.writeAsBytes(<int>[99]);
      final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
      addTearDown(() => server.close(force: true));
      server.listen((request) async {
        request.response.add(<int>[5, 6, 7]);
        await request.response.close();
      });
      await expectLater(
        downloadVerifiedWebRuntimeAsset(
          Uri.parse('http://127.0.0.1:${server.port}/artifact'),
          destination,
          expectedDigest,
        ),
        throwsA(isA<StateError>()),
      );
      expect(await destination.readAsBytes(), <int>[99]);
    },
  );
}
