import 'dart:math';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/code_editor/CodeTextLayout.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorInput.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorSurface.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorScrollController.dart';

/// Exercises viewport-bounded paragraphs, incremental indexing, and native input.
void main() {
  test(
    'incremental source index preserves mixed separators through local edits',
    () {
      var text = '零👩‍💻e\u0301\r\n\r\nsecond\rthird\nlast';
      final layout = CodeTextLayout(text);
      addTearDown(layout.dispose);
      final random = Random(714);
      const insertions = ['', '\n', '\r', '\r\n', '中', 'a\nb', '👩‍💻'];
      for (var edit = 0; edit < 600; edit++) {
        final start = random.nextInt(text.length + 1);
        final end = start + random.nextInt(text.length - start + 1);
        text = text.replaceRange(
          start,
          end,
          insertions[random.nextInt(insertions.length)],
        );
        layout.updateText(text);
        expect(
          layout.lines.map((line) => line.content + line.separator).join(),
          text,
        );
        final starts = [
          0,
          for (final match in RegExp(r'\r\n|\r|\n').allMatches(text)) match.end,
        ];
        expect(layout.lines.map((line) => line.start).toList(), starts);
        for (var offset = 0; offset <= text.length; offset++) {
          expect(
            layout.lineAtOffset(offset),
            starts.lastIndexWhere((start) => start <= offset),
          );
        }
      }
    },
  );

  test(
    'editing one line retains unaffected shaped paragraphs and exact heights',
    () {
      final layout = CodeTextLayout(
        'first\r\n${'wrapped 中文 ' * 20}\r\nlast',
      );
      addTearDown(layout.dispose);
      layout.configure(
        160,
        const TextStyle(fontSize: 14, height: 1.45),
        TextScaler.noScaling,
        TextDirection.ltr,
      );
      final first = layout.paragraph(0);
      final last = layout.paragraph(2);
      final prior = layout.lines[2];
      layout.paragraph(1);
      final oldLayouts = layout.debugParagraphLayouts;
      layout.updateText('first\r\ninserted\r\n${'wrapped 中文 ' * 20}\r\nlast');
      expect(identical(layout.paragraph(0), first), isTrue);
      expect(identical(layout.paragraph(3), last), isTrue);
      expect(identical(layout.lines[3], prior), isTrue);
      expect(layout.debugParagraphLayouts, oldLayouts);
      layout.paragraph(1);
      expect(
        layout.topOf(4),
        closeTo(
          layout.lines.fold<double>(0, (sum, line) => sum + line.height!),
          0.0001,
        ),
      );
    },
  );

  test(
    'font invalidation retains geometry state until the next wrapping epoch',
    () {
      final layout = CodeTextLayout('first\n${'wrapped content ' * 30}');
      addTearDown(layout.dispose);
      const style = TextStyle(fontSize: 14, height: 1.45);
      layout.configure(160, style, TextScaler.noScaling, TextDirection.ltr);
      final first = layout.paragraph(0);
      final priorHeight = layout.height;
      layout.invalidateFonts();
      final uncached = layout.paragraph(1, updateHeight: false);
      expect(uncached.height, greaterThan(layout.lineHeight));
      expect(layout.height, priorHeight);
      expect(layout.cachedParagraphs, 2);
      layout.configure(160, style, TextScaler.noScaling, TextDirection.ltr);
      expect(layout.cachedParagraphs, 0);
      expect(identical(layout.paragraph(0), first), isFalse);
    },
  );
  test(
    'caret geometry cannot change indexed heights after a painted frame',
    () {
      final source = List.filled(200, 'short');
      source[0] = 'long content ' * 60;
      final layout = CodeTextLayout(source.join('\n'));
      addTearDown(layout.dispose);
      layout.configure(
        160,
        const TextStyle(fontSize: 14, height: 1.45),
        TextScaler.noScaling,
        TextDirection.ltr,
      );
      layout.paragraph(100);
      final before = layout.topOf(100);
      layout.paragraph(0, updateHeight: false);
      expect(layout.topOf(100), before);
      final count = layout.debugParagraphLayouts;
      layout.paragraph(0);
      expect(layout.debugParagraphLayouts, count);
      expect(layout.topOf(100), greaterThan(before));
    },
  );
  test(
    'unvisited heights use measured wrapping without shaping the remaining document',
    () {
      final layout = CodeTextLayout(
        List.filled(10000, 'wrapped content ' * 30).join('\n'),
      );
      addTearDown(layout.dispose);
      layout.configure(
        160,
        const TextStyle(fontSize: 14, height: 1.45),
        TextScaler.noScaling,
        TextDirection.ltr,
      );
      final height = layout.paragraph(0).height;
      expect(layout.height, closeTo(height * 10000, 0.001));
      expect(layout.debugParagraphLayouts, 1);
      for (var i = 0; i < 10000; i += 77) {
        expect(layout.lineAtY((i + 0.25) * height), i);
      }
    },
  );
  testWidgets('ten thousand logical lines shape only a viewport working set', (
    tester,
  ) async {
    await tester.pumpWidget(
      _Harness(
        text: List.filled(
          10000,
          '中文 e\u0301 👩‍💻 ${'wrap ' * 12}',
        ).join('\r\n'),
      ),
    );
    await tester.pumpAndSettle();
    final state = tester.state<CodeEditorInputState>(
      find.byType(CodeEditorInput),
    );
    final render = state.renderEditable;
    expect(render.debugParagraphLayouts, lessThan(100));
    expect(render.debugPaintedParagraphs, lessThan(50));
    final initialLayouts = render.debugParagraphLayouts;
    for (var frame = 0; frame < 20; frame++) {
      render.markNeedsPaint();
      await tester.pump();
    }
    expect(render.debugParagraphLayouts, initialLayouts);
    state.widget.scrollController.jumpTo(70000);
    await tester.pump();
    expect(render.debugParagraphLayouts - initialLayouts, lessThan(100));
    final beforeSelect = render.debugParagraphLayouts;
    state.widget.controller.selection = TextSelection(
      baseOffset: 0,
      extentOffset: state.widget.controller.text.length,
    );
    await tester.pump();
    expect(
      render.debugParagraphLayouts,
      beforeSelect,
      reason: 'A document-wide selection paints only intersecting paragraphs',
    );
    for (var page = 0; page < 150; page++) {
      state.widget.scrollController.jumpTo(page * 900.0);
      await tester.pump();
      expect(render.debugCachedParagraphs, lessThanOrEqualTo(128));
    }
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'offscreen selection edits preserve full source and CRLF with IME undo',
    (tester) async {
      final original = List.generate(
        3000,
        (i) => 'line $i 中文 👩‍💻',
      ).join('\r\n');
      await tester.pumpWidget(_Harness(text: original));
      await tester.pumpAndSettle();
      final state = tester.state<CodeEditorInputState>(
        find.byType(CodeEditorInput),
      );
      state.requestKeyboard();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 600));
      final start = original.indexOf('line 2000');
      final end = original.indexOf('line 2003');
      final edited = original.replaceRange(start, end, '中文输入\r\n👩‍💻');
      final caret = start + '中文输入\r\n👩‍💻'.length;
      tester.testTextInput.updateEditingValue(
        TextEditingValue(
          text: edited,
          selection: TextSelection.collapsed(offset: caret),
          composing: TextRange(start: start, end: caret),
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(state.currentTextEditingValue.text, edited);
      expect(
        state.currentTextEditingValue.composing,
        TextRange(start: start, end: caret),
      );
      tester.testTextInput.updateEditingValue(
        state.currentTextEditingValue.copyWith(composing: TextRange.empty),
      );
      await tester.pump(const Duration(milliseconds: 600));
      state.widget.undoController.undo();
      await tester.pump();
      expect(state.currentTextEditingValue.text, original);
      state.widget.undoController.redo();
      await tester.pump();
      expect(state.currentTextEditingValue.text, edited);
      expect(
        state.renderEditable.debugCachedParagraphs,
        lessThanOrEqualTo(128),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('keyboard editing moves and deletes complete Unicode graphemes', (
    tester,
  ) async {
    await tester.pumpWidget(const _Harness(text: 'A👩‍💻e\u0301\r\n中文'));
    final state = tester.state<CodeEditorInputState>(
      find.byType(CodeEditorInput),
    );
    state.requestKeyboard();
    state.widget.controller.selection = const TextSelection.collapsed(
      offset: 8,
    );
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
    expect(state.widget.controller.selection.extentOffset, 10);
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    expect(state.widget.controller.selection.extentOffset, 8);
    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    await tester.pump();
    expect(state.widget.controller.text, 'A👩‍💻\r\n中文');
    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    await tester.pump();
    expect(state.widget.controller.text, 'A\r\n中文');
    expect(state.widget.controller.selection.extentOffset, 1);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'native keyboard remains ordinary multiline input across refocus',
    (tester) async {
      await tester.pumpWidget(const _Harness(text: 'first\r\n'));
      final state = tester.state<CodeEditorInputState>(
        find.byType(CodeEditorInput),
      );
      state.requestKeyboard();
      await tester.pump();
      _expectOrdinaryKeyboard(tester);
      const value = TextEditingValue(
        text: 'first\r\n中文',
        selection: TextSelection.collapsed(offset: 9),
        composing: TextRange(start: 7, end: 9),
      );
      tester.testTextInput.updateEditingValue(value);
      await tester.pump();
      expect(state.textEditingValue, value);
      tester.testTextInput.updateEditingValue(
        value.copyWith(composing: TextRange.empty),
      );
      await tester.pump();
      state.widget.focusNode.unfocus();
      await tester.pump();
      expect(tester.testTextInput.hasAnyClients, isFalse);
      state.requestKeyboard();
      await tester.pump();
      _expectOrdinaryKeyboard(tester);
      expect(state.textEditingValue.text, value.text);
      expect(state.textEditingValue.selection, value.selection);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('IME newline action does not insert a duplicate newline', (
    tester,
  ) async {
    await tester.pumpWidget(const _Harness(text: 'first'));
    final state = tester.state<CodeEditorInputState>(
      find.byType(CodeEditorInput),
    );
    state.requestKeyboard();
    await tester.pump();
    tester.testTextInput.enterText('first\n');
    await tester.testTextInput.receiveAction(TextInputAction.newline);
    await tester.pump();
    expect(state.currentTextEditingValue.text, 'first\n');
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'long press handles and cross-line dragging use global source ranges',
    (tester) async {
      await tester.pumpWidget(
        const _Harness(text: 'first word\r\nsecond word\r\nthird word'),
      );
      await tester.pumpAndSettle();
      final state = tester.state<CodeEditorInputState>(
        find.byType(CodeEditorInput),
      );
      final render = state.renderEditable;
      final start = render.localToGlobal(
        render.getLocalRectForCaret(const TextPosition(offset: 2)).center,
      );
      final end = render.localToGlobal(
        render.getLocalRectForCaret(const TextPosition(offset: 19)).center,
      );
      final gesture = await tester.startGesture(
        start,
        kind: PointerDeviceKind.touch,
      );
      await tester.pump(const Duration(milliseconds: 600));
      expect(state.currentTextEditingValue.selection.isCollapsed, isFalse);
      await gesture.moveTo(end);
      await tester.pump();
      await gesture.up();
      await tester.pumpAndSettle();
      final selection = state.currentTextEditingValue.selection;
      expect(selection.start, 0);
      expect(selection.end, greaterThan(12));
      expect(
        selection.textInside(state.currentTextEditingValue.text),
        startsWith('first word\r\nsecond'),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'Flutter native selectors and multi-event input keep global source indices current',
    (tester) async {
      await tester.pumpWidget(const _Harness(text: 'old'));
      final state = tester.state<CodeEditorInputState>(
        find.byType(CodeEditorInput),
      );
      state.requestKeyboard();
      await tester.pump();
      tester.testTextInput.enterText('first\r\nsecond e\u0301 👩‍💻');
      // Native command notifications can follow an IME edit before another frame.
      state.performSelector('moveToBeginningOfDocument:');
      expect(state.currentTextEditingValue.selection.extentOffset, 0);
      state.performSelector('moveToEndOfDocument:');
      expect(
        state.currentTextEditingValue.selection.extentOffset,
        state.currentTextEditingValue.text.length,
      );
      state.performSelector('deleteBackward:');
      expect(state.currentTextEditingValue.text, 'first\r\nsecond e\u0301 ');
      state.performSelector('moveToBeginningOfDocumentAndModifySelection:');
      expect(state.currentTextEditingValue.selection.start, 0);
      expect(
        state.currentTextEditingValue.selection.end,
        state.currentTextEditingValue.text.length,
      );
      expect(
        state.currentTextEditingValue.selection.baseOffset,
        state.currentTextEditingValue.text.length,
      );
      expect(state.currentTextEditingValue.selection.extentOffset, 0);
      state.performSelector('moveToEndOfDocument:');
      state.performSelector('transpose:');
      expect(state.currentTextEditingValue.text, 'first\r\nsecond  e\u0301');
      await tester.pump();
      expect(
        state.renderEditable.textLayout.lines
            .map((line) => line.content + line.separator)
            .join(),
        state.currentTextEditingValue.text,
      );
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets(
    'semantics exposes full source instead of only the visible page',
    (tester) async {
      final text = List.generate(500, (i) => 'line $i').join('\n');
      await tester.pumpWidget(_Harness(text: text));
      await tester.pumpAndSettle();
      final state = tester.state<CodeEditorInputState>(
        find.byType(CodeEditorInput),
      );
      final semantics = tester.ensureSemantics();

      expect(
        tester
            .getSemantics(find.byType(CodeEditorSurface))
            .getSemanticsData()
            .value,
        text,
      );
      expect(
        state.renderEditable.debugCachedParagraphs,
        lessThanOrEqualTo(128),
      );
      expect(tester.takeException(), isNull);
      semantics.dispose();
    },
  );
}

/// Verifies native input flags without conflating candidate suggestions and correction.
void _expectOrdinaryKeyboard(WidgetTester tester) {
  final configuration = tester.testTextInput.setClientArgs!;
  expect(tester.testTextInput.isVisible, isTrue);
  expect(configuration['inputType'], TextInputType.multiline.toJson());
  expect(configuration['inputAction'], TextInputAction.newline.toString());
  expect(configuration['readOnly'], isFalse);
  expect(configuration['obscureText'], isFalse);
  expect(configuration['enableSuggestions'], isTrue);
  expect(configuration['autocorrect'], isFalse);
  expect(
    configuration['smartDashesType'],
    SmartDashesType.disabled.index.toString(),
  );
  expect(
    configuration['smartQuotesType'],
    SmartQuotesType.disabled.index.toString(),
  );
}

/// Owns editor resources for isolated virtual-renderer and input regression tests.
class _Harness extends StatefulWidget {
  /// Creates a fixed-size editor with the complete source document.
  const _Harness({required this.text});
  final String text;

  /// Creates the controllers shared by the input and render surface.
  @override
  State<_Harness> createState() => _HarnessState();
}

class _HarnessState extends State<_Harness> {
  late final TextEditingController _controller;
  final FocusNode _focus = FocusNode();
  final UndoHistoryController _undo = UndoHistoryController();
  final CodeEditorScrollController _scroll = CodeEditorScrollController();

  /// Seeds valid UTF-16 input state before building the editor.
  @override
  void initState() {
    super.initState();
    _controller = TextEditingController.fromValue(
      TextEditingValue(
        text: widget.text,
        selection: const TextSelection.collapsed(offset: 0),
      ),
    );
  }

  /// Releases controllers when the test editor unmounts.
  @override
  void dispose() {
    _controller.dispose();
    _focus.dispose();
    _undo.dispose();
    _scroll.dispose();
    super.dispose();
  }

  /// Observes no persistence side effects in render-only tests.
  void _textChanged(String text) {}

  /// Builds the same full-document input pipeline in a bounded Material surface.
  @override
  Widget build(BuildContext context) => MaterialApp(
    home: Scaffold(
      body: Center(
        child: SizedBox(
          width: 360,
          height: 500,
          child: CodeEditorInput(
            controller: _controller,
            focusNode: _focus,
            undoController: _undo,
            scrollController: _scroll,
            groupId: this,
            active: true,
            onChanged: _textChanged,
            scrollPhysics: null,
            style: const TextStyle(
              fontFamily: 'monospace',
              fontSize: 14,
              height: 1.45,
              color: Colors.black,
            ),
          ),
        ),
      ),
    ),
  );
}
