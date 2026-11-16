import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/code_editor/CodeEditor.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorInput.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorLineNumbers.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorZoomTransform.dart';

Widget app(Widget child) => MaterialApp(home: Scaffold(body: child));

void main() {
  testWidgets(
    'read-only source uses editor syntax and selection without input or edits',
    (tester) async {
      String? clipboard;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            clipboard = (call.arguments as Map)['text'] as String;
          }
          if (call.method == 'Clipboard.getData') return {'text': clipboard};
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
        app(
          CodeEditor(
            key: key,
            text: 'const answer = 42;',
            filePath: 'main.ts',
            readOnly: true,
          ),
        ),
      );
      await tester.pumpAndSettle();
      final input = tester.widget<CodeEditorInput>(
        find.byType(CodeEditorInput),
      );
      final state = key.currentState!.input;
      expect(input.language, CodeSyntaxLanguage.typescript);
      expect(state.renderEditable.readOnly, isTrue);
      state.selectAll(SelectionChangedCause.keyboard);
      state.requestKeyboard();
      await tester.pump();
      expect(tester.testTextInput.hasAnyClients, isFalse);
      state.copySelection(SelectionChangedCause.toolbar);
      await tester.pump();
      expect(clipboard, 'const answer = 42;');
      state.cutSelection(SelectionChangedCause.toolbar);
      clipboard = 'replacement';
      await state.pasteText(SelectionChangedCause.toolbar);
      state.userUpdateTextEditingValue(
        const TextEditingValue(
          text: 'replacement',
          selection: TextSelection.collapsed(offset: 0),
        ),
        SelectionChangedCause.keyboard,
      );
      expect(input.controller.text, 'const answer = 42;');
      await tester.pumpWidget(
        app(
          CodeEditor(
            key: key,
            text: 'const updated = true;',
            filePath: 'main.ts',
            readOnly: true,
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(state.textEditingValue.text, 'const updated = true;');
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'editor can change between editable and read-only without losing text',
    (tester) async {
      final controller = TextEditingController(text: 'before');
      addTearDown(controller.dispose);
      final key = GlobalKey<CodeEditorState>();
      Widget editor(bool readOnly) =>
          app(CodeEditor(key: key, controller: controller, readOnly: readOnly));
      await tester.pumpWidget(editor(false));
      key.currentState!.input.requestKeyboard();
      await tester.pump();
      tester.testTextInput.enterText('after');
      await tester.pump();
      expect(controller.text, 'after');
      await tester.pumpWidget(editor(true));
      await tester.pump();
      expect(tester.testTextInput.hasAnyClients, isFalse);
      key.currentState!.input.selectAll(SelectionChangedCause.keyboard);
      key.currentState!.input.cutSelection(SelectionChangedCause.keyboard);
      expect(controller.text, 'after');
      await tester.pumpWidget(editor(false));
      key.currentState!.input.requestKeyboard();
      await tester.pump();
      tester.testTextInput.enterText('editable again');
      await tester.pump();
      expect(controller.text, 'editable again');
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'read-only editor pans both axes and exposes configurable zoom bounds',
    (tester) async {
      final key = GlobalKey<CodeEditorState>();
      final scroll = CodeEditorScrollController();
      addTearDown(scroll.dispose);
      await tester.pumpWidget(
        app(
          SizedBox(
            width: 400,
            height: 250,
            child: CodeEditor(
              key: key,
              text: List.generate(
                300,
                (i) => 'const sourceLine$i = true;',
              ).join('\n'),
              readOnly: true,
              showLineNumbers: false,
              scrollController: scroll,
              wrapToViewport: false,
              contentWidth: 1200,
              minScale: 0.8,
              maxScale: 1.8,
              padding: const EdgeInsets.all(20),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final transform = find.byType(CodeEditorZoomTransform);
      final start = tester.getCenter(transform);
      final gesture = await tester.startGesture(start);
      await gesture.moveBy(const Offset(-50, -50));
      await tester.pump();
      await gesture.moveBy(const Offset(-30, -30));
      await tester.pump();
      expect(
        tester.widget<CodeEditorZoomTransform>(transform).horizontalOffset,
        lessThan(0),
      );
      expect(scroll.offset, greaterThan(0));
      await gesture.up();
      await tester.pumpAndSettle();
      key.currentState!.zoomTo(10);
      await tester.pumpAndSettle();
      expect(tester.widget<CodeEditorZoomTransform>(transform).scale, 1.8);
      key.currentState!.zoomTo(0.1);
      await tester.pumpAndSettle();
      expect(tester.widget<CodeEditorZoomTransform>(transform).scale, 0.8);
      expect(
        tester
            .widget<CodeEditorLineNumbers>(find.byType(CodeEditorLineNumbers))
            .enabled,
        isFalse,
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    },
  );
}
