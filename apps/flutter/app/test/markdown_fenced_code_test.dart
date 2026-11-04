import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/EnhancedCodeBlock.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

import 'support/semantic_markdown_fixture.dart';

void main() {
  for (final (source, code, language) in [
    (
      '   ```c\r\n   int main(void) { return 0; }\r\n   ``` \t\r\n\r\n## 后续标题\r\n- **仍有格式**\r\n',
      'int main(void) { return 0; }',
      'c',
    ),
    (
      '````markdown\n```\n## 代码中的标题\n```` \t\n\n## 后续标题\n',
      '```\n## 代码中的标题',
      'markdown',
    ),
    (' ```text\n     保留额外缩进  \n \n ``` \n', '    保留额外缩进  \n', 'text'),
    ('   ```\n   ```\n', '', ''),
    (' ```text\n  未闭合内容  \n', ' 未闭合内容  \n', 'text'),
  ]) {
    testWidgets('renders fenced code without changing its body: $source', (
      tester,
    ) async {
      await tester.pumpWidget(_surface(source));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      final block = tester.widget<EnhancedCodeBlock>(
        find.byType(EnhancedCodeBlock),
      );
      expect(block.code, code);
      expect(block.language, language);
      if (source.contains('## 后续标题')) {
        expect(find.text('后续标题', findRichText: true), findsOneWidget);
      }
      if (source.contains('**仍有格式**')) {
        expect(find.text('仍有格式', findRichText: true), findsOneWidget);
      }
      await tester.pumpWidget(const SizedBox());
    });
  }

  final report =
      semanticMarkdownFixtures.singleWhere(
            (fixture) =>
                (fixture['source'] as String).startsWith('修好了，而且这次确实能互通。'),
          )['source']
          as String;
  final contextualReport =
      '<tool_result name="read_file"><content>ok</content></tool_result>\n$report';
  for (final (label, source) in [
    ('isolated', report),
    ('after tool result', contextualReport),
  ]) {
    for (final live in [false, true]) {
      testWidgets('hostfs report keeps later formatting $label live=$live', (
        tester,
      ) async {
        final controller = StreamController<Object>.broadcast();
        await tester.pumpWidget(
          _surface(source, live ? controller.stream : null),
        );
        if (live) {
          var count = 0;
          for (final event in semanticMarkdownEvents(source)) {
            controller.add(event);
            if (++count % 17 == 0) {
              await tester.pump(const Duration(milliseconds: 200));
            }
          }
          await tester.pump();
          await controller.close();
        }
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        expect(find.text('五、小观察', findRichText: true), findsOneWidget);
        expect(find.text('六、当前工作区内容', findRichText: true), findsOneWidget);
        expect(
          find.textContaining('#include <unistd.h>', findRichText: true),
          findsOneWidget,
        );
        expect(
          find.textContaining('路径 bug 已经解决', findRichText: true),
          findsOneWidget,
        );
        final blocks = tester.widgetList<EnhancedCodeBlock>(
          find.byType(EnhancedCodeBlock),
        );
        expect(blocks, hasLength(4));
        final nestedCode = blocks.singleWhere(
          (block) => block.code.startsWith('ELF 头:'),
        );
        expect(nestedCode.code, contains('\n\$ ./hello alpha beta\n'));
        expect(nestedCode.code, endsWith('已写出 c_output.txt'));
        await tester.pumpWidget(const SizedBox());
        if (!live) await controller.close();
      });
    }
  }
}

Widget _surface(String source, [Stream<Object>? stream]) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: MaterialApp(
    supportedLocales: AppLocalizations.supportedLocales,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    home: Scaffold(
      body: SingleChildScrollView(
        child: StreamMarkdownRenderer(
          content: source,
          isStreaming: stream != null,
          contentStream: stream,
          textColor: Colors.black,
          backgroundColor: Colors.white,
          splitMarkdownContent: splitSemanticMarkdownFixture,
        ),
      ),
    ),
  ),
);
