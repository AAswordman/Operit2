import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogTrimmer.dart';

void main() {
  test('keeps text under the cap unchanged', () {
    final content = 'first record\nsecond record\n';
    expect(trimClientLogText(content), content);
  });

  test('evicts oldest whole records and keeps the newest', () {
    final line = 'x' * 1024;
    final content =
        [for (var index = 0; index < 3000; index++) 'record-$index $line\n'].join();
    expect(utf8.encode(content).length, greaterThan(clientLogMaxBytes));

    final trimmed = trimClientLogText(content);
    expect(trimmed, contains('oldest records evicted'));
    expect(trimmed, contains('record-2999 '), reason: 'newest records survive');
    expect(trimmed, isNot(contains('record-0 ')), reason: 'oldest records are evicted');
    final keptBytes = utf8.encode(trimmed).length - evictionMarker.length;
    expect(
      keptBytes,
      lessThanOrEqualTo(clientLogKeepBytes + 2 * 1024),
      reason: 'the kept tail stays near the keep target plus one record',
    );
    expect(
      trimmed.split('\n').elementAt(1).startsWith('record-'),
      isTrue,
      reason: 'eviction keeps whole records only',
    );
  });

  test('keeps whole records only, including across multi-byte cuts', () {
    final bytes = utf8.encode('first\nsecond\nthird\n');
    expect(utf8.decode(keepNewestLogBytes(bytes, 100)), 'first\nsecond\nthird\n');
    expect(utf8.decode(keepNewestLogBytes(bytes, 8)), 'third\n');
    final multibyte = utf8.encode('é\n中文记录\nlast\n');
    expect(utf8.decode(keepNewestLogBytes(multibyte, 6)), 'last\n');
    expect(keepNewestLogBytes(utf8.encode('one-single-record'), 4), isEmpty);
  });

  test('oversized text without record boundaries drains to the marker', () {
    final content = 'y' * (clientLogMaxBytes + 1);
    final trimmed = trimClientLogText(content);
    expect(trimmed, evictionMarker, reason: 'nothing can be kept without a boundary');
  });
}

const String evictionMarker = '... client log trimmed; oldest records evicted\n';
