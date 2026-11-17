import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:fvp/src/media_source.dart';

/// Verifies real file ownership at the native video input boundary.
void main() {
  test('data video bytes become a seekable file until disposal', () async {
    final bytes = List<int>.generate(4096, (index) => index % 256);
    final source = await MdkMediaSource.open(
      Uri.dataFromBytes(bytes, mimeType: 'video/mp4').toString(),
    );
    final file = File(source.uri);
    try {
      expect(await file.readAsBytes(), bytes);
      final handle = await file.open();
      try {
        await handle.setPosition(3000);
        expect(await handle.read(16), bytes.sublist(3000, 3016));
        await handle.setPosition(0);
        expect(await handle.read(16), bytes.sublist(0, 16));
      } finally {
        await handle.close();
      }
    } finally {
      await source.dispose();
    }
    expect(await file.parent.exists(), isFalse);
    await source.dispose();
  });

  test(
    'simultaneous players own separate files even for identical inputs',
    () async {
      final uri = Uri.dataFromBytes([
        1,
        2,
        3,
      ], mimeType: 'video/mp4').toString();
      final first = await MdkMediaSource.open(uri);
      final second = await MdkMediaSource.open(uri);
      try {
        expect(first.uri, isNot(second.uri));
        await first.dispose();
        expect(await File(first.uri).exists(), isFalse);
        expect(await File(second.uri).readAsBytes(), [1, 2, 3]);
      } finally {
        await first.dispose();
        await second.dispose();
      }
    },
  );

  test('external files stay owned by their caller', () async {
    final directory = await Directory.systemTemp.createTemp('fvp-test-');
    final file = await File(
      '${directory.path}/external.mp4',
    ).writeAsBytes([1, 2]);
    try {
      final source = await MdkMediaSource.open(file.path);
      expect(source.uri, file.path);
      await source.dispose();
      expect(await file.readAsBytes(), [1, 2]);
    } finally {
      await directory.delete(recursive: true);
    }
  });

  test('network URLs are passed to MDK without materialization', () async {
    const url = 'https://example.com/video.mp4?token=abc';
    final source = await MdkMediaSource.open(url);
    expect(source.uri, url);
    await source.dispose();
  });

  test(
    'malformed data URLs fail before allocating a temporary input',
    () async {
      await expectLater(
        MdkMediaSource.open('data:video/mp4;base64,%%%'),
        throwsFormatException,
      );
    },
  );
}
