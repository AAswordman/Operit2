import 'dart:math';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/part/ThinkingPreviewBuffer.dart';

void main() {
  final examples = <String>[
    '',
    '  # 标题\n1. 列表\n> 引用\n**强调** 和中文 😀  ',
    'before ```dart\nprint("<tag>");\n``` after',
    'before ```unterminated\n# heading\n<tag> tail',
    'a `` partial ``` fence ````` then ``` and `` tail',
    'a <> b <<nested> c <tag\nvalue> d <unfinished',
    '#\n\n## second\r\n1. third\r> fourth\u2028# fifth\u2029* sixth',
    '#\t\n# heading\n  # indented\n123number\n1.\ntext',
    '<tag>\u0085a\u0085 \u0085<tag>\u0085',
    'a\t\u00a0\u1680\u2000\u200a\u202f\u205f\u3000\ufeffb',
    '${'历史😀 ' * 300}last line\n# heading',
    'retained before ${'x' * 1100} ``` ${'code <tag>\n# x ' * 150}``` tail',
    'retained before ${'x' * 1100} <${'tag\n# x ' * 250}> tail',
    'retained before ${'#' * 2100} trailing markers',
    'retained before\n${'#' * 2100} trailing markers',
    'retained${'\u0085 ' * 1100}tail',
  ];

  for (var index = 0; index < examples.length; index++) {
    test('matches original cleanup at every UTF-16 boundary: $index', () {
      final buffer = ThinkingPreviewBuffer();
      final source = examples[index];
      for (var end = 0; end <= source.length; end++) {
        final snapshot = source.substring(0, end).trim();
        expect(
          buffer.update(snapshot),
          _originalPreview(snapshot),
          reason: 'example $index, prefix length $end',
        );
      }
    });
  }

  test('matches original cleanup for randomized chunks and replacements', () {
    final random = Random(317);
    final fragments = [
      'normal text ',
      '中文😀',
      '<',
      '>',
      '<>',
      '<tag>',
      '</tag>',
      '`',
      '``',
      '```',
      '````',
      '\n',
      '\r\n',
      '\r',
      '\t',
      '  ',
      '#',
      '*',
      '-',
      '.',
      '123',
      '> ',
      '\u0085',
      '\u00a0',
      '\u2028',
      '\u2029',
      '\ufeff',
      '\u200b',
      'x' * 90,
    ];
    final buffer = ThinkingPreviewBuffer();
    for (var document = 0; document < 80; document++) {
      var source = '';
      for (var chunk = 0; chunk < 90; chunk++) {
        source += fragments[random.nextInt(fragments.length)];
        final snapshot = source.trim();
        expect(
          buffer.update(snapshot),
          _originalPreview(snapshot),
          reason: 'document $document, chunk $chunk: $snapshot',
        );
      }
    }
  });

  test(
    'rebuilds after rollback, same-length edits, and source replacement',
    () {
      final buffer = ThinkingPreviewBuffer();
      for (final source in [
        '# initial',
        '# initial more',
        '# initial',
        '# edited!',
        'new source',
        '',
        '```code',
        '```code```',
        '```other',
        'same length',
        'same LENGTH',
      ]) {
        expect(buffer.update(source), _originalPreview(source));
      }
    },
  );

  test('reuses unchanged results and catches up after skipped snapshots', () {
    final buffer = ThinkingPreviewBuffer();
    final initial = 'prefix ' * 300;
    final preview = buffer.update(initial);
    expect(buffer.update(initial), same(preview));
    expect(
      buffer.update(String.fromCharCodes(initial.codeUnits)),
      same(preview),
    );
    final after = '$initial```code\n<tag>```\n# next heading';
    expect(buffer.update(after), _originalPreview(after));
    expect(buffer.update(after).length, lessThanOrEqualTo(1000));
  });
  test('tracks window position across eviction, syntax rollback and edits', () {
    final buffer = ThinkingPreviewBuffer();
    final prefix = 'x' * 1100;
    final openFence = '$prefix ```${'code ' * 400}';
    final openTag = '$prefix <${'tag ' * 400}';
    for (final source in [
      prefix,
      '$prefix more',
      openFence,
      '$openFence``` tail',
      openTag,
      '$openTag> tail',
      '$prefix${'\u0085 ' * 1500}tail',
      'replacement',
      '',
    ]) {
      expect(buffer.update(source), _originalPreview(source));
      expect(buffer.textOffset, max(0, _originalClean(source).length - 1000));
    }
  });
}

// Independent compatibility oracle: the production cleanup before buffering.
String _originalPreview(String text) {
  final cleaned = _originalClean(text);
  return cleaned.length > 1000
      ? cleaned.substring(cleaned.length - 1000)
      : cleaned;
}

String _originalClean(String text) => text
    .replaceAll(RegExp(r'```[\s\S]*?```'), ' [代码] ')
    .replaceAll(RegExp(r'<[^>]+>'), ' ')
    .replaceAll(RegExp(r'^[#*>\-\d.]+\s+', multiLine: true), '')
    .replaceAll(RegExp(r'\r?\n'), ' ')
    .replaceAll(RegExp(r'\s+'), ' ')
    .trim();
