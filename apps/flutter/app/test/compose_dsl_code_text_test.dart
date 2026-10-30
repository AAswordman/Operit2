import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/syntax/WorkspaceSyntaxPalette.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/syntax/WorkspaceSyntaxToken.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Exercises the source renderer inside the file-preview selection area.
Widget codePreview(
  String path,
  String source,
  Brightness brightness, {
  List<Map<String, Object?>> emphasis = const [],
}) => MaterialApp(
  theme: ThemeData(brightness: brightness),
  home: Scaffold(
    body: buildComposeDslLayoutForTest(
      node: {
        'type': 'SelectionContainer',
        'children': [
          {
            'type': 'CodeText',
            'props': {
              'text': source,
              'filePath': path,
              'emphasis': emphasis,
              'fontFamily': 'monospace',
            },
          },
        ],
      },
      onTextInput: (_, _) async => null,
      hostContext: ComposeDslWebViewHostContext(
        packageName: 'test.code-preview',
        routeInstanceId: 'code-preview',
        executionContextKey: 'code-preview',
        dispatchAction: (_, [payload]) async => null,
        runtimeOptionsProvider: () => <String, Object?>{},
      ),
      splitMarkdownContent: (_) async => [],
    ),
  ),
);

/// Finds an offset's effective style independently of span boundaries.
TextStyle sourceStyle(TextSpan root, int offset) {
  var cursor = 0;
  TextStyle? found;
  void visit(TextSpan span, TextStyle inherited) {
    final style = inherited.merge(span.style);
    final text = span.text ?? '';
    if (cursor <= offset && offset < cursor + text.length) found = style;
    cursor += text.length;
    for (final child in span.children ?? const <InlineSpan>[]) {
      visit(child as TextSpan, style);
    }
  }

  visit(root, const TextStyle());
  return found!;
}

/// Reads actual styled source rather than selection-overlay text.
TextSpan sourceSpan(WidgetTester tester, String source) =>
    tester
            .widget<Text>(
              find.byWidgetPredicate(
                (widget) =>
                    widget is Text && widget.textSpan?.toPlainText() == source,
              ),
            )
            .textSpan!
        as TextSpan;

void main() {
  for (final brightness in Brightness.values) {
    testWidgets(
      'whole-file syntax preserves line and block comments in $brightness',
      (tester) async {
        const source =
            '// header\r\nconst answer = 42;\r\n/* block\r\ncontinued */\r\nconst end = true;\r\n';
        final number = source.indexOf('42');
        await tester.pumpWidget(
          codePreview(
            '/workspace/main.ts',
            source,
            brightness,
            emphasis: [
              {
                'start': number,
                'end': number + 2,
                'color': '#FFFF0000',
                'fontWeight': '700',
              },
            ],
          ),
        );
        expect(tester.takeException(), isNull);
        expect(find.byType(SelectionArea), findsOneWidget);
        final span = sourceSpan(tester, source);
        final palette = WorkspaceSyntaxPalette(brightness);
        expect(span.toPlainText(), source);
        expect(
          sourceStyle(span, source.indexOf('header')).color,
          palette.color(WorkspaceSyntaxTokenKind.comment),
        );
        expect(
          sourceStyle(span, source.indexOf('continued')).color,
          palette.color(WorkspaceSyntaxTokenKind.comment),
        );
        for (final offset in [
          source.indexOf('const'),
          source.lastIndexOf('const'),
        ]) {
          expect(
            sourceStyle(span, offset).color,
            palette.color(WorkspaceSyntaxTokenKind.keyword),
          );
        }
        final emphasized = sourceStyle(span, number);
        expect(
          emphasized.color,
          palette.color(WorkspaceSyntaxTokenKind.number),
        );
        expect(emphasized.backgroundColor, const Color(0xFFFF0000));
        expect(emphasized.fontWeight, FontWeight.w700);
      },
    );

    testWidgets(
      'HTML preview highlights tags after multiline comments in $brightness',
      (tester) async {
        const source =
            '<!-- header\ncontinued -->\n<html\n  lang="zh-CN">\n</html>\n';
        await tester.pumpWidget(
          codePreview('/workspace/index.html', source, brightness),
        );
        expect(tester.takeException(), isNull);
        final span = sourceSpan(tester, source);
        final palette = WorkspaceSyntaxPalette(brightness);
        expect(span.toPlainText(), source);
        expect(
          sourceStyle(span, source.indexOf('continued')).color,
          palette.color(WorkspaceSyntaxTokenKind.comment),
        );
        expect(
          sourceStyle(span, source.indexOf('<html')).color,
          palette.color(WorkspaceSyntaxTokenKind.tag),
        );
        expect(
          sourceStyle(span, source.indexOf('lang')).color,
          palette.color(WorkspaceSyntaxTokenKind.property),
        );
        expect(
          sourceStyle(span, source.indexOf('zh-CN')).color,
          palette.color(WorkspaceSyntaxTokenKind.string),
        );
      },
    );
  }

  testWidgets('unknown extensions preserve selectable literal text', (
    tester,
  ) async {
    const source = '// literal\nconst answer = 42;\n';
    await tester.pumpWidget(
      codePreview('/workspace/notes.txt', source, Brightness.light),
    );
    expect(tester.takeException(), isNull);
    expect(find.byType(SelectionArea), findsOneWidget);
    final span = sourceSpan(tester, source);
    expect(span.toPlainText(), source);
    expect(
      sourceStyle(span, source.indexOf('//')).color,
      sourceStyle(span, source.indexOf('const')).color,
    );
  });
}
