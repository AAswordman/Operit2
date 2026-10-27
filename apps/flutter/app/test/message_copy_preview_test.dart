import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/MessageCopyPreview.dart';

import 'support/semantic_markdown_fixture.dart';

void main() {
  test(
    'copies semantic inline styles, links and code without losing spaces',
    () async {
      final source = semanticMarkdownFixtures[3]['source'] as String;
      final result = await markdownToPlainTextForCopy(
        source,
        splitMarkdownContent: splitSemanticMarkdownFixture,
      );
      expect(
        result,
        '标题 链接 (https://example.com)\n\n----dart-----\nfinal value = 1;',
      );
    },
  );

  test(
    'copies footnote markers and definitions without losing their association',
    () async {
      final source =
          semanticMarkdownFixtures.singleWhere(
                (fixture) => (fixture['source'] as String).startsWith('正文[^1]'),
              )['source']
              as String;
      final result = await markdownToPlainTextForCopy(
        source,
        splitMarkdownContent: splitSemanticMarkdownFixture,
      );
      expect(result, contains('正文[1]和加粗[note]，重复[1]'));
      expect(result, contains('[1] 第一条脚注与链接 (https://example.com)'));
      expect(result, contains('[note] 第二条注释'));
    },
  );

  test('converts tables through the same Core inline semantics', () async {
    final source = semanticMarkdownFixtures[4]['source'] as String;
    final result = await markdownToPlainTextForCopy(
      source,
      splitMarkdownContent: splitSemanticMarkdownFixture,
    );
    expect(result, '名称\t数量\n苹果\t2');
  });
}
