import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';

/// Downloads one browser runtime artifact and verifies its pinned SHA-256 digest.
Future<void> downloadVerifiedWebRuntimeAsset(
  Uri url,
  File destination,
  String expectedSha256,
) async {
  // Trust cached bytes only after rechecking the exact immutable artifact pin.
  if (destination.existsSync()) {
    final digest = await sha256.bind(destination.openRead()).first;
    if (digest.toString() == expectedSha256) {
      return;
    }
  }
  final client = HttpClient();
  client.findProxy = HttpClient.findProxyFromEnvironment;
  client.connectionTimeout = const Duration(seconds: 30);
  try {
    final request = await client.getUrl(url);
    final response = await request.close();
    if (response.statusCode != HttpStatus.ok) {
      throw HttpException(
        'Failed to download $url: HTTP ${response.statusCode}',
        uri: url,
      );
    }
    final content = await response.fold<BytesBuilder>(
      BytesBuilder(copy: false),
      (builder, chunk) => builder..add(chunk),
    );
    final bytes = content.takeBytes();
    final actualSha256 = sha256.convert(bytes).toString();
    if (actualSha256 != expectedSha256) {
      throw StateError(
        'Invalid SHA-256 for $url: expected $expectedSha256, got $actualSha256',
      );
    }
    await destination.parent.create(recursive: true);
    await destination.writeAsBytes(bytes, flush: true);
  } finally {
    client.close(force: true);
  }
}
