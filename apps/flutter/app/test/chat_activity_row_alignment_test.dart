import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/part/CustomXmlRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';

/// Verifies thinking and tool headers share geometry and theme-aware typography.
void main() {
  for (final (width, fontSize, scale, locale) in [
    (280.0, 12.0, 1.0, const Locale('zh')),
    (280.0, 16.0, 1.5, const Locale('zh')),
    (640.0, 12.0, 1.0, const Locale('en')),
    (640.0, 16.0, 1.5, const Locale('en')),
  ]) {
    for (final streaming in [false, true]) {
      testWidgets(
        'aligns activity headers at $width/$fontSize/$scale/$locale, streaming=$streaming',
        (tester) async {
          Widget thinkingRow() => CustomXmlRenderer(
            xmlContent: streaming
                ? '<think>Review files'
                : '<think>Review files</think>',
            isStreaming: streaming,
            xmlStream: streaming ? const Stream<String>.empty() : null,
            textColor: Colors.black,
            splitMarkdownContent: (_) async => [],
          );
          Widget toolRow() => ToolCallRow(
            presentation: ToolPresentation.fromCall(
              'read_file',
              '<param name="path">/app/work</param>',
              display: const ToolDisplayDeclaration(
                labels: {'en': 'Read', 'zh': 'Read'},
                icon: 'file',
                targetParameter: 'path',
              ),
            ),
            isStreaming: streaming,
          );
          final rows = [
            thinkingRow(),
            toolRow(),
            toolRow(),
            thinkingRow(),
            toolRow(),
          ];
          await tester.pumpWidget(
            MaterialApp(
              locale: locale,
              localizationsDelegates: AppLocalizations.localizationsDelegates,
              supportedLocales: AppLocalizations.supportedLocales,
              theme: ThemeData(
                textTheme: TextTheme(
                  bodySmall: TextStyle(
                    fontSize: fontSize,
                    fontWeight: FontWeight.w600,
                    letterSpacing: 0.3,
                  ),
                ),
              ),
              home: MediaQuery(
                data: MediaQueryData(textScaler: TextScaler.linear(scale)),
                child: Scaffold(
                  body: SizedBox(
                    width: width,
                    child: Padding(
                      padding: const EdgeInsets.only(left: 22),
                      child: Column(
                        children: [
                          for (var index = 0; index < rows.length; index++)
                            KeyedSubtree(
                              key: ValueKey('activity-$index'),
                              child: rows[index],
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          );
          await tester.pump();

          final thinkingTitle = find
              .text(locale.languageCode == 'zh' ? '思考过程' : 'Thinking Process')
              .first;
          final toolTitle = find.text('Read').first;
          final preview = find.text('Review files').first;
          final target = find.text('/app/work').first;
          final thinkingIcon = find.byIcon(Icons.psychology_outlined).first;
          final toolIcon = find.byIcon(Icons.description_outlined).first;

          expect(
            tester.getTopLeft(thinkingTitle).dx,
            tester.getTopLeft(toolTitle).dx,
          );
          expect(
            tester.getTopLeft(thinkingIcon).dx,
            tester.getTopLeft(toolIcon).dx,
          );
          expect(tester.getSize(thinkingIcon), tester.getSize(toolIcon));
          expect(
            tester.getTopLeft(thinkingTitle).dy -
                tester.getTopLeft(thinkingIcon).dy,
            closeTo(
              tester.getTopLeft(toolTitle).dy - tester.getTopLeft(toolIcon).dy,
              0.001,
            ),
          );
          final thinkingHeader = find.ancestor(
            of: thinkingTitle,
            matching: find.byType(InkWell),
          );
          final toolHeader = find.ancestor(
            of: toolTitle,
            matching: find.byType(InkWell),
          );
          expect(
            tester.getSize(thinkingHeader).height,
            tester.getSize(toolHeader).height,
          );

          // Compare complete rows as well as headers: trailing padding must
          // not change the rhythm of thinking/tool and tool/tool transitions.
          final rowRects = [
            for (var index = 0; index < rows.length; index++)
              tester.getRect(find.byKey(ValueKey('activity-$index'))),
          ];
          final rowHeight = tester.getSize(toolHeader).height;
          for (var index = 0; index < rowRects.length; index++) {
            expect(rowRects[index].height, closeTo(rowHeight, 0.001));
            if (index > 0) {
              expect(
                rowRects[index].center.dy - rowRects[index - 1].center.dy,
                closeTo(rowHeight, 0.001),
              );
            }
          }

          final titleStyle = tester.widget<Text>(toolTitle).style!;
          for (final text in [thinkingTitle, preview, target]) {
            final style = tester.widget<Text>(text).style!;
            expect(style.fontSize, fontSize);
            expect(style.fontSize, titleStyle.fontSize);
            expect(style.fontWeight, titleStyle.fontWeight);
            expect(style.fontFamily, titleStyle.fontFamily);
            expect(style.letterSpacing, titleStyle.letterSpacing);
            expect(style.height, titleStyle.height);
          }
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        },
      );
    }
  }
}
