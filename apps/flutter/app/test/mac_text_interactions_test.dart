import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/code_editor/CodeEditor.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorZoomTransform.dart';
import 'package:operit2/ui/common/markdown/CanvasMonospaceCodeBlockBody.dart';
import 'package:operit2/ui/common/markdown/EnhancedTableBlock.dart';
import 'package:operit2/ui/common/markdown/EnhancedCodeBlock.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/CustomXmlRenderer.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Provides the same editing shortcuts and theme as the chat's text surfaces.
Widget _app(Widget child) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  ),
);

/// Produces paragraph events so clipboard tests exercise real Markdown widgets.
Future<List<MarkdownStreamEvent>> _split(String text) async => [
  for (final type in [
    'markdownBlockStart',
    'markdownInlineStart',
    'markdownInlineChunk',
    'markdownBlockEnd',
  ])
    MarkdownStreamEvent(
      chatId: 'mac-text-interactions',
      eventType: type,
      value: type == 'markdownInlineChunk' ? text : null,
      id: null,
      blockId: 0,
      inlineId: 0,
      parentBlockId: null,
      nodeType: null,
      headerLevel: null,
      xml: null,
    ),
];

/// Sends a complete macOS copy chord through Flutter's keyboard dispatcher.
Future<void> _copy(WidgetTester tester) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
  await tester.sendKeyEvent(LogicalKeyboardKey.keyC);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
  await tester.pump();
}

/// Selects text using the same pointer sequence as a physical mouse drag.
Future<void> _dragSelection(
  WidgetTester tester,
  Offset start,
  Offset end,
) async {
  final mouse = await tester.startGesture(start, kind: PointerDeviceKind.mouse);
  await tester.pump(const Duration(milliseconds: 100));
  for (var frame = 1; frame <= 10; frame++) {
    await mouse.moveTo(
      Offset.lerp(start, end, frame / 10)!,
      timeStamp: Duration(milliseconds: 100 + frame * 20),
    );
    await tester.pump(const Duration(milliseconds: 20));
  }
  await mouse.up();
  await tester.pumpAndSettle();
}

/// Locates a character boundary in a highlighted code paragraph.
Offset _codeCaret(WidgetTester tester, String line, int offset) {
  final paragraph = tester.renderObject<RenderParagraph>(
    find.descendant(
      of: find.byType(CanvasMonospaceCodeBlockBody),
      matching: find.byWidgetPredicate(
        (widget) =>
            widget is RichText &&
            (widget.text.toPlainText() == line ||
                widget.text.toPlainText() == '$line\n'),
      ),
    ),
  );
  return paragraph.localToGlobal(
    paragraph.getOffsetForCaret(TextPosition(offset: offset), Rect.zero) +
        const Offset(0, 7),
  );
}

