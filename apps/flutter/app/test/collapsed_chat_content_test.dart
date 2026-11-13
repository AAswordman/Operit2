import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/CustomXmlRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/DetailsTagRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/FileDiffDisplay.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceInlineDiff.dart';

import 'support/markdown_event_fixture.dart';

Widget _surface(Widget child) => MaterialApp(
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(body: SingleChildScrollView(child: child)),
);

class _CountingToolPresentation extends ToolPresentation {
  _CountingToolPresentation()
    : super(
        toolName: 'Example tool',
        icon: Icons.build_outlined,
        rawParams: '<param name="source">large hidden source</param>',
        result: '',
        isSuccess: true,
        fileDiffs: const [],
      );

  var summaryReads = 0;

  @override
  String get parameterSummary {
    summaryReads++;
    return super.parameterSummary;
  }
}

void main() {
  for (final kind in ['thinking', 'details', 'search']) {
    testWidgets('$kind parses its Markdown only after expansion', (
      tester,
    ) async {
      final parsed = <String>[];
      final source = List.filled(5000, '$kind large hidden content').join('\n');
      const heading = 'Hidden details';
      Future<void> render(String text) => tester.pumpWidget(
        _surface(
          kind == 'details'
              ? DetailsTagRenderer(
                  xmlContent:
                      '<details><summary>$heading</summary>$text</details>',
                  textColor: Colors.black,
                  isStreaming: false,
                  splitMarkdownContent: (content) {
                    parsed.add(content);
                    return splitMarkdownEventFixture(content);
                  },
                )
              : CustomXmlRenderer(
                  xmlContent: kind == 'thinking'
                      ? '<think>$text</think>'
                      : '<search>$text</search>',
                  textColor: Colors.black,
                  isStreaming: false,
                  splitMarkdownContent: (content) {
                    parsed.add(content);
                    return splitMarkdownEventFixture(content);
                  },
                ),
        ),
      );
      await render(source);
      await tester.pumpAndSettle();
      expect(parsed, isEmpty);
      expect(
        find.byType(StreamMarkdownRenderer, skipOffstage: false),
        findsNothing,
      );
      final title = kind == 'details'
          ? heading
          : kind == 'thinking'
          ? 'Thinking Process'
          : 'Search Sources';
      await tester.tap(find.text(title).first);
      await tester.pumpAndSettle();
      expect(parsed, [source]);
      expect(find.byType(StreamMarkdownRenderer), findsOneWidget);
      await tester.tap(find.text(title).first);
      await tester.pumpAndSettle();
      final requestsAfterClose = parsed.length;
      await render('$kind replacement hidden content');
      await tester.pumpAndSettle();
      expect(parsed.length, requestsAfterClose);
      expect(
        find.byType(StreamMarkdownRenderer, skipOffstage: false),
        findsNothing,
      );
      await tester.tap(find.text(title).first);
      await tester.pumpAndSettle();
      expect(parsed.last, '$kind replacement hidden content');
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('collapsed thinking semantics contain no full hidden body', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    try {
      await tester.pumpWidget(
        _surface(
          CustomXmlRenderer(
            xmlContent:
                '<think>Hidden starting thought ${'visible preview ' * 1000}</think>',
            textColor: Colors.black,
            isStreaming: false,
            splitMarkdownContent: (_) =>
                throw StateError('Collapsed body must not parse'),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(
        find.bySemanticsLabel(RegExp('Hidden starting thought')),
        findsNothing,
      );
      expect(
        find.byType(StreamMarkdownRenderer, skipOffstage: false),
        findsNothing,
      );
    } finally {
      semantics.dispose();
    }
  });

  testWidgets(
    'collapsed tool payload formats on expansion or explicit copy only',
    (tester) async {
      var resolutions = 0;
      var payload = 'complete large JSON payload';
      String buildText() {
        resolutions++;
        return payload;
      }

      Future<void> render() => tester.pumpWidget(
        _surface(
          ToolInlineDetailSurface(
            title: 'Result data',
            text: '',
            initiallyCollapsed: true,
            textBuilder: buildText,
          ),
        ),
      );
      await render();
      await tester.pumpAndSettle();
      expect(resolutions, 0);
      expect(find.byType(SelectableText, skipOffstage: false), findsNothing);
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String;
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      await tester.tap(find.byTooltip('Copy'));
      await tester.pumpAndSettle();
      expect(copied, payload);
      expect(resolutions, 1);
      expect(find.byType(SelectableText, skipOffstage: false), findsNothing);
      await tester.tap(find.text('Result data'));
      await tester.pumpAndSettle();
      expect(find.text(payload), findsOneWidget);
      expect(resolutions, 1);
      await tester.tap(find.text('Result data'));
      await tester.pumpAndSettle();
      payload = 'updated complete payload';
      // A new builder identifies the updated payload while preserving disclosure state.
      await tester.pumpWidget(
        _surface(
          ToolInlineDetailSurface(
            title: 'Result data',
            text: '',
            initiallyCollapsed: true,
            textBuilder: () => buildText(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(resolutions, 1);
      await tester.tap(find.text('Result data'));
      await tester.pumpAndSettle();
      expect(find.text(payload), findsOneWidget);
      expect(resolutions, 2);
    },
  );

  testWidgets('tool parameters stay unformatted until their own fold opens', (
    tester,
  ) async {
    final presentation = _CountingToolPresentation();
    await tester.pumpWidget(
      _surface(ToolCallRow(presentation: presentation, isStreaming: false)),
    );
    await tester.pumpAndSettle();
    expect(presentation.summaryReads, 0);
    expect(find.byType(ToolInlineDetailSurface), findsNothing);

    await tester.tap(find.text('Example tool'));
    await tester.pumpAndSettle();
    expect(find.text('Parameters'), findsOneWidget);
    expect(presentation.summaryReads, 0);
    expect(find.byType(SelectableText, skipOffstage: false), findsNothing);

    await tester.tap(find.text('Parameters'));
    await tester.pumpAndSettle();
    expect(presentation.summaryReads, 1);
    expect(find.text('source: large hidden source'), findsOneWidget);

    await tester.tap(find.text('Parameters'));
    await tester.pumpAndSettle();
    expect(presentation.summaryReads, 1);
    expect(find.byType(SelectableText, skipOffstage: false), findsNothing);
  });

  testWidgets('collapsed file diffs mount only their summary', (tester) async {
    final diff =
        '@@ -1,5000 +1,5000 @@\n${List.filled(5000, '-old\n+new').join('\n')}';
    await tester.pumpWidget(
      _surface(
        FileDiffDisplay(
          diff: FileDiff(
            path: '/workspace/source.dart',
            diffContent: diff,
            details: '',
          ),
          isExpanded: false,
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('+5000'), findsOneWidget);
    expect(find.text('-5000'), findsOneWidget);
    expect(find.byType(WorkspaceInlineDiff, skipOffstage: false), findsNothing);
  });

  test(
    'diff summary ignores file headers and counts changes in multiple hunks',
    () {
      expect(
        unifiedDiffStats(
          '--- a/file\n+++ b/file\n@@ -1,2 +1,2 @@\n context\n-old\n+new\n\\ No newline at end of file\n@@ -20 +20,2 @@\n-previous\n+next\n+another',
        ),
        (additions: 3, deletions: 2),
      );
      expect(unifiedDiffStats('--- a/file\n+++ b/file\n-old\n+new'), (
        additions: 0,
        deletions: 0,
      ));
      expect(unifiedDiffStats('@@ invalid header\n-old\n+new'), (
        additions: 0,
        deletions: 0,
      ));
    },
  );
}