/// Covers native shortcut ownership and ordinary two-finger trackpad scrolling.
void main() {
  testWidgets(
    'Mac read-only source copies and navigates without an IME',
    (tester) async {
      String? clipboard;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            clipboard = (call.arguments as Map)['text'] as String;
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
      final key = GlobalKey<CodeEditorState>();
      await tester.pumpWidget(
        _app(CodeEditor(key: key, text: 'selected source 中文', readOnly: true)),
      );
      await tester.pumpAndSettle();
      key.currentState!.input.selectAll(SelectionChangedCause.keyboard);
      key.currentState!.input.requestKeyboard();
      await tester.pump();
      expect(tester.testTextInput.hasAnyClients, isFalse);
      await _copy(tester);
      expect(clipboard, 'selected source 中文');
      key.currentState!.input.userUpdateTextEditingValue(
        key.currentState!.input.textEditingValue.copyWith(
          selection: const TextSelection.collapsed(offset: 3),
        ),
        SelectionChangedCause.keyboard,
      );
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      expect(
        key.currentState!.input.textEditingValue.selection.extentOffset,
        4,
      );
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );

  final surfaces = <String, Widget>{
    'chat prose': StreamMarkdownRenderer(
      content: 'chat prose',
      isStreaming: false,
      textColor: Colors.black,
      backgroundColor: Colors.transparent,
      splitMarkdownContent: _split,
    ),
    'chat code': const CanvasMonospaceCodeBlockBody(
      lines: ['chat code'],
      autoWrapEnabled: true,
    ),
    'chat cell': EnhancedTableBlock(
      tableText: '| chat cell |\n|---|',
      textColor: Colors.black,
      splitMarkdownContent: _split,
    ),
    'chat thought': CustomXmlRenderer(
      xmlContent: '<think>chat thought</think>',
      isStreaming: false,
      textColor: Colors.black,
      initialThinkingExpanded: true,
      splitMarkdownContent: _split,
    ),
  };
  for (final surface in surfaces.entries) {
    testWidgets(
      'Mac copies ${surface.key} after leaving an editable input',
      (tester) async {
        String? clipboard;
        tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          (call) async {
            if (call.method == 'Clipboard.setData') {
              clipboard = (call.arguments as Map)['text'] as String;
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
        await tester.pumpWidget(
          _app(
            Column(
              children: [
                const TextField(autofocus: true),
                Expanded(child: SelectionArea(child: surface.value)),
              ],
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(tester.testTextInput.hasAnyClients, isTrue);
        await tester.tap(
          find.text(surface.key, findRichText: true).last,
          kind: PointerDeviceKind.mouse,
        );
        await tester.pumpAndSettle();
        final paragraph = tester.renderObject<RenderParagraph>(
          find
              .byWidgetPredicate(
                (widget) =>
                    widget is RichText &&
                    widget.text.toPlainText() == surface.key,
              )
              .last,
        );
        final start = paragraph.localToGlobal(
          paragraph.getOffsetForCaret(
                const TextPosition(offset: 0),
                Rect.zero,
              ) +
              const Offset(0, 7),
        );
        final end = paragraph.localToGlobal(
          paragraph.getOffsetForCaret(
                TextPosition(offset: surface.key.length),
                Rect.zero,
              ) +
              const Offset(0, 7),
        );
        // Let the previous click's multi-tap window expire before dragging.
        await tester.pump(const Duration(seconds: 1));
        await _dragSelection(tester, start, end);
        await _copy(tester);
        expect(clipboard?.trim(), surface.key);
        expect(tester.takeException(), isNull);
      },
      variant: TargetPlatformVariant.only(TargetPlatform.macOS),
    );
  }

  for (final readOnly in [false, true]) {
    testWidgets(
      'Mac trackpad scroll does not enter zoom mode: $readOnly',
      (tester) async {
        final scroll = CodeEditorScrollController();
        addTearDown(scroll.dispose);
        await tester.pumpWidget(
          _app(
            CodeEditor(
              text: List.generate(500, (i) => 'source line $i').join('\n'),
              readOnly: readOnly,
              scrollController: scroll,
            ),
          ),
        );
        await tester.pumpAndSettle();
        scroll.jumpTo(300);
        await tester.pump();
        final startOffset = scroll.offset;
        final center = tester.getCenter(find.byType(CodeEditorViewport));
        final trackpad = await tester.createGesture(
          kind: PointerDeviceKind.trackpad,
        );
        await trackpad.panZoomStart(center);
        await tester.pump();
        expect(
          tester
              .widget<CodeEditorViewportScope>(
                find.byType(CodeEditorViewportScope),
              )
              .pinching,
          isFalse,
          reason: 'A pan sequence is not a pinch gesture',
        );
        for (var frame = 1; frame <= 5; frame++) {
          await trackpad.panZoomUpdate(center, pan: Offset(0, -frame * 24));
          await tester.pump(const Duration(milliseconds: 16));
        }
        expect(scroll.offset, greaterThan(startOffset + 60));
        expect(
          tester
              .widget<CodeEditorZoomTransform>(
                find.byType(CodeEditorZoomTransform),
              )
              .scale,
          1,
        );
        await trackpad.panZoomUpdate(
          center,
          pan: const Offset(0, -120),
          scale: 1.25,
        );
        await tester.pump();
        expect(
          tester
              .widget<CodeEditorZoomTransform>(
                find.byType(CodeEditorZoomTransform),
              )
              .scale,
          1.25,
        );
        expect(
          tester
              .widget<CodeEditorViewportScope>(
                find.byType(CodeEditorViewportScope),
              )
              .pinching,
          isTrue,
        );
        await trackpad.panZoomEnd();
        await tester.pumpAndSettle();
        expect(
          tester
              .widget<CodeEditorViewportScope>(
                find.byType(CodeEditorViewportScope),
              )
              .pinching,
          isFalse,
        );
        expect(tester.takeException(), isNull);
      },
      variant: TargetPlatformVariant.only(TargetPlatform.macOS),
    );
  }

  for (final wrap in [true, false]) {
    testWidgets(
      'Mac drag-selects nested highlighted code and copies: wrap=$wrap',
      (tester) async {
        String? clipboard;
        tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          (call) async {
            if (call.method == 'Clipboard.setData') {
              clipboard = (call.arguments as Map)['text'] as String;
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
        const first = 'final alpha = 123;';
        const last = 'print(alpha);';
        await tester.pumpWidget(
          _app(
            Column(
              children: [
                const TextField(autofocus: true),
                Expanded(
                  child: SelectionArea(
                    child: ListView(
                      children: const [
                        Text('Before code'),
                        EnhancedCodeBlock(
                          code: '$first\n\n$last',
                          language: 'dart',
                        ),
                        Text('After code'),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
        await tester.pumpAndSettle();
        if (!wrap) {
          await tester.tap(find.byTooltip('Disable word wrap'));
          await tester.pumpAndSettle();
          await tester.tap(find.byType(TextField));
          await tester.pumpAndSettle();
        }
        final start = _codeCaret(tester, first, 6);
        final end = _codeCaret(tester, last, 5);
        await _dragSelection(tester, start, end);
        await _copy(tester);
        expect(clipboard, 'alpha = 123;\n\nprint');
        await tester.pump(const Duration(seconds: 1));
        await _dragSelection(tester, end, start);
        await _copy(tester);
        expect(clipboard, 'alpha = 123;\n\nprint');
        await tester.pump(const Duration(seconds: 1));
        await _dragSelection(
          tester,
          _codeCaret(tester, first, 6),
          _codeCaret(tester, first, 11),
        );
        await _copy(tester);
        expect(clipboard, 'alpha');
        expect(tester.takeException(), isNull);
      },
      variant: TargetPlatformVariant.only(TargetPlatform.macOS),
    );
  }

  testWidgets(
    'Mac copies code source endpoints after scrolling its lazy viewport',
    (tester) async {
      String? clipboard;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            clipboard = (call.arguments as Map)['text'] as String;
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
      await tester.pumpWidget(
        _app(
          Column(
            children: [
              const TextField(autofocus: true),
              Expanded(
                child: SelectionArea(
                  child: ListView(
                    children: [
                      EnhancedCodeBlock(
                        code: List.generate(
                          5000,
                          (i) => 'source row $i',
                        ).join('\n'),
                        language: 'dart',
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      );
      await tester.pumpAndSettle();
      final rowHeight =
          _codeCaret(tester, 'source row 1', 0).dy -
          _codeCaret(tester, 'source row 0', 0).dy;
      final list = tester.widget<ListView>(
        find.descendant(
          of: find.byType(CanvasMonospaceCodeBlockBody),
          matching: find.byType(ListView),
        ),
      );
      list.controller!.jumpTo(rowHeight * 100);
      await tester.pumpAndSettle();
      await _dragSelection(
        tester,
        _codeCaret(tester, 'source row 102', 7),
        _codeCaret(tester, 'source row 104', 10),
      );
      await _copy(tester);
      expect(clipboard, 'row 102\nsource row 103\nsource row');
      list.controller!.jumpTo(rowHeight * 300);
      await tester.pumpAndSettle();
      clipboard = null;
      await _copy(tester);
      expect(clipboard, 'row 102\nsource row 103\nsource row');
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );

  testWidgets(
    'Code selection remains valid when a streamed block replaces its source',
    (tester) async {
      final source = ValueNotifier([
        'first selected line',
        'second selected line',
      ]);
      addTearDown(source.dispose);
      await tester.pumpWidget(
        _app(
          SelectionArea(
            child: ValueListenableBuilder<List<String>>(
              valueListenable: source,
              builder: (context, lines, _) => CanvasMonospaceCodeBlockBody(
                lines: lines,
                autoWrapEnabled: true,
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await _dragSelection(
        tester,
        _codeCaret(tester, 'first selected line', 4),
        _codeCaret(tester, 'second selected line', 15),
      );
      source.value = ['x'];
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      expect(find.text('x', findRichText: true), findsOneWidget);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
